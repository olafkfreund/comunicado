//! Standardized Event Types for Comunicado
//!
//! This module defines all the event types used throughout the application,
//! organized by domain (UI, Business Logic, System) with clear hierarchies
//! and consistent patterns.

use crate::events::bus::{Event, EventMetadata, EventPriority};
use std::collections::HashMap;
use uuid::Uuid;

// =============================================================================
// UI Events - User interface interactions and state changes
// =============================================================================

/// UI events for interface state changes and user interactions
#[derive(Debug, Clone)]
pub enum UIEvent {
    // Navigation events
    PaneChanged {
        from: FocusedPane,
        to: FocusedPane,
    },
    ModeChanged {
        from: UIMode,
        to: UIMode,
    },
    ViewChanged {
        view: ViewType,
    },

    // Input events
    KeyPressed {
        key: KeyEventData,
    },
    MouseClicked {
        position: (u16, u16),
        button: MouseButton,
    },

    // Component state events
    ComponentFocused {
        component_id: String,
    },
    ComponentBlurred {
        component_id: String,
    },
    ComponentResized {
        component_id: String,
        new_size: (u16, u16),
    },

    // Window events
    WindowResized {
        new_size: (u16, u16),
    },
    WindowMinimized,
    WindowRestored,

    // Theme events
    ThemeChanged {
        theme_name: String,
    },
    ColorSchemeChanged {
        scheme: ColorScheme,
    },
}

#[derive(Debug, Clone)]
pub struct UIEventData {
    pub metadata: EventMetadata,
    pub event: UIEvent,
}

impl UIEventData {
    pub fn new(event: UIEvent) -> Self {
        Self {
            metadata: EventMetadata::new(EventPriority::High, "ui".to_string()),
            event,
        }
    }
}

impl Event for UIEventData {
    fn event_type(&self) -> &'static str {
        "UIEvent"
    }

    fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }
}

// =============================================================================
// Business Events - Core application logic and data operations
// =============================================================================

/// Email-related business events
#[derive(Debug, Clone)]
pub enum EmailEvent {
    // Email lifecycle
    EmailReceived {
        account_id: String,
        email_id: Uuid,
    },
    EmailSent {
        account_id: String,
        email_id: Uuid,
    },
    EmailDeleted {
        account_id: String,
        email_id: Uuid,
    },
    EmailArchived {
        account_id: String,
        email_id: Uuid,
    },
    EmailMarkedRead {
        account_id: String,
        email_id: Uuid,
    },
    EmailMarkedUnread {
        account_id: String,
        email_id: Uuid,
    },
    EmailFlagged {
        account_id: String,
        email_id: Uuid,
    },
    EmailUnflagged {
        account_id: String,
        email_id: Uuid,
    },

    // Email operations
    EmailComposed {
        draft_id: Uuid,
    },
    EmailReplied {
        original_id: Uuid,
        reply_id: Uuid,
    },
    EmailForwarded {
        original_id: Uuid,
        forward_id: Uuid,
    },

    // Folder operations
    FolderChanged {
        account_id: String,
        folder_path: String,
    },
    FolderSynced {
        account_id: String,
        folder_path: String,
        message_count: usize,
    },
    FolderCreated {
        account_id: String,
        folder_path: String,
    },
    FolderDeleted {
        account_id: String,
        folder_path: String,
    },
    FolderRenamed {
        account_id: String,
        old_path: String,
        new_path: String,
    },

    // Search events
    SearchStarted {
        query: String,
        scope: SearchScope,
    },
    SearchCompleted {
        query: String,
        results: Vec<Uuid>,
    },
    SearchFailed {
        query: String,
        error: String,
    },
}

#[derive(Debug, Clone)]
pub struct EmailEventData {
    pub metadata: EventMetadata,
    pub event: EmailEvent,
}

impl EmailEventData {
    pub fn new(event: EmailEvent) -> Self {
        Self {
            metadata: EventMetadata::new(EventPriority::Normal, "email".to_string()),
            event,
        }
    }
}

