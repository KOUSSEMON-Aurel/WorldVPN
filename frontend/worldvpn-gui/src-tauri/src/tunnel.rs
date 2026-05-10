use std::sync::Mutex;
use tauri_plugin_shell::process::CommandChild;
use tauri_plugin_shell::ShellExt;
use tauri::{AppHandle, Emitter};

pub struct TunnelState {
    pub child: Mutex<Option<CommandChild>>,
}

impl TunnelState {
    pub fn new() -> Self {
        Self { child: Mutex::new(None) }
    }
}

#[tauri::command]
pub async fn start_tunnel(
    app: AppHandle,
    state: tauri::State<'_, TunnelState>,
    config_path: String,
    protocol: String, // Added protocol parameter
) -> Result<(), String> {
    let mut lock = state.child.lock().unwrap();
    if lock.is_some() {
        return Err("Tunnel déjà actif".to_string());
    }

    let (mut rx, child) = if protocol == "OpenVPN" {
        println!("Starting OpenVPN engine...");
        app.shell()
            .command("openvpn")
            .args(["--config", &config_path, "--dev", "tun", "--nobind"])
            .spawn()
            .map_err(|e| format!("Impossible de démarrer openvpn: {}", e))?
    } else {
        println!("Starting Sing-box engine...");
        app.shell()
            .sidecar("sing-box")
            .map_err(|e| format!("Erreur sidecar: {}", e))?
            .args(["run", "-c", &config_path])
            .spawn()
            .map_err(|e| format!("Impossible de démarrer sing-box: {}", e))?
    };

    // Monitor logs and events (remains the same as it handles CommandChild)
    let app_clone = app.clone();
    let engine_tag = if protocol == "OpenVPN" { "OPENVPN" } else { "SING-BOX" };
    
    tauri::async_runtime::spawn(async move {
        use tauri_plugin_shell::process::CommandEvent;
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) => {
                    let log = String::from_utf8_lossy(&line).to_string();
                    println!("{}: {}", engine_tag, log);
                    let _ = app_clone.emit("tunnel-log", log);
                }
                CommandEvent::Stderr(line) => {
                    let log = String::from_utf8_lossy(&line).to_string();
                    println!("{} ERR: {}", engine_tag, log);
                    let _ = app_clone.emit("tunnel-error", log);
                }
                CommandEvent::Terminated(payload) => {
                    println!("{} TERMINATED with code {:?}", engine_tag, payload.code);
                    let _ = app_clone.emit("tunnel-stopped", payload.code);
                    break;
                }
                _ => {}
            }
        }
    });

    *lock = Some(child);
    Ok(())
}

#[tauri::command]
pub async fn stop_tunnel(
    state: tauri::State<'_, TunnelState>,
) -> Result<(), String> {
    let mut lock = state.child.lock().unwrap();
    if let Some(child) = lock.take() {
        child.kill().map_err(|e| format!("Erreur lors de l'arrêt: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
pub async fn get_tunnel_status(
    state: tauri::State<'_, TunnelState>,
) -> Result<bool, String> {
    let lock = state.child.lock().unwrap();
    Ok(lock.is_some())
}
