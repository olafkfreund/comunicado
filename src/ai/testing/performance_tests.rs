//! Performance testing and benchmarking for AI functionality

use crate::ai::{
    config::{AIConfig, AIProviderType},
    testing::{mock_providers::*, AITestConfigBuilder},
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

/// Performance benchmark results
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    /// Benchmark name
    pub name: String,
    /// Total test duration
    pub total_duration: Duration,
    /// Number of operations performed
    pub operations: usize,
    /// Operations per second
    pub ops_per_second: f64,
    /// Average latency per operation
    pub avg_latency: Duration,
    /// Minimum latency observed
    pub min_latency: Duration,
    /// Maximum latency observed
    pub max_latency: Duration,
    /// 95th percentile latency
    pub p95_latency: Duration,
    /// 99th percentile latency
    pub p99_latency: Duration,
    /// Error rate (0.0 to 1.0)
    pub error_rate: f64,
    /// Memory usage statistics
    pub memory_stats: MemoryStats,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Memory usage statistics
#[derive(Debug, Clone, Default)]
pub struct MemoryStats {
    /// Peak memory usage in bytes
    pub peak_memory_bytes: usize,
    /// Average memory usage in bytes
    pub avg_memory_bytes: usize,
    /// Memory allocations count
    pub allocations: usize,
}

/// Performance benchmark runner for AI operations
pub struct AIPerformanceBenchmark {
    /// Test configurations
    configs: Vec<BenchmarkConfig>,
    /// Concurrency level for load testing
    concurrency: usize,
    /// Warmup duration before measurements
    warmup_duration: Duration,
}

/// Configuration for a specific benchmark
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    /// Benchmark name
    pub name: String,
    /// Provider to test
    pub provider_type: AIProviderType,
    /// Number of operations to perform
    pub operations: usize,
    /// Concurrent requests
    pub concurrency: usize,
    /// Test prompts to use
    pub prompts: Vec<String>,
    /// Expected operation duration range
    pub expected_duration_range: Option<(Duration, Duration)>,
}

/// Individual operation result for analysis
#[derive(Debug, Clone)]
struct OperationResult {
    /// Operation start time
    start_time: Instant,
    /// Operation duration
    duration: Duration,
    /// Whether the operation succeeded
    success: bool,
    /// Error message if failed
    error: Option<String>,
    /// Response size in bytes
    response_size: usize,
}

impl AIPerformanceBenchmark {
    /// Create a new performance benchmark runner
    pub fn new() -> Self {
        Self {
            configs: Vec::new(),
            concurrency: 1,
            warmup_duration: Duration::from_secs(5),
        }
    }

