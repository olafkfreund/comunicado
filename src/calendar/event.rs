use chrono::{DateTime, Duration, Utc};
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
    pub sequence: u32,        // iCalendar SEQUENCE for updates
    pub etag: Option<String>, // CalDAV ETag for sync
}

impl Event {
    /// Create a new event
    pub fn new(
        calendar_id: String,
        title: String,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Self {
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

    /// Check if this event has recurrence rules
    pub fn is_recurring(&self) -> bool {
        self.recurrence.is_some()
    }

    /// Get the recurrence pattern as an RRULE string
    pub fn get_rrule(&self) -> Option<String> {
        self.recurrence.as_ref().map(|r| r.to_icalendar())
    }

    /// Set recurrence from an RRULE string
    pub fn set_rrule(&mut self, rrule: &str) -> Result<(), String> {
        self.recurrence = Some(EventRecurrence::from_icalendar(rrule)?);
        self.updated_at = Utc::now();
        self.sequence += 1;
        Ok(())
    }

    /// Clear recurrence rules
    pub fn clear_recurrence(&mut self) {
        self.recurrence = None;
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
            ical.push_str(&format!(
                "DTSTART;VALUE=DATE:{}\r\n",
                self.start_time.format("%Y%m%d")
            ));
            ical.push_str(&format!(
                "DTEND;VALUE=DATE:{}\r\n",
                self.end_time.format("%Y%m%d")
            ));
        } else {
            ical.push_str(&format!(
                "DTSTART:{}\r\n",
                self.start_time.format("%Y%m%dT%H%M%SZ")
            ));
            ical.push_str(&format!(
                "DTEND:{}\r\n",
                self.end_time.format("%Y%m%dT%H%M%SZ")
            ));
        }

        ical.push_str(&format!(
            "CREATED:{}\r\n",
            self.created_at.format("%Y%m%dT%H%M%SZ")
        ));
        ical.push_str(&format!(
            "LAST-MODIFIED:{}\r\n",
            self.updated_at.format("%Y%m%dT%H%M%SZ")
        ));
        ical.push_str(&format!("SEQUENCE:{}\r\n", self.sequence));
        ical.push_str(&format!("STATUS:{}\r\n", self.status.to_icalendar()));

        if let Some(ref organizer) = self.organizer {
            ical.push_str(&format!(
                "ORGANIZER;CN={}:mailto:{}\r\n",
                organizer.name.as_deref().unwrap_or(&organizer.email),
                organizer.email
            ));
        }

        for attendee in &self.attendees {
            ical.push_str(&format!(
                "ATTENDEE;CN={};PARTSTAT={}:mailto:{}\r\n",
                attendee.name.as_deref().unwrap_or(&attendee.email),
                attendee.status.to_icalendar(),
                attendee.email
            ));
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
            let days: Vec<String> = self
                .by_day
                .iter()
                .map(|d| d.to_icalendar().to_string())
                .collect();
            rrule.push_str(&format!(";BYDAY={}", days.join(",")));
        }

        if !self.by_month_day.is_empty() {
            let days: Vec<String> = self.by_month_day.iter().map(|d| d.to_string()).collect();
            rrule.push_str(&format!(";BYMONTHDAY={}", days.join(",")));
        }

        rrule
    }

    /// Parse an RRULE string into EventRecurrence
    pub fn from_icalendar(rrule: &str) -> Result<Self, String> {
        // Remove "RRULE:" prefix if present
        let rrule = rrule.strip_prefix("RRULE:").unwrap_or(rrule);

        let mut recurrence = EventRecurrence {
            frequency: RecurrenceFrequency::Daily, // Default, will be overridden
            interval: 1,
            count: None,
            until: None,
            by_day: Vec::new(),
            by_month_day: Vec::new(),
            by_month: Vec::new(),
            by_week_no: Vec::new(),
            by_year_day: Vec::new(),
            week_start: RecurrenceDay::Monday,
        };

        // Parse key-value pairs separated by semicolons
        for part in rrule.split(';') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }

            if let Some((key, value)) = part.split_once('=') {
                match key.to_uppercase().as_str() {
                    "FREQ" => {
                        recurrence.frequency = RecurrenceFrequency::from_icalendar(value)?;
                    }
                    "INTERVAL" => {
                        recurrence.interval = value
                            .parse()
                            .map_err(|_| format!("Invalid INTERVAL value: {}", value))?;
                    }
                    "COUNT" => {
                        recurrence.count = Some(
                            value
                                .parse()
                                .map_err(|_| format!("Invalid COUNT value: {}", value))?,
                        );
                    }
                    "UNTIL" => {
                        recurrence.until = Some(Self::parse_icalendar_datetime(value)?);
                    }
                    "BYDAY" => {
                        recurrence.by_day = Self::parse_by_day(value)?;
                    }
                    "BYMONTHDAY" => {
                        recurrence.by_month_day = Self::parse_by_month_day(value)?;
                    }
                    "BYMONTH" => {
                        recurrence.by_month = Self::parse_by_month(value)?;
                    }
                    "BYWEEKNO" => {
                        recurrence.by_week_no = Self::parse_by_week_no(value)?;
                    }
                    "BYYEARDAY" => {
                        recurrence.by_year_day = Self::parse_by_year_day(value)?;
                    }
                    "WKST" => {
                        recurrence.week_start = RecurrenceDay::from_icalendar(value)?;
                    }
                    _ => {
                        // Ignore unknown properties for forward compatibility
                        tracing::debug!("Ignoring unknown RRULE property: {}", key);
                    }
                }
            } else {
                return Err(format!("Invalid RRULE format: {}", part));
            }
        }

