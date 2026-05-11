use sqlx::PgPool;
use std::time::Duration;
use vpn_core::public_gate::fetch_github_ss_lists;
use serde_json::json;

pub async fn start_shadowsocks_sync(pool: PgPool) {
    tracing::info!("Starting Shadowsocks synchronization service...");
    
    loop {
        if let Err(e) = sync_nodes(&pool).await {
            tracing::error!("Shadowsocks sync failed: {}", e);
        }
        
        // Wait for 20 minutes before next sync
        tokio::time::sleep(Duration::from_secs(20 * 60)).await;
    }
}

async fn sync_nodes(pool: &PgPool) -> anyhow::Result<()> {
    tracing::info!("Fetching latest nodes from Shadowsocks GitHub lists...");
    
    let ss_nodes = fetch_github_ss_lists().await.map_err(|e| anyhow::anyhow!(e))?;
    let mut nodes_added = 0;

    for node in ss_nodes {
        // Generate a stable ID
        let node_id = format!("ss_{}_{}", node.ip.replace(".", "_"), node.port);
        
        // Prepare metadata for public_config_data
        let config_data = json!({
            "protocol": "Shadowsocks",
            "method": node.ss_method.clone().unwrap_or_default(),
            "password": node.ss_password.clone().unwrap_or_default(),
            "host": node.ip.clone(),
            "port": node.port,
        }).to_string();

        // Insert or update node
        sqlx::query(
            r#"INSERT INTO nodes 
               (id, node_group, is_public, country_code, protocols, 
                public_config_data, is_online, public_ip_hash)
               VALUES ($1, 'PUBLIC', TRUE, $2, '["Shadowsocks"]', $3, TRUE, $4)
               ON CONFLICT (id) DO UPDATE SET
                   is_online = TRUE,
                   public_config_data = $3,
                   last_heartbeat = CURRENT_TIMESTAMP,
                   updated_at = CURRENT_TIMESTAMP"#
        )
        .bind(&node_id)
        .bind(&node.country_code)
        .bind(&config_data)
        .bind(format!("hash_{}", node_id))
        .execute(pool)
        .await?;
        
        nodes_added += 1;
        if nodes_added >= 150 { break; } // Limit to 150 nodes to avoid bloating
    }

    tracing::info!("Successfully synced {} public nodes from Shadowsocks lists", nodes_added);
    
    // Record stats
    sqlx::query("INSERT INTO public_provider_stats (provider_name, total_nodes_found, status) VALUES ('SHADOWSOCKS', $1, 'SUCCESS')")
        .bind(nodes_added as i32)
        .execute(pool)
        .await?;

    Ok(())
}
