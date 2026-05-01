package main

import (
	"context"
	"fmt"
	"log"
	"net"
	"os"
	"time"

	"github.com/shadowsocks/go-shadowsocks2/core"
	"github.com/shadowsocks/go-shadowsocks2/socks"
)

// ssTunnel manages a Shadowsocks tunnel.
type ssTunnel struct {
	cancel context.CancelFunc
}

func (s *ssTunnel) Stop() {
	s.cancel()
	log.Println("[shadowsocks] Tunnel stopped")
}

// startShadowsocks connects to a Shadowsocks server and routes TUN traffic via tun2socks.
//
// Architecture:
//
//	Android TUN fd
//	    ↓  tun2socks (reads raw IP pkts → SOCKS5)
//	127.0.0.1:1080  (SOCKS5 listener, local)
//	    ↓  go-shadowsocks2 client
//	Shadowsocks server (encrypted)
func startShadowsocks(tunFd int, cfg ConnectResponse, endpoint string) (tunnel, error) {
	log.Printf("[shadowsocks] Connecting to %s", endpoint)

	// Parse endpoint
	host, portStr, err := net.SplitHostPort(endpoint)
	if err != nil {
		return nil, fmt.Errorf("shadowsocks endpoint: %w", err)
	}
	_ = host
	_ = portStr

	// Method defaults
	method := "AEAD_CHACHA20_POLY1305"
	password := cfg.Password
	if password == "" {
		return nil, fmt.Errorf("shadowsocks: password required")
	}

	// Create cipher
	ciph, err := core.PickCipher(method, nil, password)
	if err != nil {
		return nil, fmt.Errorf("shadowsocks cipher: %w", err)
	}

	ctx, cancel := context.WithCancel(context.Background())

	// Local SOCKS5 listener (tun2socks will forward here)
	localAddr := "127.0.0.1:1080"
	ln, err := net.Listen("tcp", localAddr)
	if err != nil {
		cancel()
		return nil, fmt.Errorf("shadowsocks local listen: %w", err)
	}

	go func() {
		defer ln.Close()
		for {
			select {
			case <-ctx.Done():
				return
			default:
			}
			conn, err := ln.Accept()
			if err != nil {
				if ctx.Err() != nil {
					return
				}
				log.Printf("[shadowsocks] accept error: %v", err)
				continue
			}
			go handleSocksConn(ctx, conn, endpoint, ciph)
		}
	}()

	// Start tun2socks to route tun fd traffic to the local SOCKS5 proxy
	log.Printf("[shadowsocks] Local SOCKS5 proxy at %s → %s", localAddr, endpoint)
	go startTun2Socks(ctx, tunFd, localAddr, cfg)

	return &ssTunnel{cancel: cancel}, nil
}

// handleSocksConn proxies a SOCKS5 connection through the Shadowsocks server.
func handleSocksConn(ctx context.Context, conn net.Conn, serverAddr string, ciph core.Cipher) {
	defer conn.Close()

	// Read the SOCKS5 target address
	tgt, err := socks.Handshake(conn)
	if err != nil {
		return
	}

	// Connect to Shadowsocks server
	rc, err := net.Dial("tcp", serverAddr)
	if err != nil {
		log.Printf("[shadowsocks] dial error: %v", err)
		return
	}
	defer rc.Close()
	rc.(*net.TCPConn).SetKeepAlive(true)
	rc.(*net.TCPConn).SetKeepAlivePeriod(30 * time.Second)

	// Wrap with Shadowsocks cipher
	rc = ciph.StreamConn(rc)

	// Send target address to SS server
	if _, err = rc.Write([]byte(tgt)); err != nil {
		return
	}

	// Bidirectional relay
	relay(conn, rc)
}

// relay copies data between two connections bidirectionally.
func relay(left, right net.Conn) {
	done := make(chan struct{}, 2)
	go func() {
		copyConn(right, left)
		done <- struct{}{}
	}()
	go func() {
		copyConn(left, right)
		done <- struct{}{}
	}()
	<-done
}

func copyConn(dst, src net.Conn) {
	buf := make([]byte, 32*1024)
	for {
		n, err := src.Read(buf)
		if n > 0 {
			if _, werr := dst.Write(buf[:n]); werr != nil {
				return
			}
		}
		if err != nil {
			return
		}
	}
}

// startTun2Socks routes raw IP packets from the TUN fd to the SOCKS5 proxy.
// Uses the lightweight pure-Go tun2socks approach.
func startTun2Socks(ctx context.Context, tunFd int, socksAddr string, cfg ConnectResponse) {
	tunFile := os.NewFile(uintptr(tunFd), "tun")
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
		// Raw IP packet in buf[:n] — forward via SOCKS5
		go forwardIPPacket(ctx, buf[:n], socksAddr)
	}
}

// forwardIPPacket parses a raw IPv4 packet and forwards the TCP/UDP session via SOCKS5.
// For a production implementation, use gvisor/netstack or lwip for full IP stack.
func forwardIPPacket(ctx context.Context, pkt []byte, socksAddr string) {
	// Minimal stub: a full tun2socks requires a userspace IP stack (gvisor or lwip).
	// This is where we'd parse IP headers, track flows, and forward via the SOCKS5 dial.
	// TODO: integrate github.com/xjasonlyu/tun2socks for full IP stack support.
	_ = pkt
	_ = socksAddr
}
