use quant_models::{
    Prediction, BettingDecision, BetType, BettingStrategy, Portfolio, 
    SimpleMarketOdds, RiskTolerance, QuantsError, Result
};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal_macros::dec;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, debug, error};
use chrono::{DateTime, Utc};

pub struct TradingEngine {
    portfolio: Arc<RwLock<Portfolio>>,
    strategies: HashMap<String, BettingStrategy>,
    market_odds: Arc<RwLock<HashMap<String, SimpleMarketOdds>>>,
    risk_manager: RiskManager,
    trade_count: Arc<RwLock<u64>>,
}

#[derive(Debug, Clone)]
pub struct RiskManager {
    pub max_daily_loss: Decimal,
    pub max_concurrent_bets: usize,
    pub max_exposure_per_match: Decimal,
    pub correlation_threshold: f64,
    pub current_daily_loss: Decimal,
    pub daily_reset_time: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct TradingSignal {
    pub match_id: String,
    pub signal_strength: f64,
    pub recommended_bet: Option<BettingDecision>,
    pub risk_assessment: RiskAssessment,
    pub reasoning: String,
}

#[derive(Debug, Clone)]
pub struct RiskAssessment {
    pub risk_score: f64, // 0.0 (low) to 1.0 (high)
    pub correlation_risk: f64,
    pub liquidity_risk: f64,
    pub volatility_risk: f64,
    pub portfolio_impact: f64,
    pub warnings: Vec<String>,
}

impl TradingEngine {
    pub fn new(initial_bankroll: Decimal) -> Self {
        let mut strategies = HashMap::new();
        strategies.insert("conservative".to_string(), BettingStrategy::conservative());
        strategies.insert("moderate".to_string(), BettingStrategy::moderate());
        strategies.insert("aggressive".to_string(), BettingStrategy::aggressive());

        let risk_manager = RiskManager {
            max_daily_loss: initial_bankroll * dec!(0.05), // 5% of bankroll
            max_concurrent_bets: 10,
            max_exposure_per_match: initial_bankroll * dec!(0.1), // 10% per match
            correlation_threshold: 0.7,
            current_daily_loss: dec!(0.0),
            daily_reset_time: Utc::now(),
        };

        Self {
            portfolio: Arc::new(RwLock::new(Portfolio::new(initial_bankroll))),
            strategies,
            market_odds: Arc::new(RwLock::new(HashMap::new())),
            risk_manager,
            trade_count: Arc::new(RwLock::new(0)),
        }
    }

    pub async fn process_prediction(&self, prediction: &Prediction) -> Result<TradingSignal> {
        debug!("🧮 Processing prediction for match {}", prediction.match_id);

        let market_odds = self.get_market_odds(&prediction.match_id).await;
        
        if market_odds.is_none() {
            warn!("📊 No market odds available for match {}", prediction.match_id);
            return Ok(TradingSignal {
                match_id: prediction.match_id.clone(),
                signal_strength: 0.0,
                recommended_bet: None,
                risk_assessment: RiskAssessment::default(),
                reasoning: "No market odds available".to_string(),
            });
        }

        let odds = market_odds.unwrap();
        let signal = self.generate_trading_signal(prediction, &odds).await?;

        if let Some(ref bet) = signal.recommended_bet {
            info!("💰 Trading signal generated for {}: {} stake with {:.1}% edge", 
                  prediction.match_id, 
                  bet.stake,
                  bet.confidence * 100.0);
        }

        Ok(signal)
    }

