//! Module P2P - Découverte et gestion de pairs
//!
//! Utilise libp2p pour la découverte de nœuds et le gossip.
//!
//! ## Améliorations de sécurité
//! - Les `CreditReceipt` Gossipsub sont vérifiés par signature avant traitement.
//! - Un cache LRU déduplicale les messages pour éviter le replay/flood.
//! - Le bootstrap Kademlia utilise un retry exponentiel + fallback hardcodé.

use crate::crypto::IdentityKey;
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
use lru::LruCache;
use std::num::NonZeroUsize;

/// Identifiant unique d'un pair
pub type PeerId = String;

/// Liste de nœuds bootstrap hardcodés — fallback si le DHT n'est pas joignable.
/// Ces nœuds sont des relais publics connus du réseau WorldVPN.
const BOOTSTRAP_PEERS: &[&str] = &[
    "/dns4/bootstrap1.worldvpn.net/tcp/4001/p2p/12D3KooWEjsGMkjWkmKvAtaVBToQDHrqp3gu2UHdVvBsZz4NQHZM",
    "/dns4/bootstrap2.worldvpn.net/tcp/4001/p2p/12D3KooWRNhNdZt2Pc7hBSKRCyqL3zPJH2HJXT4K7BBpCqNDZ7v5",
];

/// Nombre maximum de message-IDs conservés dans le cache de déduplication
const DEDUP_CACHE_SIZE: usize = 512;

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
    /// Signature Ed25519 (base64 URL-safe) du consommateur sur le payload canonique.
    /// Payload signé : "{session_id}|{consumer_pubkey}|{provider_pubkey}|{bytes_total}|{timestamp}"
    pub signature: String,
}

