use axum::{routing::{get, post}, Router};
use crate::state::AppState;

pub mod health;
pub mod vpn;
pub mod auth;
pub mod credits;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health::health_check))
        .route("/auth/login", post(auth::login))
        .route("/auth/register", post(auth::register))
        .route("/vpn/connect", post(vpn::connect))
        // Routes Crédits
        .route("/credits/balance", get(credits::get_balance))
        .route("/credits/history", get(credits::get_history))
        .route("/credits/sync", post(credits::sync_traffic))
        .with_state(state)
}
