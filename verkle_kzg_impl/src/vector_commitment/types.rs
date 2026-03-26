use ark_bls12_381::G1Projective;
// questo rappresenta i dati/valori all'interno dello stem node (coppia chiiave-valore) 
pub type Value = [u8; 48];

// commitment del vettore di un nodo 
#[derive(Debug, Clone, Copy)]
pub struct VectorCommitment {
    pub inner: G1Projective, 
}
