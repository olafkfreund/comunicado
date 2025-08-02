//! AI configuration management and settings

use crate::ai::error::{AIError, AIResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// AI provider types supported by the system
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum AIProviderType {
    /// Local Ollama instance for privacy-first processing
    Ollama,
    /// OpenAI GPT models for advanced capabilities
    OpenAI,
    /// Anthropic Claude models for alternative cloud processing
    Anthropic,
    /// Google AI (Gemini) models
    Google,
    /// Disabled - no AI functionality
    #[default]
    None,
}

impl std::fmt::Display for AIProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AIProviderType::Ollama => write!(f, "Ollama"),
            AIProviderType::OpenAI => write!(f, "OpenAI"),
            AIProviderType::Anthropic => write!(f, "Anthropic"),
            AIProviderType::Google => write!(f, "Google"),
            AIProviderType::None => write!(f, "None"),
        }
    }
}

/// Privacy mode settings for AI processing
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PrivacyMode {
    /// Only use local AI processing (Ollama)
    LocalOnly,
    /// Prefer local processing, fallback to cloud with consent
    #[default]
    LocalPreferred,
    /// Allow cloud processing with user consent
    CloudWithConsent,
    /// Allow cloud processing without explicit consent
    CloudAllowed,
}

/// Comprehensive AI configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIConfig {
    /// Whether AI functionality is enabled globally
    pub enabled: bool,
    
    /// Active AI provider
    pub provider: AIProviderType,
    
    /// Privacy mode for data handling
    pub privacy_mode: PrivacyMode,
    
    /// Local AI model settings
    pub local_model: Option<String>,
    
    /// Ollama server configuration
    pub ollama_endpoint: String,
    
    /// API keys for cloud providers (stored securely)
    pub api_keys: HashMap<String, String>,
    
    /// Whether to cache AI responses for performance
    pub cache_responses: bool,
    
    /// Cache TTL for AI responses
    pub cache_ttl: Duration,
    
    /// Maximum context length for AI requests
    pub max_context_length: usize,
    
    /// Request timeout duration
    pub request_timeout: Duration,
    
    /// Maximum number of retry attempts
    pub max_retries: u32,
    
    /// Whether to enable AI email suggestions
    pub email_suggestions_enabled: bool,
    
    /// Whether to enable automatic email summarization
    pub email_summarization_enabled: bool,
    
    /// Whether to enable AI calendar assistance
    pub calendar_assistance_enabled: bool,
    
    /// Whether to enable email categorization
    pub email_categorization_enabled: bool,
    
    /// Temperature/creativity setting for AI responses (0.0-1.0)
    pub creativity: f32,
    
    /// Fallback providers in order of preference
    pub fallback_providers: Vec<AIProviderType>,
    
    /// User consent tracking
    pub consent_given: HashMap<String, bool>,
    
    /// Feature-specific settings
    pub feature_settings: HashMap<String, serde_json::Value>,
}

impl Default for AIConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: AIProviderType::default(),
            privacy_mode: PrivacyMode::default(),
            local_model: Some("llama2".to_string()),
            ollama_endpoint: "http://localhost:11434".to_string(),
            api_keys: HashMap::new(),
            cache_responses: true,
            cache_ttl: Duration::from_secs(3600), // 1 hour
            max_context_length: 4000,
            request_timeout: Duration::from_secs(30),
            max_retries: 3,
            email_suggestions_enabled: true,
            email_summarization_enabled: true,
            calendar_assistance_enabled: true,
            email_categorization_enabled: true,
            creativity: 0.7,
            fallback_providers: vec![AIProviderType::Ollama],
            consent_given: HashMap::new(),
            feature_settings: HashMap::new(),
        }
    }
}

