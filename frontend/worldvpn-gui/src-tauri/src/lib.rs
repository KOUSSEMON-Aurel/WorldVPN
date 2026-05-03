use std::sync::{Arc, Mutex};
use tauri::State;
use serde::{Serialize, Deserialize};
use vpn_core::{
    crypto::IdentityKey,
    client::VpnApiClient,
    p2p::PeerDiscovery,
};
use tokio::sync::OnceCell;
use std::net::{IpAddr, SocketAddr};
use tauri::Manager;
use std::path::PathBuf;
use std::fs;

use vpn_core::fallback::FallbackManager;
mod tunnel;
mod config;
use tunnel::TunnelState;

// Shared state to track VPN status across the app
struct AppState {
    vpn_status: Mutex<VpnStatus>,
    is_sharing: Mutex<bool>,
    p2p: OnceCell<Arc<PeerDiscovery>>,
    api_client: VpnApiClient,
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

fn get_identity_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let mut dir = app_handle.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    fs::create_dir_all(&dir).ok();
    dir.push("identity.dat");
    Ok(dir)
}

fn save_identity(app_handle: &tauri::AppHandle, private_key: &[u8]) -> Result<(), String> {
    let path = get_identity_path(app_handle)?;
    fs::write(path, private_key).map_err(|e| format!("Failed to save identity: {}", e))
}

fn load_identity(app_handle: &tauri::AppHandle) -> Result<Vec<u8>, String> {
    let path = get_identity_path(app_handle)?;
    fs::read(path).map_err(|e| format!("Failed to load identity: {}", e))
}

#[tauri::command]
async fn generate_identity(app_handle: tauri::AppHandle) -> Result<serde_json::Value, String> {
    println!("Generating new identity...");
    let identity = IdentityKey::generate();
    let pub_key = identity.public_key_hex();
    let priv_bytes = identity.to_bytes().map_err(|e| e.to_string())?;
    println!("Identity generated: {}", pub_key);
    
    save_identity(&app_handle, &priv_bytes)?;
    
    Ok(serde_json::json!({
        "public_key": pub_key,
    }))
}

#[tauri::command]
async fn import_identity(
    private_key: Vec<u8>,
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>
) -> Result<serde_json::Value, String> {
    let _identity = IdentityKey::from_bytes(&private_key).map_err(|e| e.to_string())?;
    save_identity(&app_handle, &private_key)?;
    login_anonymously_desktop(app_handle, state).await
}

#[tauri::command]
async fn is_identity_saved(app_handle: tauri::AppHandle) -> Result<bool, String> {
    let path = get_identity_path(&app_handle)?;
    Ok(path.exists())
}

