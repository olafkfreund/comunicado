use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Calendar event representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub uid: String, // iCalendar UID
    pub calendar_id: String,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub all_day: bool,
    pub status: EventStatus,
    pub priority: EventPriority,
    pub organizer: Option<EventAttendee>,
    pub attendees: Vec<EventAttendee>,
    pub recurrence: Option<EventRecurrence>,
    pub reminders: Vec<EventReminder>,
    pub categories: Vec<String>,
    pub url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub sequence: u32, // iCalendar SEQUENCE for updates
    pub etag: Option<String>, // CalDAV ETag for sync
}

impl Event {
    /// Create a new event
    pub fn new(calendar_id: String, title: String, start_time: DateTime<Utc>, end_time: DateTime<Utc>) -> Self {
        let now = Utc::now();
        let uid = Uuid::new_v4().to_string();
        
        Self {
            id: uid.clone(),
            uid,
            calendar_id,
            title,
            description: None,
            location: None,
            start_time,
            end_time,
            all_day: false,
            status: EventStatus::Confirmed,
            priority: EventPriority::Normal,
            organizer: None,
            attendees: Vec::new(),
            recurrence: None,
            reminders: Vec::new(),
            categories: Vec::new(),
            url: None,
            created_at: now,
            updated_at: now,
            sequence: 0,
            etag: None,
        }
    }
    
    /// Create an all-day event
    pub fn new_all_day(calendar_id: String, title: String, date: DateTime<Utc>) -> Self {
        let start_time = date.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
        let end_time = start_time + Duration::days(1);
        
        let mut event = Self::new(calendar_id, title, start_time, end_time);
        event.all_day = true;
        event
    }
    
    /// Check if the event is happening now
    pub fn is_current(&self) -> bool {
        let now = Utc::now();
        now >= self.start_time && now <= self.end_time
    }
    
    /// Check if the event is in the future
    pub fn is_upcoming(&self) -> bool {
        Utc::now() < self.start_time
    }
    
    /// Check if the event is in the past
    pub fn is_past(&self) -> bool {
        Utc::now() > self.end_time
    }
    
    /// Get event duration
    pub fn duration(&self) -> Duration {
        self.end_time - self.start_time
    }
    
    /// Check if event overlaps with another event
    pub fn overlaps_with(&self, other: &Event) -> bool {
        self.start_time < other.end_time && self.end_time > other.start_time
    }
    
    /// Add an attendee to the event
    pub fn add_attendee(&mut self, attendee: EventAttendee) {
        self.attendees.push(attendee);
        self.updated_at = Utc::now();
        self.sequence += 1;
    }
    
    /// Remove an attendee from the event
    pub fn remove_attendee(&mut self, email: &str) -> bool {
        let original_len = self.attendees.len();
        self.attendees.retain(|a| a.email != email);
        
        if self.attendees.len() != original_len {
            self.updated_at = Utc::now();
            self.sequence += 1;
            true
        } else {
            false
        }
    }
    
    /// Update attendee status
    pub fn update_attendee_status(&mut self, email: &str, status: AttendeeStatus) -> bool {
        if let Some(attendee) = self.attendees.iter_mut().find(|a| a.email == email) {
            attendee.status = status;
            attendee.updated_at = Utc::now();
            self.updated_at = Utc::now();
            self.sequence += 1;
            true
        } else {
            false
        }
    }
    
    /// Add a reminder to the event
    pub fn add_reminder(&mut self, reminder: EventReminder) {
        self.reminders.push(reminder);
        self.updated_at = Utc::now();
        self.sequence += 1;
    }
    