impl Event for EmailEventData {
    fn event_type(&self) -> &'static str {
        "EmailEvent"
    }

    fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }
}

/// Calendar-related business events
#[derive(Debug, Clone)]
pub enum CalendarEvent {
    // Event lifecycle
    EventCreated {
        calendar_id: String,
        event_id: String,
    },
    EventUpdated {
        calendar_id: String,
        event_id: String,
    },
    EventDeleted {
        calendar_id: String,
        event_id: String,
    },
    EventRescheduled {
        calendar_id: String,
        event_id: String,
        old_time: i64,
        new_time: i64,
    },

    // Calendar operations
    CalendarSynced {
        calendar_id: String,
        event_count: usize,
    },
    CalendarAdded {
        calendar_id: String,
        name: String,
    },
    CalendarRemoved {
        calendar_id: String,
    },

    // Invitation events
    InvitationReceived {
        event_id: String,
        from: String,
    },
    InvitationAccepted {
        event_id: String,
    },
    InvitationDeclined {
        event_id: String,
    },
    InvitationTentative {
        event_id: String,
    },

    // Reminder events
    ReminderTriggered {
        event_id: String,
        minutes_before: u32,
    },
    ReminderDismissed {
        event_id: String,
    },
    ReminderSnoozed {
        event_id: String,
        snooze_minutes: u32,
    },
}

#[derive(Debug, Clone)]
pub struct CalendarEventData {
    pub metadata: EventMetadata,
    pub event: CalendarEvent,
}

impl CalendarEventData {
    pub fn new(event: CalendarEvent) -> Self {
        Self {
            metadata: EventMetadata::new(EventPriority::Normal, "calendar".to_string()),
            event,
        }
    }
}

impl Event for CalendarEventData {
    fn event_type(&self) -> &'static str {
        "CalendarEvent"
    }

    fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }
}

/// Contact management events
#[derive(Debug, Clone)]
pub enum ContactEvent {
    ContactAdded {
        contact_id: Uuid,
        name: String,
        email: String,
    },
    ContactUpdated {
        contact_id: Uuid,
    },
    ContactDeleted {
        contact_id: Uuid,
    },
    ContactImported {
        source: String,
        count: usize,
    },
    ContactExported {
        destination: String,
        count: usize,
    },
    ContactMerged {
        primary_id: Uuid,
        merged_ids: Vec<Uuid>,
    },
}

#[derive(Debug, Clone)]
pub struct ContactEventData {
    pub metadata: EventMetadata,
    pub event: ContactEvent,
}

impl ContactEventData {
    pub fn new(event: ContactEvent) -> Self {
        Self {
            metadata: EventMetadata::new(EventPriority::Normal, "contacts".to_string()),
            event,
        }
    }
}

impl Event for ContactEventData {
    fn event_type(&self) -> &'static str {
        "ContactEvent"
    }

    fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }
}

// =============================================================================
// System Events - Infrastructure, networking, and system-level operations
// =============================================================================

/// Account management events
#[derive(Debug, Clone)]
pub enum AccountEvent {
    AccountAdded {
        account_id: String,
        provider: String,
    },
    AccountRemoved {
        account_id: String,
    },
    AccountUpdated {
        account_id: String,
    },
    AccountConnected {
        account_id: String,
    },
    AccountDisconnected {
        account_id: String,
    },
    AccountSyncStarted {
        account_id: String,
    },
    AccountSyncCompleted {
        account_id: String,
        duration_ms: u64,
    },
    AccountSyncFailed {
        account_id: String,
        error: String,
    },
    AccountAuthRefreshed {
        account_id: String,
    },
    AccountAuthFailed {
        account_id: String,
        error: String,
    },
}

#[derive(Debug, Clone)]
pub struct AccountEventData {
    pub metadata: EventMetadata,
    pub event: AccountEvent,
}

