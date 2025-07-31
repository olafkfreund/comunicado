// Integration layer for offline storage with calendar and contacts managers
// Provides sync capabilities between online services and local .ics/.vcf files

use crate::calendar::{Calendar, CalendarManager};
use crate::contacts::{Contact, ContactsManager};
use crate::offline_storage::{OfflineStorageManager, OfflineStorageError};
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use tokio::sync::RwLock;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Integration manager that coordinates between online services and offline storage
pub struct OfflineIntegrationManager {
    /// Calendar manager for online operations
    calendar_manager: Option<Arc<CalendarManager>>,
    /// Contacts manager for online operations
    contacts_manager: Option<Arc<ContactsManager>>,
    /// Offline storage manager
    offline_storage: Arc<RwLock<OfflineStorageManager>>,
    /// Sync strategy configuration
    sync_config: SyncConfig,
}

/// Configuration for sync behavior
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// Enable automatic sync on startup
    pub auto_sync_on_startup: bool,
    /// Sync interval in minutes (0 = disabled)
    pub sync_interval_minutes: u64,
    /// Conflict resolution strategy
    pub conflict_resolution: ConflictResolution,
    /// Enable offline-first mode (prefer local data)
    pub offline_first: bool,
    /// Backup before sync
    pub backup_before_sync: bool,
}

/// Conflict resolution strategies
#[derive(Debug, Clone, PartialEq)]
pub enum ConflictResolution {
    /// Local changes take precedence
    LocalWins,
    /// Remote changes take precedence  
    RemoteWins,
    /// Use most recent timestamp
    MostRecent,
    /// Ask user to resolve conflicts
    Manual,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            auto_sync_on_startup: true,
            sync_interval_minutes: 30, // Sync every 30 minutes
            conflict_resolution: ConflictResolution::MostRecent,
            offline_first: true,
            backup_before_sync: true,
        }
    }
}

impl OfflineIntegrationManager {
    /// Create a new integration manager
    pub async fn new(
        storage_dir: Option<PathBuf>,
        sync_config: Option<SyncConfig>,
    ) -> Result<Self, OfflineIntegrationError> {
        let base_dir = storage_dir.unwrap_or_else(|| OfflineStorageManager::default_storage_dir());
        let offline_storage = OfflineStorageManager::new(base_dir).await
            .map_err(OfflineIntegrationError::StorageError)?;

        info!("Initialized offline integration manager");

        Ok(Self {
            calendar_manager: None,
            contacts_manager: None,
            offline_storage: Arc::new(RwLock::new(offline_storage)),
            sync_config: sync_config.unwrap_or_default(),
        })
    }

    /// Set the calendar manager for online operations
    pub fn set_calendar_manager(&mut self, manager: Arc<CalendarManager>) {
        self.calendar_manager = Some(manager);
        debug!("Calendar manager connected to offline integration");
    }

    /// Set the contacts manager for online operations
    pub fn set_contacts_manager(&mut self, manager: Arc<ContactsManager>) {
        self.contacts_manager = Some(manager);
        debug!("Contacts manager connected to offline integration");
    }

    /// Load calendars with offline-first strategy
    pub async fn load_calendars(&self) -> Result<Vec<Calendar>, OfflineIntegrationError> {
        if self.sync_config.offline_first {
            debug!("Loading calendars offline-first");
            
            // Try loading from offline storage first
            match self.offline_storage.write().await.load_calendars().await {
                Ok(calendars) => {
                    info!("Loaded {} calendars from offline storage", calendars.len());
                    
                    // Optionally sync with online if manager is available
                    if let Some(ref manager) = self.calendar_manager {
                        tokio::spawn({
                            let manager = manager.clone();
                            let storage = self.offline_storage.clone();
                            async move {
                                if let Err(e) = Self::background_calendar_sync(manager, storage).await {
                                    warn!("Background calendar sync failed: {}", e);
                                }
                            }
                        });
                    }
                    
                    return Ok(calendars);
                }
                Err(e) => {
                    warn!("Failed to load calendars from offline storage: {}", e);
                }
            }
        }

        // Fallback to online loading
        if let Some(ref manager) = self.calendar_manager {
            debug!("Loading calendars from online service");
            let calendars = manager.get_calendars().await;
            info!("Loaded {} calendars from online service", calendars.len());
            
            // Save to offline storage for future use
            let mut storage = self.offline_storage.write().await;
            for calendar in &calendars {
                if let Err(e) = storage.save_calendar(calendar).await {
                    warn!("Failed to save calendar {} to offline storage: {}", calendar.name, e);
                }
            }
            
            return Ok(calendars);
        }

        Err(OfflineIntegrationError::NoDataSource("No calendar manager available".to_string()))
    }

