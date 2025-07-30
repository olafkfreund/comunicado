use tokio::sync::{broadcast, mpsc};
use tracing::{debug, error, info, warn};

use crate::calendar::CalendarNotification;
use crate::email::EmailNotification;
use crate::notifications::{
    types::{
        CalendarEventType, EmailEventType, NotificationConfig, NotificationEvent,
        NotificationPriority, SystemEventType,
    },
    DesktopNotificationService,
};

/// Unified notification manager that coordinates email, calendar, and system notifications
pub struct UnifiedNotificationManager {
    /// Broadcast sender for unified notifications
    unified_sender: broadcast::Sender<NotificationEvent>,

    /// Internal sender for publishing notifications
    internal_sender: mpsc::UnboundedSender<NotificationEvent>,

    /// Desktop notification service
    desktop_service: Option<DesktopNotificationService>,

    /// Configuration
    config: NotificationConfig,
}

impl UnifiedNotificationManager {
    /// Create a new unified notification manager
    pub fn new() -> Self {
        let (unified_sender, _) = broadcast::channel(1000);
        let (internal_sender, internal_receiver) = mpsc::unbounded_channel();

        let manager = Self {
            unified_sender: unified_sender.clone(),
            internal_sender,
            desktop_service: None,
            config: NotificationConfig::default(),
        };

        // Start the internal notification processing loop
        manager.start_internal_processor(internal_receiver, unified_sender);

        manager
    }

    /// Initialize with desktop notification support
    pub fn with_desktop_notifications(mut self, config: NotificationConfig) -> Self {
        self.config = config.clone();

        if DesktopNotificationService::is_supported() {
            let desktop_service = DesktopNotificationService::with_config(config);
            let receiver = self.unified_sender.subscribe();

            // Start the desktop notification service
            tokio::spawn(async move {
                desktop_service.start(receiver).await;
            });

            info!("Desktop notifications initialized and enabled");
        } else {
            warn!("Desktop notifications are not supported on this system");
        }

        self
    }

    /// Subscribe to unified notifications
    pub fn subscribe(&self) -> broadcast::Receiver<NotificationEvent> {
        self.unified_sender.subscribe()
    }

    /// Get the internal sender for publishing notifications
    pub fn get_sender(&self) -> mpsc::UnboundedSender<NotificationEvent> {
        self.internal_sender.clone()
    }

    /// Update the notification configuration
    pub fn update_config(&mut self, config: NotificationConfig) {
        self.config = config.clone();

        if let Some(ref mut desktop_service) = self.desktop_service {
            desktop_service.update_config(config);
        }

        info!("Notification configuration updated");
    }

    /// Start the internal notification processing loop
    fn start_internal_processor(
        &self,
        mut internal_receiver: mpsc::UnboundedReceiver<NotificationEvent>,
        unified_sender: broadcast::Sender<NotificationEvent>,
    ) {
        tokio::spawn(async move {
            while let Some(event) = internal_receiver.recv().await {
                // Broadcast the unified notification
                if let Err(e) = unified_sender.send(event.clone()) {
                    warn!("Failed to broadcast unified notification: {}", e);
                }

                debug!("Processed unified notification: {:?}", event);
            }
        });
    }

