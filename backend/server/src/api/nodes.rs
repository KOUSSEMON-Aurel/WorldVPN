use axum::{
    extract::{State, Query, Path},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;

use crate::{auth::AuthUser, state::AppState};

/// Node registration request from desktop/mobile client
#[derive(Deserialize)]
pub struct RegisterNodeRequest {
    pub country_code: String,
    pub city: Option<String>,
    pub available_bandwidth_mbps: Option<i32>,
    pub protocols: Vec<String>,
    pub allow_streaming: Option<bool>,
    pub allow_torrents: Option<bool>,
    pub max_daily_gb: Option<i32>,
}

#[derive(Serialize)]
pub struct NodeInfo {
    pub id: String,
    pub country_code: String,
    pub reputation_score: i32,
    pub current_connections: i32,
    pub is_online: bool,
}

/// POST /nodes/register - Register current device as a sharing node
pub async fn register_node(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(payload): Json<RegisterNodeRequest>,
) -> impl IntoResponse {
    let pool = state.db.as_ref().expect("DB not initialized");
    
    let node_id = uuid::Uuid::new_v4().to_string();
    let protocols_json = serde_json::to_string(&payload.protocols).unwrap_or("[]".to_string());
    
    // Hash the user's IP for privacy (in production, get real IP from request)
    let ip_hash = format!("hash_{}", uuid::Uuid::new_v4());
    
    let result = sqlx::query(
        r#"INSERT INTO nodes 
           (id, user_id, public_ip_hash, country_code, city, available_bandwidth_mbps, 
            protocols, allow_streaming, allow_torrents, max_daily_gb, is_online)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, TRUE)
           ON CONFLICT (id) DO UPDATE SET
               is_online = TRUE,
               last_heartbeat = CURRENT_TIMESTAMP,
               updated_at = CURRENT_TIMESTAMP
           RETURNING id"#
    )
    .bind(&node_id)
    .bind(&user.sub)
    .bind(&ip_hash)
    .bind(&payload.country_code)
    .bind(&payload.city)
    .bind(payload.available_bandwidth_mbps.unwrap_or(50))
    .bind(&protocols_json)
    .bind(payload.allow_streaming.unwrap_or(true))
    .bind(payload.allow_torrents.unwrap_or(false))
    .bind(payload.max_daily_gb.unwrap_or(50))
    .fetch_one(pool)
    .await;

    match result {
        Ok(_) => {
            tracing::info!("Node registered: {} for user {}", node_id, user.sub);
            (StatusCode::CREATED, Json(json!({
                "node_id": node_id,
                "status": "online",
                "message": "Node registered successfully. You are now sharing bandwidth."
            }))).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to register node: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({
                "error": format!("Registration failed: {}", e)
            }))).into_response()
        }
    }
}

/// Query parameters for node discovery
#[derive(Deserialize)]
pub struct DiscoverQuery {
    pub country: Option<String>,
    pub protocol: Option<String>,
    pub min_bandwidth: Option<i32>,
    pub node_group: Option<String>,
    pub limit: Option<i32>,
}

