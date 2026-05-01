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

    /// Returns the explicit 4-level connection cascade sequence
    /// Level 1: WireGuard P2P (Performance)
    /// Level 2: Hysteria2 P2P (Turbo/Mobile stability)
    /// Level 3: Shadowsocks P2P (Bypass DPI/Symmetric NAT)
    /// Level 4: Fallback handled by `FallbackManager` if all above fail
    pub fn build_connection_cascade(&self, context: &SelectionContext, symmetric_nat: bool) -> Vec<VpnProtocol> {
        tracing::info!("Building protocol cascade for context: {:?}", context);
        let mut cascade = Vec::new();

        // Level 1 & 2: UDP Based P2P (WireGuard & Hysteria2)
        // If NAT is symmetric, P2P UDP is almost impossible, skip to TCP
        if !symmetric_nat {
            // Check for censorship
            if self.is_censored_country(&context.user_country) {
                // In censored countries, regular WG is blocked, try Obfuscated then Hysteria
                cascade.push(VpnProtocol::WireGuardObfuscated);
                cascade.push(VpnProtocol::Hysteria2);
            } else {
                cascade.push(VpnProtocol::WireGuard);
                cascade.push(VpnProtocol::Hysteria2);
            }
        } else {
            tracing::info!("Symmetric NAT detected, skipping UDP P2P (WG/H2) directly to TCP/Shadowsocks");
        }

        // Level 3: TCP Based P2P (Shadowsocks / Trojan)
        let level = self.censorship_level(&context.user_country);
        if level >= CensorshipLevel::High {
            cascade.push(VpnProtocol::Trojan);
            cascade.push(VpnProtocol::VLESS);
        } else {
            cascade.push(VpnProtocol::Shadowsocks);
        }

        // Level 4: Fallback is handled implicitly when the cascade array is exhausted
        // Return the sequence to attempt
        cascade
    }

    /// Primary entry point for selecting a protocol for a single connection (Legacy)
    pub fn select_best_protocol(&self, context: &SelectionContext) -> VpnProtocol {
        tracing::info!("Selecting single protocol for context (Legacy): {:?}", context);

        if self.is_censored_country(&context.user_country) {
            let level = self.censorship_level(&context.user_country);
            return self.select_anti_censorship_protocol(level);
        }

        if context.device_type == DeviceType::Mobile {
            if let Some(battery) = context.battery_level {
                if battery < 0.20 {
                    return VpnProtocol::WireGuard;
                }
            }
        }

        if context.network_quality.packet_loss > 0.05 {
            return VpnProtocol::Hysteria2;
        }

        if context.firewall_profile == FirewallProfile::Corporate {
            return VpnProtocol::Shadowsocks;
        }

        match context.use_case {
            UseCase::Gaming => return VpnProtocol::WireGuard,
            UseCase::Privacy => return VpnProtocol::WireGuardObfuscated,
            _ => {}
        }

        VpnProtocol::WireGuard
    }

    fn select_anti_censorship_protocol(&self, level: CensorshipLevel) -> VpnProtocol {
        match level {
            CensorshipLevel::None | CensorshipLevel::Low => VpnProtocol::Shadowsocks,
            CensorshipLevel::Medium => VpnProtocol::WireGuardObfuscated,
            CensorshipLevel::High => VpnProtocol::Trojan,
            CensorshipLevel::Extreme => VpnProtocol::VLESS,
        }
    }

    /// Computes and ranks all protocols by a combined score
    pub fn rank_all_protocols(&self, context: &SelectionContext) -> Vec<(VpnProtocol, f64)> {
        let protocols = vec![
            VpnProtocol::WireGuard,
            VpnProtocol::WireGuardObfuscated,
            VpnProtocol::Shadowsocks,
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
            VpnProtocol::WireGuard => 1.0,
            VpnProtocol::Shadowsocks | VpnProtocol::Hysteria2 => 0.95,
            VpnProtocol::Trojan | VpnProtocol::VLESS => 0.98,
            _ => 0.9,
        };
        score += security_score * security_w;

        // Stealth (Anti-DPI)
        score += protocol.stealth_score() * stealth_w;

        // Energy efficiency
        let battery_score = match protocol {
            VpnProtocol::WireGuard => 1.0,
            VpnProtocol::Shadowsocks => 0.90,
            VpnProtocol::Hysteria2 => 0.85,
            _ => 0.80,
        };
        score += battery_score * battery_w;

        // Reliability / Stability
        let stability_score = match protocol {
            VpnProtocol::Hysteria2 => 1.0,
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
        let security_w = 0.20;
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
            if matches!(protocol, VpnProtocol::WireGuard) {
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
                VpnProtocol::WireGuard => 1.0,
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
        assert_eq!(protocol, VpnProtocol::WireGuard);
    }

    #[test]
    fn test_cascade_symmetric_nat() {
        let selector = ProtocolSelector::new();
        let context = SelectionContext {
            network_quality: NetworkQuality {
                latency_ms: 50, packet_loss: 0.0, bandwidth_mbps: 100.0, stability: 1.0,
            },
            firewall_profile: FirewallProfile::Open,
            user_country: "US".to_string(),
            device_type: DeviceType::Desktop,
            battery_level: None,
            use_case: UseCase::Browsing,
        };

        // When NAT is symmetric, we should skip UDP P2P (WG, H2)
        let cascade = selector.build_connection_cascade(&context, true);
        assert!(!cascade.contains(&VpnProtocol::WireGuard));
        assert!(!cascade.contains(&VpnProtocol::Hysteria2));
        assert!(cascade.contains(&VpnProtocol::Shadowsocks));
    }

    #[test]
    fn test_cascade_censored_country() {
        let selector = ProtocolSelector::new();
        let context = SelectionContext {
            network_quality: NetworkQuality {
                latency_ms: 50, packet_loss: 0.0, bandwidth_mbps: 100.0, stability: 1.0,
            },
            firewall_profile: FirewallProfile::NationalCensorship,
            user_country: "CN".to_string(),
            device_type: DeviceType::Desktop,
            battery_level: None,
            use_case: UseCase::Browsing,
        };

        // In censored country, regular WG is replaced by Obfuscated or Trojan/VLESS depending on level
        let cascade = selector.build_connection_cascade(&context, false);
        assert!(!cascade.contains(&VpnProtocol::WireGuard));
        assert!(cascade.contains(&VpnProtocol::WireGuardObfuscated));
        assert!(cascade.contains(&VpnProtocol::Trojan));
        assert!(cascade.contains(&VpnProtocol::VLESS));
    }
}
