use anyhow::{anyhow, Result};
use crossterm::event::{KeyCode, KeyModifiers};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Represents a keyboard shortcut with key and modifiers
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyboardShortcut {
    #[serde(with = "keycode_serde")]
    pub key: KeyCode,
    #[serde(with = "keymodifiers_serde")]
    pub modifiers: KeyModifiers,
}

/// Serde serialization module for KeyCode
mod keycode_serde {
    use crossterm::event::KeyCode;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(key: &KeyCode, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let key_str = match key {
            KeyCode::Char(c) => format!("Char({})", c),
            KeyCode::F(n) => format!("F{}", n),
            KeyCode::Enter => "Enter".to_string(),
            KeyCode::Esc => "Esc".to_string(),
            KeyCode::Tab => "Tab".to_string(),
            KeyCode::BackTab => "BackTab".to_string(),
            KeyCode::Backspace => "Backspace".to_string(),
            KeyCode::Delete => "Delete".to_string(),
            KeyCode::Insert => "Insert".to_string(),
            KeyCode::Home => "Home".to_string(),
            KeyCode::End => "End".to_string(),
            KeyCode::PageUp => "PageUp".to_string(),
            KeyCode::PageDown => "PageDown".to_string(),
            KeyCode::Up => "Up".to_string(),
            KeyCode::Down => "Down".to_string(),
            KeyCode::Left => "Left".to_string(),
            KeyCode::Right => "Right".to_string(),
            _ => format!("{:?}", key),
        };
        key_str.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<KeyCode, D::Error>
    where
        D: Deserializer<'de>,
    {
        let key_str = String::deserialize(deserializer)?;

        match key_str.as_str() {
            "Enter" => Ok(KeyCode::Enter),
            "Esc" => Ok(KeyCode::Esc),
            "Tab" => Ok(KeyCode::Tab),
            "BackTab" => Ok(KeyCode::BackTab),
            "Backspace" => Ok(KeyCode::Backspace),
            "Delete" => Ok(KeyCode::Delete),
            "Insert" => Ok(KeyCode::Insert),
            "Home" => Ok(KeyCode::Home),
            "End" => Ok(KeyCode::End),
            "PageUp" => Ok(KeyCode::PageUp),
            "PageDown" => Ok(KeyCode::PageDown),
            "Up" => Ok(KeyCode::Up),
            "Down" => Ok(KeyCode::Down),
            "Left" => Ok(KeyCode::Left),
            "Right" => Ok(KeyCode::Right),
            s if s.starts_with("Char(") && s.ends_with(")") => {
                let char_str = &s[5..s.len() - 1];
                if let Some(c) = char_str.chars().next() {
                    Ok(KeyCode::Char(c))
                } else {
                    Err(serde::de::Error::custom(format!(
                        "Invalid char in KeyCode: {}",
                        s
                    )))
                }
            }
            s if s.starts_with("F") && s.len() > 1 => {
                let num_str = &s[1..];
                if let Ok(num) = num_str.parse::<u8>() {
                    Ok(KeyCode::F(num))
                } else {
                    Err(serde::de::Error::custom(format!("Invalid F-key: {}", s)))
                }
            }
            _ => Err(serde::de::Error::custom(format!(
                "Unknown KeyCode: {}",
                key_str
            ))),
        }
    }
}

/// Serde serialization module for KeyModifiers
mod keymodifiers_serde {
    use crossterm::event::KeyModifiers;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(modifiers: &KeyModifiers, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut parts = Vec::new();

        if modifiers.contains(KeyModifiers::CONTROL) {
            parts.push("CONTROL");
        }
        if modifiers.contains(KeyModifiers::ALT) {
            parts.push("ALT");
        }
        if modifiers.contains(KeyModifiers::SHIFT) {
            parts.push("SHIFT");
        }

        if parts.is_empty() {
            "NONE".serialize(serializer)
        } else {
            parts.join("|").serialize(serializer)
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<KeyModifiers, D::Error>
    where
        D: Deserializer<'de>,
    {
        let modifiers_str = String::deserialize(deserializer)?;

        if modifiers_str == "NONE" {
            return Ok(KeyModifiers::NONE);
        }

        let mut modifiers = KeyModifiers::NONE;

        for part in modifiers_str.split('|') {
            match part {
                "CONTROL" => modifiers |= KeyModifiers::CONTROL,
                "ALT" => modifiers |= KeyModifiers::ALT,
                "SHIFT" => modifiers |= KeyModifiers::SHIFT,
                _ => {
                    return Err(serde::de::Error::custom(format!(
                        "Unknown KeyModifier: {}",
                        part
                    )))
                }
            }
        }

        Ok(modifiers)
    }
}

impl KeyboardShortcut {
    /// Create a new keyboard shortcut with the specified key and modifiers
    pub fn new(key: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { key, modifiers }
    }

    /// Create a simple key shortcut without modifiers
    pub fn simple(key: KeyCode) -> Self {
        Self::new(key, KeyModifiers::NONE)
    }

    /// Create a Ctrl+key shortcut
    pub fn ctrl(key: KeyCode) -> Self {
        Self::new(key, KeyModifiers::CONTROL)
    }

    /// Create an Alt+key shortcut
    pub fn alt(key: KeyCode) -> Self {
        Self::new(key, KeyModifiers::ALT)
    }

    /// Create a Shift+key shortcut
    pub fn shift(key: KeyCode) -> Self {
        Self::new(key, KeyModifiers::SHIFT)
    }
}

impl std::fmt::Display for KeyboardShortcut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts = Vec::new();

        if self.modifiers.contains(KeyModifiers::CONTROL) {
            parts.push("Ctrl");
        }
        if self.modifiers.contains(KeyModifiers::ALT) {
            parts.push("Alt");
        }
        if self.modifiers.contains(KeyModifiers::SHIFT) {
            parts.push("Shift");
        }

        let key_str = match self.key {
            KeyCode::Char(' ') => "Space".to_string(),
            KeyCode::Char(c) => c.to_uppercase().to_string(),
            KeyCode::F(n) => format!("F{}", n),
            KeyCode::Enter => "Enter".to_string(),
            KeyCode::Esc => "Esc".to_string(),
            KeyCode::Tab => "Tab".to_string(),
            KeyCode::BackTab => "Shift+Tab".to_string(),
            KeyCode::Backspace => "Backspace".to_string(),
            KeyCode::Delete => "Delete".to_string(),
            KeyCode::Insert => "Insert".to_string(),
            KeyCode::Home => "Home".to_string(),
            KeyCode::End => "End".to_string(),
            KeyCode::PageUp => "PageUp".to_string(),
            KeyCode::PageDown => "PageDown".to_string(),
            KeyCode::Up => "Up".to_string(),
            KeyCode::Down => "Down".to_string(),
            KeyCode::Left => "Left".to_string(),
            KeyCode::Right => "Right".to_string(),
            _ => format!("{:?}", self.key),
        };

        if parts.is_empty() {
            write!(f, "{}", key_str)
        } else {
            write!(f, "{}+{}", parts.join("+"), key_str)
        }
    }
}

/// Actions that can be triggered by keyboard shortcuts
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyboardAction {
    // Global actions
    Quit,
    ForceQuit,
    ShowKeyboardShortcuts,

    // Navigation
    NextPane,
    PreviousPane,
    VimMoveLeft,
    VimMoveRight,
    VimMoveUp,
    VimMoveDown,
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,

    // Selection and interaction
    Select,
    Escape,
    ToggleExpanded,

    // Email actions
    ComposeEmail,
    ShowDraftList,
    ReplyEmail,
    ReplyAllEmail,
    ForwardEmail,
    DeleteEmail,
    ArchiveEmail,
    MarkAsRead,
    MarkAsUnread,

    // Account management
    AddAccount,
    RemoveAccount,
    RefreshAccount,
    SwitchAccount,

    // Search and filter
    StartSearch,
    StartFolderSearch,
    EndSearch,

    // View controls
    ToggleThreadedView,
    ExpandThread,
    CollapseThread,
    ToggleViewMode,
    ToggleHeaders,
    OpenEmailViewer,

    // Sorting
    SortByDate,
    SortBySender,
    SortBySubject,

    // Content preview
    ScrollToTop,
    ScrollToBottom,
    SelectFirstAttachment,
    SaveAttachment,
    ViewAttachment,
    OpenAttachmentWithSystem,

    // Folder operations
    CreateFolder,
    DeleteFolder,
    RefreshFolder,

    // Copy operations
    CopyEmailContent,
    CopyAttachmentInfo,

    // Function keys for folder operations
    FolderRefresh,
    FolderRename,
    FolderDelete,

    // Message navigation
    NextMessage,
    PreviousMessage,
    
    // Email viewer actions
    EmailViewerReply,
    EmailViewerReplyAll,
    EmailViewerForward,
    EmailViewerEdit,
    EmailViewerDelete,
    EmailViewerArchive,
    EmailViewerMarkRead,
    EmailViewerMarkUnread,
    EmailViewerClose,


    // Calendar actions
    ShowCalendar,
    ShowEmail,
    CreateEvent,
    EditEvent,
    DeleteEvent,
    ViewEventDetails,
    CreateTodo,
    ToggleTodoComplete,
    ViewTodos,
    CalendarNextMonth,
    CalendarPrevMonth,
    CalendarToday,
    CalendarWeekView,
    CalendarMonthView,
    CalendarDayView,
    CalendarAgendaView,

    // Attachment navigation
    NextAttachment,
    PreviousAttachment,

    // Contacts actions
    ContactsPopup,
    ViewSenderContact,
    EditSenderContact,
    AddSenderToContacts,
    RemoveSenderFromContacts,
    ContactQuickActions,
}

/// Configuration for keyboard shortcuts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyboardConfig {
    /// Mapping of shortcuts to actions
    shortcuts: HashMap<KeyboardShortcut, KeyboardAction>,
    /// Description of each action for help display
    action_descriptions: HashMap<KeyboardAction, String>,
}

