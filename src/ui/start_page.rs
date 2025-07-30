use chrono::{DateTime, Local};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::theme::Theme;
// Widget helper functions
fn create_border_block<'a>(title: &'a str, theme: &Theme, is_selected: bool) -> Block<'a> {
    let border_style = if is_selected {
        Style::default().fg(theme.colors.palette.accent)
    } else {
        Style::default().fg(theme.colors.palette.border)
    };

    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style)
}

/// Weather information
#[derive(Debug, Clone)]
pub struct WeatherInfo {
    pub location: String,
    pub temperature: i32,
    pub condition: String,
    pub humidity: u32,
    pub wind_speed: f32,
    pub forecast: Vec<WeatherForecast>,
}

#[derive(Debug, Clone)]
pub struct WeatherForecast {
    pub day: String,
    pub high: i32,
    pub low: i32,
    pub condition: String,
}

/// Task/Todo item
#[derive(Debug, Clone)]
pub struct TaskItem {
    pub id: String,
    pub title: String,
    pub completed: bool,
    pub priority: TaskPriority,
    pub due_date: Option<DateTime<Local>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskPriority {
    Low,
    Medium,
    High,
    Urgent,
}

impl TaskPriority {
    pub fn color(&self, theme: &Theme) -> Color {
        match self {
            TaskPriority::Low => theme.colors.palette.text_secondary,
            TaskPriority::Medium => theme.colors.palette.text_primary,
            TaskPriority::High => theme.colors.palette.accent,
            TaskPriority::Urgent => Color::White,
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            TaskPriority::Low => "○",
            TaskPriority::Medium => "◦",
            TaskPriority::High => "●",
            TaskPriority::Urgent => "█",
        }
    }
}

/// System statistics
#[derive(Debug, Clone)]
pub struct SystemStats {
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub disk_usage: f32,
    pub network_rx: f32,
    pub network_tx: f32,
    pub uptime: chrono::Duration,
}

/// Calendar event
#[derive(Debug, Clone)]
pub struct CalendarEvent {
    pub id: String,
    pub title: String,
    pub start_time: DateTime<Local>,
    pub end_time: DateTime<Local>,
    pub location: Option<String>,
    pub attendees: Vec<String>,
}

/// Quick action item
#[derive(Debug, Clone)]
pub struct QuickAction {
    pub id: String,
    pub title: String,
    pub description: String,
    pub icon: String,
    pub shortcut: Option<String>,
}

/// Start page dashboard state
pub struct StartPage {
    weather: Option<WeatherInfo>,
    tasks: Vec<TaskItem>,
    system_stats: Option<SystemStats>,
    calendar_events: Vec<CalendarEvent>,
    quick_actions: Vec<QuickAction>,
    selected_widget: usize,
    widget_count: usize,
}

impl StartPage {
    pub fn new() -> Self {
        let quick_actions = vec![
            QuickAction {
                id: "compose".to_string(),
                title: "Compose Email".to_string(),
                description: "Write a new email".to_string(),
                icon: "COMPOSE".to_string(),
                shortcut: Some("c".to_string()),
            },
            QuickAction {
                id: "calendar".to_string(),
                title: "Open Calendar".to_string(),
                description: "View your schedule".to_string(),
                icon: "CALENDAR".to_string(),
                shortcut: Some("C".to_string()),
            },
            QuickAction {
                id: "contacts".to_string(),
                title: "Address Book".to_string(),
                description: "Manage contacts".to_string(),
                icon: "CONTACTS".to_string(),
                shortcut: Some("a".to_string()),
            },
            QuickAction {
                id: "search".to_string(),
                title: "Search Emails".to_string(),
                description: "Find messages".to_string(),
                icon: "SEARCH".to_string(),
                shortcut: Some("/".to_string()),
            },
        ];

        Self {
            weather: None,
            tasks: Vec::new(),
            system_stats: None,
            calendar_events: Vec::new(),
            quick_actions,
            selected_widget: 0,
            widget_count: 2, // datetime, shortcuts
        }
    }

    /// Set weather information
    pub fn set_weather(&mut self, weather: WeatherInfo) {
        self.weather = Some(weather);
    }

    /// Set task list
    pub fn set_tasks(&mut self, tasks: Vec<TaskItem>) {
        self.tasks = tasks;
    }

    /// Set system statistics
    pub fn set_system_stats(&mut self, stats: SystemStats) {
        self.system_stats = Some(stats);
    }

