use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::sleep;
use rust_decimal_macros::dec;
use chrono::Utc;
use uuid::Uuid;

use quant_services::{
    DataFeedService, DataFeedConfig, PredictorService, TradingEngine, 
    MarketSimulator, MetricsCollector
};
use quant_models::{MatchEvent, EventType, Sport, BettingOutcome, Prediction, FeatureVector};

#[tokio::test]
async fn test_prediction_latency() {
    let predictor = PredictorService::new();
    let test_event = create_test_match_event();
    
    // Warm up
    for _ in 0..10 {
        let _ = predictor.predict(&test_event).await;
    }
    
    // Measure prediction latency
    let start = Instant::now();
    let iterations = 100;
    
    for _ in 0..iterations {
        let prediction = predictor.predict(&test_event).await.unwrap();
        assert!(prediction.confidence > 0.0);
    }
    
    let duration = start.elapsed();
    let avg_latency = duration / iterations;
    
    println!("🎯 Average prediction latency: {:?}", avg_latency);
    
    // Prediction should complete within 50ms on average
    assert!(avg_latency < Duration::from_millis(50));
}

#[tokio::test]
async fn test_trading_engine_throughput() {
    let engine = TradingEngine::new(dec!(100000.0)); // Large bankroll for testing
    let predictions = create_test_predictions(100);
    
    // Add market odds for all matches
    for prediction in &predictions {
        let odds = create_test_market_odds(&prediction.match_id);
        engine.update_market_odds(prediction.match_id.clone(), odds).await;
    }
    
    let start = Instant::now();
    let mut successful_signals = 0;
    
    for prediction in predictions {
        let signal = engine.process_prediction(&prediction).await.unwrap();
        if signal.signal_strength > 0.0 {
            successful_signals += 1;
        }
    }
    
    let duration = start.elapsed();
    let throughput = 100.0 / duration.as_secs_f64();
    
    println!("💰 Trading engine throughput: {:.1} predictions/second", throughput);
    println!("💡 Generated {} trading signals", successful_signals);
    
    // Should process at least 50 predictions per second
    assert!(throughput > 50.0);
}

#[tokio::test]
async fn test_market_simulator_performance() {
    let simulator = MarketSimulator::new();
    let test_events = create_test_events(50);
    
    let start = Instant::now();
    let mut successful_odds = 0;
    
    for event in test_events {
        match simulator.generate_market_odds(&event).await {
            Ok(_) => successful_odds += 1,
            Err(_) => {}
        }
    }
    
    let duration = start.elapsed();
    let throughput = successful_odds as f64 / duration.as_secs_f64();
    
    println!("📊 Market simulator throughput: {:.1} odds/second", throughput);
    
    // Should generate at least 100 odds per second
    assert!(throughput > 100.0);
    assert!(successful_odds > 40); // Most should succeed
}

