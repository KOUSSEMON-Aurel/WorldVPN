# 🛡 WorldVPN Tunnel Core (Go)

<div align="center">

<br/>

```
██╗   ██╗██████╗ ███╗   ██╗   ██████╗  ██████╗ 
██║   ██║██╔══██╗████╗  ██║  ██╔════╝ ██╔═══██╗
██║   ██║██████╔╝██╔██╗ ██║  ██║  ███╗██║   ██║
╚██╗ ██╔╝██╔═══╝ ██║╚██╗██║  ██║   ██║██║   ██║
 ╚████╔╝ ██║     ██║ ╚████║  ╚██████╔╝╚██████╔╝
  ╚═══╝  ╚═╝     ╚═╝  ╚═══╝   ╚═════╝  ╚═════╝ 
```

**High-Performance Traffic Routing Engine**

<br/>

![Go](https://img.shields.io/badge/Language-Go-00ADD8?style=flat-square&logo=go)
![WireGuard](https://img.shields.io/badge/Protocol-WireGuard-881717?style=flat-square)
![Shadowsocks](https://img.shields.io/badge/Protocol-Shadowsocks-white?style=flat-square)

</div>

## 📖 Overview
`vpn-go` is the high-performance core responsible for the actual low-level traffic encapsulation and routing. By leveraging Go's efficient concurrency model and native networking stack, it ensures a lightweight but heavy-duty tunnel for all WorldVPN users.

## 🛠 Features
- **Multiprotocol Support**: WireGuard, Hysteria2, Shadowsocks, and Trojan.
- **Cross-Platform**: Compiles to native libraries for Linux, Android, and Windows.
- **Low Latency**: Zero-copy packet processing where possible.
- **Integration**: Works as a sidecar or embedded library for the Rust `vpn-core`.

## 📂 Structure
- `/protocol`: Pure Go implementations of transport protocols.
- `/tunnel`: Virtual interface (TUN/TAP) management.
- `/bridge`: C-bindings for Rust and Flutter integration.

## 🚀 Building
To build as a standalone binary:
```bash
go build -o vpn-core ./main.go
```

To build as a shared library (for Android/C):
```bash
go build -buildmode=c-shared -o libvpn.so ./bridge
```

---
**WorldVPN** · Private. Decentralized. Transparent.