    /// Set calendar events
    pub fn set_calendar_events(&mut self, events: Vec<CalendarEvent>) {
        self.calendar_events = events;
    }

    /// Add a new task
    pub fn add_task(&mut self, task: TaskItem) {
        self.tasks.push(task);
    }

    /// Toggle task completion
    pub fn toggle_task(&mut self, task_id: &str) {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == task_id) {
            task.completed = !task.completed;
        }
    }

    /// Remove completed tasks
    pub fn clear_completed_tasks(&mut self) {
        self.tasks.retain(|task| !task.completed);
    }

    /// Navigation methods
    pub fn next_widget(&mut self) {
        self.selected_widget = (self.selected_widget + 1) % self.widget_count;
    }

    pub fn previous_widget(&mut self) {
        self.selected_widget = if self.selected_widget == 0 {
            self.widget_count - 1
        } else {
            self.selected_widget - 1
        };
    }

    pub fn get_selected_widget(&self) -> usize {
        self.selected_widget
    }

    /// Get quick action by ID
    pub fn get_quick_action(&self, action_id: &str) -> Option<&QuickAction> {
        self.quick_actions.iter().find(|a| a.id == action_id)
    }

    /// Render the start page dashboard - simplified single clock layout
    pub fn render(&mut self, f: &mut Frame, area: Rect, theme: &Theme) {
        // Create simple layout focused on clock and shortcuts
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(15), // Large datetime display
                Constraint::Min(6),     // Shortcuts
            ])
            .split(area);

        // Render single large clock and shortcuts only
        self.render_datetime_block(f, main_chunks[0], theme, self.selected_widget == 0);
        self.render_shortcuts_block(f, main_chunks[1], theme, self.selected_widget == 1);
    }


    fn render_datetime_block(&self, f: &mut Frame, area: Rect, theme: &Theme, is_selected: bool) {
        let block = create_border_block("datetime", theme, is_selected);
        let inner_area = block.inner(area);

        let now = Local::now();

        // Large time display like the reference image
        let time_str = now.format("%H:%M:%S").to_string();
        let date_str = now.format("%A, %B %d, %Y").to_string();
        let am_pm = now.format("%p").to_string().to_lowercase();

        // Create layout for time + date + am/pm
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Spacing
                Constraint::Length(6), // Much larger time section
                Constraint::Length(2), // Date
                Constraint::Min(0),    // Bottom spacing
            ])
            .split(inner_area);

        // Render HUGE time display with much bigger text by using multiple lines
        let time_lines = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                format!("█████ {} █████", time_str),
                Style::default()
                    .fg(theme.colors.palette.text_primary)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(
                format!("█████ {} █████", am_pm.to_uppercase()),
                Style::default()
                    .fg(theme.colors.palette.accent)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
        ];

        let time_widget = Paragraph::new(time_lines).alignment(Alignment::Center);
        f.render_widget(time_widget, chunks[1]);

        // Date display - also make it bigger and bold
        let date_lines = vec![Line::from(vec![Span::styled(
            format!("▓▓▓ {} ▓▓▓", date_str.to_uppercase()),
            Style::default()
                .fg(theme.colors.palette.text_secondary)
                .add_modifier(Modifier::BOLD),
        )])];

        let date_widget = Paragraph::new(date_lines).alignment(Alignment::Center);
        f.render_widget(date_widget, chunks[2]);

        f.render_widget(block, area);
    }

    fn render_shortcuts_block(&self, f: &mut Frame, area: Rect, theme: &Theme, is_selected: bool) {
        let block = create_border_block("shortcuts", theme, is_selected);
        let inner_area = block.inner(area);

        // Create a 4-column layout for shortcuts
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ])
            .split(inner_area);

        // Column 1: Global Navigation
        let col1_lines = vec![
            Line::from(vec![Span::styled(
                "GLOBAL",
                Style::default()
                    .fg(theme.colors.palette.accent)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("q", Style::default().fg(theme.colors.palette.accent)),
                Span::styled(
                    " quit",
                    Style::default().fg(theme.colors.palette.text_primary),
                ),
            ]),
            Line::from(vec![
                Span::styled("Tab", Style::default().fg(theme.colors.palette.accent)),
                Span::styled(
                    " next pane",
                    Style::default().fg(theme.colors.palette.text_primary),
                ),
            ]),
            Line::from(vec![
                Span::styled("~", Style::default().fg(theme.colors.palette.accent)),
                Span::styled(
                    " start page",
                    Style::default().fg(theme.colors.palette.text_primary),
                ),
            ]),
            Line::from(vec![
                Span::styled("Esc", Style::default().fg(theme.colors.palette.accent)),
                Span::styled(
                    " cancel",
                    Style::default().fg(theme.colors.palette.text_primary),
                ),
            ]),
        ];

        // Column 2: Email Management
        let col2_lines = vec![
            Line::from(vec![Span::styled(
                "EMAIL",
                Style::default()
                    .fg(theme.colors.palette.accent)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("c", Style::default().fg(theme.colors.palette.accent)),
                Span::styled(
                    " compose",
                    Style::default().fg(theme.colors.palette.text_primary),
                ),
            ]),
            Line::from(vec![
                Span::styled("p", Style::default().fg(theme.colors.palette.accent)),
                Span::styled(
                    " reply/forward",
                    Style::default().fg(theme.colors.palette.text_primary),
                ),
            ]),
            Line::from(vec![
                Span::styled("/", Style::default().fg(theme.colors.palette.accent)),
                Span::styled(
                    " search",
                    Style::default().fg(theme.colors.palette.text_primary),
                ),
            ]),
            Line::from(vec![
                Span::styled("Enter", Style::default().fg(theme.colors.palette.accent)),
                Span::styled(
                    " open",
                    Style::default().fg(theme.colors.palette.text_primary),
                ),
            ]),
        ];

        // Column 3: Navigation & Lists
        let col3_lines = vec![
            Line::from(vec![Span::styled(
                "NAVIGATE",
                Style::default()
                    .fg(theme.colors.palette.accent)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("hjkl", Style::default().fg(theme.colors.palette.accent)),
                Span::styled(
                    " vim move",
                    Style::default().fg(theme.colors.palette.text_primary),
                ),
            ]),
            Line::from(vec![
                Span::styled("↑↓", Style::default().fg(theme.colors.palette.accent)),
                Span::styled(
                    " up/down",
                    Style::default().fg(theme.colors.palette.text_primary),
                ),
            ]),
            Line::from(vec![
                Span::styled("t", Style::default().fg(theme.colors.palette.accent)),
                Span::styled(
                    " thread view",
                    Style::default().fg(theme.colors.palette.text_primary),
                ),
            ]),
            Line::from(vec![
                Span::styled("s", Style::default().fg(theme.colors.palette.accent)),
                Span::styled(
                    " sort date",
                    Style::default().fg(theme.colors.palette.text_primary),
                ),
            ]),
        ];

        // Column 4: Calendar & System
        let col4_lines = vec![
            Line::from(vec![Span::styled(
                "CALENDAR",
                Style::default()
                    .fg(theme.colors.palette.accent)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("F3", Style::default().fg(theme.colors.palette.accent)),
                Span::styled(
                    " calendar",
                    Style::default().fg(theme.colors.palette.text_primary),
                ),
            ]),
            Line::from(vec![
                Span::styled("Ctrl+A", Style::default().fg(theme.colors.palette.accent)),
                Span::styled(
                    " add account",
                    Style::default().fg(theme.colors.palette.text_primary),
                ),
            ]),
            Line::from(vec![
                Span::styled("Ctrl+R", Style::default().fg(theme.colors.palette.accent)),
                Span::styled(
                    " refresh",
                    Style::default().fg(theme.colors.palette.text_primary),
                ),
            ]),
            Line::from(vec![
                Span::styled("m", Style::default().fg(theme.colors.palette.accent)),
                Span::styled(
                    " view mode",
                    Style::default().fg(theme.colors.palette.text_primary),
                ),
            ]),
        ];

        // Render each column
        let col1_widget = Paragraph::new(col1_lines).alignment(Alignment::Left);
        let col2_widget = Paragraph::new(col2_lines).alignment(Alignment::Left);
        let col3_widget = Paragraph::new(col3_lines).alignment(Alignment::Left);
        let col4_widget = Paragraph::new(col4_lines).alignment(Alignment::Left);

        f.render_widget(col1_widget, cols[0]);
        f.render_widget(col2_widget, cols[1]);
        f.render_widget(col3_widget, cols[2]);
        f.render_widget(col4_widget, cols[3]);

        f.render_widget(block, area);
    }

}

impl Default for StartPage {
    fn default() -> Self {
        Self::new()
    }
}