impl Default for KeyboardConfig {
    fn default() -> Self {
        let mut config = KeyboardConfig {
            shortcuts: HashMap::new(),
            action_descriptions: HashMap::new(),
        };

        // Set up default shortcuts
        config.setup_default_shortcuts();
        config.setup_action_descriptions();

        config
    }
}

impl KeyboardConfig {
    /// Create a new keyboard configuration with default shortcuts
    pub fn new() -> Self {
        Self::default()
    }

    /// Set up default keyboard shortcuts matching the current hardcoded ones
    fn setup_default_shortcuts(&mut self) {
        // Global actions
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Char('q')),
            KeyboardAction::Quit,
        );
        self.shortcuts.insert(
            KeyboardShortcut::ctrl(KeyCode::Char('c')),
            KeyboardAction::ForceQuit,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Char('?')),
            KeyboardAction::ShowKeyboardShortcuts,
        );

        // Navigation
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Tab),
            KeyboardAction::NextPane,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::BackTab),
            KeyboardAction::PreviousPane,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Char('h')),
            KeyboardAction::VimMoveLeft,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Char('l')),
            KeyboardAction::VimMoveRight,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Char('j')),
            KeyboardAction::VimMoveDown,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Char('k')),
            KeyboardAction::VimMoveUp,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Down),
            KeyboardAction::MoveDown,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Up),
            KeyboardAction::MoveUp,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Left),
            KeyboardAction::MoveLeft,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Right),
            KeyboardAction::MoveRight,
        );

        // Selection and interaction
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Enter),
            KeyboardAction::Select,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Esc),
            KeyboardAction::Escape,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Char(' ')),
            KeyboardAction::ToggleExpanded,
        );

        // Email actions
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Char('c')),
            KeyboardAction::ComposeEmail,
        );
        self.shortcuts.insert(
            KeyboardShortcut::ctrl(KeyCode::Char('d')),
            KeyboardAction::ShowDraftList,
        );
        
        // Message actions - using context-aware shortcuts
        self.shortcuts.insert(
            KeyboardShortcut::ctrl(KeyCode::Char('r')),
            KeyboardAction::ReplyEmail,
        );
        self.shortcuts.insert(
            KeyboardShortcut::shift(KeyCode::Char('R')),
            KeyboardAction::ReplyAllEmail,
        );
        self.shortcuts.insert(
            KeyboardShortcut::ctrl(KeyCode::Char('f')),
            KeyboardAction::ForwardEmail,
        );
        self.shortcuts.insert(
            KeyboardShortcut::shift(KeyCode::Delete),
            KeyboardAction::DeleteEmail,
        );
        self.shortcuts.insert(
            KeyboardShortcut::shift(KeyCode::Char('A')),
            KeyboardAction::ArchiveEmail,
        );
        self.shortcuts.insert(
            KeyboardShortcut::shift(KeyCode::Char('U')),
            KeyboardAction::MarkAsUnread,
        );
        self.shortcuts.insert(
            KeyboardShortcut::shift(KeyCode::Char('M')), 
            KeyboardAction::MarkAsRead,
        );
        
        // Message navigation
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Char('n')),
            KeyboardAction::NextMessage,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Char('p')),
            KeyboardAction::PreviousMessage,
        );

        // Account management
        self.shortcuts.insert(
            KeyboardShortcut::ctrl(KeyCode::Char('a')),
            KeyboardAction::AddAccount,
        );
        self.shortcuts.insert(
            KeyboardShortcut::ctrl(KeyCode::Char('x')),
            KeyboardAction::RemoveAccount,
        );
        self.shortcuts.insert(
            KeyboardShortcut::new(KeyCode::Char('r'), KeyModifiers::CONTROL | KeyModifiers::SHIFT),
            KeyboardAction::RefreshAccount,
        );

        // Search
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Char('/')),
            KeyboardAction::StartSearch,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Char('f')),
            KeyboardAction::StartFolderSearch,
        );

        // View controls
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Char('t')),
            KeyboardAction::ToggleThreadedView,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Char('o')),
            KeyboardAction::ExpandThread,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Char('C')),
            KeyboardAction::CollapseThread,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Char('m')),
            KeyboardAction::ToggleViewMode,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Char('H')),
            KeyboardAction::ToggleHeaders,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Char('V')),
            KeyboardAction::OpenEmailViewer,
        );

        // Sorting
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Char('s')),
            KeyboardAction::SortByDate,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Char('r')),
            KeyboardAction::SortBySender,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Char('u')),
            KeyboardAction::SortBySubject,
        );

        // Content preview
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Home),
            KeyboardAction::ScrollToTop,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::End),
            KeyboardAction::ScrollToBottom,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Char('a')),
            KeyboardAction::SelectFirstAttachment,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Char('A')),
            KeyboardAction::ViewAttachment,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Char('O')),
            KeyboardAction::OpenAttachmentWithSystem,
        );

        // Folder operations
        self.shortcuts.insert(
            KeyboardShortcut::ctrl(KeyCode::Char('n')),
            KeyboardAction::CreateFolder,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Char('d')),
            KeyboardAction::DeleteFolder,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Char('R')),
            KeyboardAction::RefreshFolder,
        );

        // Copy operations
        self.shortcuts.insert(
            KeyboardShortcut::ctrl(KeyCode::Char('y')),
            KeyboardAction::CopyEmailContent,
        );
        self.shortcuts.insert(
            KeyboardShortcut::alt(KeyCode::Char('c')),
            KeyboardAction::CopyAttachmentInfo,
        );

        // Function keys
        self.shortcuts.insert(
            KeyboardShortcut::alt(KeyCode::Char('r')),
            KeyboardAction::FolderRefresh,
        );
        self.shortcuts.insert(
            KeyboardShortcut::alt(KeyCode::Char('n')),
            KeyboardAction::FolderRename,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Delete),
            KeyboardAction::FolderDelete,
        );

        // Email viewer shortcuts - Use Esc instead of 'q' to avoid conflict with Quit
        // EmailViewerClose is handled by Escape action in email viewer mode

        // Attachment navigation
        self.shortcuts.insert(
            KeyboardShortcut::ctrl(KeyCode::Char('j')),
            KeyboardAction::NextAttachment,
        );
        self.shortcuts.insert(
            KeyboardShortcut::ctrl(KeyCode::Char('k')),
            KeyboardAction::PreviousAttachment,
        );

        // Calendar actions
        self.shortcuts.insert(
            KeyboardShortcut::ctrl(KeyCode::Char('l')),
            KeyboardAction::ShowCalendar,
        );
        self.shortcuts.insert(
            KeyboardShortcut::ctrl(KeyCode::Char('m')),
            KeyboardAction::ShowEmail,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Char('e')),
            KeyboardAction::CreateEvent,
        );
        self.shortcuts.insert(
            KeyboardShortcut::ctrl(KeyCode::Char('e')),
            KeyboardAction::EditEvent,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Delete),
            KeyboardAction::DeleteEvent,
        );
        // ViewEventDetails should use a different key (Enter conflicts with Select)
        // Use Space or another key for viewing event details in calendar context
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Char('T')),
            KeyboardAction::CreateTodo,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Char(' ')),
            KeyboardAction::ToggleTodoComplete,
        );
        self.shortcuts.insert(
            KeyboardShortcut::ctrl(KeyCode::Char('t')),
            KeyboardAction::ViewTodos,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Right),
            KeyboardAction::CalendarNextMonth,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Left),
            KeyboardAction::CalendarPrevMonth,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Char('.')),
            KeyboardAction::CalendarToday,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Char('1')),
            KeyboardAction::CalendarDayView,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Char('2')),
            KeyboardAction::CalendarWeekView,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Char('3')),
            KeyboardAction::CalendarMonthView,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Char('4')),
            KeyboardAction::CalendarAgendaView,
        );

        // Contacts actions - temporarily changed from Ctrl+Shift+C to Ctrl+K for testing
        self.shortcuts.insert(
            KeyboardShortcut::new(KeyCode::Char('k'), KeyModifiers::CONTROL),
            KeyboardAction::ContactsPopup,
        );
        
        // Contact quick actions (context-aware)
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Char('i')),
            KeyboardAction::ViewSenderContact,
        );
        self.shortcuts.insert(
            KeyboardShortcut::ctrl(KeyCode::Char('i')),
            KeyboardAction::EditSenderContact,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Char('+')),
            KeyboardAction::AddSenderToContacts,
        );
        self.shortcuts.insert(
            KeyboardShortcut::simple(KeyCode::Char('-')),
            KeyboardAction::RemoveSenderFromContacts,
        );
        self.shortcuts.insert(
            KeyboardShortcut::shift(KeyCode::Char('C')),
            KeyboardAction::ContactQuickActions,
        );
    }

    /// Set up descriptions for each action
    fn setup_action_descriptions(&mut self) {
        self.action_descriptions
            .insert(KeyboardAction::Quit, "Quit application".to_string());
        self.action_descriptions.insert(
            KeyboardAction::ForceQuit,
            "Force quit application".to_string(),
        );

        self.action_descriptions
            .insert(KeyboardAction::NextPane, "Move to next pane".to_string());
        self.action_descriptions.insert(
            KeyboardAction::PreviousPane,
            "Move to previous pane".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::VimMoveLeft,
            "Move left (vim-style)".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::VimMoveRight,
            "Move right (vim-style)".to_string(),
        );
        self.action_descriptions
            .insert(KeyboardAction::VimMoveUp, "Move up (vim-style)".to_string());
        self.action_descriptions.insert(
            KeyboardAction::VimMoveDown,
            "Move down (vim-style)".to_string(),
        );

        self.action_descriptions
            .insert(KeyboardAction::Select, "Select current item".to_string());
        self.action_descriptions.insert(
            KeyboardAction::Escape,
            "Cancel/escape current operation".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::ToggleExpanded,
            "Toggle expanded/collapsed state".to_string(),
        );

        self.action_descriptions.insert(
            KeyboardAction::ComposeEmail,
            "Compose new email".to_string(),
        );
        self.action_descriptions
            .insert(KeyboardAction::ShowDraftList, "Show draft list".to_string());
        self.action_descriptions.insert(
            KeyboardAction::ReplyEmail,
            "Reply to selected message".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::ReplyAllEmail,
            "Reply to all recipients".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::ForwardEmail,
            "Forward selected message".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::DeleteEmail,
            "Delete selected message".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::ArchiveEmail,
            "Archive selected message".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::MarkAsRead,
            "Mark message as read".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::MarkAsUnread,
            "Mark message as unread".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::NextMessage,
            "Navigate to next message".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::PreviousMessage,
            "Navigate to previous message".to_string(),
        );

        self.action_descriptions
            .insert(KeyboardAction::AddAccount, "Add new account".to_string());
        self.action_descriptions.insert(
            KeyboardAction::RemoveAccount,
            "Remove current account".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::RefreshAccount,
            "Refresh account connection".to_string(),
        );

        self.action_descriptions.insert(
            KeyboardAction::StartSearch,
            "Start message search".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::StartFolderSearch,
            "Start folder search".to_string(),
        );

        self.action_descriptions.insert(
            KeyboardAction::ToggleThreadedView,
            "Toggle threaded view".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::ExpandThread,
            "Expand selected thread".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::CollapseThread,
            "Collapse selected thread".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::ToggleViewMode,
            "Toggle view mode".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::ToggleHeaders,
            "Toggle extended headers".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::OpenEmailViewer,
            "Open email in full-screen viewer".to_string(),
        );

        self.action_descriptions.insert(
            KeyboardAction::SortByDate,
            "Sort messages by date".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::SortBySender,
            "Sort messages by sender".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::SortBySubject,
            "Sort messages by subject".to_string(),
        );

        self.action_descriptions
            .insert(KeyboardAction::ScrollToTop, "Scroll to top".to_string());
        self.action_descriptions.insert(
            KeyboardAction::ScrollToBottom,
            "Scroll to bottom".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::SelectFirstAttachment,
            "Select first attachment".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::ViewAttachment,
            "View selected attachment".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::OpenAttachmentWithSystem,
            "Open attachment with system app".to_string(),
        );

        self.action_descriptions.insert(
            KeyboardAction::CreateFolder,
            "Create new folder".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::DeleteFolder,
            "Delete selected folder".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::RefreshFolder,
            "Refresh selected folder".to_string(),
        );

        self.action_descriptions.insert(
            KeyboardAction::CopyEmailContent,
            "Copy email content to clipboard".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::CopyAttachmentInfo,
            "Copy attachment info to clipboard".to_string(),
        );

        self.action_descriptions
            .insert(KeyboardAction::FolderRefresh, "Refresh folder".to_string());
        self.action_descriptions
            .insert(KeyboardAction::FolderRename, "Rename folder".to_string());
        self.action_descriptions
            .insert(KeyboardAction::FolderDelete, "Delete (Del)".to_string());

        self.action_descriptions.insert(
            KeyboardAction::NextAttachment,
            "Next attachment".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::PreviousAttachment,
            "Previous attachment".to_string(),
        );

        // Calendar actions
        self.action_descriptions.insert(
            KeyboardAction::ShowCalendar,
            "Open calendar view".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::ShowEmail,
            "Return to email view".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::CreateEvent,
            "Create new calendar event".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::EditEvent,
            "Edit selected event".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::DeleteEvent,
            "Delete selected event".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::ViewEventDetails,
            "View event details".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::CreateTodo,
            "Create new todo/task".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::ToggleTodoComplete,
            "Toggle todo completion status".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::ViewTodos,
            "View todos and tasks".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::CalendarNextMonth,
            "Navigate to next month".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::CalendarPrevMonth,
            "Navigate to previous month".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::CalendarToday,
            "Go to today's date".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::CalendarDayView,
            "Switch to day view".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::CalendarWeekView,
            "Switch to week view".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::CalendarMonthView,
            "Switch to month view".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::CalendarAgendaView,
            "Switch to agenda view".to_string(),
        );

        // Contacts actions
        self.action_descriptions.insert(
            KeyboardAction::ContactsPopup,
            "Open contacts popup".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::ViewSenderContact,
            "View sender contact details".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::EditSenderContact,
            "Edit sender contact".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::AddSenderToContacts,
            "Add sender to contacts".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::RemoveSenderFromContacts,
            "Remove sender from contacts".to_string(),
        );
        self.action_descriptions.insert(
            KeyboardAction::ContactQuickActions,
            "Show contact quick actions menu".to_string(),
        );
    }

    /// Get the action for a given keyboard shortcut
    pub fn get_action(&self, shortcut: &KeyboardShortcut) -> Option<&KeyboardAction> {
        self.shortcuts.get(shortcut)
    }

    /// Get the shortcut(s) for a given action
    pub fn get_shortcuts_for_action(&self, action: &KeyboardAction) -> Vec<&KeyboardShortcut> {
        self.shortcuts
            .iter()
            .filter(|(_, a)| *a == action)
            .map(|(k, _)| k)
            .collect()
    }

    /// Set a keyboard shortcut for an action
    pub fn set_shortcut(&mut self, shortcut: KeyboardShortcut, action: KeyboardAction) {
        self.shortcuts.insert(shortcut, action);
    }

    /// Remove a keyboard shortcut
    pub fn remove_shortcut(&mut self, shortcut: &KeyboardShortcut) {
        self.shortcuts.remove(shortcut);
    }

    /// Get all shortcuts grouped by category for help display
    pub fn get_shortcuts_by_category(
        &self,
    ) -> HashMap<String, Vec<(&KeyboardShortcut, &KeyboardAction, &str)>> {
        let mut categories: HashMap<String, Vec<(&KeyboardShortcut, &KeyboardAction, &str)>> =
            HashMap::new();

        for (shortcut, action) in &self.shortcuts {
            let category = self.get_action_category(action);
            let description = self
                .action_descriptions
                .get(action)
                .map(|s| s.as_str())
                .unwrap_or("No description");

            categories
                .entry(category)
                .or_default()
                .push((shortcut, action, description));
        }

        // Sort shortcuts within each category
        for shortcuts in categories.values_mut() {
            shortcuts.sort_by(|a, b| a.0.to_string().cmp(&b.0.to_string()));
        }

        categories
    }

    /// Get the category for an action (for organizing help display)
    fn get_action_category(&self, action: &KeyboardAction) -> String {
        match action {
            KeyboardAction::Quit
            | KeyboardAction::ForceQuit
            | KeyboardAction::ShowKeyboardShortcuts => "Global".to_string(),
            KeyboardAction::NextPane
            | KeyboardAction::PreviousPane
            | KeyboardAction::VimMoveLeft
            | KeyboardAction::VimMoveRight
            | KeyboardAction::VimMoveUp
            | KeyboardAction::VimMoveDown
            | KeyboardAction::MoveUp
            | KeyboardAction::MoveDown
            | KeyboardAction::MoveLeft
            | KeyboardAction::MoveRight => "Navigation".to_string(),
            KeyboardAction::Select | KeyboardAction::Escape | KeyboardAction::ToggleExpanded => {
                "Selection".to_string()
            }
            KeyboardAction::ComposeEmail
            | KeyboardAction::ShowDraftList
            | KeyboardAction::ReplyEmail
            | KeyboardAction::ReplyAllEmail
            | KeyboardAction::ForwardEmail
            | KeyboardAction::DeleteEmail
            | KeyboardAction::ArchiveEmail
            | KeyboardAction::MarkAsRead
            | KeyboardAction::MarkAsUnread
            | KeyboardAction::NextMessage
            | KeyboardAction::PreviousMessage => "Email".to_string(),
            KeyboardAction::AddAccount
            | KeyboardAction::RemoveAccount
            | KeyboardAction::RefreshAccount
            | KeyboardAction::SwitchAccount => "Account Management".to_string(),
            KeyboardAction::StartSearch
            | KeyboardAction::StartFolderSearch
            | KeyboardAction::EndSearch => "Search".to_string(),
            KeyboardAction::ToggleThreadedView
            | KeyboardAction::ExpandThread
            | KeyboardAction::CollapseThread
            | KeyboardAction::ToggleViewMode
            | KeyboardAction::ToggleHeaders
            | KeyboardAction::OpenEmailViewer => "View Controls".to_string(),
            KeyboardAction::SortByDate
            | KeyboardAction::SortBySender
            | KeyboardAction::SortBySubject => "Sorting".to_string(),
            KeyboardAction::ScrollToTop
            | KeyboardAction::ScrollToBottom
            | KeyboardAction::SelectFirstAttachment
            | KeyboardAction::SaveAttachment
            | KeyboardAction::ViewAttachment
            | KeyboardAction::OpenAttachmentWithSystem
            | KeyboardAction::NextAttachment
            | KeyboardAction::PreviousAttachment => "Content Preview".to_string(),
            KeyboardAction::CreateFolder
            | KeyboardAction::DeleteFolder
            | KeyboardAction::RefreshFolder
            | KeyboardAction::FolderRefresh
            | KeyboardAction::FolderRename
            | KeyboardAction::FolderDelete => "Folder Operations".to_string(),
            KeyboardAction::CopyEmailContent | KeyboardAction::CopyAttachmentInfo => {
                "Copy Operations".to_string()
            }
            KeyboardAction::EmailViewerReply
            | KeyboardAction::EmailViewerReplyAll
            | KeyboardAction::EmailViewerForward
            | KeyboardAction::EmailViewerEdit
            | KeyboardAction::EmailViewerDelete
            | KeyboardAction::EmailViewerArchive
            | KeyboardAction::EmailViewerMarkRead
            | KeyboardAction::EmailViewerMarkUnread
            | KeyboardAction::EmailViewerClose => "Email Viewer".to_string(),
            KeyboardAction::ShowCalendar
            | KeyboardAction::ShowEmail
            | KeyboardAction::CreateEvent
            | KeyboardAction::EditEvent
            | KeyboardAction::DeleteEvent
            | KeyboardAction::ViewEventDetails
            | KeyboardAction::CreateTodo
            | KeyboardAction::ToggleTodoComplete
            | KeyboardAction::ViewTodos
            | KeyboardAction::CalendarNextMonth
            | KeyboardAction::CalendarPrevMonth
            | KeyboardAction::CalendarToday
            | KeyboardAction::CalendarWeekView
            | KeyboardAction::CalendarMonthView
            | KeyboardAction::CalendarDayView
            | KeyboardAction::CalendarAgendaView => "Calendar".to_string(),
            KeyboardAction::ContactsPopup 
            | KeyboardAction::ViewSenderContact
            | KeyboardAction::EditSenderContact
            | KeyboardAction::AddSenderToContacts
            | KeyboardAction::RemoveSenderFromContacts
            | KeyboardAction::ContactQuickActions => "Contacts".to_string(),
        }
    }

    /// Load keyboard configuration from file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: KeyboardConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save keyboard configuration to file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Get the configuration directory for the application
    pub fn get_config_dir() -> Result<PathBuf> {
        let config_dir = if let Ok(config_dir) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(config_dir).join("comunicado")
        } else if let Ok(home_dir) = std::env::var("HOME") {
            PathBuf::from(home_dir).join(".config").join("comunicado")
        } else {
            return Err(anyhow!("Cannot determine config directory"));
        };

        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)?;
        }

        Ok(config_dir)
    }

    /// Get the default path for the keyboard configuration file
    pub fn get_default_config_path() -> Result<PathBuf> {
        Ok(Self::get_config_dir()?.join("keyboard.toml"))
    }

    /// Load the keyboard configuration from the default location, creating it if it doesn't exist
    pub fn load_or_create_default() -> Result<Self> {
        let config_path = Self::get_default_config_path()?;

        if config_path.exists() {
            Self::load_from_file(config_path)
        } else {
            let config = Self::default();
            config.save_to_file(&config_path)?;
            Ok(config)
        }
    }

    /// Reset to default configuration
    pub fn reset_to_defaults(&mut self) {
        *self = Self::default();
    }

    /// Get a description of an action
    pub fn get_action_description(&self, action: &KeyboardAction) -> Option<&str> {
        self.action_descriptions.get(action).map(|s| s.as_str())
    }

    /// Check if a shortcut conflicts with existing shortcuts
    pub fn has_conflict(&self, shortcut: &KeyboardShortcut) -> bool {
        self.shortcuts.contains_key(shortcut)
    }

    /// Validate the configuration for conflicts and issues
    pub fn validate(&self) -> Vec<String> {
        let mut issues = Vec::new();

        // Check for essential shortcuts that must exist
        let essential_actions = vec![
            KeyboardAction::Quit,
            KeyboardAction::NextPane,
            KeyboardAction::PreviousPane,
            KeyboardAction::Select,
            KeyboardAction::Escape,
        ];

        for action in essential_actions {
            if !self.shortcuts.values().any(|a| *a == action) {
                issues.push(format!(
                    "Missing essential shortcut for action: {:?}",
                    action
                ));
            }
        }

        // Check for duplicate shortcuts (should not happen with HashMap, but good to verify)
        let mut shortcuts_count = HashMap::new();
        for shortcut in self.shortcuts.keys() {
            *shortcuts_count.entry(shortcut).or_insert(0) += 1;
        }

        for (shortcut, count) in shortcuts_count {
            if count > 1 {
                issues.push(format!("Duplicate shortcut found: {}", shortcut));
            }
        }

        issues
    }
}

