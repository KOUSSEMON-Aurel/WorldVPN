# WorldVPN Desktop - Roadmap & Pending Tasks

This document tracks the remaining technical implementations needed to move from the current UI prototype to a fully functional VPN client.

## 1. Core VPN Engine Integration (`vpn-core`)

- [ ] **Native Connection**: Replace the simulated connection (`tokio::time::sleep`) in `src-tauri/src/lib.rs` with actual calls to `vpn_core`.
- [ ] **WireGuard Implementation**: Configure TUN/TAP interfaces natively on Windows/Linux.
- [ ] **Key Management**: Implement secure generation and storage (Keyring/Encrypted storage) of User Private Keys.
- [ ] **Protocols**: Support switching between WireGuard, Shadowsocks, and Obfuscated protocols.

## 2. Backend Synchronization (Central Server)

- [ ] **Real Authentication**: Replace the splash screen simulation with a real Login/Register form communicating with the Axum backend.
- [ ] **JWT Management**: Securely store the JWT and use it for all authenticated API calls.
- [ ] **Dynamic Node Discovery**: Fetch the live list of P2P nodes from the central server to populate the "Map/Peers" view.
- [ ] **Real Wallet Balance**: Sync the credit balance with the PostgreSQL database in real-time.

## 3. Transparency & P2P Sharing

- [ ] **Traffic Monitoring**: Capture real-time bandwidth usage (Upload/Download) from the local network interface.
- [ ] **Live Peer Dashboard**: Fetch information about who is using your bandwidth (anonymized) through the central server's transparency API.
- [ ] **Earning Logic**: Ensure credits are correctly calculated and sent to the backend based on actual traffic shared.

## 4. System Security

- [ ] **Kill Switch**: Implement firewall rules (iptables/nftables/Windows Filtering Platform) to block non-VPN traffic when enabled.
- [ ] **DNS Leak Protection**: Force DNS queries through encrypted tunnels (DoH/DoT).
- [ ] **Auto-Updates**: Configure the Tauri updater for seamless security patches.

## 5. UI/UX Refinement

- [ ] **Settings Persistence**: Save user preferences (Theme, Protocol, Auto-connect) to local storage or a config file.
- [ ] **System Tray**: Add a minimize-to-tray feature with quick connect/disconnect options.
- [ ] **Notifications**: Native system notifications for connection status changes.
