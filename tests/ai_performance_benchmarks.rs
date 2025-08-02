//! AI Performance Benchmarks
//! 
//! Comprehensive performance testing for AI operations including latency,
//! throughput, memory usage, and scalability benchmarks.

use comunicado::ai::{
    AIService, AIConfig, AIProviderType, PrivacyMode,
    EmailTriageConfig, SmartComposeConfig,
    cache::AIResponseCache,
    provider::AIProviderManager,
    background::{AIBackgroundProcessor, BackgroundConfig, AIOperationType},
    streaming::AIStreamingManager,
};
use comunicado::email::StoredMessage;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::Utc;

/// Performance benchmark configuration
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    /// Number of iterations for each benchmark
    pub iterations: usize,
    /// Concurrency level for stress tests
    pub concurrency: usize,
    /// Warm-up iterations before measuring
    pub warmup_iterations: usize,
    /// Maximum duration for any single operation
    pub operation_timeout: Duration,
    /// Whether to measure memory usage
    pub measure_memory: bool,
    /// Target percentiles for latency analysis
    pub latency_percentiles: Vec<f64>,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            iterations: 100,
            concurrency: 10,
            warmup_iterations: 10,
            operation_timeout: Duration::from_secs(30),
            measure_memory: true,
            latency_percentiles: vec![50.0, 90.0, 95.0, 99.0],
        }
    }
}

/// Performance metrics for a specific operation
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    /// Operation name
    pub operation: String,
    /// Provider type tested
    pub provider: AIProviderType,
    /// Total operations performed
    pub operations: usize,
    /// Total duration including setup/teardown
    pub total_duration: Duration,
    /// Net operation time (excluding setup)
    pub net_duration: Duration,
    /// Operations per second
    pub ops_per_second: f64,
    /// Latency statistics
    pub latency_stats: LatencyStats,
    /// Memory usage statistics
    pub memory_stats: MemoryStats,
    /// Error rate (0.0 to 1.0)
    pub error_rate: f64,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Latency statistics
#[derive(Debug, Clone)]
pub struct LatencyStats {
    /// Minimum latency observed
    pub min: Duration,
    /// Maximum latency observed
    pub max: Duration,
    /// Average latency
    pub mean: Duration,
    /// Median latency
    pub median: Duration,
    /// Latency percentiles (P50, P90, P95, P99)
    pub percentiles: HashMap<f64, Duration>,
    /// Standard deviation
    pub std_dev: Duration,
}

/// Memory usage statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    /// Initial memory usage (bytes)
    pub initial_memory_bytes: usize,
    /// Peak memory usage (bytes)
    pub peak_memory_bytes: usize,
    /// Final memory usage (bytes)
    pub final_memory_bytes: usize,
    /// Memory allocated during benchmark (bytes)
    pub allocated_bytes: usize,
    /// Memory growth rate (bytes per operation)
    pub growth_rate: f64,
}

/// AI performance benchmark runner
pub struct AIPerformanceBenchmark {
    config: BenchmarkConfig,
    services: HashMap<AIProviderType, Arc<AIService>>,
    test_data: BenchmarkTestData,
}

/// Test data for performance benchmarks
#[derive(Debug, Clone)]
pub struct BenchmarkTestData {
    /// Sample emails of varying sizes
    pub emails: Vec<StoredMessage>,
    /// Test prompts of varying complexity
    pub prompts: Vec<String>,
    /// Calendar text samples
    pub calendar_texts: Vec<String>,
}

impl BenchmarkTestData {
    pub fn new() -> Self {
        Self {
            emails: Self::generate_benchmark_emails(),
            prompts: Self::generate_benchmark_prompts(),
            calendar_texts: Self::generate_calendar_texts(),
        }
    }

