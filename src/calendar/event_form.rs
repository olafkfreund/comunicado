use chrono::{DateTime, Duration, Local, NaiveDate, NaiveTime, TimeZone, Timelike, Utc};
use ratatui::{layout::Rect, widgets::ListState, Frame};

use crate::calendar::database::CalendarDatabase;
use crate::calendar::event::{AttendeeStatus, Event, EventAttendee, EventRecurrence, EventStatus};
use crate::calendar::{Calendar, CalendarError, CalendarResult};
use crate::ui::date_picker::DatePicker;
use crate::ui::time_picker::TimePicker;

/// Event conflict types
#[derive(Debug, Clone, PartialEq)]
pub enum EventConflict {
    TimeOverlap {
        conflicting_event_id: String,
        conflicting_event_title: String,
        overlap_start: DateTime<Utc>,
        overlap_end: DateTime<Utc>,
    },
    ResourceBooking {
        resource_name: String,
        conflicting_event_id: String,
        conflicting_event_title: String,
    },
    AttendeeConflict {
        attendee_email: String,
        conflicting_event_id: String,
        conflicting_event_title: String,
    },
}

/// Validation severity levels
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationSeverity {
    Error,   // Prevents saving
    Warning, // Allows saving with confirmation
    Info,    // Just informational
}

/// Comprehensive validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<(ValidationSeverity, String)>,
    pub conflicts: Vec<EventConflict>,
    pub warnings: Vec<String>,
    pub suggestions: Vec<String>,
}

/// Event form actions
#[derive(Debug, Clone, PartialEq)]
pub enum EventFormAction {
    Save,
    Cancel,
    Delete,
    AddAttendee,
    RemoveAttendee(usize),
    ToggleRecurrence,
    UpdateRecurrence(EventRecurrence),
}

/// Event form field types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventFormField {
    Title,
    Description,
    Location,
    StartDate,
    StartTime,
    EndDate,
    EndTime,
    Calendar,
    Status,
    AllDay,
    Recurrence,
    Attendees,
    Notes,
}

/// Event form mode
#[derive(Debug, Clone, PartialEq)]
pub enum EventFormMode {
    Create,
    Edit(String), // Event ID
    View(String), // Event ID
}

/// Event form validation errors
#[derive(Debug, Clone)]
pub struct EventFormValidation {
    pub title_error: Option<String>,
    pub date_error: Option<String>,
    pub time_error: Option<String>,
    pub calendar_error: Option<String>,
    pub attendee_error: Option<String>,
    pub conflict_error: Option<String>,
    pub duration_error: Option<String>,
    pub recurrence_error: Option<String>,
    pub location_error: Option<String>,
}

impl Default for EventFormValidation {
    fn default() -> Self {
        Self {
            title_error: None,
            date_error: None,
            time_error: None,
            calendar_error: None,
            attendee_error: None,
            conflict_error: None,
            duration_error: None,
            recurrence_error: None,
            location_error: None,
        }
    }
}

/// Event form UI component
pub struct EventFormUI {
    pub mode: EventFormMode,
    pub event: Event,
    pub calendars: Vec<Calendar>,
    pub current_field: EventFormField,
    pub validation: EventFormValidation,

    // Form state
    pub title_input: String,
    pub description_input: String,
    pub location_input: String,
    pub start_date: NaiveDate,
    pub start_time: NaiveTime,
    pub end_date: NaiveDate,
    pub end_time: NaiveTime,
    pub selected_calendar_id: String,
    pub event_status: EventStatus,
    pub is_all_day: bool,
    pub recurrence_rule: Option<EventRecurrence>,
    pub attendees: Vec<EventAttendee>,
    pub notes_input: String,

    // UI state
    pub calendar_list_state: ListState,
    pub status_list_state: ListState,
    pub attendee_list_state: ListState,
    pub date_picker: DatePicker,
    pub time_picker: TimePicker,
    pub show_date_picker: bool,
    pub show_time_picker: bool,
    pub show_calendar_selector: bool,
    pub show_status_selector: bool,
    pub show_recurrence_editor: bool,
    pub attendee_input: String,
    pub is_modified: bool,
}

