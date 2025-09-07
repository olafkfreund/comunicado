//! Event-Driven Notification Bridge
//!
//! This module provides integration between the event system and the notification
//! system, automatically triggering notifications based on processed events.

use crate::events::bus::{EventHandler, HandlerPriority};
use crate::events::types::{
    AccountEvent, AccountEventData, AppEvent, AppEventData, CalendarEvent, CalendarEventData,
    EmailEvent, EmailEventData, UIEvent, UIEventData,
};
use crate::events::EventError;
use crate::notifications::{
    types::{
        CalendarEventType, EmailEventType, NotificationEvent, NotificationPriority, SystemEventType,
    },
    UnifiedNotificationManager,
};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

/// Bridge between the event system and notification system
pub struct NotificationBridge {
    notification_sender: mpsc::UnboundedSender<NotificationEvent>,
}

impl NotificationBridge {
    /// Create a new notification bridge with a connection to the notification manager
    pub fn new(notification_manager: &UnifiedNotificationManager) -> Self {
        Self {
            notification_sender: notification_manager.get_sender(),
        }
    }

    /// Convert an email event to a notification event
    fn email_event_to_notification(
        &self,
        event_data: &EmailEventData,
    ) -> Option<NotificationEvent> {
        let notification_event = match &event_data.event {
            EmailEvent::EmailDeleted {
                account_id,
                email_id,
            } => {
                debug!(
                    "Converting email deletion event to notification: {} in {}",
                    email_id, account_id
                );
                // Low priority for deletion events as they're user-initiated
                NotificationEvent::Email {
                    event_type: EmailEventType::MessageDeleted,
                    account_id: account_id.clone(),
                    folder_name: None,
                    message: None,
                    message_id: Some(email_id.to_string()),
                    priority: NotificationPriority::Low,
                }
            }

            EmailEvent::EmailMarkedRead {
                account_id,
                email_id,
            } => {
                debug!("Email marked as read: {} in {}", email_id, account_id);
                // Very low priority for read status changes
                NotificationEvent::Email {
                    event_type: EmailEventType::MessageUpdated,
                    account_id: account_id.clone(),
                    folder_name: None,
                    message: None,
                    message_id: Some(email_id.to_string()),
                    priority: NotificationPriority::Low,
                }
            }

            EmailEvent::EmailMarkedUnread {
                account_id: _,
                email_id: _,
            } => {
                // Don't notify for unread events - they're typically user-initiated
                return None;
            }

            EmailEvent::EmailArchived {
                account_id,
                email_id,
            } => {
                debug!("Email archived: {} in {}", email_id, account_id);
                NotificationEvent::Email {
                    event_type: EmailEventType::MessageUpdated,
                    account_id: account_id.clone(),
                    folder_name: Some("Archive".to_string()),
                    message: None,
                    message_id: Some(email_id.to_string()),
                    priority: NotificationPriority::Low,
                }
            }

            EmailEvent::EmailComposed { draft_id } => {
                info!("Email composed: {}", draft_id);
                NotificationEvent::Email {
                    event_type: EmailEventType::MessageSent,
                    account_id: "Unknown".to_string(), // Draft doesn't have account_id
                    folder_name: Some("Sent".to_string()),
                    message: None,
                    message_id: Some(draft_id.to_string()),
                    priority: NotificationPriority::Normal,
                }
            }

            EmailEvent::EmailReplied {
                original_id,
                reply_id,
            } => {
                info!("Email reply sent: {} replying to {}", reply_id, original_id);
                NotificationEvent::Email {
                    event_type: EmailEventType::MessageSent,
                    account_id: "Unknown".to_string(), // No account_id in event
                    folder_name: Some("Sent".to_string()),
                    message: None,
                    message_id: Some(reply_id.to_string()),
                    priority: NotificationPriority::Normal,
                }
            }

            EmailEvent::EmailForwarded {
                original_id,
                forward_id,
            } => {
                info!("Email forwarded: {} forwarding {}", forward_id, original_id);
                NotificationEvent::Email {
                    event_type: EmailEventType::MessageSent,
                    account_id: "Unknown".to_string(), // No account_id in event
                    folder_name: Some("Sent".to_string()),
                    message: None,
                    message_id: Some(forward_id.to_string()),
                    priority: NotificationPriority::Normal,
                }
            }

            EmailEvent::EmailReceived {
                account_id,
                email_id,
            } => {
                info!("New email received: {} in {}", email_id, account_id);
                NotificationEvent::Email {
                    event_type: EmailEventType::NewMessage,
                    account_id: account_id.clone(),
                    folder_name: None,
                    message: None, // TODO: Retrieve actual message details
                    message_id: Some(email_id.to_string()),
                    priority: NotificationPriority::High,
                }
            }

            EmailEvent::EmailSent {
                account_id,
                email_id,
            } => {
                info!("Email sent: {} in {}", email_id, account_id);
                NotificationEvent::Email {
                    event_type: EmailEventType::MessageSent,
                    account_id: account_id.clone(),
                    folder_name: Some("Sent".to_string()),
                    message: None,
                    message_id: Some(email_id.to_string()),
                    priority: NotificationPriority::Normal,
                }
            }

            EmailEvent::SearchStarted { query, scope: _ } => {
                debug!("Search started: {}", query);
                // Don't notify for search start - it's a user-initiated action
                return None;
            }

            EmailEvent::SearchCompleted { query, results } => {
                info!(
                    "Search completed: '{}' found {} results",
                    query,
                    results.len()
                );
                if results.len() > 0 {
                    NotificationEvent::System {
                        event_type: SystemEventType::ConfigurationChanged,
                        message: format!("Search '{}' found {} results", query, results.len()),
                        priority: NotificationPriority::Low,
                    }
                } else {
                    // Don't notify for searches with no results
                    return None;
                }
            }

            EmailEvent::SearchFailed { query, error } => {
                info!("Search failed: '{}' error: {}", query, error);
                NotificationEvent::System {
                    event_type: SystemEventType::ConnectionError, // Generic error type
                    message: format!("Search '{}' failed: {}", query, error),
                    priority: NotificationPriority::Normal,
                }
            }

            _ => {
                debug!("Unhandled email event type: {:?}", event_data.event);
                return None;
            }
        };

        Some(notification_event)
    }

