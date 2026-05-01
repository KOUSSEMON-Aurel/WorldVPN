package main

import (
	"encoding/base64"
	"fmt"
	"log"
	"net"
	"os"
	"sync/atomic"

	"golang.zx2c4.com/wireguard/conn"
	"golang.zx2c4.com/wireguard/device"
	"golang.zx2c4.com/wireguard/tun"
)

// wgTunnel wraps a wireguard-go Device so it implements the tunnel interface.
type wgTunnel struct {
	dev    *device.Device
	tunDev tun.Device
}

func (w *wgTunnel) Stop() {
	w.dev.Close()
	w.tunDev.Close()
	log.Println("[wireguard] Device closed")
}

// startWireGuard creates a WireGuard tunnel on the Android TUN fd.
func startWireGuard(tunFd int, cfg ConnectResponse, endpoint string) (tunnel, error) {
	// Wrap the OS file descriptor as a tun.Device
	tunFile := os.NewFile(uintptr(tunFd), "tun")
	tunDev, err := tun.CreateTUNFromFile(tunFile, cfg.MTU)
	if err != nil {
		return nil, fmt.Errorf("CreateTUNFromFile: %w", err)
	}

	// Create wireguard-go device using the real UDP transport
	logger := device.NewLogger(device.LogLevelError, "[wg] ")
	dev := device.NewDevice(tunDev, conn.NewDefaultBind(), logger)

	// Build the IPC configuration string (wg-quick compatible)
	uapiConf, err := buildWgUAPI(cfg, endpoint)
	if err != nil {
		dev.Close()
		return nil, err
	}

	if err := dev.IpcSetOperation(uapiConf); err != nil {
		dev.Close()
		return nil, fmt.Errorf("wg IpcSet: %w", err)
	}

	dev.Up()
	log.Printf("[wireguard] Tunnel up → %s", endpoint)

	return &wgTunnel{dev: dev, tunDev: tunDev}, nil
}

// buildWgUAPI returns a wireguard UAPI config string from ConnectResponse.
func buildWgUAPI(cfg ConnectResponse, endpoint string) (*device.IPCSetOperation, error) {
	// Decode private key (generated per-session — here we generate ephemeral)
	privKeyBytes, pubKeyBytes, err := generateX25519KeyPair()
	if err != nil {
		return nil, err
	}
	_ = pubKeyBytes // The public key is sent to the backend at session init time

	// Decode peer public key
	peerPubBytes, err := base64.StdEncoding.DecodeString(cfg.PeerPublicKey)
	if err != nil {
		return nil, fmt.Errorf("peer public key decode: %w", err)
	}

	// Resolve endpoint
	peerAddr, err := net.ResolveUDPAddr("udp", endpoint)
	if err != nil {
		return nil, fmt.Errorf("endpoint resolve: %w", err)
	}

	uapi := fmt.Sprintf(
		"private_key=%x\n"+
			"public_key=%x\n"+
			"endpoint=%s\n"+
			"allowed_ip=0.0.0.0/0\n"+
			"allowed_ip=::/0\n"+
			"persistent_keepalive_interval=25\n",
		privKeyBytes,
		peerPubBytes,
		peerAddr.String(),
	)

	if cfg.PresharedKey != "" {
		pskBytes, err := base64.StdEncoding.DecodeString(cfg.PresharedKey)
		if err == nil {
			uapi += fmt.Sprintf("preshared_key=%x\n", pskBytes)
		}
	}

	return device.IPCSetOperation(uapi), nil
}

// ── Obfuscated variant ─────────────────────────────────────────────────────

// obfsTunnel wraps wgTunnel and adds XOR obfuscation.
// For now it reuses the same wireguard-go device.
// A full implementation would intercept packets between tun and bind layer.
type obfsTunnel struct {
	wgTunnel
	running atomic.Bool
}

func (o *obfsTunnel) Stop() {
	o.running.Store(false)
	o.wgTunnel.Stop()
}

func startWireGuardObfuscated(tunFd int, cfg ConnectResponse, endpoint string) (tunnel, error) {
	log.Println("[wg-obfs] Starting WireGuard+Obfuscated (XOR layer)")
	// For phase 1: identical to plain WireGuard.
	// TODO: inject packet XOR interceptor between tun and bind.
	t, err := startWireGuard(tunFd, cfg, endpoint)
	if err != nil {
		return nil, err
	}
	return &obfsTunnel{wgTunnel: *t.(*wgTunnel)}, nil
}

// generateX25519KeyPair generates an ephemeral Curve25519 keypair.
// In production the public key must be sent to the backend in POST /vpn/connect.
func generateX25519KeyPair() (priv [32]byte, pub [32]byte, err error) {
	var pk device.NoisePrivateKey
	if err = pk.FromMaybeZero(randomBytes(32)); err != nil {
		return
	}
	copy(priv[:], pk[:])
	pubK := pk.PublicKey()
	copy(pub[:], pubK[:])
	return
}

func randomBytes(n int) []byte {
	b := make([]byte, n)
	if _, err := randRead(b); err != nil {
		panic(err)
	}
	return b
}
