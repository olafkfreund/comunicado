//! Calendar Component Module
//!
//! Implements a modular calendar component with multi-view support and event management.

use super::{
    ComponentError, ComponentId, ComponentMetrics, ComponentResult, ComponentState, EventResult,
    RenderContext, UIComponent, UIEvent,
};
use crate::calendar::{Calendar, CalendarManager, Event};
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs},
};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration as StdDuration, Instant};

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

/// Calendar panes for focus management
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalendarPane {
    Calendar,
    EventList,
    CalendarList,
    EventDetails,
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

/// Calendar component that manages calendar views and event operations
pub struct CalendarComponent {
    // Component metadata
    id: ComponentId,
    state: ComponentState,
    metrics: ComponentMetrics,

    // View state
    current_view: CalendarViewMode,
    current_date: DateTime<Local>,
    selected_date: NaiveDate,
    focused_pane: CalendarPane,

    // Data and services
    calendar_manager: Option<Arc<CalendarManager>>,
    events: Vec<Event>,
    calendars: Vec<Calendar>,
    enabled_calendars: HashSet<String>,

    // UI state
    view_tab_index: usize,
    #[allow(dead_code)]
    event_list_state: ListState,
    #[allow(dead_code)]
    calendar_list_state: ListState,
    show_event_details: bool,
    show_calendar_list: bool,
    show_delete_confirmation: bool,

    // Event management
    selected_event: Option<Event>,
    selected_event_id: Option<String>,
    event_to_delete: Option<String>,
    delete_confirmation_selected: usize,

    // Performance tracking
    #[allow(dead_code)]
    last_render_time: Instant,
    render_count: u64,
}

impl CalendarComponent {
    /// Create a new calendar component
    pub fn new() -> Self {
        let mut enabled_calendars = HashSet::new();
        enabled_calendars.insert("local".to_string());

        Self {
            id: ComponentId::new::<Self>(),
            state: ComponentState::Uninitialized,
            metrics: ComponentMetrics::default(),
            current_view: CalendarViewMode::Month,
            current_date: Local::now(),
            selected_date: Local::now().date_naive(),
            focused_pane: CalendarPane::Calendar,
            calendar_manager: None,
            events: Vec::new(),
            calendars: Vec::new(),
            enabled_calendars,
            view_tab_index: 0,
            event_list_state: ListState::default(),
            calendar_list_state: ListState::default(),
            show_event_details: false,
            show_calendar_list: false,
            show_delete_confirmation: false,
            selected_event: None,
            selected_event_id: None,
            event_to_delete: None,
            delete_confirmation_selected: 0,
            last_render_time: Instant::now(),
            render_count: 0,
        }
    }

    /// Initialize with calendar manager
    pub fn with_calendar_manager(mut self, calendar_manager: Arc<CalendarManager>) -> Self {
        self.calendar_manager = Some(calendar_manager);
        self
    }

    /// Get the current view mode
    pub fn current_view(&self) -> CalendarViewMode {
        self.current_view
    }

    /// Set the view mode
    pub fn set_view_mode(&mut self, mode: CalendarViewMode) -> ComponentResult<()> {
        self.current_view = mode;
        self.view_tab_index = CalendarViewMode::all()
            .iter()
            .position(|&m| m == mode)
            .unwrap_or(0);
        Ok(())
    }

    /// Get the current date
    pub fn current_date(&self) -> DateTime<Local> {
        self.current_date
    }

    /// Set the current date
    pub fn set_current_date(&mut self, date: DateTime<Local>) {
        self.current_date = date;
        self.selected_date = date.date_naive();
    }

    /// Get selected event
    pub fn selected_event(&self) -> Option<&Event> {
        self.selected_event.as_ref()
    }

    /// Set events
    pub fn set_events(&mut self, events: Vec<Event>) {
        self.events = events;
    }

    /// Set calendars
    pub fn set_calendars(&mut self, calendars: Vec<Calendar>) {
        self.calendars = calendars;
    }

    /// Toggle calendar visibility
    pub fn toggle_calendar(&mut self, calendar_id: &str) {
        if self.enabled_calendars.contains(calendar_id) {
            self.enabled_calendars.remove(calendar_id);
        } else {
            self.enabled_calendars.insert(calendar_id.to_string());
        }
    }

    /// Handle calendar actions
    fn handle_calendar_action(&mut self, action: CalendarAction) -> ComponentResult<EventResult> {
        match action {
            CalendarAction::NextPeriod => {
                self.navigate_next_period();
                Ok(EventResult::Handled)
            }
            CalendarAction::PreviousPeriod => {
                self.navigate_previous_period();
                Ok(EventResult::Handled)
            }
            CalendarAction::Today => {
                self.set_current_date(Local::now());
                Ok(EventResult::Handled)
            }
            CalendarAction::ChangeView(mode) => {
                self.set_view_mode(mode)?;
                Ok(EventResult::Handled)
            }
            CalendarAction::SelectEvent(event_id) => {
                self.selected_event_id = Some(event_id.clone());
                // Find and set the selected event
                self.selected_event = self.events.iter().find(|e| e.id == event_id).cloned();
                Ok(EventResult::Handled)
            }
            CalendarAction::ShowEventDetails(event_id) => {
                self.selected_event_id = Some(event_id.clone());
                self.selected_event = self.events.iter().find(|e| e.id == event_id).cloned();
                self.show_event_details = true;
                Ok(EventResult::Handled)
            }
            CalendarAction::CreateEvent => {
                // TODO: Open event creation dialog
                Ok(EventResult::Handled)
            }
            CalendarAction::EditEvent(_event_id) => {
                // TODO: Open event editing dialog
                Ok(EventResult::Handled)
            }
            CalendarAction::DeleteEvent(event_id) => {
                self.event_to_delete = Some(event_id);
                self.show_delete_confirmation = true;
                self.delete_confirmation_selected = 0;
                Ok(EventResult::Handled)
            }
            CalendarAction::ToggleCalendar(calendar_id) => {
                self.toggle_calendar(&calendar_id);
                Ok(EventResult::Handled)
            }
            CalendarAction::Refresh => {
                // TODO: Refresh events from calendar manager
                Ok(EventResult::Handled)
            }
            _ => Ok(EventResult::Ignored),
        }
    }

    /// Navigate to next period based on current view
    fn navigate_next_period(&mut self) {
        match self.current_view {
            CalendarViewMode::Month => {
                self.current_date = self
                    .current_date
                    .checked_add_months(chrono::Months::new(1))
                    .unwrap_or(self.current_date);
            }
            CalendarViewMode::Week => {
                self.current_date = self.current_date + Duration::weeks(1);
            }
            CalendarViewMode::Day => {
                self.current_date = self.current_date + Duration::days(1);
            }
            CalendarViewMode::Agenda => {
                self.current_date = self.current_date + Duration::weeks(1);
            }
        }
        self.selected_date = self.current_date.date_naive();
    }

    /// Navigate to previous period based on current view
    fn navigate_previous_period(&mut self) {
        match self.current_view {
            CalendarViewMode::Month => {
                self.current_date = self
                    .current_date
                    .checked_sub_months(chrono::Months::new(1))
                    .unwrap_or(self.current_date);
            }
            CalendarViewMode::Week => {
                self.current_date = self.current_date - Duration::weeks(1);
            }
            CalendarViewMode::Day => {
                self.current_date = self.current_date - Duration::days(1);
            }
            CalendarViewMode::Agenda => {
                self.current_date = self.current_date - Duration::weeks(1);
            }
        }
        self.selected_date = self.current_date.date_naive();
    }

    /// Render the calendar header with tabs and navigation
    fn render_header(&self, context: &mut RenderContext<'_>, area: Rect) -> ComponentResult<()> {
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
            .highlight_style(
                Style::default()
                    .fg(context.theme.colors.palette.accent)
                    .add_modifier(Modifier::BOLD),
            )
            .select(self.view_tab_index);

        context.frame.render_widget(tabs, chunks[0]);

        // Render date navigation
        let date_text = match self.current_view {
            CalendarViewMode::Month => self.current_date.format("%B %Y").to_string(),
            CalendarViewMode::Week => {
                let week_start = self.current_date
                    - Duration::days(self.current_date.weekday().num_days_from_monday() as i64);
                let week_end = week_start + Duration::days(6);
                format!(
                    "{} - {}",
                    week_start.format("%b %d"),
                    week_end.format("%b %d, %Y")
                )
            }
            CalendarViewMode::Day => self.current_date.format("%A, %B %d, %Y").to_string(),
            CalendarViewMode::Agenda => "Agenda View".to_string(),
        };

        let date_widget = Paragraph::new(date_text)
            .block(Block::default().borders(Borders::ALL).title("Date"))
            .alignment(Alignment::Center)
            .style(Style::default().fg(context.theme.colors.palette.text_primary));

        context.frame.render_widget(date_widget, chunks[1]);

        // Render controls
        let controls = Paragraph::new("←→: Navigate\nT: Today\n1-4: Views")
            .block(Block::default().borders(Borders::ALL).title("Controls"))
            .style(Style::default().fg(context.theme.colors.palette.text_muted));

        context.frame.render_widget(controls, chunks[2]);

        Ok(())
    }

    /// Render month view
    fn render_month_view(
        &self,
        context: &mut RenderContext<'_>,
        area: Rect,
    ) -> ComponentResult<()> {
        let month_block = Block::default()
            .title("Month View")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(
                if self.focused_pane == CalendarPane::Calendar && context.is_focused {
                    context.theme.colors.palette.accent
                } else {
                    context.theme.colors.palette.border
                },
            ));

        // Create a simple month grid placeholder
        let mut lines = Vec::new();
        lines.push(Line::from("Su  Mo  Tu  We  Th  Fr  Sa"));

        // Get first day of month and calculate grid
        let first_day = self.current_date.with_day(1).ok_or_else(|| {
            ComponentError::RenderFailed("Invalid date: cannot get first day of month".to_string())
        })?;
        let days_in_month = first_day.with_day(32).unwrap_or(first_day).day() - 1;
        let start_weekday = first_day.weekday().num_days_from_sunday() as usize;

        let mut current_line = " ".repeat(start_weekday * 4);
        let mut day_count = 0;

        for day in 1..=days_in_month {
            if day_count > 0 && day_count % 7 == 0 {
                lines.push(Line::from(current_line.clone()));
                current_line.clear();
            }

            let is_selected = day == self.selected_date.day();
            let day_str = if is_selected {
                format!("[{:2}]", day)
            } else {
                format!(" {:2} ", day)
            };

            current_line.push_str(&day_str);
            day_count = (start_weekday + day as usize - 1) % 7 + 1;
        }

        if !current_line.trim().is_empty() {
            lines.push(Line::from(current_line));
        }

        let content = Paragraph::new(lines)
            .block(month_block)
            .alignment(Alignment::Left);

        context.frame.render_widget(content, area);
        Ok(())
    }

    /// Render week view
    fn render_week_view(&self, context: &mut RenderContext<'_>, area: Rect) -> ComponentResult<()> {
        let week_block = Block::default()
            .title("Week View")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(
                if self.focused_pane == CalendarPane::Calendar && context.is_focused {
                    context.theme.colors.palette.accent
                } else {
                    context.theme.colors.palette.border
                },
            ));

        let content = Paragraph::new("Week view placeholder - showing 7 days with hourly slots")
            .block(week_block)
            .alignment(Alignment::Center);

        context.frame.render_widget(content, area);
        Ok(())
    }

    /// Render day view
    fn render_day_view(&self, context: &mut RenderContext<'_>, area: Rect) -> ComponentResult<()> {
        let day_block = Block::default()
            .title("Day View")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(
                if self.focused_pane == CalendarPane::Calendar && context.is_focused {
                    context.theme.colors.palette.accent
                } else {
                    context.theme.colors.palette.border
                },
            ));

        let content =
            Paragraph::new("Day view placeholder - showing hourly schedule for selected day")
                .block(day_block)
                .alignment(Alignment::Center);

        context.frame.render_widget(content, area);
        Ok(())
    }

    /// Render agenda view
    fn render_agenda_view(
        &self,
        context: &mut RenderContext<'_>,
        area: Rect,
    ) -> ComponentResult<()> {
        let agenda_block = Block::default()
            .title("Agenda View")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(
                if self.focused_pane == CalendarPane::Calendar && context.is_focused {
                    context.theme.colors.palette.accent
                } else {
                    context.theme.colors.palette.border
                },
            ));

        // Create event list for agenda view
        let event_items: Vec<ListItem> = self
            .events
            .iter()
            .filter(|event| {
                // Show events from current date onward
                event.start_time.date_naive() >= self.selected_date
            })
            .take(10) // Limit to first 10 events
            .map(|event| {
                let date_str = event.start_time.format("%m/%d").to_string();
                let time_str = event.start_time.format("%H:%M").to_string();
                let content = format!("{} {} - {}", date_str, time_str, event.title);

                ListItem::new(content)
                    .style(Style::default().fg(context.theme.colors.palette.text_primary))
            })
            .collect();

        let event_list = List::new(event_items).block(agenda_block).highlight_style(
            Style::default()
                .fg(context.theme.colors.palette.accent)
                .add_modifier(Modifier::BOLD),
        );

        context.frame.render_widget(event_list, area);
        Ok(())
    }

    /// Render status line
    fn render_status_line(
        &self,
        context: &mut RenderContext<'_>,
        area: Rect,
    ) -> ComponentResult<()> {
        let status_text = format!(
            "Events: {} | Calendars: {} | View: {} | Focus: {:?}",
            self.events.len(),
            self.calendars.len(),
            self.current_view.name(),
            self.focused_pane
        );

        let status = Paragraph::new(status_text)
            .style(Style::default().fg(context.theme.colors.palette.text_muted))
            .alignment(Alignment::Left);

        context.frame.render_widget(status, area);
        Ok(())
    }

    /// Handle key events
    fn handle_key_event(&mut self, key: KeyEvent) -> ComponentResult<EventResult> {
        match key.code {
            KeyCode::Char('1') => {
                self.handle_calendar_action(CalendarAction::ChangeView(CalendarViewMode::Month))
            }
            KeyCode::Char('2') => {
                self.handle_calendar_action(CalendarAction::ChangeView(CalendarViewMode::Week))
            }
            KeyCode::Char('3') => {
                self.handle_calendar_action(CalendarAction::ChangeView(CalendarViewMode::Day))
            }
            KeyCode::Char('4') => {
                self.handle_calendar_action(CalendarAction::ChangeView(CalendarViewMode::Agenda))
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                self.handle_calendar_action(CalendarAction::Today)
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.handle_calendar_action(CalendarAction::PreviousPeriod)
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.handle_calendar_action(CalendarAction::NextPeriod)
            }
            KeyCode::Char('n') => self.handle_calendar_action(CalendarAction::CreateEvent),
            KeyCode::Char('r') => self.handle_calendar_action(CalendarAction::Refresh),
            KeyCode::Tab => {
                // Cycle through panes
                self.focused_pane = match self.focused_pane {
                    CalendarPane::Calendar => CalendarPane::EventList,
                    CalendarPane::EventList => CalendarPane::CalendarList,
                    CalendarPane::CalendarList => CalendarPane::EventDetails,
                    CalendarPane::EventDetails => CalendarPane::Calendar,
                };
                Ok(EventResult::Handled)
            }
            KeyCode::Esc => {
                if self.show_event_details {
                    self.show_event_details = false;
                    Ok(EventResult::Handled)
                } else if self.show_calendar_list {
                    self.show_calendar_list = false;
                    Ok(EventResult::Handled)
                } else if self.show_delete_confirmation {
                    self.show_delete_confirmation = false;
                    Ok(EventResult::Handled)
                } else {
                    Ok(EventResult::Ignored)
                }
            }
            _ => Ok(EventResult::Ignored),
        }
    }
}

