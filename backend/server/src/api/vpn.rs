use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use serde_json::json;
use vpn_core::protocol::VpnProtocol;
use sqlx::Row;

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct ConnectRequest {
    pub protocol: VpnProtocol,
    pub username: String,
    pub public_key: Option<String>,
    pub preferred_country: Option<String>,
}

#[derive(Serialize)]
pub struct ConnectResponse {
    pub session_id: String,
    pub server_endpoint: String,
    pub assigned_ip: String,
    pub server_public_key: Option<String>,
    pub node_country: Option<String>,
}

/// POST /vpn/connect - Connect to VPN via P2P node or fallback server
pub async fn connect(
    State(state): State<AppState>,
    user: crate::auth::AuthUser,
    Json(payload): Json<ConnectRequest>,
) -> impl IntoResponse {
    tracing::info!("Connection request from user: {} (JWT: {})", payload.username, user.0.sub);

    let pool = state.db.as_ref().expect("DB not initialized");

    // 1. Check user balance (must have credits to connect)
    let balance_check = sqlx::query("SELECT credits FROM users WHERE id = $1")
        .bind(&user.0.sub)
        .fetch_optional(pool)
        .await;

    let credits: i32 = match balance_check {
        Ok(Some(row)) => row.get("credits"),
        Ok(None) => 0,
        Err(_) => 0,
    };

    if credits < 10 {
        return (StatusCode::PAYMENT_REQUIRED, Json(json!({
            "error": "Insufficient credits",
            "credits": credits,
            "required": 10,
            "hint": "Share bandwidth to earn credits, or upgrade to premium"
        }))).into_response();
    }

    // 2. Find best available P2P node
    let preferred = payload.preferred_country.as_deref().unwrap_or("*");
    
    let node_query = if preferred == "*" {
        sqlx::query(
            r#"SELECT id, country_code, public_ip_hash 
               FROM nodes 
               WHERE is_online = TRUE 
                 AND current_connections < max_connections
                 AND user_id != $1
               ORDER BY reputation_score DESC, avg_latency_ms ASC
               LIMIT 1"#
        )
        .bind(&user.0.sub)
    } else {
        sqlx::query(
            r#"SELECT id, country_code, public_ip_hash 
               FROM nodes 
               WHERE is_online = TRUE 
                 AND current_connections < max_connections
                 AND user_id != $1
                 AND country_code = $2
               ORDER BY reputation_score DESC, avg_latency_ms ASC
               LIMIT 1"#
        )
        .bind(&user.0.sub)
        .bind(preferred)
    };

    let node_result = node_query.fetch_optional(pool).await;

    let (node_id, node_country, endpoint, is_fallback) = match node_result {
        Ok(Some(row)) => {
            let nid: String = row.get("id");
            let country: String = row.get("country_code");
            let public_endpoint: Option<String> = row.try_get("public_endpoint").ok();
            
            // Use real STUN endpoint if available, otherwise fallback to simulated (for legacy)
            let ep = public_endpoint.unwrap_or_else(|| {
                let simulated_ip = format!("198.51.100.{}", (nid.as_bytes()[0] % 200) + 10);
                format!("{}:51820", simulated_ip)
            });
            
            (Some(nid), Some(country), ep, false)
        }
        _ => {
            // Fallback to VPNGate nodes stored in the database
            tracing::warn!("No P2P nodes available, attempting VPNGate fallback");
            let fallback_query = sqlx::query(
                "SELECT id, country_code, public_ip_hash FROM nodes WHERE id LIKE 'vpngate_%' ORDER BY RANDOM() LIMIT 1"
            ).fetch_optional(pool).await;

            match fallback_query {
                Ok(Some(row)) => {
                    let nid: String = row.get("id");
                    let country: String = row.get("country_code");
                    // Extract IP from vpngate_IP_ADDRESS pattern
                    let parts: Vec<&str> = nid.split('_').collect();
                    let ip = if parts.len() == 5 {
                        format!("{}.{}.{}.{}", parts[1], parts[2], parts[3], parts[4])
                    } else {
                        "fallback.worldvpn.net".to_string()
                    };
                    let ep = format!("{}:1194", ip);
                    (Some(nid), Some(country), ep, true)
                }
                _ => {
                    tracing::error!("Absolute failure: Neither P2P nor VPNGate nodes available.");
                    (None, None, "error.worldvpn.net:0".to_string(), true)
                }
            }
        }
    };

    // 3. Create session
    let session_id = uuid::Uuid::new_v4().to_string();
    let virtual_ip = format!("10.0.0.{}", rand::random::<u8>());

    let proto_str = format!("{:?}", payload.protocol);
    let credentials = match payload.protocol {
        VpnProtocol::WireGuard | VpnProtocol::WireGuardObfuscated => {
            Some("ServerPublicKey_BASE64_PLACEHOLDER".to_string())
        }
        VpnProtocol::Shadowsocks => {
            Some("chacha20-ietf-poly1305:worldvpn-secure-password".to_string())
        }
        _ => None,
    };

    // Insert session
    let _ = sqlx::query(
        "INSERT INTO sessions (id, user_id, protocol, virtual_ip, endpoint) VALUES ($1, $2, $3, $4, $5)"
    )
    .bind(&session_id)
    .bind(&user.0.sub)
    .bind(&proto_str)
    .bind(&virtual_ip)
    .bind(&endpoint)
    .execute(pool)
    .await;

    // 4. If using P2P node (and not a fallback), increment connection count and create peer session
    if let Some(ref nid) = node_id {
        if !is_fallback {
            let _ = sqlx::query("UPDATE nodes SET current_connections = current_connections + 1 WHERE id = $1")
                .bind(nid)
                .execute(pool)
                .await;

            // Create transparency record
            let peer_session_id = uuid::Uuid::new_v4().to_string();
            let client_country = "XX"; // In production: detect from IP
            let client_hash = format!("hash_{}", &user.0.sub[..8]);

            let _ = sqlx::query(
                r#"INSERT INTO peer_sessions 
                   (id, node_id, node_owner_id, client_country, client_id_hash, traffic_type)
                   SELECT $1, $2, user_id, $3, $4, 'browsing'
                   FROM nodes WHERE id = $2"#
            )
            .bind(&peer_session_id)
            .bind(nid)
            .bind(client_country)
            .bind(&client_hash)
            .execute(pool)
            .await;
        }
    }

    tracing::info!("Session created: {} -> {} via {:?}", session_id, endpoint, payload.protocol);

    // Chiffrement E2E de l'endpoint si une clé publique est fournie (Phase 4)
    let final_endpoint = if let Some(ref pubkey) = payload.public_key {
        match vpn_core::crypto::IdentityKey::encrypt_for_identity(&endpoint, pubkey) {
            Ok(enc) => {
                tracing::info!("Endpoint chiffré pour le client {}", pubkey);
                format!("e2e:{}", enc)
            },
            Err(e) => {
                tracing::error!("Erreur chiffrement endpoint: {}", e);
                endpoint
            }
        }
    } else {
        endpoint
    };

    let response = ConnectResponse {
        session_id,
        server_endpoint: final_endpoint,
        assigned_ip: virtual_ip,
        server_public_key: credentials,
        node_country,
    };

    (StatusCode::OK, Json(response)).into_response()
}

/// POST /vpn/disconnect - End VPN session
pub async fn disconnect(
    State(state): State<AppState>,
    user: crate::auth::AuthUser,
    Json(payload): Json<DisconnectRequest>,
) -> impl IntoResponse {
    let pool = state.db.as_ref().expect("DB not initialized");

    // End the session
    let _ = sqlx::query("DELETE FROM sessions WHERE id = $1 AND user_id = $2")
        .bind(&payload.session_id)
        .bind(&user.0.sub)
        .execute(pool)
        .await;

    // Mark peer session as ended
    let _ = sqlx::query(
        r#"UPDATE peer_sessions 
           SET is_active = FALSE, ended_at = CURRENT_TIMESTAMP
           WHERE client_id_hash LIKE $1 AND is_active = TRUE"#
    )
    .bind(format!("hash_{}%", &user.0.sub[..8]))
    .execute(pool)
    .await;

    (StatusCode::OK, Json(json!({
        "status": "disconnected",
        "session_id": payload.session_id
    }))).into_response()
}

#[derive(Deserialize)]
pub struct DisconnectRequest {
    pub session_id: String,
}
