/// Application model following TEA pattern
/// 
/// Contains all application state in a centralized, immutable structure
/// that can be updated through the update function based on messages.

use crate::tea::message::{ViewMode, ToastLevel, CalendarView, SyncStatus};
use crate::oauth2::AccountConfig;
use crate::contacts::Contact;
use crate::calendar::Event;
use crate::theme::Theme;
use std::collections::HashMap;
use chrono::{DateTime, Local, NaiveDate};
use tokio::time::{Duration, Instant};

/// Main application model containing all state
#[derive(Debug, Clone)]
pub struct Model {
    /// Application lifecycle state
    pub app_state: AppState,
    
    /// Current view mode
    pub current_view: ViewMode,
    
    /// UI state
    pub ui_state: UIState,
    
    /// Email state
    pub email_state: EmailState,
    
    /// Calendar state
    pub calendar_state: CalendarState,
    
    /// Contacts state
    pub contacts_state: ContactsState,
    
    /// Account management state
    pub account_state: AccountState,
    
    /// Background tasks state
    pub background_state: BackgroundState,
    
    /// Notification state
    pub notification_state: NotificationState,
    
    /// Application configuration
    pub config: AppConfig,
    
    /// Theme settings
    pub theme: Theme,
    
    /// Auto-sync state
    pub auto_sync: AutoSyncState,
}

/// Application lifecycle state
#[derive(Debug, Clone)]
pub struct AppState {
    /// Whether the application should quit
    pub should_quit: bool,
    
    /// Initialization state
    pub initialization: InitializationState,
    
    /// Terminal dimensions
    pub terminal_size: (u16, u16),
    
    /// Last tick time for periodic updates
    pub last_tick: Instant,
    
    /// Application start time
    pub start_time: Instant,
}

/// Initialization state tracking
#[derive(Debug, Clone)]
pub struct InitializationState {
    /// Whether initialization is complete
    pub complete: bool,
    
    /// Whether initialization is in progress
    pub in_progress: bool,
    
    /// Initialization phases and their status
    pub phases: HashMap<String, PhaseStatus>,
    
    /// Current initialization phase
    pub current_phase: Option<String>,
    
    /// Initialization error if any
    pub error: Option<String>,
}

/// Phase status for initialization tracking
#[derive(Debug, Clone)]
pub enum PhaseStatus {
    Pending,
    InProgress,
    Complete,
    Failed(String),
}

/// UI-specific state
#[derive(Debug, Clone)]
pub struct UIState {
    /// Whether sidebar is visible
    pub sidebar_visible: bool,
    
    /// Whether status bar is visible
    pub status_bar_visible: bool,
    
    /// Whether help overlay is shown
    pub help_visible: bool,
    
    /// Search bar state
    pub search: SearchState,
    
    /// Toast notifications
    pub toasts: Vec<Toast>,
    
    /// Context menu state
    pub context_menu: Option<ContextMenu>,
    
    /// Modal dialogs
    pub modal: Option<Modal>,
    
    /// Layout preferences
    pub layout: LayoutState,
}

/// Search functionality state
#[derive(Debug, Clone)]
pub struct SearchState {
    /// Whether search is active
    pub active: bool,
    
    /// Current search query
    pub query: String,
    
    /// Search results count
    pub results_count: Option<usize>,
    
    /// Whether search is loading
    pub loading: bool,
}

/// Toast notification
#[derive(Debug, Clone)]
pub struct Toast {
    pub id: String,
    pub message: String,
    pub level: ToastLevel,
    pub created_at: Instant,
    pub duration: Duration,
}

/// Context menu state
#[derive(Debug, Clone)]
pub struct ContextMenu {
    pub menu_type: crate::tea::message::ContextMenuType,
    pub position: (u16, u16),
    pub items: Vec<ContextMenuItem>,
    pub selected_index: usize,
}

/// Context menu item
#[derive(Debug, Clone)]
pub struct ContextMenuItem {
    pub label: String,
    pub shortcut: Option<String>,
    pub enabled: bool,
    pub action: String, // Action identifier
}

/// Modal dialog state
#[derive(Debug, Clone)]
pub struct Modal {
    pub modal_type: ModalType,
    pub title: String,
    pub content: String,
    pub buttons: Vec<ModalButton>,
    pub selected_button: usize,
}

/// Modal dialog types
#[derive(Debug, Clone)]
pub enum ModalType {
    Confirmation,
    Error,
    Info,
    Input(String), // Input value
}

/// Modal button
#[derive(Debug, Clone)]
pub struct ModalButton {
    pub label: String,
    pub action: String,
    pub is_default: bool,
}

/// Layout state and preferences
#[derive(Debug, Clone)]
pub struct LayoutState {
    /// Sidebar width percentage
    pub sidebar_width: u16,
    
    /// Status bar position
    pub status_bar_position: StatusBarPosition,
    
    /// Panel splits for different views
    pub panel_splits: HashMap<ViewMode, Vec<u16>>,
}

/// Status bar position
#[derive(Debug, Clone)]
pub enum StatusBarPosition {
    Top,
    Bottom,
    Hidden,
}

