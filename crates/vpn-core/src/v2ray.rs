//! Implémentation V2Ray / Xray (VLESS, Trojan, VMess)
//!
//! Utilise le binaire `v2ray` pour contourner la censure avancée (GFW).
//! Proxy SOCKS5 local.

use async_trait::async_trait;
use serde_json::json; // Utile pour générer le JSON complexe
use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::process::{Child, Command};
use tracing::{info}; // warn, error removed as unused

use crate::{
    error::{Result, VpnError},
    protocol::VpnProtocol,
    tunnel::{ConnectionConfig, Credentials, TunnelHandle, TunnelStats, VpnTunnel},
};

pub struct V2RayTunnel {
    process: Option<Child>,
    config_file: Option<PathBuf>,
    start_time: Option<Instant>,
    local_port: u16,
    protocol_type: VpnProtocol, // Trojan ou VLESS
}

impl V2RayTunnel {
    pub fn new(protocol_type: VpnProtocol) -> Self {
        Self {
            process: None,
            config_file: None,
            start_time: None,
            local_port: 1088,
            protocol_type,
        }
    }

    async fn check_v2ray_installed() -> Result<()> {
        // Supporte v2ray ou xray
        if Command::new("v2ray").arg("version").output().await.is_ok() {
            return Ok(());
        }
        if Command::new("xray").arg("version").output().await.is_ok() {
            return Ok(());
        }
        Err(VpnError::InvalidConfig("v2ray/xray non installé".into()))
    }
}

#[async_trait]
impl VpnTunnel for V2RayTunnel {
    async fn connect(&mut self, config: &ConnectionConfig) -> Result<TunnelHandle> {
        info!("🔌 Initialisation V2Ray ({:?}) vers {}", self.protocol_type, config.server_addr);
        
        Self::check_v2ray_installed().await?;

        // Port aléatoire
        self.local_port = 1088 + (rand::random::<u8>() % 20) as u16;

        let uuid_or_pass = match &config.credentials {
            Credentials::Password { password, .. } => password.clone(),
            _ => return Err(VpnError::InvalidConfig("V2Ray nécessite un UUID/Password".into())),
        };

        // Construction Config V2Ray
        // Note: Trojan et VLESS ont des structures proches mais différentes
        let outbound_settings = match self.protocol_type {
            VpnProtocol::Trojan => json!({
                "servers": [{
                    "address": config.server_addr.ip().to_string(),
                    "port": config.server_addr.port(),
                    "password": [uuid_or_pass],
                }]
            }),
            _ => json!({ // Default VLESS
                "vnext": [{
                    "address": config.server_addr.ip().to_string(),
                    "port": config.server_addr.port(),
                    "users": [{
                        "id": uuid_or_pass,
                        "encryption": "none"
                    }]
                }]
            }),
        };
        
        // Protocol name string
        let proto_name = match self.protocol_type {
            VpnProtocol::Trojan => "trojan",
            _ => "vless",
        };

        let v2ray_config = json!({
            "log": { "loglevel": "warning" },
            "inbounds": [{
                "port": self.local_port,
                "protocol": "socks",
                "settings": { "auth": "noauth" },
                "sniffing": { "enabled": true, "destOverride": ["http", "tls"] }
            }],
            "outbounds": [{
                "protocol": proto_name,
                "settings": outbound_settings,
                "streamSettings": {
                    "network": "tcp", // ou ws
                    "security": "tls",
                    "tlsSettings": {
                        "serverName": "google.com", // Fake SNI
                        "allowInsecure": true
                    }
                }
            }]
        });

        let config_str = serde_json::to_string_pretty(&v2ray_config)
            .map_err(|e| VpnError::InvalidConfig(format!("JSON Error: {}", e)))?;

        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join(format!("v2ray_{}.json", uuid::Uuid::new_v4()));

        tokio::fs::write(&config_path, config_str).await
            .map_err(|e| VpnError::InvalidConfig(format!("Erreur écriture config: {}", e)))?;

        // Lancement (essaie v2ray puis xray)
        let bin_name = if Command::new("v2ray").arg("version").output().await.is_ok() { "v2ray" } else { "xray" };
        
        info!("🚀 Lancement {} (config: {:?})", bin_name, config_path);
        
        // v2ray run -c config.json
        let mut child = Command::new(bin_name)
            .arg("run")
            .arg("-c")
            .arg(&config_path)
            .stdout(std::process::Stdio::null()) 
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| VpnError::ConnectionFailed(format!("Échec lancement v2ray: {}", e)))?;

        tokio::time::sleep(Duration::from_millis(500)).await;

        if let Ok(Some(status)) = child.try_wait() {
             return Err(VpnError::ConnectionFailed(format!("V2Ray crashé (Exit {}).", status)));
        }

        self.process = Some(child);
        self.config_file = Some(config_path);
        self.start_time = Some(Instant::now());
        
        info!("✅ V2Ray connecté ! Proxy SOCKS5 sur 127.0.0.1:{}", self.local_port);

        Ok(TunnelHandle {
            id: uuid::Uuid::new_v4().to_string(),
            protocol: self.protocol_type.clone(),
            assigned_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            remote_endpoint: config.server_addr,
        })
    }

    async fn send(&mut self, data: &[u8]) -> Result<usize> { Ok(data.len()) }
    async fn recv(&mut self, _buf: &mut [u8]) -> Result<usize> { tokio::time::sleep(Duration::from_millis(100)).await; Ok(0) }
    
    async fn disconnect(&mut self) -> Result<()> {
        if let Some(mut child) = self.process.take() { let _ = child.kill().await; }
        if let Some(p) = self.config_file.take() { let _ = tokio::fs::remove_file(p).await; }
        info!("🛑 V2Ray arrêté");
        Ok(())
    }

    fn stats(&self) -> TunnelStats {
        TunnelStats {
            bytes_sent: 0, bytes_received: 0, avg_latency_ms: 60, packet_loss: 0.0,
            uptime: self.start_time.map(|t| t.elapsed()).unwrap_or_default(),
            current_throughput_mbps: 0.0,
        }
    }

    fn protocol(&self) -> VpnProtocol {
        self.protocol_type.clone()
    }
}

impl Drop for V2RayTunnel {
    fn drop(&mut self) {
        if let Some(mut child) = self.process.take() { let _ = child.start_kill(); }
    }
}