impl EventFormUI {
    /// Create a new event form for creating an event
    pub fn new_create(calendars: Vec<Calendar>, default_calendar_id: Option<String>) -> Self {
        let now = Local::now();
        let start_time = now.time();
        let end_time = (now + Duration::hours(1)).time();

        let default_calendar = default_calendar_id
            .or_else(|| calendars.first().map(|c| c.id.clone()))
            .unwrap_or_default();

        let event = Event::new(
            default_calendar.clone(),
            String::new(),
            now.with_timezone(&Utc),
            (now + Duration::hours(1)).with_timezone(&Utc),
        );

        Self {
            mode: EventFormMode::Create,
            event,
            calendars,
            current_field: EventFormField::Title,
            validation: EventFormValidation::default(),

            title_input: String::new(),
            description_input: String::new(),
            location_input: String::new(),
            start_date: now.date_naive(),
            start_time,
            end_date: now.date_naive(),
            end_time,
            selected_calendar_id: default_calendar,
            event_status: EventStatus::Confirmed,
            is_all_day: false,
            recurrence_rule: None,
            attendees: Vec::new(),
            notes_input: String::new(),

            calendar_list_state: ListState::default(),
            status_list_state: ListState::default(),
            attendee_list_state: ListState::default(),
            date_picker: DatePicker::new(now.date_naive()),
            time_picker: TimePicker::new(start_time),
            show_date_picker: false,
            show_time_picker: false,
            show_calendar_selector: false,
            show_status_selector: false,
            show_recurrence_editor: false,
            attendee_input: String::new(),
            is_modified: false,
        }
    }

    /// Create a new event form for editing an existing event
    pub fn new_edit(event: Event, calendars: Vec<Calendar>) -> Self {
        let start_local = event.start_time.with_timezone(&Local);
        let end_local = event.end_time.with_timezone(&Local);

        let mut form = Self {
            mode: EventFormMode::Edit(event.id.clone()),
            calendars,
            current_field: EventFormField::Title,
            validation: EventFormValidation::default(),

            title_input: event.title.clone(),
            description_input: event.description.clone().unwrap_or_default(),
            location_input: event.location.clone().unwrap_or_default(),
            start_date: start_local.date_naive(),
            start_time: start_local.time(),
            end_date: end_local.date_naive(),
            end_time: end_local.time(),
            selected_calendar_id: event.calendar_id.clone(),
            event_status: event.status,
            is_all_day: event.all_day,
            recurrence_rule: event.recurrence.clone(),
            attendees: event.attendees.clone(),
            notes_input: String::new(), // Events don't have separate notes field
            event: event.clone(),

            calendar_list_state: ListState::default(),
            status_list_state: ListState::default(),
            attendee_list_state: ListState::default(),
            date_picker: DatePicker::new(start_local.date_naive()),
            time_picker: TimePicker::new(start_local.time()),
            show_date_picker: false,
            show_time_picker: false,
            show_calendar_selector: false,
            show_status_selector: false,
            show_recurrence_editor: false,
            attendee_input: String::new(),
            is_modified: false,
        };

        // Set initial calendar selection
        if let Some(index) = form
            .calendars
            .iter()
            .position(|c| c.id == event.calendar_id)
        {
            form.calendar_list_state.select(Some(index));
        }

        // Set initial status selection
        form.status_list_state
            .select(Some(form.event_status as usize));

        form
    }

    /// Create a new event form for viewing an existing event (read-only)
    pub fn new_view(event: Event, calendars: Vec<Calendar>) -> Self {
        let mut form = Self::new_edit(event, calendars);
        form.mode = EventFormMode::View(form.event.id.clone());
        form
    }

    /// Handle character input for the current field
    pub fn handle_char_input(&mut self, c: char) {
        if matches!(self.mode, EventFormMode::View(_)) {
            return;
        }

        self.is_modified = true;

        match self.current_field {
            EventFormField::Title => {
                self.title_input.push(c);
                self.validation.title_error = None;
            }
            EventFormField::Description => {
                self.description_input.push(c);
            }
            EventFormField::Location => {
                self.location_input.push(c);
            }
            EventFormField::Attendees => {
                self.attendee_input.push(c);
                self.validation.attendee_error = None;
            }
            EventFormField::Notes => {
                self.notes_input.push(c);
            }
            _ => {}
        }
    }

    /// Handle backspace for the current field
    pub fn handle_backspace(&mut self) {
        if matches!(self.mode, EventFormMode::View(_)) {
            return;
        }

        self.is_modified = true;

        match self.current_field {
            EventFormField::Title => {
                self.title_input.pop();
            }
            EventFormField::Description => {
                self.description_input.pop();
            }
            EventFormField::Location => {
                self.location_input.pop();
            }
            EventFormField::Attendees => {
                self.attendee_input.pop();
            }
            EventFormField::Notes => {
                self.notes_input.pop();
            }
            _ => {}
        }
    }

