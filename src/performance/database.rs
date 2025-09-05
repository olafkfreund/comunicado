//! Database performance optimization and monitoring

use crate::performance::{PerformanceResult, PerformanceError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
// use uuid::Uuid;

/// Database connection pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionPoolConfig {
    /// Minimum number of connections in pool
    pub min_connections: u32,
    /// Maximum number of connections in pool
    pub max_connections: u32,
    /// Connection timeout duration
    pub connect_timeout: Duration,
    /// Idle connection timeout
    pub idle_timeout: Duration,
    /// Maximum lifetime of a connection
    pub max_lifetime: Duration,
    /// Connection acquisition timeout
    pub acquire_timeout: Duration,
}

/// Query optimization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryOptimizerConfig {
    /// Enable query caching
    pub enable_query_cache: bool,
    /// Maximum queries in cache
    pub max_cached_queries: usize,
    /// Query execution timeout
    pub query_timeout: Duration,
    /// Enable slow query logging
    pub log_slow_queries: bool,
    /// Slow query threshold
    pub slow_query_threshold: Duration,
    /// Enable query plan analysis
    pub analyze_query_plans: bool,
    /// Maximum prepared statements
    pub max_prepared_statements: usize,
}

/// Index analysis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexAnalysisConfig {
    /// Enable automatic index suggestions
    pub auto_suggest_indexes: bool,
    /// Minimum query frequency for index suggestions
    pub min_query_frequency: u64,
    /// Enable unused index detection
    pub detect_unused_indexes: bool,
    /// Threshold for unused index detection (days)
    pub unused_threshold_days: u64,
    /// Enable index fragmentation analysis
    pub analyze_fragmentation: bool,
}

/// Query execution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryStats {
    /// Query hash or identifier
    pub query_id: String,
    /// Number of executions
    pub execution_count: u64,
    /// Total execution time
    pub total_duration: Duration,
    /// Average execution time
    pub avg_duration: Duration,
    /// Minimum execution time
    pub min_duration: Duration,
    /// Maximum execution time
    pub max_duration: Duration,
    /// Last execution time
    pub last_executed: Option<Instant>,
    /// Number of rows affected (average)
    pub avg_rows_affected: f64,
    /// Query complexity score
    pub complexity_score: f32,
    /// Cache hit ratio for this query
    pub cache_hit_ratio: f64,
}

/// Database performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseMetrics {
    /// Total number of queries executed
    pub total_queries: u64,
    /// Successful query count
    pub successful_queries: u64,
    /// Failed query count
    pub failed_queries: u64,
    /// Average query duration
    pub avg_query_duration: Duration,
    /// 95th percentile query duration
    pub p95_query_duration: Duration,
    /// 99th percentile query duration
    pub p99_query_duration: Duration,
    /// Connection pool utilization (0.0-1.0)
    pub pool_utilization: f64,
    /// Active connections
    pub active_connections: u32,
    /// Idle connections
    pub idle_connections: u32,
    /// Query cache hit ratio
    pub cache_hit_ratio: f64,
    /// Deadlock count
    pub deadlock_count: u64,
    /// Lock wait time (total)
    pub total_lock_wait_time: Duration,
}

/// Database statistics aggregation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStatistics {
    /// Overall database metrics
    pub metrics: DatabaseMetrics,
    /// Per-query statistics
    pub query_stats: HashMap<String, QueryStats>,
    /// Connection pool statistics
    pub pool_stats: ConnectionPoolStats,
    /// Index usage statistics
    pub index_stats: HashMap<String, IndexUsageStats>,
    /// Table statistics
    pub table_stats: HashMap<String, TableStats>,
    /// Last statistics update
    pub last_updated: Option<Instant>,
}

