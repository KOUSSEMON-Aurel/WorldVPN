use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Standard JWT payload
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,      // User ID (subject)
    pub username: String, // Human-readable identifier
    pub exp: i64,         // Expiration timestamp
    pub iat: i64,         // Issued at timestamp
}

impl Claims {
    /// Constructs a new claim valid for 24 hours
    pub fn new(user_id: String, username: String) -> Self {
        let now = Utc::now();
        Self {
            sub: user_id,
            username,
            iat: now.timestamp(),
            exp: (now + Duration::hours(24)).timestamp(),
        }
    }
}

/// Signs a new JWT for the given user
pub fn create_jwt(user_id: String, username: String) -> Result<String, jsonwebtoken::errors::Error> {
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "default_secret".to_string());
    let claims = Claims::new(user_id, username);
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

/// Validates and decodes a base64 encoded JWT string
pub fn verify_jwt(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "default_secret".to_string());
    let validation = Validation::default();
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )?;
    Ok(token_data.claims)
}

/// Axum extractor that mandates a valid Bearer token for protected routes
#[derive(Debug, Clone)]
pub struct AuthUser(pub Claims);

#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Retrieve Authorization header
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({"error": "Missing Authorization header"})),
                )
                    .into_response()
            })?;

        // Enforce "Bearer <token>" format
        if !auth_header.starts_with("Bearer ") {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid Authorization format"})),
            )
                .into_response());
        }

        let token = &auth_header[7..];

        // Validate token integrity and expiration
        match verify_jwt(token) {
            Ok(claims) => Ok(AuthUser(claims)),
            Err(_) => Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid or expired token"})),
            )
                .into_response()),
        }
    }
}
