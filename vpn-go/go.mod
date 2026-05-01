module github.com/worldvpn/vpn-go

go 1.22

require (
	github.com/apernet/hysteria/core/v2 v2.0.0-00010101000000-000000000000
	github.com/apernet/hysteria/extras/v2 v2.0.0-00010101000000-000000000000
	github.com/shadowsocks/go-shadowsocks2 v0.1.5
	github.com/xtls/xray-core v1.8.10
	golang.zx2c4.com/wireguard v0.0.0-20231211153847-12269c276173
	golang.org/x/net v0.24.0
)

replace (
	github.com/apernet/hysteria/core/v2 => github.com/apernet/hysteria/core/v2 v2.4.5
	github.com/apernet/hysteria/extras/v2 => github.com/apernet/hysteria/extras/v2 v2.4.5
)
