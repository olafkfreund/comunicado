//! Status line integration for multiplexers

use super::{MultiplexerError, MultiplexerResult};
use serde::{Deserialize, Serialize};

/// Status line provider trait
pub trait StatusLineProvider: Send + Sync {
    fn update_status(&mut self, status: String) -> MultiplexerResult<()>;
    fn set_format(&mut self, format: StatusFormat) -> MultiplexerResult<()>;
    fn get_current_status(&self) -> Option<&str>;
}

/// Status format configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusFormat {
    pub template: String,
    pub show_email_count: bool,
    pub show_calendar_events: bool,
    pub show_time: bool,
    pub color_scheme: ColorScheme,
}

/// Color scheme for status line
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorScheme {
    pub background: String,
    pub foreground: String,
    pub highlight: String,
    pub alert: String,
}

/// Status update information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusUpdate {
    pub email_count: Option<u32>,
    pub unread_count: Option<u32>,
    pub calendar_events_today: Option<u32>,
    pub next_event: Option<String>,
    pub connection_status: ConnectionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Connected,
    Connecting,
    Disconnected,
    Error(String),
}

impl Default for StatusFormat {
    fn default() -> Self {
        Self {
            template: "📧 {unread}/{total} | 📅 {events} | {time}".to_string(),
            show_email_count: true,
            show_calendar_events: true,
            show_time: true,
            color_scheme: ColorScheme::default(),
        }
    }
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self {
            background: "colour234".to_string(),
            foreground: "colour137".to_string(),
            highlight: "colour166".to_string(),
            alert: "colour196".to_string(),
        }
    }
}