impl AccountEventData {
    pub fn new(event: AccountEvent) -> Self {
        Self {
            metadata: EventMetadata::new(EventPriority::High, "account".to_string()),
            event,
        }
    }
}

impl Event for AccountEventData {
    fn event_type(&self) -> &'static str {
        "AccountEvent"
    }

    fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }
}

/// Network and connectivity events
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    NetworkConnected,
    NetworkDisconnected,
    NetworkLatencyChanged { latency_ms: u32 },
    ServerConnected { server: String, protocol: String },
    ServerDisconnected { server: String, protocol: String },
    ServerTimeout { server: String, timeout_ms: u64 },
    ServerError { server: String, error: String },
    RateLimitHit { service: String, reset_time: i64 },
}

#[derive(Debug, Clone)]
pub struct NetworkEventData {
    pub metadata: EventMetadata,
    pub event: NetworkEvent,
}

impl NetworkEventData {
    pub fn new(event: NetworkEvent) -> Self {
        Self {
            metadata: EventMetadata::new(EventPriority::High, "network".to_string()),
            event,
        }
    }
}

impl Event for NetworkEventData {
    fn event_type(&self) -> &'static str {
        "NetworkEvent"
    }

    fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }
}

/// Application lifecycle events
#[derive(Debug, Clone)]
pub enum AppEvent {
    AppStarted {
        version: String,
        startup_time_ms: u64,
    },
    AppShuttingDown,
    AppSuspended,
    AppResumed,
    ConfigLoaded {
        config_path: String,
    },
    ConfigSaved {
        config_path: String,
    },
    ConfigChanged {
        setting: String,
        old_value: String,
        new_value: String,
    },
    PluginLoaded {
        plugin_name: String,
    },
    PluginUnloaded {
        plugin_name: String,
    },
    PluginError {
        plugin_name: String,
        error: String,
    },
    BackgroundTaskStarted {
        task_id: Uuid,
        task_type: String,
    },
    BackgroundTaskCompleted {
        task_id: Uuid,
        duration_ms: u64,
    },
    BackgroundTaskFailed {
        task_id: Uuid,
        error: String,
    },
}

#[derive(Debug, Clone)]
pub struct AppEventData {
    pub metadata: EventMetadata,
    pub event: AppEvent,
}

impl AppEventData {
    pub fn new(event: AppEvent) -> Self {
        Self {
            metadata: EventMetadata::new(EventPriority::Critical, "app".to_string()),
            event,
        }
    }
}

impl Event for AppEventData {
    fn event_type(&self) -> &'static str {
        "AppEvent"
    }

    fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }
}

/// Performance monitoring events
#[derive(Debug, Clone)]
pub enum PerformanceEvent {
    MemoryUsageChanged {
        usage_mb: u64,
        threshold_exceeded: bool,
    },
    CpuUsageChanged {
        usage_percent: f32,
    },
    RenderTimeChanged {
        time_ms: u32,
    },
    DatabaseQuerySlow {
        query: String,
        duration_ms: u64,
    },
    CacheHit {
        cache_type: String,
        key: String,
    },
    CacheMiss {
        cache_type: String,
        key: String,
    },
    PerformanceProfileStarted {
        profile_id: Uuid,
    },
    PerformanceProfileCompleted {
        profile_id: Uuid,
        results: HashMap<String, u64>,
    },
}

#[derive(Debug, Clone)]
pub struct PerformanceEventData {
    pub metadata: EventMetadata,
    pub event: PerformanceEvent,
}

impl PerformanceEventData {
    pub fn new(event: PerformanceEvent) -> Self {
        Self {
            metadata: EventMetadata::new(EventPriority::Low, "performance".to_string()),
            event,
        }
    }
}

impl Event for PerformanceEventData {
    fn event_type(&self) -> &'static str {
        "PerformanceEvent"
    }

    fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }
}

