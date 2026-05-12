// Build: 2026-05-12 — force recompile and port fusion
mod state;
mod api;
mod auth;
mod services;
mod metrics;

use crate::state::AppState;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize structured logging
    tracing_subscriber::fmt::init();

    info!("🚀 Starting WorldVPN server (Integrated Mode)...");

    // Load configuration
    dotenvy::dotenv().ok();
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().expect("Invalid address");

    // --- 1. BIND IMMEDIATELY ---
    // We bind the listener first so Render's health check succeeds instantly.
    let listener = tokio::net::TcpListener::bind(&addr).await
        .expect("Failed to bind to port 3000. Is it already in use?");
    info!("✅ Port {} binded. Health checks will pass now.", port);

    // --- 2. SETUP METRICS ---
    // Metrics are now integrated into the main port 3000 via Axum.
    let metrics_handle = metrics::setup_metrics_recorder();
    metrics::init_metrics_descriptions();

    // --- 3. APP STATE ---
    // Start with None for DB, it will be populated asynchronously.
    let state = AppState::new(metrics_handle);

    // Rate limiter
    let governor_config = Arc::new(
        GovernorConfigBuilder::default()
            .per_millisecond(100)
            .burst_size(10)
            .finish()
            .expect("Invalid rate limiter config"),
    );
    let governor_layer = GovernorLayer { config: governor_config };

    // Register API routes
    let app = api::router(state.clone()).layer(governor_layer);

    // --- 4. BACKGROUND INITIALIZATION ---
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let background_state = state.clone();
    
    // We don't block the main thread. We run everything in the background.
    tokio::spawn(async move {
        info!("📦 Connecting to database in background...");
        match sqlx::postgres::PgPoolOptions::new()
            .max_connections(20)
            .acquire_timeout(std::time::Duration::from_secs(30)) // Extra patience for Neon
            .connect(&db_url)
            .await 
        {
            Ok(db_pool) => {
                info!("✅ Database connected. Starting migrations and background services.");
                
                // Initialize the OnceCell in state
                let _ = background_state.db.set(db_pool.clone());
                
                // Cleanup corrupted migration record workaround
                let _ = sqlx::query("DELETE FROM _sqlx_migrations WHERE version = 20260510163000")
                    .execute(&db_pool).await;

                // Run migrations
                if let Err(e) = sqlx::migrate!("./migrations").run(&db_pool).await {
                    tracing::error!("❌ Migration failed: {}", e);
                } else {
                    info!("✅ Migrations synced.");
                }

                // Background tasks
                let p1 = db_pool.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                    services::vpngate::start_vpngate_sync(p1).await;
                });

                let p2 = db_pool.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(15)).await;
                    services::pruning::start_pruning_service(p2).await;
                });

                let p3 = db_pool.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(20)).await;
                    services::vpnbook::start_vpnbook_sync(p3).await;
                });

                let p4 = db_pool.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(25)).await;
                    services::shadowsocks::start_shadowsocks_sync(p4).await;
                });
            }
            Err(e) => tracing::error!("❌ Fatal DB connection error: {}", e),
        }
    });

    // --- 5. RUN SERVER ---
    info!("🎧 Web server listening on http://{}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}