    /// Convert a calendar event to a notification event
    fn calendar_event_to_notification(
        &self,
        event_data: &CalendarEventData,
    ) -> Option<NotificationEvent> {
        let notification_event = match &event_data.event {
            CalendarEvent::EventCreated {
                calendar_id,
                event_id,
            } => {
                info!("Calendar event created: {} in {}", event_id, calendar_id);
                NotificationEvent::Calendar {
                    event_type: CalendarEventType::EventCreated,
                    calendar_id: calendar_id.clone(),
                    event: None, // TODO: Retrieve actual event details
                    event_id: Some(event_id.clone()),
                    priority: NotificationPriority::Normal,
                }
            }

            CalendarEvent::EventUpdated {
                calendar_id,
                event_id,
            } => {
                info!("Calendar event updated: {} in {}", event_id, calendar_id);
                NotificationEvent::Calendar {
                    event_type: CalendarEventType::EventUpdated,
                    calendar_id: calendar_id.clone(),
                    event: None, // TODO: Retrieve actual event details
                    event_id: Some(event_id.clone()),
                    priority: NotificationPriority::Normal,
                }
            }

            CalendarEvent::EventDeleted {
                calendar_id,
                event_id,
            } => {
                info!("Calendar event deleted: {} in {}", event_id, calendar_id);
                NotificationEvent::Calendar {
                    event_type: CalendarEventType::EventDeleted,
                    calendar_id: calendar_id.clone(),
                    event: None,
                    event_id: Some(event_id.clone()),
                    priority: NotificationPriority::Low,
                }
            }

            CalendarEvent::CalendarSynced {
                calendar_id,
                event_count,
            } => {
                info!(
                    "Calendar synced: {} with {} events",
                    calendar_id, event_count
                );
                NotificationEvent::Calendar {
                    event_type: CalendarEventType::SyncCompleted {
                        new_count: *event_count as u32,
                        updated_count: 0,
                    },
                    calendar_id: calendar_id.clone(),
                    event: None,
                    event_id: None,
                    priority: NotificationPriority::Low,
                }
            }

            CalendarEvent::InvitationReceived { event_id, from } => {
                info!(
                    "Calendar invitation received for {} from {}",
                    event_id, from
                );
                NotificationEvent::Calendar {
                    event_type: CalendarEventType::EventCreated, // Treat as new event notification
                    calendar_id: "Unknown".to_string(), // No calendar_id in invitation event
                    event: None,
                    event_id: Some(event_id.clone()),
                    priority: NotificationPriority::High,
                }
            }

            CalendarEvent::InvitationAccepted { event_id } => {
                info!("Calendar invitation accepted for {}", event_id);
                NotificationEvent::Calendar {
                    event_type: CalendarEventType::RSVPSent {
                        response: "Accepted".to_string(),
                    },
                    calendar_id: "Unknown".to_string(),
                    event: None,
                    event_id: Some(event_id.clone()),
                    priority: NotificationPriority::Normal,
                }
            }

            CalendarEvent::InvitationDeclined { event_id } => {
                info!("Calendar invitation declined for {}", event_id);
                NotificationEvent::Calendar {
                    event_type: CalendarEventType::RSVPSent {
                        response: "Declined".to_string(),
                    },
                    calendar_id: "Unknown".to_string(),
                    event: None,
                    event_id: Some(event_id.clone()),
                    priority: NotificationPriority::Normal,
                }
            }

            _ => {
                debug!("Unhandled calendar event type: {:?}", event_data.event);
                return None;
            }
        };

        Some(notification_event)
    }

