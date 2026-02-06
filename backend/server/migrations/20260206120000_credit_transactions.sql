-- Credit transaction history

CREATE TABLE IF NOT EXISTS credit_transactions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    amount BIGINT NOT NULL, -- Positive (earned) or Negative (spent)
    transaction_type TEXT NOT NULL, -- 'EARNED', 'SPENT', 'BONUS', 'PENALTY'
    description TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Index for fast user lookup
CREATE INDEX idx_credit_logs_user ON credit_transactions(user_id);
