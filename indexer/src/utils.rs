//funzione per trasformare esadecimale in i64
pub fn hex_to_i64(hex: &str) -> Result<i64, Box<dyn std::error::Error + Send + Sync>> {
    let no_prefix = hex.trim_start_matches("0x");

    Ok(i64::from_str_radix(no_prefix, 16)?)
}

//block_number è un i64 — un intero a 64 bit, ovvero 8 byte.
pub fn block_number_to_key(block_number: i64) -> [u8; 32] {
    //creo la chiave 
    let mut key = [0u8; 32];
    //uso il metodo to_be_bytes che scrive il numero in un array usando la notazione
    //big endian, quindi i byte meno significativi a dx 
    //Prende il tuo numero e lo "spacchetta" in un array di byte nell'ordine big-endian


    //`to_be_bytes()` lo converte in un array di 8 byte in big-endian. 
    //Per esempio il numero `10618001` diventa: `[0, 0, 0, 0, 0, 161, 249, 193]`
    let bytes = block_number.to_be_bytes(); 

    // copia quei 8 byte nei primi 8 posti dell'array 
    key[0..8].copy_from_slice(&bytes);
    
    key
}

//metodo che prende la stringa e lo trasforma in tipo Value

//ma value potrebbe essere anche una stringa o deve solamente essere un numero?
pub fn hash_to_value(v: &str) -> [u8; 48] {

    let mut value = [0u8; 48];

    //viene tolto //0x che serve per rappresentare gli esadecimali
    let no_prefix = v.trim_start_matches("0x");
    //hex::decode metodo che prende rende la stringa pulita e la trasforma in una lista di byte (Vec<u8>
    let bytes = hex::decode(no_prefix).unwrap_or_default();

    let len = bytes.len();
    //ci aggiungo i restanti 0
    value[0..len].copy_from_slice(&bytes[0..len]);
    value
}