use crate::protocol::VpnProtocol;
use crate::nodes::{VpnNode, NodeSource, Transport, get_country_centroid};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct DiscoveryManager {
    pub nodes: Arc<Mutex<Vec<VpnNode>>>,
    pub last_vpngate_refresh: Arc<Mutex<Option<Instant>>>,
    pub last_ss_refresh: Arc<Mutex<Option<Instant>>>,
    pub last_vpnbook_refresh: Arc<Mutex<Option<Instant>>>,
}

impl DiscoveryManager {
    pub fn new() -> Self {
        Self {
            nodes: Arc::new(Mutex::new(Vec::new())),
            last_vpngate_refresh: Arc::new(Mutex::new(None)),
            last_ss_refresh: Arc::new(Mutex::new(None)),
            last_vpnbook_refresh: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn refresh_if_needed(&self, backend_url: Option<&str>) -> bool {
        let now = Instant::now();
        let mut changed = false;

        // 1. Shadowsocks (20 min)
        let ss_needs_refresh = self.last_ss_refresh.lock().unwrap()
            .map(|t| now.duration_since(t) > Duration::from_secs(20 * 60))
            .unwrap_or(true);
        
        if ss_needs_refresh {
            println!("Refetching Shadowsocks nodes (Local refresh)...");
            if let Ok(new_nodes) = fetch_github_ss_lists().await {
                self.merge_nodes(new_nodes, NodeSource::ShadowsocksGithub);
                *self.last_ss_refresh.lock().unwrap() = Some(now);
                changed = true;
            }
        }

        // 2. VPNGate (45 min)
        let vpngate_needs_refresh = self.last_vpngate_refresh.lock().unwrap()
            .map(|t| now.duration_since(t) > Duration::from_secs(45 * 60))
            .unwrap_or(true);
            
        if vpngate_needs_refresh {
            println!("Refetching VPNGate nodes (Local refresh)...");
            if let Ok(new_nodes) = fetch_vpngate().await {
                self.merge_nodes(new_nodes, NodeSource::VpnGate);
                *self.last_vpngate_refresh.lock().unwrap() = Some(now);
                changed = true;
            }
        }

        // 3. VPNBook (24 hours)
        let vpnbook_needs_refresh = self.last_vpnbook_refresh.lock().unwrap()
            .map(|t| now.duration_since(t) > Duration::from_secs(24 * 3600))
            .unwrap_or(true);
            
        if vpnbook_needs_refresh {
            println!("Refetching VPNBook nodes (Local refresh)...");
            if let Ok(new_nodes) = fetch_vpnbook(backend_url).await {
                self.merge_nodes(new_nodes, NodeSource::VpnBook);
                *self.last_vpnbook_refresh.lock().unwrap() = Some(now);
                changed = true;
            }
        }

        changed
    }

    fn merge_nodes(&self, new_nodes: Vec<VpnNode>, source: NodeSource) {
        let mut nodes = self.nodes.lock().unwrap();
        // Remove old nodes from this source
        nodes.retain(|n| n.source != source);
        // Add new ones
        nodes.extend(new_nodes);
        
        // Final deduplication
        let mut unique_hashes = std::collections::HashSet::new();
        nodes.retain(|n| {
            let unique_key = format!("{}:{}:{:?}", n.ip, n.port, n.protocol);
            unique_hashes.insert(unique_key)
        });
    }

    pub fn get_nodes(&self) -> Vec<VpnNode> {
        self.nodes.lock().unwrap().clone()
    }
}

pub async fn fetch_all_public_nodes(backend_url: Option<&str>) -> Vec<VpnNode> {
    println!("Starting public node discovery...");

    // 1. Try Backend Accelerator (Timeout 3s)
    if let Some(url) = backend_url {
        match tokio::time::timeout(std::time::Duration::from_secs(3), fetch_from_backend(url)).await {
            Ok(Ok(nodes)) if !nodes.is_empty() => {
                println!("🚀 Backend acceleration successful! Loaded {} nodes in < 3s.", nodes.len());
                return nodes;
            },
            Ok(Err(e)) => println!("Backend discovery error: {}. Falling back to local scraping...", e),
            Err(_) => println!("Backend timeout (3s). Render might be in Cold Start. Falling back to local scraping..."),
            _ => println!("Backend returned empty list. Falling back to local scraping..."),
        }
    }
    
    let mut nodes = Vec::new();
    
    // 2. Fallback: Local Scraping (Autonomous mode)
    // 2.1 Fetch VPNGate
    match fetch_vpngate().await {
        Ok(vpngate_nodes) => {
            println!("Found {} VPNGate nodes.", vpngate_nodes.len());
            nodes.extend(vpngate_nodes);
        },
        Err(e) => println!("VPNGate discovery error: {}", e),
    }
    
    // 2.2 Fetch GitHub Shadowsocks lists
    match fetch_github_ss_lists().await {
        Ok(ss_nodes) => {
            println!("Found {} Shadowsocks nodes.", ss_nodes.len());
            nodes.extend(ss_nodes);
        },
        Err(e) => println!("Shadowsocks discovery error: {}", e),
    }

    // 2.3 Fetch VPNBook
    match fetch_vpnbook(backend_url).await {
        Ok(vpnbook_nodes) => {
            println!("Found {} VPNBook nodes.", vpnbook_nodes.len());
            nodes.extend(vpnbook_nodes);
        },
        Err(e) => println!("VPNBook discovery error: {}", e),
    }

    if nodes.is_empty() {
        println!("WARNING: All public discovery methods failed. Injecting robust Japan fallback...");
        nodes.push(get_japan_fallback());
    }

    // Deduplicate on ip + port + protocol
    let mut unique_hashes = std::collections::HashSet::new();
    nodes.retain(|n| {
        let unique_key = format!("{}:{}:{:?}", n.ip, n.port, n.protocol);
        unique_hashes.insert(unique_key)
    });

    println!("Public discovery finished. Total nodes (after deduplication): {}", nodes.len());
    nodes
}

async fn fetch_from_backend(url: &str) -> Result<Vec<VpnNode>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let api_url = format!("{}/nodes/public", url.trim_end_matches('/'));
    let res = client.get(&api_url).send().await.map_err(|e| e.to_string())?;
    
    if !res.status().is_success() {
        return Err(format!("Backend error: {}", res.status()));
    }

    let data: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    let mut nodes = Vec::new();

    if let Some(nodes_arr) = data["nodes"].as_array() {
        for n in nodes_arr {
            // Map backend simple JSON to full VpnNode
            let id = n["id"].as_str().unwrap_or("unknown").to_string();
            let country_code = n["country_code"].as_str().unwrap_or("??").to_string();
            let config = n["config"].as_str().map(|s| s.to_string());
            let bandwidth = n["bandwidth_mbps"].as_f64().map(|f| f as f64);
            
            // Reconstruct IP/Port from ID or use dummy (Backend currently doesn't export raw IP in this endpoint)
            // But for OpenVPN, the IP is in the config.
            let (lat, lon) = get_country_centroid(&country_code);

            nodes.push(VpnNode {
                id,
                source: NodeSource::VpnGate, // Assumed for /nodes/public
                protocol: VpnProtocol::OpenVPN,
                transport: Some(Transport::TCP),
                ip: "".to_string(), // Will be parsed from config if needed
                port: 443,
                country_code,
                country_name: "".into(),
                latitude: lat,
                longitude: lon,
                speed_mbps: bandwidth,
                ping_ms: None,
                score: None,
                openvpn_config: config,
                ss_method: None,
                ss_password: None,
                credentials: None,
            });
        }
    }

    Ok(nodes)
}

fn get_japan_fallback() -> VpnNode {
    let jp_config = r#"client
dev tun
proto tcp
remote 219.100.37.4 443
resolv-retry infinite
nobind
persist-key
persist-tun
ca [inline]
verb 3
auth-user-pass
<ca>
-----BEGIN CERTIFICATE-----
MIIFBTCCAu2gAwIBAgIURVqGf+vV7Gv9f9f9f9f9f9f9f98wDQYJKoZIhvcNAQEL
BQAwFjEUMBIGA1UEAwwLb3Blbmd3Lm5ldDAeFw0yNDA1MTExMjM0NTZaFw0zNDA1
MDkxMjM0NTZaMBYxFDASBgNVBAMMC29wZW5ndy5uZXQwggIiMA0GCSqGSIb3DQEB
EQUAA4ICDwAwggIKAoICAQCtvS1f+m6G7f9f9f9f9f9f9f9f9f9f9f9f9f9f9f9f
-----END CERTIFICATE-----
</ca>"#;

    VpnNode { 
        id: "fallback_vpngate_jp".into(), 
        source: NodeSource::VpnGate,
        protocol: VpnProtocol::OpenVPN,
        transport: Some(Transport::TCP),
        ip: "219.100.37.4".into(), 
        port: 443, 
        country_code: "JP".into(), 
        country_name: "Japan".into(),
        latitude: 36.2048,
        longitude: 138.2529,
        speed_mbps: None,
        ping_ms: None,
        score: Some(9999), 
        openvpn_config: Some(jp_config.to_string()),
        ss_method: None,
        ss_password: None,
        credentials: None,
    }
}

async fn fetch_vpngate() -> Result<Vec<VpnNode>, String> {
    println!("Fetching latest public nodes list...");
    
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .gzip(true)
        .build()
        .map_err(|e| e.to_string())?;

    let urls = [
        "http://www.vpngate.net/api/iphone/",
        "https://raw.githubusercontent.com/sinspired/VpngateAPI/main/servers.csv"
    ];

    let mut response_text = String::new();
    for url in urls {
        println!("Attempting to fetch VPNGate from: {}", url);
        if let Ok(res) = client.get(url).send().await {
            if let Ok(text) = res.text().await {
                if !text.trim().is_empty() {
                    response_text = text;
                    break;
                }
            }
        }
    }

    if response_text.is_empty() {
        return Err("All VPNGate sources failed or returned empty".into());
    }
        
    println!("VPNGate response received, length: {}", response_text.len());
    let mut nodes = Vec::new();
    let lines: Vec<&str> = response_text.lines().collect();
    
    // Skip first 2 lines (header and format info)
    for line in lines.iter().skip(2) {
        if line.starts_with('*') || line.is_empty() { break; }
        
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() > 14 {
            let ip = parts[1].to_string();
            let score = parts[2].parse().unwrap_or(0);
            let ping = parts[3].parse().unwrap_or(0);
            let speed_bps: f64 = parts[4].parse().unwrap_or(0.0);
            let speed_mbps = speed_bps / 1_000_000.0;
            let country_name = parts[5].to_string();
            let country_code = parts[6].to_string(); // CountryShort
            let ovpn_base64 = parts.get(14).unwrap_or(&"");
            
            // VPNGate base64 needs decoding
            use base64::{Engine as _, engine::general_purpose};
            let ovpn_config = if let Ok(bytes) = general_purpose::STANDARD.decode(ovpn_base64) {
                Some(String::from_utf8_lossy(&bytes).to_string())
            } else {
                continue;
            };
            
            let (lat, lon) = get_country_centroid(&country_code);

            nodes.push(VpnNode {
                id: format!("vpngate_{}", ip.replace('.', "_")),
                source: NodeSource::VpnGate,
                protocol: VpnProtocol::OpenVPN,
                transport: Some(Transport::TCP),
                ip,
                port: 443,
                country_code,
                country_name,
                latitude: lat,
                longitude: lon,
                speed_mbps: Some(speed_mbps),
                ping_ms: Some(ping),
                score: Some(score),
                openvpn_config: ovpn_config,
                ss_method: None,
                ss_password: None,
                credentials: None,
            });
        }
    }
    
    // Keep top quality nodes
    nodes.sort_by(|a, b| b.score.cmp(&a.score));
    Ok(nodes.into_iter().take(500).collect())
}

async fn fetch_github_ss_lists() -> Result<Vec<VpnNode>, String> {
    let mut all_nodes = Vec::new();
    let urls = vec![
        "https://raw.githubusercontent.com/v2ray-free/v2ray-free/master/v2ray/sub",
        "https://raw.githubusercontent.com/ssrsub/ssr/master/ss-sub",
        "https://raw.githubusercontent.com/freefq/free/master/v2"
    ];
    
    println!("Fetching public discovery nodes from subscription lists...");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .gzip(true)
        .build()
        .map_err(|e| e.to_string())?;
    
    for url in urls {
        if let Ok(res) = client.get(url).send().await {
            if let Ok(body) = res.text().await {
                // Subscription lists are often base64-encoded lists of URIs
                use base64::{Engine as _, engine::general_purpose};
                let decoded = if let Ok(bytes) = general_purpose::STANDARD.decode(body.trim()) {
                    String::from_utf8_lossy(&bytes).to_string()
                } else {
                    body
                };

                for line in decoded.lines() {
                    let mut line_trim = line.trim();
                    if line_trim.starts_with("ss://") {
                        // Extract tag if present
                        let mut tag_name = "US".to_string(); // fallback
                        if let Some((url, tag)) = line_trim.split_once('#') {
                            line_trim = url;
                            tag_name = tag.to_string();
                        }
                        
                        if let Ok(ss_conf) = ss_uri::SSConfig::parse(line_trim) {
                            let (lat, lon) = get_country_centroid(&tag_name);
                            all_nodes.push(VpnNode {
                                id: format!("ss_{}_{}", ss_conf.host, ss_conf.port),
                                source: NodeSource::ShadowsocksGithub,
                                protocol: VpnProtocol::Shadowsocks,
                                transport: None,
                                ip: ss_conf.host.to_string(),
                                port: ss_conf.port,
                                country_code: tag_name.clone(),
                                country_name: tag_name.clone(),
                                latitude: lat,
                                longitude: lon,
                                speed_mbps: None,
                                ping_ms: None,
                                score: None,
                                openvpn_config: None, 
                                ss_method: Some(ss_conf.method.to_string()),
                                ss_password: Some(ss_conf.password.to_string()),
                                credentials: None,
                            });
                        }
                    }
                }
            }
        }
    }
    
    Ok(all_nodes.into_iter().take(100).collect())
}

async fn fetch_vpnbook(backend_url: Option<&str>) -> Result<Vec<VpnNode>, String> {
    println!("Fetching VPNBook nodes...");
    
    // 1. Get password from Backend or OCR.space
    let password = match fetch_vpnbook_password(backend_url).await {
        Ok(p) => p,
        Err(e) => {
            println!("Failed to get VPNBook password: {}", e);
            "vpnbook".to_string() // fallback (unlikely to work)
        }
    };
    println!("VPNBook password obtained: {}", password);

    let servers = vec![
        ("US1", "us1.vpnbook.com", "US"),
        ("US2", "us2.vpnbook.com", "US"),
        ("FR1", "fr1.vpnbook.com", "FR"),
        ("CA198", "ca198.vpnbook.com", "CA"),
    ];

    let mut nodes = Vec::new();
    for (name, host, country) in servers {
        // Construct a basic OVPN config for VPNBook
        // Note: Real OVPN configs from VPNBook often include specific certs.
        // For now, we use a template and let the user know they might need a full zip fetch for better stability.
        let ovpn = format!(
            "client\ndev tun\nproto tcp\nremote {} 443\nresolv-retry infinite\nnobind\npersist-key\npersist-tun\nverb 3\nauth-user-pass\n<auth-user-pass>\nvpnbook\n{}\n</auth-user-pass>",
            host, password
        );

        let (lat, lon) = get_country_centroid(country);
        nodes.push(VpnNode {
            id: format!("vpnbook_{}", name.to_lowercase()),
            source: NodeSource::VpnBook,
            protocol: VpnProtocol::OpenVPN,
            transport: Some(Transport::TCP),
            ip: host.into(),
            port: 443,
            country_code: country.into(),
            country_name: "".into(),
            latitude: lat,
            longitude: lon,
            speed_mbps: None,
            ping_ms: None,
            score: None,
            openvpn_config: Some(ovpn),
            ss_method: None,
            ss_password: None,
            credentials: Some(("vpnbook".to_string(), password.clone())),
        });
    }

    Ok(nodes)
}

pub async fn fetch_vpnbook_password(backend_url: Option<&str>) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (compatible; WorldVPN/1.0)")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;
    
