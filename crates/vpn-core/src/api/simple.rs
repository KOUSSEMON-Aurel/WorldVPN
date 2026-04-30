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
use crate::client::VpnApiClient;
use crate::selector::{ProtocolSelector, SelectionContext, NetworkQuality, FirewallProfile, DeviceType, UseCase};
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
    static ref AUTH_TOKEN: Mutex<Option<String>> = Mutex::new(None);
    static ref LOGGED_USER: Mutex<Option<String>> = Mutex::new(None);
    static ref IS_SHARING: Mutex<bool> = Mutex::new(false);
    static ref BACKEND_URL: Mutex<String> = Mutex::new("http://localhost:3000".to_string());
}

pub fn set_backend_url(url: String) {
    if let Ok(mut guard) = BACKEND_URL.lock() {
        *guard = url;
    }
}

fn get_backend_url() -> String {
    BACKEND_URL.lock().unwrap().clone()
}

pub fn greet(name: String) -> String {
    format!("Hello, {}! This message comes from your Rust backend 🦀", name)
}

pub fn login_user(username: String, password: String) -> anyhow::Result<String> {
    if username == "Guest" {
        if let Ok(mut token_guard) = AUTH_TOKEN.lock() {
            *token_guard = Some("guest_token".to_string());
        }
        if let Ok(mut user_guard) = LOGGED_USER.lock() {
            *user_guard = Some(username.clone());
        }
        return Ok("guest_token".to_string());
    }

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let client = VpnApiClient::new(get_backend_url());
        let response = client.login(username.clone(), password).await?;
        
        if let Ok(mut token_guard) = AUTH_TOKEN.lock() {
            *token_guard = Some(response.token.clone());
        }
        if let Ok(mut user_guard) = LOGGED_USER.lock() {
            *user_guard = Some(username);
        }
        
        Ok(response.token)
    })
}

pub fn start_vpn_connection(protocol_str: String, country_code: String) -> anyhow::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        tracing::info!("Mobile request: Connect {} to {}", protocol_str, country_code);
        
        let token_opt = AUTH_TOKEN.lock().unwrap().clone();
        let user_opt = LOGGED_USER.lock().unwrap().clone();
        
        let token = token_opt.ok_or_else(|| anyhow::anyhow!("Not authenticated"))?;
        let username = user_opt.ok_or_else(|| anyhow::anyhow!("No user session"))?;
        
        // 1. Notify UI
        send_status("Connecting", &protocol_str, 0.0, 0.0);

        // 2. Protocol Selection & NAT Detection
        let nat_config = crate::nat::NatConfig::default();
        let nat_traversal = crate::nat::NatTraversal::new(nat_config);
        let nat_type = nat_traversal.detect_nat_type().await.unwrap_or(crate::nat::NatType::Unknown);
        let is_symmetric = nat_type == crate::nat::NatType::Symmetric;

        let selector = ProtocolSelector::new();
        let mut context = SelectionContext {
            network_quality: NetworkQuality { latency_ms: 100, packet_loss: 0.0, bandwidth_mbps: 10.0, stability: 0.8 },
            firewall_profile: FirewallProfile::Residential,
            user_country: "BJ".to_string(), // In production: detect from IP
            device_type: DeviceType::Mobile,
            battery_level: Some(0.5),
            use_case: UseCase::Browsing,
        };

        if username == "Guest" {
            context.firewall_profile = FirewallProfile::Open; // Guest only accesses Open/Public relays
        }
        
        let best_protocols = selector.build_connection_cascade(&context, is_symmetric);
        let chosen_protocol = best_protocols[0];

        // 3. Backend Matchmaking (Skip for Guest)
        if username == "Guest" {
            tracing::info!("Guest connection bypassing backend matchmaking. Target: {:?}", chosen_protocol);
        } else {
            let client = VpnApiClient::new(get_backend_url());
            let conn_info = client.connect(chosen_protocol, username, None, &token).await?;
            tracing::info!("Connection successful to: {}", conn_info.server_endpoint);
        }
        
        // 4. Trigger Tunnel (Simulated for Now, Real FFI Tunneling following)
        send_status("Connected", &format!("{:?}", chosen_protocol), 1.2, 0.4);
        
        Ok(())
    })
}

pub fn start_sharing() -> anyhow::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        tracing::info!("Mobile request: Start sharing bandwidth (Node Provider)");
        
        let token_opt = AUTH_TOKEN.lock().unwrap().clone();
        let user_opt = LOGGED_USER.lock().unwrap().clone();
        
        let _token = token_opt.ok_or_else(|| anyhow::anyhow!("Not authenticated"))?;
        let _username = user_opt.ok_or_else(|| anyhow::anyhow!("No user session"))?;

        // 1. Detect NAT & Open Ports (UPnP Simulation)
        let nat_config = crate::nat::NatConfig::default();
        let nat_traversal = crate::nat::NatTraversal::new(nat_config);
        let nat_type = nat_traversal.detect_nat_type().await.unwrap_or(crate::nat::NatType::Unknown);
        
        tracing::info!("NAT Type detected for sharing: {:?}", nat_type);
        tracing::info!("UPnP: Requesting port mapping for WireGuard (51820) and Hysteria2 (44343)");

        // 2. Register on Backend
        let client = VpnApiClient::new(get_backend_url());
        // In a real impl, we'd use reqwest directly here or extend VpnApiClient
        // For now, we simulate the registration
        tracing::info!("Registering node on backend: country=BJ, nat={:?}", nat_type);
        
        if let Ok(mut sharing_guard) = IS_SHARING.lock() {
            *sharing_guard = true;
        }

        Ok(())
    })
}

pub fn stop_sharing() -> anyhow::Result<()> {
    tracing::info!("Mobile request: Stop sharing");
    if let Ok(mut sharing_guard) = IS_SHARING.lock() {
        *sharing_guard = false;
    }
    Ok(())
}

pub fn is_sharing() -> bool {
    *IS_SHARING.lock().unwrap()
}

fn send_status(status: &str, protocol: &str, dl: f64, ul: f64) {
    if let Ok(stream_guard) = (*STATUS_STREAM).lock() {
        if let Some(sink) = stream_guard.as_ref() {
            let _ = sink.add(VpnStatusEvent {
                status: status.to_string(),
                protocol: protocol.to_string(),
                download_speed: dl,
                upload_speed: ul,
            });
        }
    }
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
