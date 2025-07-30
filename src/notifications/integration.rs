//! Integration module for desktop notifications with email and calendar systems
//!
//! This module provides the high-level integration between the notification system
//! and the email/calendar components, handling event routing and coordination.

use crate::calendar::{Event, EventStatus};
use crate::email::StoredMessage;
use crate::notifications::desktop::{DesktopNotificationService, NotificationAction};
use crate::notifications::types::{NotificationConfig, NotificationEvent, NotificationPriority, CalendarEventType, EmailEventType};
use chrono::{DateTime, Utc, Duration as ChronoDuration};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::time::interval;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Comprehensive notification system that coordinates desktop notifications
/// with email and calendar events
pub struct NotificationIntegrationService {
    desktop_service: DesktopNotificationService,
    event_sender: broadcast::Sender<NotificationEvent>,
    #[allow(dead_code)]
    action_receiver: mpsc::UnboundedReceiver<NotificationAction>,
    email_tracking: Arc<RwLock<HashMap<Uuid, EmailTrackingInfo>>>,
    calendar_tracking: Arc<RwLock<HashMap<Uuid, CalendarTrackingInfo>>>,
    config: NotificationConfig,
}

/// Email notification tracking information
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct EmailTrackingInfo {
    message_id: Uuid,
    account_id: String,
    folder: String,
    sender: String,
    subject: String,
    received_at: DateTime<Utc>,
    is_read: bool,
    is_important: bool,
    notification_sent: bool,
}

/// Calendar event tracking information
#[derive(Debug, Clone)]
struct CalendarTrackingInfo {
    event_id: Uuid,
    title: String,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    status: EventStatus,
    reminders_scheduled: Vec<u32>, // Minutes before event
    last_reminder_sent: Option<DateTime<Utc>>,
}

impl NotificationIntegrationService {
    /// Create a new notification integration service
    pub fn new(config: NotificationConfig) -> Self {
        let (event_sender, _) = broadcast::channel(1000);
        let (_action_sender, action_receiver) = mpsc::unbounded_channel();
        let desktop_service = DesktopNotificationService::with_config(config.clone());
        
        Self {
            desktop_service,
            event_sender,
            action_receiver,
            email_tracking: Arc::new(RwLock::new(HashMap::new())),
            calendar_tracking: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }
    
    /// Start the notification integration service
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Starting notification integration service");
        
        // Start desktop notification service
        let event_receiver = self.event_sender.subscribe();
        self.desktop_service.start(event_receiver).await;
        
        // Start action handler
        self.start_action_handler().await;
        
        // Start periodic tasks
        self.start_reminder_scheduler().await;
        self.start_cleanup_tasks().await;
        
        info!("Notification integration service started successfully");
        Ok(())
    }
    
    /// Handle new email notification
    pub async fn handle_new_email(&self, message: &StoredMessage) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let is_important = self.determine_email_importance(message);
        let priority = if is_important {
            NotificationPriority::High
        } else {
            NotificationPriority::Normal
        };
        
        // Track email
        let tracking_info = EmailTrackingInfo {
            message_id: message.id,
            account_id: message.account_id.clone(),
            folder: message.folder_name.clone(),
            sender: message.from_addr.clone(),
            subject: message.subject.clone(),
            received_at: message.date,
            is_read: message.flags.iter().any(|flag| flag.as_str() == "\\Seen"),
            is_important,
            notification_sent: false,
        };
        
        self.email_tracking.write().await.insert(message.id, tracking_info);
        
        // Send notification event
        let event = NotificationEvent::Email {
            event_type: EmailEventType::NewMessage,
            account_id: message.account_id.clone(),
            folder_name: Some(message.folder_name.clone()),
            message: Some(message.clone()),
            message_id: Some(message.id.to_string()),
            priority,
        };
        
        if let Err(e) = self.event_sender.send(event) {
            warn!("Failed to send email notification event: {}", e);
        }
        
        // Mark notification as sent
        if let Some(tracking) = self.email_tracking.write().await.get_mut(&message.id) {
            tracking.notification_sent = true;
        }
        
