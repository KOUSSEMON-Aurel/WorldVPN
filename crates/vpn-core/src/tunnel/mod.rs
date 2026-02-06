use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use crate::error::Result;
use crate::protocol::VpnProtocol;

pub mod windows;
pub use windows::WindowsTunnel;

/// Provides a handle to an active VPN tunnel connection
#[derive(Debug, Clone)]
pub struct TunnelHandle {
    pub id: String,
    pub protocol: VpnProtocol,
    pub assigned_ip: IpAddr,
    pub remote_endpoint: SocketAddr,
}

/// Consolidated configuration for establishing a new connection
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    pub protocol: VpnProtocol,
    pub server_addr: SocketAddr,
    pub credentials: Credentials,
    pub timeout: Duration,
}

/// Multi-protocol authentication types
#[derive(Debug, Clone)]
pub enum Credentials {
    KeyPair {
        private_key: Vec<u8>,
        peer_public_key: Vec<u8>,
    },
    Password {
        username: Option<String>,
        password: String,
    },
    Certificate {
        cert: Vec<u8>,
        key: Vec<u8>,
        ca: Vec<u8>,
    },
}

/// Runtime performance and health metrics for a tunnel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelStats {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub avg_latency_ms: u32,
    pub packet_loss: f64,
    pub uptime: Duration,
    pub current_throughput_mbps: f64,
}

/// Represents a physical or logical network interface (eth0, wlan0, etc.)
#[derive(Debug, Clone)]
pub struct Interface {
    pub name: String,
    pub interface_type: InterfaceType,
    pub local_ip: IpAddr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceType {
    Ethernet,
    WiFi,
    Cellular,
    Other,
}

/// Unified interface for all supported VPN protocols
#[async_trait]
pub trait VpnTunnel: Send + Sync {
    /// Launches the tunnel and returns a handle if successful
    async fn connect(&mut self, config: &ConnectionConfig) -> Result<TunnelHandle>;

    /// Encapsulates and sends raw data through the tunnel
    async fn send(&mut self, data: &[u8]) -> Result<usize>;

    /// Decapsulates and receives raw data from the tunnel
    async fn recv(&mut self, buf: &mut [u8]) -> Result<usize>;

    /// Gracefully shuts down the connection and cleans up resources
    async fn disconnect(&mut self) -> Result<()>;

    /// Returns the latest performance statistics
    fn stats(&self) -> TunnelStats;

    /// Triggers protocol-specific roaming (e.g., MOBIKE for IKEv2)
    async fn handle_network_change(&mut self, _new_interface: Interface) -> Result<()> {
        Ok(())
    }

    /// Identifies the underlying active protocol
    fn protocol(&self) -> VpnProtocol;

    /// Optional reachability check
    async fn ping(&mut self) -> Result<Duration> {
        Ok(Duration::from_millis(0))
    }
}
