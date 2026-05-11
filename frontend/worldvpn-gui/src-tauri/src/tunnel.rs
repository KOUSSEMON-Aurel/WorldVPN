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
    protocol: String,
) -> Result<(), String> {
    // 1. Check if active without holding the lock too long
    {
        let lock = state.child.lock().unwrap();
        if lock.is_some() {
            return Err("Tunnel déjà actif".to_string());
        }
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

    let (tx, mut rx_done) = tokio::sync::oneshot::channel();
    let mut tx_opt = Some(tx);

    // Monitor logs and events
    let app_clone = app.clone();
    let engine_tag = if protocol == "OpenVPN" { "OPENVPN" } else { "SING-BOX" };
    
    tauri::async_runtime::spawn(async move {
        use tauri_plugin_shell::process::CommandEvent;
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) => {
                    let log = String::from_utf8_lossy(&line).to_string();
                    println!("{}: {}", engine_tag, log);
                    let _ = app_clone.emit("tunnel-log", log.clone());

                    // Success Detection
                    if log.contains("Initialization Sequence Completed") || log.contains("netstack in-out initialized") {
                        if let Some(chan) = tx_opt.take() {
                            let _ = chan.send(Ok(()));
                        }
                    }
                    // Failure Detection in logs
                    if log.contains("AUTH_FAILED") || log.contains("fatal ERROR") {
                        if let Some(chan) = tx_opt.take() {
                            let _ = chan.send(Err(format!("Tunnel failure detected in logs: {}", log)));
                        }
                    }
                }
                CommandEvent::Stderr(line) => {
                    let log = String::from_utf8_lossy(&line).to_string();
                    println!("{} ERR: {}", engine_tag, log);
                    let _ = app_clone.emit("tunnel-error", log.clone());
                    
                    if log.contains("ERROR") || log.contains("failed") {
                         if let Some(chan) = tx_opt.take() {
                            let _ = chan.send(Err(log));
                        }
                    }
                }
                CommandEvent::Terminated(payload) => {
                    println!("{} TERMINATED with code {:?}", engine_tag, payload.code);
                    let _ = app_clone.emit("tunnel-stopped", payload.code);
                    if let Some(chan) = tx_opt.take() {
                        let _ = chan.send(Err(format!("Process terminated with code {:?}", payload.code)));
                    }
                    break;
                }
                _ => {}
            }
        }
    });

    // Wait for success or timeout (30s)
    let result = tokio::time::timeout(std::time::Duration::from_secs(30), &mut rx_done).await;
    
    match result {
        Ok(Ok(Ok(()))) => {
            let mut lock = state.child.lock().unwrap();
            *lock = Some(child);
            Ok(())
        },
        Ok(Ok(Err(e))) => {
            child.kill().ok();
            Err(e)
        },
        Ok(Err(_)) => {
             child.kill().ok();
             Err("Signal failure".to_string())
        },
        Err(_) => {
            child.kill().ok();
            Err("Connection timeout (30s)".to_string())
        }
    }
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
