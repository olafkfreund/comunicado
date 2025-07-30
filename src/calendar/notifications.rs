use crate::calendar::{CalendarDatabase, Event};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};

/// Types of calendar notifications
#[derive(Debug, Clone)]
pub enum CalendarNotification {
    /// New event created
    EventCreated { calendar_id: String, event: Event },
    /// Event updated
    EventUpdated { calendar_id: String, event: Event },
    /// Event deleted
    EventDeleted {
        calendar_id: String,
        event_id: String,
    },
    /// Upcoming event reminder
    EventReminder {
        calendar_id: String,
        event: Event,
        minutes_until: i64,
    },
    /// Calendar synchronization started
    SyncStarted { calendar_id: String },
    /// Calendar synchronization completed
    SyncCompleted {
        calendar_id: String,
        new_count: u32,
        updated_count: u32,
    },
    /// Calendar synchronization failed
    SyncFailed { calendar_id: String, error: String },
    /// RSVP response sent
    RSVPSent { event_id: String, response: String },
}

impl CalendarNotification {
    /// Get the calendar ID from the notification
    pub fn calendar_id(&self) -> &str {
        match self {
            CalendarNotification::EventCreated { calendar_id, .. } => calendar_id,
            CalendarNotification::EventUpdated { calendar_id, .. } => calendar_id,
            CalendarNotification::EventDeleted { calendar_id, .. } => calendar_id,
            CalendarNotification::EventReminder { calendar_id, .. } => calendar_id,
            CalendarNotification::SyncStarted { calendar_id, .. } => calendar_id,
            CalendarNotification::SyncCompleted { calendar_id, .. } => calendar_id,
            CalendarNotification::SyncFailed { calendar_id, .. } => calendar_id,
            CalendarNotification::RSVPSent { .. } => "unknown",
        }
    }
}

/// Calendar notification manager
pub struct CalendarNotificationManager {
    /// Broadcast sender for sending notifications to all subscribers
    sender: broadcast::Sender<CalendarNotification>,
    /// Channel for receiving notifications from calendar sync processes
    notification_receiver: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<CalendarNotification>>>,
    /// Sender for internal notification publishing
    notification_sender: mpsc::UnboundedSender<CalendarNotification>,
    /// Database reference for event operations
    #[allow(dead_code)]
    database: Arc<CalendarDatabase>,
}

impl CalendarNotificationManager {
    /// Create a new calendar notification manager
    pub fn new(database: Arc<CalendarDatabase>) -> Self {
        let (sender, _) = broadcast::channel(1000); // Buffer for 1000 notifications
        let (notification_sender, notification_receiver) = mpsc::unbounded_channel();

        Self {
            sender,
            notification_receiver: Arc::new(tokio::sync::Mutex::new(notification_receiver)),
            notification_sender,
            database,
        }
    }

    /// Subscribe to calendar notifications
    pub fn subscribe(&self) -> broadcast::Receiver<CalendarNotification> {
        self.sender.subscribe()
    }

    /// Get a sender for publishing notifications
    pub fn get_sender(&self) -> mpsc::UnboundedSender<CalendarNotification> {
        self.notification_sender.clone()
    }

    /// Start the notification processing loop
    pub async fn start(&self) {
        let receiver = self.notification_receiver.clone();
        let sender = self.sender.clone();

        tokio::spawn(async move {
            let mut receiver = receiver.lock().await;

            while let Some(notification) = receiver.recv().await {
                // Broadcast the notification to all subscribers
                if let Err(e) = sender.send(notification.clone()) {
                    tracing::warn!("Failed to broadcast calendar notification: {}", e);
                }

                // Log the notification for debugging
                tracing::debug!("Processed calendar notification: {:?}", notification);
            }
        });
    }

    /// Publish an event created notification
    pub async fn notify_event_created(&self, calendar_id: String, event: Event) {
        let notification = CalendarNotification::EventCreated { calendar_id, event };

        if let Err(e) = self.notification_sender.send(notification) {
            tracing::error!("Failed to send event created notification: {}", e);
        }
    }

