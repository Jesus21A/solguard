mod db;
mod handlers;
mod models;

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
    let db = Arc::new(
        db::Database::new("solguard.db").expect("No se pudo abrir la base de datos"),
    );
    db.init().expect("No se pudo inicializar la base de datos");

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

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000")
        .await
        .expect("No se pudo bindear el puerto 8000");

    println!("SolGuard API (Rust) corriendo en http://0.0.0.0:8000");
    axum::serve(listener, app).await.unwrap();
}