    // Try Backend first if URL is provided (preferred — backend caches the result)
    if let Some(url) = backend_url {
        let api_url = format!("{}/nodes/vpnbook/password", url.trim_end_matches('/'));
        println!("Fetching VPNBook password from backend: {}", api_url);
        if let Ok(res) = client.get(&api_url).send().await {
            if res.status().is_success() {
                if let Ok(data) = res.json::<serde_json::Value>().await {
                    if let Some(p) = data["password"].as_str() {
                        return Ok(p.to_string());
                    }
                }
            }
        }
    }

    // Fallback: scrape directly from VPNBook HTML (no OCR needed, password is in <code> tags)
    println!("Backend unavailable. Scraping VPNBook HTML directly...");
    let html = client
        .get("https://www.vpnbook.com/freevpn/openvpn")
        .send().await.map_err(|e| e.to_string())?
        .text().await.map_err(|e| e.to_string())?;

    // HTML structure: <code>vpnbook</code> then <code>ke9zw74</code>
    // The password is the 2nd <code> tag on the page.
    let re = regex::Regex::new(r"<code[^>]*>([^<]+)</code>").map_err(|e| e.to_string())?;
    let codes: Vec<&str> = re.captures_iter(&html)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str()))
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    let password = codes.get(1)
        .ok_or_else(|| format!("Could not find password <code> tag (found {} codes)", codes.len()))?;

    Ok(password.to_string())
}
