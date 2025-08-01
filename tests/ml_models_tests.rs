use std::collections::HashMap;
use quant_ml::models::{LogisticRegressionModel, PoissonModel, EnsembleModel, Model, ModelFeedback};
use quant_models::{FeatureVector, BettingOutcome};
use chrono::Utc;
use uuid::Uuid;

#[tokio::test]
async fn test_logistic_regression_model_creation() {
    let model = LogisticRegressionModel::new();
    assert_eq!(model.model_name(), "LogisticRegression");
    assert_eq!(model.model_version(), "v1.0");
}

#[tokio::test]
async fn test_logistic_regression_prediction() {
    let model = LogisticRegressionModel::new();
    let features = create_test_feature_vector();
    
    let prediction = model.predict(&features).await.unwrap();
    
    // Test basic prediction properties
    assert_eq!(prediction.match_id, "test_match_123");
    assert_eq!(prediction.model_version, "v1.0");
    assert!(prediction.timestamp <= Utc::now());
    
    // Test probability constraints
    assert!(prediction.home_win_prob >= 0.01 && prediction.home_win_prob <= 0.98);
    assert!(prediction.away_win_prob >= 0.01 && prediction.away_win_prob <= 0.98);
    assert!(prediction.draw_prob.unwrap() >= 0.01 && prediction.draw_prob.unwrap() <= 0.98);
    
    // Test probabilities sum to approximately 1
    let total_prob = prediction.home_win_prob + 
                    prediction.away_win_prob + 
                    prediction.draw_prob.unwrap();
    assert!((total_prob - 1.0).abs() < 0.001);
    
    // Test confidence is in valid range
    assert!(prediction.confidence >= 0.0 && prediction.confidence <= 1.0);
}

#[tokio::test]
async fn test_logistic_regression_different_features() {
    let model = LogisticRegressionModel::new();
    
    // Test with strong home advantage features
    let mut strong_home_features = create_test_feature_vector();
    strong_home_features.features.insert("home_advantage".to_string(), 2.0);
    strong_home_features.features.insert("elo_difference".to_string(), 200.0);
    strong_home_features.features.insert("form_difference".to_string(), 1.5);
    
    let strong_home_pred = model.predict(&strong_home_features).await.unwrap();
    
    // Test with strong away advantage features
    let mut strong_away_features = create_test_feature_vector();
    strong_away_features.features.insert("home_advantage".to_string(), 0.5);
    strong_away_features.features.insert("elo_difference".to_string(), -200.0);
    strong_away_features.features.insert("form_difference".to_string(), -1.5);
    
    let strong_away_pred = model.predict(&strong_away_features).await.unwrap();
    
    // The model should produce different predictions for different features
    assert_ne!(strong_home_pred.home_win_prob, strong_away_pred.home_win_prob);
    assert_ne!(strong_home_pred.away_win_prob, strong_away_pred.away_win_prob);
}

#[tokio::test]
async fn test_poisson_model_creation() {
    let model = PoissonModel::new();
    assert_eq!(model.model_name(), "PoissonGoals");
    assert_eq!(model.model_version(), "v1.0");
}

#[tokio::test]
async fn test_poisson_model_prediction() {
    let model = PoissonModel::new();
    let features = create_test_feature_vector();
    
    let prediction = model.predict(&features).await.unwrap();
    
    // Test basic prediction properties
    assert_eq!(prediction.match_id, "test_match_123");
    assert_eq!(prediction.model_version, "v1.0");
    
    // Test probability constraints
    assert!(prediction.home_win_prob >= 0.01 && prediction.home_win_prob <= 0.98);
    assert!(prediction.away_win_prob >= 0.01 && prediction.away_win_prob <= 0.98);
    assert!(prediction.draw_prob.unwrap() >= 0.01 && prediction.draw_prob.unwrap() <= 0.98);
    
    // Test probabilities sum to approximately 1
    let total_prob = prediction.home_win_prob + 
                    prediction.away_win_prob + 
                    prediction.draw_prob.unwrap();
    assert!((total_prob - 1.0).abs() < 0.001);
    
    // Test confidence is in valid range
    assert!(prediction.confidence >= 0.0 && prediction.confidence <= 1.0);
}

