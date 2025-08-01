use sqlx::{FromRow, Type};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct MatchRecord {
    pub id: Uuid,
    pub match_id: String,
    pub team_home: String,
    pub team_away: String,
    pub league: String,
    pub season: String,
    pub match_date: DateTime<Utc>,
    pub status: String,
    pub home_score: Option<i32>,
    pub away_score: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct EventRecord {
    pub id: Uuid,
    pub match_id: String,
    pub event_type: String,
    pub timestamp: DateTime<Utc>,
    pub minute: Option<i32>,
    pub team: Option<String>,
    pub player: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct PredictionRecord {
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
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct BetRecord {
    pub id: Uuid,
    pub match_id: String,
    pub bet_type: String,
    pub stake: Decimal,
    pub odds: Decimal,
    pub expected_value: f64,
    pub kelly_fraction: f64,
    pub confidence: f64,
    pub strategy: String,
    pub status: String,
    pub placed_at: DateTime<Utc>,
    pub settled_at: Option<DateTime<Utc>>,
    pub payout: Option<Decimal>,
    pub profit_loss: Option<Decimal>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct OddsRecord {
    pub id: Uuid,
    pub match_id: String,
    pub bookmaker: String,
    pub market_type: String,
    pub home_odds: Option<Decimal>,
    pub draw_odds: Option<Decimal>,
    pub away_odds: Option<Decimal>,
    pub timestamp: DateTime<Utc>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ModelPerformanceRecord {
    pub id: Uuid,
    pub model_name: String,
    pub model_version: String,
    pub total_predictions: i32,
    pub correct_predictions: i32,
    pub accuracy: f64,
    pub log_loss: f64,
    pub brier_score: f64,
    pub roi: f64,
    pub sharpe_ratio: f64,
    pub max_drawdown: f64,
    pub calibration_slope: f64,
    pub calibration_intercept: f64,
    pub evaluation_period_start: DateTime<Utc>,
    pub evaluation_period_end: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}