    /// Convert to iCalendar format
    pub fn to_icalendar(&self) -> String {
        let mut ical = String::new();
        
        ical.push_str("BEGIN:VCALENDAR\r\n");
        ical.push_str("VERSION:2.0\r\n");
        ical.push_str("PRODID:-//Comunicado//Calendar//EN\r\n");
        ical.push_str("BEGIN:VEVENT\r\n");
        
        ical.push_str(&format!("UID:{}\r\n", self.uid));
        ical.push_str(&format!("SUMMARY:{}\r\n", self.title));
        
        if let Some(ref description) = self.description {
            ical.push_str(&format!("DESCRIPTION:{}\r\n", description));
        }
        
        if let Some(ref location) = self.location {
            ical.push_str(&format!("LOCATION:{}\r\n", location));
        }
        
        if self.all_day {
            ical.push_str(&format!("DTSTART;VALUE=DATE:{}\r\n", 
                self.start_time.format("%Y%m%d")));
            ical.push_str(&format!("DTEND;VALUE=DATE:{}\r\n", 
                self.end_time.format("%Y%m%d")));
        } else {
            ical.push_str(&format!("DTSTART:{}\r\n", 
                self.start_time.format("%Y%m%dT%H%M%SZ")));
            ical.push_str(&format!("DTEND:{}\r\n", 
                self.end_time.format("%Y%m%dT%H%M%SZ")));
        }
        
        ical.push_str(&format!("CREATED:{}\r\n", 
            self.created_at.format("%Y%m%dT%H%M%SZ")));
        ical.push_str(&format!("LAST-MODIFIED:{}\r\n", 
            self.updated_at.format("%Y%m%dT%H%M%SZ")));
        ical.push_str(&format!("SEQUENCE:{}\r\n", self.sequence));
        ical.push_str(&format!("STATUS:{}\r\n", self.status.to_icalendar()));
        
        if let Some(ref organizer) = self.organizer {
            ical.push_str(&format!("ORGANIZER;CN={}:mailto:{}\r\n", 
                organizer.name.as_deref().unwrap_or(&organizer.email), 
                organizer.email));
        }
        
        for attendee in &self.attendees {
            ical.push_str(&format!("ATTENDEE;CN={};PARTSTAT={}:mailto:{}\r\n",
                attendee.name.as_deref().unwrap_or(&attendee.email),
                attendee.status.to_icalendar(),
                attendee.email));
        }
        
        if let Some(ref recurrence) = self.recurrence {
            ical.push_str(&format!("RRULE:{}\r\n", recurrence.to_icalendar()));
        }
        
        for reminder in &self.reminders {
            ical.push_str("BEGIN:VALARM\r\n");
            ical.push_str(&format!("TRIGGER:{}\r\n", reminder.to_icalendar()));
            ical.push_str(&format!("ACTION:{}\r\n", reminder.action.to_icalendar()));
            if let Some(ref description) = reminder.description {
                ical.push_str(&format!("DESCRIPTION:{}\r\n", description));
            }
            ical.push_str("END:VALARM\r\n");
        }
        
        ical.push_str("END:VEVENT\r\n");
        ical.push_str("END:VCALENDAR\r\n");
        
        ical
    }
}

/// Event status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventStatus {
    Tentative,
    Confirmed,
    Cancelled,
}

impl EventStatus {
    pub fn to_icalendar(&self) -> &str {
        match self {
            EventStatus::Tentative => "TENTATIVE",
            EventStatus::Confirmed => "CONFIRMED",
            EventStatus::Cancelled => "CANCELLED",
        }
    }
    
    pub fn from_icalendar(value: &str) -> Self {
        match value.to_uppercase().as_str() {
            "TENTATIVE" => EventStatus::Tentative,
            "CONFIRMED" => EventStatus::Confirmed,
            "CANCELLED" => EventStatus::Cancelled,
            _ => EventStatus::Confirmed,
        }
    }
}

/// Event priority
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventPriority {
    High,
    Normal,
    Low,
}

impl EventPriority {
    pub fn to_number(&self) -> u8 {
        match self {
            EventPriority::High => 1,
            EventPriority::Normal => 5,
            EventPriority::Low => 9,
        }
    }
    
