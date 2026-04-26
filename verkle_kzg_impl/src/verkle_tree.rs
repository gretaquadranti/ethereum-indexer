use crate::kzg::PublicKey;
use crate::vector_commitment::{VectorCommitment,  commit_vector, prove_element, verify_element,  commitment_to_value};
use ark_bls12_381::G1Projective;
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

    //recupero il commitment del tree
    pub fn get_root_commitment(&self) -> Option<VectorCommitment> {
    self.root.commitment
}



    pub fn getter_pk(&self) -> &PublicKey {
            &self.pk
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

        // raccoglie i nodi visitati durante la discesa + posizione
        let mut ls_nodi_visitati: Vec<(&BranchNode, usize)> = Vec::new();
        let mut current_node = &self.root;


        //scorro l'array dello stem
        for &indice in stem.iter() {
            //indica il numero del prossimo figlio da visitare
            let child_indice = indice as usize;

            //se il figlio nella posizione 'indice' di current node è vuoto:
            match &current_node.children[child_indice] {
                None => {
                println!("non esiste il percorso");
                return None;}

                //se il figlio del current node nella posizione 'indice' è di tipo branch:
                Some(NodeRef::Branch(branch)) => {
                    // salvo current node e l'indice 
                    ls_nodi_visitati.push((current_node, child_indice));
                    current_node = branch;
                }

                //se il figlio è uno stem node
                Some(NodeRef::Stem(stem_node)) => {
                    let value = stem_node.values[suffix as usize]?;
                    let commitment_stem = stem_node.commitment?;
                    
                    let mut values_array = [[0u8; 48]; 256];

                    //recupero tutti i valori dello stem
                    for i in 0..256 {
                        if let Some(v) = stem_node.values[i] {
                        values_array[i] = v;
                        }
                    }

                    //chiamo per la prova. in input: array dei valori e suffisso e pk
                    let witness_stem = prove_element(&values_array, suffix as usize, &self.pk);

                    let step_stem = ProofStep {
                    commitment: commitment_stem,
                    index: suffix as usize,
                    value,
                    witness: witness_stem,
                    };

                    //salvo le prove
                    let mut steps: Vec<ProofStep> = Vec::new();
                    steps.push(step_stem);
                   //-----------------------------------------

                    //recupero il commitment dello stemnode
                    let parent_commitment = current_node.commitment?;

                    let mut children_values_parent = [[0u8; 48]; 256];

                    //recupera i vari commitment, trasformandoli in value e li salvo nell'array  
                    for i in 0..256 {
                            match &current_node.children[i] {
                                None => {}
                                Some(NodeRef::Stem(s)) => {
                                    if let Some(c) = s.commitment {
                                        children_values_parent[i] = commitment_to_value(c);
                                    }
                                }
                                Some(NodeRef::Branch(b)) => {
                                    if let Some(c) = b.commitment {
                                        children_values_parent[i] = commitment_to_value(c);
                                    }
                                }
                            }
                        }
                        //calcola la prova del padre 
                        let witness_parent = prove_element(&children_values_parent, child_indice, &self.pk);
                        
                        steps.push(ProofStep {
                            commitment: parent_commitment,
                            index: child_indice,
                            value: commitment_to_value(commitment_stem),
                            witness: witness_parent,
                        });

                        
                        //salvo momentaneamente il commitment (trasformato in value) del current node
                        let mut child_commitment_as_value = commitment_to_value(parent_commitment);

                        //scorro la lista dei nodi visitati al contrario
                        for (branch_node, idx) in ls_nodi_visitati.iter().rev() {
                            //recupero il commitment
                            let branch_commitment = branch_node.commitment?;

                            //recupero ciascun commit e lo trasformo in value
                            let mut children_values = [[0u8; 48]; 256];
                            for i in 0..256 {
                                match &branch_node.children[i] {
                                    None => {}
                                    Some(NodeRef::Stem(s)) => {
                                        if let Some(c) = s.commitment {
                                            children_values[i] = commitment_to_value(c);
                                        }
                                    }
                                    Some(NodeRef::Branch(b)) => {
                                        if let Some(c) = b.commitment {
                                            children_values[i] = commitment_to_value(c);
                                        }
                                    }
                                }
                            }

                            // idx è la posizione del figlio (già noto) in questo branch_node, calcolo la prova
                            let witness = prove_element(&children_values, *idx, &self.pk);
                            
                            
                            steps.push(ProofStep {
                                commitment: branch_commitment,
                                index: *idx,
                                value: child_commitment_as_value,
                                witness,
                            });

                            child_commitment_as_value = commitment_to_value(branch_commitment);
                        }

                        return Some(MembershipProof { steps, value });
                                } 
                            }
                        }
                        None
                    }


    // verifica della prova
   pub fn verify_proof(proof: &MembershipProof, pk: &PublicKey, root: VectorCommitment) -> bool {
    for (i, step) in proof.steps.iter().enumerate() {
        let valid = verify_element(
            step.commitment,
            step.index,
            step.value,
            step.witness.clone(),
            pk,
        );

        if !valid {
            return false;
        }
    }

    // controllo anche che l'ultimo step deve avere il commitment uguale alla radice
    if let Some(last_step) = proof.steps.last() {
        return last_step.commitment.inner == root.inner;
    }
   
   false
}
}




// un singolo step della catena
#[derive(Debug, Clone)]
pub struct ProofStep {
    pub commitment: VectorCommitment,  // commitment del nodo a questo livello
    pub index: usize,                  // quale figlio/valore
    pub value: Value,                  // il valore che si sta provando a questo livello
    pub witness: G1Projective,         // proof KZG
}

// la proof completa
#[derive(Debug, Clone)]
pub struct MembershipProof {
    pub steps: Vec<ProofStep>,  // dal StemNode fino alla radice
    pub value: Value,           // il value finale (l'hash del blocco)
}