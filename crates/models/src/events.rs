use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MatchEvent {
    pub id: Uuid,
    pub match_id: String,
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
    pub team_home: String,
    pub team_away: String,
    pub league: String,
    pub season: String,
    pub match_status: MatchStatus,
    pub score: Option<Score>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EventType {
    MatchStart,
    Goal { team: String, player: Option<String>, minute: u8 },
    Card { team: String, player: String, card_type: CardType, minute: u8 },
    Substitution { team: String, player_in: String, player_out: String, minute: u8 },
    HalfTime,
    FullTime,
    MatchEnd,
    OddsUpdate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CardType {
    Yellow,
    Red,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MatchStatus {
    Scheduled,
    Live,
    HalfTime,
    Finished,
    Postponed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Score {
    pub home: u8,
    pub away: u8,
    pub half_time_home: Option<u8>,
    pub half_time_away: Option<u8>,
}

impl MatchEvent {
    pub fn new(
        match_id: String,
        event_type: EventType,
        team_home: String,
        team_away: String,
        league: String,
        season: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            match_id,
            timestamp: Utc::now(),
            event_type,
            team_home,
            team_away,
            league,
            season,
            match_status: MatchStatus::Scheduled,
            score: None,
            metadata: serde_json::Value::Null,
        }
    }
    
    pub fn with_score(mut self, score: Score) -> Self {
        self.score = Some(score);
        self
    }
    
    pub fn with_status(mut self, status: MatchStatus) -> Self {
        self.match_status = status;
        self
    }
    
    pub fn is_live(&self) -> bool {
        matches!(self.match_status, MatchStatus::Live | MatchStatus::HalfTime)
    }
    
    pub fn is_finished(&self) -> bool {
        matches!(self.match_status, MatchStatus::Finished)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_event_creation() {
        let event = MatchEvent::new(
            "match_123".to_string(),
            EventType::MatchStart,
            "Arsenal".to_string(),
            "Chelsea".to_string(),
            "Premier League".to_string(),
            "2024-25".to_string(),
        );
        
        assert_eq!(event.match_id, "match_123");
        assert_eq!(event.team_home, "Arsenal");
        assert_eq!(event.team_away, "Chelsea");
        assert!(!event.is_live());
        assert!(!event.is_finished());
    }
    
    #[test]
    fn test_match_status_helpers() {
        let mut event = MatchEvent::new(
            "match_123".to_string(),
            EventType::MatchStart,
            "Arsenal".to_string(),
            "Chelsea".to_string(),
            "Premier League".to_string(),
            "2024-25".to_string(),
        );
        
        event = event.with_status(MatchStatus::Live);
        assert!(event.is_live());
        assert!(!event.is_finished());
        
        event = event.with_status(MatchStatus::Finished);
        assert!(!event.is_live());
        assert!(event.is_finished());
    }
}