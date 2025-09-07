//! Notification bridge for mobile companion app

use super::{MobileError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Notification bridge manager
pub struct NotificationBridge {
    pending_notifications: RwLock<HashMap<Uuid, NotificationPayload>>,
    sent_notifications: RwLock<Vec<SentNotification>>,
    statistics: RwLock<NotificationStatistics>,
    rate_limiter: RateLimiter,
}

/// Notification payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPayload {
    pub id: Uuid,
    pub title: String,
    pub body: String,
    pub notification_type: String,
    pub priority: NotificationPriority,
    pub category: NotificationCategory,
    pub actions: Vec<NotificationAction>,
    pub custom_data: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub sound: Option<String>,
    pub badge_count: Option<u32>,
    pub icon: Option<String>,
}

/// Notification priority levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum NotificationPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Notification categories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationCategory {
    Email,
    Calendar,
    Task,
    Message,
    Alert,
    Social,
    Update,
    Reminder,
}

/// Notification action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationAction {
    pub id: String,
    pub title: String,
    pub icon: Option<String>,
    pub action_type: ActionType,
    pub requires_auth: bool,
}

/// Action types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    Reply,
    Archive,
    Delete,
    MarkRead,
    Snooze,
    Accept,
    Decline,
    Custom(String),
}

/// Sent notification record
#[derive(Debug, Clone)]
pub struct SentNotification {
    pub id: Uuid,
    pub device_id: Uuid,
    pub notification_id: Uuid,
    pub sent_at: DateTime<Utc>,
    pub delivery_method: DeliveryMethod,
    pub status: DeliveryStatus,
    pub retry_count: u32,
}

/// Delivery methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeliveryMethod {
    WebSocket,
    Push,
    KdeConnect,
    Email,
    Sms,
}

/// Delivery status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeliveryStatus {
    Pending,
    Sent,
    Delivered,
    Failed,
    Expired,
}

/// Notification statistics
#[derive(Debug, Clone, Default)]
pub struct NotificationStatistics {
    pub total_sent: u64,
    pub total_delivered: u64,
    pub total_failed: u64,
    pub by_priority: HashMap<NotificationPriority, u64>,
    pub by_category: HashMap<String, u64>,
    pub by_method: HashMap<String, u64>,
}

/// Rate limiter for notifications
pub struct RateLimiter {
    limits: HashMap<NotificationPriority, RateLimit>,
    usage: RwLock<HashMap<NotificationPriority, Vec<DateTime<Utc>>>>,
}

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimit {
    pub max_per_minute: u32,
    pub max_per_hour: u32,
    pub burst_size: u32,
}

impl NotificationBridge {
    pub fn new() -> Result<Self> {
        Ok(Self {
            pending_notifications: RwLock::new(HashMap::new()),
            sent_notifications: RwLock::new(Vec::new()),
            statistics: RwLock::new(NotificationStatistics::default()),
            rate_limiter: RateLimiter::new(),
        })
    }

    /// Initialize the notification bridge
    pub async fn initialize(&mut self) -> Result<()> {
        // Initialize rate limiter
        self.rate_limiter.initialize().await?;

        // Clean up expired notifications
        self.cleanup_expired_notifications().await?;

        Ok(())
    }

    /// Create a new notification
    pub async fn create_notification(
        &self,
        title: String,
        body: String,
        notification_type: String,
        priority: NotificationPriority,
        category: NotificationCategory,
    ) -> Result<Uuid> {
        let notification = NotificationPayload {
            id: Uuid::new_v4(),
            title,
            body,
            notification_type,
            priority,
            category,
            actions: vec![],
            custom_data: HashMap::new(),
            created_at: Utc::now(),
            expires_at: None,
            sound: None,
            badge_count: None,
            icon: None,
        };

        let id = notification.id;
        let mut pending = self.pending_notifications.write().await;
        pending.insert(id, notification);

        Ok(id)
    }

    /// Add action to notification
    pub async fn add_action(
        &self,
        notification_id: Uuid,
        action: NotificationAction,
    ) -> Result<()> {
        let mut pending = self.pending_notifications.write().await;

        if let Some(notification) = pending.get_mut(&notification_id) {
            notification.actions.push(action);
            Ok(())
        } else {
            Err(MobileError::ConfigurationError(
                "Notification not found".to_string(),
            ))
        }
    }