impl CreditReceipt {
    /// Construit le message canonique qui doit être signé.
    pub fn signing_message(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}",
            self.session_id,
            self.consumer_pubkey,
            self.provider_pubkey,
            self.bytes_total,
            self.timestamp,
        )
    }

    /// Vérifie que la signature de ce reçu est valide (signée par `consumer_pubkey`).
    pub fn verify(&self) -> bool {
        let message = self.signing_message();
        IdentityKey::verify_signature_from_hex(&self.consumer_pubkey, &message, &self.signature)
    }
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
                
                // Gossipsub — authentification des messages activée
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
                let credit_topic = gossipsub::IdentTopic::new("worldvpn/credits/v1");
                gossipsub_behaviour.subscribe(&credit_topic).unwrap();

                // Kademlia
                let store = kad::store::MemoryStore::new(peer_id);
                let mut kad_config = kad::Config::default();
                kad_config.set_protocol_names(vec![StreamProtocol::new("/worldvpn/kad/1.0.0")]);
                let kademlia = kad::Behaviour::with_config(peer_id, store, kad_config);

                // Mdns (local network discovery)
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

        // Bootstrap Kademlia — retry exponentiel avec fallback hardcodé
        let mut bootstrapped = false;
        for attempt in 0..3u32 {
            if swarm.behaviour_mut().kademlia.bootstrap().is_ok() {
                tracing::info!("Kademlia bootstrap lancé (tentative {})", attempt + 1);
                bootstrapped = true;
                break;
            }
            let wait = Duration::from_millis(500 * 2u64.pow(attempt));
            tracing::warn!("Bootstrap Kademlia échoué, retry dans {:?}...", wait);
            tokio::time::sleep(wait).await;
        }

        // Fallback : ajouter les peers hardcodés si le bootstrap automatique échoue
        if !bootstrapped {
            tracing::warn!("Bootstrap Kademlia échoué après 3 tentatives. Injection de peers hardcodés.");
            for addr_str in BOOTSTRAP_PEERS {
                if let Ok(addr) = addr_str.parse::<libp2p::Multiaddr>() {
                    // Extraire le PeerId de la multiaddr (dernier composant /p2p/<id>)
                    use libp2p::multiaddr::Protocol;
                    let peer_id = addr.iter().find_map(|p| {
                        if let Protocol::P2p(id) = p { Some(id) } else { None }
                    });
                    if let Some(pid) = peer_id {
                        swarm.behaviour_mut().kademlia.add_address(&pid, addr.clone());
                        tracing::info!("Peer de fallback ajouté: {}", addr_str);
                    }
                }
            }
        }

        let (tx, mut rx) = mpsc::channel(32);

        tokio::spawn(async move {
            // Cache LRU pour déduplication des messages Gossipsub (anti-flood / anti-replay)
            let mut seen_messages: LruCache<String, ()> =
                LruCache::new(NonZeroUsize::new(DEDUP_CACHE_SIZE).unwrap());

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
                        SwarmEvent::Behaviour(WorldVPNBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                            message_id,
                            message,
                            ..
                        })) => {
                            let msg_id_str = message_id.to_string();

                            // Déduplication — ignorer les messages déjà traités
                            if seen_messages.contains(&msg_id_str) {
                                tracing::debug!("Message Gossipsub dupliqué ignoré: {}", msg_id_str);
                                continue;
                            }
                            seen_messages.put(msg_id_str, ());

                            // Traitement selon le topic
                            let topic_str = message.topic.to_string();
                            tracing::debug!("Gossipsub message reçu sur topic: {}", topic_str);

                            if topic_str.contains("credits") {
                                // Désérialiser et VALIDER la signature du CreditReceipt
                                match serde_json::from_slice::<CreditReceipt>(&message.data) {
                                    Ok(receipt) => {
                                        if receipt.verify() {
                                            tracing::info!(
                                                "✅ CreditReceipt valide reçu: session={}, bytes={}",
                                                receipt.session_id, receipt.bytes_total
                                            );
                                            // TODO: Transmettre au service de crédits via un channel dédié
                                        } else {
                                            tracing::warn!(
                                                "⚠️  CreditReceipt SIGNATURE INVALIDE — rejeté (consumer_pubkey: {})",
                                                receipt.consumer_pubkey
                                            );
                                        }
                                    }
                                    Err(e) => {
                                        tracing::warn!("⚠️  CreditReceipt désérialisation échouée: {}", e);
                                    }
                                }
                            }
                        },
                        _ => {}
                    },
                    cmd = rx.recv() => {
                        if let Some(command) = cmd {
                            match command {
                                P2pCommand::FindPeers(_criteria, reply) => {
                                    let _ = reply.send(vec![]); // TODO: requête DHT réelle
                                }
                                P2pCommand::Announce(_info, reply) => {
                                    let _ = reply.send(Ok(()));
                                }
                                P2pCommand::SubmitReceipt(receipt, reply) => {
                                    // Vérifier la signature AVANT de publier
                                    if !receipt.verify() {
                                        let _ = reply.send(Err(VpnError::P2pError(
                                            "CreditReceipt: signature invalide — publication refusée".into()
                                        )));
                                        continue;
                                    }
                                    let topic = gossipsub::IdentTopic::new("worldvpn/credits/v1");
                                    let data = serde_json::to_vec(&receipt).unwrap();
                                    let result = swarm.behaviour_mut().gossipsub.publish(topic, data)
                                        .map(|_| ())
                                        .map_err(|e| VpnError::P2pError(format!("Publish error: {e}")));
                                    let _ = reply.send(result);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::IdentityKey;

    fn make_signed_receipt(identity: &IdentityKey, bytes: u64) -> CreditReceipt {
        let session_id = "test-session-001".to_string();
        let consumer_pubkey = identity.public_key_hex();
        let provider_pubkey = "ed25519:aabbccdd".to_string();
        let timestamp = 1714512000i64;

        // Construire le payload canonique et signer
        let message = format!(
            "{}|{}|{}|{}|{}",
            session_id, consumer_pubkey, provider_pubkey, bytes, timestamp
        );
        let signature = identity.sign_challenge(&message);

        CreditReceipt {
            session_id,
            consumer_pubkey,
            provider_pubkey,
            bytes_total: bytes,
            timestamp,
            signature,
        }
    }

    #[test]
    fn test_credit_receipt_signing_message() {
        let identity = IdentityKey::generate();
        let receipt = make_signed_receipt(&identity, 1024);
        let expected = format!(
            "{}|{}|{}|{}|{}",
            receipt.session_id,
            receipt.consumer_pubkey,
            receipt.provider_pubkey,
            receipt.bytes_total,
            receipt.timestamp,
        );
        assert_eq!(receipt.signing_message(), expected);
    }

    #[test]
    fn test_credit_receipt_valid_signature() {
        let identity = IdentityKey::generate();
        let receipt = make_signed_receipt(&identity, 2048);
        assert!(receipt.verify(), "Signature valide doit être acceptée");
    }

    #[test]
    fn test_credit_receipt_invalid_signature() {
        let identity = IdentityKey::generate();
        let mut receipt = make_signed_receipt(&identity, 2048);
        // Falsifier les bytes pour invalider la signature
        receipt.bytes_total = 999999;
        assert!(!receipt.verify(), "Signature invalide doit être rejetée");
    }

    #[test]
    fn test_credit_receipt_tampered_pubkey() {
        let identity = IdentityKey::generate();
        let mut receipt = make_signed_receipt(&identity, 512);
        // Remplacer la clé publique par une autre identité
        let other = IdentityKey::generate();
        receipt.consumer_pubkey = other.public_key_hex();
        assert!(!receipt.verify(), "Clé publique incorrecte doit être rejetée");
    }

    #[test]
    fn test_credit_receipt_serialization() {
        let identity = IdentityKey::generate();
        let receipt = make_signed_receipt(&identity, 4096);
        let json = serde_json::to_vec(&receipt).expect("Sérialisation JSON doit réussir");
        let restored: CreditReceipt = serde_json::from_slice(&json).expect("Désérialisation JSON doit réussir");
        assert_eq!(receipt.session_id, restored.session_id);
        assert_eq!(receipt.bytes_total, restored.bytes_total);
        // La signature doit rester valide après sérialisation/désérialisation
        assert!(restored.verify(), "Signature doit être valide après round-trip JSON");
    }
}
