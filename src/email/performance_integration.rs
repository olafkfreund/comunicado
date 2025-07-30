//! Performance integration module for seamless optimization
//!
//! This module provides a unified interface for integrating database optimizations
//! with the main Comunicado email client, enabling smooth performance enhancements
//! for large mailboxes without breaking existing functionality.

use crate::email::database::{EmailDatabase, DatabaseResult, StoredMessage};
use crate::email::database_optimizations::{
    OptimizedDatabase, DatabaseOptimizationConfig, PaginationConfig, SearchFilters,
    SortDirection, QueryStats, BatchOperationResult, FolderMessageCount,
};
use crate::email::performance_benchmarks::{
    PerformanceBenchmarkSuite, BenchmarkResults, BenchmarkConfig,
};

use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Performance-enhanced email database manager
pub struct PerformanceEnhancedDatabase {
    /// Base email database
    base_db: EmailDatabase,
    /// Optimized database layer
    optimized_db: OptimizedDatabase,
    /// Performance monitoring
    performance_monitor: Arc<RwLock<PerformanceMonitor>>,
    /// Configuration
    config: PerformanceConfig,
}

/// Performance configuration for the enhanced database
#[derive(Debug, Clone)]
pub struct PerformanceConfig {
    /// Enable performance optimizations
    pub enable_optimizations: bool,
    /// Enable performance monitoring
    pub enable_monitoring: bool,
    /// Threshold for using optimized queries (message count)
    pub optimization_threshold: u32,
    /// Auto-optimization settings
    pub auto_optimize: bool,
    /// Auto-optimization interval in seconds
    pub auto_optimize_interval: u64,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            enable_optimizations: true,
            enable_monitoring: true,
            optimization_threshold: 1000,
            auto_optimize: true,
            auto_optimize_interval: 3600, // 1 hour
        }
    }
}

/// Performance monitoring data
#[derive(Debug, Clone)]
struct PerformanceMonitor {
    query_history: Vec<QueryMetrics>,
    operation_stats: HashMap<String, OperationStats>,
    optimization_history: Vec<OptimizationEvent>,
    last_optimization: Option<DateTime<Utc>>,
}

/// Query performance metrics
#[derive(Debug, Clone)]
struct QueryMetrics {
    operation: String,
    execution_time_ms: u64,
    message_count: u32,
    timestamp: DateTime<Utc>,
    optimized: bool,
}

/// Operation statistics
#[derive(Debug, Clone)]
struct OperationStats {
    total_executions: u64,
    total_time_ms: u64,
    average_time_ms: f64,
    fastest_time_ms: u64,
    slowest_time_ms: u64,
    optimized_executions: u64,
}

/// Optimization event record
#[derive(Debug, Clone)]
struct OptimizationEvent {
    timestamp: DateTime<Utc>,
    operation: String,
    before_time_ms: u64,
    after_time_ms: u64,
    improvement_percent: f64,
}

/// Unified query result with performance information
#[derive(Debug)]
pub struct PerformanceAwareResult<T> {
    pub data: T,
    pub stats: QueryStats,
    pub optimized: bool,
    pub recommendations: Vec<String>,
}

impl PerformanceEnhancedDatabase {
    /// Create a new performance-enhanced database
    pub async fn new(
        db_path: &str,
        performance_config: PerformanceConfig,
    ) -> DatabaseResult<Self> {
        // Create base database
        let base_db = EmailDatabase::new(db_path).await?;
        
        // Create optimized database
        let optimization_config = DatabaseOptimizationConfig {
            max_cached_messages: 2000,
            cache_ttl_seconds: 600,
            batch_size: 250,
            enable_query_cache: performance_config.enable_optimizations,
            enable_connection_pooling: true,
            max_parallel_queries: 12,
        };
        
        let optimized_db = OptimizedDatabase::new(
            base_db.pool.clone(),
            optimization_config,
        ).await?;
        
        // Initialize performance monitor
        let performance_monitor = Arc::new(RwLock::new(PerformanceMonitor {
            query_history: Vec::new(),
            operation_stats: HashMap::new(),
            optimization_history: Vec::new(),
            last_optimization: None,
        }));
        
        Ok(Self {
            base_db,
            optimized_db,
            performance_monitor,
            config: performance_config,
        })
    }

