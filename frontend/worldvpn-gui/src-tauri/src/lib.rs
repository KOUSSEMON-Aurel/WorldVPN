use std::sync::{Arc, Mutex};
use base64::prelude::*;
use serde::{Serialize, Deserialize};
use vpn_core::{
    crypto::IdentityKey,
    client::VpnApiClient,
    p2p::PeerDiscovery,
};
use tokio::sync::OnceCell;
use std::path::PathBuf;
use std::fs;
use tauri::{Manager, Emitter, State};

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
    discovery_manager: vpn_core::public_gate::DiscoveryManager,
    is_discovering: Mutex<bool>,
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
    country: Option<String>,
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
            country: None,
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
    ovpn_config: Option<String>,
    ss_metadata: Option<String>,
    provider: Option<String>,
) -> Result<VpnStatus, String> {
    let private_key = load_identity(&app_handle)?;
    let identity = IdentityKey::from_bytes(&private_key).map_err(|e| e.to_string())?;
    
    // 1. Update state to Connecting
    {
        let mut status = state.vpn_status.lock().map_err(|_| "Failed to lock state")?;
        status.state = ConnectionState::Connecting;
    }

    let mut blacklist: Vec<String> = Vec::new();
    let mut last_error = "Unknown connection error".to_string();

    // Load the public node list ONCE before the retry loop
    // Check cache synchronously, then fetch if needed (outside any lock)
    // Load the public node list via DiscoveryManager
    let public_nodes = {
        let nodes = state.discovery_manager.get_nodes();
        if nodes.is_empty() {
            let backend_url = state.api_client.base_url().to_string();
            vpn_core::public_gate::fetch_all_public_nodes(Some(&backend_url)).await
        } else {
            nodes
        }
    };
    println!("Using {} public nodes for retry attempts.", public_nodes.len());

    let mut current_ovpn_config = ovpn_config;
    let mut current_ss_metadata = ss_metadata;

    for attempt in 1..=3 {
        println!("Connection attempt {}/3...", attempt);
        
        // 2. Perform matchmaking and connect
        let mut chosen_proto = match protocol.as_str() {
            "WireGuard" => vpn_core::protocol::VpnProtocol::WireGuard,
            "Hysteria 2" => vpn_core::protocol::VpnProtocol::Hysteria2,
            "Shadowsocks" => vpn_core::protocol::VpnProtocol::Shadowsocks,
            "Trojan" => vpn_core::protocol::VpnProtocol::Trojan,
            "VLESS" => vpn_core::protocol::VpnProtocol::VLESS,
            "OpenVPN" => vpn_core::protocol::VpnProtocol::OpenVPN,
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
        let mut final_peer_pub_raw = info.server_public_key.clone().unwrap_or_default();
        let mut final_ovpn_content = String::new();
        // Tier 1.5: If direct metadata was provided (from map selection), use it immediately
        if let Some(ref config) = current_ovpn_config {
            println!("Using direct OVPN config provided by frontend");
            final_ovpn_content = config.clone();
            chosen_proto = vpn_core::protocol::VpnProtocol::OpenVPN;
        } else if let Some(ref meta) = current_ss_metadata {
            println!("Using direct Shadowsocks metadata provided by frontend");
            final_peer_pub_raw = meta.clone(); // For Shadowsocks, we store "method:password" here
            chosen_proto = vpn_core::protocol::VpnProtocol::Shadowsocks;
        }
        
        // Tier 2: Automatic Fallback if no P2P nodes available
        if final_endpoint.starts_with("error.worldvpn.net") {
            println!("No P2P nodes available, selecting from public node pool (excluding {} blacklisted)...", blacklist.len());
            
            let fallback_manager = FallbackManager::new();
            // Use the pre-loaded node list — NO re-fetch
            match fallback_manager.get_fallback_from_nodes(&public_nodes, &country, &blacklist) {
                Ok(fallback) => {
                    println!("Public Gate Fallback obtained: {:?} at {}", fallback.provider, fallback.server_ip);
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
                        final_peer_pub_raw = pub_key_b64.clone();
                    } else if chosen_proto == vpn_core::protocol::VpnProtocol::Shadowsocks {
                        final_peer_pub_raw = "chacha20-ietf-poly1305:m".to_string();
                    }

                    if let Some(config) = fallback.raw_config {
                        final_ovpn_content = config;
                    }
                },
                Err(e) => {
                    last_error = format!("No more public nodes available: {}", e);
                    break;
                }
            }
        }
        
        // Phase 4: E2E Decryption
        if final_endpoint.starts_with("e2e:") {
            let encrypted = &final_endpoint[4..];
            match identity.decrypt_with_identity(encrypted) {
                Ok(decrypted) => final_endpoint = decrypted,
                Err(e) => {
                    last_error = format!("Échec déchiffrement endpoint: {}", e);
                    continue;
                }
            }
        }

        if !final_endpoint.contains(':') {
            final_endpoint = format!("{}:51820", final_endpoint);
        } else if final_endpoint.ends_with(":0") {
            final_endpoint = format!("{}2408", &final_endpoint[..final_endpoint.len()-1]);
        }

        // 3. Establish Tunnel Settings
        let mut addrs = match tokio::net::lookup_host(&final_endpoint).await {
            Ok(a) => a,
            Err(e) => {
                last_error = format!("Invalid endpoint or DNS failure ({}): {}", final_endpoint, e);
                continue;
            }
        };
        
        let server_addr = match addrs.next() {
            Some(a) => a,
            None => {
                last_error = format!("Could not resolve endpoint: {}", final_endpoint);
                continue;
            }
        };
        
        println!("Checking endpoint health: {} (Port: {})", server_addr.ip(), server_addr.port());

        let mut health_ok = true;
        if chosen_proto == vpn_core::protocol::VpnProtocol::Shadowsocks {
            if let Err(e) = tokio::time::timeout(std::time::Duration::from_secs(1), tokio::net::TcpStream::connect(&server_addr)).await {
                println!("Health check failed for {}: {}", server_addr, e);
                health_ok = false;
            }
        } else if chosen_proto == vpn_core::protocol::VpnProtocol::OpenVPN {
            if final_ovpn_content.contains("proto tcp") || !final_ovpn_content.contains("proto udp") {
                if let Err(e) = tokio::time::timeout(std::time::Duration::from_secs(1), tokio::net::TcpStream::connect(&server_addr)).await {
                    println!("OpenVPN Health check failed for {}: {}", server_addr, e);
                    health_ok = false;
                }
            }
        }

        if !health_ok {
            blacklist.push(server_addr.ip().to_string());
            last_error = format!("Health check failed (Server {} Reachable)", if attempt < 3 { "Not yet" } else { "Un" });
            continue;
        }

        println!("Health check successful. Proceeding to tunnel start.");
        
        // 4. Start Tunnel (Phase 5)
        let tunnel_result = if chosen_proto == vpn_core::protocol::VpnProtocol::OpenVPN {
            // Special Path for OpenVPN: Write .ovpn file
            let mut config_path = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
            fs::create_dir_all(&config_path).ok();
            config_path.push("config.ovpn");

            let ovpn_content = final_ovpn_content;

            // The config from VPNGate is Base64
            let decoded_config = if ovpn_content.len() > 100 {
                 use base64::{Engine as _, engine::general_purpose};
                 general_purpose::STANDARD.decode(ovpn_content.trim())
                    .map(|b| String::from_utf8_lossy(&b).to_string())
                    .unwrap_or(ovpn_content)
            } else {
                ovpn_content
            };

            let mut creds_username = "vpn".to_string();
            let mut creds_password = "vpn".to_string();

            if let Some(ref p) = provider {
                if p.to_lowercase() == "vpnbook" {
                    creds_username = "vpnbook".to_string();
                    let backend_url = state.api_client.base_url();
                    if let Ok(pwd) = vpn_core::public_gate::fetch_vpnbook_password(Some(backend_url)).await {
                        creds_password = pwd;
                    }
                }
            }

            let mut creds_path = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
            creds_path.push("creds.txt");
            let _ = fs::write(&creds_path, format!("{}\n{}\n", creds_username, creds_password));
            
            // Set permissions to 600 (read/write for owner only) to fix OpenVPN warning
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&creds_path, fs::Permissions::from_mode(0o600));
            }

            let mut new_config: Vec<String> = decoded_config
                .lines()
                .filter(|l| {
                    let trimmed = l.trim();
                    !trimmed.starts_with("auth-user-pass")
                        && !trimmed.starts_with("data-ciphers")
                })
                .map(|s| s.to_string())
                .collect();

            new_config.push(format!("auth-user-pass {}", creds_path.to_string_lossy()));
            new_config.push("data-ciphers DEFAULT:AES-128-GCM:AES-256-GCM:AES-128-CBC:AES-256-CBC:CHACHA20-POLY1305".to_string());
            new_config.push("data-ciphers-fallback AES-128-CBC".to_string());

            let final_config = new_config.join("\n");
            if let Err(e) = fs::write(&config_path, &final_config) {
                Err(e.to_string())
            } else {
                println!("OpenVPN config written to: {:?}", config_path);
                let tunnel_state: State<'_, TunnelState> = app_handle.state();
                tunnel::start_tunnel(app_handle.clone(), tunnel_state, config_path.to_string_lossy().to_string(), "OpenVPN".to_string()).await.map_err(|e| e.to_string())
            }
        } else {
            // Standard Sing-box Path
            let priv_key_32 = if final_private_key.len() >= 32 { &final_private_key[..32] } else { &final_private_key };
            let priv_key_b64 = BASE64_STANDARD.encode(priv_key_32);
            let proto_name = format!("{:?}", chosen_proto);
            
            let sb_config = config::build_sing_box_config(
                &proto_name,
                server_addr,
                &final_assigned_ip,
                &priv_key_b64,
                &final_peer_pub_raw,
                1280
            );

            let mut config_path = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
            fs::create_dir_all(&config_path).ok();
            config_path.push("sing-box-config.json");
            
            match serde_json::to_string_pretty(&sb_config) {
                Ok(config_json) => {
                    let _ = fs::write(&config_path, &config_json);
                    println!("Sing-box config written to: {:?}", config_path);
                    let tunnel_state: State<'_, TunnelState> = app_handle.state();
                    tunnel::start_tunnel(app_handle.clone(), tunnel_state, config_path.to_string_lossy().to_string(), proto_name).await.map_err(|e| e.to_string())
                },
                Err(e) => Err(e.to_string())
            }
        };

        match tunnel_result {
            Ok(_) => {
                println!("VPN Tunnel Started successfully.");
                let status = {
                    let mut status = state.vpn_status.lock().map_err(|_| "VPN status lock poisoned")?;
                    status.state = ConnectionState::Connected;
                    status.current_ip = Some(final_assigned_ip);
                    status.country = Some(country);
                    status.protocol = Some(protocol);
                    status.connected_since = Some(chrono::Utc::now().timestamp());
                    status.clone()
                };
                return Ok(status);
            },
            Err(e) => {
                println!("Tunnel attempt {} failed: {}. Retrying...", attempt, e);
                last_error = e;
                blacklist.push(server_addr.ip().to_string());
                // Also blacklist by original endpoint if it was a hostname
                if final_endpoint.contains('.') {
                     let parts: Vec<&str> = final_endpoint.split(':').collect();
                     blacklist.push(parts[0].to_string());
                }
                // Clear manual selection for next attempts
                current_ovpn_config = None;
                current_ss_metadata = None;
                continue;
            }
        }
    }

    Err(last_error)
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
async fn get_nodes(group: String, app_handle: tauri::AppHandle, state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    if group == "COMMUNITY" {
        Ok(serde_json::json!([]))
    } else {
        let (_nodes, trigger_discovery) = {
            let nodes = state.discovery_manager.get_nodes();
            let discovering = state.is_discovering.lock().map_err(|_| "Status lock failed")?;
            
            // If empty, always trigger initial discovery
            if nodes.is_empty() {
                (nodes, !*discovering)
            } else {
                // Return cached, but background task will check TTLs
                (nodes, !*discovering) 
            }
        };

        if trigger_discovery {
            {
                let mut discovering = state.is_discovering.lock().map_err(|_| "Status lock failed")?;
                *discovering = true;
            }
            
            let backend_url = state.api_client.base_url().to_string();
            let handle_clone = app_handle.clone();
            
            tokio::spawn(async move {
                let state: State<'_, AppState> = handle_clone.state();
                
                // If it's the very first load, use the "Turbo" fetch_all_public_nodes
                let current_nodes = state.discovery_manager.get_nodes();
                if current_nodes.is_empty() {
                    let initial_nodes = vpn_core::public_gate::fetch_all_public_nodes(Some(&backend_url)).await;
                    let mut nodes_lock = state.discovery_manager.nodes.lock().unwrap();
                    *nodes_lock = initial_nodes;
                    
                    // Mark all as just updated for the initial cycle
                    let now = std::time::Instant::now();
                    *state.discovery_manager.last_vpngate_refresh.lock().unwrap() = Some(now);
                    *state.discovery_manager.last_ss_refresh.lock().unwrap() = Some(now);
                    *state.discovery_manager.last_vpnbook_refresh.lock().unwrap() = Some(now);
                } else {
                    // Regular maintenance refresh based on individual TTLs
                    state.discovery_manager.refresh_if_needed(Some(&backend_url)).await;
                }

                {
                    let disc_lock = state.is_discovering.lock();
                    if let Ok(mut discovering) = disc_lock {
                        *discovering = false;
                        println!("Discovery maintenance task completed.");
                    }
                };
                
                // Emit event to refresh UI if needed
                let _ = handle_clone.emit("nodes-updated", ());
            });
        }

        Ok(serde_json::json!(nodes_to_json(&state.discovery_manager.get_nodes())))
    }
}

