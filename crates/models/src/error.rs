use thiserror::Error;

#[derive(Error, Debug)]
pub enum QuantsError {
    #[error("Invalid odds format: {0}")]
    InvalidOdds(String),
    
    #[error("Invalid probability: {prob}, must be between 0.0 and 1.0")]
    InvalidProbability { prob: f64 },
    
    #[error("Invalid stake amount: {amount}")]
    InvalidStake { amount: String },
    
    #[error("Match not found: {match_id}")]
    MatchNotFound { match_id: String },
    
    #[error("Model prediction failed: {reason}")]
    PredictionFailed { reason: String },
    
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Configuration error: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, QuantsError>;