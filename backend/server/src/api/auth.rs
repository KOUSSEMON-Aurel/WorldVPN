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
    if !IdentityKey::verify_signature_from_hex(&payload.public_key, &payload.timestamp, &payload.signature) {
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
#[derive(Deserialize)]
pub struct MigrateCreditsRequest {
    pub old_public_key: String,
    pub new_public_key: String,
    pub signature_old: String, // Signature de (new_public_key) par la clé privée de old_public_key
}

/// POST /auth/migrate
/// Migrations anonyme de crédits entre deux identités
pub async fn migrate_credits(
    State(state): State<AppState>,
    Json(payload): Json<MigrateCreditsRequest>,
) -> impl IntoResponse {
    let pool = state.db.as_ref().expect("DB non initialisée");

    // 1. Vérification de la signature (Preuve de possession de l'ancienne clé)
    use vpn_core::crypto::IdentityKey;
    if !IdentityKey::verify_signature_from_hex(&payload.old_public_key, &payload.new_public_key, &payload.signature_old) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Invalid migration signature"})),
        )
            .into_response();
    }

    // 2. Transaction atomique pour le transfert
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    };

    // Récupérer le solde de l'ancienne clé
    let old_user = sqlx::query("SELECT credits FROM users WHERE ed25519_pubkey = $1")
        .bind(&payload.old_public_key)
        .fetch_optional(&mut *tx)
        .await;

    let credits = match old_user {
        Ok(Some(row)) => row.get::<i64, _>("credits"),
        Ok(None) => return (StatusCode::NOT_FOUND, Json(json!({"error": "Old identity not found"}))).into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    };

    // Supprimer (ou désactiver) l'ancienne clé pour éviter le double transfert
    let _ = sqlx::query("DELETE FROM users WHERE ed25519_pubkey = $1")
        .bind(&payload.old_public_key)
        .execute(&mut *tx)
        .await;

    // Ajouter les crédits à la nouvelle clé (ou créer si elle n'existe pas)
    let new_user = sqlx::query("SELECT id FROM users WHERE ed25519_pubkey = $1")
        .bind(&payload.new_public_key)
        .fetch_optional(&mut *tx)
        .await;

    match new_user {
        Ok(Some(row)) => {
            let user_id: String = row.get("id");
            let _ = sqlx::query("UPDATE users SET credits = credits + $1 WHERE id = $2")
                .bind(credits)
                .bind(&user_id)
                .execute(&mut *tx)
                .await;
        },
        Ok(None) => {
            let new_id = uuid::Uuid::new_v4().to_string();
            let _ = sqlx::query("INSERT INTO users (id, ed25519_pubkey, credits) VALUES ($1, $2, $3)")
                .bind(&new_id)
                .bind(&payload.new_public_key)
                .bind(credits)
                .execute(&mut *tx)
                .await;
        },
        Err(e) => {
            let _ = tx.rollback().await;
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response();
        }
    }

    if let Err(e) = tx.commit().await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Commit failed: {}", e)}))).into_response();
    }

    (StatusCode::OK, Json(json!({
        "message": "Credits successfully migrated",
        "migrated_credits": credits
    }))).into_response()
}
