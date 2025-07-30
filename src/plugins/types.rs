//! Specialized plugin traits for different plugin types
//!
//! This module defines specific interfaces for different categories of plugins,
//! providing typed access to Comunicado's functionality.

use super::core::{Plugin, PluginResult, PluginError};
use crate::email::{StoredMessage, AttachmentInfo};
use crate::calendar::event::Event;
use crate::contacts::Contact;

use ratatui::widgets::Widget;
use ratatui::layout::Rect;
use ratatui::Frame;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::HashMap;
use tokio;

// ============================================================================
// Email Plugin Types
// ============================================================================

/// Trait for plugins that process email messages
pub trait EmailPlugin: Plugin {
    /// Process an incoming email message
    fn process_incoming_email(
        &mut self,
        message: &StoredMessage,
        context: &EmailPluginContext,
    ) -> impl std::future::Future<Output = PluginResult<EmailProcessResult>> + Send;

    /// Process an outgoing email message
    fn process_outgoing_email(
        &mut self,
        message: &StoredMessage,
        context: &EmailPluginContext,
    ) -> impl std::future::Future<Output = PluginResult<EmailProcessResult>> + Send;

    /// Filter email messages based on criteria
    fn filter_emails(
        &self,
        messages: &[StoredMessage],
        context: &EmailPluginContext,
    ) -> impl std::future::Future<Output = PluginResult<Vec<bool>>> + Send;

    /// Get supported email processing capabilities
    fn get_email_capabilities(&self) -> Vec<EmailCapability>;
}

/// Context provided to email plugins
#[derive(Debug, Clone)]
pub struct EmailPluginContext {
    /// Account ID for the email
    pub account_id: String,
    /// Folder name where the email is located
    pub folder_name: String,
    /// Additional context data
    pub context_data: HashMap<String, serde_json::Value>,
}

/// Result of email processing
#[derive(Debug, Clone)]
pub enum EmailProcessResult {
    /// Email processed successfully, no changes
    NoChange,
    /// Email was modified
    Modified(StoredMessage),
    /// Email should be moved to a different folder
    Move(String),
    /// Email should be deleted
    Delete,
    /// Email should be copied to additional folders
    Copy(Vec<String>),
    /// Email should be marked with specific flags
    SetFlags(Vec<String>),
    /// Custom processing result
    Custom(HashMap<String, serde_json::Value>),
}

/// Email processing capabilities
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmailCapability {
    /// Spam filtering
    SpamFilter,
    /// Content filtering
    ContentFilter,
    /// Auto-responder
    AutoResponder,
    /// Email forwarding
    Forwarding,
    /// Attachment processing
    AttachmentProcessing,
    /// Content analysis
    ContentAnalysis,
    /// Thread management
    ThreadManagement,
    /// Custom capability
    Custom(String),
}

// ============================================================================
// UI Plugin Types
// ============================================================================

/// Trait for plugins that extend the user interface
pub trait UIPlugin: Plugin {
    /// Render a custom UI component
    fn render_component(
        &self,
        frame: &mut Frame,
        area: Rect,
        context: &UIPluginContext,
    ) -> PluginResult<UIComponentResult>;

    /// Handle keyboard input for the plugin's UI
    fn handle_input(
        &mut self,
        key_event: crossterm::event::KeyEvent,
        context: &UIPluginContext,
    ) -> impl std::future::Future<Output = PluginResult<UIInputResult>> + Send;

    /// Get the plugin's UI layout preferences
    fn get_layout_preferences(&self) -> UILayoutPreferences;

    /// Get supported UI capabilities
    fn get_ui_capabilities(&self) -> Vec<UICapability>;
}

/// Context provided to UI plugins
#[derive(Debug, Clone)]
pub struct UIPluginContext {
    /// Current application state
    pub app_state: String,
    /// Selected items or data
    pub selected_data: Option<serde_json::Value>,
    /// Theme information
    pub theme_data: HashMap<String, serde_json::Value>,
    /// Screen dimensions
    pub screen_size: (u16, u16),
}

/// Result of UI component rendering
#[derive(Debug)]
pub enum UIComponentResult {
    /// Component rendered successfully
    Rendered,
    /// Component needs to be redrawn
    NeedsRedraw,
    /// Component requests focus
    RequestFocus,
    /// Component emits an event
    Event(String, serde_json::Value),
}

/// Result of UI input handling
#[derive(Debug)]
pub enum UIInputResult {
    /// Input was handled
    Handled,
    /// Input was not handled, pass to next handler
    NotHandled,
    /// Input triggered an action
    Action(String, serde_json::Value),
    /// Input requests view change
    ChangeView(String),
}

