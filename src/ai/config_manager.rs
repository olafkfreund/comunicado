//! AI configuration management and persistence

use crate::ai::config::{AIConfig, AIProviderType, PrivacyMode};
use crate::ai::error::{AIError, AIResult};
use crate::ai::factory::AIFactory;
use crate::ai::service::AIService;
use crate::ui::ai_privacy_dialog::{ConsentDecision, PrivacyConsentManager};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// AI configuration manager handles settings persistence and validation
pub struct AIConfigManager {
    /// Current AI configuration
    config: Arc<RwLock<AIConfig>>,
    /// Configuration file path
    config_path: PathBuf,
    /// Privacy consent manager
    consent_manager: Arc<RwLock<PrivacyConsentManager>>,
    /// Active AI service instance
    ai_service: Arc<RwLock<Option<Arc<AIService>>>>,
}

impl AIConfigManager {
    /// Create new AI configuration manager
    pub fn new(config_path: PathBuf) -> Self {
        Self {
            config: Arc::new(RwLock::new(AIConfig::default())),
            config_path,
            consent_manager: Arc::new(RwLock::new(PrivacyConsentManager::new())),
            ai_service: Arc::new(RwLock::new(None)),
        }
    }

    /// Initialize the configuration manager
    pub async fn initialize(&self) -> AIResult<()> {
        // Load configuration from file
        self.load_config().await?;
        
        // Initialize AI service if enabled
        let config = self.config.read().await;
        if config.enabled {
            drop(config);
            self.initialize_ai_service().await?;
        }
        
        Ok(())
    }

    /// Load configuration from file
    pub async fn load_config(&self) -> AIResult<()> {
        let loaded_config = AIConfig::load_from_file(&self.config_path).await?;
        
        // Validate the loaded configuration
        loaded_config.validate()?;
        
        let mut config = self.config.write().await;
        *config = loaded_config;
        
        Ok(())
    }

    /// Save configuration to file
    pub async fn save_config(&self) -> AIResult<()> {
        let config = self.config.read().await;
        config.save_to_file(&self.config_path).await?;
        Ok(())
    }

    /// Get current configuration
    pub async fn get_config(&self) -> AIConfig {
        self.config.read().await.clone()
    }

    /// Update configuration
    pub async fn update_config(&self, new_config: AIConfig) -> AIResult<()> {
        // Validate the new configuration
        new_config.validate()?;
        
        let mut config = self.config.write().await;
        let old_enabled = config.enabled;
        let old_provider = config.provider.clone();
        
        *config = new_config;
        
        // If AI was enabled/disabled or provider changed, reinitialize service
        if config.enabled != old_enabled || config.provider != old_provider {
            drop(config);
            if self.get_config().await.enabled {
                self.initialize_ai_service().await?;
            } else {
                self.shutdown_ai_service().await;
            }
        }
        
        // Save the updated configuration
        self.save_config().await?;
        
        Ok(())
    }

    /// Enable AI functionality
    pub async fn enable_ai(&self) -> AIResult<()> {
        {
            let mut config = self.config.write().await;
            config.enabled = true;
        }
        
        self.initialize_ai_service().await?;
        self.save_config().await?;
        
        Ok(())
    }

    /// Disable AI functionality
    pub async fn disable_ai(&self) -> AIResult<()> {
        {
            let mut config = self.config.write().await;
            config.enabled = false;
        }
        
        self.shutdown_ai_service().await;
        self.save_config().await?;
        
        Ok(())
    }

    /// Set AI provider
    pub async fn set_provider(&self, provider: AIProviderType) -> AIResult<()> {
        {
            let mut config = self.config.write().await;
            config.provider = provider;
        }
        
        // Reinitialize AI service with new provider
        if self.is_ai_enabled().await {
            self.initialize_ai_service().await?;
        }
        
        self.save_config().await?;
        Ok(())
    }

    /// Set privacy mode
    pub async fn set_privacy_mode(&self, privacy_mode: PrivacyMode) -> AIResult<()> {
        {
            let mut config = self.config.write().await;
            config.privacy_mode = privacy_mode;
        }
        
        self.save_config().await?;
        Ok(())
    }

