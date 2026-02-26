use crate::kzg::Scalar;
use ark_ff::Field;

//lagrange: serve per ottenere un polinomio che passa dai punti all'interno del vettore di valori 'children'
pub fn interpolate_lagrange(values: &[Scalar; 256]) -> Vec<Scalar> {
    
    let n = values.len();
    let mut coefficients = vec![Scalar::ZERO; n];
    
    for i in 0..n {
        let lagrange_poly = compute_lagrange_basis(i, n);
        
        //moltiplicazione tra i coef di un unico polinomio base * y
        for j in 0..n {
            coefficients[j] += lagrange_poly[j] * values[i];
        }
    }
    coefficients
}



// crea il polinomio base L_i(x)
fn compute_lagrange_basis(i: usize, n: usize) -> Vec<Scalar> {

    let mut poly = vec![Scalar::ONE]; // INIZIA DA "1"

    for j in 0..n {
        if j != i {
            let a = Scalar::from(j as u64);
            // poly = poly * (x - j)
            poly = multiply_poly_by_linear(&poly, a); //poly viene aggiornato
            // primo ciclo 1 * (x - 1)
            // secondo ciclo (x - 1) * (x - 2)
        }
    }
    
    //faccio (i-j), qundi scorro j
   	let mut denominator = Scalar::ONE;
    for j in 0..n {
            if j != i {
            let i_scalar = Scalar::from(i as u64);
            let j_scalar = Scalar::from(j as u64);
                denominator *= i_scalar - j_scalar; //viene aggiornato 
        }
    }
    
    // eseguo la divisione tra num e denominatore 
    let inv = denominator.inverse().expect("err");
    for j in 0..n {
        poly[j]*=inv
    }

    poly
}


// poly = [1]       -->  (1)
// poly = [-1, 1]   -->  (1x - 1)
// a è l'indice
fn multiply_poly_by_linear(poly: &[Scalar], a: Scalar) -> Vec<Scalar> {
    let n = poly.len();
    let mut result = vec![Scalar::ZERO; n + 1]; 
    
    //  moltiplico per x
    for i in 0..n {
        result[i + 1] += poly[i];
    }

    //(9x^3 + 2x^2 - 3x -5) -> poly = [-5, -3, 2, 9] -> result = [0, -5, -3, 2, 9]

    //  moltiplico per -a
    for i in 0..n {
        result[i] =  result[i] - poly[i] * a;
    }

    result
}
 