/// Email-specific state
#[derive(Debug, Clone)]
pub struct EmailState {
    /// Current folder
    pub current_folder: Option<String>,
    
    /// Loaded messages
    pub messages: Vec<crate::email::EmailMessage>,
    
    /// Selected message ID
    pub selected_message: Option<String>,
    
    /// Currently reading message ID
    pub reading_message: Option<String>,
    
    /// Folder tree state
    pub folder_tree: FolderTreeState,
    
    /// Compose state
    pub compose: Option<ComposeState>,
    
    /// Loading state
    pub loading: bool,
    
    /// Last sync time
    pub last_sync: Option<DateTime<Local>>,
    
    /// Sync status
    pub sync_status: SyncStatus,
}

/// Folder tree UI state
#[derive(Debug, Clone)]
pub struct FolderTreeState {
    /// Expanded folders
    pub expanded: HashMap<String, bool>,
    
    /// Selected folder
    pub selected: Option<String>,
    
    /// Folder unread counts
    pub unread_counts: HashMap<String, u32>,
    
    /// Folder message counts
    pub message_counts: HashMap<String, u32>,
}

/// Email composition state
#[derive(Debug, Clone)]
pub struct ComposeState {
    pub to: String,
    pub cc: String,
    pub bcc: String,
    pub subject: String,
    pub body: String,
    pub in_reply_to: Option<String>,
    pub attachments: Vec<String>,
    pub current_field: ComposeField,
}

/// Email compose fields
#[derive(Debug, Clone)]
pub enum ComposeField {
    To,
    Cc,
    Bcc,
    Subject,
    Body,
}

/// Calendar-specific state
#[derive(Debug, Clone)]
pub struct CalendarState {
    /// Current view mode
    pub view: CalendarView,
    
    /// Current viewing date
    pub current_date: NaiveDate,
    
    /// Loaded events
    pub events: Vec<Event>,
    
    /// Selected event ID
    pub selected_event: Option<String>,
    
    /// Event creation/editing state
    pub editing_event: Option<EventEditState>,
    
    /// Calendar visibility
    pub visible_calendars: HashMap<String, bool>,
    
    /// Loading state
    pub loading: bool,
    
    /// Last sync time
    pub last_sync: Option<DateTime<Local>>,
    
    /// Sync status
    pub sync_status: SyncStatus,
}

/// Event editing state
#[derive(Debug, Clone)]
pub struct EventEditState {
    pub event_id: Option<String>, // None for new event
    pub title: String,
    pub description: String,
    pub location: String,
    pub start_time: DateTime<Local>,
    pub end_time: DateTime<Local>,
    pub all_day: bool,
    pub calendar_id: String,
    pub current_field: EventField,
}

/// Event editing fields
#[derive(Debug, Clone)]
pub enum EventField {
    Title,
    Description,
    Location,
    StartTime,
    EndTime,
    Calendar,
}

/// Contacts-specific state
#[derive(Debug, Clone)]
pub struct ContactsState {
    /// Loaded contacts
    pub contacts: Vec<Contact>,
    
    /// Selected contact ID
    pub selected_contact: Option<String>,
    
    /// Contact editing state
    pub editing_contact: Option<ContactEditState>,
    
    /// Loading state
    pub loading: bool,
    
    /// Last sync time
    pub last_sync: Option<DateTime<Local>>,
    
    /// Sync status
    pub sync_status: SyncStatus,
}

/// Contact editing state
#[derive(Debug, Clone)]
pub struct ContactEditState {
    pub contact_id: Option<String>, // None for new contact
    pub name: String,
    pub email: String,
    pub phone: String,
    pub organization: String,
    pub notes: String,
    pub current_field: ContactField,
}

/// Contact editing fields
#[derive(Debug, Clone)]
pub enum ContactField {
    Name,
    Email,
    Phone,
    Organization,
    Notes,
}

/// Account management state
#[derive(Debug, Clone)]
pub struct AccountState {
    /// Configured accounts
    pub accounts: Vec<AccountConfig>,
    
    /// Account sync status
    pub sync_status: HashMap<String, SyncStatus>,
    
    /// Active account for operations
    pub active_account: Option<String>,
    
    /// Account loading state
    pub loading: bool,
}

/// Background tasks state
#[derive(Debug, Clone)]
pub struct BackgroundState {
    /// Running tasks
    pub tasks: HashMap<String, TaskState>,
    
    /// Task queue
    pub queue: Vec<String>,
    
    /// Overall processing state
    pub processing: bool,
}

/// Individual task state
#[derive(Debug, Clone)]
pub struct TaskState {
    pub name: String,
    pub started_at: Instant,
    pub progress: Option<(u32, u32)>, // current, total
    pub status: TaskStatus,
}

/// Task execution status
#[derive(Debug, Clone)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed(String),
}

/// Notification system state
#[derive(Debug, Clone)]
pub struct NotificationState {
    /// Pending notifications
    pub notifications: Vec<Notification>,
    
    /// Notification settings
    pub settings: NotificationSettings,
}