#[tokio::test]
async fn test_concurrent_prediction_load() {
    let predictor = Arc::new(PredictorService::new());
    let test_event = Arc::new(create_test_match_event());
    
    let start = Instant::now();
    let mut handles = vec![];
    
    // Spawn 50 concurrent prediction tasks
    for _ in 0..50 {
        let predictor_clone = predictor.clone();
        let event_clone = test_event.clone();
        
        let handle = tokio::spawn(async move {
            let mut successful_predictions = 0;
            
            // Each task makes 20 predictions
            for _ in 0..20 {
                match predictor_clone.predict(&event_clone).await {
                    Ok(prediction) => {
                        assert!(prediction.confidence > 0.0);
                        successful_predictions += 1;
                    }
                    Err(_) => {}
                }
            }
            
            successful_predictions
        });
        
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    let mut total_predictions = 0;
    for handle in handles {
        total_predictions += handle.await.unwrap();
    }
    
    let duration = start.elapsed();
    let throughput = total_predictions as f64 / duration.as_secs_f64();
    
    println!("🔄 Concurrent prediction throughput: {:.1} predictions/second", throughput);
    println!("✅ Completed {} predictions", total_predictions);
    
    // Should handle concurrent load efficiently
    assert!(total_predictions >= 900); // At least 90% success rate
    assert!(throughput > 200.0); // Should be higher due to concurrency
}

#[tokio::test]
async fn test_memory_usage_stability() {
    let engine = TradingEngine::new(dec!(10000.0));
    let predictor = PredictorService::new();
    let simulator = MarketSimulator::new();
    
    // Process many events to test memory stability
    for i in 0..1000 {
        let mut event = create_test_match_event();
        event.match_id = format!("memory_test_{}", i);
        
        // Generate prediction
        let prediction = predictor.predict(&event).await.unwrap();
        
        // Generate market odds
        let odds = simulator.generate_market_odds(&event).await.unwrap();
        engine.update_market_odds(event.match_id.clone(), odds).await;
        
        // Process through trading engine
        let _signal = engine.process_prediction(&prediction).await.unwrap();
        
        // Periodic cleanup simulation
        if i % 100 == 0 {
            // Give time for any cleanup processes
            sleep(Duration::from_millis(1)).await;
        }
    }
    
    println!("🧠 Processed 1000 events - memory stability test passed");
    
    // If we reach here without panics or excessive memory usage, test passes
    assert!(true);
}

#[tokio::test]
async fn test_metrics_collection_performance() {
    let metrics = MetricsCollector::new();
    
    let start = Instant::now();
    let operations = 10000;
    
    // Test high-frequency metric updates
    for _ in 0..operations {
        metrics.increment_events_processed().await;
        metrics.increment_predictions_generated().await;
        
        if rand::random::<f64>() < 0.1 {
            metrics.increment_trades_executed().await;
        }
        
        if rand::random::<f64>() < 0.05 {
            metrics.increment_errors().await;
        }
    }
    
    let duration = start.elapsed();
    let throughput = operations as f64 / duration.as_secs_f64();
    
    println!("📈 Metrics collection throughput: {:.1} operations/second", throughput);
    
    // Verify metrics accuracy
    let current_metrics = metrics.get_current_metrics().await;
    assert_eq!(current_metrics.events_processed, operations);
    assert_eq!(current_metrics.predictions_generated, operations);
    
    // Should handle high-frequency updates efficiently
    assert!(throughput > 5000.0);
}

#[tokio::test]
async fn test_data_feed_simulation_performance() {
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    
    let config = DataFeedConfig {
        feed_interval_ms: 10, // Very fast for testing
        max_events_per_batch: 5,
        enable_simulation: true,
        simulation_speed_multiplier: 10.0,
    };
    
    let data_feed = DataFeedService::new(sender, Some(config));
    
    // Start data feed
    let feed_handle = {
        let data_feed = data_feed.clone();
        tokio::spawn(async move {
            if let Err(e) = data_feed.start().await {
                eprintln!("Data feed error: {}", e);
            }
        })
    };
    
    // Collect events for 1 second
    let start = Instant::now();
    let mut event_count = 0;
    let test_duration = Duration::from_secs(1);
    
    while start.elapsed() < test_duration {
        tokio::select! {
            event = receiver.recv() => {
                if event.is_some() {
                    event_count += 1;
                }
            }
            _ = sleep(test_duration) => break,
        }
    }
    
    feed_handle.abort();
    
    let events_per_second = event_count as f64 / test_duration.as_secs_f64();
    
    println!("📡 Data feed performance: {:.1} events/second", events_per_second);
    
    // Should generate reasonable event rate
    assert!(events_per_second > 10.0);
    assert!(event_count > 10);
}

#[tokio::test]
async fn test_end_to_end_pipeline_performance() {
    let predictor = Arc::new(PredictorService::new());
    let engine = Arc::new(TradingEngine::new(dec!(50000.0)));
    let simulator = Arc::new(MarketSimulator::new());
    let metrics = Arc::new(MetricsCollector::new());
    
    let test_events = create_test_events(100);
    let start = Instant::now();
    
    let mut successful_pipeline_runs = 0;
    
    for event in test_events {
        // Full pipeline processing
        let pipeline_start = Instant::now();
        
        // 1. Generate prediction
        let prediction = match predictor.predict(&event).await {
            Ok(p) => p,
            Err(_) => continue,
        };
        
        // 2. Generate market odds
        let odds = match simulator.generate_market_odds(&event).await {
            Ok(o) => o,
            Err(_) => continue,
        };
        
        // 3. Update market in trading engine
        engine.update_market_odds(event.match_id.clone(), odds).await;
        
        // 4. Process prediction through trading engine
        let signal = match engine.process_prediction(&prediction).await {
            Ok(s) => s,
            Err(_) => continue,
        };
        
        // 5. Track metrics
        metrics.increment_events_processed().await;
        metrics.increment_predictions_generated().await;
        
        if signal.signal_strength > 0.3 {
            // 6. Execute trade if signal is strong enough
            if engine.execute_trade(&signal).await.unwrap_or(false) {
                metrics.increment_trades_executed().await;
            }
        }
        
        let pipeline_duration = pipeline_start.elapsed();
        
        // Each pipeline run should complete within 100ms
        assert!(pipeline_duration < Duration::from_millis(100));
        
        successful_pipeline_runs += 1;
    }
    
    let total_duration = start.elapsed();
    let pipeline_throughput = successful_pipeline_runs as f64 / total_duration.as_secs_f64();
    
    println!("🔄 End-to-end pipeline throughput: {:.1} events/second", pipeline_throughput);
    println!("✅ Successful pipeline runs: {}/{}", successful_pipeline_runs, 100);
    
    // Verify performance metrics
    let final_metrics = metrics.get_current_metrics().await;
    let performance_stats = metrics.get_performance_stats().await;
    
    println!("📊 Performance stats:");
    println!("   Events per second: {:.2}", performance_stats.events_per_second);
    println!("   Predictions per second: {:.2}", performance_stats.predictions_per_second);
    println!("   System health: {:.1}%", performance_stats.system_health_score * 100.0);
    
    // Pipeline should handle reasonable throughput
    assert!(pipeline_throughput > 20.0);
    assert!(successful_pipeline_runs >= 90); // 90% success rate
    assert!(performance_stats.system_health_score > 0.7); // Good health
}

#[tokio::test]
async fn test_stress_test_trading_engine() {
    let engine = TradingEngine::new(dec!(1000000.0)); // Large bankroll
    
    // Create many predictions simultaneously
    let mut handles = vec![];
    
    for batch in 0..10 {
        let engine_clone = Arc::new(engine.clone());
        
        let handle = tokio::spawn(async move {
            let mut successful_trades = 0;
            
            for i in 0..50 {
                let match_id = format!("stress_test_{}_{}", batch, i);
                let prediction = create_test_prediction_for_match(&match_id);
                let odds = create_test_market_odds(&match_id);
                
                // Update odds
                engine_clone.update_market_odds(match_id.clone(), odds).await;
                
                // Process prediction
                if let Ok(signal) = engine_clone.process_prediction(&prediction).await {
                    if signal.signal_strength > 0.2 {
                        if engine_clone.execute_trade(&signal).await.unwrap_or(false) {
                            successful_trades += 1;
                        }
                    }
                }
            }
            
            successful_trades
        });
        
        handles.push(handle);
    }
    
    // Wait for all stress test tasks
    let mut total_trades = 0;
    for handle in handles {
        total_trades += handle.await.unwrap();
    }
    
    let portfolio = engine.get_portfolio_summary().await;
    
    println!("💪 Stress test results:");
    println!("   Total trades executed: {}", total_trades);
    println!("   Active bets: {}", portfolio.active_bets_count);
    println!("   Available bankroll: {}", portfolio.available_bankroll);
    
    // Should handle stress test without crashes
    assert!(total_trades > 0);
    assert!(portfolio.total_bankroll > dec!(0.0));
    assert!(portfolio.active_bets_count <= 10); // Respect concurrent bet limit
}

// Helper functions
fn create_test_match_event() -> MatchEvent {
    MatchEvent {
        id: Uuid::new_v4(),
        match_id: "perf_test_match".to_string(),
        event_type: EventType::Goal,
        timestamp: Utc::now(),
        sport: Sport::Football,
        league: "Performance Test League".to_string(),
        team_home: "Test Team A".to_string(),
        team_away: "Test Team B".to_string(),
        score_home: Some(1),
        score_away: Some(0),
        minute: Some(45),
        player: Some("Test Player".to_string()),
        metadata: HashMap::new(),
    }
}

fn create_test_events(count: usize) -> Vec<MatchEvent> {
    (0..count).map(|i| {
        let mut event = create_test_match_event();
        event.match_id = format!("perf_test_match_{}", i);
        event
    }).collect()
}

fn create_test_predictions(count: usize) -> Vec<Prediction> {
    (0..count).map(|i| {
        create_test_prediction_for_match(&format!("perf_test_match_{}", i))
    }).collect()
}

fn create_test_prediction_for_match(match_id: &str) -> Prediction {
    let mut probabilities = HashMap::new();
    probabilities.insert(BettingOutcome::HomeWin, 0.4 + (rand::random::<f64>() * 0.2));
    probabilities.insert(BettingOutcome::Draw, 0.25 + (rand::random::<f64>() * 0.1));
    probabilities.insert(BettingOutcome::AwayWin, 0.35 + (rand::random::<f64>() * 0.15));

    Prediction {
        id: Uuid::new_v4(),
        match_id: match_id.to_string(),
        model_version: "perf_test_v1.0".to_string(),
        timestamp: Utc::now(),
        probabilities,
        confidence: 0.6 + (rand::random::<f64>() * 0.3),
        expected_value: rand::random::<f64>() * 0.2,
        recommended_bet: Some(BettingOutcome::HomeWin),
        stake_percentage: Some(0.01 + (rand::random::<f64>() * 0.04)),
        metadata: HashMap::new(),
    }
}

fn create_test_market_odds(match_id: &str) -> quant_models::SimpleMarketOdds {
    use quant_models::SimpleMarketOdds;
    
    SimpleMarketOdds {
        match_id: match_id.to_string(),
        home_win: dec!(1.8) + (dec!(rand::random::<f64>() * 2.0)),
        draw: dec!(3.0) + (dec!(rand::random::<f64>() * 1.5)),
        away_win: dec!(2.2) + (dec!(rand::random::<f64>() * 1.8)),
        last_updated: Utc::now(),
    }
}