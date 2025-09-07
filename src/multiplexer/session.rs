//! Session management and persistence

use super::{MultiplexerError, MultiplexerResult};

// Type alias for consistency
pub type SessionResult<T> = Result<T, MultiplexerError>;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

/// Session state that can be saved and restored
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub id: Uuid,
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_accessed: chrono::DateTime<chrono::Utc>,
    pub email_state: EmailSessionState,
    pub calendar_state: CalendarSessionState,
    pub ui_state: UISessionState,
    pub window_layout: WindowLayoutState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailSessionState {
    pub current_folder: String,
    pub selected_email: Option<String>,
    pub draft_emails: Vec<DraftEmail>,
    pub search_query: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarSessionState {
    pub current_view: String,
    pub selected_date: chrono::NaiveDate,
    pub active_calendars: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UISessionState {
    pub active_pane: String,
    pub sidebar_width: u16,
    pub theme: String,
    pub notification_settings: NotificationSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowLayoutState {
    pub arrangement: PaneArrangement,
    pub pane_sizes: HashMap<String, (u16, u16)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftEmail {
    pub id: Uuid,
    pub to: String,
    pub subject: String,
    pub body: String,
    pub saved_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    pub email_notifications: bool,
    pub calendar_notifications: bool,
    pub sound_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaneArrangement {
    Single,
    Horizontal,
    Vertical,
    ThreePane,
    Custom(String),
}

/// Session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub auto_save_interval: u64, // seconds
    pub max_saved_sessions: usize,
    pub session_storage_path: PathBuf,
    pub restore_on_startup: bool,
}

/// Session manager for handling persistence
#[allow(dead_code)]
pub struct SessionManager {
    config: SessionConfig,
    current_state: Option<SessionState>,
    saved_sessions: HashMap<Uuid, SessionState>,
}

impl SessionManager {
    pub fn new() -> MultiplexerResult<Self> {
        Ok(Self {
            config: SessionConfig::default(),
            current_state: None,
            saved_sessions: HashMap::new(),
        })
    }

    /// Save current session state
    pub fn save_current_state(&self) -> MultiplexerResult<SessionState> {
        if let Some(ref state) = self.current_state {
            Ok(state.clone())
        } else {
            // Create a new session state
            let state = SessionState {
                id: Uuid::new_v4(),
                name: "Default Session".to_string(),
                created_at: chrono::Utc::now(),
                last_accessed: chrono::Utc::now(),
                email_state: EmailSessionState::default(),
                calendar_state: CalendarSessionState::default(),
                ui_state: UISessionState::default(),
                window_layout: WindowLayoutState::default(),
            };
            Ok(state)
        }
    }

    /// Restore session state
    pub fn restore_state(&mut self, state: SessionState) -> MultiplexerResult<()> {
        self.current_state = Some(state);
        Ok(())
    }

    /// Persist session state to disk
    pub fn persist_state(&self, state: &SessionState) -> MultiplexerResult<()> {
        let session_file = self
            .config
            .session_storage_path
            .join(format!("{}.json", state.id));

        let json =
            serde_json::to_string_pretty(state).map_err(|e| MultiplexerError::Serialization(e))?;

        std::fs::create_dir_all(&self.config.session_storage_path)?;
        std::fs::write(session_file, json)?;

        Ok(())
    }

    /// Load session state from disk
    pub fn load_state(&mut self, session_id: Uuid) -> MultiplexerResult<SessionState> {
        let session_file = self
            .config
            .session_storage_path
            .join(format!("{}.json", session_id));

        if !session_file.exists() {
            return Err(MultiplexerError::SessionError(
                "Session file not found".to_string(),
            ));
        }

        let json = std::fs::read_to_string(session_file)?;
        let state: SessionState =
            serde_json::from_str(&json).map_err(|e| MultiplexerError::Serialization(e))?;

        Ok(state)
    }

    /// List all saved sessions
    pub fn list_saved_sessions(&self) -> MultiplexerResult<Vec<SessionState>> {
        let mut sessions = Vec::new();

        if !self.config.session_storage_path.exists() {
            return Ok(sessions);
        }

        for entry in std::fs::read_dir(&self.config.session_storage_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(json) = std::fs::read_to_string(&path) {
                    if let Ok(state) = serde_json::from_str::<SessionState>(&json) {
                        sessions.push(state);
                    }
                }
            }
        }

        // Sort by last accessed time
        sessions.sort_by(|a, b| b.last_accessed.cmp(&a.last_accessed));

        Ok(sessions)
    }

    /// Clean up old sessions
    pub fn cleanup_old_sessions(&self) -> MultiplexerResult<()> {
        let sessions = self.list_saved_sessions()?;

        if sessions.len() <= self.config.max_saved_sessions {
            return Ok(());
        }

        // Remove oldest sessions beyond the limit
        for session in sessions.iter().skip(self.config.max_saved_sessions) {
            let session_file = self
                .config
                .session_storage_path
                .join(format!("{}.json", session.id));
            if session_file.exists() {
                std::fs::remove_file(session_file)?;
            }
        }

        Ok(())
    }

    /// Update current session with new data
    pub fn update_email_state(&mut self, email_state: EmailSessionState) -> MultiplexerResult<()> {
        if let Some(ref mut state) = self.current_state {
            state.email_state = email_state;
            state.last_accessed = chrono::Utc::now();
        }
        Ok(())
    }

    pub fn update_calendar_state(
        &mut self,
        calendar_state: CalendarSessionState,
    ) -> MultiplexerResult<()> {
        if let Some(ref mut state) = self.current_state {
            state.calendar_state = calendar_state;
            state.last_accessed = chrono::Utc::now();
        }
        Ok(())
    }

    pub fn update_ui_state(&mut self, ui_state: UISessionState) -> MultiplexerResult<()> {
        if let Some(ref mut state) = self.current_state {
            state.ui_state = ui_state;
            state.last_accessed = chrono::Utc::now();
        }
        Ok(())
    }

    /// Get current session state
    pub fn current_state(&self) -> Option<&SessionState> {
        self.current_state.as_ref()
    }
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            auto_save_interval: 300, // 5 minutes
            max_saved_sessions: 10,
            session_storage_path: PathBuf::from("~/.config/comunicado/sessions"),
            restore_on_startup: true,
        }
    }
}

impl Default for EmailSessionState {
    fn default() -> Self {
        Self {
            current_folder: "INBOX".to_string(),
            selected_email: None,
            draft_emails: Vec::new(),
            search_query: None,
        }
    }
}

impl Default for CalendarSessionState {
    fn default() -> Self {
        Self {
            current_view: "month".to_string(),
            selected_date: chrono::Local::now().date_naive(),
            active_calendars: vec!["primary".to_string()],
        }
    }
}

impl Default for UISessionState {
    fn default() -> Self {
        Self {
            active_pane: "email".to_string(),
            sidebar_width: 25,
            theme: "default".to_string(),
            notification_settings: NotificationSettings::default(),
        }
    }
}

impl Default for WindowLayoutState {
    fn default() -> Self {
        Self {
            arrangement: PaneArrangement::ThreePane,
            pane_sizes: HashMap::new(),
        }
    }
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            email_notifications: true,
            calendar_notifications: true,
            sound_enabled: false, // Default to quiet in terminal
        }
    }
}
