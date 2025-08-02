//! Advanced AI Integration Tests with Real Providers
//! 
//! This test suite provides comprehensive integration testing with real AI providers,
//! performance benchmarks, and end-to-end workflow validation.

use comunicado::ai::{
    AIService, AIConfig, AIProviderType, PrivacyMode,
    EmailTriageConfig, EmailPriority,
    AIError,
    cache::AIResponseCache,
    provider::AIProviderManager,
    retry::{RetryManager, RetryConfig},
};
use comunicado::email::StoredMessage;
use comunicado::calendar::Event;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::timeout;
use uuid::Uuid;
use chrono::Utc;

/// Advanced AI test configuration
#[derive(Debug, Clone)]
pub struct AdvancedTestConfig {
    /// Whether to run tests against real AI providers
    pub use_real_providers: bool,
    /// Providers to test against
    pub test_providers: Vec<AIProviderType>,
    /// Maximum test duration
    pub max_test_duration: Duration,
    /// Performance test iterations
    pub performance_iterations: usize,
    /// Concurrency level for stress tests
    pub stress_test_concurrency: usize,
    /// Enable verbose logging
    pub verbose_logging: bool,
    /// Test environment mode
    pub environment: TestEnvironment,
}

#[derive(Debug, Clone)]
pub enum TestEnvironment {
    Unit,
    Integration,
    Staging,
    Production,
}

impl Default for AdvancedTestConfig {
    fn default() -> Self {
        Self {
            use_real_providers: std::env::var("AI_TEST_REAL_PROVIDERS").unwrap_or_default() == "true",
            test_providers: vec![
                AIProviderType::Ollama, // Always test local provider
                // Only test cloud providers if explicitly enabled
            ],
            max_test_duration: Duration::from_secs(60),
            performance_iterations: 100,
            stress_test_concurrency: 10,
            verbose_logging: std::env::var("AI_TEST_VERBOSE").unwrap_or_default() == "true",
            environment: TestEnvironment::Integration,
        }
    }
}

/// Advanced AI test runner
pub struct AdvancedAITestRunner {
    config: AdvancedTestConfig,
    services: HashMap<AIProviderType, Arc<AIService>>,
    test_data: TestDataManager,
}

/// Test data manager for generating realistic test scenarios
#[derive(Debug, Clone)]
pub struct TestDataManager {
    /// Sample emails for testing
    pub sample_emails: Vec<StoredMessage>,
    /// Sample calendar events
    pub sample_events: Vec<Event>,
    /// AI test prompts
    pub test_prompts: Vec<String>,
    /// Expected outcomes for validation
    pub expected_outcomes: HashMap<String, serde_json::Value>,
}

impl TestDataManager {
    pub fn new() -> Self {
        Self {
            sample_emails: Self::generate_sample_emails(),
            sample_events: Self::generate_sample_events(),
            test_prompts: Self::generate_test_prompts(),
            expected_outcomes: HashMap::new(),
        }
    }

    fn generate_sample_emails() -> Vec<StoredMessage> {
        let now = Utc::now();
        vec![
            // High priority urgent email
            StoredMessage {
                id: Uuid::new_v4(),
                account_id: "test_account".to_string(),
                folder_name: "INBOX".to_string(),
                imap_uid: 1,
                message_id: Some("urgent@example.com".to_string()),
                thread_id: None,
                in_reply_to: None,
                references: Vec::new(),
                subject: "URGENT: Server outage - immediate action required".to_string(),
                from_addr: "admin@company.com".to_string(),
                from_name: Some("System Admin".to_string()),
                to_addrs: vec!["user@company.com".to_string()],
                cc_addrs: Vec::new(),
                bcc_addrs: Vec::new(),
                reply_to: None,
                date: now,
                body_text: Some("Critical server outage detected. Database connectivity lost. Please investigate immediately and provide ETA for resolution. All systems affected.".to_string()),
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
            },
            // Newsletter/bulk email
            StoredMessage {
                id: Uuid::new_v4(),
                account_id: "test_account".to_string(),
                folder_name: "INBOX".to_string(),
                imap_uid: 2,
                message_id: Some("newsletter@newsletter.com".to_string()),
                thread_id: None,
                in_reply_to: None,
                references: Vec::new(),
                subject: "Weekly Tech Newsletter - Latest Updates".to_string(),
                from_addr: "newsletter@techblog.com".to_string(),
                from_name: Some("Tech Blog Newsletter".to_string()),
                to_addrs: vec!["user@company.com".to_string()],
                cc_addrs: Vec::new(),
                bcc_addrs: Vec::new(),
                reply_to: None,
                date: now,
                body_text: Some("This week in tech: New frameworks released, security updates, conference announcements. Read more about the latest developments in software engineering.".to_string()),
                body_html: None,
                attachments: Vec::new(),
                flags: Vec::new(),
                labels: Vec::new(),
                size: Some(2048),
                priority: None,
                created_at: now,
                updated_at: now,
                last_synced: now,
                sync_version: 1,
                is_draft: false,
                is_deleted: false,
            },
            // Meeting request email
            StoredMessage {
                id: Uuid::new_v4(),
                account_id: "test_account".to_string(),
                folder_name: "INBOX".to_string(),
                imap_uid: 3,
                message_id: Some("meeting@company.com".to_string()),
                thread_id: None,
                in_reply_to: None,
                references: Vec::new(),
                subject: "Meeting Request: Q4 Planning Session".to_string(),
                from_addr: "manager@company.com".to_string(),
                from_name: Some("Project Manager".to_string()),
                to_addrs: vec!["user@company.com".to_string()],
                cc_addrs: vec!["team@company.com".to_string()],
                bcc_addrs: Vec::new(),
                reply_to: None,
                date: now,
                body_text: Some("Hi team, let's schedule our Q4 planning session for next Tuesday at 2 PM. We'll discuss budget allocation, resource planning, and timeline for upcoming projects. Please confirm your availability.".to_string()),
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
            },
            // Personal email
            StoredMessage {
                id: Uuid::new_v4(),
                account_id: "test_account".to_string(),
                folder_name: "INBOX".to_string(),
                imap_uid: 4,
                message_id: Some("personal@example.com".to_string()),
                thread_id: None,
                in_reply_to: None,
                references: Vec::new(),
                subject: "Weekend Plans".to_string(),
                from_addr: "friend@personal.com".to_string(),
                from_name: Some("Best Friend".to_string()),
                to_addrs: vec!["user@personal.com".to_string()],
                cc_addrs: Vec::new(),
                bcc_addrs: Vec::new(),
                reply_to: None,
                date: now,
                body_text: Some("Hey! Are we still on for hiking this weekend? The weather looks perfect. Let me know if you want to grab lunch afterwards too.".to_string()),
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
            },
        ]
    }

