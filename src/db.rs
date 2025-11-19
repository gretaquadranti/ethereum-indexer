use sqlx::{PgPool, Transaction, Postgres, Row};
use crate::models::Block;
use crate::utils::hex_to_i64;
use std::error::Error;

//metodo per ottenere l'ultimo blocco 
pub async fn get_last_indexed_block(pool: &PgPool) -> Result<i64, Box<dyn Error + Send + Sync>> {
    let row = sqlx::query("SELECT last_block_indexed FROM indexer_state WHERE id = 1")
        .fetch_one(pool)
        .await?;
    
    let last_block: i64 = row.get(0);
    Ok(last_block)
}

//metodo per fare UPDATE sull'ultimo blocco salvato sul db
pub async fn update_last_indexed_block(
    pool: &PgPool, 
    block_number: i64
) -> Result<(), Box<dyn Error + Send + Sync>> {
    sqlx::query(
        "UPDATE indexer_state 
         SET last_block_indexed = $1, last_update = NOW() 
         WHERE id = 1"
    )
    .bind(block_number)
    .execute(pool)
    .await?;
    
    Ok(())
}

//metodo per salvare blocco
pub async fn save_block(
    db_transazione: &mut Transaction<'_, Postgres>,
    block: &Block
) -> Result<(), Box<dyn Error + Send + Sync>> {
    
    
    //trasformo da esadecimali
    let block_number = hex_to_i64(&block.number)?;
    let timestamp = hex_to_i64(&block.timestamp)?;
    let gas_used = hex_to_i64(&block.gas_used)?;
    let gas_limit = hex_to_i64(&block.gas_limit)?;
    let size = hex_to_i64(&block.size)?;
    let tx_count = block.transactions.len() as i32;
    
    sqlx::query(
        "INSERT INTO blocks 
         (number, hash, parent_hash, timestamp, miner, gas_used, gas_limit, transactions_count, size)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
         ON CONFLICT (number) DO NOTHING"
    )
    .bind(block_number)
    .bind(&block.hash)
    .bind(&block.parent_hash)
    .bind(timestamp)
    .bind(&block.miner)
    .bind(gas_used)
    .bind(gas_limit)
    .bind(tx_count)
    .bind(size)
    .execute(&mut **db_transazione)
    .await?;
    
    Ok(())
}


