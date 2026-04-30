//! CLI de test WorldVPN
//!
//! Petit outil pour tester manuellement le core VPN et la sélection de protocole.

use clap::{Parser, Subcommand};
use std::net::SocketAddr;
use std::time::Duration;
use tracing::{info, warn, error};
use tracing_subscriber::EnvFilter;
use vpn_core::{
    crypto::SecretKey,
    selector::{ProtocolSelector, SelectionContext, NetworkQuality, FirewallProfile, DeviceType, UseCase},
    tunnel::{ConnectionConfig, Credentials, VpnTunnel},
    protocol::VpnProtocol,
    wireguard::WireGuardTunnel,
    openvpn::OpenVpnTunnel,
    mock::MockTunnel,
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
            let mut ctx = SelectionContext {
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
        Commands::Connect { proto, server } => {
            println!("⚠️ Mode simulation locale uniquement.");
        }
        Commands::RemoteConnect { api, user, proto } => {
            let protocol = match proto.to_lowercase().as_str() {
                "wg" | "wireguard" => VpnProtocol::WireGuard,
                "ss" | "shadowsocks" => VpnProtocol::Shadowsocks,
                "ovpn" | "openvpn" => VpnProtocol::OpenVpnTcp,
                "ovpn-udp" => VpnProtocol::OpenVpnUdp,
                "ikev2" | "ipsec" => VpnProtocol::IKEv2,
                "hy2" | "hysteria" => VpnProtocol::Hysteria2,
                "trojan" => VpnProtocol::Trojan,
                "vless" | "v2ray" => VpnProtocol::VLESS,
                _ => {
                    error!("Protocole inconnu '{}', utilisation WireGuard défaut", proto);
                    VpnProtocol::WireGuard
                }
            };

            println!("🌍 Connexion au serveur WorldVPN ({}) via {}", api, protocol.name());
            
            // 1. Login pour obtenir le JWT
            println!("🔐 Authentification...");
            let client = vpn_core::client::VpnApiClient::new(api.clone());
            
            let login_response = match client.login(user.clone(), user.clone()).await {
                Ok(r) => r,
                Err(e) => {
                    println!("❌ Erreur Login: {}", e);
                    println!("💡 L'utilisateur n'existe peut-être pas. Utilisez /auth/register d'abord.");
                    return Ok(());
                }
            };
            println!("✅ Authentification réussie !");

            // 2. Connexion VPN avec le token
            println!("\n🔌 Demande de connexion VPN...");
            // Initialisation de la session
            let session = match client.connect(
                protocol,
                user, 
                Some("pubkey_placeholder".into()),
                &login_response.token
            ).await {
                Ok(s) => s,
                Err(e) => {
                    println!("❌ Erreur API: {}", e);
                    return Ok(());
                }
            };

            println!("🔑 Session obtenue ! ID: {}", session.session_id);
            println!("   🎯 Endpoint: {}", session.server_endpoint);
            if let Some(ref creds) = session.server_public_key {
                println!("   🔑 Credentials: {}", creds);
            }

            // 3. Initialisation du Tunnel
            let server_addr: SocketAddr = session.server_endpoint.parse().expect("Adresse invalide");

            // Configuration Credentials selon protocole
            let credentials = match protocol {
                VpnProtocol::Shadowsocks => {
                    let pwd = session.server_public_key.unwrap_or("chacha20-ietf-poly1305:secret".into());
                    Credentials::Password { username: None, password: pwd }
                },
                _ => {
                    let key = SecretKey::generate(32).unwrap();
                    let peer_key = SecretKey::generate(32).unwrap();
                    Credentials::KeyPair {
                        private_key: key.as_bytes().to_vec(),
                        peer_public_key: peer_key.as_bytes().to_vec(),
                    }
                }
            };

            let config = ConnectionConfig {
                protocol,
                server_addr,
                credentials,
                timeout: Duration::from_secs(10),
            };

            // Création du tunnel abstrait
            // Instanciation tunnel
            let mut tunnel: Box<dyn VpnTunnel> = match protocol {
                VpnProtocol::Shadowsocks => Box::new(vpn_core::shadowsocks::ShadowsocksTunnel::new()),
                VpnProtocol::WireGuard | VpnProtocol::WireGuardObfuscated => Box::new(WireGuardTunnel::new()),
                VpnProtocol::OpenVpnTcp | VpnProtocol::OpenVpnUdp => Box::new(OpenVpnTunnel::new()),
                VpnProtocol::IKEv2 => Box::new(vpn_core::ikev2::IKEv2Tunnel::new()),
                VpnProtocol::Hysteria2 => Box::new(vpn_core::hysteria::HysteriaTunnel::new()),
                VpnProtocol::Trojan => Box::new(vpn_core::v2ray::V2RayTunnel::new(VpnProtocol::Trojan)),
                VpnProtocol::VLESS => Box::new(vpn_core::v2ray::V2RayTunnel::new(VpnProtocol::VLESS)),
                _ => Box::new(WireGuardTunnel::new()),
            };
            println!("\n🔌 Initialisation du tunnel {}...", protocol.name());
            
            match tunnel.connect(&config).await {
                Ok(handle) => {
                    println!("✅ TUNNEL ÉTABLI avec succès !");
                    
                    if protocol == VpnProtocol::Shadowsocks {
                         println!("   🚀 Proxy SOCKS5 local actif sur le port 1086");
                         println!("   Configurez votre navigateur/système pour utiliser 127.0.0.1:1086");
                    } else {
                         println!("   • Interface locale : {}", handle.assigned_ip);
                    }
                    
                    if let Err(e) = tunnel.send(b"Ping").await {
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
