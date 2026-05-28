use ark_bls12_381::G1Projective;
pub type Value = [u8; 48];

#[derive(Debug, Clone, Copy)]
pub struct VectorCommitment {
    pub inner: G1Projective, 
}
