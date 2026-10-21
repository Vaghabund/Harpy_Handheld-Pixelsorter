#!/bin/bash
# Harpy Pixel Sorter - Unified Launcher
# Waits for X11, then starts the application

# Project root = parent of this script's deployment/ folder
APP_DIR="$(cd "$(dirname "$0")/.." && pwd)"
BINARY="$APP_DIR/target/release/pixelsort-pi"
MAX_X11_WAIT=30  # Maximum seconds to wait for X11
RETRY_INTERVAL=0.2  # Check every 200ms

# ============================================
# Step 1: Wait for X11 to be ready
# ============================================
echo "=========================================="
echo "Harpy Pixel Sorter - Starting..."
echo "=========================================="
echo "Waiting for X11 display server..."

elapsed=0
while [ $(echo "$elapsed < $MAX_X11_WAIT" | bc) -eq 1 ]; do
    if DISPLAY=:0 xset q &>/dev/null; then
        echo "✓ X11 ready (${elapsed}s)"
        break
    fi
    sleep $RETRY_INTERVAL
    elapsed=$(echo "$elapsed + $RETRY_INTERVAL" | bc)
done

if [ $(echo "$elapsed >= $MAX_X11_WAIT" | bc) -eq 1 ]; then
    echo "⚠ X11 timeout after ${MAX_X11_WAIT}s, starting anyway..."
fi

# ============================================
# Step 2: Prepare and run application
# ============================================
cd "$APP_DIR" || {
    echo "ERROR: Could not change to $APP_DIR"
    exit 1
}

# Ensure git ignores file permission changes
git config core.fileMode false 2>/dev/null

# Check if binary exists
if [ ! -f "$BINARY" ]; then
    echo "ERROR: Binary not found at $BINARY"
    echo "Please build with: cargo build --release"
    exit 1
fi

echo "Starting Harpy Pixel Sorter..."
echo "Updates will be checked in background"
echo "=========================================="

# Run the application
"$BINARY"

# Cleanup after exit
echo ""
echo "Application closed."
sleep 2
