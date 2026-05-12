use sqlx::PgPool;
use metrics_exporter_prometheus::PrometheusHandle;
use std::sync::Arc;
use tokio::sync::OnceCell;

/// Shared application state accessible across all API handlers
#[derive(Clone)]
pub struct AppState {
    /// PostgreSQL connection pool (initialisée de manière asynchrone)
    pub db: Arc<OnceCell<PgPool>>,
    /// Handle pour exposer les métriques Prometheus
    pub metrics_handle: PrometheusHandle,
}

impl AppState {
    pub fn new(metrics_handle: PrometheusHandle) -> Self {
        Self { 
            db: Arc::new(OnceCell::new()), 
            metrics_handle 
        }
    }
}
