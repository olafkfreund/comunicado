use chrono::{Datelike, NaiveDate, Weekday};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Clear, Paragraph, Table, Row, Cell},
    Frame,
};

use crate::theme::Theme;

/// Date picker widget for selecting dates
pub struct DatePicker {
    pub selected_date: NaiveDate,
    pub viewing_month: NaiveDate,
    pub selected_day: Option<u32>,
    pub is_open: bool,
}

impl DatePicker {
    /// Create a new date picker with the given initial date
    pub fn new(initial_date: NaiveDate) -> Self {
        Self {
            selected_date: initial_date,
            viewing_month: initial_date.with_day(1).unwrap(),
            selected_day: Some(initial_date.day()),
            is_open: false,
        }
    }
    
    /// Open the date picker
    pub fn open(&mut self) {
        self.is_open = true;
    }
    
    /// Close the date picker
    pub fn close(&mut self) {
        self.is_open = false;
    }
    
    /// Toggle the date picker open/closed state
    pub fn toggle(&mut self) {
        self.is_open = !self.is_open;
    }
    
    /// Move to the next month
    pub fn next_month(&mut self) {
        if let Some(next_month) = self.viewing_month.checked_add_months(chrono::Months::new(1)) {
            self.viewing_month = next_month;
        }
    }
    
    /// Move to the previous month
    pub fn previous_month(&mut self) {
        if let Some(prev_month) = self.viewing_month.checked_sub_months(chrono::Months::new(1)) {
            self.viewing_month = prev_month;
        }
    }
    
    /// Move to the next year
    pub fn next_year(&mut self) {
        if let Some(next_year) = self.viewing_month.checked_add_months(chrono::Months::new(12)) {
            self.viewing_month = next_year;
        }
    }
    
    /// Move to the previous year
    pub fn previous_year(&mut self) {
        if let Some(prev_year) = self.viewing_month.checked_sub_months(chrono::Months::new(12)) {
            self.viewing_month = prev_year;
        }
    }
    
    /// Select a specific day in the current viewing month
    pub fn select_day(&mut self, day: u32) -> bool {
        if let Some(new_date) = self.viewing_month.with_day(day) {
            self.selected_date = new_date;
            self.selected_day = Some(day);
            true
        } else {
            false
        }
    }
    
    /// Move selection to the next day
    pub fn next_day(&mut self) {
        let current_day = self.selected_day.unwrap_or(1);
        let days_in_month = self.days_in_viewing_month();
        
        if current_day < days_in_month {
            self.select_day(current_day + 1);
        } else {
            // Move to next month, first day
            self.next_month();
            self.select_day(1);
        }
    }
    
    /// Move selection to the previous day
    pub fn previous_day(&mut self) {
        let current_day = self.selected_day.unwrap_or(1);
        
        if current_day > 1 {
            self.select_day(current_day - 1);
        } else {
            // Move to previous month, last day
            self.previous_month();
            let last_day = self.days_in_viewing_month();
            self.select_day(last_day);
        }
    }
    
    /// Move selection one week forward
    pub fn next_week(&mut self) {
        let current_day = self.selected_day.unwrap_or(1);
        let days_in_month = self.days_in_viewing_month();
        
        if current_day + 7 <= days_in_month {
            self.select_day(current_day + 7);
        } else {
            // Move to next month
            self.next_month();
            let overflow = (current_day + 7) - days_in_month;
            self.select_day(overflow);
        }
    }
    
    /// Move selection one week backward
    pub fn previous_week(&mut self) {
        let current_day = self.selected_day.unwrap_or(1);
        
        if current_day > 7 {
            self.select_day(current_day - 7);
        } else {
            // Move to previous month
            self.previous_month();
            let prev_month_days = self.days_in_viewing_month();
            let new_day = prev_month_days - (7 - current_day);
            self.select_day(new_day);
        }
    }
    
    /// Get the currently selected date
    pub fn get_selected_date(&self) -> NaiveDate {
        self.selected_date
    }
    
    /// Get the number of days in the currently viewing month
    pub fn days_in_viewing_month(&self) -> u32 {
        // Get the last day of the month by going to the first day of next month and subtracting 1
        let next_month = if self.viewing_month.month() == 12 {
            NaiveDate::from_ymd_opt(self.viewing_month.year() + 1, 1, 1).unwrap()
        } else {
            NaiveDate::from_ymd_opt(self.viewing_month.year(), self.viewing_month.month() + 1, 1).unwrap()
        };
        
        let last_day_of_month = next_month.pred_opt().unwrap();
        last_day_of_month.day()
    }
    
