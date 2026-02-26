use super::types::{G1Point, PublicKey, Scalar};
use super::commit::kzg_commit;
use ark_ff::Field;

// calcolo witness
pub fn kzg_open(
    coefficients: &[Scalar],
    index: usize,
    pk: &PublicKey,
) ->  G1Point {

    let x0 = Scalar::from(index as u64);
    
    //y
    let f_in_x0 = evaluate_polynomial(coefficients, &x0);
    
    let quotient_coeffs = compute_quotient(coefficients, &x0, &f_in_x0);
    
    // commitment del quoziente - mi resituisce un G1point
    let w_commitment = kzg_commit(&quotient_coeffs, pk);
    
    w_commitment
}


// per calcolare il polinomio in x0/suffix
fn evaluate_polynomial(coefficients: &[Scalar], x: &Scalar) -> Scalar {
    if coefficients.is_empty() {
        return Scalar::ZERO;
    }
    
    let mut result = coefficients[coefficients.len() - 1];
    
    for i in (0..coefficients.len() - 1).rev() {
        result = result * x + coefficients[i];
    }
    result
}

//metodo per calcolare il quoziente usando ruffini :(
fn compute_quotient(
    coefficients: &[Scalar],
    x0: &Scalar,
    f_in_x0: &Scalar,
) -> Vec<Scalar> {
    
    if coefficients.len() == 0 {
        return vec![];
    }
    
    //vettore dove metto i coef del quoziente, il grado è sempre -1 rispetto al polinomio originale
    let mut quoziente = vec![Scalar::ZERO; coefficients.len() - 1];
    
    //costruisco il numeratore:  p(x) = f(x) - f(x0)
    let mut p = coefficients.to_vec();
    p[0] = coefficients[0] - f_in_x0;  
    
    let n = quoziente.len();
    //calcolo il quoziente usando ruffini
    quoziente[n - 1] = p[ p.len()-1];
    
    for i in (0.. p.len() - 2).rev() {
        quoziente[i] = p[i + 1] + quoziente[i + 1] * x0;
    }
    
    quoziente
}