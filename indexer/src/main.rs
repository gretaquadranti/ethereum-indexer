mod models;
mod db;
mod alchemy;
mod utils;

use std::time::Instant;
use dotenv::dotenv;
use std::env;
use std::time::Duration;
use std::sync::Arc;
use std::io::{self, Write};
use sqlx::PgPool;
use std::pin::Pin;
use tokio::sync::Mutex;
use verkle_project::{VerkleTree, kzg::trusted_setup};


//senno va in stackoverflow
fn main() {
    let stack_size = 64 * 1024 * 1024; // 64MB
    let builder = std::thread::Builder::new().stack_size(stack_size);
    let handler = builder.spawn(|| {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async_main())
            .unwrap();
    }).unwrap();
    handler.join().unwrap();
}

async fn async_main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenv().ok();

    // db setup
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

    // inizializzo il verkle tree
    let pk = trusted_setup(255);
    let tree = Arc::new(Mutex::new(VerkleTree::new(pk)));
    println!("verkle tree initialized");

    // setup AlchemyClient
    let api_key = env::var("ALCHEMY_API_KEY").expect("ALCHEMY_API_KEY mancante");
    let alchemy_http = Arc::new(alchemy::AlchemyClient::new(api_key.clone()));

    // catch-up
    let last_indexed = db::get_last_indexed_block(&db_pool).await?;
    let latest_on_chain = alchemy_http.get_latest_block_number().await?;
    println!("last indexed: {}", last_indexed);
    println!("latest on chain: {}", latest_on_chain);

    let gap = latest_on_chain - last_indexed;
    if gap > 0 {
        println!("gap detected: {} blocks", gap);
        for block_num in (last_indexed + 1)..=latest_on_chain {
            let res = index_block(&alchemy_http, &db_pool, block_num).await;
            if let Ok(_) = res {
                db::update_last_indexed_block(&db_pool, block_num).await?;
            } else if let Err(e) = res {
                eprintln!("error indexing block {}: {}. skipping.", block_num, e);
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
        println!("catch-up complete");
    } else {
        println!("already up to date");
    }

    // carico tutti i blocchi dal DB nel tree
    println!("caricamento blocchi nel verkle tree...");
    let inizio = Instant::now();
    {
        let blocks = db::get_all_blocks(&db_pool).await?;
        let mut tree_lock = tree.lock().await;
        for (numero, hash) in &blocks {
            let key = utils::block_number_to_key(*numero);
            let value = utils::hash_to_value(hash);
            tree_lock.insert(key, value);
        }

        let durata = inizio.elapsed();
        println!("caricati {} blocchi in {:.2?}", blocks.len(), durata);
        println!("caricati {} blocchi nel tree", blocks.len());

        // stampo la radice
        if let Some(root) = tree_lock.get_root_commitment() {
            
            println!("RADICE DEL TREE: {:?}", root);
            println!("conserva questo valore per verificare le proof!");
           
        }
    } // il lock viene rilasciato qui
//---------------------------------------------------------------------------
    // clono per i due task
    let tree_ws = Arc::clone(&tree);
    let tree_terminale = Arc::clone(&tree);
    let alchemy_clone = Arc::clone(&alchemy_http);
    let db_clone = Arc::clone(&db_pool);

      // task 1: WebSocket in background - silenzioso
    let ws_task = tokio::spawn(async move {
    let ws = alchemy::AlchemyWebSocket::new(api_key);


        let callback = move |block_hex: String| -> Pin<Box<dyn std::future::Future<Output = ()> + Send + 'static>> {
            let alchemy = Arc::clone(&alchemy_clone);
            let db = Arc::clone(&db_clone);
            let tree = Arc::clone(&tree_ws);

            Box::pin(async move {
                let result = utils::hex_to_i64(&block_hex);

                if let Ok(block_num) = result {
                    eprintln!("new block: {}", block_num);

                    let last_result = db::get_last_indexed_block(&db).await;

                    if let Ok(last_indexed) = last_result {
                        if block_num > last_indexed {
                            for num in (last_indexed + 1)..=block_num {
                                let add_block = index_block(&alchemy, &db, num).await;

                                if let Ok(_) = add_block {
                                    let update_db = db::update_last_indexed_block(&db, num).await;

                                    if let Ok(_) = update_db {
                                        // inserisco nel tree
                                        let block = alchemy.get_block(num).await;
                                        if let Ok(b) = block {
                                            let key = utils::block_number_to_key(num);
                                            let value = utils::hash_to_value(&b.hash);
                                            let mut tree_lock = tree.lock().await;
                                            tree_lock.insert(key, value);
                                            eprintln!("block {} indexed e inserito nel tree", num);
                                        }
                                    } else if let Err(e) = update_db {
                                        eprintln!("error updating state: {}", e);
                                    }
                                } else if let Err(e) = add_block {
                                    eprintln!("error indexing block: {}", e);
                                }
                            }
                        } else {
                            eprintln!("block {} already indexed", block_num);
                        }
                    } else if let Err(e) = last_result {
                        eprintln!("error getting last indexed: {}", e);
                    }
                } else if let Err(e) = result {
                    eprintln!("error parsing block number: {}", e);
                }
            })
        };

        ws.subscribe_new_blocks(callback).await.unwrap();
    });

     // task 2: terminale - risponde alle richieste di proof
    let terminale_task = tokio::spawn(async move {
        loop {
            print!("inserisci il numero del blocco (o 'end' per uscire): ");
            io::stdout().flush().unwrap();

            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            let input = input.trim();

            if input == "end" {
                break;
            }

            let blocco_input: i64 = match input.parse() {
                Ok(n) => n,
                Err(_) => {
                    println!("inserisci un numero valido");
                    continue;
                }
            };

            let key = utils::block_number_to_key(blocco_input);
            let tree_lock = tree_terminale.lock().await;

            let root = match tree_lock.get_root_commitment() {
                Some(r) => r,
                None => {
                    println!("tree vuoto");
                    continue;
                }
            };

            let proof = tree_lock.prove(&key);

            match proof {
                Some(p) => {
                    let verifica = VerkleTree::verify_proof(&p, tree_lock.getter_pk(), root);
                    println!("blocco:       {}", blocco_input);
                    println!("hash:         0x{}", hex::encode(&p.value[0..32]));
                    println!("proof valida: {}", verifica);

                    // test manomissione
                    let mut prova_manomessa = p.clone();
                    prova_manomessa.steps[0].value = [0u8; 48];
                    let falsa = VerkleTree::verify_proof(&prova_manomessa, tree_lock.getter_pk(), root);
                    println!("prova manomessa: {}", falsa);
                }
                None => {
                    println!("blocco {} non trovato nel tree", blocco_input);
                }
            }
        }
    });

    // aspetto entrambi i task
    tokio::select! {
        _ = ws_task => { println!("ws task terminato"); }
        _ = terminale_task => { println!("terminale task terminato"); }
    }

    Ok(())
}

async fn index_block(
    alchemy: &alchemy::AlchemyClient,
    db_pool: &PgPool,
    block_number: i64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let block = alchemy.get_block(block_number).await?;
    let mut db_transazione = db_pool.begin().await?;
    db::save_block(&mut db_transazione, &block).await?;
    db_transazione.commit().await?;
    Ok(())
}