/// Connection pool statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionPoolStats {
    /// Total connections created
    pub total_created: u64,
    /// Total connections closed
    pub total_closed: u64,
    /// Current active connections
    pub active_connections: u32,
    /// Current idle connections
    pub idle_connections: u32,
    /// Average connection acquisition time
    pub avg_acquire_time: Duration,
    /// Connection timeouts
    pub timeout_count: u64,
    /// Pool high water mark
    pub max_connections_used: u32,
}

/// Index usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexUsageStats {
    /// Index name
    pub index_name: String,
    /// Table name
    pub table_name: String,
    /// Number of seeks (precise lookups)
    pub seeks: u64,
    /// Number of scans (range queries)
    pub scans: u64,
    /// Number of lookups (key lookups)
    pub lookups: u64,
    /// Last used timestamp
    pub last_used: Option<Instant>,
    /// Size in bytes
    pub size_bytes: u64,
    /// Fragmentation percentage
    pub fragmentation_percent: f32,
    /// User update count
    pub user_updates: u64,
    /// System update count
    pub system_updates: u64,
}

/// Table statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableStats {
    /// Table name
    pub table_name: String,
    /// Number of rows
    pub row_count: u64,
    /// Table size in bytes
    pub size_bytes: u64,
    /// Index size in bytes
    pub index_size_bytes: u64,
    /// User table scans
    pub user_scans: u64,
    /// User seeks
    pub user_seeks: u64,
    /// User lookups
    pub user_lookups: u64,
    /// User updates
    pub user_updates: u64,
    /// Last user scan
    pub last_user_scan: Option<Instant>,
    /// Last user seek
    pub last_user_seek: Option<Instant>,
    /// Last user update
    pub last_user_update: Option<Instant>,
}

/// Database optimizer for performance tuning
pub struct DatabaseOptimizer {
    /// Optimizer configuration
    config: QueryOptimizerConfig,
    /// Query statistics tracking
    query_stats: Arc<RwLock<HashMap<String, QueryStats>>>,
    /// Slow query log
    slow_queries: Arc<RwLock<Vec<SlowQuery>>>,
    /// Query cache
    query_cache: Arc<RwLock<HashMap<String, CachedQuery>>>,
}

/// Slow query information
#[derive(Debug, Clone)]
struct SlowQuery {
    query: String,
    duration: Duration,
    timestamp: Instant,
    parameters: Option<Vec<String>>,
    execution_plan: Option<String>,
}

/// Cached query result
#[derive(Debug, Clone)]
struct CachedQuery {
    result_hash: u64,
    created_at: Instant,
    last_accessed: Instant,
    access_count: u64,
}

