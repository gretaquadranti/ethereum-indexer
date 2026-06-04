use ark_bls12_381::{Fr, G1Projective, G2Projective};
use ark_ec::Group;
use rand::thread_rng;
use ark_ff::UniformRand;

#[derive(Clone)]
    pub struct PublicKey {
    pub t: usize,
     // [g1, alpha·g1, α^2·g1, ..., α^max_degree·g1]
    pub g1_vector: Vec<G1Projective>,
    // [g2, alpha·g2]
    pub g2_vector: Vec<G2Projective>,
}

pub fn trusted_setup(t: usize) -> PublicKey  {
        //genero scalar è un numero nel campo finito F_p
        let mut random_number_generator = thread_rng(); 
        let s = Fr::rand(&mut random_number_generator);
        
        let g1 = G1Projective::generator();
        
        let g2 = G2Projective::generator();
        
        //creo un vettore della grandezza del massimo degree +1, perchè c'è da contare s^0 
        let mut g1_vector = Vec::with_capacity(t + 1);
        
        let mut current_pw = Fr::from(1u64);
        
        //inserisco ogni valore g1*valore segreto 
        for _ in 0..=t {
            g1_vector.push(g1 * current_pw);
            current_pw = current_pw * s;
        }
        
        let g2_vector = vec![
            g2,        
            (g2 * s)   
        ];

        //s non viene piu usato, quindi "distrutto"

        PublicKey {
        t, 
        g1_vector,
        g2_vector,
    }
        
    }