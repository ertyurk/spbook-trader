-- Initial database schema for the sports betting prediction system

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";

-- Matches table
CREATE TABLE matches (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    match_id VARCHAR(255) UNIQUE NOT NULL,
    team_home VARCHAR(255) NOT NULL,
    team_away VARCHAR(255) NOT NULL,
    league VARCHAR(255) NOT NULL,
    season VARCHAR(50) NOT NULL,
    match_date TIMESTAMPTZ NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'scheduled',
    home_score INTEGER,
    away_score INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Events table for match events
CREATE TABLE events (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    match_id VARCHAR(255) NOT NULL REFERENCES matches(match_id),
    event_type VARCHAR(100) NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    minute INTEGER,
    team VARCHAR(255),
    player VARCHAR(255),
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Predictions table
CREATE TABLE predictions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    match_id VARCHAR(255) NOT NULL REFERENCES matches(match_id),
    model_name VARCHAR(255) NOT NULL,
    model_version VARCHAR(50) NOT NULL,
    home_win_prob DOUBLE PRECISION NOT NULL CHECK (home_win_prob >= 0 AND home_win_prob <= 1),
    draw_prob DOUBLE PRECISION CHECK (draw_prob >= 0 AND draw_prob <= 1),
    away_win_prob DOUBLE PRECISION NOT NULL CHECK (away_win_prob >= 0 AND away_win_prob <= 1),
    confidence DOUBLE PRECISION NOT NULL CHECK (confidence >= 0 AND confidence <= 1),
    expected_goals_home DOUBLE PRECISION,
    expected_goals_away DOUBLE PRECISION,
    features_used TEXT[] NOT NULL DEFAULT '{}',
    prediction_timestamp TIMESTAMPTZ NOT NULL,
    match_timestamp TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT valid_probabilities CHECK (
        home_win_prob + away_win_prob + COALESCE(draw_prob, 0) <= 1.001
    )
);

-- Bets table
CREATE TABLE bets (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    match_id VARCHAR(255) NOT NULL REFERENCES matches(match_id),
    bet_type VARCHAR(100) NOT NULL,
    stake DECIMAL(12,2) NOT NULL CHECK (stake > 0),
    odds DECIMAL(8,2) NOT NULL CHECK (odds > 1.0),
    expected_value DOUBLE PRECISION NOT NULL,
    kelly_fraction DOUBLE PRECISION NOT NULL,
    confidence DOUBLE PRECISION NOT NULL,
    strategy VARCHAR(255) NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    placed_at TIMESTAMPTZ NOT NULL,
    settled_at TIMESTAMPTZ,
    payout DECIMAL(12,2),
    profit_loss DECIMAL(12,2),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Odds table for market data
CREATE TABLE odds (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    match_id VARCHAR(255) NOT NULL REFERENCES matches(match_id),
    bookmaker VARCHAR(255) NOT NULL,
    market_type VARCHAR(100) NOT NULL,
    home_odds DECIMAL(8,2),
    draw_odds DECIMAL(8,2),
    away_odds DECIMAL(8,2),
    timestamp TIMESTAMPTZ NOT NULL,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Model performance tracking
CREATE TABLE model_performance (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    model_name VARCHAR(255) NOT NULL,
    model_version VARCHAR(50) NOT NULL,
    total_predictions INTEGER NOT NULL DEFAULT 0,
    correct_predictions INTEGER NOT NULL DEFAULT 0,
    accuracy DOUBLE PRECISION NOT NULL DEFAULT 0,
    log_loss DOUBLE PRECISION NOT NULL DEFAULT 0,
    brier_score DOUBLE PRECISION NOT NULL DEFAULT 0,
    roi DOUBLE PRECISION NOT NULL DEFAULT 0,
    sharpe_ratio DOUBLE PRECISION NOT NULL DEFAULT 0,
    max_drawdown DOUBLE PRECISION NOT NULL DEFAULT 0,
    calibration_slope DOUBLE PRECISION NOT NULL DEFAULT 1.0,
    calibration_intercept DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    evaluation_period_start TIMESTAMPTZ NOT NULL,
    evaluation_period_end TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(model_name, model_version, evaluation_period_start)
);

-- Indexes for performance
CREATE INDEX idx_matches_match_id ON matches(match_id);
CREATE INDEX idx_matches_date ON matches(match_date);
CREATE INDEX idx_matches_league_season ON matches(league, season);
CREATE INDEX idx_matches_teams ON matches(team_home, team_away);

CREATE INDEX idx_events_match_id ON events(match_id);
CREATE INDEX idx_events_timestamp ON events(timestamp);
CREATE INDEX idx_events_type ON events(event_type);

CREATE INDEX idx_predictions_match_id ON predictions(match_id);
CREATE INDEX idx_predictions_model ON predictions(model_name, model_version);
CREATE INDEX idx_predictions_timestamp ON predictions(prediction_timestamp);

CREATE INDEX idx_bets_match_id ON bets(match_id);
CREATE INDEX idx_bets_status ON bets(status);
CREATE INDEX idx_bets_strategy ON bets(strategy);
CREATE INDEX idx_bets_placed_at ON bets(placed_at);

CREATE INDEX idx_odds_match_id ON odds(match_id);
CREATE INDEX idx_odds_bookmaker ON odds(bookmaker);
CREATE INDEX idx_odds_timestamp ON odds(timestamp);
CREATE INDEX idx_odds_active ON odds(is_active) WHERE is_active = TRUE;

CREATE INDEX idx_model_performance_model ON model_performance(model_name, model_version);
CREATE INDEX idx_model_performance_period ON model_performance(evaluation_period_start, evaluation_period_end);

-- Triggers for updated_at timestamps
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_matches_updated_at BEFORE UPDATE ON matches 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_bets_updated_at BEFORE UPDATE ON bets 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_model_performance_updated_at BEFORE UPDATE ON model_performance 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Views for common queries
CREATE VIEW active_matches AS
SELECT * FROM matches 
WHERE status IN ('scheduled', 'live', 'halftime')
ORDER BY match_date;

CREATE VIEW recent_predictions AS
SELECT p.*, m.team_home, m.team_away, m.league, m.match_date
FROM predictions p
JOIN matches m ON p.match_id = m.match_id
WHERE p.prediction_timestamp >= NOW() - INTERVAL '7 days'
ORDER BY p.prediction_timestamp DESC;

CREATE VIEW betting_summary AS
SELECT 
    strategy,
    COUNT(*) as total_bets,
    SUM(stake) as total_staked,
    SUM(CASE WHEN status = 'won' THEN 1 ELSE 0 END) as won_bets,
    SUM(CASE WHEN status = 'won' THEN payout ELSE 0 END) as total_payouts,
    SUM(COALESCE(profit_loss, 0)) as total_profit_loss,
    AVG(CASE WHEN status IN ('won', 'lost') THEN 
        CASE WHEN status = 'won' THEN 1.0 ELSE 0.0 END 
    END) as win_rate,
    CASE WHEN SUM(stake) > 0 THEN 
        SUM(COALESCE(profit_loss, 0)) / SUM(stake) 
    ELSE 0 END as roi
FROM bets
WHERE status IN ('won', 'lost')
GROUP BY strategy;

-- Comments for documentation
COMMENT ON TABLE matches IS 'Core match information and results';
COMMENT ON TABLE events IS 'Live match events (goals, cards, substitutions, etc.)';
COMMENT ON TABLE predictions IS 'Model predictions for match outcomes';
COMMENT ON TABLE bets IS 'Betting decisions and their outcomes';
COMMENT ON TABLE odds IS 'Market odds from various bookmakers';
COMMENT ON TABLE model_performance IS 'Performance metrics for prediction models';

COMMENT ON COLUMN predictions.features_used IS 'Array of feature names used in the prediction';
COMMENT ON COLUMN bets.kelly_fraction IS 'Kelly criterion fraction for optimal stake sizing';
COMMENT ON COLUMN model_performance.brier_score IS 'Lower is better probability calibration metric';
COMMENT ON COLUMN model_performance.calibration_slope IS 'Should be close to 1.0 for well-calibrated models';