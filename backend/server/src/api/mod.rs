use axum::{routing::{get, post}, Router};
use crate::state::AppState;

pub mod health;
pub mod vpn;
pub mod auth;
pub mod credits;
pub mod nodes;
pub mod transparency;
pub mod vpnbook;

pub fn router(state: AppState) -> Router {
    Router::new()
        // (Health check is now handled in main.rs to bypass rate limiting)
        
        // Prometheus metrics
        .route("/metrics", {
            let metrics_handle = state.metrics_handle.clone();
            get(|| async move { metrics_handle.render() })
        })
        
        // VPNBook password
        .route("/nodes/vpnbook/password", get(vpnbook::get_vpnbook_password))
        
        // Authentication
        .route("/auth/challenge", get(auth::get_challenge))
        .route("/auth/login", post(auth::login))
        .route("/auth/identity", post(auth::identity_login))
        .route("/auth/migrate", post(auth::migrate_credits))
        
        // VPN connection
        .route("/vpn/connect", post(vpn::connect))
        .route("/vpn/disconnect", post(vpn::disconnect))
        
        // Credits system
        .route("/credits/balance", get(credits::get_balance))
        .route("/credits/history", get(credits::get_history))
        .route("/credits/sync", post(credits::sync_traffic))
        .route("/credits/submit", post(credits::submit_receipt))
        
        // P2P Node management
        .route("/nodes/register", post(nodes::register_node))
        .route("/nodes/discover", get(nodes::discover_nodes))
        .route("/nodes/heartbeat", post(nodes::heartbeat))
        .route("/nodes/offline", post(nodes::go_offline))
        .route("/nodes/public", get(nodes::public_nodes))
        .route("/nodes/my", get(nodes::my_node))
        
        // Transparency dashboard (real-time monitoring)
        .route("/transparency/sessions", get(transparency::get_active_sessions))
        .route("/transparency/history", get(transparency::get_session_history))
        .route("/transparency/stats", get(transparency::get_stats))
        
        .with_state(state)
}