    /// Get the weekday of the first day of the viewing month (0 = Sunday, 6 = Saturday)
    pub fn first_day_weekday(&self) -> u32 {
        match self.viewing_month.weekday() {
            Weekday::Sun => 0,
            Weekday::Mon => 1,
            Weekday::Tue => 2,
            Weekday::Wed => 3,
            Weekday::Thu => 4,
            Weekday::Fri => 5,
            Weekday::Sat => 6,
        }
    }
    
    /// Render the date picker
    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.is_open {
            return;
        }
        
        // Clear the area
        frame.render_widget(Clear, area);
        
        // Create the calendar layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header with month/year
                Constraint::Length(1), // Day names
                Constraint::Min(6),    // Calendar grid
            ])
            .split(area);
        
        // Render header with month and year
        self.render_header(frame, chunks[0], theme);
        
        // Render day names
        self.render_day_names(frame, chunks[1], theme);
        
        // Render calendar grid
        self.render_calendar_grid(frame, chunks[2], theme);
    }
    
    /// Render the header with month/year and navigation
    fn render_header(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let month_names = [
            "January", "February", "March", "April", "May", "June",
            "July", "August", "September", "October", "November", "December"
        ];
        
        let month_name = month_names.get((self.viewing_month.month() - 1) as usize)
            .unwrap_or(&"Unknown");
        
        let header_text = format!("{} {}", month_name, self.viewing_month.year());
        
        let header = Paragraph::new(header_text)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Select Date"))
            .alignment(Alignment::Center);
        
        frame.render_widget(header, area);
    }
    
    /// Render the day names header
    fn render_day_names(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let day_names = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];
        
        let cells: Vec<Cell> = day_names
            .iter()
            .map(|&name| Cell::from(name))
            .collect();
        
        let row = Row::new(cells);
        let table = Table::new(vec![row], &[Constraint::Ratio(1, 7); 7]);
        
        frame.render_widget(table, area);
    }
    
    /// Render the calendar grid with dates
    fn render_calendar_grid(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let days_in_month = self.days_in_viewing_month();
        let first_weekday = self.first_day_weekday();
        
        let mut rows = Vec::new();
        let mut current_week = Vec::new();
        
        // Add empty cells for days before the first day of the month
        for _ in 0..first_weekday {
            current_week.push(Cell::from(""));
        }
        
        // Add all days of the month
        for day in 1..=days_in_month {
            let is_selected = self.selected_day == Some(day);
            let is_today = {
                let today = chrono::Local::now().date_naive();
                self.viewing_month.with_day(day) == Some(today)
            };
            
            let style = ratatui::style::Style::default();
            
            let cell = Cell::from(format!("{:2}", day)).style(style);
            current_week.push(cell);
            
            // If we've filled a week (7 days) or reached the end, start a new row
            if current_week.len() == 7 {
                rows.push(Row::new(current_week.clone()));
                current_week.clear();
            }
        }
        
        // Add remaining empty cells if the last week is incomplete
        while current_week.len() < 7 && !current_week.is_empty() {
            current_week.push(Cell::from(""));
        }
        
        if !current_week.is_empty() {
            rows.push(Row::new(current_week));
        }
        
        let table = Table::new(rows, &[Constraint::Ratio(1, 7); 7])
            .block(Block::default().borders(Borders::ALL));
        
        frame.render_widget(table, area);
    }
    
    /// Handle navigation keys
    pub fn handle_key(&mut self, key: char) -> bool {
        match key {
            'h' | 'H' => {
                self.previous_day();
                true
            }
            'l' | 'L' => {
                self.next_day();
                true
            }
            'j' | 'J' => {
                self.next_week();
                true
            }
            'k' | 'K' => {
                self.previous_week();
                true
            }
            'n' | 'N' => {
                self.next_month();
                true
            }
            'p' | 'P' => {
                self.previous_month();
                true
            }
            'y' | 'Y' => {
                self.next_year();
                true
            }
            'u' | 'U' => {
                self.previous_year();
                true
            }
            't' | 'T' => {
                // Go to today
                let today = chrono::Local::now().date_naive();
                self.viewing_month = today.with_day(1).unwrap();
                self.selected_date = today;
                self.selected_day = Some(today.day());
                true
            }
            _ => false,
        }
    }
    
    /// Handle number input for direct day selection
    pub fn handle_number(&mut self, digit: char) -> bool {
        if let Some(d) = digit.to_digit(10) {
            // For simplicity, handle single digit selection
            // In a more sophisticated implementation, you could handle multi-digit input
            if d >= 1 && d <= 9 {
                let day = d;
                if day <= self.days_in_viewing_month() {
                    self.select_day(day);
                    return true;
                }
            }
        }
        false
    }
}