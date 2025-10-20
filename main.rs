use dotenv::dotenv;
use std::env;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio_postgres::{NoTls};


#[derive(Serialize)]
struct JRPCRequest {
    jsonrpc: String,
    method: String,
    params: Vec<Value>,
    id: i32,
}

// Struttura per la risposta JSON-RPC
#[derive(Deserialize, Debug)]
struct JRPCResponse {
    id: i32,
    jsonrpc: String,
    result: Block,
}


#[derive(Debug, Deserialize)]
struct Block {
    number: String,
    hash: String,
    #[serde(rename = "parentHash")]
    parent_hash: String,
    timestamp: String,
    miner: String,
    #[serde(rename = "gasUsed")]
    gas_used: String,
    #[serde(rename = "gasLimit")]
    gas_limit: String,
    transactions: Vec<serde_json::Value>,
    size: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    //carico le variabili da env
    dotenv().ok();


    let api_key = env::var("ALCHEMY_API_KEY").expect("ALCHEMY_API_KEY is missing");
    let db_pass = env::var("DB_PASSWORD").expect("DB_PASSWORD is missing");
    let db_name = env::var("DB_NAME").expect("DB_NAME is missing");
    let db_user: String = env::var("DB_USER").expect("DB_USER is missing");
    let db_host: String = env::var("DB_HOST").expect("DB_HOST is missing");


    let db_conn = format!("host={} user={} password={} dbname={}", db_host, db_user, db_pass,db_name);
    //db connection
    let (db_client, connection) = tokio_postgres::connect(&db_conn, NoTls).await?;

    println!("db connected");

    tokio::spawn(async move {
    if let Err(e) = connection.await {
        eprintln!("Errore connessione DB: {}", e);
    }
});


    //http request
    let http_client = Client::new();

    let url = format!("https://eth-sepolia.g.alchemy.com/v2/{}", api_key);



    let block_request = JRPCRequest {
            jsonrpc: "2.0".to_string(),
            method: "eth_getBlockByNumber".to_string(),
            params: vec![json!("latest") , json!(true)],
            id: 1,
        };
    
    let response= http_client.post(url).json(&block_request).send().await?;

    let block : JRPCResponse = response.json().await?;

    let block_number = hex_to_i64(&block.result.number)?;
    let timestamp = hex_to_i64(&block.result.timestamp)?;
    let gas_used = hex_to_i64(&block.result.gas_used)?;
    let gas_limit = hex_to_i64(&block.result.gas_limit)?;
    let size = hex_to_i64(&block.result.size)?;
    let tx_count = block.result.transactions.len() as i32;


    db_client.execute(
        "INSERT INTO blocks (number, hash, parent_hash, timestamp, miner, gas_used, gas_limit, transactions_count, size)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        &[
            &block_number,
            &block.result.hash,
            &block.result.parent_hash,
            &timestamp,
            &block.result.miner,
            &gas_used,
            &gas_limit,
            &tx_count,
            &size,
        ],
    ).await?;

    Ok(())

}


//funzione per trasformare esadecimale 
fn hex_to_i64(hex: &str) -> Result<i64, Box<dyn std::error::Error>> {
    let no_prefix = hex.trim_start_matches("0x");

//trasforma la stringa del numero esadecimale in i64 decimale
    Ok(i64::from_str_radix(no_prefix, 16)?)
}


