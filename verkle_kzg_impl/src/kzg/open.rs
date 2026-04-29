use super::setup::PublicKey;
use super::commit::kzg_commit;
use ark_bls12_381::{Fr, G1Projective};
 
// calcolo witness
pub fn kzg_open(
    coefficients: &[Fr],
    index: usize,
    pk: &PublicKey,
) ->  G1Projective {

    //posizione
    let x0 = Fr::from(index as u64);
    
    //y
    let f_in_x0 = evaluate_polynomial(coefficients, &x0);
    
    let quotient_coeffs = compute_quotient(coefficients, &x0, &f_in_x0);
    
    // commitment del quoziente - mi resituisce un G1point
    let w_commitment = kzg_commit(&quotient_coeffs, pk);
    
    w_commitment
}


// per calcolare il polinomio in x0/suffix
fn evaluate_polynomial(coefficients: &[Fr], x: &Fr) -> Fr {
    
    if coefficients.is_empty() {
        return Fr::from(0u64);
    }
    
    let mut result = coefficients[coefficients.len() - 1];
    
    for i in (0..coefficients.len() - 1).rev() {
        result = result * x + coefficients[i];
    }
    result
}

// calcolare il quoziente usando ruffini 
fn compute_quotient(
    coefficients: &[Fr],
    x0: &Fr,
    f_in_x0: &Fr,
) -> Vec<Fr> {
    
    if coefficients.len() == 0 {
        return vec![];
    }
    
    //vettore dove metto i coef del quoziente, il grado è sempre -1 rispetto al polinomio originale
    let mut quoziente = vec![Fr::from(0u64); coefficients.len() - 1];
    
    //costruisco il numeratore:  p(x) = f(x) - f(x0)/y
    let mut p = coefficients.to_vec();
    p[0] = coefficients[0] - f_in_x0;  
    
    let n = quoziente.len();


    //calcolo il quoziente usando Horner
    quoziente[n - 1] = p[ p.len()-1];
    
    for i in (0.. p.len() - 2).rev() {
        quoziente[i] = p[i + 1] + quoziente[i + 1] * x0;
    }
    
    quoziente
}