use crate::kzg::PublicKey;
use crate::vector_commitment::{VectorCommitment,  commit_vector, prove_element, verify_element,  commitment_to_value};
use crate::kzg::{G1Point};
//la chiave è un vettore da 32 byte
pub type Key = [u8; 32];     
pub type Value = [u8; 48];  
pub type Stem = [u8; 31];    //primi 31 byte della chiave
pub type Suffix = u8;        //ultimo byte 


#[derive(Debug, Clone)]
//nodi interni, ogni nodo puo avere al max 256 figli
pub struct BranchNode {
    //array dei 256 figli
    pub children: [Option<NodeRef>; 256],
    //commitment al commitment dei figli
    pub commitment: Option<VectorCommitment>,
}


#[derive(Debug, Clone)]
//stem node, ovvero 31 byte uguali, conterrà al max 256 valori che condividono i primi 31 byte
pub struct StemNode {
    pub stem: Stem,
    pub values: [Option<Value>; 256],
    //commitment ai valori
    pub commitment: Option<VectorCommitment>,
}

#[derive(Debug, Clone)]
pub enum NodeRef {
    Branch(Box<BranchNode>),
    Stem(Box<StemNode>),
}

//------------------------------------------------------------
// metodi per le chiavi
pub fn get_stem(key: &Key) -> Stem {
    let mut stem = [0u8; 31];
    stem.copy_from_slice(&key[0..31]);
    stem
}

pub fn get_suffix(key: &Key) -> Suffix {
    key[31]
}

//------------------------------------------------------------
//BRANCH E STEM IMPL
impl BranchNode {
    pub fn new() -> Self {
        BranchNode {
            children: [const { None }; 256],
            commitment: None,  //perchè all'inizio il commitment non esiste ancora
        }
    }
    
    // per calcolare il commitment del branchNode
    pub fn compute_commitment(&mut self, pk: &PublicKey) {    
        
        let mut child_values = [[0u8; 48]; 256];
        
        for i in 0..256 {
            match &self.children[i] {
                None => {
                    //lascio lo zero che c'è dall'iniizializzazione
                }
                
                 Some(NodeRef::Stem(stem_node)) => {
                    //controllo se il figlio stemNode ha già il commitment
                    if let Some(commitment) = stem_node.commitment {
                        child_values[i] = commitment_to_value(commitment);
                    }
                }

                 Some(NodeRef::Branch(branch_node)) => {
                    if let Some(commitment) = branch_node.commitment {
                        child_values[i] = commitment_to_value(commitment);
                    }
                }
            }
        }
        self.commitment = Some(commit_vector(&child_values, pk));
    }
}



impl StemNode {

    pub fn new(stem: Stem) -> Self {
        StemNode {
            stem,
            values: [const { None }; 256],
            commitment: None, 
        }
    }

    // commitment per gli StemNode: fa commitment ai 256 valori
    pub fn compute_commitment(&mut self, pk: &PublicKey) { 
        let mut values_array = [[0u8; 48]; 256];
        
        for i in 0..256 {
            match self.values[i] {
                None => {}
                Some(value) => {
                    values_array[i] = value;
                }
            } 
        }       
        self.commitment = Some(commit_vector(&values_array, pk));  
    }
}

//------------------------------------------------------------
// VERKLE TREE impl

pub struct VerkleTree {
    root: BranchNode,
    pk: PublicKey,  
}

impl VerkleTree {

    pub fn new(pk: PublicKey) -> Self {
        VerkleTree {
            root: BranchNode::new(),
            pk,
        }
    }
    
    //getter
    pub fn getter_root(&self) -> &BranchNode {
        &self.root
    }