    /// Set API key for a provider
    pub async fn set_api_key(&self, provider: String, api_key: String) -> AIResult<()> {
        {
            let mut config = self.config.write().await;
            config.set_api_key(provider, api_key);
        }
        
        // Reinitialize AI service if using the updated provider
        if self.is_ai_enabled().await {
            self.initialize_ai_service().await?;
        }
        
        self.save_config().await?;
        Ok(())
    }

    /// Enable/disable a specific AI feature
    pub async fn set_feature_enabled(&self, feature: &str, enabled: bool) -> AIResult<()> {
        {
            let mut config = self.config.write().await;
            match feature {
                "email_suggestions" => config.email_suggestions_enabled = enabled,
                "email_summarization" => config.email_summarization_enabled = enabled,
                "calendar_assistance" => config.calendar_assistance_enabled = enabled,
                "email_categorization" => config.email_categorization_enabled = enabled,
                _ => return Err(AIError::config_error(format!("Unknown feature: {}", feature))),
            }
        }
        
        self.save_config().await?;
        Ok(())
    }

    /// Check if AI is enabled
    pub async fn is_ai_enabled(&self) -> bool {
        self.config.read().await.enabled
    }

    /// Check if a specific feature is enabled
    pub async fn is_feature_enabled(&self, feature: &str) -> bool {
        let config = self.config.read().await;
        config.is_feature_enabled(feature)
    }

    /// Get current AI service instance
    pub async fn get_ai_service(&self) -> Option<Arc<AIService>> {
        self.ai_service.read().await.clone()
    }

    /// Check if consent is required for an operation
    pub async fn is_consent_required(&self, operation: &str) -> bool {
        let config = self.config.read().await;
        let consent_manager = self.consent_manager.read().await;
        
        consent_manager.is_consent_required(
            operation,
            &config.provider,
            &config.privacy_mode,
        )
    }

    /// Record consent decision
    pub async fn record_consent(&self, operation: String, decision: ConsentDecision) {
        let mut consent_manager = self.consent_manager.write().await;
        consent_manager.record_consent(operation, decision);
    }

    /// Check if an operation is allowed based on consent
    pub async fn is_operation_allowed(&self, operation: &str) -> Option<bool> {
        let consent_manager = self.consent_manager.read().await;
        consent_manager.is_operation_allowed(operation)
    }

    /// Clear all consent decisions
    pub async fn clear_all_consent(&self) {
        let mut consent_manager = self.consent_manager.write().await;
        consent_manager.clear_all_consent();
    }

    /// Initialize AI service based on current configuration
    async fn initialize_ai_service(&self) -> AIResult<()> {
        let config = self.config.read().await.clone();
        
        if !config.enabled {
            return Ok(());
        }

        let service = AIFactory::create_ai_service(config).await?;
        
        let mut ai_service = self.ai_service.write().await;
        *ai_service = Some(Arc::new(service));
        
        Ok(())
    }

    /// Shutdown AI service
    async fn shutdown_ai_service(&self) {
        let mut ai_service = self.ai_service.write().await;
        *ai_service = None;
    }

    /// Validate current configuration
    pub async fn validate_config(&self) -> AIResult<()> {
        let config = self.config.read().await;
        config.validate()
    }

    /// Reset configuration to defaults
    pub async fn reset_to_defaults(&self) -> AIResult<()> {
        let default_config = AIConfig::default();
        self.update_config(default_config).await?;
        self.clear_all_consent().await;
        Ok(())
    }

    /// Export configuration to JSON
    pub async fn export_config(&self) -> AIResult<String> {
        let config = self.config.read().await;
        serde_json::to_string_pretty(&*config)
            .map_err(|e| AIError::config_error(format!("Failed to export config: {}", e)))
    }

    /// Import configuration from JSON
    pub async fn import_config(&self, json_data: &str) -> AIResult<()> {
        let new_config: AIConfig = serde_json::from_str(json_data)
            .map_err(|e| AIError::config_error(format!("Failed to import config: {}", e)))?;
        
        self.update_config(new_config).await?;
        Ok(())
    }

