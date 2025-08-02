//! Configuration persistence for email synchronization settings
//!
//! This module handles saving and loading auto-sync configuration to ensure
//! settings persist across application restarts.

use crate::email::auto_sync_scheduler::AutoSyncConfig;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use tracing::{debug, info, warn};

/// Configuration file for email sync settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfigFile {
    /// Version of the config file format
    pub version: u32,
    /// Auto sync configuration
    pub auto_sync: AutoSyncConfig,
    /// Per-account sync settings
    pub account_settings: std::collections::HashMap<String, AccountSyncSettings>,
    /// Last update timestamp
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

/// Per-account sync settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountSyncSettings {
    /// Whether sync is enabled for this account
    pub enabled: bool,
    /// Custom sync interval for this account (overrides global)
    pub custom_sync_interval_minutes: Option<u64>,
    /// Whether to use incremental sync for this account
    pub use_incremental_sync: bool,
    /// Folders to exclude from sync
    pub excluded_folders: Vec<String>,
    /// Folders to prioritize (sync first)
    pub priority_folders: Vec<String>,
    /// Last successful sync timestamp
    pub last_sync: Option<chrono::DateTime<chrono::Utc>>,
    /// Sync failure count
    pub failure_count: u32,
}

impl Default for AccountSyncSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            custom_sync_interval_minutes: None,
            use_incremental_sync: true,
            excluded_folders: vec!["Trash".to_string(), "Junk".to_string()],
            priority_folders: vec!["INBOX".to_string()],
            last_sync: None,
            failure_count: 0,
        }
    }
}

impl Default for SyncConfigFile {
    fn default() -> Self {
        Self {
            version: 1,
            auto_sync: AutoSyncConfig::default(),
            account_settings: std::collections::HashMap::new(),
            last_updated: chrono::Utc::now(),
        }
    }
}

/// Configuration manager for sync settings
pub struct SyncConfigManager {
    config_path: PathBuf,
    config: SyncConfigFile,
}

impl SyncConfigManager {
    /// Create a new sync config manager
    pub fn new<P: AsRef<Path>>(config_dir: P) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let config_path = config_dir.as_ref().join("sync_config.toml");
        
        // Ensure config directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }

        let config = if config_path.exists() {
            Self::load_config(&config_path)?
        } else {
            info!("No existing sync config found, creating default");
            SyncConfigFile::default()
        };

        Ok(Self {
            config_path,
            config,
        })
    }

    /// Load configuration from file
    fn load_config(path: &Path) -> Result<SyncConfigFile, Box<dyn std::error::Error + Send + Sync>> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read sync config file: {}", e))?;
        
        let config: SyncConfigFile = toml::from_str(&content)
            .map_err(|e| format!("Failed to parse sync config: {}", e))?;

        // Validate config version
        if config.version > 1 {
            warn!("Sync config version {} is newer than supported version 1", config.version);
        }

        debug!("Loaded sync configuration from {}", path.display());
        Ok(config)
    }

    /// Save configuration to file
    pub fn save_config(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.config.last_updated = chrono::Utc::now();

        let content = toml::to_string_pretty(&self.config)
            .map_err(|e| format!("Failed to serialize sync config: {}", e))?;

        fs::write(&self.config_path, content)
            .map_err(|e| format!("Failed to write sync config file: {}", e))?;

        debug!("Saved sync configuration to {}", self.config_path.display());
        Ok(())
    }

    /// Get the current auto sync configuration
    pub fn get_auto_sync_config(&self) -> &AutoSyncConfig {
        &self.config.auto_sync
    }

    /// Update the auto sync configuration
    pub fn update_auto_sync_config(&mut self, config: AutoSyncConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.config.auto_sync = config;
        self.save_config()
    }

    /// Get account sync settings
    pub fn get_account_settings(&self, account_id: &str) -> AccountSyncSettings {
        self.config.account_settings
            .get(account_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Update account sync settings
    pub fn update_account_settings(&mut self, account_id: String, settings: AccountSyncSettings) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.config.account_settings.insert(account_id, settings);
        self.save_config()
    }

    /// Remove account settings
    pub fn remove_account_settings(&mut self, account_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.config.account_settings.remove(account_id);
        self.save_config()
    }

    /// Update last sync time for an account
    pub fn update_last_sync(&mut self, account_id: &str, sync_time: chrono::DateTime<chrono::Utc>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut settings = self.get_account_settings(account_id);
        settings.last_sync = Some(sync_time);
        settings.failure_count = 0; // Reset failure count on successful sync
        self.update_account_settings(account_id.to_string(), settings)
    }

    /// Increment failure count for an account
    pub fn increment_failure_count(&mut self, account_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut settings = self.get_account_settings(account_id);
        settings.failure_count += 1;
        self.update_account_settings(account_id.to_string(), settings)
    }

    /// Get all account IDs with sync settings
    pub fn get_all_account_ids(&self) -> Vec<String> {
        self.config.account_settings.keys().cloned().collect()
    }

    /// Check if sync is enabled for an account
    pub fn is_account_sync_enabled(&self, account_id: &str) -> bool {
        self.get_account_settings(account_id).enabled
    }

    /// Get effective sync interval for an account
    pub fn get_account_sync_interval(&self, account_id: &str) -> u64 {
        let settings = self.get_account_settings(account_id);
        settings.custom_sync_interval_minutes
            .unwrap_or(self.config.auto_sync.sync_interval_minutes)
    }

    /// Export configuration for backup
    pub fn export_config(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        toml::to_string_pretty(&self.config)
            .map_err(|e| format!("Failed to export sync config: {}", e).into())
    }

    /// Import configuration from backup
    pub fn import_config(&mut self, config_str: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let imported_config: SyncConfigFile = toml::from_str(config_str)
            .map_err(|e| format!("Failed to parse imported sync config: {}", e))?;

        // Validate version compatibility
        if imported_config.version > 1 {
            return Err("Imported config version is not supported".into());
        }

        self.config = imported_config;
        self.save_config()?;
        
        info!("Successfully imported sync configuration");
        Ok(())
    }

    /// Reset configuration to defaults
    pub fn reset_to_defaults(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.config = SyncConfigFile::default();
        self.save_config()?;
        
        info!("Reset sync configuration to defaults");
        Ok(())
    }

    /// Get configuration file path
    pub fn get_config_path(&self) -> &Path {
        &self.config_path
    }

    /// Get configuration statistics
    pub fn get_config_stats(&self) -> ConfigStats {
        ConfigStats {
            total_accounts: self.config.account_settings.len(),
            enabled_accounts: self.config.account_settings.values()
                .filter(|s| s.enabled)
                .count(),
            accounts_with_failures: self.config.account_settings.values()
                .filter(|s| s.failure_count > 0)
                .count(),
            last_updated: self.config.last_updated,
            auto_sync_enabled: self.config.auto_sync.enabled,
            global_sync_interval: self.config.auto_sync.sync_interval_minutes,
        }
    }
}