    /// Move to the next field
    pub fn next_field(&mut self) {
        self.current_field = match self.current_field {
            EventFormField::Title => EventFormField::Description,
            EventFormField::Description => EventFormField::Location,
            EventFormField::Location => EventFormField::StartDate,
            EventFormField::StartDate => EventFormField::StartTime,
            EventFormField::StartTime => EventFormField::EndDate,
            EventFormField::EndDate => EventFormField::EndTime,
            EventFormField::EndTime => EventFormField::Calendar,
            EventFormField::Calendar => EventFormField::Status,
            EventFormField::Status => EventFormField::AllDay,
            EventFormField::AllDay => EventFormField::Recurrence,
            EventFormField::Recurrence => EventFormField::Attendees,
            EventFormField::Attendees => EventFormField::Notes,
            EventFormField::Notes => EventFormField::Title,
        };
    }

    /// Move to the previous field
    pub fn previous_field(&mut self) {
        self.current_field = match self.current_field {
            EventFormField::Title => EventFormField::Notes,
            EventFormField::Description => EventFormField::Title,
            EventFormField::Location => EventFormField::Description,
            EventFormField::StartDate => EventFormField::Location,
            EventFormField::StartTime => EventFormField::StartDate,
            EventFormField::EndDate => EventFormField::StartTime,
            EventFormField::EndTime => EventFormField::EndDate,
            EventFormField::Calendar => EventFormField::EndTime,
            EventFormField::Status => EventFormField::Calendar,
            EventFormField::AllDay => EventFormField::Status,
            EventFormField::Recurrence => EventFormField::AllDay,
            EventFormField::Attendees => EventFormField::Recurrence,
            EventFormField::Notes => EventFormField::Attendees,
        };
    }

    /// Handle Enter key press
    pub fn handle_enter(&mut self) -> Option<EventFormAction> {
        match self.current_field {
            EventFormField::StartDate | EventFormField::EndDate => {
                self.show_date_picker = true;
                None
            }
            EventFormField::StartTime | EventFormField::EndTime => {
                self.show_time_picker = true;
                None
            }
            EventFormField::Calendar => {
                self.show_calendar_selector = true;
                None
            }
            EventFormField::Status => {
                self.show_status_selector = true;
                None
            }
            EventFormField::AllDay => {
                if !matches!(self.mode, EventFormMode::View(_)) {
                    self.is_all_day = !self.is_all_day;
                    self.is_modified = true;
                }
                None
            }
            EventFormField::Recurrence => {
                self.show_recurrence_editor = true;
                None
            }
            EventFormField::Attendees => {
                if !self.attendee_input.trim().is_empty() {
                    return Some(EventFormAction::AddAttendee);
                }
                None
            }
            _ => None,
        }
    }

    /// Add an attendee from the current input
    pub fn add_attendee(&mut self) -> Result<(), String> {
        let email = self.attendee_input.trim();
        if email.is_empty() {
            return Err("Email address cannot be empty".to_string());
        }

        // Basic email validation
        if !email.contains('@') || !email.contains('.') {
            return Err("Invalid email address format".to_string());
        }

        // Check for duplicates
        if self.attendees.iter().any(|a| a.email == email) {
            return Err("Attendee already added".to_string());
        }

        let mut attendee = EventAttendee::new(email.to_string(), None);
        attendee.status = AttendeeStatus::NeedsAction;

        self.attendees.push(attendee);
        self.attendee_input.clear();
        self.is_modified = true;
        self.validation.attendee_error = None;

        Ok(())
    }

    /// Remove an attendee at the given index
    pub fn remove_attendee(&mut self, index: usize) {
        if index < self.attendees.len() {
            self.attendees.remove(index);
            self.is_modified = true;
        }
    }

    /// Validate the form data
    pub fn validate(&mut self) -> bool {
        self.validation = EventFormValidation::default();
        let mut is_valid = true;

        // Title validation
        if self.title_input.trim().is_empty() {
            self.validation.title_error = Some("Title is required".to_string());
            is_valid = false;
        } else if self.title_input.len() > 255 {
            self.validation.title_error =
                Some("Title is too long (max 255 characters)".to_string());
            is_valid = false;
        }

        // Date validation
        if self.end_date < self.start_date {
            self.validation.date_error = Some("End date cannot be before start date".to_string());
            is_valid = false;
        } else if self.end_date == self.start_date && self.end_time < self.start_time {
            self.validation.time_error = Some("End time cannot be before start time".to_string());
            is_valid = false;
        }

        // Duration validation
        let start_datetime = self.start_date.and_time(if self.is_all_day {
            NaiveTime::from_hms_opt(0, 0, 0).unwrap()
        } else {
            self.start_time
        });

        let end_datetime = self.end_date.and_time(if self.is_all_day {
            NaiveTime::from_hms_opt(23, 59, 59).unwrap()
        } else {
            self.end_time
        });

        let duration = end_datetime - start_datetime;
        if duration > Duration::days(7) {
            self.validation.duration_error =
                Some("Event duration cannot exceed 7 days".to_string());
            is_valid = false;
        } else if !self.is_all_day && duration < Duration::minutes(1) {
            self.validation.duration_error =
                Some("Event must be at least 1 minute long".to_string());
            is_valid = false;
        }

        // Calendar validation
        if !self
            .calendars
            .iter()
            .any(|c| c.id == self.selected_calendar_id)
        {
            self.validation.calendar_error = Some("Please select a valid calendar".to_string());
            is_valid = false;
        }

        // Location validation
        if !self.location_input.trim().is_empty() && self.location_input.len() > 500 {
            self.validation.location_error =
                Some("Location is too long (max 500 characters)".to_string());
            is_valid = false;
        }

        // Attendee email validation
        for attendee in &self.attendees {
            if !self.is_valid_email(&attendee.email) {
                self.validation.attendee_error =
                    Some(format!("Invalid email address: {}", attendee.email));
                is_valid = false;
                break;
            }
        }

        is_valid
    }

