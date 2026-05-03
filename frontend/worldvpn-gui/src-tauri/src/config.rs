use serde_json::{json, Value};
use std::net::SocketAddr;

pub fn build_sing_box_config(
    protocol: &str,
    server_addr: SocketAddr,
    assigned_ip: &str,
    private_key: &[u8],
    peer_public_key: &[u8],
    mtu: u32,
) -> Value {
    let host = server_addr.ip().to_string();
    let port = server_addr.port();

    // 1. Outbounds
    let outbound = if protocol == "WireGuard" {
        json!({
            "type": "wireguard",
            "tag": "proxy",
            "address": [format!("{}/32", assigned_ip)],
            "private_key": hex::encode(private_key),
            "peers": [
                {
                    "address": host,
                    "port": port,
                    "public_key": hex::encode(peer_public_key)
                }
            ],
            "mtu": mtu,
        })
    } else {
        // Fallback for other protocols if needed (currently minimal)
        json!({
            "type": "direct",
            "tag": "proxy"
        })
    };

    let dns_outbound = json!({
        "type": "dns",
        "tag": "dns-out"
    });

    // 2. Inbounds
    let tun_inbound = json!({
        "type": "tun",
        "tag": "tun-in",
        "mtu": mtu,
        "inet4_address": [format!("{}/32", assigned_ip)],
        "stack": "gvisor",
        "auto_route": true,
        "strict_route": true
    });

    // 3. DNS
    let dns_config = json!({
        "servers": [
            {
                "address": "1.1.1.1",
                "tag": "cloudflare"
            }
        ],
        "strategy": "prefer_ipv4"
    });

    // 4. Route
    let route_config = json!({
        "auto_route": true,
        "rules": [
            {
                "protocol": "dns",
                "outbound": "dns-out"
            },
            {
                "inbound": ["tun-in"],
                "outbound": "proxy"
            }
        ]
    });

    json!({
        "log": {
            "level": "trace",
            "timestamp": true
        },
        "inbounds": [tun_inbound],
        "outbounds": [outbound, dns_outbound],
        "dns": dns_config,
        "route": route_config
    })
}