/// GET /nodes/discover - Find available nodes to connect to
pub async fn discover_nodes(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Query(params): Query<DiscoverQuery>,
) -> impl IntoResponse {
    let pool = state.db.as_ref().expect("DB not initialized");
    
    let limit = params.limit.unwrap_or(10).min(50);
    
    // Build dynamic query based on filters
    let mut query = String::from(
        r#"SELECT id, country_code, reputation_score, current_connections,
                  available_bandwidth_mbps, avg_latency_ms, protocols, node_group
           FROM nodes 
           WHERE is_online = TRUE 
             AND current_connections < max_connections
             AND (user_id IS NULL OR user_id != $1)"#
    );
    
    let mut bind_count = 1;
    
    if params.country.is_some() {
        bind_count += 1;
        query.push_str(&format!(" AND country_code = ${}", bind_count));
    }
    
    if params.min_bandwidth.is_some() {
        bind_count += 1;
        query.push_str(&format!(" AND available_bandwidth_mbps >= ${}", bind_count));
    }
    
    if params.node_group.is_some() {
        bind_count += 1;
        query.push_str(&format!(" AND node_group = ${}", bind_count));
    }
    
    query.push_str(" ORDER BY reputation_score DESC, avg_latency_ms ASC");
    query.push_str(&format!(" LIMIT {}", limit));

    // Execute with dynamic bindings
    let mut q = sqlx::query(&query).bind(&user.sub);
    
    if let Some(ref country) = params.country {
        q = q.bind(country);
    }
    if let Some(bw) = params.min_bandwidth {
        q = q.bind(bw);
    }
    if let Some(ref group) = params.node_group {
        q = q.bind(group);
    }

    let result = q.fetch_all(pool).await;

    match result {
        Ok(rows) => {
            let nodes: Vec<serde_json::Value> = rows.iter().map(|row| {
                json!({
                    "id": row.get::<String, _>("id"),
                    "country_code": row.get::<String, _>("country_code"),
                    "reputation_score": row.get::<i32, _>("reputation_score"),
                    "connections": row.get::<i32, _>("current_connections"),
                    "bandwidth_mbps": row.get::<i32, _>("available_bandwidth_mbps"),
                    "latency_ms": row.get::<i32, _>("avg_latency_ms"),
                    "protocols": row.get::<String, _>("protocols"),
                    "group": row.get::<String, _>("node_group")
                })
            }).collect();

            (StatusCode::OK, Json(json!({
                "nodes": nodes,
                "count": nodes.len()
            }))).into_response()
        }
        Err(e) => {
            tracing::error!("Node discovery failed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({
                "error": "Discovery failed"
            }))).into_response()
        }
    }
}

/// POST /nodes/heartbeat - Keep node alive and update status
pub async fn heartbeat(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> impl IntoResponse {
    let pool = state.db.as_ref().expect("DB not initialized");

    let result = sqlx::query(
        r#"UPDATE nodes 
           SET last_heartbeat = CURRENT_TIMESTAMP, is_online = TRUE
           WHERE user_id = $1
           RETURNING id, current_connections"#
    )
    .bind(&user.sub)
    .fetch_optional(pool)
    .await;

    match result {
        Ok(Some(row)) => {
            (StatusCode::OK, Json(json!({
                "status": "alive",
                "node_id": row.get::<String, _>("id"),
                "active_connections": row.get::<i32, _>("current_connections")
            }))).into_response()
        }
        Ok(None) => {
            (StatusCode::NOT_FOUND, Json(json!({
                "error": "No node registered for this user"
            }))).into_response()
        }
        Err(e) => {
            tracing::error!("Heartbeat failed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({
                "error": "Heartbeat failed"
            }))).into_response()
        }
    }
}

/// POST /nodes/offline - Mark node as offline (user stops sharing)
pub async fn go_offline(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> impl IntoResponse {
    let pool = state.db.as_ref().expect("DB not initialized");

    let _ = sqlx::query("UPDATE nodes SET is_online = FALSE WHERE user_id = $1")
        .bind(&user.sub)
        .execute(pool)
        .await;

    (StatusCode::OK, Json(json!({
        "status": "offline",
        "message": "Node is now offline. You stopped sharing bandwidth."
    }))).into_response()
}

/// GET /nodes/my - Get current user's node info
pub async fn my_node(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> impl IntoResponse {
    let pool = state.db.as_ref().expect("DB not initialized");

    let result = sqlx::query(
        r#"SELECT id, country_code, is_online, reputation_score, current_connections,
                  available_bandwidth_mbps, allow_streaming, allow_torrents, created_at
           FROM nodes WHERE user_id = $1"#
    )
    .bind(&user.sub)
    .fetch_optional(pool)
    .await;

    match result {
        Ok(Some(row)) => {
            (StatusCode::OK, Json(json!({
                "id": row.get::<String, _>("id"),
                "country_code": row.get::<String, _>("country_code"),
                "is_online": row.get::<bool, _>("is_online"),
                "reputation_score": row.get::<i32, _>("reputation_score"),
                "current_connections": row.get::<i32, _>("current_connections"),
                "bandwidth_mbps": row.get::<i32, _>("available_bandwidth_mbps"),
                "allow_streaming": row.get::<bool, _>("allow_streaming"),
                "allow_torrents": row.get::<bool, _>("allow_torrents")
            }))).into_response()
        }
        Ok(None) => {
            (StatusCode::NOT_FOUND, Json(json!({
                "error": "No node registered",
                "hint": "Use POST /nodes/register to start sharing"
            }))).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({
                "error": e.to_string()
            }))).into_response()
        }
    }
}