    /// Perform comprehensive validation including conflict detection
    pub async fn validate_comprehensive(
        &mut self,
        database: &CalendarDatabase,
    ) -> ValidationResult {
        let mut result = ValidationResult {
            is_valid: true,
            errors: Vec::new(),
            conflicts: Vec::new(),
            warnings: Vec::new(),
            suggestions: Vec::new(),
        };

        // Basic validation first
        if !self.validate() {
            result.is_valid = false;

            if let Some(ref error) = self.validation.title_error {
                result
                    .errors
                    .push((ValidationSeverity::Error, error.clone()));
            }
            if let Some(ref error) = self.validation.date_error {
                result
                    .errors
                    .push((ValidationSeverity::Error, error.clone()));
            }
            if let Some(ref error) = self.validation.time_error {
                result
                    .errors
                    .push((ValidationSeverity::Error, error.clone()));
            }
            if let Some(ref error) = self.validation.calendar_error {
                result
                    .errors
                    .push((ValidationSeverity::Error, error.clone()));
            }
            if let Some(ref error) = self.validation.attendee_error {
                result
                    .errors
                    .push((ValidationSeverity::Error, error.clone()));
            }
            if let Some(ref error) = self.validation.duration_error {
                result
                    .errors
                    .push((ValidationSeverity::Error, error.clone()));
            }
            if let Some(ref error) = self.validation.location_error {
                result
                    .errors
                    .push((ValidationSeverity::Error, error.clone()));
            }
        }

        // Build temporary event for conflict checking
        let temp_event = self.build_event();

        // Check for time conflicts with existing events
        if let Ok(conflicts) = self.check_time_conflicts(database, &temp_event).await {
            if !conflicts.is_empty() {
                result.conflicts.extend(conflicts);
                result
                    .warnings
                    .push("This event conflicts with existing events".to_string());
            }
        }

        // Check for attendee conflicts
        if let Ok(attendee_conflicts) = self.check_attendee_conflicts(database, &temp_event).await {
            if !attendee_conflicts.is_empty() {
                result.conflicts.extend(attendee_conflicts);
                result
                    .warnings
                    .push("Some attendees have scheduling conflicts".to_string());
            }
        }

        // Add suggestions for optimization
        self.add_suggestions(&mut result, &temp_event);

        result
    }

    /// Check for time conflicts with existing events
    async fn check_time_conflicts(
        &self,
        database: &CalendarDatabase,
        event: &Event,
    ) -> CalendarResult<Vec<EventConflict>> {
        let mut conflicts = Vec::new();

        // Get all events in the same calendar within the time range
        let start_range = event.start_time - Duration::hours(24); // Look 24 hours before
        let end_range = event.end_time + Duration::hours(24); // Look 24 hours after

        let existing_events = database
            .get_events(&event.calendar_id, Some(start_range), Some(end_range))
            .await
            .map_err(|e| {
                CalendarError::DatabaseError(format!(
                    "Failed to fetch events for conflict check: {}",
                    e
                ))
            })?;

        for existing_event in existing_events {
            // Skip the current event if we're editing
            if existing_event.id == event.id {
                continue;
            }

            // Check for time overlap
            if event.overlaps_with(&existing_event) {
                let overlap_start = event.start_time.max(existing_event.start_time);
                let overlap_end = event.end_time.min(existing_event.end_time);

                conflicts.push(EventConflict::TimeOverlap {
                    conflicting_event_id: existing_event.id.clone(),
                    conflicting_event_title: existing_event.title.clone(),
                    overlap_start,
                    overlap_end,
                });
            }
        }

        Ok(conflicts)
    }

