use sqlx::PgPool;
use metrics_exporter_prometheus::PrometheusHandle;
use std::sync::Arc;
use tokio::sync::OnceCell;
use crate::auth::NonceStore;

/// Shared application state accessible across all API handlers
#[derive(Clone)]
pub struct AppState {
    /// PostgreSQL connection pool (initialisée de manière asynchrone)
    pub db: Arc<OnceCell<PgPool>>,
    /// Handle pour exposer les métriques Prometheus
    pub metrics_handle: PrometheusHandle,
    /// Store for authentication nonces (anti-replay)
    pub nonce_store: Arc<NonceStore>,
}

impl AppState {
    pub fn new(metrics_handle: PrometheusHandle) -> Self {
        Self { 
            db: Arc::new(OnceCell::new()), 
            metrics_handle,
            nonce_store: Arc::new(NonceStore::new())
        }
    }
}
