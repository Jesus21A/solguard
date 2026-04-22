use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use chrono::Utc;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::{models::*, AppState};

type ApiResult<T> = Result<Json<T>, (StatusCode, Json<Value>)>;

fn db_err(e: rusqlite::Error) -> (StatusCode, Json<Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({"error": e.to_string()})),
    )
}

fn sha256_hex(input: &str) -> String {
    let mut h = Sha256::new();
    h.update(input.as_bytes());
    hex::encode(h.finalize())
}

fn fake_tx(prefix: char, data: &str) -> String {
    format!("{}{}", prefix, &sha256_hex(data)[..43])
}

fn now_iso() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

// ── Handlers ──────────────────────────────────────────────────────────────────

pub async fn root() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "proyecto": "SolGuard",
        "version": "0.1.0-hackathon",
        "runtime": "Rust + Axum + Solana Anchor"
    }))
}

pub async fn list_wallets(State(db): State<AppState>) -> ApiResult<Vec<Wallet>> {
    db.list_wallets().map(Json).map_err(db_err)
}

pub async fn get_wallet(
    State(db): State<AppState>,
    Path(wallet_id): Path<String>,
) -> ApiResult<Wallet> {
    match db.get_wallet(&wallet_id).map_err(db_err)? {
        Some(w) => Ok(Json(w)),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(json!({"detail": "Wallet no encontrada"})),
        )),
    }
}

pub async fn submit_kyc(
    State(db): State<AppState>,
    Json(req): Json<KycRequest>,
) -> ApiResult<Value> {
    // Si ya tiene KYC retornamos inmediatamente
    if let Some(w) = db.get_wallet(&req.wallet_id).map_err(db_err)? {
        if w.kyc_level > 0 {
            return Ok(Json(json!({
                "status": "ya_verificado",
                "kyc_level": w.kyc_level
            })));
        }
    }

    let now = now_iso();
    // Nunca guardamos el documento real — solo los primeros 16 hex del hash
    let doc_hash = &sha256_hex(&req.documento)[..16];

    db.upsert_kyc(&req.wallet_id, req.kyc_level, &req.nombre, doc_hash, &now)
        .map_err(db_err)?;

    let fake = fake_tx('5', &format!("{}{}", req.wallet_id, now));
    db.insert_tx_log(
        &req.wallet_id,
        "kyc_mint_sbt",
        "ok",
        &serde_json::to_string(&json!({
            "tx_hash": fake,
            "kyc_level": req.kyc_level
        }))
        .unwrap(),
    )
    .map_err(db_err)?;

    Ok(Json(json!({
        "status": "kyc_aprobado",
        "wallet_id": req.wallet_id,
        "kyc_level": req.kyc_level,
        "sbt_minteado": true,
        "tx_hash_simulado": fake,
        "nota": "En prod: Oracle Signer escribe esto on-chain en Solana devnet"
    })))
}

pub async fn validate_access(
    State(db): State<AppState>,
    Json(req): Json<ValidateRequest>,
) -> ApiResult<Value> {
    let juego = req.juego_id.as_deref().unwrap_or("crypto_arena");
    let accion = req.accion.as_deref().unwrap_or("claim_rewards");

    let (resultado, motivo, permitido) = match db.get_wallet(&req.wallet_id).map_err(db_err)? {
        None => (
            "bloqueado",
            "wallet_sin_registro".to_string(),
            false,
        ),
        Some(w) if w.kyc_level == 0 => ("bloqueado", "sin_kyc".to_string(), false),
        Some(w) if w.risk_score >= 7 => (
            "bloqueado",
            format!("risk_score_alto_{}", w.risk_score),
            false,
        ),
        Some(w) if w.frozen != 0 => ("bloqueado", "wallet_congelada".to_string(), false),
        Some(w) => (
            "permitido",
            format!("kyc_level_{}_risk_{}", w.kyc_level, w.risk_score),
            true,
        ),
    };

    db.insert_tx_log(
        &req.wallet_id,
        accion,
        resultado,
        &serde_json::to_string(&json!({"motivo": motivo, "juego": juego})).unwrap(),
    )
    .map_err(db_err)?;

    if !permitido {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({
                "error": "AccessDenied",
                "motivo": motivo,
                "wallet": req.wallet_id
            })),
        ));
    }

    Ok(Json(json!({
        "permitido": true,
        "motivo": motivo,
        "wallet_id": req.wallet_id
    })))
}

pub async fn update_risk_score(
    State(db): State<AppState>,
    Json(req): Json<RiskScoreUpdate>,
) -> ApiResult<Value> {
    if !(1..=10).contains(&req.risk_score) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"detail": "risk_score debe estar entre 1 y 10"})),
        ));
    }

    match db.get_wallet(&req.wallet_id).map_err(db_err)? {
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(json!({"detail": "Wallet no encontrada"})),
            ))
        }
        Some(_) => {}
    }

    let frozen = req.frozen.unwrap_or(req.risk_score >= 7);
    db.update_risk_score(&req.wallet_id, req.risk_score, frozen)
        .map_err(db_err)?;

    let motivo = req
        .motivo
        .as_deref()
        .unwrap_or("actualización manual");
    let fake = fake_tx('U', &format!("{}{}", req.wallet_id, now_iso()));

    db.insert_tx_log(
        &req.wallet_id,
        "oracle_update_score",
        "ok",
        &serde_json::to_string(&json!({
            "nuevo_score": req.risk_score,
            "frozen": frozen,
            "motivo": motivo,
            "tx_hash": fake
        }))
        .unwrap(),
    )
    .map_err(db_err)?;

    if req.risk_score >= 7 {
        db.insert_roi_report(&req.wallet_id, 0.0, 1, motivo)
            .map_err(db_err)?;
    }

    Ok(Json(json!({
        "status": "score_actualizado",
        "wallet_id": req.wallet_id,
        "risk_score": req.risk_score,
        "frozen": frozen,
        "tx_hash_simulado": fake
    })))
}

pub async fn get_tx_log(
    State(db): State<AppState>,
    Query(q): Query<TxLogQuery>,
) -> ApiResult<Vec<TxLog>> {
    db.get_tx_log(q.wallet_id.as_deref(), q.limit)
        .map(Json)
        .map_err(db_err)
}

pub async fn get_roi_reports(State(db): State<AppState>) -> ApiResult<Vec<RoiReport>> {
    db.get_roi_reports().map(Json).map_err(db_err)
}

pub async fn get_stats(State(db): State<AppState>) -> ApiResult<crate::db::Stats> {
    db.get_stats().map(Json).map_err(db_err)
}
