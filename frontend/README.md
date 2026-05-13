# 📱 WorldVPN Frontend Applications

<div align="center">

<br/>

```
███████╗██████╗  ██████╗ ███╗   ██╗████████╗███████╗███╗   ██╗██████╗ 
██╔════╝██╔══██╗██╔═══██╗████╗  ██║╚══██╔══╝██╔════╝████╗  ██║██╔══██╗
█████╗  ██████╔╝██║   ██║██╔██╗ ██║   ██║   █████╗  ██╔██╗ ██║██║  ██║
██╔══╝  ██╔══██╗██║   ██║██║╚██╗██║   ██║   ██╔══╝  ██║╚██╗██║██║  ██║
██║     ██║  ██║╚██████╔╝██║ ╚████║   ██║   ███████╗██║ ╚████║██████╔╝
╚═╝     ╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═══╝   ╚═╝   ╚══════╝╚═╝  ╚═══╝╚═════╝ 
```

**Premium Cross-Platform Privacy Interfaces**

<br/>

![Flutter](https://img.shields.io/badge/Toolkit-Flutter-02569B?style=flat-square&logo=flutter)
![Dart](https://img.shields.io/badge/Language-Dart-0175C2?style=flat-square&logo=dart)
![Lucide](https://img.shields.io/badge/Iconography-Lucide-FF8C00?style=flat-square)

</div>

## 📖 Overview
The `frontend` directory contains the user-facing applications of WorldVPN. Designed for speed and aesthetics, these apps provide a seamless bridge between the decentralized core engine and the end-user.

## 📱 Applications

### 🔹 `worldvpn-mobile`
The flagship mobile experience built with **Flutter**.
- **Wallet**: Real-time credit management and sharing history.
- **Node Selection**: Intelligent filtering (Auto, Country, Protocol).
- **Security**: Resident-grade encryption status monitoring.
- **Bridge**: Deep integration with the Rust `vpn-core` via `flutter_rust_bridge`.

### 💻 `worldvpn-desktop` (Coming Soon)
Native desktop experience for Windows, macOS, and Linux, leveraging the same core Rust logic for maximum performance.

## 🚀 Getting Started (Mobile)
1. Ensure you have the [Flutter SDK](https://docs.flutter.dev/get-started/install) installed.
2. Install dependencies:
   ```bash
   flutter pub get
   ```
3. Generate Rust bridges:
   ```bash
   flutter_rust_bridge_codegen generate
   ```
4. Run the app:
   ```bash
   flutter run
   ```

---
**WorldVPN** · Private. Decentralized. Transparent.
