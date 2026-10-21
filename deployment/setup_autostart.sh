#!/bin/bash
# One-time setup script for Pixelsort auto-start on boot
# Installs the systemd service for the current user and checkout location

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
APP_DIR="$(dirname "$SCRIPT_DIR")"
APP_USER="$(id -un)"
APP_HOME="$HOME"

echo "Installing Pixelsort auto-start service..."
echo "  App directory: $APP_DIR"
echo "  User:          $APP_USER"

# Make scripts executable
chmod +x "$SCRIPT_DIR/start_pixelsort.sh" "$SCRIPT_DIR/update_and_rebuild.sh"

# Ignore file permission changes (e.g. the chmod above) in git
git -C "$APP_DIR" config core.fileMode false

# Install systemd service, adapted to this user and checkout location
sed -e "s|/home/pixelsort/Pixelsort|$APP_DIR|g" \
    -e "s|^User=pixelsort|User=$APP_USER|" \
    -e "s|/home/pixelsort/.Xauthority|$APP_HOME/.Xauthority|" \
    "$SCRIPT_DIR/pixelsort-kiosk.service" | sudo tee /etc/systemd/system/pixelsort-kiosk.service >/dev/null
sudo systemctl daemon-reload
sudo systemctl enable pixelsort-kiosk.service

echo "✓ Setup complete! Service will start on boot."
echo ""
echo "Useful commands:"
echo "  Start now:   sudo systemctl start pixelsort-kiosk.service"
echo "  Stop:        sudo systemctl stop pixelsort-kiosk.service"
echo "  Status:      sudo systemctl status pixelsort-kiosk.service"
echo "  View logs:   journalctl -u pixelsort-kiosk.service -f"
echo ""
echo "Next steps: See README.md for auto-login setup (raspi-config)"
