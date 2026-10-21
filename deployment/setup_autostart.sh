#!/bin/bash
# One-time setup script for Pixelsort auto-start on boot (Raspberry Pi OS Lite)
# Installs cage and the kiosk service for the current user and checkout location

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
APP_DIR="$(dirname "$SCRIPT_DIR")"
APP_USER="$(id -un)"

echo "Installing Pixelsort auto-start service..."
echo "  App directory: $APP_DIR"
echo "  User:          $APP_USER"

# cage: minimal Wayland compositor that shows one fullscreen app (no desktop needed)
sudo apt-get install -y cage

# GPU, display and touch access for the kiosk (default on Pi OS for the first user)
sudo usermod -aG video,render,input "$APP_USER"

# Make scripts executable
chmod +x "$SCRIPT_DIR/start_pixelsort.sh" "$SCRIPT_DIR/update_and_rebuild.sh"

# Ignore file permission changes (e.g. the chmod above) in git
git -C "$APP_DIR" config core.fileMode false

# Install systemd service, adapted to this user and checkout location
sed -e "s|/home/pixelsort/Pixelsort|$APP_DIR|g" \
    -e "s|^User=pixelsort|User=$APP_USER|" \
    "$SCRIPT_DIR/pixelsort-kiosk.service" | sudo tee /etc/systemd/system/pixelsort-kiosk.service >/dev/null
sudo systemctl daemon-reload

# Auto-mount USB sticks under /media/$APP_USER/ (Pi OS Lite has no desktop automounter)
sed -e "s|PIXELSORT_USER|$APP_USER|g" \
    -e "s|PIXELSORT_UID|$(id -u)|g" \
    -e "s|PIXELSORT_GID|$(id -g)|g" \
    "$SCRIPT_DIR/99-harpy-usb-automount.rules" | sudo tee /etc/udev/rules.d/99-harpy-usb-automount.rules >/dev/null
sudo udevadm control --reload-rules

# The kiosk takes over tty1; console logins stay available on tty2 (Ctrl+Alt+F2)
sudo systemctl disable getty@tty1.service
sudo systemctl enable pixelsort-kiosk.service
# Boot to the console (the kiosk replaces any desktop)
sudo systemctl set-default multi-user.target

echo "✓ Setup complete! Service will start on boot."
echo ""
echo "Useful commands:"
echo "  Start now:   sudo systemctl start pixelsort-kiosk.service"
echo "  Stop:        sudo systemctl stop pixelsort-kiosk.service"
echo "  Status:      sudo systemctl status pixelsort-kiosk.service"
echo "  View logs:   journalctl -u pixelsort-kiosk.service -f"
echo ""
echo "Reboot to start the kiosk: sudo reboot"