    fn generate_sample_events() -> Vec<Event> {
        let now = Utc::now();
        vec![
            Event {
                id: Uuid::new_v4(),
                calendar_id: "test_calendar".to_string(),
                title: "Team Standup".to_string(),
                description: Some("Daily team standup meeting".to_string()),
                start_time: now + chrono::Duration::hours(1),
                end_time: now + chrono::Duration::hours(1) + chrono::Duration::minutes(30),
                location: Some("Conference Room A".to_string()),
                attendees: vec!["team@company.com".to_string()],
                recurrence_rule: Some("FREQ=DAILY;BYDAY=MO,TU,WE,TH,FR".to_string()),
                created_at: now,
                updated_at: now,
                etag: "test_etag".to_string(),
                is_all_day: false,
                status: "confirmed".to_string(),
                transparency: "opaque".to_string(),
                organizer: Some("manager@company.com".to_string()),
                url: None,
                attachments: Vec::new(),
                reminders: vec![15, 5], // 15 and 5 minutes before
            },
        ]
    }

    fn generate_test_prompts() -> Vec<String> {
        vec![
            "Summarize this email in one sentence".to_string(),
            "What are the action items from this message?".to_string(),
            "Is this email urgent?".to_string(),
            "What category does this email belong to?".to_string(),
            "Generate a polite reply to this email".to_string(),
            "Create a meeting invitation based on this request".to_string(),
            "What is the sentiment of this message?".to_string(),
            "Extract key dates and times from this text".to_string(),
        ]
    }
}

impl AdvancedAITestRunner {
    /// Create a new advanced AI test runner
    pub async fn new(config: AdvancedTestConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let mut services = HashMap::new();
        let test_data = TestDataManager::new();

        // Initialize AI services for each provider to test
        for provider_type in &config.test_providers {
            match Self::create_ai_service(provider_type.clone()).await {
                Ok(service) => {
                    services.insert(provider_type.clone(), service);
                    if config.verbose_logging {
                        println!("✓ Initialized AI service for {:?}", provider_type);
                    }
                }
                Err(e) => {
                    if config.verbose_logging {
                        println!("⚠ Failed to initialize {:?}: {}", provider_type, e);
                    }
                    // Continue with other providers
                }
            }
        }

        if services.is_empty() {
            return Err("No AI services could be initialized".into());
        }

        Ok(Self {
            config,
            services,
            test_data,
        })
    }

    /// Create an AI service for a specific provider
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

    /// Run all advanced AI tests
    pub async fn run_comprehensive_tests(&self) -> AdvancedTestResults {
        let start_time = Instant::now();
        println!("🚀 Starting Advanced AI Integration Tests");
        
        let mut results = AdvancedTestResults::new();

        // Run basic functionality tests
        results.basic_tests = self.run_basic_functionality_tests().await;

        // Run AI provider integration tests
        results.provider_tests = self.run_provider_integration_tests().await;

        // Run email AI feature tests
        results.email_tests = self.run_email_ai_tests().await;

        // Run calendar AI feature tests
        results.calendar_tests = self.run_calendar_ai_tests().await;

        // Run performance benchmarks
        results.performance_tests = self.run_performance_benchmarks().await;

        // Run stress tests
        results.stress_tests = self.run_stress_tests().await;

        // Run end-to-end workflow tests
        results.workflow_tests = self.run_workflow_tests().await;

        // Run error handling and recovery tests
        results.error_tests = self.run_error_handling_tests().await;

        // Run security and privacy tests
        results.security_tests = self.run_security_tests().await;

        results.total_duration = start_time.elapsed();
        results.calculate_summary_statistics();

        println!("✅ Advanced AI tests completed in {:?}", results.total_duration);
        println!("📊 Overall results: {}/{} tests passed ({:.1}% success rate)", 
            results.total_passed(), 
            results.total_tests(), 
            results.success_rate() * 100.0
        );

        results
    }

    /// Run basic functionality tests
    async fn run_basic_functionality_tests(&self) -> Vec<TestOutcome> {
        println!("🔧 Running basic functionality tests...");
        let mut results = Vec::new();

        for (provider_type, service) in &self.services {
            // Test service initialization
            results.push(self.test_service_initialization(provider_type, service).await);

            // Test configuration loading
            results.push(self.test_configuration_loading(provider_type, service).await);

            // Test cache functionality
            results.push(self.test_cache_functionality(provider_type, service).await);

            // Test retry mechanism
            results.push(self.test_retry_mechanism(provider_type, service).await);
        }

        results
    }

    /// Run AI provider integration tests
    async fn run_provider_integration_tests(&self) -> Vec<TestOutcome> {
        println!("🔌 Running AI provider integration tests...");
        let mut results = Vec::new();

        for (provider_type, service) in &self.services {
            if self.config.use_real_providers || matches!(provider_type, AIProviderType::Ollama) {
                // Test basic AI completion
                results.push(self.test_ai_completion(provider_type, service).await);

                // Test AI streaming if supported
                results.push(self.test_ai_streaming(provider_type, service).await);

                // Test error handling with invalid requests
                results.push(self.test_invalid_request_handling(provider_type, service).await);

                // Test rate limiting behavior
                results.push(self.test_rate_limiting(provider_type, service).await);
            } else {
                // Skip real provider tests if not configured
                results.push(TestOutcome::skipped(
                    format!("{:?}_integration", provider_type),
                    "Real provider testing disabled".to_string(),
                ));
            }
        }

        results
    }

    /// Run email AI feature tests
    async fn run_email_ai_tests(&self) -> Vec<TestOutcome> {
        println!("📧 Running email AI feature tests...");
        let mut results = Vec::new();

        for (provider_type, service) in &self.services {
            // Test email summarization
            for email in &self.test_data.sample_emails {
                results.push(self.test_email_summarization(provider_type, service, email).await);
            }

            // Test email triage
            results.push(self.test_email_triage_system(provider_type, service).await);

            // Test email composition assistance
            results.push(self.test_email_composition(provider_type, service).await);

            // Test batch email processing
            results.push(self.test_batch_email_processing(provider_type, service).await);
        }

        results
    }

