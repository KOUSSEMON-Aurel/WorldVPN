-- Historique des transactions de crédits

CREATE TABLE IF NOT EXISTS credit_transactions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    amount BIGINT NOT NULL, -- Positif (gain) ou Négatif (dépense)
    transaction_type TEXT NOT NULL, -- 'EARNED', 'SPENT', 'BONUS', 'PENALTY'
    description TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Index pour recherche rapide par user
CREATE INDEX idx_credit_logs_user ON credit_transactions(user_id);
