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
    binary_manager::BinaryManager,
};

/// Config format for the `sslocal` subprocess (Standard JSON)
#[derive(Serialize)]
struct ShadowsocksConfig {
    server: String,
    server_port: u16,
    local_address: String,
    local_port: u16,
    password: String,
    method: String,
    timeout: u64,
}

/// Tunnel implementation using shadowsocks-rust's `sslocal` binary
pub struct ShadowsocksTunnel {
    process: Option<Child>,
    config_file: Option<PathBuf>,
    bytes_sent: u64,
    bytes_received: u64,
    start_time: Option<Instant>,
    local_port: u16,
}

impl ShadowsocksTunnel {
    pub fn new() -> Self {
        Self {
            process: None,
            config_file: None,
            bytes_sent: 0,
            bytes_received: 0,
            start_time: None,
            local_port: 1080,
        }
    }

    /// Generates a unique temporary config file for the Shadowsocks client
    async fn create_config_file(&self, config: &ShadowsocksConfig) -> Result<PathBuf> {
        let config_json = serde_json::to_string_pretty(config)
            .map_err(|e| VpnError::InvalidConfig(format!("JSON serialization error: {}", e)))?;

        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join(format!("ss_config_{}.json", uuid::Uuid::new_v4()));

        tokio::fs::write(&config_path, config_json)
            .await
            .map_err(|e| VpnError::InvalidConfig(format!("Failed to write config file: {}", e)))?;

        info!("Shadowsocks config created: {:?}", config_path);
        Ok(config_path)
    }
}

#[async_trait]
impl VpnTunnel for ShadowsocksTunnel {
    async fn connect(&mut self, config: &ConnectionConfig) -> Result<TunnelHandle> {
        info!("Initializing Shadowsocks to {}", config.server_addr);

        let bin_manager = BinaryManager::new().map_err(|e| VpnError::InvalidConfig(e.to_string()))?;
        
        // Ensure sslocal binary is present
        if !bin_manager.is_installed("sslocal").await {
            info!("sslocal not found, attempting auto-install...");
            let specs = crate::binary_manager::get_binary_specs();
            if let Some(spec) = specs.iter().find(|s| s.name == "sslocal") {
                bin_manager.auto_install(spec).await.map_err(|e| 
                    VpnError::ConnectionFailed(format!("Failed to install sslocal: {}", e))
                )?;
            } else {
                return Err(VpnError::InvalidConfig("sslocal binary spec not found".into()));
            }
        }

        let bin_path = bin_manager.get_binary_path("sslocal").ok_or_else(|| 
            VpnError::InvalidConfig("Failed to locate sslocal after installation".into())
        )?;

        info!("Using binary: {:?}", bin_path);

        // Parse credentials formatted as "method:password"
        let (method, password) = match &config.credentials {
            Credentials::Password { password, .. } => {
                if password.contains(':') {
                    let parts: Vec<&str> = password.splitn(2, ':').collect();
                    (parts[0].to_string(), parts[1].to_string())
                } else {
                    ("chacha20-ietf-poly1305".to_string(), password.clone())
                }
            }
            _ => {
                return Err(VpnError::InvalidConfig(
                    "Password credentials required for Shadowsocks".into(),
                ))
            }
        };

        if method.is_empty() || password.is_empty() {
            return Err(VpnError::InvalidConfig("Empty method or password".into()));
        }

        // Assign a random local port for the SOCKS5 proxy
        self.local_port = 1080 + (rand::random::<u8>() % 20) as u16;

        let ss_config = ShadowsocksConfig {
            server: config.server_addr.ip().to_string(),
            server_port: config.server_addr.port(),
            local_address: "127.0.0.1".to_string(),
            local_port: self.local_port,
            password,
            method,
            timeout: 300,
        };

        let config_path = self.create_config_file(&ss_config).await?;

        // Spawn sslocal as a background process
        info!("Launching sslocal with config {:?}", config_path);
        let mut child = Command::new(bin_path)
            .arg("-c")
            .arg(&config_path)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| VpnError::ConnectionFailed(format!("Failed to launch sslocal: {}", e)))?;

        // Grace period for process startup
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Health check on the spawned process
        match child.try_wait() {
            Ok(Some(status)) => {
                return Err(VpnError::ConnectionFailed(format!(
                    "sslocal crashed on startup: {}",
                    status
                )));
            }
            Ok(None) => {}
            Err(e) => {
                warn!("Could not check sslocal status: {}", e);
            }
        }

        self.process = Some(child);
        self.config_file = Some(config_path);
        self.start_time = Some(Instant::now());

        info!(
            "Shadowsocks connected! Local SOCKS5 proxy on 127.0.0.1:{}",
            self.local_port
        );

        Ok(TunnelHandle {
            id: uuid::Uuid::new_v4().to_string(),
            protocol: VpnProtocol::Shadowsocks,
            assigned_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            remote_endpoint: config.server_addr,
        })
    }

    async fn send(&mut self, data: &[u8]) -> Result<usize> {
        // Traffic is managed by the SOCKS5 proxy, send/recv are just statistics here
        self.bytes_sent += data.len() as u64;
        Ok(data.len())
    }

    async fn recv(&mut self, _buf: &mut [u8]) -> Result<usize> {
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok(0)
    }

    async fn disconnect(&mut self) -> Result<()> {
        info!("Stopping Shadowsocks tunnel...");

        if let Some(mut process) = self.process.take() {
            let _ = process.kill().await;
            let _ = process.wait().await;
            info!("sslocal process terminated");
        }

        // Clean up temporary config
        if let Some(config_path) = self.config_file.take() {
            if let Err(e) = tokio::fs::remove_file(&config_path).await {
                error!("Error removing config {:?}: {}", config_path, e);
            } else {
                info!("Temporary config removed");
            }
        }

        self.start_time = None;
        Ok(())
    }

    fn stats(&self) -> TunnelStats {
        TunnelStats {
            bytes_sent: self.bytes_sent,
            bytes_received: self.bytes_received,
            avg_latency_ms: 150,
            packet_loss: 0.0,
            uptime: self.start_time.map(|t| t.elapsed()).unwrap_or_default(),
            current_throughput_mbps: 0.0,
        }
    }

    fn protocol(&self) -> VpnProtocol {
        VpnProtocol::Shadowsocks
    }
}

impl Drop for ShadowsocksTunnel {
    fn drop(&mut self) {
        // Safe process teardown if disconnect wasn't explicitly called
        if let Some(mut process) = self.process.take() {
            let _ = process.start_kill();
        }
    }
}
