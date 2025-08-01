use quant_models::{MatchEvent, EventType, MatchStatus, Score};
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::sync::{Arc, RwLock};
use dashmap::DashMap;

#[derive(Debug, Clone)]
pub struct DataFeedConfig {
    pub feed_interval_ms: u64,
    pub max_events_per_batch: usize,
    pub enable_simulation: bool,
    pub simulation_speed_multiplier: f64,
}

impl Default for DataFeedConfig {
    fn default() -> Self {
        Self {
            feed_interval_ms: 1000, // 1 second
            max_events_per_batch: 100,
            enable_simulation: true,
            simulation_speed_multiplier: 1.0,
        }
    }
}

#[derive(Clone)]
pub struct DataFeedService {
    event_sender: mpsc::UnboundedSender<MatchEvent>,
    config: DataFeedConfig,
    active_matches: Arc<DashMap<String, MatchState>>,
    simulation_data: Arc<RwLock<SimulationData>>,
}

#[derive(Debug, Clone)]
struct MatchState {
    pub match_id: String,
    pub team_home: String,
    pub team_away: String,
    pub league: String,
    pub season: String,
    pub status: MatchStatus,
    pub score: Option<Score>,
    pub minute: u8,
    pub last_event_time: DateTime<Utc>,
}

#[derive(Debug)]
struct SimulationData {
    pub matches: Vec<SimulatedMatch>,
    pub current_index: usize,
}

#[derive(Debug, Clone)]
struct SimulatedMatch {
    pub match_id: String,
    pub team_home: String,
    pub team_away: String,
    pub league: String,
    pub events: Vec<SimulatedEvent>,
}

#[derive(Debug, Clone)]
struct SimulatedEvent {
    pub minute: u8,
    pub event_type: EventType,
    pub team: Option<String>,
    pub player: Option<String>,
}

impl DataFeedService {
    pub fn new(
        event_sender: mpsc::UnboundedSender<MatchEvent>,
        config: Option<DataFeedConfig>,
    ) -> Self {
        let config = config.unwrap_or_default();
        let simulation_data = Arc::new(RwLock::new(SimulationData {
            matches: Self::generate_sample_matches(),
            current_index: 0,
        }));
        
        Self {
            event_sender,
            config,
            active_matches: Arc::new(DashMap::new()),
            simulation_data,
        }
    }
    
    pub async fn start(&self) -> Result<()> {
        tracing::info!("🎯 Starting DataFeedService");
        tracing::info!("⚙️  Feed interval: {}ms", self.config.feed_interval_ms);
        tracing::info!("📊 Max events per batch: {}", self.config.max_events_per_batch);
        tracing::info!("🎮 Simulation mode: {}", self.config.enable_simulation);
        
        let mut ticker = interval(Duration::from_millis(self.config.feed_interval_ms));
        
        loop {
            ticker.tick().await;
            
            if let Err(e) = self.process_feed_cycle().await {
                tracing::error!("❌ Error in feed cycle: {}", e);
                continue;
            }
        }
    }
    
    async fn process_feed_cycle(&self) -> Result<()> {
        if self.config.enable_simulation {
            self.process_simulation_events().await?;
        } else {
            // TODO: Implement real data source integration
            self.process_external_api_events().await?;
        }
        
        Ok(())
    }
    
    async fn process_simulation_events(&self) -> Result<()> {
        let mut events_sent = 0;
        let max_events = self.config.max_events_per_batch;
        
        // Clone the matches data to avoid holding the lock across await points
        let matches = {
            let simulation_data = self.simulation_data.read().unwrap();
            if simulation_data.matches.is_empty() {
                return Ok(());
            }
            simulation_data.matches.clone()
        };
        
        // Cycle through simulated matches
        for match_data in &matches {
            if events_sent >= max_events {
                break;
            }
            
            // Check if match is already active
            let mut match_state = self.active_matches.entry(match_data.match_id.clone())
                .or_insert_with(|| MatchState {
                    match_id: match_data.match_id.clone(),
                    team_home: match_data.team_home.clone(),
                    team_away: match_data.team_away.clone(),
                    league: match_data.league.clone(),
                    season: "2024-25".to_string(),
                    status: MatchStatus::Scheduled,
                    score: None,
                    minute: 0,
                    last_event_time: Utc::now(),
                });
            
            // Generate events based on match progression
            if let Some(event) = self.generate_next_event(&match_data, &match_state).await? {
                self.send_event(event).await?;
                events_sent += 1;
                
                // Update match state
                match_state.last_event_time = Utc::now();
                match_state.minute = match_state.minute.saturating_add(1);
                
                if match_state.minute >= 90 {
                    match_state.status = MatchStatus::Finished;
                }
            }
        }
        
        if events_sent > 0 {
            tracing::debug!("📡 Sent {} simulated events", events_sent);
        }
        
        Ok(())
    }
    
