use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc, Timelike};
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub timestamp: DateTime<Utc>,
    pub uptime_seconds: u64,
    pub events_processed: u64,
    pub predictions_generated: u64,
    pub trades_executed: u64,
    pub api_requests: u64,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub active_connections: u32,
    pub prediction_latency_ms: f64,
    pub trading_latency_ms: f64,
    pub error_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceStats {
    pub avg_prediction_time_ms: f64,
    pub avg_trading_decision_time_ms: f64,
    pub predictions_per_second: f64,
    pub events_per_second: f64,
    pub system_health_score: f64, // 0.0 to 1.0
    pub error_rate_percent: f64,
    pub memory_efficiency: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPerformance {
    pub model_name: String,
    pub accuracy: f64,
    pub precision: f64,
    pub recall: f64,
    pub f1_score: f64,
    pub roi: f64,
    pub sharpe_ratio: f64,
    pub total_predictions: u64,
    pub correct_predictions: u64,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct LatencyTracker {
    start_time: Instant,
    operation: String,
}

impl LatencyTracker {
    pub fn new(operation: String) -> Self {
        Self {
            start_time: Instant::now(),
            operation,
        }
    }
    
    pub fn finish(self, metrics: &MetricsCollector) {
        let duration = self.start_time.elapsed();
        metrics.record_operation_latency(&self.operation, duration);
    }
}

pub struct MetricsCollector {
    start_time: Instant,
    metrics: Arc<RwLock<SystemMetrics>>,
    operation_times: Arc<RwLock<HashMap<String, Vec<Duration>>>>,
    model_performance: Arc<RwLock<HashMap<String, ModelPerformance>>>,
    hourly_stats: Arc<RwLock<Vec<(DateTime<Utc>, SystemMetrics)>>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        let start_time = Instant::now();
        let initial_metrics = SystemMetrics {
            timestamp: Utc::now(),
            uptime_seconds: 0,
            events_processed: 0,
            predictions_generated: 0,
            trades_executed: 0,
            api_requests: 0,
            memory_usage_mb: 0.0,
            cpu_usage_percent: 0.0,
            active_connections: 0,
            prediction_latency_ms: 0.0,
            trading_latency_ms: 0.0,
            error_count: 0,
        };

        Self {
            start_time,
            metrics: Arc::new(RwLock::new(initial_metrics)),
            operation_times: Arc::new(RwLock::new(HashMap::new())),
            model_performance: Arc::new(RwLock::new(HashMap::new())),
            hourly_stats: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn increment_events_processed(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.events_processed += 1;
        metrics.timestamp = Utc::now();
        metrics.uptime_seconds = self.start_time.elapsed().as_secs();
    }

    pub async fn increment_predictions_generated(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.predictions_generated += 1;
    }

    pub async fn increment_trades_executed(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.trades_executed += 1;
    }

    pub async fn increment_api_requests(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.api_requests += 1;
    }

    pub async fn increment_errors(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.error_count += 1;
    }

    pub async fn update_active_connections(&self, count: u32) {
        let mut metrics = self.metrics.write().await;
        metrics.active_connections = count;
    }

    pub fn record_operation_latency(&self, operation: &str, duration: Duration) {
        tokio::spawn({
            let operation = operation.to_string();
            let operation_times = self.operation_times.clone();
            let metrics = self.metrics.clone();
            
            async move {
                let duration_ms = duration.as_secs_f64() * 1000.0;
                
                // Store individual operation time
                {
                    let mut times = operation_times.write().await;
                    times.entry(operation.clone())
                        .or_insert_with(Vec::new)
                        .push(duration);
                    
                    // Keep only last 1000 measurements per operation
                    if let Some(op_times) = times.get_mut(&operation) {
                        if op_times.len() > 1000 {
                            op_times.remove(0);
                        }
                    }
                }
                
                // Update relevant metric
                let mut metrics = metrics.write().await;
                match operation.as_str() {
                    "prediction" => metrics.prediction_latency_ms = duration_ms,
                    "trading_decision" => metrics.trading_latency_ms = duration_ms,
                    _ => {}
                }
            }
        });
    }

    pub fn start_latency_tracking(&self, operation: String) -> LatencyTracker {
        LatencyTracker::new(operation)
    }

    pub async fn get_current_metrics(&self) -> SystemMetrics {
        let mut metrics = self.metrics.read().await.clone();
        metrics.uptime_seconds = self.start_time.elapsed().as_secs();
        metrics.timestamp = Utc::now();
        
        // Update system resource usage (simplified)
        metrics.memory_usage_mb = self.get_memory_usage_mb().await;
        metrics.cpu_usage_percent = self.get_cpu_usage_percent().await;
        
        metrics
    }

    pub async fn get_performance_stats(&self) -> PerformanceStats {
        let metrics = self.get_current_metrics().await;
        let operation_times = self.operation_times.read().await;
        
        let avg_prediction_time = operation_times
            .get("prediction")
            .map(|times| {
                let sum: Duration = times.iter().sum();
                sum.as_secs_f64() * 1000.0 / times.len() as f64
            })
            .unwrap_or(0.0);
            
        let avg_trading_time = operation_times
            .get("trading_decision")
            .map(|times| {
                let sum: Duration = times.iter().sum();
                sum.as_secs_f64() * 1000.0 / times.len() as f64
            })
            .unwrap_or(0.0);

        let uptime_hours = metrics.uptime_seconds as f64 / 3600.0;
        let predictions_per_second = if uptime_hours > 0.0 {
            metrics.predictions_generated as f64 / (metrics.uptime_seconds as f64).max(1.0)
        } else {
            0.0
        };
        
        let events_per_second = if uptime_hours > 0.0 {
            metrics.events_processed as f64 / (metrics.uptime_seconds as f64).max(1.0)
        } else {
            0.0
        };

        let error_rate = if metrics.events_processed > 0 {
            (metrics.error_count as f64 / metrics.events_processed as f64) * 100.0
        } else {
            0.0
        };

        // Calculate system health score (0-1)
        let health_factors = vec![
            (1.0 - (error_rate / 100.0).min(1.0)),  // Error rate factor
            (1.0 - (metrics.memory_usage_mb / 1000.0).min(1.0)), // Memory factor
            (1.0 - (metrics.cpu_usage_percent / 100.0).min(1.0)), // CPU factor
            if avg_prediction_time < 100.0 { 1.0 } else { 0.5 }, // Latency factor
        ];
        let system_health_score = health_factors.iter().sum::<f64>() / health_factors.len() as f64;

        PerformanceStats {
            avg_prediction_time_ms: avg_prediction_time,
            avg_trading_decision_time_ms: avg_trading_time,
            predictions_per_second,
            events_per_second,
            system_health_score,
            error_rate_percent: error_rate,
            memory_efficiency: (1.0 - (metrics.memory_usage_mb / 1000.0)).max(0.0),
        }
    }

    pub async fn update_model_performance(&self, model_name: String, performance: ModelPerformance) {
        let mut models = self.model_performance.write().await;
        models.insert(model_name, performance);
    }

    pub async fn get_model_performance(&self) -> HashMap<String, ModelPerformance> {
        self.model_performance.read().await.clone()
    }

    pub async fn record_hourly_snapshot(&self) {
        let current_metrics = self.get_current_metrics().await;
        let mut hourly = self.hourly_stats.write().await;
        
        hourly.push((Utc::now(), current_metrics));
        
        // Keep only last 24 hours of data
        if hourly.len() > 24 {
            hourly.remove(0);
        }
    }

    pub async fn get_hourly_stats(&self) -> Vec<(DateTime<Utc>, SystemMetrics)> {
        self.hourly_stats.read().await.clone()
    }

    // Simplified system resource monitoring
    async fn get_memory_usage_mb(&self) -> f64 {
        // In a real implementation, this would use system APIs
        // For now, return a simulated value
        50.0 + (rand::random::<f64>() * 20.0)
    }

    async fn get_cpu_usage_percent(&self) -> f64 {
        // In a real implementation, this would use system APIs
        // For now, return a simulated value
        5.0 + (rand::random::<f64>() * 15.0)
    }

    pub async fn start_periodic_collection(&self) {
        let metrics_collector = self.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60)); // Every minute
            
            loop {
                interval.tick().await;
                
                // Record hourly snapshot every hour
                if Utc::now().minute() == 0 {
                    metrics_collector.record_hourly_snapshot().await;
                }
                
                // Log current performance stats
                let stats = metrics_collector.get_performance_stats().await;
                info!(
                    "📊 Performance: {:.1} pred/s, {:.1} events/s, {:.1}ms avg latency, {:.1}% health",
                    stats.predictions_per_second,
                    stats.events_per_second,
                    stats.avg_prediction_time_ms,
                    stats.system_health_score * 100.0
                );
                
                // Warn if performance is degrading
                if stats.system_health_score < 0.7 {
                    warn!(
                        "⚠️ System health degraded: {:.1}% (Error rate: {:.2}%, Memory: {:.1}MB)",
                        stats.system_health_score * 100.0,
                        stats.error_rate_percent,
                        metrics_collector.get_current_metrics().await.memory_usage_mb
                    );
                }
            }
        });
    }

    pub async fn log_performance_summary(&self) {
        let stats = self.get_performance_stats().await;
        let metrics = self.get_current_metrics().await;
        
        info!("📈 Performance Summary:");
        info!("   Events processed: {}", metrics.events_processed);
        info!("   Predictions generated: {}", metrics.predictions_generated);
        info!("   Trades executed: {}", metrics.trades_executed);
        info!("   API requests: {}", metrics.api_requests);
        info!("   Average prediction time: {:.2}ms", stats.avg_prediction_time_ms);
        info!("   Average trading time: {:.2}ms", stats.avg_trading_decision_time_ms);
        info!("   Events per second: {:.2}", stats.events_per_second);
        info!("   Predictions per second: {:.2}", stats.predictions_per_second);
        info!("   System health: {:.1}%", stats.system_health_score * 100.0);
        info!("   Error rate: {:.2}%", stats.error_rate_percent);
        info!("   Uptime: {} seconds", metrics.uptime_seconds);
    }
}

