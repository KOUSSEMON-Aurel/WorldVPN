# 📦 WorldVPN Core Crates

<div align="center">

<br/>

```
 ██████╗██████╗  █████╗ ████████╗███████╗███████╗
██╔════╝██╔══██╗██╔══██╗╚══██╔══╝██╔════╝██╔════╝
██║     ██████╔╝███████║   ██║   █████╗  ███████╗
██║     ██╔══██╗██╔══██║   ██║   ██╔══╝  ╚════██║
╚██████╗██║  ██║██║  ██║   ██║   ███████╗███████║
 ╚═════╝╚═╝  ╚═╝╚═╝  ╚═╝   ╚═╝   ╚══════╝╚══════╝
```

**The Engine of Decentralized Privacy**

<br/>

![Rust](https://img.shields.io/badge/Language-Rust-orange?style=flat-square&logo=rust)
![libp2p](https://img.shields.io/badge/Network-libp2p-blue?style=flat-square)
![ChaCha20](https://img.shields.io/badge/Crypto-ChaCha20-10B981?style=flat-square)

</div>

## 📖 Overview
The `crates` directory contains the low-level logic that powers the WorldVPN ecosystem. From high-performance cryptography to the P2P discovery swarm, these crates are designed to be fast, memory-safe, and cross-platform.

## 🛠 Included Crates

### 🛠 `vpn-core`
The backbone of the system.
- **`crypto`**: XChaCha20-Poly1305 and Ed25519 identity management.
- **`p2p`**: Peer discovery via Kademlia DHT, Gossipsub, and mDNS.
- **`nat`**: Intelligent NAT traversal and type detection.
- **`api`**: Client implementation for communication with the Render backend.

### 🤖 `worldvpn-daemon`
The background service for P2P sharing.
- Manages the local node identity.
- Handles registration and heartbeats with the backend.
- Monitors peer reputation and latency for the selection engine.

## 🚀 Development
To build all core components:
```bash
cargo build --all
```

To run tests:
```bash
cargo test
```

---
**WorldVPN** · Private. Decentralized. Transparent.
