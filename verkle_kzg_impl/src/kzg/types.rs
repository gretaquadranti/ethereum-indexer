use ark_bls12_381::{Fr, G1Projective, G2Projective};

//tipo per punti  G1/G2
pub type G1Point = G1Projective; // è un tipo che indica un punto sulla curva ellittica formata da 3 coordinate
pub type G2Point = G2Projective;

// tipo per gli scalari (elementi del campo)
pub type Scalar = Fr;

#[derive(Debug, Clone)]
pub struct PublicKey {
    // grado max del polinomio 
    pub t: usize,

    // [g1, alpha·g1, α^2·g1, ..., α^max_degree·g1]
    //alpha è il segreto
    pub g1_vector: Vec<G1Point>,
    
    // [g2, alpha·g2]
    pub g2_vector: Vec<G2Point>,
}

