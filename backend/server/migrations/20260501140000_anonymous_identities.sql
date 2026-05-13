-- migrate:up
-- Migration to support anonymous identities via Ed25519 public keys
ALTER TABLE users ADD COLUMN IF NOT EXISTS ed25519_pubkey TEXT UNIQUE;
ALTER TABLE users ALTER COLUMN password_hash DROP NOT NULL;
ALTER TABLE users ALTER COLUMN username DROP NOT NULL;

-- Ensure we can lookup by public key quickly
CREATE INDEX IF NOT EXISTS idx_users_pubkey ON users(ed25519_pubkey);

-- Add a column to track the last activity for TTL pruning
ALTER TABLE users ADD COLUMN IF NOT EXISTS last_active TIMESTAMP DEFAULT CURRENT_TIMESTAMP;

-- migrate:down
