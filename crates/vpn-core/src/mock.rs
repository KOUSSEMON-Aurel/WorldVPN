//! Mock Tunnel pour tests et développement sans réseau

use crate::error::Result;
use crate::protocol::VpnProtocol;
use crate::tunnel::{ConnectionConfig, TunnelHandle, TunnelStats, VpnTunnel};
use async_trait::async_trait;
use std::time::Duration;

/// Tunnel simulé qui logue juste les actions
pub struct MockTunnel {
    protocol: VpnProtocol,
    connected: bool,
}

impl MockTunnel {
    /// Crée un nouveau mock
    pub fn new(protocol: VpnProtocol) -> Self {
        Self {
            protocol,
            connected: false,
        }
    }
}

#[async_trait]
impl VpnTunnel for MockTunnel {
    async fn connect(&mut self, config: &ConnectionConfig) -> Result<TunnelHandle> {
        tracing::info!("[MOCK] Connexion mock ({:?}) vers {:?}", self.protocol, config.server_addr);
        tokio::time::sleep(Duration::from_millis(500)).await; // Simule latence handshake
        
        self.connected = true;
        
        Ok(TunnelHandle {
            id: "mock-1".to_string(),
            protocol: self.protocol,
            assigned_ip: "192.168.1.100".parse().unwrap(),
            remote_endpoint: config.server_addr,
        })
    }

    async fn send(&mut self, data: &[u8]) -> Result<usize> {
        if !self.connected {
            return Err(crate::error::VpnError::ConnectionFailed("Non connecté".into()));
        }
        tracing::debug!("[MOCK] Envoi {} bytes", data.len());
        Ok(data.len())
    }

    async fn recv(&mut self, _buf: &mut [u8]) -> Result<usize> {
        if !self.connected {
            return Err(crate::error::VpnError::ConnectionFailed("Non connecté".into()));
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
        Ok(0)
    }

    async fn disconnect(&mut self) -> Result<()> {
        tracing::info!("[MOCK] Déconnexion");
        self.connected = false;
        Ok(())
    }

    fn stats(&self) -> TunnelStats {
        TunnelStats {
            bytes_sent: 1000,
            bytes_received: 2000,
            avg_latency_ms: 10,
            packet_loss: 0.0,
            uptime: Duration::from_secs(300),
            current_throughput_mbps: 100.0,
        }
    }
    
    fn protocol(&self) -> VpnProtocol {
        self.protocol
    }
}
