#!/bin/bash
# TLS Self-signed Certificate Generation Script for Development
# WARNING: For production, use Let's Encrypt or a real CA!

set -e

CERT_DIR="backend/server"
DAYS_VALID=365

echo "🔐 Generating development TLS certificates..."

# Create directory if necessary
mkdir -p "$CERT_DIR"

# ===== RSA Certificate (maximum compatibility) =====
echo "📜 Generating RSA private key..."
openssl genrsa -out "$CERT_DIR/key.pem" 4096

echo "📜 Generating RSA self-signed certificate..."
openssl req -new -x509 -sha256 \
    -key "$CERT_DIR/key.pem" \
    -out "$CERT_DIR/cert.pem" \
    -days $DAYS_VALID \
    -subj "/C=CH/ST=Vaud/L=Lausanne/O=WorldVPN Dev/CN=localhost"

# ===== EC Certificate (Elliptic Curve - modern and fast) =====
echo "📜 Generating EC private key (P-256)..."
openssl ecparam -genkey -name prime256v1 -out "$CERT_DIR/ec-key.pem"

echo "📜 Generating EC self-signed certificate..."
openssl req -new -x509 -sha256 \
    -key "$CERT_DIR/ec-key.pem" \
    -out "$CERT_DIR/ec-cert.pem" \
    -days $DAYS_VALID \
    -subj "/C=CH/ST=Vaud/L=Lausanne/O=WorldVPN Dev/CN=localhost"

# ===== JWT_SECRET Generation =====
echo "🔑 Generating JWT_SECRET..."
JWT_SECRET=$(openssl rand -base64 64 | tr -d '\n')

# Create .env if it doesn't exist
if [ ! -f .env ]; then
    echo "📝 Creating .env from .env.example..."
    cp .env.example .env
    
    # Replace JWT_SECRET
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS
        sed -i '' "s|JWT_SECRET=.*|JWT_SECRET=$JWT_SECRET|" .env
    else
        # Linux
        sed -i "s|JWT_SECRET=.*|JWT_SECRET=$JWT_SECRET|" .env
    fi
    
    echo "✅ .env created with generated JWT_SECRET"
else
    echo "⚠️  .env already exists, JWT_SECRET not modified"
    echo "   New secret (if needed): $JWT_SECRET"
fi

# ===== Security Permissions =====
chmod 600 "$CERT_DIR"/*.pem
chmod 600 .env 2>/dev/null || true

echo ""
echo "✅ TLS certificates generated in $CERT_DIR/"
echo "   - RSA: cert.pem + key.pem (compatibility)"
echo "   - EC:  ec-cert.pem + ec-key.pem (performance)"
echo ""
echo "⚠️  THESE CERTIFICATES ARE SELF-SIGNED - Development only!"
echo "   In production, use Let's Encrypt (certbot) or a real CA."
echo ""
echo "🔒 Secure permissions applied (600)"
echo ""
echo "🚀 You can now start the server with:"
echo "   cargo run -p vpn-server"
