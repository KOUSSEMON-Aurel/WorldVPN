# WorldVPN Mobile - Roadmap & Pending Tasks

This document tracks the mobile-specific implementations for Android and iOS (Tauri v2).

## 1. Android Specifics

- [ ] **VpnService Integration**: Implement the Android `VpnService` in Kotlin to handle TUN interface creation.
- [ ] **Rust JNI Bridge**: Setup the JNI bridge to pass the TUN file descriptor (fd) from Kotlin to the Rust `vpn-core`.
- [ ] **Foreground Service**: Ensure the VPN runs in a foreground service with a persistent notification (Android requirement).
- [ ] **App Links**: Implement deep linking for authentication or node sharing.
- [ ] **Biometric Lock**: Integrate `tauri-plugin-biometric` for app entry security.

## 2. Core VPN Engine (Mobile)

- [ ] **Cross-Compilation**: Setup Cargo config for Android targets (`aarch64`, `armv7`, `x86_64`).
- [ ] **Dynamic Permissions**: Handle runtime permission requests for VPN and Notifications in Android 13+.
- [ ] **Battery Optimization**: Handle "Ignore Battery Optimizations" to prevent OS from killing the VPN process.

## 3. UI/UX Mobile Optimization

- [ ] **Responsive Design**: Verify the glassmorphism layout on various screen sizes and notches.
- [ ] **Mobile Wallet**: Integrate with mobile payment systems (optional).
- [ ] **Dark/Light Mode**: Full system theme support.

## 4. Security & Privacy

- [ ] **Certificate Pinning**: Ensure secure connection to the central server.
- [ ] **Secure Storage**: Use Android EncryptedSharedPreferences for JWT and private keys.
- [ ] **Traffic obfuscation**: Test mobile network (LTE/5G) compatibility with obfuscated protocols.
