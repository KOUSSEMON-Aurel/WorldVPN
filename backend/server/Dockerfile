# Build stage
FROM rust:1.75-bookworm as builder

WORKDIR /usr/src/worldvpn

# Copy the entire workspace to handle path dependencies
COPY . .

# Build the backend server in release mode
RUN cargo build --release -p vpn-server

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/local/bin

# Copy the binary from the builder stage
COPY --from=builder /usr/src/worldvpn/target/release/vpn-server .

# Expose the API port
EXPOSE 3000

# Set environment variables (can be overridden by Render)
ENV DATABASE_URL=""
ENV JWT_SECRET=""
ENV PORT=3000
ENV USE_TLS=false

CMD ["./vpn-server"]
