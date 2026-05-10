use sqlx::PgPool;
use std::time::Duration;

/// VPNBook OpenVPN page — credentials are in plain <code> tags
const VPNBOOK_URL: &str = "https://www.vpnbook.com/freevpn/openvpn";

pub async fn start_vpnbook_sync(pool: PgPool) {
    tracing::info!("Starting VPNBook synchronization service...");
    
    loop {
        if let Err(e) = sync_vpnbook_password(&pool).await {
            tracing::error!("VPNBook password sync failed: {}", e);
        }
        
        // Check every 12 hours (passwords change weekly, but safer to check twice a day)
        tokio::time::sleep(Duration::from_secs(12 * 3600)).await;
    }
}

async fn sync_vpnbook_password(pool: &PgPool) -> anyhow::Result<()> {
    tracing::info!("Scraping VPNBook credentials from HTML...");
    
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (compatible; WorldVPN/1.0)")
        .timeout(Duration::from_secs(15))
        .build()?;
    
    let html = client.get(VPNBOOK_URL).send().await?.text().await?;
    
    // VPNBook displays: <code>vpnbook</code> then <code>ke9zw74</code>
    // under the "VPN Credentials" section. We extract all <code> values
    // and the password is the 2nd one (index 1).
    let re = regex::Regex::new(r"<code[^>]*>([^<]+)</code>")?;
    let codes: Vec<&str> = re.captures_iter(&html)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str()))
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    // codes[0] = username ("vpnbook"), codes[1] = password
    let password = codes.get(1)
        .ok_or_else(|| anyhow::anyhow!("Could not find password <code> tag (found {} codes)", codes.len()))?;
    
    // Basic sanity check: password should be alphanumeric, 5–20 chars
    if password.len() < 4 || password.len() > 30 || !password.chars().all(|c| c.is_alphanumeric()) {
        return Err(anyhow::anyhow!("Suspicious password value scraped: '{}'", password));
    }

    tracing::info!("VPNBook password obtained: {}", password);
    
    // Upsert into database
    sqlx::query(
        r#"INSERT INTO public_provider_metadata (provider_name, key, value, updated_at)
           VALUES ('VPNBOOK', 'password', $1, CURRENT_TIMESTAMP)
           ON CONFLICT (provider_name, key) DO UPDATE SET
               value = $1,
               updated_at = CURRENT_TIMESTAMP"#
    )
    .bind(password)
    .execute(pool)
    .await?;

    tracing::info!("VPNBook password updated in database.");
    Ok(())
}


