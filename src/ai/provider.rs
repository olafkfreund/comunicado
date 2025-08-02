//! AI provider trait and management system

use crate::ai::{AIContext, AIResult};
use crate::ai::config::{AIConfig, AIProviderType};
use crate::ai::error::AIError;
use crate::ai::service::{EmailCategory, SchedulingIntent};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Capabilities that an AI provider supports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    /// Provider name
    pub name: String,
    /// Supports text completion
    pub text_completion: bool,
    /// Supports content summarization
    pub summarization: bool,
    /// Supports email reply suggestions
    pub email_replies: bool,
    /// Supports natural language scheduling
    pub scheduling: bool,
    /// Supports email categorization
    pub categorization: bool,
    /// Maximum context length supported
    pub max_context_length: usize,
    /// Supports streaming responses
    pub streaming: bool,
    /// Whether this is a local provider (privacy-focused)
    pub local_processing: bool,
    /// Available models for this provider
    pub available_models: Vec<String>,
}

/// Core trait that all AI providers must implement
#[async_trait]
pub trait AIProvider: Send + Sync {
    /// Get provider name
    fn name(&self) -> &str;

    /// Get provider capabilities
    fn capabilities(&self) -> &ProviderCapabilities;

    /// Test if provider is available and responsive
    async fn health_check(&self) -> AIResult<bool>;

    /// Complete text based on prompt and context
    async fn complete_text(&self, prompt: &str, context: Option<&AIContext>) -> AIResult<String>;

    /// Summarize content into a concise overview
    async fn summarize_content(&self, content: &str, max_length: Option<usize>) -> AIResult<String>;

    /// Generate email reply suggestions based on email content and context
    async fn suggest_reply(&self, email_content: &str, context: &str) -> AIResult<Vec<String>>;

    /// Parse natural language text for scheduling intent
    async fn parse_schedule_request(&self, text: &str) -> AIResult<SchedulingIntent>;

    /// Categorize email content into predefined categories
    async fn categorize_email(&self, content: &str) -> AIResult<EmailCategory>;

    /// Generate a professional email response
    async fn compose_email(&self, prompt: &str, context: Option<&str>) -> AIResult<String>;

    /// Extract key information from text content
    async fn extract_key_info(&self, content: &str) -> AIResult<Vec<String>>;

    /// Check if the provider supports a specific feature
    fn supports_feature(&self, feature: &str) -> bool {
        match feature {
            "text_completion" => self.capabilities().text_completion,
            "summarization" => self.capabilities().summarization,
            "email_replies" => self.capabilities().email_replies,
            "scheduling" => self.capabilities().scheduling,
            "categorization" => self.capabilities().categorization,
            "streaming" => self.capabilities().streaming,
            _ => false,
        }
    }
}

/// Manages multiple AI providers and handles provider switching
pub struct AIProviderManager {
    providers: HashMap<AIProviderType, Box<dyn AIProvider>>,
    active_provider: AIProviderType,
    config: Arc<RwLock<AIConfig>>,
    health_status: Arc<RwLock<HashMap<AIProviderType, bool>>>,
}

