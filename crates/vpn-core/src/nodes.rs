use serde::{Deserialize, Serialize};
use crate::protocol::VpnProtocol;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeSource {
    VpnGate,
    ShadowsocksGithub,
    VpnBook,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Transport {
    TCP,
    UDP,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VpnNode {
    /// Unique hash of IP+port+protocol
    pub id: String,
    pub source: NodeSource,
    pub protocol: VpnProtocol,
    pub transport: Option<Transport>, // Mostly for OpenVPN
    
    pub ip: String,
    pub port: u16,
    pub country_code: String,
    pub country_name: String,
    pub latitude: f64,
    pub longitude: f64,
    
    // Metadata/Quality parameters
    pub speed_mbps: Option<f64>,
    pub ping_ms: Option<u32>,
    pub score: Option<i64>,
    
    // Protocol specific connection parameters
    pub openvpn_config: Option<String>,
    pub ss_method: Option<String>,
    pub ss_password: Option<String>,
    pub credentials: Option<(String, String)>,
}

/// Helper method to return hardcoded centroid by country code (ISO 2-letter).
/// This serves as a fallback until a full GeoLite2 database is locally embedded.
pub fn get_country_centroid(cc: &str) -> (f64, f64) {
    match cc.to_uppercase().as_str() {
        "JP" => (36.2048, 138.2529),
        "US" => (37.0902, -95.7129),
        "KR" => (35.9078, 127.7669),
        "GB" | "UK" => (55.3781, -3.4360),
        "VN" => (14.0583, 108.2772),
        "TW" => (23.6978, 120.9605),
        "TH" => (15.8700, 100.9925),
        "ID" => (-0.7893, 113.9213),
        "CN" => (35.8617, 104.1954),
        "IN" => (20.5937, 78.9629),
        "DE" => (51.1657, 10.4515),
        "FR" => (46.2276, 2.2137),
        "CA" => (56.1304, -106.3468),
        "AU" => (-25.2744, 133.7751),
        "BR" => (-14.2350, -51.9253),
        "RU" => (61.5240, 105.3188),
        "SG" => (1.3521, 103.8198),
        "PH" => (12.8797, 121.7740),
        "MY" => (4.2105, 101.9758),
        "HK" => (22.3193, 114.1694),
        "UA" => (48.3794, 31.1656),
        "PL" => (51.9194, 19.1451),
        "TR" => (38.9637, 35.2433),
        // Fallback: Atlantic Ocean (null-island approx)
        _ => (0.0, 0.0),
    }
}
