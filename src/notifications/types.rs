// use chrono::{DateTime, Utc};
use crate::calendar::Event;
use crate::email::StoredMessage;
use serde::{Deserialize, Serialize};

/// Priority levels for notifications
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum NotificationPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Unified notification event that encompasses both email and calendar events
#[derive(Debug, Clone)]
pub enum NotificationEvent {
    /// Email-related notifications
    Email {
        event_type: EmailEventType,
        account_id: String,
        folder_name: Option<String>,
        message: Option<StoredMessage>,
        message_id: Option<String>,
        priority: NotificationPriority,
    },
    /// Calendar-related notifications
    Calendar {
        event_type: CalendarEventType,
        calendar_id: String,
        event: Option<Event>,
        event_id: Option<String>,
        priority: NotificationPriority,
    },
    /// System-level notifications
    System {
        event_type: SystemEventType,
        message: String,
        priority: NotificationPriority,
    },
}

/// Types of email notification events
#[derive(Debug, Clone)]
pub enum EmailEventType {
    NewMessage,
    MessageUpdated,
    MessageDeleted,
    SyncStarted,
    SyncCompleted { new_count: u32, updated_count: u32 },
    SyncFailed { error: String },
    MessageSent,
    MessageDelivered,
    MessageFailed { error: String },
}

/// Types of calendar notification events
#[derive(Debug, Clone)]
pub enum CalendarEventType {
    EventCreated,
    EventUpdated,
    EventDeleted,
    EventReminder { minutes_until: i64 },
    RSVPSent { response: String },
    SyncStarted,
    SyncCompleted { new_count: u32, updated_count: u32 },
    SyncFailed { error: String },
}

/// Types of system notification events
#[derive(Debug, Clone)]
pub enum SystemEventType {
    AppStarted,
    AppShutdown,
    ConfigurationChanged,
    ConnectionError,
    AuthenticationRequired,
    UpdateAvailable,
}

/// Configuration for the notification system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    /// Whether notifications are globally enabled
    pub enabled: bool,

    /// Show message content in notifications (privacy setting)
    pub show_content_preview: bool,

    /// Enable email notifications
    pub email_notifications: bool,

    /// Enable calendar notifications
    pub calendar_notifications: bool,

    /// Enable system notifications
    pub system_notifications: bool,

    /// Minimum priority level for notifications
    pub min_priority: NotificationPriority,

    /// Enable notification batching for high-volume periods
    pub enable_batching: bool,

    /// Batching window in seconds
    pub batch_window_seconds: u64,

    /// Maximum notifications per batch
    pub max_batch_size: usize,

    /// Quiet hours configuration
    pub quiet_hours: Option<QuietHours>,

    /// Custom notification sounds
    pub notification_sounds: NotificationSounds,

    /// VIP senders (always notify regardless of other settings)
    pub vip_senders: Vec<String>,

    /// Keywords that trigger high-priority notifications
    pub priority_keywords: Vec<String>,
}

/// Quiet hours configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuietHours {
    pub enabled: bool,
    pub start_hour: u8, // 0-23
    pub end_hour: u8,   // 0-23
    pub weekends_only: bool,
}

/// Notification sound configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSounds {
    pub enabled: bool,
    pub email_sound: Option<String>,
    pub calendar_sound: Option<String>,
    pub system_sound: Option<String>,
    pub high_priority_sound: Option<String>,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            show_content_preview: true,
            email_notifications: true,
            calendar_notifications: true,
            system_notifications: true,
            min_priority: NotificationPriority::Normal,
            enable_batching: true,
            batch_window_seconds: 30,
            max_batch_size: 5,
            quiet_hours: None,
            notification_sounds: NotificationSounds::default(),
            vip_senders: Vec::new(),
            priority_keywords: vec![
                "urgent".to_string(),
                "asap".to_string(),
                "immediate".to_string(),
                "important".to_string(),
            ],
        }
    }
}

