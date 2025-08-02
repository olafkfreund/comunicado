//! Notification persistence and recovery system
//!
//! This module provides functionality to persist important notifications
//! across application restarts and recover them when the application starts.

use crate::notifications::types::{NotificationEvent, NotificationPriority};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use chrono::{DateTime, Utc, Duration};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Persistent notification data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistentNotification {
    pub id: Uuid,
    pub event: NotificationEvent,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub shown_count: u32,
    pub dismissed: bool,
    pub persistent: bool, // Whether this notification should survive app restarts
}

impl PersistentNotification {
    /// Create a new persistent notification
    pub fn new(event: NotificationEvent, persistent: bool) -> Self {
        let expires_at = match event.priority() {
            NotificationPriority::Critical => None, // Never expires
            NotificationPriority::High => Some(Utc::now() + Duration::hours(24)),
            _ => Some(Utc::now() + Duration::hours(1)),
        };

        Self {
            id: Uuid::new_v4(),
            event,
            created_at: Utc::now(),
            expires_at,
            shown_count: 0,
            dismissed: false,
            persistent,
        }
    }

    /// Check if the notification has expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() > expires_at
        } else {
            false
        }
    }

    /// Check if the notification should be shown
    pub fn should_show(&self) -> bool {
        !self.dismissed && !self.is_expired()
    }

    /// Mark as shown
    pub fn mark_shown(&mut self) {
        self.shown_count += 1;
    }

    /// Mark as dismissed
    pub fn mark_dismissed(&mut self) {
        self.dismissed = true;
    }
}

/// Notification persistence storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationStorage {
    pub version: u32,
    pub notifications: HashMap<Uuid, PersistentNotification>,
    pub settings: PersistenceSettings,
    pub last_cleanup: DateTime<Utc>,
}

impl Default for NotificationStorage {
    fn default() -> Self {
        Self {
            version: 1,
            notifications: HashMap::new(),
            settings: PersistenceSettings::default(),
            last_cleanup: Utc::now(),
        }
    }
}

/// Settings for notification persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceSettings {
    /// Maximum number of notifications to persist
    pub max_persistent_notifications: usize,
    /// How long to keep dismissed notifications (hours)
    pub dismissed_retention_hours: u64,
    /// How long to keep expired notifications (hours)  
    pub expired_retention_hours: u64,
    /// Whether to persist low priority notifications
    pub persist_low_priority: bool,
    /// Cleanup interval in hours
    pub cleanup_interval_hours: u64,
}

impl Default for PersistenceSettings {
    fn default() -> Self {
        Self {
            max_persistent_notifications: 100,
            dismissed_retention_hours: 24,
            expired_retention_hours: 1,
            persist_low_priority: false,
            cleanup_interval_hours: 6,
        }
    }
}

/// Notification persistence manager
pub struct NotificationPersistenceManager {
    storage_path: PathBuf,
    storage: NotificationStorage,
    pending_saves: Vec<PersistentNotification>,
}

impl NotificationPersistenceManager {
    /// Create a new persistence manager
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let storage_path = data_dir.as_ref().join("notifications.json");
        