    /// Get messages with automatic optimization selection
    pub async fn get_messages_adaptive(
        &self,
        account_id: &str,
        folder_name: &str,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> DatabaseResult<PerformanceAwareResult<Vec<StoredMessage>>> {
        let start_time = std::time::Instant::now();
        
        // Check if we should use optimized path
        let should_optimize = self.should_use_optimization(account_id, folder_name).await?;
        
        let (messages, stats, optimized) = if should_optimize {
            // Use optimized database
            let pagination = PaginationConfig {
                page_size: limit.unwrap_or(100),
                current_page: offset.unwrap_or(0) / limit.unwrap_or(100),
                sort_field: "date".to_string(),
                sort_direction: SortDirection::Descending,
            };
            
            let (messages, stats) = self.optimized_db.get_messages_paginated(
                account_id,
                folder_name,
                &pagination,
            ).await?;
            
            (messages, stats, true)
        } else {
            // Use base database
            let messages = self.base_db.get_messages(
                account_id,
                folder_name,
                limit,
                offset,
            ).await?;
            
            let execution_time = start_time.elapsed().as_millis() as u64;
            let stats = QueryStats {
                execution_time_ms: execution_time,
                rows_examined: messages.len() as u64,
                rows_returned: messages.len() as u64,
                cache_hit: false,
                memory_used_bytes: 0,
            };
            
            (messages, stats, false)
        };
        
        // Record performance metrics
        if self.config.enable_monitoring {
            self.record_query_metrics("get_messages", &stats, messages.len() as u32, optimized).await;
        }
        
        // Generate recommendations
        let recommendations = self.generate_recommendations(&stats, optimized, messages.len()).await;
        
        Ok(PerformanceAwareResult {
            data: messages,
            stats,
            optimized,
            recommendations,
        })
    }

    /// Search messages with adaptive optimization
    pub async fn search_messages_adaptive(
        &self,
        account_id: &str,
        query: &str,
        filters: Option<SearchFilters>,
        limit: Option<u32>,
    ) -> DatabaseResult<PerformanceAwareResult<Vec<StoredMessage>>> {
        let start_time = std::time::Instant::now();
        
        // Always use optimized search for complex queries
        let should_optimize = !query.is_empty() || filters.is_some();
        
        let (messages, stats, optimized) = if should_optimize && self.config.enable_optimizations {
            let pagination = PaginationConfig {
                page_size: limit.unwrap_or(100),
                current_page: 0,
                sort_field: "date".to_string(),
                sort_direction: SortDirection::Descending,
            };
            
            let search_filters = filters.unwrap_or_default();
            
            let (messages, stats) = self.optimized_db.search_messages_optimized(
                account_id,
                query,
                &search_filters,
                &pagination,
            ).await?;
            
            (messages, stats, true)
        } else {
            // Fallback to base database
            let messages = self.base_db.search_messages(
                account_id,
                query,
                limit,
            ).await?;
            
            let execution_time = start_time.elapsed().as_millis() as u64;
            let stats = QueryStats {
                execution_time_ms: execution_time,
                rows_examined: messages.len() as u64,
                rows_returned: messages.len() as u64,
                cache_hit: false,
                memory_used_bytes: 0,
            };
            
            (messages, stats, false)
        };
        
        // Record metrics
        if self.config.enable_monitoring {
            self.record_query_metrics("search_messages", &stats, messages.len() as u32, optimized).await;
        }
        
        let recommendations = self.generate_recommendations(&stats, optimized, messages.len()).await;
        
        Ok(PerformanceAwareResult {
            data: messages,
            stats,
            optimized,
            recommendations,
        })
    }

    /// Batch insert messages with optimization
    pub async fn batch_store_messages(
        &self,
        messages: &[StoredMessage],
    ) -> DatabaseResult<BatchOperationResult> {
        let start_time = std::time::Instant::now();
        
        let result = if self.config.enable_optimizations && messages.len() > 50 {
            // Use optimized batch insertion for large batches
            self.optimized_db.batch_insert_messages(messages).await?
        } else {
            // Use individual insertions for small batches
            let mut successful = 0;
            let mut failed = 0;
            let mut errors = Vec::new();
            
            for message in messages {
                match self.base_db.store_message(message).await {
                    Ok(()) => successful += 1,
                    Err(e) => {
                        failed += 1;
                        errors.push(format!("Failed to store message {}: {}", message.id, e));
                    }
                }
            }
            
            BatchOperationResult {
                successful_operations: successful,
                failed_operations: failed,
                errors,
                execution_time_ms: start_time.elapsed().as_millis() as u64,
            }
        };
        
        // Record metrics
        if self.config.enable_monitoring {
            let stats = QueryStats {
                execution_time_ms: result.execution_time_ms,
                rows_examined: messages.len() as u64,
                rows_returned: result.successful_operations as u64,
                cache_hit: false,
                memory_used_bytes: 0,
            };
            
            self.record_query_metrics(
                "batch_store_messages",
                &stats,
                messages.len() as u32,
                messages.len() > 50,
            ).await;
        }
        
        Ok(result)
    }

    /// Get folder message counts with caching
    pub async fn get_folder_counts_adaptive(
        &self,
        account_id: &str,
    ) -> DatabaseResult<PerformanceAwareResult<HashMap<String, FolderMessageCount>>> {
        let start_time = std::time::Instant::now();
        
        let (counts, optimized) = if self.config.enable_optimizations {
            // Use optimized folder counts
            (self.optimized_db.get_folder_message_counts(account_id).await?, true)
        } else {
            // Calculate manually using base database
            let folders = self.base_db.get_folders(account_id).await?;
            let mut counts = HashMap::new();
            
            for folder in folders {
                let messages = self.base_db.get_messages(
                    account_id,
                    &folder.name,
                    None,
                    None,
                ).await?;
                
                let unread_count = messages.iter()
                    .filter(|m| !m.flags.contains(&"\\Seen".to_string()))
                    .count() as u32;
                
                let latest_date = messages.iter()
                    .map(|m| m.date)
                    .max();
                
                counts.insert(folder.name.clone(), FolderMessageCount {
                    folder_name: folder.name,
                    total_count: messages.len() as u32,
                    unread_count,
                    draft_count: 0, // Would need additional query
                    latest_message_date: latest_date,
                });
            }
            
            (counts, false)
        };
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        let stats = QueryStats {
            execution_time_ms: execution_time,
            rows_examined: counts.len() as u64,
            rows_returned: counts.len() as u64,
            cache_hit: false,
            memory_used_bytes: 0,
        };
        
        if self.config.enable_monitoring {
            self.record_query_metrics("get_folder_counts", &stats, counts.len() as u32, optimized).await;
        }
        
        let recommendations = self.generate_recommendations(&stats, optimized, counts.len()).await;
        
        Ok(PerformanceAwareResult {
            data: counts,
            stats,
            optimized,
            recommendations,
        })
    }

    /// Run performance benchmark
    #[cfg(test)]
    pub async fn run_performance_benchmark(
        &self,
        config: Option<BenchmarkConfig>,
    ) -> Result<Vec<BenchmarkResults>, Box<dyn std::error::Error>> {
        // Create a temporary database for benchmarking
        let temp_file = tempfile::NamedTempFile::new()?;
        let benchmark_path = temp_file.path().to_str().unwrap();
        
        let benchmark_suite = PerformanceBenchmarkSuite::new(benchmark_path).await?;
        let benchmark_config = config.unwrap_or_default();
        
        benchmark_suite.run_full_benchmark_suite(&benchmark_config).await
    }

    /// Get performance statistics
    pub async fn get_performance_stats(&self) -> PerformanceStats {
        let monitor = self.performance_monitor.read().await;
        
        let total_queries = monitor.query_history.len();
        let optimized_queries = monitor.query_history.iter()
            .filter(|q| q.optimized)
            .count();
        
        let average_execution_time = if total_queries > 0 {
            monitor.query_history.iter()
                .map(|q| q.execution_time_ms)
                .sum::<u64>() as f64 / total_queries as f64
        } else {
            0.0
        };
        
        let optimization_rate = if total_queries > 0 {
            optimized_queries as f64 / total_queries as f64
        } else {
            0.0
        };
        
        PerformanceStats {
            total_queries: total_queries as u64,
            optimized_queries: optimized_queries as u64,
            optimization_rate,
            average_execution_time_ms: average_execution_time,
            recent_optimizations: monitor.optimization_history.len() as u32,
            cache_enabled: self.config.enable_optimizations,
        }
    }

    /// Optimize database (run maintenance)
    pub async fn optimize_database(&self) -> DatabaseResult<()> {
        if self.config.enable_optimizations {
            let report = self.optimized_db.optimize_database().await?;
            
            // Record optimization event
            if self.config.enable_monitoring {
                let mut monitor = self.performance_monitor.write().await;
                monitor.last_optimization = Some(Utc::now());
                
                monitor.optimization_history.push(OptimizationEvent {
                    timestamp: Utc::now(),
                    operation: "database_optimization".to_string(),
                    before_time_ms: 0, // Would need before measurement
                    after_time_ms: report.execution_time_ms,
                    improvement_percent: 0.0, // Would calculate based on before/after
                });
            }
        }
        
        Ok(())
    }

    /// Private helper methods
    async fn should_use_optimization(
        &self,
        account_id: &str,
        folder_name: &str,
    ) -> DatabaseResult<bool> {
        if !self.config.enable_optimizations {
            return Ok(false);
        }
        
        // Check message count to determine if optimization is beneficial
        let stats = self.base_db.get_stats().await?;
        Ok(stats.message_count > self.config.optimization_threshold)
    }

    async fn record_query_metrics(
        &self,
        operation: &str,
        stats: &QueryStats,
        message_count: u32,
        optimized: bool,
    ) {
        let mut monitor = self.performance_monitor.write().await;
        
        // Add to query history
        monitor.query_history.push(QueryMetrics {
            operation: operation.to_string(),
            execution_time_ms: stats.execution_time_ms,
            message_count,
            timestamp: Utc::now(),
            optimized,
        });
        
        // Update operation statistics
        let op_stats = monitor.operation_stats
            .entry(operation.to_string())
            .or_insert(OperationStats {
                total_executions: 0,
                total_time_ms: 0,
                average_time_ms: 0.0,
                fastest_time_ms: u64::MAX,
                slowest_time_ms: 0,
                optimized_executions: 0,
            });
        
        op_stats.total_executions += 1;
        op_stats.total_time_ms += stats.execution_time_ms;
        op_stats.average_time_ms = op_stats.total_time_ms as f64 / op_stats.total_executions as f64;
        op_stats.fastest_time_ms = op_stats.fastest_time_ms.min(stats.execution_time_ms);
        op_stats.slowest_time_ms = op_stats.slowest_time_ms.max(stats.execution_time_ms);
        
        if optimized {
            op_stats.optimized_executions += 1;
        }
        
        // Keep only recent history (last 1000 queries)
        if monitor.query_history.len() > 1000 {
            let len = monitor.query_history.len();
            monitor.query_history.drain(0..len - 1000);
        }
    }

    async fn generate_recommendations(
        &self,
        stats: &QueryStats,
        optimized: bool,
        result_count: usize,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        // Performance-based recommendations
        if stats.execution_time_ms > 1000 {
            recommendations.push("Query took over 1 second. Consider using pagination for large result sets.".to_string());
        }
        
        if !optimized && result_count > 100 {
            recommendations.push("Large result set detected. Enable optimizations for better performance.".to_string());
        }
        
        if stats.memory_used_bytes > 50 * 1024 * 1024 {
            recommendations.push("High memory usage detected. Consider reducing page size or enabling caching.".to_string());
        }
        
        if !stats.cache_hit && optimized {
            recommendations.push("Cache miss on optimized query. Consider warming up the cache.".to_string());
        }
        
        recommendations
    }
}

/// Performance statistics summary
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub total_queries: u64,
    pub optimized_queries: u64,
    pub optimization_rate: f64,
    pub average_execution_time_ms: f64,
    pub recent_optimizations: u32,
    pub cache_enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_performance_enhanced_database_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_str().unwrap();
        
        let config = PerformanceConfig::default();
        let enhanced_db = PerformanceEnhancedDatabase::new(db_path, config).await;
        
        assert!(enhanced_db.is_ok());
    }

    #[tokio::test]
    async fn test_performance_config() {
        let config = PerformanceConfig {
            enable_optimizations: true,
            enable_monitoring: true,
            optimization_threshold: 500,
            auto_optimize: false,
            auto_optimize_interval: 1800,
        };
        
        assert!(config.enable_optimizations);
        assert_eq!(config.optimization_threshold, 500);
    }

    #[tokio::test]
    async fn test_performance_stats() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_str().unwrap();
        
        let config = PerformanceConfig::default();
        let enhanced_db = PerformanceEnhancedDatabase::new(db_path, config).await.unwrap();
        
        let stats = enhanced_db.get_performance_stats().await;
        assert_eq!(stats.total_queries, 0);
        assert!(stats.cache_enabled);
    }
}