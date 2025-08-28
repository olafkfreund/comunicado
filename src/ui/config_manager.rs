//! Configuration management abstractions
//!
//! This module provides trait abstractions for configuration operations,
//! allowing UI components to be decoupled from specific configuration implementations.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Trait for configuration management operations
pub trait ConfigurationManager: Send + Sync {
    /// Load configuration from storage
    fn load(&mut self) -> Result<()>;
    
    /// Save configuration to storage
    fn save(&self) -> Result<()>;
    
    /// Check if this is the first run
    fn is_first_run(&self) -> bool;
    
    /// Mark onboarding as completed
    fn mark_onboarding_completed(&mut self) -> Result<()>;
    
    /// Check if onboarding is completed
    fn is_onboarding_completed(&self) -> bool;
    
    /// Get configuration value by key as JSON
    fn get_value_json(&self, key: &str) -> Option<serde_json::Value>;
    
    /// Set configuration value by key from JSON
    fn set_value_json(&mut self, key: &str, value: serde_json::Value) -> Result<()>;
    
    /// Get all available configuration keys
    fn available_keys(&self) -> Vec<String>;
    
    /// Reset to defaults
    fn reset_to_defaults(&mut self) -> Result<()>;
    
    /// Backup current configuration
    fn backup(&self) -> Result<ConfigurationBackup>;
    
    /// Restore from backup
    fn restore(&mut self, backup: ConfigurationBackup) -> Result<()>;
}

/// Extension trait for type-safe configuration access
pub trait ConfigurationManagerExt {
    /// Get configuration value with type safety
    fn get_value<T>(&self, key: &str) -> Option<T>
    where
        T: for<'de> Deserialize<'de>;
        
    /// Set configuration value with type safety
    fn set_value<T>(&mut self, key: &str, value: T) -> Result<()>
    where
        T: Serialize;
}

impl<C: ConfigurationManager> ConfigurationManagerExt for C {
    fn get_value<T>(&self, key: &str) -> Option<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        self.get_value_json(key)
            .and_then(|v| serde_json::from_value(v).ok())
    }
    
    fn set_value<T>(&mut self, key: &str, value: T) -> Result<()>
    where
        T: Serialize,
    {
        let json_value = serde_json::to_value(value)?;
        self.set_value_json(key, json_value)
    }
}

/// Configuration backup data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationBackup {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub data: serde_json::Value,
    pub version: String,
}

/// Adapter for the existing AppConfig
pub struct AppConfigAdapter {
    config: crate::config::AppConfig,
    needs_save: bool,
}

impl AppConfigAdapter {
    /// Create a new adapter with the existing config
    pub fn new(config: crate::config::AppConfig) -> Self {
        Self {
            config,
            needs_save: false,
        }
    }
    
    /// Create adapter and load from default location
    pub fn load_default() -> Result<Self> {
        let config = crate::config::AppConfig::load()?;
        Ok(Self::new(config))
    }
    
    /// Get reference to the underlying config
    pub fn inner(&self) -> &crate::config::AppConfig {
        &self.config
    }
    
    /// Get mutable reference to the underlying config
    pub fn inner_mut(&mut self) -> &mut crate::config::AppConfig {
        self.needs_save = true;
        &mut self.config
    }
    
    /// Consume adapter and return the inner config
    pub fn into_inner(self) -> crate::config::AppConfig {
        self.config
    }
}

impl ConfigurationManager for AppConfigAdapter {
    fn load(&mut self) -> Result<()> {
        self.config = crate::config::AppConfig::load()?;
        self.needs_save = false;
        Ok(())
    }
    
    fn save(&self) -> Result<()> {
        self.config.save()?;
        Ok(())
    }
    
    fn is_first_run(&self) -> bool {
        self.config.general.first_run
    }
    
    fn mark_onboarding_completed(&mut self) -> Result<()> {
        self.config.general.first_run = false;
        self.config.general.onboarding_completed = true;
        self.needs_save = true;
        Ok(())
    }
    
    fn is_onboarding_completed(&self) -> bool {
        self.config.general.onboarding_completed
    }
    
    fn get_value_json(&self, key: &str) -> Option<serde_json::Value> {
        // Simplified implementation - would need proper key mapping
        match key {
            "first_run" => Some(serde_json::Value::Bool(self.config.general.first_run)),
            "onboarding_completed" => Some(serde_json::Value::Bool(self.config.general.onboarding_completed)),
            "sync_interval" => Some(serde_json::Value::Number(serde_json::Number::from(self.config.general.sync_interval_minutes))),
            "ui_theme" => Some(serde_json::Value::String(self.config.ui.theme.clone())),
            _ => None,
        }
    }
    
