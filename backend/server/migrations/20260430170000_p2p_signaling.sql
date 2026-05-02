-- Migration: Add real endpoint for P2P signaling
ALTER TABLE nodes ADD COLUMN public_endpoint TEXT;
