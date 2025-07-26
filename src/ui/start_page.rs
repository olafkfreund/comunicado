use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, List, ListItem, Paragraph,
    },
    Frame,
};
use chrono::{DateTime, Local};

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
            widget_count: 6, // weather, datetime, system, tasks, calendar, quick actions
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

    /// Render the start page dashboard
    pub fn render(&mut self, f: &mut Frame, area: Rect, theme: &Theme) {
        // Create main layout
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),  // Header with greeting
                Constraint::Min(0),     // Dashboard content
            ])
            .split(area);

        // Render header
        self.render_header(f, main_chunks[0], theme);

        // Create main dashboard layout - focus on large time display
        let main_layout_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(45),  // Large central clock area
                Constraint::Percentage(55),  // Bottom widgets area
            ])
            .split(main_chunks[1]);

        // Top section: large clock with small weather info
        let top_section = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),  // Weather (compact)
                Constraint::Percentage(50),  // Large clock (central focus)
                Constraint::Percentage(25),  // System stats (compact)
            ])
            .split(main_layout_chunks[0]);

        // Bottom section: remaining widgets
        let bottom_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(40),  // Tasks
                Constraint::Percentage(35),  // Calendar  
                Constraint::Percentage(25),  // Quick actions
            ])
            .split(main_layout_chunks[1]);

        // Render all widgets with new layout
        self.render_weather_compact(f, top_section[0], theme, self.selected_widget == 0);
        self.render_datetime_large(f, top_section[1], theme, self.selected_widget == 1);
        self.render_system_stats_compact(f, top_section[2], theme, self.selected_widget == 2);
        self.render_tasks(f, bottom_chunks[0], theme, self.selected_widget == 3);
        self.render_calendar(f, bottom_chunks[1], theme, self.selected_widget == 4);
        self.render_quick_actions(f, bottom_chunks[2], theme, self.selected_widget == 5);
    }

    fn render_header(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let header_text = vec![
            Line::from(vec![
                Span::styled(
                    "COMUNICADO",
                    Style::default()
                        .fg(theme.colors.palette.text_primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " | TUI Email Client",
                    Style::default()
                        .fg(theme.colors.palette.text_secondary),
                ),
            ])
        ];

        let header = Paragraph::new(header_text)
            .alignment(Alignment::Center);

        f.render_widget(header, area);
    }

    fn render_weather_compact(&self, f: &mut Frame, area: Rect, theme: &Theme, _is_selected: bool) {
        if let Some(ref weather) = self.weather {
            let weather_text = vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled(
                        "WEATHER",
                        Style::default()
                            .fg(theme.colors.palette.text_secondary)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled(
                        format!("{}°", weather.temperature),
                        Style::default()
                            .fg(theme.colors.palette.text_primary)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::styled(
                        weather.condition.to_uppercase(),
                        Style::default()
                            .fg(theme.colors.palette.text_secondary),
                    ),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled(
                        weather.location.to_uppercase(),
                        Style::default()
                            .fg(theme.colors.palette.text_secondary),
                    ),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled(
                        format!("H {}% | W {:.0}km/h", weather.humidity, weather.wind_speed),
                        Style::default()
                            .fg(theme.colors.palette.text_secondary),
                    ),
                ]),
            ];

            let weather_widget = Paragraph::new(weather_text)
                .alignment(Alignment::Center);

            f.render_widget(weather_widget, area);
        } else {
            let loading_text = vec![
                Line::from(""),
                Line::from("WEATHER"),
                Line::from(""),
                Line::from("Loading..."),
            ];

            let weather_widget = Paragraph::new(loading_text)
                .style(Style::default().fg(theme.colors.palette.text_secondary))
                .alignment(Alignment::Center);

            f.render_widget(weather_widget, area);
        }
    }

    fn render_tasks(&self, f: &mut Frame, area: Rect, theme: &Theme, is_selected: bool) {
        let block = create_border_block("TASKS", theme, is_selected);

        if self.tasks.is_empty() {
            let empty_text = vec![
                Line::from(""),
                Line::from("NO TASKS TODAY"),
                Line::from(""),
                Line::from("Press 't' to add a task"),
            ];

            let tasks_widget = Paragraph::new(empty_text)
                .block(block)
                .style(Style::default().fg(theme.colors.palette.text_secondary))
                .alignment(Alignment::Center);

            f.render_widget(tasks_widget, area);
        } else {
            let task_items: Vec<ListItem> = self.tasks
                .iter()
                .take(8) // Limit to fit in widget
                .map(|task| {
                    let checkbox = if task.completed { "■" } else { "□" };
                    let style = if task.completed {
                        Style::default()
                            .fg(theme.colors.palette.text_secondary)
                            .add_modifier(Modifier::CROSSED_OUT)
                    } else {
                        Style::default().fg(theme.colors.palette.text_primary)
                    };

                    let content = vec![
                        Line::from(vec![
                            Span::styled(checkbox, Style::default().fg(theme.colors.palette.text_secondary)),
                            Span::raw(" "),
                            Span::styled(task.priority.symbol(), Style::default().fg(task.priority.color(theme))),
                            Span::raw(" "),
                            Span::styled(&task.title, style),
                        ])
                    ];

                    ListItem::new(content)
                })
                .collect();

            let tasks_list = List::new(task_items)
                .block(block);

            f.render_widget(tasks_list, area);
        }
    }

    fn render_datetime_large(&self, f: &mut Frame, area: Rect, theme: &Theme, _is_selected: bool) {
        let now = Local::now();
        
        // Create very large time display (peaclock inspired)
        let time_str = now.format("%H:%M").to_string();
        let date_str = now.format("%A, %B %d").to_string();
        let year_str = now.format("%Y").to_string();
        
        // Split area for massive time, date, and year
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),   // Spacing
                Constraint::Length(8),   // Massive time (8 lines high)
                Constraint::Length(3),   // Date  
                Constraint::Length(2),   // Year
                Constraint::Min(0),      // Bottom spacing
            ])
            .split(area);

        // Create ASCII-art style large time (simplified block letters)
        let time_lines = self.create_large_time_display(&time_str, theme);
        let time_widget = Paragraph::new(time_lines)
            .alignment(Alignment::Center);
        f.render_widget(time_widget, chunks[1]);

        // Date display
        let date_text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    date_str.to_uppercase(),
                    Style::default()
                        .fg(theme.colors.palette.text_primary)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
        ];

        let date_widget = Paragraph::new(date_text)
            .alignment(Alignment::Center);
        f.render_widget(date_widget, chunks[2]);

        // Year
        let year_text = vec![
            Line::from(vec![
                Span::styled(
                    year_str,
                    Style::default()
                        .fg(theme.colors.palette.text_secondary),
                ),
            ]),
        ];

        let year_widget = Paragraph::new(year_text)
            .alignment(Alignment::Center);
        f.render_widget(year_widget, chunks[3]);
    }

    fn create_large_time_display(&self, time_str: &str, theme: &Theme) -> Vec<Line<'static>> {
        // Create a massive, centered time display
        let border_char = "■";
        let padding_spaces = "  ";
        let time_str_owned = time_str.to_string();
        
        vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    format!("{}{}{}{}{}",
                        border_char.repeat(10),
                        padding_spaces,
                        time_str_owned,
                        padding_spaces,
                        border_char.repeat(10)
                    ),
                    Style::default()
                        .fg(theme.colors.palette.accent)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    time_str_owned,
                    Style::default()
                        .fg(theme.colors.palette.text_primary)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
        ]
    }

    fn render_calendar(&self, f: &mut Frame, area: Rect, theme: &Theme, is_selected: bool) {
        let block = create_border_block("CALENDAR", theme, is_selected);

        if self.calendar_events.is_empty() {
            let empty_text = vec![
                Line::from(""),
                Line::from("NO UPCOMING EVENTS"),
                Line::from(""),
                Line::from("Schedule is clear"),
            ];

            let calendar_widget = Paragraph::new(empty_text)
                .block(block)
                .style(Style::default().fg(theme.colors.palette.text_secondary))
                .alignment(Alignment::Center);

            f.render_widget(calendar_widget, area);
        } else {
            let event_items: Vec<ListItem> = self.calendar_events
                .iter()
                .take(6) // Limit to fit in widget
                .map(|event| {
                    let time_str = event.start_time.format("%H:%M").to_string();
                    let title_upper = event.title.to_uppercase();
                    let content = vec![
                        Line::from(vec![
                            Span::styled(
                                format!("{}", time_str),
                                Style::default()
                                    .fg(theme.colors.palette.accent)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::raw(" "),
                            Span::styled(
                                title_upper,
                                Style::default().fg(theme.colors.palette.text_primary),
                            ),
                        ])
                    ];

                    ListItem::new(content)
                })
                .collect();

            let calendar_list = List::new(event_items)
                .block(block);

            f.render_widget(calendar_list, area);
        }
    }

    fn render_quick_actions(&self, f: &mut Frame, area: Rect, theme: &Theme, is_selected: bool) {
        let block = create_border_block("ACTIONS", theme, is_selected);

        let action_items: Vec<ListItem> = self.quick_actions
            .iter()
            .map(|action| {
                let shortcut_text = if let Some(ref shortcut) = action.shortcut {
                    format!(" [{}]", shortcut)
                } else {
                    String::new()
                };
                let title_upper = action.title.to_uppercase();

                let content = vec![
                    Line::from(vec![
                        Span::styled(
                            "▶",
                            Style::default().fg(theme.colors.palette.accent),
                        ),
                        Span::raw(" "),
                        Span::styled(
                            title_upper,
                            Style::default().fg(theme.colors.palette.text_primary),
                        ),
                        Span::styled(
                            shortcut_text,
                            Style::default().fg(theme.colors.palette.text_secondary),
                        ),
                    ])
                ];

                ListItem::new(content)
            })
            .collect();

        let actions_list = List::new(action_items)
            .block(block);

        f.render_widget(actions_list, area);
    }

    fn render_system_stats_compact(&self, f: &mut Frame, area: Rect, theme: &Theme, _is_selected: bool) {
        if let Some(ref stats) = self.system_stats {
            let stats_text = vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled(
                        "SYSTEM",
                        Style::default()
                            .fg(theme.colors.palette.text_secondary)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled(
                        format!("CPU {:.0}%", stats.cpu_usage),
                        Style::default()
                            .fg(theme.colors.palette.text_primary),
                    ),
                ]),
                Line::from(vec![
                    Span::styled(
                        format!("MEM {:.0}%", stats.memory_usage),
                        Style::default()
                            .fg(theme.colors.palette.text_primary),
                    ),
                ]),
                Line::from(vec![
                    Span::styled(
                        format!("DSK {:.0}%", stats.disk_usage),
                        Style::default()
                            .fg(theme.colors.palette.text_primary),
                    ),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled(
                        format!("UP {}D {}H", 
                            stats.uptime.num_days(),
                            stats.uptime.num_hours() % 24
                        ),
                        Style::default()
                            .fg(theme.colors.palette.text_secondary),
                    ),
                ]),
            ];

            let stats_widget = Paragraph::new(stats_text)
                .alignment(Alignment::Center);

            f.render_widget(stats_widget, area);
        } else {
            let loading_text = vec![
                Line::from(""),
                Line::from("SYSTEM"),
                Line::from(""),
                Line::from("Loading..."),
            ];

            let stats_widget = Paragraph::new(loading_text)
                .style(Style::default().fg(theme.colors.palette.text_secondary))
                .alignment(Alignment::Center);

            f.render_widget(stats_widget, area);
        }
    }

}

impl Default for StartPage {
    fn default() -> Self {
        Self::new()
    }
}