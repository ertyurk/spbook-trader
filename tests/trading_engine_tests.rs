use std::collections::HashMap;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use chrono::Utc;

use quant_services::trader::{TradingEngine, TradingSignal, RiskAssessment, RiskManager};
use quant_models::{
    Prediction, BettingDecision, BetType, BettingStrategy, Portfolio, 
    BettingOutcome, SimpleMarketOdds, RiskTolerance, PortfolioSummary
};

#[tokio::test]
async fn test_trading_engine_creation() {
    let engine = TradingEngine::new(dec!(10000.0));
    let portfolio = engine.get_portfolio_summary().await;
    
    assert_eq!(portfolio.total_bankroll, dec!(10000.0));
    assert_eq!(portfolio.available_bankroll, dec!(10000.0));
    assert_eq!(portfolio.total_exposure, dec!(0.0));
    assert_eq!(portfolio.active_bets_count, 0);
    assert_eq!(portfolio.total_trades, 0);
}

#[tokio::test]
async fn test_market_odds_update() {
    let engine = TradingEngine::new(dec!(10000.0));
    
    let odds = SimpleMarketOdds {
        match_id: "test_match_123".to_string(),
        home_win: dec!(2.5),
        draw: dec!(3.2),
        away_win: dec!(2.8),
        last_updated: Utc::now(),
    };
    
    engine.update_market_odds("test_match_123".to_string(), odds.clone()).await;
    
    let retrieved_odds = engine.get_market_odds("test_match_123").await;
    assert!(retrieved_odds.is_some());
    let retrieved = retrieved_odds.unwrap();
    assert_eq!(retrieved.home_win, dec!(2.5));
    assert_eq!(retrieved.draw, dec!(3.2));
    assert_eq!(retrieved.away_win, dec!(2.8));
}

#[tokio::test]
async fn test_prediction_processing_no_odds() {
    let engine = TradingEngine::new(dec!(10000.0));
    let prediction = create_test_prediction();
    
    // Process prediction without market odds
    let signal = engine.process_prediction(&prediction).await.unwrap();
    
    assert_eq!(signal.match_id, "test_match_123");
    assert_eq!(signal.signal_strength, 0.0);
    assert!(signal.recommended_bet.is_none());
    assert_eq!(signal.reasoning, "No market odds available");
}

#[tokio::test]
async fn test_prediction_processing_with_odds() {
    let engine = TradingEngine::new(dec!(10000.0));
    let prediction = create_test_prediction();
    
    // Add market odds
    let odds = SimpleMarketOdds {
        match_id: "test_match_123".to_string(),
        home_win: dec!(3.0), // Implies 33.3% probability
        draw: dec!(3.5),     // Implies 28.6% probability
        away_win: dec!(2.5), // Implies 40% probability
        last_updated: Utc::now(),
    };
    engine.update_market_odds("test_match_123".to_string(), odds).await;
    
    let signal = engine.process_prediction(&prediction).await.unwrap();
    
    assert_eq!(signal.match_id, "test_match_123");
    // Should have positive signal strength since our prediction (50% home win) 
    // is higher than market implied probability (33.3%)
    assert!(signal.signal_strength > 0.0);
}

#[tokio::test]
async fn test_trading_signal_generation_with_edge() {
    let engine = TradingEngine::new(dec!(10000.0));
    
    // Create prediction favoring home win
    let mut prediction = create_test_prediction();
    prediction.probabilities.insert(BettingOutcome::HomeWin, 0.6); // 60% chance
    prediction.probabilities.insert(BettingOutcome::Draw, 0.2);    // 20% chance
    prediction.probabilities.insert(BettingOutcome::AwayWin, 0.2); // 20% chance
    prediction.confidence = 0.8;
    
    // Market odds that undervalue home win
    let odds = SimpleMarketOdds {
        match_id: "test_match_123".to_string(),
        home_win: dec!(2.5), // Implies 40% probability, we predict 60%
        draw: dec!(4.0),
        away_win: dec!(4.0),
        last_updated: Utc::now(),
    };
    engine.update_market_odds("test_match_123".to_string(), odds).await;
    
    let signal = engine.process_prediction(&prediction).await.unwrap();
    
    assert!(signal.signal_strength > 0.3); // Should have strong signal
    assert!(signal.recommended_bet.is_some());
    
    let bet = signal.recommended_bet.unwrap();
    assert_eq!(bet.bet_type, BetType::HomeWin);
    assert!(bet.stake > dec!(0.0));
    assert!(signal.reasoning.contains("Home win edge"));
}

