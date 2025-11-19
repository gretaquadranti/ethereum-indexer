//funzione per trasformare esadecimale in i64
pub fn hex_to_i64(hex: &str) -> Result<i64, Box<dyn std::error::Error + Send + Sync>> {
    let no_prefix = hex.trim_start_matches("0x");

    Ok(i64::from_str_radix(no_prefix, 16)?)
}

