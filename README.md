# Harpy — Handheld Pixel Sorter

**Harpy** is a portable handheld device for algorithmic image fragmentation. Point it at anything, capture, and apply pixel sorting effects with a touch — turning photographs into glitchy, fragmented, structured noise.

It runs as a self-contained kiosk on a Raspberry Pi 5 with a 7" touchscreen. No laptop. No app. Just the device.

---

## What Is Pixel Sorting?

Pixel sorting is a glitch-art technique that reorders rows, columns, or diagonals of pixels by a property like brightness or hue. The result is a controlled kind of visual disintegration — images that feel both broken and deliberate.

Harpy makes this process immediate and physical: capture, sort, iterate, export.

---

## The Device

- **Raspberry Pi 5** with a **7" TFT touchscreen** (HDMI)
- Camera module for live preview and one-tap capture
- Optional UPS HAT for battery-powered portable use
- 3D-printable enclosure (files coming soon)
- Boots straight into the sorting interface — no desktop, no distractions

---

## How It Works

1. **Capture** — tap to take a photo from the live camera feed, or pick an image from a USB drive
2. **Sort** — choose horizontal, vertical, or diagonal sorting
3. **Tune** — drag the threshold slider to control where segments break; optionally tint with the hue slider
4. **Crop** — drag handles to frame the result
5. **Iterate** — Save & Iterate writes the sorted image and loads it as the new source; keep going as deep as you want
6. **Export** — plug in a USB drive; Harpy auto-detects it and copies the sessions you select
7. **Gallery** — browse saved sessions, view, re-edit or delete images

---

## Controls

| Control | What it does |
|---|---|
| Threshold slider | Controls sensitivity of pixel segment breaks — low = long streaks, high = short bursts |
| Hue slider | Adds a color tint to the display (non-destructive until saved) |
| Algorithm button | Cycles: Horizontal → Vertical → Diagonal |
| Sort Mode button | Brightness / Black: dark → light, White: light → dark |
| Save & Iterate | Saves current result and loads it as next input |
| New Image | Start over with a fresh capture (asks to save unsaved edits first) |
| USB (camera screen) | Browse a USB drive and pick an image to sort |
| Gallery (camera screen) | Saved sessions: view full screen, re-edit, delete |
| Export to USB | Copies the selected sessions to any mounted USB drive |

---

## Build Your Own

### Hardware

| Part | Notes |
|---|---|
| Raspberry Pi 5 (4GB or 8GB) | Main board |
| 7" HDMI TFT touchscreen | Any 1024×600 or 1920×1080 HDMI panel with USB touch |
| Raspberry Pi Camera Module 3 | Or compatible |
| UPS HAT (optional) | For battery-powered use |
| MicroSD card (32GB+) | OS + app |
| 3D-printed enclosure | Files coming soon |

### Software Setup

**Install dependencies on your Pi:**
```bash
sudo apt update && sudo apt upgrade -y
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
sudo apt install -y rpicam-apps libxkbcommon-dev libxcb-render0-dev \
    libxcb-shape0-dev libxcb-xfixes0-dev
```

**Clone, build, and set up auto-start:**
```bash
git clone https://github.com/Vaghabund/Pixelsort.git
cd Pixelsort
cargo build --release

cd deployment
chmod +x setup_autostart.sh
./setup_autostart.sh
```

**Enable auto-login for kiosk mode:**
```bash
sudo raspi-config
# System Options → Boot / Auto Login → Desktop Autologin
```

Reboot — Harpy starts automatically.

---

## Development (Desktop)

Harpy runs on Windows, macOS, and Linux for development. The camera is replaced with an animated test pattern.

```bash
cargo run
```

Set `HARPY_USB_DIR=/some/folder` to treat a folder as the USB drive (for the USB browser and export) on machines without a mounted stick.

Saved images (`sorted_images/`) and `ups_config.toml` live in the project checkout, which the app finds from its executable path (`<checkout>/target/<profile>/pixelsort-pi`), so it works from any working directory. Set `HARPY_APP_DIR` to use a different folder.

**Cross-compile for Pi from your dev machine:**
```bash
cargo install cross
cross build --release --target aarch64-unknown-linux-gnu
```

---

## Deployment Scripts

The `deployment/` folder contains three scripts:

- **setup_autostart.sh** — one-time installer; registers the systemd service
- **start_pixelsort.sh** — runtime launcher; waits for X11 then starts the app
- **update_and_rebuild.sh** — triggered by the in-app "Pull & Restart" button; pulls latest code and rebuilds in the background

**Manual service control:**
```bash
sudo systemctl start pixelsort-kiosk.service
sudo systemctl stop pixelsort-kiosk.service
journalctl -u pixelsort-kiosk.service -f
```

---

## Kiosk Mode

On the device, Harpy runs fullscreen with no window chrome. After 5 minutes idle it dims to a sleep screen — tap anywhere to wake. Exit by pressing ESC or tapping the top-left corner 5 times.

---

## Project Structure

```
src/
  hardware/       # Camera and UPS battery integration
  processing/     # Sorting algorithms, image ops, crop
  session/        # Save pipeline and USB export
  system/         # Update manager, exit handling
  ui/             # All UI: layouts, styles, components, screens
assets/
  Harpy_ICON.png
deployment/
  setup_autostart.sh
  start_pixelsort.sh
  update_and_rebuild.sh
  pixelsort-kiosk.service
```

The UI layer is designed for easy customization — colors, sizes, and positions are all grouped as constants at the top of their respective files (`styles.rs`, `layouts.rs`, `components.rs`).

---

## Troubleshooting

**Camera not detected:**
```bash
rpicam-hello
vcgencmd get_camera
sudo apt install -y rpicam-apps
```

**App won't start on boot:**
```bash
systemctl status pixelsort-kiosk.service
journalctl -u pixelsort-kiosk.service -n 50
```

**Display resolution wrong** — add to `/boot/config.txt`:
```
hdmi_force_hotplug=1
hdmi_group=2
hdmi_mode=87
hdmi_cvt=1920 1080 60 6 0 0 0
```

**USB export not detecting:** drives must mount under `/media/*` or `/mnt/*`; wait a few seconds after plugging in.

**Can't exit:** ESC key, or tap top-left corner 5 times within 3 seconds.

**Uninstall:**
```bash
sudo systemctl stop pixelsort-kiosk.service
sudo systemctl disable pixelsort-kiosk.service
sudo rm /etc/systemd/system/pixelsort-kiosk.service
sudo systemctl daemon-reload
rm -rf ~/Pixelsort
```

---

## Links

- [UPS Setup Guide](docs/UPS_SETUP.md)
- [Battery Display Documentation](docs/BATTERY_DISPLAY.md)

---

## License

MIT — see [LICENSE](LICENSE)
