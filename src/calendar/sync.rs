use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::calendar::{
    caldav::{CalDAVClient, CalDAVConfig, CalDAVQuery},
    event::Event,
    CalendarError, CalendarManager, CalendarResult,
};

/// Calendar synchronization engine
pub struct CalendarSyncEngine {
    calendar_manager: Arc<CalendarManager>,
    sync_configs: RwLock<HashMap<String, CalendarSyncConfig>>,
    sync_status: RwLock<HashMap<String, CalendarSyncStatus>>,
    is_running: RwLock<bool>,
}

/// Configuration for calendar synchronization
#[derive(Debug, Clone)]
pub struct CalendarSyncConfig {
    pub calendar_id: String,
    pub account_id: String,
    pub sync_interval_minutes: u64,
    pub enabled: bool,
    pub last_sync: Option<DateTime<Utc>>,
    pub sync_token: Option<String>,
    pub caldav_config: Option<CalDAVConfig>,
}

/// Calendar synchronization status
#[derive(Debug, Clone)]
pub struct CalendarSyncStatus {
    pub calendar_id: String,
    pub status: SyncState,
    pub last_sync: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub events_synced: u32,
    pub progress: Option<CalendarSyncProgress>,
}

/// Synchronization state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncState {
    Idle,
    Syncing,
    Completed,
    Error,
    Disabled,
}

/// Detailed synchronization progress
#[derive(Debug, Clone)]
pub struct CalendarSyncProgress {
    pub phase: SyncPhase,
    pub events_processed: u32,
    pub total_events: u32,
    pub started_at: DateTime<Utc>,
    pub estimated_completion: Option<DateTime<Utc>>,
    pub current_operation: String,
}

/// Synchronization phases
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncPhase {
    Initializing,
    DiscoveringCalendars,
    FetchingEvents,
    ProcessingEvents,
    UpdatingLocal,
    Complete,
    Error(String),
}

impl CalendarSyncEngine {
    /// Create a new sync engine
    pub fn new(calendar_manager: Arc<CalendarManager>) -> Self {
        Self {
            calendar_manager,
            sync_configs: RwLock::new(HashMap::new()),
            sync_status: RwLock::new(HashMap::new()),
            is_running: RwLock::new(false),
        }
    }

    /// Add a calendar sync configuration
    pub async fn add_sync_config(&self, config: CalendarSyncConfig) {
        let mut configs = self.sync_configs.write().await;
        let mut status_map = self.sync_status.write().await;

        // Add configuration
        configs.insert(config.calendar_id.clone(), config.clone());

        // Initialize status
        let status = CalendarSyncStatus {
            calendar_id: config.calendar_id.clone(),
            status: if config.enabled {
                SyncState::Idle
            } else {
                SyncState::Disabled
            },
            last_sync: config.last_sync,
            last_error: None,
            events_synced: 0,
            progress: None,
        };
        status_map.insert(config.calendar_id, status);

        info!("Added sync config for calendar: {}", config.account_id);
    }

    /// Remove a calendar sync configuration
    pub async fn remove_sync_config(&self, calendar_id: &str) {
        let mut configs = self.sync_configs.write().await;
        let mut status_map = self.sync_status.write().await;

        configs.remove(calendar_id);
        status_map.remove(calendar_id);

        info!("Removed sync config for calendar: {}", calendar_id);
    }

    /// Update sync configuration
    pub async fn update_sync_config(&self, config: CalendarSyncConfig) {
        let mut configs = self.sync_configs.write().await;
        let mut status_map = self.sync_status.write().await;

        configs.insert(config.calendar_id.clone(), config.clone());

        // Update status enabled state
        if let Some(status) = status_map.get_mut(&config.calendar_id) {
            status.status = if config.enabled {
                SyncState::Idle
            } else {
                SyncState::Disabled
            };
        }

        info!("Updated sync config for calendar: {}", config.calendar_id);
    }

