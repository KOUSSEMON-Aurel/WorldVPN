//! CLI de test WorldVPN
//!
//! Petit outil pour tester manuellement le core VPN et la sélection de protocole.

use clap::{Parser, Subcommand};
use std::net::SocketAddr;
use std::time::Duration;
use tracing::error;
use tracing_subscriber::EnvFilter;
use vpn_core::{
    crypto::SecretKey,
    selector::{ProtocolSelector, SelectionContext, NetworkQuality, FirewallProfile, DeviceType, UseCase},
    tunnel::{ConnectionConfig, Credentials, VpnTunnel, GoTunnel},
    protocol::VpnProtocol,
};

#[derive(Parser)]
#[command(name = "worldvpn-cli")]
#[command(about = "Outil de test pour WorldVPN Core", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Teste la sélection intelligente de protocole
    Select {
        #[arg(long, default_value = "FR")]
        country: String,
        #[arg(long)]
        censored: bool,
        #[arg(long)]
        mobile: bool,
        #[arg(long)]
        battery: Option<f32>,
    },
    /// Établit une connexion VPN simulée
    Connect {
        #[arg(long, default_value = "wireguard")]
        proto: String,
        #[arg(long, default_value = "127.0.0.1:51820")]
        server: String,
    },
    /// Connexion via le serveur API
    RemoteConnect {
        #[arg(long, default_value = "http://127.0.0.1:3000")]
        api: String,
        #[arg(long, default_value = "user_cli")]
        user: String,
        #[arg(long, default_value = "wireguard")]
        proto: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialisation logs
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("vpn_core=debug".parse()?))
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Select { country, censored, mobile, battery } => {
            let ctx = SelectionContext {
                user_country: country,
                network_quality: NetworkQuality {
                    latency_ms: 50,
                    packet_loss: 0.0,
                    bandwidth_mbps: 100.0,
                    stability: 1.0,
                },
                firewall_profile: if censored { FirewallProfile::Corporate } else { FirewallProfile::Residential }, // Correction enum names
                device_type: if mobile { DeviceType::Mobile } else { DeviceType::Desktop },
                use_case: UseCase::Browsing,
                battery_level: Some(battery.unwrap_or(1.0)),
            };
            
            println!("🔍 Analyse pour contexte:");
            println!("   📍 Pays: {}", ctx.user_country);
            println!("   🛡️ Pare-feu: {:?}", ctx.firewall_profile);
            
            let selector = ProtocolSelector::new();
            let best = selector.select_best_protocol(&ctx);
            
            println!("\n🏆 Protocole Recommandé: {} (Score: {:.2})", best.name(), best.performance_score());
            if best.is_anti_censorship() {
                println!("   🛡️ Mode anti-censure activé");
            }
        }
        Commands::Connect { proto: _, server: _ } => {
            println!("⚠️ Mode simulation locale uniquement.");
        }
        Commands::RemoteConnect { api, user: _, proto } => {
            let protocol = match proto.to_lowercase().as_str() {
                "wg" | "wireguard" => VpnProtocol::WireGuard,
                "ss" | "shadowsocks" => VpnProtocol::Shadowsocks,
                "hy2" | "hysteria" => VpnProtocol::Hysteria2,
                _ => {
                    error!("Protocole inconnu '{}', utilisation WireGuard défaut", proto);
                    VpnProtocol::WireGuard
                }
            };

            println!("🌍 Connexion au serveur WorldVPN ({}) via {}", api, protocol.name());
            
            // 1. Identité Anonyme (V2) - Persistance locale
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            let id_path = std::path::PathBuf::from(home).join(".worldvpn").join("cli_id.der");
            
            let identity = if id_path.exists() {
                println!("🔐 Chargement de l'identité existante...");
                let bytes = std::fs::read(&id_path)?;
                vpn_core::crypto::IdentityKey::from_bytes(&bytes)?
            } else {
                println!("🔐 Génération d'une nouvelle identité anonyme...");
                let id = vpn_core::crypto::IdentityKey::generate();
                let bytes = id.to_bytes()?;
                std::fs::create_dir_all(id_path.parent().unwrap())?;
                std::fs::write(&id_path, &*bytes)?;
                id
            };

            let pubkey = identity.public_key_hex();
            let timestamp = chrono::Utc::now().timestamp().to_string();
            let signature = identity.sign_challenge(&timestamp);

            // 2. Login pour obtenir le JWT
            println!("🔐 Authentification via Ed25519...");
            let client = vpn_core::client::VpnApiClient::new(api.clone());
            
            let login_response = match client.login_with_identity(pubkey.clone(), signature, timestamp).await {
                Ok(r) => r,
                Err(e) => {
                    println!("❌ Erreur Authentification: {}", e);
                    return Ok(());
                }
            };
            println!("✅ Connecté avec succès ! (Anonyme)");

            // 3. Connexion VPN avec le token et PubKey pour E2EE
            println!("\n🔌 Demande de connexion VPN (Failover/P2P)...");
            let mut session = match client.connect(
                protocol,
                pubkey, 
                Some("FR".into()),
                &login_response.token
            ).await {
                Ok(s) => s,
                Err(e) => {
                    println!("❌ Erreur API Connexion: {}", e);
                    return Ok(());
                }
            };

            // Phase 4 : Déchiffrement E2E automatique si nécessaire
            if session.server_endpoint.starts_with("e2e:") {
                print!("🛡️  Endpoint chiffré reçu. Déchiffrement... ");
                let encrypted = &session.server_endpoint[4..];
                match identity.decrypt_with_identity(encrypted) {
                    Ok(dec) => {
                        session.server_endpoint = dec;
                        println!("✅");
                    },
                    Err(e) => {
                        println!("❌ Erreur: {}", e);
                        return Ok(());
                    }
                }
            }

            println!("🔑 Session obtenue ! ID: {}", session.session_id);
            println!("   🎯 Endpoint: {}", session.server_endpoint);
            if let Some(ref creds) = session.server_public_key {
                println!("   🔑 Credentials: {}", creds);
            }

            // 3. Initialisation du Tunnel
            // Initialisation du Tunnel
            let server_addr: SocketAddr = session.server_endpoint.parse().expect("Adresse invalide");

            // Configuration Credentials (KeyPair for WireGuard)
            let key = SecretKey::generate(32).unwrap();
            let peer_key = SecretKey::generate(32).unwrap();
            let credentials = Credentials::KeyPair {
                private_key: key.as_bytes().to_vec(),
                peer_public_key: peer_key.as_bytes().to_vec(),
            };

            let config = ConnectionConfig {
                protocol,
                server_addr,
                assigned_ip: session.assigned_ip.parse().unwrap_or("127.0.0.1".parse().unwrap()),
                credentials,
                timeout: Duration::from_secs(10),
            };

            // Création du tunnel réel (CLI rust)
            let session_id = "cli-test-session".to_string();
            let mut tunnel: Box<dyn VpnTunnel> = Box::new(GoTunnel::new(session_id, protocol.clone()));
            println!("\n🔌 Initialisation du tunnel {} (Réel)...", protocol.name());
            
            match tunnel.connect(&config).await {
                Ok(handle) => {
                    println!("✅ TUNNEL ÉTABLI avec succès !");
                    
                    if protocol == VpnProtocol::Shadowsocks {
                         println!("   🚀 Proxy SOCKS5 local actif sur le port 1086");
                         println!("   Configurez votre navigateur/système pour utiliser 127.0.0.1:1086");
                    } else {
                         println!("   • Interface locale : {}", handle.assigned_ip);
                    }
                    
                    if let Err(_e) = tunnel.send(b"Ping").await {
                        // En mode SOCKS, send n'envoie rien (simulation)
                        if protocol != VpnProtocol::Shadowsocks {
                             println!("⚠️  Note: L'envoi a échoué (normal sans serveur réel)");
                        }
                    }
                    
                    // Maintenir ouvert quelques secondes pour la démo
                    println!("⏳ Tunnel actif... (Ctrl+C pour arrêter)");
                    tokio::time::sleep(Duration::from_secs(10)).await;
                    
                    tunnel.disconnect().await?;
                },
                Err(e) => println!("❌ Erreur Tunnel: {}", e),
            }
        }
    }
    
    Ok(())
}
