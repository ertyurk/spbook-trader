use quant_models::{MatchEvent, FeatureVector, EventType, MatchStatus};
use anyhow::Result;
use std::collections::HashMap;
use chrono::{DateTime, Utc, Timelike, Datelike};
use std::sync::{Arc, RwLock};
use dashmap::DashMap;

#[derive(Debug, Clone)]
pub struct TeamStats {
    pub goals_for: u32,
    pub goals_against: u32,
    pub shots: u32,
    pub possession: f64,
    pub yellow_cards: u32,
    pub red_cards: u32,
    pub corners: u32,
    pub fouls: u32,
    pub offsides: u32,
    pub recent_form: Vec<bool>, // Win = true, Loss/Draw = false
    pub elo_rating: f64,
    pub attack_strength: f64,
    pub defense_strength: f64,
}

impl Default for TeamStats {
    fn default() -> Self {
        Self {
            goals_for: 0,
            goals_against: 0,
            shots: 0,
            possession: 50.0,
            yellow_cards: 0,
            red_cards: 0,
            corners: 0,
            fouls: 0,
            offsides: 0,
            recent_form: Vec::new(),
            elo_rating: 1500.0, // Standard Elo starting rating
            attack_strength: 1.0,
            defense_strength: 1.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MatchContext {
    pub minute: u8,
    pub home_score: u8,
    pub away_score: u8,
    pub momentum: f64, // -1.0 (away) to +1.0 (home)
    pub intensity: f64, // 0.0 to 1.0
    pub last_goal_minute: Option<u8>,
    pub last_goal_team: Option<String>,
}

pub struct FeatureEngineer {
    team_stats: Arc<DashMap<String, TeamStats>>,
    match_contexts: Arc<DashMap<String, MatchContext>>,
    league_averages: Arc<RwLock<HashMap<String, LeagueAverages>>>,
}

#[derive(Debug, Clone)]
struct LeagueAverages {
    avg_goals_per_match: f64,
    avg_cards_per_match: f64,
    home_advantage: f64,
}

impl Default for LeagueAverages {
    fn default() -> Self {
        Self {
            avg_goals_per_match: 2.7,
            avg_cards_per_match: 4.2,
            home_advantage: 0.6, // 60% home win rate
        }
    }
}

impl FeatureEngineer {
    pub fn new() -> Self {
        Self {
            team_stats: Arc::new(DashMap::new()),
            match_contexts: Arc::new(DashMap::new()),
            league_averages: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn extract_features(&self, event: &MatchEvent) -> Result<FeatureVector> {
        self.update_context(event).await?;
        
        let mut features = HashMap::new();
        
        // Basic match state features
        self.add_match_state_features(&mut features, event);
        
        // Team performance features
        self.add_team_features(&mut features, event);
        
        // Situational features
        self.add_situational_features(&mut features, event);
        
        // Time-based features
        self.add_temporal_features(&mut features, event);
        
        // League context features
        self.add_league_features(&mut features, event);
        
        Ok(FeatureVector {
            match_id: event.match_id.clone(),
            features,
            timestamp: Utc::now(),
        })
    }
    
    async fn update_context(&self, event: &MatchEvent) -> Result<()> {
        let mut context = self.match_contexts
            .entry(event.match_id.clone())
            .or_insert_with(|| MatchContext {
                minute: 0,
                home_score: 0,
                away_score: 0,
                momentum: 0.0,
                intensity: 0.5,
                last_goal_minute: None,
                last_goal_team: None,
            });
        
        // Update based on event type
        match &event.event_type {
            EventType::Goal { team, minute, .. } => {
                if team == &event.team_home {
                    context.home_score += 1;
                    context.momentum = (context.momentum + 0.3).min(1.0);
                } else {
                    context.away_score += 1;
                    context.momentum = (context.momentum - 0.3).max(-1.0);
                }
                context.last_goal_minute = Some(*minute);
                context.last_goal_team = Some(team.clone());
                context.intensity = (context.intensity + 0.2).min(1.0);
            }
            EventType::Card { minute, .. } => {
                context.minute = *minute;
                context.intensity = (context.intensity + 0.1).min(1.0);
            }
            _ => {}
        }
        
        // Decay momentum over time
        let time_factor = 1.0 - (context.minute as f64 / 90.0) * 0.1;
        context.momentum *= time_factor;
        
        Ok(())
    }
    
    fn add_match_state_features(&self, features: &mut HashMap<String, f64>, event: &MatchEvent) {
        let context = self.match_contexts.get(&event.match_id);
        
        if let Some(ctx) = context {
            features.insert("minute".to_string(), ctx.minute as f64);
            features.insert("home_score".to_string(), ctx.home_score as f64);
            features.insert("away_score".to_string(), ctx.away_score as f64);
            features.insert("score_difference".to_string(), 
                           (ctx.home_score as i8 - ctx.away_score as i8) as f64);
            features.insert("total_goals".to_string(), 
                           (ctx.home_score + ctx.away_score) as f64);
            features.insert("momentum".to_string(), ctx.momentum);
            features.insert("intensity".to_string(), ctx.intensity);
            
            // Game phase features
            let game_phase = if ctx.minute <= 15 {
                0.0 // Early
            } else if ctx.minute <= 45 {
                1.0 // First half
            } else if ctx.minute <= 60 {
                2.0 // Early second half
            } else if ctx.minute <= 75 {
                3.0 // Mid second half
            } else {
                4.0 // Late game
            };
            features.insert("game_phase".to_string(), game_phase);
            
            // Time pressure
            let time_pressure = if ctx.minute > 80 { 1.0 } else { 0.0 };
            features.insert("time_pressure".to_string(), time_pressure);
        }
    }
    
    fn add_team_features(&self, features: &mut HashMap<String, f64>, event: &MatchEvent) {
        let home_stats = self.team_stats.entry(event.team_home.clone())
            .or_insert_with(TeamStats::default);
        let away_stats = self.team_stats.entry(event.team_away.clone())
            .or_insert_with(TeamStats::default);
        
        // Elo ratings
        features.insert("home_elo".to_string(), home_stats.elo_rating);
        features.insert("away_elo".to_string(), away_stats.elo_rating);
        features.insert("elo_difference".to_string(), 
                       home_stats.elo_rating - away_stats.elo_rating);
        
        // Attack/Defense strength
        features.insert("home_attack".to_string(), home_stats.attack_strength);
        features.insert("home_defense".to_string(), home_stats.defense_strength);
        features.insert("away_attack".to_string(), away_stats.attack_strength);
        features.insert("away_defense".to_string(), away_stats.defense_strength);
        
        // Expected goals based on strength
        let home_xg = home_stats.attack_strength * away_stats.defense_strength;
        let away_xg = away_stats.attack_strength * home_stats.defense_strength;
        features.insert("home_expected_goals".to_string(), home_xg);
        features.insert("away_expected_goals".to_string(), away_xg);
        
        // Form features
        let home_form = self.calculate_form_score(&home_stats.recent_form);
        let away_form = self.calculate_form_score(&away_stats.recent_form);
        features.insert("home_form".to_string(), home_form);
        features.insert("away_form".to_string(), away_form);
        features.insert("form_difference".to_string(), home_form - away_form);
        
        // Disciplinary record
        let home_discipline = (home_stats.yellow_cards + home_stats.red_cards * 2) as f64;
        let away_discipline = (away_stats.yellow_cards + away_stats.red_cards * 2) as f64;
        features.insert("home_discipline".to_string(), home_discipline);
        features.insert("away_discipline".to_string(), away_discipline);
    }
    
    fn add_situational_features(&self, features: &mut HashMap<String, f64>, event: &MatchEvent) {
        // Match status
        let status_value = match event.match_status {
            MatchStatus::Scheduled => 0.0,
            MatchStatus::Live => 1.0,
            MatchStatus::HalfTime => 2.0,
            MatchStatus::Finished => 3.0,
            _ => 0.0,
        };
        features.insert("match_status".to_string(), status_value);
        
        // Event type influence
        let event_influence = match &event.event_type {
            EventType::Goal { .. } => 1.0,
            EventType::Card { .. } => 0.7,
            EventType::HalfTime => 0.3,
            EventType::FullTime => 0.0,
            _ => 0.1,
        };
        features.insert("event_influence".to_string(), event_influence);
        
        // Home advantage
        features.insert("home_advantage".to_string(), 1.0);
    }
    
    fn add_temporal_features(&self, features: &mut HashMap<String, f64>, event: &MatchEvent) {
        let now = Utc::now();
        let hour = now.hour() as f64;
        let day_of_week = now.weekday().number_from_monday() as f64;
        
        // Time of day influence (evening games often have different dynamics)
        features.insert("hour_of_day".to_string(), hour);
        features.insert("is_evening".to_string(), if hour >= 18.0 { 1.0 } else { 0.0 });
        features.insert("day_of_week".to_string(), day_of_week);
        features.insert("is_weekend".to_string(), if day_of_week >= 6.0 { 1.0 } else { 0.0 });
    }
    
    fn add_league_features(&self, features: &mut HashMap<String, f64>, event: &MatchEvent) {
        let league_avgs = self.league_averages.read().unwrap();
        let avgs = league_avgs.get(&event.league)
            .cloned()
            .unwrap_or_default();
        
        features.insert("league_avg_goals".to_string(), avgs.avg_goals_per_match);
        features.insert("league_avg_cards".to_string(), avgs.avg_cards_per_match);
        features.insert("league_home_advantage".to_string(), avgs.home_advantage);
        
        // League competitiveness (based on league name)
        let competitiveness = match event.league.as_str() {
            "Premier League" => 0.95,
            "La Liga" => 0.90,
            "Bundesliga" => 0.85,
            "Serie A" => 0.85,
            "Ligue 1" => 0.80,
            _ => 0.70,
        };
        features.insert("league_competitiveness".to_string(), competitiveness);
    }
    
    fn calculate_form_score(&self, recent_form: &[bool]) -> f64 {
        if recent_form.is_empty() {
            return 0.5; // Neutral form
        }
        
        let wins = recent_form.iter().filter(|&&result| result).count() as f64;
        let total = recent_form.len() as f64;
        
        // Weight more recent games higher
        let mut weighted_score = 0.0;
        let mut total_weight = 0.0;
        
        for (i, &result) in recent_form.iter().rev().enumerate() {
            let weight = 1.0 / (i as f64 + 1.0);
            weighted_score += if result { weight } else { 0.0 };
            total_weight += weight;
        }
        
        if total_weight > 0.0 {
            weighted_score / total_weight
        } else {
            wins / total
        }
    }
    
    pub fn update_team_stats(&self, team: &str, goals_for: u32, goals_against: u32) {
        let mut stats = self.team_stats.entry(team.to_string())
            .or_insert_with(TeamStats::default);
        
        stats.goals_for += goals_for;
        stats.goals_against += goals_against;
        
        // Update Elo rating (simplified)
        let expected_score = 1.0 / (1.0 + 10_f64.powf((1500.0 - stats.elo_rating) / 400.0));
        let actual_score = if goals_for > goals_against { 1.0 } 
                          else if goals_for == goals_against { 0.5 } 
                          else { 0.0 };
        
        let k_factor = 32.0;
        stats.elo_rating += k_factor * (actual_score - expected_score);
        
        // Update attack/defense strength
        stats.attack_strength = (stats.goals_for as f64 / 10.0).max(0.1).min(3.0);
        stats.defense_strength = (10.0 / (stats.goals_against as f64 + 1.0)).max(0.1).min(3.0);
        
        // Update form
        stats.recent_form.push(actual_score > 0.5);
        if stats.recent_form.len() > 10 {
            stats.recent_form.remove(0);
        }
    }
    
    pub fn get_team_stats(&self, team: &str) -> Option<TeamStats> {
        self.team_stats.get(team).map(|entry| entry.clone())
    }
}