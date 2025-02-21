use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LendingPool {
    pub pool_name: String,
    pub deposits: HashMap<String, u64>, // User -> Deposited amount
    pub loans: HashMap<String, u64>,    // User -> Borrowed amount
    pub interest_rate: f64,            // Annual interest rate
    pub flash_loan_fee: f64,           // Flash loan fee percentage
}

impl LendingPool {
    /// Creates a new lending pool
    pub fn new(pool_name: String, interest_rate: f64, flash_loan_fee: f64) -> Self {
        LendingPool {
            pool_name,
            deposits: HashMap::new(),
            loans: HashMap::new(),
            interest_rate,
            flash_loan_fee,
        }
    }    

    /// Deposit funds into the pool
    pub fn deposit(&mut self, user: String, amount: u64) {
        *self.deposits.entry(user).or_insert(0) += amount;
    }

    /// Borrow funds from the pool
    pub fn borrow(&mut self, user: String, amount: u64) -> Result<(), String> {
        let total_available = self.total_available_liquidity();
        if amount > total_available {
            return Err("Insufficient liquidity in pool".to_string());
        }

        *self.loans.entry(user).or_insert(0) += amount;
        Ok(())
    }

    /// Initiate a flash loan
    pub fn flash_loan(&self, amount: u64) -> Result<u64, String> {
        let total_available = self.total_available_liquidity();
        if amount > total_available {
            return Err("Insufficient liquidity for flash loan".to_string());
        }

        let fee = ((amount as f64) * self.flash_loan_fee / 100.0).round() as u64;
        Ok(amount - fee)
    }

    /// Calculate total available liquidity
    pub fn total_available_liquidity(&self) -> u64 {
        self.deposits.values().sum::<u64>() - self.loans.values().sum::<u64>()
    }
}
