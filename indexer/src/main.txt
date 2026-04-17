mod models;
mod db;
mod alchemy;
mod utils;
 
use dotenv::dotenv;
use std::env;
use std::time::Duration;
use std::sync::Arc;
use sqlx::PgPool;
use std::pin::Pin;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenv().ok();
    
    //db setup:
    let db_conn = format!(
        "postgres://{}:{}@{}/{}",
        env::var("DB_USER")?,
        env::var("DB_PASSWORD")?,
        env::var("DB_HOST")?,
        env::var("DB_NAME")?
    );
    

    let db_pool = PgPool::connect(&db_conn).await?;
    println!("database connected");
    
    let db_pool = Arc::new(db_pool);

//---------------------------------------------------------------------------------
    //setup AlchemyClient
    let api_key = env::var("ALCHEMY_API_KEY").expect("error");
    let alchemy_http = Arc::new(alchemy::AlchemyClient::new(api_key.clone()));
    
    //serve per verificare se siamo up to date oppure bisogna fare catch up
    let last_indexed = db::get_last_indexed_block(&db_pool).await?;
    let latest_on_chain = alchemy_http.get_latest_block_number().await?;
    
    println!("last indexed: {}", last_indexed);
    println!("latest on chain: {}", latest_on_chain);
    
    let gap = latest_on_chain - last_indexed;
    if gap > 0 {
        println!("gap detected: {} blocks", gap);
        
        for block_num in (last_indexed + 1)..=latest_on_chain {

            //salvo ogni blocco chiamando il metodo index_blockchain
            let res = index_block(&alchemy_http, &db_pool, block_num).await;
            if let Ok(_) = res {
                    // aggiorno la tabella che tiene traccia dell'ultimo blocco salvato
                    db::update_last_indexed_block(&db_pool, block_num).await?;
            } else if let Err(e) = res {
                eprintln!("Error indexing block {}: {}. Skipping.", block_num, e);
            }
        }
            
        //rallento il loop
        tokio::time::sleep(Duration::from_millis(100)).await;
        println!("catch-up complete");
    } else {
        println!("already up to date");
    }
    
    //-------------------------------------------------------------------------------------------
    //parte webSocket
    println!("webSocket sync");
    let ws = alchemy::AlchemyWebSocket::new(api_key);

    //clono per usarli nel callback
    let alchemy_clone = Arc::clone(&alchemy_http);
    let db_clone = Arc::clone(&db_pool); 

    //definisco la callback, chiamata in futuro da subscribe new head
    let callback = move |block_hex: String|  -> Pin<Box<dyn std::future::Future<Output = ()> + Send + 'static>>{
    
        //clono per usarli dentro async block
        let alchemy = Arc::clone(&alchemy_clone); 
        let db = Arc::clone(&db_clone); 
    
        Box::pin(async move { 
            //il blocco che mi è arrivato esadecimale viene trasformato
            let result = utils::hex_to_i64(&block_hex); 

            if let Ok(block_num) = result {
                println!("new block: {}", block_num);

                //metodo che viene chiamato per vedere sul db l'ultimo blocco salvato
                let last_result = db::get_last_indexed_block(&db).await;

                if let Ok(last_indexed) = last_result {
                    if block_num > last_indexed {

                        for num in (last_indexed + 1)..=block_num {
                            //chiamo il metodo salva il nuovo blocco sul db
                            let add_block = index_block(&alchemy, &db, num).await;

                            if let Ok(_) = add_block {
                                //update sul db per tener traccia dell'ultimo blocco 
                                let update_db = db::update_last_indexed_block(&db, num).await;

                                if let Ok(_) = update_db {
                                    println!(" block {} indexed (from WS)", num);
                                } else if let Err(e) = update_db {
                                    eprintln!("error updating state: {}", e);
                                }
                            } else if let Err(e) = add_block {
                                eprintln!("error indexing block: {}", e);
                            }
                        } 
                    } else {
                        println!("block {} already indexed", block_num);
                    }
                } else if let Err(e) = last_result {
                    eprintln!("error getting last indexed: {}", e);
                    return;
                }
            } else if let Err(e) = result {
                eprintln!("error parsing block number from WS: {}", e);
                return;
            }
            () 
        }
    )
};
    

ws.subscribe_new_blocks(callback).await?;

Ok(())
}


//metodo che serve per prendere un blocco e salvo sul db
async fn index_block(
    alchemy: &alchemy::AlchemyClient,
    db_pool: &PgPool, 
    block_number: i64
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    
    //richiedo il blocco
    let block = alchemy.get_block(block_number).await?;
    
    //salvo il blocco 
    let mut db_transazione = db_pool.begin().await?;
    db::save_block(&mut db_transazione, &block).await?;
    
    db_transazione.commit().await?;
    
    Ok(())
}