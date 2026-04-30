use sqlx::{PgPool, Transaction, Postgres, Row};
use crate::models::Block;
use crate::utils::hex_to_i64;
use std::error::Error;

// legge dal database l'ultimo numero di blocco che è stato indicizzato
// viene usato all'avvio per sapere da dove riprendere il catch-up, e dalla
// callback WebSocket per rilevare eventuali gap tra una notifica e l'altra
pub async fn get_last_indexed_block(pool: &PgPool) -> Result<i64, Box<dyn Error + Send + Sync>> {
    let row = sqlx::query("SELECT last_block_indexed FROM indexer_state WHERE id = 1")
        .fetch_one(pool)
        .await?;
    
    let last_block: i64 = row.get(0);
    Ok(last_block)
}

// aggiorna il numero dell'ultimo blocco indicizzato nella tabella indexer_state
// viene chiamato ogni volta che un blocco viene salvato con successo, in modo che
// in caso di riavvio il catch-up sappia esattamente da dove riprendere
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

// inserisce un blocco nella tabella blocks all'interno di una transazione aperta
// i campi del blocco arrivano in esadecimale da Alchemy e vengono convertiti in
// interi prima di essere salvati
// ON CONFLICT DO NOTHING evita errori se il blocco è già presente
pub async fn save_block(
    db_transazione: &mut Transaction<'_, Postgres>,
    block: &Block
) -> Result<(), Box<dyn Error + Send + Sync>> {
    
    // i valori numerici arrivano come stringhe esadecimali e vengono convertiti in i64 prima di essere passati alla query
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


// recupera tutti i blocchi salvati nel database ordinati per numero crescente
// viene usato all'avvio quando non c'è gap: invece di riscaricaricare tutto
// si ricostruisce il Verkle tree direttamente dai dati già in locale
pub async fn get_all_blocks(pool: &PgPool)-> Result<Vec<(i64, String)>, Box<dyn Error + Send + Sync>> {
    
    let rows = sqlx::query("SELECT number, hash FROM blocks ORDER BY number ASC")
        .fetch_all(pool)
        .await?;
    
    // trasforma ogni riga in una tupla (numero, hash) 
    let blocks = rows
        .iter()
        .map(|row| {
            let number: i64 = row.get(0);
            let hash: String = row.get(1);
            (number, hash)
        })
        .collect();
 
    Ok(blocks)
}