impl DatabaseOptimizer {
    /// Create a new database optimizer
    pub fn new(config: QueryOptimizerConfig) -> Self {
        Self {
            config,
            query_stats: Arc::new(RwLock::new(HashMap::new())),
            slow_queries: Arc::new(RwLock::new(Vec::new())),
            query_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Record query execution statistics
    pub async fn record_query_execution(
        &self,
        query_id: String,
        duration: Duration,
        rows_affected: u64,
        success: bool,
    ) -> PerformanceResult<()> {
        let mut stats = self.query_stats.write().await;
        
        let query_stat = stats.entry(query_id.clone()).or_insert_with(|| QueryStats {
            query_id: query_id.clone(),
            execution_count: 0,
            total_duration: Duration::from_secs(0),
            avg_duration: Duration::from_secs(0),
            min_duration: Duration::from_secs(u64::MAX),
            max_duration: Duration::from_secs(0),
            last_executed: None,
            avg_rows_affected: 0.0,
            complexity_score: 0.0,
            cache_hit_ratio: 0.0,
        });

        // Update statistics
        query_stat.execution_count += 1;
        query_stat.total_duration += duration;
        query_stat.avg_duration = query_stat.total_duration / query_stat.execution_count as u32;
        query_stat.min_duration = query_stat.min_duration.min(duration);
        query_stat.max_duration = query_stat.max_duration.max(duration);
        query_stat.last_executed = Some(Instant::now());
        
        // Update average rows affected
        let total_rows = query_stat.avg_rows_affected * (query_stat.execution_count - 1) as f64;
        query_stat.avg_rows_affected = (total_rows + rows_affected as f64) / query_stat.execution_count as f64;

        // Log slow queries if enabled
        if self.config.log_slow_queries && duration >= self.config.slow_query_threshold {
            let mut slow_queries = self.slow_queries.write().await;
            slow_queries.push(SlowQuery {
                query: query_id.clone(),
                duration,
                timestamp: Instant::now(),
                parameters: None,
                execution_plan: None,
            });
            
            // Limit slow query log size
            if slow_queries.len() > 1000 {
                slow_queries.truncate(800);
            }
        }

        Ok(())
    }

    /// Get query statistics
    pub async fn get_query_stats(&self, query_id: &str) -> Option<QueryStats> {
        let stats = self.query_stats.read().await;
        stats.get(query_id).cloned()
    }

    /// Get all query statistics
    pub async fn get_all_query_stats(&self) -> HashMap<String, QueryStats> {
        let stats = self.query_stats.read().await;
        stats.clone()
    }

    /// Get slow queries
    pub async fn get_slow_queries(&self, limit: Option<usize>) -> Vec<SlowQuery> {
        let slow_queries = self.slow_queries.read().await;
        let limit = limit.unwrap_or(100);
        
        if slow_queries.len() <= limit {
            slow_queries.clone()
        } else {
            slow_queries[slow_queries.len() - limit..].to_vec()
        }
    }

    /// Analyze query performance and provide recommendations
    pub async fn analyze_query_performance(&self, query_id: &str) -> PerformanceResult<QueryAnalysis> {
        let stats = self.query_stats.read().await;
        
        if let Some(query_stat) = stats.get(query_id) {
            let mut recommendations = Vec::new();
            
            // Check for slow queries
            if query_stat.avg_duration > Duration::from_millis(100) {
                recommendations.push("Consider optimizing query - average execution time is high".to_string());
            }
            
            // Check for frequent queries
            if query_stat.execution_count > 1000 {
                recommendations.push("Consider caching this frequently executed query".to_string());
            }
            
            // Check for high variance in execution time
            let variance_ratio = query_stat.max_duration.as_millis() as f64 / 
                               query_stat.min_duration.as_millis().max(1) as f64;
            if variance_ratio > 10.0 {
                recommendations.push("High variance in execution time - investigate query plan instability".to_string());
            }
            
            // Check cache hit ratio
            if query_stat.cache_hit_ratio < 0.8 && query_stat.execution_count > 100 {
                recommendations.push("Low cache hit ratio - consider query optimization or cache warming".to_string());
            }

            let analysis = QueryAnalysis {
                query_id: query_id.to_string(),
                performance_score: calculate_performance_score(query_stat),
                recommendations,
                optimization_potential: calculate_optimization_potential(query_stat),
                resource_impact: calculate_resource_impact(query_stat),
            };
            
            Ok(analysis)
        } else {
            Err(PerformanceError::DatabaseError(format!(
                "No statistics found for query: {}", query_id
            )))
        }
    }

    /// Clear old statistics to prevent memory growth
    pub async fn cleanup_old_stats(&self, retention_days: u64) -> PerformanceResult<usize> {
        let cutoff = Instant::now() - Duration::from_secs(retention_days * 24 * 3600);
        let mut removed_count = 0;
        
        // Clean up query stats
        {
            let mut stats = self.query_stats.write().await;
            let keys_to_remove: Vec<String> = stats
                .iter()
                .filter(|(_, stat)| {
                    stat.last_executed
                        .map(|last_exec| last_exec < cutoff)
                        .unwrap_or(true)
                })
                .map(|(key, _)| key.clone())
                .collect();
            
            for key in keys_to_remove {
                stats.remove(&key);
                removed_count += 1;
            }
        }
        
        // Clean up slow queries
        {
            let mut slow_queries = self.slow_queries.write().await;
            slow_queries.retain(|query| query.timestamp >= cutoff);
        }
        
        // Clean up query cache
        {
            let mut cache = self.query_cache.write().await;
            cache.retain(|_, cached| cached.created_at >= cutoff);
        }
        
        Ok(removed_count)
    }
}

/// Query performance analysis result
#[derive(Debug, Clone)]
pub struct QueryAnalysis {
    pub query_id: String,
    pub performance_score: f32,
    pub recommendations: Vec<String>,
    pub optimization_potential: f32,
    pub resource_impact: ResourceImpact,
}

/// Resource impact assessment
#[derive(Debug, Clone)]
pub struct ResourceImpact {
    pub cpu_impact: f32,
    pub memory_impact: f32,
    pub io_impact: f32,
    pub network_impact: f32,
}

/// Query optimizer for automatic optimization
pub struct QueryOptimizer {
    config: QueryOptimizerConfig,
    optimizer: DatabaseOptimizer,
}

impl QueryOptimizer {
    /// Create a new query optimizer
    pub fn new(config: QueryOptimizerConfig) -> Self {
        let optimizer = DatabaseOptimizer::new(config.clone());
        Self {
            config,
            optimizer,
        }
    }

    /// Optimize query execution plan
    pub async fn optimize_query(&self, query: &str) -> PerformanceResult<OptimizedQuery> {
        // This would integrate with actual database query planner
        // For now, we provide basic optimization suggestions
        
        let mut optimizations = Vec::new();
        let mut estimated_improvement = 0.0;
        
        // Basic query pattern analysis
        let query_lower = query.to_lowercase();
        
        // Check for missing WHERE clause
        if query_lower.contains("select") && !query_lower.contains("where") && !query_lower.contains("limit") {
            optimizations.push(QueryOptimization {
                optimization_type: "Add WHERE clause".to_string(),
                description: "Consider adding WHERE clause to limit result set".to_string(),
                estimated_improvement: 0.3,
            });
            estimated_improvement += 0.3;
        }
        
        // Check for SELECT *
        if query_lower.contains("select *") {
            optimizations.push(QueryOptimization {
                optimization_type: "Avoid SELECT *".to_string(),
                description: "Select only required columns instead of using SELECT *".to_string(),
                estimated_improvement: 0.15,
            });
            estimated_improvement += 0.15;
        }
        
        // Check for ORDER BY without LIMIT
        if query_lower.contains("order by") && !query_lower.contains("limit") {
            optimizations.push(QueryOptimization {
                optimization_type: "Add LIMIT clause".to_string(),
                description: "Consider adding LIMIT clause when using ORDER BY".to_string(),
                estimated_improvement: 0.2,
            });
            estimated_improvement += 0.2;
        }
        
        // Check for subqueries that could be joins
        if query_lower.contains("in (select") {
            optimizations.push(QueryOptimization {
                optimization_type: "Convert subquery to JOIN".to_string(),
                description: "Consider converting IN subquery to JOIN for better performance".to_string(),
                estimated_improvement: 0.25,
            });
            estimated_improvement += 0.25;
        }

        Ok(OptimizedQuery {
            original_query: query.to_string(),
            optimizations,
            estimated_improvement: estimated_improvement.min(1.0f64),
            complexity_score: calculate_query_complexity(query),
        })
    }

    /// Suggest indexes for query optimization
    pub async fn suggest_indexes(&self, query: &str) -> PerformanceResult<Vec<IndexSuggestion>> {
        let mut suggestions = Vec::new();
        let query_lower = query.to_lowercase();
        
        // Parse WHERE clauses for potential indexes
        if let Some(where_start) = query_lower.find("where") {
            let where_clause = &query_lower[where_start + 5..];
            
            // Look for column comparisons
            let conditions: Vec<&str> = where_clause
                .split(&['=', '<', '>', '!'][..])
                .map(|s| s.trim())
                .filter(|s| !s.is_empty() && !s.contains(' '))
                .collect();
            
            for condition in conditions {
                if condition.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    suggestions.push(IndexSuggestion {
                        table_name: "unknown".to_string(), // Would be parsed from query
                        column_names: vec![condition.to_string()],
                        index_type: IndexType::BTree,
                        estimated_benefit: 0.4,
                        reasoning: format!("Column '{}' used in WHERE clause", condition),
                    });
                }
            }
        }
        
        // Look for ORDER BY columns
        if let Some(order_start) = query_lower.find("order by") {
            let order_clause = &query_lower[order_start + 8..];
            let order_column = order_clause.split_whitespace().next().unwrap_or("");
            
            if !order_column.is_empty() {
                suggestions.push(IndexSuggestion {
                    table_name: "unknown".to_string(),
                    column_names: vec![order_column.to_string()],
                    index_type: IndexType::BTree,
                    estimated_benefit: 0.3,
                    reasoning: format!("Column '{}' used in ORDER BY clause", order_column),
                });
            }
        }

        Ok(suggestions)
    }
}

/// Optimized query result
#[derive(Debug, Clone)]
pub struct OptimizedQuery {
    pub original_query: String,
    pub optimizations: Vec<QueryOptimization>,
    pub estimated_improvement: f32,
    pub complexity_score: f32,
}

/// Individual query optimization
#[derive(Debug, Clone)]
pub struct QueryOptimization {
    pub optimization_type: String,
    pub description: String,
    pub estimated_improvement: f32,
}

/// Index suggestion
#[derive(Debug, Clone)]
pub struct IndexSuggestion {
    pub table_name: String,
    pub column_names: Vec<String>,
    pub index_type: IndexType,
    pub estimated_benefit: f32,
    pub reasoning: String,
}

/// Index type enumeration
#[derive(Debug, Clone)]
pub enum IndexType {
    BTree,
    Hash,
    Bitmap,
    Partial,
    Composite,
}

/// Connection pool manager
pub struct ConnectionPool {
    config: ConnectionPoolConfig,
    stats: Arc<RwLock<ConnectionPoolStats>>,
    active_connections: Arc<RwLock<u32>>,
    idle_connections: Arc<RwLock<u32>>,
}

impl ConnectionPool {
    /// Create a new connection pool
    pub fn new(config: ConnectionPoolConfig) -> Self {
        Self {
            config,
            stats: Arc::new(RwLock::new(ConnectionPoolStats {
                total_created: 0,
                total_closed: 0,
                active_connections: 0,
                idle_connections: 0,
                avg_acquire_time: Duration::from_millis(0),
                timeout_count: 0,
                max_connections_used: 0,
            })),
            active_connections: Arc::new(RwLock::new(0)),
            idle_connections: Arc::new(RwLock::new(0)),
        }
    }

