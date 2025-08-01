mod config;

use anyhow::Result;
use config::AppConfig;
use quant_models::MatchEvent;
use quant_services::{DataFeedService, DataFeedConfig};
use tokio::sync::mpsc;
use tracing::{info, warn, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenvy::dotenv().ok();
    
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "quant_rs=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("🚀 Starting Quant-RS Sports Betting Prediction System");

    // Load configuration
    let config = AppConfig::new()?;
    info!("✅ Configuration loaded successfully");
    info!("📊 Database: {}", config.database_url());
    info!("🔄 Redis: {}", config.redis_url());
    info!("🌐 Server will bind to: {}", config.server_addr());

    // Create event channel for internal communication
    let (event_sender, mut event_receiver) = mpsc::unbounded_channel::<MatchEvent>();
    
    // Initialize data feed service
    let feed_config = DataFeedConfig {
        feed_interval_ms: 2000, // 2 seconds for demo
        max_events_per_batch: 10,
        enable_simulation: true,
        simulation_speed_multiplier: 1.0,
    };
    
    let data_feed = DataFeedService::new(event_sender, Some(feed_config));
    
    // Start data feed service in background
    let feed_handle = {
        let data_feed = data_feed.clone();
        tokio::spawn(async move {
            if let Err(e) = data_feed.start().await {
                error!("❌ Data feed service error: {}", e);
            }
        })
    };
    
    // Start event processor in background
    let processor_handle = tokio::spawn(async move {
        let mut event_count = 0;
        while let Some(event) = event_receiver.recv().await {
            event_count += 1;
            info!("🏈 Received event #{}: {} - {:?} vs {} vs {}", 
                  event_count,
                  event.match_id, 
                  event.event_type,
                  event.team_home,
                  event.team_away
            );
            
            // TODO: Process event through prediction engine
            // TODO: Process event through trading engine
            // TODO: Store event in database
        }
    });
    
    info!("✅ All services started successfully");
    info!("🎮 Running in simulation mode - generating live match events");
    info!("⌨️  Press Ctrl+C to stop");
    
    // TODO: Initialize database connection and run migrations
    // TODO: Initialize Redis streams  
    // TODO: Start prediction engine
    // TODO: Start trading engine
    // TODO: Start REST API server
    
    // Keep the application running
    tokio::signal::ctrl_c().await?;
    info!("👋 Shutting down gracefully");
    
    // Clean shutdown
    feed_handle.abort();
    processor_handle.abort();

    Ok(())
}