    /// Load contacts with offline-first strategy
    pub async fn load_contacts(&self) -> Result<Vec<Contact>, OfflineIntegrationError> {
        if self.sync_config.offline_first {
            debug!("Loading contacts offline-first");
            
            // Try loading from offline storage first
            match self.offline_storage.write().await.load_contacts().await {
                Ok(contacts) => {
                    info!("Loaded {} contacts from offline storage", contacts.len());
                    
                    // Optionally sync with online if manager is available
                    if let Some(ref manager) = self.contacts_manager {
                        tokio::spawn({
                            let manager = manager.clone();
                            let storage = self.offline_storage.clone();
                            async move {
                                if let Err(e) = Self::background_contacts_sync(manager, storage).await {
                                    warn!("Background contacts sync failed: {}", e);
                                }
                            }
                        });
                    }
                    
                    return Ok(contacts);
                }
                Err(e) => {
                    warn!("Failed to load contacts from offline storage: {}", e);
                }
            }
        }

        // Fallback to online loading
        if let Some(ref _manager) = self.contacts_manager {
            debug!("Loading contacts from online service");
            // Note: ContactsManager doesn't have a get_all_contacts method in the current implementation
            // This would need to be added or we'd use search with empty criteria
            warn!("Online contacts loading not implemented - ContactsManager needs get_all_contacts method");
        }

        Err(OfflineIntegrationError::NoDataSource("No contacts data available".to_string()))
    }

    /// Save a calendar with dual storage
    pub async fn save_calendar(&self, calendar: &Calendar) -> Result<(), OfflineIntegrationError> {
        debug!("Saving calendar: {}", calendar.name);

        // Always save to offline storage first
        self.offline_storage.write().await.save_calendar(calendar).await
            .map_err(OfflineIntegrationError::StorageError)?;

        // TODO: Add support for syncing with online services
        // The calendar manager doesn't have a generic create_calendar method
        // Each provider (Google, CalDAV) has its own creation method
        if let Some(ref _manager) = self.calendar_manager {
            debug!("Online calendar sync not yet implemented for saving");
        }

        Ok(())
    }

    /// Save a contact with dual storage
    pub async fn save_contact(&self, contact: &Contact) -> Result<(), OfflineIntegrationError> {
        debug!("Saving contact: {}", contact.display_name);

        // Always save to offline storage first
        self.offline_storage.write().await.save_contact(contact).await
            .map_err(OfflineIntegrationError::StorageError)?;

        // TODO: Add support for syncing with online services
        // The contacts manager create_contact method may not exist or have different signature
        if let Some(ref _manager) = self.contacts_manager {
            debug!("Online contacts sync not yet implemented for saving");
        }

        Ok(())
    }

    /// Perform full sync between online services and offline storage
    pub async fn full_sync(&self) -> Result<SyncResults, OfflineIntegrationError> {
        info!("Starting full sync between online services and offline storage");

        let mut results = SyncResults::default();

        // Backup before sync if configured
        if self.sync_config.backup_before_sync {
            if let Err(e) = self.create_backup().await {
                warn!("Failed to create backup before sync: {}", e);
            }
        }

        // Sync calendars
        if let Some(ref manager) = self.calendar_manager {
            match self.sync_calendars(manager.clone()).await {
                Ok(calendar_results) => {
                    results.calendars_synced = calendar_results.calendars_synced;
                    results.calendar_conflicts = calendar_results.calendar_conflicts;
                }
                Err(e) => {
                    error!("Calendar sync failed: {}", e);
                    results.errors.push(format!("Calendar sync: {}", e));
                }
            }
        }

        // Sync contacts
        if let Some(ref manager) = self.contacts_manager {
            match self.sync_contacts(manager.clone()).await {
                Ok(contact_results) => {
                    results.contacts_synced = contact_results.contacts_synced;
                    results.contact_conflicts = contact_results.contact_conflicts;
                }
                Err(e) => {
                    error!("Contacts sync failed: {}", e);
                    results.errors.push(format!("Contacts sync: {}", e));
                }
            }
        }

        results.sync_completed_at = Some(Utc::now());
        info!("Full sync completed: {} calendars, {} contacts synced", 
              results.calendars_synced, results.contacts_synced);

        Ok(results)
    }

    /// Export all data to a directory
    pub async fn export_all(&self, export_dir: &std::path::Path) -> Result<ExportResults, OfflineIntegrationError> {
        info!("Exporting all data to: {}", export_dir.display());

        let calendar_count = self.offline_storage.read().await.export_calendars(export_dir).await
            .map_err(OfflineIntegrationError::StorageError)?;

        let contact_count = self.offline_storage.read().await.export_contacts(export_dir).await
            .map_err(OfflineIntegrationError::StorageError)?;

        let results = ExportResults {
            calendar_count,
            contact_count,
            export_path: export_dir.to_path_buf(),
            exported_at: Utc::now(),
        };

        info!("Export completed: {} calendars, {} contacts", calendar_count, contact_count);
        Ok(results)
    }

