//! Performance metrics collection and analysis

use super::{PerformanceError, PerformanceResult, PerformanceConfig};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
// use uuid::Uuid;

/// Performance metrics collector
pub struct MetricsCollector {
    registry: MetricsRegistry,
    active_timers: Arc<RwLock<HashMap<String, Instant>>>,
    counters: Arc<RwLock<HashMap<String, AtomicU64>>>,
    gauges: Arc<RwLock<HashMap<String, Arc<AtomicU64>>>>,
    histograms: Arc<RwLock<HashMap<String, Histogram>>>,
    config: PerformanceConfig,
}

/// Metrics registry for organizing different metric types
pub struct MetricsRegistry {
    metrics: RwLock<HashMap<String, PerformanceMetrics>>,
    metric_definitions: HashMap<String, MetricDefinition>,
}

/// Individual performance metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub name: String,
    pub metric_type: MetricType,
    pub value: f64,
    pub unit: String,
    pub tags: HashMap<String, String>,
    pub timestamp: DateTime<Utc>,
    pub samples: Vec<MetricSample>,
}

/// Types of metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
    Timer,
    Rate,
}

/// Metric definition
#[derive(Debug, Clone)]
pub struct MetricDefinition {
    pub name: String,
    pub description: String,
    pub metric_type: MetricType,
    pub unit: String,
    pub labels: Vec<String>,
}

/// Metric sample point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricSample {
    pub value: f64,
    pub timestamp: DateTime<Utc>,
    pub tags: HashMap<String, String>,
}

/// Performance counter for tracking events
pub struct PerformanceCounter {
    name: String,
    value: Arc<AtomicU64>,
    tags: HashMap<String, String>,
}

/// Latency tracker for measuring response times
#[allow(dead_code)]
pub struct LatencyTracker {
    name: String,
    samples: Arc<RwLock<Vec<Duration>>>,
    percentiles: Arc<RwLock<Percentiles>>,
}

/// Throughput tracker for measuring rates
pub struct ThroughputTracker {
    name: String,
    window_size: Duration,
    samples: Arc<RwLock<Vec<(DateTime<Utc>, u64)>>>,
    current_rate: Arc<AtomicU64>,
}

/// Percentile calculations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Percentiles {
    pub p50: f64,
    pub p90: f64,
    pub p95: f64,
    pub p99: f64,
    pub max: f64,
    pub min: f64,
    pub mean: f64,
    pub count: u64,
}

/// Histogram for distribution analysis
#[derive(Debug, Clone)]
pub struct Histogram {
    name: String,
    buckets: Vec<HistogramBucket>,
    total_count: u64,
    total_sum: f64,
}

/// Histogram bucket
#[derive(Debug, Clone)]
pub struct HistogramBucket {
    pub upper_bound: f64,
    pub count: u64,
}

/// Metrics statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsStatistics {
    pub total_metrics: usize,
    pub counters_count: usize,
    pub gauges_count: usize,
    pub histograms_count: usize,
    pub timers_count: usize,
    pub collection_errors: u64,
    pub last_collection: Option<DateTime<Utc>>,
    pub collection_duration_ms: u64,
}

impl MetricsCollector {
    pub fn new(config: &PerformanceConfig) -> PerformanceResult<Self> {
        Ok(Self {
            registry: MetricsRegistry::new(),
            active_timers: Arc::new(RwLock::new(HashMap::new())),
            counters: Arc::new(RwLock::new(HashMap::new())),
            gauges: Arc::new(RwLock::new(HashMap::new())),
            histograms: Arc::new(RwLock::new(HashMap::new())),
            config: config.clone(),
        })
    }

    /// Initialize metrics collection
    pub async fn initialize(&mut self) -> PerformanceResult<()> {
        // Register default metrics
        self.register_default_metrics().await?;
        
        // Start collection loop if enabled
        if self.config.monitoring_enabled {
            self.start_collection_loop().await?;
        }

        Ok(())
    }

    /// Increment a counter metric
    pub async fn increment_counter(&self, name: &str, value: u64, tags: HashMap<String, String>) -> PerformanceResult<()> {
        let counters = self.counters.read().await;
        
        if let Some(counter) = counters.get(name) {
            counter.fetch_add(value, Ordering::Relaxed);
        } else {
            drop(counters);
            
            // Create new counter
            let mut counters = self.counters.write().await;
            let new_counter = AtomicU64::new(value);
            counters.insert(name.to_string(), new_counter);
        }

        // Record metric
        self.registry.record_metric(PerformanceMetrics {
            name: name.to_string(),
            metric_type: MetricType::Counter,
            value: value as f64,
            unit: "count".to_string(),
            tags,
            timestamp: Utc::now(),
            samples: vec![],
        }).await;

        Ok(())
    }

