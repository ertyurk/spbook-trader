use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::error::{QuantsError, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BettingDecision {
    pub id: Uuid,
    pub match_id: String,
    pub bet_type: BetType,
    pub stake: Decimal,
    pub odds: Decimal,
    pub expected_value: f64,
    pub kelly_fraction: f64,
    pub confidence: f64,
    pub strategy: String,
    pub timestamp: DateTime<Utc>,
    pub status: BetStatus,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BetType {
    HomeWin,
    Draw,
    AwayWin,
    OverUnder { line: Decimal, over: bool },
    AsianHandicap { line: Decimal, team: String },
    BothTeamsToScore { yes: bool },
    CorrectScore { home_goals: u8, away_goals: u8 },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BetStatus {
    Pending,
    Placed,
    Won,
    Lost,
    Void,
    CashedOut { amount: Decimal },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BettingStrategy {
    pub name: String,
    pub description: String,
    pub min_odds: Decimal,
    pub max_odds: Decimal,
    pub min_edge: f64,
    pub max_stake_percent: f64,
    pub kelly_multiplier: f64,
    pub min_confidence: f64,
    pub max_correlation: f64,
    pub risk_tolerance: RiskTolerance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskTolerance {
    Conservative,
    Moderate,
    Aggressive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Portfolio {
    pub total_bankroll: Decimal,
    pub available_bankroll: Decimal,
    pub active_bets: Vec<BettingDecision>,
    pub historical_bets: Vec<BettingDecision>,
    pub total_profit_loss: Decimal,
    pub roi: f64,
    pub win_rate: f64,
    pub sharpe_ratio: f64,
    pub max_drawdown: f64,
    pub last_updated: DateTime<Utc>,
}

impl BettingDecision {
    pub fn new(
        match_id: String,
        bet_type: BetType,
        stake: Decimal,
        odds: Decimal,
        true_probability: f64,
        strategy: String,
    ) -> Result<Self> {
        if stake <= Decimal::ZERO {
            return Err(QuantsError::InvalidStake { 
                amount: stake.to_string() 
            });
        }
        
        if odds <= dec!(1.0) {
            return Err(QuantsError::InvalidOdds(
                format!("Odds must be greater than 1.0, got {}", odds)
            ));
        }
        
        let implied_probability = 1.0 / odds.to_f64().unwrap();
        let expected_value = (true_probability * odds.to_f64().unwrap()) - 1.0;
        let edge = true_probability - implied_probability;
        
        // Kelly criterion: f = (bp - q) / b
        // where b = odds - 1, p = true probability, q = 1 - p
        let b = odds.to_f64().unwrap() - 1.0;
        let q = 1.0 - true_probability;
        let kelly_fraction = if b > 0.0 {
            (b * true_probability - q) / b
        } else {
            0.0
        }.max(0.0); // Don't bet if Kelly is negative
        
        Ok(Self {
            id: Uuid::new_v4(),
            match_id,
            bet_type,
            stake,
            odds,
            expected_value,
            kelly_fraction,
            confidence: edge,
            strategy,
            timestamp: Utc::now(),
            status: BetStatus::Pending,
            metadata: serde_json::Value::Null,
        })
    }
    
    pub fn has_positive_ev(&self) -> bool {
        self.expected_value > 0.0
    }
    
    pub fn potential_payout(&self) -> Decimal {
        self.stake * self.odds
    }
    
    pub fn potential_profit(&self) -> Decimal {
        self.potential_payout() - self.stake
    }
    
    pub fn risk_reward_ratio(&self) -> f64 {
        let potential_profit = self.potential_profit().to_f64().unwrap();
        let stake = self.stake.to_f64().unwrap();
        potential_profit / stake
    }
    
    pub fn update_status(&mut self, status: BetStatus) {
        self.status = status;
    }
    
    pub fn is_active(&self) -> bool {
        matches!(self.status, BetStatus::Pending | BetStatus::Placed)
    }
}

impl BettingStrategy {
    pub fn conservative() -> Self {
        Self {
            name: "Conservative Value".to_string(),
            description: "Low-risk value betting with strict criteria".to_string(),
            min_odds: dec!(1.5),
            max_odds: dec!(3.0),
            min_edge: 0.05, // 5% minimum edge
            max_stake_percent: 0.02, // Max 2% of bankroll per bet
            kelly_multiplier: 0.25, // Quarter Kelly
            min_confidence: 0.8,
            max_correlation: 0.3,
            risk_tolerance: RiskTolerance::Conservative,
        }
    }
    
    pub fn moderate() -> Self {
        Self {
            name: "Moderate Growth".to_string(),
            description: "Balanced approach between growth and risk".to_string(),
            min_odds: dec!(1.3),
            max_odds: dec!(5.0),
            min_edge: 0.03, // 3% minimum edge
            max_stake_percent: 0.05, // Max 5% of bankroll per bet
            kelly_multiplier: 0.5, // Half Kelly
            min_confidence: 0.6,
            max_correlation: 0.5,
            risk_tolerance: RiskTolerance::Moderate,
        }
    }
    
    pub fn aggressive() -> Self {
        Self {
            name: "Aggressive Growth".to_string(),
            description: "High-growth strategy with increased risk".to_string(),
            min_odds: dec!(1.2),
            max_odds: dec!(10.0),
            min_edge: 0.01, // 1% minimum edge
            max_stake_percent: 0.1, // Max 10% of bankroll per bet
            kelly_multiplier: 0.75, // Three-quarter Kelly
            min_confidence: 0.4,
            max_correlation: 0.7,
            risk_tolerance: RiskTolerance::Aggressive,
        }
    }
    
    pub fn should_bet(
        &self,
        odds: Decimal,
        true_probability: f64,
        confidence: f64,
    ) -> bool {
        let implied_probability = 1.0 / odds.to_f64().unwrap();
        let edge = true_probability - implied_probability;
        
        odds >= self.min_odds
            && odds <= self.max_odds
            && edge >= self.min_edge
            && confidence >= self.min_confidence
    }
    
    pub fn calculate_stake(
        &self,
        bankroll: Decimal,
        kelly_fraction: f64,
    ) -> Decimal {
        let kelly_stake = bankroll.to_f64().unwrap() * kelly_fraction * self.kelly_multiplier;
        let max_stake = bankroll.to_f64().unwrap() * self.max_stake_percent;
        
        Decimal::from_f64_retain(kelly_stake.min(max_stake))
            .unwrap_or(Decimal::ZERO)
            .max(Decimal::ZERO)
    }
}

impl Portfolio {
    pub fn new(initial_bankroll: Decimal) -> Self {
        Self {
            total_bankroll: initial_bankroll,
            available_bankroll: initial_bankroll,
            active_bets: Vec::new(),
            historical_bets: Vec::new(),
            total_profit_loss: Decimal::ZERO,
            roi: 0.0,
            win_rate: 0.0,
            sharpe_ratio: 0.0,
            max_drawdown: 0.0,
            last_updated: Utc::now(),
        }
    }
    
    pub fn place_bet(&mut self, mut bet: BettingDecision) -> Result<()> {
        if bet.stake > self.available_bankroll {
            return Err(QuantsError::InvalidStake { 
                amount: format!("Insufficient funds: {} > {}", bet.stake, self.available_bankroll)
            });
        }
        
        self.available_bankroll -= bet.stake;
        bet.update_status(BetStatus::Placed);
        self.active_bets.push(bet);
        self.last_updated = Utc::now();
        
        Ok(())
    }
    
    pub fn settle_bet(&mut self, bet_id: Uuid, won: bool) -> Result<()> {
        let bet_index = self.active_bets
            .iter()
            .position(|bet| bet.id == bet_id)
            .ok_or_else(|| QuantsError::MatchNotFound { 
                match_id: bet_id.to_string() 
            })?;
        
        let mut bet = self.active_bets.remove(bet_index);
        
        let payout = if won {
            bet.update_status(BetStatus::Won);
            bet.potential_payout()
        } else {
            bet.update_status(BetStatus::Lost);
            Decimal::ZERO
        };
        
        self.available_bankroll += payout;
        let profit_loss = payout - bet.stake;
        self.total_profit_loss += profit_loss;
        
        self.historical_bets.push(bet);
        self.update_metrics();
        
        Ok(())
    }
    
    pub fn total_exposure(&self) -> Decimal {
        self.active_bets.iter().map(|bet| bet.stake).sum()
    }
    
    pub fn potential_total_payout(&self) -> Decimal {
        self.active_bets.iter().map(|bet| bet.potential_payout()).sum()
    }
    
    fn update_metrics(&mut self) {
        if self.historical_bets.is_empty() {
            return;
        }
        
        let total_bets = self.historical_bets.len();
        let won_bets = self.historical_bets
            .iter()
            .filter(|bet| matches!(bet.status, BetStatus::Won))
            .count();
        
        self.win_rate = won_bets as f64 / total_bets as f64;
        
        let total_staked: Decimal = self.historical_bets
            .iter()
            .map(|bet| bet.stake)
            .sum();
        
        if total_staked > Decimal::ZERO {
            self.roi = (self.total_profit_loss / total_staked).to_f64().unwrap();
        }
        
        self.last_updated = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_betting_decision_creation() {
        let decision = BettingDecision::new(
            "match_123".to_string(),
            BetType::HomeWin,
            dec!(100),
            dec!(2.5),
            0.5, // 50% true probability
            "TestStrategy".to_string(),
        ).unwrap();
        
        assert_eq!(decision.match_id, "match_123");
        assert_eq!(decision.stake, dec!(100));
        assert_eq!(decision.odds, dec!(2.5));
        assert!(decision.has_positive_ev());
        assert_eq!(decision.potential_payout(), dec!(250));
        assert_eq!(decision.potential_profit(), dec!(150));
    }
    
    #[test]
    fn test_kelly_calculation() {
        // With 60% true probability and 2.0 odds (50% implied)
        // Kelly = (0.6 * 1.0 - 0.4) / 1.0 = 0.2 = 20%
        let decision = BettingDecision::new(
            "match_123".to_string(),
            BetType::HomeWin,
            dec!(100),
            dec!(2.0),
            0.6,
            "TestStrategy".to_string(),
        ).unwrap();
        
        assert!((decision.kelly_fraction - 0.2).abs() < 0.001);
    }
    
    #[test]
    fn test_betting_strategy() {
        let strategy = BettingStrategy::conservative();
        
        // Should bet: good odds, high edge, high confidence
        assert!(strategy.should_bet(dec!(2.0), 0.6, 0.9));
        
        // Should not bet: low confidence
        assert!(!strategy.should_bet(dec!(2.0), 0.6, 0.5));
        
        // Should not bet: no edge
        assert!(!strategy.should_bet(dec!(2.0), 0.5, 0.9));
    }
    
    #[test]
    fn test_portfolio_management() {
        let mut portfolio = Portfolio::new(dec!(1000));
        
        let bet = BettingDecision::new(
            "match_123".to_string(),
            BetType::HomeWin,
            dec!(100),
            dec!(2.0),
            0.6,
            "TestStrategy".to_string(),
        ).unwrap();
        
        let bet_id = bet.id;
        
        // Place bet
        portfolio.place_bet(bet).unwrap();
        assert_eq!(portfolio.available_bankroll, dec!(900));
        assert_eq!(portfolio.active_bets.len(), 1);
        
        // Win bet
        portfolio.settle_bet(bet_id, true).unwrap();
        assert_eq!(portfolio.available_bankroll, dec!(1100)); // 900 + 200 payout
        assert_eq!(portfolio.active_bets.len(), 0);
        assert_eq!(portfolio.historical_bets.len(), 1);
        assert_eq!(portfolio.total_profit_loss, dec!(100));
    }
}