    /// Get configuration statistics
    pub async fn get_config_stats(&self) -> ConfigStats {
        let config = self.config.read().await;
        let consent_manager = self.consent_manager.read().await;
        
        ConfigStats {
            ai_enabled: config.enabled,
            current_provider: config.provider.clone(),
            privacy_mode: config.privacy_mode.clone(),
            features_enabled: vec![
                ("email_suggestions".to_string(), config.email_suggestions_enabled),
                ("email_summarization".to_string(), config.email_summarization_enabled),
                ("calendar_assistance".to_string(), config.calendar_assistance_enabled),
                ("email_categorization".to_string(), config.email_categorization_enabled),
            ].into_iter().collect(),
            api_keys_configured: config.api_keys.keys().cloned().collect(),
            consent_decisions_count: consent_manager.get_all_consent().len(),
        }
    }

    /// Check AI service health
    pub async fn check_ai_health(&self) -> AIHealthStatus {
        if !self.is_ai_enabled().await {
            return AIHealthStatus::Disabled;
        }

        let service = match self.get_ai_service().await {
            Some(service) => service,
            None => return AIHealthStatus::ServiceNotInitialized,
        };

        if service.is_enabled().await {
            AIHealthStatus::Healthy
        } else {
            AIHealthStatus::ServiceUnavailable
        }
    }
}

/// Configuration statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigStats {
    pub ai_enabled: bool,
    pub current_provider: AIProviderType,
    pub privacy_mode: PrivacyMode,
    pub features_enabled: std::collections::HashMap<String, bool>,
    pub api_keys_configured: Vec<String>,
    pub consent_decisions_count: usize,
}

/// AI service health status
#[derive(Debug, Clone, PartialEq)]
pub enum AIHealthStatus {
    /// AI is disabled
    Disabled,
    /// AI is enabled and service is healthy
    Healthy,
    /// AI service is not initialized
    ServiceNotInitialized,
    /// AI service is unavailable
    ServiceUnavailable,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_config_manager_initialization() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("ai_config.toml");
        
        let manager = AIConfigManager::new(config_path);
        assert!(manager.initialize().await.is_ok());
        
        // Should start with AI disabled
        assert!(!manager.is_ai_enabled().await);
    }

    #[tokio::test]
    async fn test_config_persistence() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("ai_config.toml");
        
        {
            let manager = AIConfigManager::new(config_path.clone());
            manager.initialize().await.unwrap();
            
            // Enable AI and set provider
            manager.enable_ai().await.unwrap();
            manager.set_provider(AIProviderType::Ollama).await.unwrap();
        }
        
        // Create new manager and load config
        {
            let manager = AIConfigManager::new(config_path);
            manager.initialize().await.unwrap();
            
            let config = manager.get_config().await;
            assert!(config.enabled);
            assert_eq!(config.provider, AIProviderType::Ollama);
        }
    }

    #[tokio::test]
    async fn test_feature_management() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("ai_config.toml");
        
        let manager = AIConfigManager::new(config_path);
        manager.initialize().await.unwrap();
        
        // Test feature enabling/disabling
        assert!(manager.is_feature_enabled("email_suggestions").await);
        
        manager.set_feature_enabled("email_suggestions", false).await.unwrap();
        assert!(!manager.is_feature_enabled("email_suggestions").await);
        
        manager.set_feature_enabled("email_suggestions", true).await.unwrap();
        assert!(manager.is_feature_enabled("email_suggestions").await);
    }

    #[tokio::test]
    async fn test_consent_management() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("ai_config.toml");
        
        let manager = AIConfigManager::new(config_path);
        manager.initialize().await.unwrap();
        
        // Test consent recording
        manager.record_consent(
            "email_summary".to_string(),
            ConsentDecision::AllowAlways
        ).await;
        
        assert_eq!(
            manager.is_operation_allowed("email_summary").await,
            Some(true)
        );
        
        // Clear consent
        manager.clear_all_consent().await;
        assert_eq!(
            manager.is_operation_allowed("email_summary").await,
            None
        );
    }

    #[tokio::test]
    async fn test_config_validation() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("ai_config.toml");
        
        let manager = AIConfigManager::new(config_path);
        manager.initialize().await.unwrap();
        
        // Test valid configuration
        let mut valid_config = AIConfig::default();
        valid_config.creativity = 0.5;
        assert!(manager.update_config(valid_config).await.is_ok());
        
        // Test invalid configuration
        let mut invalid_config = AIConfig::default();
        invalid_config.creativity = 1.5; // Invalid value
        assert!(manager.update_config(invalid_config).await.is_err());
    }
}