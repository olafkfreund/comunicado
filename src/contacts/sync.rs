use crate::contacts::{ContactsError, ContactsResult, ContactsManager, ContactSource};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, mpsc};
use tokio::time;

/// Contacts synchronization engine for background sync operations
pub struct ContactsSyncEngine {
    manager: Arc<ContactsManager>,
    sync_interval: Duration,
    is_running: Arc<RwLock<bool>>,
    progress_sender: Option<mpsc::UnboundedSender<SyncProgress>>,
}

impl ContactsSyncEngine {
    /// Create a new sync engine
    pub fn new(manager: Arc<ContactsManager>, sync_interval_minutes: u64) -> Self {
        Self {
            manager,
            sync_interval: Duration::from_secs(sync_interval_minutes * 60),
            is_running: Arc::new(RwLock::new(false)),
            progress_sender: None,
        }
    }
    
    /// Start the background sync engine
    pub async fn start(&mut self) -> ContactsResult<mpsc::UnboundedReceiver<SyncProgress>> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.progress_sender = Some(tx.clone());
        
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Err(ContactsError::SyncError("Sync engine is already running".to_string()));
        }
        *is_running = true;
        drop(is_running);
        
        let manager = Arc::clone(&self.manager);
        let sync_interval = self.sync_interval;
        let is_running = Arc::clone(&self.is_running);
        
        // Spawn background sync task
        tokio::spawn(async move {
            let mut interval = time::interval(sync_interval);
            
            // Send initial status
            let _ = tx.send(SyncProgress::Started);
            
            loop {
                interval.tick().await;
                
                // Check if sync is still enabled and running
                let running = *is_running.read().await;
                if !running {
                    let _ = tx.send(SyncProgress::Stopped);
                    break;
                }
                
                let sync_enabled = manager.is_sync_enabled().await;
                if !sync_enabled {
                    continue;
                }
                
                // Perform sync
                let _ = tx.send(SyncProgress::SyncStarted { 
                    timestamp: chrono::Utc::now() 
                });
                
                match manager.sync_all_contacts().await {
                    Ok(summary) => {
                        let _ = tx.send(SyncProgress::SyncCompleted { 
                            summary,
                            timestamp: chrono::Utc::now(),
                        });
                    }
                    Err(e) => {
                        let _ = tx.send(SyncProgress::SyncFailed { 
                            error: e.to_string(),
                            timestamp: chrono::Utc::now(),
                        });
                    }
                }
            }
        });
        
        Ok(rx)
    }
    
    /// Stop the background sync engine
    pub async fn stop(&self) -> ContactsResult<()> {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        
        if let Some(sender) = &self.progress_sender {
            let _ = sender.send(SyncProgress::Stopped);
        }
        
        Ok(())
    }
    
    /// Check if the sync engine is running
    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }
    
    /// Perform a manual sync now
    pub async fn sync_now(&self) -> ContactsResult<()> {
        if let Some(sender) = &self.progress_sender {
            let _ = sender.send(SyncProgress::ManualSyncStarted { 
                timestamp: chrono::Utc::now() 
            });
            
            match self.manager.sync_all_contacts().await {
                Ok(summary) => {
                    let _ = sender.send(SyncProgress::SyncCompleted { 
                        summary,
                        timestamp: chrono::Utc::now(),
                    });
                    Ok(())
                }
                Err(e) => {
                    let _ = sender.send(SyncProgress::SyncFailed { 
                        error: e.to_string(),
                        timestamp: chrono::Utc::now(),
                    });
                    Err(e)
                }
            }
        } else {
            // Engine not started, perform sync directly
            self.manager.sync_all_contacts().await.map(|_| ())
        }
    }
    
    /// Sync specific account
    pub async fn sync_account(&self, account_id: &str, provider_type: &str) -> ContactsResult<()> {
        if let Some(sender) = &self.progress_sender {
            let _ = sender.send(SyncProgress::AccountSyncStarted { 
                account_id: account_id.to_string(),
                provider: provider_type.to_string(),
                timestamp: chrono::Utc::now(),
            });
            
            match self.manager.sync_account_contacts(account_id, provider_type).await {
                Ok(summary) => {
                    let _ = sender.send(SyncProgress::AccountSyncCompleted { 
                        account_id: account_id.to_string(),
                        summary,
                        timestamp: chrono::Utc::now(),
                    });
                    Ok(())
                }
                Err(e) => {
                    let _ = sender.send(SyncProgress::AccountSyncFailed { 
                        account_id: account_id.to_string(),
                        error: e.to_string(),
                        timestamp: chrono::Utc::now(),
                    });
                    Err(e)
                }
            }
        } else {
            // Engine not started, perform sync directly
            self.manager.sync_account_contacts(account_id, provider_type).await.map(|_| ())
        }
    }
    
    /// Update sync interval
    pub fn set_sync_interval(&mut self, interval_minutes: u64) {
        self.sync_interval = Duration::from_secs(interval_minutes * 60);
    }
    
    /// Get current sync interval in minutes
    pub fn get_sync_interval_minutes(&self) -> u64 {
        self.sync_interval.as_secs() / 60
    }
}

