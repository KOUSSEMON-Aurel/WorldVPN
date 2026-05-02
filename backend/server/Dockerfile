# Build stage
FROM rust:latest AS builder

WORKDIR /usr/src/worldvpn

# Install build dependencies early for better caching
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Copy the entire workspace to handle path dependencies
COPY . .

# Build the backend server in release mode
# Limit parallel jobs to 1 to prevent OOM on Render free tier (512MB RAM)
RUN CARGO_BUILD_JOBS=1 cargo build --release -p vpn-server

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
