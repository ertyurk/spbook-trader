use config::{Config, ConfigError, Environment, File};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub server: ServerConfig,
    pub ml: MlConfig,
    pub trading: TradingConfig,
    pub monitoring: MonitoringConfig,
    pub external_apis: ExternalApiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub stream_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlConfig {
    pub model_update_interval_hours: u64,
    pub prediction_confidence_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingConfig {
    pub initial_bankroll: Decimal,
    pub max_stake_percent: f64,
    pub kelly_multiplier: f64,
    pub min_odds: Decimal,
    pub max_odds: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub metrics_port: u16,
    pub health_check_interval_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalApiConfig {
    pub sports_api_key: Option<String>,
    pub sports_api_base_url: String,
}

impl AppConfig {
    pub fn new() -> Result<Self, ConfigError> {
        let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "development".into());

        let config = Config::builder()
            // Start with default values
            .set_default("database.url", "postgresql://localhost:5432/quant_rs_dev")?
            .set_default("database.max_connections", 20)?
            .set_default("redis.url", "redis://localhost:6379")?
            .set_default("redis.stream_key", "sports_events")?
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 8080)?
            .set_default("ml.model_update_interval_hours", 24)?
            .set_default("ml.prediction_confidence_threshold", 0.7)?
            .set_default("trading.initial_bankroll", "10000.00")?
            .set_default("trading.max_stake_percent", 0.05)?
            .set_default("trading.kelly_multiplier", 0.5)?
            .set_default("trading.min_odds", "1.20")?
            .set_default("trading.max_odds", "10.00")?
            .set_default("monitoring.metrics_port", 9090)?
            .set_default("monitoring.health_check_interval_seconds", 30)?
            .set_default("external_apis.sports_api_base_url", "https://api.sportsdataapi.com")?
            // Add in settings from configuration file
            .add_source(File::with_name("config/default").required(false))
            .add_source(File::with_name(&format!("config/{}", run_mode)).required(false))
            .add_source(File::with_name("config/local").required(false))
            // Add in settings from environment variables
            .add_source(Environment::new().separator("_"))
            .build()?;

        config.try_deserialize()
    }
    
    pub fn database_url(&self) -> &str {
        &self.database.url
    }
    
    pub fn redis_url(&self) -> &str {
        &self.redis.url
    }
    
    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }
}