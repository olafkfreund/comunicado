//! Modern Dashboard Widget Implementations
//!
//! Individual widget rendering implementations for the modern dashboard

use super::modern_dashboard::*;
use chrono::{DateTime, Local, Timelike, Datelike};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, Gauge, LineGauge, Paragraph, Sparkline, Wrap,
        canvas::{Canvas, Context, Line as CanvasLine, Points, Rectangle},
        List, ListItem, Row, Table, Cell,
    },
    Frame,
};

use crate::theme::Theme;

impl ModernDashboard {
    /// Render the real-time clock widget with animations
    pub fn render_clock_widget(&self, f: &mut Frame<'_>, area: Rect, theme: &Theme) {
        let time = &self.clock_state.current_time;
        
        // Create animated border with pulse effect
        let pulse_intensity = (self.animation_state.pulse_phase.sin() + 1.0) / 2.0;
        let border_color = Color::Rgb(
            (theme.colors.palette.accent.r as f32 * (0.5 + pulse_intensity * 0.5)) as u8,
            (theme.colors.palette.accent.g as f32 * (0.5 + pulse_intensity * 0.5)) as u8,
            (theme.colors.palette.accent.b as f32 * (0.5 + pulse_intensity * 0.5)) as u8,
        );

        let block = Block::default()
            .title("🕒 Real-Time Clock")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        let inner = block.inner(area);
        f.render_widget(block, area);

        // Format time with animations
        let time_str = match self.clock_state.time_format {
            TimeFormat::TwentyFourHour => {
                if self.clock_state.show_seconds {
                    format!("{:02}:{:02}:{:02}", time.hour(), time.minute(), time.second())
                } else {
                    format!("{:02}:{:02}", time.hour(), time.minute())
                }
            }
            TimeFormat::TwelveHour => {
                let (hour, am_pm) = if time.hour() == 0 {
                    (12, "AM")
                } else if time.hour() < 12 {
                    (time.hour(), "AM")
                } else if time.hour() == 12 {
                    (12, "PM")
                } else {
                    (time.hour() - 12, "PM")
                };
                
                if self.clock_state.show_seconds {
                    format!("{:02}:{:02}:{:02} {}", hour, time.minute(), time.second(), am_pm)
                } else {
                    format!("{:02}:{:02} {}", hour, time.minute(), am_pm)
                }
            }
            TimeFormat::Custom(ref format) => time.format(format).to_string(),
        };

        // Format date
        let date_str = match self.clock_state.date_format {
            DateFormat::Standard => time.format("%B %d, %Y").to_string(),
            DateFormat::Compact => time.format("%m/%d/%y").to_string(),
            DateFormat::ISO => time.format("%Y-%m-%d").to_string(),
            DateFormat::Verbose => {
                let day_suffix = match time.day() {
                    1 | 21 | 31 => "st",
                    2 | 22 => "nd",
                    3 | 23 => "rd",
                    _ => "th",
                };
                format!("{}, {} {}{}, {}", 
                    time.format("%A"), 
                    time.format("%B"), 
                    time.day(), 
                    day_suffix, 
                    time.year()
                )
            }
        };

        // Create animated time display
        let time_lines = vec![
            Line::from(vec![
                Span::styled(
                    time_str,
                    Style::default()
                        .fg(theme.colors.palette.primary)
                        .add_modifier(Modifier::BOLD)
                )
            ]),
            Line::from(vec![
                Span::styled(
                    date_str,
                    Style::default().fg(theme.colors.palette.text_dim)
                )
            ]),
        ];

        // Add timezone if enabled
        let mut lines = time_lines;
        if self.clock_state.timezone_display {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("🌍 {}", time.format("%Z")),
                    Style::default().fg(theme.colors.palette.text_dim)
                )
            ]));
        }

        // Add animated seconds indicator
        if self.clock_state.show_seconds {
            let seconds_bar_width = (inner.width as f32 * (time.second() as f32 / 60.0)) as u16;
            lines.push(Line::from(vec![
                Span::styled(
                    "▓".repeat(seconds_bar_width as usize),
                    Style::default().fg(theme.colors.palette.accent)
                ),
                Span::styled(
                    "░".repeat((inner.width - seconds_bar_width) as usize),
                    Style::default().fg(theme.colors.palette.text_dim)
                )
            ]));
        }

        let clock_paragraph = Paragraph::new(lines)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        f.render_widget(clock_paragraph, inner);
    }

    /// Render enhanced weather widget with animations
    pub fn render_weather_widget(&self, f: &mut Frame<'_>, area: Rect, theme: &Theme) {
        let block = Block::default()
            .title("🌤️ Weather")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.colors.palette.border));

        let inner = block.inner(area);
        f.render_widget(block, area);

        if let Some(ref weather) = self.weather_widget.current_weather {
            let mut lines = vec![
                Line::from(vec![
                    Span::styled(
                        format!("📍 {}", weather.location),
                        Style::default().fg(theme.colors.palette.text)
                    )
                ]),
                Line::from(vec![
                    Span::styled(
                        format!("{}°C", weather.temperature as i32),
                        Style::default()
                            .fg(theme.colors.palette.primary)
                            .add_modifier(Modifier::BOLD)
                    ),
                    Span::styled(
                        format!(" (feels like {}°C)", weather.feels_like as i32),
                        Style::default().fg(theme.colors.palette.text_dim)
                    )
                ]),
                Line::from(vec![
                    Span::styled(
                        self.get_weather_icon(&weather.condition),
                        Style::default().fg(self.get_weather_color(&weather.condition, theme))
                    ),
                    Span::styled(
                        format!(" {}", self.format_weather_condition(&weather.condition)),
                        Style::default().fg(theme.colors.palette.text)
                    )
                ]),
            ];

            // Add additional weather info
            lines.push(Line::from(vec![
                Span::styled(
                    format!("💧 {}% • 💨 {:.1}km/h • 👁 {:.1}km",
                        weather.humidity,
                        weather.wind_speed,
                        weather.visibility
                    ),
                    Style::default().fg(theme.colors.palette.text_dim)
                )
            ]));

            // Add UV index with color coding
            let uv_color = match weather.uv_index {
                0..=2 => Color::Green,
                3..=5 => Color::Yellow,
                6..=7 => Color::LightRed,
                8..=10 => Color::Red,
                _ => Color::Magenta,
            };

            lines.push(Line::from(vec![
                Span::styled(
                    format!("☀️ UV: {}", weather.uv_index),
                    Style::default().fg(uv_color)
                )
            ]));

            let weather_paragraph = Paragraph::new(lines)
                .alignment(Alignment::Left)
                .wrap(Wrap { trim: true });

            f.render_widget(weather_paragraph, inner);
        } else {
            let loading_text = vec![
                Line::from(vec![
                    Span::styled(
                        "🔄 Loading weather data...",
                        Style::default().fg(theme.colors.palette.text_dim)
                    )
                ])
            ];

            let loading_paragraph = Paragraph::new(loading_text)
                .alignment(Alignment::Center);

            f.render_widget(loading_paragraph, inner);
        }
    }

    /// Render system monitoring with visual gauges and graphs
    pub fn render_system_monitoring(&self, f: &mut Frame<'_>, area: Rect, theme: &Theme) {
        let block = Block::default()
            .title("📊 System Monitor")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.colors.palette.border));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let monitor_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // CPU
                Constraint::Length(2), // Memory
                Constraint::Length(2), // Disk
                Constraint::Length(4), // Network
                Constraint::Min(0),    // Graphs
            ])
            .split(inner);

        // CPU Usage Gauge
        self.render_system_gauge(
            f,
            monitor_chunks[0],
            theme,
            "CPU",
            self.system_monitor.cpu_usage,
            Color::Cyan,
            "🔥"
        );

        // Memory Usage Gauge
        self.render_system_gauge(
            f,
            monitor_chunks[1],
            theme,
            "RAM",
            self.system_monitor.memory_usage,
            Color::Green,
            "💾"
        );

        // Disk Usage Gauge
        self.render_system_gauge(
            f,
            monitor_chunks[2],
            theme,
            "Disk",
            self.system_monitor.disk_usage,
            Color::Yellow,
            "💿"
        );

        // Network Activity
        self.render_network_activity(f, monitor_chunks[3], theme);

        // Historical graphs
        if monitor_chunks[4].height > 3 {
            self.render_system_graphs(f, monitor_chunks[4], theme);
        }
    }

    /// Render individual system gauge
    fn render_system_gauge(
        &self,
        f: &mut Frame<'_>,
        area: Rect,
        theme: &Theme,
        label: &str,
        value: f64,
        color: Color,
        icon: &str,
    ) {
        let gauge_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(8), Constraint::Min(0)])
            .split(area);

        // Label
        let label_text = Paragraph::new(format!("{} {}:", icon, label))
            .style(Style::default().fg(theme.colors.palette.text));
        f.render_widget(label_text, gauge_chunks[0]);

        // Gauge
        let gauge_color = if value > 80.0 {
            Color::Red
        } else if value > 60.0 {
            Color::Yellow
        } else {
            color
        };

        let gauge = LineGauge::default()
            .block(Block::default())
            .gauge_style(Style::default().fg(gauge_color))
            .line_set(symbols::line::THICK)
            .ratio(value / 100.0)
            .label(format!("{:.1}%", value));

        f.render_widget(gauge, gauge_chunks[1]);
    }

    /// Render network activity display
    fn render_network_activity(&self, f: &mut Frame<'_>, area: Rect, theme: &Theme) {
        let net_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Length(1)])
            .split(area);

        // Upload
        let upload_text = Paragraph::new(format!(
            "📤 Upload: {:.1} KB/s",
            self.system_monitor.network_activity.upload_speed
        ))
        .style(Style::default().fg(theme.colors.palette.text));
        f.render_widget(upload_text, net_chunks[0]);

        // Download
        let download_text = Paragraph::new(format!(
            "📥 Download: {:.1} KB/s",
            self.system_monitor.network_activity.download_speed
        ))
        .style(Style::default().fg(theme.colors.palette.text));
        f.render_widget(download_text, net_chunks[1]);

        // Total
        let total_text = Paragraph::new(format!(
            "📊 Total: ↑{}MB ↓{}MB",
            self.system_monitor.network_activity.total_upload / 1024 / 1024,
            self.system_monitor.network_activity.total_download / 1024 / 1024
        ))
        .style(Style::default().fg(theme.colors.palette.text_dim));
        f.render_widget(total_text, net_chunks[2]);
    }

    /// Render system performance graphs
    fn render_system_graphs(&self, f: &mut Frame<'_>, area: Rect, theme: &Theme) {
        let graph_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // CPU History Sparkline
        if !self.system_monitor.cpu_history.is_empty() {
            let cpu_data: Vec<u64> = self.system_monitor.cpu_history
                .iter()
                .map(|&x| x as u64)
                .collect();

            let cpu_sparkline = Sparkline::default()
                .block(Block::default().title("CPU History").borders(Borders::ALL))
                .data(&cpu_data)
                .style(Style::default().fg(Color::Cyan));

            f.render_widget(cpu_sparkline, graph_chunks[0]);
        }

        // Memory History Sparkline
        if !self.system_monitor.memory_history.is_empty() {
            let mem_data: Vec<u64> = self.system_monitor.memory_history
                .iter()
                .map(|&x| x as u64)
                .collect();

            let mem_sparkline = Sparkline::default()
                .block(Block::default().title("Memory History").borders(Borders::ALL))
                .data(&mem_data)
                .style(Style::default().fg(Color::Green));

            f.render_widget(mem_sparkline, graph_chunks[1]);
        }
    }

    /// Render quick stats in header
    pub fn render_quick_stats(&self, f: &mut Frame<'_>, area: Rect, theme: &Theme) {
        let block = Block::default()
            .title("⚡ Quick Stats")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.colors.palette.border));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let stats_lines = vec![
            Line::from(vec![
                Span::styled(
                    format!("🔥 CPU: {:.1}%", self.system_monitor.cpu_usage),
                    Style::default().fg(self.get_usage_color(self.system_monitor.cpu_usage, theme))
                )
            ]),
            Line::from(vec![
                Span::styled(
                    format!("💾 RAM: {:.1}%", self.system_monitor.memory_usage),
                    Style::default().fg(self.get_usage_color(self.system_monitor.memory_usage, theme))
                )
            ]),
            Line::from(vec![
                Span::styled(
                    format!("📊 Load: {:.2}", self.system_monitor.load_average[0]),
                    Style::default().fg(theme.colors.palette.text_dim)
                )
            ]),
        ];

        let stats_paragraph = Paragraph::new(stats_lines)
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });

        f.render_widget(stats_paragraph, inner);
    }

    /// Get color based on usage percentage
    fn get_usage_color(&self, usage: f64, theme: &Theme) -> Color {
        if usage > 80.0 {
            Color::Red
        } else if usage > 60.0 {
            Color::Yellow
        } else {
            theme.colors.palette.primary
        }
    }

    /// Get weather icon for condition
    fn get_weather_icon(&self, condition: &WeatherCondition) -> &str {
        let frame = self.weather_widget.animation_frame;
        let icons = &self.weather_widget.weather_icons;
        
        let icon_set = match condition {
            WeatherCondition::Clear => &icons.clear_day,
            WeatherCondition::PartlyCloudy | WeatherCondition::Cloudy => &icons.cloudy,
            WeatherCondition::Rain | WeatherCondition::HeavyRain => &icons.rain,
            WeatherCondition::Snow => &icons.snow,
            WeatherCondition::Thunderstorm => &icons.thunderstorm,
            _ => &icons.cloudy,
        };
        
        icon_set[frame % icon_set.len()]
    }

    /// Get weather color for condition
    fn get_weather_color(&self, condition: &WeatherCondition, theme: &Theme) -> Color {
        match condition {
            WeatherCondition::Clear => Color::Yellow,
            WeatherCondition::PartlyCloudy | WeatherCondition::Cloudy => Color::Cyan,
            WeatherCondition::Rain | WeatherCondition::HeavyRain => Color::Blue,
            WeatherCondition::Snow => Color::White,
            WeatherCondition::Thunderstorm => Color::Magenta,
            WeatherCondition::Fog => Color::Gray,
            _ => theme.colors.palette.text,
        }
    }

    /// Format weather condition name
    fn format_weather_condition(&self, condition: &WeatherCondition) -> &str {
        match condition {
            WeatherCondition::Clear => "Clear",
            WeatherCondition::PartlyCloudy => "Partly Cloudy",
            WeatherCondition::Cloudy => "Cloudy",
            WeatherCondition::Overcast => "Overcast",
            WeatherCondition::Rain => "Rain",
            WeatherCondition::HeavyRain => "Heavy Rain",
            WeatherCondition::Snow => "Snow",
            WeatherCondition::Thunderstorm => "Thunderstorm",
            WeatherCondition::Fog => "Fog",
            WeatherCondition::Windy => "Windy",
        }
    }
}