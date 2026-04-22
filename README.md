# SolGuard — Demo Hackathon (Rust + Solana)

## Estructura del proyecto

```
solguard/
├── Cargo.toml                   ← workspace Rust
├── Anchor.toml                  ← configuración Anchor CLI
├── programs/
│   └── solguard/
│       └── src/lib.rs           ← Programa on-chain (Anchor)
├── api/
│   └── src/
│       ├── main.rs              ← Servidor Axum (HTTP)
│       ├── db.rs                ← SQLite via rusqlite
│       ├── handlers.rs          ← Handlers de cada endpoint
│       └── models.rs            ← Structs de request/response
├── index.html                   ← Dashboard (abrir en browser)
└── main.py                      ← Versión original FastAPI (referencia)
```

## Requisitos

| Herramienta | Versión | Instalación |
|-------------|---------|-------------|
| Rust | ≥ 1.75 | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Anchor CLI | 0.30.1 | `cargo install --git https://github.com/coral-xyz/anchor anchor-cli` |
| Solana CLI | ≥ 1.18 | https://docs.solana.com/cli/install-solana-cli-tools |

## Correr la API REST (sin Solana, modo demo)

```bash
cd api
cargo run
# → SolGuard API (Rust) corriendo en http://0.0.0.0:8000
```

La primera vez crea `solguard.db` con wallets demo pre-cargadas.

## Compilar y desplegar el programa Anchor (Solana devnet)

```bash
# 1. Generar keypair del programa
anchor keys generate

# 2. Actualizar el ID en programs/solguard/src/lib.rs y Anchor.toml
#    Reemplaza "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS" con el nuevo ID

# 3. Compilar
anchor build

# 4. Deploy en devnet
anchor deploy --provider.cluster devnet
```

## Endpoints disponibles

| Método | Ruta | Descripción |
|--------|------|-------------|
| GET | `/` | Health check |
| GET | `/wallets` | Lista todas las wallets |
| GET | `/wallets/{id}` | Estado de una wallet (simula leer SBT) |
| POST | `/kyc` | Registrar KYC + mintear SBT |
| POST | `/validate-access` | CPI simulado — el juego valida acceso |
| PATCH | `/risk-score` | Oracle Signer actualiza score on-chain |
| GET | `/tx-log` | Log de transacciones |
| GET | `/roi-reports` | Reportes SAGRILAFT/UIAF generados |
| GET | `/stats` | Métricas generales |

## Wallets pre-cargadas

| Wallet | Nombre | KYC | Score | Estado |
|--------|--------|-----|-------|--------|
| `7xKp3mNa...` | Carlos A. | ✅ nivel 1 | 1 | Activa |
| `SmurfWallet...` | Smurf | ❌ sin KYC | 9 | Congelada |
| `NoKycWallet...` | Sin KYC | ❌ sin KYC | 1 | Activa |

## Arquitectura on-chain

El programa Anchor en `programs/solguard/src/lib.rs` expone tres instrucciones:

- **`submit_kyc`** — Crea/actualiza un PDA `WalletRecord` con KYC level, nombre_hash y doc_hash. Emite el evento `KycMinted`.
- **`update_risk_score`** — El Oracle Signer actualiza el risk_score y frozen flag on-chain. Emite `RiskScoreUpdated`.
- **`validate_access`** — Cualquier programa de juego puede hacer CPI a esta instrucción; falla con error específico si la wallet no pasa los controles.

```
WalletRecord PDA seeds: ["wallet_record", wallet_pubkey]
Tamaño de cuenta: 8 (discriminator) + 84 = 92 bytes
```

## Flujo del demo (3 min)

1. **KYC** → POST `/kyc` con nueva wallet + cédula
2. **Validar** → POST `/validate-access` con wallet de Carlos → pasa ✅
3. **Risk Score** → PATCH `/risk-score` subir score del Smurf a 9
4. **Validar** → POST `/validate-access` con Smurf → bloqueada ❌
5. **Reportes** → GET `/roi-reports` → reporte SAGRILAFT generado automáticamente

## Diferencias clave vs versión Python

| Aspecto | Python (FastAPI) | Rust (Axum + Anchor) |
|---------|-----------------|---------------------|
| Rendimiento | ~2k req/s | ~200k req/s |
| Seguridad de tipos | Runtime | Compilación |
| Programa on-chain | Simulado | Anchor (real BPF) |
| CPI | Mock | Instrucción Anchor real |
| Estado on-chain | SQLite off-chain | PDA `WalletRecord` |