    pub fn from_number(value: u8) -> Self {
        match value {
            1..=3 => EventPriority::High,
            4..=6 => EventPriority::Normal,
            7..=9 => EventPriority::Low,
            _ => EventPriority::Normal,
        }
    }
}

/// Event attendee
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventAttendee {
    pub email: String,
    pub name: Option<String>,
    pub status: AttendeeStatus,
    pub role: AttendeeRole,
    pub rsvp: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl EventAttendee {
    pub fn new(email: String, name: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            email,
            name,
            status: AttendeeStatus::NeedsAction,
            role: AttendeeRole::RequiredParticipant,
            rsvp: true,
            created_at: now,
            updated_at: now,
        }
    }
    
    pub fn organizer(email: String, name: Option<String>) -> Self {
        let mut attendee = Self::new(email, name);
        attendee.role = AttendeeRole::Chair;
        attendee.status = AttendeeStatus::Accepted;
        attendee.rsvp = false;
        attendee
    }
}

/// Attendee status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttendeeStatus {
    NeedsAction,
    Accepted,
    Declined,
    Tentative,
    Delegated,
}

impl AttendeeStatus {
    pub fn to_icalendar(&self) -> &str {
        match self {
            AttendeeStatus::NeedsAction => "NEEDS-ACTION",
            AttendeeStatus::Accepted => "ACCEPTED",
            AttendeeStatus::Declined => "DECLINED",
            AttendeeStatus::Tentative => "TENTATIVE",
            AttendeeStatus::Delegated => "DELEGATED",
        }
    }
    
    pub fn from_icalendar(value: &str) -> Self {
        match value.to_uppercase().as_str() {
            "NEEDS-ACTION" => AttendeeStatus::NeedsAction,
            "ACCEPTED" => AttendeeStatus::Accepted,
            "DECLINED" => AttendeeStatus::Declined,
            "TENTATIVE" => AttendeeStatus::Tentative,
            "DELEGATED" => AttendeeStatus::Delegated,
            _ => AttendeeStatus::NeedsAction,
        }
    }
}

/// Attendee role
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttendeeRole {
    Chair,
    RequiredParticipant,
    OptionalParticipant,
    NonParticipant,
}

/// Event recurrence pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecurrence {
    pub frequency: RecurrenceFrequency,
    pub interval: u32,
    pub count: Option<u32>,
    pub until: Option<DateTime<Utc>>,
    pub by_day: Vec<RecurrenceDay>,
    pub by_month_day: Vec<i8>,
    pub by_month: Vec<u8>,
    pub by_week_no: Vec<i8>,
    pub by_year_day: Vec<i16>,
    pub week_start: RecurrenceDay,
}

impl EventRecurrence {
    pub fn daily(interval: u32) -> Self {
        Self {
            frequency: RecurrenceFrequency::Daily,
            interval,
            count: None,
            until: None,
            by_day: Vec::new(),
            by_month_day: Vec::new(),
            by_month: Vec::new(),
            by_week_no: Vec::new(),
            by_year_day: Vec::new(),
            week_start: RecurrenceDay::Monday,
        }
    }
    
    pub fn weekly(interval: u32, days: Vec<RecurrenceDay>) -> Self {
        Self {
            frequency: RecurrenceFrequency::Weekly,
            interval,
            count: None,
            until: None,
            by_day: days,
            by_month_day: Vec::new(),
            by_month: Vec::new(),
            by_week_no: Vec::new(),
            by_year_day: Vec::new(),
            week_start: RecurrenceDay::Monday,
        }
    }
    
    pub fn monthly(interval: u32) -> Self {
        Self {
            frequency: RecurrenceFrequency::Monthly,
            interval,
            count: None,
            until: None,
            by_day: Vec::new(),
            by_month_day: Vec::new(),
            by_month: Vec::new(),
            by_week_no: Vec::new(),
            by_year_day: Vec::new(),
            week_start: RecurrenceDay::Monday,
        }
    }
    