    /// Set a gauge value
    pub async fn set_gauge(&self, name: &str, value: u64, tags: HashMap<String, String>) -> PerformanceResult<()> {
        let gauges = self.gauges.read().await;
        
        if let Some(gauge) = gauges.get(name) {
            gauge.store(value, Ordering::Relaxed);
        } else {
            drop(gauges);
            
            // Create new gauge
            let mut gauges = self.gauges.write().await;
            let new_gauge = Arc::new(AtomicU64::new(value));
            gauges.insert(name.to_string(), new_gauge);
        }

        // Record metric
        self.registry.record_metric(PerformanceMetrics {
            name: name.to_string(),
            metric_type: MetricType::Gauge,
            value: value as f64,
            unit: "value".to_string(),
            tags,
            timestamp: Utc::now(),
            samples: vec![],
        }).await;

        Ok(())
    }

    /// Start timing an operation
    pub async fn start_timer(&self, name: &str) -> PerformanceResult<()> {
        let mut timers = self.active_timers.write().await;
        timers.insert(name.to_string(), Instant::now());
        Ok(())
    }

    /// End timing an operation and record the duration
    pub async fn end_timer(&self, name: &str, tags: HashMap<String, String>) -> PerformanceResult<Duration> {
        let mut timers = self.active_timers.write().await;
        
        if let Some(start_time) = timers.remove(name) {
            let duration = start_time.elapsed();
            
            // Record timing metric
            self.registry.record_metric(PerformanceMetrics {
                name: name.to_string(),
                metric_type: MetricType::Timer,
                value: duration.as_millis() as f64,
                unit: "milliseconds".to_string(),
                tags,
                timestamp: Utc::now(),
                samples: vec![],
            }).await;

            Ok(duration)
        } else {
            Err(PerformanceError::MetricsError(
                format!("Timer '{}' was not started", name)
            ))
        }
    }

    /// Record histogram value
    pub async fn record_histogram(&self, name: &str, value: f64, tags: HashMap<String, String>) -> PerformanceResult<()> {
        let mut histograms = self.histograms.write().await;
        
        let histogram = histograms.entry(name.to_string())
            .or_insert_with(|| Histogram::new(name));
        
        histogram.record(value);

        // Record metric
        self.registry.record_metric(PerformanceMetrics {
            name: name.to_string(),
            metric_type: MetricType::Histogram,
            value,
            unit: "value".to_string(),
            tags,
            timestamp: Utc::now(),
            samples: vec![],
        }).await;

        Ok(())
    }

    /// Get current metric values
    pub async fn get_metrics(&self) -> HashMap<String, PerformanceMetrics> {
        self.registry.get_all_metrics().await
    }

    /// Get counter value
    pub async fn get_counter_value(&self, name: &str) -> Option<u64> {
        let counters = self.counters.read().await;
        counters.get(name).map(|c| c.load(Ordering::Relaxed))
    }

    /// Get gauge value
    pub async fn get_gauge_value(&self, name: &str) -> Option<u64> {
        let gauges = self.gauges.read().await;
        gauges.get(name).map(|g| g.load(Ordering::Relaxed))
    }

    /// Get histogram percentiles
    pub async fn get_histogram_percentiles(&self, name: &str) -> Option<Percentiles> {
        let histograms = self.histograms.read().await;
        histograms.get(name).map(|h| h.calculate_percentiles())
    }

    /// Create performance counter
    pub fn create_counter(&self, name: &str, tags: HashMap<String, String>) -> PerformanceCounter {
        PerformanceCounter::new(name, tags)
    }

    /// Create latency tracker
    pub fn create_latency_tracker(&self, name: &str) -> LatencyTracker {
        LatencyTracker::new(name)
    }

    /// Create throughput tracker
    pub fn create_throughput_tracker(&self, name: &str, window_size: Duration) -> ThroughputTracker {
        ThroughputTracker::new(name, window_size)
    }

    /// Get collection statistics
    pub async fn get_statistics(&self) -> PerformanceResult<MetricsStatistics> {
        let metrics = self.get_metrics().await;
        
        let mut counters_count = 0;
        let mut gauges_count = 0;
        let mut histograms_count = 0;
        let mut timers_count = 0;

        for metric in metrics.values() {
            match metric.metric_type {
                MetricType::Counter => counters_count += 1,
                MetricType::Gauge => gauges_count += 1,
                MetricType::Histogram => histograms_count += 1,
                MetricType::Timer => timers_count += 1,
                _ => {}
            }
        }

        Ok(MetricsStatistics {
            total_metrics: metrics.len(),
            counters_count,
            gauges_count,
            histograms_count,
            timers_count,
            collection_errors: 0, // Would track actual errors
            last_collection: Some(Utc::now()),
            collection_duration_ms: 0, // Would measure actual collection time
        })
    }

