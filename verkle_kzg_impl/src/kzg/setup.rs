use super::types::{PublicKey, Scalar, G1Point, G2Point};  
use ark_ff::Field;
use ark_ec::Group;
use rand::thread_rng;
use ark_ff::UniformRand;

pub fn trusted_setup(t: usize) -> PublicKey  {

        //genera numeri casuali. per generare s in modo "segreto"
        let mut casual = thread_rng();
        
        //scalar è un numero nel campo finito F_p 
        let s = Scalar::rand(&mut casual);
        
        //genero G1 che genera numeri che appartengono a curva1, curva ellittica
        let g1 = G1Point::generator();
        
        //genero G2, generatore di punti su curva2
        let g2 = G2Point::generator();
        
        //creo un vettore della grandezza del massimo degree +1, perchè c'è da contare s^0 
        let mut g1_vector = Vec::with_capacity(t + 1);
        
        let mut current_pw = Scalar::ONE;
        //inserisco ogni valore g1*valore segreto 
        for _ in 0..=t {
            g1_vector.push(g1 * current_pw);
            current_pw = current_pw * s;
        }
        
        // Calcola [1, s] in G2 (per KZG serve solo s^0 e s^1) perchè serve solo per verify
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