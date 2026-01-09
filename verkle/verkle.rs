use std::collections::HashMap;

//la chiave la rappresento con un vettore composto da 32 spazi, e in ogni spazio contiene un numero da 0 a 255, ovvero 1 byte --> 1 byte sono 8 bit, 11111111, al max rappresento fino a 255 in int.
pub type Key = [u8; 32];     
//questo è il valore intero da 32 byte che è contenuto nella leaf?? che lo rappresento sempre usando un vettore da 32 celle, che quindi corrispondono a 32 byte
pub type Value = [u8; 32];  
pub type Stem = [u8; 31];    // Primi 31 byte della chiave, stesso suffisso
pub type Suffix = u8;        // Ultimo byte (0-255)
pub type Commitment = [u8; 32];  //per dopo


#[derive(Debug, Clone)]
//nodi interni, ogni nodo puo avere al max 256 figli, possono essere o stem o branch node
pub struct BranchNode {
    // children[i] = None significa "nessun figlio all'indice i"
    // children[i] = Some(NodeRef) significa "c'è un figlio all'indice i"
    pub children: [Option<NodeRef>; 256],  //array di max 256 nodeRef
   //serve poi
    pub commitment: Commitment,
}


#[derive(Debug, Clone)]
//stem node, ovvero 31 byte uguali, conterrà al max 256 valori che condividono i primi 31 byte
pub struct StemNode {
    // prefisso
    pub stem: Stem,                    
    
    // values[suffix] = None significa "nessun valore per questo suffix"
    // values[suffix] = Some(value) significa "c'è un valore per questo suffix"
    pub values: [Option<Value>; 256],  

    //serve x dopo
    pub commitment: Commitment,
}


#[derive(Debug, Clone)]
//enum perchè un nodo puo essere o un nodo branch o un nodo stem
//box serve per allocare sull'heap e non sullo stack,... ma perchè viene fatto??
pub enum NodeRef {
    //punta a un branch
    Branch(Box<BranchNode>),  
    //punta a uno stem
    Stem(Box<StemNode>),      
}

//------------------------------------------------------------
//metodi per chiavi

// serve per estrarre lo stem (primi 31 byte) da una chiave
pub fn get_stem(key: &Key) -> Stem {
    let mut stem = [0u8; 31];
    stem.copy_from_slice(&key[0..31]);
    stem
}

// serve x estrarre il suffix da una chiave, ultimo byte
pub fn get_suffix(key: &Key) -> Suffix {
    key[31]
}


// ricostruisco una chiave con stem + suffix
pub fn make_key(stem: &Stem, suffix: Suffix) -> Key {
    let mut key = [0u8; 32];
    key[0..31].copy_from_slice(stem);
    key[31] = suffix;
    key
}


//.--------------------------------------------------------------------------------

impl BranchNode {
    pub fn new () -> Self {
        BranchNode {
            children: [const { None }; 256],
            // Commitment zero (calcolato dopo)
            commitment: [0u8; 32],
        }
    }
}

impl StemNode {
    /// Crea un nuovo StemNode vuoto per uno stem dato
    pub fn new(stem: Stem) -> Self {
        StemNode {
            stem,
            values: [const { None }; 256],
            // Commitment zero (calcolato dopo)
            commitment: [0u8; 32],
        }
    }
}



// VERKLE TREE 

//classe
pub struct VerkleTree {
    root: BranchNode,
}

//metodi
impl VerkleTree {
    
    // costruttore
    pub fn new() -> Self {
        VerkleTree {
            root: BranchNode :: new(),
        }
    }
   

