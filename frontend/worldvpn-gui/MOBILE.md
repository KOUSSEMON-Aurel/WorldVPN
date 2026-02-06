# WorldVPN Mobile - Android Setup & Development

This project uses **Tauri v2** to target Android using the same React + Rust codebase.

## Prerequisites

- **Android SDK & NDK**: Currently being installed via `tauri android init`.
- **Rust Android Targets**: Run these commands to add support for mobile architectures:

  ```bash
  rustup target add aarch64-linux-android
  rustup target add armv7-linux-androideabi
  rustup target add x86_64-linux-android
  rustup target add i686-linux-android
  ```

## Development

To run the app on a connected Android phone or emulator:

```bash
npx tauri android dev
```

## How it Works

1. **Frontend**: The React (Vite) app is bundled into the Android assets.
2. **Backend**: The Rust code (`src-tauri` + `vpn-core`) is compiled into a shared library (`.so`) using JNI.
3. **Bridge**: Tauri handles the communication between the WebView (UI) and the Rust logic.

## Specific Mobile Tasks

- [ ] **Permissions**: Add `INTERNET`, `FOREGROUND_SERVICE`, and `VPN_SERVICE` to `AndroidManifest.xml`.
- [ ] **VpnService Integration**: Implement the Android `VpnService` class in Kotlin to allow the OS to route traffic through our Rust tunnel.
- [ ] **UI Scaling**: Ensure the glassmorphism design looks great on vertical screens.
