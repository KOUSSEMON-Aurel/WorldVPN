use crate::protocol::VpnProtocol;
use serde::{Deserialize, Serialize};

/// Comprehensive data used to determine the optimal VPN protocol
#[derive(Debug, Clone)]
pub struct SelectionContext {
    pub network_quality: NetworkQuality,
    pub firewall_profile: FirewallProfile,
    pub user_country: String,
    pub device_type: DeviceType,
    pub battery_level: Option<f32>, // 0.0 to 1.0
    pub use_case: UseCase,
}

#[derive(Debug, Clone)]
pub struct NetworkQuality {
    pub latency_ms: u32,
    pub packet_loss: f64,
    pub bandwidth_mbps: f64,
    pub stability: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirewallProfile {
    Open,
    Residential,
    Corporate,
    NationalCensorship, // Strict DPI environments
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    Desktop,
    Mobile,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UseCase {
    Browsing,
    Streaming,
    Gaming, // Low latency priority
    Torrenting,
    Privacy,
    AntiCensorship,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CensorshipLevel {
    None,
    Low,
    Medium,
    High,
    Extreme,
}

/// Intelligent logic to choose the best protocol based on dynamic network conditions
pub struct ProtocolSelector {
    censored_countries: Vec<String>,
}

impl ProtocolSelector {
    pub fn new() -> Self {
        Self {
            censored_countries: vec![
                "CN".to_string(), // China
                "IR".to_string(), // Iran
                "RU".to_string(), // Russia
                "BY".to_string(), // Belarus
                "TM".to_string(), // Turkmenistan
                "KP".to_string(), // North Korea
            ],
        }
    }

    /// Primary entry point for selecting a protocol for a new connection
    pub fn select_best_protocol(&self, context: &SelectionContext) -> VpnProtocol {
        tracing::info!("Selecting protocol for context: {:?}", context);

        // 1. High-priority censorship check
        if self.is_censored_country(&context.user_country) {
            let level = self.censorship_level(&context.user_country);
            return self.select_anti_censorship_protocol(level);
        }

        // 2. Battery-saving check for mobile
        if context.device_type == DeviceType::Mobile {
            if let Some(battery) = context.battery_level {
                if battery < 0.20 {
                    tracing::info!("Low battery detected, using IKEv2 (battery efficient)");
                    return VpnProtocol::IKEv2;
                }
            }
        }

        // 3. Lossy networks (packet loss > 5%)
        if context.network_quality.packet_loss > 0.05 {
            tracing::info!("Unstable network detected, using Hysteria2 (QUIC-based resilience)");
            return VpnProtocol::Hysteria2;
        }

        // 4. Restricted environments
        if context.firewall_profile == FirewallProfile::Corporate {
            tracing::info!("Corporate firewall detected, using OpenVPN TCP/443 (Firewall bypass)");
            return VpnProtocol::OpenVpnTcp;
        }

        // 5. Explicit user intent
        match context.use_case {
            UseCase::Gaming => {
                tracing::info!("Gaming mode, using WireGuard (Lowest latency)");
                return VpnProtocol::WireGuard;
            }
            UseCase::Privacy => {
                tracing::info!("Privacy mode, using WireGuard Obfuscated");
                return VpnProtocol::WireGuardObfuscated;
            }
            _ => {}
        }

        // Default: High-performance WireGuard
        tracing::info!("Standard configuration, defaulting to WireGuard");
        VpnProtocol::WireGuard
    }

    fn select_anti_censorship_protocol(&self, level: CensorshipLevel) -> VpnProtocol {
        match level {
            CensorshipLevel::None | CensorshipLevel::Low => {
                tracing::info!("Low censorship, using Shadowsocks");
                VpnProtocol::Shadowsocks
            }
            CensorshipLevel::Medium => {
                tracing::info!("Medium censorship, using WireGuard Obfuscated");
                VpnProtocol::WireGuardObfuscated
            }
            CensorshipLevel::High => {
                tracing::info!("High censorship, using Trojan");
                VpnProtocol::Trojan
            }
            CensorshipLevel::Extreme => {
                tracing::info!("Extreme censorship, using VLESS (ShadowTLS-like stealth)");
                VpnProtocol::VLESS
            }
        }
    }

    /// Computes and ranks all protocols by a combined score
    pub fn rank_all_protocols(&self, context: &SelectionContext) -> Vec<(VpnProtocol, f64)> {
        let protocols = vec![
            VpnProtocol::WireGuard,
            VpnProtocol::WireGuardObfuscated,
            VpnProtocol::Shadowsocks,
            VpnProtocol::OpenVpnTcp,
            VpnProtocol::OpenVpnUdp,
            VpnProtocol::IKEv2,
            VpnProtocol::Hysteria2,
            VpnProtocol::Trojan,
            VpnProtocol::VLESS,
        ];

        let mut ranked: Vec<_> = protocols
            .into_iter()
            .map(|p| (p, self.score_protocol_advanced(p, context)))
            .collect();

        // Sort descending by score
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        ranked
    }

    /// Internal scoring engine using weighted criteria
    fn score_protocol_advanced(&self, protocol: VpnProtocol, context: &SelectionContext) -> f64 {
        let mut score = 0.0;

        let (speed_w, security_w, stealth_w, battery_w, stability_w) = 
            self.calculate_weights(context);

        // Core performance score from protocol definition
        score += protocol.performance_score() * speed_w;

        // Security baseline
        let security_score = match protocol {
            VpnProtocol::WireGuard | VpnProtocol::IKEv2 => 1.0,
            VpnProtocol::Shadowsocks | VpnProtocol::Hysteria2 => 0.95,
            VpnProtocol::Trojan | VpnProtocol::VLESS => 0.98,
            _ => 0.9,
        };
        score += security_score * security_w;

        // Stealth (Anti-DPI)
        score += protocol.stealth_score() * stealth_w;

        // Energy efficiency
        let battery_score = match protocol {
            VpnProtocol::IKEv2 => 1.0,
            VpnProtocol::WireGuard => 0.95,
            VpnProtocol::Shadowsocks => 0.90,
            VpnProtocol::Hysteria2 => 0.85,
            _ => 0.80,
        };
        score += battery_score * battery_w;

        // Reliability / Stability
        let stability_score = match protocol {
            VpnProtocol::Hysteria2 => 1.0,
            VpnProtocol::IKEv2 => 0.95,
            VpnProtocol::OpenVpnUdp => 0.85,
            VpnProtocol::WireGuard => 0.90,
            _ => 0.80,
        };
        score += stability_score * stability_w;

        // Contextual penalties/bonuses
        score = self.apply_penalties(protocol, context, score);

        score
    }

    /// Dynamically shifts weights based on environment (e.g., prioritize stealth in China)
    fn calculate_weights(&self, context: &SelectionContext) -> (f64, f64, f64, f64, f64) {
        let mut speed_w = 0.30;
        let mut security_w = 0.20;
        let mut stealth_w = 0.20;
        let mut battery_w = 0.15;
        let mut stability_w = 0.15;

        if self.is_censored_country(&context.user_country) {
            stealth_w = 0.40;
            speed_w = 0.20;
        }

        if context.device_type == DeviceType::Mobile {
            if let Some(battery) = context.battery_level {
                if battery < 0.30 {
                    battery_w = 0.35;
                    speed_w = 0.20;
                }
            }
        }

        if context.use_case == UseCase::Gaming {
            speed_w = 0.50;
            stealth_w = 0.10;
        }

        if context.network_quality.packet_loss > 0.05 {
            stability_w = 0.40;
            speed_w = 0.20;
        }

        let total = speed_w + security_w + stealth_w + battery_w + stability_w;
        (speed_w/total, security_w/total, stealth_w/total, battery_w/total, stability_w/total)
    }

    fn apply_penalties(&self, protocol: VpnProtocol, context: &SelectionContext, base_score: f64) -> f64 {
        let mut score = base_score;

        // Penalize detectable protocols in censored countries
        if self.is_censored_country(&context.user_country) {
            if matches!(protocol, VpnProtocol::WireGuard | VpnProtocol::OpenVpnUdp) {
                score *= 0.50; // Heavy penalty
            }
        }

        // Case-specific bonuses
        if context.use_case == UseCase::Gaming && protocol == VpnProtocol::WireGuard {
            score *= 1.10;
        }
        if context.use_case == UseCase::AntiCensorship && matches!(protocol, VpnProtocol::Trojan | VpnProtocol::VLESS) {
            score *= 1.15;
        }

        score
    }

    fn is_censored_country(&self, country_code: &str) -> bool {
        self.censored_countries.contains(&country_code.to_uppercase())
    }

    fn censorship_level(&self, country_code: &str) -> CensorshipLevel {
        match country_code.to_uppercase().as_str() {
            "CN" | "KP" => CensorshipLevel::Extreme,
            "IR" | "TM" => CensorshipLevel::High,
            "RU" | "BY" => CensorshipLevel::Medium,
            _ => CensorshipLevel::Low,
        }
    }

    #[deprecated(note = "Use score_protocol_advanced for better accuracy")]
    pub fn score_protocol(
        &self,
        protocol: VpnProtocol,
        context: &SelectionContext,
    ) -> f64 {
        let mut score = 0.0;

        const SPEED_WEIGHT: f64 = 0.3;
        const SECURITY_WEIGHT: f64 = 0.25;
        const STEALTH_WEIGHT: f64 = 0.25;
        const BATTERY_WEIGHT: f64 = 0.2;

        score += protocol.performance_score() * SPEED_WEIGHT;
        score += 0.9 * SECURITY_WEIGHT;

        let stealth = if self.is_censored_country(&context.user_country) {
            protocol.stealth_score()
        } else {
            0.5
        };
        score += stealth * STEALTH_WEIGHT;

        let battery = if context.device_type == DeviceType::Mobile {
            match protocol {
                VpnProtocol::IKEv2 => 1.0,
                VpnProtocol::WireGuard => 0.9,
                VpnProtocol::Shadowsocks => 0.85,
                _ => 0.7,
            }
        } else {
            0.8
        };
        score += battery * BATTERY_WEIGHT;

        score
    }
}

impl Default for ProtocolSelector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_censorship_detection() {
        let selector = ProtocolSelector::new();
        assert!(selector.is_censored_country("CN"));
        assert!(selector.is_censored_country("IR"));
        assert!(!selector.is_censored_country("FR"));
    }

    #[test]
    fn test_protocol_selection_china() {
        let selector = ProtocolSelector::new();
        let context = SelectionContext {
            network_quality: NetworkQuality {
                latency_ms: 50,
                packet_loss: 0.01,
                bandwidth_mbps: 100.0,
                stability: 0.9,
            },
            firewall_profile: FirewallProfile::NationalCensorship,
            user_country: "CN".to_string(),
            device_type: DeviceType::Desktop,
            battery_level: None,
            use_case: UseCase::Browsing,
        };

        let protocol = selector.select_best_protocol(&context);
        assert!(protocol.is_anti_censorship());
    }

    #[test]
    fn test_protocol_selection_mobile_low_battery() {
        let selector = ProtocolSelector::new();
        let context = SelectionContext {
            network_quality: NetworkQuality {
                latency_ms: 30,
                packet_loss: 0.02,
                bandwidth_mbps: 50.0,
                stability: 0.95,
            },
            firewall_profile: FirewallProfile::Open,
            user_country: "FR".to_string(),
            device_type: DeviceType::Mobile,
            battery_level: Some(0.15),
            use_case: UseCase::Browsing,
        };

        let protocol = selector.select_best_protocol(&context);
        assert_eq!(protocol, VpnProtocol::IKEv2);
    }
}
