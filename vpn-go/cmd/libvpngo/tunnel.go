package main

/*
#include <stdlib.h>
*/
import "C"

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"net"
	"sync"

	box "github.com/sagernet/sing-box"
	_ "github.com/sagernet/sing-box/include"
	"github.com/sagernet/sing-box/option"
)

// ConnectResponse mirrors the backend's response.
type ConnectResponse struct {
	SessionID      string `json:"session_id"`
	Protocol       string `json:"protocol"`
	VirtualIP      string `json:"assigned_ip"`
	PeerEndpoint   string `json:"peer_endpoint"`
	PeerPublicKey  string `json:"peer_public_key"`
	PrivateKey     string `json:"private_key"`
	ServerEndpoint string `json:"server_endpoint"`
	Password       string `json:"password"`
	UUID           string `json:"uuid"`
	DNS            string `json:"dns"`
	MTU            int    `json:"mtu"`
}

var (
	instance *box.Box
	lock     sync.Mutex
)

//export StartTunnel
func StartTunnel(tunFd C.int, configJSON *C.char) C.int {
	lock.Lock()
	defer lock.Unlock()

	if instance != nil {
		log.Println("[vpn-go] Tunnel already running")
		return 0
	}

	cfgStr := C.GoString(configJSON)
	var cfg ConnectResponse
	if err := json.Unmarshal([]byte(cfgStr), &cfg); err != nil {
		log.Printf("[vpn-go] JSON unmarshal error: %v", err)
		return -1
	}

	// 1. Build Minimal sing-box config (No auto_route, no complex registries)
	host, port, _ := splitAddr(cfg.PeerEndpoint)
	outbound := map[string]interface{}{
		"type":          "wireguard",
		"tag":           "proxy",
		"server":        host,
		"server_port":   port,
		"local_address": []string{cfg.VirtualIP + "/32"},
		"private_key":   cfg.PrivateKey,
		"public_key":    cfg.PeerPublicKey,
		"mtu":           cfg.MTU,
	}

	tunInbound := map[string]interface{}{
		"type":          "tun",
		"tag":           "tun-in",
		"mtu":           cfg.MTU,
		"inet4_address": []string{cfg.VirtualIP + "/32"},
		"stack":         "gvisor",
		"auto_route":    false, // TEST: Disable auto_route to avoid platform registry
	}

	configMap := map[string]interface{}{
		"log": map[string]interface{}{
			"level": "trace",
		},
		"inbounds":  []interface{}{tunInbound},
		"outbounds": []interface{}{outbound},
	}

	fullCfgBytes, _ := json.Marshal(configMap)

	var opts option.Options
	if err := json.Unmarshal(fullCfgBytes, &opts); err != nil {
		log.Printf("[vpn-go] Config parse error: %v", err)
		return -1
	}

	// 2. Initialize Box
	b, err := box.New(box.Options{
		Context: context.Background(),
		Options: opts,
	})
	if err != nil {
		log.Printf("[vpn-go] sing-box create error: %v", err)
		return -1
	}

	if err := b.Start(); err != nil {
		log.Printf("[vpn-go] sing-box start error: %v", err)
		b.Close()
		return -1
	}

	instance = b
	log.Printf("[vpn-go] Tunnel started minimal")
	return 0
}

//export StopTunnel
func StopTunnel() {
	lock.Lock()
	defer lock.Unlock()

	if instance != nil {
		instance.Close()
		instance = nil
		log.Println("[vpn-go] Tunnel stopped")
	}
}

func buildConfigMap(tunFd int, cfg ConnectResponse) map[string]interface{} {
	endpoint := cfg.PeerEndpoint
	if endpoint == "" {
		endpoint = cfg.ServerEndpoint
	}
	host, port, _ := splitAddr(endpoint)

	// Build Inbound
	tunInbound := map[string]interface{}{
		"type":          "tun",
		"tag":           "tun-in",
		"mtu":           cfg.MTU,
		"inet4_address": []string{cfg.VirtualIP + "/32"},
		"stack":         "gvisor",
		"auto_route":    true,
		"strict_route":  true,
	}

	// On Android, use the provided File Descriptor
	if tunFd > 0 {
		tunInbound["fd"] = tunFd
	}

	// Build Outbound
	var outbound map[string]interface{}
	switch cfg.Protocol {
	case "WireGuard":
		outbound = map[string]interface{}{
			"type":          "wireguard",
			"tag":           "proxy",
			"server":        host,
			"server_port":   port,
			"local_address": []string{cfg.VirtualIP + "/32"},
			"private_key":   cfg.PrivateKey,
			"public_key":    cfg.PeerPublicKey,
			"mtu":           cfg.MTU,
		}
	case "Shadowsocks":
		outbound = map[string]interface{}{
			"type":        "shadowsocks",
			"tag":         "proxy",
			"server":      host,
			"server_port": port,
			"method":      "aes-256-gcm",
			"password":    cfg.Password,
		}
	case "Hysteria2":
		outbound = map[string]interface{}{
			"type":        "hysteria2",
			"tag":         "proxy",
			"server":      host,
			"server_port": port,
			"password":    cfg.Password,
		}
	case "VLESS":
		outbound = map[string]interface{}{
			"type":        "vless",
			"tag":         "proxy",
			"server":      host,
			"server_port": port,
			"uuid":        cfg.UUID,
		}
	case "Trojan":
		outbound = map[string]interface{}{
			"type":        "trojan",
			"tag":         "proxy",
			"server":      host,
			"server_port": port,
			"password":    cfg.Password,
		}
	}

	// Build DNS
	dnsConfig := map[string]interface{}{
		"servers": []interface{}{
			map[string]interface{}{
				"address": "1.1.1.1",
				"tag":     "cloudflare",
			},
		},
		"strategy": "prefer_ipv4",
	}

	// Build Route
	routeConfig := map[string]interface{}{
		"auto_route": true,
		"rules": []interface{}{
			map[string]interface{}{
				"protocol": "dns",
				"outbound": "dns-out",
			},
			map[string]interface{}{
				"inbound":  []string{"tun-in"},
				"outbound": "proxy",
			},
		},
	}

	// Add DNS Outbound
	dnsOutbound := map[string]interface{}{
		"type": "dns",
		"tag":  "dns-out",
	}

	return map[string]interface{}{
		"inbounds":  []interface{}{tunInbound},
		"outbounds": []interface{}{outbound, dnsOutbound},
		"dns":       dnsConfig,
		"route":     routeConfig,
	}
}

func splitAddr(addr string) (string, uint16, error) {
	h, p, err := net.SplitHostPort(addr)
	if err != nil {
		return addr, 443, nil
	}
	var port uint16
	fmt.Sscanf(p, "%d", &port)
	return h, port, nil
}

func main() {}
