//! Modern Dashboard - Redesigned startup interface with real-time updates
//!
//! Features:
//! - Real-time clock with animations
//! - System load visualization with gauges and graphs
//! - Enhanced calendar with visual events
//! - Modern weather widget with animations
//! - Contact cards with avatars
//! - Startup progress with real-time updates

use chrono::{DateTime, Local, Timelike};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Style},
    widgets::{Paragraph, Wrap},
    Frame,
};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crate::theme::Theme;

/// Modern dashboard with real-time updates and animations
pub struct ModernDashboard {
    /// Real-time clock state
    pub(crate) clock_state: ClockState,
    /// System monitoring
    pub(crate) system_monitor: SystemMonitor,
    /// Weather widget
    pub(crate) weather_widget: WeatherWidget,
    /// Calendar widget
    pub(crate) calendar_widget: CalendarWidget,
    /// Contacts widget
    pub(crate) contacts_widget: ContactsWidget,
    /// Startup progress widget
    pub(crate) startup_widget: StartupWidget,
    /// Animation state
    pub(crate) animation_state: AnimationState,
    /// Last update time
    pub(crate) last_update: Instant,
    /// Update interval
    pub(crate) update_interval: Duration,
}

/// Real-time clock with animations
#[derive(Debug, Clone)]
pub struct ClockState {
    pub(crate) current_time: DateTime<Local>,
    pub(crate) time_format: TimeFormat,
    pub(crate) show_seconds: bool,
    pub(crate) animation_phase: f32,
    pub(crate) timezone_display: bool,
    pub(crate) date_format: DateFormat,
}

/// System monitoring with visual gauges
#[derive(Debug, Clone)]
pub struct SystemMonitor {
    pub(crate) cpu_usage: f64,
    pub(crate) memory_usage: f64,
    pub(crate) disk_usage: f64,
    pub(crate) network_activity: NetworkActivity,
    pub(crate) cpu_history: VecDeque<f64>,
    pub(crate) memory_history: VecDeque<f64>,
    #[allow(dead_code)]
    pub(crate) temperature: Option<f64>,
    pub(crate) load_average: [f64; 3],
    #[allow(dead_code)]
    pub(crate) uptime: Duration,
}

/// Network activity tracking
#[derive(Debug, Clone)]
pub struct NetworkActivity {
    pub(crate) upload_speed: f64,
    pub(crate) download_speed: f64,
    pub(crate) upload_history: VecDeque<f64>,
    pub(crate) download_history: VecDeque<f64>,
    pub(crate) total_upload: u64,
    pub(crate) total_download: u64,
}

/// Enhanced weather widget with animations
#[derive(Debug, Clone)]
pub struct WeatherWidget {
    pub(crate) current_weather: Option<CurrentWeather>,
    pub(crate) forecast: Vec<WeatherForecast>,
    pub(crate) animation_frame: usize,
    pub(crate) weather_icons: WeatherIcons,
    #[allow(dead_code)]
    pub(crate) show_forecast: bool,
    pub(crate) update_time: Option<DateTime<Local>>,
}

/// Current weather information
#[derive(Debug, Clone)]
pub struct CurrentWeather {
    pub(crate) location: String,
    pub(crate) temperature: f64,
    pub(crate) feels_like: f64,
    pub(crate) condition: WeatherCondition,
    pub(crate) humidity: u32,
    pub(crate) wind_speed: f64,
    #[allow(dead_code)]
    pub(crate) wind_direction: u16,
    #[allow(dead_code)]
    pub(crate) pressure: f64,
    pub(crate) visibility: f64,
    pub(crate) uv_index: u32,
}

/// Weather conditions with visual representations
#[derive(Debug, Clone, PartialEq)]
pub enum WeatherCondition {
    Clear,
    PartlyCloudy,
    Cloudy,
    Overcast,
    Rain,
    HeavyRain,
    Snow,
    Thunderstorm,
    Fog,
    Windy,
}

