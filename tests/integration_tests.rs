use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;
use std::sync::Arc;
use tokio::sync::RwLock;
use rust_decimal_macros::dec;

use quant_api::{create_routes, AppState};
use quant_services::{TradingEngine, MarketSimulator, PredictorService};
use quant_models::{MatchEvent, Prediction, EventType, BettingOutcome, Sport};

#[tokio::test]
async fn test_health_endpoint() {
    let app_state = create_test_app_state().await;
    let app = create_routes().with_state(app_state);

    let response = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let health_response: Value = serde_json::from_slice(&body).unwrap();
    
    assert_eq!(health_response["status"], "healthy");
    assert!(health_response["timestamp"].is_string());
    assert!(health_response["version"].is_string());
}

#[tokio::test]
async fn test_system_status_endpoint() {
    let app_state = create_test_app_state().await;
    let app = create_routes().with_state(app_state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let status_response: Value = serde_json::from_slice(&body).unwrap();
    
    assert_eq!(status_response["success"], true);
    assert!(status_response["data"]["portfolio"].is_object());
    assert!(status_response["data"]["data_pipeline"].is_object());
    assert!(status_response["data"]["services"].is_object());
}

#[tokio::test]
async fn test_recent_events_endpoint() {
    let app_state = create_test_app_state().await;
    
    // Add test events
    let test_event = create_test_match_event();
    app_state.recent_events.write().await.push(test_event);
    
    let app = create_routes().with_state(app_state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/events?page=1&limit=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let events_response: Value = serde_json::from_slice(&body).unwrap();
    
    assert_eq!(events_response["success"], true);
    assert!(events_response["data"].is_array());
    assert!(events_response["pagination"].is_object());
    assert_eq!(events_response["pagination"]["page"], 1);
    assert_eq!(events_response["pagination"]["limit"], 10);
}

#[tokio::test]
async fn test_live_events_endpoint() {
    let app_state = create_test_app_state().await;
    
    // Add test events
    for i in 0..15 {
        let mut event = create_test_match_event();
        event.match_id = format!("match_{}", i);
        app_state.recent_events.write().await.push(event);
    }
    
    let app = create_routes().with_state(app_state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/events/live")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let events_response: Value = serde_json::from_slice(&body).unwrap();
    
    assert_eq!(events_response["success"], true);
    let events = events_response["data"].as_array().unwrap();
    assert_eq!(events.len(), 10); // Should return last 10 events
}

#[tokio::test]
async fn test_recent_predictions_endpoint() {
    let app_state = create_test_app_state().await;
    
    // Add test predictions
    let test_prediction = create_test_prediction();
    app_state.recent_predictions.write().await.push(test_prediction);
    
    let app = create_routes().with_state(app_state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/predictions?page=1&limit=5")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let predictions_response: Value = serde_json::from_slice(&body).unwrap();
    
    assert_eq!(predictions_response["success"], true);
    assert!(predictions_response["data"].is_array());
    assert!(predictions_response["pagination"].is_object());
}

#[tokio::test]
async fn test_prediction_by_match_endpoint() {
    let app_state = create_test_app_state().await;
    
    // Add test prediction with specific match ID
    let mut test_prediction = create_test_prediction();
    test_prediction.match_id = "specific_match_123".to_string();
    app_state.recent_predictions.write().await.push(test_prediction);
    
    let app = create_routes().with_state(app_state);

    // Test existing prediction
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/predictions/specific_match_123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let prediction_response: Value = serde_json::from_slice(&body).unwrap();
    
    assert_eq!(prediction_response["success"], true);
    assert_eq!(prediction_response["data"]["match_id"], "specific_match_123");

    // Test non-existing prediction
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/predictions/non_existing_match")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_portfolio_endpoint() {
    let app_state = create_test_app_state().await;
    let app = create_routes().with_state(app_state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/portfolio")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let portfolio_response: Value = serde_json::from_slice(&body).unwrap();
    
    assert_eq!(portfolio_response["success"], true);
    let portfolio_data = &portfolio_response["data"];
    assert!(portfolio_data["total_bankroll"].is_string());
    assert!(portfolio_data["available_bankroll"].is_string());
    assert!(portfolio_data["total_exposure"].is_string());
    assert!(portfolio_data["active_bets_count"].is_number());
    assert!(portfolio_data["total_trades"].is_number());
    assert!(portfolio_data["roi"].is_number());
    assert!(portfolio_data["win_rate"].is_number());
}

#[tokio::test]
async fn test_markets_endpoint() {
    let app_state = create_test_app_state().await;
    
    // Add test events to generate markets
    for i in 0..5 {
        let mut event = create_test_match_event();
        event.match_id = format!("market_match_{}", i);
        app_state.recent_events.write().await.push(event);
    }
    
    let app = create_routes().with_state(app_state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/markets")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let markets_response: Value = serde_json::from_slice(&body).unwrap();
    
    assert_eq!(markets_response["success"], true);
    assert!(markets_response["data"].is_object());
    assert!(markets_response["message"].as_str().unwrap().contains("Current markets"));
}

#[tokio::test]
async fn test_pagination_limits() {
    let app_state = create_test_app_state().await;
    
    // Add many test events
    for i in 0..150 {
        let mut event = create_test_match_event();
        event.match_id = format!("pagination_test_{}", i);
        app_state.recent_events.write().await.push(event);
    }
    
    let app = create_routes().with_state(app_state);

    // Test exceeding maximum limit
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/events?page=1&limit=200") // Exceeds max of 100
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let events_response: Value = serde_json::from_slice(&body).unwrap();
    
    assert_eq!(events_response["pagination"]["limit"], 100); // Should be capped at 100

    // Test multiple pages
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/events?page=2&limit=50")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let events_response: Value = serde_json::from_slice(&body).unwrap();
    
    assert_eq!(events_response["pagination"]["page"], 2);
    assert_eq!(events_response["pagination"]["limit"], 50);
    assert!(events_response["pagination"]["total"].as_u64().unwrap() >= 150);
}

#[tokio::test]
async fn test_error_handling() {
    let app_state = create_test_app_state().await;
    let app = create_routes().with_state(app_state);

    // Test invalid route
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/invalid_endpoint")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_concurrent_requests() {
    let app_state = create_test_app_state().await;
    let app = Arc::new(create_routes().with_state(app_state));

    // Add some test data
    for i in 0..10 {
        let mut event = create_test_match_event();
        event.match_id = format!("concurrent_test_{}", i);
        app.clone().layer().recent_events.write().await.push(event);
    }

    // Make multiple concurrent requests
    let mut handles = vec![];
    for _ in 0..10 {
        let app_clone = app.clone();
        let handle = tokio::spawn(async move {
            let response = app_clone
                .clone()
                .oneshot(
                    Request::builder()
                        .uri("/health")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        });
        handles.push(handle);
    }

    // Wait for all requests to complete
    for handle in handles {
        handle.await.unwrap();
    }
}

// Helper functions
async fn create_test_app_state() -> AppState {
    let trading_engine = Arc::new(TradingEngine::new(dec!(10000.0)));
    let market_simulator = Arc::new(MarketSimulator::new());
    let predictor = Arc::new(PredictorService::new());
    let recent_events = Arc::new(RwLock::new(Vec::new()));
    let recent_predictions = Arc::new(RwLock::new(Vec::new()));

    AppState {
        trading_engine,
        market_simulator,
        predictor,
        recent_events,
        recent_predictions,
    }
}

fn create_test_match_event() -> MatchEvent {
    MatchEvent {
        id: uuid::Uuid::new_v4(),
        match_id: "test_match_123".to_string(),
        event_type: EventType::Goal,
        timestamp: chrono::Utc::now(),
        sport: Sport::Football,
        league: "Test League".to_string(),
        team_home: "Test Team A".to_string(),
        team_away: "Test Team B".to_string(),
        score_home: Some(1),
        score_away: Some(0),
        minute: Some(45),
        player: Some("Test Player".to_string()),
        metadata: std::collections::HashMap::new(),
    }
}

fn create_test_prediction() -> Prediction {
    use std::collections::HashMap;
    
    let mut probabilities = HashMap::new();
    probabilities.insert(BettingOutcome::HomeWin, 0.4);
    probabilities.insert(BettingOutcome::Draw, 0.3);
    probabilities.insert(BettingOutcome::AwayWin, 0.3);

    Prediction {
        id: uuid::Uuid::new_v4(),
        match_id: "test_match_123".to_string(),
        model_version: "test_v1.0".to_string(),
        timestamp: chrono::Utc::now(),
        probabilities,
        confidence: 0.75,
        expected_value: 0.15,
        recommended_bet: Some(BettingOutcome::HomeWin),
        stake_percentage: Some(0.02),
        metadata: HashMap::new(),
    }
}