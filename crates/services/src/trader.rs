// Trading decision service

use quant_models::{BettingDecision, Prediction, MarketOdds};
use anyhow::Result;

pub struct TradingService {
    strategy_name: String,
}

impl TradingService {
    pub fn new(strategy_name: String) -> Self {
        Self { strategy_name }
    }
    
    pub async fn make_decision(&self, _prediction: &Prediction, _odds: &MarketOdds) -> Result<Option<BettingDecision>> {
        // TODO: Implement trading logic
        Ok(None)
    }
}