    fn set_value_json(&mut self, key: &str, value: serde_json::Value) -> Result<()> {
        // Simplified implementation - would need proper key mapping
        match key {
            "first_run" => {
                if let Some(v) = value.as_bool() {
                    self.config.general.first_run = v;
                    self.needs_save = true;
                }
            },
            "onboarding_completed" => {
                if let Some(v) = value.as_bool() {
                    self.config.general.onboarding_completed = v;
                    self.needs_save = true;
                }
            },
            "sync_interval" => {
                if let Some(v) = value.as_u64() {
                    self.config.general.sync_interval_minutes = v;
                    self.needs_save = true;
                }
            },
            "ui_theme" => {
                if let Some(v) = value.as_str() {
                    self.config.ui.theme = v.to_string();
                    self.needs_save = true;
                }
            },
            _ => return Err(anyhow::anyhow!("Unknown configuration key: {}", key)),
        }
        
        Ok(())
    }
    
    fn available_keys(&self) -> Vec<String> {
        vec![
            "first_run".to_string(),
            "onboarding_completed".to_string(),
            "sync_interval".to_string(),
            // Would be expanded based on actual config structure
        ]
    }
    
    fn reset_to_defaults(&mut self) -> Result<()> {
        self.config = crate::config::AppConfig::default();
        self.needs_save = true;
        Ok(())
    }
    
    fn backup(&self) -> Result<ConfigurationBackup> {
        let data = serde_json::to_value(&self.config)?;
        Ok(ConfigurationBackup {
            timestamp: chrono::Utc::now(),
            data,
            version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }
    
    fn restore(&mut self, backup: ConfigurationBackup) -> Result<()> {
        self.config = serde_json::from_value(backup.data)?;
        self.needs_save = true;
        Ok(())
    }
}

/// Mock configuration manager for testing
#[cfg(test)]
pub struct MockConfigManager {
    data: std::collections::HashMap<String, serde_json::Value>,
    first_run: bool,
    onboarding_completed: bool,
}

#[cfg(test)]
impl MockConfigManager {
    pub fn new() -> Self {
        Self {
            data: std::collections::HashMap::new(),
            first_run: true,
            onboarding_completed: false,
        }
    }
}

#[cfg(test)]
impl ConfigurationManager for MockConfigManager {
    fn load(&mut self) -> Result<()> {
        // Mock implementation - no actual loading
        Ok(())
    }
    
    fn save(&self) -> Result<()> {
        // Mock implementation - no actual saving
        Ok(())
    }
    
    fn is_first_run(&self) -> bool {
        self.first_run
    }
    
    fn mark_onboarding_completed(&mut self) -> Result<()> {
        self.first_run = false;
        self.onboarding_completed = true;
        Ok(())
    }
    
    fn is_onboarding_completed(&self) -> bool {
        self.onboarding_completed
    }
    
    fn get_value_json(&self, key: &str) -> Option<serde_json::Value> {
        self.data.get(key).cloned()
    }
    
    fn set_value_json(&mut self, key: &str, value: serde_json::Value) -> Result<()> {
        self.data.insert(key.to_string(), value);
        Ok(())
    }
    
    fn available_keys(&self) -> Vec<String> {
        self.data.keys().cloned().collect()
    }
    
    fn reset_to_defaults(&mut self) -> Result<()> {
        self.data.clear();
        self.first_run = true;
        self.onboarding_completed = false;
        Ok(())
    }
    
    fn backup(&self) -> Result<ConfigurationBackup> {
        Ok(ConfigurationBackup {
            timestamp: chrono::Utc::now(),
            data: serde_json::to_value(&self.data)?,
            version: "test".to_string(),
        })
    }
    
    fn restore(&mut self, backup: ConfigurationBackup) -> Result<()> {
        self.data = serde_json::from_value(backup.data)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mock_config_manager() {
        let mut manager = MockConfigManager::new();
        
        assert!(manager.is_first_run());
        assert!(!manager.is_onboarding_completed());
        
        // Test setting values
        assert!(manager.set_value("test_key", "test_value").is_ok());
        assert_eq!(manager.get_value::<String>("test_key"), Some("test_value".to_string()));
        
        // Test onboarding completion
        assert!(manager.mark_onboarding_completed().is_ok());
        assert!(!manager.is_first_run());
        assert!(manager.is_onboarding_completed());
    }
    
    #[test]
    fn test_backup_restore() {
        let mut manager = MockConfigManager::new();
        manager.set_value("test", 42u32).unwrap();
        
        let backup = manager.backup().unwrap();
        manager.set_value("test", 100u32).unwrap();
        
        assert_eq!(manager.get_value::<u32>("test"), Some(100));
        
        manager.restore(backup).unwrap();
        assert_eq!(manager.get_value::<u32>("test"), Some(42));
    }
}