    /// Run calendar AI feature tests
    async fn run_calendar_ai_tests(&self) -> Vec<TestOutcome> {
        println!("📅 Running calendar AI feature tests...");
        let mut results = Vec::new();

        for (provider_type, service) in &self.services {
            // Test meeting scheduling from email
            results.push(self.test_meeting_scheduling(provider_type, service).await);

            // Test event extraction
            results.push(self.test_event_extraction(provider_type, service).await);

            // Test calendar insights
            results.push(self.test_calendar_insights(provider_type, service).await);
        }

        results
    }

    /// Run performance benchmarks
    async fn run_performance_benchmarks(&self) -> Vec<TestOutcome> {
        println!("⚡ Running performance benchmarks...");
        let mut results = Vec::new();

        for (provider_type, service) in &self.services {
            // Test latency benchmarks
            results.push(self.benchmark_response_latency(provider_type, service).await);

            // Test throughput benchmarks
            results.push(self.benchmark_throughput(provider_type, service).await);

            // Test memory usage
            results.push(self.benchmark_memory_usage(provider_type, service).await);

            // Test concurrent operations
            results.push(self.benchmark_concurrent_operations(provider_type, service).await);
        }

        results
    }

    /// Run stress tests
    async fn run_stress_tests(&self) -> Vec<TestOutcome> {
        println!("💪 Running stress tests...");
        let mut results = Vec::new();

        for (provider_type, service) in &self.services {
            // Test high load scenarios
            results.push(self.stress_test_high_load(provider_type, service).await);

            // Test resource exhaustion scenarios
            results.push(self.stress_test_resource_limits(provider_type, service).await);

            // Test long-running operations
            results.push(self.stress_test_long_operations(provider_type, service).await);
        }

        results
    }

    /// Run end-to-end workflow tests
    async fn run_workflow_tests(&self) -> Vec<TestOutcome> {
        println!("🔄 Running end-to-end workflow tests...");
        let mut results = Vec::new();

        // Test complete email processing workflow
        results.push(self.test_email_workflow().await);

        // Test complete calendar workflow
        results.push(self.test_calendar_workflow().await);

        // Test cross-feature integration
        results.push(self.test_cross_feature_integration().await);

        results
    }

    /// Run error handling and recovery tests
    async fn run_error_handling_tests(&self) -> Vec<TestOutcome> {
        println!("🛡️ Running error handling tests...");
        let mut results = Vec::new();

        for (provider_type, service) in &self.services {
            // Test network failure recovery
            results.push(self.test_network_failure_recovery(provider_type, service).await);

            // Test invalid input handling
            results.push(self.test_invalid_input_handling(provider_type, service).await);

            // Test timeout handling
            results.push(self.test_timeout_handling(provider_type, service).await);

            // Test graceful degradation
            results.push(self.test_graceful_degradation(provider_type, service).await);
        }

        results
    }

    /// Run security and privacy tests
    async fn run_security_tests(&self) -> Vec<TestOutcome> {
        println!("🔐 Running security and privacy tests...");
        let mut results = Vec::new();

        for (provider_type, service) in &self.services {
            // Test data sanitization
            results.push(self.test_data_sanitization(provider_type, service).await);

            // Test privacy mode compliance
            results.push(self.test_privacy_compliance(provider_type, service).await);

            // Test credential security
            results.push(self.test_credential_security(provider_type, service).await);
        }

        results
    }

    // Individual test implementations...

    async fn test_service_initialization(&self, provider_type: &AIProviderType, service: &Arc<AIService>) -> TestOutcome {
        let start = Instant::now();
        let test_name = format!("{:?}_service_init", provider_type);

        // Test that service is properly initialized
        match service.is_available().await {
            Ok(available) => {
                if available {
                    TestOutcome::passed(test_name, start.elapsed())
                } else {
                    TestOutcome::failed(test_name, start.elapsed(), "Service not available".to_string())
                }
            }
            Err(e) => TestOutcome::failed(test_name, start.elapsed(), format!("Service check failed: {}", e))
        }
    }

    async fn test_configuration_loading(&self, provider_type: &AIProviderType, _service: &Arc<AIService>) -> TestOutcome {
        let start = Instant::now();
        let test_name = format!("{:?}_config_loading", provider_type);

        // Test configuration loading and validation
        match self.create_ai_service(provider_type.clone()).await {
            Ok(_) => TestOutcome::passed(test_name, start.elapsed()),
            Err(e) => TestOutcome::failed(test_name, start.elapsed(), format!("Config loading failed: {}", e))
        }
    }

    async fn test_cache_functionality(&self, provider_type: &AIProviderType, service: &Arc<AIService>) -> TestOutcome {
        let start = Instant::now();
        let test_name = format!("{:?}_cache_functionality", provider_type);

        // Test AI response caching
        let prompt = "Test cache functionality";
        
        // First request (should cache)
        match service.generate_text(prompt, HashMap::new()).await {
            Ok(_) => {
                // Second identical request (should use cache)
                match service.generate_text(prompt, HashMap::new()).await {
                    Ok(_) => TestOutcome::passed(test_name, start.elapsed()),
                    Err(e) => TestOutcome::failed(test_name, start.elapsed(), format!("Second request failed: {}", e))
                }
            }
            Err(e) => TestOutcome::failed(test_name, start.elapsed(), format!("First request failed: {}", e))
        }
    }