    fn generate_benchmark_emails() -> Vec<StoredMessage> {
        let now = Utc::now();
        let mut emails = Vec::new();

        // Small email (< 1KB)
        emails.push(StoredMessage {
            id: Uuid::new_v4(),
            account_id: "bench_account".to_string(),
            folder_name: "INBOX".to_string(),
            imap_uid: 1,
            message_id: Some("small@test.com".to_string()),
            thread_id: None,
            in_reply_to: None,
            references: Vec::new(),
            subject: "Quick question".to_string(),
            from_addr: "user@test.com".to_string(),
            from_name: Some("Test User".to_string()),
            to_addrs: vec!["bench@test.com".to_string()],
            cc_addrs: Vec::new(),
            bcc_addrs: Vec::new(),
            reply_to: None,
            date: now,
            body_text: Some("Quick question about the meeting tomorrow.".to_string()),
            body_html: None,
            attachments: Vec::new(),
            flags: Vec::new(),
            labels: Vec::new(),
            size: Some(256),
            priority: None,
            created_at: now,
            updated_at: now,
            last_synced: now,
            sync_version: 1,
            is_draft: false,
            is_deleted: false,
        });

        // Medium email (~5KB)
        let medium_body = "This is a detailed project update email. ".repeat(200);
        emails.push(StoredMessage {
            id: Uuid::new_v4(),
            account_id: "bench_account".to_string(),
            folder_name: "INBOX".to_string(),
            imap_uid: 2,
            message_id: Some("medium@test.com".to_string()),
            thread_id: None,
            in_reply_to: None,
            references: Vec::new(),
            subject: "Project Update - Q4 Progress Report".to_string(),
            from_addr: "manager@test.com".to_string(),
            from_name: Some("Project Manager".to_string()),
            to_addrs: vec!["bench@test.com".to_string()],
            cc_addrs: vec!["team@test.com".to_string()],
            bcc_addrs: Vec::new(),
            reply_to: None,
            date: now,
            body_text: Some(medium_body),
            body_html: None,
            attachments: Vec::new(),
            flags: Vec::new(),
            labels: Vec::new(),
            size: Some(5120),
            priority: None,
            created_at: now,
            updated_at: now,
            last_synced: now,
            sync_version: 1,
            is_draft: false,
            is_deleted: false,
        });

        // Large email (~20KB)
        let large_body = "This is a comprehensive quarterly report with detailed analysis, charts, and recommendations. ".repeat(800);
        emails.push(StoredMessage {
            id: Uuid::new_v4(),
            account_id: "bench_account".to_string(),
            folder_name: "INBOX".to_string(),
            imap_uid: 3,
            message_id: Some("large@test.com".to_string()),
            thread_id: None,
            in_reply_to: None,
            references: Vec::new(),
            subject: "Q4 Comprehensive Analysis Report - Detailed Review".to_string(),
            from_addr: "analyst@test.com".to_string(),
            from_name: Some("Senior Analyst".to_string()),
            to_addrs: vec!["bench@test.com".to_string()],
            cc_addrs: vec!["executives@test.com".to_string()],
            bcc_addrs: Vec::new(),
            reply_to: None,
            date: now,
            body_text: Some(large_body),
            body_html: None,
            attachments: Vec::new(),
            flags: Vec::new(),
            labels: Vec::new(),
            size: Some(20480),
            priority: Some("High".to_string()),
            created_at: now,
            updated_at: now,
            last_synced: now,
            sync_version: 1,
            is_draft: false,
            is_deleted: false,
        });

        emails
    }

    fn generate_benchmark_prompts() -> Vec<String> {
        vec![
            // Simple prompts
            "Hello".to_string(),
            "What is AI?".to_string(),
            "Summarize this email.".to_string(),
            
            // Medium complexity prompts
            "Provide a detailed analysis of the email content and suggest appropriate actions.".to_string(),
            "Generate a professional reply to this business email with proper tone and structure.".to_string(),
            "Extract all dates, times, and action items from this message.".to_string(),
            
            // Complex prompts
            "Analyze this email thread, identify the key decision points, summarize the discussion outcomes, and provide recommendations for next steps including timeline and stakeholder responsibilities.".to_string(),
            "Create a comprehensive meeting agenda based on this email discussion, including time allocations, participant assignments, preparation requirements, and success metrics for each agenda item.".to_string(),
            
            // Very long prompt
            "Analyze the following business communication comprehensively: ".to_string() + &"Consider context, tone, urgency, stakeholders, implications, and recommended actions. ".repeat(100),
        ]
    }

    fn generate_calendar_texts() -> Vec<String> {
        vec![
            "Meeting tomorrow at 2 PM".to_string(),
            "Let's schedule a team review for next Friday at 10 AM in conference room B".to_string(),
            "Please set up a quarterly planning session for the first week of December, inviting all department heads and project managers".to_string(),
        ]
    }
}

impl AIPerformanceBenchmark {
    /// Create a new performance benchmark runner
    pub async fn new(config: BenchmarkConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let mut services = HashMap::new();
        let test_data = BenchmarkTestData::new();

        // Initialize Ollama service for benchmarking (most reliable for consistent testing)
        let ollama_service = Self::create_ai_service(AIProviderType::Ollama).await?;
        services.insert(AIProviderType::Ollama, ollama_service);

        // Add other providers if available
        for provider_type in [AIProviderType::OpenAI, AIProviderType::Anthropic, AIProviderType::Google] {
            if let Ok(service) = Self::create_ai_service(provider_type.clone()).await {
                services.insert(provider_type, service);
            }
        }

        Ok(Self {
            config,
            services,
            test_data,
        })
    }