impl AIProviderManager {
    /// Create a new provider manager
    pub fn new(config: Arc<RwLock<AIConfig>>) -> Self {
        Self {
            providers: HashMap::new(),
            active_provider: AIProviderType::None,
            config,
            health_status: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a provider with the manager
    pub fn register_provider(&mut self, provider_type: AIProviderType, provider: Box<dyn AIProvider>) {
        tracing::info!("Registering AI provider: {}", provider_type);
        self.providers.insert(provider_type, provider);
    }

    /// Get the currently active provider
    pub async fn get_active_provider(&self) -> AIResult<&dyn AIProvider> {
        let config = self.config.read().await;
        let provider_type = if config.enabled {
            config.provider.clone()
        } else {
            AIProviderType::None
        };

        if provider_type == AIProviderType::None {
            return Err(AIError::config_error("AI functionality is disabled"));
        }

        self.providers
            .get(&provider_type)
            .map(|p| p.as_ref())
            .ok_or_else(|| {
                AIError::provider_unavailable(format!(
                    "Provider {} is not registered",
                    provider_type
                ))
            })
    }

    /// Switch to a different provider
    pub async fn switch_provider(&mut self, provider_type: AIProviderType) -> AIResult<()> {
        if !self.providers.contains_key(&provider_type) {
            return Err(AIError::provider_unavailable(format!(
                "Provider {} is not available",
                provider_type
            )));
        }

        // Test provider connectivity before switching
        if let Some(provider) = self.providers.get(&provider_type) {
            provider.health_check().await?;
        }

        let mut config = self.config.write().await;
        config.provider = provider_type.clone();
        self.active_provider = provider_type;

        tracing::info!("Switched to AI provider: {}", self.active_provider);
        Ok(())
    }

    /// Test connectivity for a specific provider
    pub async fn test_provider_connectivity(&self, provider_type: AIProviderType) -> AIResult<bool> {
        if let Some(provider) = self.providers.get(&provider_type) {
            match provider.health_check().await {
                Ok(healthy) => {
                    // Update health status
                    let mut health_status = self.health_status.write().await;
                    health_status.insert(provider_type, healthy);
                    Ok(healthy)
                }
                Err(e) => {
                    // Update health status to false
                    let mut health_status = self.health_status.write().await;
                    health_status.insert(provider_type, false);
                    Err(e)
                }
            }
        } else {
            Err(AIError::provider_unavailable(format!(
                "Provider {} is not registered",
                provider_type
            )))
        }
    }

    /// Get capabilities for a specific provider
    pub fn get_provider_capabilities(&self, provider_type: AIProviderType) -> Option<&ProviderCapabilities> {
        self.providers
            .get(&provider_type)
            .map(|provider| provider.capabilities())
    }

    /// Get all available providers
    pub fn get_available_providers(&self) -> Vec<AIProviderType> {
        self.providers.keys().cloned().collect()
    }

    /// Get health status for all providers
    pub async fn get_health_status(&self) -> HashMap<AIProviderType, bool> {
        self.health_status.read().await.clone()
    }

    /// Find the best available provider based on configuration and health
    pub async fn find_best_provider(&self) -> AIResult<AIProviderType> {
        let config = self.config.read().await;
        let health_status = self.health_status.read().await;

        // If current provider is healthy, use it
        if health_status.get(&config.provider).copied().unwrap_or(false) {
            return Ok(config.provider.clone());
        }

        // Try fallback providers
        for provider_type in &config.fallback_providers {
            if health_status.get(provider_type).copied().unwrap_or(false) {
                return Ok(provider_type.clone());
            }
        }

        // If no healthy providers found, try to find any available provider
        for (provider_type, provider) in &self.providers {
            if let Ok(true) = provider.health_check().await {
                return Ok(provider_type.clone());
            }
        }

        Err(AIError::provider_unavailable(
            "No healthy AI providers available".to_string(),
        ))
    }

    /// Attempt automatic provider failover
    pub async fn attempt_failover(&mut self) -> AIResult<AIProviderType> {
        let best_provider = self.find_best_provider().await?;
        
        if best_provider != self.active_provider {
            tracing::warn!(
                "Failing over from {} to {} due to provider issues",
                self.active_provider,
                best_provider
            );
            self.switch_provider(best_provider.clone()).await?;
        }

        Ok(best_provider)
    }

    /// Refresh health status for all providers
    pub async fn refresh_health_status(&self) {
        let mut health_status = self.health_status.write().await;
        
        for (provider_type, provider) in &self.providers {
            let is_healthy = provider.health_check().await.unwrap_or(false);
            health_status.insert(provider_type.clone(), is_healthy);
            
            tracing::debug!(
                "Provider {} health status: {}",
                provider_type,
                if is_healthy { "healthy" } else { "unhealthy" }
            );
        }
    }

    /// Get provider statistics
    pub async fn get_provider_stats(&self) -> ProviderStats {
        let health_status = self.health_status.read().await;
        let total_providers = self.providers.len();
        let healthy_providers = health_status.values().filter(|&&healthy| healthy).count();
        
        ProviderStats {
            total_providers,
            healthy_providers,
            active_provider: self.active_provider.clone(),
            health_status: health_status.clone(),
        }
    }
}

/// Statistics about AI providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStats {
    pub total_providers: usize,
    pub healthy_providers: usize,
    pub active_provider: AIProviderType,
    pub health_status: HashMap<AIProviderType, bool>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::config::AIConfig;
    
    // Mock provider for testing
    struct MockProvider {
        name: String,
        capabilities: ProviderCapabilities,
        healthy: bool,
    }

    #[async_trait]
    impl AIProvider for MockProvider {
        fn name(&self) -> &str {
            &self.name
        }

        fn capabilities(&self) -> &ProviderCapabilities {
            &self.capabilities
        }

        async fn health_check(&self) -> AIResult<bool> {
            Ok(self.healthy)
        }

        async fn complete_text(&self, _prompt: &str, _context: Option<&AIContext>) -> AIResult<String> {
            Ok("Mock completion".to_string())
        }

        async fn summarize_content(&self, _content: &str, _max_length: Option<usize>) -> AIResult<String> {
            Ok("Mock summary".to_string())
        }

        async fn suggest_reply(&self, _email_content: &str, _context: &str) -> AIResult<Vec<String>> {
            Ok(vec!["Mock reply".to_string()])
        }

        async fn parse_schedule_request(&self, _text: &str) -> AIResult<SchedulingIntent> {
            Ok(SchedulingIntent {
                intent_type: "meeting".to_string(),
                title: Some("Mock meeting".to_string()),
                datetime: None,
                duration: None,
                participants: vec![],
                location: None,
                description: None,
                confidence: 0.8,
            })
        }

        async fn categorize_email(&self, _content: &str) -> AIResult<EmailCategory> {
            Ok(EmailCategory::Personal)
        }

        async fn compose_email(&self, _prompt: &str, _context: Option<&str>) -> AIResult<String> {
            Ok("Mock email".to_string())
        }

        async fn extract_key_info(&self, _content: &str) -> AIResult<Vec<String>> {
            Ok(vec!["Mock info".to_string()])
        }
    }

    fn create_mock_provider(name: &str, healthy: bool) -> Box<dyn AIProvider> {
        Box::new(MockProvider {
            name: name.to_string(),
            capabilities: ProviderCapabilities {
                name: name.to_string(),
                text_completion: true,
                summarization: true,
                email_replies: true,
                scheduling: true,
                categorization: true,
                max_context_length: 4000,
                streaming: false,
                local_processing: name == "mock_local",
                available_models: vec!["mock-model".to_string()],
            },
            healthy,
        })
    }

    #[tokio::test]
    async fn test_provider_manager_registration() {
        let config = Arc::new(RwLock::new(AIConfig::default()));
        let mut manager = AIProviderManager::new(config);

        let provider = create_mock_provider("test", true);
        manager.register_provider(AIProviderType::Ollama, provider);

        assert!(manager.providers.contains_key(&AIProviderType::Ollama));
        assert_eq!(manager.get_available_providers().len(), 1);
    }

    #[tokio::test]
    async fn test_provider_health_check() {
        let config = Arc::new(RwLock::new(AIConfig::default()));
        let mut manager = AIProviderManager::new(config);

        let healthy_provider = create_mock_provider("healthy", true);
        let unhealthy_provider = create_mock_provider("unhealthy", false);

        manager.register_provider(AIProviderType::Ollama, healthy_provider);
        manager.register_provider(AIProviderType::OpenAI, unhealthy_provider);

        assert!(manager.test_provider_connectivity(AIProviderType::Ollama).await.unwrap());
        assert!(!manager.test_provider_connectivity(AIProviderType::OpenAI).await.unwrap());
    }

    #[tokio::test]
    async fn test_provider_failover() {
        let mut config = AIConfig::default();
        config.enabled = true;
        config.provider = AIProviderType::OpenAI;
        config.fallback_providers = vec![AIProviderType::Ollama];
        
        let config = Arc::new(RwLock::new(config));
        let mut manager = AIProviderManager::new(config);

        // Register unhealthy primary provider and healthy fallback
        let unhealthy_provider = create_mock_provider("unhealthy", false);
        let healthy_provider = create_mock_provider("healthy", true);

        manager.register_provider(AIProviderType::OpenAI, unhealthy_provider);
        manager.register_provider(AIProviderType::Ollama, healthy_provider);

        // Refresh health status
        manager.refresh_health_status().await;

        // Test failover
        let best_provider = manager.find_best_provider().await.unwrap();
        assert_eq!(best_provider, AIProviderType::Ollama);
    }
}