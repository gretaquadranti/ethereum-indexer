use super::setup::PublicKey;
use ark_bls12_381::{Fr, G1Projective};
use ark_ec::{VariableBaseMSM,CurveGroup};


// crea un commitment a un polinomio. restituisce quindi nel mio caso:
// - C = f(alpha)*G_1
// - w = q(alpha)*G_1
pub fn kzg_commit(coefficients: &[Fr], pk: &PublicKey) -> G1Projective {   

    let punti_projective = &pk.g1_vector[0..coefficients.len()];

    let mut vec_punti_affine = Vec::new();

    for p in punti_projective {
        let p_affine = p.into_affine();
        vec_punti_affine.push(p_affine);
    }

    //la funzione msm prende i coef (di tipo Scalar), prende le g1*alpha (in base alle potenze) e crea il commitment 
    G1Projective::msm(&vec_punti_affine, coefficients).expect("MSM fallito")
}