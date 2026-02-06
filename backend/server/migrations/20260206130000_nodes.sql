-- Peer nodes registry for P2P VPN network
CREATE TABLE IF NOT EXISTS nodes (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    
    -- Network info (encrypted or hashed for privacy)
    public_ip_hash TEXT NOT NULL,
    country_code CHAR(2) NOT NULL,
    city TEXT,
    
    -- Capabilities
    available_bandwidth_mbps INTEGER DEFAULT 50,
    max_connections INTEGER DEFAULT 10,
    current_connections INTEGER DEFAULT 0,
    
    -- Protocols supported (JSON array)
    protocols TEXT NOT NULL DEFAULT '["WireGuard"]',
    
    -- Quality metrics
    uptime_percentage REAL DEFAULT 100.0,
    avg_latency_ms INTEGER DEFAULT 50,
    reputation_score INTEGER DEFAULT 100,
    
    -- Status
    is_online BOOLEAN DEFAULT FALSE,
    last_heartbeat TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    -- Preferences (what traffic the node owner allows)
    allow_streaming BOOLEAN DEFAULT TRUE,
    allow_torrents BOOLEAN DEFAULT FALSE,
    allow_countries TEXT DEFAULT '["*"]',
    block_countries TEXT DEFAULT '[]',
    max_daily_gb INTEGER DEFAULT 50,
    
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Active P2P sessions (for transparency dashboard)
CREATE TABLE IF NOT EXISTS peer_sessions (
    id TEXT PRIMARY KEY,
    
    -- Who is providing bandwidth
    node_id TEXT NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    node_owner_id TEXT NOT NULL REFERENCES users(id),
    
    -- Who is consuming bandwidth (anonymized for privacy)
    client_country CHAR(2) NOT NULL,
    client_id_hash TEXT NOT NULL,
    
    -- Traffic info (for transparency)
    traffic_type TEXT DEFAULT 'browsing',
    bytes_transferred BIGINT DEFAULT 0,
    
    -- Timing
    started_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    ended_at TIMESTAMP,
    is_active BOOLEAN DEFAULT TRUE,
    
    -- For credit calculation
    credits_earned INTEGER DEFAULT 0
);

-- Index for fast lookups
CREATE INDEX IF NOT EXISTS idx_nodes_online ON nodes(is_online, country_code);
CREATE INDEX IF NOT EXISTS idx_nodes_user ON nodes(user_id);
CREATE INDEX IF NOT EXISTS idx_peer_sessions_active ON peer_sessions(is_active, node_id);
CREATE INDEX IF NOT EXISTS idx_peer_sessions_node_owner ON peer_sessions(node_owner_id);
