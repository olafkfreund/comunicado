//! Common Event Handlers
//!
//! This module provides standard event handlers for common system operations
//! like logging, notifications, state updates, and cross-component communication.

use crate::events::bus::{Event, EventHandler, EventError, HandlerPriority};
use crate::events::types::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

// =============================================================================
// UI State Handler - Manages global UI state changes
// =============================================================================

/// Handles UI state changes and maintains global UI state consistency
pub struct UIStateHandler {
    current_pane: Arc<Mutex<FocusedPane>>,
    current_mode: Arc<Mutex<UIMode>>,
    current_view: Arc<Mutex<ViewType>>,
    state_change_listeners: Vec<Box<dyn Fn(&UIEvent) + Send + Sync>>,
}

impl UIStateHandler {
    pub fn new() -> Self {
        Self {
            current_pane: Arc::new(Mutex::new(FocusedPane::MessageList)),
            current_mode: Arc::new(Mutex::new(UIMode::Normal)),
            current_view: Arc::new(Mutex::new(ViewType::EmailList)),
            state_change_listeners: Vec::new(),
        }
    }
    
    pub fn add_listener<F>(&mut self, listener: F)
    where
        F: Fn(&UIEvent) + Send + Sync + 'static,
    {
        self.state_change_listeners.push(Box::new(listener));
    }
    
    pub fn get_current_pane(&self) -> FocusedPane {
        self.current_pane.lock().unwrap().clone()
    }
    
    pub fn get_current_mode(&self) -> UIMode {
        self.current_mode.lock().unwrap().clone()
    }
    
    pub fn get_current_view(&self) -> ViewType {
        self.current_view.lock().unwrap().clone()
    }
}

impl Default for UIStateHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl EventHandler<UIEventData> for UIStateHandler {
    fn handle(&mut self, event: &UIEventData) -> Result<(), EventError> {
        match &event.event {
            UIEvent::PaneChanged { to, .. } => {
                *self.current_pane.lock().unwrap() = to.clone();
                tracing::debug!("UI pane changed to: {:?}", to);
            }
            UIEvent::ModeChanged { to, .. } => {
                *self.current_mode.lock().unwrap() = to.clone();
                tracing::debug!("UI mode changed to: {:?}", to);
            }
            UIEvent::ViewChanged { view } => {
                *self.current_view.lock().unwrap() = view.clone();
                tracing::debug!("UI view changed to: {:?}", view);
            }
            UIEvent::ThemeChanged { theme_name } => {
                tracing::info!("Theme changed to: {}", theme_name);
            }
            _ => {
                // Handle other UI events as needed
                tracing::trace!("Unhandled UI event: {:?}", event.event);
            }
        }
        
        // Notify listeners
        for listener in &self.state_change_listeners {
            listener(&event.event);
        }
        
        Ok(())
    }
    
    fn priority(&self) -> HandlerPriority {
        HandlerPriority::High
    }
}

// =============================================================================
// Notification Handler - Manages system notifications
// =============================================================================

/// Handles events that should trigger user notifications
pub struct NotificationHandler {
    notification_service: Arc<Mutex<dyn NotificationService>>,
    enabled_notifications: HashMap<String, bool>,
}

pub trait NotificationService: Send + Sync {
    fn show_notification(&mut self, title: &str, message: &str, priority: NotificationPriority);
    fn show_toast(&mut self, message: &str, duration_ms: u64);
    fn clear_notifications(&mut self);
}

#[derive(Debug, Clone, Copy)]
pub enum NotificationPriority {
    Low,
    Normal,
    High,
    Urgent,
}

impl NotificationHandler {
    pub fn new<N: NotificationService + 'static>(service: N) -> Self {
        Self {
            notification_service: Arc::new(Mutex::new(service)),
            enabled_notifications: HashMap::from([
                ("email_received".to_string(), true),
                ("calendar_reminder".to_string(), true),
                ("account_error".to_string(), true),
                ("network_disconnected".to_string(), true),
            ]),
        }
    }
    
    pub fn enable_notification(&mut self, notification_type: String, enabled: bool) {
        self.enabled_notifications.insert(notification_type, enabled);
    }
    
    fn is_notification_enabled(&self, notification_type: &str) -> bool {
        self.enabled_notifications.get(notification_type).copied().unwrap_or(false)
    }
}

