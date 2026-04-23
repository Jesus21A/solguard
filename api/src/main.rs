mod config;
mod db;
mod handlers;
mod models;
pub mod solana;

use axum::{
    response::Html,
    routing::{get, patch, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

pub type AppState = Arc<db::Database>;

async fn dashboard() -> Html<&'static str> {
    Html(include_str!("../../index.html"))
}

#[tokio::main]
async fn main() {
    let cfg = config::Config::load();

    let db = Arc::new(
        db::Database::new(&cfg.database_url).expect("No se pudo abrir la base de datos"),
    );
    db.init(cfg.seed_demo_data)
        .expect("No se pudo inicializar la base de datos");

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/dashboard", get(dashboard))
        .route("/", get(handlers::root))
        .route("/wallets", get(handlers::list_wallets))
        .route("/wallets/:wallet_id", get(handlers::get_wallet))
        .route("/kyc", post(handlers::submit_kyc))
        .route("/validate-access", post(handlers::validate_access))
        .route("/risk-score", patch(handlers::update_risk_score))
        .route("/tx-log", get(handlers::get_tx_log))
        .route("/roi-reports", get(handlers::get_roi_reports))
        .route("/stats", get(handlers::get_stats))
        .layer(cors)
        .with_state(db);

    let addr = format!("0.0.0.0:{}", cfg.port);
    println!("SolGuard API corriendo en http://{}", addr);
    println!("  RPC:     {}", cfg.solana_rpc_url);
    println!("  Program: {}", cfg.solana_program_id);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|_| panic!("No se pudo bindear {}", addr));

    axum::serve(listener, app).await.unwrap();
}
