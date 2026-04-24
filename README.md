# SolGuard

**Sistema de identidad y cumplimiento regulatorio para gaming Web3 en Solana.**

> Construido durante un hackathon como respuesta a un problema real: el gaming blockchain en Latinoamérica no tiene infraestructura de cumplimiento. Cualquiera puede jugar, lavar dinero o evadir controles cambiando de wallet. Las plataformas no pueden cumplir con SAGRILAFT ni reportar a la UIAF porque no existe la capa de identidad.

---

## Por qué existe este proyecto

Trabajo en el ecosistema Web3 en Colombia y vi de primera mano que los juegos blockchain tienen un problema serio: sus usuarios son anónimos por diseño, pero la regulación colombiana (SAGRILAFT) exige identificar a los jugadores que mueven ciertos montos y reportarlos a la UIAF.

El problema no es legal — es técnico. No existe una forma estándar, open-source y nativa de Solana para hacer KYC, asignar perfiles de riesgo y bloquear wallets sospechosas. Cada plataforma reinventa la rueda, mal.

SolGuard es mi propuesta: una capa de identidad on-chain que cualquier juego puede integrar con una sola instrucción.

---

## Cómo funciona

```
Dashboard / Game Contract
         ↓
  API Rust (Oracle Signer)     ← verifica KYC, firma transacciones
         ↓
  Programa Anchor (Solana)     ← guarda estado on-chain, inmutable
         ↓
     Devnet / Mainnet
```

Cada jugador tiene un `WalletRecord` — una cuenta PDA en Solana con su nivel de KYC, risk score y estado de congelamiento. Los juegos consultan ese estado vía CPI. El Oracle Signer es el único autorizado a escribirlo.

---

## Estructura del proyecto

```
solguard/
├── programs/solguard/src/lib.rs   ← Contrato Anchor on-chain
├── api/src/
│   ├── main.rs                    ← Servidor Axum
│   ├── solana.rs                  ← Oracle Signer (firma txs)
│   ├── handlers.rs                ← Lógica de endpoints
│   ├── db.rs                      ← SQLite (logs, reportes)
│   ├── config.rs                  ← Variables de entorno
│   └── models.rs                  ← Tipos request/response
├── index.html                     ← Dashboard web
├── .env.example                   ← Variables de entorno
└── main.py                        ← Ver nota abajo
```

> **¿Por qué está `main.py`?**
> El prototipo inicial se construyó en Python + FastAPI para validar la idea rápido. Una vez confirmado que el diseño funcionaba, se migró completamente a Rust + Anchor para tener tipos seguros, rendimiento real y un programa on-chain verdadero. `main.py` se conserva como referencia del proceso de desarrollo — muestra la evolución del proyecto, no es código en uso.

---

## Probar el sistema

### Opción A — GitHub Codespaces (recomendado, sin instalar nada)

1. Ir a **https://codespaces.new/Jesus21A/solguard**
2. Crear el codespace
3. En la terminal:

```bash
# Instalar Rust (primera vez, ~2 min)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source ~/.cargo/env

# Instalar Solana CLI (primera vez, ~1 min)
sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)"
echo 'export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# Copiar configuración
cp .env.example .env

# Arrancar la API
cd api && cargo run
```

4. En la pestaña **Ports**, hacer puerto 8000 público
5. Abrir `https://TU-CODESPACE-URL/dashboard`

### Opción B — Local (requiere Rust instalado)

```bash
git clone https://github.com/Jesus21A/solguard
cd solguard
cp .env.example .env
cd api && cargo run
# → http://localhost:8000/dashboard
```

---

## Variables de entorno

Copia `.env.example` a `.env` y ajusta:

```env
PORT=8000
DATABASE_URL=solguard.db
SOLANA_RPC_URL=https://api.devnet.solana.com
SOLANA_PROGRAM_ID=9cuFsdRhYpm2JjJ1NbzA8W2nG5tSgzHto3ABt5qycsN9
ORACLE_KEYPAIR_PATH=/ruta/a/tu/keypair.json
SEED_DEMO_DATA=true
```