    /// Create AI service for benchmarking
    async fn create_ai_service(provider_type: AIProviderType) -> Result<Arc<AIService>, Box<dyn std::error::Error>> {
        let config = Arc::new(RwLock::new(AIConfig {
            enabled: true,
            provider: provider_type,
            email_triage_enabled: true,
            privacy_mode: PrivacyMode::Balanced,
            ..Default::default()
        }));

        let cache = Arc::new(AIResponseCache::new(1000, Duration::from_secs(3600)));
        let provider_manager = Arc::new(RwLock::new(AIProviderManager::new(config.clone())));

        Ok(Arc::new(AIService::new(provider_manager, cache, config)))
    }

    /// Run comprehensive performance benchmarks
    pub async fn run_all_benchmarks(&self) -> Vec<PerformanceMetrics> {
        println!("🚀 Starting AI Performance Benchmarks");
        let mut all_metrics = Vec::new();

        for (provider_type, service) in &self.services {
            println!("📊 Benchmarking {:?} provider...", provider_type);

            // Text generation benchmarks
            all_metrics.extend(self.benchmark_text_generation(provider_type, service).await);

            // Email processing benchmarks
            all_metrics.extend(self.benchmark_email_processing(provider_type, service).await);

            // Triage system benchmarks
            all_metrics.extend(self.benchmark_triage_system(provider_type, service).await);

            // Concurrent operation benchmarks
            all_metrics.extend(self.benchmark_concurrent_operations(provider_type, service).await);

            // Cache performance benchmarks
            all_metrics.extend(self.benchmark_cache_performance(provider_type, service).await);

            // Memory usage benchmarks
            all_metrics.extend(self.benchmark_memory_usage(provider_type, service).await);
        }

        println!("✅ Performance benchmarks completed");
        all_metrics
    }

    /// Benchmark text generation performance
    async fn benchmark_text_generation(&self, provider_type: &AIProviderType, service: &Arc<AIService>) -> Vec<PerformanceMetrics> {
        let mut metrics = Vec::new();

        for (i, prompt) in self.test_data.prompts.iter().enumerate() {
            let operation_name = format!("text_generation_prompt_{}", i);
            let benchmark_result = self.run_operation_benchmark(
                &operation_name,
                provider_type,
                |service, _| {
                    let prompt = prompt.clone();
                    Box::pin(async move {
                        service.generate_text(&prompt, HashMap::new()).await.map(|_| ())
                    })
                },
                service,
            ).await;

            metrics.push(benchmark_result);
        }

        metrics
    }

    /// Benchmark email processing performance
    async fn benchmark_email_processing(&self, provider_type: &AIProviderType, service: &Arc<AIService>) -> Vec<PerformanceMetrics> {
        let mut metrics = Vec::new();

        for (i, email) in self.test_data.emails.iter().enumerate() {
            let operation_name = format!("email_summary_size_{}", i);
            let benchmark_result = self.run_operation_benchmark(
                &operation_name,
                provider_type,
                |service, _| {
                    let email = email.clone();
                    Box::pin(async move {
                        service.summarize_email(&email).await.map(|_| ())
                    })
                },
                service,
            ).await;

            metrics.push(benchmark_result);
        }

        metrics
    }

    /// Benchmark triage system performance
    async fn benchmark_triage_system(&self, provider_type: &AIProviderType, service: &Arc<AIService>) -> Vec<PerformanceMetrics> {
        let mut metrics = Vec::new();
        let config = EmailTriageConfig::default();

        // Single email triage
        let benchmark_result = self.run_operation_benchmark(
            "triage_single_email",
            provider_type,
            |service, _| {
                let email = self.test_data.emails[0].clone();
                let config = config.clone();
                Box::pin(async move {
                    service.triage_email(&email, &config).await.map(|_| ())
                })
            },
            service,
        ).await;
        metrics.push(benchmark_result);

        // Batch email triage
        let emails: Vec<&StoredMessage> = self.test_data.emails.iter().collect();
        let benchmark_result = self.run_operation_benchmark(
            "triage_batch_emails",
            provider_type,
            |service, _| {
                let emails = emails.clone();
                let config = config.clone();
                Box::pin(async move {
                    service.triage_emails_batch(emails, &config).await.map(|_| ())
                })
            },
            service,
        ).await;
        metrics.push(benchmark_result);

        metrics
    }

