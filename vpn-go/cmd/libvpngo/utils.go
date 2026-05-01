package main

import (
	"crypto/rand"
	"fmt"
	"net"

	"google.golang.org/protobuf/proto"
	"google.golang.org/protobuf/types/known/anypb"
)

// randRead is an alias for crypto/rand.Read to keep wireguard.go clean.
var randRead = rand.Read

// splitHostPort wraps net.SplitHostPort with a nicer error.
func splitHostPort(addr string) (host, port string, err error) {
	host, port, err = net.SplitHostPort(addr)
	if err != nil {
		err = fmt.Errorf("invalid address %q: %w", addr, err)
	}
	return
}

// mustMarshal wraps proto.Marshal and panics on error (only for compile-time-valid protos).
func mustMarshal(m proto.Message) *anypb.Any {
	a, err := anypb.New(m)
	if err != nil {
		panic(fmt.Sprintf("mustMarshal: %v", err))
	}
	return a
}
