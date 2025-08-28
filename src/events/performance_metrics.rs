//! Performance metrics collection for event processing
//!
//! This module provides comprehensive performance monitoring for the event bus system,
//! including processing times, throughput, queue depths, and handler performance.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::info;

/// Performance metrics for event processing
#[derive(Debug, Default)]
pub struct EventMetrics {
    /// Total number of events processed
    pub total_events: AtomicU64,
    /// Total number of events that failed processing
    pub failed_events: AtomicU64,
    /// Total processing time across all events
    pub total_processing_time: AtomicU64,
    /// Current queue depth
    pub current_queue_depth: AtomicUsize,
    /// Maximum queue depth observed
    pub max_queue_depth: AtomicUsize,
    /// Number of events currently being processed
    pub events_in_processing: AtomicUsize,
    /// Number of active handlers
    pub active_handlers: AtomicUsize,
}

/// Performance metrics for individual event handlers
#[derive(Debug, Clone)]
pub struct HandlerMetrics {
    /// Handler name/type identifier
    pub handler_name: String,
    /// Total events processed by this handler
    pub events_processed: u64,
    /// Total events that failed in this handler
    pub events_failed: u64,
    /// Average processing time for this handler
    pub avg_processing_time: Duration,
    /// Minimum processing time observed
    pub min_processing_time: Duration,
    /// Maximum processing time observed
    pub max_processing_time: Duration,
    /// Last processing time
    pub last_processing_time: Duration,
    /// Timestamp of last event processed
    pub last_processed: Instant,
}

impl Default for HandlerMetrics {
    fn default() -> Self {
        Self {
            handler_name: String::new(),
            events_processed: 0,
            events_failed: 0,
            avg_processing_time: Duration::from_millis(0),
            min_processing_time: Duration::from_millis(u64::MAX),
            max_processing_time: Duration::from_millis(0),
            last_processing_time: Duration::from_millis(0),
            last_processed: Instant::now(),
        }
    }
}

/// Throughput statistics over time windows
#[derive(Debug, Clone)]
pub struct ThroughputMetrics {
    /// Events per second in the last minute
    pub events_per_second_1m: f64,
    /// Events per second in the last 5 minutes
    pub events_per_second_5m: f64,
    /// Events per second in the last 15 minutes
    pub events_per_second_15m: f64,
    /// Peak events per second observed
    pub peak_events_per_second: f64,
    /// Timestamp of peak throughput
    pub peak_timestamp: Option<Instant>,
}

impl Default for ThroughputMetrics {
    fn default() -> Self {
        Self {
            events_per_second_1m: 0.0,
            events_per_second_5m: 0.0,
            events_per_second_15m: 0.0,
            peak_events_per_second: 0.0,
            peak_timestamp: None,
        }
    }
}

/// Memory usage statistics and monitoring
#[derive(Debug, Clone)]
pub struct MemoryMetrics {
    /// Current memory usage in bytes
    pub current_memory_usage: u64,
    /// Peak memory usage observed
    pub peak_memory_usage: u64,
    /// Memory usage over time
    pub memory_usage_history: VecDeque<MemoryDataPoint>,
    /// Current size of event queue
    pub event_queue_size: usize,
    /// Number of cached handlers
    pub handler_cache_size: usize,
    /// Size of metrics history buffer
    pub metrics_history_size: usize,
    /// Total allocated memory since start
    pub allocated_memory: u64,
    /// Total deallocated memory since start
    pub deallocated_memory: u64,
    /// Memory efficiency ratio (0.0 to 1.0)
    pub memory_efficiency_ratio: f64,
}

impl Default for MemoryMetrics {
    fn default() -> Self {
        Self {
            current_memory_usage: 0,
            peak_memory_usage: 0,
            memory_usage_history: VecDeque::new(),
            event_queue_size: 0,
            handler_cache_size: 0,
            metrics_history_size: 0,
            allocated_memory: 0,
            deallocated_memory: 0,
            memory_efficiency_ratio: 1.0,
        }
    }
}