    /// Get connection pool statistics
    pub async fn get_statistics(&self) -> ConnectionPoolStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Monitor pool health and performance
    pub async fn monitor_pool_health(&self) -> PerformanceResult<PoolHealth> {
        let stats = self.get_statistics().await;
        let active = *self.active_connections.read().await;
        let idle = *self.idle_connections.read().await;
        
        let utilization = active as f32 / self.config.max_connections as f32;
        let health_score = calculate_pool_health_score(&stats, utilization);
        
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();
        
        // Check for high utilization
        if utilization > 0.8 {
            issues.push("High connection pool utilization".to_string());
            recommendations.push("Consider increasing max_connections".to_string());
        }
        
        // Check for frequent timeouts
        if stats.timeout_count > stats.total_created / 20 {
            issues.push("Frequent connection timeouts".to_string());
            recommendations.push("Consider increasing acquire_timeout or optimizing queries".to_string());
        }
        
        // Check for connection churn
        let churn_ratio = stats.total_closed as f32 / stats.total_created.max(1) as f32;
        if churn_ratio > 0.5 {
            issues.push("High connection churn".to_string());
            recommendations.push("Consider increasing connection max_lifetime".to_string());
        }

        Ok(PoolHealth {
            utilization,
            health_score,
            active_connections: active,
            idle_connections: idle,
            issues,
            recommendations,
        })
    }
}

/// Connection pool health assessment
#[derive(Debug, Clone)]
pub struct PoolHealth {
    pub utilization: f32,
    pub health_score: f32,
    pub active_connections: u32,
    pub idle_connections: u32,
    pub issues: Vec<String>,
    pub recommendations: Vec<String>,
}

/// Index analyzer for database optimization
pub struct IndexAnalyzer {
    config: IndexAnalysisConfig,
}

impl IndexAnalyzer {
    /// Create a new index analyzer
    pub fn new(config: IndexAnalysisConfig) -> Self {
        Self { config }
    }

