//! Mock AI providers for testing

use crate::ai::{
    error::{AIError, AIResult},
    provider::AIProvider,
    AIContext, AIConfig,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Mock AI provider for testing scenarios
pub struct MockAIProvider {
    /// Provider name
    name: String,
    /// Current behavior configuration
    behavior: Arc<RwLock<MockProviderBehavior>>,
    /// Call history for verification
    call_history: Arc<RwLock<Vec<MockCall>>>,
}

/// Mock provider behavior configuration
#[derive(Debug, Clone)]
pub struct MockProviderBehavior {
    /// Whether the provider should appear available
    pub available: bool,
    /// Simulated latency for responses
    pub latency: Duration,
    /// Predefined responses for specific prompts
    pub responses: HashMap<String, MockResponse>,
    /// Default response when no specific match found
    pub default_response: MockResponse,
    /// Whether to simulate errors
    pub error_rate: f32, // 0.0 to 1.0
    /// Error to return when simulating failures
    pub error_type: AIError,
    /// Maximum context length to simulate
    pub max_context_length: usize,
}

/// Mock response configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockResponse {
    /// Response content
    pub content: String,
    /// Whether this response should trigger an error
    pub is_error: bool,
    /// Simulated processing time
    pub processing_time: Option<Duration>,
    /// Response metadata
    pub metadata: HashMap<String, String>,
}

/// Record of a call made to the mock provider
#[derive(Debug, Clone)]
pub struct MockCall {
    /// When the call was made
    pub timestamp: std::time::Instant,
    /// The prompt that was sent
    pub prompt: String,
    /// Context that was provided
    pub context: Option<AIContext>,
    /// Response that was returned
    pub response: Result<String, AIError>,
    /// Processing duration
    pub duration: Duration,
}

