use serde::{Deserialize, Serialize};
use crate::error::{Result, VpnError};
use std::net::SocketAddr;

/// Defines the final path used for a successful Peer-to-Peer connection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionPath {
    Direct,        // Public IP to Public IP
    HolePunching,  // Successfully pierced NAT via STUN/ICE
    Relay,         // Failed P2P, falling back to TURN relay
}

/// NAT behavior type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NatType {
    Open,
    FullCone,
    RestrictedCone,
    PortRestrictedCone,
    Symmetric,
    Unknown,
}

/// Parameters for discovering network topology and piercing firewalls
#[derive(Debug, Clone)]
pub struct NatConfig {
    pub stun_servers: Vec<String>,
    pub turn_servers: Vec<TurnServer>,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone)]
pub struct TurnServer {
    pub url: String,
    pub username: String,
    pub password: String,
}

/// Orchestrates NAT traversal techniques to maximize P2P success rates
pub struct NatTraversal {
    #[allow(dead_code)]
    config: NatConfig,
}

impl NatTraversal {
    pub fn new(config: NatConfig) -> Self {
        Self { config }
    }

    /// Detects the NAT type using configured STUN servers.
    /// Crucial for deciding if UDP Hole Punching is feasible.
    pub async fn detect_nat_type(&self) -> Result<NatType> {
        tracing::info!("Detecting NAT type using STUN...");
        
        let mut mapped_addresses = Vec::new();

        for server in &self.config.stun_servers {
            match self.get_mapped_address(server).await {
                Ok(addr) => {
                    tracing::info!("STUN Server {}: Mapped Address is {:?}", server, addr);
                    mapped_addresses.push(addr);
                }
                Err(e) => {
                    tracing::warn!("STUN Server {} failed: {}", server, e);
                }
            }
        }

        if mapped_addresses.is_empty() {
            return Ok(NatType::Unknown);
        }

        // Basic heuristic: if the mapped port changes between servers, it's likely Symmetric
        let first = mapped_addresses[0];
        let is_symmetric = mapped_addresses.iter().any(|&addr| addr.port() != first.port());

        if is_symmetric {
            tracing::warn!("Symmetric NAT detected! P2P will require Hysteria2/TCP or Relay.");
            Ok(NatType::Symmetric)
        } else {
            // For now, simplify Cone types to PortRestrictedCone (most common behavior)
            Ok(NatType::PortRestrictedCone)
        }
    }

    /// Returns a single public endpoint (SocketAddr) for signaling.
    pub async fn get_public_endpoint(&self) -> Result<SocketAddr> {
        // Try the first working STUN server
        for server in &self.config.stun_servers {
            if let Ok(addr) = self.get_mapped_address(server).await {
                return Ok(addr);
            }
        }
        Err(VpnError::NatTraversalFailed("Could not discover public endpoint from any STUN server".into()))
    }

    async fn get_mapped_address(&self, server_url: &str) -> Result<SocketAddr> {
        use stun::message::*;
        use stun::agent::*;
        use tokio::net::UdpSocket;

        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.connect(server_url).await?;

        let mut msg = Message::new();
        msg.build(&[
            Box::new(TransactionId::default()),
            Box::new(BINDING_REQUEST),
        ])?;

        socket.send(&msg.marshal_binary()?).await?;

        let mut buf = [0u8; 1024];
        let len = tokio::time::timeout(
            std::time::Duration::from_millis(self.config.timeout_ms),
            socket.recv(&mut buf)
        ).await.map_err(|_| VpnError::ConnectionFailed("STUN Timeout".to_string()))??;

        let mut response = Message::new();
        response.unmarshal_binary(&buf[..len])?;

        let mut mapped_addr = None;
        
        // Try XOR-MAPPED-ADDRESS first (RFC 5389)
        let mut xor_addr = stun::xoraddr::XorMappedAddress::default();
        if xor_addr.get_from(&response).is_ok() {
            mapped_addr = Some(SocketAddr::new(xor_addr.ip, xor_addr.port));
        }

        mapped_addr.ok_or_else(|| VpnError::NatTraversalFailed("No mapped address in STUN response".to_string()))
    }


    /// Attempts to establish a connection using a prioritized progressive strategy
    pub async fn establish_connection(&self, peer_addr: SocketAddr) -> Result<ConnectionPath> {
        tracing::info!("Establishing connection to {:?}", peer_addr);

        // Tier 1: Direct attempt (Optimistic)
        if let Ok(_) = self.try_direct_connection(peer_addr).await {
            tracing::info!("Direct connection successful");
            return Ok(ConnectionPath::Direct);
        }

        // Tier 2: STUN/ICE hole punching (Intermediate)
        if let Ok(_) = self.try_hole_punching(peer_addr).await {
            tracing::info!("Hole punching successful");
            return Ok(ConnectionPath::HolePunching);
        }

        // Tier 3: TURN relay (Fail-safe)
        if let Ok(_) = self.try_relay_connection(peer_addr).await {
            tracing::info!("Relay connection successful");
            return Ok(ConnectionPath::Relay);
        }

        Err(VpnError::NatTraversalFailed(
            "All connection methods failed".to_string(),
        ))
    }

    async fn try_direct_connection(&self, _peer_addr: SocketAddr) -> Result<()> {
        tracing::debug!("Attempting direct connection");
        // Logic for raw UDP/TCP binding goes here
        Err(VpnError::NatTraversalFailed("Not implemented".to_string()))
    }

    async fn try_hole_punching(&self, _peer_addr: SocketAddr) -> Result<()> {
        tracing::debug!("Attempting hole punching");
        // Integration with libp2p or webrtc-ice expected here
        Err(VpnError::NatTraversalFailed("Not implemented".to_string()))
    }

    async fn try_relay_connection(&self, _peer_addr: SocketAddr) -> Result<()> {
        tracing::debug!("Attempting TURN relay");
        // TURN-specific encapsulation logic goes here
        Err(VpnError::NatTraversalFailed("Not implemented".to_string()))
    }
}

impl Default for NatConfig {
    fn default() -> Self {
        Self {
            stun_servers: vec![
                "stun.l.google.com:19302".to_string(),
                "stun1.l.google.com:19302".to_string(),
            ],
            turn_servers: vec![],
            timeout_ms: 5000,
        }
    }
}