// =============================================================================
// Supporting Types and Enums
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FocusedPane {
    AccountList,
    FolderTree,
    MessageList,
    ContentPreview,
    Calendar,
    Contacts,
    Compose,
    Search,
    Settings,
    Help,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UIMode {
    Normal,
    Search,
    Compose,
    Calendar,
    Contacts,
    Settings,
    Help,
    Modal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewType {
    EmailList,
    EmailContent,
    CalendarMonth,
    CalendarWeek,
    CalendarDay,
    ContactList,
    ContactDetails,
    Settings,
    Help,
}

#[derive(Debug, Clone)]
pub struct KeyEventData {
    pub code: String,
    pub modifiers: Vec<String>,
    pub char: Option<char>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u8),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColorScheme {
    Light,
    Dark,
    Auto,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchScope {
    CurrentFolder,
    CurrentAccount,
    AllAccounts,
    Specific(Vec<String>), // Folder paths
}

// =============================================================================
// Event Factory Functions
// =============================================================================

/// Factory functions for creating common events
pub mod events {
    use super::*;

    // UI event factories
    pub fn pane_changed(from: FocusedPane, to: FocusedPane) -> UIEventData {
        UIEventData::new(UIEvent::PaneChanged { from, to })
    }

    pub fn mode_changed(from: UIMode, to: UIMode) -> UIEventData {
        UIEventData::new(UIEvent::ModeChanged { from, to })
    }

    pub fn theme_changed(theme_name: String) -> UIEventData {
        UIEventData::new(UIEvent::ThemeChanged { theme_name })
    }

    // Email event factories
    pub fn email_received(account_id: String, email_id: Uuid) -> EmailEventData {
        EmailEventData::new(EmailEvent::EmailReceived {
            account_id,
            email_id,
        })
    }

    pub fn email_sent(account_id: String, email_id: Uuid) -> EmailEventData {
        EmailEventData::new(EmailEvent::EmailSent {
            account_id,
            email_id,
        })
    }

    pub fn folder_synced(
        account_id: String,
        folder_path: String,
        message_count: usize,
    ) -> EmailEventData {
        EmailEventData::new(EmailEvent::FolderSynced {
            account_id,
            folder_path,
            message_count,
        })
    }

    // Calendar event factories
    pub fn event_created(calendar_id: String, event_id: String) -> CalendarEventData {
        CalendarEventData::new(CalendarEvent::EventCreated {
            calendar_id,
            event_id,
        })
    }

    pub fn invitation_received(event_id: String, from: String) -> CalendarEventData {
        CalendarEventData::new(CalendarEvent::InvitationReceived { event_id, from })
    }

    // System event factories
    pub fn account_connected(account_id: String) -> AccountEventData {
        AccountEventData::new(AccountEvent::AccountConnected { account_id })
    }

    pub fn network_disconnected() -> NetworkEventData {
        NetworkEventData::new(NetworkEvent::NetworkDisconnected)
    }

    pub fn app_started(version: String, startup_time_ms: u64) -> AppEventData {
        AppEventData::new(AppEvent::AppStarted {
            version,
            startup_time_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_event_creation() {
        let event = events::pane_changed(FocusedPane::MessageList, FocusedPane::ContentPreview);
        assert_eq!(event.event_type(), "UIEvent");
        assert_eq!(event.metadata().priority, EventPriority::High);
        assert_eq!(event.metadata().source, "ui");
    }

    #[test]
    fn test_email_event_creation() {
        let event = events::email_received("account1".to_string(), Uuid::new_v4());
        assert_eq!(event.event_type(), "EmailEvent");
        assert_eq!(event.metadata().priority, EventPriority::Normal);
        assert_eq!(event.metadata().source, "email");
    }

    #[test]
    fn test_system_event_creation() {
        let event = events::app_started("1.0.0".to_string(), 1500);
        assert_eq!(event.event_type(), "AppEvent");
        assert_eq!(event.metadata().priority, EventPriority::Critical);
        assert_eq!(event.metadata().source, "app");
    }
}