    /// Set custom data for notification
    pub async fn set_custom_data(
        &self,
        notification_id: Uuid,
        key: String,
        value: String,
    ) -> Result<()> {
        let mut pending = self.pending_notifications.write().await;

        if let Some(notification) = pending.get_mut(&notification_id) {
            notification.custom_data.insert(key, value);
            Ok(())
        } else {
            Err(MobileError::ConfigurationError(
                "Notification not found".to_string(),
            ))
        }
    }

    /// Get pending notification
    pub async fn get_notification(&self, notification_id: Uuid) -> Option<NotificationPayload> {
        let pending = self.pending_notifications.read().await;
        pending.get(&notification_id).cloned()
    }

    /// Mark notification as sent
    pub async fn mark_sent(
        &self,
        notification_id: Uuid,
        device_id: Uuid,
        delivery_method: DeliveryMethod,
    ) -> Result<()> {
        // Remove from pending
        let mut pending = self.pending_notifications.write().await;
        let notification = pending.remove(&notification_id);
        drop(pending);

        if notification.is_none() {
            return Err(MobileError::ConfigurationError(
                "Notification not found".to_string(),
            ));
        }

        // Add to sent notifications
        let sent_notification = SentNotification {
            id: Uuid::new_v4(),
            device_id,
            notification_id,
            sent_at: Utc::now(),
            delivery_method,
            status: DeliveryStatus::Sent,
            retry_count: 0,
        };

        let mut sent = self.sent_notifications.write().await;
        sent.push(sent_notification);

        // Update statistics
        self.update_statistics(&notification.unwrap()).await;

        Ok(())
    }

    /// Mark notification as delivered
    pub async fn mark_delivered(&self, notification_id: Uuid) -> Result<()> {
        let mut sent = self.sent_notifications.write().await;

        if let Some(notification) = sent
            .iter_mut()
            .find(|n| n.notification_id == notification_id)
        {
            notification.status = DeliveryStatus::Delivered;

            // Update statistics
            let mut stats = self.statistics.write().await;
            stats.total_delivered += 1;
        }

        Ok(())
    }

    /// Mark notification as failed
    pub async fn mark_failed(&self, notification_id: Uuid, retry: bool) -> Result<()> {
        let mut sent = self.sent_notifications.write().await;

        if let Some(notification) = sent
            .iter_mut()
            .find(|n| n.notification_id == notification_id)
        {
            if retry && notification.retry_count < 3 {
                notification.retry_count += 1;
                notification.status = DeliveryStatus::Pending;

                // Move back to pending for retry
                // This would typically involve re-queuing the notification
            } else {
                notification.status = DeliveryStatus::Failed;

                // Update statistics
                let mut stats = self.statistics.write().await;
                stats.total_failed += 1;
            }
        }

        Ok(())
    }

    /// Check rate limits
    pub async fn check_rate_limit(&self, priority: &NotificationPriority) -> Result<bool> {
        self.rate_limiter.check_limit(priority).await
    }

    /// Get all pending notifications
    pub async fn get_pending_notifications(&self) -> Vec<NotificationPayload> {
        let pending = self.pending_notifications.read().await;
        pending.values().cloned().collect()
    }

    /// Get notification statistics
    pub async fn get_statistics(&self) -> NotificationStatistics {
        let stats = self.statistics.read().await;
        stats.clone()
    }

    /// Get total sent count
    pub async fn get_total_sent(&self) -> u64 {
        let stats = self.statistics.read().await;
        stats.total_sent
    }

    /// Clean up expired notifications
    pub async fn cleanup_expired_notifications(&self) -> Result<()> {
        let now = Utc::now();
        let mut pending = self.pending_notifications.write().await;

        pending.retain(|_, notification| {
            if let Some(expires_at) = notification.expires_at {
                expires_at > now
            } else {
                true // Keep notifications without expiry
            }
        });

        // Clean up old sent notifications (keep last 1000)
        let mut sent = self.sent_notifications.write().await;
        if sent.len() > 1000 {
            let len = sent.len();
            sent.drain(0..len - 1000);
        }

        Ok(())
    }

