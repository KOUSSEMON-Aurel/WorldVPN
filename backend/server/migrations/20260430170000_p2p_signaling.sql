-- migrate:up
-- Migration: Add real endpoint for P2P signaling
ALTER TABLE nodes ADD COLUMN IF NOT EXISTS public_endpoint TEXT;

-- migrate:down
