#!/bin/bash
set -e

echo "Installing WorldVPN Daemon..."

# Build
cargo build -p worldvpn-daemon --release

# Copy binary
sudo cp target/release/worldvpn-daemon /usr/local/bin/

# Create config dir
sudo mkdir -p /etc/worldvpn

# Copy systemd unit
sudo cp crates/worldvpn-daemon/systemd/worldvpn.service /etc/systemd/system/

# Reload systemd
sudo systemctl daemon-reload

echo "Installation complete. You can start the service with: sudo systemctl start worldvpn"
