use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

/// Thresholds for detecting abusive behavior
#[derive(Debug, Clone)]
pub struct AbuseThresholds {
    pub max_traffic_per_minute: u64,
    pub max_connections_per_minute: u32,
    pub max_unique_ports_per_minute: u32,
    pub min_share_ratio: f64,
    pub ban_duration_secs: u64,
}

impl Default for AbuseThresholds {
    fn default() -> Self {
        Self {
            max_traffic_per_minute: 1_073_741_824, // 1 GB/min
            max_connections_per_minute: 1000,
            max_unique_ports_per_minute: 100,
            min_share_ratio: 0.1, // Minimum 10% upload/download ratio
            ban_duration_secs: 3600, // 1 hour ban by default
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AbuseType {
    TrafficFlooding,
    PortScanning,
    LowShareRatio,
    SuspiciousConnections,
    DdosPattern,
}

/// Represents a recorded abuse incident
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbuseEvent {
    pub user_id: String,
    pub abuse_type: AbuseType,
    pub severity: u8, // 1 (low) to 10 (critical)
    pub timestamp: i64,
    pub details: String,
}

#[derive(Debug, Clone)]
struct UserMetrics {
    traffic_windows: Vec<(Instant, u64)>,
    contacted_ips: HashMap<IpAddr, u32>,
    contacted_ports: HashMap<u16, u32>,
    connection_count: u32,
    last_reset: Instant,
    risk_score: u8,
}

impl Default for UserMetrics {
    fn default() -> Self {
        Self {
            traffic_windows: Vec::new(),
            contacted_ips: HashMap::new(),
            contacted_ports: HashMap::new(),
            connection_count: 0,
            last_reset: Instant::now(),
            risk_score: 0,
        }
    }
}

/// Core engine for identifying malicious or abusive network activity
pub struct AbuseDetector {
    thresholds: AbuseThresholds,
    user_metrics: HashMap<String, UserMetrics>,
    banned_users: HashMap<String, Instant>,
    abuse_events: Vec<AbuseEvent>,
}

impl AbuseDetector {
    pub fn new(thresholds: AbuseThresholds) -> Self {
        Self {
            thresholds,
            user_metrics: HashMap::new(),
            banned_users: HashMap::new(),
            abuse_events: Vec::new(),
        }
    }

    /// Logs traffic volume for a specific user and checks against quotas
    pub fn record_traffic(&mut self, user_id: &str, bytes: u64) {
        let metrics = self.user_metrics.entry(user_id.to_string()).or_default();
        
        let now = Instant::now();
        metrics.traffic_windows.push((now, bytes));
        
        // Retain only the last 60 seconds of traffic data
        metrics.traffic_windows.retain(|(timestamp, _)| {
            now.duration_since(*timestamp) < Duration::from_secs(60)
        });
        
        let total_traffic: u64 = metrics.traffic_windows.iter().map(|(_, b)| b).sum();
        if total_traffic > self.thresholds.max_traffic_per_minute {
            self.report_abuse(user_id, AbuseType::TrafficFlooding, 8, 
                format!("Excessive traffic: {} bytes/min", total_traffic));
        }
    }

    /// Tracks connection targets to identify port scanning or connection flooding
    pub fn record_connection(&mut self, user_id: &str, dest_ip: IpAddr, dest_port: u16) {
        let metrics = self.user_metrics.entry(user_id.to_string()).or_default();
        
        let now = Instant::now();
        
        // Reset metrics window every minute
        if now.duration_since(metrics.last_reset) > Duration::from_secs(60) {
            metrics.contacted_ips.clear();
            metrics.contacted_ports.clear();
            metrics.connection_count = 0;
            metrics.last_reset = now;
        }
        
        *metrics.contacted_ips.entry(dest_ip).or_insert(0) += 1;
        *metrics.contacted_ports.entry(dest_port).or_insert(0) += 1;
        metrics.connection_count += 1;
        
        let num_ports = metrics.contacted_ports.len();
        let num_connections = metrics.connection_count;
        
        if num_ports > self.thresholds.max_unique_ports_per_minute as usize {
            self.report_abuse(user_id, AbuseType::PortScanning, 9,
                format!("Port scan detected: {} unique ports", num_ports));
        }
        
        if num_connections > self.thresholds.max_connections_per_minute {
            self.report_abuse(user_id, AbuseType::SuspiciousConnections, 7,
                format!("Excessive connections: {} connections", num_connections));
        }
    }

    /// Enforces the P2P economy by checking the sharing ratio
    pub fn check_share_ratio(&mut self, user_id: &str, shared_bytes: u64, consumed_bytes: u64) {
        if consumed_bytes == 0 {
            return;
        }
        
        let ratio = shared_bytes as f64 / consumed_bytes as f64;
        
        if ratio < self.thresholds.min_share_ratio {
            self.report_abuse(user_id, AbuseType::LowShareRatio, 5,
                format!("Low share ratio: {:.2}%", ratio * 100.0));
        }
    }

    /// Detects patterns indicative of participation in a DDoS attack
    pub fn detect_ddos_pattern(&mut self, user_id: &str) -> bool {
        let metrics = match self.user_metrics.get(user_id) {
            Some(m) => m,
            None => return false,
        };
        
        let unique_ips = metrics.contacted_ips.len();
        let total_traffic: u64 = metrics.traffic_windows.iter().map(|(_, b)| b).sum();
        
        if unique_ips > 50 && total_traffic > 524_288_000 { // 500 MB/min
            self.report_abuse(user_id, AbuseType::DdosPattern, 10,
                format!("DDoS pattern detected: {} IPs, {} bytes", unique_ips, total_traffic));
            return true;
        }
        
        false
    }

    /// Internal helper to record violations and trigger bans
    fn report_abuse(&mut self, user_id: &str, abuse_type: AbuseType, severity: u8, details: String) {
        let event = AbuseEvent {
            user_id: user_id.to_string(),
            abuse_type,
            severity,
            timestamp: chrono::Utc::now().timestamp(),
            details,
        };
        
        tracing::warn!("Abuse detected: {:?}", event);
        self.abuse_events.push(event);
        
        // Automatically ban for high-severity violations
        if severity >= 8 {
            self.ban_user(user_id);
        }
        
        if let Some(metrics) = self.user_metrics.get_mut(user_id) {
            metrics.risk_score = (metrics.risk_score + severity * 10).min(100);
        }
    }

    pub fn ban_user(&mut self, user_id: &str) {
        let ban_until = Instant::now() + Duration::from_secs(self.thresholds.ban_duration_secs);
        self.banned_users.insert(user_id.to_string(), ban_until);
        tracing::warn!("User banned: {} until {:?}", user_id, ban_until);
    }

    pub fn is_banned(&mut self, user_id: &str) -> bool {
        if let Some(ban_until) = self.banned_users.get(user_id) {
            if Instant::now() < *ban_until {
                return true;
            } else {
                self.banned_users.remove(user_id);
            }
        }
        false
    }

    pub fn get_risk_score(&self, user_id: &str) -> u8 {
        self.user_metrics.get(user_id).map(|m| m.risk_score).unwrap_or(0)
    }

    pub fn get_abuse_history(&self, user_id: Option<&str>, limit: usize) -> Vec<AbuseEvent> {
        let events: Vec<_> = if let Some(uid) = user_id {
            self.abuse_events.iter()
                .filter(|e| e.user_id == uid)
                .cloned()
                .collect()
        } else {
            self.abuse_events.clone()
        };
        
        events.into_iter().rev().take(limit).collect()
    }

    pub fn reset_user_score(&mut self, user_id: &str) {
        if let Some(metrics) = self.user_metrics.get_mut(user_id) {
            metrics.risk_score = 0;
        }
        self.banned_users.remove(user_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_traffic_flooding_detection() {
        let mut detector = AbuseDetector::new(AbuseThresholds {
            max_traffic_per_minute: 1000,
            ..Default::default()
        });

        detector.record_traffic("user1", 1500);
        
        assert_eq!(detector.abuse_events.len(), 1);
        assert_eq!(detector.abuse_events[0].abuse_type, AbuseType::TrafficFlooding);
        assert!(detector.is_banned("user1"));
    }

    #[test]
    fn test_port_scanning_detection() {
        let mut detector = AbuseDetector::new(AbuseThresholds {
            max_unique_ports_per_minute: 10,
            ..Default::default()
        });

        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        for port in 1..=15 {
            detector.record_connection("scanner", ip, port);
        }

        assert!(detector.abuse_events.iter().any(|e| e.abuse_type == AbuseType::PortScanning));
    }

    #[test]
    fn test_share_ratio_check() {
        let mut detector = AbuseDetector::new(AbuseThresholds {
            min_share_ratio: 0.5,
            ..Default::default()
        });

        detector.check_share_ratio("good_user", 500, 1000);
        assert_eq!(detector.abuse_events.len(), 0);

        detector.check_share_ratio("bad_user", 100, 1000);
        assert!(detector.abuse_events.iter().any(|e| e.abuse_type == AbuseType::LowShareRatio));
    }

    #[test]
    fn test_ban_expiration() {
        let mut detector = AbuseDetector::new(AbuseThresholds {
            ban_duration_secs: 0,
            ..Default::default()
        });

        detector.ban_user("test_user");
        std::thread::sleep(std::time::Duration::from_millis(10));
        
        assert!(!detector.is_banned("test_user"));
    }
}