    /// Analyze index usage and provide optimization suggestions
    pub async fn analyze_index_usage(&self) -> PerformanceResult<Vec<IndexAnalysisResult>> {
        // This would integrate with actual database metadata
        // For now, we provide a framework for analysis
        
        let mut results = Vec::new();
        
        // This would query database metadata tables to get actual index statistics
        // For example: sys.dm_db_index_usage_stats in SQL Server
        // or information_schema.statistics in MySQL
        
        results.push(IndexAnalysisResult {
            index_name: "example_idx".to_string(),
            table_name: "example_table".to_string(),
            analysis_type: IndexAnalysisType::Unused,
            recommendation: "Consider dropping this unused index".to_string(),
            impact_score: 0.3,
            details: "Index has not been used in the last 30 days".to_string(),
        });

        Ok(results)
    }

    /// Suggest new indexes based on query patterns
    pub async fn suggest_missing_indexes(&self) -> PerformanceResult<Vec<IndexSuggestion>> {
        // This would analyze query execution plans and missing index hints
        let mut suggestions = Vec::new();
        
        // Framework for missing index detection
        suggestions.push(IndexSuggestion {
            table_name: "users".to_string(),
            column_names: vec!["email".to_string()],
            index_type: IndexType::BTree,
            estimated_benefit: 0.6,
            reasoning: "Frequently queried column in WHERE clauses".to_string(),
        });

        Ok(suggestions)
    }
}

/// Index analysis result
#[derive(Debug, Clone)]
pub struct IndexAnalysisResult {
    pub index_name: String,
    pub table_name: String,
    pub analysis_type: IndexAnalysisType,
    pub recommendation: String,
    pub impact_score: f32,
    pub details: String,
}

/// Type of index analysis
#[derive(Debug, Clone)]
pub enum IndexAnalysisType {
    Unused,
    Duplicate,
    Fragmented,
    Inefficient,
    Missing,
}

// Helper functions

fn calculate_performance_score(stats: &QueryStats) -> f32 {
    // Simple scoring based on execution time and frequency
    let frequency_score = (stats.execution_count as f32).ln().min(10.0) / 10.0;
    let speed_score = 1.0 - (stats.avg_duration.as_millis() as f32 / 1000.0).min(1.0);
    let cache_score = stats.cache_hit_ratio as f32;
    
    (frequency_score + speed_score + cache_score) / 3.0
}

fn calculate_optimization_potential(stats: &QueryStats) -> f32 {
    // Higher potential for slow, frequent queries
    let frequency_factor = (stats.execution_count as f32).ln() / 10.0;
    let slowness_factor = stats.avg_duration.as_millis() as f32 / 1000.0;
    let variance_factor = if stats.max_duration.as_millis() > 0 {
        stats.max_duration.as_millis() as f32 / stats.min_duration.as_millis().max(1) as f32
    } else {
        1.0
    };
    
    (frequency_factor * slowness_factor * variance_factor.ln()).min(1.0)
}

fn calculate_resource_impact(stats: &QueryStats) -> ResourceImpact {
    let base_impact = stats.avg_duration.as_millis() as f32 / 1000.0;
    let frequency_multiplier = (stats.execution_count as f32).ln() / 10.0;
    
    ResourceImpact {
        cpu_impact: base_impact * frequency_multiplier,
        memory_impact: (stats.avg_rows_affected as f32 / 10000.0).min(1.0),
        io_impact: base_impact * 0.8, // Assume most queries are I/O bound
        network_impact: (stats.avg_rows_affected as f32 / 1000.0).min(1.0) * 0.1,
    }
}

fn calculate_query_complexity(query: &str) -> f32 {
    let query_lower = query.to_lowercase();
    let mut complexity = 0.0;
    
    // Basic complexity indicators
    complexity += query_lower.matches("join").count() as f32 * 0.2;
    complexity += query_lower.matches("subquery").count() as f32 * 0.3;
    complexity += query_lower.matches("union").count() as f32 * 0.25;
    complexity += query_lower.matches("order by").count() as f32 * 0.1;
    complexity += query_lower.matches("group by").count() as f32 * 0.15;
    complexity += query_lower.matches("having").count() as f32 * 0.1;
    
    complexity.min(1.0)
}

fn calculate_pool_health_score(stats: &ConnectionPoolStats, utilization: f32) -> f32 {
    let mut score = 1.0;
    
    // Penalize high utilization
    if utilization > 0.8 {
        score -= (utilization - 0.8) * 2.0;
    }
    
    // Penalize timeouts
    let timeout_ratio = stats.timeout_count as f32 / stats.total_created.max(1) as f32;
    score -= timeout_ratio;
    
    // Penalize high connection churn
    let churn_ratio = stats.total_closed as f32 / stats.total_created.max(1) as f32;
    if churn_ratio > 0.3 {
        score -= (churn_ratio - 0.3) * 0.5;
    }
    
    score.max(0.0).min(1.0)
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            min_connections: 5,
            max_connections: 20,
            connect_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(600), // 10 minutes
            max_lifetime: Duration::from_secs(3600), // 1 hour
            acquire_timeout: Duration::from_secs(10),
        }
    }
}

