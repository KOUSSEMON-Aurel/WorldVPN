//! Client HTTP pour l'API WorldVPN
//!
//! Gère l'authentification et l'obtention des configurations VPN depuis le serveur.

use crate::error::{Result, VpnError};
use crate::protocol::VpnProtocol;
use serde::{Deserialize, Serialize};

/// Client API
#[derive(Clone)]
pub struct VpnApiClient {
    base_url: String,
    client: reqwest::Client,
}

#[derive(Serialize)]
struct ConnectRequest {
    protocol: VpnProtocol,
    public_key: Option<String>,
    preferred_country: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct BalanceResponse {
    pub credits: i64,
}



/// Response de connexion VPN
#[derive(Deserialize, Serialize, Debug)]
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

    /// Login anonyme (V2) via clé publique Ed25519 et signature (Proof of Work/Identity)
    pub async fn login_with_identity(&self, public_key: String, signature: String, timestamp: String) -> Result<LoginResponse> {
        let url = format!("{}/auth/identity", self.base_url);
        
        let payload = serde_json::json!({
            "public_key": public_key,
            "signature": signature,
            "timestamp": timestamp,
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
        public_key: Option<String>,
        preferred_country: Option<String>,
        token: &str,
    ) -> Result<ConnectionInfo> {
        let url = format!("{}/vpn/connect", self.base_url);
        
        let payload = ConnectRequest {
            protocol,
            public_key,
            preferred_country,
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

    /// Enregistre ce terminal comme nœud de partage sur le réseau
    pub async fn register_node(
        &self,
        token: &str,
        country_code: &str,
        city: Option<String>,
        nat_type: String,
        public_endpoint: Option<String>,
        protocols: Vec<String>,
    ) -> Result<String> {
        let url = format!("{}/nodes/register", self.base_url);
        
        let payload = serde_json::json!({
            "country_code": country_code,
            "city": city,
            "nat_type": Some(nat_type),
            "public_endpoint": public_endpoint,
            "protocols": protocols,
            "available_bandwidth_mbps": 50,
        });

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&payload)
            .send()
            .await
            .map_err(|e| VpnError::ConnectionFailed(format!("Registration failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let err = response.text().await.unwrap_or_default();
            return Err(VpnError::ConnectionFailed(format!("Registration error ({}): {}", status, err)));
        }

        let data: serde_json::Value = response.json().await
            .map_err(|e| VpnError::Internal(format!("Invalid registration response: {}", e)))?;
        
        let node_id = data["node_id"].as_str().ok_or_else(|| VpnError::Internal("Missing node_id".into()))?;
        
        Ok(node_id.to_string())
    }

    /// Envoie un heartbeat pour maintenir le nœud en ligne et mettre à jour ses métadonnées P2P
    pub async fn heartbeat(
        &self,
        token: &str,
        nat_type: Option<String>,
        current_connections: Option<i32>,
        public_endpoint: Option<String>,
    ) -> Result<()> {
        let url = format!("{}/nodes/heartbeat", self.base_url);
        
        let payload = serde_json::json!({
            "nat_type": nat_type,
            "current_connections": current_connections,
            "public_endpoint": public_endpoint,
        });

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&payload)
            .send()
            .await
            .map_err(|e| VpnError::ConnectionFailed(format!("Heartbeat failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(VpnError::ConnectionFailed(format!("Heartbeat error: {}", response.status())));
        }

        Ok(())
    }

    /// Récupère la liste des nœuds publics (VPNGate) optimisés par le serveur
    pub async fn fetch_public_nodes(&self) -> Result<serde_json::Value> {
        let url = format!("{}/nodes/public", self.base_url);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| VpnError::ConnectionFailed(format!("Public nodes fetch failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(VpnError::ConnectionFailed(format!("Public nodes error: {}", response.status())));
        }

        let nodes = response
            .json::<serde_json::Value>()
            .await
            .map_err(|e| VpnError::Internal(format!("Invalid public nodes response: {}", e)))?;

        Ok(nodes)
    }

    pub async fn fetch_balance(&self, token: &str) -> Result<i64> {
        let url = format!("{}/credits/balance", self.base_url);
        let resp = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| VpnError::ConnectionFailed(format!("Balance fetch failed: {}", e)))?;

        if !resp.status().is_success() {
            return Ok(0);
        }

        let bal = resp
            .json::<BalanceResponse>()
            .await
            .map_err(|e| VpnError::Internal(format!("Invalid balance response: {}", e)))?;
        Ok(bal.credits)
    }
}