/// Manager for keyboard shortcuts and configuration
pub struct KeyboardManager {
    config: KeyboardConfig,
}

impl KeyboardManager {
    /// Create a new keyboard manager with default configuration
    pub fn new() -> Result<Self> {
        let config = KeyboardConfig::load_or_create_default()?;
        Ok(Self { config })
    }

    /// Create a keyboard manager with a specific configuration
    pub fn with_config(config: KeyboardConfig) -> Self {
        Self { config }
    }

    /// Get the action for a key event
    pub fn get_action(
        &self,
        key_code: KeyCode,
        modifiers: KeyModifiers,
    ) -> Option<&KeyboardAction> {
        let shortcut = KeyboardShortcut::new(key_code, modifiers);
        
        self.config.get_action(&shortcut)
    }

    /// Get the keyboard configuration
    pub fn config(&self) -> &KeyboardConfig {
        &self.config
    }

    /// Get mutable reference to the keyboard configuration
    pub fn config_mut(&mut self) -> &mut KeyboardConfig {
        &mut self.config
    }

    /// Save the current configuration to file
    pub fn save_config(&self) -> Result<()> {
        let config_path = KeyboardConfig::get_default_config_path()?;
        self.config.save_to_file(config_path)
    }

    /// Reload configuration from file
    pub fn reload_config(&mut self) -> Result<()> {
        self.config = KeyboardConfig::load_or_create_default()?;
        Ok(())
    }

