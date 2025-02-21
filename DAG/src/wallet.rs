use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallet {
    pub address: String,
    pub private_key: String,
    pub balances: HashMap<String, u64>, // Token balances
}

impl Wallet {
    // Create a new wallet with a provided private key
    pub fn new(private_key: String) -> Self {
        let address = Uuid::new_v4().to_string(); // Use UUID as a dummy address generator
        Wallet {
            address,
            private_key,
            balances: HashMap::new(), // Initialize with empty balances
        }
    }

    // Update token balances
    pub fn update_balances(&mut self, token: &str, amount: i64) {
        let balances = self.balances.entry(token.to_string()).or_insert(0);
        if amount.is_positive() || *balances as i64 + amount >= 0 {
            *balances = (*balances as i64 + amount) as u64;
        }
    }
}