    /// Connect to email notifications and convert them to unified events
    pub fn connect_email_notifications(
        &self,
        email_receiver: broadcast::Receiver<EmailNotification>,
    ) {
        let sender = self.internal_sender.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut receiver = email_receiver;

            while let Ok(email_notification) = receiver.recv().await {
                let unified_event = Self::convert_email_notification(email_notification, &config);

                if let Err(e) = sender.send(unified_event) {
                    error!("Failed to send unified email notification: {}", e);
                }
            }
        });

        info!("Connected to email notification stream");
    }

    /// Connect to calendar notifications and convert them to unified events
    pub fn connect_calendar_notifications(
        &self,
        calendar_receiver: broadcast::Receiver<CalendarNotification>,
    ) {
        let sender = self.internal_sender.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut receiver = calendar_receiver;

            while let Ok(calendar_notification) = receiver.recv().await {
                let unified_event =
                    Self::convert_calendar_notification(calendar_notification, &config);

                if let Err(e) = sender.send(unified_event) {
                    error!("Failed to send unified calendar notification: {}", e);
                }
            }
        });

        info!("Connected to calendar notification stream");
    }

    /// Convert an email notification to a unified notification event
    fn convert_email_notification(
        notification: EmailNotification,
        config: &NotificationConfig,
    ) -> NotificationEvent {
        match notification {
            EmailNotification::NewMessage {
                account_id,
                folder_name,
                message,
            } => {
                let priority = Self::determine_email_priority(&message, config);

                NotificationEvent::Email {
                    event_type: EmailEventType::NewMessage,
                    account_id,
                    folder_name: Some(folder_name),
                    message: Some(message),
                    message_id: None,
                    priority,
                }
            }
            EmailNotification::MessageUpdated {
                account_id,
                folder_name,
                message_id,
                message,
            } => NotificationEvent::Email {
                event_type: EmailEventType::MessageUpdated,
                account_id,
                folder_name: Some(folder_name),
                message: Some(message),
                message_id: Some(message_id.to_string()),
                priority: NotificationPriority::Low,
            },
            EmailNotification::MessageDeleted {
                account_id,
                folder_name,
                message_id,
            } => NotificationEvent::Email {
                event_type: EmailEventType::MessageDeleted,
                account_id,
                folder_name: Some(folder_name),
                message: None,
                message_id: Some(message_id.to_string()),
                priority: NotificationPriority::Low,
            },
            EmailNotification::SyncStarted {
                account_id,
                folder_name,
            } => NotificationEvent::Email {
                event_type: EmailEventType::SyncStarted,
                account_id,
                folder_name: Some(folder_name),
                message: None,
                message_id: None,
                priority: NotificationPriority::Low,
            },
            EmailNotification::SyncCompleted {
                account_id,
                folder_name,
                new_count,
                updated_count,
            } => {
                let priority = if new_count > 0 {
                    NotificationPriority::Normal
                } else {
                    NotificationPriority::Low
                };

                NotificationEvent::Email {
                    event_type: EmailEventType::SyncCompleted {
                        new_count,
                        updated_count,
                    },
                    account_id,
                    folder_name: Some(folder_name),
                    message: None,
                    message_id: None,
                    priority,
                }
            }
            EmailNotification::SyncFailed {
                account_id,
                folder_name,
                error,
            } => NotificationEvent::Email {
                event_type: EmailEventType::SyncFailed { error },
                account_id,
                folder_name: Some(folder_name),
                message: None,
                message_id: None,
                priority: NotificationPriority::High,
            },
        }
    }

    /// Convert a calendar notification to a unified notification event
    fn convert_calendar_notification(
        notification: CalendarNotification,
        _config: &NotificationConfig,
    ) -> NotificationEvent {
        match notification {
            CalendarNotification::EventCreated { calendar_id, event } => {
                NotificationEvent::Calendar {
                    event_type: CalendarEventType::EventCreated,
                    calendar_id,
                    event: Some(event),
                    event_id: None,
                    priority: NotificationPriority::Normal,
                }
            }
            CalendarNotification::EventUpdated { calendar_id, event } => {
                NotificationEvent::Calendar {
                    event_type: CalendarEventType::EventUpdated,
                    calendar_id,
                    event: Some(event),
                    event_id: None,
                    priority: NotificationPriority::Low,
                }
            }
            CalendarNotification::EventDeleted {
                calendar_id,
                event_id,
            } => NotificationEvent::Calendar {
                event_type: CalendarEventType::EventDeleted,
                calendar_id,
                event: None,
                event_id: Some(event_id),
                priority: NotificationPriority::Low,
            },
            CalendarNotification::EventReminder {
                calendar_id,
                event,
                minutes_until,
            } => {
                let priority = match minutes_until {
                    0..=5 => NotificationPriority::Critical,
                    6..=15 => NotificationPriority::High,
                    _ => NotificationPriority::Normal,
                };

                NotificationEvent::Calendar {
                    event_type: CalendarEventType::EventReminder { minutes_until },
                    calendar_id,
                    event: Some(event),
                    event_id: None,
                    priority,
                }
            }
            CalendarNotification::SyncStarted { calendar_id } => NotificationEvent::Calendar {
                event_type: CalendarEventType::SyncStarted,
                calendar_id,
                event: None,
                event_id: None,
                priority: NotificationPriority::Low,
            },
            CalendarNotification::SyncCompleted {
                calendar_id,
                new_count,
                updated_count,
            } => {
                let priority = if new_count > 0 || updated_count > 0 {
                    NotificationPriority::Normal
                } else {
                    NotificationPriority::Low
                };

                NotificationEvent::Calendar {
                    event_type: CalendarEventType::SyncCompleted {
                        new_count,
                        updated_count,
                    },
                    calendar_id,
                    event: None,
                    event_id: None,
                    priority,
                }
            }
            CalendarNotification::SyncFailed { calendar_id, error } => {
                NotificationEvent::Calendar {
                    event_type: CalendarEventType::SyncFailed { error },
                    calendar_id,
                    event: None,
                    event_id: None,
                    priority: NotificationPriority::High,
                }
            }
            CalendarNotification::RSVPSent { event_id, response } => NotificationEvent::Calendar {
                event_type: CalendarEventType::RSVPSent { response },
                calendar_id: "unknown".to_string(),
                event: None,
                event_id: Some(event_id),
                priority: NotificationPriority::Normal,
            },
        }
    }

    /// Determine the priority of an email notification
    fn determine_email_priority(
        message: &crate::email::StoredMessage,
        config: &NotificationConfig,
    ) -> NotificationPriority {
        // Check for VIP senders
        if config.vip_senders.contains(&message.from_addr) {
            return NotificationPriority::High;
        }

        // Check for priority keywords in subject and body
        let content = format!(
            "{} {}",
            message.subject,
            message.body_text.as_deref().unwrap_or("")
        );
        let content_lower = content.to_lowercase();

        if config
            .priority_keywords
            .iter()
            .any(|keyword| content_lower.contains(&keyword.to_lowercase()))
        {
            return NotificationPriority::High;
        }

        // Default priority
        NotificationPriority::Normal
    }

    /// Publish a system notification
    pub async fn notify_system_event(
        &self,
        event_type: SystemEventType,
        message: String,
        priority: NotificationPriority,
    ) {
        let event = NotificationEvent::System {
            event_type,
            message,
            priority,
        };

        if let Err(e) = self.internal_sender.send(event) {
            error!("Failed to send system notification: {}", e);
        }
    }

    /// Send a test notification
    pub async fn send_test_notification(&self) {
        self.notify_system_event(
            SystemEventType::AppStarted,
            "Desktop notifications are working correctly!".to_string(),
            NotificationPriority::Normal,
        )
        .await;
    }
}

