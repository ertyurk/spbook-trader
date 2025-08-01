use axum::{
    Router, 
    routing::{get, post},
    extract::{Query, Path, State},
    response::Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use quant_services::{TradingEngine, MarketSimulator, PredictorService};
use quant_models::{MatchEvent, Prediction, SimpleMarketOdds};

#[derive(Clone)]
pub struct AppState {
    pub trading_engine: Arc<TradingEngine>,
    pub market_simulator: Arc<MarketSimulator>,
    pub predictor: Arc<PredictorService>,
    pub recent_events: Arc<RwLock<Vec<MatchEvent>>>,
    pub recent_predictions: Arc<RwLock<Vec<Prediction>>>,
}

#[derive(Deserialize)]
pub struct PaginationParams {
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
    pub pagination: Option<PaginationInfo>,
}

#[derive(Serialize)]
pub struct PaginationInfo {
    pub page: u32,
    pub limit: u32,
    pub total: u32,
    pub pages: u32,
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: String,
    pub version: String,
    pub uptime: String,
}

#[derive(Serialize)]
pub struct PortfolioResponse {
    pub total_bankroll: String,
    pub available_bankroll: String,
    pub total_exposure: String,
    pub active_bets_count: usize,
    pub total_trades: u64,
    pub roi: f64,
    pub win_rate: f64,
    pub profit_loss: String,
}

pub fn create_routes() -> Router<AppState> {
    Router::new()
        // Health and status
        .route("/health", get(health_check))
        .route("/api/v1/status", get(get_system_status))
        
        // Live data endpoints
        .route("/api/v1/events", get(get_recent_events))
        .route("/api/v1/events/live", get(get_live_events))
        
        // Predictions
        .route("/api/v1/predictions", get(get_recent_predictions))
        .route("/api/v1/predictions/:match_id", get(get_prediction_by_match))
        
        // Market data
        .route("/api/v1/odds/:match_id", get(get_market_odds))
        .route("/api/v1/markets", get(get_all_markets))
        
        // Trading and portfolio
        .route("/api/v1/portfolio", get(get_portfolio))
        .route("/api/v1/trades", get(get_recent_trades))
        .route("/api/v1/trades/signals", get(get_trading_signals))
        
        // Analytics
        .route("/api/v1/analytics/performance", get(get_performance_analytics))
        .route("/api/v1/analytics/models", get(get_model_performance))
        
        // Simulation controls
        .route("/api/v1/simulation/start", post(start_simulation))
        .route("/api/v1/simulation/stop", post(stop_simulation))
        .route("/api/v1/simulation/status", get(get_simulation_status))
}

// Health check endpoint
async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime: "unknown".to_string(), // TODO: Track actual uptime
    })
}

// System status with detailed information
async fn get_system_status(State(state): State<AppState>) -> Json<ApiResponse<serde_json::Value>> {
    let portfolio = state.trading_engine.get_portfolio_summary().await;
    let events_count = state.recent_events.read().await.len();
    let predictions_count = state.recent_predictions.read().await.len();
    
    let mut status = serde_json::Map::new();
    status.insert("portfolio".to_string(), serde_json::json!({
        "bankroll": portfolio.total_bankroll.to_string(),
        "available": portfolio.available_bankroll.to_string(),
        "active_bets": portfolio.active_bets_count,
        "total_trades": portfolio.total_trades,
        "roi": format!("{:.2}%", portfolio.roi * 100.0)
    }));
    status.insert("data_pipeline".to_string(), serde_json::json!({
        "recent_events": events_count,
        "recent_predictions": predictions_count,
        "status": "active"
    }));
    status.insert("services".to_string(), serde_json::json!({
        "trading_engine": "online",
        "predictor": "online", 
        "market_simulator": "online"
    }));
    
    Json(ApiResponse {
        success: true,
        data: Some(serde_json::Value::Object(status)),
        message: Some("System operational".to_string()),
        pagination: None,
    })
}

// Get recent match events
async fn get_recent_events(
    Query(params): Query<PaginationParams>,
    State(state): State<AppState>,
) -> Json<ApiResponse<Vec<MatchEvent>>> {
    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(50).min(100); // Max 100 per page
    
    let events = state.recent_events.read().await;
    let total = events.len() as u32;
    let start = ((page - 1) * limit) as usize;
    let end = (start + limit as usize).min(events.len());
    
    let page_events = if start < events.len() {
        events[start..end].to_vec()
    } else {
        vec![]
    };
    
    Json(ApiResponse {
        success: true,
        data: Some(page_events),
        message: None,
        pagination: Some(PaginationInfo {
            page,
            limit,
            total,
            pages: (total + limit - 1) / limit,
        }),
    })
}

// Get live events (last 10)
async fn get_live_events(State(state): State<AppState>) -> Json<ApiResponse<Vec<MatchEvent>>> {
    let events = state.recent_events.read().await;
    let recent = events.iter().rev().take(10).cloned().collect();
    
    Json(ApiResponse {
        success: true,
        data: Some(recent),
        message: Some("Live events".to_string()),
        pagination: None,
    })
}

