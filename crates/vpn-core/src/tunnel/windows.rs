#[cfg(target_os = "windows")]
use std::sync::Arc;
#[cfg(target_os = "windows")]
use wintun;

pub struct WindowsTunnel {
    #[cfg(target_os = "windows")]
    _adapter: Arc<wintun::Adapter>,
}

impl WindowsTunnel {
    #[cfg(target_os = "windows")]
    pub fn new(name: &str) -> anyhow::Result<Self> {
        let wintun = unsafe { wintun::load_from_path("wintun.dll") }
            .map_err(|e| anyhow::anyhow!("Failed to load wintun.dll: {}", e))?;
        
        let adapter = wintun::Adapter::create(&wintun, "WorldVPN", name, None)
            .map_err(|e| anyhow::anyhow!("Failed to create wintun adapter: {}", e))?;
        
        Ok(Self { _adapter: adapter })
    }

    #[cfg(not(target_os = "windows"))]
    pub fn new(_name: &str) -> anyhow::Result<Self> {
        Err(anyhow::anyhow!("WindowsTunnel is only available on Windows"))
    }
}
