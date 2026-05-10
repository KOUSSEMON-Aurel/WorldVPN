use std::net::IpAddr;
use std::time::{Duration, Instant};
use async_trait::async_trait;
use serde_json::json;
use tracing::info;

use crate::error::{VpnError, Result};
use crate::protocol::VpnProtocol;
use crate::tunnel::{VpnTunnel, TunnelHandle, TunnelStats, ConnectionConfig, Credentials};
use crate::tunnel::go_bridge::GoBridge;

pub struct GoTunnel {
    id: String,
    protocol: VpnProtocol,
    assigned_ip: Option<IpAddr>,
    stats: TunnelStats,
    start_time: Option<Instant>,
}

impl GoTunnel {
    pub fn new(id: String, protocol: VpnProtocol) -> Self {
        Self {
            id,
            protocol,
            assigned_ip: None,
            stats: TunnelStats {
                bytes_sent: 0,
                bytes_received: 0,
                avg_latency_ms: 0,
                packet_loss: 0.0,
                uptime: Duration::from_secs(0),
                current_throughput_mbps: 0.0,
            },
            start_time: None,
        }
    }
}

#[async_trait]
impl VpnTunnel for GoTunnel {
    async fn connect(&mut self, config: &ConnectionConfig) -> Result<TunnelHandle> {
        info!("Connecting Go tunnel: protocol={:?}, endpoint={}", config.protocol, config.server_addr);

        // Map credentials to Go terminology
        let (password, private_key, peer_public_key, uuid) = match &config.credentials {
            Credentials::Password { password, .. } => (Some(password.clone()), None::<String>, None::<String>, None::<String>),
            Credentials::KeyPair { private_key, peer_public_key } => (
                None::<String>, 
                Some(hex::encode(private_key)), 
                Some(hex::encode(peer_public_key)), 
                None::<String>
            ),
            _ => (None::<String>, None::<String>, None::<String>, None::<String>), 
        };

        let config_json = json!({
            "session_id": self.id,
            "protocol": format!("{:?}", config.protocol),
            "assigned_ip": config.assigned_ip.to_string(),
            "peer_endpoint": config.server_addr.to_string(),
            "private_key": private_key.unwrap_or_default(),
            "peer_public_key": peer_public_key.unwrap_or_default(),
            "password": password.unwrap_or_default(),
            "uuid": uuid.unwrap_or_default(),
            "mtu": 1420,
        }).to_string();

        // 0 FD for Linux as Go handles it. Android will provide a real FD.
        GoBridge::start_tunnel(0, &config_json).map_err(|e| VpnError::ConnectionFailed(e.to_string()))?;

        self.start_time = Some(Instant::now());
        self.assigned_ip = Some(config.assigned_ip);

        Ok(TunnelHandle {
            id: self.id.clone(),
            protocol: config.protocol,
            assigned_ip: self.assigned_ip.unwrap(),
            remote_endpoint: config.server_addr,
        })
    }

    async fn send(&mut self, data: &[u8]) -> Result<usize> {
        // In a unified Go engine, packets are usually routed through the TUN interface by the OS.
        // This method would be used if we were doing custom encapsulation in Rust.
        Ok(data.len())
    }

    async fn recv(&mut self, _buf: &mut [u8]) -> Result<usize> {
        // Same as send.
        Ok(0)
    }

    async fn disconnect(&mut self) -> Result<()> {
        GoBridge::stop_tunnel().map_err(|e| VpnError::ConnectionFailed(e.to_string()))?;
        self.start_time = None;
        Ok(())
    }

    fn stats(&self) -> TunnelStats {
        let mut stats = self.stats.clone();
        if let Some(start) = self.start_time {
            stats.uptime = start.elapsed();
        }
        stats
    }

    fn protocol(&self) -> VpnProtocol {
        self.protocol
    }
}
