param(
    [string]$OutputDir = "$PSScriptRoot\..\src-tauri\webview2-runtime"
)

$ErrorActionPreference = "Stop"

Write-Host "Fetching latest WebView2 Fixed Version download URL..."

$page = Invoke-WebRequest -Uri "https://developer.microsoft.com/en-us/microsoft-edge/webview2/" -UseBasicParsing
$nuxtData = ($page.Content | Select-String -Pattern '(?s)<script[^>]*id="__NUXT_DATA__"[^>]*>(.*?)</script>').Matches[0].Groups[1].Value
$json = $nuxtData | ConvertFrom-Json

# Walk the Nuxt data array to find the first x64 cab URL
$cabUrl = $null
$version = $null
for ($i = 0; $i -lt $json.Count; $i++) {
    $val = $json[$i]
    if ($val -is [string] -and $val -match 'FixedVersionRuntime.*\.x64\.cab$') {
        $cabUrl = $val
        break
    }
}

for ($i = 0; $i -lt $json.Count; $i++) {
    $val = $json[$i]
    if ($val -is [string] -and $val -match '^\d+\.\d+\.\d+\.\d+$') {
        $version = $val
        break
    }
}

if (-not $cabUrl) {
    Write-Error "Could not find WebView2 Fixed Version x64 download URL"
    exit 1
}

Write-Host "WebView2 Fixed Version: $version"
Write-Host "Download URL: $cabUrl"

if (Test-Path $OutputDir) {
    Remove-Item -Recurse -Force $OutputDir
}

$CabPath = Join-Path $env:TEMP "webview2-fixed.cab"
$ExtractDir = Join-Path $env:TEMP "webview2-extract"

Write-Host "Downloading..."
Invoke-WebRequest -Uri $cabUrl -OutFile $CabPath -UseBasicParsing

Write-Host "Extracting..."
if (Test-Path $ExtractDir) { Remove-Item -Recurse -Force $ExtractDir }
New-Item -ItemType Directory -Force -Path $ExtractDir | Out-Null
expand $CabPath -F:* $ExtractDir | Out-Null

$SubDir = Get-ChildItem -Path $ExtractDir -Directory | Select-Object -First 1
if ($SubDir) {
    Move-Item -Path $SubDir.FullName -Destination $OutputDir
}
else {
    Move-Item -Path $ExtractDir -Destination $OutputDir
}

Remove-Item $CabPath -Force -ErrorAction SilentlyContinue
Remove-Item $ExtractDir -Recurse -Force -ErrorAction SilentlyContinue

if (-not (Test-Path (Join-Path $OutputDir "msedgewebview2.exe"))) {
    Write-Host "Contents of output dir:"
    Get-ChildItem $OutputDir | Select-Object Name
    Write-Error "Extraction failed: msedgewebview2.exe not found in $OutputDir"
    exit 1
}

Write-Host "WebView2 fixed runtime v$version ready at $OutputDir"
