#!/bin/bash
# Harpy Pixel Sorter - Unified Launcher
# Waits for X11, then starts the application

# Project root = parent of this script's deployment/ folder
APP_DIR="$(cd "$(dirname "$0")/.." && pwd)"
BINARY="$APP_DIR/target/release/pixelsort-pi"
MAX_X11_WAIT=30     # Maximum seconds to wait for X11
CHECKS_PER_SECOND=5 # Check every 200ms

# ============================================
# Step 1: Wait for X11 to be ready
# ============================================
echo "=========================================="
echo "Harpy Pixel Sorter - Starting..."
echo "=========================================="
echo "Waiting for X11 display server..."

x11_ready=false
for ((i = 0; i < MAX_X11_WAIT * CHECKS_PER_SECOND; i++)); do
    if DISPLAY=:0 xset q &>/dev/null; then
        echo "✓ X11 ready (after $((i / CHECKS_PER_SECOND))s)"
        x11_ready=true
        break
    fi
    sleep 0.2
done

if [ "$x11_ready" != true ]; then
    echo "⚠ X11 timeout after ${MAX_X11_WAIT}s, starting anyway..."
fi

# ============================================
# Step 2: Run application
# ============================================
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

echo "Starting Harpy Pixel Sorter..."
echo "=========================================="

# exec: the app replaces this shell so systemd signals (stop/restart) reach it directly
exec "$BINARY"
