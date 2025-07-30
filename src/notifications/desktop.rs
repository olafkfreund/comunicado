use chrono::{DateTime, Datelike, Timelike, Utc};
use notify_rust::{Notification, Timeout, Urgency, NotificationHandle};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::notifications::types::{NotificationConfig, NotificationEvent, NotificationPriority};
use crate::calendar::Event;
use crate::email::StoredMessage;

/// Enhanced desktop notification service using native Rust notifications
pub struct DesktopNotificationService {
    config: NotificationConfig,
    last_notification_times: HashMap<String, Instant>,
    notification_batch: Vec<NotificationEvent>,
    batch_timer: Option<Instant>,
    active_notifications: Arc<RwLock<HashMap<Uuid, ActiveNotification>>>,
    reminder_scheduler: ReminderScheduler,
    action_sender: mpsc::UnboundedSender<NotificationAction>,
}

/// Active notification tracking
#[derive(Debug, Clone)]
struct ActiveNotification {
    id: Uuid,
    handle: Option<NotificationHandle>,
    title: String,
    notification_type: NotificationEventType,
    created_at: DateTime<Utc>,
    expires_at: Option<DateTime<Utc>>,
}

/// Notification event types for categorization
#[derive(Debug, Clone, PartialEq)]
enum NotificationEventType {
    Email,
    Calendar,
    System,
    Reminder,
}

/// Notification actions that can be triggered
#[derive(Debug, Clone)]
pub enum NotificationAction {
    OpenEmail(Uuid),
    MarkEmailRead(Uuid),
    OpenCalendar(Uuid),
    AcceptCalendarInvite(Uuid),
    DeclineCalendarInvite(Uuid),
    SnoozeReminder(Uuid, Duration),
    DismissNotification(Uuid),
}

/// Calendar event reminder scheduler
pub struct ReminderScheduler {
    scheduled_reminders: Arc<RwLock<HashMap<Uuid, Vec<tokio::task::JoinHandle<()>>>>>,
    reminder_sender: mpsc::UnboundedSender<NotificationEvent>,
}

impl ReminderScheduler {
    pub fn new(reminder_sender: mpsc::UnboundedSender<NotificationEvent>) -> Self {
        Self {
            scheduled_reminders: Arc::new(RwLock::new(HashMap::new())),
            reminder_sender,
        }
    }
    
    /// Schedule reminders for a calendar event
    pub async fn schedule_event_reminders(&self, event: &Event) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let reminder_minutes = vec![15, 5, 0]; // Default reminder times
        let mut handles = Vec::new();
        
        for minutes in reminder_minutes {
            let reminder_time = event.start_time - chrono::Duration::minutes(minutes);
            let now = Utc::now();
            
            if reminder_time > now {
                let event_clone = event.clone();
                let sender = self.reminder_sender.clone();
                let delay = (reminder_time - now).to_std()
                    .map_err(|_| "Invalid reminder time")?;
                
                let handle = tokio::spawn(async move {
                    tokio::time::sleep(delay).await;
                    
                    let reminder_event = NotificationEvent::Calendar {
                        event_type: crate::notifications::types::CalendarEventType::EventReminder {
                            minutes_until: minutes as u32,
                        },
                        event: Some(event_clone),
                        priority: if minutes == 0 { 
                            NotificationPriority::High 
                        } else { 
                            NotificationPriority::Normal 
                        },
                    };
                    
                    if let Err(e) = sender.send(reminder_event) {
                        error!("Failed to send reminder notification: {}", e);
                    }
                });
                
                handles.push(handle);
            }
        }
        
        if !handles.is_empty() {
            self.scheduled_reminders.write().await.insert(event.id, handles);
        }
        
        Ok(())
    }
    
    /// Cancel reminders for an event
    pub async fn cancel_event_reminders(&self, event_id: Uuid) {
        if let Some(handles) = self.scheduled_reminders.write().await.remove(&event_id) {
            for handle in handles {
                handle.abort();
            }
            debug!("Cancelled reminders for event {}", event_id);
        }
    }
    
    /// Clean up completed reminder tasks
    pub async fn cleanup_completed_reminders(&self) {
        let mut reminders = self.scheduled_reminders.write().await;
        reminders.retain(|_, handles| {
            handles.retain(|handle| !handle.is_finished());
            !handles.is_empty()
        });
    }
}