/// UI layout preferences for plugins
#[derive(Debug, Clone)]
pub struct UILayoutPreferences {
    /// Preferred position in the UI
    pub preferred_position: UIPosition,
    /// Minimum size requirements
    pub min_size: (u16, u16),
    /// Maximum size constraints
    pub max_size: Option<(u16, u16)>,
    /// Whether the component can be resized
    pub resizable: bool,
    /// Whether the component can be moved
    pub movable: bool,
}

/// UI position preferences
#[derive(Debug, Clone)]
pub enum UIPosition {
    /// Top panel
    Top,
    /// Bottom panel
    Bottom,
    /// Left sidebar
    Left,
    /// Right sidebar
    Right,
    /// Main content area
    Main,
    /// Floating/modal
    Floating,
    /// Custom position
    Custom(u16, u16),
}

/// UI capabilities
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UICapability {
    /// Custom widgets
    CustomWidgets,
    /// Keyboard shortcuts
    KeyboardShortcuts,
    /// Theming support
    Theming,
    /// Layout modification
    LayoutModification,
    /// Event handling
    EventHandling,
    /// Custom capability
    Custom(String),
}

// ============================================================================
// Calendar Plugin Types
// ============================================================================

/// Trait for plugins that extend calendar functionality
pub trait CalendarPlugin: Plugin {
    /// Process a calendar event
    fn process_event(
        &mut self,
        event: &Event,
        context: &CalendarPluginContext,
    ) -> impl std::future::Future<Output = PluginResult<CalendarEventResult>> + Send;

    /// Handle calendar invitations
    fn handle_invitation(
        &mut self,
        invitation_data: &serde_json::Value,
        context: &CalendarPluginContext,
    ) -> impl std::future::Future<Output = PluginResult<CalendarEventResult>> + Send;

    /// Get available calendar sources
    fn get_calendar_sources(&self) -> impl std::future::Future<Output = PluginResult<Vec<CalendarSource>>> + Send;

    /// Sync with external calendar systems
    fn sync_calendars(
        &mut self,
        context: &CalendarPluginContext,
    ) -> impl std::future::Future<Output = PluginResult<CalendarSyncResult>> + Send;

    /// Get supported calendar capabilities
    fn get_calendar_capabilities(&self) -> Vec<CalendarCapability>;
}

/// Context provided to calendar plugins
#[derive(Debug, Clone)]
pub struct CalendarPluginContext {
    /// Calendar ID
    pub calendar_id: String,
    /// User timezone
    pub timezone: String,
    /// Additional context data
    pub context_data: HashMap<String, serde_json::Value>,
}

/// Result of calendar event processing
#[derive(Debug, Clone)]
pub enum CalendarEventResult {
    /// Event processed successfully
    Success,
    /// Event was modified
    Modified(Event),
    /// Event should be deleted
    Delete,
    /// Event should be moved to different calendar
    Move(String),
    /// Custom result
    Custom(HashMap<String, serde_json::Value>),
}

/// External calendar source
#[derive(Debug, Clone)]
pub struct CalendarSource {
    /// Source identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Source type (CalDAV, Google, etc.)
    pub source_type: String,
    /// Source URL or configuration
    pub config: HashMap<String, serde_json::Value>,
}

/// Calendar synchronization result
#[derive(Debug, Clone)]
pub struct CalendarSyncResult {
    /// Number of events synced
    pub events_synced: usize,
    /// Number of events updated
    pub events_updated: usize,
    /// Number of events deleted
    pub events_deleted: usize,
    /// Sync errors
    pub errors: Vec<String>,
}

/// Calendar capabilities
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalendarCapability {
    /// Event creation
    EventCreation,
    /// Event modification
    EventModification,
    /// External sync
    ExternalSync,
    /// Invitation handling
    InvitationHandling,
    /// Recurring events
    RecurringEvents,
    /// Reminders
    Reminders,
    /// Custom capability
    Custom(String),
}

// ============================================================================
// Notification Plugin Types
// ============================================================================

/// Trait for plugins that handle notifications
pub trait NotificationPlugin: Plugin {
    /// Send a notification
    fn send_notification(
        &mut self,
        notification: &NotificationMessage,
        context: &NotificationPluginContext,
    ) -> impl std::future::Future<Output = PluginResult<NotificationResult>> + Send;