    /// Publish an event updated notification
    pub async fn notify_event_updated(&self, calendar_id: String, event: Event) {
        let notification = CalendarNotification::EventUpdated { calendar_id, event };

        if let Err(e) = self.notification_sender.send(notification) {
            tracing::error!("Failed to send event updated notification: {}", e);
        }
    }

    /// Publish an event deleted notification
    pub async fn notify_event_deleted(&self, calendar_id: String, event_id: String) {
        let notification = CalendarNotification::EventDeleted {
            calendar_id,
            event_id,
        };

        if let Err(e) = self.notification_sender.send(notification) {
            tracing::error!("Failed to send event deleted notification: {}", e);
        }
    }

    /// Publish an event reminder notification
    pub async fn notify_event_reminder(
        &self,
        calendar_id: String,
        event: Event,
        minutes_until: i64,
    ) {
        let notification = CalendarNotification::EventReminder {
            calendar_id,
            event,
            minutes_until,
        };

        if let Err(e) = self.notification_sender.send(notification) {
            tracing::error!("Failed to send event reminder notification: {}", e);
        }
    }

    /// Publish a sync started notification
    pub async fn notify_sync_started(&self, calendar_id: String) {
        let notification = CalendarNotification::SyncStarted { calendar_id };

        if let Err(e) = self.notification_sender.send(notification) {
            tracing::error!("Failed to send sync started notification: {}", e);
        }
    }

    /// Publish a sync completed notification
    pub async fn notify_sync_completed(
        &self,
        calendar_id: String,
        new_count: u32,
        updated_count: u32,
    ) {
        let notification = CalendarNotification::SyncCompleted {
            calendar_id,
            new_count,
            updated_count,
        };

        if let Err(e) = self.notification_sender.send(notification) {
            tracing::error!("Failed to send sync completed notification: {}", e);
        }
    }

    /// Publish a sync failed notification
    pub async fn notify_sync_failed(&self, calendar_id: String, error: String) {
        let notification = CalendarNotification::SyncFailed { calendar_id, error };

        if let Err(e) = self.notification_sender.send(notification) {
            tracing::error!("Failed to send sync failed notification: {}", e);
        }
    }

    /// Publish an RSVP sent notification
    pub async fn notify_rsvp_sent(&self, event_id: String, response: String) {
        let notification = CalendarNotification::RSVPSent { event_id, response };

        if let Err(e) = self.notification_sender.send(notification) {
            tracing::error!("Failed to send RSVP sent notification: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::Event;

    #[tokio::test]
    async fn test_notification_manager_creation() {
        let database = Arc::new(CalendarDatabase::new_in_memory().await.unwrap());
        let manager = CalendarNotificationManager::new(database);

        // Test that we can get a sender
        let _sender = manager.get_sender();

        // Test that we can subscribe
        let _receiver = manager.subscribe();
    }

    #[tokio::test]
    async fn test_notification_broadcast() {
        let database = Arc::new(CalendarDatabase::new_in_memory().await.unwrap());
        let manager = CalendarNotificationManager::new(database);

        // Start the notification processing
        manager.start().await;

        // Subscribe to notifications
        let mut receiver = manager.subscribe();

        // Create a test event
        let event = Event::new(
            "test-event".to_string(),
            "Test Event".to_string(),
            chrono::Utc::now(),
            chrono::Utc::now() + chrono::Duration::hours(1),
        );

        // Send a notification
        manager
            .notify_event_created("test-calendar".to_string(), event.clone())
            .await;

        // Verify we received the notification
        if let Ok(notification) = receiver.recv().await {
            match notification {
                CalendarNotification::EventCreated {
                    calendar_id,
                    event: received_event,
                } => {
                    assert_eq!(calendar_id, "test-calendar");
                    assert_eq!(received_event.id, event.id);
                }
                _ => panic!("Unexpected notification type"),
            }
        }
    }
}