        debug!("Handled new email notification for: {}", message.subject);
        Ok(())
    }
    
    /// Handle email read status change
    pub async fn handle_email_read(&self, message_id: Uuid) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(tracking) = self.email_tracking.write().await.get_mut(&message_id) {
            tracking.is_read = true;
            debug!("Marked email {} as read", message_id);
        }
        Ok(())
    }
    
    /// Handle calendar event creation or update
    pub async fn handle_calendar_event(&self, event: &Event) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Determine event priority based on urgency and type
        let priority = self.determine_calendar_priority(event);
        
        // Track calendar event
        let tracking_info = CalendarTrackingInfo {
            event_id: event.id.parse().unwrap_or_else(|_| Uuid::new_v4()),
            title: event.title.clone(),
            start_time: event.start_time,
            end_time: event.end_time,
            status: event.status.clone(),
            reminders_scheduled: vec![15, 5], // Default reminder times
            last_reminder_sent: None,
        };
        
        self.calendar_tracking.write().await.insert(event.id.parse().unwrap_or_else(|_| Uuid::new_v4()), tracking_info);
        
        // Schedule reminders
        self.desktop_service.schedule_calendar_reminders(event).await?;
        
        // Send immediate notification for certain event types
        match event.status {
            EventStatus::Confirmed => {
                let event_type = CalendarEventType::EventCreated;
                let notification_event = NotificationEvent::Calendar {
                    event_type,
                    calendar_id: event.calendar_id.clone(),
                    event: Some(event.clone()),
                    event_id: Some(event.id.clone()),
                    priority,
                };
                
                if let Err(e) = self.event_sender.send(notification_event) {
                    warn!("Failed to send calendar notification event: {}", e);
                }
            }
            _ => {
                debug!("Calendar event status doesn't require immediate notification: {:?}", event.status);
            }
        }
        
        debug!("Handled calendar event: {}", event.title);
        Ok(())
    }
    
    /// Handle calendar event invitation
    pub async fn handle_calendar_invitation(&self, event: &Event) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let event_type = CalendarEventType::EventCreated; // Use existing variant instead of non-existent MeetingInvitation
        let notification_event = NotificationEvent::Calendar {
            event_type,
            calendar_id: event.calendar_id.clone(),
            event: Some(event.clone()),
            event_id: Some(event.id.clone()),
            priority: NotificationPriority::High,
        };
        
        if let Err(e) = self.event_sender.send(notification_event) {
            warn!("Failed to send calendar invitation notification: {}", e);
        }
        
        debug!("Handled calendar invitation: {}", event.title);
        Ok(())
    }
    
    /// Handle system notification
    pub async fn handle_system_notification(&self, message: &str, priority: NotificationPriority) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let event = NotificationEvent::System {
            event_type: crate::notifications::types::SystemEventType::ConfigurationChanged,
            message: message.to_string(),
            priority,
        };
        
        if let Err(e) = self.event_sender.send(event) {
            warn!("Failed to send system notification: {}", e);
        }
        
        Ok(())
    }
    
    /// Send sync completion notification
    pub async fn notify_sync_complete(&self, account_name: &str, new_messages: usize, errors: usize) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let message = if errors > 0 {
            format!("Sync completed for {} with {} new messages and {} errors", account_name, new_messages, errors)
        } else {
            format!("Sync completed for {} with {} new messages", account_name, new_messages)
        };
        
        let priority = if errors > 0 {
            NotificationPriority::High
        } else if new_messages > 0 {
            NotificationPriority::Normal
        } else {
            NotificationPriority::Low
        };
        
        self.handle_system_notification(&message, priority).await
    }
    
    /// Determine email importance based on various factors
    fn determine_email_importance(&self, message: &StoredMessage) -> bool {
        // Check flags
        if message.flags.iter().any(|flag| flag.as_str() == "\\Flagged") {
            return true;
        }
        
        // Check subject for urgent keywords
        let subject_lower = message.subject.to_lowercase();
        let urgent_keywords = ["urgent", "important", "critical", "asap", "emergency"];
        if urgent_keywords.iter().any(|keyword| subject_lower.contains(keyword)) {
            return true;
        }
        
        // Check sender (could be enhanced with VIP lists)
        // For now, just basic heuristics
        if message.from_addr.contains("noreply") || message.from_addr.contains("no-reply") {
            return false;
        }
        
        // Default to not important
        false
    }
    
    /// Determine calendar event priority
    fn determine_calendar_priority(&self, event: &Event) -> NotificationPriority {
        let now = Utc::now();
        let time_until_event = event.start_time - now;
        
        // High priority for events starting soon
        if time_until_event <= ChronoDuration::hours(1) {
            return NotificationPriority::High;
        }
        
        // High priority for all-day important events
        if event.all_day && event.title.to_lowercase().contains("deadline") {
            return NotificationPriority::High;
        }
        
        // Check for urgent keywords in title
        let title_lower = event.title.to_lowercase();
        let urgent_keywords = ["urgent", "critical", "important", "deadline"];
        if urgent_keywords.iter().any(|keyword| title_lower.contains(keyword)) {
            return NotificationPriority::High;
        }
        
        NotificationPriority::Normal
    }
    
    /// Start action handler for notification actions
    async fn start_action_handler(&mut self) {
        let _email_tracking = Arc::clone(&self.email_tracking);
        let _calendar_tracking = Arc::clone(&self.calendar_tracking);
        
        tokio::spawn(async move {
            // In a real implementation, this would handle action_receiver
            // For now, this is a placeholder for action handling logic
            debug!("Action handler started");
        });
    }
    
    /// Start reminder scheduler for calendar events
    async fn start_reminder_scheduler(&self) {
        let calendar_tracking = Arc::clone(&self.calendar_tracking);
        let event_sender = self.event_sender.clone();
        
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(60)); // Check every minute
            
            loop {
                interval.tick().await;
                
                let now = Utc::now();
                let mut tracking = calendar_tracking.write().await;
                
                for (_, event_tracking) in tracking.iter_mut() {
                    // Check if we need to send reminders
                    for &minutes_before in &event_tracking.reminders_scheduled {
                        let reminder_time = event_tracking.start_time - ChronoDuration::minutes(minutes_before as i64);
                        
                        // Send reminder if it's time and we haven't sent it yet
                        if now >= reminder_time && 
                           (event_tracking.last_reminder_sent.is_none() || 
                            event_tracking.last_reminder_sent.unwrap() < reminder_time) {
                            
                            let event_type = CalendarEventType::EventReminder { minutes_until: minutes_before as i64 };
                            let priority = if minutes_before <= 5 {
                                NotificationPriority::High
                            } else {
                                NotificationPriority::Normal
                            };
                            
                            // Create a basic event for the reminder
                            let reminder_event = Event {
                                id: event_tracking.event_id.to_string(),
                                uid: event_tracking.event_id.to_string(),
                                title: event_tracking.title.clone(),
                                description: None,
                                start_time: event_tracking.start_time,
                                end_time: event_tracking.end_time,
                                all_day: false,
                                location: None,
                                attendees: Vec::new(),
                                organizer: None,
                                status: event_tracking.status.clone(),
                                calendar_id: "default".to_string(),
                                recurrence: None,
                                created_at: now,
                                updated_at: now,
                                categories: Vec::new(),
                                priority: crate::calendar::EventPriority::Normal,
                                reminders: Vec::new(),
                                sequence: 0,
                                url: None,
                                etag: None,
                            };
                            
                            let notification_event = NotificationEvent::Calendar {
                                event_type,
                                calendar_id: event_tracking.title.clone(), // Use title as calendar_id placeholder
                                event: Some(reminder_event),
                                event_id: Some(event_tracking.event_id.to_string()),
                                priority,
                            };
                            
                            if let Err(e) = event_sender.send(notification_event) {
                                error!("Failed to send reminder notification: {}", e);
                            } else {
                                event_tracking.last_reminder_sent = Some(now);
                                debug!("Sent reminder for event: {} ({} minutes before)", 
                                      event_tracking.title, minutes_before);
                            }
                        }
                    }
                }
            }
        });
    }
    
    /// Start cleanup tasks for expired tracking data
    async fn start_cleanup_tasks(&self) {
        let email_tracking = Arc::clone(&self.email_tracking);
        let calendar_tracking = Arc::clone(&self.calendar_tracking);
        
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(3600)); // Clean up every hour
            
            loop {
                interval.tick().await;
                
                let now = Utc::now();
                let cutoff = now - ChronoDuration::days(7); // Keep data for 7 days
                
                // Clean up old email tracking
                {
                    let mut emails = email_tracking.write().await;
                    let initial_count = emails.len();
                    emails.retain(|_, tracking| tracking.received_at > cutoff);
                    let removed = initial_count - emails.len();
                    if removed > 0 {
                        debug!("Cleaned up {} old email tracking entries", removed);
                    }
                }
                
                // Clean up old calendar tracking
                {
                    let mut events = calendar_tracking.write().await;
                    let initial_count = events.len();
                    events.retain(|_, tracking| tracking.end_time > cutoff);
                    let removed = initial_count - events.len();
                    if removed > 0 {
                        debug!("Cleaned up {} old calendar tracking entries", removed);
                    }
                }
            }
        });
    }
    
    /// Get notification statistics
    pub async fn get_notification_stats(&self) -> NotificationStatistics {
        let desktop_stats = self.desktop_service.get_stats().await;
        let email_count = self.email_tracking.read().await.len();
        let calendar_count = self.calendar_tracking.read().await.len();
        
        NotificationStatistics {
            desktop_stats,
            tracked_emails: email_count,
            tracked_calendar_events: calendar_count,
        }
    }
    
    /// Update notification configuration
    pub fn update_config(&mut self, config: NotificationConfig) {
        self.config = config.clone();
        self.desktop_service.update_config(config);
    }
    
    /// Test notification system
    pub async fn send_test_notification(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.desktop_service.send_test_notification().await
    }
}

