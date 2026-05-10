use axum::{extract::State, Json, response::IntoResponse, http::StatusCode};
use serde_json::json;
use crate::state::AppState;
use sqlx::Row;

pub async fn get_vpnbook_password(State(state): State<AppState>) -> impl IntoResponse {
    let pool = state.db.as_ref().expect("DB not initialized");
    
    let row = sqlx::query(
        "SELECT value FROM public_provider_metadata WHERE provider_name = 'VPNBOOK' AND key = 'password' LIMIT 1"
    )
    .fetch_optional(pool)
    .await;
    
    match row {
        Ok(Some(r)) => {
            let password: String = r.get("value");
            (StatusCode::OK, Json(json!({ "password": password }))).into_response()
        },
        _ => (StatusCode::NOT_FOUND, Json(json!({ "error": "VPNBook password not found" }))).into_response(),
    }
}
