use serde::{Deserialize, Serialize};
use chrono::NaiveTime;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileConfig {
    pub enabled: bool,
    pub kde_connect_device_id: Option<String>,
    pub auto_pair: bool,
    pub sms: SmsSettings,
    pub notifications: NotificationSettings,
    pub storage: StorageSettings,
    pub privacy: PrivacySettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmsSettings {
    pub enabled: bool,
    pub sync_interval_seconds: u64,
    pub auto_mark_read: bool,
    pub max_conversations: usize,
    pub archive_after_days: u64,
    pub download_mms_automatically: bool,
    pub notification_on_receive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    pub enabled: bool,
    pub show_preview: bool,
    pub filtered_apps: Vec<String>,
    pub quiet_hours: QuietHoursSettings,
    pub priority_apps: HashMap<String, NotificationPriority>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuietHoursSettings {
    pub enabled: bool,
    #[serde(with = "time_format")]
    pub start_time: NaiveTime,
    #[serde(with = "time_format")]
    pub end_time: NaiveTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageSettings {
    pub database_path: String,
    pub backup_enabled: bool,
    pub backup_interval_hours: u64,
    pub max_backup_files: usize,
    pub retention_days: i64,
    pub cleanup_interval_hours: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacySettings {
    pub log_message_content: bool,
    pub store_contact_names: bool,
    pub notification_preview: bool,
    pub analytics_enabled: bool,
    pub backup_include_content: bool,
    pub encrypt_database: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NotificationPriority {
    Low,
    Normal,
    High,
    Critical,
}

impl Default for MobileConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Disabled by default until user enables
            kde_connect_device_id: None,
            auto_pair: true,
            sms: SmsSettings::default(),
            notifications: NotificationSettings::default(),
            storage: StorageSettings::default(),
            privacy: PrivacySettings::default(),
        }
    }
}

impl Default for SmsSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            sync_interval_seconds: 30,
            auto_mark_read: false,
            max_conversations: 100,
            archive_after_days: 90,
            download_mms_automatically: true,
            notification_on_receive: true,
        }
    }
}

impl Default for NotificationSettings {
    fn default() -> Self {
        let mut priority_apps = HashMap::new();
        
        // High priority messaging apps
        for app in ["Messages", "SMS", "WhatsApp", "Telegram", "Signal"] {
            priority_apps.insert(app.to_string(), NotificationPriority::High);
        }
        
        // Normal priority email apps
        for app in ["Email", "Gmail", "Outlook", "Mail"] {
            priority_apps.insert(app.to_string(), NotificationPriority::Normal);
        }
        
        // Low priority social/news apps
        for app in ["Twitter", "Facebook", "Instagram", "News", "Reddit"] {
            priority_apps.insert(app.to_string(), NotificationPriority::Low);
        }

        Self {
            enabled: true,
            show_preview: true,
            filtered_apps: vec![
                "Messages".to_string(),
                "WhatsApp".to_string(),
                "Telegram".to_string(),
                "Signal".to_string(),
            ],
            quiet_hours: QuietHoursSettings {
                enabled: false,
                start_time: NaiveTime::from_hms_opt(22, 0, 0).unwrap(),
                end_time: NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
            },
            priority_apps,
        }
    }
}

impl Default for StorageSettings {
    fn default() -> Self {
        Self {
            database_path: "data/mobile.db".to_string(),
            backup_enabled: true,
            backup_interval_hours: 24,
            max_backup_files: 7,
            retention_days: 365, // Keep data for 1 year
            cleanup_interval_hours: 6,
        }
    }
}

impl Default for PrivacySettings {
    fn default() -> Self {
        Self {
            log_message_content: false, // Privacy-first default
            store_contact_names: true,
            notification_preview: true,
            analytics_enabled: false, // Privacy-first default
            backup_include_content: false, // Privacy-first default
            encrypt_database: true, // Security-first default
        }
    }
}

