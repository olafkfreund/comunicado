use crate::{
    calendar::{Event, EventPriority, EventStatus},
    theme::Theme,
};
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, Timelike, Utc};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs, Wrap},
    Frame,
};
use std::collections::HashMap;

/// Calendar view modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalendarViewMode {
    Month,
    Week,
    Day,
    Agenda,
}

impl CalendarViewMode {
    pub fn name(&self) -> &'static str {
        match self {
            CalendarViewMode::Month => "Month",
            CalendarViewMode::Week => "Week",
            CalendarViewMode::Day => "Day",
            CalendarViewMode::Agenda => "Agenda",
        }
    }

    pub fn all() -> &'static [CalendarViewMode] {
        &[
            CalendarViewMode::Month,
            CalendarViewMode::Week,
            CalendarViewMode::Day,
            CalendarViewMode::Agenda,
        ]
    }
}

/// Calendar action events
#[derive(Debug, Clone)]
pub enum CalendarAction {
    NextPeriod,
    PreviousPeriod,
    Today,
    ChangeView(CalendarViewMode),
    SelectEvent(String), // Event ID
    CreateEvent,
    EditEvent(String),
    DeleteEvent(String),
    CancelEvent(String),
    ShowEventDetails(String),
    ToggleCalendar(String), // Calendar ID
    Refresh,
    Search(String),
    ExportCalendar,
    ImportCalendar,
}

/// Calendar UI state
pub struct CalendarUI {
    // View state
    current_view: CalendarViewMode,
    current_date: DateTime<Local>,
    selected_date: NaiveDate,
    #[allow(dead_code)] // Used through get_selected_event_id() method
    selected_event_id: Option<String>,

    // Data
    events: Vec<Event>,
    calendars: Vec<crate::calendar::Calendar>,
    enabled_calendars: std::collections::HashSet<String>,

    // Navigation
    view_tab_index: usize,
    event_list_state: ListState,
    calendar_list_state: ListState,

    // UI state
    show_event_details: bool,
    show_calendar_list: bool,
    is_focused: bool,
    focused_pane: CalendarPane,

    // Search functionality placeholder

    // Event details
    selected_event: Option<Event>,

    // Deletion confirmation
    show_delete_confirmation: bool,
    event_to_delete: Option<String>,     // Event ID to delete
    delete_confirmation_selected: usize, // 0 = Cancel, 1 = Delete
}

/// Calendar UI panes for focus management
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalendarPane {
    Calendar,
    EventList,
    CalendarList,
    EventDetails,
}

impl CalendarUI {
    pub fn new() -> Self {
        let mut enabled_calendars = std::collections::HashSet::new();
        // By default, enable local calendar
        enabled_calendars.insert("local".to_string());

        Self {
            current_view: CalendarViewMode::Month,
            current_date: Local::now(),
            selected_date: Local::now().date_naive(),
            selected_event_id: None,
            events: Vec::new(),
            calendars: Vec::new(),
            enabled_calendars,
            view_tab_index: 0,
            event_list_state: ListState::default(),
            calendar_list_state: ListState::default(),
            show_event_details: false,
            show_calendar_list: false,
            is_focused: true,
            focused_pane: CalendarPane::Calendar,
            selected_event: None,
            show_delete_confirmation: false,
            event_to_delete: None,
            delete_confirmation_selected: 0,
        }
    }