    /// Check for attendee conflicts (attendees with overlapping meetings)
    async fn check_attendee_conflicts(
        &self,
        database: &CalendarDatabase,
        event: &Event,
    ) -> CalendarResult<Vec<EventConflict>> {
        let mut conflicts = Vec::new();

        // Only check if there are attendees
        if event.attendees.is_empty() {
            return Ok(conflicts);
        }

        // Get all calendars to check across different calendars
        let calendars = database.get_calendars().await.map_err(|e| {
            CalendarError::DatabaseError(format!("Failed to fetch calendars: {}", e))
        })?;

        for calendar in calendars {
            let start_range = event.start_time - Duration::minutes(30);
            let end_range = event.end_time + Duration::minutes(30);

            let existing_events = database
                .get_events(&calendar.id, Some(start_range), Some(end_range))
                .await
                .map_err(|e| {
                    CalendarError::DatabaseError(format!("Failed to fetch events: {}", e))
                })?;

            for existing_event in existing_events {
                // Skip the current event
                if existing_event.id == event.id {
                    continue;
                }

                // Check if any attendees overlap
                if event.overlaps_with(&existing_event) {
                    for our_attendee in &event.attendees {
                        for existing_attendee in &existing_event.attendees {
                            if our_attendee.email == existing_attendee.email {
                                conflicts.push(EventConflict::AttendeeConflict {
                                    attendee_email: our_attendee.email.clone(),
                                    conflicting_event_id: existing_event.id.clone(),
                                    conflicting_event_title: existing_event.title.clone(),
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(conflicts)
    }

    /// Add helpful suggestions for the event
    fn add_suggestions(&self, result: &mut ValidationResult, event: &Event) {
        // Suggest better timing
        let hour = event.start_time.hour();
        if hour < 8 || hour > 18 {
            result.suggestions.push(
                "Consider scheduling during business hours (8 AM - 6 PM) for better attendance"
                    .to_string(),
            );
        }

        // Suggest location for meetings with multiple attendees
        if event.attendees.len() > 1 && event.location.is_none() {
            result.suggestions.push(
                "Consider adding a location for meetings with multiple attendees".to_string(),
            );
        }

        // Suggest shorter meetings
        let duration = event.end_time - event.start_time;
        if duration > Duration::hours(2) {
            result.suggestions.push(
                "Consider breaking long meetings into shorter sessions for better engagement"
                    .to_string(),
            );
        }

        // Suggest description for complex meetings
        if event.attendees.len() > 5
            && event
                .description
                .as_ref()
                .map_or(true, |d| d.trim().is_empty())
        {
            result
                .suggestions
                .push("Consider adding a description for meetings with many attendees".to_string());
        }

        // Suggest reminders
        if event.reminders.is_empty() && !event.all_day {
            result
                .suggestions
                .push("Consider adding a reminder to avoid missing this event".to_string());
        }
    }

    /// Validate email address format
    fn is_valid_email(&self, email: &str) -> bool {
        // Basic email validation - check for @ symbol and basic format
        email.contains('@')
            && email.len() > 3
            && email.contains('.')
            && !email.starts_with('@')
            && !email.ends_with('@')
    }

    /// Build the event from form data
    pub fn build_event(&self) -> Event {
        let start_datetime = self.start_date.and_time(if self.is_all_day {
            NaiveTime::from_hms_opt(0, 0, 0).unwrap()
        } else {
            self.start_time
        });

        let end_datetime = self.end_date.and_time(if self.is_all_day {
            NaiveTime::from_hms_opt(23, 59, 59).unwrap()
        } else {
            self.end_time
        });

        let start_utc = Local
            .from_local_datetime(&start_datetime)
            .single()
            .unwrap()
            .with_timezone(&Utc);
        let end_utc = Local
            .from_local_datetime(&end_datetime)
            .single()
            .unwrap()
            .with_timezone(&Utc);

        let mut event = Event::new(
            self.selected_calendar_id.clone(),
            self.title_input.clone(),
            start_utc,
            end_utc,
        );

        event.description = if self.description_input.is_empty() {
            None
        } else {
            Some(self.description_input.clone())
        };

        event.location = if self.location_input.is_empty() {
            None
        } else {
            Some(self.location_input.clone())
        };

        event.status = self.event_status;
        event.all_day = self.is_all_day;
        event.recurrence = self.recurrence_rule.clone();
        event.attendees = self.attendees.clone();

        // Note: Event struct doesn't have notes field, we could extend description instead
        if !self.notes_input.is_empty() {
            if let Some(desc) = &event.description {
                event.description = Some(format!("{}\n\nNotes: {}", desc, self.notes_input));
            } else {
                event.description = Some(format!("Notes: {}", self.notes_input));
            }
        }

        // Preserve original event data for edits
        if let EventFormMode::Edit(_) = self.mode {
            event.id = self.event.id.clone();
            event.uid = self.event.uid.clone();
            event.created_at = self.event.created_at;
            event.sequence = self.event.sequence + 1;
            event.etag = self.event.etag.clone();
        }

        event
    }

    /// Check if the form has unsaved changes
    pub fn has_unsaved_changes(&self) -> bool {
        self.is_modified
    }

    /// Get the current form title based on mode
    pub fn get_form_title(&self) -> String {
        match &self.mode {
            EventFormMode::Create => "Create New Event".to_string(),
            EventFormMode::Edit(_) => format!("Edit Event: {}", self.title_input),
            EventFormMode::View(_) => format!("View Event: {}", self.title_input),
        }
    }

    /// Check if the form is in read-only mode
    pub fn is_read_only(&self) -> bool {
        matches!(self.mode, EventFormMode::View(_))
    }

    /// Handle key input for the event form
    pub async fn handle_key(&mut self, key: crossterm::event::KeyCode) -> Option<EventFormAction> {
        use crossterm::event::KeyCode;

        // Handle popup dialogs first
        if self.show_date_picker {
            match key {
                KeyCode::Esc => {
                    self.show_date_picker = false;
                }
                KeyCode::Enter => {
                    // Apply selected date
                    match self.current_field {
                        EventFormField::StartDate => {
                            self.start_date = self.date_picker.get_selected_date();
                        }
                        EventFormField::EndDate => {
                            self.end_date = self.date_picker.get_selected_date();
                        }
                        _ => {}
                    }
                    self.show_date_picker = false;
                    self.is_modified = true;
                }
                KeyCode::Left => self.date_picker.previous_month(),
                KeyCode::Right => self.date_picker.next_month(),
                KeyCode::Up => {
                    if let Some(day) = self.date_picker.selected_day {
                        if day > 7 {
                            self.date_picker.select_day(day - 7);
                        }
                    }
                }
                KeyCode::Down => {
                    if let Some(day) = self.date_picker.selected_day {
                        if day <= 24 {
                            self.date_picker.select_day(day + 7);
                        }
                    }
                }
                _ => {}
            }
            return None;
        }

        if self.show_time_picker {
            match key {
                KeyCode::Esc => {
                    self.show_time_picker = false;
                }
                KeyCode::Enter => {
                    // Apply selected time
                    match self.current_field {
                        EventFormField::StartTime => {
                            self.start_time = self.time_picker.get_selected_time();
                        }
                        EventFormField::EndTime => {
                            self.end_time = self.time_picker.get_selected_time();
                        }
                        _ => {}
                    }
                    self.show_time_picker = false;
                    self.is_modified = true;
                }
                KeyCode::Up => {
                    if matches!(
                        self.time_picker.editing_field,
                        crate::ui::time_picker::TimeField::Hour
                    ) {
                        self.time_picker.increment_hour();
                    } else {
                        self.time_picker.increment_minute();
                    }
                }
                KeyCode::Down => {
                    if matches!(
                        self.time_picker.editing_field,
                        crate::ui::time_picker::TimeField::Hour
                    ) {
                        self.time_picker.decrement_hour();
                    } else {
                        self.time_picker.decrement_minute();
                    }
                }
                KeyCode::Tab => {
                    self.time_picker.toggle_field();
                }
                _ => {}
            }
            return None;
        }

        // Handle main form input
        match key {
            KeyCode::Esc => {
                if self.is_modified {
                    // TODO: Show confirmation dialog
                }
                return Some(EventFormAction::Cancel);
            }
            KeyCode::F(1) => {
                if !self.is_read_only() {
                    return Some(EventFormAction::Save);
                }
            }
            KeyCode::F(3) => {
                if matches!(self.mode, EventFormMode::Edit(_)) {
                    return Some(EventFormAction::Delete);
                }
            }
            KeyCode::Tab => {
                self.next_field();
            }
            KeyCode::BackTab => {
                self.previous_field();
            }
            KeyCode::Enter => {
                return self.handle_enter();
            }
            KeyCode::Backspace => {
                self.handle_backspace();
            }
            KeyCode::Char(c) => {
                self.handle_char_input(c);
            }
            _ => {}
        }

        None
    }

    /// Render the event form
    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &crate::theme::Theme) {
        use ratatui::{
            layout::{Constraint, Direction, Layout},
            style::{Modifier, Style},
            widgets::{Block, Borders, Paragraph, Wrap},
        };

        // Create main layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(0),    // Form content
                Constraint::Length(3), // Instructions
            ])
            .split(area);

        // Render title
        let title_text = self.get_form_title();
        let title = Paragraph::new(title_text)
            .block(
                Block::default()
                    .title("Event Form")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.colors.palette.border)),
            )
            .style(
                Style::default()
                    .fg(theme.colors.palette.accent)
                    .add_modifier(Modifier::BOLD),
            )
            .wrap(Wrap { trim: true });
        frame.render_widget(title, chunks[0]);

        // Render form content
        self.render_form_content(frame, chunks[1], theme);

        // Render instructions
        let instructions = if self.is_read_only() {
            "Press 'e' to edit, 'd' to delete, or Esc to close"
        } else {
            "F1: Save | F3: Delete | Tab/Shift+Tab: Navigate | Esc: Cancel"
        };

        let instruction_paragraph = Paragraph::new(instructions)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(theme.colors.palette.text_muted))
            .wrap(Wrap { trim: true });
        frame.render_widget(instruction_paragraph, chunks[2]);

        // Render popups
        if self.show_date_picker {
            self.render_date_picker_popup(frame, area, theme);
        }

        if self.show_time_picker {
            self.render_time_picker_popup(frame, area, theme);
        }
    }

    /// Render the main form content
    fn render_form_content(&self, frame: &mut Frame, area: Rect, theme: &crate::theme::Theme) {
        use ratatui::layout::{Constraint, Direction, Layout};

        // Create two-column layout
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Left column fields
        let start_date_str = self.start_date.format("%Y-%m-%d").to_string();
        let start_time_str = self.start_time.format("%H:%M").to_string();

        let left_fields = vec![
            ("Title", self.title_input.as_str(), EventFormField::Title),
            (
                "Description",
                self.description_input.as_str(),
                EventFormField::Description,
            ),
            (
                "Location",
                self.location_input.as_str(),
                EventFormField::Location,
            ),
            (
                "Start Date",
                start_date_str.as_str(),
                EventFormField::StartDate,
            ),
            (
                "Start Time",
                start_time_str.as_str(),
                EventFormField::StartTime,
            ),
        ];

        // Right column fields
        let calendar_name = self
            .calendars
            .iter()
            .find(|c| c.id == self.selected_calendar_id)
            .map(|c| c.name.as_str())
            .unwrap_or("Unknown");

        let status_name = format!("{:?}", self.event_status);
        let all_day_text = if self.is_all_day { "Yes" } else { "No" };
        let end_date_str = self.end_date.format("%Y-%m-%d").to_string();
        let end_time_str = self.end_time.format("%H:%M").to_string();

        let right_fields = vec![
            ("End Date", end_date_str.as_str(), EventFormField::EndDate),
            ("End Time", end_time_str.as_str(), EventFormField::EndTime),
            ("Calendar", calendar_name, EventFormField::Calendar),
            ("Status", status_name.as_str(), EventFormField::Status),
            ("All Day", all_day_text, EventFormField::AllDay),
        ];

        // Render left column
        self.render_field_column(frame, columns[0], &left_fields, theme);

        // Render right column
        self.render_field_column(frame, columns[1], &right_fields, theme);
    }

    /// Render a column of form fields
    fn render_field_column(
        &self,
        frame: &mut Frame,
        area: Rect,
        fields: &[(&str, &str, EventFormField)],
        theme: &crate::theme::Theme,
    ) {
        use ratatui::{
            layout::{Constraint, Direction, Layout},
            style::{Modifier, Style},
            widgets::{Block, Borders, Paragraph},
        };

        let field_height = 3;
        let constraints: Vec<Constraint> = fields
            .iter()
            .map(|_| Constraint::Length(field_height))
            .collect();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        for (i, (label, value, field_type)) in fields.iter().enumerate() {
            if i < chunks.len() {
                let is_focused = self.current_field == *field_type;
                let border_style = if is_focused {
                    Style::default().fg(theme.colors.palette.accent)
                } else {
                    Style::default().fg(theme.colors.palette.border)
                };

                let content_style = if is_focused {
                    Style::default()
                        .fg(theme.colors.palette.accent)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.colors.content_preview.body)
                };

                let block = Block::default()
                    .title(*label)
                    .borders(Borders::ALL)
                    .border_style(border_style);

                let paragraph = Paragraph::new(*value).block(block).style(content_style);

                frame.render_widget(paragraph, chunks[i]);
            }
        }
    }

    /// Render date picker popup
    fn render_date_picker_popup(&self, frame: &mut Frame, area: Rect, theme: &crate::theme::Theme) {
        use ratatui::{
            layout::{Alignment, Constraint, Direction, Layout},
            style::Style,
            widgets::{Block, Borders, Clear, Paragraph},
        };

        // Calculate popup area (centered)
        let popup_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Length(10),
                Constraint::Percentage(30),
            ])
            .split(area)[1];

