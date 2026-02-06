//! WorldVPN Core Library
//!
//! Provides unified abstractions and implementations for multiple VPN protocols
//! including WireGuard, Shadowsocks, OpenVPN, and more.

pub mod abuse;
pub mod binary_manager;
pub mod client;
pub mod credits;
pub mod crypto;
pub mod error;
pub mod hysteria;
pub mod ikev2;
pub mod mock;
pub mod nat;
pub mod obfuscation;
pub mod openvpn;
pub mod p2p;
pub mod protocol;
pub mod selector;
pub mod shadowsocks;
pub mod tunnel;
pub mod v2ray;
pub mod wireguard;
