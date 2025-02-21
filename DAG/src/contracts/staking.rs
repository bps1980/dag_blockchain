use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingContract {
    pub interest_rate: f64, // Annual interest rate
    pub stakes: HashMap<String, u64>, // User -> Amount staked
    pub rewards: HashMap<String, f64>, // User -> Rewards accumulated
}

impl StakingContract {
    /// Creates a new staking contract
    pub fn new(interest_rate: f64) -> Self {
        StakingContract {
            interest_rate,
            stakes: HashMap::new(),
            rewards: HashMap::new(),
        }
    }

    /// Stakes tokens for a user
    pub fn stake(&mut self, user: String, amount: u64) -> Result<(), String> {
        if amount == 0 {
            return Err("Stake amount must be greater than 0".to_string());
        }

        *self.stakes.entry(user.clone()).or_insert(0) += amount;
        Ok(())
    }

    /// Unstakes tokens for a user
    pub fn unstake(&mut self, user: String, amount: u64) -> Result<(), String> {
        let staked_amount = self.stakes.get_mut(&user).ok_or("No stake found for user")?;
        if *staked_amount < amount {
            return Err("Unstake amount exceeds staked amount".to_string());
        }

        *staked_amount -= amount;
        if *staked_amount == 0 {
            self.stakes.remove(&user);
        }
        Ok(())
    }

    /// Calculate rewards for a user
    pub fn calculate_rewards(&mut self, user: &String) -> Result<f64, String> {
        let staked_amount = self.stakes.get(user).ok_or("User not found in stakes")?;
        let rewards = (*staked_amount as f64) * (self.interest_rate / 100.0);
        *self.rewards.entry(user.clone()).or_insert(0.0) += rewards;
        Ok(rewards)
    }
}
