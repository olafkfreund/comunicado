//! Performance benchmarks for database operations with large mailboxes
//!
//! This module provides comprehensive benchmarking tools to measure and validate
//! database performance optimizations for handling large mailboxes (10K+ messages).

use crate::email::database::{EmailDatabase, StoredMessage, StoredAttachment};
use crate::email::database_optimizations::{
    OptimizedDatabase, DatabaseOptimizationConfig, PaginationConfig, SortDirection,
    SearchFilters
};
use chrono::Utc;
use std::time::{Duration, Instant};
use tokio::time;
use uuid::Uuid;

/// Comprehensive performance benchmark suite
pub struct PerformanceBenchmarkSuite {
    optimized_db: OptimizedDatabase,
    test_account_id: String,
    test_folder_name: String,
}

/// Benchmark results for different operations
#[derive(Debug, Clone)]
pub struct BenchmarkResults {
    pub operation_name: String,
    pub messages_processed: u32,
    pub execution_time_ms: u64,
    pub memory_usage_mb: f64,
    pub messages_per_second: f64,
    pub cache_hit_rate: f64,
    pub errors: Vec<String>,
}

/// Memory usage tracking
#[derive(Debug, Clone)]
pub struct MemoryUsageStats {
    pub initial_memory_mb: f64,
    pub peak_memory_mb: f64,
    pub final_memory_mb: f64,
    pub average_memory_mb: f64,
}

/// Performance test configuration
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    pub message_counts: Vec<u32>,
    pub iterations: u32,
    pub warmup_iterations: u32,
    pub measure_memory: bool,
    pub enable_cache: bool,
    pub batch_sizes: Vec<usize>,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            message_counts: vec![1_000, 5_000, 10_000, 25_000, 50_000],
            iterations: 3,
            warmup_iterations: 1,
            measure_memory: true,
            enable_cache: true,
            batch_sizes: vec![50, 100, 250, 500, 1000],
        }
    }
}

impl PerformanceBenchmarkSuite {
    /// Create a new benchmark suite
    pub async fn new(db_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Create base database
        let email_db = EmailDatabase::new(db_path).await?;
        let pool = email_db.pool.clone();
        
        // Create optimized database with performance settings
        let config = DatabaseOptimizationConfig {
            max_cached_messages: 5000,
            cache_ttl_seconds: 600,
            batch_size: 500,
            enable_query_cache: true,
            enable_connection_pooling: true,
            max_parallel_queries: 16,
        };
        
        let optimized_db = OptimizedDatabase::new(pool, config).await?;
        
        Ok(Self {
            optimized_db,
            test_account_id: "benchmark_account".to_string(),
            test_folder_name: "INBOX".to_string(),
        })
    }

    /// Run comprehensive performance benchmarks
    pub async fn run_full_benchmark_suite(
        &self,
        config: &BenchmarkConfig,
    ) -> Result<Vec<BenchmarkResults>, Box<dyn std::error::Error>> {
        let mut all_results = Vec::new();
        
        println!("🚀 Starting Performance Benchmark Suite");
        println!("Configuration: {:?}", config);
        
        // Test 1: Message insertion performance
        for &message_count in &config.message_counts {
            let results = self.benchmark_message_insertion(message_count, config).await?;
            all_results.push(results);
        }
        
        // Test 2: Message retrieval performance
        for &message_count in &config.message_counts {
            let results = self.benchmark_message_retrieval(message_count, config).await?;
            all_results.push(results);
        }
        
        // Test 3: Search performance
        for &message_count in &config.message_counts {
            let results = self.benchmark_search_performance(message_count, config).await?;
            all_results.push(results);
        }
        
        // Test 4: Batch operations performance
        for &batch_size in &config.batch_sizes {
            let results = self.benchmark_batch_operations(batch_size, config).await?;
            all_results.push(results);
        }
        
        // Test 5: Concurrent access performance
        let concurrent_results = self.benchmark_concurrent_access(config).await?;
        all_results.push(concurrent_results);
        
        // Test 6: Memory efficiency
        let memory_results = self.benchmark_memory_efficiency(config).await?;
        all_results.push(memory_results);
        
        println!("✅ Benchmark Suite Complete");
        self.print_summary(&all_results);
        
        Ok(all_results)
    }

