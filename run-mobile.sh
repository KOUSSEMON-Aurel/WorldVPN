#!/usr/bin/env bash

# WorldVPN Mobile Launcher
# Builds the Flutter app in debug mode and installs it on the connected device via adb.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MOBILE_DIR="$SCRIPT_DIR/frontend/worldvpn-mobile"

# Set Android SDK & NDK paths for Rust compilation
export ANDROID_HOME="/home/aurel/Android/Sdk"
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/30.0.14904198"
# Ensure Rustup and Android tools are in PATH
export PATH="/home/aurel/.cargo/bin:$ANDROID_HOME/platform-tools:$PATH"

echo "📱 WorldVPN Mobile Deployer"
echo "📁 Directory: $MOBILE_DIR"
echo ""

cd "$MOBILE_DIR"

# 1. Detect device architecture
echo "🔍 Detecting device architecture..."
DEVICE_ABI=$(adb shell getprop ro.product.cpu.abi 2>/dev/null || echo "unknown")
echo "📱 Device ABI: $DEVICE_ABI"

RUST_TARGET=""
JNILIBS_DIR="android/app/src/main/jniLibs"

case $DEVICE_ABI in
  "arm64-v8a")
    RUST_TARGET="aarch64-linux-android"
    ABI_DIR="arm64-v8a"
    ;;
  "x86_64")
    RUST_TARGET="x86_64-linux-android"
    ABI_DIR="x86_64"
    ;;
  "unknown")
    echo "⚠️  No device detected via ADB. Building for arm64 by default..."
    RUST_TARGET="aarch64-linux-android"
    ABI_DIR="arm64-v8a"
    ;;
  *)
    echo "⚠️  Unsupported or unknown architecture: $DEVICE_ABI. Trying arm64..."
    RUST_TARGET="aarch64-linux-android"
    ABI_DIR="arm64-v8a"
    ;;
esac

# 2. Build Rust Library for the target
echo "🦀 Building Rust core for $RUST_TARGET..."
mkdir -p "$JNILIBS_DIR/$ABI_DIR"
cd "$SCRIPT_DIR/crates/vpn-core"
cargo ndk -t "$RUST_TARGET" build --release

# Copy the built library to jniLibs
cp "../../target/$RUST_TARGET/release/libvpn_core.so" "$MOBILE_DIR/$JNILIBS_DIR/$ABI_DIR/"

# 3. Build and Install Flutter App
cd "$MOBILE_DIR"
echo "🛠️  Building Flutter APK (Debug)..."
flutter build apk --debug

echo "📲 Installing APK via ADB..."
adb install -r build/app/outputs/flutter-apk/app-debug.apk

echo "✅ App installed successfully on the device!"