impl DesktopNotificationService {
    /// Create a new desktop notification service with default configuration
    pub fn new() -> Self {
        Self::with_config(NotificationConfig::default())
    }

    /// Create a new desktop notification service with custom configuration
    pub fn with_config(config: NotificationConfig) -> Self {
        let (action_sender, _action_receiver) = mpsc::unbounded_channel();
        let (reminder_sender, _reminder_receiver) = mpsc::unbounded_channel();
        
        Self {
            config,
            last_notification_times: HashMap::new(),
            notification_batch: Vec::new(),
            batch_timer: None,
            active_notifications: Arc::new(RwLock::new(HashMap::new())),
            reminder_scheduler: ReminderScheduler::new(reminder_sender),
            action_sender,
        }
    }

    /// Create a disabled notification service
    pub fn disabled() -> Self {
        let mut config = NotificationConfig::default();
        config.enabled = false;
        Self::with_config(config)
    }
    
    /// Schedule calendar event reminders
    pub async fn schedule_calendar_reminders(&self, event: &Event) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.reminder_scheduler.schedule_event_reminders(event).await
    }
    
    /// Cancel calendar event reminders
    pub async fn cancel_calendar_reminders(&self, event_id: Uuid) {
        self.reminder_scheduler.cancel_event_reminders(event_id).await
    }
    
    /// Send email notification with actions
    pub async fn notify_email(&self, email: &StoredMessage, is_important: bool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let priority = if is_important {
            NotificationPriority::High
        } else {
            NotificationPriority::Normal
        };
        
        let event = NotificationEvent::Email {
            event_type: crate::notifications::types::EmailEventType::NewMessage,
            account_id: email.account_id.clone(),
            folder_name: Some(email.folder.clone()),
            message: Some(email.clone()),
            message_id: Some(email.id),
            priority,
        };
        
        self.handle_notification_event(event).await
    }
    
    /// Send calendar event notification
    pub async fn notify_calendar_event(&self, event: &Event, event_type: crate::notifications::types::CalendarEventType) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let priority = match event_type {
            crate::notifications::types::CalendarEventType::EventReminder { minutes_until } => {
                if minutes_until <= 5 {
                    NotificationPriority::High
                } else {
                    NotificationPriority::Normal
                }
            }
            crate::notifications::types::CalendarEventType::MeetingInvitation => NotificationPriority::High,
            _ => NotificationPriority::Normal,
        };
        
        let notification_event = NotificationEvent::Calendar {
            event_type,
            event: Some(event.clone()),
            priority,
        };
        
        self.handle_notification_event(notification_event).await
    }
    
    /// Dismiss specific notification
    pub async fn dismiss_notification(&self, notification_id: Uuid) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut active = self.active_notifications.write().await;
        if let Some(notification) = active.remove(&notification_id) {
            if let Some(handle) = notification.handle {
                // Note: notify-rust doesn't provide a dismiss method
                // This would be platform-specific
                debug!("Dismissed notification: {}", notification.title);
            }
        }
        Ok(())
    }
    
    /// Dismiss all active notifications
    pub async fn dismiss_all_notifications(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut active = self.active_notifications.write().await;
        let count = active.len();
        active.clear();
        debug!("Dismissed {} active notifications", count);
        Ok(())
    }
    
    /// Get count of active notifications
    pub async fn active_notification_count(&self) -> usize {
        self.active_notifications.read().await.len()
    }
    
    /// Clean up expired notifications
    pub async fn cleanup_expired_notifications(&self) {
        let now = Utc::now();
        let mut active = self.active_notifications.write().await;
        let initial_count = active.len();
        
        active.retain(|_, notification| {
            if let Some(expires_at) = notification.expires_at {
                expires_at > now
            } else {
                true // Keep notifications without expiration
            }
        });
        
        let removed_count = initial_count - active.len();
        if removed_count > 0 {
            debug!("Cleaned up {} expired notifications", removed_count);
        }
        
        // Also clean up reminder scheduler
        self.reminder_scheduler.cleanup_completed_reminders().await;
    }

    /// Update the notification configuration
    pub fn update_config(&mut self, config: NotificationConfig) {
        self.config = config;
        info!("Desktop notification configuration updated");
    }

    /// Check if desktop notifications are supported on this system
    pub fn is_supported() -> bool {
        // The notify-rust crate handles platform detection internally
        true
    }

    /// Start listening for notification events
    pub async fn start(&self, mut receiver: broadcast::Receiver<NotificationEvent>) {
        if !self.config.enabled {
            info!("Desktop notifications are disabled");
            return;
        }

        info!("Starting desktop notification service");

        // Clone necessary data for the async task
        let active_notifications = Arc::clone(&self.active_notifications);
        let config = self.config.clone();
        let action_sender = self.action_sender.clone();

        tokio::spawn(async move {
            let service = Self::with_config(config);
            
            while let Ok(event) = receiver.recv().await {
                if let Err(e) = service.handle_notification_event(event).await {
                    error!("Failed to handle notification event: {}", e);
                }
            }
        });
        
        // Start cleanup task
        self.start_cleanup_task();
    }
    
    /// Start periodic cleanup task for expired notifications
    fn start_cleanup_task(&self) {
        let active_notifications = Arc::clone(&self.active_notifications);
        let reminder_scheduler = self.reminder_scheduler.scheduled_reminders.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            
            loop {
                interval.tick().await;
                
                // Clean up expired notifications
                let now = Utc::now();
                let mut active = active_notifications.write().await;
                let initial_count = active.len();
                
                active.retain(|_, notification| {
                    if let Some(expires_at) = notification.expires_at {
                        expires_at > now
                    } else {
                        true
                    }
                });
                
                let removed_count = initial_count - active.len();
                if removed_count > 0 {
                    debug!("Cleaned up {} expired notifications", removed_count);
                }
                
                // Clean up completed reminders
                let mut reminders = reminder_scheduler.write().await;
                reminders.retain(|_, handles| {
                    handles.retain(|handle| !handle.is_finished());
                    !handles.is_empty()
                });
            }
        });
    }

    /// Handle a single notification event  
    pub async fn handle_notification_event(
        &self,
        event: NotificationEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Check if notifications are enabled
        if !self.config.enabled {
            return Ok(());
        }

        // Check notification type filters
        if !self.should_show_notification(&event) {
            debug!("Notification filtered out: {:?}", event);
            return Ok(());
        }

        // Check quiet hours
        if self.is_quiet_hours() && !event.should_show_during_quiet_hours() {
            debug!("Notification suppressed due to quiet hours: {:?}", event);
            return Ok(());
        }

        // Check priority threshold
        if event.priority() < self.config.min_priority {
            debug!("Notification below priority threshold: {:?}", event);
            return Ok(());
        }

        // Handle batching for high-volume notifications
        // Note: Batching is disabled in this async context for simplicity
        // In a production implementation, this would need proper state management
        self.send_immediate_notification(event).await?;

        Ok(())
    }

    /// Check if a notification should be shown based on type filters
    fn should_show_notification(&self, event: &NotificationEvent) -> bool {
        match event {
            NotificationEvent::Email { .. } => self.config.email_notifications,
            NotificationEvent::Calendar { .. } => self.config.calendar_notifications,
            NotificationEvent::System { .. } => self.config.system_notifications,
        }
    }

    /// Check if we're currently in quiet hours
    fn is_quiet_hours(&self) -> bool {
        if let Some(ref quiet_hours) = self.config.quiet_hours {
            if !quiet_hours.enabled {
                return false;
            }

            let now = chrono::Local::now();
            let current_hour = now.hour() as u8;
            let is_weekend = now.weekday().num_days_from_monday() >= 5;

            if quiet_hours.weekends_only && !is_weekend {
                return false;
            }

            // Handle the case where quiet hours span midnight
            if quiet_hours.start_hour <= quiet_hours.end_hour {
                current_hour >= quiet_hours.start_hour && current_hour < quiet_hours.end_hour
            } else {
                current_hour >= quiet_hours.start_hour || current_hour < quiet_hours.end_hour
            }
        } else {
            false
        }
    }

    /// Check if a notification should be batched
    fn should_batch_notification(&self, event: &NotificationEvent) -> bool {
        // Don't batch high-priority notifications
        if event.priority() >= NotificationPriority::High {
            return false;
        }

        // Don't batch VIP notifications
        if event.is_vip_notification(&self.config) {
            return false;
        }

        // Don't batch calendar reminders
        if let NotificationEvent::Calendar { event_type, .. } = event {
            if matches!(
                event_type,
                crate::notifications::types::CalendarEventType::EventReminder { .. }
            ) {
                return false;
            }
        }

        true
    }

    /// Add a notification to the batch
    async fn add_to_batch(
        &mut self,
        event: NotificationEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.notification_batch.push(event);

        // Start batch timer if this is the first notification in the batch
        if self.batch_timer.is_none() {
            self.batch_timer = Some(Instant::now());
        }

        // Check if we should send the batch
        let should_send_batch = self.notification_batch.len() >= self.config.max_batch_size
            || self.batch_timer.map_or(false, |timer| {
                timer.elapsed() >= Duration::from_secs(self.config.batch_window_seconds)
            });

        if should_send_batch {
            self.send_batch_notification().await?;
        }

        Ok(())
    }

    /// Send a batched notification
    async fn send_batch_notification(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.notification_batch.is_empty() {
            return Ok(());
        }

        let batch_size = self.notification_batch.len();
        let email_count = self
            .notification_batch
            .iter()
            .filter(|e| matches!(e, NotificationEvent::Email { .. }))
            .count();
        let calendar_count = self
            .notification_batch
            .iter()
            .filter(|e| matches!(e, NotificationEvent::Calendar { .. }))
            .count();

        let title = if batch_size == 1 {
            self.notification_batch[0].title()
        } else {
            format!("Comunicado - {} Notifications", batch_size)
        };

        let body = if batch_size == 1 {
            self.notification_batch[0].body(&self.config)
        } else {
            let mut parts = Vec::new();
            if email_count > 0 {
                parts.push(format!(
                    "{} email notification{}",
                    email_count,
                    if email_count == 1 { "" } else { "s" }
                ));
            }
            if calendar_count > 0 {
                parts.push(format!(
                    "{} calendar notification{}",
                    calendar_count,
                    if calendar_count == 1 { "" } else { "s" }
                ));
            }
            parts.join(", ")
        };

        // Use the icon from the highest priority notification
        let icon = self
            .notification_batch
            .iter()
            .max_by_key(|e| e.priority())
            .map(|e| e.icon())
            .unwrap_or("mail-message");

        self.send_native_notification(&title, &body, icon, NotificationPriority::Normal)
            .await?;

        // Clear the batch
        self.notification_batch.clear();
        self.batch_timer = None;

        debug!("Sent batch notification with {} events", batch_size);
        Ok(())
    }

    /// Send an immediate notification
    async fn send_immediate_notification(
        &self,
        event: NotificationEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let title = event.title();
        let body = event.body(&self.config);
        let icon = event.icon();
        let priority = event.priority();

        self.send_native_notification(&title, &body, icon, priority)
            .await?;
        debug!("Sent immediate notification: {}", title);

        Ok(())
    }

    /// Send a native desktop notification using notify-rust
    async fn send_native_notification(
        &self,
        title: &str,
        body: &str,
        icon: &str,
        priority: NotificationPriority,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Rate limiting: don't send the same notification too frequently
        let notification_key = format!("{}:{}", title, body);
        if let Some(last_time) = self.last_notification_times.get(&notification_key) {
            if last_time.elapsed() < Duration::from_secs(10) {
                debug!("Rate limiting notification: {}", title);
                return Ok(());
            }
        }

        let mut notification = Notification::new();
        notification
            .appname("Comunicado")
            .summary(title)
            .body(body)
            .icon(icon)
            .timeout(match priority {
                NotificationPriority::Critical => Timeout::Never,
                NotificationPriority::High => Timeout::Milliseconds(10000),
                _ => Timeout::Milliseconds(5000),
            })
            .urgency(match priority {
                NotificationPriority::Critical => Urgency::Critical,
                NotificationPriority::High => Urgency::Normal,
                _ => Urgency::Low,
            });

        // Add sound if enabled
        if self.config.notification_sounds.enabled {
            let sound = match priority {
                NotificationPriority::Critical | NotificationPriority::High => self
                    .config
                    .notification_sounds
                    .high_priority_sound
                    .as_deref(),
                _ => None,
            };

            if let Some(sound_path) = sound {
                notification.sound_name(sound_path);
            }
        }

        // Attempt to send the notification
        match notification.show() {
            Ok(handle) => {
                debug!("Desktop notification sent successfully: {}", title);

                // Track active notification
                let notification_id = Uuid::new_v4();
                let active_notification = ActiveNotification {
                    id: notification_id,
                    handle: Some(handle),
                    title: title.to_string(),
                    notification_type: NotificationEventType::System, // Default type
                    created_at: Utc::now(),
                    expires_at: match priority {
                        NotificationPriority::Critical => None, // Never expires
                        NotificationPriority::High => Some(Utc::now() + chrono::Duration::seconds(10)),
                        _ => Some(Utc::now() + chrono::Duration::seconds(5)),
                    },
                };
                
                self.active_notifications.write().await.insert(notification_id, active_notification);

                // Update rate limiting
                // Note: Need to make this method compatible with async
                // For now, we'll skip rate limiting updates or handle differently
                debug!("Added notification to active tracking: {}", notification_id);
            }
            Err(e) => {
                warn!("Failed to send desktop notification '{}': {}", title, e);
                // Fallback to logging the notification
                info!("Notification (fallback): {} - {}", title, body);
            }
        }

        Ok(())
    }

    /// Send a test notification to verify the system is working
    pub async fn send_test_notification(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.send_native_notification(
            "Comunicado Test",
            "Desktop notifications are working correctly!",
            "mail-message-new",
            NotificationPriority::Normal,
        )
        .await
    }

    /// Force send any pending batched notifications
    pub async fn flush_pending_notifications(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Batching is simplified in this implementation
        // In a full implementation, this would flush any pending batches
        Ok(())
    }

    /// Get notification statistics
    pub async fn get_stats(&self) -> NotificationStats {
        NotificationStats {
            pending_batch_size: 0, // Batching simplified
            rate_limited_notifications: self.last_notification_times.len(),
            is_quiet_hours: self.is_quiet_hours(),
            is_enabled: self.config.enabled,
            active_notifications: self.active_notifications.read().await.len(),
            scheduled_reminders: self.reminder_scheduler.scheduled_reminders.read().await.len(),
        }
    }
}

