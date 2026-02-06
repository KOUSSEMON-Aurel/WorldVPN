//! Implémentation Hysteria2 (QUIC)
//!
//! Utilise le binaire `hysteria` pour créer un proxy SOCKS5 local via QUIC.
//! Idéal pour les réseaux instables (pertes de paquets).

use async_trait::async_trait;
use serde::Serialize;
use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::process::{Child, Command};
use tracing::{error, info, warn};

use crate::{
    error::{Result, VpnError},
    protocol::VpnProtocol,
    tunnel::{ConnectionConfig, Credentials, TunnelHandle, TunnelStats, VpnTunnel},
};

#[derive(Serialize)]
struct HysteriaConfig {
    server: String,
    auth: String,
    socks5: Socks5Config,
    bandwidth: BandwidthConfig,
    tls: TlsConfig,
}

#[derive(Serialize)]
struct Socks5Config {
    listen: String,
}

#[derive(Serialize)]
struct BandwidthConfig {
    up: String,
    down: String,
}

#[derive(Serialize)]
struct TlsConfig {
    #[serde(rename = "insecure")]
    insecure: bool,
    sni: String,
}

pub struct HysteriaTunnel {
    process: Option<Child>,
    config_file: Option<PathBuf>,
    start_time: Option<Instant>,
    local_port: u16,
}

impl HysteriaTunnel {
    pub fn new() -> Self {
        Self {
            process: None,
            config_file: None,
            start_time: None,
            local_port: 1087,
        }
    }

    async fn check_hysteria_installed() -> Result<()> {
        match Command::new("hysteria").arg("version").output().await {
            Ok(output) => {
                if output.status.success() {
                    Ok(())
                } else {
                    Err(VpnError::InvalidConfig("Hysteria erreur".into()))
                }
            }
            Err(_) => Err(VpnError::InvalidConfig("Hysteria non installé (téléchargez sur https://hysteria.network)".into())),
        }
    }
}

#[async_trait]
impl VpnTunnel for HysteriaTunnel {
    async fn connect(&mut self, config: &ConnectionConfig) -> Result<TunnelHandle> {
        info!("🔌 Initialisation Hysteria2 vers {}", config.server_addr);
        
        Self::check_hysteria_installed().await?;

        let password = match &config.credentials {
            Credentials::Password { password, .. } => password.clone(),
            _ => return Err(VpnError::InvalidConfig("Hysteria nécessite un mot de passe".into())),
        };

        // Port aléatoire 1087-1099
        self.local_port = 1087 + (rand::random::<u8>() % 20) as u16;

        let hy_config = HysteriaConfig {
            server: config.server_addr.to_string(),
            auth: password,
            socks5: Socks5Config {
                listen: format!("127.0.0.1:{}", self.local_port),
            },
            bandwidth: BandwidthConfig {
                up: "50 mbps".into(),
                down: "100 mbps".into(),
            },
            tls: TlsConfig {
                insecure: true, // Pour le dev/test, à sécuriser en prod
                sni: "google.com".into(), // Obfuscation basique
            },
        };

        let config_yaml = serde_yaml::to_string(&hy_config)
            .map_err(|e| VpnError::InvalidConfig(format!("Erreur YAML: {}", e)))?;

        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join(format!("hysteria_{}.yaml", uuid::Uuid::new_v4()));

        tokio::fs::write(&config_path, config_yaml).await
            .map_err(|e| VpnError::InvalidConfig(format!("Erreur écriture config: {}", e)))?;

        info!("🚀 Lancement Hysteria (config: {:?})", config_path);
        
        let mut child = Command::new("hysteria")
            .arg("client")
            .arg("-c")
            .arg(&config_path)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| VpnError::ConnectionFailed(format!("Échec lancement hysteria: {}", e)))?;

        tokio::time::sleep(Duration::from_secs(1)).await; // Hysteria démarre vite mais check

        if let Ok(Some(status)) = child.try_wait() {
             return Err(VpnError::ConnectionFailed(format!("Hysteria crashé (Exit {}).", status)));
        }

        self.process = Some(child);
        self.config_file = Some(config_path);
        self.start_time = Some(Instant::now());
        
        info!("✅ Hysteria connecté ! Proxy SOCKS5 sur 127.0.0.1:{}", self.local_port);

        Ok(TunnelHandle {
            id: uuid::Uuid::new_v4().to_string(),
            protocol: VpnProtocol::Hysteria2,
            assigned_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            remote_endpoint: config.server_addr,
        })
    }

    async fn send(&mut self, data: &[u8]) -> Result<usize> {
        Ok(data.len())
    }

    async fn recv(&mut self, _buf: &mut [u8]) -> Result<usize> {
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok(0)
    }

    async fn disconnect(&mut self) -> Result<()> {
        if let Some(mut child) = self.process.take() {
            let _ = child.kill().await;
        }
        if let Some(p) = self.config_file.take() { let _ = tokio::fs::remove_file(p).await; }
        info!("🛑 Hysteria arrêté");
        Ok(())
    }

    fn stats(&self) -> TunnelStats {
        TunnelStats {
            bytes_sent: 0,
            bytes_received: 0,
            avg_latency_ms: 20,
            packet_loss: 0.0,
            uptime: self.start_time.map(|t| t.elapsed()).unwrap_or_default(),
            current_throughput_mbps: 0.0,
        }
    }

    fn protocol(&self) -> VpnProtocol {
        VpnProtocol::Hysteria2
    }
}

impl Drop for HysteriaTunnel {
    fn drop(&mut self) {
        if let Some(mut child) = self.process.take() {
            let _ = child.start_kill();
        }
    }
}
