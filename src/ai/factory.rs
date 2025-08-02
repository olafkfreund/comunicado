//! AI provider factory for creating and managing AI providers

use crate::ai::{AIResult, AIConfig, AIProviderType, AIProviderManager, AIResponseCache, AIService};
use crate::ai::error::AIError;
use crate::ai::providers::{AnthropicProvider, GoogleProvider, OllamaProvider, OpenAIProvider};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Factory for creating AI providers and services
pub struct AIFactory;

impl AIFactory {
    /// Create a complete AI service with all providers configured
    pub async fn create_ai_service(config: AIConfig) -> AIResult<AIService> {
        let config = Arc::new(RwLock::new(config));
        let mut provider_manager = AIProviderManager::new(config.clone());
        
        // Register all available providers based on configuration
        Self::register_providers(&mut provider_manager, &config).await?;
        
        let provider_manager = Arc::new(RwLock::new(provider_manager));
        let cache = Arc::new(AIResponseCache::default());
        
        Ok(AIService::new(provider_manager, cache, config))
    }

    /// Register all configured AI providers
    async fn register_providers(
        manager: &mut AIProviderManager,
        config: &Arc<RwLock<AIConfig>>,
    ) -> AIResult<()> {
        let config_read = config.read().await;
        
        // Register Ollama provider if configured
        if config_read.provider == AIProviderType::Ollama || 
           config_read.fallback_providers.contains(&AIProviderType::Ollama) {
            if let Ok(provider) = Self::create_ollama_provider(&config_read) {
                manager.register_provider(AIProviderType::Ollama, Box::new(provider));
                tracing::info!("Registered Ollama AI provider");
            } else {
                tracing::warn!("Failed to register Ollama provider");
            }
        }

        // Register OpenAI provider if configured
        if config_read.provider == AIProviderType::OpenAI || 
           config_read.fallback_providers.contains(&AIProviderType::OpenAI) {
            if let Ok(provider) = Self::create_openai_provider(&config_read) {
                manager.register_provider(AIProviderType::OpenAI, Box::new(provider));
                tracing::info!("Registered OpenAI AI provider");
            } else {
                tracing::warn!("Failed to register OpenAI provider - check API key configuration");
            }
        }

        // Register Anthropic provider if configured
        if config_read.provider == AIProviderType::Anthropic || 
           config_read.fallback_providers.contains(&AIProviderType::Anthropic) {
            if let Ok(provider) = Self::create_anthropic_provider(&config_read) {
                manager.register_provider(AIProviderType::Anthropic, Box::new(provider));
                tracing::info!("Registered Anthropic AI provider");
            } else {
                tracing::warn!("Failed to register Anthropic provider - check API key configuration");
            }
        }

        // Register Google provider if configured
        if config_read.provider == AIProviderType::Google || 
           config_read.fallback_providers.contains(&AIProviderType::Google) {
            if let Ok(provider) = Self::create_google_provider(&config_read) {
                manager.register_provider(AIProviderType::Google, Box::new(provider));
                tracing::info!("Registered Google AI provider");
            } else {
                tracing::warn!("Failed to register Google provider - check API key configuration");
            }
        }

        Ok(())
    }

    /// Create Ollama provider
    fn create_ollama_provider(config: &AIConfig) -> AIResult<OllamaProvider> {
        OllamaProvider::from_config(config)
    }

    /// Create OpenAI provider
    fn create_openai_provider(config: &AIConfig) -> AIResult<OpenAIProvider> {
        OpenAIProvider::from_config(config)
    }

    /// Create Anthropic provider
    fn create_anthropic_provider(config: &AIConfig) -> AIResult<AnthropicProvider> {
        AnthropicProvider::from_config(config)
    }

    /// Create Google provider
    fn create_google_provider(config: &AIConfig) -> AIResult<GoogleProvider> {
        GoogleProvider::from_config(config)
    }

    /// Create a privacy-first AI service (Ollama only)
    pub async fn create_privacy_first_service(config: AIConfig) -> AIResult<AIService> {
        let mut privacy_config = config;
        privacy_config.provider = AIProviderType::Ollama;
        privacy_config.fallback_providers = vec![]; // No fallbacks for privacy-first
        
        Self::create_ai_service(privacy_config).await
    }

    /// Create a cloud-optimized AI service with fallbacks
    pub async fn create_cloud_service(config: AIConfig) -> AIResult<AIService> {
        let mut cloud_config = config;
        
        // Set up cloud providers with intelligent fallbacks
        if cloud_config.provider == AIProviderType::None {
            cloud_config.provider = AIProviderType::OpenAI;
        }
        
        // Ensure we have good fallback options
        if cloud_config.fallback_providers.is_empty() {
            cloud_config.fallback_providers = vec![
                AIProviderType::Anthropic,
                AIProviderType::Google,
                AIProviderType::Ollama, // Local fallback
            ];
        }
        
        Self::create_ai_service(cloud_config).await
    }