#[tokio::test]
async fn test_no_edge_no_bet() {
    let engine = TradingEngine::new(dec!(10000.0));
    let prediction = create_test_prediction();
    
    // Market odds that are fairly priced (no edge)
    let odds = SimpleMarketOdds {
        match_id: "test_match_123".to_string(),
        home_win: dec!(2.0), // Implies 50% probability, matches our prediction
        draw: dec!(4.0),     // Implies 25% probability, matches our prediction
        away_win: dec!(4.0), // Implies 25% probability, matches our prediction
        last_updated: Utc::now(),
    };
    engine.update_market_odds("test_match_123".to_string(), odds).await;
    
    let signal = engine.process_prediction(&prediction).await.unwrap();
    
    // Should have low signal strength due to no edge
    assert!(signal.signal_strength < 0.1);
}

#[tokio::test]
async fn test_bet_execution_success() {
    let engine = TradingEngine::new(dec!(10000.0));
    
    let signal = TradingSignal {
        match_id: "test_match_123".to_string(),
        signal_strength: 0.6,
        recommended_bet: Some(BettingDecision {
            id: uuid::Uuid::new_v4(),
            match_id: "test_match_123".to_string(),
            bet_type: BetType::HomeWin,
            stake: dec!(100.0),
            odds: dec!(2.5),
            expected_return: dec!(250.0),
            confidence: 0.7,
            strategy: "moderate".to_string(),
            timestamp: Utc::now(),
        }),
        risk_assessment: RiskAssessment::default(),
        reasoning: "Test bet".to_string(),
    };
    
    let executed = engine.execute_trade(&signal).await.unwrap();
    assert!(executed);
    
    let portfolio = engine.get_portfolio_summary().await;
    assert_eq!(portfolio.available_bankroll, dec!(9900.0)); // 10000 - 100
    assert_eq!(portfolio.total_exposure, dec!(100.0));
    assert_eq!(portfolio.active_bets_count, 1);
    assert_eq!(portfolio.total_trades, 1);
}

#[tokio::test]
async fn test_bet_execution_insufficient_funds() {
    let engine = TradingEngine::new(dec!(50.0)); // Small bankroll
    
    let signal = TradingSignal {
        match_id: "test_match_123".to_string(),
        signal_strength: 0.6,
        recommended_bet: Some(BettingDecision {
            id: uuid::Uuid::new_v4(),
            match_id: "test_match_123".to_string(),
            bet_type: BetType::HomeWin,
            stake: dec!(100.0), // More than available
            odds: dec!(2.5),
            expected_return: dec!(250.0),
            confidence: 0.7,
            strategy: "moderate".to_string(),
            timestamp: Utc::now(),
        }),
        risk_assessment: RiskAssessment::default(),
        reasoning: "Test bet".to_string(),
    };
    
    let executed = engine.execute_trade(&signal).await.unwrap();
    assert!(!executed); // Should not execute due to insufficient funds
    
    let portfolio = engine.get_portfolio_summary().await;
    assert_eq!(portfolio.available_bankroll, dec!(50.0)); // Unchanged
    assert_eq!(portfolio.active_bets_count, 0);
    assert_eq!(portfolio.total_trades, 0);
}

#[tokio::test]
async fn test_portfolio_summary_calculations() {
    let engine = TradingEngine::new(dec!(10000.0));
    
    // Place multiple bets
    for i in 1..=3 {
        let signal = TradingSignal {
            match_id: format!("match_{}", i),
            signal_strength: 0.5,
            recommended_bet: Some(BettingDecision {
                id: uuid::Uuid::new_v4(),
                match_id: format!("match_{}", i),
                bet_type: BetType::HomeWin,
                stake: dec!(200.0),
                odds: dec!(2.0),
                expected_return: dec!(400.0),
                confidence: 0.6,
                strategy: "moderate".to_string(),
                timestamp: Utc::now(),
            }),
            risk_assessment: RiskAssessment::default(),
            reasoning: "Test bet".to_string(),
        };
        
        engine.execute_trade(&signal).await.unwrap();
    }
    
    let portfolio = engine.get_portfolio_summary().await;
    assert_eq!(portfolio.total_bankroll, dec!(10000.0));
    assert_eq!(portfolio.available_bankroll, dec!(9400.0)); // 10000 - (3 * 200)
    assert_eq!(portfolio.total_exposure, dec!(600.0));      // 3 * 200
    assert_eq!(portfolio.active_bets_count, 3);
    assert_eq!(portfolio.total_trades, 3);
}

