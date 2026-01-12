
use std::collections::HashMap;

//la chiave la rappresento con un vettore composto da 32 spazi, 
//e in ogni spazio contiene un numero da 0 a 255, ovvero 1 byte 
pub type Key = [u8; 32];     
//questo è il valore intero da 32 byte che è contenuto nella leaf 
pub type Value = [u8; 32];  
pub type Stem = [u8; 31];    //primi 31 byte della chiave, stesso suffisso
pub type Suffix = u8;        //ultimo byte (0-255)



#[derive(Debug, Clone)]
//nodi interni, ogni nodo puo avere al max 256 figli, possono essere o stem o branch node
pub struct BranchNode {
    // children[i] = None significa "nessun figlio all'indice i"
    // children[i] = Some(NodeRef) significa "c'è un figlio all'indice i"
    pub children: [Option<NodeRef>; 256],  //array di max 256 nodeRef
   
}


#[derive(Debug, Clone)]
//stem node, ovvero 31 byte uguali, conterrà al max 256 valori che condividono i primi 31 byte
pub struct StemNode {
    // prefisso
    pub stem: Stem,                    
    
    // values[suffix] = None significa "nessun valore per questo suffix"
    // values[suffix] = Some(value) significa "c'è un valore per questo suffix"
    pub values: [Option<Value>; 256],     
}


#[derive(Debug, Clone)]
//enum perchè un nodo puo essere o un nodo branch o un nodo stem
//box serve per allocare sull'heap e non sullo stack
pub enum NodeRef {
    //punta a un branch
    Branch(Box<BranchNode>),  
    //punta a uno stem
    Stem(Box<StemNode>),      
}

//------------------------------------------------------------
//metodi per chiavi

//serve per estrarre lo stem (primi 31 byte) da una chiave
pub fn get_stem(key: &Key) -> Stem {
    let mut stem = [0u8; 31];
    stem.copy_from_slice(&key[0..31]);
    stem
}

//serve x estrarre il suffix da una chiave, ultimo byte
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
            
        }
    }
}

impl StemNode {
    //creo un nuovo stemnode vuoto 
    pub fn new(stem: Stem) -> Self {
        StemNode {
            stem,
            values: [const { None }; 256],
            
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
    
    //costruttore
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

        for (&byte) in stem.iter(){
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
        //se non esiste lo stem
        None
    }


    
    //per inserire una coppia (chiave, valore) nell'albero
    //ritorna il vecchio valore se la chiave esisteva già
    pub fn insert(&mut self, key: Key, value: Value) -> Option<Value> {
        let stem = get_stem(&key);
        let suffix = get_suffix(&key);

        Self::insert_recursive(&mut self.root, &stem, 0, suffix, value)
    }
    
    fn insert_recursive(
       node: &mut BranchNode,
        stem: &Stem,
        level: usize,
        suffix: u8,
        value: Value,
    ) -> Option<Value> {
        
        //per l'array dello stem che contiene 31 byte, accedo al byte in base a quale livello mi trovo
        let index = stem[level]; 

       //trasformo in usize perchè index con quel tipo non puo essere usato
       let child_index = index as usize;
        
        //prendo il nodo, chiamo il suo array children con 256 posizioni e accedo in posizione byte, che sarebbe il byte dello stem 
        match &mut node.children[child_index] {

            //nessun figlio ha questo indice → crea il percorso
            None=>{
                //ultimo byte dello stem
                if level == 30 {
                    //creo lo stem node che dentro avrà l'array di 256 posti per i valori(da 32 byte ciascuno)
                    let mut stem_node = StemNode::new(*stem);
                    //usando il suffisso come posizione inserisco il valore
                    stem_node.values[suffix as usize] = Some(value);
                    //aggiorno: serve a mettere lo stem node appena creato come figlio
                    node.children[child_index] = Some(NodeRef::Stem(Box::new(stem_node))); 
                    None
                } else {
                    //non è all'ultimo byte, devo quindi creare il percorso
                    let mut new_branch = BranchNode::new();
                    //richiamo il metodo
                     let old_value = Self::insert_recursive(
                        &mut new_branch,
                        stem,
                        level + 1,
                        suffix,
                        value,
                    );
                    //aggiungo il nuovo branch node al nodo
                    node.children[child_index] = Some(NodeRef::Branch(Box::new(new_branch)));
                    None
                }}
            
            
            //se esiste già il branch
            Some(NodeRef::Branch(branch)) => {
                Self::insert_recursive(branch, stem, level + 1, suffix, value)
            }
            
            //è uno stemnode
            Some(NodeRef::Stem(stem_node)) => {
               
                    //stesso stem → aggiorna/inserisci nel stemnode esistente
                    let old_value = stem_node.values[suffix as usize];
                    stem_node.values[suffix as usize] = Some(value);
                    old_value //restituisco il valore che è stato sovrascritto
               
            
            }
        }
    }
    
    

}




fn main() {
   

    let mut tree = VerkleTree::new();

    // inserimento 
    let mut key1 = [0u8; 32];
    key1[30] = 1; // stem termina con ...01
    key1[31] = 5; // suffix = 5
    
    let value1 = [1u8; 32];

    println!("inserimento chiave con stem[30]=1, suffix=5");
    let old = tree.insert(key1, value1);
    println!("value precedente: {:?}", old);
    println!("value recuperato: {:?}\n", tree.get(&key1));

    // inserimento con stesso stem, suffix diverso
    let mut key2 = [0u8; 32];
    key2[30] = 1; // stesso stem di key1
    key2[31] = 10; // suffix diverso
    let value2 = [2u8; 32];

    println!("inserisco con stesso stem, suffix=10");
    tree.insert(key2, value2);
    println!(" key2: {:?}\n", tree.get(&key2));

    

    // update valore esistente, quindi stesso stem
    let value_updated = [99u8; 32];
    println!("cambio il valore associato a key1");
    let old = tree.insert(key1, value_updated);
    println!("value precedente: {:?}", old);
    println!("nuovo value: {:?}\n", tree.get(&key1));

    // chiave non esiste
    let mut key4 = [0u8; 32];
    key4[30] = 99;
    key4[31] = 99;
    println!("ricerca chiave non esistente");
    println!("ris: {:?}\n", tree.get(&key4));

}


