mod state;
mod api;
mod auth;

use crate::state::AppState;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize structured logging
    tracing_subscriber::fmt::init();

    info!("🚀 Starting WorldVPN server...");

    // Load configuration from environment
    dotenvy::dotenv().ok();
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let use_tls = std::env::var("USE_TLS").unwrap_or_else(|_| "false".to_string()) == "true";

    // Establish persistent database connection pool
    info!("📦 Connecting to database: {}", db_url);
    let db_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(20)
        .acquire_timeout(std::time::Duration::from_secs(3))
        .connect(&db_url)
        .await
        .expect("Failed to connect to PostgreSQL database");

    // Liveness probe on startup
    sqlx::query("SELECT 1").execute(&db_pool).await.expect("DB Health check failed");

    // Initialize global application state
    let state = AppState::new(Some(db_pool));

    // Register API routes
    let app = api::router(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));

    if use_tls {
        info!("🔒 HTTPS/TLS mode enabled");
        
        let cert_path = std::env::var("TLS_CERT_PATH")
            .unwrap_or_else(|_| "backend/server/cert.pem".to_string());
        let key_path = std::env::var("TLS_KEY_PATH")
            .unwrap_or_else(|_| "backend/server/key.pem".to_string());

        info!("📜 Loading certificate: {}", cert_path);
        info!("🔑 Loading private key: {}", key_path);

        let cert_file = std::fs::File::open(&cert_path)
            .expect("Failed to open certificate file");
        let mut cert_reader = std::io::BufReader::new(cert_file);
        
        let certs: Vec<rustls::pki_types::CertificateDer> = rustls_pemfile::certs(&mut cert_reader)
            .collect::<Result<Vec<_>, _>>()
            .expect("Error reading certificates");

        let key_file = std::fs::File::open(&key_path)
            .expect("Failed to open key file");
        let mut key_reader = std::io::BufReader::new(key_file);
        
        let key = rustls_pemfile::private_key(&mut key_reader)
            .expect("Error reading key")
            .expect("Private key not found");

        let mut server_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .expect("Invalid TLS configuration");

        // Negotiate ALPN for H2 and HTTP/1.1
        server_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

        let tls_acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(server_config));

        info!("🎧 HTTPS API Server listening on https://{}", addr);

        let listener = tokio::net::TcpListener::bind(addr).await?;

        // Primary connection accept loop
        loop {
            let (tcp_stream, remote_addr) = listener.accept().await?;
            let tls_acceptor = tls_acceptor.clone();
            let app = app.clone();

            tokio::spawn(async move {
                let tls_stream = match tls_acceptor.accept(tcp_stream).await {
                    Ok(stream) => stream,
                    Err(e) => {
                        tracing::error!("TLS handshake error from {}: {}", remote_addr, e);
                        return;
                    }
                };

                // Serve connection via hyper (low-level handling for custom TLS)
                if let Err(e) = hyper::server::conn::http1::Builder::new()
                    .serve_connection(
                        hyper_util::rt::TokioIo::new(tls_stream),
                        hyper::service::service_fn(move |req| {
                            tower::ServiceExt::oneshot(app.clone(), req)
                        })
                    )
                    .await
                {
                    tracing::error!("HTTPS connection error: {}", e);
                }
            });
        }
    } else {
        info!("⚠️  HTTP mode (unsecured) - Use USE_TLS=true for HTTPS");
        info!("🎧 HTTP API Server listening on http://{}", addr);
        
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;
    }

    Ok(())
}
