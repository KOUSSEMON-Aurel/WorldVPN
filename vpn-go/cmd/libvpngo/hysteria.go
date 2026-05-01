package main

import (
	"context"
	"fmt"
	"log"
	"os"

	hyClient "github.com/apernet/hysteria/core/v2/client"
	"github.com/apernet/hysteria/core/v2/client/tun"
)

// hysteriaTunnel wraps a Hysteria2 client.
type hysteriaTunnel struct {
	cancel context.CancelFunc
	client hyClient.Client
}

func (h *hysteriaTunnel) Stop() {
	h.cancel()
	if h.client != nil {
		h.client.Close()
	}
	log.Println("[hysteria2] Tunnel stopped")
}

// startHysteria2 connects to a Hysteria2 server via QUIC and routes TUN traffic.
func startHysteria2(tunFd int, cfg ConnectResponse, endpoint string) (tunnel, error) {
	log.Printf("[hysteria2] Connecting to %s", endpoint)

	if cfg.Password == "" {
		return nil, fmt.Errorf("hysteria2: password required")
	}

	ctx, cancel := context.WithCancel(context.Background())

	// Build Hysteria2 client config
	clientCfg := &hyClient.Config{
		ServerAddr: endpoint,
		Auth:       cfg.Password,
		TLSConfig: hyClient.TLSConfig{
			InsecureSkipVerify: true, // Set to false in production with real certs
		},
		BandwidthConfig: hyClient.BandwidthConfig{
			MaxRx: 200 * 1024 * 1024, // 200 Mbps download
			MaxTx: 50 * 1024 * 1024,  // 50 Mbps upload
		},
	}

	// Create Hysteria2 client
	c, err := hyClient.NewClient(clientCfg)
	if err != nil {
		cancel()
		return nil, fmt.Errorf("hysteria2 client: %w", err)
	}

	// Attach the TUN fd — Hysteria2 has first-class TUN support
	tunFile := os.NewFile(uintptr(tunFd), "tun")
	tunCfg := &tun.Config{
		MTU:        cfg.MTU,
		Inet4Addr:  cfg.VirtualIP,
		AllowedIPs: []string{"0.0.0.0/0"},
	}

	go func() {
		if err := tun.Run(ctx, tunFile, c, tunCfg); err != nil {
			if ctx.Err() == nil {
				log.Printf("[hysteria2] tun.Run error: %v", err)
			}
		}
	}()

	log.Printf("[hysteria2] Tunnel up → %s | virtual_ip=%s", endpoint, cfg.VirtualIP)
	return &hysteriaTunnel{cancel: cancel, client: c}, nil
}
