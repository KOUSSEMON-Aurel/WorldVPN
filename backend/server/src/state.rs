use sqlx::PgPool;

/// Shared application state accessible across all API handlers
#[derive(Clone)]
pub struct AppState {
    /// PostgreSQL connection pool (optional for testing/mocking)
    pub db: Option<PgPool>,
}

impl AppState {
    pub fn new(db: Option<PgPool>) -> Self {
        Self { db }
    }
}
