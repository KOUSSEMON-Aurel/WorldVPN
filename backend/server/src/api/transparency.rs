use axum::{
    extract::{State, Query},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;

use crate::{auth::AuthUser, state::AppState};

/// Real-time session visible to the node owner (transparency dashboard)
#[derive(Serialize)]
pub struct ActiveSession {
    pub session_id: String,
    pub client_country: String,
    pub traffic_type: String,
    pub bytes_transferred: i64,
    pub duration_seconds: i64,
    pub credits_earned: i32,
}

/// GET /transparency/sessions - Get active sessions on your node (what's using YOUR bandwidth)
pub async fn get_active_sessions(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> impl IntoResponse {
    let pool = state.db.as_ref().expect("DB not initialized");

    let result = sqlx::query(
        r#"SELECT ps.id, ps.client_country, ps.traffic_type, ps.bytes_transferred,
                  ps.credits_earned,
                  EXTRACT(EPOCH FROM (CURRENT_TIMESTAMP - ps.started_at))::INTEGER as duration_secs
           FROM peer_sessions ps
           WHERE ps.node_owner_id = $1 AND ps.is_active = TRUE
           ORDER BY ps.started_at DESC
           LIMIT 50"#
    )
    .bind(&user.sub)
    .fetch_all(pool)
    .await;

    match result {
        Ok(rows) => {
            let sessions: Vec<ActiveSession> = rows.iter().map(|row| {
                ActiveSession {
                    session_id: row.get("id"),
                    client_country: row.get("client_country"),
                    traffic_type: row.get("traffic_type"),
                    bytes_transferred: row.get("bytes_transferred"),
                    duration_seconds: row.get::<i32, _>("duration_secs") as i64,
                    credits_earned: row.get("credits_earned"),
                }
            }).collect();

            let total_bytes: i64 = sessions.iter().map(|s| s.bytes_transferred).sum();
            let total_credits: i32 = sessions.iter().map(|s| s.credits_earned).sum();

            (StatusCode::OK, Json(json!({
                "active_sessions": sessions,
                "count": sessions.len(),
                "total_bytes_shared": total_bytes,
                "total_credits_earning": total_credits,
                "formatted_bandwidth": format_bytes(total_bytes)
            }))).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to fetch sessions: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({
                "error": "Failed to fetch active sessions"
            }))).into_response()
        }
    }
}