impl Default for NotificationSounds {
    fn default() -> Self {
        Self {
            enabled: true,
            email_sound: None,
            calendar_sound: None,
            system_sound: None,
            high_priority_sound: None,
        }
    }
}

impl NotificationEvent {
    /// Get the priority of this notification event
    pub fn priority(&self) -> NotificationPriority {
        match self {
            NotificationEvent::Email { priority, .. } => *priority,
            NotificationEvent::Calendar { priority, .. } => *priority,
            NotificationEvent::System { priority, .. } => *priority,
        }
    }

    /// Get a short title for this notification
    pub fn title(&self) -> String {
        match self {
            NotificationEvent::Email {
                event_type,
                account_id,
                ..
            } => match event_type {
                EmailEventType::NewMessage => format!("New Email - {}", account_id),
                EmailEventType::MessageSent => "Email Sent".to_string(),
                EmailEventType::SyncCompleted { new_count, .. } => {
                    if *new_count > 0 {
                        format!("Email Sync - {} new messages", new_count)
                    } else {
                        "Email Sync Complete".to_string()
                    }
                }
                EmailEventType::SyncFailed { .. } => format!("Email Sync Failed - {}", account_id),
                _ => format!("Email Update - {}", account_id),
            },
            NotificationEvent::Calendar {
                event_type,
                calendar_id,
                ..
            } => match event_type {
                CalendarEventType::EventCreated => format!("Event Created - {}", calendar_id),
                CalendarEventType::EventReminder { minutes_until } => {
                    if *minutes_until <= 5 {
                        "Calendar Reminder - Starting Soon!".to_string()
                    } else {
                        "Calendar Reminder".to_string()
                    }
                }
                CalendarEventType::SyncCompleted { new_count, .. } => {
                    if *new_count > 0 {
                        format!("Calendar Sync - {} new events", new_count)
                    } else {
                        "Calendar Sync Complete".to_string()
                    }
                }
                CalendarEventType::SyncFailed { .. } => {
                    format!("Calendar Sync Failed - {}", calendar_id)
                }
                _ => format!("Calendar Update - {}", calendar_id),
            },
            NotificationEvent::System { event_type, .. } => match event_type {
                SystemEventType::AppStarted => "Comunicado Started".to_string(),
                SystemEventType::ConnectionError => "Connection Error".to_string(),
                SystemEventType::AuthenticationRequired => "Authentication Required".to_string(),
                SystemEventType::UpdateAvailable => "Update Available".to_string(),
                _ => "System Notification".to_string(),
            },
        }
    }

