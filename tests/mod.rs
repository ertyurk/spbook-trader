// Test module declarations

pub mod integration_tests;
pub mod ml_models_tests;  
pub mod trading_engine_tests;
pub mod performance_tests;

// Common test utilities and helpers
use chrono::Utc;
use uuid::Uuid;
use rust_decimal_macros::dec;

use quant_models::{MatchEvent, EventType, Sport, Prediction, SimpleMarketOdds};

/// Create a standard test match event for consistent testing
pub fn create_standard_test_event() -> MatchEvent {
    MatchEvent {
        id: Uuid::new_v4(),
        match_id: "standard_test_match".to_string(),
        event_type: EventType::Goal,
        timestamp: Utc::now(),
        sport: Sport::Football,
        league: "Test League".to_string(),
        team_home: "Home Team".to_string(),
        team_away: "Away Team".to_string(),
        score_home: Some(1),
        score_away: Some(0),
        minute: Some(45),
        player: Some("Test Player".to_string()),
        metadata: serde_json::Value::Null,
    }
}

/// Create a standard test prediction for consistent testing
pub fn create_standard_test_prediction() -> Prediction {
    Prediction::new(
        "standard_test_match".to_string(),
        "test_model".to_string(),
        "test_v1.0".to_string(),
        0.5,  // home_win_prob
        0.25, // away_win_prob
        Utc::now(),
    ).unwrap()
    .with_draw_prob(0.25).unwrap()
    .with_confidence(0.75).unwrap()
}

/// Create standard test market odds
pub fn create_standard_test_odds() -> SimpleMarketOdds {
    SimpleMarketOdds::new(
        dec!(2.0),
        dec!(3.5),
        dec!(3.0),
    )
}

#[cfg(test)]
mod test_helpers {
    use super::*;

    #[test]
    fn test_standard_helpers() {
        let event = create_standard_test_event();
        assert_eq!(event.match_id, "standard_test_match");
        assert_eq!(event.sport, Sport::Football);
        
        let prediction = create_standard_test_prediction();  
        assert_eq!(prediction.match_id, "standard_test_match");
        
        // Test probability values sum to approximately 1.0
        let total_prob = prediction.home_win_prob + 
                        prediction.away_win_prob + 
                        prediction.draw_prob.unwrap_or(0.0);
        assert!((total_prob - 1.0).abs() < 0.001);
        
        let odds = create_standard_test_odds();
        assert!(odds.home_win > dec!(0.0));
        assert!(odds.draw > dec!(0.0));
        assert!(odds.away_win > dec!(0.0));
    }
}