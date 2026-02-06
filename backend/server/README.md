# WorldVPN Server

API Server and Infrastructure for the WorldVPN network.

## Architecture

- **Framework**: Axum (Web) + Tokio (Async Runtime)
- **Database**: SQLx (PostgreSQL)
- **Logging**: Tracing

## Running

```bash
# Start server locally (port 3000)
cargo run -p vpn-server
```

## Endpoints

### Health Check

`GET /health`

```json
{
  "status": "ok",
  "service": "worldvpn-server",
  "version": "0.1.0"
}
```

### VPN Connection (Simulation)

`POST /vpn/connect`

**Payload:**

```json
{
  "protocol": "WireGuard",
  "username": "user",
  "public_key": "optional_pub_key"
}
```

**Response:**

```json
{
  "session_id": "uuid...",
  "assigned_ip": "10.0.0.x",
  "server_endpoint": "127.0.0.1:51820"
}
```
