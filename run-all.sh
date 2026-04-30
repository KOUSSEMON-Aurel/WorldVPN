#!/usr/bin/env bash

# WorldVPN - Full Stack Launcher
# Starts: Backend server + Desktop GUI (Tauri)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BACKEND_DIR="$SCRIPT_DIR/backend/server"
GUI_DIR="$SCRIPT_DIR/frontend/worldvpn-gui"

cleanup() {
  echo ""
  echo "🛑 Stopping all services..."
  kill "$BACKEND_PID" 2>/dev/null || true
  wait "$BACKEND_PID" 2>/dev/null || true
  echo "✅ Done."
}

trap cleanup EXIT INT TERM

echo "🌍 WorldVPN - Full Stack"
echo "========================"
echo ""

# 1. Start backend
echo "🔧 Starting backend server..."
if [ -f "$BACKEND_DIR/.env" ]; then
  export $(grep -v '^#' "$BACKEND_DIR/.env" | xargs) 2>/dev/null
fi
cd "$BACKEND_DIR"
cargo run --bin worldvpn-server &
BACKEND_PID=$!
echo "   ✔ Backend started (PID: $BACKEND_PID)"

# Wait a moment for the backend to boot
echo "   ⏳ Waiting for backend to be ready..."
sleep 4

# 2. Install frontend deps if needed
if [ ! -d "$GUI_DIR/node_modules" ]; then
  echo "📦 Installing frontend dependencies..."
  cd "$GUI_DIR" && npm install
fi

# 3. Start Tauri desktop app
echo ""
echo "🚀 Launching Tauri desktop app..."
cd "$GUI_DIR" && npm run tauri dev
