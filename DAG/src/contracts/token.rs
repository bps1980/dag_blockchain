use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub id: String,                     // Unique identifier for the token
    pub name: String,                   // Name of the token
    pub symbol: String,                 // Symbol of the token
    pub total_supply: u64,              // Total supply of the token
    pub balances: HashMap<String, u64>, // Balances of users
    pub decimals: u8,                   // Number of decimals for the token
}

impl Token {
    pub fn new(name: String, symbol: String, total_supply: u64, decimals: u8) -> Self {
        Token {
            id: Uuid::new_v4().to_string(), // Automatically generates a unique UUID
            name,
            symbol,
            total_supply,
            balances: HashMap::new(),
            decimals,
        }
    }

    pub fn transfer(&mut self, from: String, to: String, amount: u64) -> Result<(), String> {
        let sender_balance = self.balances.get_mut(&from).ok_or("Sender not found")?;
        if *sender_balance < amount {
            return Err("Insufficient balance".to_string());
        }
        *sender_balance -= amount;
        *self.balances.entry(to).or_insert(0) += amount;
        Ok(())
    }

    pub fn mint(&mut self, to: String, amount: u64) -> Result<(), String> {
        self.total_supply += amount;
        *self.balances.entry(to).or_insert(0) += amount;
        Ok(())
    }

    pub fn burn(&mut self, from: String, amount: u64) -> Result<(), String> {
        let balance = self.balances.get_mut(&from).ok_or("Address not found")?;
        if *balance < amount {
            return Err("Insufficient balance".to_string());
        }
        *balance -= amount;
        self.total_supply -= amount;
        Ok(())
    }
}