#[tauri::command]
async fn login_anonymously_desktop(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>
) -> Result<serde_json::Value, String> {
    let private_key = load_identity(&app_handle)?;
    println!("login_anonymously_desktop called with {} saved bytes", private_key.len());
    
    let identity = IdentityKey::from_bytes(&private_key).map_err(|e| {
        println!("Failed to parse identity key bytes: {}", e);
        e.to_string()
    })?;
    
    println!("Logging in as: {}", identity.public_key_hex());
    
    // Handshake V2 : Signature du timestamp actuel
    let timestamp = chrono::Utc::now().timestamp().to_string();
    let signature = identity.sign_challenge(&timestamp);
    
    println!("Sending handshake to backend...");
    let response = state.api_client.login_with_identity(
        identity.public_key_hex(),
        signature,
        timestamp
    ).await.map_err(|e| {
        println!("Backend login failure: {}", e);
        e.to_string()
    })?;
    
    println!("Login successful for user: {}", response.username);
    
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
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
    protocol: String,
    country: String,
    token: String,
) -> Result<VpnStatus, String> {
    let private_key = load_identity(&app_handle)?;
    let identity = IdentityKey::from_bytes(&private_key).map_err(|e| e.to_string())?;
    
    // 1. Update state to Connecting
    {
        let mut status = state.vpn_status.lock().map_err(|_| "Failed to lock state")?;
        status.state = ConnectionState::Connecting;
    }

    // 2. Perform matchmaking and connect
    let mut chosen_proto = match protocol.as_str() {
        "WireGuard" => vpn_core::protocol::VpnProtocol::WireGuard,
        "Hysteria 2" => vpn_core::protocol::VpnProtocol::Hysteria2,
        "Shadowsocks" => vpn_core::protocol::VpnProtocol::Shadowsocks,
        "Trojan" => vpn_core::protocol::VpnProtocol::Trojan,
        "VLESS" => vpn_core::protocol::VpnProtocol::VLESS,
        _ => vpn_core::protocol::VpnProtocol::WireGuard,
    };

    let info = state.api_client.connect(
        chosen_proto,
        identity.public_key_hex(),
        Some(country.clone()),
        &token
    ).await.map_err(|e| e.to_string())?;
    
    let mut final_endpoint = info.server_endpoint.clone();
    let mut final_assigned_ip = info.assigned_ip.clone();
    let mut final_private_key = private_key.to_vec();
    let mut final_peer_pub = info.server_public_key.as_ref()
        .and_then(|k| hex::decode(k).ok())
        .unwrap_or_default();
    
    // Tier 2: Automatic Fallback to Cloudflare WARP if P2P fails
    if final_endpoint.starts_with("error.worldvpn.net") {
        println!("No P2P nodes available, triggering Fallback (Cloudflare WARP)...");
        let fallback_manager = FallbackManager::new(true);
        match fallback_manager.get_fallback_config(&country).await {
            Ok(fallback) => {
                println!("Fallback obtained: {:?} at {}", fallback.provider, fallback.server_ip);
                final_endpoint = format!("{}:{}", fallback.server_ip, fallback.port);
                chosen_proto = fallback.protocol;
                final_assigned_ip = fallback.assigned_ip;
                
                if let Some(ref priv_key_b64) = fallback.private_key {
                    use base64::{Engine as _, engine::general_purpose};
                    if let Ok(bytes) = general_purpose::STANDARD.decode(priv_key_b64) {
                        final_private_key = bytes;
                    }
                }
                
                if let Some(ref pub_key_b64) = fallback.peer_public_key {
                    use base64::{Engine as _, engine::general_purpose};
                    if let Ok(bytes) = general_purpose::STANDARD.decode(pub_key_b64) {
                        final_peer_pub = bytes;
                    }
                }
            },
            Err(e) => return Err(format!("Total network failure: Both P2P and Fallback failed. {}", e)),
        }
    }
    
    // Phase 4: E2E Decryption
    if final_endpoint.starts_with("e2e:") {
        let encrypted = &final_endpoint[4..];
        match identity.decrypt_with_identity(encrypted) {
            Ok(decrypted) => final_endpoint = decrypted,
            Err(e) => return Err(format!("Échec déchiffrement endpoint: {}", e)),
        }
    }

    if !final_endpoint.contains(':') {
        final_endpoint = format!("{}:51820", final_endpoint);
    }

    // 3. Establish Tunnel Settings
    let server_addr: SocketAddr = final_endpoint.parse().map_err(|e| format!("Invalid endpoint ({}): {}", final_endpoint, e))?;
    
    // 4. Generate sing-box config
    let sb_config = config::build_sing_box_config(
        if chosen_proto == vpn_core::protocol::VpnProtocol::WireGuard { "WireGuard" } else { "Other" },
        server_addr,
        &final_assigned_ip,
        if final_private_key.len() >= 32 { &final_private_key[..32] } else { &final_private_key },
        if final_peer_pub.len() >= 32 { &final_peer_pub[..32] } else { &final_peer_pub },
        1420
    );

    let mut config_path = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&config_path).ok();
    config_path.push("sing-box-config.json");
    
    fs::write(&config_path, serde_json::to_string_pretty(&sb_config).map_err(|e| e.to_string())?)
        .map_err(|e| format!("Failed to write config: {}", e))?;

    // 5. Start Tunnel Sidecar
    let tunnel_state: State<'_, TunnelState> = app_handle.state();
    tunnel::start_tunnel(app_handle.clone(), tunnel_state, config_path.to_string_lossy().to_string())
        .await
        .map_err(|e| e.to_string())?;

    // Update status
    let status = {
        let mut status = state.vpn_status.lock().map_err(|_| "VPN status lock poisoned")?;
        status.state = ConnectionState::Connected;
        status.current_ip = Some(final_assigned_ip);
        status.protocol = Some(protocol);
        status.connected_since = Some(chrono::Utc::now().timestamp());
        status.clone()
    };
    Ok(status)
}

#[tauri::command]
async fn disconnect_vpn(app_handle: tauri::AppHandle, state: State<'_, AppState>) -> Result<VpnStatus, String> {
    // 1. Stop active tunnel sidecar
    let tunnel_state: State<'_, TunnelState> = app_handle.state();
    let _ = tunnel::stop_tunnel(tunnel_state).await;

    // 2. Update status
    let status = {
        let mut status = state.vpn_status.lock().map_err(|_| "VPN status lock poisoned")?;
        status.state = ConnectionState::Disconnected;
        status.current_ip = None;
        status.protocol = None;
        status.connected_since = None;
        status.clone()
    };

    Ok(status)
}

#[tauri::command]
async fn start_sharing(state: State<'_, AppState>) -> Result<bool, String> {
    {
        let mut sharing = state.is_sharing.lock().map_err(|_| "Failed to lock state")?;
        *sharing = true;
    }
    
    // Start P2P Discovery if not already started
    if state.p2p.get().is_none() {
        match PeerDiscovery::new().await {
            Ok(discovery) => {
                let _ = state.p2p.set(Arc::new(discovery));
                println!("P2P Discovery initialized successfully");
            },
            Err(e) => {
                println!("Failed to initialize P2P Discovery: {}", e);
                // Even if P2P fails, we might still want to share in an degraded mode or fallback.
                // For now, return the error clearly to the UI.
                return Err(format!("P2P Init failed: {}", e));
            }
        }
    }
    
    Ok(true)
}

