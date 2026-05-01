//! Module P2P - Découverte et gestion de pairs
//!
//! Utilise libp2p pour la découverte de nœuds et le gossip.

use crate::error::VpnError;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::time::Duration;
use libp2p::{
    gossipsub, identity, kad, mdns, noise, swarm::{NetworkBehaviour, SwarmEvent}, tcp, yamux, SwarmBuilder,
    StreamProtocol,
};
use libp2p::futures::StreamExt;
use tokio::sync::mpsc;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Identifiant unique d'un pair
pub type PeerId = String;

/// Information sur un pair (nœud VPN)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub id: PeerId,
    pub country: String,
    pub city: Option<String>,
    pub public_addr: Option<IpAddr>,
    pub reputation: u32,
    pub capabilities: PeerCapabilities,
    pub latency_ms: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerCapabilities {
    pub streaming: bool,
    pub p2p_torrents: bool,
    pub bandwidth_mbps: f64,
    pub uptime: f64,
}

#[derive(Debug, Clone)]
pub struct PeerCriteria {
    pub country: Option<String>,
    pub min_reputation: u32,
    pub min_bandwidth_mbps: f64,
    pub use_case: crate::selector::UseCase,
}

#[derive(NetworkBehaviour)]
struct WorldVPNBehaviour {
    kademlia: kad::Behaviour<kad::store::MemoryStore>,
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
}

pub struct PeerDiscovery {
    sender: mpsc::Sender<P2pCommand>,
}

/// Reçu de crédit signé
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditReceipt {
    pub session_id: String,
    pub consumer_pubkey: String,
    pub provider_pubkey: String,
    pub bytes_total: u64,
    pub timestamp: i64,
    pub signature: String, // Signé par le consommateur
}

enum P2pCommand {
    FindPeers(PeerCriteria, tokio::sync::oneshot::Sender<Vec<PeerInfo>>),
    Announce(PeerInfo, tokio::sync::oneshot::Sender<std::result::Result<(), VpnError>>),
    SubmitReceipt(CreditReceipt, tokio::sync::oneshot::Sender<std::result::Result<(), VpnError>>),
}

impl PeerDiscovery {
    pub async fn new() -> std::result::Result<Self, VpnError> {
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = libp2p::PeerId::from(local_key.public());
        tracing::info!("Démarrage du nœud P2P local: {local_peer_id}");

        let mut swarm = SwarmBuilder::with_existing_identity(local_key)
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            ).map_err(|e| VpnError::P2pError(e.to_string()))?
            .with_dns().map_err(|e| VpnError::P2pError(e.to_string()))?
            .with_behaviour(|key| {
                let peer_id = libp2p::PeerId::from(key.public());
                
                // Gossipsub
                let message_id_fn = |message: &gossipsub::Message| {
                    let mut s = DefaultHasher::new();
                    message.data.hash(&mut s);
                    gossipsub::MessageId::from(s.finish().to_string())
                };
                let gossipsub_config = gossipsub::ConfigBuilder::default()
                    .heartbeat_interval(Duration::from_secs(10))
                    .validation_mode(gossipsub::ValidationMode::Strict)
                    .message_id_fn(message_id_fn)
                    .build()
                    .expect("Valid gossip config");
                let mut gossipsub_behaviour = gossipsub::Behaviour::new(
                    gossipsub::MessageAuthenticity::Signed(key.clone()),
                    gossipsub_config,
                ).expect("Valid gossip behaviour");
                
                let topic = gossipsub::IdentTopic::new("worldvpn/nodes/v1");
                gossipsub_behaviour.subscribe(&topic).unwrap();

                // Kademlia
                let store = kad::store::MemoryStore::new(peer_id);
                let mut kad_config = kad::Config::default();
                kad_config.set_protocol_names(vec![StreamProtocol::new("/worldvpn/kad/1.0.0")]);
                let kademlia = kad::Behaviour::with_config(peer_id, store, kad_config);

                // Mdns
                let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), peer_id).unwrap();

                WorldVPNBehaviour {
                    kademlia,
                    gossipsub: gossipsub_behaviour,
                    mdns,
                }
            }).map_err(|e| VpnError::P2pError(e.to_string()))?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
            .build();

        let _ = swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse().unwrap());

        let (tx, mut rx) = mpsc::channel(32);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    event = swarm.select_next_some() => match event {
                        SwarmEvent::Behaviour(WorldVPNBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                            for (peer_id, multiaddr) in list {
                                tracing::debug!("mDNS découverte: {peer_id} {multiaddr}");
                                swarm.behaviour_mut().kademlia.add_address(&peer_id, multiaddr);
                            }
                        },
                        SwarmEvent::Behaviour(WorldVPNBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                            for (peer_id, multiaddr) in list {
                                tracing::debug!("mDNS expiré: {peer_id} {multiaddr}");
                                swarm.behaviour_mut().kademlia.remove_address(&peer_id, &multiaddr);
                            }
                        },
                        SwarmEvent::Behaviour(WorldVPNBehaviourEvent::Gossipsub(gossipsub::Event::Message { message, .. })) => {
                            tracing::info!("Gossipsub Message reçu: {:?}", String::from_utf8_lossy(&message.data));
                        },
                        _ => {}
                    },
                    cmd = rx.recv() => {
                        if let Some(command) = cmd {
                            match command {
                                P2pCommand::FindPeers(_criteria, reply) => {
                                    let _ = reply.send(vec![]); // Simple mock
                                }
                                P2pCommand::Announce(_info, reply) => {
                                    let _ = reply.send(Ok(()));
                                }
                                P2pCommand::SubmitReceipt(receipt, reply) => {
                                    // Gossip the credit receipt to the network (and specifically to the provider/backend)
                                    let topic = gossipsub::IdentTopic::new("worldvpn/credits/v1");
                                    let data = serde_json::to_vec(&receipt).unwrap();
                                    let _ = swarm.behaviour_mut().gossipsub.publish(topic, data);
                                    let _ = reply.send(Ok(()));
                                }
                            }
                        } else {
                            break;
                        }
                    }
                }
            }
        });

        Ok(Self { sender: tx })
    }
    
    pub async fn submit_receipt(&self, receipt: CreditReceipt) -> std::result::Result<(), VpnError> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.sender.send(P2pCommand::SubmitReceipt(receipt, tx)).await.unwrap();
        rx.await.unwrap()
    }

    pub async fn find_peers(&self, criteria: &PeerCriteria) -> std::result::Result<Vec<PeerInfo>, VpnError> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.sender.send(P2pCommand::FindPeers(criteria.clone(), tx)).await.unwrap();
        Ok(rx.await.unwrap())
    }

    pub async fn announce(&self, info: &PeerInfo) -> std::result::Result<(), VpnError> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.sender.send(P2pCommand::Announce(info.clone(), tx)).await.unwrap();
        rx.await.unwrap()
    }
}