   //recupera il valore associato a una chiave che la passo in input, se non esiste ritorna none
    pub fn get(&self, key: &Key) -> Option<Value> {
        //ottengo stem
        let stem = get_stem(key);

        //ottengo suffisso che serve poi per ottenere il valore (rappresenta l'indice nel vec dello stem)
        let suffix = get_suffix(key);
        
        //prendo il root e inizio a navigare l'albero
        let mut current_node = &self.root;

        for (livello, &byte) in stem.iter().enumerate() {
            //controlla se c'è un figlio all'indice 'byte'
            match &current_node.children[byte as usize] {
                None => {
                    //il percorso non esiste
                    return None;
                }
                
                //se il figlio è un nodo di tipo branch
                Some(NodeRef::Branch(branch)) => {
                    //navigo al branch successivo
                    current_node = branch;
                }
                
                //se il figlio di current node matcha con nodo di tipo stem
                Some(NodeRef::Stem(stem_node)) => {
                    //ritorna il valore al suffix
                    return stem_node.values[suffix as usize];
                }
            }
        } 
        // se non esiste lo stem
        None
    }




    
    // per inserire una coppia (chiave, valore) nell'albero
    // ritorna il vecchio valore se la chiave esisteva già
    pub fn insert(&mut self, key: Key, value: Value) -> Option<Value> {
        // dalla chiave separo stem e suffisso
        let stem = get_stem(&key);
        let suffix = get_suffix(&key);
        
        //chiamo il metodo
        self.insert_recursive(&mut self.root, &stem, 0, suffix, value)
    }
    
    // funzione ricorsiva x inserimento
    /// # Parametri
    /// - `node`: il nodo corrente (BranchNode)
    /// - `stem`: lo stem della chiave da inserire
    /// - `livello`: a che profondità siamo (quale byte dello stem stiamo guardando)
    /// - `suffix`: il suffix della chiave
    /// - `value`: il valore da inserire
    fn insert_recursive(
        &mut self,
        node: &mut BranchNode,
        stem: &Stem,
        livello: usize,
        suffix: Suffix,
        value: Value,
    ) -> Option<Value> {
        
        //per l'array dello stem che contiene 31 byte, accedo al byte in base a quale livello mi trovo, questo byte indica 
        //il percorso da seguire
        let byte = stem[livello]; 
       //perche serve fare questo??
       // let child_index = byte as usize;
        
        //prendo il nodo, chiamo il suo array children con 256 posizioni e accedo in posizione byte, che sarebbe il byte dello stem 
        match &mut node.children[byte] {

            // CASO 1: nessun figlio ha questo indice → crea il percorso
            None => {
                //ultimo byte dello stem
                if livello == 30 {
                    //creo lo stem node che dentro avrà l'array di 256 posti per i valori(da 32 byte ciascuno)
                    let mut stem_node = StemNode::new(*stem);
                    //usando il suffisso come posizione inserisco il valore
                    stem_node.values[suffix as usize] = Some(value);
                    //serve a mettere lo stem node appena creato come figlio
                    node.children[child_index] = Some(NodeRef::Stem(Box::new(stem_node))); 

                    None // Non c'era un valore precedente
                } else {
                    // Non siamo all'ultimo byte → crea un BranchNode intermedio
                    //creo un branch node
                    let mut new_branch = BranchNode::default();
                    //richiamo il metodo
                    let old_value = self.insert_recursive(
                        &mut new_branch,
                        stem,
                        depth + 1,
                        suffix,
                        value
                    );
                    //aggiungo il nuovo branch node al nodo
                    node.children[child_index] = Some(NodeRef::Branch(Box::new(new_branch)));
                    old_value
                }
            }
            
            // CASO 2: C'è già un Branch → continua a navigare
            Some(NodeRef::Branch(branch)) => {
                self.insert_recursive(branch, stem, depth + 1, suffix, value)
            }
            
            // CASO 3: C'è uno StemNode
            Some(NodeRef::Stem(stem_node)) => {
                if stem_node.stem == *stem {
                    // CASO 3a: Stesso stem → aggiorna/inserisci nel StemNode esistente
                    let old_value = stem_node.values[suffix as usize];
                    stem_node.values[suffix as usize] = Some(value);
                    old_value
                } else {
                    // CASO 3b: Stem diverso → dobbiamo fare lo "split"
                    // Questo è il caso più complesso!
                    self.split_stem_node(node, child_index, stem, depth, suffix, value)
                }
            }
        }
    }
    
