//! Automatic email synchronization scheduler
//!
//! This module provides automatic periodic email synchronization functionality
//! that complements the real-time IMAP IDLE system. It handles scheduled syncs,
//! startup synchronization, and configurable sync intervals.

use crate::email::async_sync_service::AsyncSyncService;
use crate::email::sync_engine::{SyncStrategy, SyncProgress};
use crate::email::notifications::EmailNotificationManager;
use crate::imap::ImapAccountManager;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock, Mutex};
use tokio::task::JoinHandle;
use tokio::time::{interval, sleep, Instant};
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Configuration for automatic email synchronization
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AutoSyncConfig {
    /// Whether automatic sync is enabled
    pub enabled: bool,
    /// Sync interval in minutes
    pub sync_interval_minutes: u64,
    /// Whether to sync on application startup
    pub sync_on_startup: bool,
    /// Whether to use incremental sync by default
    pub use_incremental_sync: bool,
    /// Maximum number of concurrent account syncs
    pub max_concurrent_syncs: usize,
    /// Retry attempts for failed syncs
    pub retry_attempts: u32,
    /// Delay between retry attempts in seconds
    pub retry_delay_seconds: u64,
    /// Whether to sync only on network changes
    pub sync_on_network_change: bool,
    /// Whether to respect system power management (reduce sync frequency on battery)
    pub respect_power_management: bool,
}

impl Default for AutoSyncConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sync_interval_minutes: 15,
            sync_on_startup: true,
            use_incremental_sync: true,
            max_concurrent_syncs: 3,
            retry_attempts: 3,
            retry_delay_seconds: 30,
            sync_on_network_change: false,
            respect_power_management: true,
        }
    }
}

/// Statistics about automatic synchronization
#[derive(Debug, Clone)]
pub struct AutoSyncStats {
    /// Whether auto-sync is currently active
    pub is_active: bool,
    /// Next scheduled sync time
    pub next_sync_time: Option<DateTime<Utc>>,
    /// Number of accounts being monitored
    pub monitored_accounts: usize,
    /// Number of active sync operations
    pub active_syncs: usize,
    /// Total sync operations completed
    pub total_syncs_completed: u64,
    /// Total sync operations failed
    pub total_syncs_failed: u64,
    /// Last sync completion time
    pub last_sync_time: Option<DateTime<Utc>>,
    /// Average sync duration
    pub average_sync_duration: Option<Duration>,
}

/// Information about a scheduled sync operation
#[derive(Debug, Clone)]
struct ScheduledSync {
    account_id: String,
    task_id: Option<Uuid>,
    last_sync: Option<DateTime<Utc>>,
    next_sync: DateTime<Utc>,
    retry_count: u32,
    sync_duration_history: Vec<Duration>,
}

/// Automatic email synchronization scheduler
pub struct AutoSyncScheduler {
    config: Arc<RwLock<AutoSyncConfig>>,
    async_sync_service: Arc<AsyncSyncService>,
    account_manager: Arc<ImapAccountManager>,
    notification_manager: Arc<EmailNotificationManager>,
    scheduled_syncs: Arc<RwLock<HashMap<String, ScheduledSync>>>,
    active_sync_tasks: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    stats: Arc<RwLock<AutoSyncStats>>,
    scheduler_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    progress_receiver: Arc<Mutex<Option<mpsc::UnboundedReceiver<SyncProgress>>>>,
    is_running: Arc<RwLock<bool>>,
}

impl AutoSyncScheduler {
    /// Create a new automatic sync scheduler
    pub fn new(
        async_sync_service: Arc<AsyncSyncService>,
        account_manager: Arc<ImapAccountManager>,
        notification_manager: Arc<EmailNotificationManager>,
    ) -> Self {
        Self {
            config: Arc::new(RwLock::new(AutoSyncConfig::default())),
            async_sync_service,
            account_manager,
            notification_manager,
            scheduled_syncs: Arc::new(RwLock::new(HashMap::new())),
            active_sync_tasks: Arc::new(Mutex::new(HashMap::new())),
            stats: Arc::new(RwLock::new(AutoSyncStats {
                is_active: false,
                next_sync_time: None,
                monitored_accounts: 0,
                active_syncs: 0,
                total_syncs_completed: 0,
                total_syncs_failed: 0,
                last_sync_time: None,
                average_sync_duration: None,
            })),
            scheduler_handle: Arc::new(Mutex::new(None)),
            progress_receiver: Arc::new(Mutex::new(None)),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the automatic sync scheduler
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Ok(()); // Already running
        }

        let config = self.config.read().await;
        if !config.enabled {
            info!("Automatic sync is disabled");
            return Ok(());
        }

        info!("Starting automatic email sync scheduler (interval: {} minutes)", config.sync_interval_minutes);

        // Initialize account schedules
        self.initialize_account_schedules().await?;

        // Perform startup sync if enabled
        if config.sync_on_startup {
            info!("Performing startup email sync");
            self.trigger_startup_sync().await?;
        }

        // Start the main scheduler loop
        self.start_scheduler_loop().await?;

        // Start progress monitoring
        self.start_progress_monitoring().await?;

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.is_active = true;
            stats.monitored_accounts = self.scheduled_syncs.read().await.len();
        }