impl EventHandler<EmailEventData> for NotificationHandler {
    fn handle(&mut self, event: &EmailEventData) -> Result<(), EventError> {
        match &event.event {
            EmailEvent::EmailReceived { account_id, .. } => {
                if self.is_notification_enabled("email_received") {
                    let mut service = self.notification_service.lock().unwrap();
                    service.show_notification(
                        "New Email",
                        &format!("New email received in account {}", account_id),
                        NotificationPriority::Normal,
                    );
                }
            }
            EmailEvent::FolderSynced { account_id, folder_path, message_count } => {
                if message_count > &0 {
                    let mut service = self.notification_service.lock().unwrap();
                    service.show_toast(
                        &format!("Synced {} messages from {}/{}", message_count, account_id, folder_path),
                        3000,
                    );
                }
            }
            _ => {}
        }
        
        Ok(())
    }
    
    fn priority(&self) -> HandlerPriority {
        HandlerPriority::Normal
    }
}

impl EventHandler<CalendarEventData> for NotificationHandler {
    fn handle(&mut self, event: &CalendarEventData) -> Result<(), EventError> {
        match &event.event {
            CalendarEvent::ReminderTriggered { event_id, minutes_before } => {
                if self.is_notification_enabled("calendar_reminder") {
                    let mut service = self.notification_service.lock().unwrap();
                    service.show_notification(
                        "Calendar Reminder",
                        &format!("Event {} starts in {} minutes", event_id, minutes_before),
                        NotificationPriority::High,
                    );
                }
            }
            CalendarEvent::InvitationReceived { event_id, from } => {
                let mut service = self.notification_service.lock().unwrap();
                service.show_notification(
                    "Calendar Invitation",
                    &format!("Invitation to {} from {}", event_id, from),
                    NotificationPriority::Normal,
                );
            }
            _ => {}
        }
        
        Ok(())
    }
    
    fn priority(&self) -> HandlerPriority {
        HandlerPriority::Normal
    }
}

impl EventHandler<NetworkEventData> for NotificationHandler {
    fn handle(&mut self, event: &NetworkEventData) -> Result<(), EventError> {
        match &event.event {
            NetworkEvent::NetworkDisconnected => {
                if self.is_notification_enabled("network_disconnected") {
                    let mut service = self.notification_service.lock().unwrap();
                    service.show_notification(
                        "Network Disconnected",
                        "Lost network connection - working offline",
                        NotificationPriority::High,
                    );
                }
            }
            NetworkEvent::NetworkConnected => {
                let mut service = self.notification_service.lock().unwrap();
                service.show_toast("Network reconnected", 2000);
            }
            NetworkEvent::ServerError { server, error } => {
                let mut service = self.notification_service.lock().unwrap();
                service.show_notification(
                    "Server Error",
                    &format!("Error connecting to {}: {}", server, error),
                    NotificationPriority::High,
                );
            }
            _ => {}
        }
        
        Ok(())
    }
    
    fn priority(&self) -> HandlerPriority {
        HandlerPriority::High
    }
}

// =============================================================================
// Analytics Handler - Collects usage analytics and metrics
// =============================================================================

/// Handles events for analytics and usage tracking
pub struct AnalyticsHandler {
    metrics: Arc<Mutex<UsageMetrics>>,
    session_id: Uuid,
    enabled: bool,
}

#[derive(Debug, Default)]
pub struct UsageMetrics {
    pub session_events: HashMap<String, u64>,
    pub ui_interactions: u64,
    pub email_operations: u64,
    pub calendar_operations: u64,
    pub errors_encountered: u64,
    pub performance_issues: u64,
}