   //recupera il valore associato a una chiave che la passo in input, se non esiste ritorna none
    pub fn get(&self, key: &Key) -> Option<Value> {
      
        let stem = get_stem(key);
        let suffix = get_suffix(key);
        
        let mut current_node = &self.root;

        for &byte in stem.iter() {
            //controlla se c'è un figlio all'indice 'byte'
            match &current_node.children[byte as usize] {
                None =>{
                         return None;
                }
                Some(NodeRef::Branch(branch)) => {
                    //navigo al branch successivo
                    current_node = branch;
                }
                Some(NodeRef::Stem(stem_node)) => {
                    return stem_node.values[suffix as usize];
                }
            }
        }
        None
    }
    


//metodo per inserire una coppia chiave-valore nell'albero
    //ritorna il vecchio valore se la chiave esisteva già
    pub fn insert(&mut self, key: Key, value: Value) -> Option<Value> {
        let stem = get_stem(&key);
        let suffix = get_suffix(&key);

        let old_value = Self::insert_recursive(&mut self.root, &stem, 0, suffix, value);
        self.update_commitments_after_insert(&stem);
        
        old_value
    }
    
    
    fn insert_recursive(
    node: &mut BranchNode,
    stem: &Stem,
    level: usize,
    suffix: u8,
    value: Value,
) -> Option<Value> {
    let index = stem[level];
    let child_index = index as usize;

    match &mut node.children[child_index] {
        // se la posizione è vuota, bisogna creare il percorso
        None => {
            if level == 30 {
                let mut stem_node = StemNode::new(*stem);
                stem_node.values[suffix as usize] = Some(value);
                node.children[child_index] = Some(NodeRef::Stem(Box::new(stem_node)));
                None
            } else {
                let mut new_branch = Box::new(BranchNode::new());
                let old_value = Self::insert_recursive(
                    &mut new_branch,
                    stem,
                    level + 1,
                    suffix,
                    value,
                );
                node.children[child_index] = Some(NodeRef::Branch(new_branch));
                old_value
            }
        }

        // esiste già un BranchNode, scendo ricorsivamente
        Some(NodeRef::Branch(branch)) => {
            Self::insert_recursive(branch, stem, level + 1, suffix, value)
        }

        // esiste già uno StemNode, aggiorno il valore
        Some(NodeRef::Stem(stem_node)) => {
            let old_value = stem_node.values[suffix as usize];
            stem_node.values[suffix as usize] = Some(value);
            old_value
        }
    }
}
    
   fn update_commitments_after_insert(&mut self, stem: &Stem) {
      let pk = &self.pk;
        Self::update_commitments_recursive(&mut self.root, stem, 0, pk);
    }
    
    
    fn update_commitments_recursive(node: &mut BranchNode, stem: &Stem, level: usize,  pk: &PublicKey,) {

         if level >= 31 { //check
        return;  
    }
        let index = stem[level];
        let child_index = index as usize;
        
        match &mut node.children[child_index] {
           
            Some(NodeRef::Stem(stem_node)) => {
                stem_node.compute_commitment(pk);
            }
      
            Some(NodeRef::Branch(branch)) => {
            if level < 30 {
                Self::update_commitments_recursive(branch, stem, level + 1, pk);

                // FASE DI RISALITA: ora che tutti i nodi sottostanti sono stati aggiornati,
                // ricalcolo il commitment di questo nodo figlio specifico
                branch.compute_commitment(pk);}
            }
            None => {}
        }
        node.compute_commitment(pk);
    }
    
   
    // metodo chiamato x calcolare la prova e "inviare" <x,y,w> 
    pub fn prove(&self, key: &Key) -> Option<MembershipProof> {
        let stem = get_stem(key);
        let suffix = get_suffix(key);

        let mut current_node = &self.root;

        //scorro l'array dello stem
        for &byte in stem.iter() {
            let child_index = byte as usize;
            
            match &current_node.children[child_index] {
                None => return None, //se la chiave non esiste

                Some(NodeRef::Branch(branch)) => {
                    current_node = branch;
                }
                
                Some(NodeRef::Stem(stem_node)) => {
                    
                    //uso il suffisso per poter accedere a value in posizione values[suffix]
                    let value = stem_node.values[suffix as usize]?;
                   
                    let commitment = stem_node.commitment?;
                    
                    let mut values_array = [[0u8; 48]; 256];
                    for i in 0..256 {
                        if let Some(v) = stem_node.values[i] {
                        values_array[i] = v;
                        }
                    }
                  
                    let witness = prove_element(&values_array, suffix as usize, &self.pk);
                    
                    return Some(MembershipProof {
                        commitment,
                        index: suffix as usize,
                        value,
                        witness, 
                    });
                }
            }
        }
        None
    }

    // verifica della prova
    pub fn verify_proof(proof: &MembershipProof, pk: &PublicKey) -> bool {
        verify_element(
            proof.commitment,
            proof.index,
            proof.value,
            proof.witness.clone(),
            pk,
        )
    }
}



// struttura per la prova di membership
#[derive(Debug, Clone)]
pub struct MembershipProof {
    pub commitment: VectorCommitment,  // commitment del StemNode
    pub index: usize,                  // rappresenta x0
    pub value: Value,                  // y
    pub witness: G1Point,           // witness KZG 
}