//! Implémentation OpenVPN Subprocess
//!
//! Utilise le binaire `openvpn` système.
//! Nécessite les privilèges root (sudo) pour créer l'interface TUN,
//! sauf si lancé avec des capabilities spécifiques.

use async_trait::async_trait;
use std::net::{IpAddr, Ipv4Addr};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::process::{Child, Command};
use tracing::{error, info, warn};

use crate::{
    error::{Result, VpnError},
    protocol::VpnProtocol,
    tunnel::{ConnectionConfig, Credentials, TunnelHandle, TunnelStats, VpnTunnel},
};

/// Tunnel OpenVPN
pub struct OpenVpnTunnel {
    process: Option<Child>,
    config_file: Option<PathBuf>,
    auth_file: Option<PathBuf>,
    start_time: Option<Instant>,
    bytes_sent: u64,
    bytes_received: u64,
}

impl OpenVpnTunnel {
    pub fn new() -> Self {
        Self {
            process: None,
            config_file: None,
            auth_file: None,
            start_time: None,
            bytes_sent: 0,
            bytes_received: 0,
        }
    }

    async fn check_openvpn_installed() -> Result<()> {
        match Command::new("openvpn").arg("--version").output().await {
            Ok(output) => {
                if output.status.success() {
                    Ok(())
                } else {
                    Err(VpnError::InvalidConfig("openvpn erreur".into()))
                }
            }
            Err(_) => Err(VpnError::InvalidConfig("openvpn non installé".into())),
        }
    }

    async fn create_config_files(
        &self,
        config: &ConnectionConfig,
        username: &str,
        password: &str
    ) -> Result<(PathBuf, PathBuf)> {
        let temp_dir = std::env::temp_dir();
        let uuid = uuid::Uuid::new_v4();
        
        let config_path = temp_dir.join(format!("ovpn_{}.ovpn", uuid));
        let auth_path = temp_dir.join(format!("ovpn_{}.auth", uuid));

        // 1. Fichier Auth
        let auth_content = format!("{}\n{}", username, password);
        tokio::fs::write(&auth_path, auth_content).await
            .map_err(|e| VpnError::InvalidConfig(format!("Erreur écriture auth: {}", e)))?;

        // 2. Fichier Config (.ovpn)
        // Configuration minimale standard
        let proto = match config.protocol {
            VpnProtocol::OpenVpnUdp => "udp",
            _ => "tcp",
        };

        let ovpn_content = format!(
            "client\n\
            dev tun\n\
            proto {}\n\
            remote {} {}\n\
            resolv-retry infinite\n\
            nobind\n\
            persist-key\n\
            persist-tun\n\
            auth-user-pass {}\n\
            cipher AES-256-GCM\n\
            auth SHA256\n\
            verb 3\n\
            # Obfuscation basique si supporté par serveur\n\
            # scramble obfuscate password\n",
            proto,
            config.server_addr.ip(),
            config.server_addr.port(),
            auth_path.to_string_lossy()
        );

        tokio::fs::write(&config_path, ovpn_content).await
            .map_err(|e| VpnError::InvalidConfig(format!("Erreur écriture config: {}", e)))?;

        Ok((config_path, auth_path))
    }
}

#[async_trait]
impl VpnTunnel for OpenVpnTunnel {
    async fn connect(&mut self, config: &ConnectionConfig) -> Result<TunnelHandle> {
        info!("🔌 Initialisation OpenVPN vers {}", config.server_addr);
        
        Self::check_openvpn_installed().await?;

        let (username, password) = match &config.credentials {
            Credentials::Password { username: Some(u), password: p } => (u.clone(), p.clone()),
            _ => return Err(VpnError::InvalidConfig("OpenVPN nécessite username/password".into())),
        };

        let (config_path, auth_path) = self.create_config_files(config, &username, &password).await?;
        
        info!("🚀 Lancement openvpn (sudo requis pour TUN)...");
        
        // Note: Sur Linux, openvpn nécessite souvent root pour ouvrir /dev/net/tun
        // Dans une app desktop, on utiliserait pkexec ou un service helper.
        // Ici on tente l'appel direct (échouera si non-root sauf si cap_net_admin set)
        
        let mut child = Command::new("openvpn")
            .arg("--config")
            .arg(&config_path)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| VpnError::ConnectionFailed(format!("Échec lancement openvpn: {}", e)))?;

        // Wait to see if it crashes immediately
        tokio::time::sleep(Duration::from_secs(1)).await;
        
        if let Ok(Some(status)) = child.try_wait() {
             return Err(VpnError::ConnectionFailed(format!("OpenVPN crashé (Exit {}). Root requis ?", status)));
        }

        self.process = Some(child);
        self.config_file = Some(config_path);
        self.auth_file = Some(auth_path);
        self.start_time = Some(Instant::now());

        Ok(TunnelHandle {
            id: uuid::Uuid::new_v4().to_string(),
            protocol: config.protocol,
            assigned_ip: IpAddr::V4(Ipv4Addr::new(10, 8, 0, 2)), // IP devinée
            remote_endpoint: config.server_addr,
        })
    }

    async fn send(&mut self, data: &[u8]) -> Result<usize> {
        self.bytes_sent += data.len() as u64;
        Ok(data.len())
    }

    async fn recv(&mut self, _buf: &mut [u8]) -> Result<usize> {
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok(0)
    }

    async fn disconnect(&mut self) -> Result<()> {
        if let Some(mut child) = self.process.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        
        // Cleanup files
        if let Some(p) = self.config_file.take() { let _ = tokio::fs::remove_file(p).await; }
        if let Some(p) = self.auth_file.take() { let _ = tokio::fs::remove_file(p).await; }
        
        info!("🛑 OpenVPN arrêté");
        Ok(())
    }

    fn stats(&self) -> TunnelStats {
        TunnelStats {
            bytes_sent: self.bytes_sent,
            bytes_received: self.bytes_received,
            avg_latency_ms: 50,
            packet_loss: 0.0,
            uptime: self.start_time.map(|t| t.elapsed()).unwrap_or_default(),
            current_throughput_mbps: 0.0,
        }
    }

    fn protocol(&self) -> VpnProtocol {
        VpnProtocol::OpenVpnTcp
    }
}

impl Drop for OpenVpnTunnel {
    fn drop(&mut self) {
        if let Some(mut child) = self.process.take() {
            let _ = child.start_kill();
        }
    }
}