impl AnalyticsHandler {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(Mutex::new(UsageMetrics::default())),
            session_id: Uuid::new_v4(),
            enabled: true,
        }
    }
    
    pub fn enable(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
    
    pub fn get_metrics(&self) -> UsageMetrics {
        let metrics = self.metrics.lock().unwrap();
        UsageMetrics {
            session_events: metrics.session_events.clone(),
            ui_interactions: metrics.ui_interactions,
            email_operations: metrics.email_operations,
            calendar_operations: metrics.calendar_operations,
            errors_encountered: metrics.errors_encountered,
            performance_issues: metrics.performance_issues,
        }
    }
    
    pub fn get_session_id(&self) -> Uuid {
        self.session_id
    }
    
    fn track_event(&self, event_type: &str) {
        if !self.enabled {
            return;
        }
        
        let mut metrics = self.metrics.lock().unwrap();
        *metrics.session_events.entry(event_type.to_string()).or_insert(0) += 1;
    }
}

impl Default for AnalyticsHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl EventHandler<UIEventData> for AnalyticsHandler {
    fn handle(&mut self, event: &UIEventData) -> Result<(), EventError> {
        self.track_event(&format!("ui_{:?}", event.event));
        
        let mut metrics = self.metrics.lock().unwrap();
        metrics.ui_interactions += 1;
        
        Ok(())
    }
    
    fn priority(&self) -> HandlerPriority {
        HandlerPriority::Low
    }
}

impl EventHandler<EmailEventData> for AnalyticsHandler {
    fn handle(&mut self, event: &EmailEventData) -> Result<(), EventError> {
        self.track_event(&format!("email_{:?}", event.event));
        
        let mut metrics = self.metrics.lock().unwrap();
        metrics.email_operations += 1;
        
        Ok(())
    }
    
    fn priority(&self) -> HandlerPriority {
        HandlerPriority::Low
    }
}

impl EventHandler<CalendarEventData> for AnalyticsHandler {
    fn handle(&mut self, event: &CalendarEventData) -> Result<(), EventError> {
        self.track_event(&format!("calendar_{:?}", event.event));
        
        let mut metrics = self.metrics.lock().unwrap();
        metrics.calendar_operations += 1;
        
        Ok(())
    }
    
    fn priority(&self) -> HandlerPriority {
        HandlerPriority::Low
    }
}

// =============================================================================
// Error Recovery Handler - Handles error events and recovery strategies
// =============================================================================

/// Handles error events and implements recovery strategies
pub struct ErrorRecoveryHandler {
    retry_attempts: Arc<Mutex<HashMap<String, u32>>>,
    max_retries: u32,
    recovery_strategies: HashMap<String, Box<dyn Fn(&dyn Event) -> RecoveryAction + Send + Sync>>,
}

#[derive(Debug, Clone)]
pub enum RecoveryAction {
    Retry,
    Ignore,
    Fallback(String),
    UserIntervention(String),
}

impl ErrorRecoveryHandler {
    pub fn new(max_retries: u32) -> Self {
        Self {
            retry_attempts: Arc::new(Mutex::new(HashMap::new())),
            max_retries,
            recovery_strategies: HashMap::new(),
        }
    }
    
    pub fn add_strategy<F>(&mut self, error_type: String, strategy: F)
    where
        F: Fn(&dyn Event) -> RecoveryAction + Send + Sync + 'static,
    {
        self.recovery_strategies.insert(error_type, Box::new(strategy));
    }
    
    fn handle_error(&mut self, event: &dyn Event, error_type: &str) -> RecoveryAction {
        let event_id = event.metadata().id.to_string();
        
        let retry_count = {
            let mut attempts = self.retry_attempts.lock().unwrap();
            let count = attempts.entry(event_id.clone()).or_insert(0);
            *count += 1;
            *count
        };
        
        if retry_count > self.max_retries {
            tracing::error!("Max retries exceeded for {}: {}", error_type, event_id);
            return RecoveryAction::UserIntervention(
                format!("Failed to process {} after {} attempts", error_type, retry_count)
            );
        }
        
        if let Some(strategy) = self.recovery_strategies.get(error_type) {
            strategy(event)
        } else {
            if retry_count <= self.max_retries {
                RecoveryAction::Retry
            } else {
                RecoveryAction::Ignore
            }
        }
    }
}

