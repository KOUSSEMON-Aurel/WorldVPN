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
    static ref PRIVATE_KEY: Mutex<Option<Vec<u8>>> = Mutex::new(None);
    static ref IS_SHARING: Mutex<bool> = Mutex::new(false);
    static ref SHARING_TASK: Mutex<Option<tokio::task::JoinHandle<()>>> = Mutex::new(None);
    static ref BACKEND_URL: Mutex<String> = Mutex::new("http://localhost:3000".to_string());
    static ref ACTIVE_TUNNEL: Mutex<Option<Box<dyn crate::tunnel::VpnTunnel>>> = Mutex::new(None);
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

pub fn generate_identity() -> anyhow::Result<Vec<u8>> {
    let identity = crate::crypto::IdentityKey::generate();
    let bytes = identity.to_bytes()?.to_vec();
    Ok(bytes)
}

pub fn login_anonymously(private_key_bytes: Vec<u8>) -> anyhow::Result<String> {
    tracing::info!("login_anonymously called");
    
    let identity = crate::crypto::IdentityKey::from_bytes(&private_key_bytes)?;
    let public_key = identity.public_key_hex();
    
    // Save private key for E2EE decryption in future matchmaking calls
    if let Ok(mut key_guard) = PRIVATE_KEY.lock() {
        *key_guard = Some(private_key_bytes.clone());
    }

    // Create challenge payload
    let timestamp = chrono::Utc::now().timestamp().to_string();
    let signature = identity.sign_challenge(&timestamp);

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let client = VpnApiClient::new(get_backend_url());
        let response = client.login_with_identity(public_key.clone(), signature, timestamp).await?;
        
        if let Ok(mut token_guard) = AUTH_TOKEN.lock() {
            *token_guard = Some(response.token.clone());
        }
        if let Ok(mut user_guard) = LOGGED_USER.lock() {
            *user_guard = Some(public_key); // Use public_key as unique identifier locally
        }
        
        Ok(response.token)
    })
}

pub fn start_vpn_matchmaking(protocol_str: String, country_code: String, node_group: String) -> anyhow::Result<String> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        tracing::info!("Matchmaking: protocol={} country={} group={}", protocol_str, country_code, node_group);
        
        let token_opt = AUTH_TOKEN.lock().unwrap().clone();
        let user_opt = LOGGED_USER.lock().unwrap().clone();
        let key_opt = PRIVATE_KEY.lock().unwrap().clone();
        
        let token = token_opt.ok_or_else(|| anyhow::anyhow!("Not authenticated"))?;
        let username = user_opt.ok_or_else(|| anyhow::anyhow!("No user session"))?;
        let private_key_bytes = key_opt.ok_or_else(|| anyhow::anyhow!("Missing identity key"))?;
        
        let identity = crate::crypto::IdentityKey::from_bytes(&private_key_bytes)?;

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
            context.firewall_profile = FirewallProfile::Open;
        }
        
        let best_protocols = selector.build_connection_cascade(&context, is_symmetric);
        // Force the protocol selected in the UI if not Auto, otherwise use cascade
        let chosen_protocol = if protocol_str == "Auto" {
            best_protocols[0]
        } else {
            match protocol_str.as_str() {
                "WireGuard" => crate::protocol::VpnProtocol::WireGuard,
                "Hysteria 2" => crate::protocol::VpnProtocol::Hysteria2,
                "Shadowsocks" => crate::protocol::VpnProtocol::Shadowsocks,
                "Trojan" => crate::protocol::VpnProtocol::Trojan,
                "VLESS" => crate::protocol::VpnProtocol::VLESS,
                _ => best_protocols[0],
            }
        };

        // 3. Backend Matchmaking
        let client = VpnApiClient::new(get_backend_url());
        let mut conn_info = client.connect(
            chosen_protocol, 
            Some(username.clone()), 
            Some(country_code), 
            &token
        ).await?;

        // 4. E2EE Decryption
        if conn_info.server_endpoint.starts_with("e2e:") {
            tracing::info!("Decrypting E2E endpoint...");
            let encrypted_b64 = &conn_info.server_endpoint[4..];
            let decrypted = identity.decrypt_with_identity(encrypted_b64)?;
            tracing::info!("Decryption OK: {}", decrypted);
            conn_info.server_endpoint = decrypted;
        }

        tracing::info!("Matchmaking OK: endpoint={}", conn_info.server_endpoint);
        
        // Build JSON for Go's WorldVpnService
        let mut map = serde_json::Map::new();
        map.insert("session_id".into(),      serde_json::Value::String(conn_info.session_id.clone()));
        map.insert("server_endpoint".into(), serde_json::Value::String(conn_info.server_endpoint.clone()));
        map.insert("peer_endpoint".into(),   serde_json::Value::String(conn_info.server_endpoint.clone()));
        map.insert("assigned_ip".into(),     serde_json::Value::String(conn_info.assigned_ip.clone()));
        map.insert("protocol".into(),        serde_json::Value::String(format!("{:?}", chosen_protocol)));
        if let Some(key) = conn_info.server_public_key {
            map.insert("peer_public_key".into(), serde_json::Value::String(key));
        }
        map.insert("dns".into(), serde_json::Value::String("1.1.1.1".into()));
        map.insert("mtu".into(), serde_json::Value::Number(1420.into()));
        
        let config_json = serde_json::to_string(&serde_json::Value::Object(map))?;
        
        #[cfg(target_os = "linux")]
        {
            let tunnel = crate::tunnel::go_tunnel::GoTunnel::new(conn_info.session_id.clone(), chosen_protocol);
            let _config = crate::tunnel::ConnectionConfig {
                protocol: chosen_protocol,
                server_addr: conn_info.server_endpoint.parse()?,
                credentials: crate::tunnel::Credentials::Password { 
                    username: Some(username), 
                    password: "".to_string() // Password mapping depends on protocol
                },
                timeout: std::time::Duration::from_secs(10),
            };
            
            // Note: assigned_ip and other details are already in map/config_json
            crate::tunnel::go_bridge::GoBridge::start_tunnel(0, &config_json)?;
            
            if let Ok(mut guard) = ACTIVE_TUNNEL.lock() {
                *guard = Some(Box::new(tunnel));
            }
        }

        send_status("Connected", &format!("{:?}", chosen_protocol), 0.0, 0.0);
        Ok(config_json)
    })
}