#[tokio::test]
async fn test_risk_assessment() {
    let engine = TradingEngine::new(dec!(10000.0));
    
    let bet = BettingDecision {
        id: uuid::Uuid::new_v4(),
        match_id: "test_match_123".to_string(),
        bet_type: BetType::HomeWin,
        stake: dec!(1000.0), // 10% of bankroll
        odds: dec!(2.0),
        expected_return: dec!(2000.0),
        confidence: 0.8,
        strategy: "aggressive".to_string(),
        timestamp: Utc::now(),
    };
    
    let risk_assessment = engine.assess_risk("test_match_123", &Some(bet)).await;
    
    assert!(risk_assessment.risk_score >= 0.0 && risk_assessment.risk_score <= 1.0);
    assert!(risk_assessment.portfolio_impact >= 0.0);
    // High stake should generate some warnings
    assert!(!risk_assessment.warnings.is_empty());
}

#[tokio::test]
async fn test_concurrent_bet_limit() {
    let engine = TradingEngine::new(dec!(10000.0));
    
    // Place maximum number of concurrent bets (10)
    for i in 1..=12 { // Try to place 12 bets, should only execute 10
        let signal = TradingSignal {
            match_id: format!("match_{}", i),
            signal_strength: 0.5,
            recommended_bet: Some(BettingDecision {
                id: uuid::Uuid::new_v4(),
                match_id: format!("match_{}", i),
                bet_type: BetType::HomeWin,
                stake: dec!(100.0),
                odds: dec!(2.0),
                expected_return: dec!(200.0),
                confidence: 0.6,
                strategy: "moderate".to_string(),
                timestamp: Utc::now(),
            }),
            risk_assessment: RiskAssessment::default(),
            reasoning: "Test bet".to_string(),
        };
        
        engine.execute_trade(&signal).await.unwrap();
    }
    
    let portfolio = engine.get_portfolio_summary().await;
    assert_eq!(portfolio.active_bets_count, 10); // Should be capped at 10
    assert_eq!(portfolio.total_exposure, dec!(1000.0)); // 10 * 100
}

#[tokio::test]
async fn test_strategy_selection() {
    let engine = TradingEngine::new(dec!(10000.0));
    
    // Test that engine uses appropriate strategy
    let strategy = engine.get_active_strategy().await;
    assert!(strategy.name == "conservative" || strategy.name == "moderate" || strategy.name == "aggressive");
    
    // Verify strategy properties
    assert!(strategy.risk_tolerance != RiskTolerance::VeryHigh); // Should not be extremely risky
    assert!(strategy.min_edge > 0.0); // Should require some edge
    assert!(strategy.max_stake_percentage <= 0.2); // Should not risk more than 20% on single bet
}

#[tokio::test]  
async fn test_bet_outcome_processing() {
    let engine = TradingEngine::new(dec!(10000.0));
    
    // Place a winning bet
    let winning_signal = TradingSignal {
        match_id: "winning_match".to_string(),
        signal_strength: 0.6,
        recommended_bet: Some(BettingDecision {
            id: uuid::Uuid::new_v4(),
            match_id: "winning_match".to_string(),
            bet_type: BetType::HomeWin,
            stake: dec!(100.0),
            odds: dec!(2.0),
            expected_return: dec!(200.0),
            confidence: 0.7,
            strategy: "moderate".to_string(),
            timestamp: Utc::now(),
        }),
        risk_assessment: RiskAssessment::default(),
        reasoning: "Test winning bet".to_string(),
    };
    
    engine.execute_trade(&winning_signal).await.unwrap();
    
    // Simulate bet outcome - win
    engine.process_bet_outcome("winning_match", BetType::HomeWin, true).await.unwrap();
    
    let portfolio = engine.get_portfolio_summary().await;
    assert_eq!(portfolio.total_bankroll, dec!(10100.0)); // 10000 + 100 profit
    assert_eq!(portfolio.available_bankroll, dec!(10100.0));
    assert_eq!(portfolio.active_bets_count, 0); // Bet should be settled
    assert!(portfolio.win_rate > 0.0); // Should have positive win rate
}