impl MobileConfig {
    pub fn load_from_file(path: &str) -> crate::mobile::Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| crate::mobile::MobileError::IoError(e))?;
        
        let config: Self = toml::from_str(&content)
            .map_err(|e| crate::mobile::MobileError::ConfigurationError(e.to_string()))?;
        
        config.validate()?;
        Ok(config)
    }

    pub fn save_to_file(&self, path: &str) -> crate::mobile::Result<()> {
        self.validate()?;
        
        let content = toml::to_string_pretty(self)
            .map_err(|e| crate::mobile::MobileError::ConfigurationError(e.to_string()))?;
        
        // Ensure directory exists
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| crate::mobile::MobileError::IoError(e))?;
        }
        
        std::fs::write(path, content)
            .map_err(|e| crate::mobile::MobileError::IoError(e))?;
        
        Ok(())
    }

    pub fn validate(&self) -> crate::mobile::Result<()> {
        // Validate sync interval
        if self.sms.sync_interval_seconds == 0 {
            return Err(crate::mobile::MobileError::ConfigurationError(
                "Sync interval cannot be zero".to_string()
            ));
        }

        // Validate max conversations
        if self.sms.max_conversations == 0 {
            return Err(crate::mobile::MobileError::ConfigurationError(
                "Max conversations must be greater than zero".to_string()
            ));
        }

        // Validate retention days
        if self.storage.retention_days <= 0 {
            return Err(crate::mobile::MobileError::ConfigurationError(
                "Retention days must be positive".to_string()
            ));
        }

        // Validate backup settings
        if self.storage.backup_enabled && self.storage.max_backup_files == 0 {
            return Err(crate::mobile::MobileError::ConfigurationError(
                "Max backup files must be greater than zero when backup is enabled".to_string()
            ));
        }

        Ok(())
    }

    pub fn get_effective_sync_interval(&self) -> u64 {
        if self.enabled && self.sms.enabled {
            self.sms.sync_interval_seconds
        } else {
            u64::MAX // Effectively disabled
        }
    }

    pub fn is_in_quiet_hours(&self) -> bool {
        if !self.notifications.quiet_hours.enabled {
            return false;
        }

        let now = chrono::Local::now().time();
        let start = self.notifications.quiet_hours.start_time;
        let end = self.notifications.quiet_hours.end_time;

        if start <= end {
            // Same day range (e.g., 09:00 to 17:00)
            now >= start && now <= end
        } else {
            // Overnight range (e.g., 22:00 to 08:00)
            now >= start || now <= end
        }
    }

    pub fn get_notification_priority(&self, app_name: &str) -> NotificationPriority {
        self.notifications.priority_apps
            .get(app_name)
            .cloned()
            .unwrap_or(NotificationPriority::Normal)
    }

    pub fn should_forward_notification(&self, app_name: &str) -> bool {
        if !self.notifications.enabled {
            return false;
        }

        if self.is_in_quiet_hours() {
            // Only allow critical notifications during quiet hours
            return self.get_notification_priority(app_name) == NotificationPriority::Critical;
        }

        if self.notifications.filtered_apps.is_empty() {
            // No filter means all apps are allowed
            true
        } else {
            // Check if app is in the allowed list
            self.notifications.filtered_apps.contains(&app_name.to_string())
        }
    }

    pub fn update_device_id(&mut self, device_id: Option<String>) {
        self.kde_connect_device_id = device_id;
    }

    pub fn enable_mobile_integration(&mut self) {
        self.enabled = true;
        self.sms.enabled = true;
        self.notifications.enabled = true;
    }

    pub fn disable_mobile_integration(&mut self) {
        self.enabled = false;
        self.sms.enabled = false;
        self.notifications.enabled = false;
    }
}

// Custom serialization for NaiveTime to handle TOML format
mod time_format {
    use chrono::NaiveTime;
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FORMAT: &str = "%H:%M";

    pub fn serialize<S>(time: &NaiveTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", time.format(FORMAT));
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        NaiveTime::parse_from_str(&s, FORMAT).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = MobileConfig::default();
        assert!(!config.enabled); // Should be disabled by default
        assert!(config.sms.enabled); // SMS should be enabled when mobile is enabled
        assert!(config.notifications.enabled);
        assert_eq!(config.sms.sync_interval_seconds, 30);
        assert_eq!(config.storage.retention_days, 365);
    }

    #[test]
    fn test_config_validation() {
        let mut config = MobileConfig::default();
        assert!(config.validate().is_ok());

        // Test invalid sync interval
        config.sms.sync_interval_seconds = 0;
        assert!(config.validate().is_err());

        config.sms.sync_interval_seconds = 30;
        assert!(config.validate().is_ok());

        // Test invalid max conversations
        config.sms.max_conversations = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_quiet_hours() {
        let mut config = MobileConfig::default();
        config.notifications.quiet_hours.enabled = true;
        
        // Test same-day range (9 AM to 5 PM)
        config.notifications.quiet_hours.start_time = NaiveTime::from_hms_opt(9, 0, 0).unwrap();
        config.notifications.quiet_hours.end_time = NaiveTime::from_hms_opt(17, 0, 0).unwrap();
        
        // Note: This test would need to mock the current time for proper testing
        // For now, just test that the method doesn't panic
        let _ = config.is_in_quiet_hours();
    }

    #[test]
    fn test_notification_priority() {
        let config = MobileConfig::default();
        
        assert_eq!(config.get_notification_priority("Messages"), NotificationPriority::High);
        assert_eq!(config.get_notification_priority("WhatsApp"), NotificationPriority::High);
        assert_eq!(config.get_notification_priority("Gmail"), NotificationPriority::Normal);
        assert_eq!(config.get_notification_priority("Twitter"), NotificationPriority::Low);
        assert_eq!(config.get_notification_priority("UnknownApp"), NotificationPriority::Normal);
    }

    #[test]
    fn test_notification_filtering() {
        let config = MobileConfig::default();
        
        // Should forward apps in the filtered list
        assert!(config.should_forward_notification("Messages"));
        assert!(config.should_forward_notification("WhatsApp"));
        
        // Should not forward apps not in the filtered list
        assert!(!config.should_forward_notification("Twitter"));
        assert!(!config.should_forward_notification("UnknownApp"));
    }

    #[test]
    fn test_config_serialization() {
        let config = MobileConfig::default();
        
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();
        
        // Test save
        config.save_to_file(path).unwrap();
        
        // Test load
        let loaded_config = MobileConfig::load_from_file(path).unwrap();
        
        // Compare key fields
        assert_eq!(config.enabled, loaded_config.enabled);
        assert_eq!(config.sms.sync_interval_seconds, loaded_config.sms.sync_interval_seconds);
        assert_eq!(config.storage.retention_days, loaded_config.storage.retention_days);
    }

    #[test]
    fn test_mobile_integration_toggle() {
        let mut config = MobileConfig::default();
        assert!(!config.enabled);
        
        config.enable_mobile_integration();
        assert!(config.enabled);
        assert!(config.sms.enabled);
        assert!(config.notifications.enabled);
        
        config.disable_mobile_integration();
        assert!(!config.enabled);
        assert!(!config.sms.enabled);
        assert!(!config.notifications.enabled);
    }
}