use std::env;

pub struct Config {
    pub port: String,
    pub database_url: String,
    pub solana_rpc_url: String,
    pub solana_program_id: String,
    pub oracle_keypair_path: String,
    pub seed_demo_data: bool,
}

impl Config {
    pub fn load() -> Self {
        // Carga .env si existe — en prod las vars vienen del sistema
        let _ = dotenvy::dotenv();

        Config {
            port: env::var("PORT").unwrap_or_else(|_| "8000".to_string()),

            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "solguard.db".to_string()),

            solana_rpc_url: env::var("SOLANA_RPC_URL")
                .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string()),

            solana_program_id: env::var("SOLANA_PROGRAM_ID")
                .unwrap_or_else(|_| "9cuFsdRhYpm2JjJ1NbzA8W2nG5tSgzHto3ABt5qycsN9".to_string()),

            oracle_keypair_path: env::var("ORACLE_KEYPAIR_PATH")
                .unwrap_or_else(|_| {
                    // Detecta ruta según entorno
                    let codespace = "/home/codespace/.config/solana/id.json";
                    let local     = dirs_next::home_dir()
                        .map(|h| h.join(".config/solana/id.json"))
                        .and_then(|p| p.to_str().map(|s| s.to_string()))
                        .unwrap_or_else(|| "~/.config/solana/id.json".to_string());

                    if std::path::Path::new(codespace).exists() {
                        codespace.to_string()
                    } else {
                        local
                    }
                }),

            seed_demo_data: env::var("SEED_DEMO_DATA")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(true),
        }
    }
}
