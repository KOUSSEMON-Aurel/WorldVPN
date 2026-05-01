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
        
        // Automated registration for Zero-Config experience
        let warp_config = crate::warp::register_warp_device().await?;
        
        Ok(FallbackConfig {
            provider: FallbackProvider::CloudflareWarp,
            server_ip: warp_config.endpoint.split(':').next().unwrap_or("162.159.193.1").to_string(),
            port: warp_config.endpoint.split(':').nth(1).and_then(|p| p.parse().ok()).unwrap_or(2408),
            protocol: VpnProtocol::WireGuard,
            credentials: Some(warp_config.private_key), // We store the private key in credentials for the tunnel to use
        })
    }

    async fn get_vpngate_server(&self, country: &str) -> Result<FallbackConfig> {
        tracing::info!("Fetching VPNGate fallback for {}", country);
        // FIXME: Query Backend API for latest VPNGate servers
        // VPNGate fallback will now prefer Shadowsocks for better anti-censorship
        Ok(FallbackConfig {
            provider: FallbackProvider::VpnGate,
            server_ip: "219.100.37.55".to_string(), // Fake IP for scaffolding
            port: 8388,
            protocol: VpnProtocol::Shadowsocks,
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
        assert_eq!(config.protocol, VpnProtocol::Shadowsocks);
    }
}
