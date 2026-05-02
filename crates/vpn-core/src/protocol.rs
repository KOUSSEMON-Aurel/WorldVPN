use serde::{Deserialize, Serialize};

/// Supported VPN and anti-censorship protocols
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VpnProtocol {
    /// High-performance modern VPN (UDP)
    WireGuard,
    /// WireGuard with additional obfuscation layer
    WireGuardObfuscated,
    /// Traditional proxy protocol used to bypass filters
    Shadowsocks,
    /// High-performance QUIC-based protocol for unstable links
    Hysteria2,
    /// Modern anti-censorship protocol (TLS-mimicry)
    Trojan,
    /// Advanced stealth protocol for high-censorship areas
    VLESS,
}
impl VpnProtocol {
    /// Returns the name of the protocol
    pub fn name(&self) -> &'static str {
        match self {
            VpnProtocol::WireGuard => "WireGuard",
            VpnProtocol::WireGuardObfuscated => "WireGuard Obfuscated",
            VpnProtocol::Shadowsocks => "Shadowsocks",
            VpnProtocol::Hysteria2 => "Hysteria2",
            VpnProtocol::Trojan => "Trojan",
            VpnProtocol::VLESS => "VLESS",
        }
    }

    /// Returns the default server port for a protocol
    pub fn default_port(&self) -> u16 {
        match self {
            VpnProtocol::WireGuard | VpnProtocol::WireGuardObfuscated => 51820,
            VpnProtocol::Shadowsocks => 8388,
            VpnProtocol::Trojan | VpnProtocol::VLESS => 443,
            VpnProtocol::Hysteria2 => 32400,
        }
    }

    /// Determines if the protocol is designed specifically for stealth
    pub fn is_anti_censorship(&self) -> bool {
        matches!(
            self,
            VpnProtocol::Shadowsocks
                | VpnProtocol::WireGuardObfuscated
                | VpnProtocol::Hysteria2
                | VpnProtocol::Trojan
                | VpnProtocol::VLESS
        )
    }

    /// Heuristic score for protocol performance (1.0 = best)
    pub fn performance_score(&self) -> f64 {
        match self {
            VpnProtocol::WireGuard => 1.0,
            VpnProtocol::Hysteria2 => 0.95,
            VpnProtocol::Shadowsocks => 0.85,
            VpnProtocol::WireGuardObfuscated => 0.75,
            VpnProtocol::Trojan | VpnProtocol::VLESS => 0.7,
        }
    }

    /// Heuristic score for protocol stealth/evasion (1.0 = most stealthy)
    pub fn stealth_score(&self) -> f64 {
        match self {
            VpnProtocol::VLESS => 1.0,
            VpnProtocol::Trojan => 0.95,
            VpnProtocol::Hysteria2 => 0.9,
            VpnProtocol::Shadowsocks => 0.85,
            VpnProtocol::WireGuardObfuscated => 0.8,
            VpnProtocol::WireGuard => 0.3,
        }
    }
}

impl std::fmt::Display for VpnProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