        // Ensure data directory exists
        if let Some(parent) = storage_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create data directory: {}", e))?;
        }

        let storage = if storage_path.exists() {
            Self::load_storage(&storage_path)?
        } else {
            info!("No existing notification storage found, creating new");
            NotificationStorage::default()
        };

        Ok(Self {
            storage_path,
            storage,
            pending_saves: Vec::new(),
        })
    }

    /// Load storage from file
    fn load_storage(path: &Path) -> Result<NotificationStorage, Box<dyn std::error::Error + Send + Sync>> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read notification storage: {}", e))?;
        
        let mut storage: NotificationStorage = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse notification storage: {}", e))?;

        // Validate and migrate if needed
        if storage.version > 1 {
            warn!("Notification storage version {} is newer than supported version 1", storage.version);
        }

        // Clean up expired notifications on load
        Self::cleanup_storage(&mut storage);

        debug!("Loaded {} notifications from storage", storage.notifications.len());
        Ok(storage)
    }

    /// Save storage to file
    pub fn save_storage(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Add any pending notifications
        for notification in self.pending_saves.drain(..) {
            self.storage.notifications.insert(notification.id, notification);
        }

        // Cleanup before saving
        Self::cleanup_storage(&mut self.storage);

        let content = serde_json::to_string_pretty(&self.storage)
            .map_err(|e| format!("Failed to serialize notification storage: {}", e))?;

        fs::write(&self.storage_path, content)
            .map_err(|e| format!("Failed to write notification storage: {}", e))?;

        debug!("Saved notification storage with {} notifications", self.storage.notifications.len());
        Ok(())
    }

    /// Add a notification to persistence
    pub fn persist_notification(&mut self, event: NotificationEvent) -> Uuid {
        let persistent = self.should_persist_notification(&event);
        let notification = PersistentNotification::new(event, persistent);
        let id = notification.id;

        if persistent {
            self.pending_saves.push(notification);
            debug!("Added notification {} to persistence queue", id);
        }

        id
    }

    /// Get all notifications that should be recovered on startup
    pub fn get_recovery_notifications(&self) -> Vec<PersistentNotification> {
        self.storage.notifications
            .values()
            .filter(|n| n.should_show() && n.persistent)
            .cloned()
            .collect()
    }

    /// Mark notification as shown
    pub fn mark_notification_shown(&mut self, id: Uuid) {
        if let Some(notification) = self.storage.notifications.get_mut(&id) {
            notification.mark_shown();
        }
    }

    /// Mark notification as dismissed
    pub fn mark_notification_dismissed(&mut self, id: Uuid) {
        if let Some(notification) = self.storage.notifications.get_mut(&id) {
            notification.mark_dismissed();
        }
    }

    /// Remove a notification completely
    pub fn remove_notification(&mut self, id: Uuid) {
        self.storage.notifications.remove(&id);
    }

    /// Get notification statistics
    pub fn get_stats(&self) -> PersistenceStats {
        let total = self.storage.notifications.len();
        let active = self.storage.notifications.values()
            .filter(|n| n.should_show())
            .count();
        let dismissed = self.storage.notifications.values()
            .filter(|n| n.dismissed)
            .count();
        let expired = self.storage.notifications.values()
            .filter(|n| n.is_expired())
            .count();

        PersistenceStats {
            total_notifications: total,
            active_notifications: active,
            dismissed_notifications: dismissed,
            expired_notifications: expired,
            last_cleanup: self.storage.last_cleanup,
            storage_size_bytes: self.calculate_storage_size(),
        }
    }

    /// Update persistence settings
    pub fn update_settings(&mut self, settings: PersistenceSettings) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.storage.settings = settings;
        self.save_storage()
    }

    /// Force cleanup of old notifications
    pub fn force_cleanup(&mut self) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        let initial_count = self.storage.notifications.len();
        Self::cleanup_storage(&mut self.storage);
        let removed_count = initial_count - self.storage.notifications.len();
        
        self.save_storage()?;
        
        info!("Cleaned up {} old notifications", removed_count);
        Ok(removed_count)
    }

    /// Export notifications for backup
    pub fn export_notifications(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        serde_json::to_string_pretty(&self.storage)
            .map_err(|e| format!("Failed to export notifications: {}", e).into())
    }

    /// Import notifications from backup
    pub fn import_notifications(&mut self, data: &str) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        let imported_storage: NotificationStorage = serde_json::from_str(data)
            .map_err(|e| format!("Failed to parse imported notifications: {}", e))?;

        let imported_count = imported_storage.notifications.len();
        
        // Merge with existing notifications (imported take precedence)
        for (id, notification) in imported_storage.notifications {
            self.storage.notifications.insert(id, notification);
        }

        self.save_storage()?;
        
        info!("Imported {} notifications", imported_count);
        Ok(imported_count)
    }

    /// Check if a notification should be persisted
    fn should_persist_notification(&self, event: &NotificationEvent) -> bool {
        match event.priority() {
            NotificationPriority::Critical => true,
            NotificationPriority::High => true,
            NotificationPriority::Normal => false,
            NotificationPriority::Low => self.storage.settings.persist_low_priority,
        }
    }

    /// Cleanup expired and old notifications
    fn cleanup_storage(storage: &mut NotificationStorage) {
        let now = Utc::now();
        let settings = &storage.settings;

        // Check if cleanup is needed
        let last_cleanup_age = now.signed_duration_since(storage.last_cleanup);
        if last_cleanup_age < Duration::hours(settings.cleanup_interval_hours as i64) {
            return; // Too soon for cleanup
        }

        let initial_count = storage.notifications.len();

        // Remove expired notifications (older than retention period)
        let expired_cutoff = now - Duration::hours(settings.expired_retention_hours as i64);
        let dismissed_cutoff = now - Duration::hours(settings.dismissed_retention_hours as i64);

        storage.notifications.retain(|_, notification| {
            // Keep if not expired
            if !notification.is_expired() {
                return true;
            }

            // Keep recent expired notifications
            if notification.created_at > expired_cutoff {
                return true;
            }

            // Keep recent dismissed notifications
            if notification.dismissed && notification.created_at > dismissed_cutoff {
                return true;
            }

            // Remove everything else
            false
        });

        // Enforce maximum count limit
        if storage.notifications.len() > settings.max_persistent_notifications {
            // Sort by creation time and keep only the most recent
            let mut notifications: Vec<_> = storage.notifications.drain().collect();
            notifications.sort_by(|a, b| b.1.created_at.cmp(&a.1.created_at));
            notifications.truncate(settings.max_persistent_notifications);
            storage.notifications = notifications.into_iter().collect();
        }

        storage.last_cleanup = now;

        let removed_count = initial_count - storage.notifications.len();
        if removed_count > 0 {
            debug!("Cleaned up {} old notifications", removed_count);
        }
    }

    /// Calculate approximate storage size in bytes
    fn calculate_storage_size(&self) -> usize {
        // Rough estimation based on JSON serialization
        match serde_json::to_string(&self.storage) {
            Ok(json) => json.len(),
            Err(_) => 0,
        }
    }
}

