use chrono::{Duration, Local, NaiveDate, NaiveTime, Utc, Timelike, Datelike, TimeZone};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use std::collections::HashMap;

use crate::calendar::event::{Event, EventStatus, EventAttendee, AttendeeStatus, EventRecurrence};
use crate::calendar::Calendar;
use crate::theme::Theme;
use crate::ui::date_picker::DatePicker;
use crate::ui::time_picker::TimePicker;

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
}

impl Default for EventFormValidation {
    fn default() -> Self {
        Self {
            title_error: None,
            date_error: None,
            time_error: None,
            calendar_error: None,
            attendee_error: None,
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
        if let Some(index) = form.calendars.iter().position(|c| c.id == event.calendar_id) {
            form.calendar_list_state.select(Some(index));
        }
        
        // Set initial status selection
        form.status_list_state.select(Some(form.event_status as usize));
        
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
        
        let mut attendee = EventAttendee::new(
            email.to_string(),
            None,
        );
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
        }
        
        // Date validation
        if self.end_date < self.start_date {
            self.validation.date_error = Some("End date cannot be before start date".to_string());
            is_valid = false;
        } else if self.end_date == self.start_date && self.end_time < self.start_time {
            self.validation.time_error = Some("End time cannot be before start time".to_string());
            is_valid = false;
        }
        
        // Calendar validation
        if !self.calendars.iter().any(|c| c.id == self.selected_calendar_id) {
            self.validation.calendar_error = Some("Please select a valid calendar".to_string());
            is_valid = false;
        }
        
        is_valid
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
        
        let start_utc = Local.from_local_datetime(&start_datetime).single().unwrap().with_timezone(&Utc);
        let end_utc = Local.from_local_datetime(&end_datetime).single().unwrap().with_timezone(&Utc);
        
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
            if let Some(next_month) = self.viewing_month.checked_add_months(chrono::Months::new(1)) {
                self.viewing_month = next_month;
            }
        }
        
        pub fn previous_month(&mut self) {
            if let Some(prev_month) = self.viewing_month.checked_sub_months(chrono::Months::new(1)) {
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