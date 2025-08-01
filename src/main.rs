mod config;

use anyhow::Result;
use config::AppConfig;
use quant_models::MatchEvent;
use quant_services::{DataFeedService, DataFeedConfig, PredictorService, TradingEngine, MarketSimulator};
use rust_decimal_macros::dec;
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
    
    // Initialize prediction service
    let predictor = PredictorService::new();
    
    // Initialize trading engine with $10,000 starting bankroll
    let trading_engine = TradingEngine::new(dec!(10000.0));
    
    // Initialize market simulator
    let market_simulator = MarketSimulator::new();
    
    // Start event processor in background
    let processor_handle = tokio::spawn(async move {
        let mut event_count = 0;
        while let Some(event) = event_receiver.recv().await {
            event_count += 1;
            info!("🏈 Event #{}: {} - {:?} ({} vs {})", 
                  event_count,
                  event.match_id, 
                  event.event_type,
                  event.team_home,
                  event.team_away
            );
            
            // Generate market odds for this event
            let market_odds = match market_simulator.generate_market_odds(&event).await {
                Ok(odds) => {
                    trading_engine.update_market_odds(event.match_id.clone(), odds.clone()).await;
                    Some(odds)
                }
                Err(e) => {
                    warn!("📊 Failed to generate market odds for {}: {}", event.match_id, e);
                    None
                }
            };
            
            // Process event through prediction engine
            match predictor.predict(&event).await {
                Ok(prediction) => {
                    info!("🎯 Generated prediction - Most likely: {:?}", 
                          prediction.most_likely_outcome());
                    
                    // Send prediction to trading engine
                    match trading_engine.process_prediction(&prediction).await {
                        Ok(signal) => {
                            if signal.signal_strength > 0.0 {
                                info!("💡 Trading signal: {:.1}% strength - {}", 
                                      signal.signal_strength * 100.0,
                                      signal.reasoning);
                                
                                // Execute trade if signal is strong enough
                                if signal.signal_strength > 0.3 { // 30% threshold
                                    match trading_engine.execute_trade(&signal).await {
                                        Ok(executed) => {
                                            if executed {
                                                let summary = trading_engine.get_portfolio_summary().await;
                                                info!("💼 Portfolio: ${} available, {} active bets, ROI: {:.1}%",
                                                      summary.available_bankroll,
                                                      summary.active_bets_count,
                                                      summary.roi * 100.0);
                                            }
                                        }
                                        Err(e) => error!("❌ Trade execution failed: {}", e),
                                    }
                                }
                            }
                        }
                        Err(e) => error!("❌ Trading signal generation failed: {}", e),
                    }
                    
                    // TODO: Store prediction in database
                }
                Err(e) => {
                    error!("❌ Prediction failed for {}: {}", event.match_id, e);
                }
            }
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