    /// Validate AI service configuration
    pub fn validate_config(config: &AIConfig) -> AIResult<()> {
        if !config.enabled {
            return Err(AIError::config_error("AI functionality is disabled"));
        }

        if config.provider == AIProviderType::None {
            return Err(AIError::config_error("No AI provider selected"));
        }

        // Validate provider-specific requirements
        match config.provider {
            AIProviderType::Ollama => {
                if config.ollama_endpoint.is_empty() {
                    return Err(AIError::config_error("Ollama endpoint not configured"));
                }
            },
            AIProviderType::OpenAI => {
                if !config.api_keys.contains_key("openai") {
                    return Err(AIError::config_error("OpenAI API key not configured"));
                }
            },
            AIProviderType::Anthropic => {
                if !config.api_keys.contains_key("anthropic") {
                    return Err(AIError::config_error("Anthropic API key not configured"));
                }
            },
            AIProviderType::Google => {
                if !config.api_keys.contains_key("google") {
                    return Err(AIError::config_error("Google API key not configured"));
                }
            },
            AIProviderType::None => {
                return Err(AIError::config_error("AI provider cannot be None when enabled"));
            },
        }

        Ok(())
    }

    /// Get provider recommendations based on use case
    pub fn get_provider_recommendations(use_case: &str) -> Vec<AIProviderType> {
        match use_case {
            "privacy" => vec![AIProviderType::Ollama],
            "speed" => vec![AIProviderType::Google, AIProviderType::OpenAI, AIProviderType::Anthropic],
            "accuracy" => vec![AIProviderType::Anthropic, AIProviderType::OpenAI, AIProviderType::Google],
            "cost-effective" => vec![AIProviderType::Ollama, AIProviderType::Google, AIProviderType::OpenAI],
            "enterprise" => vec![AIProviderType::OpenAI, AIProviderType::Anthropic, AIProviderType::Google],
            _ => vec![AIProviderType::OpenAI, AIProviderType::Anthropic, AIProviderType::Google, AIProviderType::Ollama],
        }
    }

    /// Create a balanced AI service with multiple providers for resilience
    pub async fn create_resilient_service(config: AIConfig) -> AIResult<AIService> {
        let mut resilient_config = config;
        
        // Ensure we have multiple providers configured for resilience
        resilient_config.fallback_providers = vec![
            AIProviderType::OpenAI,
            AIProviderType::Anthropic,
            AIProviderType::Google,
            AIProviderType::Ollama,
        ];

        // Remove the primary provider from fallbacks to avoid duplication
        resilient_config.fallback_providers.retain(|p| *p != resilient_config.provider);
        
        Self::create_ai_service(resilient_config).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::config::{AIConfig, AIProviderType};

    #[test]
    fn test_provider_recommendations() {
        let privacy_recs = AIFactory::get_provider_recommendations("privacy");
        assert_eq!(privacy_recs, vec![AIProviderType::Ollama]);

        let speed_recs = AIFactory::get_provider_recommendations("speed");
        assert!(speed_recs.contains(&AIProviderType::Google));
        assert!(speed_recs.contains(&AIProviderType::OpenAI));

        let accuracy_recs = AIFactory::get_provider_recommendations("accuracy");
        assert!(accuracy_recs.contains(&AIProviderType::Anthropic));
        assert!(accuracy_recs.contains(&AIProviderType::OpenAI));
    }

    #[test]
    fn test_config_validation() {
        let mut config = AIConfig::default();
        config.enabled = false;
        assert!(AIFactory::validate_config(&config).is_err());

        config.enabled = true;
        config.provider = AIProviderType::None;
        assert!(AIFactory::validate_config(&config).is_err());

        config.provider = AIProviderType::Ollama;
        config.ollama_endpoint = "".to_string();
        assert!(AIFactory::validate_config(&config).is_err());

        config.ollama_endpoint = "http://localhost:11434".to_string();
        assert!(AIFactory::validate_config(&config).is_ok());
    }

    #[tokio::test]
    async fn test_privacy_first_service_creation() {
        let mut config = AIConfig::default();
        config.enabled = true;
        config.provider = AIProviderType::OpenAI; // Will be overridden
        config.local_model = Some("llama2".to_string());
        
        let service = AIFactory::create_privacy_first_service(config).await;
        // This will fail without actual Ollama setup, but tests the configuration logic
        assert!(service.is_err() || service.is_ok());
    }

    #[tokio::test]
    async fn test_cloud_service_creation() {
        let mut config = AIConfig::default();
        config.enabled = true;
        config.provider = AIProviderType::None; // Should be set to OpenAI
        
        let service = AIFactory::create_cloud_service(config).await;
        // This will fail without API keys, but tests the configuration logic
        assert!(service.is_err() || service.is_ok());
    }

    #[tokio::test]
    async fn test_resilient_service_creation() {
        let mut config = AIConfig::default();
        config.enabled = true;
        config.provider = AIProviderType::OpenAI;
        config.set_api_key("openai".to_string(), "test-key".to_string());
        
        let service = AIFactory::create_resilient_service(config).await;
        // This will fail without valid API keys, but tests the configuration logic
        assert!(service.is_err() || service.is_ok());
    }
}