    /// Convert an account event to a notification event
    fn account_event_to_notification(
        &self,
        event_data: &AccountEventData,
    ) -> Option<NotificationEvent> {
        let notification_event = match &event_data.event {
            AccountEvent::AccountSyncCompleted {
                account_id,
                duration_ms,
            } => {
                info!(
                    "Account sync completed: {} in {} ms",
                    account_id, duration_ms
                );
                NotificationEvent::Email {
                    event_type: EmailEventType::SyncCompleted {
                        new_count: 0,
                        updated_count: 0,
                    },
                    account_id: account_id.clone(),
                    folder_name: None,
                    message: None,
                    message_id: None,
                    priority: NotificationPriority::Low,
                }
            }

            AccountEvent::AccountSyncFailed { account_id, error } => {
                info!("Account sync failed: {} - {}", account_id, error);
                NotificationEvent::Email {
                    event_type: EmailEventType::SyncFailed {
                        error: error.clone(),
                    },
                    account_id: account_id.clone(),
                    folder_name: None,
                    message: None,
                    message_id: None,
                    priority: NotificationPriority::High,
                }
            }

            AccountEvent::AccountAdded {
                account_id,
                provider,
            } => {
                info!("New account added: {} ({})", account_id, provider);
                NotificationEvent::System {
                    event_type: SystemEventType::ConfigurationChanged,
                    message: format!(
                        "Account '{}' ({}) has been added successfully",
                        account_id, provider
                    ),
                    priority: NotificationPriority::Normal,
                }
            }

            AccountEvent::AccountRemoved { account_id } => {
                info!("Account removed: {}", account_id);
                NotificationEvent::System {
                    event_type: SystemEventType::ConfigurationChanged,
                    message: format!("Account '{}' has been removed", account_id),
                    priority: NotificationPriority::Normal,
                }
            }

            _ => {
                debug!("Unhandled account event type: {:?}", event_data.event);
                return None;
            }
        };

        Some(notification_event)
    }

    /// Convert a UI event to a notification event
    fn ui_event_to_notification(&self, event_data: &UIEventData) -> Option<NotificationEvent> {
        // Most UI events don't need notifications, but some system-level ones might
        match &event_data.event {
            UIEvent::ThemeChanged { theme_name } => {
                debug!("UI theme changed to: {}", theme_name);
                NotificationEvent::System {
                    event_type: SystemEventType::ConfigurationChanged,
                    message: format!("Theme changed to '{}'", theme_name),
                    priority: NotificationPriority::Low,
                }
            }

            // Most other UI events don't need notifications
            UIEvent::ModeChanged { .. }
            | UIEvent::PaneChanged { .. }
            | UIEvent::ViewChanged { .. } => {
                // These are frequent UI state changes that don't need notifications
                return None;
            }

            _ => {
                // Most UI events (mode changes, pane switches) don't need notifications
                return None;
            }
        }
        .into()
    }

    /// Convert an application event to a notification event
    fn app_event_to_notification(&self, event_data: &AppEventData) -> Option<NotificationEvent> {
        let notification_event = match &event_data.event {
            AppEvent::AppStarted {
                version,
                startup_time_ms,
            } => {
                info!(
                    "Application started (v{}) in {} ms",
                    version, startup_time_ms
                );
                NotificationEvent::System {
                    event_type: SystemEventType::AppStarted,
                    message: format!("Comunicado v{} has started successfully", version),
                    priority: NotificationPriority::Low,
                }
            }

            AppEvent::AppShuttingDown => {
                info!("Application shutting down");
                NotificationEvent::System {
                    event_type: SystemEventType::AppShutdown,
                    message: "Comunicado is shutting down".to_string(),
                    priority: NotificationPriority::Low,
                }
            }

            AppEvent::ConfigLoaded { config_path } => {
                info!("Configuration loaded from: {}", config_path);
                NotificationEvent::System {
                    event_type: SystemEventType::ConfigurationChanged,
                    message: "Configuration has been reloaded".to_string(),
                    priority: NotificationPriority::Low,
                }
            }

            AppEvent::ConfigChanged {
                setting,
                old_value: _,
                new_value: _,
            } => {
                info!("Configuration setting changed: {}", setting);
                NotificationEvent::System {
                    event_type: SystemEventType::ConfigurationChanged,
                    message: format!("Setting '{}' has been updated", setting),
                    priority: NotificationPriority::Low,
                }
            }

            _ => {
                debug!("Unhandled app event type: {:?}", event_data.event);
                return None;
            }
        };

        Some(notification_event)
    }

