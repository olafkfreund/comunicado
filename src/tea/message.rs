/// Central message type for the entire application following TEA pattern
/// 
/// All user interactions, system events, and async operation results
/// flow through this message system for centralized state management.

use crate::cli::StartupMode;
use crate::oauth2::AccountConfig;
use crate::contacts::Contact;
use crate::calendar::Event;
use crossterm::event::{KeyEvent, MouseEvent};

/// Main application message type
#[derive(Debug, Clone)]
pub enum Message {
    /// System-level messages
    System(SystemMessage),
    
    /// UI interaction messages
    UI(UIMessage),
    
    /// Email-related messages
    Email(EmailMessage),
    
    /// Calendar-related messages
    Calendar(CalendarMessage),
    
    /// Contacts-related messages
    Contacts(ContactsMessage),
    
    /// Account management messages
    Account(AccountMessage),
    
    /// Background task messages
    Background(BackgroundMessage),
    
    /// Notification messages
    Notification(NotificationMessage),
}

/// System-level messages for application lifecycle
#[derive(Debug, Clone)]
pub enum SystemMessage {
    /// Application should quit
    Quit,
    
    /// Initialize application with startup mode
    Initialize(StartupMode),
    
    /// Initialization completed
    InitializationComplete,
    
    /// Initialization failed with error
    InitializationFailed(String),
    
    /// Resize terminal
    Resize(u16, u16),
    
    /// Tick for periodic updates
    Tick,
    
    /// Request auto-sync
    AutoSync,
}

/// UI-related messages for interface interactions
#[derive(Debug, Clone)]
pub enum UIMessage {
    /// Keyboard input
    KeyPressed(KeyEvent),
    
    /// Mouse input
    MouseEvent(MouseEvent),
    
    /// Navigate to different view
    Navigate(ViewMode),
    
    /// Toggle UI element
    Toggle(ToggleTarget),
    
    /// Show/hide help overlay
    ToggleHelp,
    
    /// Show context menu
    ShowContextMenu(ContextMenuType),
    
    /// Hide context menu
    HideContextMenu,
    
    /// Show toast notification
    ShowToast(String, ToastLevel),
    
    /// Search query changed
    SearchChanged(String),
    
    /// Search submitted
    SearchSubmit,
    
    /// Clear search
    SearchClear,
}

/// Email-specific messages
#[derive(Debug, Clone)]
pub enum EmailMessage {
    /// Load messages for folder
    LoadMessages(String),
    
    /// Messages loaded successfully
    MessagesLoaded(Vec<crate::email::EmailMessage>),
    
    /// Message loading failed
    LoadingFailed(String),
    
    /// Select message
    SelectMessage(String),
    
    /// Open message for reading
    OpenMessage(String),
    
    /// Compose new message
    ComposeNew,
    
    /// Reply to message
    Reply(String),
    
    /// Forward message
    Forward(String),
    
    /// Delete message
    Delete(String),
    
    /// Mark as read/unread
    ToggleRead(String),
    
    /// Flag message
    ToggleFlag(String),
    
    /// Move message to folder
    MoveToFolder(String, String),
    
    /// Sync folder
    SyncFolder(String),
    
    /// Sync all folders
    SyncAll,
    
    /// Search in messages
    Search(String),
}

/// Calendar-specific messages
#[derive(Debug, Clone)]
pub enum CalendarMessage {
    /// Load events for date range
    LoadEvents(chrono::NaiveDate, chrono::NaiveDate),
    
    /// Events loaded successfully
    EventsLoaded(Vec<Event>),
    
    /// Event loading failed
    LoadingFailed(String),
    
    /// Select event
    SelectEvent(String),
    
    /// Create new event
    CreateEvent,
    
    /// Edit event
    EditEvent(String),
    
    /// Delete event
    DeleteEvent(String),
    
    /// Change calendar view
    ChangeView(CalendarView),
    
    /// Navigate to date
    NavigateToDate(chrono::NaiveDate),
    
    /// Sync calendar
    SyncCalendar,
    
