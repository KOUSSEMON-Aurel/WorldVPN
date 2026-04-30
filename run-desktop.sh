#!/usr/bin/env bash

# WorldVPN Desktop GUI Launcher
# Starts the Tauri/React desktop application in development mode.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GUI_DIR="$SCRIPT_DIR/frontend/worldvpn-gui"

echo "🌍 WorldVPN Desktop GUI"
echo "📁 Directory: $GUI_DIR"
echo ""

if [ ! -d "$GUI_DIR/node_modules" ]; then
  echo "📦 Installing dependencies..."
  cd "$GUI_DIR" && npm install
fi

echo "🚀 Launching Tauri desktop app..."
cd "$GUI_DIR" && npm run tauri dev
