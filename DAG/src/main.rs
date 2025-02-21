mod dag;
mod crypto_utils;
mod contracts;
mod wallet; // Import the wallet module

use actix_web::HttpRequest;
use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use serde::{Serialize, Deserialize};
use std::sync::{Arc, Mutex};
use clap::Parser;
use crate::dag::{Consensus, DAG};
use crate::crypto_utils::{create_signed_transaction};
use std::collections::HashMap;
use crate::contracts::{nft, subscription, staking, lending};
use crate::contracts::defi::DeFiPool;
use crate::contracts::token;
use crate::contracts::token::Token;
use serde_json::json;
use uuid::Uuid;
use chrono::Utc;
use sqlx::PgPool;
use crate::wallet::Wallet;
use rand::RngCore;
use rand::rngs::OsRng;
use hex;
use dotenv::dotenv;
use crate::dag::Transaction as DagTransaction;
use chrono::NaiveDateTime;
use time::OffsetDateTime;
use sqlx::Error;


#[derive(Parser, Debug)]
#[command(name = "DAG", about = "Blockchain-based Proof-of-Stake DAG system")]
struct Config {
    #[clap(long, default_value = "8080")]
    port: u16,

    #[clap(long, help = "Run as validator node")]
    validator: bool,

    #[clap(long, help = "Validator node address")]
    address: Option<String>,
}

// Shared State for the DAG
struct AppState {
    dag: Mutex<DAG>,
    consensus: Consensus,
    validator_load: Arc<Mutex<HashMap<String, u32>>>,
    subscription_data: Mutex<HashMap<String, subscription::Subscription>>,
    nft_data: Mutex<HashMap<String, nft::NFT>>,
    token_data: Mutex<HashMap<String, token::Token>>,
    staking_data: Mutex<HashMap<String, staking::StakingContract>>,
    lending_data: Mutex<HashMap<String, lending::LendingPool>>,
    liquidity_pools: Mutex<HashMap<String, LiquidityPool>>,
    wallets: Mutex<HashMap<String, wallet::Wallet>>, // Wallet store
    transfer_history: Mutex<Vec<TransferRequest>>,    // List of transfers
    swap_history: Mutex<Vec<SwapRequest>>,            // List of swaps
}

