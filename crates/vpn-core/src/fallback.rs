//! Fallback Module - Public Rescue Networks
//! Maintains VPNGate and Public Shadowsocks integration when P2P fails.

use crate::error::{Result, VpnError};
use crate::protocol::VpnProtocol;
use crate::public_gate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FallbackProvider {
    PublicGate,
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
    pub raw_config: Option<String>,
}

pub struct FallbackManager;

impl FallbackManager {
    pub fn new() -> Self {
        Self
    }

    pub async fn get_fallback_config(&self, country: &str, backend_url: Option<&str>, blacklist: &[String]) -> Result<FallbackConfig> {
        tracing::info!("Fetching public fallback for {} (excluding {} nodes)", country, blacklist.len());
        
        let nodes = public_gate::fetch_all_public_nodes(backend_url).await;
        
        self.get_fallback_from_nodes(&nodes, country, blacklist)
    }

    /// Select a fallback node from a pre-loaded list to avoid re-fetching.
    pub fn get_fallback_from_nodes(&self, nodes: &[crate::nodes::VpnNode], country: &str, blacklist: &[String]) -> Result<FallbackConfig> {
        // Filter by country and blacklist, then by any country if needed
        let best_node = nodes.iter()
            .filter(|n| !blacklist.contains(&n.ip) && !blacklist.contains(&n.id))
            .find(|n| n.country_code == country)
            .or_else(|| nodes.iter()
                .filter(|n| !blacklist.contains(&n.ip) && !blacklist.contains(&n.id))
                .next());
            
        if let Some(node) = best_node {
            Ok(FallbackConfig {
                provider: FallbackProvider::PublicGate,
                server_ip: node.ip.clone(),
                port: node.port,
                protocol: node.protocol,
                assigned_ip: "10.10.0.2".to_string(),
                private_key: None,
                peer_public_key: None,
                raw_config: node.openvpn_config.clone(),
            })
        } else {
            Err(VpnError::ConnectionFailed("No public nodes available".into()))
        }
    }
}
