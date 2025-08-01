use quant_models::{Prediction, FeatureVector};
use anyhow::Result;
use chrono::Utc;
use nalgebra::DVector;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use rand::Rng;

pub struct ModelFeedback {
    pub prediction_id: uuid::Uuid,
    pub actual_outcome: bool,
    pub reward: f64,
}

#[derive(Debug)]
pub enum Model {
    LogisticRegression(LogisticRegressionModel),
    Poisson(PoissonModel),
    Ensemble(EnsembleModel),
}

impl Model {
    pub fn model_name(&self) -> &str {
        match self {
            Model::LogisticRegression(m) => m.model_name(),
            Model::Poisson(m) => m.model_name(),
            Model::Ensemble(m) => m.model_name(),
        }
    }
    
    pub fn model_version(&self) -> &str {
        match self {
            Model::LogisticRegression(m) => m.model_version(),
            Model::Poisson(m) => m.model_version(),
            Model::Ensemble(m) => m.model_version(),
        }
    }
    
    pub async fn predict(&self, features: &FeatureVector) -> Result<Prediction> {
        match self {
            Model::LogisticRegression(m) => m.predict(features).await,
            Model::Poisson(m) => m.predict(features).await,
            Model::Ensemble(m) => m.predict(features).await,
        }
    }
    
