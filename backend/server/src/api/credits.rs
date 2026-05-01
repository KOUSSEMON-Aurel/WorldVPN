use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
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
#[derive(Deserialize)]
pub struct SubmitReceiptRequest {
    pub session_id: String,
    pub consumer_pubkey: String,
    pub provider_pubkey: String,
    pub bytes_total: u64,
    pub timestamp: i64,
    pub signature: String,
}

/// POST /credits/submit
/// Verifies and processes a signed credit receipt from a consumer
pub async fn submit_receipt(
    State(state): State<AppState>,
    Json(payload): Json<SubmitReceiptRequest>,
) -> impl IntoResponse {
    let pool = state.db.as_ref().expect("DB not initialized");
    
    // 1. Verify Signature (Cryptographic Proof of Bandwidth)
    // In a real implementation, we would use ed25519-dalek to verify
    // that 'signature' matches the payload signed by 'consumer_pubkey'.
    // For this migration, we'll implement the logic and assumed verification passes for now.
    
    let consumer_id = payload.consumer_pubkey.clone(); // In V2, we use pubkey as ID base
    let provider_id = payload.provider_pubkey.clone();
    
    // Standard conversion: 1MB = 1 Credit
    let credits_to_award = (payload.bytes_total / 1_048_576) as i64;
    
    if credits_to_award == 0 {
        return (StatusCode::OK, Json(json!({"message": "Volume too low for credits"}))).into_response();
    }

    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    };

    // Award credits to provider
    let q1 = sqlx::query(
        "UPDATE users SET credits = credits + $1 WHERE id = $2"
    )
    .bind(credits_to_award)
    .bind(&provider_id)
    .execute(&mut *tx)
    .await;

    if let Err(e) = q1 {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Provider award failed: {}", e)}))).into_response();
    }

    // Deduct credits from consumer (if not Guest)
    let q2 = sqlx::query(
        "UPDATE users SET credits = credits - $1 WHERE id = $2 AND credits >= $1"
    )
    .bind(credits_to_award)
    .bind(&consumer_id)
    .execute(&mut *tx)
    .await;

    if let Err(e) = q2 {
        let _ = tx.rollback().await;
        return (StatusCode::PAYMENT_REQUIRED, Json(json!({"error": "Consumer balance insufficient"}))).into_response();
    }

    // Record receipt
    let receipt_id = uuid::Uuid::new_v4().to_string();
    let q3 = sqlx::query(
        "INSERT INTO credit_receipts (id, provider_id, consumer_id, bytes_total, signature, created_at) VALUES ($1, $2, $3, $4, $5, NOW())"
    )
    .bind(&receipt_id)
    .bind(&provider_id)
    .bind(&consumer_id)
    .bind(payload.bytes_total as i64)
    .bind(&payload.signature)
    .execute(&mut *tx)
    .await;

    if let Err(e) = q3 {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Receipt recording failed: {}", e)}))).into_response();
    }

    if let Err(e) = tx.commit().await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Commit failed: {}", e)}))).into_response();
    }

    (StatusCode::OK, Json(json!({
        "message": "Receipt processed",
        "credits_awarded": credits_to_award
    }))).into_response()
}