        let popup_area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(50),
                Constraint::Percentage(25),
            ])
            .split(popup_area)[1];

        // Clear the area
        frame.render_widget(Clear, popup_area);

        // Render date picker content
        let month_year = self.date_picker.viewing_month.format("%B %Y").to_string();
        let selected_date = self
            .date_picker
            .get_selected_date()
            .format("%Y-%m-%d")
            .to_string();

        let content = format!(
            "{}\\n\\nSelected: {}\\n\\nUse arrows to navigate\\nEnter to select, Esc to cancel",
            month_year, selected_date
        );

        let block = Block::default()
            .title("Select Date")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.colors.palette.accent));

        let paragraph = Paragraph::new(content)
            .block(block)
            .style(Style::default().fg(theme.colors.content_preview.body))
            .alignment(Alignment::Center);

        frame.render_widget(paragraph, popup_area);
    }

    /// Render time picker popup
    fn render_time_picker_popup(&self, frame: &mut Frame, area: Rect, theme: &crate::theme::Theme) {
        use ratatui::{
            layout::{Alignment, Constraint, Direction, Layout},
            style::Style,
            widgets::{Block, Borders, Clear, Paragraph},
        };

        // Calculate popup area (centered)
        let popup_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(35),
                Constraint::Length(8),
                Constraint::Percentage(35),
            ])
            .split(area)[1];

        let popup_area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(40),
                Constraint::Percentage(30),
            ])
            .split(popup_area)[1];

        // Clear the area
        frame.render_widget(Clear, popup_area);

        // Render time picker content
        let time_display = self
            .time_picker
            .get_selected_time()
            .format("%H:%M")
            .to_string();
        let editing_field = if matches!(
            self.time_picker.editing_field,
            crate::ui::time_picker::TimeField::Hour
        ) {
            "hour"
        } else {
            "minute"
        };

        let content = format!(
            "{}\\n\\nEditing: {}\\n\\nUse ↑↓ to change\\nTab to switch field\\nEnter to select, Esc to cancel",
            time_display, editing_field
        );

        let block = Block::default()
            .title("Select Time")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.colors.palette.accent));

        let paragraph = Paragraph::new(content)
            .block(block)
            .style(Style::default().fg(theme.colors.content_preview.body))
            .alignment(Alignment::Center);

        frame.render_widget(paragraph, popup_area);
    }
}