    /// Get the notification body/content
    pub fn body(&self, config: &NotificationConfig) -> String {
        match self {
            NotificationEvent::Email {
                event_type,
                message,
                folder_name,
                ..
            } => match event_type {
                EmailEventType::NewMessage => {
                    if let Some(msg) = message {
                        if config.show_content_preview {
                            let from_display = if let Some(ref name) = msg.from_name {
                                format!("{} <{}>", name, msg.from_addr)
                            } else {
                                msg.from_addr.clone()
                            };
                            format!(
                                "From: {}\nSubject: {}\nFolder: {}",
                                from_display,
                                if msg.subject.is_empty() {
                                    "(No Subject)"
                                } else {
                                    &msg.subject
                                },
                                folder_name.as_deref().unwrap_or("Unknown")
                            )
                        } else {
                            format!(
                                "New message in folder: {}",
                                folder_name.as_deref().unwrap_or("Unknown")
                            )
                        }
                    } else {
                        "New email received".to_string()
                    }
                }
                EmailEventType::SyncCompleted {
                    new_count,
                    updated_count,
                } => {
                    format!(
                        "Sync completed: {} new, {} updated messages",
                        new_count, updated_count
                    )
                }
                EmailEventType::SyncFailed { error } => {
                    format!("Sync failed: {}", error)
                }
                EmailEventType::MessageSent => {
                    "Your message has been sent successfully".to_string()
                }
                EmailEventType::MessageFailed { error } => {
                    format!("Failed to send message: {}", error)
                }
                _ => "Email event occurred".to_string(),
            },
            NotificationEvent::Calendar {
                event_type, event, ..
            } => match event_type {
                CalendarEventType::EventCreated => {
                    if let Some(evt) = event {
                        if config.show_content_preview {
                            format!(
                                "Event: {}\nDate: {}",
                                evt.title,
                                evt.start_time.format("%Y-%m-%d %H:%M")
                            )
                        } else {
                            "New calendar event created".to_string()
                        }
                    } else {
                        "Calendar event created".to_string()
                    }
                }
                CalendarEventType::EventReminder { minutes_until } => {
                    if let Some(evt) = event {
                        if config.show_content_preview {
                            format!(
                                "Upcoming: {}\nStarts in {} minute{}",
                                evt.title,
                                minutes_until,
                                if *minutes_until == 1 { "" } else { "s" }
                            )
                        } else {
                            format!("Event starting in {} minutes", minutes_until)
                        }
                    } else {
                        format!("Event reminder - {} minutes until start", minutes_until)
                    }
                }
                CalendarEventType::SyncCompleted {
                    new_count,
                    updated_count,
                } => {
                    format!(
                        "Sync complete: {} new, {} updated events",
                        new_count, updated_count
                    )
                }
                CalendarEventType::SyncFailed { error } => {
                    format!("Calendar sync failed: {}", error)
                }
                CalendarEventType::RSVPSent { response } => {
                    format!("RSVP response sent: {}", response)
                }
                _ => "Calendar event occurred".to_string(),
            },
            NotificationEvent::System { message, .. } => message.clone(),
        }
    }

    /// Get the appropriate icon for this notification
    pub fn icon(&self) -> &'static str {
        match self {
            NotificationEvent::Email { event_type, .. } => match event_type {
                EmailEventType::NewMessage => "mail-message-new",
                EmailEventType::MessageSent => "mail-send",
                EmailEventType::SyncCompleted { .. } => "mail-folder-inbox",
                EmailEventType::SyncFailed { .. } => "dialog-error",
                EmailEventType::MessageFailed { .. } => "dialog-error",
                _ => "mail-message",
            },
            NotificationEvent::Calendar { event_type, .. } => match event_type {
                CalendarEventType::EventReminder { .. } => "appointment-soon",
                CalendarEventType::SyncFailed { .. } => "dialog-error",
                _ => "calendar",
            },
            NotificationEvent::System { event_type, .. } => match event_type {
                SystemEventType::ConnectionError => "network-error",
                SystemEventType::AuthenticationRequired => "dialog-password",
                SystemEventType::UpdateAvailable => "software-update-available",
                _ => "dialog-information",
            },
        }
    }

    /// Check if this notification should be shown during quiet hours
    pub fn should_show_during_quiet_hours(&self) -> bool {
        match self.priority() {
            NotificationPriority::Critical => true,
            NotificationPriority::High => true,
            _ => false,
        }
    }

    /// Check if this is a high-priority VIP notification
    pub fn is_vip_notification(&self, config: &NotificationConfig) -> bool {
        if let NotificationEvent::Email { message, .. } = self {
            if let Some(msg) = message {
                return config.vip_senders.contains(&msg.from_addr)
                    || config.vip_senders.iter().any(|vip| {
                        msg.from_name
                            .as_ref()
                            .map_or(false, |name| name.contains(vip))
                    });
            }
        }
        false
    }

    /// Check if this notification contains priority keywords
    pub fn contains_priority_keywords(&self, config: &NotificationConfig) -> bool {
        if let NotificationEvent::Email { message, .. } = self {
            if let Some(msg) = message {
                let content = format!("{} {}", msg.subject, msg.body_text.as_deref().unwrap_or(""));
                let content_lower = content.to_lowercase();

                return config
                    .priority_keywords
                    .iter()
                    .any(|keyword| content_lower.contains(&keyword.to_lowercase()));
            }
        }
        false
    }
}