/// Comprehensive notification statistics
#[derive(Debug, Clone)]
pub struct NotificationStatistics {
    pub desktop_stats: crate::notifications::desktop::NotificationStats,
    pub tracked_emails: usize,
    pub tracked_calendar_events: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notifications::types::NotificationConfig;
    use chrono::Utc;

    #[tokio::test]
    async fn test_integration_service_creation() {
        let config = NotificationConfig::default();
        let service = NotificationIntegrationService::new(config);
        
        // Basic creation test
        assert_eq!(service.email_tracking.read().await.len(), 0);
        assert_eq!(service.calendar_tracking.read().await.len(), 0);
    }

    #[test]
    fn test_email_importance_detection() {
        let config = NotificationConfig::default();
        let service = NotificationIntegrationService::new(config);
        
        let urgent_message = StoredMessage {
            id: Uuid::new_v4(),
            account_id: "test".to_string(),
            folder_name: "INBOX".to_string(),
            from_addr: "important@example.com".to_string(),
            from_name: None,
            subject: "URGENT: Action Required".to_string(),
            date: Utc::now(),
            flags: vec!["\\Seen".to_string()],
            body_text: Some("Important message".to_string()),
            body_html: None,
            attachments: Vec::new(),
            message_id: "123".to_string(),
            in_reply_to: None,
            references: Vec::new(),
            thread_id: None,
        };
        
        assert!(service.determine_email_importance(&urgent_message));
        
        let normal_message = StoredMessage {
            subject: "Regular email".to_string(),
            ..urgent_message
        };
        
        assert!(!service.determine_email_importance(&normal_message));
    }

    #[test]
    fn test_calendar_priority_determination() {
        let config = NotificationConfig::default();
        let service = NotificationIntegrationService::new(config);
        
        let urgent_event = Event {
            id: Uuid::new_v4().to_string(),
            uid: Uuid::new_v4().to_string(),
            title: "URGENT: Critical Meeting".to_string(),
            description: None,
            start_time: Utc::now() + ChronoDuration::minutes(30),
            end_time: Utc::now() + ChronoDuration::hours(1),
            all_day: false,
            location: None,
            attendees: Vec::new(),
            organizer: None,
            status: EventStatus::Confirmed,
            calendar_id: "default".to_string(),
            recurrence: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            categories: Vec::new(),
            priority: crate::calendar::EventPriority::Normal,
            reminders: Vec::new(),
            sequence: 0,
            url: None,
            etag: None,
        };
        
        assert_eq!(service.determine_calendar_priority(&urgent_event), NotificationPriority::High);
        
        let normal_event = Event {
            title: "Regular meeting".to_string(),
            start_time: Utc::now() + ChronoDuration::days(1),
            ..urgent_event
        };
        
        assert_eq!(service.determine_calendar_priority(&normal_event), NotificationPriority::Normal);
    }
}