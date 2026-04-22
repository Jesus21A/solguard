"""
SolGuard API — Hackathon Demo
FastAPI + SQLite (sin dependencias pesadas para correr rápido)

Instalar:
    pip install fastapi uvicorn python-jose[cryptography] pydantic

Correr:
    uvicorn main:app --reload --port 8000
"""

from fastapi import FastAPI, HTTPException, Header
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel
from typing import Optional
import sqlite3, json, time, hashlib, os

app = FastAPI(title="SolGuard API", version="0.1.0-hackathon")

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_methods=["*"],
    allow_headers=["*"],
)

DB = "solguard.db"

# ── Base de datos ──────────────────────────────────────────────────────────────

def get_db():
    conn = sqlite3.connect(DB)
    conn.row_factory = sqlite3.Row
    return conn

def init_db():
    conn = get_db()
    conn.executescript("""
        CREATE TABLE IF NOT EXISTS wallets (
            wallet_id   TEXT PRIMARY KEY,
            kyc_level   INTEGER DEFAULT 0,
            risk_score  INTEGER DEFAULT 1,
            frozen      INTEGER DEFAULT 0,
            nombre      TEXT,
            documento   TEXT,
            kyc_at      TEXT,
            created_at  TEXT DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS tx_log (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            wallet_id   TEXT,
            accion      TEXT,
            resultado   TEXT,
            detalle     TEXT,
            ts          TEXT DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS roi_reports (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            wallet_id       TEXT,
            monto_usdc      REAL,
            n_wallets_red   INTEGER,
            patron          TEXT,
            estado          TEXT DEFAULT 'pendiente_uiaf',
            created_at      TEXT DEFAULT (datetime('now'))
        );
    """)
    # Wallets de demo pre-cargadas
    demo_wallets = [
        ("7xKp3mNaAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", 1, 1, 0, "Carlos A.", "CC-1023456789"),
        ("SmurfWalletCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC", 0, 9, 1, None, None),
        ("NoKycWalletDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD", 0, 1, 0, None, None),
    ]
    for w in demo_wallets:
        conn.execute("""
            INSERT OR IGNORE INTO wallets (wallet_id, kyc_level, risk_score, frozen, nombre, documento)
            VALUES (?, ?, ?, ?, ?, ?)
        """, w)
    # Reporte demo
    conn.execute("""
        INSERT OR IGNORE INTO roi_reports (id, wallet_id, monto_usdc, n_wallets_red, patron)
        VALUES (1, 'SmurfWalletCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC', 8400, 47, 'estrella_invertida')
    """)
    conn.commit()
    conn.close()

init_db()

# ── Modelos ────────────────────────────────────────────────────────────────────

class KYCRequest(BaseModel):
    wallet_id: str
    nombre: str
    documento: str          # número de cédula (en prod: hash del documento)
    kyc_level: int = 1      # 1=básico, 2=avanzado

class RiskScoreUpdate(BaseModel):
    wallet_id: str
    risk_score: int         # 1–10
    frozen: Optional[bool] = None
    motivo: Optional[str] = "actualización manual"

class ValidateRequest(BaseModel):
    wallet_id: str
    juego_id: Optional[str] = "crypto_arena"
    accion: Optional[str] = "claim_rewards"

# ── Endpoints ──────────────────────────────────────────────────────────────────

@app.get("/")
def root():
    return {"status": "ok", "proyecto": "SolGuard", "version": "0.1.0-hackathon"}


@app.get("/wallets")
def list_wallets():
    """Lista todas las wallets registradas (para el dashboard)."""
    conn = get_db()
    rows = conn.execute("SELECT * FROM wallets ORDER BY created_at DESC").fetchall()
    conn.close()
    return [dict(r) for r in rows]


@app.get("/wallets/{wallet_id}")
def get_wallet(wallet_id: str):
    """Estado actual de una wallet (simula leer el SBT on-chain)."""
    conn = get_db()
    row = conn.execute("SELECT * FROM wallets WHERE wallet_id = ?", (wallet_id,)).fetchone()
    conn.close()
    if not row:
        raise HTTPException(404, detail="Wallet no encontrada")
    return dict(row)


