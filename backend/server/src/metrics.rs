//! Module métriques — Prometheus anonymisées
//!
//! Expose des compteurs agrégés sur /metrics (port 9090).
//! AUCUNE donnée d'identification n'est enregistrée.

use metrics_exporter_prometheus::PrometheusBuilder;
use std::net::SocketAddr;
use tracing::info;

/// Compteurs anonymisées disponibles dans le code
pub mod counters {
    /// Incrémenter à chaque tentative d'authentification (success ou non)
    pub const AUTH_ATTEMPTS: &str = "vpn_auth_attempts_total";
    /// Incrémenter à chaque authentification réussie
    pub const AUTH_SUCCESS: &str = "vpn_auth_success_total";
    /// Incrémenter à chaque nouvelle session VPN créée
    pub const SESSIONS_CREATED: &str = "vpn_sessions_created_total";
    /// Incrémenter à chaque session déconnectée
    pub const SESSIONS_ENDED: &str = "vpn_sessions_ended_total";
    /// Incrémenter avec le nombre de bytes relayés (cumulatif anonymisé)
    pub const BYTES_RELAYED: &str = "vpn_bytes_relayed_total";
    /// Incrémenter à chaque nœud enregistré dans le réseau
    pub const NODES_REGISTERED: &str = "vpn_nodes_registered_total";
    /// Mettre à jour avec le nombre de nœuds actuellement actifs
    pub const NODES_ACTIVE: &str = "vpn_nodes_active";
}

/// Démarre le serveur de métriques Prometheus sur le port 9090.
/// Cette fonction est non-bloquante — elle doit être lancée via tokio::spawn.
///
/// # Données exposées
/// Toutes les métriques sont des agrégats anonymisés.
/// Aucune IP, clé publique, ou identifiant utilisateur n'est présent.
pub async fn start_metrics_server() {
    let addr: SocketAddr = ([0, 0, 0, 0], 9090).into();

    let builder = PrometheusBuilder::new();
    
    match builder.with_http_listener(addr).install() {
        Ok(()) => {
            info!("📊 Prometheus metrics available at http://{}/metrics", addr);
        }
        Err(e) => {
            tracing::warn!("⚠️  Failed to start metrics server: {}. Metrics will be unavailable.", e);
        }
    }
}

/// Initialise les descrictions des métriques au démarrage (bonne pratique Prometheus).
pub fn init_metrics() {
    metrics::describe_counter!(
        counters::AUTH_ATTEMPTS,
        "Total number of authentication attempts (anonymous identity)"
    );
    metrics::describe_counter!(
        counters::AUTH_SUCCESS,
        "Total number of successful authentications"
    );
    metrics::describe_counter!(
        counters::SESSIONS_CREATED,
        "Total number of VPN sessions created"
    );
    metrics::describe_counter!(
        counters::SESSIONS_ENDED,
        "Total number of VPN sessions ended"
    );
    metrics::describe_counter!(
        counters::BYTES_RELAYED,
        "Total bytes relayed through community nodes (aggregated, anonymous)"
    );
    metrics::describe_counter!(
        counters::NODES_REGISTERED,
        "Total number of nodes registered to the P2P network"
    );
    metrics::describe_gauge!(
        counters::NODES_ACTIVE,
        "Current number of active online nodes"
    );
}
