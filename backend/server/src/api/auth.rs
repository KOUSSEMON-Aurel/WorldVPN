//! Endpoints d'authentification

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;

use crate::{auth::create_jwt, state::AppState};

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user_id: String,
    pub username: String,
}

/// POST /auth/login
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> impl IntoResponse {
    let pool = state.db.as_ref().expect("DB non initialisée");

    // Récupération de l'utilisateur
    let user = sqlx::query("SELECT id, username, password_hash FROM users WHERE username = $1")
        .bind(&payload.username)
        .fetch_optional(pool)
        .await;

    match user {
        Ok(Some(row)) => {
            let user_id: String = row.get("id");
            let username: String = row.get("username");
            let password_hash: String = row.get("password_hash");

            // Vérification du mot de passe avec Argon2
            use argon2::{Argon2, PasswordHash, PasswordVerifier};
            let parsed_hash = match PasswordHash::new(&password_hash) {
                Ok(h) => h,
                Err(_) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": "Invalid password hash stored"})),
                    )
                        .into_response()
                }
            };

            if Argon2::default()
                .verify_password(payload.password.as_bytes(), &parsed_hash)
                .is_err()
            {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({"error": "Invalid credentials"})),
                )
                    .into_response();
            }

            // Génération du JWT
            match create_jwt(user_id.clone(), username.clone()) {
                Ok(token) => {
                    let response = LoginResponse {
                        token,
                        user_id,
                        username,
                    };
                    (StatusCode::OK, Json(response)).into_response()
                }
                Err(e) => {
                    tracing::error!("Erreur JWT: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": "Token generation failed"})),
                    )
                        .into_response()
                }
            }
        }
        Ok(None) => (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Invalid credentials"})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Erreur DB: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
                .into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct IdentityLoginRequest {
    pub public_key: String,
    pub signature: String,
    pub timestamp: String,
}

/// POST /auth/identity
pub async fn identity_login(
    State(state): State<AppState>,
    Json(payload): Json<IdentityLoginRequest>,
) -> impl IntoResponse {
    let pool = state.db.as_ref().expect("DB non initialisée");

    // 1. Vérification de la signature
    use vpn_core::crypto::IdentityKey;
    if !IdentityKey::verify_signature(&payload.public_key, &payload.timestamp, &payload.signature) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Invalid signature"})),
        )
            .into_response();
    }

    // 2. Vérification de la fraîcheur du timestamp (anti-replay)
    // On accepte une fenêtre de 5 minutes
    let ts: i64 = payload.timestamp.parse().unwrap_or(0);
    let now = chrono::Utc::now().timestamp();
    if (now - ts).abs() > 300 {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Challenge expired or invalid timestamp"})),
        )
            .into_response();
    }

    // 3. Recherche ou création de l'utilisateur anonyme
    let user = sqlx::query("SELECT id FROM users WHERE ed25519_pubkey = $1")
        .bind(&payload.public_key)
        .fetch_optional(pool)
        .await;

    let user_id = match user {
        Ok(Some(row)) => row.get::<String, _>("id"),
        Ok(None) => {
            // Création automatique pour les nouveaux anonymes
            let new_id = uuid::Uuid::new_v4().to_string();
            let res = sqlx::query(
                "INSERT INTO users (id, ed25519_pubkey, credits) VALUES ($1, $2, $3)",
            )
            .bind(&new_id)
            .bind(&payload.public_key)
            .bind(50) // 50 crédits pour les anonymes
            .execute(pool)
            .await;

            if let Err(e) = res {
                tracing::error!("Erreur création user anonyme: {:?}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": "Auto-registration failed"})),
                )
                    .into_response();
            }
            new_id
        }
        Err(e) => {
            tracing::error!("Erreur DB identity: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
                .into_response();
        }
    };

    // 4. Update last active
    let _ = sqlx::query("UPDATE users SET last_active = CURRENT_TIMESTAMP WHERE id = $1")
        .bind(&user_id)
        .execute(pool)
        .await;

    // 5. Génération du JWT
    match create_jwt(user_id.clone(), payload.public_key.clone()) {
        Ok(token) => {
            let response = LoginResponse {
                token,
                user_id,
                username: payload.public_key, // On utilise la pubkey comme username pour le JWT
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            tracing::error!("Erreur JWT identity: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Token generation failed"})),
            )
                .into_response()
        }
    }
}