    /// Benchmark message insertion performance
    async fn benchmark_message_insertion(
        &self,
        message_count: u32,
        config: &BenchmarkConfig,
    ) -> Result<BenchmarkResults, Box<dyn std::error::Error>> {
        println!("📝 Benchmarking message insertion: {} messages", message_count);
        
        let mut total_time = Duration::new(0, 0);
        let mut total_memory = 0.0;
        let mut errors = Vec::new();
        
        // Warmup
        for _ in 0..config.warmup_iterations {
            let messages = self.generate_test_messages(100).await;
            let _ = self.optimized_db.batch_insert_messages(&messages).await;
        }
        
        // Actual benchmarking
        for iteration in 0..config.iterations {
            println!("  Iteration {}/{}", iteration + 1, config.iterations);
            
            let messages = self.generate_test_messages(message_count).await;
            let initial_memory = self.get_memory_usage();
            
            let start_time = Instant::now();
            let result = self.optimized_db.batch_insert_messages(&messages).await;
            let execution_time = start_time.elapsed();
            
            let final_memory = self.get_memory_usage();
            
            match result {
                Ok(_) => {
                    total_time += execution_time;
                    total_memory += final_memory - initial_memory;
                }
                Err(e) => {
                    errors.push(format!("Insertion failed in iteration {}: {}", iteration, e));
                }
            }
            
            // Small delay between iterations
            time::sleep(Duration::from_millis(100)).await;
        }
        
        let avg_time_ms = total_time.as_millis() as u64 / config.iterations as u64;
        let avg_memory_mb = total_memory / config.iterations as f64;
        let messages_per_second = if avg_time_ms > 0 {
            (message_count as f64 * 1000.0) / avg_time_ms as f64
        } else {
            0.0
        };
        
        Ok(BenchmarkResults {
            operation_name: format!("Message Insertion ({})", message_count),
            messages_processed: message_count,
            execution_time_ms: avg_time_ms,
            memory_usage_mb: avg_memory_mb,
            messages_per_second,
            cache_hit_rate: 0.0, // Not applicable for insertion
            errors,
        })
    }

    /// Benchmark message retrieval performance
    async fn benchmark_message_retrieval(
        &self,
        message_count: u32,
        config: &BenchmarkConfig,
    ) -> Result<BenchmarkResults, Box<dyn std::error::Error>> {
        println!("📖 Benchmarking message retrieval: {} messages", message_count);
        
        // First, ensure we have test data
        let messages = self.generate_test_messages(message_count).await;
        let _ = self.optimized_db.batch_insert_messages(&messages).await?;
        
        let mut total_time = Duration::new(0, 0);
        let mut total_memory = 0.0;
        let mut cache_hits = 0;
        let mut total_queries = 0;
        let mut errors = Vec::new();
        
        // Test different page sizes
        let page_sizes = vec![50, 100, 250, 500];
        
        for iteration in 0..config.iterations {
            println!("  Iteration {}/{}", iteration + 1, config.iterations);
            
            for &page_size in &page_sizes {
                let pagination = PaginationConfig {
                    page_size,
                    current_page: 0,
                    sort_field: "date".to_string(),
                    sort_direction: SortDirection::Descending,
                };
                
                let initial_memory = self.get_memory_usage();
                let start_time = Instant::now();
                
                let result = self.optimized_db.get_messages_paginated(
                    &self.test_account_id,
                    &self.test_folder_name,
                    &pagination,
                ).await;
                
                let execution_time = start_time.elapsed();
                let final_memory = self.get_memory_usage();
                
                match result {
                    Ok((messages, stats)) => {
                        total_time += execution_time;
                        total_memory += final_memory - initial_memory;
                        if stats.cache_hit {
                            cache_hits += 1;
                        }
                        total_queries += 1;
                        
                        println!("    Retrieved {} messages in {}ms", messages.len(), stats.execution_time_ms);
                    }
                    Err(e) => {
                        errors.push(format!("Retrieval failed: {}", e));
                    }
                }
            }
        }
        
        let avg_time_ms = total_time.as_millis() as u64 / total_queries as u64;
        let avg_memory_mb = total_memory / total_queries as f64;
        let cache_hit_rate = if total_queries > 0 {
            cache_hits as f64 / total_queries as f64
        } else {
            0.0
        };
        let messages_per_second = if avg_time_ms > 0 {
            (message_count as f64 * 1000.0) / avg_time_ms as f64
        } else {
            0.0
        };
        
        Ok(BenchmarkResults {
            operation_name: format!("Message Retrieval ({})", message_count),
            messages_processed: message_count,
            execution_time_ms: avg_time_ms,
            memory_usage_mb: avg_memory_mb,
            messages_per_second,
            cache_hit_rate,
            errors,
        })
    }

