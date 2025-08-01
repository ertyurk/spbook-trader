use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::error::{QuantsError, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Prediction {
    pub id: Uuid,
    pub match_id: String,
    pub model_name: String,
    pub model_version: String,
    pub home_win_prob: f64,
    pub draw_prob: Option<f64>,
    pub away_win_prob: f64,
    pub confidence: f64,
    pub expected_goals_home: Option<f64>,
    pub expected_goals_away: Option<f64>,
    pub features_used: Vec<String>,
    pub prediction_timestamp: DateTime<Utc>,
    pub match_timestamp: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPerformance {
    pub model_name: String,
    pub model_version: String,
    pub total_predictions: u32,
    pub correct_predictions: u32,
    pub accuracy: f64,
    pub log_loss: f64,
    pub brier_score: f64,
    pub roi: f64,
    pub sharpe_ratio: f64,
    pub max_drawdown: f64,
    pub calibration_slope: f64,
    pub calibration_intercept: f64,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureVector {
    pub match_id: String,
    pub features: std::collections::HashMap<String, f64>,
    pub timestamp: DateTime<Utc>,
}

impl Prediction {
    pub fn new(
        match_id: String,
        model_name: String,
        model_version: String,
        home_win_prob: f64,
        away_win_prob: f64,
        match_timestamp: DateTime<Utc>,
    ) -> Result<Self> {
        if !(0.0..=1.0).contains(&home_win_prob) {
            return Err(QuantsError::InvalidProbability { prob: home_win_prob });
        }
        if !(0.0..=1.0).contains(&away_win_prob) {
            return Err(QuantsError::InvalidProbability { prob: away_win_prob });
        }
        
        let total_prob = home_win_prob + away_win_prob;
        if total_prob > 1.0 {
            return Err(QuantsError::InvalidProbability { prob: total_prob });
        }
        
        Ok(Self {
            id: Uuid::new_v4(),
            match_id,
            model_name,
            model_version,
            home_win_prob,
            draw_prob: if total_prob < 1.0 { Some(1.0 - total_prob) } else { None },
            away_win_prob,
            confidence: 0.0,
            expected_goals_home: None,
            expected_goals_away: None,
            features_used: Vec::new(),
            prediction_timestamp: Utc::now(),
            match_timestamp,
            metadata: serde_json::Value::Null,
        })
    }
    
    pub fn with_draw_prob(mut self, draw_prob: f64) -> Result<Self> {
        if !(0.0..=1.0).contains(&draw_prob) {
            return Err(QuantsError::InvalidProbability { prob: draw_prob });
        }
        
        let total = self.home_win_prob + self.away_win_prob + draw_prob;
        if (total - 1.0).abs() > 0.001 {
            return Err(QuantsError::InvalidProbability { prob: total });
        }
        
        self.draw_prob = Some(draw_prob);
        Ok(self)
    }
    
    pub fn with_confidence(mut self, confidence: f64) -> Result<Self> {
        if !(0.0..=1.0).contains(&confidence) {
            return Err(QuantsError::InvalidProbability { prob: confidence });
        }
        self.confidence = confidence;
        Ok(self)
    }
    
    pub fn with_expected_goals(mut self, home_goals: f64, away_goals: f64) -> Self {
        self.expected_goals_home = Some(home_goals);
        self.expected_goals_away = Some(away_goals);
        self
    }
    
    pub fn with_features(mut self, features: Vec<String>) -> Self {
        self.features_used = features;
        self
    }
    
    pub fn is_confident(&self, threshold: f64) -> bool {
        self.confidence >= threshold
    }
    
    pub fn most_likely_outcome(&self) -> PredictedOutcome {
        let home_prob = self.home_win_prob;
        let away_prob = self.away_win_prob;
        let draw_prob = self.draw_prob.unwrap_or(0.0);
        
        if home_prob >= away_prob && home_prob >= draw_prob {
            PredictedOutcome::HomeWin
        } else if away_prob >= draw_prob {
            PredictedOutcome::AwayWin
        } else {
            PredictedOutcome::Draw
        }
    }
    
    pub fn entropy(&self) -> f64 {
        let mut entropy = 0.0;
        
        if self.home_win_prob > 0.0 {
            entropy -= self.home_win_prob * self.home_win_prob.log2();
        }
        if self.away_win_prob > 0.0 {
            entropy -= self.away_win_prob * self.away_win_prob.log2();
        }
        if let Some(draw_prob) = self.draw_prob {
            if draw_prob > 0.0 {
                entropy -= draw_prob * draw_prob.log2();
            }
        }
        
        entropy
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PredictedOutcome {
    HomeWin,
    Draw,
    AwayWin,
}

impl ModelPerformance {
    pub fn new(model_name: String, model_version: String) -> Self {
        Self {
            model_name,
            model_version,
            total_predictions: 0,
            correct_predictions: 0,
            accuracy: 0.0,
            log_loss: 0.0,
            brier_score: 0.0,
            roi: 0.0,
            sharpe_ratio: 0.0,
            max_drawdown: 0.0,
            calibration_slope: 1.0,
            calibration_intercept: 0.0,
            last_updated: Utc::now(),
        }
    }
    
    pub fn update_accuracy(&mut self, is_correct: bool) {
        self.total_predictions += 1;
        if is_correct {
            self.correct_predictions += 1;
        }
        self.accuracy = self.correct_predictions as f64 / self.total_predictions as f64;
        self.last_updated = Utc::now();
    }
    
    pub fn update_brier_score(&mut self, predicted_prob: f64, actual_outcome: bool) {
        let outcome_value = if actual_outcome { 1.0 } else { 0.0 };
        let score = (predicted_prob - outcome_value).powi(2);
        
        // Running average of Brier score
        let weight = 1.0 / self.total_predictions as f64;
        self.brier_score = (1.0 - weight) * self.brier_score + weight * score;
    }
    
    pub fn is_well_calibrated(&self) -> bool {
        // A well-calibrated model should have slope close to 1 and intercept close to 0
        (self.calibration_slope - 1.0).abs() < 0.1 && self.calibration_intercept.abs() < 0.05
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    #[test]
    fn test_prediction_creation() {
        let match_timestamp = Utc::now() + Duration::hours(2);
        let prediction = Prediction::new(
            "match_123".to_string(),
            "LogisticRegression".to_string(),
            "v1.0".to_string(),
            0.6,
            0.3,
            match_timestamp,
        ).unwrap();
        
        assert_eq!(prediction.match_id, "match_123");
        assert_eq!(prediction.home_win_prob, 0.6);
        assert_eq!(prediction.away_win_prob, 0.3);
        assert_eq!(prediction.draw_prob, Some(0.1));
        assert_eq!(prediction.most_likely_outcome(), PredictedOutcome::HomeWin);
    }
    
    #[test]
    fn test_invalid_probabilities() {
        let match_timestamp = Utc::now() + Duration::hours(2);
        
        // Test probability > 1.0
        let result = Prediction::new(
            "match_123".to_string(),
            "LogisticRegression".to_string(),
            "v1.0".to_string(),
            1.5,
            0.3,
            match_timestamp,
        );
        assert!(result.is_err());
        
        // Test probabilities sum > 1.0
        let result = Prediction::new(
            "match_123".to_string(),
            "LogisticRegression".to_string(),
            "v1.0".to_string(),
            0.7,
            0.8,
            match_timestamp,
        );
        assert!(result.is_err());
    }
    
    #[test]
    fn test_entropy_calculation() {
        let match_timestamp = Utc::now() + Duration::hours(2);
        
        // High certainty prediction (low entropy)
        let certain_prediction = Prediction::new(
            "match_123".to_string(),
            "LogisticRegression".to_string(),
            "v1.0".to_string(),
            0.9,
            0.1,
            match_timestamp,
        ).unwrap();
        
        // Uncertain prediction (high entropy)
        let uncertain_prediction = Prediction::new(
            "match_456".to_string(),
            "LogisticRegression".to_string(),
            "v1.0".to_string(),
            0.5,
            0.5,
            match_timestamp,
        ).unwrap();
        
        assert!(certain_prediction.entropy() < uncertain_prediction.entropy());
    }
    
    #[test]
    fn test_model_performance() {
        let mut performance = ModelPerformance::new(
            "TestModel".to_string(),
            "v1.0".to_string(),
        );
        
        // Add some correct predictions
        performance.update_accuracy(true);
        performance.update_accuracy(true);
        performance.update_accuracy(false);
        
        assert_eq!(performance.total_predictions, 3);
        assert_eq!(performance.correct_predictions, 2);
        assert!((performance.accuracy - 0.6666666666666666).abs() < 0.0001);
    }
}