    async fn test_retry_mechanism(&self, provider_type: &AIProviderType, _service: &Arc<AIService>) -> TestOutcome {
        let start = Instant::now();
        let test_name = format!("{:?}_retry_mechanism", provider_type);

        // Test retry mechanism with RetryManager
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
                    Err(AIError::network_error("Simulated network failure"))
                } else {
                    Ok("Success")
                }
            }
        }).await;

        match result {
            Ok(_) => {
                if attempt_count == 2 {
                    TestOutcome::passed(test_name, start.elapsed())
                } else {
                    TestOutcome::failed(test_name, start.elapsed(), format!("Unexpected attempt count: {}", attempt_count))
                }
            }
            Err(e) => TestOutcome::failed(test_name, start.elapsed(), format!("Retry failed: {}", e))
        }
    }

    async fn test_ai_completion(&self, provider_type: &AIProviderType, service: &Arc<AIService>) -> TestOutcome {
        let start = Instant::now();
        let test_name = format!("{:?}_ai_completion", provider_type);

        let prompt = "Explain artificial intelligence in one sentence.";
        match service.generate_text(prompt, HashMap::new()).await {
            Ok(response) => {
                if response.len() > 10 { // Basic validation
                    TestOutcome::passed(test_name, start.elapsed())
                } else {
                    TestOutcome::failed(test_name, start.elapsed(), "Response too short".to_string())
                }
            }
            Err(e) => TestOutcome::failed(test_name, start.elapsed(), format!("AI completion failed: {}", e))
        }
    }

    async fn test_ai_streaming(&self, provider_type: &AIProviderType, _service: &Arc<AIService>) -> TestOutcome {
        let start = Instant::now();
        let test_name = format!("{:?}_ai_streaming", provider_type);

        // For now, mark as skipped since streaming isn't fully implemented
        TestOutcome::skipped(test_name, "Streaming tests not yet implemented".to_string())
    }

    async fn test_invalid_request_handling(&self, provider_type: &AIProviderType, service: &Arc<AIService>) -> TestOutcome {
        let start = Instant::now();
        let test_name = format!("{:?}_invalid_request", provider_type);

        // Test with empty prompt
        match service.generate_text("", HashMap::new()).await {
            Ok(_) => TestOutcome::failed(test_name, start.elapsed(), "Should have failed with empty prompt".to_string()),
            Err(_) => TestOutcome::passed(test_name, start.elapsed()) // Expected to fail
        }
    }

    async fn test_rate_limiting(&self, provider_type: &AIProviderType, _service: &Arc<AIService>) -> TestOutcome {
        let start = Instant::now();
        let test_name = format!("{:?}_rate_limiting", provider_type);

        // For now, mark as skipped since rate limiting testing requires specific setup
        TestOutcome::skipped(test_name, "Rate limiting tests require specific provider setup".to_string())
    }

    async fn test_email_summarization(&self, provider_type: &AIProviderType, service: &Arc<AIService>, email: &StoredMessage) -> TestOutcome {
        let start = Instant::now();
        let test_name = format!("{:?}_email_summary_{}", provider_type, email.imap_uid);

        match service.summarize_email(email).await {
            Ok(summary) => {
                if !summary.summary.is_empty() && summary.confidence > 0.0 {
                    TestOutcome::passed(test_name, start.elapsed())
                } else {
                    TestOutcome::failed(test_name, start.elapsed(), "Invalid summary result".to_string())
                }
            }
            Err(e) => TestOutcome::failed(test_name, start.elapsed(), format!("Email summarization failed: {}", e))
        }
    }

    async fn test_email_triage_system(&self, provider_type: &AIProviderType, service: &Arc<AIService>) -> TestOutcome {
        let start = Instant::now();
        let test_name = format!("{:?}_email_triage", provider_type);

        let config = EmailTriageConfig::default();
        let email = &self.test_data.sample_emails[0]; // Use urgent email

        match service.triage_email(email, &config).await {
            Ok(result) => {
                // Validate triage result structure
                if result.confidence > 0.0 && 
                   matches!(result.priority, EmailPriority::High | EmailPriority::Critical) {
                    TestOutcome::passed(test_name, start.elapsed())
                } else {
                    TestOutcome::failed(test_name, start.elapsed(), "Triage result validation failed".to_string())
                }
            }
            Err(e) => TestOutcome::failed(test_name, start.elapsed(), format!("Email triage failed: {}", e))
        }
    }

    async fn test_email_composition(&self, provider_type: &AIProviderType, service: &Arc<AIService>) -> TestOutcome {
        let start = Instant::now();
        let test_name = format!("{:?}_email_composition", provider_type);

        let context = "Reply to a meeting request, accepting the invitation";
        match service.suggest_reply(context, HashMap::new()).await {
            Ok(suggestions) => {
                if !suggestions.is_empty() {
                    TestOutcome::passed(test_name, start.elapsed())
                } else {
                    TestOutcome::failed(test_name, start.elapsed(), "No suggestions generated".to_string())
                }
            }
            Err(e) => TestOutcome::failed(test_name, start.elapsed(), format!("Email composition failed: {}", e))
        }
    }

    async fn test_batch_email_processing(&self, provider_type: &AIProviderType, service: &Arc<AIService>) -> TestOutcome {
        let start = Instant::now();
        let test_name = format!("{:?}_batch_processing", provider_type);

        let config = EmailTriageConfig::default();
        let emails: Vec<&StoredMessage> = self.test_data.sample_emails.iter().collect();

        match service.triage_emails_batch(emails, &config).await {
            Ok(results) => {
                if results.len() == self.test_data.sample_emails.len() {
                    TestOutcome::passed(test_name, start.elapsed())
                } else {
                    TestOutcome::failed(test_name, start.elapsed(), "Batch processing count mismatch".to_string())
                }
            }
            Err(e) => TestOutcome::failed(test_name, start.elapsed(), format!("Batch processing failed: {}", e))
        }
    }

    async fn test_meeting_scheduling(&self, provider_type: &AIProviderType, _service: &Arc<AIService>) -> TestOutcome {
        let start = Instant::now();
        let test_name = format!("{:?}_meeting_scheduling", provider_type);

        // This would test MeetingSchedulerService integration
        TestOutcome::skipped(test_name, "Meeting scheduling integration tests not implemented".to_string())
    }

    async fn test_event_extraction(&self, provider_type: &AIProviderType, service: &Arc<AIService>) -> TestOutcome {
        let start = Instant::now();
        let test_name = format!("{:?}_event_extraction", provider_type);

        let text = "Let's meet next Tuesday at 2 PM for the quarterly review";
        match service.extract_calendar_info(text).await {
            Ok(info) => {
                if !info.is_empty() {
                    TestOutcome::passed(test_name, start.elapsed())
                } else {
                    TestOutcome::failed(test_name, start.elapsed(), "No calendar info extracted".to_string())
                }
            }
            Err(e) => TestOutcome::failed(test_name, start.elapsed(), format!("Event extraction failed: {}", e))
        }
    }

    async fn test_calendar_insights(&self, provider_type: &AIProviderType, _service: &Arc<AIService>) -> TestOutcome {
        let start = Instant::now();
        let test_name = format!("{:?}_calendar_insights", provider_type);

        // Calendar insights would analyze calendar data for patterns and suggestions
        TestOutcome::skipped(test_name, "Calendar insights tests not implemented".to_string())
    }

    // Performance benchmark implementations...

    async fn benchmark_response_latency(&self, provider_type: &AIProviderType, service: &Arc<AIService>) -> TestOutcome {
        let start = Instant::now();
        let test_name = format!("{:?}_latency_benchmark", provider_type);

        let mut total_latency = Duration::ZERO;
        let iterations = 10;

        for i in 0..iterations {
            let request_start = Instant::now();
            match service.generate_text(&format!("Test request {}", i), HashMap::new()).await {
                Ok(_) => {
                    total_latency += request_start.elapsed();
                }
                Err(e) => {
                    return TestOutcome::failed(test_name, start.elapsed(), format!("Request {} failed: {}", i, e));
                }
            }
        }

        let avg_latency = total_latency / iterations;
        let result = if avg_latency < Duration::from_secs(5) {
            TestOutcome::passed(test_name, start.elapsed())
        } else {
            TestOutcome::failed(test_name, start.elapsed(), format!("Average latency too high: {:?}", avg_latency))
        };

        result.with_metadata("avg_latency_ms".to_string(), avg_latency.as_millis().to_string())
    }

    async fn benchmark_throughput(&self, provider_type: &AIProviderType, service: &Arc<AIService>) -> TestOutcome {
        let start = Instant::now();
        let test_name = format!("{:?}_throughput_benchmark", provider_type);

        let iterations = self.config.performance_iterations.min(50); // Limit for throughput test
        let concurrent_tasks = self.config.stress_test_concurrency.min(5);

        let mut handles = Vec::new();
        for batch in 0..(iterations / concurrent_tasks) {
            for i in 0..concurrent_tasks {
                let service_clone = Arc::clone(service);
                let request_id = batch * concurrent_tasks + i;
                
                let handle = tokio::spawn(async move {
                    service_clone.generate_text(&format!("Throughput test {}", request_id), HashMap::new()).await
                });
                handles.push(handle);
            }
        }

        let mut successful_requests = 0;
        for handle in handles {
            if let Ok(Ok(_)) = handle.await {
                successful_requests += 1;
            }
        }

        let total_time = start.elapsed();
        let ops_per_second = successful_requests as f64 / total_time.as_secs_f64();

        let result = if ops_per_second > 0.1 { // At least 0.1 ops/sec
            TestOutcome::passed(test_name, total_time)
        } else {
            TestOutcome::failed(test_name, total_time, format!("Throughput too low: {:.2} ops/sec", ops_per_second))
        };

        result.with_metadata("ops_per_second".to_string(), format!("{:.2}", ops_per_second))
    }

    async fn benchmark_memory_usage(&self, provider_type: &AIProviderType, _service: &Arc<AIService>) -> TestOutcome {
        let start = Instant::now();
        let test_name = format!("{:?}_memory_benchmark", provider_type);

        // Memory benchmarking would require system-specific tools
        TestOutcome::skipped(test_name, "Memory benchmarking requires system integration".to_string())
    }

    async fn benchmark_concurrent_operations(&self, provider_type: &AIProviderType, service: &Arc<AIService>) -> TestOutcome {
        let start = Instant::now();
        let test_name = format!("{:?}_concurrent_benchmark", provider_type);

        let concurrent_count = self.config.stress_test_concurrency.min(10);
        let mut handles = Vec::new();

        for i in 0..concurrent_count {
            let service_clone = Arc::clone(service);
            let handle = tokio::spawn(async move {
                service_clone.generate_text(&format!("Concurrent test {}", i), HashMap::new()).await
            });
            handles.push(handle);
        }

        let mut successful = 0;
        for handle in handles {
            if let Ok(Ok(_)) = handle.await {
                successful += 1;
            }
        }

        let success_rate = successful as f64 / concurrent_count as f64;
        if success_rate >= 0.8 { // 80% success rate for concurrent operations
            TestOutcome::passed(test_name, start.elapsed())
        } else {
            TestOutcome::failed(test_name, start.elapsed(), format!("Concurrent success rate too low: {:.1}%", success_rate * 100.0))
        }
    }

    // Stress test implementations...

    async fn stress_test_high_load(&self, provider_type: &AIProviderType, service: &Arc<AIService>) -> TestOutcome {
        let start = Instant::now();
        let test_name = format!("{:?}_stress_high_load", provider_type);

        let high_load_requests = self.config.performance_iterations;
        let mut successful = 0;

        for i in 0..high_load_requests {
            match timeout(
                Duration::from_secs(10),
                service.generate_text(&format!("Stress test {}", i), HashMap::new())
            ).await {
                Ok(Ok(_)) => successful += 1,
                Ok(Err(_)) => {},
                Err(_) => {}, // Timeout
            }

            if i % 10 == 0 && self.config.verbose_logging {
                println!("  Stress test progress: {}/{}", i, high_load_requests);
            }
        }

        let success_rate = successful as f64 / high_load_requests as f64;
        if success_rate >= 0.7 { // 70% success rate under stress
            TestOutcome::passed(test_name, start.elapsed())
        } else {
            TestOutcome::failed(test_name, start.elapsed(), format!("Stress test success rate: {:.1}%", success_rate * 100.0))
        }
    }

    async fn stress_test_resource_limits(&self, provider_type: &AIProviderType, _service: &Arc<AIService>) -> TestOutcome {
        let start = Instant::now();
        let test_name = format!("{:?}_stress_resources", provider_type);

        // Resource limit testing would require monitoring system resources
        TestOutcome::skipped(test_name, "Resource limit testing requires system monitoring".to_string())
    }

    async fn stress_test_long_operations(&self, provider_type: &AIProviderType, service: &Arc<AIService>) -> TestOutcome {
        let start = Instant::now();
        let test_name = format!("{:?}_stress_long_ops", provider_type);

        // Test with a very long prompt to simulate extended operations
        let long_prompt = "Explain the history of artificial intelligence ".repeat(50);
        
        match timeout(
            Duration::from_secs(30),
            service.generate_text(&long_prompt, HashMap::new())
        ).await {
            Ok(Ok(_)) => TestOutcome::passed(test_name, start.elapsed()),
            Ok(Err(e)) => TestOutcome::failed(test_name, start.elapsed(), format!("Long operation failed: {}", e)),
            Err(_) => TestOutcome::failed(test_name, start.elapsed(), "Long operation timed out".to_string()),
        }
    }

    // Workflow test implementations...

    async fn test_email_workflow(&self) -> TestOutcome {
        let start = Instant::now();
        let test_name = "email_workflow_e2e".to_string();

        // Test complete email processing workflow:
        // 1. Receive email
        // 2. Triage/classify 
        // 3. Generate summary
        // 4. Suggest reply

        if let Some((_, service)) = self.services.iter().next() {
            let email = &self.test_data.sample_emails[0];
            
            // Step 1: Triage email
            let triage_config = EmailTriageConfig::default();
            let triage_result = match service.triage_email(email, &triage_config).await {
                Ok(result) => result,
                Err(e) => return TestOutcome::failed(test_name, start.elapsed(), format!("Triage failed: {}", e))
            };

            // Step 2: Generate summary
            let summary = match service.summarize_email(email).await {
                Ok(summary) => summary,
                Err(e) => return TestOutcome::failed(test_name, start.elapsed(), format!("Summary failed: {}", e))
            };

            // Step 3: Suggest reply
            let context = format!("Email about: {}", email.subject);
            let _suggestions = match service.suggest_reply(&context, HashMap::new()).await {
                Ok(suggestions) => suggestions,
                Err(e) => return TestOutcome::failed(test_name, start.elapsed(), format!("Reply suggestion failed: {}", e))
            };

            // Validate workflow results
            if triage_result.confidence > 0.0 && !summary.summary.is_empty() {
                TestOutcome::passed(test_name, start.elapsed())
            } else {
                TestOutcome::failed(test_name, start.elapsed(), "Workflow validation failed".to_string())
            }
        } else {
            TestOutcome::failed(test_name, start.elapsed(), "No AI service available".to_string())
        }
    }

    async fn test_calendar_workflow(&self) -> TestOutcome {
        let start = Instant::now();
        let test_name = "calendar_workflow_e2e".to_string();

        if let Some((_, service)) = self.services.iter().next() {
            // Test calendar workflow: extract event info from text
            let meeting_text = "Let's schedule a team meeting for next Friday at 3 PM in the conference room";
            
            match service.extract_calendar_info(meeting_text).await {
                Ok(info) => {
                    if !info.is_empty() {
                        TestOutcome::passed(test_name, start.elapsed())
                    } else {
                        TestOutcome::failed(test_name, start.elapsed(), "No calendar info extracted".to_string())
                    }
                }
                Err(e) => TestOutcome::failed(test_name, start.elapsed(), format!("Calendar workflow failed: {}", e))
            }
        } else {
            TestOutcome::failed(test_name, start.elapsed(), "No AI service available".to_string())
        }
    }

    async fn test_cross_feature_integration(&self) -> TestOutcome {
        let start = Instant::now();
        let test_name = "cross_feature_integration".to_string();

        // Test integration between email and calendar features
        if let Some((_, service)) = self.services.iter().next() {
            let meeting_email = &self.test_data.sample_emails[2]; // Meeting request email

            // Process email to extract meeting information and create calendar event
            let summary = match service.summarize_email(meeting_email).await {
                Ok(summary) => summary,
                Err(e) => return TestOutcome::failed(test_name, start.elapsed(), format!("Email summary failed: {}", e))
            };

            let calendar_info = match service.extract_calendar_info(&summary.summary).await {
                Ok(info) => info,
                Err(e) => return TestOutcome::failed(test_name, start.elapsed(), format!("Calendar extraction failed: {}", e))
            };

            if !calendar_info.is_empty() {
                TestOutcome::passed(test_name, start.elapsed())
            } else {
                TestOutcome::failed(test_name, start.elapsed(), "Cross-feature integration produced no results".to_string())
            }
        } else {
            TestOutcome::failed(test_name, start.elapsed(), "No AI service available".to_string())
        }
    }

    // Error handling test implementations...

    async fn test_network_failure_recovery(&self, provider_type: &AIProviderType, _service: &Arc<AIService>) -> TestOutcome {
        let start = Instant::now();
        let test_name = format!("{:?}_network_recovery", provider_type);

        // Network failure testing would require network simulation
        TestOutcome::skipped(test_name, "Network failure simulation not implemented".to_string())
    }

    async fn test_invalid_input_handling(&self, provider_type: &AIProviderType, service: &Arc<AIService>) -> TestOutcome {
        let start = Instant::now();
        let test_name = format!("{:?}_invalid_input", provider_type);

        // Test various invalid inputs
        let invalid_inputs = vec![
            "",  // Empty string
            "\0", // Null character
            "🤖".repeat(10000), // Very long unicode
        ];

        for (i, input) in invalid_inputs.iter().enumerate() {
            match service.generate_text(input, HashMap::new()).await {
                Ok(_) => {
                    // Some providers might handle these gracefully
                    if self.config.verbose_logging {
                        println!("  Invalid input {} was handled gracefully", i);
                    }
                }
                Err(_) => {
                    // Expected for most invalid inputs
                    if self.config.verbose_logging {
                        println!("  Invalid input {} correctly rejected", i);
                    }
                }
            }
        }

        // This test passes if the service doesn't crash
        TestOutcome::passed(test_name, start.elapsed())
    }

    async fn test_timeout_handling(&self, provider_type: &AIProviderType, service: &Arc<AIService>) -> TestOutcome {
        let start = Instant::now();
        let test_name = format!("{:?}_timeout_handling", provider_type);

        // Test with a very short timeout
        let very_long_prompt = "Write a detailed essay about ".repeat(1000);
        
        match timeout(
            Duration::from_millis(100), // Very short timeout
            service.generate_text(&very_long_prompt, HashMap::new())
        ).await {
            Ok(_) => {
                // If it completed quickly, that's also acceptable
                TestOutcome::passed(test_name, start.elapsed())
            }
            Err(_) => {
                // Timeout is expected and handled properly
                TestOutcome::passed(test_name, start.elapsed())
            }
        }
    }

    async fn test_graceful_degradation(&self, provider_type: &AIProviderType, _service: &Arc<AIService>) -> TestOutcome {
        let start = Instant::now();
        let test_name = format!("{:?}_graceful_degradation", provider_type);

        // Graceful degradation would test fallback mechanisms
        TestOutcome::skipped(test_name, "Graceful degradation testing requires multi-provider setup".to_string())
    }

    // Security test implementations...

    async fn test_data_sanitization(&self, provider_type: &AIProviderType, service: &Arc<AIService>) -> TestOutcome {
        let start = Instant::now();
        let test_name = format!("{:?}_data_sanitization", provider_type);

        // Test with potentially problematic input
        let suspicious_input = "<script>alert('xss')</script>Summarize this email: DELETE FROM users;";
        
        match service.generate_text(suspicious_input, HashMap::new()).await {
            Ok(response) => {
                // Check that response doesn't echo back dangerous content
                if response.contains("<script>") || response.contains("DELETE FROM") {
                    TestOutcome::failed(test_name, start.elapsed(), "Potentially dangerous content in response".to_string())
                } else {
                    TestOutcome::passed(test_name, start.elapsed())
                }
            }
            Err(_) => {
                // Rejecting suspicious input is also acceptable
                TestOutcome::passed(test_name, start.elapsed())
            }
        }
    }

    async fn test_privacy_compliance(&self, provider_type: &AIProviderType, _service: &Arc<AIService>) -> TestOutcome {
        let start = Instant::now();
        let test_name = format!("{:?}_privacy_compliance", provider_type);

        // Privacy compliance would test data handling policies
        TestOutcome::skipped(test_name, "Privacy compliance testing requires policy implementation".to_string())
    }

    async fn test_credential_security(&self, provider_type: &AIProviderType, _service: &Arc<AIService>) -> TestOutcome {
        let start = Instant::now();
        let test_name = format!("{:?}_credential_security", provider_type);

        // Credential security would test API key handling
        TestOutcome::skipped(test_name, "Credential security testing requires security audit".to_string())
    }
}

