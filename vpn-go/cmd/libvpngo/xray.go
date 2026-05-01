package main

import (
	"context"
	"fmt"
	"log"
	"os"

	"github.com/xtls/xray-core/app/dispatcher"
	"github.com/xtls/xray-core/app/proxyman"
	"github.com/xtls/xray-core/common/net"
	"github.com/xtls/xray-core/core"
	_ "github.com/xtls/xray-core/main/distro/all" // registers all protocols
	"github.com/xtls/xray-core/proxy/freedom"
	trojanProxy "github.com/xtls/xray-core/proxy/trojan"
	vlessProxy "github.com/xtls/xray-core/proxy/vless/outbound"

	"google.golang.org/protobuf/types/known/anypb"
)

// xrayTunnel wraps Xray-core instance + tun routing.
type xrayTunnel struct {
	cancel   context.CancelFunc
	instance *core.Instance
}

func (x *xrayTunnel) Stop() {
	x.cancel()
	if x.instance != nil {
		x.instance.Close()
	}
	log.Println("[xray] Tunnel stopped")
}

// startXray starts a VLESS or Trojan tunnel through Xray-core.
// protoName is "vless" or "trojan".
func startXray(tunFd int, cfg ConnectResponse, endpoint string, protoName string) (tunnel, error) {
	log.Printf("[xray/%s] Connecting to %s", protoName, endpoint)

	host, portStr, err := splitHostPort(endpoint)
	if err != nil {
		return nil, fmt.Errorf("xray endpoint: %w", err)
	}

	port, err := net.PortFromString(portStr)
	if err != nil {
		return nil, fmt.Errorf("xray port: %w", err)
	}

	// Build outbound proxy settings
	var outboundProxySettings *anypb.Any
	switch protoName {
	case "vless":
		outboundProxySettings, err = buildVlessSettings(cfg, host, port)
	case "trojan":
		outboundProxySettings, err = buildTrojanSettings(cfg, host, port)
	default:
		return nil, fmt.Errorf("unknown xray protocol: %s", protoName)
	}
	if err != nil {
		return nil, err
	}

	// Build full Xray config
	config := &core.Config{
		App: []*anypb.Any{
			mustMarshal(&dispatcher.Config{}),
			mustMarshal(&proxyman.InboundConfig{}),
			mustMarshal(&proxyman.OutboundConfig{}),
		},
		Inbound:  buildTunInbound(cfg),
		Outbound: buildOutbound(protoName, outboundProxySettings),
	}

	instance, err := core.New(config)
	if err != nil {
		return nil, fmt.Errorf("xray New: %w", err)
	}

	ctx, cancel := context.WithCancel(context.Background())

	// Attach TUN fd — routes raw IP packets into Xray's dispatcher
	tunFile := os.NewFile(uintptr(tunFd), "tun")
	go routeTunToXray(ctx, tunFile, instance, cfg)

	if err := instance.Start(); err != nil {
		cancel()
		return nil, fmt.Errorf("xray Start: %w", err)
	}

	log.Printf("[xray/%s] Instance started → %s", protoName, endpoint)
	return &xrayTunnel{cancel: cancel, instance: instance}, nil
}

// buildVlessSettings constructs VLESS outbound settings protobuf.
func buildVlessSettings(cfg ConnectResponse, host string, port net.Port) (*anypb.Any, error) {
	uuid := cfg.UUID
	if uuid == "" {
		return nil, fmt.Errorf("vless: uuid required")
	}
	s := &vlessProxy.Config{
		Vnext: []*vlessProxy.ServerEndpoint{
			{
				Address: net.NewIPOrDomain(net.ParseAddress(host)),
				Port:    uint32(port),
				User: []*core.User{
					{
						Account: mustMarshal(&vlessProxy.Account{
							Id:         uuid,
							Encryption: "none",
						}),
					},
				},
			},
		},
	}
	return anypb.New(s)
}

// buildTrojanSettings constructs Trojan outbound settings protobuf.
func buildTrojanSettings(cfg ConnectResponse, host string, port net.Port) (*anypb.Any, error) {
	password := cfg.Password
	if password == "" {
		return nil, fmt.Errorf("trojan: password required")
	}
	s := &trojanProxy.ClientConfig{
		Server: []*trojanProxy.ServerEndpoint{
			{
				Address:  net.NewIPOrDomain(net.ParseAddress(host)),
				Port:     uint32(port),
				Password: []string{password},
			},
		},
	}
	return anypb.New(s)
}

// buildOutbound assembles an Xray outbound handler config.
func buildOutbound(tag string, settings *anypb.Any) []*core.OutboundHandlerConfig {
	return []*core.OutboundHandlerConfig{
		{
			Tag:           tag,
			ProxySettings: settings,
			// TLS via StreamSettings added as needed
		},
		// Freedom outbound for direct traffic (DNS etc.)
		{
			Tag:           "direct",
			ProxySettings: mustMarshal(&freedom.Config{}),
		},
	}
}

// buildTunInbound creates a TUN-mode inbound (Xray ≥1.8 supports tun directly).
func buildTunInbound(cfg ConnectResponse) []*core.InboundHandlerConfig {
	// For Xray <1.8 without native TUN: use SOCKS5 inbound + external tun2socks.
	// For Xray ≥1.8 with tun support: configure TUN device here.
	// Currently we handle routing externally via routeTunToXray().
	return nil
}

// routeTunToXray reads raw IP packets from the TUN fd and injects them into Xray.
func routeTunToXray(ctx context.Context, tunFile *os.File, instance *core.Instance, cfg ConnectResponse) {
	buf := make([]byte, cfg.MTU+4)
	for {
		select {
		case <-ctx.Done():
			return
		default:
		}
		n, err := tunFile.Read(buf)
		if err != nil || n == 0 {
			if ctx.Err() != nil {
				return
			}
			continue
		}
		// Raw IP packet in buf[:n].
		// TODO: feed into Xray dispatcher via net.Conn abstraction.
		// Full implementation requires gvisor/netstack TCP/IP stack.
		_ = instance
	}
}