impl UIComponent for CalendarComponent {
    fn component_id(&self) -> ComponentId {
        self.id
    }

    fn component_name(&self) -> &str {
        "CalendarComponent"
    }

    fn state(&self) -> ComponentState {
        self.state
    }

    fn initialize(&mut self) -> ComponentResult<()> {
        self.state = ComponentState::Ready;
        Ok(())
    }

    fn render(&mut self, context: &mut RenderContext<'_>) -> ComponentResult<()> {
        let start_time = Instant::now();

        // Create main layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(10),   // Main content
                Constraint::Length(1), // Status line
            ])
            .split(context.area);

        // Render header
        self.render_header(context, chunks[0])?;

        // Render main content based on current view
        match self.current_view {
            CalendarViewMode::Month => self.render_month_view(context, chunks[1])?,
            CalendarViewMode::Week => self.render_week_view(context, chunks[1])?,
            CalendarViewMode::Day => self.render_day_view(context, chunks[1])?,
            CalendarViewMode::Agenda => self.render_agenda_view(context, chunks[1])?,
        }

        // Render status line
        self.render_status_line(context, chunks[2])?;

        // Update render metrics
        let render_time = start_time.elapsed();
        self.metrics.last_render_time = render_time;
        self.metrics.render_calls += 1;
        self.render_count += 1;

        // Update average render time
        let weight = 0.1;
        self.metrics.avg_render_time = StdDuration::from_nanos(
            (self.metrics.avg_render_time.as_nanos() as f64 * (1.0 - weight)
                + render_time.as_nanos() as f64 * weight) as u64,
        );

        self.metrics.last_updated = Instant::now();

        Ok(())
    }

    fn handle_event(&mut self, event: &UIEvent) -> ComponentResult<EventResult> {
        match event {
            UIEvent::Key(key) => {
                self.metrics.events_processed += 1;
                self.handle_key_event(*key)
            }
            UIEvent::FocusGained => Ok(EventResult::Handled),
            UIEvent::FocusLost => Ok(EventResult::Handled),
            _ => Ok(EventResult::Ignored),
        }
    }

    fn metrics(&self) -> &ComponentMetrics {
        &self.metrics
    }

    fn set_state(&mut self, new_state: ComponentState) -> ComponentResult<()> {
        self.state = new_state;
        Ok(())
    }

    fn can_focus(&self) -> bool {
        matches!(self.state, ComponentState::Ready | ComponentState::Focused)
    }

    fn cleanup(&mut self) -> ComponentResult<()> {
        self.state = ComponentState::Destroying;
        Ok(())
    }
}

