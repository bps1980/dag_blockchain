use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeFiPool {
    pub name: String,
    pub liquidity_providers: HashMap<String, u64>, // User -> Liquidity amount
    pub reward_rate: f64,                         // Reward rate for liquidity providers
}

impl DeFiPool {
    /// Create a new DeFiPool
    pub fn new(name: String, reward_rate: f64) -> Self {
        DeFiPool {
            name,
            liquidity_providers: HashMap::new(),
            reward_rate,
        }
    }

    pub fn list_liquidity(&self) -> Vec<(String, u64)> {
        self.liquidity_providers.iter()
            .map(|(user, amount)| (user.clone(), *amount))
            .collect()
    }

    /// Add liquidity to the pool
    pub fn add_liquidity(&mut self, user: String, amount: u64) -> Result<(), String> {
        if amount == 0 {
            return Err("Liquidity amount must be greater than 0.".to_string());
        }

        *self.liquidity_providers.entry(user).or_insert(0) += amount;
        Ok(())
    }

    /// Remove liquidity from the pool
    pub fn remove_liquidity(&mut self, user: String, amount: u64) -> Result<(), String> {
        let liquidity = self
            .liquidity_providers
            .get_mut(&user)
            .ok_or(format!("No liquidity found for user '{}'.", user))?;

        if amount == 0 {
            return Err("Liquidity removal amount must be greater than 0.".to_string());
        }

        if *liquidity < amount {
            return Err(format!(
                "Insufficient liquidity: User '{}' has only {} available.",
                user, liquidity
            ));
        }

        *liquidity -= amount;
        if *liquidity == 0 {
            self.liquidity_providers.remove(&user);
        }
        Ok(())
    }

    /// Calculate rewards for a specific user
    pub fn calculate_rewards(&self, user: &str) -> Result<u64, String> {
        // Retrieve user's liquidity
        let liquidity = self
            .liquidity_providers
            .get(user)
            .ok_or(format!("No liquidity found for user '{}'.", user))?;

        // Calculate rewards
        let rewards = (*liquidity as f64 * self.reward_rate) as u64;
        Ok(rewards)
    }
}
