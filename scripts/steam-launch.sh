#!/bin/bash
# Steam launch script for CM-SS13 Launcher (sharun format)

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

exec "$SCRIPT_DIR/AppRun" "$@"