impl Default for UnifiedNotificationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::CalendarNotification;
    use crate::email::EmailNotification;
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn test_manager_creation() {
        let manager = UnifiedNotificationManager::new();
        let _receiver = manager.subscribe();
        let _sender = manager.get_sender();
    }

    #[tokio::test]
    async fn test_system_notification() {
        let manager = UnifiedNotificationManager::new();
        let mut receiver = manager.subscribe();

        manager
            .notify_system_event(
                SystemEventType::AppStarted,
                "Test message".to_string(),
                NotificationPriority::Normal,
            )
            .await;

        // Give it a moment to process
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        if let Ok(event) = receiver.try_recv() {
            match event {
                NotificationEvent::System { message, .. } => {
                    assert_eq!(message, "Test message");
                }
                _ => panic!("Expected system notification"),
            }
        }
    }

    #[test]
    fn test_email_notification_conversion() {
        let config = NotificationConfig::default();

        let email_notification = EmailNotification::NewMessage {
            account_id: "test@example.com".to_string(),
            folder_name: "INBOX".to_string(),
            message: crate::email::StoredMessage {
                id: Uuid::new_v4(),
                account_id: "test@example.com".to_string(),
                folder_name: "INBOX".to_string(),
                imap_uid: 1,
                message_id: Some("test@example.com".to_string()),
                thread_id: None,
                in_reply_to: None,
                references: vec![],
                subject: "Test Subject".to_string(),
                from_addr: "sender@example.com".to_string(),
                from_name: Some("Test Sender".to_string()),
                to_addrs: vec!["test@example.com".to_string()],
                cc_addrs: vec![],
                bcc_addrs: vec![],
                reply_to: None,
                date: Utc::now(),
                body_text: Some("Test body".to_string()),
                body_html: None,
                attachments: vec![],
                flags: vec![],
                labels: vec![],
                size: None,
                priority: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                last_synced: Utc::now(),
                sync_version: 1,
                is_draft: false,
                is_deleted: false,
            },
        };

        let unified_event =
            UnifiedNotificationManager::convert_email_notification(email_notification, &config);

        match unified_event {
            NotificationEvent::Email {
                event_type,
                account_id,
                ..
            } => {
                assert!(matches!(event_type, EmailEventType::NewMessage));
                assert_eq!(account_id, "test@example.com");
            }
            _ => panic!("Expected email notification"),
        }
    }
}
