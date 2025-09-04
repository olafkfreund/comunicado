//! Performance optimization and monitoring system

pub mod metrics;
pub mod background_processor;
pub mod cache;
pub mod database;
pub mod memory;
pub mod network;
pub mod profiling;
pub mod optimization;
pub mod monitoring;

pub use metrics::{
    PerformanceMetrics, MetricsCollector, MetricsRegistry, MetricType, 
    PerformanceCounter, LatencyTracker, ThroughputTracker, MetricsStatistics
};
pub use cache::{
    PerformanceCache, CacheManager, CachePolicy, CacheStatistics, 
    CacheLevel, CacheEvictionStrategy
};
pub use database::{
    DatabaseOptimizer, QueryOptimizer, ConnectionPool, IndexAnalyzer,
    QueryStats, DatabaseMetrics, DatabaseStatistics
};
pub use memory::{
    MemoryManager, MemoryProfiler, AllocationTracker, GarbageCollector,
    MemoryStats, HeapAnalyzer, MemoryStatistics
};
pub use network::{
    NetworkOptimizer, ConnectionManager, RequestBatcher, CircuitBreaker,
    NetworkMetrics, BandwidthManager, NetworkStatistics
};
pub use profiling::{
    Profiler, ProfileResult, FlameGraph, HotspotAnalyzer,
    ProfilerConfig, ProfilingSession
};
pub use optimization::{
    OptimizationEngine, OptimizationRule, PerformanceTuner, 
    AutoOptimizer, OptimizationRecommendation
};
pub use background_processor::{
    BackgroundProcessor, BackgroundTask, TaskResult, TaskStatus, TaskPriority,
    BackgroundTaskType, TaskResultData, ProcessorSettings, CalendarSyncType, CalendarDbOperationType
};
pub use monitoring::{
    PerformanceMonitor, AlertManager, Threshold, Alert,
    MonitoringDashboard, HealthCheck
};

use serde::{Deserialize, Serialize};
// Unused imports commented out for now
// use std::collections::HashMap;
// use std::time::{Duration, Instant};
// use tokio::sync::RwLock;
// use uuid::Uuid;

/// Performance optimization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub monitoring_enabled: bool,
    pub profiling_enabled: bool,
    pub auto_optimization: bool,
    pub cache_enabled: bool,
    pub database_optimization: bool,
    pub network_optimization: bool,
    pub memory_optimization: bool,
    pub alert_thresholds: AlertThresholds,
    pub optimization_intervals: OptimizationIntervals,
}

/// Alert threshold configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub disk_usage_percent: f64,
    pub response_time_ms: u64,
    pub error_rate_percent: f64,
    pub queue_depth: usize,
}

/// Optimization interval configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationIntervals {
    pub metrics_collection_seconds: u64,
    pub cache_cleanup_seconds: u64,
    pub database_optimization_seconds: u64,
    pub memory_gc_seconds: u64,
    pub profiling_duration_seconds: u64,
}

/// Performance optimization errors
#[derive(Debug, thiserror::Error)]
pub enum PerformanceError {
    #[error("Metrics collection failed: {0}")]
    MetricsError(String),
    
    #[error("Cache operation failed: {0}")]
    CacheError(String),
    
    #[error("Database optimization failed: {0}")]
    DatabaseError(String),
    
    #[error("Memory management failed: {0}")]
    MemoryError(String),
    
    #[error("Network optimization failed: {0}")]
    NetworkError(String),
    
    #[error("Profiling failed: {0}")]
    ProfilingError(String),
    
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type PerformanceResult<T> = Result<T, PerformanceError>;

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            monitoring_enabled: true,
            profiling_enabled: true,
            auto_optimization: false,
            cache_enabled: true,
            database_optimization: true,
            network_optimization: true,
            memory_optimization: true,
            alert_thresholds: AlertThresholds::default(),
            optimization_intervals: OptimizationIntervals::default(),
        }
    }
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            cpu_usage_percent: 80.0,
            memory_usage_percent: 85.0,
            disk_usage_percent: 90.0,
            response_time_ms: 1000,
            error_rate_percent: 5.0,
            queue_depth: 1000,
        }
    }
}

impl Default for OptimizationIntervals {
    fn default() -> Self {
        Self {
            metrics_collection_seconds: 60,
            cache_cleanup_seconds: 300,
            database_optimization_seconds: 3600,
            memory_gc_seconds: 120,
            profiling_duration_seconds: 300,
        }
    }
}