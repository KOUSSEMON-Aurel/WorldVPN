//! Module métriques — Prometheus anonymisées intégrées
//!
//! Expose des compteurs agrégés sur /metrics sur le port principal (3000).

use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use tracing::info;

/// Compteurs anonymisées disponibles dans le code
pub mod counters {
    pub const AUTH_ATTEMPTS: &str = "vpn_auth_attempts_total";
    pub const AUTH_SUCCESS: &str = "vpn_auth_success_total";
    pub const SESSIONS_CREATED: &str = "vpn_sessions_created_total";
    pub const SESSIONS_ENDED: &str = "vpn_sessions_ended_total";
    pub const BYTES_RELAYED: &str = "vpn_bytes_relayed_total";
    pub const NODES_REGISTERED: &str = "vpn_nodes_registered_total";
    pub const NODES_ACTIVE: &str = "vpn_nodes_active";
}

/// Initialise l'enregistreur Prometheus et retourne un handle pour Axum
pub fn setup_metrics_recorder() -> PrometheusHandle {
    info!("📊 Initializing Prometheus metrics recorder (integrated mode)");
    PrometheusBuilder::new()
        .install_recorder()
        .expect("failed to install metrics recorder")
}

/// Initialise les descriptions des métriques au démarrage
pub fn init_metrics_descriptions() {
    metrics::describe_counter!(counters::AUTH_ATTEMPTS, "Total number of authentication attempts");
    metrics::describe_counter!(counters::AUTH_SUCCESS, "Total number of successful authentications");
    metrics::describe_counter!(counters::SESSIONS_CREATED, "Total number of VPN sessions created");
    metrics::describe_counter!(counters::SESSIONS_ENDED, "Total number of VPN sessions ended");
    metrics::describe_counter!(counters::BYTES_RELAYED, "Total bytes relayed (anonymous aggregate)");
    metrics::describe_counter!(counters::NODES_REGISTERED, "Total nodes registered in P2P network");
    metrics::describe_gauge!(counters::NODES_ACTIVE, "Current number of active online nodes");
}

pub async fn start_metrics_server() {
    // Obsolete - keeping for compatibility during refactoring if needed
}
