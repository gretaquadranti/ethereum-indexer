mod models;
mod db;
mod alchemy;
mod utils;
mod api;
use models::Block;
use dotenv::dotenv;
use std::env;
use sqlx::PgPool;
use std::pin::Pin;
use tokio::sync::Mutex;
use std::sync::Arc;
use verkle_project::{VerkleTree, kzg::trusted_setup};



// viene avviato un thread dedicato con uno stack allargato (64 MB invece
// dei 2 MB predefiniti) perché sennò stack overflow
fn main() {
    let stack_size = 64 * 1024 * 1024; 
    let builder = std::thread::Builder::new().stack_size(stack_size);
    
    let handler = builder.spawn(|| {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async_operation()) 
            .unwrap();
    }).unwrap();

    handler.join().unwrap();
}


async fn async_operation() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenv().ok();

    // -------------------------DB CONNESSIONE--------------------------------------
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

    // ----------------------SETUP DEL VERKLE-----------------------------------------

    let pk = trusted_setup(255);
    //l'albero può essere condiviso + mutex 
    let tree = Arc::new(Mutex::new(VerkleTree::new(pk)));
    println!("verkle tree initialized");

    // -----------------------ALCHEMY SETUP----------------------------------------

    let api_key = env::var("ALCHEMY_API_KEY").expect("ALCHEMY_API_KEY mancante");
    let alchemy_http = Arc::new(alchemy::AlchemyClient::new(api_key.clone()));

    // --------------------------FASE DI CATCH UP-------------------------------------
    
    // all'avvio confronta l'ultimo blocco registrato nel db con
    // l'ultimo blocco disponibile sulla blockchain
    //se esiste un gap, i blocchi mancanti vengono scaricati, salvati sul DB e inseriti nel Verkle tree prima che il listener
    let last_indexed = db::get_last_indexed_block(&db_pool).await?; 
    let latest_on_chain = alchemy_http.get_latest_block_number().await?;
    println!("last indexed: {}", last_indexed);
    println!("latest on chain: {}", latest_on_chain);

    let gap = latest_on_chain - last_indexed;

    let mut tree_lock = tree.lock().await;
    
    let blocks = db::get_all_blocks(&db_pool).await?;
    let nodes  = db::load_all_nodes(&db_pool).await?;

    for (numero, hash) in &blocks {
            let key   = utils::block_number_to_key(*numero);
            let value = utils::hash_to_value(hash);

            tree_lock.insert(key, value, false); //FALSE perche non ricalcola commitment
        }
        tree_lock.load_commitments_from_db(nodes);


        if gap > 0 {
            println!("gap detected: {} blocks", gap);
            for block_num in (last_indexed + 1)..=latest_on_chain {
            println!("inizio elaborazione blocco {}", block_num); 

            match index_block(&alchemy_http, &db_pool, block_num).await {
                Ok(block) => {
                    println!("blocco {} scaricato da alchemy", block_num);
                    let key = utils::block_number_to_key(block_num);
                    let value = utils::hash_to_value(&block.hash);
                    
                    println!("insert nel tree...", ); 

                    let vec_Mod_Nodes= tree_lock.insert(key, value, true); 
                    println!(" insert completato, nodi modificati: {}", vec_Mod_Nodes.len()); 
                    
                    for node in vec_Mod_Nodes{
                        if let Err(e) = db::save_node(&db_pool, &node.path, &node.node_type, &node.commitment_bytes).await {
                            eprintln!("errore salvataggio nodo: {}", e);
                        }                   
                    }
                    println!("nodi salvati", ); 
                }
                Err(e) => {
                    eprintln!("errore blocco {}: {}. skipping.", block_num, e);
                }  
            }
            println!("aggiorno last_indexed a {}", block_num); 

            if let Err(e) = db::update_last_indexed_block(&db_pool, block_num).await {
                eprintln!("errore update last indexed: {}", e);
            } 
        }
    }
    println!("scaricamento completato");


    drop(tree_lock);
    println!("catch-up completo");
    
    let tree_lock: tokio::sync::MutexGuard<'_, VerkleTree> = tree.lock().await;
    if let Some(root) = tree_lock.get_root_commitment() {
        println!("RADICE DEL TREE: {:?}", root);
    }

       
    // ----------------------------SETUP PER TASK-----------------------------------
    
    let tree_ws    = Arc::clone(&tree);
    let tree_api   = Arc::clone(&tree);
    let alchemy_clone = Arc::clone(&alchemy_http);
    let db_clone      = Arc::clone(&db_pool); //pgPool è thread safe

    // -------------------------TASK 1: WEBSOCKET--------------------------------------
    
    
    let ws_task = tokio::spawn(async move {
        let ws = alchemy::AlchemyWebSocket::new(api_key);

        //riceve blocco hex
        let callback = move |block_hex: String| -> Pin<Box<dyn std::future::Future<Output = ()> + Send + 'static>> {
            
            let alchemy = Arc::clone(&alchemy_clone);
            let db      = Arc::clone(&db_clone);
            let tree    = Arc::clone(&tree_ws);

            Box::pin(async move { 
                let Ok(block_num) = utils::hex_to_i64(&block_hex) else {
                    eprintln!("error parsing block number: {}", block_hex);
                    return;
                };

                eprintln!("new block: {}", block_num);

                // legge l'ultimo numero di blocco salvato per vedere 
                //se sono stati saltati dei blocchi tra la notifica e la precedente
                let Ok(ultimo_blocco_letto) = db::get_last_indexed_block(&db).await else {
                    eprintln!("error getting last indexed block");
                    return;
                };

                if block_num <= ultimo_blocco_letto {
                    eprintln!("block {} already indexed", block_num);
                    return;
                }

                //scorro dall'ultimo blocco che ho letto fino all'ultimo blocco che si trova su eth
                for num in (ultimo_blocco_letto + 1)..=block_num {
                    match index_block(&alchemy, &db, num).await {
                       
                        Ok(block) => {
                            if let Err(e) = db::update_last_indexed_block(&db, num).await {
                                eprintln!("error updating state: {}", e);
                                continue;
                            }
                            let key   = utils::block_number_to_key(num);
                            let value = utils::hash_to_value(&block.hash);
                            tree.lock().await.insert(key, value, true);
                            eprintln!("block {} indexed e inserito nel tree", num);
                        }
                        Err(e) => {
                            eprintln!("error indexing block {}: {}", num, e);
                        }
                    }
                }
            })
        };
     _ = ws.subscribe_new_blocks(callback).await;
        
    });

    // -----------------------TASK 2: API HTTP---------------------------------------
   
    let api_task = tokio::spawn(async move {

        let router = api::build_router(tree_api);

        //apre la porta 
        let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
            .await
            .expect("impossibile aprire porta 3000");

        
        println!("API server in ascolto su http://localhost:3000");
        axum::serve(listener, router)
            .await
            .expect("errore server HTTP");
    });

    // entrambi i task sono progettati per girare indefinitamente. il join mantiene
    // il processo in vita finché almeno uno dei due task è attivo.
    tokio::join!(ws_task, api_task);

    Ok(())

}    

// scarica un singolo blocco dall'RPC Alchemy, lo salva nel database
// la transazione garantisce che un blocco non venga mai scritto parzialmente: o tutti i suoi dati vengono salvati, o niente.
pub async fn index_block(
    alchemy: &alchemy::AlchemyClient,
    db_pool: &PgPool,
    block_number: i64,
) -> Result<Block, Box<dyn std::error::Error + Send + Sync>> {

    let block = alchemy.get_block(block_number).await?;

    let mut db_transazione = db_pool.begin().await?;
    db::save_block(&mut db_transazione, &block).await?;
    db_transazione.commit().await?;

    Ok(block)
}    