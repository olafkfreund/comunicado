//! Performance monitoring and optimization system
//!
//! This module provides comprehensive performance monitoring, metrics collection,
//! and optimization suggestions for all system components.

use chrono::{DateTime, Utc, Duration as ChronoDuration};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Performance monitoring thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceThresholds {
    pub cpu_usage_warning: f64,
    pub cpu_usage_critical: f64,
    pub memory_usage_warning: f64,
    pub memory_usage_critical: f64,
    pub database_query_time_warning_ms: u64,
    pub database_query_time_critical_ms: u64,
    pub email_sync_time_warning_s: u64,
    pub email_sync_time_critical_s: u64,
    pub search_response_time_warning_ms: u64,
    pub search_response_time_critical_ms: u64,
}

impl Default for PerformanceThresholds {
    fn default() -> Self {
        Self {
            cpu_usage_warning: 70.0,
            cpu_usage_critical: 90.0,
            memory_usage_warning: 80.0,
            memory_usage_critical: 95.0,
            database_query_time_warning_ms: 1000,
            database_query_time_critical_ms: 5000,
            email_sync_time_warning_s: 300,
            email_sync_time_critical_s: 600,
            search_response_time_warning_ms: 500,
            search_response_time_critical_ms: 2000,
        }
    }
}

/// Performance alert levels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlertLevel {
    Info,
    Warning,
    Critical,
}

/// Performance alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAlert {
    pub id: String,
    pub level: AlertLevel,
    pub component: String,
    pub metric: String,
    pub current_value: f64,
    pub threshold: f64,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub suggestions: Vec<String>,
}

/// System resource usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemResourceUsage {
    pub cpu_usage_percent: f64,
    pub memory_usage_mb: f64,
    pub memory_total_mb: f64,
    pub memory_usage_percent: f64,
    pub disk_usage_mb: f64,
    pub disk_free_mb: f64,
    pub network_rx_mb: f64,
    pub network_tx_mb: f64,
    pub open_file_descriptors: u32,
    pub thread_count: u32,
    pub timestamp: DateTime<Utc>,
}

/// Component-specific performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentPerformance {
    pub component_name: String,
    pub average_response_time_ms: f64,
    pub max_response_time_ms: f64,
    pub min_response_time_ms: f64,
    pub total_operations: u64,
    pub failed_operations: u64,
    pub success_rate_percent: f64,
    pub operations_per_second: f64,
    pub last_operation_time: Option<DateTime<Utc>>,
    pub memory_usage_mb: f64,
    pub error_count: u64,
    pub warning_count: u64,
}

impl Default for ComponentPerformance {
    fn default() -> Self {
        Self {
            component_name: String::new(),
            average_response_time_ms: 0.0,
            max_response_time_ms: 0.0,
            min_response_time_ms: f64::MAX,
            total_operations: 0,
            failed_operations: 0,
            success_rate_percent: 100.0,
            operations_per_second: 0.0,
            last_operation_time: None,
            memory_usage_mb: 0.0,
            error_count: 0,
            warning_count: 0,
        }
    }
}

/// Comprehensive performance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    pub generated_at: DateTime<Utc>,
    pub report_period: ChronoDuration,
    pub system_resources: SystemResourceUsage,
    pub component_performances: HashMap<String, ComponentPerformance>,
    pub active_alerts: Vec<PerformanceAlert>,
    pub resolved_alerts: Vec<PerformanceAlert>,
    pub optimization_suggestions: Vec<OptimizationSuggestion>,
    pub trend_analysis: TrendAnalysis,
}

/// Optimization suggestions based on performance data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationSuggestion {
    pub category: String,
    pub priority: OptimizationPriority,
    pub title: String,
    pub description: String,
    pub estimated_impact: String,
    pub implementation_difficulty: DifficultyLevel,
    pub component: String,
}

/// Optimization priority levels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OptimizationPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// Implementation difficulty levels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DifficultyLevel {
    Easy,
    Medium,
    Hard,
    Expert,
}

/// Performance trend analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysis {
    pub cpu_trend: TrendDirection,
    pub memory_trend: TrendDirection,
    pub response_time_trend: TrendDirection,
    pub error_rate_trend: TrendDirection,
    pub throughput_trend: TrendDirection,
    pub analysis_period_hours: u32,
    pub confidence_level: f64,
}