/// Statistics about notification persistence
#[derive(Debug, Clone)]
pub struct PersistenceStats {
    pub total_notifications: usize,
    pub active_notifications: usize,
    pub dismissed_notifications: usize,
    pub expired_notifications: usize,
    pub last_cleanup: DateTime<Utc>,
    pub storage_size_bytes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use crate::notifications::types::*;

    #[test]
    fn test_persistence_manager_creation() {
        let temp_dir = tempdir().unwrap();
        let manager = NotificationPersistenceManager::new(temp_dir.path()).unwrap();
        
        assert_eq!(manager.storage.notifications.len(), 0);
        assert_eq!(manager.storage.version, 1);
    }

    #[test]
    fn test_notification_persistence() {
        let temp_dir = tempdir().unwrap();
        let mut manager = NotificationPersistenceManager::new(temp_dir.path()).unwrap();

        let event = NotificationEvent::Email {
            event_type: EmailEventType::NewMessage,
            account_id: "test".to_string(),
            folder_name: Some("INBOX".to_string()),
            message: None,
            message_id: None,
            priority: NotificationPriority::High,
        };

        let id = manager.persist_notification(event);
        manager.save_storage().unwrap();

        // Create new manager to test persistence
        let manager2 = NotificationPersistenceManager::new(temp_dir.path()).unwrap();
        let recovery_notifications = manager2.get_recovery_notifications();
        
        assert_eq!(recovery_notifications.len(), 1);
        assert_eq!(recovery_notifications[0].id, id);
    }

    #[test]
    fn test_notification_expiration() {
        let mut notification = PersistentNotification::new(
            NotificationEvent::System {
                event_type: crate::notifications::types::SystemEventType::AppStarted,
                message: "Test".to_string(),
                priority: NotificationPriority::Low,
            },
            true
        );

        // Simulate expiration
        notification.expires_at = Some(Utc::now() - Duration::hours(1));
        
        assert!(notification.is_expired());
        assert!(!notification.should_show());
    }

    #[test]
    fn test_cleanup() {
        let temp_dir = tempdir().unwrap();
        let mut manager = NotificationPersistenceManager::new(temp_dir.path()).unwrap();

        // Add expired notification
        let mut expired_notification = PersistentNotification::new(
            NotificationEvent::System {
                event_type: crate::notifications::types::SystemEventType::AppShutdown,
                message: "Expired".to_string(),
                priority: NotificationPriority::Low,
            },
            true
        );
        expired_notification.expires_at = Some(Utc::now() - Duration::hours(25));
        expired_notification.created_at = Utc::now() - Duration::hours(25);
        
        manager.storage.notifications.insert(expired_notification.id, expired_notification);
        
        let removed = manager.force_cleanup().unwrap();
        assert_eq!(removed, 1);
    }
}