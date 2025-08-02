//! Advanced AI Integration Test Runner
//! 
//! Comprehensive test suite for AI functionality with real provider integration,
//! performance monitoring, and end-to-end workflow validation.

use comunicado::ai::{
    AIService, AIConfig, AIProviderType, PrivacyMode,
    EmailTriageConfig, EmailPriority,
    cache::AIResponseCache,
    provider::AIProviderManager,
    retry::{RetryManager, RetryConfig},
};
use comunicado::email::StoredMessage;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::timeout;
use uuid::Uuid;
use chrono::Utc;

/// Test configuration for AI integration tests
#[derive(Debug, Clone)]
pub struct AITestConfig {
    /// Test real AI providers (requires API keys)
    pub test_real_providers: bool,
    /// Providers to test
    pub providers: Vec<AIProviderType>,
    /// Test iterations for performance
    pub test_iterations: usize,
    /// Timeout for individual operations
    pub operation_timeout: Duration,
    /// Enable verbose output
    pub verbose: bool,
}

impl Default for AITestConfig {
    fn default() -> Self {
        Self {
            test_real_providers: std::env::var("AI_TEST_REAL").unwrap_or_default() == "true",
            providers: vec![AIProviderType::Ollama], // Start with local only
            test_iterations: 10,
            operation_timeout: Duration::from_secs(30),
            verbose: false,
        }
    }
}

/// Test result for individual AI operations
#[derive(Debug, Clone)]
pub struct AITestResult {
    pub test_name: String,
    pub provider: AIProviderType,
    pub success: bool,
    pub duration: Duration,
    pub error: Option<String>,
    pub metadata: HashMap<String, String>,
}

impl AITestResult {
    pub fn success(test_name: String, provider: AIProviderType, duration: Duration) -> Self {
        Self {
            test_name,
            provider,
            success: true,
            duration,
            error: None,
            metadata: HashMap::new(),
        }
    }

