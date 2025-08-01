# Quant-RS  - !EXPERIMENTATION Do not rely on it

A high-performance **sports betting prediction and trading system** built in Rust. Quant-RS combines machine learning models, real-time data processing, and automated trading strategies to predict sports outcomes and execute profitable betting decisions.

## 🚀 Features

- **🎯 ML Prediction Engine**: Multiple models (Logistic Regression, Poisson, Ensemble) for accurate outcome prediction
- **💰 Automated Trading**: Risk-managed bet placement with portfolio optimization
- **⚡ Real-time Processing**: Live event streaming and instant prediction updates
- **📊 Market Simulation**: Dynamic odds generation and market condition modeling
- **🛡️ Risk Management**: Sophisticated bankroll management and exposure controls
- **📈 Performance Monitoring**: Comprehensive metrics collection and system health tracking
- **🌐 REST API**: Complete HTTP API for integration and monitoring
- **🏗️ Modular Architecture**: Clean separation of concerns with workspace-based structure

## 📁 Project Structure

```
quant-rs/
├── src/                    # Main application entry point
├── crates/                 # Modular workspace crates
│   ├── api/               # REST API endpoints and handlers
│   ├── models/            # Core data structures and types
│   ├── services/          # Business logic and services
│   ├── ml/                # Machine learning models and features
│   ├── db/                # Database operations and schema
│   └── stream/            # Real-time data streaming
├── tests/                 # Integration and unit tests
├── docs/                  # Documentation
├── scripts/               # Build and deployment scripts
└── migrations/            # Database migrations
```

## 🛠️ Prerequisites

