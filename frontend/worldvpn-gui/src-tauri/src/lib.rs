use std::sync::Mutex;
use tauri::State;
use serde::{Serialize, Deserialize};

// Shared state to track VPN status across the app
struct AppState {
    vpn_status: Mutex<VpnStatus>,
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
struct VpnStatus {
    state: ConnectionState,
    current_ip: Option<String>,
    protocol: Option<String>,
    bytes_up: u64,
    bytes_down: u64,
    connected_since: Option<i64>,
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
        }
    }
}

#[tauri::command]
async fn connect_vpn(
    protocol: String,
    _country: String,
    state: State<'_, AppState>,
    #[allow(unused_variables)]
    app_handle: tauri::AppHandle,
) -> Result<VpnStatus, String> {
    // 1. Update state to Connecting
    {
        let mut status = state.vpn_status.lock().map_err(|_| "Failed to lock state")?;
        status.state = ConnectionState::Connecting;
    }

    // 2. Platform Specific logic
    #[cfg(target_os = "windows")]
    {
        tracing::info!("Initializing Windows Tunnel...");
        if !check_admin_privileges() {
            return Err("Administrator privileges required for VPN tunnel".into());
        }
        
        let _tunnel = vpn_core::tunnel::WindowsTunnel::new("wg0")
            .map_err(|e| format!("Tunnel initialization failed: {}", e))?;
        
        tracing::info!("Windows Tunnel 'wg0' established effectively.");
    }

    #[cfg(target_os = "android")]
    {
        use tauri::Manager;
        
        // Start WorldVpnService via JNI/Android Intent
        tracing::info!("Triggering Android VPN Service Intent...");
        // In the future, we will use the native JNI plugin here
    }

    // 3. Simulate Connection Delay
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

    // 4. Update state to Connected
    let mut status = state.vpn_status.lock().map_err(|_| "Failed to lock state")?;
    status.state = ConnectionState::Connected;
    status.current_ip = Some(format!("10.8.0.{}", rand::random::<u8>()));
    status.protocol = Some(protocol);
    status.connected_since = Some(chrono::Utc::now().timestamp());

    Ok(status.clone())
}

#[tauri::command]
async fn disconnect_vpn(state: State<'_, AppState>) -> Result<VpnStatus, String> {
    let mut status = state.vpn_status.lock().map_err(|_| "Failed to lock state")?;
    
    // Simulate Disconnection
    status.state = ConnectionState::Disconnected;
    status.current_ip = None;
    status.protocol = None;
    status.connected_since = None;

    Ok(status.clone())
}

#[tauri::command]
fn get_vpn_status(state: State<'_, AppState>) -> VpnStatus {
    state.vpn_status.lock().unwrap().clone()
}

#[cfg(target_os = "windows")]
fn check_admin_privileges() -> bool {
    // Basic check for admin on Windows using a light method
    // In a real app, we might use the 'is_executable_admin' crate
    std::process::Command::new("net")
        .arg("session")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "windows")]
    if !check_admin_privileges() {
        tracing::warn!("Application not running with administrator privileges. VPN tunnel may fail.");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            vpn_status: Mutex::new(VpnStatus::default()),
        })
        .invoke_handler(tauri::generate_handler![
            connect_vpn, 
            disconnect_vpn, 
            get_vpn_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
