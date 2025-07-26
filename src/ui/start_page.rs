use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Gauge, List, ListItem, Paragraph, Wrap,
    },
    Frame,
};
use chrono::{DateTime, Local, Timelike};

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
    pub fn color(&self) -> Color {
        match self {
            TaskPriority::Low => Color::Green,
            TaskPriority::Medium => Color::Yellow,
            TaskPriority::High => Color::LightRed,
            TaskPriority::Urgent => Color::Red,
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            TaskPriority::Low => "●",
            TaskPriority::Medium => "◆",
            TaskPriority::High => "▲",
            TaskPriority::Urgent => "⚠",
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
                icon: "✉".to_string(),
                shortcut: Some("c".to_string()),
            },
            QuickAction {
                id: "calendar".to_string(),
                title: "Open Calendar".to_string(),
                description: "View your schedule".to_string(),
                icon: "📅".to_string(),
                shortcut: Some("C".to_string()),
            },
            QuickAction {
                id: "contacts".to_string(),
                title: "Address Book".to_string(),
                description: "Manage contacts".to_string(),
                icon: "👥".to_string(),
                shortcut: Some("a".to_string()),
            },
            QuickAction {
                id: "search".to_string(),
                title: "Search Emails".to_string(),
                description: "Find messages".to_string(),
                icon: "🔍".to_string(),
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
            widget_count: 6, // weather, tasks, system, calendar, quick actions, time
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

        // Create dashboard grid layout
        let dashboard_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),  // Left column
                Constraint::Percentage(34),  // Middle column  
                Constraint::Percentage(33),  // Right column
            ])
            .split(main_chunks[1]);

        // Left column layout
        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(40),  // Weather
                Constraint::Percentage(60),  // Tasks
            ])
            .split(dashboard_chunks[0]);

        // Middle column layout
        let middle_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(50),  // Date/Time (larger for peaclock style)
                Constraint::Percentage(30),  // Calendar
                Constraint::Percentage(20),  // Quick Actions
            ])
            .split(dashboard_chunks[1]);

        // Right column layout
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(50),  // System Stats
                Constraint::Percentage(50),  // Additional info
            ])
            .split(dashboard_chunks[2]);

        // Render all widgets
        self.render_weather(f, left_chunks[0], theme, self.selected_widget == 0);
        self.render_tasks(f, left_chunks[1], theme, self.selected_widget == 1);
        self.render_datetime(f, middle_chunks[0], theme, self.selected_widget == 2);
        self.render_calendar(f, middle_chunks[1], theme, self.selected_widget == 3);
        self.render_quick_actions(f, middle_chunks[2], theme, self.selected_widget == 4);
        self.render_system_stats(f, right_chunks[0], theme, self.selected_widget == 5);
        self.render_additional_info(f, right_chunks[1], theme, false);
    }

    fn render_header(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let now = Local::now();
        let greeting = match now.hour() {
            5..=11 => "Good Morning",
            12..=17 => "Good Afternoon", 
            18..=21 => "Good Evening",
            _ => "Good Night",
        };

        let header_text = vec![
            Line::from(vec![
                Span::styled(
                    format!("{} ", greeting),
                    Style::default()
                        .fg(theme.colors.palette.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "| Welcome to Comunicado",
                    Style::default()
                        .fg(theme.colors.palette.text_secondary)
                        .add_modifier(Modifier::ITALIC),
                ),
            ])
        ];

        let header = Paragraph::new(header_text)
            .block(create_border_block("Dashboard", theme, false))
            .alignment(Alignment::Center);

        f.render_widget(header, area);
    }

    fn render_weather(&self, f: &mut Frame, area: Rect, theme: &Theme, _is_selected: bool) {
        if let Some(ref weather) = self.weather {
            // Split area for location, temperature, and details
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(2),   // Location
                    Constraint::Length(3),   // Large temperature
                    Constraint::Min(0),      // Details
                ])
                .split(area);

            // Location
            let location_text = vec![
                Line::from(vec![
                    Span::styled("📍 ", Style::default().fg(Color::Blue)),
                    Span::styled(
                        &weather.location,
                        Style::default()
                            .fg(theme.colors.palette.text_primary)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
            ];
            let location_widget = Paragraph::new(location_text)
                .alignment(Alignment::Center);
            f.render_widget(location_widget, chunks[0]);

            // Large temperature display
            let temp_text = vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled(
                        format!("{}°C", weather.temperature),
                        Style::default()
                            .fg(theme.colors.palette.accent)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!(" {}", weather.condition),
                        Style::default()
                            .fg(theme.colors.palette.text_primary),
                    ),
                ]),
            ];
            let temp_widget = Paragraph::new(temp_text)
                .alignment(Alignment::Center);
            f.render_widget(temp_widget, chunks[1]);

            // Weather details
            let details_text = vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("💧 ", Style::default().fg(Color::Blue)),
                    Span::raw(format!("{}%", weather.humidity)),
                ]),
                Line::from(vec![
                    Span::styled("💨 ", Style::default().fg(Color::Cyan)),
                    Span::raw(format!("{:.1} km/h", weather.wind_speed)),
                ]),
            ];
            let details_widget = Paragraph::new(details_text)
                .alignment(Alignment::Center);
            f.render_widget(details_widget, chunks[2]);
        } else {
            let loading_text = vec![
                Line::from(""),
                Line::from("🌤️ Loading weather..."),
                Line::from(""),
            ];

            let weather_widget = Paragraph::new(loading_text)
                .alignment(Alignment::Center);

            f.render_widget(weather_widget, area);
        }
    }

    fn render_tasks(&self, f: &mut Frame, area: Rect, theme: &Theme, is_selected: bool) {
        let block = create_border_block("Tasks", theme, is_selected);

        if self.tasks.is_empty() {
            let empty_text = vec![
                Line::from("No tasks today! 🎉"),
                Line::from(""),
                Line::from("Press 't' to add a task"),
            ];

            let tasks_widget = Paragraph::new(empty_text)
                .block(block)
                .alignment(Alignment::Center);

            f.render_widget(tasks_widget, area);
        } else {
            let task_items: Vec<ListItem> = self.tasks
                .iter()
                .take(8) // Limit to fit in widget
                .map(|task| {
                    let checkbox = if task.completed { "☑" } else { "☐" };
                    let style = if task.completed {
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::CROSSED_OUT)
                    } else {
                        Style::default().fg(theme.colors.palette.text_primary)
                    };

                    let content = vec![
                        Line::from(vec![
                            Span::raw(format!("{} ", checkbox)),
                            Span::styled(task.priority.symbol(), Style::default().fg(task.priority.color())),
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

    fn render_datetime(&self, f: &mut Frame, area: Rect, theme: &Theme, _is_selected: bool) {
        let now = Local::now();
        
        // Create peaclock-style large time display
        let time_str = now.format("%H:%M").to_string();
        let date_str = now.format("%A, %B %d, %Y").to_string();
        
        // Split area for large time and date
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(60),  // Large time
                Constraint::Percentage(25),  // Date
                Constraint::Percentage(15),  // Timezone
            ])
            .split(area);

        // Large time display (peaclock style)
        let time_text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    time_str,
                    Style::default()
                        .fg(theme.colors.palette.accent)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
        ];

        let time_widget = Paragraph::new(time_text)
            .alignment(Alignment::Center);
        f.render_widget(time_widget, chunks[0]);

        // Date display
        let date_text = vec![
            Line::from(vec![
                Span::styled(
                    date_str,
                    Style::default()
                        .fg(theme.colors.palette.text_primary)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
        ];

        let date_widget = Paragraph::new(date_text)
            .alignment(Alignment::Center);
        f.render_widget(date_widget, chunks[1]);

        // Timezone
        let tz_text = vec![
            Line::from(vec![
                Span::styled(
                    now.format("%Z").to_string(),
                    Style::default()
                        .fg(theme.colors.palette.text_secondary),
                ),
            ]),
        ];

        let tz_widget = Paragraph::new(tz_text)
            .alignment(Alignment::Center);
        f.render_widget(tz_widget, chunks[2]);
    }

    fn render_calendar(&self, f: &mut Frame, area: Rect, theme: &Theme, is_selected: bool) {
        let block = create_border_block("Upcoming Events", theme, is_selected);

        if self.calendar_events.is_empty() {
            let empty_text = vec![
                Line::from("No upcoming events"),
                Line::from(""),
                Line::from("📅 Your schedule is clear"),
            ];

            let calendar_widget = Paragraph::new(empty_text)
                .block(block)
                .alignment(Alignment::Center);

            f.render_widget(calendar_widget, area);
        } else {
            let event_items: Vec<ListItem> = self.calendar_events
                .iter()
                .take(5) // Limit to fit in widget
                .map(|event| {
                    let time_str = event.start_time.format("%H:%M").to_string();
                    let content = vec![
                        Line::from(vec![
                            Span::styled(
                                format!("{} ", time_str),
                                Style::default().fg(theme.colors.palette.accent),
                            ),
                            Span::raw(&event.title),
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
        let block = create_border_block("Quick Actions", theme, is_selected);

        let action_items: Vec<ListItem> = self.quick_actions
            .iter()
            .map(|action| {
                let shortcut_text = if let Some(ref shortcut) = action.shortcut {
                    format!(" ({})", shortcut)
                } else {
                    String::new()
                };

                let content = vec![
                    Line::from(vec![
                        Span::raw(&action.icon),
                        Span::raw(" "),
                        Span::styled(&action.title, Style::default().fg(theme.colors.palette.text_primary)),
                        Span::styled(shortcut_text, Style::default().fg(Color::DarkGray)),
                    ])
                ];

                ListItem::new(content)
            })
            .collect();

        let actions_list = List::new(action_items)
            .block(block);

        f.render_widget(actions_list, area);
    }

    fn render_system_stats(&self, f: &mut Frame, area: Rect, theme: &Theme, is_selected: bool) {
        let block = create_border_block("System Stats", theme, is_selected);

        if let Some(ref stats) = self.system_stats {
            let inner = block.inner(area);
            f.render_widget(block, area);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(2),  // CPU
                    Constraint::Length(2),  // Memory
                    Constraint::Length(2),  // Disk
                    Constraint::Min(0),     // Additional info
                ])
                .split(inner);

            // CPU gauge
            let cpu_gauge = Gauge::default()
                .block(Block::default().title("CPU"))
                .gauge_style(Style::default().fg(Color::Cyan))
                .ratio(stats.cpu_usage as f64 / 100.0)
                .label(format!("{:.1}%", stats.cpu_usage));
            f.render_widget(cpu_gauge, chunks[0]);

            // Memory gauge
            let memory_gauge = Gauge::default()
                .block(Block::default().title("Memory"))
                .gauge_style(Style::default().fg(Color::Yellow))
                .ratio(stats.memory_usage as f64 / 100.0)
                .label(format!("{:.1}%", stats.memory_usage));
            f.render_widget(memory_gauge, chunks[1]);

            // Disk gauge
            let disk_gauge = Gauge::default()
                .block(Block::default().title("Disk"))
                .gauge_style(Style::default().fg(Color::Magenta))
                .ratio(stats.disk_usage as f64 / 100.0)
                .label(format!("{:.1}%", stats.disk_usage));
            f.render_widget(disk_gauge, chunks[2]);

            // Additional stats
            let uptime_text = vec![
                Line::from(format!("⏱ Uptime: {}d {}h {}m", 
                    stats.uptime.num_days(),
                    stats.uptime.num_hours() % 24,
                    stats.uptime.num_minutes() % 60
                )),
            ];

            let uptime_widget = Paragraph::new(uptime_text);
            f.render_widget(uptime_widget, chunks[3]);
        } else {
            let loading_text = vec![
                Line::from("Loading system stats..."),
                Line::from(""),
                Line::from("📊 Gathering data"),
            ];

            let stats_widget = Paragraph::new(loading_text)
                .block(block)
                .alignment(Alignment::Center);

            f.render_widget(stats_widget, area);
        }
    }

    fn render_additional_info(&self, f: &mut Frame, area: Rect, theme: &Theme, is_selected: bool) {
        let block = create_border_block("Tips & Info", theme, is_selected);

        let tips = vec![
            Line::from("💡 Keyboard Shortcuts:"),
            Line::from(""),
            Line::from("h/l - Navigate widgets"),
            Line::from("c - Compose email"),
            Line::from("/ - Search emails"),
            Line::from("a - Open address book"),
            Line::from("q - Quit application"),
            Line::from(""),
            Line::from("📧 Comunicado v0.1.0"),
        ];

        let tips_widget = Paragraph::new(tips)
            .block(block)
            .wrap(Wrap { trim: true });

        f.render_widget(tips_widget, area);
    }
}

impl Default for StartPage {
    fn default() -> Self {
        Self::new()
    }
}