// Request Body for Adding a Transaction
#[derive(Deserialize)]
struct AddTransactionRequest {
    receiver: String,
    amount: f64,
    parents: Vec<String>,
    priority: Option<u8>,
    contract: Option<String>,
    metadata: Option<serde_json::Value>, // Optional metadata
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
struct Transaction {
    id: Uuid,
    sender: String,
    receiver: String,
    amount: f64,
    parents: Option<Vec<String>>,
    priority: req.priority.map(|p| p as i16),
    contract: Option<String>,
    timestamp: OffsetDateTime, // Use OffsetDateTime
    metadata: Option<serde_json::Value>,
}

async fn add_transaction2(
    data: web::Data<AppState>,
    req: web::Json<AddTransactionRequest>,
    pool: web::Data<PgPool>, // Database connection pool
    http_request: HttpRequest, // Use HttpRequest to retrieve the sender
) -> impl Responder {
    // Extract sender address (assumes it's passed in a custom header "X-Sender-Address")
    let sender_address = match http_request.headers().get("X-Sender-Address") {
        Some(header_value) => header_value.to_str().unwrap_or("").to_string(),
        None => return HttpResponse::BadRequest().body("Sender address not found in headers"),
    };

    let mut dag = data.dag.lock().unwrap();
    let tx_id = Uuid::new_v4().to_string();
    timestamp: OffsetDateTime::from_unix_timestamp(tx.timestamp.timestamp()).unwrap(),

    // Create the transaction
    let tx = Transaction {
        id: tx_id.clone(),
        sender: sender_address.clone(),
        receiver: req.receiver.clone(),
        amount: req.amount,
        parents: req.parents.clone(),
        priority: req.priority,
        contract: req.contract.clone(),
        timestamp,
        metadata: req.metadata.clone(),
    };

    // Add transaction to DAG
    if let Some(contract_logic) = &req.contract {
        dag.attach_contract(&tx_id, contract_logic.clone());
    }

    match data.consensus.propose_transaction(&mut dag, &tx) {
        Ok(_) => {
            // Save to database
            let result = sqlx::query!(
                "INSERT INTO transactions (id, sender, receiver, amount, parents, priority, contract, timestamp, metadata)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
                tx.id,
                tx.sender,
                tx.receiver,
                tx.amount,
                tx.parents, // If parents is a JSON array
                tx.priority,
                tx.contract,
                tx.timestamp,
                tx.metadata
            )
            .execute(pool.get_ref())
            .await?;

            match result {
                Ok(_) => HttpResponse::Ok().json(tx),
                Err(e) => HttpResponse::InternalServerError().body(format!("Database error: {}", e)),
            }
        }
        Err(e) => HttpResponse::BadRequest().body(e),
    }
}

async fn get_all_transactions(
    pool: web::Data<PgPool>,
) -> impl Responder {
    match sqlx::query_as!(
        Transaction,
        "SELECT * FROM transactions ORDER BY timestamp DESC"
    )
    .fetch_all(pool.get_ref())
    .await
    {
        Ok(transactions) => HttpResponse::Ok().json(transactions),
        Err(e) => HttpResponse::InternalServerError().body(format!("Database error: {}", e)),
    }
}

fn generate_private_key() -> String {
    let mut key = [0u8; 32]; // 256-bit key
    OsRng.fill_bytes(&mut key);
    hex::encode(key) // Convert to hex string
}

// Initialize the Server State
fn init_state() -> AppState {
    let consensus = Consensus::new(
        "Leader".to_string(),
        vec!["Validator1".to_string(), "Validator2".to_string()],
    );
    let mut liquidity_pools = HashMap::new();

    // Create a stablecoin-to-USD pool
    let stablecoin_pool = LiquidityPool::new("USD".to_string(), "STABLECOIN".to_string());
    liquidity_pools.insert("stablecoin_pool".to_string(), stablecoin_pool);

    // Create a DAG-native-token-to-USDT pool
    let dag_pool = LiquidityPool::new("DAG".to_string(), "USDT".to_string());
    liquidity_pools.insert("dag_pool".to_string(), dag_pool);

    let genesis_private_key = generate_private_key(); // Generate a secure key
    let genesis_wallet = Wallet::new(genesis_private_key);

    let mut initial_balances = HashMap::new();
    initial_balances.insert("GENESIS_TOKEN".to_string(), 37_890_274);

    let mut wallets = HashMap::new();
    wallets.insert(
        genesis_wallet.address.clone(),
        Wallet {
            address: genesis_wallet.address.clone(),
            private_key: genesis_wallet.private_key.clone(),
            balances: initial_balances,
        },
    );

    AppState {
        dag: Mutex::new(DAG::new()),
        consensus,
        validator_load: Arc::new(Mutex::new(HashMap::new())),
        wallets: Mutex::new(wallets),
        subscription_data: Mutex::new(HashMap::new()),
        nft_data: Mutex::new(HashMap::new()),
        token_data: Mutex::new(HashMap::new()),
        staking_data: Mutex::new(HashMap::new()),
        lending_data: Mutex::new(HashMap::new()),
        liquidity_pools: Mutex::new(liquidity_pools),
        transfer_history: Mutex::new(Vec::new()),
        swap_history: Mutex::new(Vec::new()),
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityPool {
    pub token_a: String,       // First token in the pool
    pub token_b: String,       // Second token in the pool
    pub reserves_a: u64,       // Reserves for token_a
    pub reserves_b: u64,       // Reserves for token_b
}

impl LiquidityPool {
    pub fn new(token_a: String, token_b: String) -> Self {
        LiquidityPool {
            token_a,
            token_b,
            reserves_a: 0, // Initialize reserves to zero
            reserves_b: 0,
        }
    }

    pub fn add_liquidity(&mut self, token_a_amount: u64, token_b_amount: u64) {
        self.reserves_a += token_a_amount;
        self.reserves_b += token_b_amount;
    }
}

pub async fn add_liquidity(
    data: web::Data<AppState>,
    req: web::Json<HashMap<String, String>>, // {"pool_name": "pool1", "token1": "USD", "token2": "DAG", "amount1": "100", "amount2": "200"}
) -> impl Responder {
    let mut liquidity_pools = data.liquidity_pools.lock().unwrap();

    // Extract data from the request
    let pool_name = req.get("pool_name").cloned().unwrap_or_default();
    let token_a = req.get("token1").cloned().unwrap_or_default();
    let token_b = req.get("token2").cloned().unwrap_or_default();
    let amount_a = req.get("amount1").and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
    let amount_b = req.get("amount2").and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);

    if pool_name.is_empty() || token_a.is_empty() || token_b.is_empty() || amount_a == 0 || amount_b == 0 {
        return HttpResponse::BadRequest().body("Invalid pool or token data");
    }

    // Retrieve or create the pool
    let pool = liquidity_pools
        .entry(pool_name.clone())
        .or_insert(LiquidityPool::new(token_a.clone(), token_b.clone()));

    // Add reserves
    pool.reserves_a += amount_a;
    pool.reserves_b += amount_b;

    HttpResponse::Ok().json(json!({
        "message": "Liquidity added successfully",
        "pool": {
            "name": pool_name,
            "token_a": pool.token_a,
            "token_b": pool.token_b,
            "reserves_a": pool.reserves_a,
            "reserves_b": pool.reserves_b,
        }
    }))
}


// View all liquidity pools
pub async fn get_liquidity_pools(data: web::Data<AppState>) -> impl Responder {
    let liquidity_pools = data.liquidity_pools.lock().unwrap();

    let pools: Vec<_> = liquidity_pools
        .iter()
        .map(|(name, pool)| {
            json!({
                "name": name,
                "token_a": pool.token_a,
                "token_b": pool.token_b,
                "reserves_a": pool.reserves_a,
                "reserves_b": pool.reserves_b,
            })
        })
        .collect();

    HttpResponse::Ok().json(pools)
}

#[derive(Deserialize)]
struct MintRequest {
    recipient: String,
    amount: u64,
    token_symbol: String,
}

async fn mint_tokens(
    data: web::Data<AppState>,
    req: web::Json<MintRequest>,
) -> impl Responder {
    let mut token_store = data.token_data.lock().unwrap();

    if let Some(token) = token_store.get_mut(&req.token_symbol) {
        token.mint(req.recipient.clone(), req.amount);
        HttpResponse::Ok().json(json!({
            "message": "Tokens minted successfully",
            "token_symbol": req.token_symbol,
            "recipient": req.recipient,
            "amount_minted": req.amount,
            "total_supply": token.total_supply
        }))
    } else {
        HttpResponse::BadRequest().body("Token not found")
    }
}


async fn get_minted_tokens(
    data: web::Data<AppState>,
) -> impl Responder {
    let token_store = data.token_data.lock().unwrap();

    let tokens: Vec<_> = token_store
        .values()
        .map(|token| {
            json!({
                "id": token.id,
                "name": token.name,
                "symbol": token.symbol,
                "total_supply": token.total_supply,
                "balances": token.balances,
                "decimals": token.decimals
            })
        })
        .collect();

    HttpResponse::Ok().json(tokens)
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TransferRequest {
    pub from: String,
    pub to: String,
    pub token: String,
    pub amount: u64,
}

async fn transfer_tokens(
    data: web::Data<AppState>,
    req: web::Json<TransferRequest>,
) -> impl Responder {
    let mut wallets = data.wallets.lock().unwrap();
    let mut history = data.transfer_history.lock().unwrap();

    let from = req.from.clone();
    let to = req.to.clone();
    let token = req.token.clone();
    let amount = req.amount;

    if from == to {
        return HttpResponse::BadRequest().body("Sender and receiver cannot be the same");
    }

    // Workaround to avoid multiple mutable borrows at the same time
    let sender_wallet_exists = wallets.contains_key(&from);
    let receiver_wallet_exists = wallets.contains_key(&to);

    if !sender_wallet_exists || !receiver_wallet_exists {
        return HttpResponse::BadRequest().body("Sender or receiver wallet not found");
    }

    let sender_wallet_balances;
    let receiver_wallet_balances;

    {
        // Retrieve sender and receiver wallets safely
        let sender_wallet = wallets.get_mut(&from).unwrap();
        sender_wallet_balances = sender_wallet.balances.entry(token.clone()).or_insert(0);

        if *sender_wallet_balances < amount {
            return HttpResponse::BadRequest().body("Insufficient balances");
        }

        *sender_wallet_balances -= amount;
    }

    {
        let receiver_wallet = wallets.get_mut(&to).unwrap();
        receiver_wallet_balances = receiver_wallet.balances.entry(token.clone()).or_insert(0);
        *receiver_wallet_balances += amount;
    }

    // Record the transfer in history
    history.push(req.into_inner());

    HttpResponse::Ok().json(json!({
        "from": from,
        "to": to,
        "token": token,
        "amount": amount
    }))
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SwapRequest {
    pub address: String,
    pub from_token: String,
    pub to_token: String,
    pub amount: u64,
}

async fn swap_tokens(
    data: web::Data<AppState>,
    req: web::Json<SwapRequest>,
) -> impl Responder {
    let mut wallets = data.wallets.lock().unwrap();
    let mut history = data.swap_history.lock().unwrap();

    let address = req.address.clone();
    let from_token = req.from_token.clone();
    let to_token = req.to_token.clone();
    let amount = req.amount;

    if let Some(wallet) = wallets.get_mut(&address) {
        let from_balances = wallet.balances.entry(from_token.clone()).or_insert(0);
        if *from_balances < amount {
            return HttpResponse::BadRequest().body("Insufficient balances for swap");
        }

        *from_balances -= amount;
        let to_balances = wallet.balances.entry(to_token.clone()).or_insert(0);
        *to_balances += amount;

        history.push(req.into_inner());
        HttpResponse::Ok().json(json!({
            "address": address,
            "from_token": from_token,
            "to_token": to_token,
            "amount": amount,
            "updated_balances": wallet.balances
        }))
    } else {
        HttpResponse::BadRequest().body("Wallet not found")
    }
}

// GET/ Transfer History
async fn get_transfer_history(data: web::Data<AppState>) -> impl Responder {
    let history = data.transfer_history.lock().unwrap();
    HttpResponse::Ok().json(&*history)
}

// GET/ Swap History
async fn get_swap_history(data: web::Data<AppState>) -> impl Responder {
    let history = data.swap_history.lock().unwrap();
    HttpResponse::Ok().json(&*history)
}

// POST /wallets: Generate a new wallet
async fn create_wallet(data: web::Data<AppState>) -> impl Responder {
    let private_key = generate_private_key(); // Securely generate a private key
    let wallet = Wallet::new(private_key); // Use the generated private key

    let mut wallets = data.wallets.lock().unwrap();
    wallets.insert(wallet.address.clone(), wallet.clone());

    HttpResponse::Ok().json(wallet)
}


async fn get_wallet_balances(
    data: web::Data<AppState>,
    address: web::Path<String>,
) -> impl Responder {
    let wallets = data.wallets.lock().unwrap();

    if let Some(wallet) = wallets.get(&address.into_inner()) {
        HttpResponse::Ok().json(json!({
            "address": wallet.address,
            "private_key": wallet.private_key, // Display private key (not secure in production!)
            "balances": wallet.balances
        }))
    } else {
        HttpResponse::NotFound().body("Wallet not found")
    }
}

// POST /transactions: Add a new transaction
async fn add_transaction(
    data: web::Data<AppState>,
    req: web::Json<AddTransactionRequest>,
) -> Result<HttpResponse, sqlx::Error> {
    let mut dag = data.dag.lock().unwrap();
    let tx = create_signed_transaction(
        &data.consensus.get_key_pair(),
        req.receiver.clone(),
        req.amount,
        req.parents.clone(),
        req.priority.unwrap_or(0),
    );

    // Attach contract logic if provided
    if let Some(contract_logic) = &req.contract {
        dag.attach_contract(&tx.id, contract_logic.clone());
    }

    // Propose the transaction
    data.consensus.propose_transaction(&mut dag, &tx)?;

    // Return a success response
    Ok(HttpResponse::Ok().json(tx))
}

// GET /transactions/{id}: Get details of a transaction by ID
async fn get_transaction(
    data: web::Data<AppState>,
    id: web::Path<String>,
) -> impl Responder {
    let dag = data.dag.lock().unwrap();
    match dag.transactions.get(&id.into_inner()) {
        Some(tx) => HttpResponse::Ok().json(tx),
        None => HttpResponse::NotFound().body("Transaction not found"),
    }
}

// Define NFT Minting Endpoint
async fn mint_nft(
    data: web::Data<AppState>,
    req: web::Json<HashMap<String, String>>,
) -> impl Responder {
    let owner = req.get("owner").cloned().unwrap_or_default();
    let metadata = req.get("metadata")
        .map(|v| serde_json::from_str::<HashMap<String, String>>(v).unwrap_or_default())
        .unwrap_or_default();
    let nft = nft::NFT::new(owner, metadata, Some(2.5));

    let mut nft_store = data.nft_data.lock().unwrap();
    nft_store.insert(nft.id.clone(), nft.clone()); // Store NFT
    HttpResponse::Ok().json(nft)
}

async fn get_nft(
    data: web::Data<AppState>,
    id: web::Path<String>,
) -> impl Responder {
    let nft_store = data.nft_data.lock().unwrap();
    if let Some(nft) = nft_store.get(&id.into_inner()) {
        HttpResponse::Ok().json(nft)
    } else {
        HttpResponse::NotFound().body("NFT not found")
    }
}

// Define Subscription Endpoint
async fn create_subscription(
    data: web::Data<AppState>,
    req: web::Json<HashMap<String, String>>,
) -> impl Responder {
    let subscription = subscription::Subscription::new(
        req.get("subscriber").cloned().unwrap_or_default(),
        req.get("service").cloned().unwrap_or_default(),
        req.get("amount").and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0),
        req.get("duration").cloned().unwrap_or("30d".to_string()),
    );

    let mut subscriptions = data.subscription_data.lock().unwrap();
    subscriptions.insert(subscription.id.clone(), subscription.clone()); // Store the subscription
    HttpResponse::Ok().json(subscription)
}

// GET /contracts/subscription/search: Search for subscriptions by query
async fn get_subscription(
    data: web::Data<AppState>,
    id: web::Path<String>,
) -> impl Responder {
    let subscriptions = data.subscription_data.lock().unwrap();
    if let Some(subscription) = subscriptions.get(&id.into_inner()) {
        HttpResponse::Ok().json(subscription)
    } else {
        HttpResponse::NotFound().body("Subscription not found")
    }
}

// Define Token Creation Endpoint
async fn create_token(
    data: web::Data<AppState>,
    req: web::Json<HashMap<String, serde_json::Value>>,
) -> impl Responder {
    let name = req.get("name").and_then(|v| v.as_str()).unwrap_or_default().to_string();
    let symbol = req.get("symbol").and_then(|v| v.as_str()).unwrap_or_default().to_string();
    let total_supply = req.get("total_supply").and_then(|v| v.as_u64()).unwrap_or(0);
    let decimals = req.get("decimals").and_then(|v| v.as_u64()).unwrap_or(0) as u8;

    let token = Token {
        id: Uuid::new_v4().to_string(),
        name,
        symbol,
        total_supply,
        balances: HashMap::new(),
        decimals,
    };

    let mut token_store = data.token_data.lock().unwrap();
    token_store.insert(token.symbol.clone(), token::Token::from(token.clone().into()));


    HttpResponse::Ok().json(token)
}

// Define Staking Endpoint
async fn stake_tokens(
    data: web::Data<AppState>,
    req: web::Json<HashMap<String, u64>>,
) -> impl Responder {
    let user = req.get("user").cloned().unwrap_or_default().to_string();
    let amount = *req.get("amount").unwrap_or(&0);

    let mut staking_contract = staking::StakingContract::new(5.0);
    match staking_contract.stake(user.clone(), amount) {
        Ok(_) => {
            let mut staking_store = data.staking_data.lock().unwrap();
            staking_store.insert(user.clone(), staking_contract.clone()); // Store staking record
            HttpResponse::Ok().json(staking_contract)
        }
        Err(err) => HttpResponse::BadRequest().body(err),
    }
}

async fn get_staking(
    data: web::Data<AppState>,
    id: web::Path<String>,
) -> impl Responder {
    let staking_store = data.staking_data.lock().unwrap();
    if let Some(staking) = staking_store.get(&id.into_inner()) {
        HttpResponse::Ok().json(staking)
    } else {
        HttpResponse::NotFound().body("Staking record not found")
    }
}

// Define Lending Endpoint
async fn borrow_tokens(
    data: web::Data<AppState>,
    req: web::Json<HashMap<String, u64>>,
) -> impl Responder {
    let user = req.get("user").cloned().unwrap_or_default().to_string();
    let amount = *req.get("amount").unwrap_or(&0);

    let mut lending_pool = lending::LendingPool::new("Main Pool".to_string(), 3.0, 0.5);
    match lending_pool.borrow(user.clone(), amount) {
        Ok(_) => {
            let mut lending_store = data.lending_data.lock().unwrap();
            lending_store.insert(user.clone(), lending_pool.clone()); // Store lending record
            HttpResponse::Ok().json(lending_pool)
        }
        Err(err) => HttpResponse::BadRequest().body(err),
    }
}

async fn get_lending(
    data: web::Data<AppState>,
    id: web::Path<String>,
) -> impl Responder {
    let lending_store = data.lending_data.lock().unwrap();
    if let Some(lending) = lending_store.get(&id.into_inner()) {
        HttpResponse::Ok().json(lending)
    } else {
        HttpResponse::NotFound().body("Lending record not found")
    }
}

async fn get_token(
    data: web::Data<AppState>,
    id: web::Path<String>,
) -> impl Responder {
    let token_store = data.token_data.lock().unwrap();
    if let Some(token) = token_store.get(&id.into_inner()) {
        HttpResponse::Ok().json(token)
    } else {
        HttpResponse::NotFound().body("Token not found")
    }
}

// GET /token/list: Retrieve all tokens
async fn list_tokens(data: web::Data<AppState>) -> impl Responder {
    let token_store = data.token_data.lock().unwrap();

    let tokens: Vec<_> = token_store.values().cloned().collect(); // Collect all tokens into a Vec
    HttpResponse::Ok().json(tokens) // Return the list as JSON
}


// GET /consensus/load: Get validator load
async fn get_validator_load(data: web::Data<AppState>) -> impl Responder {
    let validator_load = data.validator_load.lock().unwrap();
    HttpResponse::Ok().json(&*validator_load)
}

// POST /consensus/rebalance: Rebalance validator loads
async fn rebalance_validator_load(data: web::Data<AppState>) -> impl Responder {
    let mut validator_load = data.validator_load.lock().unwrap();
    validator_load.iter_mut().for_each(|(_, load)| *load = 0);

    let result = serde_json::to_string(&*validator_load).unwrap_or_else(|_| "{}".to_string());
    HttpResponse::Ok().body(format!("Validator load rebalanced: {}", result))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok(); // Load environment variables from `.env`
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&database_url).await.expect("Failed to connect to database");
    // Initialize a new DeFi pool
    let mut pool = DeFiPool::new("Main Pool".to_string(), 0.05);

    // Add liquidity to the pool
    pool.add_liquidity("user1".to_string(), 100).unwrap();
    pool.add_liquidity("user2".to_string(), 200).unwrap();

    // Remove liquidity
    pool.remove_liquidity("user1".to_string(), 50).unwrap();

    // Calculate rewards
    let rewards = pool.calculate_rewards("user2").unwrap();
    println!("User2 rewards: {}", rewards);

    // List all liquidity providers
    let liquidity = pool.list_liquidity();
    println!("Liquidity providers: {:?}", liquidity);
    let config = Config::parse();
    let app_state = web::Data::new(init_state());

    if config.validator {
        // Run as a validator node
        let address = config.address.unwrap_or_else(|| "127.0.0.1:8080".to_string());
        println!("Starting validator node at http://{}", address);

        HttpServer::new(move || {
            App::new()
            .app_data(app_state.clone())
            .route("/transactions", web::post().to(add_transaction)) // POST route for transactions
            .route("/transactions", web::get().to(get_all_transactions)) // GET route to list all transactions
            .route("/transactions/{id}", web::get().to(get_transaction))
            .route("/contracts/nft/mint", web::post().to(mint_nft))
            .route("/contracts/nft/{id}", web::get().to(get_nft)) // NFT search
            .route("/contracts/subscription", web::post().to(create_subscription))
            .route("/contracts/subscription/{id}", web::get().to(get_subscription)) // Subscription search
            .route("/liquidity/add", web::post().to(add_liquidity))
            .route("/liquidity/pools", web::get().to(get_liquidity_pools))
            .route("/token/mint", web::post().to(mint_tokens))
            .route("/token/mint", web::get().to(get_minted_tokens))
            .route("/token/create", web::post().to(create_token))
            .route("/token/{id}", web::get().to(get_token)) // Token search
            .route("/token/list", web::get().to(list_tokens)) // Token listing
            .route("/tokens/transfer", web::post().to(transfer_tokens))     // Transfer tokens
            .route("/tokens/transfer/history", web::get().to(get_transfer_history)) // Get transfer history
            .route("/tokens/swap", web::post().to(swap_tokens))             // Swap tokens
            .route("/tokens/swap/history", web::get().to(get_swap_history)) // Get swap history
            .route("/staking/stake", web::post().to(stake_tokens))
            .route("/staking/{id}", web::get().to(get_staking)) // Staking search
            .route("/lending/borrow", web::post().to(borrow_tokens))
            .route("/lending/{id}", web::get().to(get_lending)) // Lending search
            .route("/wallets", web::post().to(create_wallet))
            .route("/wallets/{address}", web::get().to(get_wallet_balances)) // Get wallet balances
            .route("/consensus/load", web::get().to(get_validator_load))
            .route("/consensus/rebalance", web::post().to(rebalance_validator_load))
        })
        .bind(&address)?
        .run()
        .await
    } else {
        // Run as a standard node
        println!("Starting server at http://127.0.0.1:{}", config.port);

        HttpServer::new(move || {
            App::new()
            .app_data(app_state.clone())
            .route("/transactions", web::post().to(add_transaction)) // POST route for transactions
            .route("/transactions", web::get().to(get_all_transactions)) // GET route to list all transactions
            .route("/transactions/{id}", web::get().to(get_transaction))
            .route("/contracts/nft/mint", web::post().to(mint_nft))
            .route("/contracts/nft/{id}", web::get().to(get_nft)) // NFT search
            .route("/contracts/subscription", web::post().to(create_subscription))
            .route("/contracts/subscription/{id}", web::get().to(get_subscription)) // Subscription search
            .route("/liquidity/add", web::post().to(add_liquidity))
            .route("/liquidity/pools", web::get().to(get_liquidity_pools))
            .route("/token/mint", web::post().to(mint_tokens))
            .route("/token/mint", web::get().to(get_minted_tokens))
            .route("/token/create", web::post().to(create_token))
            .route("/token/{id}", web::get().to(get_token)) // Token search
            .route("/token/list", web::get().to(list_tokens)) // Token listing
            .route("/tokens/transfer", web::post().to(transfer_tokens))     // Transfer tokens
            .route("/tokens/transfer/history", web::get().to(get_transfer_history)) // Get transfer history
            .route("/tokens/swap", web::post().to(swap_tokens))             // Swap tokens
            .route("/tokens/swap/history", web::get().to(get_swap_history)) // Get swap history
            .route("/staking/stake", web::post().to(stake_tokens))
            .route("/staking/{id}", web::get().to(get_staking)) // Staking search
            .route("/lending/borrow", web::post().to(borrow_tokens))
            .route("/lending/{id}", web::get().to(get_lending)) // Lending search
            .route("/wallets", web::post().to(create_wallet))
            .route("/wallets/{address}", web::get().to(get_wallet_balances)) // Get wallet balances
            .route("/consensus/load", web::get().to(get_validator_load))
            .route("/consensus/rebalance", web::post().to(rebalance_validator_load))
        })
        .bind(("127.0.0.1", config.port))?
        .run()
        .await
    }
}