    /// Benchmark concurrent operations
    async fn benchmark_concurrent_operations(&self, provider_type: &AIProviderType, service: &Arc<AIService>) -> Vec<PerformanceMetrics> {
        let mut metrics = Vec::new();
        let concurrency_levels = vec![1, 2, 5, 10, 20];

        for concurrency in concurrency_levels {
            let operation_name = format!("concurrent_ops_level_{}", concurrency);
            let metrics_result = self.benchmark_concurrency_level(
                &operation_name,
                provider_type,
                service,
                concurrency,
            ).await;
            metrics.push(metrics_result);
        }

        metrics
    }

    /// Benchmark cache performance
    async fn benchmark_cache_performance(&self, provider_type: &AIProviderType, service: &Arc<AIService>) -> Vec<PerformanceMetrics> {
        let mut metrics = Vec::new();

        // Cache hit performance
        let prompt = "Cache performance test prompt";
        
        // First request (cache miss)
        let cache_miss_result = self.run_operation_benchmark(
            "cache_miss",
            provider_type,
            |service, _| {
                Box::pin(async move {
                    service.generate_text(prompt, HashMap::new()).await.map(|_| ())
                })
            },
            service,
        ).await;
        metrics.push(cache_miss_result);

        // Second request (cache hit)
        let cache_hit_result = self.run_operation_benchmark(
            "cache_hit",
            provider_type,
            |service, _| {
                Box::pin(async move {
                    service.generate_text(prompt, HashMap::new()).await.map(|_| ())
                })
            },
            service,
        ).await;
        metrics.push(cache_hit_result);

        metrics
    }

    /// Benchmark memory usage patterns
    async fn benchmark_memory_usage(&self, provider_type: &AIProviderType, service: &Arc<AIService>) -> Vec<PerformanceMetrics> {
        let mut metrics = Vec::new();

        // Memory usage under sustained load
        let benchmark_result = self.run_memory_benchmark(
            "sustained_load_memory",
            provider_type,
            service,
        ).await;
        metrics.push(benchmark_result);

        metrics
    }

    /// Run a generic operation benchmark
    async fn run_operation_benchmark<F, Fut>(
        &self,
        operation_name: &str,
        provider_type: &AIProviderType,
        operation: F,
        service: &Arc<AIService>,
    ) -> PerformanceMetrics
    where
        F: Fn(Arc<AIService>, usize) -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<(), comunicado::ai::AIError>> + Send,
    {
        let start_time = Instant::now();
        let initial_memory = self.get_memory_usage();

        // Warm-up phase
        for i in 0..self.config.warmup_iterations {
            let _ = operation(Arc::clone(service), i).await;
        }

        // Measurement phase
        let mut latencies = Vec::new();
        let mut errors = 0;
        let measurement_start = Instant::now();

        for i in 0..self.config.iterations {
            let op_start = Instant::now();
            match tokio::time::timeout(
                self.config.operation_timeout,
                operation(Arc::clone(service), i)
            ).await {
                Ok(Ok(())) => {
                    latencies.push(op_start.elapsed());
                }
                Ok(Err(_)) | Err(_) => {
                    errors += 1;
                }
            }
        }

        let net_duration = measurement_start.elapsed();
        let total_duration = start_time.elapsed();
        let peak_memory = self.get_memory_usage();
        let final_memory = self.get_memory_usage();

        let latency_stats = self.calculate_latency_stats(&latencies);
        let memory_stats = MemoryStats {
            initial_memory_bytes: initial_memory,
            peak_memory_bytes: peak_memory,
            final_memory_bytes: final_memory,
            allocated_bytes: peak_memory.saturating_sub(initial_memory),
            growth_rate: if self.config.iterations > 0 {
                (final_memory.saturating_sub(initial_memory)) as f64 / self.config.iterations as f64
            } else {
                0.0
            },
        };

        let successful_ops = self.config.iterations - errors;
        let ops_per_second = if net_duration.as_secs_f64() > 0.0 {
            successful_ops as f64 / net_duration.as_secs_f64()
        } else {
            0.0
        };

        let error_rate = errors as f64 / self.config.iterations as f64;

        let mut metadata = HashMap::new();
        metadata.insert("iterations".to_string(), self.config.iterations.to_string());
        metadata.insert("errors".to_string(), errors.to_string());
        metadata.insert("warmup_iterations".to_string(), self.config.warmup_iterations.to_string());

        PerformanceMetrics {
            operation: operation_name.to_string(),
            provider: provider_type.clone(),
            operations: successful_ops,
            total_duration,
            net_duration,
            ops_per_second,
            latency_stats,
            memory_stats,
            error_rate,
            metadata,
        }
    }

