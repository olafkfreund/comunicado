use sqlx::{SqlitePool, Row, sqlite::SqlitePoolOptions};
use sqlx::migrate::MigrateDatabase;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use crate::calendar::{Calendar, CalendarSource};
use crate::calendar::event::{Event, EventStatus, EventPriority, EventAttendee, AttendeeStatus, AttendeeRole, EventRecurrence, EventReminder};
use thiserror::Error;

/// Calendar database-related errors
#[derive(Error, Debug)]
pub enum CalendarDatabaseError {
    #[error("Database connection error: {0}")]
    Connection(#[from] sqlx::Error),
    
    #[error("Migration error: {0}")]
    Migration(String),
    
    #[error("Query error: {0}")]
    Query(String),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("UUID error: {0}")]
    Uuid(#[from] uuid::Error),
    
    #[error("Date parsing error: {0}")]
    DateParse(#[from] chrono::ParseError),
    
    #[error("Parse error: {0}")]
    ParseError(String),
}

pub type CalendarDatabaseResult<T> = Result<T, CalendarDatabaseError>;

/// Stored calendar event in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub id: String,
    pub uid: String, // iCalendar UID
    pub calendar_id: String,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub all_day: bool,
    pub status: String, // Serialized EventStatus
    pub priority: u8,
    pub organizer_email: Option<String>,
    pub organizer_name: Option<String>,
    pub attendees: Vec<CalendarEventAttendee>,
    pub recurrence_rule: Option<String>, // Serialized recurrence
    pub reminders: Vec<CalendarEventReminder>,
    pub categories: Vec<String>,
    pub url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub sequence: u32,
    pub etag: Option<String>,
    pub sync_status: String, // local, synced, pending, error
}

/// Stored event attendee
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEventAttendee {
    pub email: String,
    pub name: Option<String>,
    pub status: String, // Serialized AttendeeStatus
    pub role: String, // Serialized AttendeeRole
    pub rsvp: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Stored event reminder
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEventReminder {
    pub trigger_minutes: i32, // Minutes before event (negative for before, positive for after)
    pub action: String, // display, email, audio
    pub description: Option<String>,
    pub attendees: Vec<String>, // Email addresses
}

/// Stored event recurrence pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEventRecurrence {
    pub frequency: String, // daily, weekly, monthly, yearly
    pub interval: u32,
    pub count: Option<u32>,
    pub until: Option<DateTime<Utc>>,
    pub by_day: Vec<String>, // MO, TU, WE, etc.
    pub by_month_day: Vec<i8>,
    pub by_month: Vec<u8>,
    pub week_start: String,
}

/// Calendar database manager
pub struct CalendarDatabase {
    pub pool: SqlitePool,
    #[allow(dead_code)]
    db_path: String,
}

impl CalendarDatabase {
    /// Create a new calendar database
    pub async fn new(db_path: &str) -> CalendarDatabaseResult<Self> {
        // Create database if it doesn't exist
        if !sqlx::Sqlite::database_exists(db_path).await.unwrap_or(false) {
            sqlx::Sqlite::create_database(db_path).await
                .map_err(|e| CalendarDatabaseError::Migration(format!("Failed to create database: {}", e)))?;
        }
        
        // Create connection pool
        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .connect(db_path)
            .await
            .map_err(CalendarDatabaseError::Connection)?;
        
        let db = Self {
            pool,
            db_path: db_path.to_string(),
        };
        
        // Run migrations
        db.migrate().await?;
        
        Ok(db)
    }
    
    /// Create a new in-memory calendar database for testing
    pub async fn new_in_memory() -> CalendarDatabaseResult<Self> {
        // Create connection pool for in-memory database
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(":memory:")
            .await
            .map_err(CalendarDatabaseError::Connection)?;
        
        let db = Self {
            pool,
            db_path: ":memory:".to_string(),
        };
        
        // Run migrations
        db.migrate().await?;
        
        Ok(db)
    }
    