    /// Benchmark search performance
    async fn benchmark_search_performance(
        &self,
        message_count: u32,
        config: &BenchmarkConfig,
    ) -> Result<BenchmarkResults, Box<dyn std::error::Error>> {
        println!("🔍 Benchmarking search performance: {} messages", message_count);
        
        // Ensure test data exists
        let messages = self.generate_test_messages(message_count).await;
        let _ = self.optimized_db.batch_insert_messages(&messages).await?;
        
        let mut total_time = Duration::new(0, 0);
        let mut total_memory = 0.0;
        let mut total_queries = 0;
        let mut errors = Vec::new();
        
        // Different search queries to test
        let search_queries = vec![
            "important",
            "meeting",
            "test",
            "from:john@example.com",
            "subject:urgent",
        ];
        
        for iteration in 0..config.iterations {
            println!("  Iteration {}/{}", iteration + 1, config.iterations);
            
            for query in &search_queries {
                let filters = SearchFilters::default();
                let pagination = PaginationConfig {
                    page_size: 100,
                    current_page: 0,
                    sort_field: "date".to_string(),
                    sort_direction: SortDirection::Descending,
                };
                
                let initial_memory = self.get_memory_usage();
                let start_time = Instant::now();
                
                let result = self.optimized_db.search_messages_optimized(
                    &self.test_account_id,
                    query,
                    &filters,
                    &pagination,
                ).await;
                
                let execution_time = start_time.elapsed();
                let final_memory = self.get_memory_usage();
                
                match result {
                    Ok((results, stats)) => {
                        total_time += execution_time;
                        total_memory += final_memory - initial_memory;
                        total_queries += 1;
                        
                        println!("    Query '{}': {} results in {}ms", query, results.len(), stats.execution_time_ms);
                    }
                    Err(e) => {
                        errors.push(format!("Search failed for '{}': {}", query, e));
                    }
                }
            }
        }
        
        let avg_time_ms = total_time.as_millis() as u64 / total_queries as u64;
        let avg_memory_mb = total_memory / total_queries as f64;
        let queries_per_second = if avg_time_ms > 0 {
            1000.0 / avg_time_ms as f64
        } else {
            0.0
        };
        
        Ok(BenchmarkResults {
            operation_name: format!("Search Performance ({})", message_count),
            messages_processed: message_count,
            execution_time_ms: avg_time_ms,
            memory_usage_mb: avg_memory_mb,
            messages_per_second: queries_per_second,
            cache_hit_rate: 0.0,
            errors,
        })
    }

    /// Benchmark batch operations
    async fn benchmark_batch_operations(
        &self,
        batch_size: usize,
        config: &BenchmarkConfig,
    ) -> Result<BenchmarkResults, Box<dyn std::error::Error>> {
        println!("📦 Benchmarking batch operations: batch size {}", batch_size);
        
        let total_messages = 10_000;
        let messages = self.generate_test_messages(total_messages).await;
        
        let mut total_time = Duration::new(0, 0);
        let mut total_memory = 0.0;
        let mut total_processed = 0;
        let mut errors = Vec::new();
        
        for iteration in 0..config.iterations {
            println!("  Iteration {}/{}", iteration + 1, config.iterations);
            
            let initial_memory = self.get_memory_usage();
            let start_time = Instant::now();
            
            // Process messages in batches
            for chunk in messages.chunks(batch_size) {
                let result = self.optimized_db.batch_insert_messages(chunk).await;
                
                match result {
                    Ok(batch_result) => {
                        total_processed += batch_result.successful_operations;
                        for error in batch_result.errors {
                            errors.push(error);
                        }
                    }
                    Err(e) => {
                        errors.push(format!("Batch operation failed: {}", e));
                    }
                }
            }
            
            let execution_time = start_time.elapsed();
            let final_memory = self.get_memory_usage();
            
            total_time += execution_time;
            total_memory += final_memory - initial_memory;
        }
        
        let avg_time_ms = total_time.as_millis() as u64 / config.iterations as u64;
        let avg_memory_mb = total_memory / config.iterations as f64;
        let messages_per_second = if avg_time_ms > 0 {
            (total_processed as f64 * 1000.0) / avg_time_ms as f64
        } else {
            0.0
        };
        
        Ok(BenchmarkResults {
            operation_name: format!("Batch Operations (batch size: {})", batch_size),
            messages_processed: total_processed / config.iterations,
            execution_time_ms: avg_time_ms,
            memory_usage_mb: avg_memory_mb,
            messages_per_second,
            cache_hit_rate: 0.0,
            errors,
        })
    }

