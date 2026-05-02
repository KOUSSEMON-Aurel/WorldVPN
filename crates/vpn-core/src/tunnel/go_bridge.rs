use std::ffi::CString;
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;
use libloading::{Library, Symbol};
use std::sync::Mutex;
use tracing::info;

lazy_static::lazy_static! {
    static ref GO_LIB: Mutex<Option<Library>> = Mutex::new(None);
}

type StartTunnelFn = unsafe extern "C" fn(c_int, *const c_char) -> c_int;
type StopTunnelFn = unsafe extern "C" fn();

pub struct GoBridge;

impl GoBridge {
    fn load_library() -> anyhow::Result<()> {
        let mut lib_lock = GO_LIB.lock().unwrap();
        if lib_lock.is_some() {
            return Ok(());
        }

        let lib_path = if cfg!(target_os = "linux") {
            // Path relative to the executable or a standard location
            // For development, we look into the vpn-core/lib directory
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("lib/libvpngo.so")
        } else {
            anyhow::bail!("Unsupported OS for GoBridge");
        };

        if !lib_path.exists() {
            anyhow::bail!("libvpngo.so not found at {:?}", lib_path);
        }

        unsafe {
            let lib = Library::new(lib_path)?;
            *lib_lock = Some(lib);
        }
        Ok(())
    }

    pub fn start_tunnel(tun_fd: i32, config_json: &str) -> anyhow::Result<()> {
        Self::load_library()?;
        let lib_lock = GO_LIB.lock().unwrap();
        let lib = lib_lock.as_ref().unwrap();

        unsafe {
            let start_func: Symbol<StartTunnelFn> = lib.get(b"StartTunnel")?;
            let c_json = CString::new(config_json)?;
            let result = start_func(tun_fd as c_int, c_json.as_ptr());
            if result != 0 {
                anyhow::bail!("Go StartTunnel failed with code {}", result);
            }
        }
        info!("Go tunnel started successfully");
        Ok(())
    }

    pub fn stop_tunnel() -> anyhow::Result<()> {
        Self::load_library()?;
        let lib_lock = GO_LIB.lock().unwrap();
        let lib = lib_lock.as_ref().unwrap();

        unsafe {
            let stop_func: Symbol<StopTunnelFn> = lib.get(b"StopTunnel")?;
            stop_func();
        }
        info!("Go tunnel stopped successfully");
        Ok(())
    }
}
