use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures_util::{StreamExt, SinkExt}; 
use serde_json::json;
use std::error::Error;
use std::pin::Pin;
use std::future::Future;
use reqwest::Client;
use crate::models::{JRPCRequest, JRPCResponse, Block};


pub struct AlchemyWebSocket {
    url: String,
}


impl AlchemyWebSocket {
    //costruttore
    pub fn new(api_key: String) -> Self {
        // uso wss x aprire un canale di comunicazione PERMANENTE 
        let url = format!("wss://eth-sepolia.g.alchemy.com/v2/{}", api_key);
        Self { url }
    }

    //metodo che serve per il 
    async fn connect_and_listen<F>(
    &self,
    callback: &mut F
) -> Result<(), Box<dyn Error + Send + Sync>>
where F: FnMut(String) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + 'static
{
    //apro una connessione webSocket con alchemy 
    let (ws_stream, _) = connect_async(&self.url).await?; 
    println!("connected to webSocket");
    
    //suddivisione dei canali di scrittura e lettura 
    let (mut write, mut read) = ws_stream.split();

    //richiesta di iscrivermi al feed che annuncia quando ci sono nuovi blocchi
    let iscrizione = crate::models::JRPCRequest {
        jsonrpc: "2.0".to_string(),
        id: 1,
        method: "eth_subscribe".to_string(),
        params: vec![serde_json::json!("newHeads")], 
    };

    let iscrizione_str = serde_json::to_string(&iscrizione)?;
    write.send(Message::Text(iscrizione_str)).await?;
    
    while let Some(mesg) = read.next().await {
        if mesg.is_err() {
            println!("network error: {:?}", mesg.err());
            continue; 
        }

        let messaggio = mesg.unwrap();

        // se messaggio è di tipo testo
        if messaggio.is_text() {
            let testo = messaggio.to_string();
            
            //converto la stringa in json
            let json_testo = serde_json::from_str::<serde_json::Value>(&testo);
            
            if json_testo.is_err() {
                continue;
            }

            let json_value = json_testo.unwrap();

            let params = json_value.get("params");
            if params.is_some() {
                
                let result = params.unwrap().get("result");
                
                if result.is_some() {
                    let number = result.unwrap().get("number");
                    if number.is_some() {
                        //ottengo il numero del blocco
                        let numero_hex = number.unwrap().as_str().unwrap();
                        callback(numero_hex.to_string()).await; //chiamo la callback
                    }
                }
            }
        }

        else if messaggio.is_ping() {
            let dati = messaggio.into_data();
            write.send(Message::Pong(dati)).await?;
        }

        else if messaggio.is_close() {
            println!("server ha chiuso la connessione");
            break;
        }
    }
    
    println!("disconnected");
    Ok(())
}


    //funzione che serve ad INIZIARE per poi mettersi in ascolto
    pub async fn subscribe_new_blocks<F>(
        &self, 
        mut callback: F 
    ) -> Result<(), Box<dyn Error + Send + Sync>> where  
        F: FnMut(String) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + 'static
    {
        
        loop {
            println!("connecting to webSocket...");

            let res = self.connect_and_listen(&mut callback).await;
            
            match res {
                Ok(()) => { println!("webSocket connection closed gracefully. Reconnecting..."); }
                Err(e) => {
                    eprintln!("webSocket error: {}", e);
                    println!("reconnecting in 5 sec...");

                    //metto in pausa 5 sec prima di ritentare di nuovo di connettere
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
        }
    }
}


//----------------------------------------------------------------------------------------------------------------
// per fare richieste "una tantum" --> serve per recuperare il passato
pub struct AlchemyClient {
    http_client: Client,
    url: String,
}

impl AlchemyClient {
    pub fn new(api_key: String) -> Self {
        let http_client = Client::new();
        let url = format!("https://eth-sepolia.g.alchemy.com/v2/{}", api_key);
        
        Self { http_client, url }
    }
    
   
    // per ottienere il numero dell'ultimo blocco
    pub async fn get_latest_block_number(&self) -> Result<i64, Box<dyn Error + Send + Sync>> {
       
        let request = JRPCRequest {
            jsonrpc: "2.0".to_string(),
            method: "eth_blockNumber".to_string(),  
            params: vec![],  
            id: 1,
        };
        
        //invio la richiesta HTTP POST
        let response = self.http_client
            .post(&self.url)
            .json(&request)
            .send()
            .await?;
        
        let result: JRPCResponse<String> = response.json().await?;
        //trasformo esadecimale
        let block_number = crate::utils::hex_to_i64(&result.result)?;
        Ok(block_number)
    }
    
    
    //ottengo un blocco specifico
    pub async fn get_block(&self, block_number: i64) -> Result<Block, Box<dyn Error  + Send + Sync>> {
        //converto il numero del blocco in numero esadecimale
        let block_hex = format!("0x{:x}", block_number);
        
        //prepraro la richiesta per ottnere QUEL blocco
        let request = JRPCRequest {
            jsonrpc: "2.0".to_string(),
            method: "eth_getBlockByNumber".to_string(),
            params: vec![
                json!(block_hex),  
                json!(true)
            ],
            id: 1,
        };
        
        //invio la richiesta
        let response = self.http_client
            .post(&self.url)
            .json(&request)
            .send()
            .await?;
        
        //ottengo la rispsota
        let result: JRPCResponse<Block> = response.json().await?;
        Ok(result.result)
    }
    
    
}