    /// Benchmark specific concurrency level
    async fn benchmark_concurrency_level(
        &self,
        operation_name: &str,
        provider_type: &AIProviderType,
        service: &Arc<AIService>,
        concurrency: usize,
    ) -> PerformanceMetrics {
        let start_time = Instant::now();
        let initial_memory = self.get_memory_usage();

        let mut handles = Vec::new();
        let operations_per_task = self.config.iterations / concurrency;

        for task_id in 0..concurrency {
            let service_clone = Arc::clone(service);
            let prompt = format!("Concurrent benchmark task {}", task_id);
            
            let handle = tokio::spawn(async move {
                let mut task_latencies = Vec::new();
                let mut task_errors = 0;

                for _i in 0..operations_per_task {
                    let op_start = Instant::now();
                    match service_clone.generate_text(&prompt, HashMap::new()).await {
                        Ok(_) => {
                            task_latencies.push(op_start.elapsed());
                        }
                        Err(_) => {
                            task_errors += 1;
                        }
                    }
                }

                (task_latencies, task_errors)
            });

            handles.push(handle);
        }

        let mut all_latencies = Vec::new();
        let mut total_errors = 0;

        for handle in handles {
            if let Ok((latencies, errors)) = handle.await {
                all_latencies.extend(latencies);
                total_errors += errors;
            }
        }

        let total_duration = start_time.elapsed();
        let peak_memory = self.get_memory_usage();
        let final_memory = self.get_memory_usage();

        let latency_stats = self.calculate_latency_stats(&all_latencies);
        let memory_stats = MemoryStats {
            initial_memory_bytes: initial_memory,
            peak_memory_bytes: peak_memory,
            final_memory_bytes: final_memory,
            allocated_bytes: peak_memory.saturating_sub(initial_memory),
            growth_rate: 0.0, // Not meaningful for concurrent tests
        };

        let successful_ops = all_latencies.len();
        let ops_per_second = if total_duration.as_secs_f64() > 0.0 {
            successful_ops as f64 / total_duration.as_secs_f64()
        } else {
            0.0
        };

        let error_rate = total_errors as f64 / (self.config.iterations as f64);

        let mut metadata = HashMap::new();
        metadata.insert("concurrency_level".to_string(), concurrency.to_string());
        metadata.insert("total_errors".to_string(), total_errors.to_string());
        metadata.insert("operations_per_task".to_string(), operations_per_task.to_string());

        PerformanceMetrics {
            operation: operation_name.to_string(),
            provider: provider_type.clone(),
            operations: successful_ops,
            total_duration,
            net_duration: total_duration, // Same for concurrent tests
            ops_per_second,
            latency_stats,
            memory_stats,
            error_rate,
            metadata,
        }
    }

    /// Run memory-focused benchmark
    async fn run_memory_benchmark(
        &self,
        operation_name: &str,
        provider_type: &AIProviderType,
        service: &Arc<AIService>,
    ) -> PerformanceMetrics {
        let start_time = Instant::now();
        let initial_memory = self.get_memory_usage();

        let mut memory_samples = Vec::new();
        let mut latencies = Vec::new();
        let mut errors = 0;

        // Take memory samples throughout the benchmark
        for i in 0..self.config.iterations {
            let current_memory = self.get_memory_usage();
            memory_samples.push(current_memory);

            let op_start = Instant::now();
            match service.generate_text("Memory benchmark test", HashMap::new()).await {
                Ok(_) => {
                    latencies.push(op_start.elapsed());
                }
                Err(_) => {
                    errors += 1;
                }
            }

            // Sample every 10 operations
            if i % 10 == 0 {
                tokio::time::sleep(Duration::from_millis(10)).await; // Allow GC
            }
        }

        let total_duration = start_time.elapsed();
        let final_memory = self.get_memory_usage();
        let peak_memory = memory_samples.iter().max().copied().unwrap_or(final_memory);

        let latency_stats = self.calculate_latency_stats(&latencies);
        let memory_stats = MemoryStats {
            initial_memory_bytes: initial_memory,
            peak_memory_bytes: peak_memory,
            final_memory_bytes: final_memory,
            allocated_bytes: peak_memory.saturating_sub(initial_memory),
            growth_rate: (final_memory.saturating_sub(initial_memory)) as f64 / self.config.iterations as f64,
        };

        let successful_ops = latencies.len();
        let ops_per_second = if total_duration.as_secs_f64() > 0.0 {
            successful_ops as f64 / total_duration.as_secs_f64()
        } else {
            0.0
        };

        let error_rate = errors as f64 / self.config.iterations as f64;

        let mut metadata = HashMap::new();
        metadata.insert("memory_samples".to_string(), memory_samples.len().to_string());
        metadata.insert("peak_memory_mb".to_string(), (peak_memory / 1024 / 1024).to_string());
        metadata.insert("memory_growth_kb_per_op".to_string(), (memory_stats.growth_rate / 1024.0).to_string());

        PerformanceMetrics {
            operation: operation_name.to_string(),
            provider: provider_type.clone(),
            operations: successful_ops,
            total_duration,
            net_duration: total_duration,
            ops_per_second,
            latency_stats,
            memory_stats,
            error_rate,
            metadata,
        }
    }

