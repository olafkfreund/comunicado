//! Modern Dashboard Calendar Widget Implementation

use super::modern_dashboard::*;
use chrono::{DateTime, Local, Datelike, Weekday, NaiveDate, Duration as ChronoDuration};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, Paragraph, Wrap, List, ListItem, Row, Table, Cell,
    },
    Frame,
};

use crate::theme::Theme;

impl ModernDashboard {
    /// Render enhanced calendar widget
    pub fn render_calendar_widget(&self, f: &mut Frame<'_>, area: Rect, theme: &Theme) {
        let block = Block::default()
            .title("📅 Calendar")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.colors.palette.border));

        let inner = block.inner(area);
        f.render_widget(block, area);

        match self.calendar_widget.view_mode {
            CalendarViewMode::Month => self.render_month_view(f, inner, theme),
            CalendarViewMode::Week => self.render_week_view(f, inner, theme),
            CalendarViewMode::Day => self.render_day_view(f, inner, theme),
            CalendarViewMode::Agenda => self.render_agenda_view(f, inner, theme),
        }
    }

    /// Render month view calendar
    fn render_month_view(&self, f: &mut Frame<'_>, area: Rect, theme: &Theme) {
        let calendar_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Month header
                Constraint::Length(1), // Day headers
                Constraint::Min(0),    // Calendar grid
            ])
            .split(area);

        // Month header with navigation
        self.render_month_header(f, calendar_chunks[0], theme);
        
        // Day headers (Mon, Tue, Wed, etc.)
        self.render_day_headers(f, calendar_chunks[1], theme);
        
        // Calendar grid
        self.render_calendar_grid(f, calendar_chunks[2], theme);
    }

    /// Render month header with current month/year
    fn render_month_header(&self, f: &mut Frame<'_>, area: Rect, theme: &Theme) {
        let date = self.calendar_widget.current_date;
        let month_year = format!("{} {}", 
            date.format("%B"), 
            date.year()
        );

        let header_lines = vec![
            Line::from(vec![
                Span::styled(
                    "← ".to_string(),
                    Style::default().fg(theme.colors.palette.accent)
                ),
                Span::styled(
                    month_year,
                    Style::default()
                        .fg(theme.colors.palette.primary)
                        .add_modifier(Modifier::BOLD)
                ),
                Span::styled(
                    " →".to_string(),
                    Style::default().fg(theme.colors.palette.accent)
                )
            ]),
            Line::from(vec![
                Span::styled(
                    format!("Today: {}", Local::now().format("%B %d")),
                    Style::default().fg(theme.colors.palette.text_dim)
                )
            ]),
        ];

        let header_paragraph = Paragraph::new(header_lines)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        f.render_widget(header_paragraph, area);
    }

    /// Render day headers (Mon, Tue, Wed, etc.)
    fn render_day_headers(&self, f: &mut Frame<'_>, area: Rect, theme: &Theme) {
        let day_names = vec!["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        let day_spans: Vec<Span> = day_names
            .iter()
            .enumerate()
            .map(|(i, &day)| {
                let style = if i >= 5 { // Weekend
                    Style::default().fg(theme.colors.palette.accent)
                } else {
                    Style::default().fg(theme.colors.palette.text)
                };
                
                Span::styled(
                    format!("{:^9}", day),
                    style.add_modifier(Modifier::BOLD)
                )
            })
            .collect();

        let day_header_line = Line::from(day_spans);
        let day_headers = Paragraph::new(day_header_line)
            .alignment(Alignment::Left);

        f.render_widget(day_headers, area);
    }

    /// Render calendar grid with dates and events
    fn render_calendar_grid(&self, f: &mut Frame<'_>, area: Rect, theme: &Theme) {
        let date = self.calendar_widget.current_date;
        let today = Local::now().date_naive();
        
        // Get first day of month and calculate grid
        let first_of_month = date.with_day(1).unwrap();
        let first_weekday = first_of_month.weekday();
        
        // Calculate how many days to show from previous month
        let days_from_prev = match first_weekday {
            Weekday::Mon => 0,
            Weekday::Tue => 1,
            Weekday::Wed => 2,
            Weekday::Thu => 3,
            Weekday::Fri => 4,
            Weekday::Sat => 5,
            Weekday::Sun => 6,
        };

        // Create calendar rows
        let weeks_to_show = ((days_from_prev + self.days_in_month(date.year(), date.month()) as usize + 6) / 7);
        let row_height = area.height / weeks_to_show as u16;

        for week in 0..weeks_to_show {
            let week_area = Rect {
                x: area.x,
                y: area.y + (week as u16 * row_height),
                width: area.width,
                height: row_height,
            };

            self.render_calendar_week(f, week_area, theme, week, days_from_prev, &date, &today);
        }
    }

    /// Render a single week row in the calendar
    fn render_calendar_week(
        &self,
        f: &mut Frame<'_>,
        area: Rect,
        theme: &Theme,
        week_index: usize,
        days_from_prev: usize,
        current_month: &DateTime<Local>,
        today: &NaiveDate,
    ) {
        let day_width = area.width / 7;
        
        for day_of_week in 0..7 {
            let day_area = Rect {
                x: area.x + (day_of_week as u16 * day_width),
                y: area.y,
                width: day_width,
                height: area.height,
            };

            let day_number = (week_index * 7 + day_of_week) as i32 - days_from_prev as i32 + 1;
            
            self.render_calendar_day(f, day_area, theme, day_number, current_month, today, day_of_week >= 5);
        }
    }

    /// Render individual calendar day
    fn render_calendar_day(
        &self,
        f: &mut Frame<'_>,
        area: Rect,
        theme: &Theme,
        day_number: i32,
        current_month: &DateTime<Local>,
        today: &NaiveDate,
        is_weekend: bool,
    ) {
        let days_in_month = self.days_in_month(current_month.year(), current_month.month());
        
        if day_number < 1 || day_number > days_in_month as i32 {
            // Days from previous/next month
            let day_text = if day_number < 1 {
                let prev_month_days = if current_month.month() == 1 {
                    self.days_in_month(current_month.year() - 1, 12)
                } else {
                    self.days_in_month(current_month.year(), current_month.month() - 1)
                };
                format!("{}", prev_month_days + day_number)
            } else {
                format!("{}", day_number - days_in_month as i32)
            };

            let day_paragraph = Paragraph::new(day_text)
                .style(Style::default().fg(theme.colors.palette.text_dim))
                .alignment(Alignment::Center);

            f.render_widget(day_paragraph, area);
        } else {
            // Current month days
            let day_date = current_month.with_day(day_number as u32).unwrap().date_naive();
            let is_today = day_date == *today;
            let is_selected = self.calendar_widget.selected_date
                .map(|d| d.date_naive() == day_date)
                .unwrap_or(false);

            // Determine day style
            let day_style = if is_today {
                Style::default()
                    .fg(Color::Black)
                    .bg(theme.colors.palette.accent)
                    .add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default()
                    .fg(theme.colors.palette.primary)
                    .add_modifier(Modifier::BOLD)
            } else if is_weekend {
                Style::default().fg(theme.colors.palette.accent)
            } else {
                Style::default().fg(theme.colors.palette.text)
            };

            // Check for events on this day
            let events_count = self.calendar_widget.events
                .iter()
                .filter(|event| event.start_time.date_naive() == day_date)
                .count();

            let mut day_content = vec![
                Line::from(vec![
                    Span::styled(
                        format!("{:^3}", day_number),
                        day_style
                    )
                ])
            ];

            // Add event indicator
            if events_count > 0 && area.height > 1 {
                let event_indicator = if events_count == 1 {
                    "●".to_string()
                } else {
                    format!("●{}", events_count)
                };

                day_content.push(Line::from(vec![
                    Span::styled(
                        format!("{:^3}", event_indicator),
                        Style::default().fg(theme.colors.palette.primary)
                    )
                ]));
            }

            let day_paragraph = Paragraph::new(day_content)
                .alignment(Alignment::Center);

            f.render_widget(day_paragraph, area);
        }
    }

    /// Render week view
    fn render_week_view(&self, f: &mut Frame<'_>, area: Rect, theme: &Theme) {
        let week_text = vec![
            Line::from(vec![
                Span::styled(
                    "📅 Week View",
                    Style::default()
                        .fg(theme.colors.palette.primary)
                        .add_modifier(Modifier::BOLD)
                )
            ]),
            Line::from(vec![
                Span::styled(
                    "This week's schedule...",
                    Style::default().fg(theme.colors.palette.text_dim)
                )
            ]),
        ];

        let week_paragraph = Paragraph::new(week_text)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        f.render_widget(week_paragraph, area);
    }

    /// Render day view
    fn render_day_view(&self, f: &mut Frame<'_>, area: Rect, theme: &Theme) {
        let day_text = vec![
            Line::from(vec![
                Span::styled(
                    "📋 Day View",
                    Style::default()
                        .fg(theme.colors.palette.primary)
                        .add_modifier(Modifier::BOLD)
                )
            ]),
            Line::from(vec![
                Span::styled(
                    "Today's detailed schedule...",
                    Style::default().fg(theme.colors.palette.text_dim)
                )
            ]),
        ];

        let day_paragraph = Paragraph::new(day_text)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        f.render_widget(day_paragraph, area);
    }

    /// Render agenda view with upcoming events
    fn render_agenda_view(&self, f: &mut Frame<'_>, area: Rect, theme: &Theme) {
        if self.calendar_widget.events.is_empty() {
            let no_events_text = vec![
                Line::from(vec![
                    Span::styled(
                        "📋 No upcoming events",
                        Style::default().fg(theme.colors.palette.text_dim)
                    )
                ])
            ];

            let no_events_paragraph = Paragraph::new(no_events_text)
                .alignment(Alignment::Center);

            f.render_widget(no_events_paragraph, area);
        } else {
            // Show upcoming events
            let event_items: Vec<ListItem> = self.calendar_widget.events
                .iter()
                .take(5) // Show first 5 events
                .map(|event| {
                    let event_color = self.get_event_color(&event.color, theme);
                    let event_icon = self.get_event_icon(&event.event_type);
                    
                    ListItem::new(vec![
                        Line::from(vec![
                            Span::styled(
                                format!("{} {}", event_icon, event.title),
                                Style::default()
                                    .fg(event_color)
                                    .add_modifier(Modifier::BOLD)
                            )
                        ]),
                        Line::from(vec![
                            Span::styled(
                                format!("  📅 {}", event.start_time.format("%m/%d %H:%M")),
                                Style::default().fg(theme.colors.palette.text_dim)
                            )
                        ]),
                    ])
                })
                .collect();

            let events_list = List::new(event_items)
                .style(Style::default().fg(theme.colors.palette.text));

            f.render_widget(events_list, area);
        }
    }

    /// Get number of days in a month
    fn days_in_month(&self, year: i32, month: u32) -> u32 {
        if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1)
                .unwrap()
                .pred_opt()
                .unwrap()
                .day()
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1)
                .unwrap()
                .pred_opt()
                .unwrap()
                .day()
        }
    }

    /// Get color for event type
    fn get_event_color(&self, color: &EventColor, theme: &Theme) -> Color {
        match color {
            EventColor::Blue => Color::Blue,
            EventColor::Green => Color::Green,
            EventColor::Red => Color::Red,
            EventColor::Yellow => Color::Yellow,
            EventColor::Purple => Color::Magenta,
            EventColor::Orange => Color::LightRed,
            EventColor::Pink => Color::LightMagenta,
            EventColor::Gray => theme.colors.palette.text_dim,
        }
    }

    /// Get icon for event type
    fn get_event_icon(&self, event_type: &EventType) -> &str {
        match event_type {
            EventType::Meeting => "🤝",
            EventType::Appointment => "📋",
            EventType::Reminder => "⏰",
            EventType::Birthday => "🎂",
            EventType::Holiday => "🎉",
            EventType::Personal => "👤",
            EventType::Work => "💼",
            EventType::Travel => "✈️",
        }
    }
}