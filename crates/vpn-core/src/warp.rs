//! Cloudflare WARP Integration
//! Handles automated device registration and WireGuard key management.

use serde::{Deserialize, Serialize};
use crate::error::{Result, VpnError};
use base64::{engine::general_purpose, Engine as _};
use chrono::Utc;
use x25519_dalek::{PublicKey, StaticSecret};

const WARP_API_BASE: &str = "https://api.cloudflareclient.com/v0a2158";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WarpConfig {
    pub id: String,
    pub token: String,
    pub private_key: String,
    pub public_key: String,
    pub peer_public_key: String,
    pub endpoint: String,
}

#[derive(Debug, Serialize)]
struct RegisterRequest {
    key: String,
    install_id: String,
    fcm_token: String,
    referrer: String,
    warp_enabled: bool,
    tos: String,
    #[serde(rename = "type")]
    device_type: String,
    model: String,
    locale: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct RegisterResponse {
    id: String,
    token: String,
    config: WarpNetworkConfig,
}

#[derive(Debug, Deserialize)]
struct WarpNetworkConfig {
    peers: Vec<WarpPeer>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct WarpPeer {
    public_key: String,
    endpoint: WarpEndpoint,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct WarpEndpoint {
    v4: String,
    v6: String,
    host: String,
}

pub async fn register_warp_device() -> Result<WarpConfig> {
    tracing::info!("Registering new Cloudflare WARP device...");

    // 1. Generate keys
    let secret = StaticSecret::random_from_rng(rand::thread_rng());
    let public = PublicKey::from(&secret);
    
    let private_b64 = general_purpose::STANDARD.encode(secret.to_bytes());
    let public_b64 = general_purpose::STANDARD.encode(public.as_bytes());

    // 2. Prepare request
    let client = reqwest::Client::new();
    let tos = Utc::now().to_rfc3339();
    
    let req = RegisterRequest {
        key: public_b64.clone(),
        install_id: "".to_string(),
        fcm_token: "".to_string(),
        referrer: "".to_string(),
        warp_enabled: true,
        tos,
        device_type: "Linux".to_string(),
        model: "WorldVPN-PC".to_string(),
        locale: "en_US".to_string(),
    };

    // 3. Send registration
    let resp = client.post(format!("{}/reg", WARP_API_BASE))
        .json(&req)
        .send()
        .await
        .map_err(|e| VpnError::ConnectionFailed(format!("WARP Registration failed: {}", e)))?;

    if !resp.status().is_success() {
        let err_body = resp.text().await.unwrap_or_default();
        return Err(VpnError::ProtocolError(format!("WARP API Error: {}", err_body)));
    }

    let data: RegisterResponse = resp.json().await
        .map_err(|e| VpnError::ProtocolError(format!("Invalid WARP response: {}", e)))?;

    // 4. Extract endpoint & peer key
    let peer = data.config.peers.first()
        .ok_or_else(|| VpnError::ProtocolError("No peers returned from WARP API".into()))?;
    
    let endpoint = peer.endpoint.v4.clone();
    let peer_public_key = peer.public_key.clone();

    tracing::info!("WARP Device Registered Successfully: {}", data.id);

    Ok(WarpConfig {
        id: data.id,
        token: data.token,
        private_key: private_b64,
        public_key: public_b64,
        peer_public_key,
        endpoint,
    })
}
