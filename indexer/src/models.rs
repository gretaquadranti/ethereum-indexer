use serde::{Serialize, Deserialize};
use serde_json::Value;

#[derive(Serialize)]
pub struct JsonRPCRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Vec<Value>,
    pub id: i32,
}

#[derive(Deserialize, Debug)]
pub struct JsonRPCResponse<T> {  
    pub id: i32,
    pub jsonrpc: String,
    pub result: Option<T>,  
    #[serde(default)]
    pub error: Option<Value>, 
}

//struttura per quando mi arriva come risposta un blocco
#[derive(Debug, Deserialize)]
pub struct Block {
    pub number: String,
    pub hash: String,
    #[serde(rename = "parentHash")]
    pub parent_hash: String,
    pub timestamp: String,
    pub miner: String,
    #[serde(rename = "gasUsed")]
    pub gas_used: String,
    #[serde(rename = "gasLimit")]
    pub gas_limit: String,
    #[serde(default)] 
    pub transactions: Vec<Value>, 
    pub size: String,
}








