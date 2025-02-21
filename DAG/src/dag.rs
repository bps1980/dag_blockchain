use std::collections::{HashMap, HashSet};
use ring::signature::{UnparsedPublicKey, Ed25519KeyPair, ED25519};
use serde::{Serialize, Deserialize};
use crate::crypto_utils::generate_key_pair;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub timestamp: u64,
    pub sender: String,
    pub receiver: String,
    pub amount: f64,
    pub signature: Vec<u8>,
    pub parents: Vec<String>,
    pub public_key: Vec<u8>,
    pub status: String, // "valid" or "revoked"
    pub priority: u8,   // Transaction priority (0 = low, 255 = high)
    pub contract: Option<String>, // Smart contract logic (optional)
}

#[derive(Debug)]
pub struct DAG {
    pub transactions: HashMap<String, Transaction>,
    pub layers: Vec<HashSet<String>>,
}

impl DAG {
    pub fn new() -> Self {
        DAG {
            transactions: HashMap::new(),
            layers: Vec::new(),
        }
    }

    pub fn attach_contract(&mut self, tx_id: &str, contract_logic: String) {
        if let Some(tx) = self.transactions.get_mut(tx_id) {
            tx.contract = Some(contract_logic);
            println!("Contract logic attached to transaction {}", tx_id);
        } else {
            println!("Transaction {} not found. Unable to attach contract.", tx_id);
        }
    }

    pub fn execute_contract(&self, tx: &Transaction) -> Result<String, String> {
        if let Some(contract_logic) = &tx.contract {
            match contract_logic.as_str() {
                "transfer" => {
                    if tx.amount > 0.0 {
                        Ok(format!(
                            "Transfer executed: {} -> {} (amount: {})",
                            tx.sender, tx.receiver, tx.amount
                        ))
                    } else {
                        Err("Invalid transfer amount".to_string())
                    }
                }
                _ => Err("Unsupported contract logic".to_string()),
            }
        } else {
            Ok("No contract logic found".to_string())
        }
    }

    pub fn validate_transaction(
        &self,
        tx: &Transaction,
        validation_cache: &mut HashMap<String, bool>,
    ) -> bool {
        if let Some(&cached_result) = validation_cache.get(&tx.id) {
            return cached_result;
        }

        for parent_id in &tx.parents {
            if let Some(parent) = self.transactions.get(parent_id) {
                if parent.status != "valid" || !self.validate_transaction(parent, validation_cache) {
                    validation_cache.insert(tx.id.clone(), false);
                    return false;
                }
            } else {
                validation_cache.insert(tx.id.clone(), false);
                return false;
            }
        }

        let public_key = UnparsedPublicKey::new(&ED25519, &tx.public_key);
        let is_valid = public_key.verify(tx.id.as_bytes(), &tx.signature).is_ok();
        validation_cache.insert(tx.id.clone(), is_valid);
        is_valid
    }

    pub fn add_transaction(&mut self, tx: Transaction) -> Result<(), String> {
        if !self.validate_transaction(&tx, &mut HashMap::new()) {
            return Err("Transaction validation failed.".to_string());
        }

        self.transactions.insert(tx.id.clone(), tx.clone());

        let layer = if tx.parents.is_empty() {
            0
        } else {
            tx.parents
                .iter()
                .map(|parent| self.get_layer(parent).unwrap_or(0) + 1)
                .max()
                .unwrap_or(0)
        };

        while self.layers.len() <= layer {
            self.layers.push(HashSet::new());
        }

        self.layers[layer].insert(tx.id.clone());
        self.optimize_layers();
        Ok(())
    }

    pub fn get_layer(&self, tx_id: &str) -> Option<usize> {
        self.layers.iter().position(|layer| layer.contains(tx_id))
    }

    pub fn revoke_transaction(&mut self, tx_id: &str) -> Result<(), String> {
        if let Some(tx) = self.transactions.get_mut(tx_id) {
            tx.status = "revoked".to_string();
            println!("Transaction {} has been revoked.", tx_id);
            Ok(())
        } else {
            Err("Transaction not found.".to_string())
        }
    }

    pub fn process_adaptive_bundles(&self) {
        let bundle_size = if self.transactions.len() < 10 {
            1
        } else {
            self.transactions.len() / 5
        };

        let mut transactions: Vec<_> = self.transactions.values().collect();
        transactions.sort_by_key(|tx| tx.timestamp);

        for chunk in transactions.chunks(bundle_size) {
            println!("Processing bundle with {} transactions:", chunk.len());
            for tx in chunk {
                println!("- Transaction ID: {}, Amount: {}", tx.id, tx.amount);
            }
        }
    }

    pub fn optimize_layers(&mut self) {
        self.layers.retain(|layer| !layer.is_empty());
    }
}

pub struct Consensus {
    pub leader_id: String,
    pub validators: Vec<String>,
    key_pair: Ed25519KeyPair,
}

impl Consensus {
    pub fn new(leader_id: String, validators: Vec<String>) -> Self {
        let key_pair = generate_key_pair();
        Consensus {
            leader_id,
            validators,
            key_pair,
        }
    }

    pub fn get_key_pair(&self) -> &Ed25519KeyPair {
        &self.key_pair
    }

    pub fn propose_transaction(&self, dag: &mut DAG, tx: &Transaction) -> Result<(), String> {
        let mut votes = 0;

        self.validators.iter().for_each(|validator| {
            println!("Validator {} is validating the transaction...", validator);
            if dag.validate_transaction(tx, &mut HashMap::new()) {
                votes += 1;
            }
        });

        if votes > self.validators.len() / 2 {
            println!(
                "Leader {} successfully proposed transaction: {}",
                self.leader_id, tx.id
            );
            dag.add_transaction(tx.clone())
        } else {
            Err("Consensus failed: Transaction rejected.".to_string())
        }
    }
}
