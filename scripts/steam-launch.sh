#!/bin/bash
# Steam launch script for CM-SS13 Launcher (sharun format)

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

export APPDIR="$SCRIPT_DIR"

# Create path compatibility symlink for x86_64-linux-gnu
if [ ! -e "$SCRIPT_DIR/lib/x86_64-linux-gnu" ]; then
    ln -sf . "$SCRIPT_DIR/lib/x86_64-linux-gnu"
fi

# Read the random path mapping string from the hook and create APPDIR/tmp symlink
# (quick-sharun.sh patches /usr/lib to /tmp/{random}, but Tauri resolves it as $APPDIR/tmp/{random})
if [ -f "$SCRIPT_DIR/bin/path-mapping-hardcoded.hook" ]; then
    _tmp_lib=$(grep '_tmp_lib=' "$SCRIPT_DIR/bin/path-mapping-hardcoded.hook" | cut -d'=' -f2 | tr -d '"')
    if [ -n "$_tmp_lib" ]; then
        mkdir -p "$SCRIPT_DIR/tmp"
        if [ ! -e "$SCRIPT_DIR/tmp/$_tmp_lib" ]; then
            ln -sf "$SCRIPT_DIR/lib" "$SCRIPT_DIR/tmp/$_tmp_lib"
        fi
    fi
fi

exec "$SCRIPT_DIR/AppRun" "$@"
