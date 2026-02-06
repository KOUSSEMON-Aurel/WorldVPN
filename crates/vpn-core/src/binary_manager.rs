use std::path::PathBuf;
use std::process::Command;
use tokio::fs;
use tracing::info;

use crate::error::{Result, VpnError};

/// Specification for an external VPN binary (Shadowsocks, V2Ray, etc.)
#[derive(Debug, Clone)]
pub struct BinarySpec {
    pub name: String,
    pub version: String,
    pub download_url_linux: String,
    pub download_url_macos: String,
    pub download_url_windows: String,
}

/// Automates detection and installation of required external VPN binaries
pub struct BinaryManager {
    install_dir: PathBuf,
}

impl BinaryManager {
    pub fn new() -> Result<Self> {
        let install_dir = Self::get_install_directory()?;
        Ok(Self { install_dir })
    }

    /// Determines the local installation path for VPN binaries
    fn get_install_directory() -> Result<PathBuf> {
        // Priority: ~/.worldvpn/bin/ (user-space)
        if let Some(home) = std::env::var_os("HOME") {
            let dir = PathBuf::from(home).join(".worldvpn").join("bin");
            return Ok(dir);
        }

        // Fallback: system global bin
        Ok(PathBuf::from("/usr/local/bin"))
    }

    /// Checks if a required binary is available in the system PATH or local bin
    pub async fn is_installed(&self, binary_name: &str) -> bool {
        // Check system PATH
        if Command::new("which")
            .arg(binary_name)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return true;
        }

        // Check local install directory
        let local_path = self.install_dir.join(binary_name);
        local_path.exists()
    }

    /// Orchestrates automatic download and installation
    pub async fn auto_install(&self, spec: &BinarySpec) -> Result<PathBuf> {
        info!("🔽 Automatically installing {} v{}", spec.name, spec.version);

        fs::create_dir_all(&self.install_dir).await.map_err(|e| {
            VpnError::InvalidConfig(format!("Failed to create {}: {}", self.install_dir.display(), e))
        })?;

        let download_url = Self::get_platform_url(spec)?;
        
        let binary_path = self.download_binary(&spec.name, download_url).await?;

        // Ensure executable permissions on Unix systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&binary_path).await?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&binary_path, perms).await?;
        }

        self.verify_binary(&binary_path).await?;

        info!("✅ {} successfully installed in {:?}", spec.name, binary_path);
        Ok(binary_path)
    }

    fn get_platform_url(_spec: &BinarySpec) -> Result<String> {
        #[cfg(target_os = "linux")]
        return Ok(spec.download_url_linux.clone());

        #[cfg(target_os = "macos")]
        return Ok(spec.download_url_macos.clone());

        #[cfg(target_os = "windows")]
        return Ok(spec.download_url_windows.clone());

        #[cfg(target_os = "android")]
        return Err(VpnError::InvalidConfig("External binaries not supported on Android yet".into()));

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows", target_os = "android")))]
        Err(VpnError::InvalidConfig("Unsupported OS".into()))
    }

    async fn download_binary(&self, name: &str, url: String) -> Result<PathBuf> {
        info!("📥 Downloading from {}", url);

        let response = reqwest::get(&url).await.map_err(|e| {
            VpnError::ConnectionFailed(format!("Download failed: {}", e))
        })?;

        if !response.status().is_success() {
            return Err(VpnError::ConnectionFailed(format!(
                "HTTP {}: {}",
                response.status(),
                url
            )));
        }

        let bytes = response.bytes().await.map_err(|e| {
            VpnError::ConnectionFailed(format!("Failed to read bytes: {}", e))
        })?;

        let dest_path = self.install_dir.join(name);

        fs::write(&dest_path, bytes).await.map_err(|e| {
            VpnError::InvalidConfig(format!("Failed to write file: {}", e))
        })?;

        Ok(dest_path)
    }

    /// Validates that the installed binary executes correctly
    async fn verify_binary(&self, path: &PathBuf) -> Result<()> {
        let output = Command::new(path)
            .arg("--version")
            .output()
            .map_err(|e| VpnError::InvalidConfig(format!("Verification failed: {}", e)))?;

        if !output.status.success() {
            return Err(VpnError::InvalidConfig(
                "Installed binary is not functional".into(),
            ));
        }

        Ok(())
    }

    /// Resolves the absolute path for a specific binary
    pub fn get_binary_path(&self, name: &str) -> Option<PathBuf> {
        if let Ok(output) = Command::new("which").arg(name).output() {
            if output.status.success() {
                if let Ok(path_str) = String::from_utf8(output.stdout) {
                    return Some(PathBuf::from(path_str.trim()));
                }
            }
        }

        let local_path = self.install_dir.join(name);
        if local_path.exists() {
            return Some(local_path);
        }

        None
    }
}

/// Registry of supported external binaries and their release locations
pub fn get_binary_specs() -> Vec<BinarySpec> {
    vec![
        BinarySpec {
            name: "sslocal".to_string(),
            version: "1.18.0".to_string(),
            download_url_linux: "https://github.com/shadowsocks/shadowsocks-rust/releases/download/v1.18.0/shadowsocks-v1.18.0.x86_64-unknown-linux-gnu.tar.xz".to_string(),
            download_url_macos: "https://github.com/shadowsocks/shadowsocks-rust/releases/download/v1.18.0/shadowsocks-v1.18.0.x86_64-apple-darwin.tar.xz".to_string(),
            download_url_windows: "https://github.com/shadowsocks/shadowsocks-rust/releases/download/v1.18.0/shadowsocks-v1.18.0.x86_64-pc-windows-msvc.zip".to_string(),
        },
        BinarySpec {
            name: "hysteria".to_string(),
            version: "2.2.3".to_string(),
            download_url_linux: "https://github.com/apernet/hysteria/releases/download/app%2Fv2.2.3/hysteria-linux-amd64".to_string(),
            download_url_macos: "https://github.com/apernet/hysteria/releases/download/app%2Fv2.2.3/hysteria-darwin-amd64".to_string(),
            download_url_windows: "https://github.com/apernet/hysteria/releases/download/app%2Fv2.2.3/hysteria-windows-amd64.exe".to_string(),
        },
        BinarySpec {
            name: "v2ray".to_string(),
            version: "5.13.0".to_string(),
            download_url_linux: "https://github.com/v2fly/v2ray-core/releases/download/v5.13.0/v2ray-linux-64.zip".to_string(),
            download_url_macos: "https://github.com/v2fly/v2ray-core/releases/download/v5.13.0/v2ray-macos-64.zip".to_string(),
            download_url_windows: "https://github.com/v2fly/v2ray-core/releases/download/v5.13.0/v2ray-windows-64.zip".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_binary_detection() {
        let manager = BinaryManager::new().unwrap();
        
        // System binary check
        let has_ls = manager.is_installed("ls").await;
        assert!(has_ls);
        
        let has_fake = manager.is_installed("worldvpn_fake_binary_xyz").await;
        assert!(!has_fake);
    }
}