/// GET /transparency/history - Historical sessions (last 7 days)
pub async fn get_session_history(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Query(params): Query<HistoryQuery>,
) -> impl IntoResponse {
    let pool = state.db.as_ref().expect("DB not initialized");
    let days = params.days.unwrap_or(7).min(30);

    let result = sqlx::query(
        r#"SELECT ps.client_country, ps.traffic_type, ps.bytes_transferred,
                  ps.credits_earned, ps.started_at, ps.ended_at
           FROM peer_sessions ps
           WHERE ps.node_owner_id = $1 
             AND ps.started_at > CURRENT_TIMESTAMP - INTERVAL '1 day' * $2
           ORDER BY ps.started_at DESC
           LIMIT 200"#
    )
    .bind(&user.sub)
    .bind(days)
    .fetch_all(pool)
    .await;

    match result {
        Ok(rows) => {
            let history: Vec<serde_json::Value> = rows.iter().map(|row| {
                json!({
                    "country": row.get::<String, _>("client_country"),
                    "traffic_type": row.get::<String, _>("traffic_type"),
                    "bytes": row.get::<i64, _>("bytes_transferred"),
                    "credits": row.get::<i32, _>("credits_earned"),
                    "started": row.get::<chrono::NaiveDateTime, _>("started_at").to_string()
                })
            }).collect();

            let total_bytes: i64 = rows.iter().map(|r| r.get::<i64, _>("bytes_transferred")).sum();
            let total_credits: i32 = rows.iter().map(|r| r.get::<i32, _>("credits_earned")).sum();

            (StatusCode::OK, Json(json!({
                "history": history,
                "period_days": days,
                "total_sessions": history.len(),
                "total_bytes_shared": total_bytes,
                "total_credits_earned": total_credits,
                "formatted_total": format_bytes(total_bytes)
            }))).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({
                "error": e.to_string()
            }))).into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct HistoryQuery {
    pub days: Option<i32>,
}

/// GET /transparency/stats - Aggregated statistics for dashboard
pub async fn get_stats(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> impl IntoResponse {
    let pool = state.db.as_ref().expect("DB not initialized");

    // Multiple queries for comprehensive stats
    let node_info = sqlx::query(
        r#"SELECT is_online, reputation_score, current_connections, 
                  (SELECT COUNT(*) FROM peer_sessions WHERE node_owner_id = $1) as total_sessions,
                  (SELECT COALESCE(SUM(bytes_transferred), 0) FROM peer_sessions WHERE node_owner_id = $1) as total_bytes
           FROM nodes WHERE user_id = $1"#
    )
    .bind(&user.sub)
    .fetch_optional(pool)
    .await;

    // Country breakdown
    let country_stats = sqlx::query(
        r#"SELECT client_country, COUNT(*) as session_count, SUM(bytes_transferred) as bytes
           FROM peer_sessions 
           WHERE node_owner_id = $1
           GROUP BY client_country
           ORDER BY bytes DESC
           LIMIT 10"#
    )
    .bind(&user.sub)
    .fetch_all(pool)
    .await;

    match (node_info, country_stats) {
        (Ok(Some(node)), Ok(countries)) => {
            let country_breakdown: Vec<serde_json::Value> = countries.iter().map(|r| {
                json!({
                    "country": r.get::<String, _>("client_country"),
                    "sessions": r.get::<i64, _>("session_count"),
                    "bytes": r.get::<i64, _>("bytes")
                })
            }).collect();

            let total_bytes: i64 = node.get("total_bytes");

            (StatusCode::OK, Json(json!({
                "node_status": if node.get::<bool, _>("is_online") { "online" } else { "offline" },
                "reputation_score": node.get::<i32, _>("reputation_score"),
                "active_connections": node.get::<i32, _>("current_connections"),
                "lifetime_sessions": node.get::<i64, _>("total_sessions"),
                "lifetime_bytes_shared": total_bytes,
                "lifetime_formatted": format_bytes(total_bytes),
                "top_countries": country_breakdown,
                "impact_message": generate_impact_message(total_bytes)
            }))).into_response()
        }
        (Ok(None), _) => {
            (StatusCode::NOT_FOUND, Json(json!({
                "error": "No node found",
                "hint": "Register a node first with POST /nodes/register"
            }))).into_response()
        }
        (Err(e), _) | (_, Err(e)) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({
                "error": e.to_string()
            }))).into_response()
        }
    }
}

/// Format bytes to human-readable string
fn format_bytes(bytes: i64) -> String {
    const KB: i64 = 1024;
    const MB: i64 = KB * 1024;
    const GB: i64 = MB * 1024;
    const TB: i64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

/// Generate a motivational impact message
fn generate_impact_message(bytes: i64) -> String {
    const GB: i64 = 1024 * 1024 * 1024;
    let gb_shared = bytes / GB;

    if gb_shared >= 100 {
        format!("🏆 Hero! You've helped hundreds of people access free internet!")
    } else if gb_shared >= 50 {
        format!("🌟 Amazing! You've shared {} GB - enough for a small community!", gb_shared)
    } else if gb_shared >= 10 {
        format!("👏 Great work! {} GB shared - you're making a difference!", gb_shared)
    } else if gb_shared >= 1 {
        format!("🚀 Nice start! {} GB shared so far. Keep it up!", gb_shared)
    } else {
        format!("👋 Welcome! Start sharing to earn credits and help others.")
    }
}