        Ok(recurrence)
    }

    /// Parse iCalendar datetime format (YYYYMMDDTHHMMSSZ or YYYYMMDD)
    fn parse_icalendar_datetime(value: &str) -> Result<DateTime<Utc>, String> {
        use chrono::{NaiveDate, NaiveDateTime};

        if value.ends_with('Z') {
            // Full datetime format: YYYYMMDDTHHMMSSZ
            let datetime_str = &value[..value.len() - 1]; // Remove 'Z'
            NaiveDateTime::parse_from_str(datetime_str, "%Y%m%dT%H%M%S")
                .map(|dt| dt.and_utc())
                .map_err(|_| format!("Invalid datetime format: {}", value))
        } else if value.len() == 8 {
            // Date-only format: YYYYMMDD
            NaiveDate::parse_from_str(value, "%Y%m%d")
                .map(|date| date.and_hms_opt(0, 0, 0).unwrap().and_utc())
                .map_err(|_| format!("Invalid date format: {}", value))
        } else {
            Err(format!("Unsupported datetime format: {}", value))
        }
    }

    /// Parse BYDAY values (e.g., "MO,WE,FR" or "1MO,2TU")
    fn parse_by_day(value: &str) -> Result<Vec<RecurrenceDay>, String> {
        let mut days = Vec::new();

        for day_spec in value.split(',') {
            let day_spec = day_spec.trim();

            // Handle prefixed numbers (e.g., "1MO", "-1FR")
            let day_code = if day_spec.len() > 2 {
                // Extract the last 2 characters as the day code
                &day_spec[day_spec.len() - 2..]
            } else {
                day_spec
            };

            days.push(RecurrenceDay::from_icalendar(day_code)?);
        }

        Ok(days)
    }

    /// Parse BYMONTHDAY values (e.g., "1,15,-1")
    fn parse_by_month_day(value: &str) -> Result<Vec<i8>, String> {
        value
            .split(',')
            .map(|s| {
                s.trim()
                    .parse::<i8>()
                    .map_err(|_| format!("Invalid BYMONTHDAY value: {}", s))
            })
            .collect()
    }

    /// Parse BYMONTH values (e.g., "1,3,5")
    fn parse_by_month(value: &str) -> Result<Vec<u8>, String> {
        value
            .split(',')
            .map(|s| {
                let month = s
                    .trim()
                    .parse::<u8>()
                    .map_err(|_| format!("Invalid BYMONTH value: {}", s))?;
                if month >= 1 && month <= 12 {
                    Ok(month)
                } else {
                    Err(format!("BYMONTH value out of range (1-12): {}", month))
                }
            })
            .collect()
    }

    /// Parse BYWEEKNO values (e.g., "1,10,-1")
    fn parse_by_week_no(value: &str) -> Result<Vec<i8>, String> {
        value
            .split(',')
            .map(|s| {
                s.trim()
                    .parse::<i8>()
                    .map_err(|_| format!("Invalid BYWEEKNO value: {}", s))
            })
            .collect()
    }

    /// Parse BYYEARDAY values (e.g., "1,100,-1")
    fn parse_by_year_day(value: &str) -> Result<Vec<i16>, String> {
        value
            .split(',')
            .map(|s| {
                s.trim()
                    .parse::<i16>()
                    .map_err(|_| format!("Invalid BYYEARDAY value: {}", s))
            })
            .collect()
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

    pub fn from_icalendar(value: &str) -> Result<Self, String> {
        match value.to_uppercase().as_str() {
            "SECONDLY" => Ok(RecurrenceFrequency::Secondly),
            "MINUTELY" => Ok(RecurrenceFrequency::Minutely),
            "HOURLY" => Ok(RecurrenceFrequency::Hourly),
            "DAILY" => Ok(RecurrenceFrequency::Daily),
            "WEEKLY" => Ok(RecurrenceFrequency::Weekly),
            "MONTHLY" => Ok(RecurrenceFrequency::Monthly),
            "YEARLY" => Ok(RecurrenceFrequency::Yearly),
            _ => Err(format!("Invalid frequency: {}", value)),
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

    pub fn from_icalendar(value: &str) -> Result<Self, String> {
        match value.to_uppercase().as_str() {
            "SU" => Ok(RecurrenceDay::Sunday),
            "MO" => Ok(RecurrenceDay::Monday),
            "TU" => Ok(RecurrenceDay::Tuesday),
            "WE" => Ok(RecurrenceDay::Wednesday),
            "TH" => Ok(RecurrenceDay::Thursday),
            "FR" => Ok(RecurrenceDay::Friday),
            "SA" => Ok(RecurrenceDay::Saturday),
            _ => Err(format!("Invalid day: {}", value)),
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
    use chrono::{Datelike, TimeZone, Timelike};

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

        let event = Event::new_all_day("calendar1".to_string(), "All Day Event".to_string(), date);

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

        let event = Event::new("cal1".to_string(), "Test Event".to_string(), start, end);

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

    #[test]
    fn test_rrule_parsing_basic() {
        // Test basic daily recurrence
        let rrule = "FREQ=DAILY;INTERVAL=2";
        let recurrence = EventRecurrence::from_icalendar(rrule).unwrap();

        assert_eq!(recurrence.frequency, RecurrenceFrequency::Daily);
        assert_eq!(recurrence.interval, 2);
        assert!(recurrence.count.is_none());
        assert!(recurrence.until.is_none());

        // Test roundtrip conversion
        let regenerated = recurrence.to_icalendar();
        assert!(regenerated.contains("FREQ=DAILY"));
        assert!(regenerated.contains("INTERVAL=2"));
    }

    #[test]
    fn test_rrule_parsing_with_count() {
        let rrule = "FREQ=WEEKLY;INTERVAL=1;COUNT=10;BYDAY=MO,WE,FR";
        let recurrence = EventRecurrence::from_icalendar(rrule).unwrap();

        assert_eq!(recurrence.frequency, RecurrenceFrequency::Weekly);
        assert_eq!(recurrence.interval, 1);
        assert_eq!(recurrence.count, Some(10));
        assert_eq!(recurrence.by_day.len(), 3);
        assert!(recurrence.by_day.contains(&RecurrenceDay::Monday));
        assert!(recurrence.by_day.contains(&RecurrenceDay::Wednesday));
        assert!(recurrence.by_day.contains(&RecurrenceDay::Friday));
    }

    #[test]
    fn test_rrule_parsing_with_until() {
        let rrule = "FREQ=MONTHLY;UNTIL=20251231T235959Z";
        let recurrence = EventRecurrence::from_icalendar(rrule).unwrap();

        assert_eq!(recurrence.frequency, RecurrenceFrequency::Monthly);
        assert!(recurrence.until.is_some());

        let until = recurrence.until.unwrap();
        assert_eq!(until.year(), 2025);
        assert_eq!(until.month(), 12);
        assert_eq!(until.day(), 31);
        assert_eq!(until.hour(), 23);
        assert_eq!(until.minute(), 59);
        assert_eq!(until.second(), 59);
    }

    #[test]
    fn test_rrule_parsing_with_bymonthday() {
        let rrule = "FREQ=MONTHLY;BYMONTHDAY=1,15,-1";
        let recurrence = EventRecurrence::from_icalendar(rrule).unwrap();

        assert_eq!(recurrence.frequency, RecurrenceFrequency::Monthly);
        assert_eq!(recurrence.by_month_day, vec![1, 15, -1]);
    }

    #[test]
    fn test_rrule_parsing_with_bymonth() {
        let rrule = "FREQ=YEARLY;BYMONTH=1,6,12";
        let recurrence = EventRecurrence::from_icalendar(rrule).unwrap();

        assert_eq!(recurrence.frequency, RecurrenceFrequency::Yearly);
        assert_eq!(recurrence.by_month, vec![1, 6, 12]);
    }

    #[test]
    fn test_rrule_parsing_complex() {
        let rrule = "FREQ=MONTHLY;INTERVAL=2;BYDAY=1MO,3WE;BYMONTH=1,3,5,7,9,11;WKST=SU";
        let recurrence = EventRecurrence::from_icalendar(rrule).unwrap();

        assert_eq!(recurrence.frequency, RecurrenceFrequency::Monthly);
        assert_eq!(recurrence.interval, 2);
        assert_eq!(recurrence.by_day.len(), 2);
        assert!(recurrence.by_day.contains(&RecurrenceDay::Monday));
        assert!(recurrence.by_day.contains(&RecurrenceDay::Wednesday));
        assert_eq!(recurrence.by_month, vec![1, 3, 5, 7, 9, 11]);
        assert_eq!(recurrence.week_start, RecurrenceDay::Sunday);
    }

    #[test]
    fn test_rrule_parsing_with_prefix() {
        let rrule = "RRULE:FREQ=DAILY;INTERVAL=3";
        let recurrence = EventRecurrence::from_icalendar(rrule).unwrap();

        assert_eq!(recurrence.frequency, RecurrenceFrequency::Daily);
        assert_eq!(recurrence.interval, 3);
    }

    #[test]
    fn test_rrule_parsing_errors() {
        // Invalid frequency
        assert!(EventRecurrence::from_icalendar("FREQ=INVALID").is_err());

        // Invalid interval
        assert!(EventRecurrence::from_icalendar("FREQ=DAILY;INTERVAL=abc").is_err());

        // Invalid count
        assert!(EventRecurrence::from_icalendar("FREQ=DAILY;COUNT=xyz").is_err());

        // Invalid day
        assert!(EventRecurrence::from_icalendar("FREQ=WEEKLY;BYDAY=XX").is_err());

        // Invalid month
        assert!(EventRecurrence::from_icalendar("FREQ=YEARLY;BYMONTH=13").is_err());

        // Invalid format
        assert!(EventRecurrence::from_icalendar("FREQ_DAILY").is_err());
    }

    #[test]
    fn test_frequency_conversion() {
        assert_eq!(
            RecurrenceFrequency::from_icalendar("DAILY").unwrap(),
            RecurrenceFrequency::Daily
        );
        assert_eq!(
            RecurrenceFrequency::from_icalendar("weekly").unwrap(),
            RecurrenceFrequency::Weekly
        );
        assert_eq!(
            RecurrenceFrequency::from_icalendar("MONTHLY").unwrap(),
            RecurrenceFrequency::Monthly
        );
        assert_eq!(
            RecurrenceFrequency::from_icalendar("YEARLY").unwrap(),
            RecurrenceFrequency::Yearly
        );

        assert!(RecurrenceFrequency::from_icalendar("INVALID").is_err());
    }

    #[test]
    fn test_day_conversion() {
        assert_eq!(
            RecurrenceDay::from_icalendar("MO").unwrap(),
            RecurrenceDay::Monday
        );
        assert_eq!(
            RecurrenceDay::from_icalendar("tu").unwrap(),
            RecurrenceDay::Tuesday
        );
        assert_eq!(
            RecurrenceDay::from_icalendar("WE").unwrap(),
            RecurrenceDay::Wednesday
        );
        assert_eq!(
            RecurrenceDay::from_icalendar("SA").unwrap(),
            RecurrenceDay::Saturday
        );

        assert!(RecurrenceDay::from_icalendar("XX").is_err());
    }

    #[test]
    fn test_datetime_parsing() {
        // Full datetime with Z suffix
        let dt = EventRecurrence::parse_icalendar_datetime("20250128T143000Z").unwrap();
        assert_eq!(dt.year(), 2025);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 28);
        assert_eq!(dt.hour(), 14);
        assert_eq!(dt.minute(), 30);
        assert_eq!(dt.second(), 0);

        // Date only format
        let dt = EventRecurrence::parse_icalendar_datetime("20250315").unwrap();
        assert_eq!(dt.year(), 2025);
        assert_eq!(dt.month(), 3);
        assert_eq!(dt.day(), 15);
        assert_eq!(dt.hour(), 0);
        assert_eq!(dt.minute(), 0);
        assert_eq!(dt.second(), 0);

        // Invalid formats
        assert!(EventRecurrence::parse_icalendar_datetime("invalid").is_err());
        assert!(EventRecurrence::parse_icalendar_datetime("2025-01-28").is_err());
    }
}