/// Configuration statistics
#[derive(Debug, Clone)]
pub struct ConfigStats {
    pub total_accounts: usize,
    pub enabled_accounts: usize,
    pub accounts_with_failures: usize,
    pub last_updated: chrono::DateTime<chrono::Utc>,
    pub auto_sync_enabled: bool,
    pub global_sync_interval: u64,
}

/// Configuration migration utilities
pub struct ConfigMigration;

impl ConfigMigration {
    /// Migrate configuration from older versions
    pub fn migrate_if_needed(config: &mut SyncConfigFile) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let original_version = config.version;
        let migrated = false;

        // Add migration logic here as new versions are introduced
        // For now, version 1 is the only supported version

        if migrated {
            config.version = 1;
            config.last_updated = chrono::Utc::now();
            info!("Migrated sync config from version {} to {}", original_version, config.version);
        }

        Ok(migrated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_config_creation_and_save() {
        let temp_dir = tempdir().unwrap();
        let mut config_manager = SyncConfigManager::new(temp_dir.path()).unwrap();
        
        // Test default config
        assert!(config_manager.get_auto_sync_config().enabled);
        
        // Test saving
        config_manager.save_config().unwrap();
        assert!(temp_dir.path().join("sync_config.toml").exists());
    }

    #[test]
    fn test_account_settings() {
        let temp_dir = tempdir().unwrap();
        let mut config_manager = SyncConfigManager::new(temp_dir.path()).unwrap();
        
        let account_id = "test_account";
        let mut settings = AccountSyncSettings::default();
        settings.enabled = false;
        settings.custom_sync_interval_minutes = Some(30);
        
        config_manager.update_account_settings(account_id.to_string(), settings.clone()).unwrap();
        
        let retrieved_settings = config_manager.get_account_settings(account_id);
        assert!(!retrieved_settings.enabled);
        assert_eq!(retrieved_settings.custom_sync_interval_minutes, Some(30));
    }

    #[test]
    fn test_config_export_import() {
        let temp_dir = tempdir().unwrap();
        let mut config_manager = SyncConfigManager::new(temp_dir.path()).unwrap();
        
        // Modify config
        let mut new_config = AutoSyncConfig::default();
        new_config.sync_interval_minutes = 45;
        config_manager.update_auto_sync_config(new_config).unwrap();
        
        // Export
        let exported = config_manager.export_config().unwrap();
        
        // Reset and import
        config_manager.reset_to_defaults().unwrap();
        assert_eq!(config_manager.get_auto_sync_config().sync_interval_minutes, 15);
        
        config_manager.import_config(&exported).unwrap();
        assert_eq!(config_manager.get_auto_sync_config().sync_interval_minutes, 45);
    }
}