@app.post("/kyc")
def submit_kyc(req: KYCRequest):
    """
    Registra KYC de un jugador.
    En prod: valida documentos reales, guarda hash, llama Oracle Signer → escribe SBT on-chain.
    En demo: guarda en SQLite y simula la escritura on-chain.
    """
    conn = get_db()
    existing = conn.execute("SELECT kyc_level FROM wallets WHERE wallet_id = ?", (req.wallet_id,)).fetchone()

    if existing and existing["kyc_level"] > 0:
        conn.close()
        return {"status": "ya_verificado", "kyc_level": existing["kyc_level"]}

    now = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    # Hash del documento — nunca guardamos el número real en prod
    doc_hash = hashlib.sha256(req.documento.encode()).hexdigest()[:16]

    if existing:
        conn.execute("""
            UPDATE wallets SET kyc_level=?, nombre=?, documento=?, kyc_at=? WHERE wallet_id=?
        """, (req.kyc_level, req.nombre, doc_hash, now, req.wallet_id))
    else:
        conn.execute("""
            INSERT INTO wallets (wallet_id, kyc_level, risk_score, frozen, nombre, documento, kyc_at)
            VALUES (?, ?, 1, 0, ?, ?, ?)
        """, (req.wallet_id, req.kyc_level, req.nombre, doc_hash, now))

    # Simula tx on-chain
    fake_tx = "5" + hashlib.sha256(f"{req.wallet_id}{now}".encode()).hexdigest()[:43]
    conn.execute("""
        INSERT INTO tx_log (wallet_id, accion, resultado, detalle)
        VALUES (?, 'kyc_mint_sbt', 'ok', ?)
    """, (req.wallet_id, json.dumps({"tx_hash": fake_tx, "kyc_level": req.kyc_level})))
    conn.commit()
    conn.close()

    return {
        "status": "kyc_aprobado",
        "wallet_id": req.wallet_id,
        "kyc_level": req.kyc_level,
        "sbt_minteado": True,
        "tx_hash_simulado": fake_tx,
        "nota": "En prod: Oracle Signer escribe esto on-chain en Solana devnet"
    }


@app.post("/validate-access")
def validate_access(req: ValidateRequest):
    """
    Simula el CPI que haría el contrato del juego a SolGuard.
    Retorna ok o error según el estado del SBT de la wallet.
    """
    conn = get_db()
    row = conn.execute("SELECT * FROM wallets WHERE wallet_id = ?", (req.wallet_id,)).fetchone()

    if not row:
        resultado = "bloqueado"
        motivo = "wallet_sin_registro"
        permitido = False
    elif row["kyc_level"] == 0:
        resultado = "bloqueado"
        motivo = "sin_kyc"
        permitido = False
    elif row["risk_score"] >= 7:
        resultado = "bloqueado"
        motivo = f"risk_score_alto_{row['risk_score']}"
        permitido = False
    elif row["frozen"]:
        resultado = "bloqueado"
        motivo = "wallet_congelada"
        permitido = False
    else:
        resultado = "permitido"
        motivo = f"kyc_level_{row['kyc_level']}_risk_{row['risk_score']}"
        permitido = True

    conn.execute("""
        INSERT INTO tx_log (wallet_id, accion, resultado, detalle)
        VALUES (?, ?, ?, ?)
    """, (req.wallet_id, req.accion, resultado, json.dumps({"motivo": motivo, "juego": req.juego_id})))
    conn.commit()
    conn.close()

    if not permitido:
        raise HTTPException(403, detail={"error": "AccessDenied", "motivo": motivo, "wallet": req.wallet_id})

    return {"permitido": True, "motivo": motivo, "wallet_id": req.wallet_id}