/// Weather forecast item
#[derive(Debug, Clone)]
pub struct WeatherForecast {
    #[allow(dead_code)]
    pub(crate) date: DateTime<Local>,
    #[allow(dead_code)]
    pub(crate) high_temp: f64,
    #[allow(dead_code)]
    pub(crate) low_temp: f64,
    #[allow(dead_code)]
    pub(crate) condition: WeatherCondition,
    #[allow(dead_code)]
    pub(crate) precipitation_chance: u32,
    #[allow(dead_code)]
    pub(crate) wind_speed: f64,
}

/// Enhanced calendar widget
#[derive(Debug, Clone)]
pub struct CalendarWidget {
    pub(crate) current_date: DateTime<Local>,
    pub(crate) events: Vec<CalendarEvent>,
    pub(crate) view_mode: CalendarViewMode,
    pub(crate) selected_date: Option<DateTime<Local>>,
    #[allow(dead_code)]
    pub(crate) show_week_numbers: bool,
    #[allow(dead_code)]
    pub(crate) highlight_today: bool,
}

/// Calendar event with rich information
#[derive(Debug, Clone)]
pub struct CalendarEvent {
    #[allow(dead_code)]
    pub(crate) id: String,
    pub(crate) title: String,
    #[allow(dead_code)]
    pub(crate) description: Option<String>,
    pub(crate) start_time: DateTime<Local>,
    #[allow(dead_code)]
    pub(crate) end_time: DateTime<Local>,
    pub(crate) event_type: EventType,
    #[allow(dead_code)]
    pub(crate) location: Option<String>,
    #[allow(dead_code)]
    pub(crate) attendees: Vec<String>,
    #[allow(dead_code)]
    pub(crate) reminder: Option<Duration>,
    pub(crate) color: EventColor,
}

/// Event types with different visual styles
#[derive(Debug, Clone, PartialEq)]
pub enum EventType {
    Meeting,
    Appointment,
    Reminder,
    Birthday,
    Holiday,
    Personal,
    Work,
    Travel,
}

/// Event color coding
#[derive(Debug, Clone, PartialEq)]
pub enum EventColor {
    Blue,
    Green,
    Red,
    Yellow,
    Purple,
    Orange,
    Pink,
    Gray,
}

/// Calendar view modes
#[derive(Debug, Clone, PartialEq)]
pub enum CalendarViewMode {
    Month,
    Week,
    Day,
    Agenda,
}

/// Modern contacts widget
#[derive(Debug, Clone)]
pub struct ContactsWidget {
    pub(crate) recent_contacts: Vec<Contact>,
    pub(crate) favorite_contacts: Vec<Contact>,
    pub(crate) contact_count: usize,
    pub(crate) show_avatars: bool,
    pub(crate) view_mode: ContactViewMode,
}

/// Contact information with avatar
#[derive(Debug, Clone)]
pub struct Contact {
    #[allow(dead_code)]
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) email: String,
    #[allow(dead_code)]
    pub(crate) phone: Option<String>,
    pub(crate) avatar: Option<String>,
    pub(crate) last_contact: Option<DateTime<Local>>,
    #[allow(dead_code)]
    pub(crate) contact_frequency: u32,
    pub(crate) is_favorite: bool,
    pub(crate) status: ContactStatus,
}

/// Contact status indicators
#[derive(Debug, Clone, PartialEq)]
pub enum ContactStatus {
    Online,
    Away,
    Busy,
    Offline,
    Unknown,
}

/// Contact view modes
#[derive(Debug, Clone, PartialEq)]
pub enum ContactViewMode {
    Recent,
    Favorites,
    All,
}

/// Startup progress with real-time updates
#[derive(Debug, Clone)]
pub struct StartupWidget {
    pub(crate) phases: Vec<StartupPhase>,
    pub(crate) current_phase: usize,
    pub(crate) overall_progress: f64,
    #[allow(dead_code)]
    pub(crate) show_detailed_progress: bool,
    pub(crate) animation_progress: f64,
    pub(crate) estimated_time_remaining: Option<Duration>,
}