    /// Run database migrations
    async fn migrate(&self) -> CalendarDatabaseResult<()> {
        // Enable foreign key constraints
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&self.pool)
            .await?;
        // Create calendars table
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS calendars (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                color TEXT,
                source_type TEXT NOT NULL, -- google, outlook, caldav, local
                source_data TEXT NOT NULL, -- JSON with source-specific data
                read_only BOOLEAN NOT NULL DEFAULT FALSE,
                timezone TEXT NOT NULL DEFAULT 'UTC',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                last_synced TEXT
            )
        "#).execute(&self.pool).await?;
        
        // Create events table
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS calendar_events (
                id TEXT PRIMARY KEY,
                uid TEXT NOT NULL, -- iCalendar UID
                calendar_id TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT,
                location TEXT,
                start_time TEXT NOT NULL,
                end_time TEXT NOT NULL,
                all_day BOOLEAN NOT NULL DEFAULT FALSE,
                status TEXT NOT NULL DEFAULT 'confirmed',
                priority INTEGER NOT NULL DEFAULT 5,
                organizer_email TEXT,
                organizer_name TEXT,
                attendees TEXT NOT NULL DEFAULT '[]', -- JSON array
                recurrence_rule TEXT, -- JSON or RRULE string
                reminders TEXT NOT NULL DEFAULT '[]', -- JSON array
                categories TEXT NOT NULL DEFAULT '[]', -- JSON array
                url TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                sequence INTEGER NOT NULL DEFAULT 0,
                etag TEXT,
                sync_status TEXT NOT NULL DEFAULT 'local'
            )
        "#).execute(&self.pool).await?;
        
        // Create calendar_sync_state table
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS calendar_sync_state (
                calendar_id TEXT PRIMARY KEY,
                last_sync TEXT NOT NULL,
                sync_token TEXT,
                ctag TEXT,
                sync_status TEXT NOT NULL, -- syncing, idle, error
                error_message TEXT,
                events_synced INTEGER NOT NULL DEFAULT 0
            )
        "#).execute(&self.pool).await?;
        
        // Create indexes for performance
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_calendar ON calendar_events(calendar_id)").execute(&self.pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_uid ON calendar_events(uid)").execute(&self.pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_start_time ON calendar_events(start_time)").execute(&self.pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_end_time ON calendar_events(end_time)").execute(&self.pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_status ON calendar_events(status)").execute(&self.pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_sync_status ON calendar_events(sync_status)").execute(&self.pool).await?;
        
