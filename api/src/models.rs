use serde::{Deserialize, Serialize};

// ── DB row types ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct Wallet {
    pub wallet_id: String,
    pub kyc_level: i64,
    pub risk_score: i64,
    pub frozen: i64,
    pub nombre: Option<String>,
    pub documento: Option<String>,
    pub kyc_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct TxLog {
    pub id: i64,
    pub wallet_id: String,
    pub accion: String,
    pub resultado: String,
    pub detalle: String,
    pub ts: String,
}

#[derive(Debug, Serialize)]
pub struct RoiReport {
    pub id: i64,
    pub wallet_id: String,
    pub monto_usdc: f64,
    pub n_wallets_red: i64,
    pub patron: String,
    pub estado: String,
    pub created_at: String,
}

// ── Request bodies ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct KycRequest {
    pub wallet_id: String,
    pub nombre: String,
    /// Número de cédula. En prod se hashea antes de guardar.
    pub documento: String,
    #[serde(default = "default_kyc_level")]
    pub kyc_level: i64,
}

fn default_kyc_level() -> i64 {
    1
}

#[derive(Debug, Deserialize)]
pub struct RiskScoreUpdate {
    pub wallet_id: String,
    pub risk_score: i64,
    pub frozen: Option<bool>,
    #[serde(default = "default_motivo")]
    pub motivo: Option<String>,
}

fn default_motivo() -> Option<String> {
    Some("actualización manual".to_string())
}

#[derive(Debug, Deserialize)]
pub struct ValidateRequest {
    pub wallet_id: String,
    #[serde(default = "default_juego")]
    pub juego_id: Option<String>,
    #[serde(default = "default_accion")]
    pub accion: Option<String>,
}

fn default_juego() -> Option<String> {
    Some("crypto_arena".to_string())
}
fn default_accion() -> Option<String> {
    Some("claim_rewards".to_string())
}

// ── Query params ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct TxLogQuery {
    pub wallet_id: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    50
}
