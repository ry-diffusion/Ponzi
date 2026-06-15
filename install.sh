#!/bin/bash
set -e

echo "=== Installing Ponzi ==="

cargo build --release -p ponzi-driver -p ponzi-ui

sudo install -Dm755 target/release/ponzi-driver /usr/bin/ponzi-driver
sudo install -Dm755 target/release/ponzi-ui /usr/bin/ponzi-ui
echo "Binaries installed to /usr/bin/"

sudo mkdir -p /etc/ponzi
sudo cp -n ponzi-driver/config.toml /etc/ponzi/config.toml 2>/dev/null || true
echo "Config at /etc/ponzi/config.toml"

sudo cp 99-pingit-tablet.rules /etc/udev/rules.d/99-ponzi-tablet.rules
sudo udevadm control --reload-rules
sudo udevadm trigger
echo "udev rule installed"

sudo cp ponzi-driver.service /usr/lib/systemd/system/ponzi-driver.service
sudo systemctl daemon-reload
sudo systemctl enable ponzi-driver
echo "Systemd service enabled"

sudo cp ponzi.desktop /usr/share/applications/ponzi.desktop
echo "Desktop entry installed"

echo ""
echo "=== Done! ==="
echo "  Start driver:  sudo systemctl start ponzi-driver"
echo "  Open UI:       ponzi-ui"
echo "  View logs:     journalctl -u ponzi-driver -f"
echo "  Edit config:   sudo nano /etc/ponzi/config.toml"
