-- Storage for external provider metadata like VPNBook dynamic passwords
CREATE TABLE IF NOT EXISTS public_provider_metadata (
    id SERIAL PRIMARY KEY,
    provider_name TEXT NOT NULL, -- e.g. 'VPNBOOK'
    key TEXT NOT NULL,           -- e.g. 'password'
    value TEXT NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(provider_name, key)
);

-- Insert initial record for VPNBOOK password
INSERT INTO public_provider_metadata (provider_name, key, value) 
VALUES ('VPNBOOK', 'password', 'vpnbook')
ON CONFLICT DO NOTHING;