### Required Software
- **Rust** (1.70+): [Install Rust](https://rustup.rs/)
- **PostgreSQL** (13+): [Install PostgreSQL](https://postgresql.org/download/)
- **Redis** (6+): [Install Redis](https://redis.io/download)

### System Requirements
- **OS**: macOS, Linux, or Windows
- **RAM**: 4GB minimum, 8GB recommended
- **CPU**: Multi-core recommended for optimal performance

## 🚀 Quick Start

### 1. Clone and Setup

```bash
# Clone the repository
git clone https://github.com/your-username/quant-rs.git
cd quant-rs

# Install dependencies
cargo build
```

### 2. Environment Configuration

Create a `.env` file in the project root:

```bash
# Database Configuration
DATABASE_URL=postgresql://username:password@localhost:5432/quant_rs
REDIS_URL=redis://localhost:6379

# Server Configuration
SERVER_HOST=127.0.0.1
SERVER_PORT=8080

# Logging
RUST_LOG=quant_rs=info,tower_http=debug

# Trading Configuration
INITIAL_BANKROLL=10000.00
MAX_EXPOSURE_PERCENTAGE=0.10
RISK_TOLERANCE=moderate
```

### 3. Database Setup

```bash
# Start PostgreSQL service
# On macOS with Homebrew:
brew services start postgresql

# On Linux:
sudo systemctl start postgresql

# Create database
createdb quant_rs

# Run migrations (if you have a migration tool setup)
# Otherwise the app will run without persistent storage
```

### 4. Redis Setup

```bash
# Start Redis service
# On macOS with Homebrew:
brew services start redis

# On Linux:
sudo systemctl start redis

# Verify Redis is running:
redis-cli ping
# Should return: PONG
```

### 5. Run the Application

```bash
# Run in development mode with logging
RUST_LOG=debug cargo run

# Or run in release mode for better performance
cargo run --release
```

The application will start and display:
```
🚀 Starting Quant-RS Sports Betting Prediction System
✅ Configuration loaded successfully
📊 Database: postgresql://localhost:5432/quant_rs
🔄 Redis: redis://localhost:6379
🌐 Server will bind to: 127.0.0.1:8080
🌐 REST API available at http://127.0.0.1:8080
```

## 🧪 Testing

### Run All Tests

```bash
# Run basic functionality tests (recommended first)
cargo test --test basic_functionality_test

# Run individual crate tests
cargo test -p quant-models
cargo test -p quant-services

# Run all tests (some may have compilation issues)
cargo test

# Run tests with output
cargo test -- --nocapture
```

### Run Test Suite Script

```bash
# Make the script executable
chmod +x scripts/run_tests.sh

# Run comprehensive test suite
./scripts/run_tests.sh
```

### Expected Test Results

✅ **Basic Functionality**: 8/8 tests should pass
✅ **Models Crate**: 13/14 tests should pass (1 floating point precision issue)
⚠️ **Integration Tests**: May have compilation issues (data model alignment needed)

## 🌐 API Usage

Once the application is running, you can interact with it via HTTP:

### Health Check
```bash
curl http://localhost:8080/health
```

### System Status
```bash
curl http://localhost:8080/api/v1/status
```

### Live Events
```bash
curl http://localhost:8080/api/v1/events/live
```

### Recent Predictions
```bash
curl http://localhost:8080/api/v1/predictions
```

### Portfolio Status
```bash
curl http://localhost:8080/api/v1/portfolio
```

### Available Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | System health check |
| `/api/v1/status` | GET | Detailed system status |
| `/api/v1/events` | GET | Recent match events (paginated) |
| `/api/v1/events/live` | GET | Live events (last 10) |
| `/api/v1/predictions` | GET | Recent predictions (paginated) |
| `/api/v1/predictions/{match_id}` | GET | Prediction for specific match |
| `/api/v1/portfolio` | GET | Portfolio status and performance |
| `/api/v1/markets` | GET | Current market odds |
| `/api/v1/odds/{match_id}` | GET | Odds for specific match |

## 📊 Monitoring

### View Real-time Logs
```bash
# Run with debug logging
RUST_LOG=debug cargo run

# Filter specific components
RUST_LOG=quant_rs::services=info,quant_rs::api=debug cargo run
```

### Performance Metrics
The system automatically logs performance metrics every 30 seconds:
- Events processed per second
- Prediction latency
- System health score
- Memory and CPU usage
- Error rates

### Example Output
```
📊 Performance: 2.3 pred/s, 5.1 events/s, 23.4ms avg latency, 94.2% health
💰 Trading signal: 65.3% strength - Home win edge: 12.4%
💼 Portfolio: $9,847 available, 3 active bets, ROI: 1.2%
```

## 🔧 Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | - | PostgreSQL connection string |
| `REDIS_URL` | `redis://localhost:6379` | Redis connection string |
| `SERVER_HOST` | `127.0.0.1` | Server bind address |
| `SERVER_PORT` | `8080` | Server port |
| `RUST_LOG` | `info` | Logging level |
| `INITIAL_BANKROLL` | `10000.0` | Starting bankroll amount |
| `MAX_EXPOSURE_PERCENTAGE` | `0.1` | Maximum exposure per match (10%) |
| `RISK_TOLERANCE` | `moderate` | Risk tolerance level |

### Configuration File
Edit `src/config.rs` to modify default configurations:
- Server settings
- Database connections
- Trading parameters
- ML model settings

## 🚀 Production Deployment

### Build for Production
```bash
# Build optimized release binary
cargo build --release

# The binary will be located at:
# target/release/quant-rs
```

### Docker Deployment (Optional)
```bash
# Build Docker image
docker build -t quant-rs .

# Run with docker-compose
docker-compose up -d
```

### Performance Tuning
- Use `--release` mode for production
- Configure appropriate `RUST_LOG` levels
- Monitor system resources
- Adjust trading parameters based on performance

## 🛡️ Security Considerations

- **Never commit** API keys or database credentials
- Use environment variables for sensitive configuration
- Enable proper logging without exposing sensitive data
- Implement rate limiting for production API usage
- Regular security audits with `cargo audit`

## 📚 Development

### Adding New Features
1. Create feature in appropriate crate (`crates/models`, `crates/services`, etc.)
2. Add tests in `tests/` directory
3. Update API endpoints if needed
4. Update documentation

### Code Quality
```bash
# Format code
cargo fmt

# Run linter
cargo clippy

# Check for security vulnerabilities
cargo audit
```

### Testing Strategy
- **Unit Tests**: Test individual components
- **Integration Tests**: Test API endpoints and workflows  
- **Performance Tests**: Validate system under load
- **Property Tests**: Verify mathematical invariants


## 🆘 Troubleshooting

### Common Issues

**Database Connection Failed**
```bash
# Check if PostgreSQL is running
pg_isready

# Verify connection string in .env
echo $DATABASE_URL
```

**Redis Connection Failed**
```bash
# Check if Redis is running
redis-cli ping

# Should return: PONG
```

**Compilation Errors**
```bash
# Clean and rebuild
cargo clean
cargo build

# Update dependencies
cargo update
```

**Tests Failing**
```bash
# Run basic tests first
cargo test --test basic_functionality_test

# Run individual crate tests
cargo test -p quant-models
```

### Performance Issues
- Check system resources (RAM, CPU)
- Reduce logging level in production
- Monitor database query performance
- Verify Redis is accessible

## 🎯 Next Steps

After getting the system running:

1. **Monitor Performance**: Watch the real-time metrics
2. **Test API Endpoints**: Try different API calls
3. **Review Predictions**: Analyze the ML model outputs
4. **Customize Configuration**: Adjust parameters for your use case
5. **Extend Functionality**: Add new features or models
