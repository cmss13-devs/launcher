#!/usr/bin/env bash
# Downloads a pinned WebView2 Fixed Version Runtime (x64) from NuGet for Linux/Wine.
# Pinned to v122, the newest version confirmed working under Wine 10.5.
# To bump: test newer versions under Wine, then update WEBVIEW2_VERSION below.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_DIR="${1:-$SCRIPT_DIR/../src-tauri/webview2-runtime}"

WEBVIEW2_VERSION="122.0.2365.92"
NUGET_URL="https://api.nuget.org/v3-flatcontainer/webview2.runtime.x64/${WEBVIEW2_VERSION}/webview2.runtime.x64.${WEBVIEW2_VERSION}.nupkg"

echo "WebView2 Fixed Version: $WEBVIEW2_VERSION (pinned for Wine compatibility)"

rm -rf "$OUTPUT_DIR"

TMPFILE=$(mktemp /tmp/webview2-XXXXXX.nupkg)
TMPDIR_EXTRACT=$(mktemp -d /tmp/webview2-extract-XXXXXX)

echo "Downloading from NuGet..."
curl -sL -o "$TMPFILE" "$NUGET_URL"

echo "Extracting..."
unzip -q -o "$TMPFILE" "contentFiles/any/any/WebView2/*" -d "$TMPDIR_EXTRACT"
mv "$TMPDIR_EXTRACT/contentFiles/any/any/WebView2" "$OUTPUT_DIR"

rm -f "$TMPFILE"
rm -rf "$TMPDIR_EXTRACT"

if [ ! -f "$OUTPUT_DIR/msedgewebview2.exe" ]; then
    echo "Contents of output dir:"
    ls "$OUTPUT_DIR"
    echo "ERROR: Extraction failed: msedgewebview2.exe not found"
    exit 1
fi

echo "WebView2 fixed runtime v$WEBVIEW2_VERSION ready at $OUTPUT_DIR"