    // Private helper methods

    async fn register_default_metrics(&mut self) -> PerformanceResult<()> {
        let default_definitions = vec![
            MetricDefinition {
                name: "email_sync_duration".to_string(),
                description: "Time taken to sync emails".to_string(),
                metric_type: MetricType::Timer,
                unit: "milliseconds".to_string(),
                labels: vec!["account".to_string(), "folder".to_string()],
            },
            MetricDefinition {
                name: "email_count".to_string(),
                description: "Number of emails processed".to_string(),
                metric_type: MetricType::Counter,
                unit: "count".to_string(),
                labels: vec!["status".to_string()],
            },
            MetricDefinition {
                name: "memory_usage".to_string(),
                description: "Current memory usage".to_string(),
                metric_type: MetricType::Gauge,
                unit: "bytes".to_string(),
                labels: vec!["type".to_string()],
            },
            MetricDefinition {
                name: "response_time".to_string(),
                description: "API response time distribution".to_string(),
                metric_type: MetricType::Histogram,
                unit: "milliseconds".to_string(),
                labels: vec!["endpoint".to_string(), "method".to_string()],
            },
        ];

        for definition in default_definitions {
            self.registry.register_metric_definition(definition);
        }

        Ok(())
    }

    async fn start_collection_loop(&self) -> PerformanceResult<()> {
        let interval = Duration::from_secs(self.config.optimization_intervals.metrics_collection_seconds);
        
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            
            loop {
                interval_timer.tick().await;
                
                // Collect system metrics
                // This would include CPU, memory, disk, network metrics
                // Implementation would use system APIs
            }
        });

        Ok(())
    }
}

impl MetricsRegistry {
    fn new() -> Self {
        Self {
            metrics: RwLock::new(HashMap::new()),
            metric_definitions: HashMap::new(),
        }
    }

    async fn record_metric(&self, metric: PerformanceMetrics) {
        let mut metrics = self.metrics.write().await;
        
        // Update existing metric or create new one
        let entry = metrics.entry(metric.name.clone()).or_insert_with(|| metric.clone());
        
        // Add sample to existing metric
        let sample = MetricSample {
            value: metric.value,
            timestamp: metric.timestamp,
            tags: metric.tags,
        };
        
        entry.samples.push(sample);
        
        // Keep only recent samples (last 1000)
        if entry.samples.len() > 1000 {
            entry.samples.drain(0..100);
        }
    }

    async fn get_all_metrics(&self) -> HashMap<String, PerformanceMetrics> {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }

    fn register_metric_definition(&mut self, definition: MetricDefinition) {
        self.metric_definitions.insert(definition.name.clone(), definition);
    }
}

impl PerformanceCounter {
    fn new(name: &str, tags: HashMap<String, String>) -> Self {
        Self {
            name: name.to_string(),
            value: Arc::new(AtomicU64::new(0)),
            tags,
        }
    }

    pub fn increment(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_by(&self, value: u64) {
        self.value.fetch_add(value, Ordering::Relaxed);
    }

    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }
}

impl LatencyTracker {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            samples: Arc::new(RwLock::new(Vec::new())),
            percentiles: Arc::new(RwLock::new(Percentiles::default())),
        }
    }

    pub async fn record(&self, duration: Duration) {
        let mut samples = self.samples.write().await;
        samples.push(duration);
        
        // Keep only recent samples
        if samples.len() > 10000 {
            samples.drain(0..1000);
        }
        
        // Recalculate percentiles
        let mut percentiles = self.percentiles.write().await;
        *percentiles = Self::calculate_percentiles(&samples);
    }

    pub async fn get_percentiles(&self) -> Percentiles {
        let percentiles = self.percentiles.read().await;
        percentiles.clone()
    }

    fn calculate_percentiles(samples: &[Duration]) -> Percentiles {
        if samples.is_empty() {
            return Percentiles::default();
        }

        let mut sorted_samples: Vec<f64> = samples.iter()
            .map(|d| d.as_millis() as f64)
            .collect();
        sorted_samples.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let count = sorted_samples.len();
        let sum: f64 = sorted_samples.iter().sum();
        let mean = sum / count as f64;

        Percentiles {
            p50: Self::percentile(&sorted_samples, 50.0),
            p90: Self::percentile(&sorted_samples, 90.0),
            p95: Self::percentile(&sorted_samples, 95.0),
            p99: Self::percentile(&sorted_samples, 99.0),
            max: sorted_samples[count - 1],
            min: sorted_samples[0],
            mean,
            count: count as u64,
        }
    }

    fn percentile(sorted_values: &[f64], percentile: f64) -> f64 {
        if sorted_values.is_empty() {
            return 0.0;
        }

        let index = (percentile / 100.0) * (sorted_values.len() as f64 - 1.0);
        let lower = index.floor() as usize;
        let upper = index.ceil() as usize;

        if lower == upper {
            sorted_values[lower]
        } else {
            let weight = index - lower as f64;
            sorted_values[lower] * (1.0 - weight) + sorted_values[upper] * weight
        }
    }
}