    /// Send a notification event to the notification system
    fn send_notification(&self, notification_event: NotificationEvent) {
        if let Err(e) = self.notification_sender.send(notification_event) {
            error!("Failed to send notification event: {}", e);
        }
    }
}

/// Email event handler that bridges to notifications
impl EventHandler<EmailEventData> for NotificationBridge {
    fn handle(&mut self, event: &EmailEventData) -> Result<(), EventError> {
        if let Some(notification) = self.email_event_to_notification(event) {
            self.send_notification(notification);
        }
        Ok(())
    }

    fn priority(&self) -> HandlerPriority {
        HandlerPriority::Low // Notifications shouldn't block other operations
    }
}

/// Calendar event handler that bridges to notifications
impl EventHandler<CalendarEventData> for NotificationBridge {
    fn handle(&mut self, event: &CalendarEventData) -> Result<(), EventError> {
        if let Some(notification) = self.calendar_event_to_notification(event) {
            self.send_notification(notification);
        }
        Ok(())
    }

    fn priority(&self) -> HandlerPriority {
        HandlerPriority::Low
    }
}

/// Account event handler that bridges to notifications
impl EventHandler<AccountEventData> for NotificationBridge {
    fn handle(&mut self, event: &AccountEventData) -> Result<(), EventError> {
        if let Some(notification) = self.account_event_to_notification(event) {
            self.send_notification(notification);
        }
        Ok(())
    }

    fn priority(&self) -> HandlerPriority {
        HandlerPriority::Low
    }
}

/// UI event handler that bridges to notifications
impl EventHandler<UIEventData> for NotificationBridge {
    fn handle(&mut self, event: &UIEventData) -> Result<(), EventError> {
        if let Some(notification) = self.ui_event_to_notification(event) {
            self.send_notification(notification);
        }
        Ok(())
    }

    fn priority(&self) -> HandlerPriority {
        HandlerPriority::Low
    }
}

/// Application event handler that bridges to notifications
impl EventHandler<AppEventData> for NotificationBridge {
    fn handle(&mut self, event: &AppEventData) -> Result<(), EventError> {
        if let Some(notification) = self.app_event_to_notification(event) {
            self.send_notification(notification);
        }
        Ok(())
    }

    fn priority(&self) -> HandlerPriority {
        HandlerPriority::Low
    }
}

/// Wrapper for email notification bridge
pub struct EmailNotificationBridge {
    bridge: Arc<std::sync::Mutex<NotificationBridge>>,
}

impl EmailNotificationBridge {
    pub fn new(notification_manager: &UnifiedNotificationManager) -> Self {
        Self {
            bridge: Arc::new(std::sync::Mutex::new(NotificationBridge::new(
                notification_manager,
            ))),
        }
    }
}

impl EventHandler<EmailEventData> for EmailNotificationBridge {
    fn handle(&mut self, event: &EmailEventData) -> Result<(), EventError> {
        let mut bridge = match self.bridge.lock() {
            Ok(bridge) => bridge,
            Err(_) => {
                return Err(EventError::ProcessingFailed(
                    "Failed to lock notification bridge".to_string(),
                ))
            }
        };
        bridge.handle(event)
    }

    fn priority(&self) -> HandlerPriority {
        HandlerPriority::Low
    }
}

/// Wrapper for calendar notification bridge
pub struct CalendarNotificationBridge {
    bridge: Arc<std::sync::Mutex<NotificationBridge>>,
}

impl CalendarNotificationBridge {
    pub fn new(notification_manager: &UnifiedNotificationManager) -> Self {
        Self {
            bridge: Arc::new(std::sync::Mutex::new(NotificationBridge::new(
                notification_manager,
            ))),
        }
    }
}

impl EventHandler<CalendarEventData> for CalendarNotificationBridge {
    fn handle(&mut self, event: &CalendarEventData) -> Result<(), EventError> {
        let mut bridge = match self.bridge.lock() {
            Ok(bridge) => bridge,
            Err(_) => {
                return Err(EventError::ProcessingFailed(
                    "Failed to lock notification bridge".to_string(),
                ))
            }
        };
        bridge.handle(event)
    }