    async fn generate_trading_signal(
        &self, 
        prediction: &Prediction, 
        market_odds: &SimpleMarketOdds
    ) -> Result<TradingSignal> {
        let mut best_bet: Option<BettingDecision> = None;
        let mut best_edge = 0.0;
        let mut reasoning = String::new();

        // Analyze home win opportunity
        if let Some(bet) = self.analyze_bet_opportunity(
            &prediction.match_id,
            BetType::HomeWin,
            prediction.home_win_prob,
            market_odds.home_win,
            prediction.confidence,
        ).await? {
            if bet.confidence > best_edge {
                best_edge = bet.confidence;
                best_bet = Some(bet);
                reasoning = format!("Home win edge: {:.1}%", best_edge * 100.0);
            }
        }

        // Analyze draw opportunity
        if let Some(draw_prob) = prediction.draw_prob {
            if let Some(bet) = self.analyze_bet_opportunity(
                &prediction.match_id,
                BetType::Draw,
                draw_prob,
                market_odds.draw,
                prediction.confidence,
            ).await? {
                if bet.confidence > best_edge {
                    best_edge = bet.confidence;
                    best_bet = Some(bet);
                    reasoning = format!("Draw edge: {:.1}%", best_edge * 100.0);
                }
            }
        }

        // Analyze away win opportunity
        if let Some(bet) = self.analyze_bet_opportunity(
            &prediction.match_id,
            BetType::AwayWin,
            prediction.away_win_prob,
            market_odds.away_win,
            prediction.confidence,
        ).await? {
            if bet.confidence > best_edge {
                best_edge = bet.confidence;
                best_bet = Some(bet);
                reasoning = format!("Away win edge: {:.1}%", best_edge * 100.0);
            }
        }

        let risk_assessment = self.assess_risk(&prediction.match_id, &best_bet).await;
        let signal_strength = if best_bet.is_some() { 
            (best_edge * prediction.confidence).min(1.0) 
        } else { 
            0.0 
        };

        Ok(TradingSignal {
            match_id: prediction.match_id.clone(),
            signal_strength,
            recommended_bet: best_bet,
            risk_assessment,
            reasoning,
        })
    }

    async fn analyze_bet_opportunity(
        &self,
        match_id: &str,
        bet_type: BetType,
        true_probability: f64,
        market_odds: Decimal,
        confidence: f64,
    ) -> Result<Option<BettingDecision>> {
        let strategy = self.get_active_strategy().await;
        
        if !strategy.should_bet(market_odds, true_probability, confidence) {
            return Ok(None);
        }

        let portfolio = self.portfolio.read().await;
        let bet = BettingDecision::new(
            match_id.to_string(),
            bet_type.clone(),
            dec!(0.0), // Will be calculated below
            market_odds,
            true_probability,
            strategy.name.clone(),
        )?;

        // Calculate optimal stake using Kelly criterion with strategy constraints
        let kelly_stake = strategy.calculate_stake(
            portfolio.available_bankroll,
            bet.kelly_fraction,
        );

        // Apply risk management constraints
        let adjusted_stake = self.apply_risk_constraints(
            kelly_stake,
            match_id,
            &portfolio,
        ).await;

        if adjusted_stake <= dec!(0.0) {
            return Ok(None);
        }

        // Create final betting decision with adjusted stake
        let final_bet = BettingDecision::new(
            match_id.to_string(),
            bet_type,
            adjusted_stake,
            market_odds,
            true_probability,
            strategy.name.clone(),
        )?;

        Ok(Some(final_bet))
    }

    async fn apply_risk_constraints(
        &self,
        proposed_stake: Decimal,
        match_id: &str,
        portfolio: &Portfolio,
    ) -> Decimal {
        let mut final_stake = proposed_stake;

        // Check available bankroll
        if final_stake > portfolio.available_bankroll {
            final_stake = portfolio.available_bankroll * dec!(0.95); // Leave 5% buffer
            debug!("🛡️ Stake reduced due to bankroll constraints: {}", final_stake);
        }

        // Check maximum exposure per match
        let current_match_exposure = portfolio.active_bets
            .iter()
            .filter(|bet| bet.match_id == match_id)
            .map(|bet| bet.stake)
            .sum::<Decimal>();

        if current_match_exposure + final_stake > self.risk_manager.max_exposure_per_match {
            final_stake = (self.risk_manager.max_exposure_per_match - current_match_exposure)
                .max(dec!(0.0));
            debug!("🛡️ Stake reduced due to match exposure limits: {}", final_stake);
        }

        // Check daily loss limits
        if self.risk_manager.current_daily_loss + final_stake > self.risk_manager.max_daily_loss {
            final_stake = (self.risk_manager.max_daily_loss - self.risk_manager.current_daily_loss)
                .max(dec!(0.0));
            debug!("🛡️ Stake reduced due to daily loss limits: {}", final_stake);
        }

        // Check concurrent bet limits
        if portfolio.active_bets.len() >= self.risk_manager.max_concurrent_bets {
            debug!("🛡️ Max concurrent bets reached, rejecting new bet");
            return dec!(0.0);
        }

        final_stake
    }

