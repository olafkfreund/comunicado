//! AI testing framework and utilities
//! 
//! This module provides comprehensive testing infrastructure for AI functionality,
//! including mock providers, integration tests, performance benchmarks, and UI testing utilities.

pub mod mock_providers;
pub mod integration_tests;
pub mod performance_tests;
pub mod ui_tests;
pub mod test_utilities;
pub mod comprehensive_test_runner;

// Re-export testing utilities for convenient access
pub use mock_providers::{MockAIProvider, MockProviderBehavior, MockResponse};
pub use test_utilities::{
    create_test_ai_config, create_test_ai_service, create_mock_email_content,
    create_mock_calendar_event, AITestContext, TestScenario
};
pub use integration_tests::{AIIntegrationTestRunner, TestResult};
pub use performance_tests::{AIPerformanceBenchmark, BenchmarkResult};
pub use comprehensive_test_runner::{ComprehensiveAITestRunner, ComprehensiveTestResults, TestRunnerConfig};

use crate::ai::{AIConfig, AIProviderType};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Test configuration builder for AI testing scenarios
pub struct AITestConfigBuilder {
    config: AIConfig,
}

impl AITestConfigBuilder {
    /// Create a new test configuration builder
    pub fn new() -> Self {
        Self {
            config: AIConfig {
                enabled: true,
                provider: AIProviderType::None,
                ..AIConfig::default()
            },
        }
    }

    /// Set the AI provider for testing
    pub fn with_provider(mut self, provider: AIProviderType) -> Self {
        self.config.provider = provider;
        self
    }

    /// Enable/disable AI functionality
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.config.enabled = enabled;
        self
    }

    /// Set creativity level
    pub fn with_creativity(mut self, creativity: f32) -> Self {
        self.config.creativity = creativity;
        self
    }

    /// Set maximum context length
    pub fn with_context_length(mut self, length: usize) -> Self {
        self.config.max_context_length = length;
        self
    }

    /// Enable email features
    pub fn with_email_features(mut self, enabled: bool) -> Self {
        self.config.email_suggestions_enabled = enabled;
        self.config.email_summarization_enabled = enabled;
        self.config.email_categorization_enabled = enabled;
        self
    }

    /// Enable calendar features
    pub fn with_calendar_features(mut self, enabled: bool) -> Self {
        self.config.calendar_assistance_enabled = enabled;
        self
    }

    /// Build the test configuration
    pub fn build(self) -> AIConfig {
        self.config
    }
}

impl Default for AITestConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Test scenario categories for AI functionality
#[derive(Debug, Clone, PartialEq)]
pub enum AITestCategory {
    /// Email-related AI functionality
    Email,
    /// Calendar-related AI functionality
    Calendar,
    /// Configuration and settings
    Configuration,
    /// Privacy and consent
    Privacy,
    /// Performance and reliability
    Performance,
    /// Integration between components
    Integration,
}

/// Test execution environment for AI tests
pub struct AITestEnvironment {
    /// Test configuration
    pub config: Arc<RwLock<AIConfig>>,
    /// Mock provider behaviors
    pub mock_behaviors: Arc<RwLock<Vec<MockProviderBehavior>>>,
    /// Test data directory
    pub test_data_dir: std::path::PathBuf,
    /// Temporary directory for test files
    pub temp_dir: tempfile::TempDir,
}

impl AITestEnvironment {
    /// Create a new test environment
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = tempfile::tempdir()?;
        let test_data_dir = temp_dir.path().join("test_data");
        tokio::fs::create_dir_all(&test_data_dir).await?;

        Ok(Self {
            config: Arc::new(RwLock::new(AIConfig::default())),
            mock_behaviors: Arc::new(RwLock::new(Vec::new())),
            test_data_dir,
            temp_dir,
        })
    }

    /// Update test configuration
    pub async fn set_config(&self, config: AIConfig) {
        let mut current_config = self.config.write().await;
        *current_config = config;
    }

    /// Add mock provider behavior
    pub async fn add_mock_behavior(&self, behavior: MockProviderBehavior) {
        let mut behaviors = self.mock_behaviors.write().await;
        behaviors.push(behavior);
    }

    /// Clear all mock behaviors
    pub async fn clear_mock_behaviors(&self) {
        let mut behaviors = self.mock_behaviors.write().await;
        behaviors.clear();
    }

    /// Get test data file path
    pub fn get_test_data_path(&self, filename: &str) -> std::path::PathBuf {
        self.test_data_dir.join(filename)
    }

    /// Create test email content file
    pub async fn create_test_email_file(&self, filename: &str, content: &str) -> Result<std::path::PathBuf, std::io::Error> {
        let file_path = self.get_test_data_path(filename);
        tokio::fs::write(&file_path, content).await?;
        Ok(file_path)
    }

    /// Create test configuration file
    pub async fn create_test_config_file(&self, config: &AIConfig) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        let config_path = self.get_test_data_path("ai_config.toml");
        config.save_to_file(&config_path).await?;
        Ok(config_path)
    }
}

/// Test assertion utilities for AI functionality
pub struct AITestAssertions;

impl AITestAssertions {
    /// Assert that an AI response contains expected content
    pub fn assert_response_contains(response: &str, expected: &str) {
        assert!(
            response.contains(expected),
            "AI response '{}' does not contain expected content '{}'",
            response,
            expected
        );
    }

    /// Assert that an AI response has minimum length
    pub fn assert_response_min_length(response: &str, min_length: usize) {
        assert!(
            response.len() >= min_length,
            "AI response length {} is less than minimum {}",
            response.len(),
            min_length
        );
    }

    /// Assert that an AI response has maximum length
    pub fn assert_response_max_length(response: &str, max_length: usize) {
        assert!(
            response.len() <= max_length,
            "AI response length {} exceeds maximum {}",
            response.len(),
            max_length
        );
    }

    /// Assert that a configuration is valid
    pub fn assert_config_valid(config: &AIConfig) {
        assert!(
            config.validate().is_ok(),
            "AI configuration is invalid: {:?}",
            config.validate().err()
        );
    }

    /// Assert that a provider is available
    pub async fn assert_provider_available<T>(provider: &T) -> bool 
    where 
        T: crate::ai::provider::AIProvider + Send + Sync,
    {
        provider.is_available().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let config = AITestConfigBuilder::new()
            .with_provider(AIProviderType::Ollama)
            .with_enabled(true)
            .with_creativity(0.8)
            .with_email_features(true)
            .build();

        assert!(config.enabled);
        assert_eq!(config.provider, AIProviderType::Ollama);
        assert_eq!(config.creativity, 0.8);
        assert!(config.email_suggestions_enabled);
    }

    #[tokio::test]
    async fn test_environment_creation() {
        let env = AITestEnvironment::new().await.unwrap();
        assert!(env.test_data_dir.exists());
        
        let test_file = env.get_test_data_path("test.txt");
        assert_eq!(test_file.file_name().unwrap(), "test.txt");
    }

    #[test]
    fn test_assertions() {
        AITestAssertions::assert_response_contains("Hello world", "world");
        AITestAssertions::assert_response_min_length("Hello", 3);
        AITestAssertions::assert_response_max_length("Hi", 10);
        
        let config = AIConfig::default();
        AITestAssertions::assert_config_valid(&config);
    }
}