    fn priority(&self) -> HandlerPriority {
        HandlerPriority::Low
    }
}

/// Wrapper for account notification bridge
pub struct AccountNotificationBridge {
    bridge: Arc<std::sync::Mutex<NotificationBridge>>,
}

impl AccountNotificationBridge {
    pub fn new(notification_manager: &UnifiedNotificationManager) -> Self {
        Self {
            bridge: Arc::new(std::sync::Mutex::new(NotificationBridge::new(
                notification_manager,
            ))),
        }
    }
}

impl EventHandler<AccountEventData> for AccountNotificationBridge {
    fn handle(&mut self, event: &AccountEventData) -> Result<(), EventError> {
        let mut bridge = match self.bridge.lock() {
            Ok(bridge) => bridge,
            Err(_) => {
                return Err(EventError::ProcessingFailed(
                    "Failed to lock notification bridge".to_string(),
                ))
            }
        };
        bridge.handle(event)
    }

    fn priority(&self) -> HandlerPriority {
        HandlerPriority::Low
    }
}

/// Wrapper for UI notification bridge
pub struct UINotificationBridge {
    bridge: Arc<std::sync::Mutex<NotificationBridge>>,
}

impl UINotificationBridge {
    pub fn new(notification_manager: &UnifiedNotificationManager) -> Self {
        Self {
            bridge: Arc::new(std::sync::Mutex::new(NotificationBridge::new(
                notification_manager,
            ))),
        }
    }
}

impl EventHandler<UIEventData> for UINotificationBridge {
    fn handle(&mut self, event: &UIEventData) -> Result<(), EventError> {
        let mut bridge = match self.bridge.lock() {
            Ok(bridge) => bridge,
            Err(_) => {
                return Err(EventError::ProcessingFailed(
                    "Failed to lock notification bridge".to_string(),
                ))
            }
        };
        bridge.handle(event)
    }

    fn priority(&self) -> HandlerPriority {
        HandlerPriority::Low
    }
}

/// Wrapper for application notification bridge
pub struct AppNotificationBridge {
    bridge: Arc<std::sync::Mutex<NotificationBridge>>,
}

impl AppNotificationBridge {
    pub fn new(notification_manager: &UnifiedNotificationManager) -> Self {
        Self {
            bridge: Arc::new(std::sync::Mutex::new(NotificationBridge::new(
                notification_manager,
            ))),
        }
    }
}

impl EventHandler<AppEventData> for AppNotificationBridge {
    fn handle(&mut self, event: &AppEventData) -> Result<(), EventError> {
        let mut bridge = match self.bridge.lock() {
            Ok(bridge) => bridge,
            Err(_) => {
                return Err(EventError::ProcessingFailed(
                    "Failed to lock notification bridge".to_string(),
                ))
            }
        };
        bridge.handle(event)
    }

    fn priority(&self) -> HandlerPriority {
        HandlerPriority::Low
    }
}

/// Registry for managing notification bridge handlers
pub struct NotificationBridgeRegistry {
    email_bridge: EmailNotificationBridge,
    calendar_bridge: CalendarNotificationBridge,
    account_bridge: AccountNotificationBridge,
    ui_bridge: UINotificationBridge,
    app_bridge: AppNotificationBridge,
}

impl NotificationBridgeRegistry {
    /// Create a new notification bridge registry
    pub fn new(notification_manager: &UnifiedNotificationManager) -> Self {
        Self {
            email_bridge: EmailNotificationBridge::new(notification_manager),
            calendar_bridge: CalendarNotificationBridge::new(notification_manager),
            account_bridge: AccountNotificationBridge::new(notification_manager),
            ui_bridge: UINotificationBridge::new(notification_manager),
            app_bridge: AppNotificationBridge::new(notification_manager),
        }
    }

    /// Register all notification bridges with the event bus
    pub fn register_all_bridges(self, bus: &mut crate::events::bus::EventBus) {
        // Subscribe notification bridges to their respective event types
        bus.subscribe::<EmailEventData, _>(self.email_bridge);
        bus.subscribe::<CalendarEventData, _>(self.calendar_bridge);
        bus.subscribe::<AccountEventData, _>(self.account_bridge);
        bus.subscribe::<UIEventData, _>(self.ui_bridge);
        bus.subscribe::<AppEventData, _>(self.app_bridge);

        info!("All notification bridges registered with event bus");
    }
}
