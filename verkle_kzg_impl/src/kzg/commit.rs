use super::types::{ PublicKey, Scalar, G1Point};
use ark_ec::VariableBaseMSM;
use ark_ec::CurveGroup;

// crea un commitment a un polinomio. restituisce quindi nel mio caso:
// - C = f(alpha)*G_1
// - w = q(alpha)*G_1
pub fn kzg_commit(coefficients: &[Scalar], pk: &PublicKey) -> G1Point {
    
    let bases_projective = &pk.g1_vector[0..coefficients.len()];

    //devo convertire da Projective(x,y,z) a affine(x,y) perchè lo richiede il metodo msm
    let mut bases_affine = Vec::new();

  
    for p in bases_projective {
        
    // trasformo il punto p da Proiettivo (X,Y,Z) ad Affine (x,y)
    let p_affine = p.into_affine();
    bases_affine.push(p_affine);
    }

    //la funzione msm prende i coef (di tipo Scalar), prende le g1*alpha (in base alle potenze) e crea il commitment 
 G1Point::msm(&bases_affine, coefficients)
        .expect("MSM fallito")
    
}