impl ThroughputTracker {
    fn new(name: &str, window_size: Duration) -> Self {
        Self {
            name: name.to_string(),
            window_size,
            samples: Arc::new(RwLock::new(Vec::new())),
            current_rate: Arc::new(AtomicU64::new(0)),
        }
    }

    pub async fn record(&self, count: u64) {
        let now = Utc::now();
        let cutoff = now - chrono::Duration::from_std(self.window_size).unwrap();

        let mut samples = self.samples.write().await;
        samples.push((now, count));

        // Remove old samples
        samples.retain(|(timestamp, _)| *timestamp > cutoff);

        // Calculate current rate
        let total_count: u64 = samples.iter().map(|(_, count)| count).sum();
        let rate = total_count * 1000 / self.window_size.as_millis() as u64; // Per second
        
        self.current_rate.store(rate, Ordering::Relaxed);
    }

    pub fn get_rate(&self) -> u64 {
        self.current_rate.load(Ordering::Relaxed)
    }
}

impl Histogram {
    fn new(name: &str) -> Self {
        // Create default buckets (exponential)
        let buckets = vec![
            HistogramBucket { upper_bound: 1.0, count: 0 },
            HistogramBucket { upper_bound: 2.0, count: 0 },
            HistogramBucket { upper_bound: 5.0, count: 0 },
            HistogramBucket { upper_bound: 10.0, count: 0 },
            HistogramBucket { upper_bound: 25.0, count: 0 },
            HistogramBucket { upper_bound: 50.0, count: 0 },
            HistogramBucket { upper_bound: 100.0, count: 0 },
            HistogramBucket { upper_bound: 250.0, count: 0 },
            HistogramBucket { upper_bound: 500.0, count: 0 },
            HistogramBucket { upper_bound: 1000.0, count: 0 },
            HistogramBucket { upper_bound: f64::INFINITY, count: 0 },
        ];

        Self {
            name: name.to_string(),
            buckets,
            total_count: 0,
            total_sum: 0.0,
        }
    }

    fn record(&mut self, value: f64) {
        self.total_count += 1;
        self.total_sum += value;

        for bucket in &mut self.buckets {
            if value <= bucket.upper_bound {
                bucket.count += 1;
            }
        }
    }

    fn calculate_percentiles(&self) -> Percentiles {
        // Simplified percentile calculation from histogram buckets
        // In practice, would use more sophisticated estimation
        
        if self.total_count == 0 {
            return Percentiles::default();
        }

        let mean = self.total_sum / self.total_count as f64;

        Percentiles {
            p50: self.estimate_percentile(50.0),
            p90: self.estimate_percentile(90.0),
            p95: self.estimate_percentile(95.0),
            p99: self.estimate_percentile(99.0),
            max: self.buckets.iter()
                .filter(|b| b.count > 0)
                .map(|b| b.upper_bound)
                .fold(0.0, f64::max),
            min: 0.0, // Would track actual minimum
            mean,
            count: self.total_count,
        }
    }

    fn estimate_percentile(&self, percentile: f64) -> f64 {
        let target_count = (percentile / 100.0) * self.total_count as f64;
        let mut cumulative_count = 0;

        for bucket in &self.buckets {
            cumulative_count += bucket.count;
            if cumulative_count as f64 >= target_count {
                return bucket.upper_bound;
            }
        }

        0.0 // Fallback
    }
}

impl Default for Percentiles {
    fn default() -> Self {
        Self {
            p50: 0.0,
            p90: 0.0,
            p95: 0.0,
            p99: 0.0,
            max: 0.0,
            min: 0.0,
            mean: 0.0,
            count: 0,
        }
    }
}