// Message serialization and deserialization

use serde::{Serialize, Deserialize};
use quant_models::MatchEvent;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamMessage {
    pub id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub event: MatchEvent,
}