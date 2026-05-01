// Package vpngo exposes VPN tunnel functions via CGO for Android.
// Entry point: StartTunnel / StopTunnel called by WorldVpnService.kt via JNI.
package main

/*
#include <stdlib.h>
*/
import "C"
import (
	"encoding/json"
	"log"
	"sync"
	"unsafe"
)

// ConnectResponse mirrors the backend's response from POST /vpn/connect.
type ConnectResponse struct {
	SessionID      string `json:"session_id"`
	Protocol       string `json:"protocol"`        // "WireGuard" | "Shadowsocks" | "Hysteria2" | "Trojan" | "VLESS"
	VirtualIP      string `json:"assigned_ip"`     // "10.0.0.X"
	PeerEndpoint   string `json:"peer_endpoint"`   // "1.2.3.4:51820"
	PeerPublicKey  string `json:"peer_public_key"` // base64 X25519 (WireGuard)
	ServerEndpoint string `json:"server_endpoint"` // fallback if peer_endpoint absent
	PresharedKey   string `json:"preshared_key"`   // optional
	Password       string `json:"password"`        // Shadowsocks / Hysteria2 / Trojan
	UUID           string `json:"uuid"`            // VLESS user uuid
	DNS            string `json:"dns"`
	MTU            int    `json:"mtu"`
}

var (
	mu      sync.Mutex
	running tunnel
)

type tunnel interface {
	Stop()
}

// StartTunnel is exported via CGO. tunFd is the Android TUN file descriptor.
// configJSON is the JSON-encoded ConnectResponse from the backend.
//
//export StartTunnel
func StartTunnel(tunFd C.int, configJSON *C.char) C.int {
	mu.Lock()
	defer mu.Unlock()

	if running != nil {
		running.Stop()
		running = nil
	}

	jsonStr := C.GoString(configJSON)
	var cfg ConnectResponse
	if err := json.Unmarshal([]byte(jsonStr), &cfg); err != nil {
		log.Printf("[vpn-go] JSON parse error: %v", err)
		return -1
	}

	fd := int(tunFd)
	if cfg.MTU == 0 {
		cfg.MTU = 1420
	}
	if cfg.DNS == "" {
		cfg.DNS = "1.1.1.1"
	}

	// Resolve server address: prefer peer_endpoint, fallback to server_endpoint
	endpoint := cfg.PeerEndpoint
	if endpoint == "" {
		endpoint = cfg.ServerEndpoint
	}

	var t tunnel
	var err error

	switch cfg.Protocol {
	case "WireGuard":
		t, err = startWireGuard(fd, cfg, endpoint)
	case "WireGuardObfuscated":
		t, err = startWireGuardObfuscated(fd, cfg, endpoint)
	case "Shadowsocks":
		t, err = startShadowsocks(fd, cfg, endpoint)
	case "Hysteria2":
		t, err = startHysteria2(fd, cfg, endpoint)
	case "Trojan":
		t, err = startXray(fd, cfg, endpoint, "trojan")
	case "VLESS":
		t, err = startXray(fd, cfg, endpoint, "vless")
	default:
		log.Printf("[vpn-go] Unknown protocol: %s", cfg.Protocol)
		return -1
	}

	if err != nil {
		log.Printf("[vpn-go] Tunnel start error (%s): %v", cfg.Protocol, err)
		return -1
	}

	running = t
	log.Printf("[vpn-go] Tunnel started: protocol=%s endpoint=%s", cfg.Protocol, endpoint)
	return 0
}

// StopTunnel stops the currently running tunnel.
//
//export StopTunnel
func StopTunnel() {
	mu.Lock()
	defer mu.Unlock()
	if running != nil {
		running.Stop()
		running = nil
		log.Println("[vpn-go] Tunnel stopped")
	}
}

// Required for CGO c-shared build — must be present but empty.
func main() {}

// Suppress unused import warning for unsafe (needed for CGO types)
var _ = unsafe.Pointer(nil)