/// Startup phase information
#[derive(Debug, Clone)]
pub struct StartupPhase {
    pub(crate) name: String,
    #[allow(dead_code)]
    pub(crate) description: String,
    pub(crate) progress: f64,
    pub(crate) status: PhaseStatus,
    pub(crate) start_time: Option<Instant>,
    pub(crate) duration: Option<Duration>,
    #[allow(dead_code)]
    pub(crate) substeps: Vec<String>,
}

/// Phase status indicators
#[derive(Debug, Clone, PartialEq)]
pub enum PhaseStatus {
    Pending,
    InProgress,
    Completed,
    Error,
    Skipped,
}

/// Animation state for smooth transitions
#[derive(Debug, Clone)]
pub struct AnimationState {
    pub(crate) time_elapsed: Duration,
    pub(crate) pulse_phase: f32,
    pub(crate) rotation_angle: f32,
    pub(crate) bounce_offset: f32,
    #[allow(dead_code)]
    pub(crate) fade_alpha: f32,
    pub(crate) sparkle_positions: Vec<(f32, f32)>,
}

/// Time display formats
#[derive(Debug, Clone, PartialEq)]
pub enum TimeFormat {
    TwentyFourHour,
    TwelveHour,
    Custom(String),
}

/// Date display formats
#[derive(Debug, Clone, PartialEq)]
pub enum DateFormat {
    Standard,     // March 15, 2024
    Compact,      // 03/15/24
    ISO,          // 2024-03-15
    Verbose,      // Friday, March 15th, 2024
}

/// Weather icon representations
#[derive(Debug, Clone)]
pub struct WeatherIcons {
    pub(crate) clear_day: Vec<&'static str>,
    #[allow(dead_code)]
    pub(crate) clear_night: Vec<&'static str>,
    pub(crate) cloudy: Vec<&'static str>,
    pub(crate) rain: Vec<&'static str>,
    pub(crate) snow: Vec<&'static str>,
    pub(crate) thunderstorm: Vec<&'static str>,
}

impl Default for WeatherIcons {
    fn default() -> Self {
        Self {
            clear_day: vec!["☀️", "🌞", "☀️"],
            clear_night: vec!["🌙", "🌛", "🌜"],
            cloudy: vec!["☁️", "⛅", "🌥️"],
            rain: vec!["🌧️", "☔", "💧"],
            snow: vec!["❄️", "🌨️", "⛄"],
            thunderstorm: vec!["⛈️", "🌩️", "⚡"],
        }
    }
}

impl ModernDashboard {
    /// Create new modern dashboard
    pub fn new() -> Self {
        Self {
            clock_state: ClockState {
                current_time: Local::now(),
                time_format: TimeFormat::TwentyFourHour,
                show_seconds: true,
                animation_phase: 0.0,
                timezone_display: true,
                date_format: DateFormat::Verbose,
            },
            system_monitor: SystemMonitor {
                cpu_usage: 0.0,
                memory_usage: 0.0,
                disk_usage: 0.0,
                network_activity: NetworkActivity {
                    upload_speed: 0.0,
                    download_speed: 0.0,
                    upload_history: VecDeque::with_capacity(60),
                    download_history: VecDeque::with_capacity(60),
                    total_upload: 0,
                    total_download: 0,
                },
                cpu_history: VecDeque::with_capacity(60),
                memory_history: VecDeque::with_capacity(60),
                temperature: None,
                load_average: [0.0, 0.0, 0.0],
                uptime: Duration::new(0, 0),
            },
            weather_widget: WeatherWidget {
                current_weather: None,
                forecast: Vec::new(),
                animation_frame: 0,
                weather_icons: WeatherIcons::default(),
                show_forecast: true,
                update_time: None,
            },
            calendar_widget: CalendarWidget {
                current_date: Local::now(),
                events: Vec::new(),
                view_mode: CalendarViewMode::Month,
                selected_date: None,
                show_week_numbers: false,
                highlight_today: true,
            },
            contacts_widget: ContactsWidget {
                recent_contacts: Vec::new(),
                favorite_contacts: Vec::new(),
                contact_count: 0,
                show_avatars: true,
                view_mode: ContactViewMode::Recent,
            },
            startup_widget: StartupWidget {
                phases: Vec::new(),
                current_phase: 0,
                overall_progress: 0.0,
                show_detailed_progress: true,
                animation_progress: 0.0,
                estimated_time_remaining: None,
            },
            animation_state: AnimationState {
                time_elapsed: Duration::new(0, 0),
                pulse_phase: 0.0,
                rotation_angle: 0.0,
                bounce_offset: 0.0,
                fade_alpha: 1.0,
                sparkle_positions: Vec::new(),
            },
            last_update: Instant::now(),
            update_interval: Duration::from_millis(100), // 10 FPS for smooth animations
        }
    }

