use axum::{Router, extract::{Path, State}, routing::get};
use askama::Template;
use axum::response::Html;
use std::sync::Arc;
use tokio::sync::Mutex;
use verkle_project::VerkleTree;
use crate::utils;
use ark_ec::CurveGroup;
use ark_serialize::CanonicalSerialize;

#[derive(Clone)]
pub struct AppState {
    pub tree: Arc<Mutex<VerkleTree>>, 
}

#[derive(Template)]
#[template(path = "proof.html")]
struct ProofTemplate {
    block_number: i64,
    block_hash: String,
    proof_valid: bool,
    root_attuale: String,
    commitment_nella_prova: String,
}

#[derive(Template)]
#[template(path = "home.html")]
struct HomeTemplate;
//-----------------HANDLER HOME-------------------
pub async fn home(
)-> Html<String>{
    let template= HomeTemplate;
    Html(template.render().unwrap())
}

pub async fn get_proof_html(
    Path(block_number): Path<i64>,
    State(state): State<AppState>,
) -> Html<String> {

    //trasformo il numero in chiave (qual è la lunghezza)
    let key = utils::block_number_to_key(block_number);
   
    //blocco il tree
    let tree_lock = state.tree.lock().await;

    //recupera il commitment della root del verkle tree 
    //restituisce o Some(root) oppure printa errore 
    let Some(root) = tree_lock.get_root_commitment() else {
        return Html("<h1>Errore: tree vuoto</h1>".to_string());
    };

    //serializzo perchè non posso printare il punto projective
    let mut bytes = Vec::new();
    root.inner.into_affine().serialize_compressed(&mut bytes).unwrap();
    let root_hex = hex::encode(bytes);

    //chiama il metodo del verkle_tree prove(key), come parametro passo la chiave ovvero il numero del blocco
    //restituisce membershipProof
    let Some(proof) = tree_lock.prove(&key) else {
        return Html(format!("<h1>Blocco {} non trovato nel tree</h1>", block_number));
    };

    //chiamo il metodo del verkletree per la verifica
    let valid = VerkleTree::verify_proof(&proof, tree_lock.getter_pk(), root, key);
    //salvo hash
    let hash = format!("0x{}", hex::encode(&proof.value[0..32]));

    // serializzo il commitment dentro la prova
    let mut bytes_prova = Vec::new();
    proof.steps.last().unwrap().commitment.inner
        .into_affine()
        .serialize_compressed(&mut bytes_prova)
        .unwrap();
    let commitment_hex = hex::encode(bytes_prova);

    //compilo il template che poi passo a html
    let template = ProofTemplate {
        block_number,
        block_hash: hash,
        proof_valid: valid,
        root_attuale: root_hex,
        commitment_nella_prova: commitment_hex,
    };

    Html(template.render().unwrap())
}



// ---------------------------------------------------------------
// costruisce gli endpoint
pub fn build_router(tree: Arc<Mutex<VerkleTree>>) -> Router {

    let state = AppState { tree };

    Router::new()
        .route("/", get(home))   
        .route("/proof/:block_number", get(get_proof_html))
        .with_state(state)
}