    /// Create email notification
    pub async fn create_email_notification(
        &self,
        from: String,
        subject: String,
        preview: String,
    ) -> Result<Uuid> {
        let id = self
            .create_notification(
                format!("New email from {}", from),
                format!("{}: {}", subject, preview),
                "email".to_string(),
                NotificationPriority::Normal,
                NotificationCategory::Email,
            )
            .await?;

        // Add email-specific actions
        self.add_action(
            id,
            NotificationAction {
                id: "reply".to_string(),
                title: "Reply".to_string(),
                icon: Some("reply".to_string()),
                action_type: ActionType::Reply,
                requires_auth: false,
            },
        )
        .await?;

        self.add_action(
            id,
            NotificationAction {
                id: "archive".to_string(),
                title: "Archive".to_string(),
                icon: Some("archive".to_string()),
                action_type: ActionType::Archive,
                requires_auth: false,
            },
        )
        .await?;

        Ok(id)
    }

    /// Create calendar notification
    pub async fn create_calendar_notification(
        &self,
        event_title: String,
        start_time: DateTime<Utc>,
        minutes_before: u32,
    ) -> Result<Uuid> {
        let time_str = start_time.format("%H:%M").to_string();
        let reminder_text = match minutes_before {
            0 => "starting now".to_string(),
            1 => "starting in 1 minute".to_string(),
            n if n < 60 => format!("starting in {} minutes", n),
            60 => "starting in 1 hour".to_string(),
            n => format!("starting in {} hours", n / 60),
        };

        let id = self
            .create_notification(
                format!("Meeting reminder: {}", event_title),
                format!("Event {} at {}", reminder_text, time_str),
                "calendar".to_string(),
                if minutes_before <= 5 {
                    NotificationPriority::High
                } else {
                    NotificationPriority::Normal
                },
                NotificationCategory::Calendar,
            )
            .await?;

        // Add calendar-specific actions
        self.add_action(
            id,
            NotificationAction {
                id: "snooze".to_string(),
                title: "Snooze 5min".to_string(),
                icon: Some("snooze".to_string()),
                action_type: ActionType::Snooze,
                requires_auth: false,
            },
        )
        .await?;

        Ok(id)
    }

    // Private methods
    async fn update_statistics(&self, notification: &NotificationPayload) {
        let mut stats = self.statistics.write().await;
        stats.total_sent += 1;

        *stats
            .by_priority
            .entry(notification.priority.clone())
            .or_insert(0) += 1;
        *stats
            .by_category
            .entry(format!("{:?}", notification.category))
            .or_insert(0) += 1;
    }
}

impl RateLimiter {
    pub fn new() -> Self {
        let mut limits = HashMap::new();
        limits.insert(
            NotificationPriority::Low,
            RateLimit {
                max_per_minute: 5,
                max_per_hour: 50,
                burst_size: 10,
            },
        );
        limits.insert(
            NotificationPriority::Normal,
            RateLimit {
                max_per_minute: 10,
                max_per_hour: 100,
                burst_size: 20,
            },
        );
        limits.insert(
            NotificationPriority::High,
            RateLimit {
                max_per_minute: 20,
                max_per_hour: 200,
                burst_size: 30,
            },
        );
        limits.insert(
            NotificationPriority::Critical,
            RateLimit {
                max_per_minute: 50,
                max_per_hour: 500,
                burst_size: 100,
            },
        );

        Self {
            limits,
            usage: RwLock::new(HashMap::new()),
        }
    }

    pub async fn initialize(&mut self) -> Result<()> {
        // Initialize usage tracking
        let mut usage = self.usage.write().await;
        for priority in [
            NotificationPriority::Low,
            NotificationPriority::Normal,
            NotificationPriority::High,
            NotificationPriority::Critical,
        ] {
            usage.insert(priority, Vec::new());
        }
        Ok(())
    }

    pub async fn check_limit(&self, priority: &NotificationPriority) -> Result<bool> {
        let limit = self.limits.get(priority).ok_or_else(|| {
            MobileError::ConfigurationError("Rate limit not configured".to_string())
        })?;

        let mut usage = self.usage.write().await;
        let usage_list = usage.get_mut(priority).unwrap();

        let now = Utc::now();
        let one_minute_ago = now - chrono::Duration::minutes(1);
        let one_hour_ago = now - chrono::Duration::hours(1);

        // Clean up old usage records
        usage_list.retain(|&timestamp| timestamp > one_hour_ago);

        // Check limits
        let recent_minute = usage_list
            .iter()
            .filter(|&&timestamp| timestamp > one_minute_ago)
            .count() as u32;

        let recent_hour = usage_list.len() as u32;

        if recent_minute >= limit.max_per_minute || recent_hour >= limit.max_per_hour {
            return Ok(false);
        }

        // Record this usage
        usage_list.push(now);

        Ok(true)
    }
}