#[tokio::test]
async fn test_poisson_model_attack_defense_features() {
    let model = PoissonModel::new();
    
    // Test with strong attacking features
    let mut high_scoring_features = create_test_feature_vector();
    high_scoring_features.features.insert("home_attack".to_string(), 2.0);
    high_scoring_features.features.insert("away_attack".to_string(), 2.0);
    high_scoring_features.features.insert("home_defense".to_string(), 0.5);
    high_scoring_features.features.insert("away_defense".to_string(), 0.5);
    
    let high_scoring_pred = model.predict(&high_scoring_features).await.unwrap();
    
    // Test with defensive features
    let mut low_scoring_features = create_test_feature_vector();
    low_scoring_features.features.insert("home_attack".to_string(), 0.5);
    low_scoring_features.features.insert("away_attack".to_string(), 0.5);
    low_scoring_features.features.insert("home_defense".to_string(), 2.0);
    low_scoring_features.features.insert("away_defense".to_string(), 2.0);
    
    let low_scoring_pred = model.predict(&low_scoring_features).await.unwrap();
    
    // High scoring games should generally have lower draw probability
    // (more likely to have a decisive result)
    assert!(high_scoring_pred.draw_prob.unwrap() != low_scoring_pred.draw_prob.unwrap());
}

#[tokio::test]
async fn test_ensemble_model_creation() {
    let model = EnsembleModel::new();
    assert_eq!(model.model_name(), "EnsembleModel");
    assert_eq!(model.model_version(), "v1.0");
}

#[tokio::test]
async fn test_ensemble_model_prediction() {
    let model = EnsembleModel::new();
    let features = create_test_feature_vector();
    
    let prediction = model.predict(&features).await.unwrap();
    
    // Test basic prediction properties
    assert_eq!(prediction.match_id, "test_match_123");
    assert_eq!(prediction.model_version, "v1.0");
    
    // Test probability constraints
    assert!(prediction.home_win_prob >= 0.01 && prediction.home_win_prob <= 0.98);
    assert!(prediction.away_win_prob >= 0.01 && prediction.away_win_prob <= 0.98);
    assert!(prediction.draw_prob.unwrap() >= 0.01 && prediction.draw_prob.unwrap() <= 0.98);
    
    // Test probabilities sum to approximately 1
    let total_prob = prediction.home_win_prob + 
                    prediction.away_win_prob + 
                    prediction.draw_prob.unwrap();
    assert!((total_prob - 1.0).abs() < 0.001);
    
    // Test confidence is in valid range
    assert!(prediction.confidence >= 0.0 && prediction.confidence <= 1.0);
}

#[tokio::test]
async fn test_ensemble_combines_models() {
    let ensemble = EnsembleModel::new();
    let logistic = LogisticRegressionModel::new();
    let poisson = PoissonModel::new();
    
    let features = create_test_feature_vector();
    
    let ensemble_pred = ensemble.predict(&features).await.unwrap();
    let logistic_pred = logistic.predict(&features).await.unwrap();
    let poisson_pred = poisson.predict(&features).await.unwrap();
    
    // Ensemble prediction should be between the individual model predictions
    // (weighted average with 60% logistic, 40% poisson)
    let expected_home_win = (logistic_pred.home_win_prob * 0.6 + poisson_pred.home_win_prob * 0.4) / 1.0;
    let expected_away_win = (logistic_pred.away_win_prob * 0.6 + poisson_pred.away_win_prob * 0.4) / 1.0;
    
    // Allow for small differences due to normalization
    assert!((ensemble_pred.home_win_prob - expected_home_win).abs() < 0.1);
    assert!((ensemble_pred.away_win_prob - expected_away_win).abs() < 0.1);
}

#[tokio::test]
async fn test_model_feedback_updates() {
    let mut model = LogisticRegressionModel::new();
    
    // Get initial weights by making a prediction
    let features = create_test_feature_vector();
    let initial_pred = model.predict(&features).await.unwrap();
    
    // Create positive feedback
    let positive_feedback = ModelFeedback {
        prediction_id: Uuid::new_v4(),
        actual_outcome: true,
        reward: 1.0,
    };
    
    // Update model with positive feedback
    model.update_weights(&positive_feedback).await.unwrap();
    
    // Create negative feedback
    let negative_feedback = ModelFeedback {
        prediction_id: Uuid::new_v4(),
        actual_outcome: false,
        reward: -0.5,
    };
    
    // Update model with negative feedback
    model.update_weights(&negative_feedback).await.unwrap();
    
    // The model should still produce valid predictions after updates
    let updated_pred = model.predict(&features).await.unwrap();
    
    assert!(updated_pred.home_win_prob >= 0.01 && updated_pred.home_win_prob <= 0.98);
    assert!(updated_pred.away_win_prob >= 0.01 && updated_pred.away_win_prob <= 0.98);
    assert!(updated_pred.draw_prob.unwrap() >= 0.01 && updated_pred.draw_prob.unwrap() <= 0.98);
    
    let total_prob = updated_pred.home_win_prob + 
                    updated_pred.away_win_prob + 
                    updated_pred.draw_prob.unwrap();
    assert!((total_prob - 1.0).abs() < 0.001);
}

