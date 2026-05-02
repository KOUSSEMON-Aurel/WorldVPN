use std::sync::{Arc, Mutex};
use tauri::State;
use serde::{Serialize, Deserialize};
use vpn_core::{
    crypto::IdentityKey,
    client::VpnApiClient,
    p2p::PeerDiscovery,
    error::VpnError,
};
use tokio::sync::OnceCell;
use vpn_core::tunnel::{VpnTunnel, GoTunnel, ConnectionConfig, Credentials};
use std::net::SocketAddr;

// Shared state to track VPN status across the app
struct AppState {
    vpn_status: Mutex<VpnStatus>,
    is_sharing: Mutex<bool>,
    p2p: OnceCell<Arc<PeerDiscovery>>,
    api_client: VpnApiClient,
    active_tunnel: Mutex<Option<Box<dyn VpnTunnel + Send>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Disconnecting,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2pStats {
    pub connected_peers: usize,
    pub known_nodes: usize,
    pub total_sent: u64,
    pub total_received: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VpnStatus {
    state: ConnectionState,
    current_ip: Option<String>,
    protocol: Option<String>,
    bytes_up: u64,
    bytes_down: u64,
    connected_since: Option<i64>,
    p2p_stats: Option<P2pStats>,
}

impl Default for VpnStatus {
    fn default() -> Self {
        Self {
            state: ConnectionState::Disconnected,
            current_ip: None,
            protocol: None,
            bytes_up: 0,
            bytes_down: 0,
            connected_since: None,
            p2p_stats: None,
        }
    }
}

#[tauri::command]
async fn generate_identity() -> Result<serde_json::Value, String> {
    let identity = IdentityKey::generate();
    let pub_key = identity.public_key_hex();
    let priv_bytes = identity.to_bytes().map_err(|e| e.to_string())?;
    
    Ok(serde_json::json!({
        "public_key": pub_key,
        "private_key": priv_bytes.to_vec(),
    }))
}

#[tauri::command]
async fn login_anonymously_desktop(
    private_key: Vec<u8>,
    state: State<'_, AppState>
) -> Result<serde_json::Value, String> {
    let identity = IdentityKey::from_bytes(&private_key).map_err(|e| e.to_string())?;
    
    // Handshake V2 : Signature du timestamp actuel
    let timestamp = chrono::Utc::now().timestamp().to_string();
    let signature = identity.sign_challenge(&timestamp);
    
    let response = state.api_client.login_with_identity(
        identity.public_key_hex(),
        signature,
        timestamp
    ).await.map_err(|e| e.to_string())?;
    
    Ok(serde_json::json!({
        "token": response.token,
        "user_id": response.user_id,
        "username": response.username,
    }))
}
#[tauri::command]
async fn migrate_credits_desktop(
    old_private_key: Vec<u8>,
    new_public_key: String,
    state: State<'_, AppState>
) -> Result<serde_json::Value, String> {
    let old_identity = IdentityKey::from_bytes(&old_private_key).map_err(|e| e.to_string())?;
    
    // Sign the new public key using the old identity
    let signature = old_identity.sign_challenge(&new_public_key);
    
    state.api_client.migrate_credits(
        old_identity.public_key_hex(),
        new_public_key,
        signature
    ).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn connect_vpn(
    protocol: String,
    country: String,
    token: String,
    private_key: Vec<u8>,
    state: State<'_, AppState>,
    #[allow(unused_variables)]
    app_handle: tauri::AppHandle,
) -> Result<VpnStatus, String> {
    let identity = IdentityKey::from_bytes(&private_key).map_err(|e| e.to_string())?;
    
    // 1. Update state to Connecting
    {
        let mut status = state.vpn_status.lock().map_err(|_| "Failed to lock state")?;
        status.state = ConnectionState::Connecting;
    }

    // 2. Perform matchmaking and connect
    let proto = match protocol.as_str() {
        "WireGuard" => vpn_core::protocol::VpnProtocol::WireGuard,
        "Hysteria 2" => vpn_core::protocol::VpnProtocol::Hysteria2,
        "Shadowsocks" => vpn_core::protocol::VpnProtocol::Shadowsocks,
        "Trojan" => vpn_core::protocol::VpnProtocol::Trojan,
        "VLESS" => vpn_core::protocol::VpnProtocol::VLESS,
        _ => vpn_core::protocol::VpnProtocol::WireGuard,
    };

    let mut info = state.api_client.connect(
        proto,
        Some(identity.public_key_hex()),
        Some(country),
        &token
    ).await.map_err(|e| e.to_string())?;
    
    // Phase 4 : Déchiffrement E2E automatique si nécessaire
    if info.server_endpoint.starts_with("e2e:") {
        let encrypted = &info.server_endpoint[4..];
        match identity.decrypt_with_identity(encrypted) {
            Ok(decrypted) => info.server_endpoint = decrypted,
            Err(e) => return Err(format!("Échec déchiffrement endpoint: {}", e)),
        }
    }

    // 3. Establish Tunnel
    let server_addr: SocketAddr = info.server_endpoint.parse().map_err(|e| format!("Invalid endpoint: {}", e))?;
    let credentials = Credentials::Password { 
        username: Some(identity.public_key_hex()), 
        password: "p2p-session-key".to_string() 
    };

    let config = ConnectionConfig {
        protocol: proto,
        server_addr,
        credentials,
        timeout: std::time::Duration::from_secs(10),
    };

    let mut tunnel: Box<dyn VpnTunnel + Send> = Box::new(GoTunnel::new(info.session_id.clone(), proto));

    match tunnel.connect(&config).await {
        Ok(handle) => {
            // Save active tunnel
            {
                let mut active = state.active_tunnel.lock().unwrap();
                *active = Some(tunnel);
            }

            // Update status
            let mut status = state.vpn_status.lock().unwrap();
            status.state = ConnectionState::Connected;
            status.current_ip = Some(handle.assigned_ip.to_string());
            status.protocol = Some(protocol);
            status.connected_since = Some(chrono::Utc::now().timestamp());
            Ok(status.clone())
        },
        Err(e) => {
            let e: VpnError = e;
            let err_msg = e.to_string();
            let mut status = state.vpn_status.lock().unwrap();
            status.state = ConnectionState::Error(err_msg.clone());
            Err(err_msg)
        }
    }
}

#[tauri::command]
async fn disconnect_vpn(state: State<'_, AppState>) -> Result<VpnStatus, String> {
    // 1. Stop active tunnel
    let tunnel = {
        let mut active = state.active_tunnel.lock().unwrap();
        active.take()
    };

    if let Some(mut t) = tunnel {
        let _ = t.disconnect().await;
    }

    // 2. Update status
    let mut status = state.vpn_status.lock().unwrap();
    status.state = ConnectionState::Disconnected;
    status.current_ip = None;
    status.protocol = None;
    status.connected_since = None;

    Ok(status.clone())
}

#[tauri::command]
async fn start_sharing(state: State<'_, AppState>) -> Result<bool, String> {
    {
        let mut sharing = state.is_sharing.lock().map_err(|_| "Failed to lock state")?;
        *sharing = true;
    }
    
    // Start P2P Discovery if not already started
    if state.p2p.get().is_none() {
        let discovery = PeerDiscovery::new().await.map_err(|e| e.to_string())?;
        let _ = state.p2p.set(Arc::new(discovery));
    }
    
    Ok(true)
}

#[tauri::command]
async fn stop_sharing(state: State<'_, AppState>) -> Result<bool, String> {
    let mut sharing = state.is_sharing.lock().map_err(|_| "Failed to lock state")?;
    *sharing = false;
    Ok(false)
}

#[tauri::command]
async fn get_p2p_status(_state: State<'_, AppState>) -> Result<P2pStats, String> {
    // In a real implementation, we would query the PeerDiscovery swarm
    // For now, returning mock data that reflects active movement
    Ok(P2pStats {
        connected_peers: 12,
        known_nodes: 156,
        total_sent: 45000,
        total_received: 120000,
    })
}

#[tauri::command]
fn get_vpn_status(state: State<'_, AppState>) -> VpnStatus {
    state.vpn_status.lock().unwrap().clone()
}

// Fixed in run()
pub fn run() {
    let api_client = VpnApiClient::new("https://api.worldvpn.com".to_string()); 

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            vpn_status: Mutex::new(VpnStatus::default()),
            is_sharing: Mutex::new(false),
            p2p: OnceCell::new(),
            api_client,
            active_tunnel: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            generate_identity,
            login_anonymously_desktop,
            migrate_credits_desktop,
            connect_vpn, 
            disconnect_vpn, 
            get_vpn_status,
            start_sharing,
            stop_sharing,
            get_p2p_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

