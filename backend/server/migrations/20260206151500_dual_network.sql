-- Add node type distinction and support for public gateways
ALTER TABLE nodes ADD COLUMN node_group TEXT DEFAULT 'COMMUNITY';
ALTER TABLE nodes ADD COLUMN is_public BOOLEAN DEFAULT FALSE;
ALTER TABLE nodes ADD COLUMN public_config_data TEXT; -- Stores OVPN config or connection string
ALTER TABLE nodes ALTER COLUMN user_id DROP NOT NULL; -- Public nodes don't have a local owner

-- Update existing nodes to be community nodes
UPDATE nodes SET node_group = 'COMMUNITY', is_public = FALSE;

-- Index for filtering by group
CREATE INDEX IF NOT EXISTS idx_nodes_group ON nodes(node_group, is_online);

-- For tracking public server details from VPN Gate
CREATE TABLE IF NOT EXISTS public_provider_stats (
    id SERIAL PRIMARY KEY,
    provider_name TEXT NOT NULL, -- e.g. 'VPN_GATE'
    last_sync TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    total_nodes_found INTEGER,
    status TEXT
);
