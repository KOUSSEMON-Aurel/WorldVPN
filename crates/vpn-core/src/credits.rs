use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Credit unit representing traffic quota (1 credit typically = 1 MB)
pub type Credits = i64;

/// Represents a history entry for credit changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditTransaction {
    pub user_id: String,
    pub amount: Credits,
    pub transaction_type: TransactionType,
    pub timestamp: i64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionType {
    Earned,   // From sharing bandwidth
    Spent,    // From using the VPN
    Bonus,    // System rewards
    Penalty,  // Abuse punishment
}

/// Manages user balances and P2P economy incentives
pub struct CreditManager {
    balances: Arc<RwLock<HashMap<String, Credits>>>,
    transactions: Arc<RwLock<Vec<CreditTransaction>>>,
    config: CreditConfig,
}

#[derive(Debug, Clone)]
pub struct CreditConfig {
    pub initial_credits: Credits,
    pub bytes_per_credit: u64,
    pub share_multiplier: f64,
    pub minimum_credits_to_connect: Credits,
}

impl Default for CreditConfig {
    fn default() -> Self {
        Self {
            initial_credits: 1000, // 1 GB starter
            bytes_per_credit: 1_048_576, // 1 MB per credit
            share_multiplier: 1.2, // 20% bonus for uploading/sharing
            minimum_credits_to_connect: 10,
        }
    }
}

impl CreditManager {
    pub fn new(config: CreditConfig) -> Self {
        Self {
            balances: Arc::new(RwLock::new(HashMap::new())),
            transactions: Arc::new(RwLock::new(Vec::new())),
            config,
        }
    }

    /// Initializes a new user account with starting credits
    pub async fn create_account(&self, user_id: String) {
        let mut balances = self.balances.write().await;
        balances.insert(user_id.clone(), self.config.initial_credits);

        let mut transactions = self.transactions.write().await;
        transactions.push(CreditTransaction {
            user_id,
            amount: self.config.initial_credits,
            transaction_type: TransactionType::Bonus,
            timestamp: chrono::Utc::now().timestamp(),
            description: "Initial credits".to_string(),
        });
    }

    pub async fn get_balance(&self, user_id: &str) -> Credits {
        let balances = self.balances.read().await;
        balances.get(user_id).copied().unwrap_or(0)
    }

    /// Awards credits based on shared traffic volume
    pub async fn record_shared_traffic(&self, user_id: String, bytes: u64) -> Credits {
        let credits = self.bytes_to_credits(bytes);
        let earned = (credits as f64 * self.config.share_multiplier) as Credits;

        let mut balances = self.balances.write().await;
        *balances.entry(user_id.clone()).or_insert(0) += earned;

        let mut transactions = self.transactions.write().await;
        transactions.push(CreditTransaction {
            user_id,
            amount: earned,
            transaction_type: TransactionType::Earned,
            timestamp: chrono::Utc::now().timestamp(),
            description: format!("Shared {} MB", bytes / 1_048_576),
        });

        earned
    }

    /// Deducts credits based on consumed traffic volume
    pub async fn record_consumed_traffic(&self, user_id: String, bytes: u64) -> Result<Credits, String> {
        let credits = self.bytes_to_credits(bytes);

        let mut balances = self.balances.write().await;
        let current_balance = balances.get(&user_id).copied().unwrap_or(0);

        if current_balance < credits {
            return Err(format!(
                "Insufficient credits: {} required, {} available",
                credits, current_balance
            ));
        }

        *balances.entry(user_id.clone()).or_insert(0) -= credits;

        let mut transactions = self.transactions.write().await;
        transactions.push(CreditTransaction {
            user_id,
            amount: credits,
            transaction_type: TransactionType::Spent,
            timestamp: chrono::Utc::now().timestamp(),
            description: format!("Consumed {} MB", bytes / 1_048_576),
        });

        Ok(credits)
    }

    pub async fn can_connect(&self, user_id: &str) -> bool {
        let balance = self.get_balance(user_id).await;
        balance >= self.config.minimum_credits_to_connect
    }

    fn bytes_to_credits(&self, bytes: u64) -> Credits {
        (bytes / self.config.bytes_per_credit) as Credits
    }

    pub async fn get_user_transactions(&self, user_id: &str, limit: usize) -> Vec<CreditTransaction> {
        let transactions = self.transactions.read().await;
        transactions
            .iter()
            .filter(|t| t.user_id == user_id)
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Aggregates system-wide economy stats
    pub async fn get_stats(&self) -> CreditStats {
        let balances = self.balances.read().await;
        let transactions = self.transactions.read().await;

        let total_users = balances.len();
        let total_credits: Credits = balances.values().sum();
        let total_transactions = transactions.len();

        let earned: Credits = transactions
            .iter()
            .filter(|t| matches!(t.transaction_type, TransactionType::Earned))
            .map(|t| t.amount)
            .sum();

        let spent: Credits = transactions
            .iter()
            .filter(|t| matches!(t.transaction_type, TransactionType::Spent))
            .map(|t| t.amount)
            .sum();

        CreditStats {
            total_users,
            total_credits,
            total_transactions,
            total_earned: earned,
            total_spent: spent,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreditStats {
    pub total_users: usize,
    pub total_credits: Credits,
    pub total_transactions: usize,
    pub total_earned: Credits,
    pub total_spent: Credits,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_credit_lifecycle() {
        let manager = CreditManager::new(CreditConfig::default());

        manager.create_account("alice".to_string()).await;
        let balance = manager.get_balance("alice").await;
        assert_eq!(balance, 1000);

        let earned = manager.record_shared_traffic("alice".to_string(), 10_485_760).await;
        assert_eq!(earned, 12);

        let new_balance = manager.get_balance("alice").await;
        assert_eq!(new_balance, 1012);

        let spent = manager.record_consumed_traffic("alice".to_string(), 5_242_880).await;
        assert!(spent.is_ok());
        assert_eq!(spent.unwrap(), 5);

        let final_balance = manager.get_balance("alice").await;
        assert_eq!(final_balance, 1007);
    }

    #[tokio::test]
    async fn test_insufficient_credits() {
        let manager = CreditManager::new(CreditConfig {
            initial_credits: 5,
            ..Default::default()
        });

        manager.create_account("bob".to_string()).await;

        let result = manager.record_consumed_traffic("bob".to_string(), 10_485_760).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_can_connect() {
        let manager = CreditManager::new(CreditConfig {
            initial_credits: 15,
            minimum_credits_to_connect: 10,
            ..Default::default()
        });

        manager.create_account("charlie".to_string()).await;
        assert!(manager.can_connect("charlie").await);

        let _ = manager.record_consumed_traffic("charlie".to_string(), 6_291_456).await;
        assert!(!manager.can_connect("charlie").await);
    }
}
