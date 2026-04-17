mod models;
mod db;
mod alchemy;
mod utils;
use std::io::{self, Write};
use verkle_project::{VerkleTree, kzg::trusted_setup};

use dotenv::dotenv;
use std::env;
use std::sync::Arc;
use sqlx::PgPool;

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

    //connessione al database
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

    // inizializzo i verkle tree
    let pk = trusted_setup(255);
    let mut tree = VerkleTree::new(pk);
    println!("verkle tree initialized");
//----------------------------RECUPERA I BLOCCHI-------------------------------
    
    //leggi i blocchi dal db e inseriscili nel tree
    println!("caricamento blocchi dal database...");
    //aggiungere tempo


    let blocks = db::get_all_blocks(&db_pool).await?;

    for (numero, hash) in &blocks {
        let key = utils::block_number_to_key(*numero);
        let value = utils::hash_to_value(hash);
        tree.insert(key, value);
    }

//------------------------------------TEST-----------------------------------------
    
    println!("blocchi disponibili: da {} a {}", 
    blocks.first().unwrap().0, 
    blocks.last().unwrap().0
);


print!("inserisci il numero del blocco: ");
io::stdout().flush(); // per obbligare prima la stampa "inserisci il numero..."
//e poi una volta stampato, permetto all'utente di scrivere

let mut input = String::new();
io::stdin().read_line(&mut input);

while input != "end" {
    //trim cancella qualsiasi tipo di spazio, parse perchè trasfomra la stringa in un i64
        let blocco_input: i64 = input.trim().parse().expect("inserisci un numero valido");

        //il numero che dovrebbe rappresentare il blocco, viene trasformato in tipo Key
        //quindi viene scritto in byte e aggiunti 0
        let key = utils::block_number_to_key(blocco_input);

        //da printare la commit del verkle tree
       
        //viene preparata la prova
        let proof = tree.prove(&key);

        match proof {
            Some(p) => {
                //chiamo la verifica della prova
                let verifica = VerkleTree::verify_proof(&p, tree.getter_pk());
                println!("blocco: {}", blocco_input);
                //p value è da 48 byte e deve essere trasformato in esadecimale
                println!("hash: 0x{}", hex::encode(&p.value[0..32]));
                println!("proof valida: {}", verifica);
            
                //cambio il valore di VALUE, cosi in teoria quando faccio la prova ottengo il resto
                //e quindi quando poi faccio il verify dovrebbe restituire FALSE perchè i due lati nn
                //sn uguali
                let mut prova_manomessa = p.clone();
                prova_manomessa.steps[0].value = [0u8; 48];
                
                let falsa = VerkleTree::verify_proof(&prova_manomessa, tree.getter_pk());
                println!("prova manomessa: {}", falsa);
                }
            None => {
                println!("blocco {} non trovato nel tree", blocco_input);
                }
        }

        //repeat 
        print!("inserisci il numero del blocco: ");
        io::stdout().flush().unwrap(); 

        input.clear();
        io::stdin().read_line(&mut input).unwrap();

    }
    
    Ok(())
            
}