impl Default for ErrorRecoveryHandler {
    fn default() -> Self {
        Self::new(3)
    }
}

impl EventHandler<NetworkEventData> for ErrorRecoveryHandler {
    fn handle(&mut self, event: &NetworkEventData) -> Result<(), EventError> {
        match &event.event {
            NetworkEvent::ServerError { server, error: _ } => {
                let recovery = self.handle_error(event, "server_error");
                
                match recovery {
                    RecoveryAction::Retry => {
                        tracing::info!("Retrying connection to {}", server);
                        // Would trigger retry logic here
                    }
                    RecoveryAction::Fallback(fallback) => {
                        tracing::info!("Using fallback strategy for {}: {}", server, fallback);
                    }
                    RecoveryAction::UserIntervention(message) => {
                        tracing::error!("User intervention required: {}", message);
                        // Would show error dialog here
                    }
                    RecoveryAction::Ignore => {
                        tracing::debug!("Ignoring error for {}", server);
                    }
                }
            }
            NetworkEvent::ServerTimeout { server, .. } => {
                tracing::warn!("Server timeout for {}, implementing recovery", server);
                let _recovery = self.handle_error(event, "server_timeout");
            }
            _ => {}
        }
        
        Ok(())
    }
    
    fn priority(&self) -> HandlerPriority {
        HandlerPriority::Critical
    }
}

// =============================================================================
// Mock Notification Service for Testing
// =============================================================================

#[derive(Debug, Default)]
pub struct MockNotificationService {
    notifications: Vec<(String, String, NotificationPriority)>,
    toasts: Vec<(String, u64)>,
}

impl MockNotificationService {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn get_notifications(&self) -> &[(String, String, NotificationPriority)] {
        &self.notifications
    }
    
    pub fn get_toasts(&self) -> &[(String, u64)] {
        &self.toasts
    }
}

impl NotificationService for MockNotificationService {
    fn show_notification(&mut self, title: &str, message: &str, priority: NotificationPriority) {
        self.notifications.push((title.to_string(), message.to_string(), priority));
    }
    
    fn show_toast(&mut self, message: &str, duration_ms: u64) {
        self.toasts.push((message.to_string(), duration_ms));
    }
    
    fn clear_notifications(&mut self) {
        self.notifications.clear();
        self.toasts.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ui_state_handler() {
        let mut handler = UIStateHandler::new();
        
        assert_eq!(handler.get_current_pane(), FocusedPane::MessageList);
        
        let event = events::pane_changed(FocusedPane::MessageList, FocusedPane::Calendar);
        assert!(handler.handle(&event).is_ok());
        
        assert_eq!(handler.get_current_pane(), FocusedPane::Calendar);
    }
    
    #[test]
    fn test_notification_handler() {
        let service = MockNotificationService::new();
        let mut handler = NotificationHandler::new(service);
        
        let event = events::email_received("account1".to_string(), Uuid::new_v4());
        assert!(handler.handle(&event).is_ok());
        
        // Test that notification was triggered
        let service = handler.notification_service.lock().unwrap();
        let mock_service = service.as_any().downcast_ref::<MockNotificationService>().unwrap();
        assert_eq!(mock_service.get_notifications().len(), 1);
    }
    
    #[test]
    fn test_analytics_handler() {
        let mut handler = AnalyticsHandler::new();
        
        let event = events::pane_changed(FocusedPane::MessageList, FocusedPane::Calendar);
        assert!(handler.handle(&event).is_ok());
        
        let metrics = handler.get_metrics();
        assert_eq!(metrics.ui_interactions, 1);
        assert!(!metrics.session_events.is_empty());
    }
}

// Helper trait for downcasting in tests
#[allow(dead_code)]
trait AsAny {
    fn as_any(&self) -> &dyn std::any::Any;
}


impl<T: NotificationService + 'static> AsAny for T {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}