/// Memory data point for time-series analysis
#[derive(Debug, Clone)]
pub struct MemoryDataPoint {
    pub timestamp: Instant,
    pub memory_usage_bytes: u64,
    pub event_queue_size: usize,
    pub handler_count: usize,
    pub garbage_collection_pressure: Option<f64>,
}

/// Time-series data point for metrics history
#[derive(Debug, Clone)]
pub struct MetricsDataPoint {
    pub timestamp: Instant,
    pub events_count: u64,
    pub processing_time_ms: u64,
    pub queue_depth: usize,
    pub failed_events: u64,
    pub memory_usage_bytes: u64,
}

/// Comprehensive performance monitoring system
pub struct PerformanceMonitor {
    /// Overall event metrics
    pub metrics: EventMetrics,
    /// Per-handler performance metrics
    handler_metrics: Arc<RwLock<HashMap<String, HandlerMetrics>>>,
    /// Throughput metrics
    throughput_metrics: Arc<RwLock<ThroughputMetrics>>,
    /// Memory usage metrics
    memory_metrics: Arc<RwLock<MemoryMetrics>>,
    /// Time-series history of metrics (circular buffer)
    metrics_history: Arc<RwLock<VecDeque<MetricsDataPoint>>>,
    /// Maximum history length to keep in memory
    max_history_length: usize,
    /// Collection interval for throughput calculations
    collection_interval: Duration,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            metrics: EventMetrics::default(),
            handler_metrics: Arc::new(RwLock::new(HashMap::new())),
            throughput_metrics: Arc::new(RwLock::new(ThroughputMetrics::default())),
            memory_metrics: Arc::new(RwLock::new(MemoryMetrics::default())),
            metrics_history: Arc::new(RwLock::new(VecDeque::new())),
            max_history_length: 1000, // Keep last 1000 data points
            collection_interval: Duration::from_secs(10),
        }
    }

    pub fn with_config(max_history_length: usize, collection_interval: Duration) -> Self {
        Self {
            metrics: EventMetrics::default(),
            handler_metrics: Arc::new(RwLock::new(HashMap::new())),
            throughput_metrics: Arc::new(RwLock::new(ThroughputMetrics::default())),
            memory_metrics: Arc::new(RwLock::new(MemoryMetrics::default())),
            metrics_history: Arc::new(RwLock::new(VecDeque::new())),
            max_history_length,
            collection_interval,
        }
    }

    /// Start background metrics collection
    pub async fn start_monitoring(&self) {
        let _handler_metrics = Arc::clone(&self.handler_metrics);
        let throughput_metrics = Arc::clone(&self.throughput_metrics);
        let metrics_history = Arc::clone(&self.metrics_history);
        let max_history_length = self.max_history_length;
        let collection_interval = self.collection_interval;

        // Clone metrics for the background task
        let metrics = Arc::new(AtomicU64::new(0));
        let failed_metrics = Arc::new(AtomicU64::new(0));
        let processing_time_metrics = Arc::new(AtomicU64::new(0));
        let queue_depth_metrics = Arc::new(AtomicUsize::new(0));

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(collection_interval);
            let mut last_total_events = 0u64;
            let mut last_measurement = Instant::now();

            loop {
                interval.tick().await;

                // Capture current metrics snapshot
                let current_total = metrics.load(Ordering::Relaxed);
                let current_failed = failed_metrics.load(Ordering::Relaxed);
                let current_processing_time = processing_time_metrics.load(Ordering::Relaxed);
                let current_queue_depth = queue_depth_metrics.load(Ordering::Relaxed);

                let now = Instant::now();
                let time_elapsed = now.duration_since(last_measurement);

                // Calculate throughput
                let events_processed_this_period = current_total.saturating_sub(last_total_events);
                let current_throughput = if time_elapsed.as_secs_f64() > 0.0 {
                    events_processed_this_period as f64 / time_elapsed.as_secs_f64()
                } else {
                    0.0
                };

                // Update throughput metrics
                {
                    let mut throughput = throughput_metrics.write().await;
                    
                    // Update throughput windows (simplified calculation)
                    throughput.events_per_second_1m = Self::update_exponential_average(
                        throughput.events_per_second_1m,
                        current_throughput,
                        0.1, // Weight for 1-minute window
                    );
                    
                    throughput.events_per_second_5m = Self::update_exponential_average(
                        throughput.events_per_second_5m,
                        current_throughput,
                        0.02, // Weight for 5-minute window
                    );
                    
                    throughput.events_per_second_15m = Self::update_exponential_average(
                        throughput.events_per_second_15m,
                        current_throughput,
                        0.007, // Weight for 15-minute window
                    );

                    // Update peak throughput
                    if current_throughput > throughput.peak_events_per_second {
                        throughput.peak_events_per_second = current_throughput;
                        throughput.peak_timestamp = Some(now);
                    }
                }

                // Store data point in history
                {
                    let mut history = metrics_history.write().await;
                    history.push_back(MetricsDataPoint {
                        timestamp: now,
                        events_count: current_total,
                        processing_time_ms: current_processing_time,
                        queue_depth: current_queue_depth,
                        failed_events: current_failed,
                        memory_usage_bytes: 0, // Will be updated by memory monitoring
                    });

                    // Trim history to max length
                    while history.len() > max_history_length {
                        history.pop_front();
                    }
                }

                // Update for next iteration
                last_total_events = current_total;
                last_measurement = now;

                // Log performance summary every minute
                if collection_interval >= Duration::from_secs(60) || 
                   (now.elapsed().as_secs() % 60 == 0 && collection_interval < Duration::from_secs(60)) {
                    
                    let throughput = throughput_metrics.read().await;
                    info!(
                        "Event performance: {:.2} events/sec (1m), queue depth: {}, total events: {}",
                        throughput.events_per_second_1m,
                        current_queue_depth,
                        current_total
                    );
                }
            }
        });
    }

    /// Record an event being processed
    pub async fn record_event_start(&self, _event_type: &str) {
        self.metrics.events_in_processing.fetch_add(1, Ordering::Relaxed);
        let new_queue_depth = self.metrics.current_queue_depth.load(Ordering::Relaxed);
        
        // Update max queue depth if necessary
        let current_max = self.metrics.max_queue_depth.load(Ordering::Relaxed);
        if new_queue_depth > current_max {
            self.metrics.max_queue_depth.store(new_queue_depth, Ordering::Relaxed);
        }
    }

    /// Record an event completion
    pub async fn record_event_completion(
        &self,
        handler_name: &str,
        processing_time: Duration,
        success: bool,
    ) {
        // Update global metrics
        self.metrics.total_events.fetch_add(1, Ordering::Relaxed);
        self.metrics.events_in_processing.fetch_sub(1, Ordering::Relaxed);
        self.metrics.total_processing_time.fetch_add(
            processing_time.as_millis() as u64,
            Ordering::Relaxed,
        );

        if !success {
            self.metrics.failed_events.fetch_add(1, Ordering::Relaxed);
        }

        // Update handler-specific metrics
        let mut handlers = self.handler_metrics.write().await;
        let handler_metrics = handlers.entry(handler_name.to_string()).or_insert_with(|| {
            HandlerMetrics {
                handler_name: handler_name.to_string(),
                ..Default::default()
            }
        });

        // Update handler metrics
        handler_metrics.events_processed += 1;
        if !success {
            handler_metrics.events_failed += 1;
        }
        
        handler_metrics.last_processing_time = processing_time;
        handler_metrics.last_processed = Instant::now();

        // Update min/max processing times
        if processing_time < handler_metrics.min_processing_time {
            handler_metrics.min_processing_time = processing_time;
        }
        if processing_time > handler_metrics.max_processing_time {
            handler_metrics.max_processing_time = processing_time;
        }

        // Update average processing time (exponential moving average)
        let alpha = 0.1; // Weight for new samples
        let old_avg_ms = handler_metrics.avg_processing_time.as_millis() as f64;
        let new_sample_ms = processing_time.as_millis() as f64;
        let new_avg_ms = old_avg_ms * (1.0 - alpha) + new_sample_ms * alpha;
        handler_metrics.avg_processing_time = Duration::from_millis(new_avg_ms as u64);
    }

    /// Update queue depth
    pub fn update_queue_depth(&self, new_depth: usize) {
        self.metrics.current_queue_depth.store(new_depth, Ordering::Relaxed);
    }

    /// Update active handler count
    pub fn update_active_handlers(&self, count: usize) {
        self.metrics.active_handlers.store(count, Ordering::Relaxed);
    }

    /// Get current performance summary
    pub async fn get_performance_summary(&self) -> PerformanceSummary {
        let total_events = self.metrics.total_events.load(Ordering::Relaxed);
        let failed_events = self.metrics.failed_events.load(Ordering::Relaxed);
        let total_processing_time = self.metrics.total_processing_time.load(Ordering::Relaxed);
        let current_queue_depth = self.metrics.current_queue_depth.load(Ordering::Relaxed);
        let max_queue_depth = self.metrics.max_queue_depth.load(Ordering::Relaxed);
        let events_in_processing = self.metrics.events_in_processing.load(Ordering::Relaxed);
        let active_handlers = self.metrics.active_handlers.load(Ordering::Relaxed);

        let avg_processing_time = if total_events > 0 {
            Duration::from_millis(total_processing_time / total_events)
        } else {
            Duration::from_millis(0)
        };

        let success_rate = if total_events > 0 {
            ((total_events - failed_events) as f64 / total_events as f64) * 100.0
        } else {
            100.0
        };

        let throughput = self.throughput_metrics.read().await.clone();
        let memory = self.memory_metrics.read().await.clone();
        let handlers = self.handler_metrics.read().await.clone();

        PerformanceSummary {
            total_events,
            failed_events,
            success_rate,
            avg_processing_time,
            current_queue_depth,
            max_queue_depth,
            events_in_processing,
            active_handlers,
            throughput,
            memory,
            handler_metrics: handlers,
        }
    }

    /// Get metrics history for a specific time range
    pub async fn get_metrics_history(&self, duration: Duration) -> Vec<MetricsDataPoint> {
        let history = self.metrics_history.read().await;
        let cutoff_time = Instant::now() - duration;

        history
            .iter()
            .filter(|point| point.timestamp >= cutoff_time)
            .cloned()
            .collect()
    }

    /// Get handler performance rankings
    pub async fn get_handler_rankings(&self) -> Vec<HandlerRanking> {
        let handlers = self.handler_metrics.read().await;
        let mut rankings: Vec<HandlerRanking> = handlers
            .values()
            .map(|metrics| HandlerRanking {
                handler_name: metrics.handler_name.clone(),
                events_processed: metrics.events_processed,
                avg_processing_time: metrics.avg_processing_time,
                success_rate: if metrics.events_processed > 0 {
                    ((metrics.events_processed - metrics.events_failed) as f64 
                     / metrics.events_processed as f64) * 100.0
                } else {
                    100.0
                },
                throughput: if metrics.last_processed.elapsed() < Duration::from_secs(60) {
                    metrics.events_processed as f64 / 60.0 // Events per second estimate
                } else {
                    0.0
                },
            })
            .collect();

        // Sort by throughput (events per second)
        rankings.sort_by(|a, b| b.throughput.partial_cmp(&a.throughput).unwrap_or(std::cmp::Ordering::Equal));

        rankings
    }

    /// Check if performance is degraded
    pub async fn check_performance_health(&self) -> PerformanceHealthStatus {
        let summary = self.get_performance_summary().await;
        let mut issues = Vec::new();

        // Check queue depth
        if summary.current_queue_depth > 1000 {
            issues.push("High queue depth detected".to_string());
        }

        // Check success rate
        if summary.success_rate < 95.0 && summary.total_events > 100 {
            issues.push(format!("Low success rate: {:.1}%", summary.success_rate));
        }

        // Check average processing time
        if summary.avg_processing_time > Duration::from_millis(1000) {
            issues.push("High average processing time".to_string());
        }

        // Check throughput trends
        if summary.throughput.events_per_second_1m < summary.throughput.events_per_second_5m * 0.5 {
            issues.push("Significant throughput decline detected".to_string());
        }

        let status = if issues.is_empty() {
            PerformanceHealth::Healthy
        } else if issues.len() <= 2 {
            PerformanceHealth::Degraded
        } else {
            PerformanceHealth::Critical
        };

        PerformanceHealthStatus { status, issues }
    }

    /// Record memory usage update
    pub async fn update_memory_usage(&self, memory_bytes: u64, queue_size: usize, handler_count: usize) {
        let mut memory_metrics = self.memory_metrics.write().await;
        
        memory_metrics.current_memory_usage = memory_bytes;
        memory_metrics.event_queue_size = queue_size;
        memory_metrics.handler_cache_size = handler_count;
        
        // Update peak memory usage
        if memory_bytes > memory_metrics.peak_memory_usage {
            memory_metrics.peak_memory_usage = memory_bytes;
        }
        
        // Add memory data point to history
        let data_point = MemoryDataPoint {
            timestamp: Instant::now(),
            memory_usage_bytes: memory_bytes,
            event_queue_size: queue_size,
            handler_count,
            garbage_collection_pressure: self.calculate_gc_pressure(&memory_metrics).await,
        };
        
        memory_metrics.memory_usage_history.push_back(data_point);
        
        // Keep history bounded
        while memory_metrics.memory_usage_history.len() > self.max_history_length {
            memory_metrics.memory_usage_history.pop_front();
        }
        
        // Update memory efficiency ratio
        if memory_metrics.allocated_memory > 0 {
            memory_metrics.memory_efficiency_ratio = 
                (memory_metrics.allocated_memory as f64 - memory_metrics.deallocated_memory as f64) 
                / memory_metrics.allocated_memory as f64;
        }
        
        // Update metrics history data point with memory info
        {
            let mut history = self.metrics_history.write().await;
            if let Some(last_point) = history.back_mut() {
                // Update the latest data point if it's recent (within 1 second)
                if last_point.timestamp.elapsed() < Duration::from_secs(1) {
                    last_point.memory_usage_bytes = memory_bytes;
                }
            }
        }
    }
    
    /// Record memory allocation/deallocation
    pub async fn record_memory_allocation(&self, allocated_bytes: u64, deallocated_bytes: u64) {
        let mut memory_metrics = self.memory_metrics.write().await;
        memory_metrics.allocated_memory += allocated_bytes;
        memory_metrics.deallocated_memory += deallocated_bytes;
    }
    
    /// Get current memory metrics
    pub async fn get_memory_metrics(&self) -> MemoryMetrics {
        self.memory_metrics.read().await.clone()
    }
    
    /// Get memory usage trend (bytes per second change)
    pub async fn get_memory_usage_trend(&self) -> f64 {
        let memory_metrics = self.memory_metrics.read().await;
        
        if memory_metrics.memory_usage_history.len() < 2 {
            return 0.0;
        }
        
        let recent = memory_metrics.memory_usage_history.back().unwrap();
        let older = &memory_metrics.memory_usage_history[memory_metrics.memory_usage_history.len() - 2];
        
        let memory_diff = recent.memory_usage_bytes as i64 - older.memory_usage_bytes as i64;
        let time_diff = recent.timestamp.duration_since(older.timestamp).as_secs_f64();
        
        if time_diff > 0.0 {
            memory_diff as f64 / time_diff
        } else {
            0.0
        }
    }
    
    /// Check for memory leaks or excessive usage
    pub async fn check_memory_health(&self) -> Vec<String> {
        let memory_metrics = self.memory_metrics.read().await;
        let mut issues = Vec::new();
        
        // Check for excessive memory usage (more than 1GB)
        if memory_metrics.current_memory_usage > 1_024_000_000 {
            issues.push(format!(
                "High memory usage: {:.2} MB", 
                memory_metrics.current_memory_usage as f64 / 1_048_576.0
            ));
        }
        
        // Check memory efficiency ratio
        if memory_metrics.memory_efficiency_ratio < 0.5 && memory_metrics.allocated_memory > 0 {
            issues.push(format!(
                "Poor memory efficiency: {:.1}%", 
                memory_metrics.memory_efficiency_ratio * 100.0
            ));
        }
        
        // Check for rapid memory growth trend
        drop(memory_metrics);
        let trend = self.get_memory_usage_trend().await;
        if trend > 10_000_000.0 { // More than 10MB/second growth
            issues.push("Rapid memory growth detected - possible memory leak".to_string());
        }
        
        // Check queue size growth
        let memory_metrics = self.memory_metrics.read().await;
        if memory_metrics.event_queue_size > 10000 {
            issues.push("Event queue growing excessively".to_string());
        }
        
        issues
    }
    
    /// Calculate garbage collection pressure estimate
    async fn calculate_gc_pressure(&self, memory_metrics: &MemoryMetrics) -> Option<f64> {
        if memory_metrics.memory_usage_history.len() < 10 {
            return None;
        }
        
        // Simple heuristic: calculate variance in recent memory usage
        let recent_points: Vec<_> = memory_metrics
            .memory_usage_history
            .iter()
            .rev()
            .take(10)
            .collect();
        
        let avg_memory: f64 = recent_points
            .iter()
            .map(|p| p.memory_usage_bytes as f64)
            .sum::<f64>() / recent_points.len() as f64;
        
        let variance: f64 = recent_points
            .iter()
            .map(|p| {
                let diff = p.memory_usage_bytes as f64 - avg_memory;
                diff * diff
            })
            .sum::<f64>() / recent_points.len() as f64;
        
        let std_dev = variance.sqrt();
        
        // Normalize by average memory usage to get pressure ratio
        if avg_memory > 0.0 {
            Some(std_dev / avg_memory)
        } else {
            None
        }
    }
    
    /// Helper function for exponential moving average
    fn update_exponential_average(current: f64, new_value: f64, alpha: f64) -> f64 {
        current * (1.0 - alpha) + new_value * alpha
    }
}

