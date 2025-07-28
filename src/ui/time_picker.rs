use chrono::{NaiveTime, Timelike};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::theme::Theme;

/// Time picker component for selecting times
pub struct TimePicker {
    pub selected_time: NaiveTime,
    pub hour: u32,
    pub minute: u32,
    pub editing_field: TimeField,
    pub is_open: bool,
    pub use_24_hour: bool,
}

/// Which field is currently being edited
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeField {
    Hour,
    Minute,
}

impl TimePicker {
    /// Create a new time picker with the given initial time
    pub fn new(initial_time: NaiveTime) -> Self {
        Self {
            selected_time: initial_time,
            hour: initial_time.hour(),
            minute: initial_time.minute(),
            editing_field: TimeField::Hour,
            is_open: false,
            use_24_hour: true,
        }
    }
    
    /// Open the time picker
    pub fn open(&mut self) {
        self.is_open = true;
    }
    
    /// Close the time picker
    pub fn close(&mut self) {
        self.is_open = false;
    }
    
    /// Toggle the time picker open/closed state
    pub fn toggle(&mut self) {
        self.is_open = !self.is_open;
    }
    
    /// Set whether to use 24-hour format
    pub fn set_24_hour_format(&mut self, use_24_hour: bool) {
        self.use_24_hour = use_24_hour;
    }
    
    /// Increment the hour
    pub fn increment_hour(&mut self) {
        self.hour = (self.hour + 1) % 24;
        self.update_time();
    }
    
    /// Decrement the hour
    pub fn decrement_hour(&mut self) {
        self.hour = if self.hour == 0 { 23 } else { self.hour - 1 };
        self.update_time();
    }
    
    /// Increment the minute by 5
    pub fn increment_minute(&mut self) {
        self.minute = (self.minute + 5) % 60;
        self.update_time();
    }
    
    /// Decrement the minute by 5
    pub fn decrement_minute(&mut self) {
        self.minute = if self.minute < 5 { 
            60 - (5 - self.minute) 
        } else { 
            self.minute - 5 
        };
        self.update_time();
    }
    
    /// Increment the minute by 1
    pub fn increment_minute_fine(&mut self) {
        self.minute = (self.minute + 1) % 60;
        self.update_time();
    }
    
    /// Decrement the minute by 1
    pub fn decrement_minute_fine(&mut self) {
        self.minute = if self.minute == 0 { 59 } else { self.minute - 1 };
        self.update_time();
    }
    
    /// Toggle between hour and minute editing
    pub fn toggle_field(&mut self) {
        self.editing_field = match self.editing_field {
            TimeField::Hour => TimeField::Minute,
            TimeField::Minute => TimeField::Hour,
        };
    }
    
    /// Set the editing field
    pub fn set_editing_field(&mut self, field: TimeField) {
        self.editing_field = field;
    }
    
    /// Get the currently selected time
    pub fn get_selected_time(&self) -> NaiveTime {
        self.selected_time
    }
    
    /// Set the time directly
    pub fn set_time(&mut self, time: NaiveTime) {
        self.selected_time = time;
        self.hour = time.hour();
        self.minute = time.minute();
    }
    
    /// Update the internal time from hour/minute values
    fn update_time(&mut self) {
        if let Some(time) = NaiveTime::from_hms_opt(self.hour, self.minute, 0) {
            self.selected_time = time;
        }
    }
    
    /// Format the time for display
    pub fn format_time(&self) -> String {
        if self.use_24_hour {
            format!("{:02}:{:02}", self.hour, self.minute)
        } else {
            let (display_hour, am_pm) = if self.hour == 0 {
                (12, "AM")
            } else if self.hour < 12 {
                (self.hour, "AM")
            } else if self.hour == 12 {
                (12, "PM")
            } else {
                (self.hour - 12, "PM")
            };
            format!("{:2}:{:02} {}", display_hour, self.minute, am_pm)
        }
    }
    
