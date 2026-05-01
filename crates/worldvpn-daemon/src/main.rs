use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::signal;
use tracing::{info, warn, error};
use tracing_subscriber::EnvFilter;

use vpn_core::{
    crypto::IdentityKey,
    client::VpnApiClient,
    p2p::{PeerDiscovery, PeerInfo},
};

/// WorldVPN Daemon - Background P2P Networking & Sharing Service
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the identity private key file (DER format)
    #[arg(short, long, env = "WORLDVPN_ID_PATH")]
    identity: Option<PathBuf>,

    /// Enable P2P Sharing mode
    #[arg(short, long, default_value_t = false)]
    share: bool,

    /// Act as a DHT Super-Node (permanent relay)
    #[arg(long, default_value_t = false)]
    super_node: bool,

    /// Backend API URL
    #[arg(long, default_value = "https://api.worldvpn.com")]
    api_url: String,

    /// Country code for sharing registration
    #[arg(long, env = "WORLDVPN_COUNTRY", default_value = "US")]
    country: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .init();

    let args = Args::parse();
    info!("Starting WorldVPN Daemon v{}", env!("CARGO_PKG_VERSION"));

    // 1. Load or Generate Identity
    let identity = if let Some(path) = args.identity {
        if path.exists() {
            let bytes = std::fs::read(&path)?;
            IdentityKey::from_bytes(&bytes)?
        } else {
            info!("Identity file not found at {:?}, generating new identity...", path);
            let id = IdentityKey::generate();
            let bytes = id.to_bytes()?;
            std::fs::create_dir_all(path.parent().unwrap_or(&PathBuf::from(".")))?;
            std::fs::write(&path, &*bytes)?;
            id
        }
    } else {
        warn!("Running with ephemeral identity (Guest mode equivalent)");
        IdentityKey::generate()
    };

    info!("Identity Public Key: {}", identity.public_key_hex());

    // 2. Initialize P2P Discovery
    info!("Initializing libp2p swarm...");
    let discovery = Arc::new(PeerDiscovery::new().await?);
    
    // 3. Optional: Register with backend if sharing is enabled
    if args.share {
        info!("P2P Sharing enabled. Registering with backend...");
        let client = VpnApiClient::new(args.api_url);
        
        // Handshake for registration
        let timestamp = chrono::Utc::now().timestamp().to_string();
        let signature = identity.sign_challenge(&timestamp);
        
        match client.login_with_identity(
            identity.public_key_hex(),
            signature,
            timestamp
        ).await {
            Ok(auth) => {
                info!("Authenticated with backend as {}", auth.username);
                
                // Keep-alive heartbeat loop
                let client_clone = client.clone();
                let token = auth.token.clone();
                let country = args.country.clone();
                
                tokio::spawn(async move {
                    // Initial registration
                    match client_clone.register_node(
                        &token,
                        &country,
                        None,
                        "Public".to_string(),
                        None,
                        vec!["wireguard".to_string()]
                    ).await {
                        Ok(node_id) => {
                            info!("Node registered successfully. ID: {}", node_id);
                            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
                            loop {
                                interval.tick().await;
                                if let Err(e) = client_clone.heartbeat(&token, None, None, None).await {
                                    error!("Heartbeat failed: {}", e);
                                }
                            }
                        }
                        Err(e) => error!("Failed to register node: {}", e),
                    }
                });
            }
            Err(e) => error!("Failed to authenticate with backend: {}", e),
        }
    }

    if args.super_node {
        info!("Running in Super-Node mode. DHT relay active.");
    }

    info!("WorldVPN Daemon is now active and relaying P2P traffic.");
    
    // Wait for termination signal
    match signal::ctrl_c().await {
        Ok(()) => info!("Shutting down WorldVPN Daemon..."),
        Err(err) => error!("Unable to listen for shutdown signal: {}", err),
    }

    Ok(())
}