#[tokio::test]
async fn test_poisson_model_weight_updates() {
    let mut model = PoissonModel::new();
    
    let features = create_test_feature_vector();
    let initial_pred = model.predict(&features).await.unwrap();
    
    // Apply multiple positive feedbacks
    for _ in 0..5 {
        let feedback = ModelFeedback {
            prediction_id: Uuid::new_v4(),
            actual_outcome: true,
            reward: 0.8,
        };
        model.update_weights(&feedback).await.unwrap();
    }
    
    let updated_pred = model.predict(&features).await.unwrap();
    
    // Model should still produce valid predictions
    assert!(updated_pred.home_win_prob >= 0.01 && updated_pred.home_win_prob <= 0.98);
    assert!(updated_pred.away_win_prob >= 0.01 && updated_pred.away_win_prob <= 0.98);
    assert!(updated_pred.confidence >= 0.0 && updated_pred.confidence <= 1.0);
}

#[tokio::test]
async fn test_model_enum_interface() {
    let logistic_model = Model::LogisticRegression(LogisticRegressionModel::new());
    let poisson_model = Model::Poisson(PoissonModel::new());
    let ensemble_model = Model::Ensemble(EnsembleModel::new());
    
    let features = create_test_feature_vector();
    
    // Test that all models can be used through the enum interface
    let logistic_pred = logistic_model.predict(&features).await.unwrap();
    let poisson_pred = poisson_model.predict(&features).await.unwrap();
    let ensemble_pred = ensemble_model.predict(&features).await.unwrap();
    
    // All predictions should be valid
    for pred in vec![&logistic_pred, &poisson_pred, &ensemble_pred] {
        assert!(pred.home_win_prob >= 0.01 && pred.home_win_prob <= 0.98);
        assert!(pred.away_win_prob >= 0.01 && pred.away_win_prob <= 0.98);
        assert!(pred.confidence >= 0.0 && pred.confidence <= 1.0);
        
        let total_prob = pred.home_win_prob + 
                        pred.away_win_prob + 
                        pred.draw_prob.unwrap_or(0.0);
        assert!((total_prob - 1.0).abs() < 0.001);
    }
    
    // Model names should be correct
    assert_eq!(logistic_model.model_name(), "LogisticRegression");
    assert_eq!(poisson_model.model_name(), "PoissonGoals");
    assert_eq!(ensemble_model.model_name(), "EnsembleModel");
}

#[tokio::test]
async fn test_prediction_consistency() {
    let model = LogisticRegressionModel::new();
    let features = create_test_feature_vector();
    
    // Make multiple predictions with the same features
    let pred1 = model.predict(&features).await.unwrap();
    let pred2 = model.predict(&features).await.unwrap();
    let pred3 = model.predict(&features).await.unwrap();
    
    // Predictions should be identical (deterministic)
    assert_eq!(pred1.home_win_prob, pred2.home_win_prob);
    assert_eq!(pred2.home_win_prob, pred3.home_win_prob);
    assert_eq!(pred1.away_win_prob, pred2.away_win_prob);
    assert_eq!(pred2.away_win_prob, pred3.away_win_prob);
    assert_eq!(pred1.confidence, pred2.confidence);
    assert_eq!(pred2.confidence, pred3.confidence);
}