    /// Get AM/PM string for 12-hour format
    pub fn get_am_pm(&self) -> &'static str {
        if self.hour < 12 { "AM" } else { "PM" }
    }
    
    /// Render the time picker
    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.is_open {
            return;
        }
        
        // Clear the area
        frame.render_widget(Clear, area);
        
        // Create the time picker layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Length(5), // Time display
                Constraint::Length(3), // Instructions
            ])
            .split(area);
        
        // Render title
        self.render_title(frame, chunks[0], theme);
        
        // Render time display
        self.render_time_display(frame, chunks[1], theme);
        
        // Render instructions
        self.render_instructions(frame, chunks[2], theme);
    }
    
    /// Render the title
    fn render_title(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let title = Paragraph::new("Select Time")
            .block(Block::default()
                .borders(Borders::ALL))
            .alignment(Alignment::Center);
        
        frame.render_widget(title, area);
    }
    
    /// Render the time display with highlighted editing field
    fn render_time_display(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let time_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(4), // Hour
                Constraint::Length(1), // Colon
                Constraint::Length(4), // Minute
                Constraint::Length(4), // AM/PM (if 12-hour format)
            ])
            .split(area);
        
        // Hour display
        let hour_text = if self.use_24_hour {
            format!("{:02}", self.hour)
        } else {
            let display_hour = if self.hour == 0 || self.hour == 12 {
                12
            } else {
                self.hour % 12
            };
            format!("{:2}", display_hour)
        };
        
        let hour_style = ratatui::style::Style::default();
        
        let hour_widget = Paragraph::new(hour_text)
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Center)
            .style(hour_style);
        
        frame.render_widget(hour_widget, time_chunks[0]);
        
        // Colon separator
        let colon = Paragraph::new(":")
            .alignment(Alignment::Center);
        
        frame.render_widget(colon, time_chunks[1]);
        
        // Minute display
        let minute_text = format!("{:02}", self.minute);
        
        let minute_style = ratatui::style::Style::default();
        
        let minute_widget = Paragraph::new(minute_text)
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Center)
            .style(minute_style);
        
        frame.render_widget(minute_widget, time_chunks[2]);
        
        // AM/PM display for 12-hour format
        if !self.use_24_hour {
            let am_pm_text = self.get_am_pm();
            let am_pm_widget = Paragraph::new(am_pm_text)
                .alignment(Alignment::Center);
            
            frame.render_widget(am_pm_widget, time_chunks[3]);
        }
    }
    
    /// Render the instructions
    fn render_instructions(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let instructions = if self.use_24_hour {
            "↑/↓: Change time  Tab: Switch field  Enter: Confirm  Esc: Cancel"
        } else {
            "↑/↓: Change time  Tab: Switch field  a/p: AM/PM  Enter: Confirm  Esc: Cancel"
        };
        
        let instructions_widget = Paragraph::new(instructions)
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Center)
            .wrap(ratatui::widgets::Wrap { trim: true });
        
        frame.render_widget(instructions_widget, area);
    }
    
    /// Handle navigation keys
    pub fn handle_key(&mut self, key: char) -> bool {
        match key {
            'k' | 'K' => {
                // Up arrow - increment current field
                match self.editing_field {
                    TimeField::Hour => self.increment_hour(),
                    TimeField::Minute => self.increment_minute(),
                }
                true
            }
            'j' | 'J' => {
                // Down arrow - decrement current field
                match self.editing_field {
                    TimeField::Hour => self.decrement_hour(),
                    TimeField::Minute => self.decrement_minute(),
                }
                true
            }
            'h' | 'H' => {
                // Left arrow - move to hour field
                self.set_editing_field(TimeField::Hour);
                true
            }
            'l' | 'L' => {
                // Right arrow - move to minute field
                self.set_editing_field(TimeField::Minute);
                true
            }
            '\t' => {
                // Tab - toggle field
                self.toggle_field();
                true
            }
            '+' | '=' => {
                // Plus - fine increment
                match self.editing_field {
                    TimeField::Hour => self.increment_hour(),
                    TimeField::Minute => self.increment_minute_fine(),
                }
                true
            }
            '-' | '_' => {
                // Minus - fine decrement
                match self.editing_field {
                    TimeField::Hour => self.decrement_hour(),
                    TimeField::Minute => self.decrement_minute_fine(),
                }
                true
            }
            'a' | 'A' => {
                // AM - set to AM for 12-hour format
                if !self.use_24_hour && self.hour >= 12 {
                    self.hour -= 12;
                    self.update_time();
                }
                true
            }
            'p' | 'P' => {
                // PM - set to PM for 12-hour format
                if !self.use_24_hour && self.hour < 12 {
                    self.hour += 12;
                    self.update_time();
                }
                true
            }
            'n' | 'N' => {
                // Now - set to current time
                let now = chrono::Local::now().time();
                self.set_time(now);
                true
            }
            _ => false,
        }
    }
    
    /// Handle number input for direct time entry
    pub fn handle_number(&mut self, digit: char) -> bool {
        if let Some(d) = digit.to_digit(10) {
            match self.editing_field {
                TimeField::Hour => {
                    let new_hour = d;
                    if new_hour < 24 {
                        self.hour = new_hour;
                        self.update_time();
                        // Auto-advance to minute field for convenience
                        self.set_editing_field(TimeField::Minute);
                        return true;
                    }
                }
                TimeField::Minute => {
                    // For minutes, handle two-digit input
                    let new_minute = d * 10; // First digit of minutes
                    if new_minute < 60 {
                        self.minute = new_minute;
                        self.update_time();
                        return true;
                    }
                }
            }
        }
        false
    }
    
    /// Reset to a specific time
    pub fn reset_to_time(&mut self, time: NaiveTime) {
        self.set_time(time);
    }
    
    /// Get a formatted string for display in other components
    pub fn display_string(&self) -> String {
        self.format_time()
    }
}