/// Performance summary containing all key metrics
#[derive(Debug, Clone)]
pub struct PerformanceSummary {
    pub total_events: u64,
    pub failed_events: u64,
    pub success_rate: f64,
    pub avg_processing_time: Duration,
    pub current_queue_depth: usize,
    pub max_queue_depth: usize,
    pub events_in_processing: usize,
    pub active_handlers: usize,
    pub throughput: ThroughputMetrics,
    pub memory: MemoryMetrics,
    pub handler_metrics: HashMap<String, HandlerMetrics>,
}

/// Handler performance ranking
#[derive(Debug, Clone)]
pub struct HandlerRanking {
    pub handler_name: String,
    pub events_processed: u64,
    pub avg_processing_time: Duration,
    pub success_rate: f64,
    pub throughput: f64, // Events per second
}

/// Performance health status
#[derive(Debug, Clone)]
pub struct PerformanceHealthStatus {
    pub status: PerformanceHealth,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PerformanceHealth {
    Healthy,
    Degraded,
    Critical,
}

impl std::fmt::Display for PerformanceHealth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PerformanceHealth::Healthy => write!(f, "Healthy"),
            PerformanceHealth::Degraded => write!(f, "Degraded"),
            PerformanceHealth::Critical => write!(f, "Critical"),
        }
    }
}

