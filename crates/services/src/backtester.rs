// Backtesting service

pub struct BacktestService {
    name: String,
}

impl BacktestService {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}