    async fn assess_risk(&self, match_id: &str, bet: &Option<BettingDecision>) -> RiskAssessment {
        let mut warnings = Vec::new();
        let mut risk_score: f64 = 0.0;

        if let Some(bet) = bet {
            // Assess stake size risk
            let portfolio = self.portfolio.read().await;
            let stake_percentage = (bet.stake / portfolio.total_bankroll).to_f64().unwrap_or(0.0);
            
            if stake_percentage > 0.05 {
                warnings.push("High stake percentage (>5%)".to_string());
                risk_score += 0.3;
            }

            // Assess odds risk
            let odds_value = bet.odds.to_f64().unwrap_or(1.0);
            if odds_value > 5.0 {
                warnings.push("High odds bet (>5.0)".to_string());
                risk_score += 0.2;
            }

            // Assess Kelly fraction risk
            if bet.kelly_fraction > 0.1 {
                warnings.push("High Kelly fraction (>10%)".to_string());
                risk_score += 0.2;
            }

            // Assess correlation risk
            let correlation_risk = self.calculate_correlation_risk(match_id, bet).await;
            if correlation_risk > self.risk_manager.correlation_threshold {
                warnings.push("High correlation with existing positions".to_string());
                risk_score += 0.3;
            }
        }

        RiskAssessment {
            risk_score: risk_score.min(1.0),
            correlation_risk: 0.0, // Simplified for now
            liquidity_risk: 0.1,   // Assume low liquidity risk
            volatility_risk: 0.2,  // Moderate volatility
            portfolio_impact: 0.0, // Calculated based on stake
            warnings,
        }
    }

    async fn calculate_correlation_risk(&self, match_id: &str, _bet: &BettingDecision) -> f64 {
        // Simplified correlation calculation
        // In a real system, this would analyze correlations between:
        // - Same league matches
        // - Same teams
        // - Similar market conditions
        
        let portfolio = self.portfolio.read().await;
        let same_league_bets = portfolio.active_bets
            .iter()
            .filter(|active_bet| {
                // Simplified: assume matches with similar IDs are correlated
                active_bet.match_id.starts_with(&match_id[..3])
            })
            .count();

        (same_league_bets as f64 * 0.2).min(1.0)
    }

    pub async fn execute_trade(&self, signal: &TradingSignal) -> Result<bool> {
        if let Some(ref bet) = signal.recommended_bet {
            // Final risk check before execution
            if signal.risk_assessment.risk_score > 0.8 {
                warn!("🚫 Trade rejected due to high risk score: {:.2}", 
                      signal.risk_assessment.risk_score);
                return Ok(false);
            }

            let mut portfolio = self.portfolio.write().await;
            portfolio.place_bet(bet.clone())?;

            let mut count = self.trade_count.write().await;
            *count += 1;

            info!("✅ Trade executed #{}: {} stake on {} (odds: {}, EV: {:.1}%)",
                  *count,
                  bet.stake,
                  match bet.bet_type {
                      BetType::HomeWin => "Home Win",
                      BetType::Draw => "Draw", 
                      BetType::AwayWin => "Away Win",
                      _ => "Other"
                  },
                  bet.odds,
                  bet.expected_value * 100.0
            );

            Ok(true)
        } else {
            debug!("📊 No trade executed - no profitable opportunity found");
            Ok(false)
        }
    }

    async fn get_active_strategy(&self) -> BettingStrategy {
        // For now, return moderate strategy
        // In a real system, this could be dynamic based on performance
        self.strategies.get("moderate").unwrap().clone()
    }

