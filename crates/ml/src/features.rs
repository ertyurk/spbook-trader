// Feature engineering

use quant_models::{MatchEvent, FeatureVector};
use anyhow::Result;

pub struct FeatureEngineer;

impl FeatureEngineer {
    pub fn new() -> Self {
        Self
    }
    
    pub async fn extract_features(&self, _event: &MatchEvent) -> Result<FeatureVector> {
        // TODO: Implement feature extraction
        todo!("Implement feature extraction")
    }
}