        // Full-text search for events
        sqlx::query(r#"
            CREATE VIRTUAL TABLE IF NOT EXISTS calendar_events_fts USING fts5(
                event_id UNINDEXED,
                title,
                description,
                location,
                content='calendar_events',
                content_rowid='rowid'
            )
        "#).execute(&self.pool).await?;
        
        // Triggers to keep FTS table in sync
        sqlx::query(r#"
            CREATE TRIGGER IF NOT EXISTS calendar_events_fts_insert AFTER INSERT ON calendar_events BEGIN
                INSERT INTO calendar_events_fts(rowid, event_id, title, description, location)
                VALUES (new.rowid, new.id, new.title, new.description, new.location);
            END
        "#).execute(&self.pool).await?;
        
        sqlx::query(r#"
            CREATE TRIGGER IF NOT EXISTS calendar_events_fts_delete AFTER DELETE ON calendar_events BEGIN
                INSERT INTO calendar_events_fts(calendar_events_fts, rowid, event_id, title, description, location)
                VALUES ('delete', old.rowid, old.id, old.title, old.description, old.location);
            END
        "#).execute(&self.pool).await?;
        
        sqlx::query(r#"
            CREATE TRIGGER IF NOT EXISTS calendar_events_fts_update AFTER UPDATE ON calendar_events BEGIN
                INSERT INTO calendar_events_fts(calendar_events_fts, rowid, event_id, title, description, location)
                VALUES ('delete', old.rowid, old.id, old.title, old.description, old.location);
                INSERT INTO calendar_events_fts(rowid, event_id, title, description, location)
                VALUES (new.rowid, new.id, new.title, new.description, new.location);
            END
        "#).execute(&self.pool).await?;
        
        Ok(())
    }
    
    /// Store a calendar
    pub async fn store_calendar(&self, calendar: &Calendar) -> CalendarDatabaseResult<()> {
        let source_data = serde_json::to_string(&calendar.source)?;
        
        sqlx::query(r#"
            INSERT OR REPLACE INTO calendars (
                id, name, description, color, source_type, source_data,
                read_only, timezone, created_at, updated_at, last_synced
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
        "#)
        .bind(&calendar.id)
        .bind(&calendar.name)
        .bind(&calendar.description)
        .bind(&calendar.color)
        .bind(calendar.source.provider_name())
        .bind(source_data)
        .bind(calendar.read_only)
        .bind(&calendar.timezone)
        .bind(calendar.created_at.to_rfc3339())
        .bind(calendar.updated_at.to_rfc3339())
        .bind(calendar.last_synced.map(|dt| dt.to_rfc3339()))
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    /// Get all calendars
    pub async fn get_calendars(&self) -> CalendarDatabaseResult<Vec<Calendar>> {
        let rows = sqlx::query(r#"
            SELECT id, name, description, color, source_type, source_data,
                   read_only, timezone, created_at, updated_at, last_synced
            FROM calendars
            ORDER BY name
        "#)
        .fetch_all(&self.pool)
        .await?;
        
        let mut calendars = Vec::new();
        for row in rows {
            let source_data: String = row.get("source_data");
            let source: CalendarSource = serde_json::from_str(&source_data)?;
            
            let created_at: DateTime<Utc> = DateTime::parse_from_rfc3339(row.get("created_at"))?.into();
            let updated_at: DateTime<Utc> = DateTime::parse_from_rfc3339(row.get("updated_at"))?.into();
            let last_synced: Option<DateTime<Utc>> = row.get::<Option<String>, _>("last_synced")
                .map(|s| DateTime::parse_from_rfc3339(&s))
                .transpose()?
                .map(|dt| dt.into());
            
            calendars.push(Calendar {
                id: row.get("id"),
                name: row.get("name"),
                description: row.get("description"),
                color: row.get("color"),
                source,
                read_only: row.get("read_only"),
                timezone: row.get("timezone"),
                created_at,
                updated_at,
                last_synced,
            });
        }
        
        Ok(calendars)
    }
    
    /// Store an event
    pub async fn store_event(&self, event: &Event) -> CalendarDatabaseResult<()> {
        let attendees_json = serde_json::to_string(&event.attendees.iter()
            .map(|a| CalendarEventAttendee {
                email: a.email.clone(),
                name: a.name.clone(),
                status: a.status.to_icalendar().to_string(),
                role: format!("{:?}", a.role).to_uppercase(),
                rsvp: a.rsvp,
                created_at: a.created_at,
                updated_at: a.updated_at,
            })
            .collect::<Vec<_>>())?;
        
        let reminders_json = serde_json::to_string(&event.reminders.iter()
            .map(|r| CalendarEventReminder {
                trigger_minutes: match &r.trigger {
                    crate::calendar::event::ReminderTrigger::BeforeStart(duration) => -(duration.num_minutes() as i32),
                    crate::calendar::event::ReminderTrigger::BeforeEnd(duration) => -(duration.num_minutes() as i32),
                    crate::calendar::event::ReminderTrigger::AtStart => 0,
                    crate::calendar::event::ReminderTrigger::AtEnd => 0,
                },
                action: r.action.to_icalendar().to_string(),
                description: r.description.clone(),
                attendees: r.attendees.clone(),
            })
            .collect::<Vec<_>>())?;
        
        let categories_json = serde_json::to_string(&event.categories)?;
        let recurrence_json = event.recurrence.as_ref()
            .map(|r| serde_json::to_string(r))
            .transpose()?;
        
        sqlx::query(r#"
            INSERT OR REPLACE INTO calendar_events (
                id, uid, calendar_id, title, description, location,
                start_time, end_time, all_day, status, priority,
                organizer_email, organizer_name, attendees, recurrence_rule,
                reminders, categories, url, created_at, updated_at,
                sequence, etag, sync_status
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20,
                ?21, ?22, ?23
            )
        "#)
        .bind(&event.id)
        .bind(&event.uid)
        .bind(&event.calendar_id)
        .bind(&event.title)
        .bind(&event.description)
        .bind(&event.location)
        .bind(event.start_time.to_rfc3339())
        .bind(event.end_time.to_rfc3339())
        .bind(event.all_day)
        .bind(event.status.to_icalendar())
        .bind(event.priority.to_number())
        .bind(event.organizer.as_ref().map(|o| &o.email))
        .bind(event.organizer.as_ref().and_then(|o| o.name.as_ref()))
        .bind(attendees_json)
        .bind(recurrence_json)
        .bind(reminders_json)
        .bind(categories_json)
        .bind(&event.url)
        .bind(event.created_at.to_rfc3339())
        .bind(event.updated_at.to_rfc3339())
        .bind(event.sequence as i64)
        .bind(&event.etag)
        .bind("local") // Default sync status
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    /// Get events from a calendar within a date range
    pub async fn get_events(&self, calendar_id: &str, start_time: Option<DateTime<Utc>>, end_time: Option<DateTime<Utc>>) -> CalendarDatabaseResult<Vec<Event>> {
        let mut query = String::from(r#"
            SELECT id, uid, calendar_id, title, description, location,
                   start_time, end_time, all_day, status, priority,
                   organizer_email, organizer_name, attendees, recurrence_rule,
                   reminders, categories, url, created_at, updated_at,
                   sequence, etag
            FROM calendar_events
            WHERE calendar_id = ?1
        "#);
        
        let mut bind_count = 1;
        
        if start_time.is_some() {
            bind_count += 1;
            query.push_str(&format!(" AND end_time >= ?{}", bind_count));
        }
        
        if end_time.is_some() {
            bind_count += 1;
            query.push_str(&format!(" AND start_time <= ?{}", bind_count));
        }
        
        query.push_str(" ORDER BY start_time ASC");
        
        let mut query_builder = sqlx::query(&query).bind(calendar_id);
        
        if let Some(start) = start_time {
            query_builder = query_builder.bind(start.to_rfc3339());
        }
        
        if let Some(end) = end_time {
            query_builder = query_builder.bind(end.to_rfc3339());
        }
        
        let rows = query_builder.fetch_all(&self.pool).await?;
        
        let mut events = Vec::new();
        for row in rows {
            events.push(self.row_to_event(row)?);
        }
        
        Ok(events)
    }
    
    /// Get a single event by ID
    pub async fn get_event(&self, event_id: &str) -> CalendarDatabaseResult<Option<Event>> {
        let row = sqlx::query(r#"
            SELECT id, uid, calendar_id, title, description, location,
                   start_time, end_time, all_day, status, priority,
                   organizer_email, organizer_name, attendees, recurrence_rule,
                   reminders, categories, url, created_at, updated_at,
                   sequence, etag
            FROM calendar_events
            WHERE id = ?1
        "#)
        .bind(event_id)
        .fetch_optional(&self.pool)
        .await?;
        
        match row {
            Some(row) => Ok(Some(self.row_to_event(row)?)),
            None => Ok(None),
        }
    }
    
    /// Search events with full-text search
    pub async fn search_events(&self, query: &str, limit: Option<u32>) -> CalendarDatabaseResult<Vec<Event>> {
        let limit = limit.unwrap_or(50) as i64;
        
        let rows = sqlx::query(r#"
            SELECT e.id, e.uid, e.calendar_id, e.title, e.description, e.location,
                   e.start_time, e.end_time, e.all_day, e.status, e.priority,
                   e.organizer_email, e.organizer_name, e.attendees, e.recurrence_rule,
                   e.reminders, e.categories, e.url, e.created_at, e.updated_at,
                   e.sequence, e.etag
            FROM calendar_events e
            JOIN calendar_events_fts fts ON e.rowid = fts.rowid
            WHERE calendar_events_fts MATCH ?1
            ORDER BY rank
            LIMIT ?2
        "#)
        .bind(query)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        
        let mut events = Vec::new();
        for row in rows {
            events.push(self.row_to_event(row)?);
        }
        
        Ok(events)
    }
    
    /// Delete an event
    pub async fn delete_event(&self, event_id: &str) -> CalendarDatabaseResult<bool> {
        let result = sqlx::query("DELETE FROM calendar_events WHERE id = ?")
            .bind(event_id)
            .execute(&self.pool)
            .await?;
        
        Ok(result.rows_affected() > 0)
    }
    
    /// Get upcoming events (starting within the next N hours)
    pub async fn get_upcoming_events(&self, hours: u32, limit: Option<u32>) -> CalendarDatabaseResult<Vec<Event>> {
        let now = Utc::now();
        let end_time = now + chrono::Duration::hours(hours as i64);
        let limit = limit.unwrap_or(10) as i64;
        
        let rows = sqlx::query(r#"
            SELECT id, uid, calendar_id, title, description, location,
                   start_time, end_time, all_day, status, priority,
                   organizer_email, organizer_name, attendees, recurrence_rule,
                   reminders, categories, url, created_at, updated_at,
                   sequence, etag
            FROM calendar_events
            WHERE start_time >= ?1 AND start_time <= ?2
            ORDER BY start_time ASC
            LIMIT ?3
        "#)
        .bind(now.to_rfc3339())
        .bind(end_time.to_rfc3339())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        
        let mut events = Vec::new();
        for row in rows {
            events.push(self.row_to_event(row)?);
        }
        
        Ok(events)
    }
    
    /// Convert database row to Event
    fn row_to_event(&self, row: sqlx::sqlite::SqliteRow) -> CalendarDatabaseResult<Event> {
        let attendees_json: String = row.get("attendees");
        let reminders_json: String = row.get("reminders");
        let categories_json: String = row.get("categories");
        
        let stored_attendees: Vec<CalendarEventAttendee> = serde_json::from_str(&attendees_json)?;
        let stored_reminders: Vec<CalendarEventReminder> = serde_json::from_str(&reminders_json)?;
        let categories: Vec<String> = serde_json::from_str(&categories_json)?;
        
        // Convert stored attendees back to EventAttendee
        let attendees = stored_attendees.into_iter().map(|a| EventAttendee {
            email: a.email,
            name: a.name,
            status: AttendeeStatus::from_icalendar(&a.status),
            role: match a.role.to_uppercase().as_str() {
                "CHAIR" => AttendeeRole::Chair,
                "REQ-PARTICIPANT" => AttendeeRole::RequiredParticipant,
                "OPT-PARTICIPANT" => AttendeeRole::OptionalParticipant,
                "NON-PARTICIPANT" => AttendeeRole::NonParticipant,
                _ => AttendeeRole::RequiredParticipant,
            },
            rsvp: a.rsvp,
            created_at: a.created_at,
            updated_at: a.updated_at,
        }).collect();
        
        // Convert stored reminders back to EventReminder
        let reminders = stored_reminders.into_iter().map(|r| EventReminder {
            trigger: if r.trigger_minutes < 0 {
                crate::calendar::event::ReminderTrigger::BeforeStart(
                    chrono::Duration::minutes(-r.trigger_minutes as i64)
                )
            } else if r.trigger_minutes > 0 {
                crate::calendar::event::ReminderTrigger::BeforeStart(
                    chrono::Duration::minutes(r.trigger_minutes as i64)
                )
            } else {
                crate::calendar::event::ReminderTrigger::AtStart
            },
            action: match r.action.to_uppercase().as_str() {
                "DISPLAY" => crate::calendar::event::ReminderAction::Display,
                "EMAIL" => crate::calendar::event::ReminderAction::Email,
                "AUDIO" => crate::calendar::event::ReminderAction::Audio,
                _ => crate::calendar::event::ReminderAction::Display,
            },
            description: r.description,
            attendees: r.attendees,
        }).collect();
        
        let organizer = if let (Some(email), name) = (row.get::<Option<String>, _>("organizer_email"), row.get::<Option<String>, _>("organizer_name")) {
            Some(EventAttendee::organizer(email, name))
        } else {
            None
        };
        
        let recurrence = row.get::<Option<String>, _>("recurrence_rule")
            .map(|rule_string| {
                // Try parsing as RRULE first (starts with FREQ= or RRULE:)
                if rule_string.starts_with("FREQ=") || rule_string.starts_with("RRULE:") {
                    EventRecurrence::from_icalendar(&rule_string)
                        .map_err(|e| CalendarDatabaseError::ParseError(format!("Failed to parse RRULE: {}", e)))
                } else {
                    // Try parsing as JSON for backward compatibility
                    serde_json::from_str::<EventRecurrence>(&rule_string)
                        .map_err(|e| CalendarDatabaseError::ParseError(format!("Failed to parse recurrence JSON: {}", e)))
                }
            })
            .transpose()?;
        
        let start_time: DateTime<Utc> = DateTime::parse_from_rfc3339(row.get("start_time"))?.into();
        let end_time: DateTime<Utc> = DateTime::parse_from_rfc3339(row.get("end_time"))?.into();
        let created_at: DateTime<Utc> = DateTime::parse_from_rfc3339(row.get("created_at"))?.into();
        let updated_at: DateTime<Utc> = DateTime::parse_from_rfc3339(row.get("updated_at"))?.into();
        
        Ok(Event {
            id: row.get("id"),
            uid: row.get("uid"),
            calendar_id: row.get("calendar_id"),
            title: row.get("title"),
            description: row.get("description"),
            location: row.get("location"),
            start_time,
            end_time,
            all_day: row.get("all_day"),
            status: EventStatus::from_icalendar(row.get("status")),
            priority: EventPriority::from_number(row.get::<i64, _>("priority") as u8),
            organizer,
            attendees,
            recurrence,
            reminders,
            categories,
            url: row.get("url"),
            created_at,
            updated_at,
            sequence: row.get::<i64, _>("sequence") as u32,
            etag: row.get("etag"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    
    #[tokio::test]
    async fn test_database_creation() {
        let db = CalendarDatabase::new_in_memory().await.unwrap();
        
        // Test calendar storage
        let calendar = Calendar::new(
            "test-cal".to_string(),
            "Test Calendar".to_string(),
            CalendarSource::Local,
        );
        
        db.store_calendar(&calendar).await.unwrap();
        
        let calendars = db.get_calendars().await.unwrap();
        assert_eq!(calendars.len(), 1);
        assert_eq!(calendars[0].name, "Test Calendar");
    }
    
    #[tokio::test]
    async fn test_event_storage_and_retrieval() {
        let db = CalendarDatabase::new_in_memory().await.unwrap();
        
        // Create a test calendar first
        let calendar = Calendar::new(
            "test-cal".to_string(),
            "Test Calendar".to_string(),
            CalendarSource::Local,
        );
        db.store_calendar(&calendar).await.unwrap();
        
        // Create and store an event
        let start = Utc.with_ymd_and_hms(2025, 1, 28, 10, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2025, 1, 28, 11, 0, 0).unwrap();
        
        let event = Event::new(
            "test-cal".to_string(),
            "Test Meeting".to_string(),
            start,
            end,
        );
        
        db.store_event(&event).await.unwrap();
        
        // Retrieve events
        let events = db.get_events("test-cal", None, None).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].title, "Test Meeting");
        assert_eq!(events[0].calendar_id, "test-cal");
    }
    
    #[tokio::test]
    async fn test_event_search() {
        let db = CalendarDatabase::new_in_memory().await.unwrap();
        
        // Create a test calendar
        let calendar = Calendar::new(
            "test-cal".to_string(),
            "Test Calendar".to_string(),
            CalendarSource::Local,
        );
        db.store_calendar(&calendar).await.unwrap();
        
        // Create events with searchable content
        let start = Utc.with_ymd_and_hms(2025, 1, 28, 10, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2025, 1, 28, 11, 0, 0).unwrap();
        
        let mut event1 = Event::new(
            "test-cal".to_string(),
            "Important Meeting".to_string(),
            start,
            end,
        );
        event1.description = Some("Quarterly review meeting".to_string());
        event1.location = Some("Conference Room A".to_string());
        
        let mut event2 = Event::new(
            "test-cal".to_string(),
            "Team Standup".to_string(),
            start + chrono::Duration::hours(1),
            end + chrono::Duration::hours(1),
        );
        event2.description = Some("Daily standup meeting".to_string());
        
        db.store_event(&event1).await.unwrap();
        db.store_event(&event2).await.unwrap();
        
        // Search for events
        let results = db.search_events("meeting", None).await.unwrap();
        assert_eq!(results.len(), 2);
        
        let results = db.search_events("quarterly", None).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Important Meeting");
        
        let results = db.search_events("conference", None).await.unwrap();
        assert_eq!(results.len(), 1);
    }
}