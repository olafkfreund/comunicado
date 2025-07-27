pub mod caldav;
pub mod database;
pub mod event;
pub mod manager;
pub mod sync;
pub mod ui;

pub use caldav::{CalDAVClient, CalDAVError, CalDAVResult, CalDAVConfig};
pub use database::{CalendarDatabase, CalendarEvent, CalendarEventAttendee, CalendarEventRecurrence};
pub use event::{Event, EventStatus, EventPriority, EventRecurrence, EventReminder};
pub use manager::CalendarManager;
pub use sync::{CalendarSyncEngine, CalendarSyncProgress};
pub use ui::{CalendarUI, CalendarAction, CalendarViewMode};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use chrono::{DateTime, Utc};

/// Calendar management errors
#[derive(Error, Debug)]
pub enum CalendarError {
    #[error("CalDAV error: {0}")]
    CalDAVError(#[from] CalDAVError),
    
    #[error("Database error: {0}")]
    DatabaseError(String),
    
    #[error("Sync error: {0}")]
    SyncError(String),
    
    #[error("Authentication error: {0}")]
    AuthError(String),
    
    #[error("Invalid event data: {0}")]
    InvalidData(String),
    
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
    
    #[error("iCalendar parsing error: {0}")]
    ICalError(String),
    
    #[error("DateTime parsing error: {0}")]
    DateTimeError(#[from] chrono::ParseError),
}

pub type CalendarResult<T> = Result<T, CalendarError>;

/// Calendar provider source
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CalendarSource {
    Google { account_id: String, calendar_id: String },
    Outlook { account_id: String, calendar_id: String },
    CalDAV { account_id: String, calendar_url: String },
    Local,
}

impl CalendarSource {
    pub fn account_id(&self) -> Option<&str> {
        match self {
            CalendarSource::Google { account_id, .. } => Some(account_id),
            CalendarSource::Outlook { account_id, .. } => Some(account_id),
            CalendarSource::CalDAV { account_id, .. } => Some(account_id),
            CalendarSource::Local => None,
        }
    }
    
    pub fn provider_name(&self) -> &str {
        match self {
            CalendarSource::Google { .. } => "Google Calendar",
            CalendarSource::Outlook { .. } => "Outlook Calendar",
            CalendarSource::CalDAV { .. } => "CalDAV",
            CalendarSource::Local => "Local",
        }
    }
    
    pub fn calendar_identifier(&self) -> String {
        match self {
            CalendarSource::Google { calendar_id, .. } => calendar_id.clone(),
            CalendarSource::Outlook { calendar_id, .. } => calendar_id.clone(),
            CalendarSource::CalDAV { calendar_url, .. } => calendar_url.clone(),
            CalendarSource::Local => "local".to_string(),
        }
    }
}

/// Calendar metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Calendar {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub source: CalendarSource,
    pub read_only: bool,
    pub timezone: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_synced: Option<DateTime<Utc>>,
}

impl Calendar {
    pub fn new(id: String, name: String, source: CalendarSource) -> Self {
        let now = Utc::now();
        Self {
            id,
            name,
            description: None,
            color: None,
            source,
            read_only: false,
            timezone: "UTC".to_string(),
            created_at: now,
            updated_at: now,
            last_synced: None,
        }
    }
}

/// Calendar statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarStats {
    pub total_calendars: usize,
    pub total_events: usize,
    pub upcoming_events: usize,
    pub overdue_events: usize,
    pub google_calendars: usize,
    pub outlook_calendars: usize,
    pub caldav_calendars: usize,
    pub local_calendars: usize,
    pub last_sync: Option<DateTime<Utc>>,
}

impl Default for CalendarStats {
    fn default() -> Self {
        Self {
            total_calendars: 0,
            total_events: 0,
            upcoming_events: 0,
            overdue_events: 0,
            google_calendars: 0,
            outlook_calendars: 0,
            caldav_calendars: 0,
            local_calendars: 0,
            last_sync: None,
        }
    }
}