    /// Import data from a directory
    pub async fn import_all(&self, import_dir: &std::path::Path) -> Result<ImportResults, OfflineIntegrationError> {
        info!("Importing data from: {}", import_dir.display());

        let calendar_count = self.offline_storage.write().await.import_calendars(import_dir).await
            .map_err(OfflineIntegrationError::StorageError)?;

        let contact_count = self.offline_storage.write().await.import_contacts(import_dir).await
            .map_err(OfflineIntegrationError::StorageError)?;

        let results = ImportResults {
            calendar_count,
            contact_count,
            import_path: import_dir.to_path_buf(),
            imported_at: Utc::now(),
        };

        info!("Import completed: {} calendars, {} contacts", calendar_count, contact_count);
        Ok(results)
    }

    /// Get storage statistics
    pub async fn get_storage_stats(&self) -> Result<crate::offline_storage::StorageStats, OfflineIntegrationError> {
        self.offline_storage.read().await.get_storage_stats().await
            .map_err(OfflineIntegrationError::StorageError)
    }

    // Private helper methods

    /// Background sync for calendars
    async fn background_calendar_sync(
        _manager: Arc<CalendarManager>,
        _storage: Arc<RwLock<OfflineStorageManager>>,
    ) -> Result<(), OfflineIntegrationError> {
        // TODO: Implement background calendar sync
        debug!("Background calendar sync not yet implemented");
        Ok(())
    }

    /// Background sync for contacts
    async fn background_contacts_sync(
        _manager: Arc<ContactsManager>,
        _storage: Arc<RwLock<OfflineStorageManager>>,
    ) -> Result<(), OfflineIntegrationError> {
        // TODO: Implement background contacts sync
        debug!("Background contacts sync not yet implemented");
        Ok(())
    }

    /// Sync calendars between online and offline
    async fn sync_calendars(&self, _manager: Arc<CalendarManager>) -> Result<CalendarSyncResults, OfflineIntegrationError> {
        // TODO: Implement calendar sync logic
        debug!("Calendar sync not yet implemented");
        Ok(CalendarSyncResults::default())
    }

    /// Sync contacts between online and offline
    async fn sync_contacts(&self, _manager: Arc<ContactsManager>) -> Result<ContactSyncResults, OfflineIntegrationError> {
        // TODO: Implement contacts sync logic
        debug!("Contacts sync not yet implemented");
        Ok(ContactSyncResults::default())
    }

    /// Create a backup of current offline storage
    async fn create_backup(&self) -> Result<(), OfflineIntegrationError> {
        // TODO: Implement backup functionality
        debug!("Backup creation not yet implemented");
        Ok(())
    }
}

// Result types for sync operations

#[derive(Debug, Default)]
pub struct SyncResults {
    pub calendars_synced: usize,
    pub contacts_synced: usize,
    pub calendar_conflicts: usize,
    pub contact_conflicts: usize,
    pub errors: Vec<String>,
    pub sync_completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Default)]
pub struct CalendarSyncResults {
    pub calendars_synced: usize,
    pub calendar_conflicts: usize,
}

#[derive(Debug, Default)]
pub struct ContactSyncResults {
    pub contacts_synced: usize,
    pub contact_conflicts: usize,
}

#[derive(Debug)]
pub struct ExportResults {
    pub calendar_count: usize,
    pub contact_count: usize,
    pub export_path: PathBuf,
    pub exported_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct ImportResults {
    pub calendar_count: usize,
    pub contact_count: usize,
    pub import_path: PathBuf,
    pub imported_at: DateTime<Utc>,
}

/// Errors that can occur during offline integration operations
#[derive(Debug, thiserror::Error)]
pub enum OfflineIntegrationError {
    #[error("Storage error: {0}")]
    StorageError(#[from] OfflineStorageError),

    #[error("Online service error: {0}")]
    OnlineError(String),

    #[error("Sync conflict: {0}")]
    SyncConflict(String),

    #[error("No data source available: {0}")]
    NoDataSource(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_integration_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = OfflineIntegrationManager::new(
            Some(temp_dir.path().to_path_buf()),
            None,
        ).await.unwrap();

        // Verify default configuration
        assert!(manager.sync_config.offline_first);
        assert_eq!(manager.sync_config.sync_interval_minutes, 30);
    }

    #[tokio::test]
    async fn test_load_calendars_offline_first() {
        let temp_dir = TempDir::new().unwrap();
        let manager = OfflineIntegrationManager::new(
            Some(temp_dir.path().to_path_buf()),
            None,
        ).await.unwrap();

        // Should fail gracefully when no data is available
        let result = manager.load_calendars().await;
        assert!(result.is_err());
    }
}