/// Trend direction indicators
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TrendDirection {
    Improving,
    Stable,
    Degrading,
    Insufficient_Data,
}

/// Operation timing helper
pub struct OperationTimer {
    start_time: Instant,
    component: String,
    operation: String,
}

impl OperationTimer {
    pub fn new(component: String, operation: String) -> Self {
        Self {
            start_time: Instant::now(),
            component,
            operation,
        }
    }
    
    pub fn finish(self) -> (String, String, Duration) {
        (self.component, self.operation, self.start_time.elapsed())
    }
}

/// Performance monitoring service
pub struct PerformanceMonitor {
    thresholds: PerformanceThresholds,
    system_metrics: Arc<RwLock<VecDeque<SystemResourceUsage>>>,
    component_metrics: Arc<RwLock<HashMap<String, ComponentPerformance>>>,
    operation_timings: Arc<RwLock<VecDeque<OperationTiming>>>,
    active_alerts: Arc<RwLock<Vec<PerformanceAlert>>>,
    resolved_alerts: Arc<RwLock<Vec<PerformanceAlert>>>,
    max_history_size: usize,
}

/// Individual operation timing record
#[derive(Debug, Clone)]
struct OperationTiming {
    component: String,
    operation: String,
    duration: Duration,
    timestamp: DateTime<Utc>,
    success: bool,
}

impl PerformanceMonitor {
    /// Create a new performance monitor
    pub fn new(thresholds: PerformanceThresholds) -> Self {
        Self {
            thresholds,
            system_metrics: Arc::new(RwLock::new(VecDeque::new())),
            component_metrics: Arc::new(RwLock::new(HashMap::new())),
            operation_timings: Arc::new(RwLock::new(VecDeque::new())),
            active_alerts: Arc::new(RwLock::new(Vec::new())),
            resolved_alerts: Arc::new(RwLock::new(Vec::new())),
            max_history_size: 10000,
        }
    }
    