#[tauri::command]
async fn refresh_nodes(app_handle: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let backend_url = state.api_client.base_url().to_string();
    state.discovery_manager.refresh_if_needed(Some(&backend_url)).await;
    let _ = app_handle.emit("nodes-updated", ());
    Ok(())
}

// Helper to convert nodes to JSON (matching original logic)
fn nodes_to_json(nodes: &[vpn_core::nodes::VpnNode]) -> Vec<serde_json::Value> {
    nodes.iter().map(|n| {
        let (lat, lon) = (n.latitude, n.longitude); // Use REAL coords from node
        serde_json::json!({
            "id": n.id,
            "country_code": n.country_code,
            "latency_ms": n.ping_ms.unwrap_or(50),
            "group": "PUBLIC",
            "provider": format!("{:?}", n.source),
            "protocol": format!("{:?}", n.protocol),
            "ovpn_config": n.openvpn_config,
            "ss_metadata": if n.protocol == vpn_core::protocol::VpnProtocol::Shadowsocks {
                Some(format!("{}:{}", n.ss_method.as_deref().unwrap_or(""), n.ss_password.as_deref().unwrap_or("")))
            } else {
                None
            },
            "bandwidth_mbps": n.speed_mbps.unwrap_or(50.0),
            "lat": lat,
            "lon": lon,
        })
    }).collect()
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
            discovery_manager: vpn_core::public_gate::DiscoveryManager::new(),
            is_discovering: Mutex::new(false),
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
            refresh_nodes,
            tunnel::start_tunnel,
            tunnel::stop_tunnel,
            tunnel::get_tunnel_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

