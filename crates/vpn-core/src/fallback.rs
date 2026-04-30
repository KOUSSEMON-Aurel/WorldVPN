//! Fallback Module - Premium & Public Rescue Networks
//! Maintains Mullvad (WG) and VPNGate integration when P2P fails.

use crate::error::Result;
use crate::protocol::VpnProtocol;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FallbackProvider {
    CloudflareWarp,
    VpnGate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackConfig {
    pub provider: FallbackProvider,
    pub server_ip: String,
    pub port: u16,
    pub protocol: VpnProtocol,
    pub credentials: Option<String>, 
}

/// FallbackManager handles retrieving connection settings for backup networks
pub struct FallbackManager {
    warp_enabled: bool,
}

impl FallbackManager {
    pub fn new(warp_enabled: bool) -> Self {
        Self { warp_enabled }
    }

    /// Tries to obtain a fallback configuration (WARP first, then VPNGate)
    pub async fn get_fallback_config(&self, country: &str) -> Result<FallbackConfig> {
        // Tiers 4: Fallback High-Speed (Cloudflare WARP)
        if self.warp_enabled {
            if let Ok(config) = self.get_warp_server(country).await {
                return Ok(config);
            }
        }
        
        // Tiers 4: Fallback Public (VPNGate)
        self.get_vpngate_server(country).await
    }

    async fn get_warp_server(&self, _country: &str) -> Result<FallbackConfig> {
        tracing::info!("Fetching Cloudflare WARP fallback");
        // FIXME: Implement Warp configuration generation logic (e.g. via wgcf logic)
        // WARP uses WireGuard natively
        Ok(FallbackConfig {
            provider: FallbackProvider::CloudflareWarp,
            server_ip: "162.159.192.1".to_string(), // Common WARP endpoint
            port: 2408,
            protocol: VpnProtocol::WireGuard,
            credentials: None, // WARP keys are usually handled in a separate config
        })
    }

    async fn get_vpngate_server(&self, country: &str) -> Result<FallbackConfig> {
        tracing::info!("Fetching VPNGate fallback for {}", country);
        // FIXME: Query Backend API for latest VPNGate servers
        // VPNGate often uses OpenVPN over UDP/TCP
        Ok(FallbackConfig {
            provider: FallbackProvider::VpnGate,
            server_ip: "219.100.37.55".to_string(), // Fake IP for scaffolding
            port: 1194,
            protocol: VpnProtocol::OpenVpnUdp,
            credentials: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fallback_guaranteed_precedence() {
        // Fallback manager with WARP enabled
        let manager = FallbackManager::new(true);
        let config = manager.get_fallback_config("FR").await.unwrap();
        
        // Should prioritize Cloudflare WARP (WireGuard)
        assert!(matches!(config.provider, FallbackProvider::CloudflareWarp));
        assert_eq!(config.protocol, VpnProtocol::WireGuard);
    }

    #[tokio::test]
    async fn test_fallback_vpngate() {
        // Fallback manager with WARP disabled
        let manager = FallbackManager::new(false);
        let config = manager.get_fallback_config("FR").await.unwrap();
        
        // Should fallback to public (VPNGate)
        assert!(matches!(config.provider, FallbackProvider::VpnGate));
        assert_eq!(config.protocol, VpnProtocol::OpenVpnUdp);
    }
}
