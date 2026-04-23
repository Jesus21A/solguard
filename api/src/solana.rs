use sha2::{Digest, Sha256};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    system_program,
    transaction::Transaction,
};
use std::str::FromStr;

fn cfg() -> crate::config::Config {
    crate::config::Config::load()
}

pub fn explorer_tx(sig: &str) -> String {
    let rpc = cfg().solana_rpc_url;
    let cluster = if rpc.contains("mainnet") { "mainnet" } else { "devnet" };
    format!("https://explorer.solana.com/tx/{}?cluster={}", sig, cluster)
}

// Anchor discriminator = SHA256("global:{name}")[..8]
fn discriminator(name: &str) -> [u8; 8] {
    let mut h = Sha256::new();
    h.update(format!("global:{}", name).as_bytes());
    h.finalize()[..8].try_into().unwrap()
}

fn load_oracle() -> Result<Keypair, String> {
    let path = cfg().oracle_keypair_path;
    let data = std::fs::read_to_string(&path)
        .map_err(|e| format!("keypair no encontrado en {}: {}", path, e))?;
    let bytes: Vec<u8> =
        serde_json::from_str(&data).map_err(|e| format!("keypair inválido: {}", e))?;
    Keypair::from_bytes(&bytes).map_err(|e| format!("keypair error: {}", e))
}

fn wallet_record_pda(wallet: &Pubkey, program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"wallet_record", wallet.as_ref()], program_id).0
}

fn send_ix(
    program_id: Pubkey,
    accounts: Vec<AccountMeta>,
    data: Vec<u8>,
    signer: &Keypair,
) -> Result<String, String> {
    let client =
        RpcClient::new_with_commitment(cfg().solana_rpc_url, CommitmentConfig::confirmed());
    let blockhash = client
        .get_latest_blockhash()
        .map_err(|e| format!("RPC error: {}", e))?;
    let ix = Instruction { program_id, accounts, data };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&signer.pubkey()),
        &[signer],
        blockhash,
    );
    client
        .send_and_confirm_transaction(&tx)
        .map(|s| s.to_string())
        .map_err(|e| format!("tx error: {}", e))
}

/// Envía `submit_kyc` al programa on-chain.
/// Retorna Ok(signature) o Err(motivo).
pub fn submit_kyc_tx(
    wallet_id: &str,
    kyc_level: u8,
    nombre: &str,
    doc_hash_hex: &str,
) -> Result<String, String> {
    let c = cfg();
    let program_id = Pubkey::from_str(&c.solana_program_id)
        .map_err(|e| format!("SOLANA_PROGRAM_ID inválido: {}", e))?;
    let wallet_pubkey = Pubkey::from_str(wallet_id)
        .map_err(|_| "wallet_id no es una pubkey Solana válida".to_string())?;
    let oracle = load_oracle()?;
    let wallet_record = wallet_record_pda(&wallet_pubkey, &program_id);

    let mut nombre_h = Sha256::new();
    nombre_h.update(nombre.as_bytes());
    let nombre_hash: [u8; 16] = nombre_h.finalize()[..16].try_into().unwrap();

    let doc_bytes = hex::decode(doc_hash_hex).map_err(|e| e.to_string())?;
    let doc_hash: [u8; 16] = doc_bytes[..16]
        .try_into()
        .map_err(|_| "doc_hash inválido".to_string())?;

    // data = discriminator(8) + kyc_level(1) + nombre_hash(16) + doc_hash(16)
    let mut data = discriminator("submit_kyc").to_vec();
    data.push(kyc_level);
    data.extend_from_slice(&nombre_hash);
    data.extend_from_slice(&doc_hash);

    let accounts = vec![
        AccountMeta::new(wallet_record, false),
        AccountMeta::new_readonly(wallet_pubkey, false),
        AccountMeta::new(oracle.pubkey(), true),          // authority: writable + signer
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    send_ix(program_id, accounts, data, &oracle)
}

/// Envía `update_risk_score` al programa on-chain.
pub fn update_risk_score_tx(
    wallet_id: &str,
    risk_score: u8,
    frozen: bool,
) -> Result<String, String> {
    let c = cfg();
    let program_id = Pubkey::from_str(&c.solana_program_id)
        .map_err(|e| format!("SOLANA_PROGRAM_ID inválido: {}", e))?;
    let wallet_pubkey = Pubkey::from_str(wallet_id)
        .map_err(|_| "wallet_id no es una pubkey Solana válida".to_string())?;
    let oracle = load_oracle()?;
    let wallet_record = wallet_record_pda(&wallet_pubkey, &program_id);

    // data = discriminator(8) + risk_score(1) + Option<bool>::Some(frozen)(2)
    let mut data = discriminator("update_risk_score").to_vec();
    data.push(risk_score);
    data.push(1u8);          // Option::Some
    data.push(frozen as u8); // bool value

    let accounts = vec![
        AccountMeta::new(wallet_record, false),
        AccountMeta::new_readonly(wallet_pubkey, false),
        AccountMeta::new_readonly(oracle.pubkey(), true), // oracle: signer
    ];

    send_ix(program_id, accounts, data, &oracle)
}
