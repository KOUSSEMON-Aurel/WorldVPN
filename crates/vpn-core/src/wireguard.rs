//! Implémentation WireGuard utilisant boringtun

use crate::error::{Result, VpnError};
use crate::protocol::VpnProtocol;
use crate::tunnel::{ConnectionConfig, Credentials, Interface, TunnelHandle, TunnelStats, VpnTunnel};
use async_trait::async_trait;
use boringtun::noise::{Tunn, TunnResult};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Tunnel WireGuard
pub struct WireGuardTunnel {
    /// État interne du tunnel (partagé pour accès concurrent)
    state: Arc<Mutex<TunnelState>>,
    /// Statistiques
    stats: Arc<Mutex<TunnelStats>>,
    /// Handle boringtun
    tunnel: Option<Box<Tunn>>,
}

struct TunnelState {
    connected: bool,
    endpoint: Option<SocketAddr>,
    local_ip: Option<IpAddr>,
}

impl WireGuardTunnel {
    /// Crée un nouveau tunnel
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(TunnelState {
                connected: false,
                endpoint: None,
                local_ip: None,
            })),
            stats: Arc::new(Mutex::new(TunnelStats {
                bytes_sent: 0,
                bytes_received: 0,
                avg_latency_ms: 0,
                packet_loss: 0.0,
                uptime: Duration::from_secs(0),
                current_throughput_mbps: 0.0,
            })),
            tunnel: None,
        }
    }
}

#[async_trait]
impl VpnTunnel for WireGuardTunnel {
    async fn connect(&mut self, config: &ConnectionConfig) -> Result<TunnelHandle> {
        tracing::info!("Connexion WireGuard vers {:?}", config.server_addr);

        // Extraction des clés
        let (private_key, peer_public_key) = match &config.credentials {
            Credentials::KeyPair {
                private_key,
                peer_public_key,
            } => (private_key, peer_public_key),
            _ => return Err(VpnError::InvalidConfig("WireGuard nécessite une paire de clés".into())),
        };

        // Conversion des clés pour boringtun (32 bytes)
        let static_private = x25519_dalek::StaticSecret::from(
            TryInto::<[u8; 32]>::try_into(private_key.as_slice())
                .map_err(|_| VpnError::InvalidConfig("Clé privée invalide (doit être 32 bytes)".into()))?
        );
        
        let peer_public = x25519_dalek::PublicKey::from(
            TryInto::<[u8; 32]>::try_into(peer_public_key.as_slice())
                .map_err(|_| VpnError::InvalidConfig("Clé publique invalide (doit être 32 bytes)".into()))?
        );

        // Initialisation boringtun (mock IP locale pour l'instant)
        // Dans une vraie implémentation, on configurerait l'interface TUN ici
        let local_ip: IpAddr = "10.0.0.2".parse().unwrap();
        
        // Création du tunnel boringtun (cette version retourne Tunn directement)
        let tunnel = Tunn::new(
            static_private,
            peer_public,
            None, // Preshared key
            None, // Persistent keepalive
            0,    // Index
            None  // Rate limiter
        );

        self.tunnel = Some(Box::new(tunnel));

        // Mise à jour état
        {
            let mut state = self.state.lock().await;
            state.connected = true;
            state.endpoint = Some(config.server_addr);
            state.local_ip = Some(local_ip);
        }

        tracing::info!("Tunnel WireGuard établi ! IP assignée: {}", local_ip);

        Ok(TunnelHandle {
            id: "wg-0".to_string(),
            protocol: VpnProtocol::WireGuard,
            assigned_ip: local_ip,
            remote_endpoint: config.server_addr,
        })
    }

    async fn send(&mut self, data: &[u8]) -> Result<usize> {
        if let Some(tunnel) = &mut self.tunnel {
            // Encapsulation boringtun
            let mut buf = vec![0u8; data.len() + 100]; // Buffer avec overhead
            match tunnel.encapsulate(data, &mut buf) {
                TunnResult::WriteToNetwork(packet) => {
                    // Ici on enverrait 'packet' sur l'interface UDP
                    // Pour le prototype, on simule l'envoi
                    let len = packet.len();
                    let mut stats = self.stats.lock().await;
                    stats.bytes_sent += len as u64;
                    Ok(len)
                },
                _ => Err(VpnError::Internal("Erreur encapsulation WireGuard".into())),
            }
        } else {
            Err(VpnError::ConnectionFailed("Tunnel non initialisé".into()))
        }
    }

    async fn recv(&mut self, buf: &mut [u8]) -> Result<usize> {
        // Simulation réception
        // Dans la réalité: lire socket UDP -> tunnel.decapsulate -> écrire dans buf
        tokio::time::sleep(Duration::from_millis(10)).await;
        Ok(0)
    }

    async fn disconnect(&mut self) -> Result<()> {
        tracing::info!("Déconnexion WireGuard");
        let mut state = self.state.lock().await;
        state.connected = false;
        self.tunnel = None;
        Ok(())
    }

    fn stats(&self) -> TunnelStats {
        // Retourne une copie des stats (non-async car bloquant minimal)
        futures::executor::block_on(async {
            self.stats.lock().await.clone()
        })
    }
    
    fn protocol(&self) -> VpnProtocol {
        VpnProtocol::WireGuard
    }
}
