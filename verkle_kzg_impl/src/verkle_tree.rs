use crate::kzg::PublicKey;
use crate::vector_commitment::{VectorCommitment,  commit_vector, prove_element, verify_element,  commitment_to_value};
use ark_bls12_381::G1Projective;
use ark_serialize::CanonicalDeserialize;
use ark_bls12_381::G1Affine;
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
    pub commitment: Option<VectorCommitment>,
}

#[derive(Debug, Clone)]
pub enum NodeRef {
    Branch(Box<BranchNode>),
    Stem(Box<StemNode>),
}

pub struct ModifyNode {
    pub path: String,
    pub node_type: String,  
    pub commitment_bytes: [u8; 48],
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
            commitment: None,  
        }
    }
    
    // per calcolare il commitment del branchNode
    pub fn compute_commitment(&mut self, pk: &PublicKey)-> VectorCommitment {    
        
        let mut child_values = [[0u8; 48]; 256];
        
        for i in 0..256 {
            match &self.children[i] {
                None => {
                    //lascio lo zero che c'è dall'iniizializzazione
                }
                
                 Some(NodeRef::Stem(stem_node)) => {
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
        let commitment = commit_vector(&child_values, pk);  
        
        self.commitment = Some(commitment); 
        commitment
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
    pub fn compute_commitment(&mut self, pk: &PublicKey) -> VectorCommitment { 
        let mut values_array = [[0u8; 48]; 256];
        
        for i in 0..256 {
            match self.values[i] {
                None => {}
                Some(value) => {
                    values_array[i] = value;
                }
            } 
        }       
        let commitment = commit_vector(&values_array, pk);  

        self.commitment= Some(commitment);
        commitment
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

    fn deserialize_commitment(bytes: &[u8]) -> VectorCommitment {
        let affine = G1Affine::deserialize_compressed(bytes).unwrap();
        VectorCommitment {
            inner: affine.into(),
        }
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
    pub fn insert(&mut self, key: Key, value: Value, update_commitment:bool) -> Vec<ModifyNode> {
        let stem = get_stem(&key);
        let suffix = get_suffix(&key);

        Self::insert_recursive(&mut self.root, &stem, 0, suffix, value);

        let mut vecModNode= Vec::new();
        if update_commitment {
           vecModNode = self.update_commitments_after_insert(&stem);
        }
        vecModNode
    }
    
    
    fn insert_recursive(
    node: &mut BranchNode,
    stem: &Stem,
    level: usize,
    suffix: u8,
    value: Value,
    ) {
        let index = stem[level];
        let child_index = index as usize;

        match &mut node.children[child_index] {
        // se la posizione è vuota, bisogna creare il percorso
            None => {
                if level == 30 {
                    let mut stem_node = StemNode::new(*stem);
                    stem_node.values[suffix as usize] = Some(value);
                    node.children[child_index] = Some(NodeRef::Stem(Box::new(stem_node)));
                } else {
                    let mut new_branch = Box::new(BranchNode::new());
                    Self::insert_recursive(
                        &mut new_branch,
                        stem,
                        level + 1,
                        suffix,
                        value,
                    );
                    node.children[child_index] = Some(NodeRef::Branch(new_branch));
                }
            }

            // esiste già un BranchNode, scendo ricorsivamente
            Some(NodeRef::Branch(branch)) => {
                Self::insert_recursive(branch, stem, level + 1, suffix, value)
            }

            // esiste già uno StemNode, aggiorno il valore
            Some(NodeRef::Stem(stem_node)) => {
                stem_node.values[suffix as usize] = Some(value);
            }
        }
    }
    
   fn update_commitments_after_insert(&mut self, stem: &Stem)-> Vec<ModifyNode>{
        let mut vecModifyNodes = Vec::new();
        let pk = &self.pk;
        Self::update_commitments_recursive(&mut self.root, stem, 0, pk, &mut vecModifyNodes, String::new());
        vecModifyNodes
    }
    
    
    fn update_commitments_recursive(node: &mut BranchNode, stem: &Stem, level: usize,  pk: &PublicKey  , modNodes: &mut Vec<ModifyNode>, path: String) {

        if level >= 31 { //check
            return;  
        }
        let index = stem[level];
        let child_index = index as usize;

        let percorso=  format!("{}{:02x}", path, index);

        
        match &mut node.children[child_index] {
           
            Some(NodeRef::Stem(stem_node)) => {
                let commit_s=stem_node.compute_commitment(pk);

                modNodes.push( ModifyNode { 
                        path: percorso.clone(),
                        node_type: "stem".to_string(), 
                        commitment_bytes: commitment_to_value(commit_s),  // devo trasformare il commitment in 48 byte perchè il db non accetta un g1porjective
                        });
            }
      
            Some(NodeRef::Branch(branch)) => {
                if level < 30 {
                    let percorso_clone = percorso.clone();

                    Self::update_commitments_recursive(branch, stem, level + 1, pk, modNodes, percorso );

                    // FASE DI RISALITA: ora che tutti i nodi sottostanti sono stati aggiornati,
                    // ricalcolo il commitment di questo nodo figlio specifico
                    let commit_b= branch.compute_commitment(pk);
                    modNodes.push( ModifyNode { 
                        path: percorso_clone,
                        node_type: "branch".to_string(), 
                        commitment_bytes: commitment_to_value(commit_b)  
                    });       
                }
            }
            None => {}
        }

        let commit_r= node.compute_commitment(pk);

        if level == 0 {
            modNodes.push(ModifyNode {
                path,  // stringa vuota = root
                node_type: "root".to_string(),
                commitment_bytes: commitment_to_value(commit_r),
            });
        }
    }




    pub fn ricalcola_tutto(&mut self) {
        let pk = &self.pk;
        Self::ricalcola_ricorsivo(&mut self.root, pk);
    }

    fn ricalcola_ricorsivo(
        node: &mut BranchNode,
        pk: &PublicKey,
    ) {
        for i in 0..256 {
            match &mut node.children[i] {
                Some(NodeRef::Stem(stem_node)) => {
                    stem_node.compute_commitment(pk);
                }
                Some(NodeRef::Branch(branch)) => {
                    Self::ricalcola_ricorsivo(branch, pk);
                    branch.compute_commitment(pk);
                }
                None => {}
            }
        }
        node.compute_commitment(pk);
    }

    
    pub fn load_commitments_from_db(
    &mut self,
    nodes: Vec<(String, String, Vec<u8>)>,
    ) {
        for (path, _node_type, bytes) in nodes {
            let commitment = Self::deserialize_commitment(&bytes);
            self.set_commitment_at_path(&path, commitment);
        }
    }


    fn set_commitment_at_path(&mut self, path: &str, commitment: VectorCommitment) {
        // path vuoto = root
        if path.is_empty() {
            self.root.commitment = Some(commitment);
            return;
        }

    
        let indices: Vec<usize> = (0..path.len())
            .step_by(2)
            .map(|i| usize::from_str_radix(&path[i..i+2], 16).unwrap())
            .collect();

        let mut current = &mut self.root;
    
        for (depth, &idx) in indices.iter().enumerate() {
            let is_last = depth == indices.len() - 1;

            if is_last {
                match &mut current.children[idx] {
                    Some(NodeRef::Branch(branch)) => {
                        branch.commitment = Some(commitment);
                    }
                    Some(NodeRef::Stem(stem)) => {
                        stem.commitment = Some(commitment);
                    }
                    None => {}
                }
                return;
            }

            // scendi al prossimo livello
            match &mut current.children[idx] {
                Some(NodeRef::Branch(branch)) => {
                    current = branch;
                }
                _ => return,
            }
        }
    }
   
    // metodo chiamato x calcolare la prova e "inviare" <x,y,w> 
    pub fn prove(&self, key: &Key) -> Option<MembershipProof> {
        let stem = get_stem(key);
        let suffix = get_suffix(key);

        // raccoglie i nodi visitati durante la discesa + posizione
        let mut ls_nodi_visitati: Vec<(&BranchNode, usize)> = Vec::new();
        let mut steps: Vec<ProofStep> = Vec::new();
        let mut current_node = &self.root;


        //scorro l'array dello stem
        for &indice in stem.iter() {
            let child_indice = indice as usize;

            //se il figlio nella posizione 'indice' di current node è vuoto:
            match &current_node.children[child_indice] {
                None => {
                    println!("non esiste il percorso");
                return None;
                }

                Some(NodeRef::Branch(branch)) => {
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

                    steps.push(ProofStep {
                        commitment: commitment_stem, 
                        index: suffix as usize, 
                        value, 
                        witness: witness_stem, 
                    });

                    //recupero il commitment dello stemnode
                    let parent_commitment = current_node.commitment?;

                    let mut children_values_parent = [[0u8; 48]; 256];
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
                        
                    let witness_parent = prove_element(&children_values_parent, child_indice, &self.pk);
                        
                    steps.push(ProofStep {
                        commitment: parent_commitment,
                        index: child_indice,
                        value: commitment_to_value(commitment_stem),
                        witness: witness_parent,
                    });
                    
                    let mut child_commitment_as_value = commitment_to_value(parent_commitment);

                        //scorro la lista dei nodi visitati al contrario
                    for (branch_node, idx) in ls_nodi_visitati.iter().rev() {
                          
                        let branch_commitment = branch_node.commitment?;
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

                    return Some(MembershipProof { steps, value, key:*key });
                } 
            }
        }
        None
    }


    // verifica della prova
   pub fn verify_proof(proof: &MembershipProof, pk: &PublicKey, root: VectorCommitment,  expected_key: Key) -> bool {
    
        if proof.key.as_ref() != expected_key.as_ref() { 
            return false;
        }

        if proof.steps.is_empty(){
            return false;
        }

        let last_step= proof.steps.last().unwrap();
        if last_step.commitment.inner!= root.inner{
            return false;
        }

        let first_step= proof.steps.first().unwrap();
        if proof.value != first_step.value{
            return false;
        }

        for i in 1..proof.steps.len(){
            let current_step= &proof.steps[i];
            let prev_step = &proof.steps[i-1];
            let prev_commitment_bytes = commitment_to_value(prev_step.commitment);
            
            if current_step.value !=prev_commitment_bytes{
                return false; 
            }
        }
        for step in proof.steps.iter() {
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
        true
    }



        
    #[cfg(test)]
    pub fn get_all_commitments(&mut self) -> Vec<[u8; 48]> {
        let mut commitments = Vec::new();
        Self::collect_commitments(&mut self.root, &mut commitments);
        commitments
    }


#[cfg(test)]
fn collect_commitments(node: &BranchNode, commitments: &mut Vec<[u8; 48]>) {
    for i in 0..256 {
        match &node.children[i] {
            Some(NodeRef::Stem(stem_node)) => {
                if let Some(c) = stem_node.commitment {
                    commitments.push(commitment_to_value(c));
                }
            }
            Some(NodeRef::Branch(branch)) => {
                Self::collect_commitments(branch, commitments);
                if let Some(c) = branch.commitment {
                    commitments.push(commitment_to_value(c));
                }
            }
            None => {}
        }
    }
    if let Some(c) = node.commitment {
        commitments.push(commitment_to_value(c));
    }
}
}



#[derive(Debug, Clone)]
pub struct ProofStep {
    pub commitment: VectorCommitment, 
    pub index: usize,                 
    pub value: Value,                  
    pub witness: G1Projective,       
}

// la proof completa
#[derive(Debug, Clone)]
pub struct MembershipProof {
    pub steps: Vec<ProofStep>,  
    pub value: Value,  
    pub key: Key
            
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::kzg::trusted_setup;

    
    fn make_key(n: i64) -> Key {
        let mut key = [0u8; 32];
        key[24..32].copy_from_slice(&n.to_be_bytes());
        key
    }

    fn make_value(hex: &str) -> Value {
        let mut value = [0u8; 48];
        let bytes = hex::decode(hex).unwrap();
        value[0..bytes.len()].copy_from_slice(&bytes);
        value
    }


    // ---------------------------INSERT e GET------------------------------------

    #[test]
    fn test_inserimento_e_recupero() {
        let pk = trusted_setup(255);
        let mut tree = VerkleTree::new(pk);

        let key = make_key(1); 
        let value = make_value("0000000000000000000000000000000000000000000000000000000000000001"); 

        tree.insert(key, value, false);

        let recuperato = tree.get(&key);
     
        assert!(recuperato.is_some(), "il valore dovrebbe essere presente nel tree"); 
        assert_eq!(recuperato.unwrap(), value);
    }

    
    #[test]
    fn test_inserimento_multiplo() {
        let pk = trusted_setup(255);
        let mut tree = VerkleTree::new(pk);

        for i in 0i64..5 {
            let key = make_key(i); //creo la chiave 0,1,2,3,5
            let value = make_value(&format!("{:064x}", i)); //valore 0,1,2,
            tree.insert(key, value, false); //inserisco senza commitment
        }

        for i in 0i64..5 {
            let key = make_key(i); 
            let value = make_value(&format!("{:064x}", i)); 
            assert_eq!(tree.get(&key).unwrap(), value);
        }
    }

    #[test]
    fn test_sovrascrittura_valore() {
        let pk = trusted_setup(255);
        let mut tree = VerkleTree::new(pk);

        let key = make_key(1);
        let value1 = make_value("0000000000000000000000000000000000000000000000000000000000000001");
        let value2 = make_value("0000000000000000000000000000000000000000000000000000000000000002");

        tree.insert(key, value1, false);
        tree.insert(key, value2, false);

        assert_eq!(tree.get(&key).unwrap(), value2, "il valore dovrebbe essere aggiornato");
    }

  
    
    // ------------------------COMMITMENT---------------------------------------

    

    #[test]
    fn test_commitment_cambia_dopo_nuovo_blocco() {
        let pk = trusted_setup(255);
        let mut tree1 = VerkleTree::new(pk.clone()); 
        let mut tree2 = VerkleTree::new(pk);

        let key1 = make_key(1);
        let key2 = make_key(2);
        let value = make_value("0000000000000000000000000000000000000000000000000000000000000001");

        tree1.insert(key1, value, true);
        tree2.insert(key1, value, true);
        tree2.insert(key2, value, true);

       
        let root1 = tree1.get_root_commitment().unwrap();
        let root2 = tree2.get_root_commitment().unwrap();

        assert_ne!(root1.inner, root2.inner);
    }


    // ---------------------------PROVE e VERIFY ------------------------------------

    #[test]
    fn test_prova_e_verifica_blocco_singolo() {
        let pk = trusted_setup(255);
        let mut tree = VerkleTree::new(pk);

        let key = make_key(42);
        let value = make_value("abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890");

        tree.insert(key, value, true);

        let root = tree.get_root_commitment().unwrap();
        let proof = tree.prove(&key).expect("la prova dovrebbe essere generabile");
        let pk_ref = tree.getter_pk();

        let valid = VerkleTree::verify_proof(&proof, pk_ref, root, key);
        assert!(valid, "la prova di membership dovrebbe essere valida");
    }

    //inserisco due blocchi, chiave diversa ma valore uguale
    #[test]
    fn test_verifica_fallisce_con_chiave_sbagliata() {
        let pk = trusted_setup(255);
        let mut tree = VerkleTree::new(pk);

        let key1 = make_key(1);
        let key2 = make_key(2);
        let value = make_value("0000000000000000000000000000000000000000000000000000000000000001");

        tree.insert(key1, value, true);
        tree.insert(key2, value, true);

        let root = tree.get_root_commitment().unwrap();

        let proof = tree.prove(&key1).unwrap(); 

        let pk_ref = tree.getter_pk();

      
        let valid = VerkleTree::verify_proof(&proof, pk_ref, root, key2);
       
        assert!(!valid, "la verifica con chiave errata dovrebbe fallire");
    }

    #[test]
    fn test_prova_e_verifica_piu_blocchi() {
        let pk = trusted_setup(255);
        let mut tree = VerkleTree::new(pk);

        let hash_fake = "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
        for i in 0i64..10 {
            let key = make_key(i);
            let value = make_value(hash_fake);
            tree.insert(key, value, true);
        }

        let root = tree.get_root_commitment().unwrap();

        for i in 0i64..10 {
            let key = make_key(i);
            let proof = tree.prove(&key).expect(&format!("prova per blocco {} non trovata", i));
            let valid = VerkleTree::verify_proof(&proof, tree.getter_pk(), root, key);
            //se i test non passassero, allora assert si blocca e stampa "prova non valida per blocco "
            assert!(valid, "prova non valida per blocco {}", i);
        }
    }



    #[test]
    fn test_ricalcola_tutto_uguale_a_insert_true() {
    let pk = trusted_setup(255);
    let mut tree1 = VerkleTree::new(pk.clone());
    let mut tree2 = VerkleTree::new(pk);

    // tree1: inserisce di volta in volta con true
    for i in 0i64..5 {
        let key = make_key(i);
        let value = make_value(&format!("{:064x}", i));
        tree1.insert(key, value, true);
    }

    // tree2: inserisce tutto con false poi ricalcola_tutto
    for i in 0i64..5 {
        let key = make_key(i);
        let value = make_value(&format!("{:064x}", i));
        tree2.insert(key, value, false);
    }
    tree2.ricalcola_tutto();

    let commits1 = tree1.get_all_commitments();
let commits2 = tree2.get_all_commitments();

println!("commitment tree1 (insert true):");
for c in &commits1 {
    println!("  {}", hex::encode(c));
}

println!("commitment tree2 (ricalcola_tutto):");
for c in &commits2 {
    println!("  {}", hex::encode(c));
}
assert_eq!(commits1, commits2);

    // le radici devono essere uguali
    let root1 = tree1.get_root_commitment().unwrap();
    let root2 = tree2.get_root_commitment().unwrap();
    assert_eq!(root1.inner, root2.inner);
}



}