    async fn generate_next_event(
        &self,
        match_data: &SimulatedMatch,
        match_state: &MatchState,
    ) -> Result<Option<MatchEvent>> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        // Skip if match is finished
        if matches!(match_state.status, MatchStatus::Finished) {
            return Ok(None);
        }
        
        // Start match if scheduled
        if matches!(match_state.status, MatchStatus::Scheduled) {
            let event = MatchEvent::new(
                match_data.match_id.clone(),
                EventType::MatchStart,
                match_data.team_home.clone(),
                match_data.team_away.clone(),
                match_data.league.clone(),
                "2024-25".to_string(),
            ).with_status(MatchStatus::Live);
            
            return Ok(Some(event));
        }
        
        // Generate random events during live match
        if matches!(match_state.status, MatchStatus::Live) {
            let event_probability = rng.gen::<f64>();
            
            let event_type = if event_probability < 0.02 {
                // 2% chance of goal
                let scoring_team = if rng.gen_bool(0.5) {
                    match_data.team_home.clone()
                } else {
                    match_data.team_away.clone()
                };
                EventType::Goal {
                    team: scoring_team,
                    player: Some(format!("Player{}", rng.gen_range(1..=23))),
                    minute: match_state.minute,
                }
            } else if event_probability < 0.05 {
                // 3% chance of card
                let team = if rng.gen_bool(0.5) {
                    match_data.team_home.clone()
                } else {
                    match_data.team_away.clone()
                };
                EventType::Card {
                    team,
                    player: format!("Player{}", rng.gen_range(1..=23)),
                    card_type: if rng.gen_bool(0.8) {
                        quant_models::CardType::Yellow
                    } else {
                        quant_models::CardType::Red
                    },
                    minute: match_state.minute,
                }
            } else if match_state.minute == 45 {
                EventType::HalfTime
            } else if match_state.minute >= 90 {
                EventType::FullTime
            } else {
                return Ok(None); // No event this cycle
            };
            
            let mut event = MatchEvent::new(
                match_data.match_id.clone(),
                event_type,
                match_data.team_home.clone(),
                match_data.team_away.clone(),
                match_data.league.clone(),
                "2024-25".to_string(),
            ).with_status(if match_state.minute >= 90 {
                MatchStatus::Finished
            } else if match_state.minute == 45 {
                MatchStatus::HalfTime
            } else {
                MatchStatus::Live
            });
            
            // Update score if it's a goal
            if let EventType::Goal { ref team, .. } = event.event_type {
                let mut score = match_state.score.clone().unwrap_or(Score {
                    home: 0,
                    away: 0,
                    half_time_home: None,
                    half_time_away: None,
                });
                
                if team == &match_data.team_home {
                    score.home += 1;
                } else {
                    score.away += 1;
                }
                
                event = event.with_score(score);
            }
            
            return Ok(Some(event));
        }
        
        Ok(None)
    }
    
    async fn process_external_api_events(&self) -> Result<()> {
        // TODO: Implement integration with real sports data APIs
        // This would involve:
        // 1. Polling external API endpoints
        // 2. Parsing API responses into MatchEvent structs
        // 3. Rate limiting and error handling
        // 4. Deduplication of events
        
        tracing::debug!("🔌 External API integration not yet implemented");
        Ok(())
    }
    
    async fn send_event(&self, event: MatchEvent) -> Result<()> {
        if let Err(_) = self.event_sender.send(event.clone()) {
            tracing::error!("❌ Failed to send event - receiver dropped");
            return Err(anyhow::anyhow!("Event receiver has been dropped"));
        }
        
        tracing::debug!("📤 Sent event: {} - {:?}", event.match_id, event.event_type);
        Ok(())
    }
    
    fn generate_sample_matches() -> Vec<SimulatedMatch> {
        vec![
            SimulatedMatch {
                match_id: "epl_match_001".to_string(),
                team_home: "Arsenal".to_string(),
                team_away: "Chelsea".to_string(),
                league: "Premier League".to_string(),
                events: vec![],
            },
            SimulatedMatch {
                match_id: "epl_match_002".to_string(),
                team_home: "Manchester City".to_string(),
                team_away: "Liverpool".to_string(),
                league: "Premier League".to_string(),
                events: vec![],
            },
            SimulatedMatch {
                match_id: "laliga_match_001".to_string(),
                team_home: "Real Madrid".to_string(),
                team_away: "Barcelona".to_string(),
                league: "La Liga".to_string(),
                events: vec![],
            },
        ]
    }
    
    pub fn get_active_matches(&self) -> Vec<String> {
        self.active_matches.iter()
            .filter(|entry| !matches!(entry.value().status, MatchStatus::Finished))
            .map(|entry| entry.key().clone())
            .collect()
    }
    
    pub fn get_match_state(&self, match_id: &str) -> Option<MatchState> {
        self.active_matches.get(match_id).map(|entry| entry.value().clone())
    }
}