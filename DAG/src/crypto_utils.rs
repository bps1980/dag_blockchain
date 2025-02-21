use ring::rand::{SystemRandom, SecureRandom};
use ring::signature::{Ed25519KeyPair, KeyPair};
use chrono::Utc;
use uuid::Uuid;
use crate::dag::Transaction;

/// Generate an Ed25519 key pair
///
/// This function generates a new Ed25519 key pair for signing transactions.
/// It uses a secure random seed to ensure cryptographic safety.
///
/// # Returns
/// - `Ed25519KeyPair`: A newly generated Ed25519 key pair.
pub fn generate_key_pair() -> Ed25519KeyPair {
    let rng = SystemRandom::new();
    let mut seed = [0u8; 32];
    rng.fill(&mut seed).expect("Failed to generate seed for key pair");
    Ed25519KeyPair::from_seed_unchecked(&seed).expect("Failed to generate key pair")
}

/// Create a signed transaction
///
/// This function creates a transaction, signs it using the provided key pair,
/// and returns the signed transaction object.
///
/// # Parameters
/// - `key_pair`: The `Ed25519KeyPair` used to sign the transaction.
/// - `receiver`: The recipient of the transaction.
/// - `amount`: The amount of the transaction.
/// - `parents`: The parent transaction IDs in the DAG.
/// - `priority`: The priority of the transaction (0 = low, 255 = high).
///
/// # Returns
/// - A `Transaction` object with the signature applied.
pub fn create_signed_transaction(
    key_pair: &Ed25519KeyPair,
    receiver: String,
    amount: f64,
    parents: Vec<String>,
    priority: u8,
) -> Transaction {
    // Generate a unique transaction ID
    let id = format!("tx-{}", Uuid::new_v4());
    let timestamp = Utc::now().timestamp() as u64;

    // Construct the transaction
    let mut tx = Transaction {
        id: id.clone(),
        timestamp,
        sender: hex::encode(key_pair.public_key()), // Encode sender's public key as a hex string
        receiver,
        amount,
        signature: vec![],
        parents,
        public_key: key_pair.public_key().as_ref().to_vec(),
        status: "valid".to_string(),
        priority,
        contract: None, // Optional: Smart contract logic placeholder
    };

    // Sign the transaction ID using the private key
    let signature = key_pair.sign(tx.id.as_bytes());
    tx.signature = signature.as_ref().to_vec();

    tx
}

/// Verify the signature of a transaction
///
/// This function verifies the cryptographic signature of a transaction using its public key.
///
/// # Parameters
/// - `tx`: The `Transaction` object to verify.
///
/// # Returns
/// - `bool`: `true` if the signature is valid, `false` otherwise.
pub fn verify_transaction_signature(tx: &Transaction) -> bool {
    let public_key = ring::signature::UnparsedPublicKey::new(&ring::signature::ED25519, &tx.public_key);
    public_key.verify(tx.id.as_bytes(), &tx.signature).is_ok()
}

/// Generate a unique transaction ID
///
/// This helper function generates a new UUID-based transaction ID.
///
/// # Returns
/// - `String`: A unique transaction ID.
pub fn generate_transaction_id() -> String {
    format!("tx-{}", Uuid::new_v4())
}