/// Progress updates from the sync engine
#[derive(Debug, Clone)]
pub enum SyncProgress {
    /// Sync engine started
    Started,
    
    /// Sync engine stopped
    Stopped,
    
    /// Automatic sync started
    SyncStarted {
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    
    /// Manual sync started
    ManualSyncStarted {
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    
    /// Sync completed successfully
    SyncCompleted {
        summary: crate::contacts::manager::SyncSummary,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    
    /// Sync failed
    SyncFailed {
        error: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    
    /// Account-specific sync started
    AccountSyncStarted {
        account_id: String,
        provider: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    
    /// Account-specific sync completed
    AccountSyncCompleted {
        account_id: String,
        summary: crate::contacts::manager::SyncSummary,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    
    /// Account-specific sync failed
    AccountSyncFailed {
        account_id: String,
        error: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
}

impl SyncProgress {
    /// Get a human-readable message for this progress update
    pub fn message(&self) -> String {
        match self {
            SyncProgress::Started => "Contacts sync engine started".to_string(),
            SyncProgress::Stopped => "Contacts sync engine stopped".to_string(),
            SyncProgress::SyncStarted { .. } => "Starting automatic contacts sync...".to_string(),
            SyncProgress::ManualSyncStarted { .. } => "Starting manual contacts sync...".to_string(),
            SyncProgress::SyncCompleted { summary, .. } => {
                format!(
                    "Sync completed: {} fetched, {} created, {} updated, {} skipped{}",
                    summary.fetched_count,
                    summary.created_count,
                    summary.updated_count,
                    summary.skipped_count,
                    if summary.has_errors() {
                        format!(" ({} errors)", summary.errors.len())
                    } else {
                        String::new()
                    }
                )
            },
            SyncProgress::SyncFailed { error, .. } => {
                format!("Sync failed: {}", error)
            },
            SyncProgress::AccountSyncStarted { account_id, provider, .. } => {
                format!("Syncing {} contacts for {}...", provider, account_id)
            },
            SyncProgress::AccountSyncCompleted { account_id, summary, .. } => {
                format!(
                    "Sync completed for {}: {} fetched, {} created, {} updated",
                    account_id, summary.fetched_count, summary.created_count, summary.updated_count
                )
            },
            SyncProgress::AccountSyncFailed { account_id, error, .. } => {
                format!("Sync failed for {}: {}", account_id, error)
            },
        }
    }
    
    /// Check if this is an error progress update
    pub fn is_error(&self) -> bool {
        matches!(self, SyncProgress::SyncFailed { .. } | SyncProgress::AccountSyncFailed { .. })
    }
    
    /// Get the timestamp of this progress update
    pub fn timestamp(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        match self {
            SyncProgress::Started | SyncProgress::Stopped => None,
            SyncProgress::SyncStarted { timestamp } |
            SyncProgress::ManualSyncStarted { timestamp } |
            SyncProgress::SyncCompleted { timestamp, .. } |
            SyncProgress::SyncFailed { timestamp, .. } |
            SyncProgress::AccountSyncStarted { timestamp, .. } |
            SyncProgress::AccountSyncCompleted { timestamp, .. } |
            SyncProgress::AccountSyncFailed { timestamp, .. } => Some(*timestamp),
        }
    }
}

/// Configuration for contacts synchronization
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// Automatic sync interval in minutes
    pub sync_interval_minutes: u64,
    
    /// Whether automatic sync is enabled
    pub auto_sync_enabled: bool,
    
    /// Maximum number of contacts to sync per account per session
    pub max_contacts_per_sync: Option<usize>,
    
    /// Retry settings for failed syncs
    pub max_retries: u32,
    pub retry_delay_seconds: u64,
    
    /// Sources to sync (empty means sync all)
    pub enabled_sources: Vec<ContactSource>,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            sync_interval_minutes: 60, // Sync every hour
            auto_sync_enabled: true,
            max_contacts_per_sync: Some(1000),
            max_retries: 3,
            retry_delay_seconds: 30,
            enabled_sources: Vec::new(), // Empty means sync all
        }
    }
}

impl SyncConfig {
    /// Create a new sync configuration
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set sync interval in minutes
    pub fn with_interval(mut self, minutes: u64) -> Self {
        self.sync_interval_minutes = minutes;
        self
    }
    
    /// Enable or disable automatic sync
    pub fn with_auto_sync(mut self, enabled: bool) -> Self {
        self.auto_sync_enabled = enabled;
        self
    }
    
    /// Set maximum contacts per sync session
    pub fn with_max_contacts(mut self, max: usize) -> Self {
        self.max_contacts_per_sync = Some(max);
        self
    }
    
    /// Add a contact source to sync
    pub fn with_source(mut self, source: ContactSource) -> Self {
        self.enabled_sources.push(source);
        self
    }
    
    /// Set retry configuration
    pub fn with_retries(mut self, max_retries: u32, delay_seconds: u64) -> Self {
        self.max_retries = max_retries;
        self.retry_delay_seconds = delay_seconds;
        self
    }
    
    /// Check if a source should be synced
    pub fn should_sync_source(&self, source: &ContactSource) -> bool {
        if self.enabled_sources.is_empty() {
            true // Sync all sources if none specified
        } else {
            self.enabled_sources.contains(source)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contacts::{ContactsDatabase, ContactsManager};
    use crate::oauth2::TokenManager;
    
    #[tokio::test]
    async fn test_sync_engine_creation() {
        let database = ContactsDatabase::new(":memory:").await.unwrap();
        let token_manager = TokenManager::new(":memory:".to_string()).await.unwrap();
        let manager = Arc::new(ContactsManager::new(database, token_manager).await.unwrap());
        
        let engine = ContactsSyncEngine::new(manager, 30);
        assert_eq!(engine.get_sync_interval_minutes(), 30);
        assert!(!engine.is_running().await);
    }
    
    #[test]
    fn test_sync_config() {
        let config = SyncConfig::new()
            .with_interval(120)
            .with_auto_sync(false)
            .with_max_contacts(500)
            .with_retries(5, 60);
        
        assert_eq!(config.sync_interval_minutes, 120);
        assert!(!config.auto_sync_enabled);
        assert_eq!(config.max_contacts_per_sync, Some(500));
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.retry_delay_seconds, 60);
    }
    
    #[test]
    fn test_sync_progress_messages() {
        let progress = SyncProgress::Started;
        assert_eq!(progress.message(), "Contacts sync engine started");
        assert!(!progress.is_error());
        assert!(progress.timestamp().is_none());
        
        let error_progress = SyncProgress::SyncFailed {
            error: "Network error".to_string(),
            timestamp: chrono::Utc::now(),
        };
        assert!(error_progress.is_error());
        assert!(error_progress.timestamp().is_some());
    }
}