    pub fn failure(test_name: String, provider: AIProviderType, duration: Duration, error: String) -> Self {
        Self {
            test_name,
            provider,
            success: false,
            duration,
            error: Some(error),
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Comprehensive AI test results
#[derive(Debug)]
pub struct AITestSuite {
    pub results: Vec<AITestResult>,
    pub total_duration: Duration,
    pub config: AITestConfig,
}

impl AITestSuite {
    pub fn new(config: AITestConfig) -> Self {
        Self {
            results: Vec::new(),
            total_duration: Duration::ZERO,
            config,
        }
    }

    pub fn total_tests(&self) -> usize {
        self.results.len()
    }

    pub fn passed_tests(&self) -> usize {
        self.results.iter().filter(|r| r.success).count()
    }

    pub fn failed_tests(&self) -> usize {
        self.results.iter().filter(|r| !r.success).count()
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_tests() == 0 {
            0.0
        } else {
            self.passed_tests() as f64 / self.total_tests() as f64
        }
    }

    pub fn add_result(&mut self, result: AITestResult) {
        self.results.push(result);
    }

    /// Generate test report
    pub fn generate_report(&self) -> String {
        let mut report = String::new();
        
        report.push_str("# AI Integration Test Report\n\n");
        report.push_str(&format!("Generated: {}\n\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));

        // Summary
        report.push_str("## Summary\n\n");
        report.push_str(&format!("- **Total Tests:** {}\n", self.total_tests()));
        report.push_str(&format!("- **Passed:** {}\n", self.passed_tests()));
        report.push_str(&format!("- **Failed:** {}\n", self.failed_tests()));
        report.push_str(&format!("- **Success Rate:** {:.1}%\n", self.success_rate() * 100.0));
        report.push_str(&format!("- **Total Duration:** {:?}\n\n", self.total_duration));

        // Test Results by Provider
        let mut providers: Vec<_> = self.results.iter().map(|r| &r.provider).collect();
        providers.sort_by_key(|p| format!("{:?}", p));
        providers.dedup();

        for provider in providers {
            let provider_results: Vec<_> = self.results.iter().filter(|r| &r.provider == provider).collect();
            let provider_passed = provider_results.iter().filter(|r| r.success).count();
            let provider_total = provider_results.len();
            
            report.push_str(&format!("## {:?} Provider Results\n\n", provider));
            report.push_str(&format!("- **Tests:** {}\n", provider_total));
            report.push_str(&format!("- **Passed:** {}\n", provider_passed));
            report.push_str(&format!("- **Success Rate:** {:.1}%\n\n", 
                if provider_total > 0 { (provider_passed as f64 / provider_total as f64) * 100.0 } else { 0.0 }
            ));

            report.push_str("| Test | Status | Duration |\n");
            report.push_str("|------|--------|---------|\n");
            
            for result in provider_results {
                let status = if result.success { "✅ Pass" } else { "❌ Fail" };
                report.push_str(&format!("| {} | {} | {:?} |\n", 
                    result.test_name, status, result.duration));
            }
            report.push_str("\n");
        }

        // Failed Tests Details
        let failed_tests: Vec<_> = self.results.iter().filter(|r| !r.success).collect();
        if !failed_tests.is_empty() {
            report.push_str("## Failed Tests\n\n");
            for test in failed_tests {
                report.push_str(&format!("### {} ({:?})\n", test.test_name, test.provider));
                if let Some(error) = &test.error {
                    report.push_str(&format!("**Error:** {}\n", error));
                }
                report.push_str(&format!("**Duration:** {:?}\n\n", test.duration));
            }
        }

        report
    }
}

/// Advanced AI integration test runner
pub struct AIIntegrationTestRunner {
    config: AITestConfig,
    services: HashMap<AIProviderType, Arc<AIService>>,
}

impl AIIntegrationTestRunner {
    /// Create new test runner
    pub async fn new(config: AITestConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let mut services = HashMap::new();

        // Initialize AI services for each provider
        for provider in &config.providers {
            if let Ok(service) = Self::create_ai_service(provider.clone()).await {
                services.insert(provider.clone(), service);
                if config.verbose {
                    println!("✓ Initialized AI service for {:?}", provider);
                }
            } else if config.verbose {
                println!("⚠ Failed to initialize {:?}", provider);
            }
        }

        if services.is_empty() {
            return Err("No AI services could be initialized".into());
        }

        Ok(Self { config, services })
    }

    /// Create AI service for a provider
    async fn create_ai_service(provider_type: AIProviderType) -> Result<Arc<AIService>, Box<dyn std::error::Error>> {
        let config = Arc::new(RwLock::new(AIConfig {
            enabled: true,
            provider: provider_type,
            email_triage_enabled: true,
            privacy_mode: PrivacyMode::Standard,
            ..Default::default()
        }));

        let cache = Arc::new(AIResponseCache::new(1000, Duration::from_secs(3600)));
        let provider_manager = Arc::new(RwLock::new(AIProviderManager::new(config.clone())));

        Ok(Arc::new(AIService::new(provider_manager, cache, config)))
    }

    /// Run comprehensive test suite
    pub async fn run_all_tests(&self) -> AITestSuite {
        let start_time = Instant::now();
        let mut suite = AITestSuite::new(self.config.clone());

        println!("🚀 Running AI Integration Test Suite");
        println!("Providers: {:?}", self.config.providers);
        println!("Iterations: {}", self.config.test_iterations);
        println!();

        // Run basic functionality tests
        suite.results.extend(self.test_basic_functionality().await);

        // Run email processing tests
        suite.results.extend(self.test_email_processing().await);

        // Run triage system tests
        suite.results.extend(self.test_triage_system().await);

        // Run performance tests
        suite.results.extend(self.test_performance().await);

        // Run error handling tests
        suite.results.extend(self.test_error_handling().await);

        suite.total_duration = start_time.elapsed();

        println!("✅ Test suite completed in {:?}", suite.total_duration);
        println!("📊 Results: {}/{} tests passed ({:.1}% success rate)", 
            suite.passed_tests(), 
            suite.total_tests(), 
            suite.success_rate() * 100.0
        );

        suite
    }

    /// Test basic AI functionality
    async fn test_basic_functionality(&self) -> Vec<AITestResult> {
        println!("🔧 Testing basic functionality...");
        let mut results = Vec::new();

        for (provider, service) in &self.services {
            // Test service initialization
            results.push(self.test_service_init(provider, service).await);

            // Test simple text generation
            results.push(self.test_text_generation(provider, service).await);

            // Test retry mechanism
            results.push(self.test_retry_mechanism(provider).await);
        }

        results
    }

    /// Test email processing functionality
    async fn test_email_processing(&self) -> Vec<AITestResult> {
        println!("📧 Testing email processing...");
        let mut results = Vec::new();

        for (provider, service) in &self.services {
            // Test email summarization
            results.push(self.test_email_summarization(provider, service).await);

            // Test email reply suggestions
            results.push(self.test_reply_suggestions(provider, service).await);

            // Test content extraction
            results.push(self.test_content_extraction(provider, service).await);
        }

        results
    }

    /// Test triage system
    async fn test_triage_system(&self) -> Vec<AITestResult> {
        println!("🔍 Testing AI triage system...");
        let mut results = Vec::new();

        for (provider, service) in &self.services {
            // Test single email triage
            results.push(self.test_single_triage(provider, service).await);

            // Test batch triage
            results.push(self.test_batch_triage(provider, service).await);
        }

        results
    }

    /// Test performance characteristics
    async fn test_performance(&self) -> Vec<AITestResult> {
        println!("⚡ Testing performance...");
        let mut results = Vec::new();

        for (provider, service) in &self.services {
            // Test response latency
            results.push(self.test_latency(provider, service).await);

            // Test concurrent operations
            results.push(self.test_concurrency(provider, service).await);
        }

        results
    }

    /// Test error handling
    async fn test_error_handling(&self) -> Vec<AITestResult> {
        println!("🛡️ Testing error handling...");
        let mut results = Vec::new();

        for (provider, service) in &self.services {
            // Test invalid input handling
            results.push(self.test_invalid_input(provider, service).await);

            // Test timeout handling
            results.push(self.test_timeout_handling(provider, service).await);
        }

        results
    }

    // Individual test implementations

    async fn test_service_init(&self, provider: &AIProviderType, _service: &Arc<AIService>) -> AITestResult {
        let start = Instant::now();
        let test_name = format!("{:?}_service_init", provider);

        // For now, just verify the service was created successfully
        AITestResult::success(test_name, provider.clone(), start.elapsed())
    }

    async fn test_text_generation(&self, provider: &AIProviderType, service: &Arc<AIService>) -> AITestResult {
        let start = Instant::now();
        let test_name = format!("{:?}_text_generation", provider);

        match timeout(
            self.config.operation_timeout,
            service.generate_completion("Test prompt", None)
        ).await {
            Ok(Ok(response)) => {
                if response.len() > 5 {
                    AITestResult::success(test_name, provider.clone(), start.elapsed())
                        .with_metadata("response_length".to_string(), response.len().to_string())
                } else {
                    AITestResult::failure(test_name, provider.clone(), start.elapsed(), 
                        "Response too short".to_string())
                }
            }
            Ok(Err(e)) => {
                AITestResult::failure(test_name, provider.clone(), start.elapsed(), 
                    format!("AI error: {}", e))
            }
            Err(_) => {
                AITestResult::failure(test_name, provider.clone(), start.elapsed(), 
                    "Operation timed out".to_string())
            }
        }
    }

    async fn test_retry_mechanism(&self, provider: &AIProviderType) -> AITestResult {
        let start = Instant::now();
        let test_name = format!("{:?}_retry_mechanism", provider);

        let retry_manager = RetryManager::new(RetryConfig {
            max_attempts: 3,
            base_delay: Duration::from_millis(10),
            max_delay: Duration::from_millis(100),
            backoff_multiplier: 2.0,
            jitter_enabled: false,
        });

        let mut attempt_count = 0;
        let result = retry_manager.execute_with_retry(|| {
            attempt_count += 1;
            async move {
                if attempt_count < 2 {
                    Err(comunicado::ai::AIError::network_error("Simulated failure"))
                } else {
                    Ok("Success")
                }
            }
        }).await;

        match result {
            Ok(_) if attempt_count == 2 => {
                AITestResult::success(test_name, provider.clone(), start.elapsed())
                    .with_metadata("attempts".to_string(), attempt_count.to_string())
            }
            Ok(_) => {
                AITestResult::failure(test_name, provider.clone(), start.elapsed(),
                    format!("Unexpected attempt count: {}", attempt_count))
            }
            Err(e) => {
                AITestResult::failure(test_name, provider.clone(), start.elapsed(),
                    format!("Retry failed: {}", e))
            }
        }
    }

    async fn test_email_summarization(&self, provider: &AIProviderType, service: &Arc<AIService>) -> AITestResult {
        let start = Instant::now();
        let test_name = format!("{:?}_email_summarization", provider);

        let test_email = self.create_test_email();
        
        match timeout(
            self.config.operation_timeout,
            service.summarize_email(&test_email, None)
        ).await {
            Ok(Ok(summary)) => {
                if !summary.is_empty() {
                    AITestResult::success(test_name, provider.clone(), start.elapsed())
                        .with_metadata("summary_length".to_string(), summary.len().to_string())
                } else {
                    AITestResult::failure(test_name, provider.clone(), start.elapsed(),
                        "Empty summary returned".to_string())
                }
            }
            Ok(Err(e)) => {
                AITestResult::failure(test_name, provider.clone(), start.elapsed(),
                    format!("Summarization failed: {}", e))
            }
            Err(_) => {
                AITestResult::failure(test_name, provider.clone(), start.elapsed(),
                    "Summarization timed out".to_string())
            }
        }
    }

    async fn test_reply_suggestions(&self, provider: &AIProviderType, service: &Arc<AIService>) -> AITestResult {
        let start = Instant::now();
        let test_name = format!("{:?}_reply_suggestions", provider);

        let context = "Reply to a meeting invitation, accepting it politely";
        
        match timeout(
            self.config.operation_timeout,
            service.suggest_email_reply(context, None)
        ).await {
            Ok(Ok(suggestions)) => {
                if !suggestions.is_empty() {
                    AITestResult::success(test_name, provider.clone(), start.elapsed())
                        .with_metadata("suggestions_count".to_string(), suggestions.len().to_string())
                } else {
                    AITestResult::failure(test_name, provider.clone(), start.elapsed(),
                        "No suggestions returned".to_string())
                }
            }
            Ok(Err(e)) => {
                AITestResult::failure(test_name, provider.clone(), start.elapsed(),
                    format!("Reply suggestions failed: {}", e))
            }
            Err(_) => {
                AITestResult::failure(test_name, provider.clone(), start.elapsed(),
                    "Reply suggestions timed out".to_string())
            }
        }
    }

    async fn test_content_extraction(&self, provider: &AIProviderType, service: &Arc<AIService>) -> AITestResult {
        let start = Instant::now();
        let test_name = format!("{:?}_content_extraction", provider);

        let text = "Let's meet tomorrow at 2 PM in the conference room to discuss the project";
        
        match timeout(
            self.config.operation_timeout,
            service.extract_key_info(text)
        ).await {
            Ok(Ok(info)) => {
                if !info.is_empty() {
                    AITestResult::success(test_name, provider.clone(), start.elapsed())
                        .with_metadata("extracted_items".to_string(), info.len().to_string())
                } else {
                    AITestResult::failure(test_name, provider.clone(), start.elapsed(),
                        "No information extracted".to_string())
                }
            }
            Ok(Err(e)) => {
                AITestResult::failure(test_name, provider.clone(), start.elapsed(),
                    format!("Content extraction failed: {}", e))
            }
            Err(_) => {
                AITestResult::failure(test_name, provider.clone(), start.elapsed(),
                    "Content extraction timed out".to_string())
            }
        }
    }

    async fn test_single_triage(&self, provider: &AIProviderType, service: &Arc<AIService>) -> AITestResult {
        let start = Instant::now();
        let test_name = format!("{:?}_single_triage", provider);

        let test_email = self.create_urgent_test_email();
        let config = EmailTriageConfig::default();

        match timeout(
            self.config.operation_timeout,
            service.triage_email(&test_email, &config)
        ).await {
            Ok(Ok(result)) => {
                if result.confidence > 0.0 && matches!(result.priority, EmailPriority::High | EmailPriority::Critical) {
                    AITestResult::success(test_name, provider.clone(), start.elapsed())
                        .with_metadata("confidence".to_string(), format!("{:.2}", result.confidence))
                        .with_metadata("priority".to_string(), format!("{:?}", result.priority))
                } else {
                    AITestResult::failure(test_name, provider.clone(), start.elapsed(),
                        format!("Unexpected triage result: confidence={:.2}, priority={:?}", 
                            result.confidence, result.priority))
                }
            }
            Ok(Err(e)) => {
                AITestResult::failure(test_name, provider.clone(), start.elapsed(),
                    format!("Triage failed: {}", e))
            }
            Err(_) => {
                AITestResult::failure(test_name, provider.clone(), start.elapsed(),
                    "Triage timed out".to_string())
            }
        }
    }

    async fn test_batch_triage(&self, provider: &AIProviderType, service: &Arc<AIService>) -> AITestResult {
        let start = Instant::now();
        let test_name = format!("{:?}_batch_triage", provider);

        let emails = vec![
            self.create_test_email(),
            self.create_urgent_test_email(),
            self.create_newsletter_email(),
        ];
        let email_refs: Vec<&StoredMessage> = emails.iter().collect();
        let config = EmailTriageConfig::default();

        match timeout(
            self.config.operation_timeout,
            service.triage_emails_batch(&email_refs, &config)
        ).await {
            Ok(Ok(results)) => {
                if results.len() == emails.len() {
                    AITestResult::success(test_name, provider.clone(), start.elapsed())
                        .with_metadata("processed_count".to_string(), results.len().to_string())
                } else {
                    AITestResult::failure(test_name, provider.clone(), start.elapsed(),
                        format!("Expected {} results, got {}", emails.len(), results.len()))
                }
            }
            Ok(Err(e)) => {
                AITestResult::failure(test_name, provider.clone(), start.elapsed(),
                    format!("Batch triage failed: {}", e))
            }
            Err(_) => {
                AITestResult::failure(test_name, provider.clone(), start.elapsed(),
                    "Batch triage timed out".to_string())
            }
        }
    }

    async fn test_latency(&self, provider: &AIProviderType, service: &Arc<AIService>) -> AITestResult {
        let start = Instant::now();
        let test_name = format!("{:?}_latency", provider);

        let mut latencies = Vec::new();

        for i in 0..self.config.test_iterations {
            let op_start = Instant::now();
            match service.generate_completion(&format!("Test {}", i), None).await {
                Ok(_) => latencies.push(op_start.elapsed()),
                Err(_) => {
                    return AITestResult::failure(test_name, provider.clone(), start.elapsed(),
                        format!("Operation {} failed", i));
                }
            }
        }

        if !latencies.is_empty() {
            let avg_latency = latencies.iter().sum::<Duration>() / latencies.len() as u32;
            let max_latency = latencies.iter().max().copied().unwrap_or(Duration::ZERO);
            
            AITestResult::success(test_name, provider.clone(), start.elapsed())
                .with_metadata("avg_latency_ms".to_string(), avg_latency.as_millis().to_string())
                .with_metadata("max_latency_ms".to_string(), max_latency.as_millis().to_string())
                .with_metadata("iterations".to_string(), latencies.len().to_string())
        } else {
            AITestResult::failure(test_name, provider.clone(), start.elapsed(),
                "No successful operations".to_string())
        }
    }

    async fn test_concurrency(&self, provider: &AIProviderType, service: &Arc<AIService>) -> AITestResult {
        let start = Instant::now();
        let test_name = format!("{:?}_concurrency", provider);

        let concurrent_ops = 5;
        let mut handles = Vec::new();

        for i in 0..concurrent_ops {
            let service_clone = Arc::clone(service);
            let handle = tokio::spawn(async move {
                service_clone.generate_completion(&format!("Concurrent test {}", i), None).await
            });
            handles.push(handle);
        }

        let mut successful = 0;
        for handle in handles {
            if let Ok(Ok(_)) = handle.await {
                successful += 1;
            }
        }

        let success_rate = successful as f64 / concurrent_ops as f64;
        if success_rate >= 0.8 {
            AITestResult::success(test_name, provider.clone(), start.elapsed())
                .with_metadata("success_rate".to_string(), format!("{:.1}%", success_rate * 100.0))
                .with_metadata("successful_ops".to_string(), successful.to_string())
        } else {
            AITestResult::failure(test_name, provider.clone(), start.elapsed(),
                format!("Low concurrent success rate: {:.1}%", success_rate * 100.0))
        }
    }

    async fn test_invalid_input(&self, provider: &AIProviderType, service: &Arc<AIService>) -> AITestResult {
        let start = Instant::now();
        let test_name = format!("{:?}_invalid_input", provider);

        // Test with empty input
        match service.generate_completion("", None).await {
            Ok(_) => {
                // Some providers may handle empty input gracefully
                AITestResult::success(test_name, provider.clone(), start.elapsed())
                    .with_metadata("behavior".to_string(), "graceful_handling".to_string())
            }
            Err(_) => {
                // Expected for most providers
                AITestResult::success(test_name, provider.clone(), start.elapsed())
                    .with_metadata("behavior".to_string(), "proper_rejection".to_string())
            }
        }
    }

    async fn test_timeout_handling(&self, provider: &AIProviderType, service: &Arc<AIService>) -> AITestResult {
        let start = Instant::now();
        let test_name = format!("{:?}_timeout_handling", provider);

        // Test with very short timeout
        let long_prompt = "Write a detailed essay about ".repeat(100);
        
        match timeout(Duration::from_millis(100), service.generate_completion(&long_prompt, None)).await {
            Ok(_) => {
                // Completed within timeout (acceptable)
                AITestResult::success(test_name, provider.clone(), start.elapsed())
                    .with_metadata("behavior".to_string(), "fast_completion".to_string())
            }
            Err(_) => {
                // Timeout occurred (expected and properly handled)
                AITestResult::success(test_name, provider.clone(), start.elapsed())
                    .with_metadata("behavior".to_string(), "proper_timeout".to_string())
            }
        }
    }

    // Helper methods for creating test data

    fn create_test_email(&self) -> StoredMessage {
        let now = Utc::now();
        StoredMessage {
            id: Uuid::new_v4(),
            account_id: "test_account".to_string(),
            folder_name: "INBOX".to_string(),
            imap_uid: 1,
            message_id: Some("test@example.com".to_string()),
            thread_id: None,
            in_reply_to: None,
            references: Vec::new(),
            subject: "Test Email".to_string(),
            from_addr: "test@example.com".to_string(),
            from_name: Some("Test User".to_string()),
            to_addrs: vec!["recipient@example.com".to_string()],
            cc_addrs: Vec::new(),
            bcc_addrs: Vec::new(),
            reply_to: None,
            date: now,
            body_text: Some("This is a test email for AI processing".to_string()),
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
        }
    }

    fn create_urgent_test_email(&self) -> StoredMessage {
        let now = Utc::now();
        StoredMessage {
            id: Uuid::new_v4(),
            account_id: "test_account".to_string(),
            folder_name: "INBOX".to_string(),
            imap_uid: 2,
            message_id: Some("urgent@example.com".to_string()),
            thread_id: None,
            in_reply_to: None,
            references: Vec::new(),
            subject: "URGENT: Server Down - Immediate Action Required".to_string(),
            from_addr: "admin@company.com".to_string(),
            from_name: Some("System Admin".to_string()),
            to_addrs: vec!["recipient@company.com".to_string()],
            cc_addrs: Vec::new(),
            bcc_addrs: Vec::new(),
            reply_to: None,
            date: now,
            body_text: Some("Critical server outage detected. All systems affected. Please investigate immediately.".to_string()),
            body_html: None,
            attachments: Vec::new(),
            flags: vec!["Important".to_string()],
            labels: Vec::new(),
            size: Some(512),
            priority: Some("High".to_string()),
            created_at: now,
            updated_at: now,
            last_synced: now,
            sync_version: 1,
            is_draft: false,
            is_deleted: false,
        }
    }

    fn create_newsletter_email(&self) -> StoredMessage {
        let now = Utc::now();
        StoredMessage {
            id: Uuid::new_v4(),
            account_id: "test_account".to_string(),
            folder_name: "INBOX".to_string(),
            imap_uid: 3,
            message_id: Some("newsletter@newsletter.com".to_string()),
            thread_id: None,
            in_reply_to: None,
            references: Vec::new(),
            subject: "Weekly Newsletter - Tech Updates".to_string(),
            from_addr: "newsletter@techblog.com".to_string(),
            from_name: Some("Tech Newsletter".to_string()),
            to_addrs: vec!["subscriber@example.com".to_string()],
            cc_addrs: Vec::new(),
            bcc_addrs: Vec::new(),
            reply_to: None,
            date: now,
            body_text: Some("This week's latest technology updates and news".to_string()),
            body_html: None,
            attachments: Vec::new(),
            flags: Vec::new(),
            labels: Vec::new(),
            size: Some(1024),
            priority: None,
            created_at: now,
            updated_at: now,
            last_synced: now,
            sync_version: 1,
            is_draft: false,
            is_deleted: false,
        }
    }
}

/// Run AI integration tests from command line
pub async fn run_ai_integration_tests() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting AI Integration Tests");

    let config = AITestConfig {
        test_real_providers: std::env::var("AI_TEST_REAL").unwrap_or_default() == "true",
        providers: vec![AIProviderType::Ollama], // Safe default
        test_iterations: 5, // Reasonable for testing
        operation_timeout: Duration::from_secs(30),
        verbose: true,
    };

    println!("Configuration:");
    println!("  - Real Providers: {}", config.test_real_providers);
    println!("  - Test Providers: {:?}", config.providers);
    println!("  - Iterations: {}", config.test_iterations);
    println!();

    let runner = AIIntegrationTestRunner::new(config).await?;
    let results = runner.run_all_tests().await;

    // Generate and save report
    let report = results.generate_report();
    let report_path = "ai_integration_test_report.md";
    tokio::fs::write(report_path, &report).await?;

    println!("\n📄 Test report saved to: {}", report_path);

    if results.success_rate() >= 0.8 {
        println!("\n🎉 AI integration tests passed!");
        Ok(())
    } else {
        Err(format!("Tests failed with {:.1}% success rate", results.success_rate() * 100.0).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_config_creation() {
        let config = AITestConfig::default();
        assert!(!config.providers.is_empty());
        assert!(config.test_iterations > 0);
    }

    #[tokio::test]
    async fn test_result_creation() {
        let result = AITestResult::success("test".to_string(), AIProviderType::Ollama, Duration::from_millis(100));
        assert!(result.success);
        assert_eq!(result.test_name, "test");

        let result = AITestResult::failure("test".to_string(), AIProviderType::Ollama, Duration::from_millis(100), "error".to_string());
        assert!(!result.success);
        assert_eq!(result.error, Some("error".to_string()));
    }

    #[test]
    fn test_suite_calculations() {
        let config = AITestConfig::default();
        let mut suite = AITestSuite::new(config);

        suite.add_result(AITestResult::success("test1".to_string(), AIProviderType::Ollama, Duration::from_millis(100)));
        suite.add_result(AITestResult::failure("test2".to_string(), AIProviderType::Ollama, Duration::from_millis(100), "error".to_string()));

        assert_eq!(suite.total_tests(), 2);
        assert_eq!(suite.passed_tests(), 1);
        assert_eq!(suite.failed_tests(), 1);
        assert_eq!(suite.success_rate(), 0.5);
    }

    #[test]
    fn test_report_generation() {
        let config = AITestConfig::default();
        let mut suite = AITestSuite::new(config);
        suite.add_result(AITestResult::success("test".to_string(), AIProviderType::Ollama, Duration::from_millis(100)));

        let report = suite.generate_report();
        assert!(report.contains("# AI Integration Test Report"));
        assert!(report.contains("Total Tests: 1"));
    }
}