    /// Set global concurrency level
    pub fn with_concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency;
        self
    }

    /// Set warmup duration
    pub fn with_warmup(mut self, duration: Duration) -> Self {
        self.warmup_duration = duration;
        self
    }

    /// Add a benchmark configuration
    pub fn add_benchmark(mut self, config: BenchmarkConfig) -> Self {
        self.configs.push(config);
        self
    }

    /// Run all configured benchmarks
    pub async fn run_all_benchmarks(&self) -> Vec<BenchmarkResult> {
        let mut results = Vec::new();
        
        for config in &self.configs {
            let result = self.run_benchmark(config).await;
            results.push(result);
        }
        
        results
    }

    /// Run a single benchmark
    pub async fn run_benchmark(&self, config: &BenchmarkConfig) -> BenchmarkResult {
        println!("Running benchmark: {}", config.name);
        
        // Create provider for testing
        let provider = self.create_test_provider(&config.provider_type).await;
        
        // Warmup phase
        if !self.warmup_duration.is_zero() {
            self.warmup_phase(&provider, config).await;
        }
        
        // Main benchmark phase
        let start_time = Instant::now();
        let operation_results = self.execute_operations(&provider, config).await;
        let total_duration = start_time.elapsed();
        
        // Analyze results
        self.analyze_results(config, operation_results, total_duration)
    }

    /// Create a test provider based on type
    async fn create_test_provider(&self, provider_type: &AIProviderType) -> Arc<MockAIProvider> {
        match provider_type {
            AIProviderType::Ollama => Arc::new(MockProviderFactory::fast_provider()),
            AIProviderType::OpenAI => Arc::new(MockProviderFactory::email_specialized_provider()),
            AIProviderType::Anthropic => Arc::new(MockProviderFactory::calendar_specialized_provider()),
            AIProviderType::Google => Arc::new(MockProviderFactory::slow_provider()),
            AIProviderType::None => Arc::new(MockProviderFactory::unavailable_provider()),
        }
    }

    /// Warmup phase to stabilize performance
    async fn warmup_phase(&self, provider: &Arc<MockAIProvider>, config: &BenchmarkConfig) {
        println!("Warming up for {:?}...", self.warmup_duration);
        
        let warmup_end = Instant::now() + self.warmup_duration;
        let mut warmup_operations = 0;
        
        while Instant::now() < warmup_end {
            let prompt = &config.prompts[warmup_operations % config.prompts.len()];
            let _ = provider.complete_text(prompt, None).await;
            warmup_operations += 1;
        }
        
        println!("Warmup completed with {} operations", warmup_operations);
    }

    /// Execute benchmark operations
    async fn execute_operations(
        &self,
        provider: &Arc<MockAIProvider>,
        config: &BenchmarkConfig,
    ) -> Vec<OperationResult> {
        let semaphore = Arc::new(Semaphore::new(config.concurrency));
        let mut handles = Vec::new();
        
        for i in 0..config.operations {
            let provider_clone = Arc::clone(provider);
            let semaphore_clone = Arc::clone(&semaphore);
            let prompt = config.prompts[i % config.prompts.len()].clone();
            
            let handle = tokio::spawn(async move {
                let _permit = semaphore_clone.acquire().await.unwrap();
                
                let start_time = Instant::now();
                let result = provider_clone.complete_text(&prompt, None).await;
                let duration = start_time.elapsed();
                
                OperationResult {
                    start_time,
                    duration,
                    success: result.is_ok(),
                    error: result.err().map(|e| e.to_string()),
                    response_size: result.as_ref().map(|s| s.len()).unwrap_or(0),
                }
            });
            
            handles.push(handle);
        }
        
        // Collect all results
        let mut results = Vec::new();
        for handle in handles {
            if let Ok(result) = handle.await {
                results.push(result);
            }
        }
        
        results
    }

    /// Analyze operation results and create benchmark result
    fn analyze_results(
        &self,
        config: &BenchmarkConfig,
        operation_results: Vec<OperationResult>,
        total_duration: Duration,
    ) -> BenchmarkResult {
        let successful_operations: Vec<_> = operation_results
            .iter()
            .filter(|r| r.success)
            .collect();
        
        let operations = operation_results.len();
        let successful_ops = successful_operations.len();
        let error_rate = if operations > 0 {
            (operations - successful_ops) as f64 / operations as f64
        } else {
            0.0
        };

        // Calculate latency statistics
        let mut durations: Vec<Duration> = successful_operations
            .iter()
            .map(|r| r.duration)
            .collect();
        durations.sort();

        let avg_latency = if !durations.is_empty() {
            durations.iter().sum::<Duration>() / durations.len() as u32
        } else {
            Duration::ZERO
        };

        let min_latency = durations.first().copied().unwrap_or(Duration::ZERO);
        let max_latency = durations.last().copied().unwrap_or(Duration::ZERO);
        
        let p95_latency = if !durations.is_empty() {
            let index = (durations.len() as f64 * 0.95) as usize;
            durations.get(index.min(durations.len() - 1)).copied().unwrap_or(Duration::ZERO)
        } else {
            Duration::ZERO
        };
        
        let p99_latency = if !durations.is_empty() {
            let index = (durations.len() as f64 * 0.99) as usize;
            durations.get(index.min(durations.len() - 1)).copied().unwrap_or(Duration::ZERO)
        } else {
            Duration::ZERO
        };

        // Calculate throughput
        let ops_per_second = if total_duration.as_secs_f64() > 0.0 {
            successful_ops as f64 / total_duration.as_secs_f64()
        } else {
            0.0
        };

        // Memory stats (simplified for testing)
        let memory_stats = MemoryStats {
            peak_memory_bytes: successful_ops * 1024, // Simulated
            avg_memory_bytes: successful_ops * 512,   // Simulated
            allocations: successful_ops * 10,         // Simulated
        };

        let mut metadata = HashMap::new();
        metadata.insert("concurrency".to_string(), config.concurrency.to_string());
        metadata.insert("prompts_count".to_string(), config.prompts.len().to_string());
        metadata.insert("successful_ops".to_string(), successful_ops.to_string());
        metadata.insert("failed_ops".to_string(), (operations - successful_ops).to_string());

        BenchmarkResult {
            name: config.name.clone(),
            total_duration,
            operations,
            ops_per_second,
            avg_latency,
            min_latency,
            max_latency,
            p95_latency,
            p99_latency,
            error_rate,
            memory_stats,
            metadata,
        }
    }

    /// Create a standard set of benchmarks
    pub fn create_standard_benchmarks() -> Self {
        Self::new()
            .with_concurrency(10)
            .with_warmup(Duration::from_secs(2))
            .add_benchmark(BenchmarkConfig {
                name: "Fast Provider - Light Load".to_string(),
                provider_type: AIProviderType::Ollama,
                operations: 100,
                concurrency: 5,
                prompts: vec![
                    "Hello".to_string(),
                    "How are you?".to_string(),
                    "What's the weather?".to_string(),
                ],
                expected_duration_range: Some((Duration::from_millis(10), Duration::from_millis(100))),
            })
            .add_benchmark(BenchmarkConfig {
                name: "Email Provider - Medium Load".to_string(),
                provider_type: AIProviderType::OpenAI,
                operations: 50,
                concurrency: 3,
                prompts: vec![
                    "Compose a professional email".to_string(),
                    "Summarize this email content".to_string(),
                    "Reply to this message".to_string(),
                ],
                expected_duration_range: Some((Duration::from_millis(200), Duration::from_millis(500))),
            })
            .add_benchmark(BenchmarkConfig {
                name: "Calendar Provider - Complex Operations".to_string(),
                provider_type: AIProviderType::Anthropic,
                operations: 30,
                concurrency: 2,
                prompts: vec![
                    "Schedule a meeting for next week".to_string(),
                    "Find optimal meeting times for the team".to_string(),
                    "Analyze my calendar patterns".to_string(),
                ],
                expected_duration_range: Some((Duration::from_millis(300), Duration::from_millis(800))),
            })
            .add_benchmark(BenchmarkConfig {
                name: "Slow Provider - Stress Test".to_string(),
                provider_type: AIProviderType::Google,
                operations: 20,
                concurrency: 1,
                prompts: vec![
                    "Complex analysis request that takes time".to_string(),
                ],
                expected_duration_range: Some((Duration::from_secs(1), Duration::from_secs(3))),
            })
    }

    /// Generate performance report
    pub fn generate_performance_report(&self, results: &[BenchmarkResult]) -> String {
        let mut report = String::new();
        
        report.push_str("# AI Performance Benchmark Report\n\n");
        report.push_str(&format!("Report generated: {}\n\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));

        // Summary table
        report.push_str("## Performance Summary\n\n");
        report.push_str("| Benchmark | Operations | Duration | Ops/sec | Avg Latency | P95 Latency | Error Rate |\n");
        report.push_str("|-----------|------------|----------|---------|-------------|-------------|------------|\n");
        
        for result in results {
            report.push_str(&format!(
                "| {} | {} | {:?} | {:.2} | {:?} | {:?} | {:.1}% |\n",
                result.name,
                result.operations,
                result.total_duration,
                result.ops_per_second,
                result.avg_latency,
                result.p95_latency,
                result.error_rate * 100.0
            ));
        }
        
        report.push_str("\n");

        // Detailed results
        for result in results {
            report.push_str(&format!("## {}\n\n", result.name));
            report.push_str(&format!("- **Total Operations:** {}\n", result.operations));
            report.push_str(&format!("- **Total Duration:** {:?}\n", result.total_duration));
            report.push_str(&format!("- **Throughput:** {:.2} ops/sec\n", result.ops_per_second));
            report.push_str(&format!("- **Error Rate:** {:.1}%\n", result.error_rate * 100.0));
            report.push_str("\n**Latency Statistics:**\n");
            report.push_str(&format!("- Average: {:?}\n", result.avg_latency));
            report.push_str(&format!("- Minimum: {:?}\n", result.min_latency));
            report.push_str(&format!("- Maximum: {:?}\n", result.max_latency));
            report.push_str(&format!("- 95th Percentile: {:?}\n", result.p95_latency));
            report.push_str(&format!("- 99th Percentile: {:?}\n", result.p99_latency));
            report.push_str("\n**Memory Usage:**\n");
            report.push_str(&format!("- Peak Memory: {} KB\n", result.memory_stats.peak_memory_bytes / 1024));
            report.push_str(&format!("- Average Memory: {} KB\n", result.memory_stats.avg_memory_bytes / 1024));
            report.push_str(&format!("- Allocations: {}\n", result.memory_stats.allocations));
            
            if !result.metadata.is_empty() {
                report.push_str("\n**Metadata:**\n");
                for (key, value) in &result.metadata {
                    report.push_str(&format!("- {}: {}\n", key, value));
                }
            }
            
            report.push_str("\n");
        }

        // Performance analysis
        report.push_str("## Performance Analysis\n\n");
        
        let fastest = results.iter().max_by(|a, b| a.ops_per_second.partial_cmp(&b.ops_per_second).unwrap());
        let slowest = results.iter().min_by(|a, b| a.ops_per_second.partial_cmp(&b.ops_per_second).unwrap());
        
        if let (Some(fastest), Some(slowest)) = (fastest, slowest) {
            report.push_str(&format!("- **Fastest Provider:** {} ({:.2} ops/sec)\n", fastest.name, fastest.ops_per_second));
            report.push_str(&format!("- **Slowest Provider:** {} ({:.2} ops/sec)\n", slowest.name, slowest.ops_per_second));
        }
        
        let avg_error_rate = results.iter().map(|r| r.error_rate).sum::<f64>() / results.len() as f64;
        report.push_str(&format!("- **Average Error Rate:** {:.1}%\n", avg_error_rate * 100.0));
        
        let total_operations: usize = results.iter().map(|r| r.operations).sum();
        let total_duration = results.iter().map(|r| r.total_duration).max().unwrap_or(Duration::ZERO);
        report.push_str(&format!("- **Total Operations Tested:** {}\n", total_operations));
        report.push_str(&format!("- **Total Test Duration:** {:?}\n", total_duration));

        report
    }
}

impl Default for AIPerformanceBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_benchmark_creation() {
        let benchmark = AIPerformanceBenchmark::create_standard_benchmarks();
        assert!(!benchmark.configs.is_empty(), "Should have benchmark configurations");
    }

    #[tokio::test]
    async fn test_single_benchmark_execution() {
        let benchmark = AIPerformanceBenchmark::new()
            .with_warmup(Duration::from_millis(100));
        
        let config = BenchmarkConfig {
            name: "Test Benchmark".to_string(),
            provider_type: AIProviderType::Ollama,
            operations: 5,
            concurrency: 1,
            prompts: vec!["test prompt".to_string()],
            expected_duration_range: None,
        };
        
        let result = benchmark.run_benchmark(&config).await;
        
        assert_eq!(result.name, "Test Benchmark");
        assert_eq!(result.operations, 5);
        assert!(result.total_duration > Duration::ZERO);
    }

    #[tokio::test]
    async fn test_concurrent_benchmark() {
        let benchmark = AIPerformanceBenchmark::new();
        
        let config = BenchmarkConfig {
            name: "Concurrent Test".to_string(),
            provider_type: AIProviderType::Ollama,
            operations: 10,
            concurrency: 3,
            prompts: vec![
                "prompt 1".to_string(),
                "prompt 2".to_string(),
                "prompt 3".to_string(),
            ],
            expected_duration_range: None,
        };
        
        let result = benchmark.run_benchmark(&config).await;
        
        assert_eq!(result.operations, 10);
        assert!(result.ops_per_second > 0.0, "Should have positive throughput");
    }

    #[tokio::test]
    async fn test_performance_report_generation() {
        let benchmark = AIPerformanceBenchmark::new();
        
        let results = vec![
            BenchmarkResult {
                name: "Test 1".to_string(),
                total_duration: Duration::from_secs(1),
                operations: 10,
                ops_per_second: 10.0,
                avg_latency: Duration::from_millis(100),
                min_latency: Duration::from_millis(50),
                max_latency: Duration::from_millis(200),
                p95_latency: Duration::from_millis(180),
                p99_latency: Duration::from_millis(190),
                error_rate: 0.1,
                memory_stats: MemoryStats::default(),
                metadata: HashMap::new(),
            }
        ];
        
        let report = benchmark.generate_performance_report(&results);
        
        assert!(report.contains("Performance Summary"));
        assert!(report.contains("Test 1"));
        assert!(report.contains("10.0"));
    }
}