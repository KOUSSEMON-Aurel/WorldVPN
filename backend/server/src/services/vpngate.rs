use sqlx::PgPool;
use std::time::Duration;
use base64::{Engine as _, engine::general_purpose};

/// VPN Gate API URL (CSV Format)
const VPNGATE_API_URL: &str = "http://www.vpngate.net/api/iphone/";

pub async fn start_vpngate_sync(pool: PgPool) {
    tracing::info!("Starting VPN Gate synchronization service...");
    
    loop {
        if let Err(e) = sync_nodes(&pool).await {
            tracing::error!("VPN Gate sync failed: {}", e);
        }
        
        // Wait for 1 hour before next sync
        tokio::time::sleep(Duration::from_secs(3600)).await;
    }
}

async fn sync_nodes(pool: &PgPool) -> anyhow::Result<()> {
    tracing::info!("Fetching latest nodes from VPN Gate...");
    
    let response = reqwest::get(VPNGATE_API_URL).await?.text().await?;
    
    // Skip first two lines (header comment and column names)
    let csv_content = response.lines()
        .skip(1)
        .collect::<Vec<_>>()
        .join("\n");
        
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(csv_content.as_bytes());

    let mut nodes_added = 0;
    
    for result in rdr.records() {
        let record = result?;
        
        // CSV columns: #HostName,IP,Score,Ping,Speed,CountryLong,CountryShort,NumVpnSessions,Uptime,TotalUsers,TotalTraffic,LogType,Operator,Message,OpenVPN_ConfigData_Base64
        if record.len() < 15 { continue; }
        
        let ip = &record[1];
        let country_short = &record[6];
        let speed = record[4].parse::<i32>().unwrap_or(0) / 1000000; // Convert to Mbps
        let config_b64 = &record[14];
        
        // Generate a stable ID based on IP
        let node_id = format!("vpngate_{}", ip.replace(".", "_"));
        
        // Insert or update node
        sqlx::query(
            r#"INSERT INTO nodes 
               (id, node_group, is_public, country_code, available_bandwidth_mbps, 
                protocols, public_config_data, is_online, public_ip_hash)
               VALUES ($1, 'PUBLIC', TRUE, $2, $3, '["OpenVPN"]', $4, TRUE, $5)
               ON CONFLICT (id) DO UPDATE SET
                   is_online = TRUE,
                   available_bandwidth_mbps = $3,
                   public_config_data = $4,
                   last_heartbeat = CURRENT_TIMESTAMP,
                   updated_at = CURRENT_TIMESTAMP"#
        )
        .bind(&node_id)
        .bind(country_short)
        .bind(speed)
        .bind(config_b64)
        .bind(format!("hash_{}", node_id))
        .execute(pool)
        .await?;
        
        nodes_added += 1;
        if nodes_added >= 100 { break; } // Limit to 100 nodes for now
    }

    tracing::info!("Successfully synced {} public nodes from VPN Gate", nodes_added);
    
    // Record stats
    sqlx::query("INSERT INTO public_provider_stats (provider_name, total_nodes_found, status) VALUES ('VPN_GATE', $1, 'SUCCESS')")
        .bind(nodes_added as i32)
        .execute(pool)
        .await?;

    Ok(())
}
