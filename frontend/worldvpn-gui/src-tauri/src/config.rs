use serde_json::{json, Value};
use std::net::SocketAddr;

pub fn build_sing_box_config(
    protocol: &str,
    server_addr: SocketAddr,
    assigned_ip: &str,
    private_key_b64: &str,
    peer_pub_b64: &str,
    mtu: u32,
) -> Value {
    let host = server_addr.ip().to_string();
    let port = server_addr.port();

    // Endpoints - WireGuard MUST be an endpoint in sing-box 1.13+
    let endpoints = if protocol == "WireGuard" {
        json!([
            {
                "type": "wireguard",
                "tag": "wg-out",
                "address": [format!("{}/32", assigned_ip)],
                "private_key": private_key_b64,
                "peers": [
                    {
                        "address": host,
                        "port": port,
                        "public_key": peer_pub_b64,
                        "allowed_ips": ["0.0.0.0/0"]
                    }
                ],
                "mtu": mtu
            }
        ])
    } else {
        json!([])
    };

    // Outbounds
    let outbounds = if protocol == "WireGuard" {
        json!([
            {
                "type": "selector",
                "tag": "proxy",
                "outbounds": ["wg-out", "direct-out"],
                "default": "wg-out"
            },
            {
                "type": "direct",
                "tag": "direct-out"
            }
        ])
    } else if protocol == "Shadowsocks" {
        let parts: Vec<&str> = peer_pub_b64.split(':').collect();
        let method = parts.get(0).unwrap_or(&"aes-256-gcm"); // Better default for VPNGate
        let password = parts.get(1).unwrap_or(&"m");

        json!([
            {
                "type": "selector",
                "tag": "proxy",
                "outbounds": ["ss-out", "direct-out"],
                "default": "ss-out"
            },
            {
                "type": "shadowsocks",
                "tag": "ss-out",
                "server": host,
                "server_port": port,
                "method": method,
                "password": password
            },
            {
                "type": "direct",
                "tag": "direct-out"
            }
        ])
    } else {
        json!([
            {
                "type": "direct",
                "tag": "proxy"
            }
        ])
    };

    // DNS - Enhanced for reliability and blocking leaks
    let dns = json!({
        "servers": [
            {
                "tag": "cloudflare",
                "address": "udp://1.1.1.1",
                "detour": "proxy"
            }
        ],
        "rules": [
            {
                "outbound": "any",
                "server": "cloudflare"
            }
        ],
        "strategy": "prefer_ipv4",
        "disable_cache": false
    });

    // Inbounds (TUN)
    let inbounds = json!([
        {
            "type": "tun",
            "tag": "tun-in",
            "address": [format!("{}/32", assigned_ip)],
            "auto_route": true, 
            "strict_route": true,
            "stack": "gvisor",
            "mtu": mtu
        }
    ]);

    // Routing
    let route = json!({
        "auto_detect_interface": true,
        "rules": [
            {
                "protocol": "dns",
                "action": "hijack-dns"
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
        "dns": dns,
        "endpoints": endpoints,
        "inbounds": inbounds,
        "outbounds": outbounds,
        "route": route
    })
}
