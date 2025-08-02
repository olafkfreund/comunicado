//! Integration test for automatic email synchronization functionality
//!
//! This test verifies that the AutoSyncScheduler configuration and basic
//! functionality works correctly.

#[cfg(test)]
mod tests {
    use comunicado::email::{
        auto_sync_scheduler::AutoSyncConfig,
        sync_config::{SyncConfigManager, SyncConfigFile, AccountSyncSettings},
    };
    use std::collections::HashMap;
    use tempfile::tempdir;

    #[test]
    fn test_auto_sync_config_default() {
        // Test default configuration values
        let config = AutoSyncConfig::default();
        
        assert!(config.enabled);
        assert_eq!(config.sync_interval_minutes, 15);
        assert!(config.sync_on_startup);
        assert!(config.use_incremental_sync);
        assert_eq!(config.max_concurrent_syncs, 3);
        assert_eq!(config.retry_attempts, 3);
        assert_eq!(config.retry_delay_seconds, 30);
        assert!(!config.sync_on_network_change);
        assert!(config.respect_power_management);
    }

    #[test]
    fn test_auto_sync_config_serialization() {
        // Test that AutoSyncConfig can be serialized and deserialized
        let config = AutoSyncConfig {
            enabled: true,
            sync_interval_minutes: 30,
            sync_on_startup: false,
            use_incremental_sync: true,
            max_concurrent_syncs: 5,
            retry_attempts: 2,
            retry_delay_seconds: 60,
            sync_on_network_change: true,
            respect_power_management: false,
        };

        // Serialize to JSON
        let json = serde_json::to_string(&config).expect("Failed to serialize config");
        assert!(json.contains("\"enabled\":true"));
        assert!(json.contains("\"sync_interval_minutes\":30"));

        // Deserialize from JSON
        let deserialized: AutoSyncConfig = serde_json::from_str(&json)
            .expect("Failed to deserialize config");
        
        assert_eq!(deserialized.enabled, config.enabled);
        assert_eq!(deserialized.sync_interval_minutes, config.sync_interval_minutes);
        assert_eq!(deserialized.sync_on_startup, config.sync_on_startup);
        assert_eq!(deserialized.max_concurrent_syncs, config.max_concurrent_syncs);
    }

    #[test]
    fn test_sync_config_file_creation() {
        // Test creating and serializing a sync config file
        let auto_sync = AutoSyncConfig::default();
        let mut account_settings = HashMap::new();
        
        account_settings.insert("test-account".to_string(), AccountSyncSettings {
            enabled: true,
            custom_sync_interval_minutes: Some(60),
            use_incremental_sync: true,
            excluded_folders: vec!["Spam".to_string()],
            priority_folders: vec!["INBOX".to_string(), "Sent".to_string()],
            last_sync: None,
            failure_count: 0,
        });

        let config_file = SyncConfigFile {
            version: 1,
            auto_sync,
            account_settings,
            last_updated: chrono::Utc::now(),
        };

        // Test serialization to TOML
        let toml_str = toml::to_string(&config_file).expect("Failed to serialize to TOML");
        assert!(toml_str.contains("enabled = true"));
        assert!(toml_str.contains("sync_interval_minutes = 15"));
        
        // Test deserialization from TOML
        let deserialized: SyncConfigFile = toml::from_str(&toml_str)
            .expect("Failed to deserialize from TOML");
        
        assert_eq!(deserialized.version, 1);
        assert!(deserialized.auto_sync.enabled);
        assert_eq!(deserialized.account_settings.len(), 1);
        
        let account_setting = deserialized.account_settings.get("test-account").unwrap();
        assert!(account_setting.enabled);
        assert_eq!(account_setting.custom_sync_interval_minutes, Some(60));
        assert_eq!(account_setting.priority_folders.len(), 2);
    }

    #[test]
    fn test_sync_config_manager_basic() {
        // Test basic SyncConfigManager functionality without file I/O
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let config_path = temp_dir.path().join("sync_config.toml");

        let config_manager = SyncConfigManager::new(config_path)
            .expect("Failed to create config manager");

        // Test default config
        let default_config = config_manager.get_auto_sync_config();
        assert!(default_config.enabled);
        assert_eq!(default_config.sync_interval_minutes, 15);
        assert_eq!(default_config.max_concurrent_syncs, 3);

        // Test stats
        let stats = config_manager.get_config_stats();
        assert_eq!(stats.total_accounts, 0);
        assert_eq!(stats.enabled_accounts, 0);
        assert_eq!(stats.accounts_with_failures, 0);
        assert!(stats.auto_sync_enabled);
        assert_eq!(stats.global_sync_interval, 15);
    }

    #[test]
    fn test_notification_persistence_config() {
        // Test that notification persistence can be configured via sync settings
        use comunicado::notifications::persistence::{PersistenceSettings, NotificationStorage};
        
        let settings = PersistenceSettings::default();
        assert_eq!(settings.max_persistent_notifications, 100);
        assert_eq!(settings.dismissed_retention_hours, 24);
        assert_eq!(settings.expired_retention_hours, 1);
        assert!(!settings.persist_low_priority);
        assert_eq!(settings.cleanup_interval_hours, 6);

        let storage = NotificationStorage::default();
        assert_eq!(storage.version, 1);
        assert_eq!(storage.notifications.len(), 0);
    }

    #[test]
    fn test_integration_completeness() {
        // This test verifies that all the major components we implemented
        // are properly exported and accessible

        // Test AutoSyncConfig
        let _config = AutoSyncConfig::default();
        
        // Test that sync config can be created
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let config_path = temp_dir.path().join("sync_config.toml");
        let _manager = SyncConfigManager::new(config_path).expect("Failed to create manager");
        
        // Test that notification persistence types are available
        use comunicado::notifications::persistence::NotificationStorage;
        let _storage = NotificationStorage::default();
        
        // Test account sync settings
        let _account_settings = AccountSyncSettings {
            enabled: true,
            custom_sync_interval_minutes: None,
            use_incremental_sync: true,
            excluded_folders: vec![],
            priority_folders: vec!["INBOX".to_string()],
            last_sync: None,
            failure_count: 0,
        };
        
        println!("✅ All automatic sync components are properly integrated and accessible");
    }
}