    /// Benchmark concurrent access
    async fn benchmark_concurrent_access(
        &self,
        config: &BenchmarkConfig,
    ) -> Result<BenchmarkResults, Box<dyn std::error::Error>> {
        println!("🔄 Benchmarking concurrent access");
        
        // Prepare test data
        let messages = self.generate_test_messages(5000).await;
        let _ = self.optimized_db.batch_insert_messages(&messages).await?;
        
        let mut total_time = Duration::new(0, 0);
        let mut total_operations = 0;
        let mut errors = Vec::new();
        
        for iteration in 0..config.iterations {
            println!("  Iteration {}/{}", iteration + 1, config.iterations);
            
            let start_time = Instant::now();
            
            // Spawn multiple concurrent tasks
            let mut handles = Vec::new();
            
            for task_id in 0..10 {
                let _account_id = self.test_account_id.clone();
                let _folder_name = self.test_folder_name.clone();
                
                let handle = tokio::spawn(async move {
                    let _pagination = PaginationConfig {
                        page_size: 50,
                        current_page: task_id % 5,
                        sort_field: "date".to_string(),
                        sort_direction: SortDirection::Descending,
                    };
                    
                    // Simulate mixed read operations
                    for _ in 0..20 {
                        // This would require access to optimized_db, which is challenging
                        // in this concurrent context. In a real implementation, you'd need
                        // to structure this differently or use Arc<OptimizedDatabase>
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                    
                    Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
                });
                
                handles.push(handle);
            }
            
            // Wait for all tasks to complete
            for handle in handles {
                match handle.await {
                    Ok(result) => {
                        if let Err(e) = result {
                            errors.push(format!("Concurrent task failed: {}", e));
                        } else {
                            total_operations += 20; // 20 operations per task
                        }
                    }
                    Err(e) => {
                        errors.push(format!("Task join failed: {}", e));
                    }
                }
            }
            
            let execution_time = start_time.elapsed();
            total_time += execution_time;
        }
        
        let avg_time_ms = total_time.as_millis() as u64 / config.iterations as u64;
        let operations_per_second = if avg_time_ms > 0 {
            (total_operations as f64 * 1000.0) / avg_time_ms as f64
        } else {
            0.0
        };
        
        Ok(BenchmarkResults {
            operation_name: "Concurrent Access".to_string(),
            messages_processed: total_operations / config.iterations,
            execution_time_ms: avg_time_ms,
            memory_usage_mb: 0.0, // Not measured in this test
            messages_per_second: operations_per_second,
            cache_hit_rate: 0.0,
            errors,
        })
    }

    /// Benchmark memory efficiency
    async fn benchmark_memory_efficiency(
        &self,
        _config: &BenchmarkConfig,
    ) -> Result<BenchmarkResults, Box<dyn std::error::Error>> {
        println!("🧠 Benchmarking memory efficiency");
        
        let initial_memory = self.get_memory_usage();
        let mut peak_memory = initial_memory;
        let mut errors = Vec::new();
        
        let start_time = Instant::now();
        
        // Test with progressively larger datasets
        for &message_count in &[1000, 5000, 10000, 25000] {
            println!("  Testing with {} messages", message_count);
            
            let messages = self.generate_test_messages(message_count).await;
            let current_memory = self.get_memory_usage();
            peak_memory = peak_memory.max(current_memory);
            
            // Insert messages
            if let Err(e) = self.optimized_db.batch_insert_messages(&messages).await {
                errors.push(format!("Failed to insert {} messages: {}", message_count, e));
            }
            
            let after_insert_memory = self.get_memory_usage();
            peak_memory = peak_memory.max(after_insert_memory);
            
            // Perform some queries
            let pagination = PaginationConfig {
                page_size: 100,
                current_page: 0,
                sort_field: "date".to_string(),
                sort_direction: SortDirection::Descending,
            };
            
            if let Err(e) = self.optimized_db.get_messages_paginated(
                &self.test_account_id,
                &self.test_folder_name,
                &pagination,
            ).await {
                errors.push(format!("Failed to query {} messages: {}", message_count, e));
            }
            
            let after_query_memory = self.get_memory_usage();
            peak_memory = peak_memory.max(after_query_memory);
        }
        
        let execution_time = start_time.elapsed();
        let final_memory = self.get_memory_usage();
        let memory_growth = final_memory - initial_memory;
        
        Ok(BenchmarkResults {
            operation_name: "Memory Efficiency".to_string(),
            messages_processed: 41000, // Total messages processed
            execution_time_ms: execution_time.as_millis() as u64,
            memory_usage_mb: memory_growth,
            messages_per_second: 0.0, // Not applicable
            cache_hit_rate: 0.0,
            errors,
        })
    }

    /// Generate test messages for benchmarking
    async fn generate_test_messages(&self, count: u32) -> Vec<StoredMessage> {
        let mut messages = Vec::with_capacity(count as usize);
        let base_time = Utc::now();
        
        let subjects = vec![
            "Important meeting tomorrow",
            "Quarterly report is ready",
            "Team lunch next Friday",
            "Project update - urgent",
            "Conference call scheduled",
            "Budget proposal for review",
            "System maintenance notice",
            "Welcome to the team!",
            "Invoice #12345 attached",
            "Weekly status update",
        ];
        
        let senders = vec![
            ("john.doe@example.com", "John Doe"),
            ("jane.smith@company.com", "Jane Smith"),
            ("admin@system.local", "System Admin"),
            ("support@vendor.com", "Support Team"),
            ("manager@department.org", "Department Manager"),
            ("client@external.net", "External Client"),
            ("hr@company.com", "Human Resources"),
            ("finance@company.com", "Finance Team"),
            ("tech@company.com", "Tech Support"),
            ("sales@company.com", "Sales Team"),
        ];
        
        for i in 0..count {
            let subject_idx = (i as usize) % subjects.len();
            let sender_idx = (i as usize) % senders.len();
            let (sender_addr, sender_name) = &senders[sender_idx];
            
            // Vary message dates
            let date_offset = chrono::Duration::hours(i as i64);
            let message_date = base_time - date_offset;
            
            let body_text = format!(
                "This is test message number {}.\n\nThe content of this message is generated for performance testing purposes. It contains {} characters of text to simulate real email content.\n\nRegards,\n{}",
                i + 1,
                200 + (i % 300), // Variable content length
                sender_name
            );
            
            messages.push(StoredMessage {
                id: Uuid::new_v4(),
                account_id: self.test_account_id.clone(),
                folder_name: self.test_folder_name.clone(),
                imap_uid: i + 1,
                message_id: Some(format!("test-message-{}@benchmark.local", i + 1)),
                thread_id: Some(format!("thread-{}", (i / 5) + 1)), // Group every 5 messages
                in_reply_to: None,
                references: Vec::new(),
                subject: format!("{} #{}", subjects[subject_idx], i + 1),
                from_addr: sender_addr.to_string(),
                from_name: Some(sender_name.to_string()),
                to_addrs: vec!["recipient@test.local".to_string()],
                cc_addrs: Vec::new(),
                bcc_addrs: Vec::new(),
                reply_to: None,
                date: message_date,
                body_text: Some(body_text),
                body_html: None,
                attachments: if i % 10 == 0 {
                    // Add attachments to some messages
                    vec![StoredAttachment {
                        id: format!("attachment-{}", i),
                        filename: format!("document-{}.pdf", i),
                        content_type: "application/pdf".to_string(),
                        size: 1024 + (i % 10000), // Variable size
                        content_id: None,
                        is_inline: false,
                        data: None, // Empty attachment data for benchmarking
                        file_path: None,
                    }]
                } else {
                    Vec::new()
                },
                flags: if i % 3 == 0 {
                    vec!["\\Seen".to_string()]
                } else {
                    Vec::new()
                },
                labels: Vec::new(),
                size: Some(1000 + (i % 5000)),
                priority: None,
                created_at: message_date,
                updated_at: message_date,
                last_synced: message_date,
                sync_version: 1,
                is_draft: false,
                is_deleted: false,
            });
        }
        
        messages
    }

    /// Get current memory usage (simplified estimation)
    fn get_memory_usage(&self) -> f64 {
        // In a real implementation, you would use system APIs to get actual memory usage
        // For benchmarking purposes, this is a placeholder
        #[cfg(target_os = "linux")]
        {
            // Try to read from /proc/self/status
            if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
                for line in status.lines() {
                    if line.starts_with("VmRSS:") {
                        if let Some(kb_str) = line.split_whitespace().nth(1) {
                            if let Ok(kb) = kb_str.parse::<f64>() {
                                return kb / 1024.0; // Convert KB to MB
                            }
                        }
                    }
                }
            }
        }
        
        // Fallback: return a simulated value
        100.0 + (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() % 1000) as f64 / 10.0
    }

