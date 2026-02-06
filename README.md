# WorldVPN

Decentralized, transparent, and ethical P2P VPN.

## 🚨 Initial Setup (IMPORTANT)

**Before running the project**, generate certificates and secrets:

```bash
./scripts/generate-dev-certs.sh
```

This script automatically creates:

- Self-signed TLS certificates (development)
- Random JWT secret
- `.env` file with configuration

**⚠️ Security**: `*.pem` and `.env` files are **automatically ignored by git**. NEVER commit them!

See [`backend/server/SECURITY.md`](backend/server/SECURITY.md) for more details.

---

## Architecture

This project is organized as a Cargo workspace:

- `crates/vpn-core`: Central library (6 VPN protocols)
- `backend/server`: REST API (Axum + PostgreSQL)
- `frontend/cli`: CLI Client
- `crates/vpn-ffi`: FFI Bindings for mobile/desktop
- `crates/vpn-wasm`: WebAssembly compilation for browser extension

## Supported Protocols

- ✅ **WireGuard**: High performance (UDP)
- ✅ **Shadowsocks**: Anti-censorship (China, Iran)
- ✅ **OpenVPN**: Maximum compatibility (TCP/443)
- ✅ **IKEv2**: Mobile, roaming
- ✅ **Hysteria2**: Unstable networks (QUIC)
- ✅ **V2Ray/Trojan**: Maximum stealth

## Compilation

```bash
# Build the entire workspace
cargo build

# Build in release mode
cargo build --release

# Run tests
cargo test
```

## Running

```bash
# API Server
cargo run -p vpn-server

# CLI Client
cargo run -p worldvpn-cli -- --help
```

## Documentation

```bash
# Generate and open documentation
cargo doc --open
```

## License

MIT
