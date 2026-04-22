use rusqlite::{params, Connection, Result};
use std::sync::Mutex;

use crate::models::*;

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        // WAL mode: mejora concurrencia para múltiples lecturas
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        Ok(Database {
            conn: Mutex::new(conn),
        })
    }

    pub fn init(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            r#"
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

            INSERT OR IGNORE INTO wallets (wallet_id, kyc_level, risk_score, frozen, nombre, documento)
            VALUES ('7xKp3mNaAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA', 1, 1, 0, 'Carlos A.', 'CC-1023456789');

            INSERT OR IGNORE INTO wallets (wallet_id, kyc_level, risk_score, frozen, nombre, documento)
            VALUES ('SmurfWalletCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC', 0, 9, 1, NULL, NULL);

            INSERT OR IGNORE INTO wallets (wallet_id, kyc_level, risk_score, frozen, nombre, documento)
            VALUES ('NoKycWalletDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD', 0, 1, 0, NULL, NULL);

            INSERT OR IGNORE INTO roi_reports (id, wallet_id, monto_usdc, n_wallets_red, patron)
            VALUES (1, 'SmurfWalletCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC', 8400, 47, 'estrella_invertida');
            "#,
        )?;
        Ok(())
    }

    // ── Wallets ───────────────────────────────────────────────────────────────

    pub fn list_wallets(&self) -> Result<Vec<Wallet>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT * FROM wallets ORDER BY created_at DESC")?;
        stmt.query_map([], wallet_from_row)?
            .collect::<Result<Vec<_>>>()
    }

    pub fn get_wallet(&self, wallet_id: &str) -> Result<Option<Wallet>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT * FROM wallets WHERE wallet_id = ?")?;
        let mut rows = stmt.query_map(params![wallet_id], wallet_from_row)?;
        rows.next().transpose()
    }

    pub fn upsert_kyc(
        &self,
        wallet_id: &str,
        kyc_level: i64,
        nombre: &str,
        doc_hash: &str,
        now: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let existing: Option<i64> = conn
            .query_row(
                "SELECT kyc_level FROM wallets WHERE wallet_id = ?",
                params![wallet_id],
                |r| r.get(0),
            )
            .ok();

        match existing {
            Some(_) => {
                conn.execute(
                    "UPDATE wallets SET kyc_level=?, nombre=?, documento=?, kyc_at=? WHERE wallet_id=?",
                    params![kyc_level, nombre, doc_hash, now, wallet_id],
                )?;
            }
            None => {
                conn.execute(
                    "INSERT INTO wallets (wallet_id, kyc_level, risk_score, frozen, nombre, documento, kyc_at) VALUES (?, ?, 1, 0, ?, ?, ?)",
                    params![wallet_id, kyc_level, nombre, doc_hash, now],
                )?;
            }
        }
        Ok(())
    }

    pub fn update_risk_score(
        &self,
        wallet_id: &str,
        risk_score: i64,
        frozen: bool,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE wallets SET risk_score=?, frozen=? WHERE wallet_id=?",
            params![risk_score, frozen as i64, wallet_id],
        )?;
        Ok(())
    }

    // ── Tx Log ────────────────────────────────────────────────────────────────

    pub fn insert_tx_log(
        &self,
        wallet_id: &str,
        accion: &str,
        resultado: &str,
        detalle: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO tx_log (wallet_id, accion, resultado, detalle) VALUES (?, ?, ?, ?)",
            params![wallet_id, accion, resultado, detalle],
        )?;
        Ok(())
    }

    pub fn get_tx_log(&self, wallet_id: Option<&str>, limit: i64) -> Result<Vec<TxLog>> {
        let conn = self.conn.lock().unwrap();
        if let Some(id) = wallet_id {
            let mut stmt = conn.prepare(
                "SELECT * FROM tx_log WHERE wallet_id = ? ORDER BY ts DESC LIMIT ?",
            )?;
            stmt.query_map(params![id, limit], txlog_from_row)?
                .collect()
        } else {
            let mut stmt =
                conn.prepare("SELECT * FROM tx_log ORDER BY ts DESC LIMIT ?")?;
            stmt.query_map(params![limit], txlog_from_row)?
                .collect()
        }
    }

    // ── ROI Reports ───────────────────────────────────────────────────────────

    pub fn insert_roi_report(
        &self,
        wallet_id: &str,
        monto_usdc: f64,
        n_wallets_red: i64,
        patron: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO roi_reports (wallet_id, monto_usdc, n_wallets_red, patron) VALUES (?, ?, ?, ?)",
            params![wallet_id, monto_usdc, n_wallets_red, patron],
        )?;
        Ok(())
    }

    pub fn get_roi_reports(&self) -> Result<Vec<RoiReport>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT * FROM roi_reports ORDER BY created_at DESC")?;
        stmt.query_map([], roi_from_row)?
            .collect::<Result<Vec<_>>>()
    }

    // ── Stats ─────────────────────────────────────────────────────────────────

    pub fn get_stats(&self) -> Result<Stats> {
        let conn = self.conn.lock().unwrap();
        let count = |sql: &str| -> Result<i64> {
            conn.query_row(sql, [], |r| r.get(0))
        };
        Ok(Stats {
            wallets_total: count("SELECT COUNT(*) FROM wallets")?,
            wallets_kyc_ok: count("SELECT COUNT(*) FROM wallets WHERE kyc_level > 0")?,
            wallets_frozen: count("SELECT COUNT(*) FROM wallets WHERE frozen = 1")?,
            wallets_alto_riesgo: count("SELECT COUNT(*) FROM wallets WHERE risk_score >= 7")?,
            tx_bloqueadas: count("SELECT COUNT(*) FROM tx_log WHERE resultado = 'bloqueado'")?,
            tx_permitidas: count("SELECT COUNT(*) FROM tx_log WHERE resultado = 'permitido'")?,
            reportes_roi: count("SELECT COUNT(*) FROM roi_reports")?,
        })
    }
}

// ── Row mappers ───────────────────────────────────────────────────────────────

fn wallet_from_row(row: &rusqlite::Row) -> rusqlite::Result<Wallet> {
    Ok(Wallet {
        wallet_id: row.get(0)?,
        kyc_level: row.get(1)?,
        risk_score: row.get(2)?,
        frozen: row.get(3)?,
        nombre: row.get(4)?,
        documento: row.get(5)?,
        kyc_at: row.get(6)?,
        created_at: row.get(7)?,
    })
}

fn txlog_from_row(row: &rusqlite::Row) -> rusqlite::Result<TxLog> {
    Ok(TxLog {
        id: row.get(0)?,
        wallet_id: row.get(1)?,
        accion: row.get(2)?,
        resultado: row.get(3)?,
        detalle: row.get(4)?,
        ts: row.get(5)?,
    })
}

fn roi_from_row(row: &rusqlite::Row) -> rusqlite::Result<RoiReport> {
    Ok(RoiReport {
        id: row.get(0)?,
        wallet_id: row.get(1)?,
        monto_usdc: row.get(2)?,
        n_wallets_red: row.get(3)?,
        patron: row.get(4)?,
        estado: row.get(5)?,
        created_at: row.get(6)?,
    })
}

// ── Extra response type ───────────────────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct Stats {
    pub wallets_total: i64,
    pub wallets_kyc_ok: i64,
    pub wallets_frozen: i64,
    pub wallets_alto_riesgo: i64,
    pub tx_bloqueadas: i64,
    pub tx_permitidas: i64,
    pub reportes_roi: i64,
}