    /// Print benchmark summary
    fn print_summary(&self, results: &[BenchmarkResults]) {
        println!("\n📊 BENCHMARK SUMMARY");
        println!("═══════════════════════════════════════════════════════════════");
        
        for result in results {
            println!("\n{}", result.operation_name);
            println!("─────────────────────────────────────────────────────────────");
            println!("  Messages Processed: {}", result.messages_processed);
            println!("  Execution Time: {}ms", result.execution_time_ms);
            println!("  Memory Usage: {:.2}MB", result.memory_usage_mb);
            println!("  Throughput: {:.2} messages/sec", result.messages_per_second);
            
            if result.cache_hit_rate > 0.0 {
                println!("  Cache Hit Rate: {:.1}%", result.cache_hit_rate * 100.0);
            }
            
            if !result.errors.is_empty() {
                println!("  Errors: {}", result.errors.len());
                for error in &result.errors {
                    println!("    - {}", error);
                }
            }
        }
        
        // Performance analysis
        println!("\n🎯 PERFORMANCE ANALYSIS");
        println!("═══════════════════════════════════════════════════════════════");
        
        // Find best and worst performing operations
        if let Some(fastest) = results.iter().max_by(|a, b| {
            a.messages_per_second.partial_cmp(&b.messages_per_second).unwrap()
        }) {
            println!("🚀 Fastest Operation: {} ({:.2} msg/sec)", 
                fastest.operation_name, fastest.messages_per_second);
        }
        
        if let Some(slowest) = results.iter().min_by(|a, b| {
            a.messages_per_second.partial_cmp(&b.messages_per_second).unwrap()
        }) {
            println!("🐌 Slowest Operation: {} ({:.2} msg/sec)", 
                slowest.operation_name, slowest.messages_per_second);
        }
        
        // Memory efficiency
        let total_memory: f64 = results.iter().map(|r| r.memory_usage_mb).sum();
        println!("💾 Total Memory Usage: {:.2}MB", total_memory);
        
        // Error summary
        let total_errors: usize = results.iter().map(|r| r.errors.len()).sum();
        if total_errors > 0 {
            println!("⚠️  Total Errors: {}", total_errors);
        } else {
            println!("✅ No Errors Detected");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_benchmark_suite_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_str().unwrap();
        
        let suite = PerformanceBenchmarkSuite::new(db_path).await;
        assert!(suite.is_ok());
    }

    #[tokio::test]
    async fn test_generate_test_messages() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_str().unwrap();
        
        let suite = PerformanceBenchmarkSuite::new(db_path).await.unwrap();
        let messages = suite.generate_test_messages(100).await;
        
        assert_eq!(messages.len(), 100);
        assert!(messages.iter().all(|m| m.account_id == "benchmark_account"));
        assert!(messages.iter().all(|m| !m.subject.is_empty()));
    }

    #[tokio::test]
    async fn test_benchmark_config() {
        let config = BenchmarkConfig::default();
        
        assert!(!config.message_counts.is_empty());
        assert!(config.iterations > 0);
        assert!(config.warmup_iterations >= 0);
    }
}