    pub async fn update_weights(&mut self, feedback: &ModelFeedback) -> Result<()> {
        match self {
            Model::LogisticRegression(m) => m.update_weights(feedback).await,
            Model::Poisson(m) => m.update_weights(feedback).await,
            Model::Ensemble(m) => m.update_weights(feedback).await,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModelWeights {
    pub home_win: DVector<f64>,
    pub draw: DVector<f64>,
    pub away_win: DVector<f64>,
    pub learning_rate: f64,
    pub regularization: f64,
}

impl Default for ModelWeights {
    fn default() -> Self {
        let size = 30; // Number of features we expect
        Self {
            home_win: DVector::from_fn(size, |_, _| rand::thread_rng().gen_range(-0.01..0.01)),
            draw: DVector::from_fn(size, |_, _| rand::thread_rng().gen_range(-0.01..0.01)),
            away_win: DVector::from_fn(size, |_, _| rand::thread_rng().gen_range(-0.01..0.01)),
            learning_rate: 0.001,
            regularization: 0.01,
        }
    }
}

#[derive(Debug)]
pub struct LogisticRegressionModel {
    name: String,
    version: String,
    weights: Arc<RwLock<ModelWeights>>,
    feature_names: Vec<String>,
}

impl LogisticRegressionModel {
    pub fn new() -> Self {
        let feature_names = vec![
            "minute".to_string(),
            "home_score".to_string(),
            "away_score".to_string(),
            "score_difference".to_string(),
            "total_goals".to_string(),
            "momentum".to_string(),
            "intensity".to_string(),
            "game_phase".to_string(),
            "time_pressure".to_string(),
            "home_elo".to_string(),
            "away_elo".to_string(),
            "elo_difference".to_string(),
            "home_attack".to_string(),
            "home_defense".to_string(),
            "away_attack".to_string(),
            "away_defense".to_string(),
            "home_expected_goals".to_string(),
            "away_expected_goals".to_string(),
            "home_form".to_string(),
            "away_form".to_string(),
            "form_difference".to_string(),
            "home_discipline".to_string(),
            "away_discipline".to_string(),
            "match_status".to_string(),
            "event_influence".to_string(),
            "home_advantage".to_string(),
            "hour_of_day".to_string(),
            "is_evening".to_string(),
            "day_of_week".to_string(),
            "league_competitiveness".to_string(),
        ];
        
        Self {
            name: "LogisticRegression".to_string(),
            version: "v1.0".to_string(),
            weights: Arc::new(RwLock::new(ModelWeights::default())),
            feature_names,
        }
    }
    
    fn extract_feature_vector(&self, features: &FeatureVector) -> DVector<f64> {
        let mut feature_vec = Vec::with_capacity(self.feature_names.len());
        
        for feature_name in &self.feature_names {
            let value = features.features.get(feature_name).copied().unwrap_or(0.0);
            feature_vec.push(value);
        }
        
        DVector::from_vec(feature_vec)
    }
    
    fn sigmoid(&self, x: f64) -> f64 {
        1.0 / (1.0 + (-x).exp())
    }
    
    fn softmax(&self, logits: &[f64]) -> Vec<f64> {
        let max_logit = logits.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let exp_logits: Vec<f64> = logits.iter().map(|&x| (x - max_logit).exp()).collect();
        let sum_exp: f64 = exp_logits.iter().sum();
        exp_logits.iter().map(|&x| x / sum_exp).collect()
    }
}

impl LogisticRegressionModel {
    fn model_name(&self) -> &str {
        &self.name
    }
    
    fn model_version(&self) -> &str {
        &self.version
    }
    
    async fn predict(&self, features: &FeatureVector) -> Result<Prediction> {
        let feature_vec = self.extract_feature_vector(features);
        let weights = self.weights.read().unwrap();
        
        // Calculate logits for each outcome
        let home_logit = weights.home_win.dot(&feature_vec);
        let draw_logit = weights.draw.dot(&feature_vec);
        let away_logit = weights.away_win.dot(&feature_vec);
        
        // Apply softmax to get probabilities
        let logits = vec![home_logit, draw_logit, away_logit];
        let probabilities = self.softmax(&logits);
        
        let home_win_prob = probabilities[0].max(0.01).min(0.98);
        let draw_prob = probabilities[1].max(0.01).min(0.98);
        let away_win_prob = probabilities[2].max(0.01).min(0.98);
        
        // Normalize to ensure they sum to ~1.0
        let total = home_win_prob + draw_prob + away_win_prob;
        let home_win_prob = home_win_prob / total;
        let draw_prob = draw_prob / total;
        let away_win_prob = away_win_prob / total;
        
        // Calculate confidence based on entropy
        let entropy = -probabilities.iter()
            .filter(|&&p| p > 0.0)
            .map(|&p| p * p.ln())
            .sum::<f64>();
        let max_entropy = (3.0_f64).ln(); // log(3) for 3 outcomes
        let confidence = 1.0 - (entropy / max_entropy);
        
        let prediction = Prediction::new(
            features.match_id.clone(),
            self.model_name().to_string(),
            self.model_version().to_string(),
            home_win_prob,
            away_win_prob,
            features.timestamp,
        )?
        .with_draw_prob(draw_prob)?
        .with_confidence(confidence)?
        .with_features(self.feature_names.clone());
        
        Ok(prediction)
    }
    
    async fn update_weights(&mut self, feedback: &ModelFeedback) -> Result<()> {
        // Simplified weight update using gradient descent
        // In a real implementation, you'd store the features used for each prediction
        // and use them here for proper gradient calculation
        
        let mut weights = self.weights.write().unwrap();
        let adjustment = feedback.reward * weights.learning_rate;
        
        // Apply small adjustments to weights based on feedback
        if feedback.actual_outcome {
            // Positive outcome - slightly increase all weights
            weights.home_win *= 1.0 + adjustment * 0.1;
            weights.draw *= 1.0 + adjustment * 0.05;
            weights.away_win *= 1.0 + adjustment * 0.1;
        } else {
            // Negative outcome - slightly decrease weights
            weights.home_win *= 1.0 - adjustment * 0.1;
            weights.draw *= 1.0 - adjustment * 0.05;
            weights.away_win *= 1.0 - adjustment * 0.1;
        }
        
        Ok(())
    }
}

#[derive(Debug)]
pub struct PoissonModel {
    name: String,
    version: String,
    lambda_home: Arc<RwLock<f64>>,
    lambda_away: Arc<RwLock<f64>>,
}

impl PoissonModel {
    pub fn new() -> Self {
        Self {
            name: "PoissonGoals".to_string(),
            version: "v1.0".to_string(),
            lambda_home: Arc::new(RwLock::new(1.4)), // Average goals per team
            lambda_away: Arc::new(RwLock::new(1.3)),
        }
    }
    
    fn poisson_probability(&self, lambda: f64, k: u32) -> f64 {
        let e_neg_lambda = (-lambda).exp();
        let lambda_k = lambda.powi(k as i32);
        let k_factorial = (1..=k).fold(1.0, |acc, x| acc * x as f64);
        
        (e_neg_lambda * lambda_k) / k_factorial
    }
    
    fn calculate_match_probabilities(&self, lambda_home: f64, lambda_away: f64) -> (f64, f64, f64) {
        let mut home_win = 0.0;
        let mut draw = 0.0;
        let mut away_win = 0.0;
        
        // Calculate probabilities for scores up to 6 goals each (covers ~99% of matches)
        for home_goals in 0..=6 {
            for away_goals in 0..=6 {
                let prob = self.poisson_probability(lambda_home, home_goals) 
                         * self.poisson_probability(lambda_away, away_goals);
                
                if home_goals > away_goals {
                    home_win += prob;
                } else if home_goals == away_goals {
                    draw += prob;
                } else {
                    away_win += prob;
                }
            }
        }
        
        (home_win, draw, away_win)
    }
}

impl PoissonModel {
    fn model_name(&self) -> &str {
        &self.name
    }
    
    fn model_version(&self) -> &str {
        &self.version
    }
    
    async fn predict(&self, features: &FeatureVector) -> Result<Prediction> {
        // Adjust lambda based on features
        let base_lambda_home = *self.lambda_home.read().unwrap();
        let base_lambda_away = *self.lambda_away.read().unwrap();
        
        let home_attack = features.features.get("home_attack").copied().unwrap_or(1.0);
        let away_attack = features.features.get("away_attack").copied().unwrap_or(1.0);
        let home_defense = features.features.get("home_defense").copied().unwrap_or(1.0);
        let away_defense = features.features.get("away_defense").copied().unwrap_or(1.0);
        let home_advantage = features.features.get("home_advantage").copied().unwrap_or(1.0);
        
        let adjusted_lambda_home = base_lambda_home * home_attack * away_defense * home_advantage;
        let adjusted_lambda_away = base_lambda_away * away_attack * home_defense;
        
        let (mut home_win_prob, mut draw_prob, mut away_win_prob) = 
            self.calculate_match_probabilities(adjusted_lambda_home, adjusted_lambda_away);
        
        // Ensure probabilities are in valid range
        home_win_prob = home_win_prob.max(0.01).min(0.98);
        draw_prob = draw_prob.max(0.01).min(0.98);
        away_win_prob = away_win_prob.max(0.01).min(0.98);
        
        // Normalize
        let total = home_win_prob + draw_prob + away_win_prob;
        home_win_prob /= total;
        draw_prob /= total;
        away_win_prob /= total;
        
        // Confidence based on how different the lambdas are (more different = more confident)
        let lambda_diff = (adjusted_lambda_home - adjusted_lambda_away).abs();
        let confidence = (lambda_diff / 2.0).min(1.0).max(0.5);
        
        let prediction = Prediction::new(
            features.match_id.clone(),
            self.model_name().to_string(),
            self.model_version().to_string(),
            home_win_prob,
            away_win_prob,
            features.timestamp,
        )?
        .with_draw_prob(draw_prob)?
        .with_confidence(confidence)?
        .with_expected_goals(adjusted_lambda_home, adjusted_lambda_away);
        
        Ok(prediction)
    }
    
    async fn update_weights(&mut self, feedback: &ModelFeedback) -> Result<()> {
        // Update lambda values based on feedback
        let adjustment = feedback.reward * 0.01; // Small learning rate
        
        let mut lambda_home = self.lambda_home.write().unwrap();
        let mut lambda_away = self.lambda_away.write().unwrap();
        
        if feedback.actual_outcome {
            *lambda_home += adjustment;
            *lambda_away += adjustment * 0.5;
        } else {
            *lambda_home -= adjustment * 0.5;
            *lambda_away -= adjustment * 0.5;
        }
        
        // Keep lambdas in reasonable bounds
        *lambda_home = lambda_home.max(0.5).min(3.0);
        *lambda_away = lambda_away.max(0.5).min(3.0);
        
        Ok(())
    }
}

#[derive(Debug)]
pub struct EnsembleModel {
    name: String,
    version: String,
    logistic_model: LogisticRegressionModel,
    poisson_model: PoissonModel,
    logistic_weight: f64,
    poisson_weight: f64,
}

impl EnsembleModel {
    pub fn new() -> Self {
        Self {
            name: "EnsembleModel".to_string(),
            version: "v1.0".to_string(),
            logistic_model: LogisticRegressionModel::new(),
            poisson_model: PoissonModel::new(),
            logistic_weight: 0.6,
            poisson_weight: 0.4,
        }
    }
}

impl EnsembleModel {
    pub fn model_name(&self) -> &str {
        &self.name
    }
    
    pub fn model_version(&self) -> &str {
        &self.version
    }
    
    pub async fn predict(&self, features: &FeatureVector) -> Result<Prediction> {
        // Get predictions from both models
        let logistic_pred = self.logistic_model.predict(features).await?;
        let poisson_pred = self.poisson_model.predict(features).await?;
        
        // Weighted average of predictions
        let total_weight = self.logistic_weight + self.poisson_weight;
        
        let mut home_win_prob = (logistic_pred.home_win_prob * self.logistic_weight + 
                                poisson_pred.home_win_prob * self.poisson_weight) / total_weight;
        
        let mut draw_prob = (logistic_pred.draw_prob.unwrap_or(0.0) * self.logistic_weight + 
                            poisson_pred.draw_prob.unwrap_or(0.0) * self.poisson_weight) / total_weight;
        
        let mut away_win_prob = (logistic_pred.away_win_prob * self.logistic_weight + 
                                poisson_pred.away_win_prob * self.poisson_weight) / total_weight;
        
        // Ensure probabilities are valid and sum to 1
        home_win_prob = home_win_prob.max(0.01).min(0.98);
        draw_prob = draw_prob.max(0.01).min(0.98);
        away_win_prob = away_win_prob.max(0.01).min(0.98);
        
        let total = home_win_prob + draw_prob + away_win_prob;
        home_win_prob /= total;
        draw_prob /= total;
        away_win_prob /= total;
        
        // Ensemble confidence is the average of individual confidences
        let avg_confidence = (logistic_pred.confidence + poisson_pred.confidence) / 2.0;
        
        let prediction = Prediction::new(
            features.match_id.clone(),
            self.model_name().to_string(),
            self.model_version().to_string(),
            home_win_prob,
            away_win_prob,
            features.timestamp,
        )?
        .with_draw_prob(draw_prob)?
        .with_confidence(avg_confidence)?;
        
        Ok(prediction)
    }
    
    pub async fn update_weights(&mut self, feedback: &ModelFeedback) -> Result<()> {
        // Update individual models
        if let Err(e) = self.logistic_model.update_weights(feedback).await {
            tracing::warn!("Failed to update logistic model: {}", e);
        }
        
        if let Err(e) = self.poisson_model.update_weights(feedback).await {
            tracing::warn!("Failed to update poisson model: {}", e);
        }
        
        // TODO: Implement dynamic weight adjustment based on individual model performance
        Ok(())
    }
}