impl Default for CalendarComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for CalendarComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CalendarComponent")
            .field("id", &self.id)
            .field("state", &self.state)
            .field("view_mode", &self.current_view)
            .field("current_date", &self.current_date)
            .field("selected_date", &self.selected_date)
            .field("focused_pane", &self.focused_pane)
            .field("events_count", &self.events.len())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calendar_component_creation() {
        let component = CalendarComponent::new();
        assert_eq!(component.state(), ComponentState::Uninitialized);
        assert_eq!(component.current_view(), CalendarViewMode::Month);
        assert_eq!(component.component_name(), "CalendarComponent");
    }

    #[test]
    fn test_calendar_component_initialization() {
        let mut component = CalendarComponent::new();
        component.initialize().unwrap();
        assert_eq!(component.state(), ComponentState::Ready);
    }

    #[test]
    fn test_view_mode_switching() {
        let mut component = CalendarComponent::new();
        component.initialize().unwrap();

        // Switch to week view
        component.set_view_mode(CalendarViewMode::Week).unwrap();
        assert_eq!(component.current_view(), CalendarViewMode::Week);

        // Switch to day view
        component.set_view_mode(CalendarViewMode::Day).unwrap();
        assert_eq!(component.current_view(), CalendarViewMode::Day);

        // Switch to agenda view
        component.set_view_mode(CalendarViewMode::Agenda).unwrap();
        assert_eq!(component.current_view(), CalendarViewMode::Agenda);
    }

    #[test]
    fn test_navigation() {
        let mut component = CalendarComponent::new();
        component.initialize().unwrap();

        let initial_date = component.current_date();

        // Navigate to next month
        component.navigate_next_period();
        assert_ne!(component.current_date(), initial_date);

        // Navigate back
        component.navigate_previous_period();
        // Should be back to original date (approximately, accounting for month boundaries)
        assert!((component.current_date() - initial_date).num_days().abs() < 32);
    }

    #[test]
    fn test_calendar_toggle() {
        let mut component = CalendarComponent::new();
        component.initialize().unwrap();

        // "local" calendar should be enabled by default
        assert!(component.enabled_calendars.contains("local"));

        // Toggle it off
        component.toggle_calendar("local");
        assert!(!component.enabled_calendars.contains("local"));

        // Toggle it back on
        component.toggle_calendar("local");
        assert!(component.enabled_calendars.contains("local"));
    }
}