impl Default for DesktopNotificationService {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the notification service
#[derive(Debug, Clone)]
pub struct NotificationStats {
    pub pending_batch_size: usize,
    pub rate_limited_notifications: usize,
    pub is_quiet_hours: bool,
    pub is_enabled: bool,
    pub active_notifications: usize,
    pub scheduled_reminders: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notifications::types::*;

    #[test]
    fn test_service_creation() {
        let service = DesktopNotificationService::new();
        assert!(service.config.enabled);

        let disabled_service = DesktopNotificationService::disabled();
        assert!(!disabled_service.config.enabled);
    }

    #[test]
    fn test_quiet_hours_detection() {
        let mut config = NotificationConfig::default();
        config.quiet_hours = Some(QuietHours {
            enabled: true,
            start_hour: 22,
            end_hour: 8,
            weekends_only: false,
        });

        let service = DesktopNotificationService::with_config(config);
        // This test will vary based on current time, but should not panic
        let _is_quiet = service.is_quiet_hours();
    }

    #[test]
    fn test_should_batch_notification() {
        let service = DesktopNotificationService::new();

        let low_priority_event = NotificationEvent::Email {
            event_type: EmailEventType::MessageUpdated,
            account_id: "test".to_string(),
            folder_name: Some("INBOX".to_string()),
            message: None,
            message_id: None,
            priority: NotificationPriority::Low,
        };

        let high_priority_event = NotificationEvent::Email {
            event_type: EmailEventType::NewMessage,
            account_id: "test".to_string(),
            folder_name: Some("INBOX".to_string()),
            message: None,
            message_id: None,
            priority: NotificationPriority::High,
        };

        assert!(service.should_batch_notification(&low_priority_event));
        assert!(!service.should_batch_notification(&high_priority_event));
    }
}