#[tokio::test]
async fn test_extreme_feature_values() {
    let model = LogisticRegressionModel::new();
    
    // Test with extreme positive values
    let mut extreme_positive_features = create_test_feature_vector();
    for (_, value) in extreme_positive_features.features.iter_mut() {
        *value = 1000.0;
    }
    
    let extreme_pos_pred = model.predict(&extreme_positive_features).await.unwrap();
    
    // Even with extreme values, probabilities should be valid
    assert!(extreme_pos_pred.home_win_prob >= 0.01 && extreme_pos_pred.home_win_prob <= 0.98);
    assert!(extreme_pos_pred.away_win_prob >= 0.01 && extreme_pos_pred.away_win_prob <= 0.98);
    assert!(extreme_pos_pred.draw_prob.unwrap() >= 0.01 && extreme_pos_pred.draw_prob.unwrap() <= 0.98);
    
    // Test with extreme negative values
    let mut extreme_negative_features = create_test_feature_vector();
    for (_, value) in extreme_negative_features.features.iter_mut() {
        *value = -1000.0;
    }
    
    let extreme_neg_pred = model.predict(&extreme_negative_features).await.unwrap();
    
    // Even with extreme values, probabilities should be valid
    assert!(extreme_neg_pred.home_win_prob >= 0.01 && extreme_neg_pred.home_win_prob <= 0.98);
    assert!(extreme_neg_pred.away_win_prob >= 0.01 && extreme_neg_pred.away_win_prob <= 0.98);
    assert!(extreme_neg_pred.draw_prob.unwrap() >= 0.01 && extreme_neg_pred.draw_prob.unwrap() <= 0.98);
}

#[tokio::test]
async fn test_missing_features() {
    let model = LogisticRegressionModel::new();
    
    // Create feature vector with only some features
    let mut sparse_features = HashMap::new();
    sparse_features.insert("minute".to_string(), 45.0);
    sparse_features.insert("home_score".to_string(), 1.0);
    sparse_features.insert("away_score".to_string(), 0.0);
    
    let feature_vector = FeatureVector {
        match_id: "sparse_test_123".to_string(),
        timestamp: Utc::now(),
        features: sparse_features,
    };
    
    let prediction = model.predict(&feature_vector).await.unwrap();
    
    // Model should handle missing features gracefully (defaults to 0.0)
    assert!(prediction.home_win_prob >= 0.01 && prediction.home_win_prob <= 0.98);
    assert!(prediction.away_win_prob >= 0.01 && prediction.away_win_prob <= 0.98);
    assert!(prediction.confidence >= 0.0 && prediction.confidence <= 1.0);
}

// Helper function to create test feature vector
fn create_test_feature_vector() -> FeatureVector {
    let mut features = HashMap::new();
    
    // Basic match state
    features.insert("minute".to_string(), 45.0);
    features.insert("home_score".to_string(), 1.0);
    features.insert("away_score".to_string(), 0.0);
    features.insert("score_difference".to_string(), 1.0);
    features.insert("total_goals".to_string(), 1.0);
    
    // Match dynamics
    features.insert("momentum".to_string(), 0.6);
    features.insert("intensity".to_string(), 0.8);
    features.insert("game_phase".to_string(), 0.5); // Mid-game
    features.insert("time_pressure".to_string(), 0.3);
    
    // Team ratings
    features.insert("home_elo".to_string(), 1600.0);
    features.insert("away_elo".to_string(), 1550.0);
    features.insert("elo_difference".to_string(), 50.0);
    
    // Team attributes
    features.insert("home_attack".to_string(), 1.2);
    features.insert("home_defense".to_string(), 1.1);
    features.insert("away_attack".to_string(), 1.0);
    features.insert("away_defense".to_string(), 0.9);
    
    // Expected goals
    features.insert("home_expected_goals".to_string(), 1.3);
    features.insert("away_expected_goals".to_string(), 1.1);
    
    // Form and discipline
    features.insert("home_form".to_string(), 0.7);
    features.insert("away_form".to_string(), 0.5);
    features.insert("form_difference".to_string(), 0.2);
    features.insert("home_discipline".to_string(), 0.8);
    features.insert("away_discipline".to_string(), 0.9);
    
    // Match context
    features.insert("match_status".to_string(), 1.0); // Active
    features.insert("event_influence".to_string(), 0.5);
    features.insert("home_advantage".to_string(), 1.2);
    
    // Temporal features
    features.insert("hour_of_day".to_string(), 15.0); // 3 PM
    features.insert("is_evening".to_string(), 0.0);
    features.insert("day_of_week".to_string(), 6.0); // Saturday
    
    // League
    features.insert("league_competitiveness".to_string(), 0.8);
    
    FeatureVector {
        match_id: "test_match_123".to_string(),
        timestamp: Utc::now(),
        features,
    }
}