/// Individual notification
#[derive(Debug, Clone)]
pub struct Notification {
    pub id: String,
    pub title: String,
    pub body: String,
    pub level: ToastLevel,
    pub created_at: Instant,
    pub read: bool,
}

/// Notification preferences
#[derive(Debug, Clone)]
pub struct NotificationSettings {
    pub desktop_enabled: bool,
    pub sound_enabled: bool,
    pub email_notifications: bool,
    pub calendar_notifications: bool,
}

/// Application configuration
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Auto-sync interval in minutes
    pub auto_sync_interval: u64,
    
    /// Maximum number of messages to load per folder
    pub max_messages_per_folder: usize,
    
    /// Theme name
    pub theme_name: String,
    
    /// Keyboard shortcuts
    pub shortcuts: HashMap<String, String>,
    
    /// UI preferences
    pub ui_preferences: UIPreferences,
}

/// UI-specific preferences
#[derive(Debug, Clone)]
pub struct UIPreferences {
    /// Show line numbers in email list
    pub show_line_numbers: bool,
    
    /// Date format
    pub date_format: String,
    
    /// Time format
    pub time_format: String,
    
    /// Show unread counts
    pub show_unread_counts: bool,
    
    /// Auto-mark as read delay (seconds)
    pub auto_mark_read_delay: u64,
}

/// Auto-sync state tracking
#[derive(Debug, Clone)]
pub struct AutoSyncState {
    /// Last auto-sync time
    pub last_sync: Instant,
    
    /// Auto-sync interval
    pub interval: Duration,
    
    /// Whether auto-sync is enabled
    pub enabled: bool,
    
    /// Auto-sync status
    pub status: SyncStatus,
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}

impl Model {
    /// Create a new model with default values
    pub fn new() -> Self {
        let now = Instant::now();
        
        Self {
            app_state: AppState {
                should_quit: false,
                initialization: InitializationState {
                    complete: false,
                    in_progress: false,
                    phases: HashMap::new(),
                    current_phase: None,
                    error: None,
                },
                terminal_size: (80, 24),
                last_tick: now,
                start_time: now,
            },
            current_view: ViewMode::Email,
            ui_state: UIState {
                sidebar_visible: true,
                status_bar_visible: true,
                help_visible: false,
                search: SearchState {
                    active: false,
                    query: String::new(),
                    results_count: None,
                    loading: false,
                },
                toasts: Vec::new(),
                context_menu: None,
                modal: None,
                layout: LayoutState {
                    sidebar_width: 25,
                    status_bar_position: StatusBarPosition::Bottom,
                    panel_splits: HashMap::new(),
                },
            },
            email_state: EmailState {
                current_folder: None,
                messages: Vec::new(),
                selected_message: None,
                reading_message: None,
                folder_tree: FolderTreeState {
                    expanded: HashMap::new(),
                    selected: None,
                    unread_counts: HashMap::new(),
                    message_counts: HashMap::new(),
                },
                compose: None,
                loading: false,
                last_sync: None,
                sync_status: SyncStatus::Idle,
            },
            calendar_state: CalendarState {
                view: CalendarView::Month,
                current_date: chrono::Local::now().date_naive(),
                events: Vec::new(),
                selected_event: None,
                editing_event: None,
                visible_calendars: HashMap::new(),
                loading: false,
                last_sync: None,
                sync_status: SyncStatus::Idle,
            },
            contacts_state: ContactsState {
                contacts: Vec::new(),
                selected_contact: None,
                editing_contact: None,
                loading: false,
                last_sync: None,
                sync_status: SyncStatus::Idle,
            },
            account_state: AccountState {
                accounts: Vec::new(),
                sync_status: HashMap::new(),
                active_account: None,
                loading: false,
            },
            background_state: BackgroundState {
                tasks: HashMap::new(),
                queue: Vec::new(),
                processing: false,
            },
            notification_state: NotificationState {
                notifications: Vec::new(),
                settings: NotificationSettings {
                    desktop_enabled: true,
                    sound_enabled: false,
                    email_notifications: true,
                    calendar_notifications: true,
                },
            },
            config: AppConfig {
                auto_sync_interval: 3,
                max_messages_per_folder: 100,
                theme_name: "gruvbox_dark".to_string(),
                shortcuts: HashMap::new(),
                ui_preferences: UIPreferences {
                    show_line_numbers: false,
                    date_format: "%Y-%m-%d".to_string(),
                    time_format: "%H:%M".to_string(),
                    show_unread_counts: true,
                    auto_mark_read_delay: 2,
                },
            },
            theme: Theme::gruvbox_dark(),
            auto_sync: AutoSyncState {
                last_sync: now,
                interval: Duration::from_secs(3 * 60), // 3 minutes
                enabled: true,
                status: SyncStatus::Idle,
            },
        }
    }
    
    /// Check if the application should quit
    pub fn should_quit(&self) -> bool {
        self.app_state.should_quit
    }
    
    /// Check if initialization is complete
    pub fn is_initialized(&self) -> bool {
        self.app_state.initialization.complete
    }
    
    /// Get current view mode
    pub fn current_view(&self) -> ViewMode {
        self.current_view
    }
}