//! Client HTTP pour l'API WorldVPN
//!
//! Gère l'authentification et l'obtention des configurations VPN depuis le serveur.

use crate::error::{Result, VpnError};
use crate::protocol::VpnProtocol;
use serde::{Deserialize, Serialize};

/// Client API
pub struct VpnApiClient {
    base_url: String,
    client: reqwest::Client,
}

#[derive(Serialize)]
struct ConnectRequest {
    protocol: VpnProtocol,
    username: String,
    public_key: Option<String>,
}

/// Response de connexion VPN
#[derive(Deserialize, Debug)]
pub struct ConnectionInfo {
    pub session_id: String,
    pub server_endpoint: String,
    pub assigned_ip: String,
    pub server_public_key: Option<String>,
}

/// Response du login
#[derive(Deserialize, Debug)]
pub struct LoginResponse {
    pub token: String,
    pub user_id: String,
    pub username: String,
}

impl VpnApiClient {
    /// Crée un nouveau client
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
        }
    }

    /// Login et récupération du JWT
    pub async fn login(&self, username: String, password: String) -> Result<LoginResponse> {
        let url = format!("{}/auth/login", self.base_url);
        
        let payload = serde_json::json!({
            "username": username,
            "password": password,
        });

        let response = self.client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| VpnError::ConnectionFailed(format!("Erreur login API: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(VpnError::ConnectionFailed(format!("Login échoué: {}", error_text)));
        }

        let login_info = response
            .json::<LoginResponse>()
            .await
            .map_err(|e| VpnError::Internal(format!("Invalid login response: {}", e)))?;

        Ok(login_info)
    }

    /// Demande une connexion VPN au serveur (avec JWT)
    pub async fn connect(
        &self,
        protocol: VpnProtocol,
        username: String,
        public_key: Option<String>,
        token: &str, // JWT token
    ) -> Result<ConnectionInfo> {
        let url = format!("{}/vpn/connect", self.base_url);
        
        let payload = ConnectRequest {
            protocol,
            username,
            public_key,
        };

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&payload)
            .send()
            .await
            .map_err(|e| VpnError::ConnectionFailed(format!("Erreur connexion API: {}", e)))?;


        if !response.status().is_success() {
            return Err(VpnError::ConnectionFailed(format!("API Error: {}", response.status())));
        }

        let info = response
            .json::<ConnectionInfo>()
            .await
            .map_err(|e| VpnError::Internal(format!("Invalid response: {}", e)))?;

        Ok(info)
    }
}
