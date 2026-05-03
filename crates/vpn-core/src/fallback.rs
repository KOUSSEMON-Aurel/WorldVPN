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
    pub assigned_ip: String,
    pub private_key: Option<String>,
    pub peer_public_key: Option<String>,
}

pub struct FallbackManager {
    warp_enabled: bool,
}

impl FallbackManager {
    pub fn new(warp_enabled: bool) -> Self {
        Self { warp_enabled }
    }

    pub async fn get_fallback_config(&self, country: &str) -> Result<FallbackConfig> {
        if self.warp_enabled {
            if let Ok(config) = self.get_warp_server(country).await {
                return Ok(config);
            }
        }
        self.get_vpngate_server(country).await
    }

    async fn get_warp_server(&self, _country: &str) -> Result<FallbackConfig> {
        tracing::info!("Fetching Cloudflare WARP fallback");
        let warp_config = crate::warp::register_warp_device().await?;
        
        Ok(FallbackConfig {
            provider: FallbackProvider::CloudflareWarp,
            server_ip: warp_config.endpoint.split(':').next().unwrap_or("162.159.193.1").to_string(),
            port: warp_config.endpoint.split(':').nth(1).and_then(|p| p.parse().ok()).unwrap_or(2408),
            protocol: VpnProtocol::WireGuard,
            assigned_ip: "172.16.0.2".to_string(), // Default WARP address
            private_key: Some(warp_config.private_key),
            peer_public_key: Some(warp_config.peer_public_key),
        })
    }

    async fn get_vpngate_server(&self, country: &str) -> Result<FallbackConfig> {
        tracing::info!("Fetching VPNGate fallback for {}", country);
        
        let api_url = std::env::var("WORLDVPN_API_URL")
            .unwrap_or_else(|_| "https://worldvpn-backend.onrender.com".to_string());
            
        let endpoint = format!("{}/api/v1/fallback?country={}", api_url, country);
        let config = match reqwest::get(&endpoint).await {
            Ok(res) if res.status().is_success() => {
                if let Ok(data) = res.json::<serde_json::Value>().await {
                    let ip = data["ip_address"].as_str().unwrap_or("219.100.37.55").to_string();
                    let port = data["port"].as_u64().unwrap_or(8388) as u16;
                    Some((ip, port))
                } else {
                    None
                }
            }
            _ => None,
        };
        
        let (server_ip, port) = config.unwrap_or_else(|| ("219.100.37.55".to_string(), 8388));

        Ok(FallbackConfig {
            provider: FallbackProvider::VpnGate,
            server_ip,
            port,
            protocol: VpnProtocol::Shadowsocks,
            assigned_ip: "10.10.0.2".to_string(),
            private_key: None,
            peer_public_key: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fallback_guaranteed_precedence() {
        let manager = FallbackManager::new(true);
        let config = manager.get_fallback_config("FR").await.unwrap();
        assert!(matches!(config.provider, FallbackProvider::CloudflareWarp));
        assert_eq!(config.protocol, VpnProtocol::WireGuard);
        assert!(config.private_key.is_some());
    }

    #[tokio::test]
    async fn test_fallback_vpngate() {
        let manager = FallbackManager::new(false);
        let config = manager.get_fallback_config("FR").await.unwrap();
        assert!(matches!(config.provider, FallbackProvider::VpnGate));
        assert_eq!(config.protocol, VpnProtocol::Shadowsocks);
    }
}