    /// Update dashboard state with real-time data
    pub fn update(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update);
        
        if elapsed >= self.update_interval {
            self.update_clock();
            self.update_animations(elapsed);
            self.update_system_monitor();
            self.update_weather_animations();
            self.update_startup_progress();
            
            self.last_update = now;
        }
    }

    /// Update real-time clock
    fn update_clock(&mut self) {
        self.clock_state.current_time = Local::now();
        
        // Animate seconds hand
        let seconds = self.clock_state.current_time.second() as f32;
        self.clock_state.animation_phase = (seconds * 6.0).to_radians(); // 6 degrees per second
    }

    /// Update animations
    fn update_animations(&mut self, elapsed: Duration) {
        self.animation_state.time_elapsed += elapsed;
        let time_ms = self.animation_state.time_elapsed.as_millis() as f32;
        
        // Pulse animation (2 second cycle)
        self.animation_state.pulse_phase = (time_ms / 2000.0) * 2.0 * std::f32::consts::PI;
        
        // Rotation animation (4 second cycle)
        self.animation_state.rotation_angle = (time_ms / 4000.0) * 2.0 * std::f32::consts::PI;
        
        // Bounce animation (1.5 second cycle)
        self.animation_state.bounce_offset = ((time_ms / 1500.0) * 2.0 * std::f32::consts::PI).sin() * 0.1;
        
        // Update sparkles
        self.update_sparkles();
    }

    /// Update sparkle positions for visual effects
    fn update_sparkles(&mut self) {
        // Generate random sparkles for visual appeal
        if self.animation_state.sparkle_positions.len() < 20 {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            
            let mut hasher = DefaultHasher::new();
            self.animation_state.time_elapsed.as_millis().hash(&mut hasher);
            let seed = hasher.finish();
            
            let x = ((seed % 1000) as f32) / 10.0;
            let y = (((seed / 1000) % 1000) as f32) / 10.0;
            
            self.animation_state.sparkle_positions.push((x, y));
        }
        
        // Remove old sparkles
        if self.animation_state.sparkle_positions.len() > 50 {
            self.animation_state.sparkle_positions.remove(0);
        }
    }

    /// Update system monitoring data
    fn update_system_monitor(&mut self) {
        // In a real implementation, this would gather actual system data
        // For demo purposes, we'll simulate realistic values
        
        let time_factor = self.animation_state.time_elapsed.as_secs_f64();
        
        // Simulate CPU usage with some variation
        self.system_monitor.cpu_usage = 20.0 + 15.0 * (time_factor * 0.1).sin() + 10.0 * (time_factor * 0.3).cos();
        self.system_monitor.cpu_usage = self.system_monitor.cpu_usage.clamp(0.0, 100.0);
        
        // Simulate memory usage
        self.system_monitor.memory_usage = 60.0 + 10.0 * (time_factor * 0.05).sin();
        
        // Simulate disk usage (more stable)
        self.system_monitor.disk_usage = 45.0 + 2.0 * (time_factor * 0.01).sin();
        
        // Update history
        self.system_monitor.cpu_history.push_back(self.system_monitor.cpu_usage);
        if self.system_monitor.cpu_history.len() > 60 {
            self.system_monitor.cpu_history.pop_front();
        }
        
        self.system_monitor.memory_history.push_back(self.system_monitor.memory_usage);
        if self.system_monitor.memory_history.len() > 60 {
            self.system_monitor.memory_history.pop_front();
        }
        
        // Simulate network activity
        self.system_monitor.network_activity.download_speed = 50.0 + 30.0 * (time_factor * 0.2).sin().abs();
        self.system_monitor.network_activity.upload_speed = 20.0 + 15.0 * (time_factor * 0.15).cos().abs();
        
        // Update network history
        self.system_monitor.network_activity.download_history.push_back(self.system_monitor.network_activity.download_speed);
        if self.system_monitor.network_activity.download_history.len() > 60 {
            self.system_monitor.network_activity.download_history.pop_front();
        }
        
        self.system_monitor.network_activity.upload_history.push_back(self.system_monitor.network_activity.upload_speed);
        if self.system_monitor.network_activity.upload_history.len() > 60 {
            self.system_monitor.network_activity.upload_history.pop_front();
        }
    }

    /// Update weather animations
    fn update_weather_animations(&mut self) {
        self.weather_widget.animation_frame = (self.weather_widget.animation_frame + 1) % 100;
    }

    /// Update startup progress
    fn update_startup_progress(&mut self) {
        // Simulate startup progress
        if self.startup_widget.overall_progress < 100.0 {
            self.startup_widget.overall_progress += 0.5; // Gradual progress
            self.startup_widget.animation_progress = 
                ((self.animation_state.pulse_phase.sin() + 1.0) / 2.0) as f64;
        }
    }

    /// Render the complete modern dashboard
    pub fn render(&mut self, f: &mut Frame<'_>, area: Rect, theme: &Theme) {
        // Update before rendering
        self.update();

        // Create main layout
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(8),  // Header with clock and weather
                Constraint::Min(0),     // Main content area
                Constraint::Length(4),  // Footer with quick stats
            ])
            .split(area);

        // Render header
        self.render_header(f, main_chunks[0], theme);
        
        // Render main content
        self.render_main_content(f, main_chunks[1], theme);
        
        // Render footer
        self.render_footer(f, main_chunks[2], theme);
    }

    /// Render the header with clock and weather
    fn render_header(&self, f: &mut Frame<'_>, area: Rect, theme: &Theme) {
        let header_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(40), // Clock and date
                Constraint::Percentage(35), // Weather
                Constraint::Percentage(25), // Quick stats
            ])
            .split(area);

        // Render clock
        self.render_clock_widget(f, header_chunks[0], theme);
        
        // Render weather
        self.render_weather_widget(f, header_chunks[1], theme);
        
        // Render compact system monitor
        self.render_compact_system_monitor(f, header_chunks[2], theme);
    }

    /// Render main content area
    fn render_main_content(&self, f: &mut Frame<'_>, area: Rect, theme: &Theme) {
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(60), // Calendar (increased, no system monitor)
                Constraint::Percentage(40), // Contacts and startup
            ])
            .split(area);

        // Render calendar
        self.render_calendar_widget(f, content_chunks[0], theme);
        
        // Render contacts and startup with better balance
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(50), // Contacts
                Constraint::Percentage(50), // Startup progress
            ])
            .split(content_chunks[1]);
            
        self.render_contacts_widget(f, right_chunks[0], theme);
        self.render_startup_widget(f, right_chunks[1], theme);
    }

    /// Render footer with system information
    fn render_footer(&self, f: &mut Frame<'_>, area: Rect, theme: &Theme) {
        let footer_text = format!(
            "📧 Comunicado Dashboard │ 🚀 Flash Fast Mode │ ⚡ {} Updates/sec │ 💾 {}MB RAM │ 🌐 Network: ↑{:.1}KB/s ↓{:.1}KB/s",
            (1000.0 / self.update_interval.as_millis() as f64) as u32,
            (self.system_monitor.memory_usage * 10.0) as u32,
            self.system_monitor.network_activity.upload_speed,
            self.system_monitor.network_activity.download_speed
        );

        let footer = Paragraph::new(footer_text)
            .style(Style::default().fg(theme.colors.palette.text_muted))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        f.render_widget(footer, area);
    }
}