    /// Start monitoring with periodic collection
    pub async fn start_monitoring(&self) {
        let system_metrics = Arc::clone(&self.system_metrics);
        let component_metrics = Arc::clone(&self.component_metrics);
        let thresholds = self.thresholds.clone();
        let active_alerts = Arc::clone(&self.active_alerts);
        let max_history = self.max_history_size;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                
                // Collect system metrics
                let usage = Self::collect_system_metrics().await;
                
                // Store metrics with size limit
                {
                    let mut metrics = system_metrics.write().await;
                    metrics.push_back(usage.clone());
                    if metrics.len() > max_history {
                        metrics.pop_front();
                    }
                }
                
                // Check for alerts
                Self::check_system_alerts(&usage, &thresholds, &active_alerts).await;
                
                // Update component performance calculations
                Self::update_component_calculations(&component_metrics).await;
            }
        });
        
        info!("Performance monitoring started");
    }
    
    /// Record an operation timing
    pub async fn record_operation(
        &self,
        component: String,
        operation: String,
        duration: Duration,
        success: bool,
    ) {
        let timing = OperationTiming {
            component: component.clone(),
            operation,
            duration,
            timestamp: Utc::now(),
            success,
        };
        
        // Store timing
        {
            let mut timings = self.operation_timings.write().await;
            timings.push_back(timing);
            if timings.len() > self.max_history_size {
                timings.pop_front();
            }
        }
        
        // Update component metrics
        self.update_component_performance(&component, duration, success).await;
    }
    
    /// Create an operation timer
    pub fn start_timer(&self, component: String, operation: String) -> OperationTimer {
        OperationTimer::new(component, operation)
    }
    
    /// Finish timing an operation
    pub async fn finish_timer(&self, timer: OperationTimer, success: bool) {
        let (component, operation, duration) = timer.finish();
        self.record_operation(component, operation, duration, success).await;
    }
    
    /// Update component performance metrics
    async fn update_component_performance(
        &self,
        component: &str,
        duration: Duration,
        success: bool,
    ) {
        let mut components = self.component_metrics.write().await;
        let performance = components.entry(component.to_string())
            .or_insert_with(|| {
                let mut perf = ComponentPerformance::default();
                perf.component_name = component.to_string();
                perf
            });
        
        let duration_ms = duration.as_millis() as f64;
        
        // Update timing statistics
        performance.total_operations += 1;
        if !success {
            performance.failed_operations += 1;
        }
        
        performance.success_rate_percent = 
            ((performance.total_operations - performance.failed_operations) as f64 
             / performance.total_operations as f64) * 100.0;
        
        // Update response time statistics
        if performance.min_response_time_ms == f64::MAX {
            performance.min_response_time_ms = duration_ms;
        } else {
            performance.min_response_time_ms = performance.min_response_time_ms.min(duration_ms);
        }
        performance.max_response_time_ms = performance.max_response_time_ms.max(duration_ms);
        
        // Calculate running average
        let total_time = performance.average_response_time_ms * (performance.total_operations - 1) as f64;
        performance.average_response_time_ms = (total_time + duration_ms) / performance.total_operations as f64;
        
        performance.last_operation_time = Some(Utc::now());
        
        // Update error/warning counts
        if !success {
            performance.error_count += 1;
        }
        
        // Check for performance alerts
        self.check_component_alerts(component, performance).await;
    }
    
    /// Check for component-specific performance alerts
    async fn check_component_alerts(&self, component: &str, performance: &ComponentPerformance) {
        let mut alerts_to_add = Vec::new();
        
        // Check response time
        if performance.average_response_time_ms > self.thresholds.database_query_time_critical_ms as f64 {
            alerts_to_add.push(PerformanceAlert {
                id: format!("{}_response_time_critical", component),
                level: AlertLevel::Critical,
                component: component.to_string(),
                metric: "average_response_time".to_string(),
                current_value: performance.average_response_time_ms,
                threshold: self.thresholds.database_query_time_critical_ms as f64,
                message: format!("Critical response time in {}: {:.2}ms", component, performance.average_response_time_ms),
                timestamp: Utc::now(),
                suggestions: vec![
                    "Check database indexes".to_string(),
                    "Optimize query patterns".to_string(),
                    "Consider database connection pooling".to_string(),
                ],
            });
        } else if performance.average_response_time_ms > self.thresholds.database_query_time_warning_ms as f64 {
            alerts_to_add.push(PerformanceAlert {
                id: format!("{}_response_time_warning", component),
                level: AlertLevel::Warning,
                component: component.to_string(),
                metric: "average_response_time".to_string(),
                current_value: performance.average_response_time_ms,
                threshold: self.thresholds.database_query_time_warning_ms as f64,
                message: format!("Elevated response time in {}: {:.2}ms", component, performance.average_response_time_ms),
                timestamp: Utc::now(),
                suggestions: vec![
                    "Monitor database performance".to_string(),
                    "Review recent changes".to_string(),
                ],
            });
        }
        
        // Check success rate
        if performance.success_rate_percent < 95.0 {
            alerts_to_add.push(PerformanceAlert {
                id: format!("{}_success_rate", component),
                level: if performance.success_rate_percent < 90.0 { AlertLevel::Critical } else { AlertLevel::Warning },
                component: component.to_string(),
                metric: "success_rate".to_string(),
                current_value: performance.success_rate_percent,
                threshold: 95.0,
                message: format!("Low success rate in {}: {:.1}%", component, performance.success_rate_percent),
                timestamp: Utc::now(),
                suggestions: vec![
                    "Check error logs".to_string(),
                    "Verify component health".to_string(),
                    "Review recent deployments".to_string(),
                ],
            });
        }
        
        // Add new alerts
        if !alerts_to_add.is_empty() {
            let mut active_alerts = self.active_alerts.write().await;
            for alert in alerts_to_add {
                // Check if alert already exists
                if !active_alerts.iter().any(|a| a.id == alert.id) {
                    warn!("Performance alert: {}", alert.message);
                    active_alerts.push(alert);
                }
            }
        }
    }
    
    /// Collect system resource metrics
    async fn collect_system_metrics() -> SystemResourceUsage {
        // In a real implementation, this would use system monitoring libraries
        // For now, provide simulated but realistic values
        
        SystemResourceUsage {
            cpu_usage_percent: Self::get_cpu_usage(),
            memory_usage_mb: Self::get_memory_usage_mb(),
            memory_total_mb: Self::get_total_memory_mb(),
            memory_usage_percent: Self::get_memory_usage_percent(),
            disk_usage_mb: Self::get_disk_usage_mb(),
            disk_free_mb: Self::get_disk_free_mb(),
            network_rx_mb: Self::get_network_rx_mb(),
            network_tx_mb: Self::get_network_tx_mb(),
            open_file_descriptors: Self::get_open_fds(),
            thread_count: Self::get_thread_count(),
            timestamp: Utc::now(),
        }
    }
    
    /// Check for system-level alerts
    async fn check_system_alerts(
        usage: &SystemResourceUsage,
        thresholds: &PerformanceThresholds,
        active_alerts: &Arc<RwLock<Vec<PerformanceAlert>>>,
    ) {
        let mut new_alerts = Vec::new();
        
        // Check CPU usage
        if usage.cpu_usage_percent > thresholds.cpu_usage_critical {
            new_alerts.push(PerformanceAlert {
                id: "system_cpu_critical".to_string(),
                level: AlertLevel::Critical,
                component: "system".to_string(),
                metric: "cpu_usage".to_string(),
                current_value: usage.cpu_usage_percent,
                threshold: thresholds.cpu_usage_critical,
                message: format!("Critical CPU usage: {:.1}%", usage.cpu_usage_percent),
                timestamp: Utc::now(),
                suggestions: vec![
                    "Check for CPU-intensive processes".to_string(),
                    "Consider scaling resources".to_string(),
                    "Review background tasks".to_string(),
                ],
            });
        } else if usage.cpu_usage_percent > thresholds.cpu_usage_warning {
            new_alerts.push(PerformanceAlert {
                id: "system_cpu_warning".to_string(),
                level: AlertLevel::Warning,
                component: "system".to_string(),
                metric: "cpu_usage".to_string(),
                current_value: usage.cpu_usage_percent,
                threshold: thresholds.cpu_usage_warning,
                message: format!("Elevated CPU usage: {:.1}%", usage.cpu_usage_percent),
                timestamp: Utc::now(),
                suggestions: vec![
                    "Monitor CPU trends".to_string(),
                    "Check recent changes".to_string(),
                ],
            });
        }
        
        // Check memory usage
        if usage.memory_usage_percent > thresholds.memory_usage_critical {
            new_alerts.push(PerformanceAlert {
                id: "system_memory_critical".to_string(),
                level: AlertLevel::Critical,
                component: "system".to_string(),
                metric: "memory_usage".to_string(),
                current_value: usage.memory_usage_percent,
                threshold: thresholds.memory_usage_critical,
                message: format!("Critical memory usage: {:.1}%", usage.memory_usage_percent),
                timestamp: Utc::now(),
                suggestions: vec![
                    "Check for memory leaks".to_string(),
                    "Restart components if needed".to_string(),
                    "Consider increasing memory".to_string(),
                ],
            });
        }
        
        // Add new alerts (avoiding duplicates)
        if !new_alerts.is_empty() {
            let mut alerts = active_alerts.write().await;
            for alert in new_alerts {
                if !alerts.iter().any(|a| a.id == alert.id) {
                    warn!("System alert: {}", alert.message);
                    alerts.push(alert);
                }
            }
        }
    }
    
    /// Update component performance calculations
    async fn update_component_calculations(
        component_metrics: &Arc<RwLock<HashMap<String, ComponentPerformance>>>,
    ) {
        let mut components = component_metrics.write().await;
        let now = Utc::now();
        
        for performance in components.values_mut() {
            // Calculate operations per second
            if let Some(last_time) = performance.last_operation_time {
                let duration = (now - last_time).num_seconds().max(1) as f64;
                performance.operations_per_second = performance.total_operations as f64 / duration;
            }
        }
    }
    
    /// Generate comprehensive performance report
    pub async fn generate_report(&self, period_hours: u32) -> PerformanceReport {
        let cutoff_time = Utc::now() - ChronoDuration::hours(period_hours as i64);
        
        // Get current system resources
        let system_resources = {
            let metrics = self.system_metrics.read().await;
            metrics.back().cloned().unwrap_or_else(|| {
                // Fallback to current metrics if no history
                SystemResourceUsage {
                    cpu_usage_percent: 0.0,
                    memory_usage_mb: 0.0,
                    memory_total_mb: 0.0,
                    memory_usage_percent: 0.0,
                    disk_usage_mb: 0.0,
                    disk_free_mb: 0.0,
                    network_rx_mb: 0.0,
                    network_tx_mb: 0.0,
                    open_file_descriptors: 0,
                    thread_count: 0,
                    timestamp: Utc::now(),
                }
            })
        };
        
        // Get component performances
        let component_performances = self.component_metrics.read().await.clone();
        
        // Get active and resolved alerts
        let active_alerts = self.active_alerts.read().await.clone();
        let resolved_alerts = self.resolved_alerts.read().await
            .iter()
            .filter(|alert| alert.timestamp > cutoff_time)
            .cloned()
            .collect();
        
        // Generate optimization suggestions
        let optimization_suggestions = self.generate_optimization_suggestions(&component_performances);
        
        // Perform trend analysis
        let trend_analysis = self.analyze_trends(period_hours).await;
        
        PerformanceReport {
            generated_at: Utc::now(),
            report_period: ChronoDuration::hours(period_hours as i64),
            system_resources,
            component_performances,
            active_alerts,
            resolved_alerts,
            optimization_suggestions,
            trend_analysis,
        }
    }
    
    /// Generate optimization suggestions based on performance data
    fn generate_optimization_suggestions(
        &self,
        components: &HashMap<String, ComponentPerformance>,
    ) -> Vec<OptimizationSuggestion> {
        let mut suggestions = Vec::new();
        
        for (component, performance) in components {
            // High response time suggestions
            if performance.average_response_time_ms > 1000.0 {
                suggestions.push(OptimizationSuggestion {
                    category: "Performance".to_string(),
                    priority: if performance.average_response_time_ms > 5000.0 {
                        OptimizationPriority::Critical
                    } else {
                        OptimizationPriority::High
                    },
                    title: format!("Optimize {} response time", component),
                    description: format!(
                        "Component {} has high average response time of {:.2}ms",
                        component, performance.average_response_time_ms
                    ),
                    estimated_impact: "Improved user experience and system throughput".to_string(),
                    implementation_difficulty: DifficultyLevel::Medium,
                    component: component.clone(),
                });
            }
            
            // Low success rate suggestions
            if performance.success_rate_percent < 95.0 {
                suggestions.push(OptimizationSuggestion {
                    category: "Reliability".to_string(),
                    priority: if performance.success_rate_percent < 90.0 {
                        OptimizationPriority::Critical
                    } else {
                        OptimizationPriority::High
                    },
                    title: format!("Improve {} reliability", component),
                    description: format!(
                        "Component {} has low success rate of {:.1}%",
                        component, performance.success_rate_percent
                    ),
                    estimated_impact: "Reduced errors and improved stability".to_string(),
                    implementation_difficulty: DifficultyLevel::Hard,
                    component: component.clone(),
                });
            }
        }
        
        suggestions
    }
    
    /// Analyze performance trends
    async fn analyze_trends(&self, period_hours: u32) -> TrendAnalysis {
        let metrics = self.system_metrics.read().await;
        let cutoff_time = Utc::now() - ChronoDuration::hours(period_hours as i64);
        
        let recent_metrics: Vec<&SystemResourceUsage> = metrics
            .iter()
            .filter(|m| m.timestamp > cutoff_time)
            .collect();
        
        if recent_metrics.len() < 2 {
            return TrendAnalysis {
                cpu_trend: TrendDirection::Insufficient_Data,
                memory_trend: TrendDirection::Insufficient_Data,
                response_time_trend: TrendDirection::Insufficient_Data,
                error_rate_trend: TrendDirection::Insufficient_Data,
                throughput_trend: TrendDirection::Insufficient_Data,
                analysis_period_hours: period_hours,
                confidence_level: 0.0,
            };
        }
        
        // Calculate trends (simplified linear trend)
        let cpu_trend = Self::calculate_trend(
            &recent_metrics.iter().map(|m| m.cpu_usage_percent).collect::<Vec<_>>()
        );
        let memory_trend = Self::calculate_trend(
            &recent_metrics.iter().map(|m| m.memory_usage_percent).collect::<Vec<_>>()
        );
        
        TrendAnalysis {
            cpu_trend,
            memory_trend,
            response_time_trend: TrendDirection::Stable, // Would calculate from operation timings
            error_rate_trend: TrendDirection::Stable,    // Would calculate from error rates
            throughput_trend: TrendDirection::Stable,    // Would calculate from operation rates
            analysis_period_hours: period_hours,
            confidence_level: if recent_metrics.len() > 10 { 0.8 } else { 0.5 },
        }
    }
    
    /// Calculate trend direction for a series of values
    fn calculate_trend(values: &[f64]) -> TrendDirection {
        if values.len() < 2 {
            return TrendDirection::Insufficient_Data;
        }
        
        let mid = values.len() / 2;
        let first_half_avg: f64 = values[0..mid].iter().sum::<f64>() / mid as f64;
        let second_half_avg: f64 = values[mid..].iter().sum::<f64>() / (values.len() - mid) as f64;
        
        let change_percent = ((second_half_avg - first_half_avg) / first_half_avg) * 100.0;
        
        match change_percent {
            x if x > 10.0 => TrendDirection::Degrading,
            x if x < -10.0 => TrendDirection::Improving,
            _ => TrendDirection::Stable,
        }
    }
    
    /// Get current alerts
    pub async fn get_active_alerts(&self) -> Vec<PerformanceAlert> {
        self.active_alerts.read().await.clone()
    }
    
    /// Resolve an alert
    pub async fn resolve_alert(&self, alert_id: &str) {
        let mut active = self.active_alerts.write().await;
        if let Some(pos) = active.iter().position(|a| a.id == alert_id) {
            let mut alert = active.remove(pos);
            alert.timestamp = Utc::now(); // Update to resolution time
            
            let mut resolved = self.resolved_alerts.write().await;
            resolved.push(alert);
            
            // Keep only recent resolved alerts
            resolved.retain(|a| a.timestamp > Utc::now() - ChronoDuration::days(7));
        }
    }
    
    /// System monitoring helper functions (would use real system APIs)
    fn get_cpu_usage() -> f64 { 25.0 }
    fn get_memory_usage_mb() -> f64 { 1024.0 }
    fn get_total_memory_mb() -> f64 { 8192.0 }
    fn get_memory_usage_percent() -> f64 { Self::get_memory_usage_mb() / Self::get_total_memory_mb() * 100.0 }
    fn get_disk_usage_mb() -> f64 { 50000.0 }
    fn get_disk_free_mb() -> f64 { 200000.0 }
    fn get_network_rx_mb() -> f64 { 5.0 }
    fn get_network_tx_mb() -> f64 { 2.5 }
    fn get_open_fds() -> u32 { 150 }
    fn get_thread_count() -> u32 { 25 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_thresholds_default() {
        let thresholds = PerformanceThresholds::default();
        assert_eq!(thresholds.cpu_usage_warning, 70.0);
        assert_eq!(thresholds.memory_usage_critical, 95.0);
    }

    #[test]
    fn test_operation_timer() {
        let timer = OperationTimer::new("test".to_string(), "operation".to_string());
        let (component, operation, _duration) = timer.finish();
        assert_eq!(component, "test");
        assert_eq!(operation, "operation");
    }

    #[test]
    fn test_trend_calculation() {
        let improving_values = vec![100.0, 90.0, 80.0, 70.0];
        assert_eq!(PerformanceMonitor::calculate_trend(&improving_values), TrendDirection::Improving);
        
        let degrading_values = vec![50.0, 60.0, 70.0, 80.0];
        assert_eq!(PerformanceMonitor::calculate_trend(&degrading_values), TrendDirection::Degrading);
        
        let stable_values = vec![50.0, 52.0, 48.0, 51.0];
        assert_eq!(PerformanceMonitor::calculate_trend(&stable_values), TrendDirection::Stable);
    }

    #[tokio::test]
    async fn test_performance_monitor_creation() {
        let thresholds = PerformanceThresholds::default();
        let monitor = PerformanceMonitor::new(thresholds);
        
        let alerts = monitor.get_active_alerts().await;
        assert!(alerts.is_empty());
    }
}