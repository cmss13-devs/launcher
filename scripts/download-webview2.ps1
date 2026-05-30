param(
    [string]$OutputDir = "$PSScriptRoot\..\src-tauri\webview2-runtime"
)

$ErrorActionPreference = "Stop"

$WebView2Version = "148.0.3967.96"
$CabUrl = "https://msedge.sf.dl.delivery.mp.microsoft.com/filestreamingservice/files/12306b32-d97b-470c-ab29-7c2f0a4f46c1/Microsoft.WebView2.FixedVersionRuntime.148.0.3967.96.x64.cab"

Write-Host "WebView2 Fixed Version: $WebView2Version"

if (Test-Path $OutputDir) {
    Write-Host "Output directory already exists, removing: $OutputDir"
    Remove-Item -Recurse -Force $OutputDir
}

$CabPath = Join-Path $env:TEMP "webview2-fixed-$WebView2Version.cab"
$ExtractDir = Join-Path $env:TEMP "webview2-extract-$WebView2Version"

Write-Host "Downloading WebView2 fixed runtime..."
Invoke-WebRequest -Uri $CabUrl -OutFile $CabPath -UseBasicParsing

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

Write-Host "WebView2 fixed runtime v$WebView2Version ready at $OutputDir"
