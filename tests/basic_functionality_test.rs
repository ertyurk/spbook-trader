// Basic functionality test to verify core system works
use quant_models::{Prediction, SimpleMarketOdds};
use rust_decimal_macros::dec;
use rust_decimal::prelude::ToPrimitive;

#[tokio::test]
async fn test_basic_prediction_creation() {
    // Test that we can create a valid prediction
    let prediction = Prediction::new(
        "test_match".to_string(),
        "test_model".to_string(),
        "v1.0".to_string(),
        0.4,  // home win probability
        0.3,  // away win probability  
        chrono::Utc::now(),
    ).unwrap();
    
    // Validate prediction properties
    assert_eq!(prediction.match_id, "test_match");
    assert_eq!(prediction.model_name, "test_model");
    assert_eq!(prediction.home_win_prob, 0.4);
    assert_eq!(prediction.away_win_prob, 0.3);
    assert!((prediction.draw_prob.unwrap() - 0.3).abs() < 0.001); // Should auto-calculate draw probability
    
    // Test probabilities sum to 1.0
    let total = prediction.home_win_prob + prediction.away_win_prob + prediction.draw_prob.unwrap();
    assert!((total - 1.0).abs() < 0.001);
}

#[tokio::test]
async fn test_prediction_with_confidence() {
    let prediction = Prediction::new(
        "test_match".to_string(),
        "test_model".to_string(),
        "v1.0".to_string(),
        0.5,
        0.2,
        chrono::Utc::now(),
    ).unwrap()
    .with_confidence(0.8).unwrap();
    
    assert_eq!(prediction.confidence, 0.8);
    assert!((prediction.draw_prob.unwrap() - 0.3).abs() < 0.001);
}

#[tokio::test]
async fn test_market_odds_creation() {
    let odds = SimpleMarketOdds::new(
        dec!(2.0),  // home win
        dec!(3.5),  // draw
        dec!(4.0),  // away win
    );
    
    assert_eq!(odds.home_win, dec!(2.0));
    assert_eq!(odds.draw, dec!(3.5));
    assert_eq!(odds.away_win, dec!(4.0));
}

#[tokio::test]
async fn test_odds_from_probabilities() {
    let odds = SimpleMarketOdds::from_probabilities(
        0.4,  // 40% home win
        0.3,  // 30% draw
        0.3,  // 30% away win
        0.05, // 5% margin
    );
    
    // With margin, odds should be slightly lower than fair odds
    // Fair odds for 40% = 2.5, with margin should be lower
    assert!(odds.home_win < dec!(2.5));
    assert!(odds.home_win > dec!(2.0));
}

#[tokio::test]
async fn test_invalid_probabilities() {
    // Test that invalid probabilities are rejected
    let result = Prediction::new(
        "test".to_string(),
        "model".to_string(),
        "v1.0".to_string(),
        1.5,  // Invalid probability > 1.0
        0.3,
        chrono::Utc::now(),
    );
    
    assert!(result.is_err());
}

#[tokio::test]
async fn test_probabilities_sum_too_high() {
    // Test that probabilities summing > 1.0 are rejected
    let result = Prediction::new(
        "test".to_string(),
        "model".to_string(), 
        "v1.0".to_string(),
        0.8,  // 80% home
        0.8,  // 80% away = 160% total
        chrono::Utc::now(),
    );
    
    assert!(result.is_err());
}

#[tokio::test]
async fn test_system_integration_basic() {
    // Test that our core components can work together
    let prediction = Prediction::new(
        "integration_test".to_string(),
        "ensemble_v1".to_string(),
        "v1.0".to_string(),
        0.45,
        0.35,
        chrono::Utc::now(),
    ).unwrap()
    .with_confidence(0.75).unwrap();
    
    let odds = SimpleMarketOdds::from_probabilities(
        prediction.home_win_prob,
        prediction.draw_prob.unwrap(),
        prediction.away_win_prob,
        0.1, // 10% bookmaker margin
    );
    
    // Verify the system produces reasonable values
    assert!(prediction.confidence > 0.0);
    assert!(odds.home_win > dec!(1.0));
    assert!(odds.draw > dec!(1.0));
    assert!(odds.away_win > dec!(1.0));
    
    // With 10% margin, implied probabilities should sum to ~1.1
    let implied_home = 1.0 / odds.home_win.to_f64().unwrap();
    let implied_draw = 1.0 / odds.draw.to_f64().unwrap();
    let implied_away = 1.0 / odds.away_win.to_f64().unwrap();
    let total_implied = implied_home + implied_draw + implied_away;
    
    assert!(total_implied > 1.05); // Should have overround due to margin
    assert!(total_implied < 1.15); // But not excessive
}

#[test]
fn test_compilation_success() {
    // This test simply verifies that the basic types compile correctly
    println!("✅ Basic functionality tests compile successfully");
}