@app.patch("/risk-score")
def update_risk_score(req: RiskScoreUpdate):
    """
    Actualiza el risk_score de una wallet (simula el Oracle Signer escribiendo on-chain).
    En el demo: lo llamas manualmente desde el dashboard para simular que NetworkX detectó algo.
    """
    if not 1 <= req.risk_score <= 10:
        raise HTTPException(400, detail="risk_score debe estar entre 1 y 10")

    conn = get_db()
    row = conn.execute("SELECT wallet_id FROM wallets WHERE wallet_id = ?", (req.wallet_id,)).fetchone()
    if not row:
        conn.close()
        raise HTTPException(404, detail="Wallet no encontrada")

    frozen = req.frozen if req.frozen is not None else (req.risk_score >= 7)
    conn.execute("""
        UPDATE wallets SET risk_score=?, frozen=? WHERE wallet_id=?
    """, (req.risk_score, int(frozen), req.wallet_id))

    fake_tx = "U" + hashlib.sha256(f"{req.wallet_id}{time.time()}".encode()).hexdigest()[:43]
    conn.execute("""
        INSERT INTO tx_log (wallet_id, accion, resultado, detalle)
        VALUES (?, 'oracle_update_score', 'ok', ?)
    """, (req.wallet_id, json.dumps({
        "nuevo_score": req.risk_score,
        "frozen": frozen,
        "motivo": req.motivo,
        "tx_hash": fake_tx
    })))

    # Si score >= 7, genera reporte ROI automático
    if req.risk_score >= 7:
        conn.execute("""
            INSERT INTO roi_reports (wallet_id, monto_usdc, n_wallets_red, patron)
            VALUES (?, ?, ?, ?)
        """, (req.wallet_id, 0, 1, req.motivo or "score_alto_detectado"))

    conn.commit()
    conn.close()

    return {
        "status": "score_actualizado",
        "wallet_id": req.wallet_id,
        "risk_score": req.risk_score,
        "frozen": frozen,
        "tx_hash_simulado": fake_tx
    }


@app.get("/tx-log")
def get_tx_log(wallet_id: Optional[str] = None, limit: int = 50):
    """Log de todas las transacciones/acciones (para el dashboard)."""
    conn = get_db()
    if wallet_id:
        rows = conn.execute("""
            SELECT * FROM tx_log WHERE wallet_id = ? ORDER BY ts DESC LIMIT ?
        """, (wallet_id, limit)).fetchall()
    else:
        rows = conn.execute("SELECT * FROM tx_log ORDER BY ts DESC LIMIT ?", (limit,)).fetchall()
    conn.close()
    return [dict(r) for r in rows]


@app.get("/roi-reports")
def get_roi_reports():
    """Reportes ROI generados automáticamente (para SAGRILAFT/UIAF)."""
    conn = get_db()
    rows = conn.execute("SELECT * FROM roi_reports ORDER BY created_at DESC").fetchall()
    conn.close()
    return [dict(r) for r in rows]


@app.get("/stats")
def get_stats():
    """Métricas generales para el dashboard."""
    conn = get_db()
    total = conn.execute("SELECT COUNT(*) as n FROM wallets").fetchone()["n"]
    kyc_ok = conn.execute("SELECT COUNT(*) as n FROM wallets WHERE kyc_level > 0").fetchone()["n"]
    frozen = conn.execute("SELECT COUNT(*) as n FROM wallets WHERE frozen = 1").fetchone()["n"]
    alto_riesgo = conn.execute("SELECT COUNT(*) as n FROM wallets WHERE risk_score >= 7").fetchone()["n"]
    bloqueadas = conn.execute("SELECT COUNT(*) as n FROM tx_log WHERE resultado = 'bloqueado'").fetchone()["n"]
    permitidas = conn.execute("SELECT COUNT(*) as n FROM tx_log WHERE resultado = 'permitido'").fetchone()["n"]
    reportes = conn.execute("SELECT COUNT(*) as n FROM roi_reports").fetchone()["n"]
    conn.close()
    return {
        "wallets_total": total,
        "wallets_kyc_ok": kyc_ok,
        "wallets_frozen": frozen,
        "wallets_alto_riesgo": alto_riesgo,
        "tx_bloqueadas": bloqueadas,
        "tx_permitidas": permitidas,
        "reportes_roi": reportes,
    }
