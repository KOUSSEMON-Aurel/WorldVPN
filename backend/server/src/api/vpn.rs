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

#[derive(Deserialize)]
pub struct ConnectRequest {
    pub protocol: VpnProtocol,
    // Pour l'instant on simule l'auth
    pub username: String,
    pub public_key: Option<String>,
}

#[derive(Serialize)]
pub struct ConnectResponse {
    pub session_id: String,
    pub server_endpoint: String,
    pub assigned_ip: String,
    pub server_public_key: Option<String>,
}

pub async fn connect(
    State(state): State<AppState>,
    user: crate::auth::AuthUser, // Protection JWT
    Json(payload): Json<ConnectRequest>,
) -> impl IntoResponse {
    tracing::info!("Demande de connexion authentifiée pour user: {} (JWT user_id: {})", payload.username, user.0.sub);

    let pool = state.db.as_ref().expect("Base de données non initialisée");

    // 1. Gestion Utilisateur (Requête préparée et sécurisée)
    // On utilise RETURNING pour récupérer l'ID en une seule requête si possible, ou SELECT fallback
    // Note: SQLx gère les Prepared Statements automatiquement avec query() + bind()
    
    // Postgres utilise $1, $2, etc. au lieu de ?
    let user = sqlx::query("SELECT id, credits FROM users WHERE username = $1")
        .bind(&payload.username)
        .fetch_optional(pool)
        .await;

    let user_id = match user {
        Ok(Some(row)) => {
            use sqlx::Row;
            row.get::<String, _>("id")
        }
        Ok(None) => {
            // Création nouvel utilisateur
            let new_id = uuid::Uuid::new_v4().to_string();
            
            // Hashage sécurisé du mot de passe
            let salt = password_hash::SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
            let argon2 = argon2::Argon2::default();
            
            // Gestion erreur hashage manuelle sans ? pour éviter problème de type de retour
            let password_hash = match argon2::PasswordHasher::hash_password(&argon2, payload.username.as_bytes(), &salt) {
                Ok(h) => h.to_string(),
                Err(e) => {
                    tracing::error!("Erreur hashage: {}", e);
                    return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Encryption error"}))).into_response();
                }
            };

            // Utilisation explicite des colonnes pour éviter injection si schéma change
            let res = sqlx::query("INSERT INTO users (id, username, password_hash) VALUES ($1, $2, $3)")
                .bind(&new_id)
                .bind(&payload.username)
                .bind(&password_hash)
                .execute(pool)
                .await;
            
            if let Err(e) = res {
                tracing::error!("Erreur création user: {:?}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"}))).into_response();
            }
            new_id
        }
        Err(e) => {
            tracing::error!("Erreur DB user: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"}))).into_response();
        }
    };

    // 2. Création Session
    let session_id = uuid::Uuid::new_v4().to_string();
    let virtual_ip = format!("10.0.0.{}", rand::random::<u8>()); 

    let (endpoint, proto_str, credentials) = match payload.protocol {
        VpnProtocol::WireGuard => (
            "127.0.0.1:51820", 
            "WireGuard", 
            Some("ServerPublicKeyPlaceholder123456".to_string())
        ),
        VpnProtocol::Shadowsocks => (
            "127.0.0.1:8388", 
            "Shadowsocks", 
            Some("chacha20-ietf-poly1305:worldvpn-secret-password".to_string())
        ),
        _ => ("127.0.0.1:443", "OpenVPN", None),
    };

    // Insertion sécurisée session
    let res = sqlx::query("INSERT INTO sessions (id, user_id, protocol, virtual_ip, endpoint) VALUES ($1, $2, $3, $4, $5)")
        .bind(&session_id)
        .bind(&user_id)
        .bind(proto_str)
        .bind(&virtual_ip)
        .bind(endpoint)
        .execute(pool)
        .await;

    if let Err(e) = res {
        tracing::error!("Erreur création session: {:?}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Session creation failed"}))).into_response();
    }
    
    tracing::info!("✅ Session créée {} pour user {} via {}", session_id, payload.username, proto_str);

    // Réponse
    let response = ConnectResponse {
        session_id,
        server_endpoint: endpoint.to_string(),
        assigned_ip: virtual_ip,
        server_public_key: credentials, // Utilisé pour PubKey WG ou Password SS
    };

    (StatusCode::OK, Json(serde_json::to_value(response).unwrap())).into_response()
}