impl MockAIProvider {
    /// Create a new mock provider
    pub fn new(name: String) -> Self {
        Self {
            name,
            behavior: Arc::new(RwLock::new(MockProviderBehavior::default())),
            call_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create a mock provider with specific behavior
    pub fn with_behavior(name: String, behavior: MockProviderBehavior) -> Self {
        Self {
            name,
            behavior: Arc::new(RwLock::new(behavior)),
            call_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Update provider behavior
    pub async fn set_behavior(&self, behavior: MockProviderBehavior) {
        let mut current_behavior = self.behavior.write().await;
        *current_behavior = behavior;
    }

    /// Get call history
    pub async fn get_call_history(&self) -> Vec<MockCall> {
        self.call_history.read().await.clone()
    }

    /// Clear call history
    pub async fn clear_call_history(&self) {
        self.call_history.write().await.clear();
    }

    /// Get number of calls made
    pub async fn call_count(&self) -> usize {
        self.call_history.read().await.len()
    }

    /// Check if a specific prompt was called
    pub async fn was_called_with(&self, prompt: &str) -> bool {
        let history = self.call_history.read().await;
        history.iter().any(|call| call.prompt.contains(prompt))
    }

    /// Get the most recent call
    pub async fn last_call(&self) -> Option<MockCall> {
        let history = self.call_history.read().await;
        history.last().cloned()
    }

    /// Add a response for a specific prompt
    pub async fn add_response(&self, prompt_pattern: String, response: MockResponse) {
        let mut behavior = self.behavior.write().await;
        behavior.responses.insert(prompt_pattern, response);
    }

    /// Set default response for unmatched prompts
    pub async fn set_default_response(&self, response: MockResponse) {
        let mut behavior = self.behavior.write().await;
        behavior.default_response = response;
    }

    /// Set error rate for simulating failures
    pub async fn set_error_rate(&self, rate: f32) {
        let mut behavior = self.behavior.write().await;
        behavior.error_rate = rate.clamp(0.0, 1.0);
    }

    /// Set simulated latency
    pub async fn set_latency(&self, latency: Duration) {
        let mut behavior = self.behavior.write().await;
        behavior.latency = latency;
    }

    /// Record a call in the history
    async fn record_call(&self, call: MockCall) {
        let mut history = self.call_history.write().await;
        history.push(call);
    }

    /// Find matching response for a prompt
    async fn find_response(&self, prompt: &str) -> MockResponse {
        let behavior = self.behavior.read().await;
        
        // Check for specific response patterns
        for (pattern, response) in &behavior.responses {
            if prompt.contains(pattern) {
                return response.clone();
            }
        }
        
        // Return default response
        behavior.default_response.clone()
    }

    /// Simulate error based on error rate
    async fn should_simulate_error(&self) -> bool {
        let behavior = self.behavior.read().await;
        if behavior.error_rate <= 0.0 {
            return false;
        }
        
        use rand::Rng;
        let mut rng = rand::thread_rng();
        rng.gen::<f32>() < behavior.error_rate
    }
}

#[async_trait]
impl AIProvider for MockAIProvider {
    async fn is_available(&self) -> bool {
        let behavior = self.behavior.read().await;
        behavior.available
    }

    async fn complete_text(&self, prompt: &str, context: Option<&AIContext>) -> AIResult<String> {
        let start_time = std::time::Instant::now();
        
        // Simulate latency
        let behavior = self.behavior.read().await;
        let latency = behavior.latency;
        drop(behavior);
        
        if latency > Duration::ZERO {
            tokio::time::sleep(latency).await;
        }

        // Check if we should simulate an error
        if self.should_simulate_error().await {
            let behavior = self.behavior.read().await;
            let error = behavior.error_type.clone();
            drop(behavior);

            let call = MockCall {
                timestamp: start_time,
                prompt: prompt.to_string(),
                context: context.cloned(),
                response: Err(error.clone()),
                duration: start_time.elapsed(),
            };
            self.record_call(call).await;
            return Err(error);
        }

        // Find appropriate response
        let mock_response = self.find_response(prompt).await;
        
        // Simulate additional processing time if specified
        if let Some(processing_time) = mock_response.processing_time {
            tokio::time::sleep(processing_time).await;
        }

        let result = if mock_response.is_error {
            let error = AIError::provider_error("Mock error response".to_string());
            Err(error.clone())
        } else {
            Ok(mock_response.content.clone())
        };

        // Record the call
        let call = MockCall {
            timestamp: start_time,
            prompt: prompt.to_string(),
            context: context.cloned(),
            response: result.clone(),
            duration: start_time.elapsed(),
        };
        self.record_call(call).await;

        result
    }

    async fn get_max_context_length(&self) -> usize {
        let behavior = self.behavior.read().await;
        behavior.max_context_length
    }

    fn get_name(&self) -> &str {
        &self.name
    }
}

impl Default for MockProviderBehavior {
    fn default() -> Self {
        Self {
            available: true,
            latency: Duration::from_millis(100), // 100ms default latency
            responses: HashMap::new(),
            default_response: MockResponse::default(),
            error_rate: 0.0,
            error_type: AIError::provider_error("Mock provider error".to_string()),
            max_context_length: 4000,
        }
    }
}

impl Default for MockResponse {
    fn default() -> Self {
        Self {
            content: "Mock AI response".to_string(),
            is_error: false,
            processing_time: None,
            metadata: HashMap::new(),
        }
    }
}

impl MockResponse {
    /// Create a successful mock response
    pub fn success(content: String) -> Self {
        Self {
            content,
            is_error: false,
            processing_time: None,
            metadata: HashMap::new(),
        }
    }

    /// Create an error mock response
    pub fn error() -> Self {
        Self {
            content: String::new(),
            is_error: true,
            processing_time: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a response with processing delay
    pub fn with_delay(content: String, delay: Duration) -> Self {
        Self {
            content,
            is_error: false,
            processing_time: Some(delay),
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the response
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Factory for creating common mock provider configurations
pub struct MockProviderFactory;

impl MockProviderFactory {
    /// Create a fast, reliable mock provider
    pub fn fast_provider() -> MockAIProvider {
        let behavior = MockProviderBehavior {
            available: true,
            latency: Duration::from_millis(10),
            error_rate: 0.0,
            ..Default::default()
        };
        MockAIProvider::with_behavior("FastMock".to_string(), behavior)
    }

    /// Create a slow mock provider
    pub fn slow_provider() -> MockAIProvider {
        let behavior = MockProviderBehavior {
            available: true,
            latency: Duration::from_secs(2),
            error_rate: 0.0,
            ..Default::default()
        };
        MockAIProvider::with_behavior("SlowMock".to_string(), behavior)
    }

    /// Create an unreliable mock provider
    pub fn unreliable_provider() -> MockAIProvider {
        let behavior = MockProviderBehavior {
            available: true,
            latency: Duration::from_millis(500),
            error_rate: 0.3, // 30% error rate
            ..Default::default()
        };
        MockAIProvider::with_behavior("UnreliableMock".to_string(), behavior)
    }

    /// Create an unavailable mock provider
    pub fn unavailable_provider() -> MockAIProvider {
        let behavior = MockProviderBehavior {
            available: false,
            ..Default::default()
        };
        MockAIProvider::with_behavior("UnavailableMock".to_string(), behavior)
    }

    /// Create a mock provider with predefined email responses
    pub fn email_specialized_provider() -> MockAIProvider {
        let mut responses = HashMap::new();
        
        responses.insert(
            "compose".to_string(),
            MockResponse::success("Here's a professional email draft for you.".to_string()),
        );
        
        responses.insert(
            "summarize".to_string(),
            MockResponse::success("Email summary: This is a meeting request for next week.".to_string()),
        );
        
        responses.insert(
            "reply".to_string(),
            MockResponse::success("Thank you for your message. I'll get back to you soon.".to_string()),
        );

        let behavior = MockProviderBehavior {
            available: true,
            latency: Duration::from_millis(200),
            responses,
            error_rate: 0.0,
            ..Default::default()
        };
        
        MockAIProvider::with_behavior("EmailMock".to_string(), behavior)
    }

    /// Create a mock provider with predefined calendar responses
    pub fn calendar_specialized_provider() -> MockAIProvider {
        let mut responses = HashMap::new();
        
        responses.insert(
            "meeting".to_string(),
            MockResponse::success(r#"{"title": "Team Meeting", "start_time": "2024-12-20T10:00:00Z", "end_time": "2024-12-20T11:00:00Z", "confidence": 0.9}"#.to_string()),
        );
        
        responses.insert(
            "schedule".to_string(),
            MockResponse::success("I recommend scheduling this meeting for Tuesday at 2 PM.".to_string()),
        );

        let behavior = MockProviderBehavior {
            available: true,
            latency: Duration::from_millis(300),
            responses,
            error_rate: 0.0,
            ..Default::default()
        };
        
        MockAIProvider::with_behavior("CalendarMock".to_string(), behavior)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_mock_provider_basic_functionality() {
        let provider = MockAIProvider::new("TestMock".to_string());
        
        assert!(provider.is_available().await);
        assert_eq!(provider.get_name(), "TestMock");
        assert_eq!(provider.call_count().await, 0);
    }

    #[tokio::test]
    async fn test_mock_provider_responses() {
        let provider = MockAIProvider::new("TestMock".to_string());
        
        // Add a specific response
        provider.add_response(
            "hello".to_string(),
            MockResponse::success("Hi there!".to_string()),
        ).await;
        
        // Test specific response
        let response = provider.complete_text("hello world", None).await.unwrap();
        assert_eq!(response, "Hi there!");
        
        // Test default response
        let default_response = provider.complete_text("something else", None).await.unwrap();
        assert_eq!(default_response, "Mock AI response");
        
        assert_eq!(provider.call_count().await, 2);
    }

    #[tokio::test]
    async fn test_mock_provider_error_simulation() {
        let provider = MockAIProvider::new("ErrorMock".to_string());
        provider.set_error_rate(1.0).await; // 100% error rate
        
        let result = provider.complete_text("test", None).await;
        assert!(result.is_err());
        
        assert_eq!(provider.call_count().await, 1);
    }

    #[tokio::test]
    async fn test_mock_provider_latency() {
        let provider = MockAIProvider::new("SlowMock".to_string());
        provider.set_latency(Duration::from_millis(100)).await;
        
        let start = std::time::Instant::now();
        let _response = provider.complete_text("test", None).await.unwrap();
        let duration = start.elapsed();
        
        assert!(duration >= Duration::from_millis(90)); // Allow some tolerance
    }

    #[tokio::test]
    async fn test_mock_provider_call_history() {
        let provider = MockAIProvider::new("HistoryMock".to_string());
        
        provider.complete_text("first call", None).await.unwrap();
        provider.complete_text("second call", None).await.unwrap();
        
        assert!(provider.was_called_with("first").await);
        assert!(provider.was_called_with("second").await);
        assert!(!provider.was_called_with("third").await);
        
        let last_call = provider.last_call().await.unwrap();
        assert!(last_call.prompt.contains("second"));
        
        provider.clear_call_history().await;
        assert_eq!(provider.call_count().await, 0);
    }

    #[tokio::test]
    async fn test_mock_provider_factory() {
        let fast = MockProviderFactory::fast_provider();
        let slow = MockProviderFactory::slow_provider();
        let unreliable = MockProviderFactory::unreliable_provider();
        let unavailable = MockProviderFactory::unavailable_provider();
        
        assert!(fast.is_available().await);
        assert!(slow.is_available().await);
        assert!(unreliable.is_available().await);
        assert!(!unavailable.is_available().await);
        
        // Test fast provider is actually fast
        let start = std::time::Instant::now();
        timeout(Duration::from_millis(50), fast.complete_text("test", None))
            .await
            .expect("Fast provider should respond quickly")
            .unwrap();
        
        // Test email specialized responses
        let email_provider = MockProviderFactory::email_specialized_provider();
        let compose_response = email_provider.complete_text("compose email", None).await.unwrap();
        assert!(compose_response.contains("professional email"));
    }
}