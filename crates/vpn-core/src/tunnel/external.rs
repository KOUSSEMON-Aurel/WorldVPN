use crate::error::Result;
use crate::protocol::VpnProtocol;
use crate::tunnel::{ConnectionConfig, TunnelHandle, TunnelStats, VpnTunnel, Credentials};
use crate::binary_manager::{BinaryManager, get_binary_specs};
use async_trait::async_trait;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Generic tunnel implementation that wraps an external binary (Shadowsocks, Hysteria2, etc.)
pub struct ExternalTunnel {
    protocol: VpnProtocol,
    child: Option<Child>,
    start_time: Option<Instant>,
}

impl ExternalTunnel {
    pub fn new(protocol: VpnProtocol) -> Self {
        Self {
            protocol,
            child: None,
            start_time: None,
        }
    }

    fn build_command(&self, config: &ConnectionConfig) -> Result<Command> {
        let manager = BinaryManager::new()?;
        let bin_name = match self.protocol {
            VpnProtocol::Shadowsocks => "sslocal",
            VpnProtocol::Hysteria2 => "hysteria",
            VpnProtocol::Trojan | VpnProtocol::VLESS => "v2ray",
            _ => return Err(crate::error::VpnError::Internal("Protocol not supported via external binary".into())),
        };

        let bin_path = manager.get_binary_path(bin_name)
            .ok_or_else(|| crate::error::VpnError::InvalidConfig(format!("Binary {} not found", bin_name)))?;

        let mut cmd = Command::new(bin_path);
        
        match self.protocol {
            VpnProtocol::Shadowsocks => {
                if let Credentials::Password { password, .. } = &config.credentials {
                    cmd.args(["-s", &config.server_addr.to_string(), "-b", "127.0.0.1:1086", "-m", "chacha20-ietf-poly1305", "-k", password]);
                }
            },
            VpnProtocol::Hysteria2 => {
                // Simplified hysteria command
                cmd.args(["client", "--server", &config.server_addr.to_string()]);
            },
            _ => {}
        }

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        Ok(cmd)
    }
}

#[async_trait]
impl VpnTunnel for ExternalTunnel {
    async fn connect(&mut self, config: &ConnectionConfig) -> Result<TunnelHandle> {
        let mut cmd = self.build_command(config)?;
        let child = cmd.spawn().map_err(|e| crate::error::VpnError::ConnectionFailed(format!("Failed to spawn process: {}", e)))?;
        
        self.child = Some(child);
        self.start_time = Some(Instant::now());

        Ok(TunnelHandle {
            id: format!("{}-local", self.protocol.name()),
            protocol: self.protocol,
            assigned_ip: "127.0.0.1".parse().unwrap(), // Proxy mode usually
            remote_endpoint: config.server_addr,
        })
    }

    async fn send(&mut self, _data: &[u8]) -> Result<usize> {
        Ok(0) // Logic handled by external binary
    }

    async fn recv(&mut self, _buf: &mut [u8]) -> Result<usize> {
        Ok(0)
    }

    async fn disconnect(&mut self) -> Result<()> {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
        }
        Ok(())
    }

    fn stats(&self) -> TunnelStats {
        TunnelStats {
            bytes_sent: 0,
            bytes_received: 0,
            avg_latency_ms: 0,
            packet_loss: 0.0,
            uptime: self.start_time.map(|t| t.elapsed()).unwrap_or(Duration::ZERO),
            current_throughput_mbps: 0.0,
        }
    }

    fn protocol(&self) -> VpnProtocol {
        self.protocol
    }
}