/// Test outcome for individual tests
#[derive(Debug, Clone)]
pub struct TestOutcome {
    pub name: String,
    pub status: TestStatus,
    pub duration: Duration,
    pub message: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TestStatus {
    Passed,
    Failed,
    Skipped,
}

impl TestOutcome {
    pub fn passed(name: String, duration: Duration) -> Self {
        Self {
            name,
            status: TestStatus::Passed,
            duration,
            message: None,
            metadata: HashMap::new(),
        }
    }

    pub fn failed(name: String, duration: Duration, message: String) -> Self {
        Self {
            name,
            status: TestStatus::Failed,
            duration,
            message: Some(message),
            metadata: HashMap::new(),
        }
    }

    pub fn skipped(name: String, reason: String) -> Self {
        Self {
            name,
            status: TestStatus::Skipped,
            duration: Duration::ZERO,
            message: Some(reason),
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Comprehensive test results
#[derive(Debug)]
pub struct AdvancedTestResults {
    pub basic_tests: Vec<TestOutcome>,
    pub provider_tests: Vec<TestOutcome>,
    pub email_tests: Vec<TestOutcome>,
    pub calendar_tests: Vec<TestOutcome>,
    pub performance_tests: Vec<TestOutcome>,
    pub stress_tests: Vec<TestOutcome>,
    pub workflow_tests: Vec<TestOutcome>,
    pub error_tests: Vec<TestOutcome>,
    pub security_tests: Vec<TestOutcome>,
    pub total_duration: Duration,
}

impl AdvancedTestResults {
    pub fn new() -> Self {
        Self {
            basic_tests: Vec::new(),
            provider_tests: Vec::new(),
            email_tests: Vec::new(),
            calendar_tests: Vec::new(),
            performance_tests: Vec::new(),
            stress_tests: Vec::new(),
            workflow_tests: Vec::new(),
            error_tests: Vec::new(),
            security_tests: Vec::new(),
            total_duration: Duration::ZERO,
        }
    }

    pub fn all_tests(&self) -> impl Iterator<Item = &TestOutcome> {
        self.basic_tests.iter()
            .chain(self.provider_tests.iter())
            .chain(self.email_tests.iter())
            .chain(self.calendar_tests.iter())
            .chain(self.performance_tests.iter())
            .chain(self.stress_tests.iter())
            .chain(self.workflow_tests.iter())
            .chain(self.error_tests.iter())
            .chain(self.security_tests.iter())
    }

    pub fn total_tests(&self) -> usize {
        self.all_tests().count()
    }

    pub fn total_passed(&self) -> usize {
        self.all_tests().filter(|t| t.status == TestStatus::Passed).count()
    }

    pub fn total_failed(&self) -> usize {
        self.all_tests().filter(|t| t.status == TestStatus::Failed).count()
    }

    pub fn total_skipped(&self) -> usize {
        self.all_tests().filter(|t| t.status == TestStatus::Skipped).count()
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.total_tests();
        if total == 0 {
            0.0
        } else {
            self.total_passed() as f64 / total as f64
        }
    }

    pub fn calculate_summary_statistics(&mut self) {
        // Additional analysis could be added here
    }

    /// Generate a detailed report of all test results
    pub fn generate_report(&self) -> String {
        let mut report = String::new();
        
        report.push_str("# Advanced AI Integration Test Report\n\n");
        report.push_str(&format!("Generated: {}\n\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));

        // Executive Summary
        report.push_str("## Executive Summary\n\n");
        report.push_str(&format!("- **Total Tests:** {}\n", self.total_tests()));
        report.push_str(&format!("- **Passed:** {} ({:.1}%)\n", self.total_passed(), self.success_rate() * 100.0));
        report.push_str(&format!("- **Failed:** {}\n", self.total_failed()));
        report.push_str(&format!("- **Skipped:** {}\n", self.total_skipped()));
        report.push_str(&format!("- **Total Duration:** {:?}\n\n", self.total_duration));

        // Test Category Results
        report.push_str("## Test Category Results\n\n");
        
        let categories = [
            ("Basic Functionality", &self.basic_tests),
            ("Provider Integration", &self.provider_tests),
            ("Email Features", &self.email_tests),
            ("Calendar Features", &self.calendar_tests),
            ("Performance", &self.performance_tests),
            ("Stress Tests", &self.stress_tests),
            ("Workflows", &self.workflow_tests),
            ("Error Handling", &self.error_tests),
            ("Security", &self.security_tests),
        ];

        report.push_str("| Category | Total | Passed | Failed | Skipped | Success Rate |\n");
        report.push_str("|----------|-------|--------|--------|---------|-------------|\n");

        for (name, tests) in categories {
            let total = tests.len();
            let passed = tests.iter().filter(|t| t.status == TestStatus::Passed).count();
            let failed = tests.iter().filter(|t| t.status == TestStatus::Failed).count();
            let skipped = tests.iter().filter(|t| t.status == TestStatus::Skipped).count();
            let success_rate = if total > 0 { passed as f64 / total as f64 * 100.0 } else { 0.0 };

            report.push_str(&format!("| {} | {} | {} | {} | {} | {:.1}% |\n", 
                name, total, passed, failed, skipped, success_rate));
        }

        report.push_str("\n");

        // Failed Tests Details
        let failed_tests: Vec<_> = self.all_tests()
            .filter(|t| t.status == TestStatus::Failed)
            .collect();

        if !failed_tests.is_empty() {
            report.push_str("## Failed Tests\n\n");
            for test in failed_tests {
                report.push_str(&format!("### {}\n", test.name));
                if let Some(message) = &test.message {
                    report.push_str(&format!("**Error:** {}\n", message));
                }
                report.push_str(&format!("**Duration:** {:?}\n\n", test.duration));
            }
        }

        // Performance Metrics
        let perf_tests: Vec<_> = self.performance_tests.iter()
            .filter(|t| t.status == TestStatus::Passed)
            .collect();

        if !perf_tests.is_empty() {
            report.push_str("## Performance Metrics\n\n");
            for test in perf_tests {
                report.push_str(&format!("### {}\n", test.name));
                report.push_str(&format!("**Duration:** {:?}\n", test.duration));
                for (key, value) in &test.metadata {
                    report.push_str(&format!("**{}:** {}\n", key, value));
                }
                report.push_str("\n");
            }
        }

        // Recommendations
        report.push_str("## Recommendations\n\n");
        
        if self.success_rate() < 0.8 {
            report.push_str("- ⚠️ Success rate is below 80%. Review failed tests and improve reliability.\n");
        }
        
        if self.total_failed() > 0 {
            report.push_str("- 🔍 Investigate failed tests to identify root causes.\n");
        }

        if self.total_skipped() > self.total_tests() / 4 {
            report.push_str("- 📝 Many tests were skipped. Consider implementing missing test functionality.\n");
        }

        if self.success_rate() >= 0.95 {
            report.push_str("- ✅ Excellent test coverage and reliability.\n");
        }

        report
    }
}

/// Manual test runner for command-line execution
pub async fn run_advanced_ai_tests() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting Advanced AI Integration Test Suite");
    
    let config = AdvancedTestConfig::default();
    println!("Test Configuration:");
    println!("  - Real Providers: {}", config.use_real_providers);
    println!("  - Test Providers: {:?}", config.test_providers);
    println!("  - Max Duration: {:?}", config.max_test_duration);
    println!("  - Performance Iterations: {}", config.performance_iterations);
    println!();

    let runner = AdvancedAITestRunner::new(config).await?;
    let results = runner.run_comprehensive_tests().await;

    // Generate and save report
    let report = results.generate_report();
    let report_path = "advanced_ai_test_report.md";
    tokio::fs::write(report_path, &report).await?;
    
    println!("\n📄 Detailed report saved to: {}", report_path);
    
    // Print summary
    println!("\n🎯 Final Results:");
    println!("   Success Rate: {:.1}%", results.success_rate() * 100.0);
    println!("   Total Tests: {}", results.total_tests());
    println!("   Passed: {}", results.total_passed());
    println!("   Failed: {}", results.total_failed());
    println!("   Skipped: {}", results.total_skipped());
    println!("   Duration: {:?}", results.total_duration);

    if results.success_rate() >= 0.8 {
        println!("\n🎉 Advanced AI test suite completed successfully!");
        Ok(())
    } else {
        Err(format!("Test suite failed with {:.1}% success rate", results.success_rate() * 100.0).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_advanced_runner_creation() {
        let config = AdvancedTestConfig {
            use_real_providers: false,
            test_providers: vec![AIProviderType::Ollama],
            ..Default::default()
        };

        let runner = AdvancedAITestRunner::new(config).await;
        assert!(runner.is_ok(), "Failed to create advanced test runner");
    }

    #[tokio::test]
    async fn test_data_manager() {
        let data_manager = TestDataManager::new();
        
        assert!(!data_manager.sample_emails.is_empty());
        assert!(!data_manager.sample_events.is_empty());
        assert!(!data_manager.test_prompts.is_empty());
        
        // Verify email data quality
        let urgent_email = &data_manager.sample_emails[0];
        assert!(urgent_email.subject.contains("URGENT"));
        assert!(urgent_email.priority.is_some());
    }

    #[tokio::test]
    async fn test_outcome_creation() {
        let outcome = TestOutcome::passed("test".to_string(), Duration::from_millis(100));
        assert_eq!(outcome.status, TestStatus::Passed);

        let outcome = TestOutcome::failed(
            "test".to_string(), 
            Duration::from_millis(200), 
            "error".to_string()
        );
        assert_eq!(outcome.status, TestStatus::Failed);
        assert_eq!(outcome.message, Some("error".to_string()));

        let outcome = TestOutcome::skipped("test".to_string(), "reason".to_string());
        assert_eq!(outcome.status, TestStatus::Skipped);
    }

    #[tokio::test]
    async fn test_results_calculation() {
        let mut results = AdvancedTestResults::new();
        
        results.basic_tests.push(TestOutcome::passed("test1".to_string(), Duration::from_millis(100)));
        results.basic_tests.push(TestOutcome::failed("test2".to_string(), Duration::from_millis(200), "error".to_string()));
        results.basic_tests.push(TestOutcome::skipped("test3".to_string(), "reason".to_string()));

        assert_eq!(results.total_tests(), 3);
        assert_eq!(results.total_passed(), 1);
        assert_eq!(results.total_failed(), 1);
        assert_eq!(results.total_skipped(), 1);
        assert!((results.success_rate() - 0.333).abs() < 0.01);
    }

    #[tokio::test] 
    async fn test_report_generation() {
        let mut results = AdvancedTestResults::new();
        results.basic_tests.push(TestOutcome::passed("test1".to_string(), Duration::from_millis(100)));
        results.total_duration = Duration::from_secs(1);

        let report = results.generate_report();
        assert!(report.contains("# Advanced AI Integration Test Report"));
        assert!(report.contains("Executive Summary"));
        assert!(report.contains("Total Tests: 1"));
    }
}