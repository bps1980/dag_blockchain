use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NFT {
    pub id: String,
    pub owner: String,
    pub metadata: HashMap<String, String>, // Attributes of the NFT
    pub royalties: Option<f64>, // Royalty percentage (e.g., 5.0 for 5%)
}

impl NFT {
    /// Create a new NFT
    pub fn new(owner: String, metadata: HashMap<String, String>, royalties: Option<f64>) -> Self {
        NFT {
            id: uuid::Uuid::new_v4().to_string(),
            owner,
            metadata,
            royalties,
        }
    }

    /// Transfer the NFT to a new owner
    pub fn transfer(&mut self, new_owner: String) -> Result<(), String> {
        if self.owner == new_owner {
            Err("Cannot transfer NFT to the same owner".to_string())
        } else {
            self.owner = new_owner;
            Ok(())
        }
    }

    /// Update metadata of the NFT
    pub fn update_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Calculate royalty amount
    pub fn calculate_royalty(&self, sale_price: f64) -> Option<f64> {
        self.royalties.map(|royalty| sale_price * (royalty / 100.0))
    }
}

/// Mint a new NFT
pub fn mint_nft(owner: String, metadata: HashMap<String, String>, royalties: Option<f64>) -> NFT {
    NFT::new(owner, metadata, royalties)
}
