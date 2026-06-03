# Verkle Tree Indexer

Indexer per la blockchain Ethereum che scarica i blocchi, li salva su un database PostgreSQL e li inserisce in un **Verkle tree** i cui commitment sono calcolati con **KZG** (Kate-Zaverucha-Goldberg).

L'applicazione espone anche un'API HTTP che permette di richiedere una **prova di membership** per un qualsiasi blocco indicizzato e di verificarne la validità.

## Architettura

Il progetto è diviso in due crate Rust:

- **`verkle_kzg_impl`** – libreria che implementa lo schema di commitment KZG e il Verkle tree.
- **`indexer`** – applicazione principale che usa la libreria, parla con Alchemy, scrive sul DB.

All'avvio l'indexer:

1. Si connette al database e carica i blocchi già salvati nel Verkle tree (senza ricalcolare i commitment, recuperandoli dal DB).
2. Calcola quanti blocchi mancano rispetto alla chain (_catch-up_) e li scarica uno a uno.
3. Avvia due task in parallelo:
   - **WebSocket** – rimane in ascolto su Alchemy e indicizza ogni nuovo blocco in tempo reale.
   - **API HTTP** – serve le richieste sulla porta `3000`.

## Prerequisiti

- Rust
- PostgreSQL
- Account Alchemy con una API key

## Setup del database

Crea un database PostgreSQL e poi esegui le seguenti query per creare le tabelle necessarie:

CREATE TABLE blocks (
number BIGINT PRIMARY KEY,
hash VARCHAR(66) NOT NULL,
parent_hash VARCHAR(66) NOT NULL,
timestamp BIGINT NOT NULL,
miner VARCHAR(42) NOT NULL,
gas_used BIGINT NOT NULL,
gas_limit BIGINT NOT NULL,
transactions_count INTEGER NOT NULL,
size BIGINT NOT NULL
);

CREATE TABLE indexer_state (
id INTEGER PRIMARY KEY,
last_block_indexed BIGINT NOT NULL,
last_update TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE verkle_nodes (
node_path TEXT PRIMARY KEY,
node_type TEXT NOT NULL,
commitment BYTEA NOT NULL
);

\*Inserisci il punto di partenza (modifica il numero con l'ultimo blocco già disponibile)

INSERT INTO indexer_state (id, last_block_indexed) VALUES (1, 0);

Il valore `0` in `indexer_state` significa che l'indexer partirà dall'inizio. Puoi impostare un numero più alto per saltare i blocchi più vecchi.

## Configurazione

Copia il file di esempio e compilalo con i tuoi dati:

cp env.example.txt .env

Contenuto del file `.env`:

ALCHEMY_API_KEY=la_tua_api_key
DB_USER=postgres
DB_PASSWORD=la_tua_password
DB_HOST=localhost
DB_NAME=nome_del_database

## Avvio

cd indexer
cargo run --release

---

## API

### `GET /`

Pagina HTML di benvenuto.

### `GET /proof/:block_number`

Genera e verifica la prova di membership per il blocco con numero `block_number`.
La pagina HTML mostra:

- Il numero del blocco e il suo hash
- Il commitment della root del Verkle tree al momento della richiesta
- Il commitment nella prova
- Se la prova è valida o meno

## Come funziona il Verkle tree

Ogni blocco viene inserito nel Verkle tree usando:

- **Chiave**: il numero del blocco codificato in 32 byte
- **Valore**: l'hash del blocco codificato in 48 byte

I commitment dei nodi vengono calcolati con KZG su BLS12-381. Per ogni blocco inserito, vengono aggiornati i commitment di tutti i nodi sul percorso dalla foglia alla radice, e le versioni aggiornate vengono salvate nel database per non doverle ricalcolare al prossimo avvio.

La prova di membership (`/proof/:block_number`) dimostra crittograficamente che un blocco è presente nel tree senza dover rivelare tutti gli altri blocchi.
