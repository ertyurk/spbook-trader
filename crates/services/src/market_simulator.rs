use quant_models::{SimpleMarketOdds, MatchEvent, Prediction};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal_macros::dec;
use anyhow::Result;
use rand::{Rng, SeedableRng};
use rand::rngs::SmallRng;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use tracing::info;

pub struct MarketSimulator {
    base_margins: Arc<RwLock<HashMap<String, f64>>>,
    market_odds: Arc<RwLock<HashMap<String, SimpleMarketOdds>>>,
    rng: Arc<Mutex<SmallRng>>,
}

impl MarketSimulator {
    pub fn new() -> Self {
        Self {
            base_margins: Arc::new(RwLock::new(HashMap::new())),
            market_odds: Arc::new(RwLock::new(HashMap::new())),
            rng: Arc::new(Mutex::new(SmallRng::from_entropy())),
        }
    }

    /// Generate realistic market odds based on match event and context
    pub async fn generate_market_odds(&self, event: &MatchEvent) -> Result<SimpleMarketOdds> {
        // Base probabilities influenced by team strength and match state
        let (mut home_prob, mut draw_prob, mut away_prob) = self.calculate_base_probabilities(event);
        
        // Adjust probabilities based on current match state
        self.adjust_for_match_state(event, &mut home_prob, &mut draw_prob, &mut away_prob);
        
        // Add some randomness to simulate market inefficiencies
        let noise_factor = 0.02; // 2% random variation
        {
            let mut rng = self.rng.lock().await;
            home_prob += rng.gen_range(-noise_factor..noise_factor);
            draw_prob += rng.gen_range(-noise_factor..noise_factor);
            away_prob += rng.gen_range(-noise_factor..noise_factor);
        }
        
        // Normalize probabilities
        let total = home_prob + draw_prob + away_prob;
        home_prob /= total;
        draw_prob /= total;
        away_prob /= total;
        
        // Get bookmaker margin (overround)
        let margin = self.get_margin_for_match(&event.match_id).await;
        
        // Convert to odds with margin
        let odds = SimpleMarketOdds::from_probabilities(home_prob, draw_prob, away_prob, margin);
        
        // Store the odds
        self.market_odds.write().await.insert(event.match_id.clone(), odds.clone());
        
        info!("📊 Generated market odds for {}: Home={:.2} Draw={:.2} Away={:.2}", 
              event.match_id, odds.home_win, odds.draw, odds.away_win);
        
        Ok(odds)
    }

    /// Update odds based on new match events (e.g., goals, cards)
    pub async fn update_odds_for_event(&self, event: &MatchEvent) -> Result<Option<SimpleMarketOdds>> {
        // Only update odds for significant events
        match &event.event_type {
            quant_models::EventType::Goal { .. } |
            quant_models::EventType::Card { .. } => {
                let updated_odds = self.generate_market_odds(event).await?;
                Ok(Some(updated_odds))
            }
            _ => Ok(None)
        }
    }

    /// Generate odds that might have value against a prediction
    pub async fn generate_odds_with_edge(&self, prediction: &Prediction, target_edge: f64) -> Result<SimpleMarketOdds> {
        // Start with fair odds from prediction
        let fair_home_odds = 1.0 / prediction.home_win_prob;
        let fair_away_odds = 1.0 / prediction.away_win_prob;
        let fair_draw_odds = prediction.draw_prob.map(|p| 1.0 / p).unwrap_or(3.0);
        
        // Add edge by making market odds slightly worse than fair value
        let edge_factor = {
            let mut rng = self.rng.lock().await;
            1.0 + target_edge + rng.gen_range(0.0..0.02)
        };
        
        let market_home_odds = fair_home_odds * edge_factor;
        let market_away_odds = fair_away_odds * edge_factor;
        let market_draw_odds = fair_draw_odds * edge_factor;
        
        let odds = SimpleMarketOdds::new(
            Decimal::from_f64_retain(market_home_odds).unwrap_or(dec!(2.0)),
            Decimal::from_f64_retain(market_draw_odds).unwrap_or(dec!(3.0)),
            Decimal::from_f64_retain(market_away_odds).unwrap_or(dec!(2.0)),
        );
        
        // Store the odds
        self.market_odds.write().await.insert(prediction.match_id.clone(), odds.clone());
        
        Ok(odds)
    }

    pub async fn get_current_odds(&self, match_id: &str) -> Option<SimpleMarketOdds> {
        self.market_odds.read().await.get(match_id).cloned()
    }

    fn calculate_base_probabilities(&self, event: &MatchEvent) -> (f64, f64, f64) {
        // Simplified base probabilities
        // In a real system, this would use team ratings, head-to-head records, etc.
        
        let league_competitiveness = match event.league.as_str() {
            "Premier League" => 0.9, // More unpredictable
            "La Liga" => 0.8,
            "Bundesliga" => 0.7,
            _ => 0.6,
        };
        
        // Home advantage
        let home_advantage = 0.55;
        let away_prob = (1.0 - home_advantage) * 0.7; // Away wins less likely
        let draw_prob = 0.25 + (league_competitiveness * 0.05); // More competitive = more draws
        let home_prob = 1.0 - draw_prob - away_prob;
        
        (home_prob, draw_prob, away_prob)
    }

