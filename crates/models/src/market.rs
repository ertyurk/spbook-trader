use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::error::{QuantsError, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SimpleMarketOdds {
    pub home_win: Decimal,
    pub draw: Decimal,
    pub away_win: Decimal,
}

impl SimpleMarketOdds {
    pub fn new(home_win: Decimal, draw: Decimal, away_win: Decimal) -> Self {
        Self { home_win, draw, away_win }
    }
    
    pub fn from_probabilities(home_prob: f64, draw_prob: f64, away_prob: f64, margin: f64) -> Self {
        // Add bookmaker margin (overround)
        let total_prob = home_prob + draw_prob + away_prob;
        let adjusted_total = total_prob * (1.0 + margin);
        
        let adjusted_home = (home_prob / total_prob) * adjusted_total;
        let adjusted_draw = (draw_prob / total_prob) * adjusted_total;
        let adjusted_away = (away_prob / total_prob) * adjusted_total;
        
        Self {
            home_win: Decimal::from_f64_retain(1.0 / adjusted_home).unwrap_or(Decimal::from(2)),
            draw: Decimal::from_f64_retain(1.0 / adjusted_draw).unwrap_or(Decimal::from(3)),
            away_win: Decimal::from_f64_retain(1.0 / adjusted_away).unwrap_or(Decimal::from(2)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MarketOdds {
    pub id: Uuid,
    pub match_id: String,
    pub market_type: MarketType,
    pub bookmaker: String,
    pub odds: OddsFormat,
    pub timestamp: DateTime<Utc>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MarketType {
    MatchWinner,
    OverUnder { line: Decimal },
    AsianHandicap { line: Decimal },
    BothTeamsToScore,
    CorrectScore,
    FirstGoalscorer,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OddsFormat {
    Decimal { home: Decimal, draw: Option<Decimal>, away: Decimal },
    American { home: i32, draw: Option<i32>, away: i32 },
    Fractional { home: String, draw: Option<String>, away: String },
}

impl OddsFormat {
    pub fn to_decimal(&self) -> Result<(Decimal, Option<Decimal>, Decimal)> {
        match self {
            OddsFormat::Decimal { home, draw, away } => Ok((*home, *draw, *away)),
            OddsFormat::American { home, draw, away } => {
                let home_decimal = american_to_decimal(*home)?;
                let away_decimal = american_to_decimal(*away)?;
                let draw_decimal = draw.map(|d| american_to_decimal(d)).transpose()?;
                Ok((home_decimal, draw_decimal, away_decimal))
            }
            OddsFormat::Fractional { home, draw, away } => {
                let home_decimal = fractional_to_decimal(home)?;
                let away_decimal = fractional_to_decimal(away)?;
                let draw_decimal = draw.as_ref().map(|d| fractional_to_decimal(d)).transpose()?;
                Ok((home_decimal, draw_decimal, away_decimal))
            }
        }
    }
    
    pub fn to_implied_probabilities(&self) -> Result<(f64, Option<f64>, f64)> {
        let (home_odds, draw_odds, away_odds) = self.to_decimal()?;
        
        let home_prob = 1.0 / home_odds.to_f64().unwrap();
        let away_prob = 1.0 / away_odds.to_f64().unwrap();
        let draw_prob = draw_odds.map(|odds| 1.0 / odds.to_f64().unwrap());
        
        Ok((home_prob, draw_prob, away_prob))
    }
    
    pub fn calculate_overround(&self) -> Result<f64> {
        let (home_prob, draw_prob, away_prob) = self.to_implied_probabilities()?;
        let total = home_prob + away_prob + draw_prob.unwrap_or(0.0);
        Ok(total)
    }
    
    pub fn has_value(&self, true_home_prob: f64, true_away_prob: f64, true_draw_prob: Option<f64>) -> Result<ValueBet> {
        let (implied_home, implied_draw, implied_away) = self.to_implied_probabilities()?;
        
        let mut value_bets = Vec::new();
        
        // Check home value
        if true_home_prob > implied_home {
            let (home_odds, _, _) = self.to_decimal()?;
            value_bets.push(ValueBetType::Home {
                true_prob: true_home_prob,
                implied_prob: implied_home,
                odds: home_odds,
                expected_value: (true_home_prob * home_odds.to_f64().unwrap()) - 1.0,
            });
        }
        
        // Check away value
        if true_away_prob > implied_away {
            let (_, _, away_odds) = self.to_decimal()?;
            value_bets.push(ValueBetType::Away {
                true_prob: true_away_prob,
                implied_prob: implied_away,
                odds: away_odds,
                expected_value: (true_away_prob * away_odds.to_f64().unwrap()) - 1.0,
            });
        }
        
        // Check draw value if applicable
        if let (Some(true_draw), Some(implied_draw)) = (true_draw_prob, implied_draw) {
            if true_draw > implied_draw {
                let (_, draw_odds, _) = self.to_decimal()?;
                if let Some(draw_odds) = draw_odds {
                    value_bets.push(ValueBetType::Draw {
                        true_prob: true_draw,
                        implied_prob: implied_draw,
                        odds: draw_odds,
                        expected_value: (true_draw * draw_odds.to_f64().unwrap()) - 1.0,
                    });
                }
            }
        }
        
        Ok(ValueBet {
            match_id: String::new(), // Will be set by caller
            opportunities: value_bets,
            timestamp: Utc::now(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueBet {
    pub match_id: String,
    pub opportunities: Vec<ValueBetType>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValueBetType {
    Home { true_prob: f64, implied_prob: f64, odds: Decimal, expected_value: f64 },
    Away { true_prob: f64, implied_prob: f64, odds: Decimal, expected_value: f64 },
    Draw { true_prob: f64, implied_prob: f64, odds: Decimal, expected_value: f64 },
}

impl ValueBetType {
    pub fn expected_value(&self) -> f64 {
        match self {
            ValueBetType::Home { expected_value, .. } => *expected_value,
            ValueBetType::Away { expected_value, .. } => *expected_value,
            ValueBetType::Draw { expected_value, .. } => *expected_value,
        }
    }
    
    pub fn odds(&self) -> Decimal {
        match self {
            ValueBetType::Home { odds, .. } => *odds,
            ValueBetType::Away { odds, .. } => *odds,
            ValueBetType::Draw { odds, .. } => *odds,
        }
    }
}

fn american_to_decimal(american: i32) -> Result<Decimal> {
    if american == 0 {
        return Err(QuantsError::InvalidOdds("American odds cannot be zero".to_string()));
    }
    
    let decimal = if american > 0 {
        Decimal::from(american) / Decimal::from(100) + Decimal::ONE
    } else {
        Decimal::from(100) / Decimal::from(-american) + Decimal::ONE
    };
    
    Ok(decimal)
}

fn fractional_to_decimal(fractional: &str) -> Result<Decimal> {
    let parts: Vec<&str> = fractional.split('/').collect();
    if parts.len() != 2 {
        return Err(QuantsError::InvalidOdds(format!("Invalid fractional odds format: {}", fractional)));
    }
    
    let numerator: i32 = parts[0].parse()
        .map_err(|_| QuantsError::InvalidOdds(format!("Invalid numerator: {}", parts[0])))?;
    let denominator: i32 = parts[1].parse()
        .map_err(|_| QuantsError::InvalidOdds(format!("Invalid denominator: {}", parts[1])))?;
    
    if denominator == 0 {
        return Err(QuantsError::InvalidOdds("Denominator cannot be zero".to_string()));
    }
    
    Ok(Decimal::from(numerator) / Decimal::from(denominator) + Decimal::ONE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_decimal_odds_conversion() {
        let odds = OddsFormat::Decimal {
            home: dec!(2.5),
            draw: Some(dec!(3.2)),
            away: dec!(2.8),
        };
        
        let (home, draw, away) = odds.to_decimal().unwrap();
        assert_eq!(home, dec!(2.5));
        assert_eq!(draw, Some(dec!(3.2)));
        assert_eq!(away, dec!(2.8));
    }
    
    #[test]
    fn test_implied_probabilities() {
        let odds = OddsFormat::Decimal {
            home: dec!(2.0),
            draw: Some(dec!(3.0)),
            away: dec!(4.0),
        };
        
        let (home_prob, draw_prob, away_prob) = odds.to_implied_probabilities().unwrap();
        assert!((home_prob - 0.5).abs() < 0.001);
        assert!((draw_prob.unwrap() - 0.333333).abs() < 0.001);
        assert!((away_prob - 0.25).abs() < 0.001);
    }
    
    #[test]
    fn test_american_to_decimal() {
        assert_eq!(american_to_decimal(100).unwrap(), dec!(2.0));
        assert_eq!(american_to_decimal(-100).unwrap(), dec!(2.0));
        assert_eq!(american_to_decimal(200).unwrap(), dec!(3.0));
        assert_eq!(american_to_decimal(-200).unwrap(), dec!(1.5));
    }
    
    #[test]
    fn test_fractional_to_decimal() {
        assert_eq!(fractional_to_decimal("1/1").unwrap(), dec!(2.0));
        assert_eq!(fractional_to_decimal("2/1").unwrap(), dec!(3.0));
        assert_eq!(fractional_to_decimal("1/2").unwrap(), dec!(1.5));
    }
}