#[tokio::test]
async fn test_losing_bet_processing() {
    let engine = TradingEngine::new(dec!(10000.0));
    
    // Place a losing bet
    let losing_signal = TradingSignal {
        match_id: "losing_match".to_string(),
        signal_strength: 0.6,
        recommended_bet: Some(BettingDecision {
            id: uuid::Uuid::new_v4(),
            match_id: "losing_match".to_string(),
            bet_type: BetType::HomeWin,
            stake: dec!(100.0),
            odds: dec!(2.0),
            expected_return: dec!(200.0),
            confidence: 0.7,
            strategy: "moderate".to_string(),
            timestamp: Utc::now(),
        }),
        risk_assessment: RiskAssessment::default(),
        reasoning: "Test losing bet".to_string(),
    };
    
    engine.execute_trade(&losing_signal).await.unwrap();
    
    // Simulate bet outcome - loss
    engine.process_bet_outcome("losing_match", BetType::HomeWin, false).await.unwrap();
    
    let portfolio = engine.get_portfolio_summary().await;
    assert_eq!(portfolio.total_bankroll, dec!(9900.0)); // 10000 - 100 loss
    assert_eq!(portfolio.available_bankroll, dec!(9900.0));
    assert_eq!(portfolio.active_bets_count, 0); // Bet should be settled
    assert!(portfolio.profit_loss < dec!(0.0)); // Should show loss
}

#[tokio::test]
async fn test_roi_calculation() {
    let engine = TradingEngine::new(dec!(10000.0));
    
    // Place and win a bet
    let signal = TradingSignal {
        match_id: "roi_test_match".to_string(),
        signal_strength: 0.6,
        recommended_bet: Some(BettingDecision {
            id: uuid::Uuid::new_v4(),
            match_id: "roi_test_match".to_string(),
            bet_type: BetType::HomeWin,
            stake: dec!(1000.0),
            odds: dec!(2.0),
            expected_return: dec!(2000.0),
            confidence: 0.8,
            strategy: "moderate".to_string(),
            timestamp: Utc::now(),
        }),
        risk_assessment: RiskAssessment::default(),
        reasoning: "ROI test bet".to_string(),
    };
    
    engine.execute_trade(&signal).await.unwrap();
    engine.process_bet_outcome("roi_test_match", BetType::HomeWin, true).await.unwrap();
    
    let portfolio = engine.get_portfolio_summary().await;
    
    // ROI should be 10% (1000 profit on 10000 initial bankroll)
    let expected_roi = 0.1;
    assert!((portfolio.roi - expected_roi).abs() < 0.01);
}

// Helper functions
fn create_test_prediction() -> Prediction {
    let mut probabilities = HashMap::new();
    probabilities.insert(BettingOutcome::HomeWin, 0.5);  // 50%
    probabilities.insert(BettingOutcome::Draw, 0.25);    // 25%
    probabilities.insert(BettingOutcome::AwayWin, 0.25); // 25%

    Prediction {
        id: uuid::Uuid::new_v4(),
        match_id: "test_match_123".to_string(),
        model_version: "test_v1.0".to_string(),
        timestamp: Utc::now(),
        probabilities,
        confidence: 0.75,
        expected_value: 0.15,
        recommended_bet: Some(BettingOutcome::HomeWin),
        stake_percentage: Some(0.02),
        metadata: HashMap::new(),
    }
}

impl Default for RiskAssessment {
    fn default() -> Self {
        Self {
            risk_score: 0.3,
            correlation_risk: 0.0,
            liquidity_risk: 0.0,
            volatility_risk: 0.2,
            portfolio_impact: 0.1,
            warnings: vec![],
        }
    }
}