// API route definitions will be implemented here

use axum::{Router, routing::get};

pub fn create_routes() -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/matches", get(get_matches))
        .route("/api/v1/predictions", get(get_predictions))
        .route("/api/v1/bets", get(get_bets))
}

async fn health_check() -> &'static str {
    "OK"
}

async fn get_matches() -> &'static str {
    "matches endpoint - TODO"
}

async fn get_predictions() -> &'static str {
    "predictions endpoint - TODO"
}

async fn get_bets() -> &'static str {
    "bets endpoint - TODO"
}