    /// Handle notification responses
    fn handle_notification_response(
        &mut self,
        response: &NotificationResponse,
        context: &NotificationPluginContext,
    ) -> impl std::future::Future<Output = PluginResult<NotificationResult>> + Send;

    /// Get supported notification types
    fn get_supported_types(&self) -> Vec<NotificationType>;

    /// Get notification capabilities
    fn get_notification_capabilities(&self) -> Vec<NotificationCapability>;
}

/// Context provided to notification plugins
#[derive(Debug, Clone)]
pub struct NotificationPluginContext {
    /// User preferences
    pub user_preferences: HashMap<String, serde_json::Value>,
    /// Notification channel
    pub channel: String,
    /// Additional context data
    pub context_data: HashMap<String, serde_json::Value>,
}

/// Notification message
#[derive(Debug, Clone)]
pub struct NotificationMessage {
    /// Notification title
    pub title: String,
    /// Notification body
    pub body: String,
    /// Notification type
    pub notification_type: NotificationType,
    /// Priority level
    pub priority: NotificationPriority,
    /// Actions available
    pub actions: Vec<NotificationAction>,
    /// Additional data
    pub data: HashMap<String, serde_json::Value>,
}

/// Notification types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationType {
    /// New email notification
    NewEmail,
    /// Calendar reminder
    CalendarReminder,
    /// System notification
    System,
    /// Error notification
    Error,
    /// Custom notification
    Custom(String),
}

/// Notification priority levels
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Notification action
#[derive(Debug, Clone)]
pub struct NotificationAction {
    /// Action identifier
    pub id: String,
    /// Action label
    pub label: String,
    /// Action type
    pub action_type: String,
}

/// Notification response
#[derive(Debug, Clone)]
pub struct NotificationResponse {
    /// Notification ID
    pub notification_id: String,
    /// Selected action
    pub action_id: Option<String>,
    /// Response data
    pub data: HashMap<String, serde_json::Value>,
}

/// Result of notification processing
#[derive(Debug, Clone)]
pub enum NotificationResult {
    /// Notification sent successfully
    Sent,
    /// Notification was queued
    Queued,
    /// Notification failed to send
    Failed(String),
    /// Custom result
    Custom(HashMap<String, serde_json::Value>),
}

/// Notification capabilities
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationCapability {
    /// Desktop notifications
    Desktop,
    /// Email notifications
    Email,
    /// SMS notifications
    SMS,
    /// Push notifications
    Push,
    /// Sound notifications
    Sound,
    /// Custom capability
    Custom(String),
}

// ============================================================================
// Search Plugin Types
// ============================================================================

/// Trait for plugins that enhance search functionality
pub trait SearchPlugin: Plugin {
    /// Perform a search operation
    fn search(
        &self,
        query: &SearchQuery,
        context: &SearchPluginContext,
    ) -> impl std::future::Future<Output = PluginResult<SearchResult>> + Send;

    /// Index content for searching
    fn index_content(
        &mut self,
        content: &SearchableContent,
        context: &SearchPluginContext,
    ) -> impl std::future::Future<Output = PluginResult<()>> + Send;

    /// Get search suggestions
    fn get_suggestions(
        &self,
        partial_query: &str,
        context: &SearchPluginContext,
    ) -> impl std::future::Future<Output = PluginResult<Vec<String>>> + Send;

    /// Get supported search capabilities
    fn get_search_capabilities(&self) -> Vec<SearchCapability>;
}

/// Context provided to search plugins
#[derive(Debug, Clone)]
pub struct SearchPluginContext {
    /// Search scope
    pub scope: SearchScope,
    /// User preferences
    pub preferences: HashMap<String, serde_json::Value>,
    /// Additional context data
    pub context_data: HashMap<String, serde_json::Value>,
}

/// Search query structure
#[derive(Debug, Clone)]
pub struct SearchQuery {
    /// Query string
    pub query: String,
    /// Search filters
    pub filters: HashMap<String, serde_json::Value>,
    /// Sort criteria
    pub sort_by: Option<String>,
    /// Maximum results
    pub limit: Option<usize>,
    /// Result offset
    pub offset: Option<usize>,
}

/// Search scope
#[derive(Debug, Clone)]
pub enum SearchScope {
    /// Search emails
    Emails,
    /// Search contacts
    Contacts,
    /// Search calendar events
    Calendar,
    /// Search all content
    All,
    /// Custom scope
    Custom(String),
}

