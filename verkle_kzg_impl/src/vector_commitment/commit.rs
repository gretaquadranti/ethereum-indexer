use super::types::{Value, VectorCommitment};
use super::interpolate::interpolate_lagrange;
use crate::kzg::{PublicKey, kzg_commit, kzg_open, kzg_verify};
use ark_bls12_381::{Fr, G1Projective};
use ark_ff::PrimeField;
use ark_serialize::CanonicalSerialize;
use sha2::{Sha256, Digest};
use ark_ec::CurveGroup;

// CONVERSIONI--------------------------------------------------------------------------------------------------------
// converto commitment in un value (48 bytes)
pub fn commitment_to_value(commitment: VectorCommitment) -> Value {

    let mut bytes = Vec::new();
    
    let point = commitment.inner;
    let p_affine = point.into_affine();

    p_affine.serialize_compressed(&mut bytes).expect("errore serializzazione");
    let mut result = [0u8; 48];
    
    //sposto i dai dall'heap allo stack
    result.copy_from_slice(&bytes); 
    
result
}


// compressione da 48 bytes (value) a 32 bytes (scalar) 
pub fn value_to_scalar(value: &Value) -> Fr {
    
    let mut hasher256 = Sha256::new();
    hasher256.update(value);  
    let valore_hashato = hasher256.finalize();  
    
    let mut bytes32 = [0u8; 32];
    bytes32.copy_from_slice(&valore_hashato);
    
    //il numero deve essere un elemento del campo finito r
    Fr::from_le_bytes_mod_order(&bytes32)
}
//-------------------------------------------------------------------------------------------------------

//CHIAMATE
// metodo x commitment a un vettore di 256 valori
pub fn commit_vector(values: &[Value; 256], pk: &PublicKey) -> VectorCommitment {
    
    let mut scalars = [Fr::from(0u64); 256];
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
) -> G1Projective {
    
    let mut scalars = [Fr::from(0u64); 256];
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
    witness: G1Projective,
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

