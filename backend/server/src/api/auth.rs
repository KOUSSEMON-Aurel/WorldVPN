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
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub email: Option<String>,
}

#[derive(Serialize)]
pub struct RegisterResponse {
    pub user_id: String,
    pub username: String,
    pub message: String,
}

/// POST /auth/register
pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> impl IntoResponse {
    let pool = state.db.as_ref().expect("DB non initialisée");

    // Vérification unicité username
    let existing = sqlx::query("SELECT id FROM users WHERE username = $1")
        .bind(&payload.username)
        .fetch_optional(pool)
        .await;

    match existing {
        Ok(Some(_)) => {
            return (
                StatusCode::CONFLICT,
                Json(json!({"error": "Username already exists"})),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Erreur DB check: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
                .into_response();
        }
        Ok(None) => {}
    }

    // Hashage du mot de passe
    let salt = password_hash::SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
    let argon2 = argon2::Argon2::default();

    let password_hash = match argon2::PasswordHasher::hash_password(
        &argon2,
        payload.password.as_bytes(),
        &salt,
    ) {
        Ok(h) => h.to_string(),
        Err(e) => {
            tracing::error!("Erreur hashage: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Encryption error"})),
            )
                .into_response();
        }
    };

    // Création de l'utilisateur
    let user_id = uuid::Uuid::new_v4().to_string();
    let res = sqlx::query(
        "INSERT INTO users (id, username, password_hash, credits) VALUES ($1, $2, $3, $4)",
    )
    .bind(&user_id)
    .bind(&payload.username)
    .bind(&password_hash)
    .bind(100) // 100 crédits de départ
    .execute(pool)
    .await;

    match res {
        Ok(_) => {
            tracing::info!("✅ Nouvel utilisateur créé: {} ({})", payload.username, user_id);
            let response = RegisterResponse {
                user_id: user_id.clone(),
                username: payload.username.clone(),
                message: format!("Bienvenue {} ! Votre compte a été créé avec succès.", payload.username),
            };
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => {
            tracing::error!("Erreur création user: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "User creation failed"})),
            )
                .into_response()
        }
    }
}