    /// Reset configuration to defaults
    pub fn reset_to_defaults(&mut self) -> Result<()> {
        self.config.reset_to_defaults();
        self.save_config()
    }

    /// Add or update a keyboard shortcut
    pub fn set_shortcut(
        &mut self,
        shortcut: KeyboardShortcut,
        action: KeyboardAction,
    ) -> Result<()> {
        self.config.set_shortcut(shortcut, action);
        self.save_config()
    }

    /// Remove a keyboard shortcut
    pub fn remove_shortcut(&mut self, shortcut: &KeyboardShortcut) -> Result<()> {
        self.config.remove_shortcut(shortcut);
        self.save_config()
    }

    /// Get help text for all keyboard shortcuts
    pub fn get_help_text(&self) -> String {
        let mut help = String::new();
        help.push_str("Keyboard Shortcuts\n");
        help.push_str("==================\n\n");

        let categories = self.config.get_shortcuts_by_category();
        let mut sorted_categories: Vec<_> = categories.keys().collect();
        sorted_categories.sort();

        for category in sorted_categories {
            help.push_str(&format!("{}:\n", category));
            help.push_str(&"-".repeat(category.len() + 1));
            help.push('\n');

            if let Some(shortcuts) = categories.get(category) {
                for (shortcut, _action, description) in shortcuts {
                    help.push_str(&format!(
                        "  {:15} - {}\n",
                        shortcut.to_string(),
                        description
                    ));
                }
            }
            help.push('\n');
        }

        help
    }