    pub fn to_icalendar(&self) -> String {
        let mut rrule = format!("FREQ={}", self.frequency.to_icalendar());
        
        if self.interval > 1 {
            rrule.push_str(&format!(";INTERVAL={}", self.interval));
        }
        
        if let Some(count) = self.count {
            rrule.push_str(&format!(";COUNT={}", count));
        }
        
        if let Some(until) = self.until {
            rrule.push_str(&format!(";UNTIL={}", until.format("%Y%m%dT%H%M%SZ")));
        }
        
        if !self.by_day.is_empty() {
            let days: Vec<String> = self.by_day.iter()
                .map(|d| d.to_icalendar().to_string())
                .collect();
            rrule.push_str(&format!(";BYDAY={}", days.join(",")));
        }
        
        if !self.by_month_day.is_empty() {
            let days: Vec<String> = self.by_month_day.iter()
                .map(|d| d.to_string())
                .collect();
            rrule.push_str(&format!(";BYMONTHDAY={}", days.join(",")));
        }
        
        rrule
    }
}

/// Recurrence frequency
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecurrenceFrequency {
    Secondly,
    Minutely,
    Hourly,
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

impl RecurrenceFrequency {
    pub fn to_icalendar(&self) -> &str {
        match self {
            RecurrenceFrequency::Secondly => "SECONDLY",
            RecurrenceFrequency::Minutely => "MINUTELY",
            RecurrenceFrequency::Hourly => "HOURLY",
            RecurrenceFrequency::Daily => "DAILY",
            RecurrenceFrequency::Weekly => "WEEKLY",
            RecurrenceFrequency::Monthly => "MONTHLY",
            RecurrenceFrequency::Yearly => "YEARLY",
        }
    }
}

/// Day of week for recurrence
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecurrenceDay {
    Sunday,
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
}

impl RecurrenceDay {
    pub fn to_icalendar(&self) -> &str {
        match self {
            RecurrenceDay::Sunday => "SU",
            RecurrenceDay::Monday => "MO",
            RecurrenceDay::Tuesday => "TU",
            RecurrenceDay::Wednesday => "WE",
            RecurrenceDay::Thursday => "TH",
            RecurrenceDay::Friday => "FR",
            RecurrenceDay::Saturday => "SA",
        }
    }
}

/// Event reminder
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventReminder {
    pub trigger: ReminderTrigger,
    pub action: ReminderAction,
    pub description: Option<String>,
    pub attendees: Vec<String>, // Email addresses
}

impl EventReminder {
    pub fn new(trigger: ReminderTrigger, action: ReminderAction) -> Self {
        Self {
            trigger,
            action,
            description: None,
            attendees: Vec::new(),
        }
    }
    
    pub fn to_icalendar(&self) -> String {
        self.trigger.to_icalendar()
    }
}

/// Reminder trigger timing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReminderTrigger {
    BeforeStart(Duration),
    BeforeEnd(Duration),
    AtStart,
    AtEnd,
}

impl ReminderTrigger {
    pub fn to_icalendar(&self) -> String {
        match self {
            ReminderTrigger::BeforeStart(duration) => {
                format!("-PT{}M", duration.num_minutes())
            }
            ReminderTrigger::BeforeEnd(duration) => {
                format!("-PT{}M", duration.num_minutes())
            }
            ReminderTrigger::AtStart => "PT0M".to_string(),
            ReminderTrigger::AtEnd => "PT0M".to_string(),
        }
    }
}

/// Reminder action type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReminderAction {
    Display,
    Email,
    Audio,
}

