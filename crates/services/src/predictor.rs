// Prediction service

use quant_models::{Prediction, MatchEvent};
use anyhow::Result;

pub struct PredictorService {
    model_name: String,
}

impl PredictorService {
    pub fn new(model_name: String) -> Self {
        Self { model_name }
    }
    
    pub async fn predict(&self, _event: &MatchEvent) -> Result<Prediction> {
        // TODO: Implement prediction logic
        todo!("Implement prediction logic")
    }
}