    async fn get_market_odds(&self, match_id: &str) -> Option<SimpleMarketOdds> {
        self.market_odds.read().await.get(match_id).cloned()
    }

    pub async fn update_market_odds(&self, match_id: String, odds: SimpleMarketOdds) {
        self.market_odds.write().await.insert(match_id, odds);
    }

    pub async fn get_portfolio_summary(&self) -> PortfolioSummary {
        let portfolio = self.portfolio.read().await;
        let trade_count = *self.trade_count.read().await;

        PortfolioSummary {
            total_bankroll: portfolio.total_bankroll,
            available_bankroll: portfolio.available_bankroll,
            total_exposure: portfolio.total_exposure(),
            active_bets_count: portfolio.active_bets.len(),
            total_trades: trade_count,
            roi: portfolio.roi,
            win_rate: portfolio.win_rate,
            profit_loss: portfolio.total_profit_loss,
        }
    }

    pub async fn settle_bet(&self, match_id: &str, outcome: BetOutcome) -> Result<()> {
        let mut portfolio = self.portfolio.write().await;
        
        // Find bets for this match and settle them
        let bet_ids: Vec<_> = portfolio.active_bets
            .iter()
            .filter(|bet| bet.match_id == match_id)
            .map(|bet| bet.id)
            .collect();

        for bet_id in bet_ids {
            let won = self.determine_bet_result(&portfolio, bet_id, &outcome)?;
            portfolio.settle_bet(bet_id, won)?;
            
            info!("🏁 Bet settled for {}: {} ({})", 
                  match_id, 
                  if won { "WON" } else { "LOST" },
                  bet_id
            );
        }

        Ok(())
    }

    fn determine_bet_result(
        &self, 
        portfolio: &Portfolio, 
        bet_id: uuid::Uuid, 
        outcome: &BetOutcome
    ) -> Result<bool> {
        let bet = portfolio.active_bets
            .iter()
            .find(|b| b.id == bet_id)
            .ok_or_else(|| QuantsError::MatchNotFound { 
                match_id: bet_id.to_string() 
            })?;

        let won = match (&bet.bet_type, outcome) {
            (BetType::HomeWin, BetOutcome::HomeWin) => true,
            (BetType::Draw, BetOutcome::Draw) => true,
            (BetType::AwayWin, BetOutcome::AwayWin) => true,
            _ => false,
        };

        Ok(won)
    }
}

#[derive(Debug, Clone)]
pub struct PortfolioSummary {
    pub total_bankroll: Decimal,
    pub available_bankroll: Decimal,
    pub total_exposure: Decimal,
    pub active_bets_count: usize,
    pub total_trades: u64,
    pub roi: f64,
    pub win_rate: f64,
    pub profit_loss: Decimal,
}

#[derive(Debug, Clone)]
pub enum BetOutcome {
    HomeWin,
    Draw,
    AwayWin,
}

impl Default for RiskAssessment {
    fn default() -> Self {
        Self {
            risk_score: 0.0,
            correlation_risk: 0.0,
            liquidity_risk: 0.0,
            volatility_risk: 0.0,
            portfolio_impact: 0.0,
            warnings: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[tokio::test]
    async fn test_trading_engine_creation() {
        let engine = TradingEngine::new(dec!(1000.0));
        let summary = engine.get_portfolio_summary().await;
        
        assert_eq!(summary.total_bankroll, dec!(1000.0));
        assert_eq!(summary.available_bankroll, dec!(1000.0));
        assert_eq!(summary.active_bets_count, 0);
    }

    #[tokio::test]
    async fn test_risk_constraints() {
        let engine = TradingEngine::new(dec!(1000.0));
        let portfolio = Portfolio::new(dec!(1000.0));
        
        // Test bankroll constraint
        let constrained_stake = engine.apply_risk_constraints(
            dec!(2000.0), // More than bankroll
            "test_match",
            &portfolio,
        ).await;
        
        assert!(constrained_stake < dec!(1000.0));
    }
}