    fn adjust_for_match_state(&self, event: &MatchEvent, home_prob: &mut f64, draw_prob: &mut f64, away_prob: &mut f64) {
        // Adjust based on match events and time
        match &event.event_type {
            quant_models::EventType::Goal { team, minute, .. } => {
                let time_factor = (*minute as f64 / 90.0).min(1.0);
                let adjustment = 0.1 * (1.0 - time_factor); // Less adjustment as match progresses
                
                if team == &event.team_home {
                    *home_prob += adjustment;
                    *away_prob -= adjustment * 0.5;
                    *draw_prob -= adjustment * 0.5;
                } else {
                    *away_prob += adjustment;
                    *home_prob -= adjustment * 0.5;
                    *draw_prob -= adjustment * 0.5;
                }
            }
            quant_models::EventType::Card { team, card_type, minute, .. } => {
                let severity = match card_type {
                    quant_models::CardType::Red => 0.15,
                    quant_models::CardType::Yellow => 0.02,
                };
                
                let time_factor = (*minute as f64 / 90.0).min(1.0);
                let adjustment = severity * (1.0 - time_factor);
                
                if team == &event.team_home {
                    *home_prob -= adjustment;
                    *away_prob += adjustment * 0.7;
                    *draw_prob += adjustment * 0.3;
                } else {
                    *away_prob -= adjustment;
                    *home_prob += adjustment * 0.7;
                    *draw_prob += adjustment * 0.3;
                }
            }
            _ => {}
        }
        
        // Ensure probabilities are valid
        let total = *home_prob + *draw_prob + *away_prob;
        if total > 0.0 {
            *home_prob /= total;
            *draw_prob /= total;
            *away_prob /= total;
        }
        
        // Clamp to reasonable ranges
        *home_prob = home_prob.max(0.1).min(0.8);
        *draw_prob = draw_prob.max(0.1).min(0.4);
        *away_prob = away_prob.max(0.1).min(0.8);
        
        // Final normalization
        let total = *home_prob + *draw_prob + *away_prob;
        *home_prob /= total;
        *draw_prob /= total;
        *away_prob /= total;
    }

    async fn get_margin_for_match(&self, match_id: &str) -> f64 {
        // Different bookmakers have different margins
        let margins = self.base_margins.read().await;
        if let Some(margin) = margins.get(match_id).copied() {
            margin
        } else {
            // Typical bookmaker margins: 2-8%
            let mut rng = self.rng.lock().await;
            rng.gen_range(0.02..0.08)
        }
    }

    pub async fn set_margin_for_match(&self, match_id: String, margin: f64) {
        self.base_margins.write().await.insert(match_id, margin);
    }

    /// Simulate market movement over time
    pub async fn simulate_market_movement(&self, match_id: &str, time_factor: f64) -> Result<()> {
        if let Some(odds) = self.get_current_odds(match_id).await {
            // Markets become more efficient (tighter) closer to match time
            let efficiency_factor = time_factor * 0.5; // 0.0 = early, 0.5 = near kickoff
            let volatility = 0.05 * (1.0 - efficiency_factor); // Less volatile near kickoff
            
            // Apply small random movements
            let (home_change, draw_change, away_change) = {
                let mut rng = self.rng.lock().await;
                (
                    rng.gen_range(-volatility..volatility),
                    rng.gen_range(-volatility..volatility),
                    rng.gen_range(-volatility..volatility),
                )
            };
            
            // Convert to probabilities, adjust, convert back
            let home_prob = 1.0 / odds.home_win.to_f64().unwrap_or(2.0);
            let draw_prob = 1.0 / odds.draw.to_f64().unwrap_or(3.0);
            let away_prob = 1.0 / odds.away_win.to_f64().unwrap_or(2.0);
            
            let new_home_prob = (home_prob + home_change).max(0.1).min(0.8);
            let new_draw_prob = (draw_prob + draw_change).max(0.1).min(0.4);
            let new_away_prob = (away_prob + away_change).max(0.1).min(0.8);
            
            // Normalize
            let total = new_home_prob + new_draw_prob + new_away_prob;
            let norm_home = new_home_prob / total;
            let norm_draw = new_draw_prob / total;
            let norm_away = new_away_prob / total;
            
            // Get margin and create new odds
            let margin = self.get_margin_for_match(match_id).await;
            let new_odds = SimpleMarketOdds::from_probabilities(norm_home, norm_draw, norm_away, margin);
            
            self.market_odds.write().await.insert(match_id.to_string(), new_odds);
        }
        
        Ok(())
    }
}

impl Default for MarketSimulator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quant_models::{EventType, MatchStatus};
    use chrono::Utc;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_market_odds_generation() {
        let simulator = MarketSimulator::new();
        
        let event = MatchEvent {
            id: Uuid::new_v4(),
            match_id: "test_match".to_string(),
            timestamp: Utc::now(),
            event_type: EventType::MatchStart,
            team_home: "Arsenal".to_string(),
            team_away: "Chelsea".to_string(),
            league: "Premier League".to_string(),
            match_status: MatchStatus::Live,
            metadata: serde_json::Value::Null,
        };
        
        let odds = simulator.generate_market_odds(&event).await.unwrap();
        
        // Odds should be reasonable
        assert!(odds.home_win > dec!(1.1));
        assert!(odds.home_win < dec!(10.0));
        assert!(odds.draw > dec!(1.1));
        assert!(odds.away_win > dec!(1.1));
        assert!(odds.away_win < dec!(10.0));
    }
}