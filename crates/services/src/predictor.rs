use quant_models::{Prediction, MatchEvent};
use quant_ml::{FeatureEngineer, Model, EnsembleModel};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct PredictorService {
    feature_engineer: Arc<FeatureEngineer>,
    model: Arc<RwLock<Model>>,
    prediction_count: Arc<RwLock<u64>>,
}

impl PredictorService {
    pub fn new() -> Self {
        let feature_engineer = Arc::new(FeatureEngineer::new());
        let model = Model::Ensemble(EnsembleModel::new());
        
        Self {
            feature_engineer,
            model: Arc::new(RwLock::new(model)),
            prediction_count: Arc::new(RwLock::new(0)),
        }
    }
    
    pub async fn predict(&self, event: &MatchEvent) -> Result<Prediction> {
        // Extract features from the event
        let features = self.feature_engineer.extract_features(event).await?;
        
        tracing::debug!("🧠 Extracted {} features for match {}", 
                       features.features.len(), 
                       event.match_id);
        
        // Generate prediction using the ML model
        let model = self.model.read().await;
        let prediction = model.predict(&features).await?;
        
        // Update prediction count
        let mut count = self.prediction_count.write().await;
        *count += 1;
        
        tracing::info!("🎯 Prediction #{} for {}: Home={:.1}% Draw={:.1}% Away={:.1}% (Confidence: {:.1}%)",
                      *count,
                      event.match_id,
                      prediction.home_win_prob * 100.0,
                      prediction.draw_prob.unwrap_or(0.0) * 100.0,
                      prediction.away_win_prob * 100.0,
                      prediction.confidence * 100.0
        );
        
        // Log key features for insight
        if let Some(elo_diff) = features.features.get("elo_difference") {
            tracing::debug!("📊 Key features - Elo diff: {:.1}, Momentum: {:.2}, Intensity: {:.2}",
                           elo_diff,
                           features.features.get("momentum").unwrap_or(&0.0),
                           features.features.get("intensity").unwrap_or(&0.0));
        }
        
        Ok(prediction)
    }
    
    pub async fn update_team_performance(&self, team: &str, goals_for: u32, goals_against: u32) {
        self.feature_engineer.update_team_stats(team, goals_for, goals_against);
        tracing::debug!("📈 Updated team stats for {}: GF={}, GA={}", team, goals_for, goals_against);
    }
    
    pub async fn get_prediction_count(&self) -> u64 {
        *self.prediction_count.read().await
    }
    
    pub fn get_feature_engineer(&self) -> Arc<FeatureEngineer> {
        self.feature_engineer.clone()
    }
}