impl ReminderAction {
    pub fn to_icalendar(&self) -> &str {
        match self {
            ReminderAction::Display => "DISPLAY",
            ReminderAction::Email => "EMAIL",
            ReminderAction::Audio => "AUDIO",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    
    #[test]
    fn test_event_creation() {
        let start = Utc.with_ymd_and_hms(2025, 1, 28, 10, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2025, 1, 28, 11, 0, 0).unwrap();
        
        let event = Event::new(
            "calendar1".to_string(),
            "Test Meeting".to_string(),
            start,
            end,
        );
        
        assert_eq!(event.title, "Test Meeting");
        assert_eq!(event.calendar_id, "calendar1");
        assert!(!event.all_day);
        assert_eq!(event.duration(), Duration::hours(1));
    }
    
    #[test]
    fn test_all_day_event() {
        let date = Utc.with_ymd_and_hms(2025, 1, 28, 0, 0, 0).unwrap();
        
        let event = Event::new_all_day(
            "calendar1".to_string(),
            "All Day Event".to_string(),
            date,
        );
        
        assert!(event.all_day);
        assert_eq!(event.duration(), Duration::days(1));
    }
    
    #[test]
    fn test_event_timing() {
        let now = Utc::now();
        let past_event = Event::new(
            "cal1".to_string(),
            "Past".to_string(),
            now - Duration::hours(2),
            now - Duration::hours(1),
        );
        
        let current_event = Event::new(
            "cal1".to_string(),
            "Current".to_string(),
            now - Duration::minutes(30),
            now + Duration::minutes(30),
        );
        
        let future_event = Event::new(
            "cal1".to_string(),
            "Future".to_string(),
            now + Duration::hours(1),
            now + Duration::hours(2),
        );
        
        assert!(past_event.is_past());
        assert!(current_event.is_current());
        assert!(future_event.is_upcoming());
    }
    
    #[test]
    fn test_attendee_management() {
        let start = Utc::now() + Duration::hours(1);
        let end = start + Duration::hours(1);
        
        let mut event = Event::new("cal1".to_string(), "Meeting".to_string(), start, end);
        
        let attendee = EventAttendee::new(
            "test@example.com".to_string(),
            Some("Test User".to_string()),
        );
        
        event.add_attendee(attendee);
        assert_eq!(event.attendees.len(), 1);
        
        let updated = event.update_attendee_status("test@example.com", AttendeeStatus::Accepted);
        assert!(updated);
        assert_eq!(event.attendees[0].status, AttendeeStatus::Accepted);
        
        let removed = event.remove_attendee("test@example.com");
        assert!(removed);
        assert_eq!(event.attendees.len(), 0);
    }
    
    #[test]
    fn test_icalendar_generation() {
        let start = Utc.with_ymd_and_hms(2025, 1, 28, 10, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2025, 1, 28, 11, 0, 0).unwrap();
        
        let event = Event::new(
            "cal1".to_string(),
            "Test Event".to_string(),
            start,
            end,
        );
        
        let ical = event.to_icalendar();
        
        assert!(ical.contains("BEGIN:VCALENDAR"));
        assert!(ical.contains("BEGIN:VEVENT"));
        assert!(ical.contains("SUMMARY:Test Event"));
        assert!(ical.contains("DTSTART:20250128T100000Z"));
        assert!(ical.contains("DTEND:20250128T110000Z"));
        assert!(ical.contains("END:VEVENT"));
        assert!(ical.contains("END:VCALENDAR"));
    }
    
    #[test]
    fn test_recurrence() {
        let daily = EventRecurrence::daily(1);
        assert_eq!(daily.frequency, RecurrenceFrequency::Daily);
        assert_eq!(daily.interval, 1);
        
        let weekly = EventRecurrence::weekly(2, vec![RecurrenceDay::Monday, RecurrenceDay::Friday]);
        assert_eq!(weekly.frequency, RecurrenceFrequency::Weekly);
        assert_eq!(weekly.interval, 2);
        assert_eq!(weekly.by_day.len(), 2);
        
        let rrule = weekly.to_icalendar();
        assert!(rrule.contains("FREQ=WEEKLY"));
        assert!(rrule.contains("INTERVAL=2"));
        assert!(rrule.contains("BYDAY=MO,FR"));
    }
}