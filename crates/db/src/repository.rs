use crate::schema::*;
use sqlx::PgPool;
use anyhow::Result;

pub struct Repository {
    pool: PgPool,
}

impl Repository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

// Individual repository traits will be implemented here
pub trait MatchRepository {
    async fn create_match(&self, match_record: &MatchRecord) -> Result<MatchRecord>;
    async fn get_match(&self, match_id: &str) -> Result<Option<MatchRecord>>;
    async fn update_match(&self, match_record: &MatchRecord) -> Result<MatchRecord>;
}

pub trait PredictionRepository {
    async fn create_prediction(&self, prediction: &PredictionRecord) -> Result<PredictionRecord>;
    async fn get_predictions_for_match(&self, match_id: &str) -> Result<Vec<PredictionRecord>>;
}

pub trait BetRepository {
    async fn create_bet(&self, bet: &BetRecord) -> Result<BetRecord>;
    async fn update_bet_status(&self, bet_id: uuid::Uuid, status: &str) -> Result<()>;
    async fn get_active_bets(&self) -> Result<Vec<BetRecord>>;
}

// TODO: Implement these traits for Repository