use axum::{routing::{get, post}, Router};
use crate::state::AppState;

pub mod health;
pub mod vpn;
pub mod auth;
pub mod credits;
pub mod nodes;
pub mod transparency;

pub fn router(state: AppState) -> Router {
    Router::new()
        // Health check
        .route("/health", get(health::health_check))
        
        // Authentication
        .route("/auth/login", post(auth::login))
        .route("/auth/register", post(auth::register))
        
        // VPN connection
        .route("/vpn/connect", post(vpn::connect))
        .route("/vpn/disconnect", post(vpn::disconnect))
        
        // Credits system
        .route("/credits/balance", get(credits::get_balance))
        .route("/credits/history", get(credits::get_history))
        .route("/credits/sync", post(credits::sync_traffic))
        
        // P2P Node management
        .route("/nodes/register", post(nodes::register_node))
        .route("/nodes/discover", get(nodes::discover_nodes))
        .route("/nodes/heartbeat", post(nodes::heartbeat))
        .route("/nodes/offline", post(nodes::go_offline))
        .route("/nodes/my", get(nodes::my_node))
        
        // Transparency dashboard (real-time monitoring)
        .route("/transparency/sessions", get(transparency::get_active_sessions))
        .route("/transparency/history", get(transparency::get_session_history))
        .route("/transparency/stats", get(transparency::get_stats))
        
        .with_state(state)
}