#[tauri::command]
fn stop_sharing(state: State<'_, AppState>) -> Result<bool, String> {
    let mut sharing = state.is_sharing.lock().map_err(|_| "Failed to lock state")?;
    *sharing = false;
    Ok(false)
}

#[tauri::command]
async fn get_p2p_status(state: State<'_, AppState>) -> Result<P2pStats, String> {
    let sharing = *state.is_sharing.lock().unwrap();
    if !sharing {
        return Ok(P2pStats {
            connected_peers: 0,
            known_nodes: 0,
            total_sent: 0,
            total_received: 0,
        });
    }

    // In a real implementation, we would query the PeerDiscovery swarm
    Ok(P2pStats {
        connected_peers: 12,
        known_nodes: 156,
        total_sent: 45000,
        total_received: 120000,
    })
}

#[tauri::command]
fn get_vpn_status(state: State<'_, AppState>) -> VpnStatus {
    state.vpn_status.lock().map(|g| g.clone()).unwrap_or_default()
}

#[derive(serde::Serialize)]
pub struct VpnMetrics {
    down_mbps: f64,
    up_mbps: f64,
    latency_ms: Option<u32>,
}

#[tauri::command]
fn get_vpn_metrics(state: State<'_, AppState>) -> VpnMetrics {
    if let Ok(status) = state.vpn_status.lock() {
        if matches!(status.state, ConnectionState::Connected) {
            // Backend stub for metrics until native tunnel stats are available
            return VpnMetrics {
                down_mbps: 15.4,
                up_mbps: 2.1,
                latency_ms: Some(23),
            };
        }
    }
    VpnMetrics {
        down_mbps: 0.0,
        up_mbps: 0.0,
        latency_ms: None,
    }
}

#[tauri::command]
async fn get_nodes(group: String, state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    if group == "COMMUNITY" {
        Ok(serde_json::json!([
            { "id": "4", "country_code": "FR", "bandwidth_mbps": 20, "latency_ms": 10, "group": "COMMUNITY" },
            { "id": "5", "country_code": "IN", "bandwidth_mbps": 15, "latency_ms": 65, "group": "COMMUNITY" }
        ]))
    } else {
        match state.api_client.fetch_public_nodes().await {
            Ok(nodes) => Ok(nodes),
            Err(_) => {
                Ok(serde_json::json!([
                    { "id": "1", "country_code": "JP", "bandwidth_mbps": 100, "latency_ms": 120, "group": "PUBLIC" },
                    { "id": "2", "country_code": "US", "bandwidth_mbps": 80, "latency_ms": 45, "group": "PUBLIC" },
                    { "id": "3", "country_code": "DE", "bandwidth_mbps": 50, "latency_ms": 15, "group": "PUBLIC" }
                ]))
            }
        }
    }
}

#[tauri::command]
async fn get_sessions(_state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!([
        { "id": 1, "country": "DE", "type": "browsing", "bytes": "15.4 MB", "earning": "+0.15 CR" },
        { "id": 2, "country": "IR", "type": "censorship-bypass", "bytes": "42.1 MB", "earning": "+0.80 CR" },
        { "id": 3, "country": "US", "type": "streaming", "bytes": "128.0 MB", "earning": "+1.20 CR" }
    ]))
}

#[tauri::command]
async fn get_wallet_balance_desktop(token: String, state: State<'_, AppState>) -> Result<i64, String> {
    state.api_client.fetch_balance(&token).await.map_err(|e| e.to_string())
}

#[tauri::command]
fn get_transactions(_token: String, _state: State<'_, AppState>) -> Result<Vec<vpn_core::client::Transaction>, String> {
    vpn_core::api::simple::get_transactions().map_err(|e| e.to_string())
}

// Fixed in run()
pub fn run() {
    let url = std::env::var("WORLDVPN_API_URL")
        .unwrap_or_else(|_| "https://worldvpn-backend.onrender.com".to_string());
    println!("Initializing API client with URL: {}", url);
    let api_client = VpnApiClient::new(url); 

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            vpn_status: Mutex::new(VpnStatus::default()),
            is_sharing: Mutex::new(false),
            p2p: OnceCell::new(),
            api_client,
        })
        .manage(TunnelState::new())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            generate_identity,
            import_identity,
            is_identity_saved,
            login_anonymously_desktop,
            migrate_credits_desktop,
            connect_vpn, 
            disconnect_vpn, 
            get_vpn_status,
            get_vpn_metrics,
            start_sharing,
            stop_sharing,
            get_p2p_status,
            get_nodes,
            get_sessions,
            get_wallet_balance_desktop,
            get_transactions,
            tunnel::start_tunnel,
            tunnel::stop_tunnel,
            tunnel::get_tunnel_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