    /// Get all keyboard shortcuts as a vector of (shortcut, action) pairs
    pub fn get_all_shortcuts(&self) -> Vec<(KeyboardShortcut, KeyboardAction)> {
        self.config
            .shortcuts
            .iter()
            .map(|(shortcut, action)| (shortcut.clone(), action.clone()))
            .collect()
    }
}

impl Default for KeyboardManager {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self::with_config(KeyboardConfig::default()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_keyboard_shortcut_display() {
        let shortcut = KeyboardShortcut::ctrl(KeyCode::Char('c'));
        assert_eq!(shortcut.to_string(), "Ctrl+C");

        let shortcut = KeyboardShortcut::simple(KeyCode::Char('q'));
        assert_eq!(shortcut.to_string(), "Q");

        let shortcut = KeyboardShortcut::new(KeyCode::F(5), KeyModifiers::NONE);
        assert_eq!(shortcut.to_string(), "F5");
    }

    #[test]
    fn test_keyboard_config_default() {
        let config = KeyboardConfig::default();

        // Test that essential shortcuts exist
        let quit_shortcut = KeyboardShortcut::simple(KeyCode::Char('q'));
        assert_eq!(
            config.get_action(&quit_shortcut),
            Some(&KeyboardAction::Quit)
        );

        let compose_shortcut = KeyboardShortcut::simple(KeyCode::Char('c'));
        assert_eq!(
            config.get_action(&compose_shortcut),
            Some(&KeyboardAction::ComposeEmail)
        );
    }

    #[test]
    fn test_keyboard_config_modification() {
        let mut config = KeyboardConfig::default();

        // Add a custom shortcut
        let custom_shortcut = KeyboardShortcut::ctrl(KeyCode::Char('z'));
        config.set_shortcut(custom_shortcut.clone(), KeyboardAction::ShowKeyboardShortcuts);

        assert_eq!(
            config.get_action(&custom_shortcut),
            Some(&KeyboardAction::ShowKeyboardShortcuts)
        );

        // Remove the shortcut
        config.remove_shortcut(&custom_shortcut);
        assert_eq!(config.get_action(&custom_shortcut), None);
    }

    #[test]
    fn test_keyboard_config_serialization() {
        let config = KeyboardConfig::default();

        // Test TOML serialization
        let toml_str = toml::to_string(&config).unwrap();
        let deserialized: KeyboardConfig = toml::from_str(&toml_str).unwrap();

        // Compare a few key shortcuts
        let quit_shortcut = KeyboardShortcut::simple(KeyCode::Char('q'));
        assert_eq!(
            config.get_action(&quit_shortcut),
            deserialized.get_action(&quit_shortcut)
        );
    }

    #[test]
    fn test_keyboard_config_file_operations() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_keyboard.toml");

        let config = KeyboardConfig::default();

        // Save to file
        config.save_to_file(&config_path).unwrap();
        assert!(config_path.exists());

        // Load from file
        let loaded_config = KeyboardConfig::load_from_file(&config_path).unwrap();

        // Compare a shortcut
        let quit_shortcut = KeyboardShortcut::simple(KeyCode::Char('q'));
        assert_eq!(
            config.get_action(&quit_shortcut),
            loaded_config.get_action(&quit_shortcut)
        );
    }

    #[test]
    fn test_keyboard_manager() {
        let config = KeyboardConfig::default();
        let manager = KeyboardManager::with_config(config);

        // Test action lookup
        let action = manager.get_action(KeyCode::Char('q'), KeyModifiers::NONE);
        assert_eq!(action, Some(&KeyboardAction::Quit));

        let action = manager.get_action(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(action, Some(&KeyboardAction::ForceQuit));
    }

    #[test]
    fn test_keyboard_config_validation() {
        let config = KeyboardConfig::default();
        let issues = config.validate();

        // Default config should have no validation issues
        assert!(
            issues.is_empty(),
            "Default config should be valid, but found issues: {:?}",
            issues
        );
    }

    #[test]
    fn test_keyboard_config_categories() {
        let config = KeyboardConfig::default();
        let categories = config.get_shortcuts_by_category();

        // Should have several categories
        assert!(categories.contains_key("Global"));
        assert!(categories.contains_key("Navigation"));
        assert!(categories.contains_key("Email"));

        // Global category should contain quit action
        let global_shortcuts = categories.get("Global").unwrap();
        assert!(global_shortcuts
            .iter()
            .any(|(_, action, _)| **action == KeyboardAction::Quit));
    }
}