    /// Calculate latency statistics from measurements
    fn calculate_latency_stats(&self, latencies: &[Duration]) -> LatencyStats {
        if latencies.is_empty() {
            return LatencyStats {
                min: Duration::ZERO,
                max: Duration::ZERO,
                mean: Duration::ZERO,
                median: Duration::ZERO,
                percentiles: HashMap::new(),
                std_dev: Duration::ZERO,
            };
        }

        let mut sorted_latencies = latencies.to_vec();
        sorted_latencies.sort();

        let min = sorted_latencies[0];
        let max = sorted_latencies[sorted_latencies.len() - 1];

        let total_nanos: u64 = latencies.iter().map(|d| d.as_nanos() as u64).sum();
        let mean = Duration::from_nanos(total_nanos / latencies.len() as u64);

        let median_index = sorted_latencies.len() / 2;
        let median = if sorted_latencies.len() % 2 == 0 {
            let lower = sorted_latencies[median_index - 1];
            let upper = sorted_latencies[median_index];
            Duration::from_nanos((lower.as_nanos() + upper.as_nanos()) as u64 / 2)
        } else {
            sorted_latencies[median_index]
        };

        let mut percentiles = HashMap::new();
        for &percentile in &self.config.latency_percentiles {
            let index = ((percentile / 100.0) * (sorted_latencies.len() as f64 - 1.0)) as usize;
            let index = index.min(sorted_latencies.len() - 1);
            percentiles.insert(percentile, sorted_latencies[index]);
        }

        // Calculate standard deviation
        let mean_nanos = mean.as_nanos() as f64;
        let variance: f64 = latencies
            .iter()
            .map(|d| {
                let diff = d.as_nanos() as f64 - mean_nanos;
                diff * diff
            })
            .sum::<f64>() / latencies.len() as f64;
        let std_dev = Duration::from_nanos(variance.sqrt() as u64);

        LatencyStats {
            min,
            max,
            mean,
            median,
            percentiles,
            std_dev,
        }
    }

    /// Get current memory usage (simplified implementation)
    fn get_memory_usage(&self) -> usize {
        // In a real implementation, this would use system APIs to get memory usage
        // For now, return a placeholder value
        if self.config.measure_memory {
            // Simulate memory measurement
            std::process::id() as usize * 1024 // Placeholder
        } else {
            0
        }
    }

