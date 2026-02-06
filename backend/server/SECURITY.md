# Backend Security Guide

This document outlines the security practices and configuration required for WorldVPN backend.

## 🚨 Critical Rules

1. **NEVER commit `.env` files** or any file containing real secrets.
2. **NEVER commit `.pem`, `.key`, or `.crt` files**. These are automatically ignored by git.
3. Ensure `JWT_SECRET` is a strong, random 64-byte string in production.
4. Use HTTPS in production (Let's Encrypt recommended).

---

## 1. Development Environment Setup

For local development, use the provided script to generate self-signed certificates and a development secret:

```bash
./scripts/generate-dev-certs.sh
```

This will create:

- `key.pem` & `cert.pem` (RSA)
- `ec-key.pem` & `ec-cert.pem` (Elliptic Curve)
- `.env` file with a random `JWT_SECRET`

### Database Configuration

Update your `.env` with your local PostgreSQL credentials:

```env
DATABASE_URL=postgresql://user:password@localhost:5432/worldvpn
```

---

## 2. Production Environment

### Secrets Management

**DO NOT** use the `generate-dev-certs.sh` script for production. Use a proper secrets manager (Vault, Secret Manager) or set environment variables directly.

Required variables:

- `DATABASE_URL`: Connection string to production DB.
- `JWT_SECRET`: 64-byte random string (e.g., generated via `openssl rand -base64 64`).
- `HOST`: `0.0.0.0`
- `PORT`: Usually `443` for HTTPS.

### TLS Certificates

Use valid certificates from a trusted CA (Certificate Authority).

**Using Let's Encrypt (Certbot):**

```bash
certbot certonly --standalone -d api.worldvpn.net
```

Then update your `.env`:

```env
TLS_CERT_PATH=/etc/letsencrypt/live/api.worldvpn.net/fullchain.pem
TLS_KEY_PATH=/etc/letsencrypt/live/api.worldvpn.net/privkey.pem
```

---

## 3. Data Protection

- **Passwords**: Hashed using **Argon2id**.
- **Tokens**: JWT with HS256 (or RS256 for large deployments).
- **Communication**: TLS 1.2+ required.

---

## 4. Troubleshooting

### "Permission Denied" on `.pem` files

Ensure files have `600` permissions:

```bash
chmod 600 *.pem
```

### Server fails to start (TLS error)

Check if the paths in `.env` match the actual file locations.

---

## 5. Security Reporting

If you find a security vulnerability, please do NOT open a public issue. Email us at `security@worldvpn.net` instead.

---

*Last updated: 2026-02-06*