/// Trait for integrating performance monitoring with event handlers
pub trait MonitoredEventHandler {
    fn handler_name(&self) -> &str;
    
    fn handle_with_monitoring<T, F, Fut>(
        &self,
        monitor: &PerformanceMonitor,
        operation: F,
    ) -> impl std::future::Future<Output = Result<T, Box<dyn std::error::Error + Send + Sync>>> + Send
    where
        F: FnOnce() -> Fut + Send,
        Fut: std::future::Future<Output = Result<T, Box<dyn std::error::Error + Send + Sync>>> + Send,
        T: Send,
        Self: Sync,
    {
        async move {
        let start_time = Instant::now();
        monitor.record_event_start("test_event").await;
        
        let result = operation().await;
        let processing_time = start_time.elapsed();
        
        monitor.record_event_completion(
            self.handler_name(),
            processing_time,
            result.is_ok(),
        ).await;
        
        result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_performance_monitor_basic_functionality() {
        let monitor = PerformanceMonitor::new();
        
        // Record some events
        monitor.record_event_start("test_event").await;
        monitor.record_event_completion(
            "test_handler",
            Duration::from_millis(100),
            true,
        ).await;
        
        monitor.record_event_start("test_event").await;
        monitor.record_event_completion(
            "test_handler",
            Duration::from_millis(200),
            false,
        ).await;

        let summary = monitor.get_performance_summary().await;
        
        assert_eq!(summary.total_events, 2);
        assert_eq!(summary.failed_events, 1);
        assert_eq!(summary.success_rate, 50.0);
        assert!(summary.handler_metrics.contains_key("test_handler"));
        
        let handler_metrics = &summary.handler_metrics["test_handler"];
        assert_eq!(handler_metrics.events_processed, 2);
        assert_eq!(handler_metrics.events_failed, 1);
    }

    #[tokio::test]
    async fn test_queue_depth_tracking() {
        let monitor = PerformanceMonitor::new();
        
        monitor.update_queue_depth(100);
        monitor.update_queue_depth(200);
        monitor.update_queue_depth(50);
        
        let summary = monitor.get_performance_summary().await;
        assert_eq!(summary.current_queue_depth, 50);
        assert_eq!(summary.max_queue_depth, 200);
    }

    #[tokio::test]
    async fn test_performance_health_check() {
        let monitor = PerformanceMonitor::new();
        
        // Add some successful events
        for _ in 0..100 {
            monitor.record_event_completion(
                "test_handler",
                Duration::from_millis(10),
                true,
            ).await;
        }
        
        let health = monitor.check_performance_health().await;
        assert_eq!(health.status, PerformanceHealth::Healthy);
        assert!(health.issues.is_empty());
        
        // Add failed events to degrade performance
        for _ in 0..20 {
            monitor.record_event_completion(
                "test_handler",
                Duration::from_millis(10),
                false,
            ).await;
        }
        
        let health = monitor.check_performance_health().await;
        assert_eq!(health.status, PerformanceHealth::Degraded);
        assert!(!health.issues.is_empty());
    }

    #[tokio::test]
    async fn test_handler_rankings() {
        let monitor = PerformanceMonitor::new();
        
        // Add metrics for multiple handlers
        monitor.record_event_completion("fast_handler", Duration::from_millis(10), true).await;
        monitor.record_event_completion("fast_handler", Duration::from_millis(20), true).await;
        
        monitor.record_event_completion("slow_handler", Duration::from_millis(100), true).await;
        monitor.record_event_completion("slow_handler", Duration::from_millis(200), true).await;
        
        let rankings = monitor.get_handler_rankings().await;
        assert_eq!(rankings.len(), 2);
        
        // Rankings should be sorted by throughput (higher is better)
        assert!(rankings[0].throughput >= rankings[1].throughput);
    }
}