impl AIConfig {
    /// Create a new AI configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Load configuration from file
    pub async fn load_from_file(path: &PathBuf) -> AIResult<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| AIError::config_error(format!("Failed to read config: {}", e)))?;

        let config: AIConfig = toml::from_str(&content)
            .map_err(|e| AIError::config_error(format!("Failed to parse config: {}", e)))?;

        config.validate()?;
        Ok(config)
    }

    /// Save configuration to file
    pub async fn save_to_file(&self, path: &PathBuf) -> AIResult<()> {
        self.validate()?;

        let content = toml::to_string_pretty(self)
            .map_err(|e| AIError::config_error(format!("Failed to serialize config: {}", e)))?;

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| AIError::config_error(format!("Failed to create config directory: {}", e)))?;
        }

        tokio::fs::write(path, content)
            .await
            .map_err(|e| AIError::config_error(format!("Failed to write config: {}", e)))?;

        Ok(())
    }

    /// Validate configuration settings
    pub fn validate(&self) -> AIResult<()> {
        // Validate creativity setting
        if !(0.0..=1.0).contains(&self.creativity) {
            return Err(AIError::config_error("Creativity must be between 0.0 and 1.0"));
        }

        // Validate context length
        if self.max_context_length == 0 {
            return Err(AIError::config_error("Max context length must be greater than 0"));
        }

        // Validate timeout settings
        if self.request_timeout.is_zero() {
            return Err(AIError::config_error("Request timeout must be greater than 0"));
        }

        // Validate provider-specific settings
        match self.provider {
            AIProviderType::Ollama => {
                if self.ollama_endpoint.is_empty() {
                    return Err(AIError::config_error("Ollama endpoint cannot be empty"));
                }
            }
            AIProviderType::OpenAI => {
                if !self.api_keys.contains_key("openai") {
                    return Err(AIError::config_error("OpenAI API key is required"));
                }
            }
            AIProviderType::Anthropic => {
                if !self.api_keys.contains_key("anthropic") {
                    return Err(AIError::config_error("Anthropic API key is required"));
                }
            }
            AIProviderType::Google => {
                if !self.api_keys.contains_key("google") {
                    return Err(AIError::config_error("Google API key is required"));
                }
            }
            AIProviderType::None => {
                // No validation needed for disabled AI
            }
        }

        Ok(())
    }

    /// Check if a specific feature is enabled
    pub fn is_feature_enabled(&self, feature: &str) -> bool {
        if !self.enabled {
            return false;
        }

        match feature {
            "email_suggestions" => self.email_suggestions_enabled,
            "email_summarization" => self.email_summarization_enabled,
            "calendar_assistance" => self.calendar_assistance_enabled,
            "email_categorization" => self.email_categorization_enabled,
            _ => false,
        }
    }

    /// Check if user has given consent for a specific operation
    pub fn has_consent(&self, operation: &str) -> bool {
        match self.privacy_mode {
            PrivacyMode::CloudAllowed => true,
            PrivacyMode::LocalOnly => false,
            _ => self.consent_given.get(operation).copied().unwrap_or(false),
        }
    }

    /// Grant consent for a specific operation
    pub fn grant_consent(&mut self, operation: String) {
        self.consent_given.insert(operation, true);
    }

    /// Revoke consent for a specific operation
    pub fn revoke_consent(&mut self, operation: String) {
        self.consent_given.insert(operation, false);
    }

    /// Get API key for a provider
    pub fn get_api_key(&self, provider: &str) -> Option<&String> {
        self.api_keys.get(provider)
    }

    /// Set API key for a provider
    pub fn set_api_key(&mut self, provider: String, api_key: String) {
        self.api_keys.insert(provider, api_key);
    }

    /// Remove API key for a provider
    pub fn remove_api_key(&mut self, provider: &str) {
        self.api_keys.remove(provider);
    }

    /// Get feature-specific setting
    pub fn get_feature_setting<T>(&self, feature: &str) -> Option<T> 
    where
        T: serde::de::DeserializeOwned,
    {
        self.feature_settings
            .get(feature)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Set feature-specific setting
    pub fn set_feature_setting<T>(&mut self, feature: String, value: T) -> AIResult<()>
    where
        T: serde::Serialize,
    {
        let json_value = serde_json::to_value(value)
            .map_err(|e| AIError::config_error(format!("Failed to serialize setting: {}", e)))?;
        
        self.feature_settings.insert(feature, json_value);
        Ok(())
    }

    /// Check if privacy mode allows cloud processing
    pub fn allows_cloud_processing(&self) -> bool {
        !matches!(self.privacy_mode, PrivacyMode::LocalOnly)
    }

    /// Check if local processing is preferred
    pub fn prefers_local_processing(&self) -> bool {
        matches!(
            self.privacy_mode,
            PrivacyMode::LocalOnly | PrivacyMode::LocalPreferred
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_default_config() {
        let config = AIConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.provider, AIProviderType::None);
        assert_eq!(config.privacy_mode, PrivacyMode::LocalPreferred);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation() {
        let mut config = AIConfig::default();
        
        // Invalid creativity
        config.creativity = 1.5;
        assert!(config.validate().is_err());
        
        // Valid creativity
        config.creativity = 0.5;
        assert!(config.validate().is_ok());
        
        // Invalid context length
        config.max_context_length = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_consent_management() {
        let mut config = AIConfig::default();
        
        assert!(!config.has_consent("test_operation"));
        
        config.grant_consent("test_operation".to_string());
        assert!(config.has_consent("test_operation"));
        
        config.revoke_consent("test_operation".to_string());
        assert!(!config.has_consent("test_operation"));
    }

    #[test]
    fn test_privacy_mode_behavior() {
        let mut config = AIConfig::default();
        
        config.privacy_mode = PrivacyMode::LocalOnly;
        assert!(!config.allows_cloud_processing());
        assert!(config.prefers_local_processing());
        
        config.privacy_mode = PrivacyMode::CloudAllowed;
        assert!(config.allows_cloud_processing());
        assert!(!config.prefers_local_processing());
    }

    #[tokio::test]
    async fn test_config_file_operations() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("ai_config.toml");
        
        let mut config = AIConfig::default();
        config.enabled = true;
        config.provider = AIProviderType::Ollama;
        
        // Save config
        assert!(config.save_to_file(&config_path).await.is_ok());
        
        // Load config
        let loaded_config = AIConfig::load_from_file(&config_path).await.unwrap();
        assert_eq!(loaded_config.enabled, true);
        assert_eq!(loaded_config.provider, AIProviderType::Ollama);
    }
}