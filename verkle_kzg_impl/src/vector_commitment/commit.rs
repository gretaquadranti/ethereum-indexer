use super::types::{Value, VectorCommitment};
use super::interpolate::interpolate_lagrange;
use crate::kzg::{G1Point, PublicKey, Scalar, kzg_commit, kzg_open, kzg_verify};
use ark_ff::PrimeField;
use ark_serialize::CanonicalSerialize;
use ark_ff::Field;
use sha2::{Sha256, Digest};

// CONVERSIONI--------------------------------------------------------------------------------------------------------

// metodo per conversione da Value (48 bytes) a Scalar (32 byte)
pub fn value_to_scalar(value: &Value) -> Scalar {
    // uso sha per compressione dati
    let mut hasher = Sha256::new();
    hasher.update(value);  // input: 48 bytes
    let hash = hasher.finalize();  // output: 32 bytes
    
    let mut bytes32 = [0u8; 32];
    bytes32.copy_from_slice(&hash);
    
    // converte in scalar, quindi se il numero è piu grande del numero p,
    //la funzione fa automaticmante mod
    Scalar::from_le_bytes_mod_order(&bytes32)
}


// mmetodo x convertire un commitment in un value (48 bytes)
pub fn commitment_to_value(commitment: VectorCommitment) -> Value {
    // punto G1
    let point = commitment.inner;
    
    let mut bytes = Vec::new();
    
    //converte (X,Y,Z) in due coordinate e alla fine prende la X
    //scrive la X (0vvero i 48 byte) nell'heap
    point.serialize_compressed(&mut bytes).expect("errore");
    let mut result = [0u8; 48];
    
    //sposto i dai dall'heap allo stack
    result.copy_from_slice(&bytes); 
    
result
}



//-------------------------------------------------------------------------------------------------------

//CHIAMATE
// metodo x commitment a un vettore di 256 valori
pub fn commit_vector(values: &[Value; 256], pk: &PublicKey) -> VectorCommitment {
    
    let mut scalars = [Scalar::ZERO; 256];
    for i in 0..256 {
        scalars[i] = value_to_scalar(&values[i]);
    }
    
    //metodo per ottenere i coef del polinomio che passa per tutti i punti Scalar
    let coefficients = interpolate_lagrange(&scalars);
    
    let commitment = kzg_commit(&coefficients, pk);
    
    VectorCommitment { inner: commitment }
}


// metodo x costruire la witness 
pub fn prove_element(
    values: &[Value; 256],
    index: usize,
    pk: &PublicKey,
) -> G1Point {
    
    let mut scalars = [Scalar::ZERO; 256];
    for i in 0..256 {
        scalars[i] = value_to_scalar(&values[i]);
    }
    
    let coefficients = interpolate_lagrange(&scalars);
     
    let  witness = kzg_open(&coefficients, index, pk);
    
   witness
}

// per verificare che la prova sia corretta
pub fn verify_element(
    commitment: VectorCommitment,
    index: usize,
    value: Value,
    witness: G1Point,
    pk: &PublicKey,
) -> bool {

    let value_scalar = value_to_scalar(&value);

    kzg_verify(
        commitment.inner,
        index,
        value_scalar,
        witness,
        pk,
    )
}