/// Searchable content
#[derive(Debug, Clone)]
pub struct SearchableContent {
    /// Content ID
    pub id: String,
    /// Content type
    pub content_type: String,
    /// Text content
    pub text: String,
    /// Metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Search result
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Found items
    pub items: Vec<SearchResultItem>,
    /// Total number of results
    pub total_count: usize,
    /// Search execution time
    pub execution_time: std::time::Duration,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Individual search result item
#[derive(Debug, Clone)]
pub struct SearchResultItem {
    /// Item ID
    pub id: String,
    /// Item type
    pub item_type: String,
    /// Relevance score
    pub score: f64,
    /// Highlighted text snippets
    pub snippets: Vec<String>,
    /// Item metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Search capabilities
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchCapability {
    /// Full-text search
    FullText,
    /// Fuzzy search
    Fuzzy,
    /// Faceted search
    Faceted,
    /// Advanced query syntax
    AdvancedQuery,
    /// Real-time indexing
    RealTimeIndexing,
    /// Custom capability
    Custom(String),
}

// ============================================================================
// Import/Export Plugin Types
// ============================================================================

/// Trait for plugins that handle data import and export
pub trait ImportExportPlugin: Plugin {
    /// Import data from external source
    fn import_data(
        &mut self,
        source: &ImportSource,
        context: &ImportExportPluginContext,
    ) -> impl std::future::Future<Output = PluginResult<ImportExportResult>> + Send;

    /// Export data to external destination
    fn export_data(
        &self,
        data: &ExportData,
        destination: &ExportDestination,
        context: &ImportExportPluginContext,
    ) -> impl std::future::Future<Output = PluginResult<ImportExportResult>> + Send;

    /// Get supported import formats
    fn get_supported_import_formats(&self) -> Vec<String>;

    /// Get supported export formats
    fn get_supported_export_formats(&self) -> Vec<String>;

    /// Get import/export capabilities
    fn get_import_export_capabilities(&self) -> Vec<ImportExportCapability>;
}

/// Context provided to import/export plugins
#[derive(Debug, Clone)]
pub struct ImportExportPluginContext {
    /// Operation type
    pub operation: ImportExportOperation,
    /// Progress callback
    pub progress_callback: Option<String>,
    /// Additional context data
    pub context_data: HashMap<String, serde_json::Value>,
}

/// Import/export operation type
#[derive(Debug, Clone)]
pub enum ImportExportOperation {
    Import,
    Export,
    Sync,
    Backup,
    Restore,
}

/// Import source specification
#[derive(Debug, Clone)]
pub struct ImportSource {
    /// Source type
    pub source_type: String,
    /// Source location (file path, URL, etc.)
    pub location: String,
    /// Source format
    pub format: String,
    /// Import options
    pub options: HashMap<String, serde_json::Value>,
}

/// Export destination specification
#[derive(Debug, Clone)]
pub struct ExportDestination {
    /// Destination type
    pub destination_type: String,
    /// Destination location
    pub location: String,
    /// Export format
    pub format: String,
    /// Export options
    pub options: HashMap<String, serde_json::Value>,
}

/// Data to be exported
#[derive(Debug, Clone)]
pub struct ExportData {
    /// Data type
    pub data_type: String,
    /// Data content
    pub content: serde_json::Value,
    /// Export filters
    pub filters: HashMap<String, serde_json::Value>,
}

/// Result of import/export operation
#[derive(Debug, Clone)]
pub struct ImportExportResult {
    /// Operation status
    pub status: ImportExportStatus,
    /// Number of items processed
    pub items_processed: usize,
    /// Number of items succeeded
    pub items_succeeded: usize,
    /// Number of items failed
    pub items_failed: usize,
    /// Error messages
    pub errors: Vec<String>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Import/export status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportExportStatus {
    Success,
    PartialSuccess,
    Failed,
    Cancelled,
}

/// Import/export capabilities
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportExportCapability {
    /// Email import/export
    Email,
    /// Contact import/export
    Contacts,
    /// Calendar import/export
    Calendar,
    /// Backup/restore
    Backup,
    /// Incremental sync
    IncrementalSync,
    /// Custom capability
    Custom(String),
}

// ============================================================================
// Helper Functions and Implementations
// ============================================================================

/// Convert any plugin to a specific plugin type
pub fn cast_plugin<T: 'static>(plugin: &dyn Plugin) -> Option<&T> {
    plugin.as_any().downcast_ref::<T>()
}

/// Convert any mutable plugin to a specific plugin type
pub fn cast_plugin_mut<T: 'static>(plugin: &mut dyn Plugin) -> Option<&mut T> {
    plugin.as_any_mut().downcast_mut::<T>()
}