impl Default for QueryOptimizerConfig {
    fn default() -> Self {
        Self {
            enable_query_cache: true,
            max_cached_queries: 1000,
            query_timeout: Duration::from_secs(30),
            log_slow_queries: true,
            slow_query_threshold: Duration::from_millis(100),
            analyze_query_plans: true,
            max_prepared_statements: 500,
        }
    }
}

impl Default for IndexAnalysisConfig {
    fn default() -> Self {
        Self {
            auto_suggest_indexes: true,
            min_query_frequency: 10,
            detect_unused_indexes: true,
            unused_threshold_days: 30,
            analyze_fragmentation: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_optimizer() {
        let config = QueryOptimizerConfig::default();
        let optimizer = DatabaseOptimizer::new(config);

        // Record some query executions
        optimizer.record_query_execution(
            "SELECT * FROM users WHERE id = ?".to_string(),
            Duration::from_millis(50),
            1,
            true,
        ).await.unwrap();

        // Get statistics
        let stats = optimizer.get_query_stats("SELECT * FROM users WHERE id = ?").await;
        assert!(stats.is_some());

        let stats = stats.unwrap();
        assert_eq!(stats.execution_count, 1);
        assert_eq!(stats.avg_duration, Duration::from_millis(50));
    }

    #[tokio::test] 
    async fn test_query_optimizer() {
        let config = QueryOptimizerConfig::default();
        let optimizer = QueryOptimizer::new(config);

        // Test query optimization
        let result = optimizer.optimize_query("SELECT * FROM users ORDER BY name").await.unwrap();
        assert!(!result.optimizations.is_empty());
        assert!(result.estimated_improvement > 0.0);
    }

    #[tokio::test]
    async fn test_connection_pool() {
        let config = ConnectionPoolConfig::default();
        let pool = ConnectionPool::new(config);

        let health = pool.monitor_pool_health().await.unwrap();
        assert!(health.health_score >= 0.0 && health.health_score <= 1.0);
    }
}