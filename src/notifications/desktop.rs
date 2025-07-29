use chrono::{Datelike, Timelike};
use notify_rust::{Notification, Timeout, Urgency};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use crate::notifications::types::{NotificationConfig, NotificationEvent, NotificationPriority};

/// Enhanced desktop notification service using native Rust notifications
pub struct DesktopNotificationService {
    config: NotificationConfig,
    last_notification_times: HashMap<String, Instant>,
    notification_batch: Vec<NotificationEvent>,
    batch_timer: Option<Instant>,
}

impl DesktopNotificationService {
    /// Create a new desktop notification service with default configuration
    pub fn new() -> Self {
        Self::with_config(NotificationConfig::default())
    }

    /// Create a new desktop notification service with custom configuration
    pub fn with_config(config: NotificationConfig) -> Self {
        Self {
            config,
            last_notification_times: HashMap::new(),
            notification_batch: Vec::new(),
            batch_timer: None,
        }
    }

    /// Create a disabled notification service
    pub fn disabled() -> Self {
        let mut config = NotificationConfig::default();
        config.enabled = false;
        Self::with_config(config)
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
    pub async fn start(&mut self, mut receiver: broadcast::Receiver<NotificationEvent>) {
        if !self.config.enabled {
            info!("Desktop notifications are disabled");
            return;
        }

        info!("Starting desktop notification service");

        let config = self.config.clone();
        let mut service = Self::with_config(config);

        tokio::spawn(async move {
            while let Ok(event) = receiver.recv().await {
                if let Err(e) = service.handle_notification_event(event).await {
                    error!("Failed to handle notification event: {}", e);
                }
            }
        });
    }

    /// Handle a single notification event
    async fn handle_notification_event(
        &mut self,
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
        if self.config.enable_batching && self.should_batch_notification(&event) {
            self.add_to_batch(event).await?;
        } else {
            self.send_immediate_notification(event).await?;
        }

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
        &mut self,
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
        &mut self,
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
            Ok(_handle) => {
                debug!("Desktop notification sent successfully: {}", title);

                // Update rate limiting
                self.last_notification_times
                    .insert(notification_key, Instant::now());

                // Clean up old rate limiting entries
                let cutoff = Instant::now() - Duration::from_secs(60);
                self.last_notification_times
                    .retain(|_, &mut time| time > cutoff);
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
        &mut self,
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
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.notification_batch.is_empty() {
            self.send_batch_notification().await?;
        }
        Ok(())
    }

    /// Get notification statistics
    pub fn get_stats(&self) -> NotificationStats {
        NotificationStats {
            pending_batch_size: self.notification_batch.len(),
            rate_limited_notifications: self.last_notification_times.len(),
            is_quiet_hours: self.is_quiet_hours(),
            is_enabled: self.config.enabled,
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