        *is_running = true;
        info!("Automatic sync scheduler started successfully");
        Ok(())
    }

    /// Stop the automatic sync scheduler
    pub async fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut is_running = self.is_running.write().await;
        if !*is_running {
            return Ok(()); // Already stopped
        }

        info!("Stopping automatic email sync scheduler");

        // Stop the main scheduler
        if let Some(handle) = self.scheduler_handle.lock().await.take() {
            handle.abort();
        }

        // Cancel all active sync tasks
        let mut active_tasks = self.active_sync_tasks.lock().await;
        for (account_id, handle) in active_tasks.drain() {
            info!("Cancelling active sync for account: {}", account_id);
            handle.abort();
        }

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.is_active = false;
            stats.active_syncs = 0;
        }

        *is_running = false;
        info!("Automatic sync scheduler stopped");
        Ok(())
    }

    /// Update the synchronization configuration
    pub async fn update_config(&self, new_config: AutoSyncConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let was_enabled = {
            let config = self.config.read().await;
            config.enabled
        };

        {
            let mut config = self.config.write().await;
            *config = new_config.clone();
        }

        info!("Updated auto-sync configuration: {:?}", new_config);

        // Restart scheduler if configuration changed significantly
        if was_enabled != new_config.enabled {
            if new_config.enabled {
                self.start().await?;
            } else {
                self.stop().await?;
            }
        } else if *self.is_running.read().await && new_config.enabled {
            // Update existing schedules
            self.reschedule_all_accounts().await?;
        }

        Ok(())
    }

    /// Force sync all accounts immediately
    pub async fn force_sync_all(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Force syncing all accounts");

        let config = self.config.read().await;
        let strategy = if config.use_incremental_sync {
            SyncStrategy::Incremental
        } else {
            SyncStrategy::Full
        };

        let scheduled_syncs = self.scheduled_syncs.read().await;
        for account_id in scheduled_syncs.keys() {
            self.trigger_account_sync(account_id.clone(), strategy.clone()).await?;
        }

        Ok(())
    }

    /// Force sync a specific account
    pub async fn force_sync_account(&self, account_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Force syncing account: {}", account_id);

        let config = self.config.read().await;
        let strategy = if config.use_incremental_sync {
            SyncStrategy::Incremental
        } else {
            SyncStrategy::Full
        };

        self.trigger_account_sync(account_id.to_string(), strategy).await?;
        Ok(())
    }

    /// Get current sync statistics
    pub async fn get_stats(&self) -> AutoSyncStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Get the current configuration
    pub async fn get_config(&self) -> AutoSyncConfig {
        let config = self.config.read().await;
        config.clone()
    }

    /// Initialize schedules for all configured accounts
    async fn initialize_account_schedules(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let accounts = self.account_manager.get_all_accounts().await;

        let config = self.config.read().await;
        let sync_interval = Duration::from_secs(config.sync_interval_minutes * 60);

        let mut scheduled_syncs = self.scheduled_syncs.write().await;

        for account in accounts {
            let next_sync = Utc::now() + chrono::Duration::from_std(sync_interval)
                .map_err(|_| "Invalid sync interval")?;

            let scheduled_sync = ScheduledSync {
                account_id: account.account_id.clone(),
                task_id: None,
                last_sync: None,
                next_sync,
                retry_count: 0,
                sync_duration_history: Vec::new(),
            };

            scheduled_syncs.insert(account.account_id.clone(), scheduled_sync);
            debug!("Scheduled sync for account {} at {}", account.account_id, next_sync);
        }

        info!("Initialized sync schedules for {} accounts", scheduled_syncs.len());
        Ok(())
    }

    /// Perform startup synchronization
    async fn trigger_startup_sync(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let config = self.config.read().await;
        let strategy = if config.use_incremental_sync {
            SyncStrategy::Incremental
        } else {
            SyncStrategy::Recent(7) // Last week for startup
        };

        let scheduled_syncs = self.scheduled_syncs.read().await;
        let account_ids: Vec<String> = scheduled_syncs.keys().cloned().collect();
        drop(scheduled_syncs);

        // Stagger startup syncs to avoid overwhelming the system
        for (i, account_id) in account_ids.iter().enumerate() {
            if i > 0 {
                sleep(Duration::from_secs(2)).await; // 2-second stagger
            }
            
            if let Err(e) = self.trigger_account_sync(account_id.clone(), strategy.clone()).await {
                warn!("Failed startup sync for account {}: {}", account_id, e);
            }
        }

        info!("Triggered startup sync for {} accounts", account_ids.len());
        Ok(())
    }

    /// Start the main scheduler loop
    async fn start_scheduler_loop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let config = self.config.clone();
        let scheduled_syncs = self.scheduled_syncs.clone();
        let stats = self.stats.clone();
        let async_sync_service = self.async_sync_service.clone();
        let active_sync_tasks = self.active_sync_tasks.clone();

        let handle = tokio::spawn(async move {
            let mut check_interval = interval(Duration::from_secs(60)); // Check every minute

            loop {
                check_interval.tick().await;

                let config_guard = config.read().await;
                if !config_guard.enabled {
                    debug!("Auto-sync disabled, scheduler loop exiting");
                    break;
                }

                let sync_interval = Duration::from_secs(config_guard.sync_interval_minutes * 60);
                let strategy = if config_guard.use_incremental_sync {
                    SyncStrategy::Incremental
                } else {
                    SyncStrategy::Full
                };
                let max_concurrent = config_guard.max_concurrent_syncs;
                drop(config_guard);

                let now = Utc::now();
                let scheduled_syncs_guard = scheduled_syncs.write().await;
                let mut stats_guard = stats.write().await;

                // Check which accounts need syncing
                let mut accounts_to_sync = Vec::new();
                
                for (account_id, scheduled_sync) in scheduled_syncs_guard.iter() {
                    if now >= scheduled_sync.next_sync {
                        accounts_to_sync.push(account_id.clone());
                    }
                }

                // Update next sync time for stats
                stats_guard.next_sync_time = scheduled_syncs_guard
                    .values()
                    .map(|s| s.next_sync)
                    .min();

                drop(scheduled_syncs_guard);
                drop(stats_guard);

                // Check current active syncs
                let active_count = active_sync_tasks.lock().await.len();
                let available_slots = max_concurrent.saturating_sub(active_count);

                // Trigger syncs for accounts that need it (respecting concurrency limit)
                for account_id in accounts_to_sync.into_iter().take(available_slots) {
                    let account_id_clone = account_id.clone();
                    let sync_service = async_sync_service.clone();
                    let scheduled_syncs_clone = scheduled_syncs.clone();
                    let stats_clone = stats.clone();
                    let active_tasks_clone = active_sync_tasks.clone();
                    let strategy_clone = strategy.clone();
                    let sync_interval_clone = sync_interval;

                    let sync_handle = tokio::spawn(async move {
                        let start_time = Instant::now();
                        
                        match sync_service.sync_account_async(account_id_clone.clone(), strategy_clone).await {
                            Ok(task_id) => {
                                info!("Started automatic sync for account {} (task: {})", account_id_clone, task_id);
                                
                                // Update scheduled sync
                                {
                                    let mut scheduled_syncs = scheduled_syncs_clone.write().await;
                                    if let Some(scheduled_sync) = scheduled_syncs.get_mut(&account_id_clone) {
                                        scheduled_sync.task_id = Some(task_id);
                                        scheduled_sync.last_sync = Some(Utc::now());
                                        scheduled_sync.next_sync = Utc::now() + chrono::Duration::from_std(sync_interval_clone).unwrap_or(chrono::Duration::minutes(15));
                                        scheduled_sync.retry_count = 0;
                                        
                                        let duration = start_time.elapsed();
                                        scheduled_sync.sync_duration_history.push(duration);
                                        
                                        // Keep only last 10 durations for average calculation
                                        if scheduled_sync.sync_duration_history.len() > 10 {
                                            scheduled_sync.sync_duration_history.drain(0..1);
                                        }
                                    }
                                }

                                // Update stats
                                {
                                    let mut stats = stats_clone.write().await;
                                    stats.total_syncs_completed += 1;
                                    stats.last_sync_time = Some(Utc::now());
                                    
                                    // Calculate average duration
                                    let all_durations: Vec<Duration> = {
                                        let scheduled_syncs = scheduled_syncs_clone.read().await;
                                        scheduled_syncs.values()
                                            .flat_map(|s| s.sync_duration_history.iter())
                                            .cloned()
                                            .collect()
                                    };
                                    
                                    if !all_durations.is_empty() {
                                        let total: Duration = all_durations.iter().sum();
                                        stats.average_sync_duration = Some(total / all_durations.len() as u32);
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to start automatic sync for account {}: {}", account_id_clone, e);
                                
                                // Update retry count and schedule retry
                                {
                                    let mut scheduled_syncs = scheduled_syncs_clone.write().await;
                                    if let Some(scheduled_sync) = scheduled_syncs.get_mut(&account_id_clone) {
                                        scheduled_sync.retry_count += 1;
                                        
                                        // Schedule retry with exponential backoff
                                        let retry_delay = Duration::from_secs(30 * (1 << scheduled_sync.retry_count.min(5)));
                                        scheduled_sync.next_sync = Utc::now() + chrono::Duration::from_std(retry_delay).unwrap_or(chrono::Duration::minutes(1));
                                    }
                                }

                                // Update stats
                                {
                                    let mut stats = stats_clone.write().await;
                                    stats.total_syncs_failed += 1;
                                }
                            }
                        }
                        
                        // Remove from active tasks
                        active_tasks_clone.lock().await.remove(&account_id_clone);
                    });

                    // Add to active tasks
                    active_sync_tasks.lock().await.insert(account_id, sync_handle);
                }

                // Update active sync count in stats
                {
                    let mut stats_guard = stats.write().await;
                    stats_guard.active_syncs = active_sync_tasks.lock().await.len();
                }
            }
        });

        *self.scheduler_handle.lock().await = Some(handle);
        Ok(())
    }

    /// Start monitoring sync progress
    async fn start_progress_monitoring(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Progress monitoring would be implemented here
        // For now, this is a placeholder as the sync service handles its own progress
        Ok(())
    }

    /// Trigger sync for a specific account
    async fn trigger_account_sync(&self, account_id: String, strategy: SyncStrategy) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Send sync started notification
        self.notification_manager.notify_sync_started(account_id.clone(), "All Folders".to_string()).await;

        match self.async_sync_service.sync_account_async(account_id.clone(), strategy).await {
            Ok(task_id) => {
                debug!("Triggered sync for account {} (task: {})", account_id, task_id);
                Ok(())
            }
            Err(e) => {
                error!("Failed to trigger sync for account {}: {}", account_id, e);
                
                // Send sync failed notification
                self.notification_manager.notify_sync_failed(
                    account_id,
                    "All Folders".to_string(),
                    format!("Sync failed: {}", e)
                ).await;
                
                Err(e.into())
            }
        }
    }

    /// Reschedule all accounts with updated configuration
    async fn reschedule_all_accounts(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let config = self.config.read().await;
        let sync_interval = Duration::from_secs(config.sync_interval_minutes * 60);

        let mut scheduled_syncs = self.scheduled_syncs.write().await;
        for scheduled_sync in scheduled_syncs.values_mut() {
            // Update next sync time based on new interval
            let time_since_last = scheduled_sync.last_sync
                .map(|last| Utc::now().signed_duration_since(last))
                .and_then(|d| d.to_std().ok())
                .unwrap_or(Duration::ZERO);

            if time_since_last >= sync_interval {
                // Should sync now
                scheduled_sync.next_sync = Utc::now();
            } else {
                // Schedule for remaining interval
                let remaining = sync_interval - time_since_last;
                scheduled_sync.next_sync = Utc::now() + chrono::Duration::from_std(remaining)
                    .unwrap_or(chrono::Duration::minutes(1));
            }
        }

        info!("Rescheduled {} accounts with new sync interval", scheduled_syncs.len());
        Ok(())
    }
}

/// Helper function to check if system is on battery power (placeholder)
#[allow(dead_code)]
fn is_on_battery_power() -> bool {
    // This would implement actual battery detection
    // For now, always return false (assume AC power)
    false
}

/// Helper function to check network connectivity (placeholder)
#[allow(dead_code)]
fn is_network_available() -> bool {
    // This would implement actual network connectivity check
    // For now, always return true
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_sync_config_default() {
        let config = AutoSyncConfig::default();
        assert!(config.enabled);
        assert_eq!(config.sync_interval_minutes, 15);
        assert!(config.sync_on_startup);
        assert!(config.use_incremental_sync);
    }

    #[test]
    fn test_auto_sync_stats_default() {
        let stats = AutoSyncStats {
            is_active: false,
            next_sync_time: None,
            monitored_accounts: 0,
            active_syncs: 0,
            total_syncs_completed: 0,
            total_syncs_failed: 0,
            last_sync_time: None,
            average_sync_duration: None,
        };
        
        assert!(!stats.is_active);
        assert_eq!(stats.monitored_accounts, 0);
    }
}