pub fn start_sharing() -> anyhow::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        tracing::info!("Mobile request: Start sharing bandwidth (Node Provider)");
        
        let token_opt = AUTH_TOKEN.lock().unwrap().clone();
        let user_opt = LOGGED_USER.lock().unwrap().clone();
        
        let token = token_opt.ok_or_else(|| anyhow::anyhow!("Not authenticated"))?;
        let _username = user_opt.ok_or_else(|| anyhow::anyhow!("No user session"))?;

        // 1. Detect NAT & Public Endpoint
        let nat_config = crate::nat::NatConfig::default();
        let nat_traversal = crate::nat::NatTraversal::new(nat_config);
        let nat_type = nat_traversal.detect_nat_type().await.unwrap_or(crate::nat::NatType::Unknown);
        
        // Discover our public XOR-mapped address for signaling
        let public_endpoint = nat_traversal.get_public_endpoint().await.ok().map(|e| e.to_string());
        
        tracing::info!("NAT Type: {:?}, Public Endpoint: {:?}", nat_type, public_endpoint);

        // 2. Register on Backend
        let client = VpnApiClient::new(get_backend_url());
        let _node_id = client.register_node(
            &token, 
            "BJ", // User country
            None, 
            format!("{:?}", nat_type), 
            public_endpoint.clone(), 
            vec!["WireGuard".to_string()]
        ).await?;
        
        if let Ok(mut sharing_guard) = IS_SHARING.lock() {
            *sharing_guard = true;
        }

        // 3. Start Heartbeat Loop
        let backend_url = get_backend_url();
        let nat_type_str = format!("{:?}", nat_type);
        let token_for_heartbeat = token.clone();
        let endpoint_for_heartbeat = public_endpoint.clone();
        
        let handle = tokio::spawn(async move {
            let inner_client = VpnApiClient::new(backend_url);
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                if !*IS_SHARING.lock().unwrap() { break; }
                
                let _ = inner_client.heartbeat(
                    &token_for_heartbeat, 
                    Some(nat_type_str.clone()), 
                    Some(0), 
                    endpoint_for_heartbeat.clone()
                ).await;
            }
        });

        if let Ok(mut task_guard) = SHARING_TASK.lock() {
            *task_guard = Some(handle);
        }

        Ok(())
    })
}

pub fn stop_sharing() -> anyhow::Result<()> {
    tracing::info!("Mobile request: Stop sharing");
    if let Ok(mut sharing_guard) = IS_SHARING.lock() {
        *sharing_guard = false;
    }
    // Task will terminate itself on next loop iteration or we could abort it
    if let Ok(mut task_guard) = SHARING_TASK.lock() {
        if let Some(handle) = task_guard.take() {
            handle.abort();
        }
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
    
    // Stop Go tunnel via bridge
    crate::tunnel::go_bridge::GoBridge::stop_tunnel()?;
    
    if let Ok(mut guard) = ACTIVE_TUNNEL.lock() {
        *guard = None;
    }
    
    send_status("Disconnected", "", 0.0, 0.0);
    Ok(())
}

pub fn register_status_stream(sink: StreamSink<VpnStatusEvent>) -> anyhow::Result<()> {
    if let Ok(mut stream_guard) = (*STATUS_STREAM).lock() {
        *stream_guard = Some(sink);
    }
    Ok(())
}

pub fn get_wallet_balance() -> anyhow::Result<i64> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let token_opt = AUTH_TOKEN.lock().unwrap().clone();
        if let Some(token) = token_opt {
            let client = VpnApiClient::new(get_backend_url());
            Ok(client.fetch_balance(&token).await.unwrap_or(0))
        } else {
            Ok(0)
        }
    })
}