    /// Render the calendar UI
    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Create main layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Tabs and controls
                Constraint::Min(10),   // Main content
                Constraint::Length(1), // Status line
            ])
            .split(area);

        // Render tabs and controls
        self.render_header(frame, chunks[0], theme);

        // Render main content based on current view
        match self.current_view {
            CalendarViewMode::Month => self.render_month_view(frame, chunks[1], theme),
            CalendarViewMode::Week => self.render_week_view(frame, chunks[1], theme),
            CalendarViewMode::Day => self.render_day_view(frame, chunks[1], theme),
            CalendarViewMode::Agenda => self.render_agenda_view(frame, chunks[1], theme),
        }

        // Render status line
        self.render_status_line(frame, chunks[2], theme);

        // Render overlays
        if self.show_event_details {
            self.render_event_details_overlay(frame, area, theme);
        }

        if self.show_calendar_list {
            self.render_calendar_list_overlay(frame, area, theme);
        }

        if self.show_delete_confirmation {
            self.render_delete_confirmation_dialog(frame, area, theme);
        }
    }

    /// Render header with tabs and navigation
    fn render_header(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(30),    // Tabs
                Constraint::Length(20), // Date navigation
                Constraint::Length(15), // View controls
            ])
            .split(area);

        // Render view mode tabs
        let tab_titles: Vec<Line> = CalendarViewMode::all()
            .iter()
            .map(|mode| Line::from(mode.name()))
            .collect();

        let tabs = Tabs::new(tab_titles)
            .block(Block::default().borders(Borders::ALL).title("View"))
            .highlight_style(theme.get_component_style("calendar_tab_selected", true))
            .select(self.view_tab_index);

        frame.render_widget(tabs, chunks[0]);

        // Render date navigation
        let date_text = match self.current_view {
            CalendarViewMode::Month => self.current_date.format("%B %Y").to_string(),
            CalendarViewMode::Week => {
                let week_start = self.get_week_start();
                let week_end = week_start + Duration::days(6);
                format!(
                    "{} - {}",
                    week_start.format("%b %d"),
                    week_end.format("%b %d, %Y")
                )
            }
            CalendarViewMode::Day => self.current_date.format("%A, %B %d, %Y").to_string(),
            CalendarViewMode::Agenda => "Upcoming Events".to_string(),
        };

        let date_para = Paragraph::new(date_text)
            .block(Block::default().borders(Borders::ALL).title("Period"))
            .style(theme.get_component_style("calendar_header", self.is_focused))
            .wrap(Wrap { trim: true });

        frame.render_widget(date_para, chunks[1]);

        // Render view controls
        let controls_text = match self.focused_pane {
            CalendarPane::Calendar => "h/l: Navigate, Space: Today, c: Create",
            CalendarPane::EventList => "j/k: Navigate, Enter: Details",
            CalendarPane::CalendarList => "j/k: Navigate, Space: Toggle",
            CalendarPane::EventDetails => "Esc: Close, e: Edit, d: Delete",
        };

        let controls = Paragraph::new(controls_text)
            .block(Block::default().borders(Borders::ALL).title("Controls"))
            .style(theme.get_component_style("calendar_controls", self.is_focused))
            .wrap(Wrap { trim: true });

        frame.render_widget(controls, chunks[2]);
    }

    /// Render month view
    fn render_month_view(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(70), // Calendar
                Constraint::Percentage(30), // Event list
            ])
            .split(area);

        // Render calendar grid
        self.render_month_calendar(frame, chunks[0], theme);

        // Render events for selected date
        self.render_event_list(frame, chunks[1], theme);
    }

    /// Render week view
    fn render_week_view(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Week header
                Constraint::Min(10),   // Week grid
            ])
            .split(area);

        // Render week header with days
        self.render_week_header(frame, chunks[0], theme);

        // Render week grid with events
        self.render_week_grid(frame, chunks[1], theme);
    }

    /// Render day view
    fn render_day_view(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(75), // Day schedule
                Constraint::Percentage(25), // Day summary
            ])
            .split(area);

        // Render day schedule
        self.render_day_schedule(frame, chunks[0], theme);

        // Render day summary
        self.render_day_summary(frame, chunks[1], theme);
    }

    /// Render agenda view
    fn render_agenda_view(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(60), // Upcoming events
                Constraint::Percentage(40), // Calendar filters
            ])
            .split(area);

        // Render upcoming events list
        self.render_upcoming_events(frame, chunks[0], theme);

        // Render calendar filter list
        self.render_calendar_filters(frame, chunks[1], theme);
    }

    /// Render month calendar grid
    fn render_month_calendar(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let is_focused = self.focused_pane == CalendarPane::Calendar;

        // Create custom calendar view since ratatui doesn't have Calendar widget
        let calendar_block = Block::default()
            .borders(Borders::ALL)
            .title("Calendar")
            .border_style(theme.get_component_style("border", is_focused));

        frame.render_widget(calendar_block, area);

        // Render custom calendar grid
        self.render_custom_calendar_grid(frame, area.inner(&Margin::new(1, 1)), theme);
    }

    /// Render custom calendar grid
    fn render_custom_calendar_grid(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Calculate dimensions for calendar cells
        let cell_width = area.width / 7;
        let cell_height = (area.height - 1) / 6; // -1 for header row
        
        // Render day header row
        let header_area = Rect::new(area.x, area.y, area.width, 1);
        let header_text = "Mo     Tu     We     Th     Fr     Sa     Su";
        let header_para = Paragraph::new(header_text)
            .style(theme.get_component_style("calendar_day_header", false))
            .block(Block::default());
        frame.render_widget(header_para, header_area);

        // Get first day of the month and calculate start date
        let first_of_month = NaiveDate::from_ymd_opt(
            self.selected_date.year(), 
            self.selected_date.month(), 
            1
        ).unwrap();
        let days_from_monday = first_of_month.weekday().num_days_from_monday();
        let start_date = first_of_month - Duration::days(days_from_monday as i64);

        // Get events grouped by date for easy lookup
        let events_by_date = self.group_events_by_date();
        let today = Local::now().date_naive();

        // Render calendar cells
        for week in 0..6 {
            for day in 0..7 {
                let current_date = start_date + Duration::days((week * 7 + day) as i64);
                
                let x = area.x + (day as u16) * cell_width;
                let y = area.y + 1 + (week as u16) * cell_height; // +1 for header
                let cell_area = Rect::new(x, y, cell_width, cell_height);
                
                self.render_calendar_day_cell(
                    frame, 
                    cell_area, 
                    current_date, 
                    &events_by_date,
                    today,
                    theme
                );
            }
        }
    }

    /// Render a single calendar day cell with events
    fn render_calendar_day_cell(
        &self,
        frame: &mut Frame,
        area: Rect,
        date: NaiveDate,
        events_by_date: &HashMap<NaiveDate, Vec<&Event>>,
        today: NaiveDate,
        theme: &Theme,
    ) {
        // Determine cell styling
        let is_today = date == today;
        let is_selected = date == self.selected_date;
        let is_current_month = date.month() == self.selected_date.month();
        
        let border_style = if is_selected {
            theme.get_component_style("calendar_selected", true)
        } else if is_today {
            theme.get_component_style("calendar_today", true)
        } else {
            theme.get_component_style("calendar_grid", false)
        };

        let day_style = if is_current_month {
            if is_today {
                theme.get_component_style("calendar_today", true)
            } else {
                theme.get_component_style("calendar_day", false)
            }
        } else {
            theme.get_component_style("calendar_day_other_month", false)
        };

        // Create cell block with border
        let cell_block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style);
        
        frame.render_widget(cell_block, area);
        
        // Get inner area for content
        let inner_area = area.inner(&Margin::new(1, 1));
        
        if inner_area.height == 0 {
            return; // No space for content
        }

        // Render day number at the top
        let day_number = format!("{:2}", date.day());
        let day_area = Rect::new(inner_area.x, inner_area.y, inner_area.width, 1);
        let day_para = Paragraph::new(day_number)
            .style(day_style)
            .alignment(Alignment::Left);
        frame.render_widget(day_para, day_area);

        // Render events if there's space
        if inner_area.height > 1 {
            let events_area = Rect::new(
                inner_area.x, 
                inner_area.y + 1, 
                inner_area.width, 
                inner_area.height - 1
            );
            
            if let Some(day_events) = events_by_date.get(&date) {
                self.render_day_cell_events(frame, events_area, day_events, theme);
            }
        }
    }

    /// Render events within a day cell
    fn render_day_cell_events(
        &self,
        frame: &mut Frame,
        area: Rect,
        events: &[&Event],
        theme: &Theme,
    ) {
        let max_events = area.height as usize;
        let visible_events = &events[..events.len().min(max_events)];
        
        for (i, event) in visible_events.iter().enumerate() {
            let y = area.y + i as u16;
            let event_area = Rect::new(area.x, y, area.width, 1);
            
            // Create event display text
            let event_text = if event.all_day {
                if event.title.len() > area.width as usize {
                    format!("{}…", &event.title[..area.width.saturating_sub(1) as usize])
                } else {
                    event.title.clone()
                }
            } else {
                let time_str = event.start_time.format("%H:%M").to_string();
                let title_space = area.width.saturating_sub(6) as usize; // 6 chars for time + space
                let title = if event.title.len() > title_space {
                    format!("{}…", &event.title[..title_space.saturating_sub(1)])
                } else {
                    event.title.clone()
                };
                format!("{} {}", time_str, title)
            };

            // Style based on event priority and status
            let event_style = match event.priority {
                EventPriority::High => Style::default()
                    .fg(Color::Red)
                    .add_modifier(Modifier::BOLD),
                EventPriority::Normal => match event.status {
                    EventStatus::Confirmed => Style::default().fg(Color::Cyan),
                    EventStatus::Tentative => Style::default().fg(Color::Yellow),
                    EventStatus::Cancelled => Style::default()
                        .fg(Color::Gray)
                        .add_modifier(Modifier::CROSSED_OUT),
                },
                EventPriority::Low => Style::default().fg(Color::Gray),
            };

            let event_para = Paragraph::new(event_text)
                .style(event_style);
            
            frame.render_widget(event_para, event_area);
        }
        
        // Show overflow indicator if there are more events
        if events.len() > max_events {
            let overflow_y = area.y + (max_events.saturating_sub(1)) as u16;
            let overflow_area = Rect::new(area.x, overflow_y, area.width, 1);
            let overflow_text = format!("+{} more", events.len() - max_events);
            let overflow_para = Paragraph::new(overflow_text)
                .style(theme.get_component_style("calendar_event_overflow", false));
            frame.render_widget(overflow_para, overflow_area);
        }
    }



    /// Group events by date for calendar display
    fn group_events_by_date(&self) -> HashMap<NaiveDate, Vec<&Event>> {
        let mut events_by_date: HashMap<NaiveDate, Vec<&Event>> = HashMap::new();

        for event in &self.events {
            if self.enabled_calendars.contains(&event.calendar_id) {
                let event_date = event.start_time.date_naive();
                events_by_date.entry(event_date).or_default().push(event);
            }
        }

        events_by_date
    }

    /// Render event list for current selection
    fn render_event_list(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let is_focused = self.focused_pane == CalendarPane::EventList;

        // Get events for selected date
        let selected_events: Vec<&Event> = self
            .events
            .iter()
            .filter(|event| {
                self.enabled_calendars.contains(&event.calendar_id)
                    && event.start_time.date_naive() == self.selected_date
            })
            .collect();

        // Create list items
        let list_items: Vec<ListItem> = selected_events
            .iter()
            .map(|event| Self::create_event_list_item_static(event, theme))
            .collect();

        let title = format!("Events ({})", selected_events.len());
        let events_list = List::new(list_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(theme.get_component_style("border", is_focused)),
            )
            .highlight_style(theme.get_component_style("list_selected", is_focused))
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(events_list, area, &mut self.event_list_state);
    }

    /// Create list item for an event (static version to avoid borrowing issues)
    fn create_event_list_item_static<'a>(event: &'a Event, _theme: &'a Theme) -> ListItem<'a> {
        let time_str = if event.all_day {
            "All Day".to_string()
        } else {
            event.start_time.format("%H:%M").to_string()
        };

        let status_symbol = match event.status {
            EventStatus::Confirmed => "●",
            EventStatus::Tentative => "◐",
            EventStatus::Cancelled => "✗",
        };

        let priority_color = match event.priority {
            EventPriority::High => Color::Red,
            EventPriority::Normal => Color::White,
            EventPriority::Low => Color::Gray,
        };

        let spans = vec![
            Span::styled(
                format!("{} ", status_symbol),
                Style::default().fg(priority_color),
            ),
            Span::styled(format!("{} ", time_str), Style::default().fg(Color::Cyan)),
            Span::styled(event.title.clone(), Style::default().fg(Color::White)),
        ];

        ListItem::new(Line::from(spans))
    }

    /// Render week header with day names
    fn render_week_header(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let week_start = self.get_week_start();
        let days = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

        let day_width = area.width / 7;

        for (i, day_name) in days.iter().enumerate() {
            let current_date = week_start + Duration::days(i as i64);
            let day_text = format!("{}\n{}", day_name, current_date.day());

            let x = area.x + (i as u16) * day_width;
            let day_area = Rect::new(x, area.y, day_width, area.height);

            let is_today = current_date.date_naive() == Local::now().date_naive();
            let style = if is_today {
                theme.get_component_style("calendar_today", true)
            } else {
                theme.get_component_style("calendar_day_header", false)
            };

            let day_para = Paragraph::new(day_text)
                .block(Block::default().borders(Borders::ALL))
                .style(style);

            frame.render_widget(day_para, day_area);
        }
    }

    /// Render week grid with time slots and events
    fn render_week_grid(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Create time slots (6 AM to 10 PM in 2-hour increments)
        let time_slots = (6..=22).step_by(2).collect::<Vec<_>>();
        let slot_height = area.height / time_slots.len() as u16;

        let week_start = self.get_week_start();
        let day_width = area.width / 7;

        // Render time grid
        for (slot_idx, hour) in time_slots.iter().enumerate() {
            let y = area.y + (slot_idx as u16) * slot_height;

            // Render time label
            let time_area = Rect::new(area.x, y, 5, slot_height);
            let time_text = format!("{:02}:00", hour);
            let time_para = Paragraph::new(time_text)
                .style(theme.get_component_style("calendar_time_label", false));
            frame.render_widget(time_para, time_area);

            // Render day columns
            for day in 0..7 {
                let x = area.x + 5 + (day as u16) * day_width;
                let day_area = Rect::new(x, y, day_width, slot_height);

                // Draw grid cell
                let cell_block = Block::default()
                    .borders(Borders::ALL)
                    .border_style(theme.get_component_style("calendar_grid", false));
                frame.render_widget(cell_block, day_area);

                // Render events in this time slot
                let current_date = week_start + Duration::days(day as i64);
                self.render_events_in_time_slot(
                    frame,
                    day_area.inner(&Margin::new(1, 0)),
                    current_date,
                    *hour,
                    theme,
                );
            }
        }
    }

    /// Render events within a specific time slot
    fn render_events_in_time_slot(
        &self,
        frame: &mut Frame,
        area: Rect,
        date: DateTime<Local>,
        hour: i32,
        _theme: &Theme,
    ) {
        let slot_start = date.date_naive().and_hms_opt(hour as u32, 0, 0).unwrap();
        let slot_end = slot_start + Duration::hours(2);

        let events_in_slot: Vec<&Event> = self
            .events
            .iter()
            .filter(|event| {
                if !self.enabled_calendars.contains(&event.calendar_id) {
                    return false;
                }

                let event_start = event.start_time.naive_local();
                let event_end = event.end_time.naive_local();

                // Check if event overlaps with this time slot
                event_start < slot_end && event_end > slot_start
            })
            .collect();

        // Render each event as a small block
        for (i, event) in events_in_slot.iter().enumerate() {
            if i >= area.height as usize {
                break; // Don't overflow the area
            }

            let event_area = Rect::new(area.x, area.y + i as u16, area.width, 1);
            let event_text = if event.title.len() > area.width as usize {
                format!(
                    "{}...",
                    &event.title[..area.width.saturating_sub(3) as usize]
                )
            } else {
                event.title.clone()
            };

            let priority_color = match event.priority {
                EventPriority::High => Color::Red,
                EventPriority::Normal => Color::Blue,
                EventPriority::Low => Color::Gray,
            };

            let event_para = Paragraph::new(event_text).style(
                Style::default()
                    .fg(priority_color)
                    .add_modifier(Modifier::BOLD),
            );

            frame.render_widget(event_para, event_area);
        }
    }

    /// Render day schedule view
    fn render_day_schedule(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let is_focused = self.focused_pane == CalendarPane::Calendar;

        // Create hourly schedule from 6 AM to 10 PM
        let hours = (6..=22).collect::<Vec<_>>();
        let hour_height = area.height / hours.len() as u16;

        let day_events: Vec<&Event> = self
            .events
            .iter()
            .filter(|event| {
                self.enabled_calendars.contains(&event.calendar_id)
                    && event.start_time.date_naive() == self.selected_date
            })
            .collect();

        let schedule_block = Block::default()
            .borders(Borders::ALL)
            .title(format!(
                "Schedule - {}",
                self.selected_date.format("%A, %B %d")
            ))
            .border_style(theme.get_component_style("border", is_focused));

        frame.render_widget(schedule_block, area);

        let inner_area = area.inner(&Margin::new(1, 1));

        // Render hourly grid
        for (i, hour) in hours.iter().enumerate() {
            let y = inner_area.y + (i as u16) * hour_height;
            let _hour_area = Rect::new(inner_area.x, y, inner_area.width, hour_height);

            // Time label
            let time_label_area = Rect::new(inner_area.x, y, 6, 1);
            let time_text = format!("{:02}:00", hour);
            let time_para = Paragraph::new(time_text)
                .style(theme.get_component_style("calendar_time_label", false));
            frame.render_widget(time_para, time_label_area);

            // Event area
            let event_area = Rect::new(inner_area.x + 7, y, inner_area.width - 7, hour_height);

            // Find events for this hour
            let hour_events: Vec<&Event> = day_events
                .iter()
                .filter(|event| {
                    let event_hour = event.start_time.hour() as i32;
                    event_hour == *hour
                })
                .cloned()
                .collect();

            // Render events in this hour
            for (j, event) in hour_events.iter().enumerate() {
                if j >= hour_height as usize {
                    break;
                }

                let event_y = y + j as u16;
                let event_rect = Rect::new(event_area.x, event_y, event_area.width, 1);

                let time_str = if event.all_day {
                    "All Day".to_string()
                } else {
                    format!(
                        "{}-{}",
                        event.start_time.format("%H:%M"),
                        event.end_time.format("%H:%M")
                    )
                };

                let event_text = format!("{} {}", time_str, event.title);
                let priority_color = match event.priority {
                    EventPriority::High => Color::Red,
                    EventPriority::Normal => Color::Blue,
                    EventPriority::Low => Color::Gray,
                };

                let event_para =
                    Paragraph::new(event_text).style(Style::default().fg(priority_color));

                frame.render_widget(event_para, event_rect);
            }

            // Draw hour separator
            if i < hours.len() - 1 {
                let separator_area =
                    Rect::new(inner_area.x, y + hour_height - 1, inner_area.width, 1);
                let separator = Paragraph::new("─".repeat(inner_area.width as usize))
                    .style(theme.get_component_style("calendar_grid", false));
                frame.render_widget(separator, separator_area);
            }
        }
    }

    /// Render day summary sidebar
    fn render_day_summary(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let day_events: Vec<&Event> = self
            .events
            .iter()
            .filter(|event| {
                self.enabled_calendars.contains(&event.calendar_id)
                    && event.start_time.date_naive() == self.selected_date
            })
            .collect();

        let summary_text = format!(
            "Events: {}\nAll Day: {}\nConfirmed: {}\nTentative: {}",
            day_events.len(),
            day_events.iter().filter(|e| e.all_day).count(),
            day_events
                .iter()
                .filter(|e| e.status == EventStatus::Confirmed)
                .count(),
            day_events
                .iter()
                .filter(|e| e.status == EventStatus::Tentative)
                .count(),
        );

        let summary = Paragraph::new(summary_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Day Summary")
                    .border_style(theme.get_component_style("border", false)),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(summary, area);
    }

    /// Render upcoming events in agenda view
    fn render_upcoming_events(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let is_focused = self.focused_pane == CalendarPane::EventList;

        // Get upcoming events (next 30 days)
        let now = Utc::now();
        let thirty_days_from_now = now + Duration::days(30);

        let upcoming_events: Vec<&Event> = self
            .events
            .iter()
            .filter(|event| {
                self.enabled_calendars.contains(&event.calendar_id)
                    && event.start_time >= now
                    && event.start_time <= thirty_days_from_now
            })
            .collect();

        // Create list items
        let list_items: Vec<ListItem> = upcoming_events
            .iter()
            .map(|event| {
                let date_str = event.start_time.format("%m/%d");
                let time_str = if event.all_day {
                    "All Day".to_string()
                } else {
                    event.start_time.format("%H:%M").to_string()
                };

                let spans = vec![
                    Span::styled(format!("{} ", date_str), Style::default().fg(Color::Cyan)),
                    Span::styled(format!("{} ", time_str), Style::default().fg(Color::Yellow)),
                    Span::styled(event.title.clone(), Style::default().fg(Color::White)),
                ];

                ListItem::new(Line::from(spans))
            })
            .collect();

        let title = format!("Upcoming Events ({})", upcoming_events.len());
        let events_list = List::new(list_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(theme.get_component_style("border", is_focused)),
            )
            .highlight_style(theme.get_component_style("list_selected", is_focused))
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(events_list, area, &mut self.event_list_state);
    }

    /// Render calendar filters in agenda view
    fn render_calendar_filters(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let is_focused = self.focused_pane == CalendarPane::CalendarList;

        // Create list items for calendars
        let list_items: Vec<ListItem> = self
            .calendars
            .iter()
            .map(|calendar| {
                let enabled = self.enabled_calendars.contains(&calendar.id);
                let checkbox = if enabled { "☑" } else { "☐" };
                let color = calendar.color.as_deref().unwrap_or("#3174ad");

                let spans = vec![
                    Span::styled(
                        format!("{} ", checkbox),
                        Style::default().fg(if enabled { Color::Green } else { Color::Gray }),
                    ),
                    Span::styled("● ", Style::default().fg(self.parse_color(color))),
                    Span::styled(calendar.name.clone(), Style::default().fg(Color::White)),
                    Span::styled(
                        format!(" ({})", calendar.source.provider_name()),
                        Style::default().fg(Color::Gray),
                    ),
                ];

                ListItem::new(Line::from(spans))
            })
            .collect();

        let calendars_list = List::new(list_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Calendars")
                    .border_style(theme.get_component_style("border", is_focused)),
            )
            .highlight_style(theme.get_component_style("list_selected", is_focused))
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(calendars_list, area, &mut self.calendar_list_state);
    }

    /// Parse color string to ratatui Color
    fn parse_color(&self, color_str: &str) -> Color {
        if color_str.starts_with('#') && color_str.len() == 7 {
            if let Ok(rgb) = u32::from_str_radix(&color_str[1..], 16) {
                let r = ((rgb >> 16) & 0xFF) as u8;
                let g = ((rgb >> 8) & 0xFF) as u8;
                let b = (rgb & 0xFF) as u8;
                return Color::Rgb(r, g, b);
            }
        }
        Color::Blue // Default color
    }

    /// Render status line with calendar information
    fn render_status_line(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let status_text = format!(
            "View: {} | Date: {} | Events: {} | Calendars: {} enabled",
            self.current_view.name(),
            self.selected_date.format("%Y-%m-%d"),
            self.events.len(),
            self.enabled_calendars.len(),
        );

        let status =
            Paragraph::new(status_text).style(theme.get_component_style("status_bar", false));

        frame.render_widget(status, area);
    }

    /// Render event details overlay
    fn render_event_details_overlay(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if let Some(ref event) = self.selected_event {
            // Calculate popup area (centered, 60% width, 70% height)
            let popup_area = self.centered_rect(60, 70, area);

            // Clear background
            frame.render_widget(Clear, popup_area);

            // Create event details text
            let details_text = self.format_event_details(event);

            let details_para = Paragraph::new(details_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Event Details")
                        .border_style(theme.get_component_style("border", true)),
                )
                .wrap(Wrap { trim: true });

            frame.render_widget(details_para, popup_area);
        }
    }

    /// Format event details for display
    fn format_event_details(&self, event: &Event) -> Text {
        let mut lines = Vec::new();

        lines.push(Line::from(vec![
            Span::styled("Title: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(event.title.clone()),
        ]));

        if let Some(ref description) = event.description {
            lines.push(Line::from(vec![
                Span::styled(
                    "Description: ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(description.clone()),
            ]));
        }

        if let Some(ref location) = event.location {
            lines.push(Line::from(vec![
                Span::styled("Location: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(location.clone()),
            ]));
        }

        let time_text = if event.all_day {
            "All Day".to_string()
        } else {
            format!(
                "{} - {}",
                event.start_time.format("%Y-%m-%d %H:%M"),
                event.end_time.format("%Y-%m-%d %H:%M")
            )
        };

        lines.push(Line::from(vec![
            Span::styled("Time: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(time_text),
        ]));

        lines.push(Line::from(vec![
            Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!("{:?}", event.status)),
        ]));

        lines.push(Line::from(vec![
            Span::styled("Priority: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!("{:?}", event.priority)),
        ]));

        if !event.attendees.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("Attendees: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{}", event.attendees.len())),
            ]));
        }

        Text::from(lines)
    }

    /// Render calendar list overlay
    fn render_calendar_list_overlay(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Calculate popup area (centered, 50% width, 60% height)
        let popup_area = self.centered_rect(50, 60, area);

        // Clear background
        frame.render_widget(Clear, popup_area);

        // Render calendar list
        self.render_calendar_filters(frame, popup_area, theme);
    }

    /// Render delete confirmation dialog
    fn render_delete_confirmation_dialog(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Calculate popup area (centered, smaller size for confirmation)
        let popup_area = self.centered_rect(40, 20, area);

        // Clear background
        frame.render_widget(Clear, popup_area);

        // Get event title for confirmation message
        let event_title = if let Some(event_id) = &self.event_to_delete {
            self.get_event_by_id(event_id)
                .map(|e| e.title.as_str())
                .unwrap_or("this event")
        } else {
            "this event"
        };

        // Create confirmation text
        let confirmation_text = format!("Delete \"{}\"?", event_title);

        // Create button states
        let cancel_style = if self.delete_confirmation_selected == 0 {
            Style::default()
                .fg(theme.colors.palette.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.colors.palette.text_muted)
        };

        let delete_style = if self.delete_confirmation_selected == 1 {
            Style::default()
                .fg(theme.colors.palette.error)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.colors.palette.text_muted)
        };

        // Create dialog content
        let dialog_content = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                confirmation_text,
                Style::default().fg(theme.colors.palette.text_primary),
            )]),
            Line::from(""),
            Line::from(""),
            Line::from(vec![
                Span::styled("  [", Style::default().fg(theme.colors.palette.border)),
                Span::styled("Cancel", cancel_style),
                Span::styled("]", Style::default().fg(theme.colors.palette.border)),
                Span::styled("    [", Style::default().fg(theme.colors.palette.border)),
                Span::styled("Delete", delete_style),
                Span::styled("]", Style::default().fg(theme.colors.palette.border)),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "← → Tab: Navigate  Enter: Confirm  Esc: Cancel",
                Style::default()
                    .fg(theme.colors.palette.text_muted)
                    .add_modifier(Modifier::ITALIC),
            )]),
        ];

        let dialog_paragraph = Paragraph::new(dialog_content)
            .block(
                Block::default()
                    .title("Confirm Deletion")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.colors.palette.error)),
            )
            .style(Style::default().bg(theme.colors.palette.background))
            .wrap(Wrap { trim: true });

        frame.render_widget(dialog_paragraph, popup_area);
    }

    /// Calculate centered rectangle
    fn centered_rect(&self, percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }

    /// Get the start of the current week (Monday)
    fn get_week_start(&self) -> DateTime<Local> {
        let days_since_monday = self.current_date.weekday().num_days_from_monday();
        self.current_date - Duration::days(days_since_monday as i64)
    }

    // Public methods for external control

    /// Set events to display
    pub fn set_events(&mut self, events: Vec<Event>) {
        self.events = events;
    }

    /// Get current events
    pub fn get_events(&self) -> &Vec<Event> {
        &self.events
    }

    /// Set available calendars
    pub fn set_calendars(&mut self, calendars: Vec<crate::calendar::Calendar>) {
        self.calendars = calendars;
    }

    /// Handle keyboard input
    pub async fn handle_key(&mut self, key: crossterm::event::KeyCode) -> Option<CalendarAction> {
        use crossterm::event::KeyCode;

        // Handle delete confirmation dialog first
        if self.show_delete_confirmation {
            return self.handle_delete_confirmation_key(key);
        }

        match key {
            // Global navigation
            KeyCode::Tab => {
                self.focused_pane = match self.focused_pane {
                    CalendarPane::Calendar => CalendarPane::EventList,
                    CalendarPane::EventList => CalendarPane::CalendarList,
                    CalendarPane::CalendarList => CalendarPane::Calendar,
                    CalendarPane::EventDetails => CalendarPane::Calendar,
                };
                None
            }

            // View switching
            KeyCode::Char('1') => {
                self.current_view = CalendarViewMode::Month;
                self.view_tab_index = 0;
                Some(CalendarAction::ChangeView(CalendarViewMode::Month))
            }
            KeyCode::Char('2') => {
                self.current_view = CalendarViewMode::Week;
                self.view_tab_index = 1;
                Some(CalendarAction::ChangeView(CalendarViewMode::Week))
            }
            KeyCode::Char('3') => {
                self.current_view = CalendarViewMode::Day;
                self.view_tab_index = 2;
                Some(CalendarAction::ChangeView(CalendarViewMode::Day))
            }
            KeyCode::Char('4') => {
                self.current_view = CalendarViewMode::Agenda;
                self.view_tab_index = 3;
                Some(CalendarAction::ChangeView(CalendarViewMode::Agenda))
            }

            // Navigation
            KeyCode::Left | KeyCode::Char('h') => match self.focused_pane {
                CalendarPane::Calendar => {
                    self.navigate_period(-1);
                    Some(CalendarAction::PreviousPeriod)
                }
                _ => None,
            },
            KeyCode::Right | KeyCode::Char('l') => match self.focused_pane {
                CalendarPane::Calendar => {
                    self.navigate_period(1);
                    Some(CalendarAction::NextPeriod)
                }
                _ => None,
            },
            KeyCode::Up | KeyCode::Char('k') => match self.focused_pane {
                CalendarPane::EventList => {
                    self.event_list_previous();
                    None
                }
                CalendarPane::CalendarList => {
                    self.calendar_list_previous();
                    None
                }
                _ => None,
            },
            KeyCode::Down | KeyCode::Char('j') => match self.focused_pane {
                CalendarPane::EventList => {
                    self.event_list_next();
                    None
                }
                CalendarPane::CalendarList => {
                    self.calendar_list_next();
                    None
                }
                _ => None,
            },

            // Actions
            KeyCode::Char(' ') => match self.focused_pane {
                CalendarPane::Calendar => Some(CalendarAction::Today),
                CalendarPane::CalendarList => {
                    self.toggle_selected_calendar();
                    None
                }
                _ => None,
            },
            KeyCode::Enter => match self.focused_pane {
                CalendarPane::EventList => {
                    if let Some(selected_event_id) = self.get_selected_event_id() {
                        if let Some(selected_event) = self.get_event_by_id(&selected_event_id) {
                            self.show_event_details(selected_event.clone());
                            Some(CalendarAction::ShowEventDetails(selected_event_id))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            },
            KeyCode::Char('c') => Some(CalendarAction::CreateEvent),
            KeyCode::Char('e') => {
                if let Some(selected_event_id) = self.get_selected_event_id() {
                    Some(CalendarAction::EditEvent(selected_event_id))
                } else {
                    None
                }
            }
            KeyCode::Char('d') => {
                if let Some(selected_event_id) = self.get_selected_event_id() {
                    // Show confirmation dialog instead of immediately deleting
                    self.show_delete_confirmation = true;
                    self.event_to_delete = Some(selected_event_id);
                    self.delete_confirmation_selected = 0; // Default to "Cancel"
                    None
                } else {
                    None
                }
            }
            KeyCode::Char('x') => {
                if let Some(selected_event_id) = self.get_selected_event_id() {
                    Some(CalendarAction::CancelEvent(selected_event_id))
                } else {
                    None
                }
            }
            KeyCode::Char('r') => Some(CalendarAction::Refresh),
            KeyCode::Char('/') => {
                // TODO: Implement search mode
                None
            }
            KeyCode::Esc => {
                if self.show_event_details {
                    self.hide_event_details();
                } else if self.show_calendar_list {
                    self.hide_calendar_list();
                }
                None
            }

            _ => None,
        }
    }

    /// Navigate calendar period
    fn navigate_period(&mut self, direction: i32) {
        match self.current_view {
            CalendarViewMode::Month => {
                if direction > 0 {
                    self.current_date = self.current_date + Duration::days(30);
                } else {
                    self.current_date = self.current_date - Duration::days(30);
                }
            }
            CalendarViewMode::Week => {
                if direction > 0 {
                    self.current_date = self.current_date + Duration::weeks(1);
                } else {
                    self.current_date = self.current_date - Duration::weeks(1);
                }
            }
            CalendarViewMode::Day => {
                if direction > 0 {
                    self.current_date = self.current_date + Duration::days(1);
                } else {
                    self.current_date = self.current_date - Duration::days(1);
                }
                self.selected_date = self.current_date.date_naive();
            }
            CalendarViewMode::Agenda => {
                // Agenda view doesn't have period navigation
            }
        }
    }

    /// Navigate to today
    pub fn navigate_to_today(&mut self) {
        self.current_date = Local::now();
        self.selected_date = Local::now().date_naive();
    }

    /// Handle key input for delete confirmation dialog
    fn handle_delete_confirmation_key(
        &mut self,
        key: crossterm::event::KeyCode,
    ) -> Option<CalendarAction> {
        use crossterm::event::KeyCode;

        match key {
            KeyCode::Left | KeyCode::Char('h') => {
                self.delete_confirmation_selected = 0; // Cancel
                None
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.delete_confirmation_selected = 1; // Delete
                None
            }
            KeyCode::Tab => {
                self.delete_confirmation_selected = 1 - self.delete_confirmation_selected; // Toggle
                None
            }
            KeyCode::Enter => {
                if self.delete_confirmation_selected == 1 {
                    // User confirmed deletion
                    if let Some(event_id) = self.event_to_delete.take() {
                        self.show_delete_confirmation = false;
                        self.delete_confirmation_selected = 0;
                        return Some(CalendarAction::DeleteEvent(event_id));
                    }
                }
                // User cancelled or no event to delete
                self.show_delete_confirmation = false;
                self.event_to_delete = None;
                self.delete_confirmation_selected = 0;
                None
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                // Cancel deletion
                self.show_delete_confirmation = false;
                self.event_to_delete = None;
                self.delete_confirmation_selected = 0;
                None
            }
            _ => None,
        }
    }

    /// Event list navigation
    fn event_list_next(&mut self) {
        let i = match self.event_list_state.selected() {
            Some(i) => {
                let event_count = self.get_current_events_count();
                if i >= event_count.saturating_sub(1) {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.event_list_state.select(Some(i));
    }

    fn event_list_previous(&mut self) {
        let i = match self.event_list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.get_current_events_count().saturating_sub(1)
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.event_list_state.select(Some(i));
    }

    fn get_current_events_count(&self) -> usize {
        match self.current_view {
            CalendarViewMode::Month | CalendarViewMode::Week | CalendarViewMode::Day => self
                .events
                .iter()
                .filter(|event| {
                    self.enabled_calendars.contains(&event.calendar_id)
                        && event.start_time.date_naive() == self.selected_date
                })
                .count(),
            CalendarViewMode::Agenda => {
                let now = Utc::now();
                let thirty_days_from_now = now + Duration::days(30);

                self.events
                    .iter()
                    .filter(|event| {
                        self.enabled_calendars.contains(&event.calendar_id)
                            && event.start_time >= now
                            && event.start_time <= thirty_days_from_now
                    })
                    .count()
            }
        }
    }

    /// Calendar list navigation
    fn calendar_list_next(&mut self) {
        let i = match self.calendar_list_state.selected() {
            Some(i) => {
                if i >= self.calendars.len().saturating_sub(1) {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.calendar_list_state.select(Some(i));
    }

    fn calendar_list_previous(&mut self) {
        let i = match self.calendar_list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.calendars.len().saturating_sub(1)
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.calendar_list_state.select(Some(i));
    }

    /// Toggle selected calendar visibility
    fn toggle_selected_calendar(&mut self) {
        if let Some(selected) = self.calendar_list_state.selected() {
            if let Some(calendar) = self.calendars.get(selected) {
                let calendar_id = calendar.id.clone();
                if self.enabled_calendars.contains(&calendar_id) {
                    self.enabled_calendars.remove(&calendar_id);
                } else {
                    self.enabled_calendars.insert(calendar_id);
                }
            }
        }
    }

    /// Get currently selected event ID
    fn get_selected_event_id(&self) -> Option<String> {
        if let Some(selected) = self.event_list_state.selected() {
            match self.current_view {
                CalendarViewMode::Month | CalendarViewMode::Week | CalendarViewMode::Day => {
                    let selected_events: Vec<&Event> = self
                        .events
                        .iter()
                        .filter(|event| {
                            self.enabled_calendars.contains(&event.calendar_id)
                                && event.start_time.date_naive() == self.selected_date
                        })
                        .collect();
                    selected_events.get(selected).map(|e| e.id.clone())
                }
                CalendarViewMode::Agenda => {
                    let now = Utc::now();
                    let thirty_days_from_now = now + Duration::days(30);

                    let upcoming_events: Vec<&Event> = self
                        .events
                        .iter()
                        .filter(|event| {
                            self.enabled_calendars.contains(&event.calendar_id)
                                && event.start_time >= now
                                && event.start_time <= thirty_days_from_now
                        })
                        .collect();
                    upcoming_events.get(selected).map(|e| e.id.clone())
                }
            }
        } else {
            None
        }
    }

    /// Get event by ID
    fn get_event_by_id(&self, event_id: &str) -> Option<&Event> {
        self.events.iter().find(|e| e.id == event_id)
    }

    /// Show event details overlay
    fn show_event_details(&mut self, event: Event) {
        self.selected_event = Some(event);
        self.show_event_details = true;
        self.focused_pane = CalendarPane::EventDetails;
    }

    /// Hide event details overlay
    fn hide_event_details(&mut self) {
        self.show_event_details = false;
        self.selected_event = None;
        self.focused_pane = CalendarPane::Calendar;
    }

    /// Show calendar list overlay
    pub fn show_calendar_list(&mut self) {
        self.show_calendar_list = true;
        self.focused_pane = CalendarPane::CalendarList;
    }

    /// Hide calendar list overlay
    fn hide_calendar_list(&mut self) {
        self.show_calendar_list = false;
        self.focused_pane = CalendarPane::Calendar;
    }

    /// Get current view mode
    pub fn current_view(&self) -> CalendarViewMode {
        self.current_view
    }

    /// Set current view mode
    pub fn set_view_mode(&mut self, mode: CalendarViewMode) {
        self.current_view = mode;
        self.view_tab_index = match mode {
            CalendarViewMode::Month => 0,
            CalendarViewMode::Week => 1,
            CalendarViewMode::Day => 2,
            CalendarViewMode::Agenda => 3,
        };
    }

    /// Get selected date
    pub fn selected_date(&self) -> NaiveDate {
        self.selected_date
    }

    /// Set selected date
    pub fn set_selected_date(&mut self, date: NaiveDate) {
        self.selected_date = date;
        self.current_date = date
            .and_hms_opt(12, 0, 0)
            .unwrap()
            .and_local_timezone(Local)
            .unwrap();
    }

    /// Get enabled calendars
    pub fn enabled_calendars(&self) -> &std::collections::HashSet<String> {
        &self.enabled_calendars
    }

    /// Set calendar enabled state
    pub fn set_calendar_enabled(&mut self, calendar_id: String, enabled: bool) {
        if enabled {
            self.enabled_calendars.insert(calendar_id);
        } else {
            self.enabled_calendars.remove(&calendar_id);
        }
    }

    /// Check if in focus
    pub fn is_focused(&self) -> bool {
        self.is_focused
    }

    /// Set focus state
    pub fn set_focus(&mut self, focused: bool) {
        self.is_focused = focused;
    }
}

impl Default for CalendarUI {
    fn default() -> Self {
        Self::new()
    }
}