    /// Start the synchronization engine
    pub async fn start(&self) -> CalendarResult<()> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Ok(());
        }

        *is_running = true;
        drop(is_running);

        info!("Starting calendar sync engine");

        // Note: The sync engine needs to be managed externally due to lifetime constraints
        // For now, we'll just mark it as running without spawning the background task
        // The actual sync loop should be called from the main application loop

        Ok(())
    }

    /// Stop the synchronization engine
    pub async fn stop(&self) {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        info!("Stopped calendar sync engine");
    }

    /// Synchronize a specific calendar
    pub async fn sync_calendar(&self, config: CalendarSyncConfig) -> CalendarResult<()> {
        let calendar_id = config.calendar_id.clone();

        info!("Starting sync for calendar: {}", calendar_id);

        // Update status to syncing
        self.update_sync_status(&calendar_id, SyncState::Syncing, None)
            .await;

        // Create progress tracker
        let progress = CalendarSyncProgress {
            phase: SyncPhase::Initializing,
            events_processed: 0,
            total_events: 0,
            started_at: Utc::now(),
            estimated_completion: None,
            current_operation: "Initializing sync".to_string(),
        };
        self.update_sync_progress(&calendar_id, progress).await;

        let result = match &config.caldav_config {
            Some(caldav_config) => self.sync_caldav_calendar(&config, caldav_config).await,
            None => {
                // Handle other calendar types (Google, Outlook) in the future
                warn!("Sync not implemented for calendar type: {}", calendar_id);
                Ok(0)
            }
        };

        match result {
            Ok(events_synced) => {
                info!(
                    "Successfully synced {} events for calendar: {}",
                    events_synced, calendar_id
                );

                // Update successful sync status
                self.update_sync_status(&calendar_id, SyncState::Completed, None)
                    .await;

                // Update last sync time in config
                self.update_last_sync_time(&calendar_id, Utc::now()).await;

                // Final progress update
                let final_progress = CalendarSyncProgress {
                    phase: SyncPhase::Complete,
                    events_processed: events_synced,
                    total_events: events_synced,
                    started_at: Utc::now(), // This would be stored from initial progress
                    estimated_completion: Some(Utc::now()),
                    current_operation: "Sync completed".to_string(),
                };
                self.update_sync_progress(&calendar_id, final_progress)
                    .await;
            }
            Err(e) => {
                error!("Failed to sync calendar {}: {}", calendar_id, e);

                // Update error status
                self.update_sync_status(&calendar_id, SyncState::Error, Some(e.to_string()))
                    .await;

                // Error progress update
                let error_progress = CalendarSyncProgress {
                    phase: SyncPhase::Error(e.to_string()),
                    events_processed: 0,
                    total_events: 0,
                    started_at: Utc::now(),
                    estimated_completion: None,
                    current_operation: format!("Sync failed: {}", e),
                };
                self.update_sync_progress(&calendar_id, error_progress)
                    .await;
            }
        }

        Ok(())
    }

    /// Synchronize a CalDAV calendar
    async fn sync_caldav_calendar(
        &self,
        config: &CalendarSyncConfig,
        caldav_config: &CalDAVConfig,
    ) -> CalendarResult<u32> {
        // Update progress
        let progress = CalendarSyncProgress {
            phase: SyncPhase::DiscoveringCalendars,
            events_processed: 0,
            total_events: 0,
            started_at: Utc::now(),
            estimated_completion: None,
            current_operation: "Connecting to CalDAV server".to_string(),
        };
        self.update_sync_progress(&config.calendar_id, progress)
            .await;

        // Create CalDAV client
        let client = CalDAVClient::new(
            &caldav_config.server_url,
            caldav_config.username.clone(),
            caldav_config.password.clone(),
        )
        .map_err(|e| CalendarError::SyncError(e.to_string()))?;

        // Test connection
        client
            .test_connection()
            .await
            .map_err(|e| CalendarError::SyncError(format!("Connection failed: {}", e)))?;

        // Update progress
        let progress = CalendarSyncProgress {
            phase: SyncPhase::FetchingEvents,
            events_processed: 0,
            total_events: 0,
            started_at: Utc::now(),
            estimated_completion: None,
            current_operation: "Fetching events from server".to_string(),
        };
        self.update_sync_progress(&config.calendar_id, progress)
            .await;

        // Fetch events from the last month to next 6 months
        let now = Utc::now();
        let start_date = now - Duration::days(30);
        let end_date = now + Duration::days(180);

        let query = CalDAVQuery {
            start_date: Some(start_date),
            end_date: Some(end_date),
            component_filter: Some("VEVENT".to_string()),
            expand_recurrence: false,
        };

        // For now, use the base URL as calendar URL - in a real implementation,
        // you would discover the actual calendar URLs first
        let calendar_url = &caldav_config.server_url;

        let caldav_events = client
            .get_events(calendar_url, &query)
            .await
            .map_err(|e| CalendarError::SyncError(format!("Failed to fetch events: {}", e)))?;

        let total_events = caldav_events.len() as u32;

        // Update progress with total count
        let progress = CalendarSyncProgress {
            phase: SyncPhase::ProcessingEvents,
            events_processed: 0,
            total_events,
            started_at: Utc::now(),
            estimated_completion: Some(Utc::now() + Duration::minutes(total_events as i64 / 10)), // Rough estimate
            current_operation: format!("Processing {} events", total_events),
        };
        self.update_sync_progress(&config.calendar_id, progress)
            .await;

        let mut events_processed = 0;

        for (i, caldav_event) in caldav_events.iter().enumerate() {
            // Parse the iCalendar data (simplified implementation)
            // In a real implementation, you would use a proper iCalendar parser
            if let Ok(event) = self.parse_caldav_event(caldav_event, &config.calendar_id) {
                // Store the event
                if let Err(e) = self.calendar_manager.create_event(event).await {
                    warn!("Failed to store event: {}", e);
                } else {
                    events_processed += 1;
                }
            }

            // Update progress every 10 events
            if i % 10 == 0 {
                let progress = CalendarSyncProgress {
                    phase: SyncPhase::ProcessingEvents,
                    events_processed: i as u32,
                    total_events,
                    started_at: Utc::now(),
                    estimated_completion: Some(
                        Utc::now() + Duration::minutes((total_events - i as u32) as i64 / 10),
                    ),
                    current_operation: format!("Processed {} of {} events", i, total_events),
                };
                self.update_sync_progress(&config.calendar_id, progress)
                    .await;
            }
        }

        // Final update progress
        let progress = CalendarSyncProgress {
            phase: SyncPhase::UpdatingLocal,
            events_processed: total_events,
            total_events,
            started_at: Utc::now(),
            estimated_completion: Some(Utc::now()),
            current_operation: "Finalizing sync".to_string(),
        };
        self.update_sync_progress(&config.calendar_id, progress)
            .await;

        Ok(events_processed)
    }

    /// Parse CalDAV event to internal Event structure
    fn parse_caldav_event(
        &self,
        caldav_event: &crate::calendar::caldav::CalDAVEvent,
        calendar_id: &str,
    ) -> CalendarResult<Event> {
        // This is a very simplified iCalendar parser
        // In a production implementation, you would use a proper iCalendar library

        let ical_data = &caldav_event.icalendar_data;

        // Extract basic information using simple string parsing
        let mut title = "Untitled Event".to_string();
        let mut description = None;
        let mut location = None;
        let mut start_time = Utc::now();
        let mut end_time = Utc::now() + Duration::hours(1);

        for line in ical_data.lines() {
            if line.starts_with("SUMMARY:") {
                title = line.strip_prefix("SUMMARY:").unwrap_or(&title).to_string();
            } else if line.starts_with("DESCRIPTION:") {
                description = Some(line.strip_prefix("DESCRIPTION:").unwrap_or("").to_string());
            } else if line.starts_with("LOCATION:") {
                location = Some(line.strip_prefix("LOCATION:").unwrap_or("").to_string());
            } else if line.starts_with("DTSTART:") {
                if let Some(datetime_str) = line.strip_prefix("DTSTART:") {
                    if let Ok(dt) = DateTime::parse_from_str(datetime_str, "%Y%m%dT%H%M%SZ") {
                        start_time = dt.with_timezone(&Utc);
                    }
                }
            } else if line.starts_with("DTEND:") {
                if let Some(datetime_str) = line.strip_prefix("DTEND:") {
                    if let Ok(dt) = DateTime::parse_from_str(datetime_str, "%Y%m%dT%H%M%SZ") {
                        end_time = dt.with_timezone(&Utc);
                    }
                }
            }
        }

        let mut event = Event::new(calendar_id.to_string(), title, start_time, end_time);

        event.description = description;
        event.location = location;
        event.uid = caldav_event.url.clone(); // Use URL as UID for now
        event.etag = Some(caldav_event.etag.clone());

        Ok(event)
    }

    /// Update sync status for a calendar
    async fn update_sync_status(
        &self,
        calendar_id: &str,
        status: SyncState,
        error: Option<String>,
    ) {
        let mut status_map = self.sync_status.write().await;

        if let Some(sync_status) = status_map.get_mut(calendar_id) {
            sync_status.status = status;
            sync_status.last_error = error;

            if matches!(status, SyncState::Completed) {
                sync_status.last_sync = Some(Utc::now());
            }
        }
    }

    /// Update sync progress for a calendar
    async fn update_sync_progress(&self, calendar_id: &str, progress: CalendarSyncProgress) {
        let mut status_map = self.sync_status.write().await;

        if let Some(sync_status) = status_map.get_mut(calendar_id) {
            sync_status.progress = Some(progress);
        }
    }

    /// Update last sync time in configuration
    async fn update_last_sync_time(&self, calendar_id: &str, last_sync: DateTime<Utc>) {
        let mut configs = self.sync_configs.write().await;

        if let Some(config) = configs.get_mut(calendar_id) {
            config.last_sync = Some(last_sync);
        }
    }

    /// Get sync status for all calendars
    pub async fn get_sync_status(&self) -> HashMap<String, CalendarSyncStatus> {
        self.sync_status.read().await.clone()
    }

    /// Get sync status for a specific calendar
    pub async fn get_calendar_sync_status(&self, calendar_id: &str) -> Option<CalendarSyncStatus> {
        self.sync_status.read().await.get(calendar_id).cloned()
    }

    /// Force sync a specific calendar immediately
    pub async fn force_sync_calendar(&self, calendar_id: &str) -> CalendarResult<()> {
        let configs = self.sync_configs.read().await;

        if let Some(config) = configs.get(calendar_id) {
            let config_clone = config.clone();
            let calendar_id_owned = calendar_id.to_string();
            drop(configs);

            // Spawn sync task
            let engine_clone = self.clone();
            tokio::spawn(async move {
                if let Err(e) = engine_clone.sync_calendar(config_clone).await {
                    error!("Failed to force sync calendar {}: {}", calendar_id_owned, e);
                }
            });

            Ok(())
        } else {
            Err(CalendarError::InvalidData(format!(
                "Calendar {} not found",
                calendar_id
            )))
        }
    }

    /// Force sync all enabled calendars
    pub async fn force_sync_all(&self) -> CalendarResult<()> {
        let configs = self.sync_configs.read().await.clone();

        for (calendar_id, config) in configs {
            if config.enabled {
                // Spawn sync task for each calendar
                let engine_clone = self.clone();
                let config_clone = config.clone();

                tokio::spawn(async move {
                    if let Err(e) = engine_clone.sync_calendar(config_clone).await {
                        error!("Failed to force sync calendar {}: {}", calendar_id, e);
                    }
                });
            }
        }

        Ok(())
    }

    /// Get sync configurations
    pub async fn get_sync_configs(&self) -> HashMap<String, CalendarSyncConfig> {
        self.sync_configs.read().await.clone()
    }

    /// Check if sync engine is running
    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }
}