impl Clone for MetricsCollector {
    fn clone(&self) -> Self {
        Self {
            start_time: self.start_time,
            metrics: self.metrics.clone(),
            operation_times: self.operation_times.clone(),
            model_performance: self.model_performance.clone(),
            hourly_stats: self.hourly_stats.clone(),
        }
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

// Macro for easy latency tracking
#[macro_export]
macro_rules! track_latency {
    ($metrics:expr, $operation:expr, $code:block) => {{
        let tracker = $metrics.start_latency_tracking($operation.to_string());
        let result = $code;
        tracker.finish(&$metrics);
        result
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_metrics_collection() {
        let collector = MetricsCollector::new();
        
        // Test basic counters
        collector.increment_events_processed().await;
        collector.increment_predictions_generated().await;
        collector.increment_trades_executed().await;
        
        let metrics = collector.get_current_metrics().await;
        assert_eq!(metrics.events_processed, 1);
        assert_eq!(metrics.predictions_generated, 1);
        assert_eq!(metrics.trades_executed, 1);
    }

    #[tokio::test]
    async fn test_latency_tracking() {
        let collector = MetricsCollector::new();
        
        {
            let tracker = collector.start_latency_tracking("test_operation".to_string());
            sleep(Duration::from_millis(10)).await;
            tracker.finish(&collector);
        }
        
        // Give async task time to complete
        sleep(Duration::from_millis(100)).await;
        
        let operation_times = collector.operation_times.read().await;
        assert!(operation_times.contains_key("test_operation"));
    }

    #[tokio::test]
    async fn test_performance_stats() {
        let collector = MetricsCollector::new();
        
        // Add some test data
        for _ in 0..10 {
            collector.increment_events_processed().await;
            collector.increment_predictions_generated().await;
        }
        
        let stats = collector.get_performance_stats().await;
        assert!(stats.system_health_score > 0.0);
        assert!(stats.system_health_score <= 1.0);
    }
}