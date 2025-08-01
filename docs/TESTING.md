# Testing Strategy for Quant-RS

This document outlines the comprehensive testing approach for the Quant-RS sports betting prediction and trading system.

## Test Structure

### 1. Unit Tests
Located in `tests/` directory, focusing on individual components:

#### ML Models Tests (`ml_models_tests.rs`)
- **LogisticRegressionModel**: Tests model creation, prediction accuracy, probability constraints
- **PoissonModel**: Tests goal prediction logic, lambda calculations, probability distributions  
- **EnsembleModel**: Tests model combination, weighted averaging, consensus predictions
- **Model Interface**: Tests unified model interface, feedback mechanisms, weight updates

**Key Test Cases:**
- Prediction consistency and determinism
- Probability validation (sum to 1, within bounds)
- Extreme value handling
- Missing feature robustness
- Model feedback and learning

#### Trading Engine Tests (`trading_engine_tests.rs`)
- **Portfolio Management**: Bankroll tracking, exposure limits, bet allocation
- **Risk Assessment**: Risk scoring, correlation analysis, position sizing
- **Signal Generation**: Edge detection, confidence thresholds, bet recommendations
- **Trade Execution**: Order placement, insufficient funds handling, concurrent bet limits

**Key Test Cases:**
- Market odds integration
- Profit/loss calculations
- ROI and performance metrics
- Risk management constraints
- Concurrent trading scenarios

### 2. Integration Tests (`integration_tests.rs`)
End-to-end API testing covering:

#### API Endpoints
- **Health Check**: System status verification
- **Events API**: Live events, pagination, filtering
- **Predictions API**: Recent predictions, match-specific queries
- **Portfolio API**: Balance tracking, trade history
- **Markets API**: Odds retrieval, market data

**Key Test Cases:**
- Request/response validation
- Error handling (404, 500)
- Pagination and limiting
- Concurrent request handling
- Data consistency across endpoints

### 3. Performance Tests (`performance_tests.rs`)
Load and performance validation:

#### Latency Tests
- **Prediction Latency**: <50ms average prediction time
- **Trading Throughput**: >50 predictions/second processing
- **Market Simulation**: >100 odds/second generation

#### Stress Tests
- **Concurrent Load**: 50 parallel prediction tasks
- **Memory Stability**: 1000+ event processing without leaks
- **Pipeline Throughput**: End-to-end processing >20 events/second

#### System Health
- **Metrics Collection**: High-frequency metric updates (>5000 ops/sec)
- **Resource Usage**: Memory and CPU stability under load
- **Error Recovery**: Graceful handling of failures

### 4. Property-Based Tests
Using `proptest` for randomized testing:

- **Model Probabilities**: Always sum to 1.0, within valid ranges
- **Portfolio Invariants**: Bankroll consistency, exposure tracking
- **API Response Format**: Consistent JSON schema validation
- **Calculation Accuracy**: Mathematical operations preserve invariants

## Test Categories

### Functional Tests
✅ **Unit Tests**: Component-level functionality
✅ **Integration Tests**: API and service integration  
✅ **End-to-End Tests**: Complete workflow validation

### Non-Functional Tests
✅ **Performance Tests**: Latency and throughput validation
✅ **Load Tests**: System behavior under stress
✅ **Memory Tests**: Resource usage and leak detection

### Quality Assurance
✅ **Property Tests**: Mathematical invariant validation
✅ **Error Handling**: Graceful failure scenarios
✅ **Edge Cases**: Boundary condition testing

## Running Tests

### Quick Test Run
```bash
# Run all tests
cargo test

# Run specific test suite
cargo test ml_models_tests
cargo test integration_tests
cargo test performance_tests
```

### Comprehensive Test Suite
```bash
# Run the full test suite with reporting
./scripts/run_tests.sh
```

### Performance Benchmarking
```bash
# Run performance tests in release mode
cargo test performance_tests --release
```

### Individual Component Testing
```bash
# Test specific crates
cargo test -p quant-models
cargo test -p quant-services
cargo test -p quant-api
```

## Test Data and Fixtures

### Standard Test Data
The `tests/mod.rs` module provides standard test fixtures:

- `create_standard_test_event()`: Consistent match events
- `create_standard_test_prediction()`: Standard prediction format
- `create_standard_test_odds()`: Market odds for testing

### Randomized Test Data
Performance tests use randomized data generation:

- Dynamic match events with varying attributes
- Random prediction probabilities within valid ranges
- Simulated market conditions and odds variations

## Success Criteria

### Unit Test Requirements
- ✅ 100% test coverage for critical paths
- ✅ All models produce valid probability distributions
- ✅ Trading engine respects risk management rules
- ✅ Error handling covers edge cases

### Integration Test Requirements  
- ✅ All API endpoints return correct response formats
- ✅ Error responses include appropriate status codes
- ✅ Concurrent requests handled without race conditions
- ✅ Data consistency maintained across services

### Performance Requirements
- ✅ Prediction latency < 50ms average
- ✅ Trading throughput > 50 signals/second
- ✅ API response time < 100ms for standard queries
- ✅ System remains stable under 10x normal load

### Quality Assurance
- ✅ No memory leaks during extended operation
- ✅ Mathematical invariants preserved under all conditions
- ✅ System degrades gracefully under resource constraints
- ✅ Recovery mechanisms work after failures

## Continuous Integration

### Pre-commit Hooks
```bash
# Run before each commit
cargo test --all
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

### CI Pipeline Steps
1. **Lint**: `cargo clippy` with strict warnings
2. **Format**: `cargo fmt` validation  
3. **Unit Tests**: `cargo test --all`
4. **Integration Tests**: Full API test suite
5. **Performance Tests**: Benchmark validation
6. **Security Audit**: `cargo audit` dependency check

## Test Environment Setup

### Dependencies
```toml
[dev-dependencies]
proptest = "1.4"      # Property-based testing
tokio-test = "0.4"    # Async test utilities
hyper = "1.0"         # HTTP client for API tests
tower = "0.4"         # Service testing utilities
serde_json = "1.0"    # JSON validation
uuid = "1.6"          # Test data generation
rand = "0.8"          # Randomized testing
```

### Environment Variables
```bash
export RUST_LOG=debug                    # Enable debug logging
export QUANT_TEST_MODE=true             # Enable test-specific behavior
export QUANT_DISABLE_RATE_LIMITS=true   # Disable rate limiting in tests
```

## Monitoring and Reporting

### Test Metrics
- **Coverage**: Aim for >90% code coverage
- **Performance**: Track latency and throughput trends
- **Reliability**: Monitor test flakiness and failures
- **Quality**: Ensure mathematical invariants always hold

### Test Reports
The test suite generates detailed reports:
- Component-level pass/fail status
- Performance benchmark results
- Memory usage statistics
- Error analysis and debugging information

## Debugging Test Failures

### Common Issues
1. **Timing Issues**: Use `tokio-test` for deterministic async testing
2. **Floating Point**: Use appropriate tolerance for probability comparisons
3. **Resource Cleanup**: Ensure proper cleanup in async tests
4. **Race Conditions**: Use proper synchronization in concurrent tests

### Debug Tools
```bash
# Run with detailed output
cargo test -- --nocapture

# Run specific failing test
cargo test test_name -- --exact

# Enable tracing for debugging
RUST_LOG=trace cargo test
```

This comprehensive testing strategy ensures the Quant-RS system is robust, performant, and ready for production deployment.