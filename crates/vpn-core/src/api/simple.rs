use serde::{Deserialize, Serialize};

/// Represents the current state of a VPN connection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Disconnecting,
    Error,
}

use crate::frb_generated::StreamSink;
use std::sync::Mutex;
use lazy_static::lazy_static;

#[derive(Debug, Clone)]
pub struct MobileVpnState {
    pub status: ConnectionStatus,
    pub current_ip: Option<String>,
    pub bytes_up: u64,
    pub bytes_down: u64,
}

#[derive(Debug, Clone)]
pub struct VpnStatusEvent {
    pub status: String,
    pub protocol: String,
    pub download_speed: f64,
    pub upload_speed: f64,
}

lazy_static! {
    static ref STATUS_STREAM: Mutex<Option<StreamSink<VpnStatusEvent>>> = Mutex::new(None);
}

pub fn greet(name: String) -> String {
    format!("Hello, {}! This message comes from your Rust backend 🦀", name)
}

pub fn start_vpn_connection(protocol: String, country_code: String) -> anyhow::Result<()> {
    tracing::info!("Mobile request: Connect {} to {}", protocol, country_code);
    
    // Dereferencing lazy_static to get Mutex
    if let Ok(stream_guard) = (*STATUS_STREAM).lock() {
        if let Some(sink) = stream_guard.as_ref() {
            let _ = sink.add(VpnStatusEvent {
                status: "Connecting".to_string(),
                protocol: protocol.clone(),
                download_speed: 0.0,
                upload_speed: 0.0,
            });
        }
    }
    
    Ok(())
}

pub fn stop_vpn_connection() -> anyhow::Result<()> {
    tracing::info!("Mobile request: Disconnect");
    Ok(())
}

pub fn register_status_stream(sink: StreamSink<VpnStatusEvent>) -> anyhow::Result<()> {
    if let Ok(mut stream_guard) = (*STATUS_STREAM).lock() {
        *stream_guard = Some(sink);
    }
    Ok(())
}