/// Date picker component for event forms
pub mod date_picker {
    use super::*;
    use chrono::Datelike;

    pub struct DatePicker {
        pub selected_date: NaiveDate,
        pub viewing_month: NaiveDate,
        pub selected_day: Option<u32>,
    }

    impl DatePicker {
        pub fn new(initial_date: NaiveDate) -> Self {
            Self {
                selected_date: initial_date,
                viewing_month: initial_date.with_day(1).unwrap(),
                selected_day: Some(initial_date.day()),
            }
        }

        pub fn next_month(&mut self) {
            if let Some(next_month) = self
                .viewing_month
                .checked_add_months(chrono::Months::new(1))
            {
                self.viewing_month = next_month;
            }
        }

        pub fn previous_month(&mut self) {
            if let Some(prev_month) = self
                .viewing_month
                .checked_sub_months(chrono::Months::new(1))
            {
                self.viewing_month = prev_month;
            }
        }

        pub fn select_day(&mut self, day: u32) {
            if let Some(new_date) = self.viewing_month.with_day(day) {
                self.selected_date = new_date;
                self.selected_day = Some(day);
            }
        }

        pub fn get_selected_date(&self) -> NaiveDate {
            self.selected_date
        }
    }
}

/// Time picker component for event forms
pub mod time_picker {
    use super::*;

