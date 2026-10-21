#!/bin/bash
# Harpy Pixel Sorter - launcher, run inside the cage Wayland kiosk compositor

# Project root = parent of this script's deployment/ folder
APP_DIR="$(cd "$(dirname "$0")/.." && pwd)"
BINARY="$APP_DIR/target/release/pixelsort-pi"

echo "=========================================="
echo "Harpy Pixel Sorter - Starting..."
echo "=========================================="

cd "$APP_DIR" || {
    echo "ERROR: Could not change to $APP_DIR"
    exit 1
}

# Check if binary exists
if [ ! -f "$BINARY" ]; then
    echo "ERROR: Binary not found at $BINARY"
    echo "Please build with: cargo build --release"
    exit 1
fi

# Use native Wayland; don't fall back to cage's Xwayland
unset DISPLAY

# exec: the app replaces this shell so systemd signals (stop/restart) reach it directly
exec "$BINARY"
