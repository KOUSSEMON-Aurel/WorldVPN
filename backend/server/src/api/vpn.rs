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

    let credits: i64 = match balance_check {
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

    let (node_id, node_country, endpoint) = match node_result {
        Ok(Some(row)) => {
            let nid: String = row.get("id");
            let country: String = row.get("country_code");
            // In production: decrypt/resolve the actual IP
            let ep = format!("peer-{}.worldvpn.net:51820", &nid[..8]);
            (Some(nid), Some(country), ep)
        }
        _ => {
            // Fallback to central server if no P2P node available
            tracing::warn!("No P2P nodes available, using fallback server");
            (None, None, "fallback.worldvpn.net:51820".to_string())
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

    // 4. If using P2P node, increment connection count and create peer session
    if let Some(ref nid) = node_id {
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

    tracing::info!("Session created: {} -> {} via {:?}", session_id, endpoint, payload.protocol);

    let response = ConnectResponse {
        session_id,
        server_endpoint: endpoint,
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