impl Clone for CalendarSyncEngine {
    fn clone(&self) -> Self {
        Self {
            calendar_manager: Arc::clone(&self.calendar_manager),
            sync_configs: RwLock::new(HashMap::new()),
            sync_status: RwLock::new(HashMap::new()),
            is_running: RwLock::new(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::{database::CalendarDatabase, manager::CalendarManager};
    use crate::oauth2::token::{TokenManager, TokenStats};

    #[tokio::test]
    async fn test_sync_engine_creation() {
        let db = Arc::new(CalendarDatabase::new_in_memory().await.unwrap());
        let token_manager = Arc::new(TokenManager::new());
        let calendar_manager = Arc::new(CalendarManager::new(db, token_manager).await.unwrap());

        let sync_engine = CalendarSyncEngine::new(calendar_manager);
        assert!(!sync_engine.is_running().await);
    }

    #[tokio::test]
    async fn test_sync_config_management() {
        let db = Arc::new(CalendarDatabase::new_in_memory().await.unwrap());
        let token_manager = Arc::new(TokenManager::new());
        let calendar_manager = Arc::new(CalendarManager::new(db, token_manager).await.unwrap());

        let sync_engine = CalendarSyncEngine::new(calendar_manager);

        let config = CalendarSyncConfig {
            calendar_id: "test-calendar".to_string(),
            account_id: "test-account".to_string(),
            sync_interval_minutes: 15,
            enabled: true,
            last_sync: None,
            sync_token: None,
            caldav_config: Some(CalDAVConfig::new(
                "Test Calendar".to_string(),
                "https://calendar.example.com/dav/".to_string(),
                "testuser".to_string(),
                "testpass".to_string(),
            )),
        };

        sync_engine.add_sync_config(config).await;

        let configs = sync_engine.get_sync_configs().await;
        assert_eq!(configs.len(), 1);
        assert!(configs.contains_key("test-calendar"));

        let status = sync_engine.get_sync_status().await;
        assert_eq!(status.len(), 1);
        assert_eq!(status.get("test-calendar").unwrap().status, SyncState::Idle);
    }
}