// Get recent predictions
async fn get_recent_predictions(
    Query(params): Query<PaginationParams>,
    State(state): State<AppState>,
) -> Json<ApiResponse<Vec<Prediction>>> {
    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(20).min(100);
    
    let predictions = state.recent_predictions.read().await;
    let total = predictions.len() as u32;
    let start = ((page - 1) * limit) as usize;
    let end = (start + limit as usize).min(predictions.len());
    
    let page_predictions = if start < predictions.len() {
        predictions[start..end].to_vec()
    } else {
        vec![]
    };
    
    Json(ApiResponse {
        success: true,
        data: Some(page_predictions),
        message: None,
        pagination: Some(PaginationInfo {
            page,
            limit,
            total,
            pages: (total + limit - 1) / limit,
        }),
    })
}

// Get prediction for specific match
async fn get_prediction_by_match(
    Path(match_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Prediction>>, StatusCode> {
    let predictions = state.recent_predictions.read().await;
    
    if let Some(prediction) = predictions.iter().find(|p| p.match_id == match_id) {
        Ok(Json(ApiResponse {
            success: true,
            data: Some(prediction.clone()),
            message: None,
            pagination: None,
        }))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

// Get market odds for specific match
async fn get_market_odds(
    Path(match_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<SimpleMarketOdds>>, StatusCode> {
    if let Some(odds) = state.market_simulator.get_current_odds(&match_id).await {
        Ok(Json(ApiResponse {
            success: true,
            data: Some(odds),
            message: None,
            pagination: None,
        }))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

// Get all current market odds
async fn get_all_markets(State(state): State<AppState>) -> Json<ApiResponse<HashMap<String, SimpleMarketOdds>>> {
    // This is a simplified version - in reality we'd store this in the market simulator
    let mut markets = HashMap::new();
    
    // Get recent match IDs from events
    let events = state.recent_events.read().await;
    let recent_match_ids: std::collections::HashSet<String> = events
        .iter()
        .rev()
        .take(20)
        .map(|e| e.match_id.clone())
        .collect();
    
    for match_id in recent_match_ids {
        if let Some(odds) = state.market_simulator.get_current_odds(&match_id).await {
            markets.insert(match_id, odds);
        }
    }
    
    let markets_count = markets.len();
    Json(ApiResponse {
        success: true,
        data: Some(markets),
        message: Some(format!("Current markets for {} matches", markets_count)),
        pagination: None,
    })
}

// Get portfolio information
async fn get_portfolio(State(state): State<AppState>) -> Json<ApiResponse<PortfolioResponse>> {
    let summary = state.trading_engine.get_portfolio_summary().await;
    
    let portfolio = PortfolioResponse {
        total_bankroll: summary.total_bankroll.to_string(),
        available_bankroll: summary.available_bankroll.to_string(),
        total_exposure: summary.total_exposure.to_string(),
        active_bets_count: summary.active_bets_count,
        total_trades: summary.total_trades,
        roi: summary.roi,
        win_rate: summary.win_rate,
        profit_loss: summary.profit_loss.to_string(),
    };
    
    Json(ApiResponse {
        success: true,
        data: Some(portfolio),
        message: None,
        pagination: None,
    })
}

// Placeholder endpoints (to be implemented)
async fn get_recent_trades(State(_state): State<AppState>) -> Json<ApiResponse<Vec<serde_json::Value>>> {
    Json(ApiResponse {
        success: true,
        data: Some(vec![]),
        message: Some("Recent trades endpoint - TODO".to_string()),
        pagination: None,
    })
}

async fn get_trading_signals(State(_state): State<AppState>) -> Json<ApiResponse<Vec<serde_json::Value>>> {
    Json(ApiResponse {
        success: true,
        data: Some(vec![]),
        message: Some("Trading signals endpoint - TODO".to_string()),
        pagination: None,
    })
}

async fn get_performance_analytics(State(_state): State<AppState>) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse {
        success: true,
        data: Some(serde_json::json!({
            "message": "Performance analytics endpoint - TODO"
        })),
        message: Some("Analytics endpoint - TODO".to_string()),
        pagination: None,
    })
}

async fn get_model_performance(State(_state): State<AppState>) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse {
        success: true,
        data: Some(serde_json::json!({
            "message": "Model performance endpoint - TODO"
        })),
        message: Some("Model performance endpoint - TODO".to_string()),
        pagination: None,
    })
}

async fn start_simulation(State(_state): State<AppState>) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse {
        success: true,
        data: Some(serde_json::json!({
            "message": "Simulation control - TODO"
        })),
        message: Some("Simulation already running".to_string()),
        pagination: None,
    })
}

async fn stop_simulation(State(_state): State<AppState>) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse {
        success: true,
        data: Some(serde_json::json!({
            "message": "Simulation control - TODO"
        })),
        message: Some("Simulation control - TODO".to_string()),
        pagination: None,
    })
}

async fn get_simulation_status(State(_state): State<AppState>) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse {
        success: true,
        data: Some(serde_json::json!({
            "status": "running",
            "uptime": "unknown",
            "events_processed": 0
        })),
        message: Some("Simulation status".to_string()),
        pagination: None,
    })
}