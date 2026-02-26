mod kzg;
mod vector_commitment;
mod verkle_tree;
use kzg::trusted_setup;
use verkle_tree::*;

 
fn main() {
    
    // setup pk kzg
    let pk = trusted_setup(255);

   //creo tree
    let mut tree = VerkleTree::new(pk.clone());

    println!("TEST: inserimento base");
    let mut key1 = [0u8; 32];
    key1[30] = 1;  
    key1[31] = 5;   
    let mut value1 = [0u8; 48]; 
    value1[0] = 42;  
    
    let old = tree.insert(key1, value1);
    if old.is_none(){
        println!("primo inserimento, quindi non c'è vecchio valore");}

    println!("TEST: recupero valore passando chiave");
    let valore = tree.get(&key1).expect("");
    let res=valore[0];
    if res==value1[0]{
        println!("il valore recuperato è corretto");
    }
   
    //provo a recuperare una valore passandogli una chiave inesistente
     println!("TEST: recupero valore passando chiave inesistente");
    let mut chiave_miss = [0u8; 32];
    chiave_miss[30] = 99;
    chiave_miss[31] = 99;
    
    let result = tree.get(&chiave_miss);
    if result.is_none(){
        println!("non eiste la chiave --> quindi corretto")
    }


    println!("TEST: inserisco nello stesso Stem, siffisso diverso");
    let mut key2 = [0u8; 32];
    key2[30] = 1;   
    key2[31] = 10;  
    
    let mut value2 = [0u8; 48];
    value2[0] = 99;
    
    println!("inserisco: stem[30]=1, suffix=10, value[0]=99");
    tree.insert(key2, value2);


    println!("TEST: sovrascrittura");
    let mut value1_nuovo = [0u8; 48];
    value1_nuovo[0] = 200;
    let old = tree.insert(key1, value1_nuovo);
    println!( "vecchio valore deve essere 42:{}", old.unwrap()[0],);
    print!("nuovo valore inserito dovrebbe essere 200:{}",tree.get(&key1).unwrap()[0]);

    
    //compute proof
    println!("TEST: chiamo la prova");
    
    println!("genero proof per key1(valore=42)");
    let proof1 = tree.prove(&key1).expect("");
    
    
    //  VERIFICA della PROOF
    println!("TEST: verifier fa la controproba");
    let verify_prove = VerkleTree::verify_proof(&proof1, &pk);
    println!("verifica prova:{}", verify_prove);


     // TEST: verifica con y falso 
    println!("TEST: modifico manualmente y");
    let mut proof_falsa = proof1.clone();
    //sto creando una nuova y, diversa da quella usata per calcolare la prova
    let mut value_falso = [0u8; 48];
    value_falso[0] = 99; 
    proof_falsa.value = value_falso; //ora al posto di 200, viene messo 99

    let falsa = VerkleTree::verify_proof(&proof_falsa, &pk);
    println!("verify proof con valore falso: {}", falsa);

    //TEST: verifica con index sbagliato
    println!("TEST: verifica con index sbagliato");
    let mut proof_index_sbagliato = proof1.clone();
    proof_index_sbagliato.index = 99; // index sbagliato

    let index_falso = VerkleTree::verify_proof(&proof_index_sbagliato, &pk);
    println!("verify proof con index sbagliato: {}", index_falso);

    // TEST: verifica con witness cambiato
    println!("TEST: verifica con witness manomesso");
    let proof2 = tree.prove(&key2).expect("");
    let mut proof_witness_sbagliato = proof1.clone();
    proof_witness_sbagliato.witness = proof2.witness; // witness di un'altra prova

    let witness_falso = VerkleTree::verify_proof(&proof_witness_sbagliato, &pk);
    println!("verify proof con witness manomesso: {}", witness_falso);
   
}