    /// RSVP to event
    RSVP(String, RsvpResponse),
}

/// Contacts-specific messages
#[derive(Debug, Clone)]
pub enum ContactsMessage {
    /// Load all contacts
    LoadContacts,
    
    /// Contacts loaded successfully
    ContactsLoaded(Vec<Contact>),
    
    /// Contact loading failed
    LoadingFailed(String),
    
    /// Select contact
    SelectContact(String),
    
    /// Create new contact
    CreateContact,
    
    /// Edit contact
    EditContact(String),
    
    /// Delete contact
    DeleteContact(String),
    
    /// Search contacts
    Search(String),
    
    /// Sync contacts
    SyncContacts,
}

/// Account management messages
#[derive(Debug, Clone)]
pub enum AccountMessage {
    /// Load accounts
    LoadAccounts,
    
    /// Accounts loaded
    AccountsLoaded(Vec<AccountConfig>),
    
    /// Add new account
    AddAccount,
    
    /// Remove account
    RemoveAccount(String),
    
    /// Refresh account tokens
    RefreshTokens(String),
    
    /// Account sync status changed
    SyncStatusChanged(String, SyncStatus),
}

/// Background task messages
#[derive(Debug, Clone)]
pub enum BackgroundMessage {
    /// Task started
    TaskStarted(String),
    
    /// Task completed successfully
    TaskCompleted(String),
    
    /// Task failed with error
    TaskFailed(String, String),
    
    /// Task progress update
    TaskProgress(String, u32, u32),
}

/// Notification messages
#[derive(Debug, Clone)]
pub enum NotificationMessage {
    /// New email notification
    NewEmail(String, String), // sender, subject
    
    /// Calendar reminder
    CalendarReminder(String, String), // event title, time
    
    /// System notification
    System(String),
    
    /// Error notification
    Error(String),
    
    /// Success notification
    Success(String),
}

/// Application view modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Email,
    Calendar,
    Contacts,
    Settings,
}

/// UI toggleable elements
#[derive(Debug, Clone, Copy)]
pub enum ToggleTarget {
    Sidebar,
    StatusBar,
    HelpOverlay,
    SearchBar,
    FilterPanel,
}

/// Context menu types
#[derive(Debug, Clone, Copy)]
pub enum ContextMenuType {
    Message,
    Folder,
    Event,
    Contact,
    Account,
}

/// Toast notification levels
#[derive(Debug, Clone, Copy)]
pub enum ToastLevel {
    Info,
    Success,
    Warning,
    Error,
}

/// Calendar view types
#[derive(Debug, Clone, Copy)]
pub enum CalendarView {
    Day,
    Week,
    Month,
    Agenda,
}

/// RSVP response types
#[derive(Debug, Clone, Copy)]
pub enum RsvpResponse {
    Accept,
    Decline,
    Tentative,
}

/// Sync status types
#[derive(Debug, Clone, Copy)]
pub enum SyncStatus {
    Idle,
    Syncing,
    Success,
    Error,
}

impl From<SystemMessage> for Message {
    fn from(msg: SystemMessage) -> Self {
        Message::System(msg)
    }
}

impl From<UIMessage> for Message {
    fn from(msg: UIMessage) -> Self {
        Message::UI(msg)
    }
}

impl From<EmailMessage> for Message {
    fn from(msg: EmailMessage) -> Self {
        Message::Email(msg)
    }
}

impl From<CalendarMessage> for Message {
    fn from(msg: CalendarMessage) -> Self {
        Message::Calendar(msg)
    }
}

impl From<ContactsMessage> for Message {
    fn from(msg: ContactsMessage) -> Self {
        Message::Contacts(msg)
    }
}

impl From<AccountMessage> for Message {
    fn from(msg: AccountMessage) -> Self {
        Message::Account(msg)
    }
}

impl From<BackgroundMessage> for Message {
    fn from(msg: BackgroundMessage) -> Self {
        Message::Background(msg)
    }
}

impl From<NotificationMessage> for Message {
    fn from(msg: NotificationMessage) -> Self {
        Message::Notification(msg)
    }
}