    // Esegue lo "split" di un percorso quando due stem diversi collidono
    /// Quando incontriamo uno StemNode con stem diverso, dobbiamo:
    /// 1. Trovare dove i due stem divergono
    /// 2. Creare branch intermedi fino al punto di divergenza
    /// 3. Posizionare entrambi gli stem nei branch corretti
    fn split_stem_node(
        &mut self,
        node: &mut BranchNode,
        child_index: usize,
        new_stem: &Stem,
        depth: usize,
        suffix: Suffix,
        value: Value,
    ) -> Option<Value> {
        // Estrai lo StemNode esistente


        //take è un metodo che permette di estrarre il valore, ovvero l'oggetto
        // Ora quel nodo non appartiene più all'albero, 
        // ma è una variabile libera che puoi manipolare, spostare o ricollegare altrove.
        let old_stem_node = match node.children[child_index].take() {
            Some(NodeRef::Stem(stem)) => stem,
            _ => panic!("Expected stem node"),
        };

        //serve per ottenere lo stem del vecchio nodo, ovvero i 31 byte
        let old_stem = old_stem_node.stem;
        
        // trovare dove i due stem divergono
        let mut divergence_depth = depth + 1;

        while divergence_depth < 31 && old_stem[divergence_depth] == new_stem[divergence_depth] {
            divergence_depth += 1;
        }
        //si va avanti fino a quando non si trova il punto in cui divergono
        
        // Crea branch intermedi fino al punto di divergenza
        let mut current_branch = BranchNode::new();
        let mut branch_chain = vec![];
        
        for d in (depth + 1)..divergence_depth {
            branch_chain.push((old_stem[d] as usize, current_branch));
            current_branch = BranchNode::new();
        }
        
        // Al punto di divergenza, inserisci entrambi gli stem
        if divergence_depth < 31 {
            // I due stem divergono prima dell'ultimo byte
            let old_index = old_stem[divergence_depth] as usize;
            let new_index = new_stem[divergence_depth] as usize;
            
            // Posiziona il vecchio stem
            if divergence_depth == 30 {
                current_branch.children[old_index] = Some(NodeRef::Stem(old_stem_node));
            } else {
                // Continua a creare il percorso per il vecchio stem
                let mut temp_branch = BranchNode::default();
                self.continue_path_for_old_stem(&mut temp_branch, &old_stem, divergence_depth + 1, old_stem_node);
                current_branch.children[old_index] = Some(NodeRef::Branch(Box::new(temp_branch)));
            }
            
            // Crea il percorso per il nuovo stem
            if divergence_depth == 30 {
                let mut new_stem_node = StemNode::new(*new_stem);
                new_stem_node.values[suffix] = Some(value);
                //aggiorno il puntatore
                current_branch.children[new_index] = Some(NodeRef::Stem(Box::new(new_stem_node)));
            } else {
                let mut temp_branch = BranchNode::new();
                //richiamo il metodo sopra
                self.insert_recursive(&mut temp_branch, new_stem, divergence_depth + 1, suffix, value);
                //aggiorno i puntatori
                current_branch.children[new_index] = Some(NodeRef::Branch(Box::new(temp_branch)));
            }
        } else {
            // I due stem sono identici (non dovrebbe succedere se siamo qui)
            panic!("Stems should be different at split");
        }
        
        // Riconnetti la catena di branch
        let mut top_branch = current_branch;
        for (idx, mut branch) in branch_chain.into_iter().rev() {
            branch.children[idx] = Some(NodeRef::Branch(Box::new(top_branch)));
            top_branch = branch;
        }
        
        // Inserisci il primo branch nella posizione originale
        node.children[child_index] = Some(NodeRef::Branch(Box::new(top_branch)));
        
        None // Non c'era un valore precedente per la nuova chiave
    }
    


    // Helper per continuare a creare il percorso per un vecchio stem durante lo split
    fn continue_path_for_old_stem(
        &mut self,
        node: &mut BranchNode,
        stem: &Stem,
        level: usize,
        stem_node: Box<StemNode>,
    ) {
        if level >= 30 {
            // siamo all'ultimo livello, inserisci lo stem node
            let index = stem[30];
            node.children[index] = Some(NodeRef::Stem(stem_node));
        } else {
            // Crea branch intermedi
            let index = stem[level];
            let mut new_branch = BranchNode::new();
            //richiamo la funzione
            self.continue_path_for_old_stem(&mut new_branch, stem, level + 1, stem_node);
            node.children[index] = Some(NodeRef::Branch(Box::new(new_branch)));
        }
    }
    
    // ritorna il commitment della radice (root hash)
    pub fn root_commitment(&self) -> &Commitment {
        &self.root.commitment
    }
}


    
    // per root hash
    /*pub fn root_commitment(&self) -> &Commitment {
        &self.root.commitment
    }*/