    pub struct TimePicker {
        pub selected_time: NaiveTime,
        pub hour: u32,
        pub minute: u32,
        pub editing_hour: bool,
    }

    impl TimePicker {
        pub fn new(initial_time: NaiveTime) -> Self {
            Self {
                selected_time: initial_time,
                hour: initial_time.hour(),
                minute: initial_time.minute(),
                editing_hour: true,
            }
        }

        pub fn increment_hour(&mut self) {
            self.hour = (self.hour + 1) % 24;
            self.update_time();
        }

        pub fn decrement_hour(&mut self) {
            self.hour = if self.hour == 0 { 23 } else { self.hour - 1 };
            self.update_time();
        }

        pub fn increment_minute(&mut self) {
            self.minute = (self.minute + 5) % 60;
            self.update_time();
        }

        pub fn decrement_minute(&mut self) {
            self.minute = if self.minute < 5 { 55 } else { self.minute - 5 };
            self.update_time();
        }

        pub fn toggle_field(&mut self) {
            self.editing_hour = !self.editing_hour;
        }

        pub fn get_selected_time(&self) -> NaiveTime {
            self.selected_time
        }

        fn update_time(&mut self) {
            if let Some(time) = NaiveTime::from_hms_opt(self.hour, self.minute, 0) {
                self.selected_time = time;
            }
        }
    }
}
