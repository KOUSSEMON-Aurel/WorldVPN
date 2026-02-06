use axum::{extract::{State, Query}, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;

use crate::{auth::AuthUser, state::AppState};

#[derive(Serialize)]
pub struct BalanceResponse {
    pub credits: i64,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct TransactionResponse {
    pub id: String,
    pub amount: i64,
    pub transaction_type: String,
    pub description: Option<String>,
    pub created_at: Option<chrono::NaiveDateTime>,
}

#[derive(Deserialize)]
pub struct SyncTrafficRequest {
    pub shared_bytes: i64,
    pub consumed_bytes: i64,
}

/// GET /credits/balance
/// Returns the current credit balance for the authenticated user
pub async fn get_balance(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> impl IntoResponse {
    let pool = state.db.as_ref().expect("DB not initialized");

    let row = sqlx::query("SELECT credits FROM users WHERE id = $1")
        .bind(&user.sub)
        .fetch_optional(pool)
        .await;

    match row {
        Ok(Some(r)) => {
            let credits: i64 = r.try_get("credits").unwrap_or(0);
            (StatusCode::OK, Json(BalanceResponse { credits })).into_response()
        },
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({"error": "User not found"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    }
}

/// GET /credits/history
/// Retrieves the last 50 credit transactions for the user
pub async fn get_history(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> impl IntoResponse {
    let pool = state.db.as_ref().expect("DB not initialized");

    let rows = sqlx::query_as::<_, TransactionResponse>(
        "SELECT id, amount, transaction_type, description, created_at FROM credit_transactions WHERE user_id = $1 ORDER BY created_at DESC LIMIT 50"
    )
    .bind(&user.sub)
    .fetch_all(pool)
    .await;

    match rows {
        Ok(transactions) => (StatusCode::OK, Json(transactions)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    }
}

/// POST /credits/sync
/// Synchronizes local traffic consumption/sharing with the central server
pub async fn sync_traffic(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(payload): Json<SyncTrafficRequest>,
) -> impl IntoResponse {
    let pool = state.db.as_ref().expect("DB not initialized");
    
    // Standard conversion factor: 1 MB = 1 Credit
    const BYTES_PER_CREDIT: i64 = 1_048_576;
    let earned = (payload.shared_bytes / BYTES_PER_CREDIT) as i64;
    let spent = (payload.consumed_bytes / BYTES_PER_CREDIT) as i64;
    
    let net_change = earned - spent;

    if net_change == 0 {
         return (StatusCode::OK, Json(json!({"message": "No change", "credits_change": 0}))).into_response();
    }

    let transaction_type = if net_change >= 0 { "EARNED" } else { "SPENT" };
    let description = format!("Sync: Shared {} MB, Consumed {} MB", 
        payload.shared_bytes / BYTES_PER_CREDIT, 
        payload.consumed_bytes / BYTES_PER_CREDIT
    );

    // Atomically update balance and record transaction history
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    };

    let log_id = uuid::Uuid::new_v4().to_string();
    let q1 = sqlx::query(
        "INSERT INTO credit_transactions (id, user_id, amount, transaction_type, description) VALUES ($1, $2, $3, $4, $5)"
    )
    .bind(&log_id)
    .bind(&user.sub)
    .bind(net_change)
    .bind(transaction_type)
    .bind(&description)
    .execute(&mut *tx)
    .await;

    if let Err(e) = q1 {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Log failed: {}", e)}))).into_response();
    }

    let q2 = sqlx::query(
        "UPDATE users SET credits = credits + $1 WHERE id = $2"
    )
    .bind(net_change)
    .bind(&user.sub)
    .execute(&mut *tx)
    .await;

    if let Err(e) = q2 {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Update failed: {}", e)}))).into_response();
    }

    if let Err(e) = tx.commit().await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Commit failed: {}", e)}))).into_response();
    }

    (StatusCode::OK, Json(json!({
        "message": "Sync successful",
        "credits_change": net_change
    }))).into_response()
}
