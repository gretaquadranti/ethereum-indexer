//funzione per trasformare esadecimale in i64 D
pub fn hex_to_i64(string_hex: &str) -> Result<i64, Box<dyn std::error::Error + Send + Sync>> {
    let no_prefix = string_hex.trim_start_matches("0x");

    Ok(i64::from_str_radix(no_prefix, 16)?)
}

// converte un numero di blocco in una chiave di 32 byte per il verkle tree
// i restanti 24 byte rimangono a zero
pub fn block_number_to_key(block_number: i64) -> [u8; 32] {
    let mut key = [0u8; 32];

    //to_be_bytes() converte l'intero a 64 bit in 8 byte in big endian
    let bytes = block_number.to_be_bytes(); 

    // copia quei 8 byte nei primi 8 posti dell'array 
    key[24..32].copy_from_slice(&bytes);
    
    key
}


pub fn hash_to_value(v: &str) -> [u8; 48] {

    let mut value = [0u8; 48];

    let no_prefix = v.trim_start_matches("0x");
    
    //hex::decode metodo che converte la stringa esadecimale in byte
    let bytes = hex::decode(no_prefix).unwrap_or_default();

    let len = bytes.len();
    value[0..len].copy_from_slice(&bytes[0..len]);
    
    value
}


#[cfg(test)] 
mod tests {
    use super::*;
    
    // ------------------------hex_to_i64---------------------------------------

    #[test] 
    fn test_hex_to_i64_con_prefisso() {
        // 0x10 = 16
        assert_eq!(hex_to_i64("0x10").unwrap(), 16); 
    }


    #[test]
    fn test_hex_to_i64_stringa_invalida() {
        assert!(hex_to_i64("xyz").is_err());
    }

  
    // -------------------------block_number_to_key--------------------------------------

    #[test]
    fn test_block_number_to_key_struttura() {
        let key = block_number_to_key(1);
       
        assert_eq!(&key[0..24], &[0u8; 24]);
       
        assert_eq!(&key[24..31], &[0u8; 7]);
    
        assert_eq!(key[31], 1);
    }

    #[test]
    fn test_block_number_to_key_lunghezza() { 
        let key = block_number_to_key(42);
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_block_number_to_key_diversi_blocchi() {
        let key1 = block_number_to_key(100);
        let key2 = block_number_to_key(200);
        assert_ne!(key1, key2);
    }

    // ----------------------- hash_to_value----------------------------------------

    #[test]
    fn test_hash_to_value_lunghezza() {
        let hash = "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
        let value = hash_to_value(hash);
        assert_eq!(value.len(), 48); 
    }

    #[test]
    fn test_hash_to_value_senza_prefisso() {
        let hash = "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
        let value = hash_to_value(hash);
        assert_eq!(value.len(), 48);
    }

    #[test]
    fn test_hash_to_value_diversi_hash() {
        let v1 = hash_to_value("0x0000000000000000000000000000000000000000000000000000000000000001");
        let v2 = hash_to_value("0x0000000000000000000000000000000000000000000000000000000000000002");
        assert_ne!(v1, v2);
    }
}