mod config;

use anyhow::Result;
use config::AppConfig;
use quant_models::MatchEvent;
use quant_services::{DataFeedService, DataFeedConfig, PredictorService, TradingEngine, MarketSimulator, MetricsCollector};
use quant_api::{create_routes, AppState};
use rust_decimal_macros::dec;
use tower_http::cors::CorsLayer;
use std::sync::Arc;
use tokio::sync::RwLock;
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
    let config = Arc::new(AppConfig::new()?);
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
    let predictor = Arc::new(PredictorService::new());
    
    // Initialize trading engine with $10,000 starting bankroll
    let trading_engine = Arc::new(TradingEngine::new(dec!(10000.0)));
    
    // Initialize market simulator
    let market_simulator = Arc::new(MarketSimulator::new());
    
    // Initialize metrics collector
    let metrics_collector = Arc::new(MetricsCollector::new());
    
    // Start metrics collection
    metrics_collector.start_periodic_collection().await;
    
    // Storage for API endpoints
    let recent_events = Arc::new(RwLock::new(Vec::<MatchEvent>::new()));
    let recent_predictions = Arc::new(RwLock::new(Vec::new()));
    
    // Create API state
    let api_state = AppState {
        trading_engine: trading_engine.clone(),
        market_simulator: market_simulator.clone(),
        predictor: predictor.clone(),
        recent_events: recent_events.clone(),
        recent_predictions: recent_predictions.clone(),
    };
    
    // Start API server
    let api_handle = {
        let router = create_routes()
            .with_state(api_state)
            .layer(CorsLayer::permissive());
        let config_clone = config.clone();
        
        tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind(&config_clone.server_addr()).await.unwrap();
            info!("🌐 API server starting on {}", config_clone.server_addr());
            axum::serve(listener, router).await.unwrap();
        })
    };
    
    // Start event processor in background
    let processor_handle = {
        let metrics = metrics_collector.clone();
        let events_storage = recent_events.clone();
        let predictions_storage = recent_predictions.clone();
        
        tokio::spawn(async move {
            let mut event_count = 0;
            while let Some(event) = event_receiver.recv().await {
                event_count += 1;
                
                // Track metrics
                metrics.increment_events_processed().await;
                
                // Store event for API
                {
                    let mut events = events_storage.write().await;
                    events.push(event.clone());
                    if events.len() > 1000 {
                        events.remove(0); // Keep only last 1000 events
                    }
                }
                
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
                        metrics.increment_errors().await;
                        warn!("📊 Failed to generate market odds for {}: {}", event.match_id, e);
                        None
                    }
                };
                
                // Process event through prediction engine with latency tracking
                let prediction_tracker = metrics.start_latency_tracking("prediction".to_string());
                match predictor.predict(&event).await {
                    Ok(prediction) => {
                        prediction_tracker.finish(&metrics);
                        metrics.increment_predictions_generated().await;
                        
                        // Store prediction for API
                        {
                            let mut predictions = predictions_storage.write().await;
                            predictions.push(prediction.clone());
                            if predictions.len() > 500 {
                                predictions.remove(0); // Keep only last 500 predictions
                            }
                        }
                        
                        info!("🎯 Generated prediction - Most likely: {:?}", 
                              prediction.most_likely_outcome());
                        
                        // Send prediction to trading engine with latency tracking
                        let trading_tracker = metrics.start_latency_tracking("trading_decision".to_string());
                        match trading_engine.process_prediction(&prediction).await {
                            Ok(signal) => {
                                trading_tracker.finish(&metrics);
                                
                                if signal.signal_strength > 0.0 {
                                    info!("💡 Trading signal: {:.1}% strength - {}", 
                                          signal.signal_strength * 100.0,
                                          signal.reasoning);
                                    
                                    // Execute trade if signal is strong enough
                                    if signal.signal_strength > 0.3 { // 30% threshold
                                        match trading_engine.execute_trade(&signal).await {
                                            Ok(executed) => {
                                                if executed {
                                                    metrics.increment_trades_executed().await;
                                                    let summary = trading_engine.get_portfolio_summary().await;
                                                    info!("💼 Portfolio: ${} available, {} active bets, ROI: {:.1}%",
                                                          summary.available_bankroll,
                                                          summary.active_bets_count,
                                                          summary.roi * 100.0);
                                                }
                                            }
                                            Err(e) => {
                                                metrics.increment_errors().await;
                                                error!("❌ Trade execution failed: {}", e);
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                trading_tracker.finish(&metrics);
                                metrics.increment_errors().await;
                                error!("❌ Trading signal generation failed: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        prediction_tracker.finish(&metrics);
                        metrics.increment_errors().await;
                        error!("❌ Prediction failed for {}: {}", event.match_id, e);
                    }
                }
            }
        })
    };
    
    info!("✅ All services started successfully");
    info!("🎮 Running in simulation mode - generating live match events");
    info!("🌐 REST API available at http://{}", config.server_addr());
    info!("📊 Available endpoints:");
    info!("   GET  /health - Health check");
    info!("   GET  /api/v1/status - System status");
    info!("   GET  /api/v1/events/live - Live events");
    info!("   GET  /api/v1/predictions - Recent predictions");
    info!("   GET  /api/v1/portfolio - Portfolio status");
    info!("   GET  /api/v1/markets - Current market odds");
    info!("⌨️  Press Ctrl+C to stop");
    
    // Log performance summary periodically
    let final_metrics = metrics_collector.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            final_metrics.log_performance_summary().await;
        }
    });
    
    // Keep the application running
    tokio::signal::ctrl_c().await?;
    info!("👋 Shutting down gracefully");
    
    // Final performance summary
    metrics_collector.log_performance_summary().await;
    
    // Clean shutdown
    feed_handle.abort();
    processor_handle.abort();
    api_handle.abort();

    Ok(())
}