    /// Generate performance report
    pub fn generate_performance_report(&self, metrics: &[PerformanceMetrics]) -> String {
        let mut report = String::new();

        report.push_str("# AI Performance Benchmark Report\n\n");
        report.push_str(&format!("Generated: {}\n\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));

        // Executive Summary
        report.push_str("## Executive Summary\n\n");
        let total_operations: usize = metrics.iter().map(|m| m.operations).sum();
        let avg_ops_per_second: f64 = metrics.iter().map(|m| m.ops_per_second).sum::<f64>() / metrics.len() as f64;
        let avg_error_rate: f64 = metrics.iter().map(|m| m.error_rate).sum::<f64>() / metrics.len() as f64;

        report.push_str(&format!("- **Total Operations:** {}\n", total_operations));
        report.push_str(&format!("- **Average Throughput:** {:.2} ops/sec\n", avg_ops_per_second));
        report.push_str(&format!("- **Average Error Rate:** {:.2}%\n", avg_error_rate * 100.0));
        report.push_str(&format!("- **Benchmark Configurations:** {} iterations, {} concurrency\n\n", 
            self.config.iterations, self.config.concurrency));

        // Performance by Provider
        let mut providers: Vec<_> = metrics.iter().map(|m| &m.provider).collect();
        providers.sort();
        providers.dedup();

        for provider in providers {
            report.push_str(&format!("## {:?} Provider Performance\n\n", provider));
            
            let provider_metrics: Vec<_> = metrics.iter().filter(|m| &m.provider == provider).collect();
            
            report.push_str("| Operation | Ops/Sec | Avg Latency | P95 Latency | Error Rate |\n");
            report.push_str("|-----------|---------|-------------|-------------|------------|\n");

            for metric in provider_metrics {
                let avg_latency_ms = metric.latency_stats.mean.as_millis();
                let p95_latency = metric.latency_stats.percentiles.get(&95.0)
                    .map(|d| d.as_millis())
                    .unwrap_or(0);

                report.push_str(&format!("| {} | {:.2} | {}ms | {}ms | {:.1}% |\n",
                    metric.operation,
                    metric.ops_per_second,
                    avg_latency_ms,
                    p95_latency,
                    metric.error_rate * 100.0
                ));
            }
            report.push_str("\n");
        }

        // Latency Analysis
        report.push_str("## Latency Analysis\n\n");
        for metric in metrics {
            if metric.operations > 0 {
                report.push_str(&format!("### {}\n", metric.operation));
                report.push_str(&format!("- **Provider:** {:?}\n", metric.provider));
                report.push_str(&format!("- **Mean:** {:?}\n", metric.latency_stats.mean));
                report.push_str(&format!("- **Median:** {:?}\n", metric.latency_stats.median));
                report.push_str(&format!("- **Min/Max:** {:?} / {:?}\n", metric.latency_stats.min, metric.latency_stats.max));
                
                report.push_str("- **Percentiles:**\n");
                let mut percentiles: Vec<_> = metric.latency_stats.percentiles.iter().collect();
                percentiles.sort_by_key(|(p, _)| *p as u64);
                for (percentile, duration) in percentiles {
                    report.push_str(&format!("  - P{}: {:?}\n", percentile, duration));
                }
                report.push_str("\n");
            }
        }

        // Memory Usage Analysis
        if self.config.measure_memory {
            report.push_str("## Memory Usage Analysis\n\n");
            for metric in metrics.iter().filter(|m| m.memory_stats.allocated_bytes > 0) {
                report.push_str(&format!("### {}\n", metric.operation));
                report.push_str(&format!("- **Initial Memory:** {} MB\n", metric.memory_stats.initial_memory_bytes / 1024 / 1024));
                report.push_str(&format!("- **Peak Memory:** {} MB\n", metric.memory_stats.peak_memory_bytes / 1024 / 1024));
                report.push_str(&format!("- **Final Memory:** {} MB\n", metric.memory_stats.final_memory_bytes / 1024 / 1024));
                report.push_str(&format!("- **Memory Growth:** {:.2} KB/operation\n", metric.memory_stats.growth_rate / 1024.0));
                report.push_str("\n");
            }
        }

        // Recommendations
        report.push_str("## Performance Recommendations\n\n");
        
        // Analyze metrics for recommendations
        let high_latency_ops: Vec<_> = metrics.iter()
            .filter(|m| m.latency_stats.mean > Duration::from_millis(2000))
            .collect();
        
        let high_error_rate_ops: Vec<_> = metrics.iter()
            .filter(|m| m.error_rate > 0.05) // >5% error rate
            .collect();

        let low_throughput_ops: Vec<_> = metrics.iter()
            .filter(|m| m.ops_per_second < 0.5) // <0.5 ops/sec
            .collect();

        if !high_latency_ops.is_empty() {
            report.push_str("### High Latency Operations\n");
            report.push_str("The following operations have high average latency (>2s):\n");
            for op in high_latency_ops {
                report.push_str(&format!("- **{}**: {:?} average latency\n", op.operation, op.latency_stats.mean));
            }
            report.push_str("**Recommendation:** Consider optimizing these operations or implementing caching.\n\n");
        }

        if !high_error_rate_ops.is_empty() {
            report.push_str("### High Error Rate Operations\n");
            report.push_str("The following operations have high error rates (>5%):\n");
            for op in high_error_rate_ops {
                report.push_str(&format!("- **{}**: {:.1}% error rate\n", op.operation, op.error_rate * 100.0));
            }
            report.push_str("**Recommendation:** Investigate error causes and improve error handling.\n\n");
        }

        if !low_throughput_ops.is_empty() {
            report.push_str("### Low Throughput Operations\n");
            report.push_str("The following operations have low throughput (<0.5 ops/sec):\n");
            for op in low_throughput_ops {
                report.push_str(&format!("- **{}**: {:.2} ops/sec\n", op.operation, op.ops_per_second));
            }
            report.push_str("**Recommendation:** Consider performance optimization or async processing.\n\n");
        }

        // General recommendations
        if metrics.iter().any(|m| m.ops_per_second > 5.0) {
            report.push_str("- ✅ Good overall throughput detected for some operations.\n");
        }

        if metrics.iter().all(|m| m.error_rate < 0.01) {
            report.push_str("- ✅ Excellent reliability across all operations.\n");
        }

        report.push_str("\n---\n");
        report.push_str(&format!("Report generated with {} iterations per benchmark.\n", self.config.iterations));

        report
    }
}

/// Run performance benchmarks from command line
pub async fn run_ai_performance_benchmarks() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting AI Performance Benchmarks");

    let config = BenchmarkConfig {
        iterations: std::env::var("BENCH_ITERATIONS")
            .unwrap_or_default()
            .parse()
            .unwrap_or(50), // Reduced for CI/testing
        concurrency: 5,
        warmup_iterations: 5,
        operation_timeout: Duration::from_secs(30),
        measure_memory: true,
        latency_percentiles: vec![50.0, 90.0, 95.0, 99.0],
    };

    println!("Benchmark Configuration:");
    println!("  - Iterations: {}", config.iterations);
    println!("  - Concurrency: {}", config.concurrency);
    println!("  - Warmup: {}", config.warmup_iterations);
    println!("  - Timeout: {:?}", config.operation_timeout);
    println!();

    let benchmark = AIPerformanceBenchmark::new(config).await?;
    let metrics = benchmark.run_all_benchmarks().await;

    // Generate and save report
    let report = benchmark.generate_performance_report(&metrics);
    let report_path = "ai_performance_benchmark_report.md";
    tokio::fs::write(report_path, &report).await?;

    println!("\n📄 Performance report saved to: {}", report_path);

    // Print summary
    let total_operations: usize = metrics.iter().map(|m| m.operations).sum();
    let avg_ops_per_second: f64 = if !metrics.is_empty() {
        metrics.iter().map(|m| m.ops_per_second).sum::<f64>() / metrics.len() as f64
    } else {
        0.0
    };

    println!("\n🎯 Performance Summary:");
    println!("   Total Operations: {}", total_operations);
    println!("   Average Throughput: {:.2} ops/sec", avg_ops_per_second);
    println!("   Benchmarks Completed: {}", metrics.len());

    if avg_ops_per_second > 1.0 {
        println!("\n🎉 Performance benchmarks completed successfully!");
        Ok(())
    } else {
        Err("Performance benchmarks indicate potential issues".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_benchmark_config() {
        let config = BenchmarkConfig::default();
        assert!(config.iterations > 0);
        assert!(config.concurrency > 0);
        assert!(!config.latency_percentiles.is_empty());
    }

    #[tokio::test]
    async fn test_benchmark_data_generation() {
        let data = BenchmarkTestData::new();
        assert!(!data.emails.is_empty());
        assert!(!data.prompts.is_empty());
        assert!(!data.calendar_texts.is_empty());

        // Verify email sizes are different
        let sizes: Vec<_> = data.emails.iter().map(|e| e.size.unwrap_or(0)).collect();
        assert!(sizes.iter().any(|&s| s < 1000)); // Small email
        assert!(sizes.iter().any(|&s| s > 10000)); // Large email
    }

    #[tokio::test]
    async fn test_latency_stats_calculation() {
        let config = BenchmarkConfig::default();
        let latencies = vec![
            Duration::from_millis(100),
            Duration::from_millis(200),
            Duration::from_millis(150),
            Duration::from_millis(300),
            Duration::from_millis(250),
        ];

        let benchmark = AIPerformanceBenchmark {
            config,
            services: HashMap::new(),
            test_data: BenchmarkTestData::new(),
        };

        let stats = benchmark.calculate_latency_stats(&latencies);
        
        assert_eq!(stats.min, Duration::from_millis(100));
        assert_eq!(stats.max, Duration::from_millis(300));
        assert!(stats.mean > Duration::from_millis(150));
        assert!(stats.mean < Duration::from_millis(250));
        assert!(!stats.percentiles.is_empty());
    }

    #[test]
    fn test_performance_metrics_creation() {
        let metrics = PerformanceMetrics {
            operation: "test_op".to_string(),
            provider: AIProviderType::Ollama,
            operations: 100,
            total_duration: Duration::from_secs(10),
            net_duration: Duration::from_secs(8),
            ops_per_second: 12.5,
            latency_stats: LatencyStats {
                min: Duration::from_millis(50),
                max: Duration::from_millis(200),
                mean: Duration::from_millis(100),
                median: Duration::from_millis(95),
                percentiles: HashMap::new(),
                std_dev: Duration::from_millis(25),
            },
            memory_stats: MemoryStats {
                initial_memory_bytes: 1024 * 1024,
                peak_memory_bytes: 2048 * 1024,
                final_memory_bytes: 1536 * 1024,
                allocated_bytes: 1024 * 1024,
                growth_rate: 10240.0,
            },
            error_rate: 0.02,
            metadata: HashMap::new(),
        };

        assert_eq!(metrics.operations, 100);
        assert_eq!(metrics.provider, AIProviderType::Ollama);
        assert!(metrics.ops_per_second > 10.0);
        assert!(metrics.error_rate < 0.05);
    }
}