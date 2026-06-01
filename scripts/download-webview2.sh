#!/usr/bin/env bash
# Downloads the latest WebView2 Fixed Version Runtime (x64) for bundling.
# Scrapes the version and URL from Microsoft's download page.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_DIR="${1:-$SCRIPT_DIR/../src-tauri/webview2-runtime}"

echo "Fetching latest WebView2 Fixed Version download URL..."

read -r VERSION CAB_URL < <(python3 -c "
import re, json, urllib.request
req = urllib.request.Request(
    'https://developer.microsoft.com/en-us/microsoft-edge/webview2/',
    headers={'User-Agent': 'Mozilla/5.0'}
)
page = urllib.request.urlopen(req).read().decode('utf-8')
match = re.search(r'<script[^>]*id=\"__NUXT_DATA__\"[^>]*>(.*?)</script>', page, re.DOTALL)
data = json.loads(match.group(1))
version = url = None
for item in data:
    if isinstance(item, str):
        if not version and re.match(r'^\d+\.\d+\.\d+\.\d+$', item):
            version = item
        if not url and 'FixedVersionRuntime' in item and 'x64.cab' in item:
            url = item
    if version and url:
        break
print(f'{version} {url}')
")

if [ -z "$CAB_URL" ]; then
    echo "ERROR: Could not find WebView2 Fixed Version x64 download URL"
    exit 1
fi

echo "WebView2 Fixed Version: $VERSION"
echo "Download URL: $CAB_URL"

rm -rf "$OUTPUT_DIR"

TMPFILE=$(mktemp /tmp/webview2-fixed-XXXXXX.cab)
TMPDIR_EXTRACT=$(mktemp -d /tmp/webview2-extract-XXXXXX)

echo "Downloading..."
curl -sL -o "$TMPFILE" "$CAB_URL"

echo "Extracting..."
cabextract -q -d "$TMPDIR_EXTRACT" "$TMPFILE"

# The .cab extracts into a versioned subdirectory — move its contents up
SUBDIR=$(find "$TMPDIR_EXTRACT" -mindepth 1 -maxdepth 1 -type d | head -1)
if [ -n "$SUBDIR" ]; then
    mv "$SUBDIR" "$OUTPUT_DIR"
else
    mv "$TMPDIR_EXTRACT" "$OUTPUT_DIR"
fi

rm -f "$TMPFILE"
rm -rf "$TMPDIR_EXTRACT"

if [ ! -f "$OUTPUT_DIR/msedgewebview2.exe" ]; then
    echo "Contents of output dir:"
    ls "$OUTPUT_DIR"
    echo "ERROR: Extraction failed: msedgewebview2.exe not found"
    exit 1
fi

echo "WebView2 fixed runtime v$VERSION ready at $OUTPUT_DIR"