Para pasar a mainnet: cambia `SOLANA_RPC_URL` y `SEED_DEMO_DATA=false`. Sin recompilar.

---

## Endpoints

| Método | Ruta | Descripción |
|--------|------|-------------|
| GET | `/dashboard` | Dashboard web |
| GET | `/wallets` | Listar wallets registradas |
| GET | `/wallets/{id}` | Estado de una wallet |
| POST | `/kyc` | Registrar KYC — escribe on-chain si wallet_id es pubkey Solana válida |
| POST | `/validate-access` | Validar acceso a un juego |
| PATCH | `/risk-score` | Actualizar risk score — escribe on-chain |
| GET | `/tx-log` | Historial de transacciones |
| GET | `/roi-reports` | Reportes UIAF generados automáticamente |
| GET | `/stats` | Métricas del dashboard |

Cuando `wallet_id` es una pubkey Solana válida, `/kyc` y `/risk-score` firman y envían una transacción real a devnet. La respuesta incluye `"on_chain": true` y el link del Solana Explorer.

---

## Demo rápido (3 minutos)

Con la API corriendo y tu address de Solana (`solana address`):

```bash
# 1. Registrar KYC — genera tx on-chain
curl -X POST http://localhost:8000/kyc \
  -H "Content-Type: application/json" \
  -d '{"wallet_id":"TU_SOLANA_ADDRESS","nombre":"Ana López","documento":"CC-9876543210"}'

# 2. Validar acceso → permitido
curl -X POST http://localhost:8000/validate-access \
  -H "Content-Type: application/json" \
  -d '{"wallet_id":"TU_SOLANA_ADDRESS"}'

# 3. Subir risk score a 9 → wallet congelada on-chain
curl -X PATCH http://localhost:8000/risk-score \
  -H "Content-Type: application/json" \
  -d '{"wallet_id":"TU_SOLANA_ADDRESS","risk_score":9,"motivo":"actividad_sospechosa"}'

# 4. Validar acceso → bloqueado (403)
curl -X POST http://localhost:8000/validate-access \
  -H "Content-Type: application/json" \
  -d '{"wallet_id":"TU_SOLANA_ADDRESS"}'
```

---

## Programa on-chain

Desplegado en Solana devnet: [`9cuFsdRhYpm2JjJ1NbzA8W2nG5tSgzHto3ABt5qycsN9`](https://explorer.solana.com/address/9cuFsdRhYpm2JjJ1NbzA8W2nG5tSgzHto3ABt5qycsN9?cluster=devnet)

Tres instrucciones:
- **`submit_kyc`** — Crea el `WalletRecord` PDA con nivel KYC y hashes del documento
- **`update_risk_score`** — Oracle Signer actualiza score y congela la wallet si es necesario
- **`validate_access`** — Cualquier juego puede hacer CPI a esta instrucción para verificar acceso

---

## Qué sigue

Este proyecto nació en un hackathon pero apunta a algo real:

- [ ] Validación de cédula via API de la Registraduría colombiana
- [ ] Oracle Signer en HSM (Hardware Security Module)
- [ ] SDK para que juegos integren `validate_access` en una línea
- [ ] Motor de riesgo con análisis de grafo (NetworkX → detección de smurf accounts)
- [ ] Soporte multi-chain (Ethereum, Base)
- [ ] Panel UIAF con exportación automática de reportes

---

## Stack

| Capa | Tecnología |
|------|-----------|
| Contrato on-chain | Rust + Anchor 0.30 |
| API / Oracle Signer | Rust + Axum |
| Base de datos | SQLite (rusqlite) |
| Blockchain | Solana Devnet |
| Dashboard | HTML + JS vanilla |
| Prototipo inicial | Python + FastAPI |
