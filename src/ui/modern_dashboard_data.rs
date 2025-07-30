//! Modern Dashboard Data Management and Initialization

use super::modern_dashboard::*;
use chrono::Local;

impl ModernDashboard {
    /// Initialize dashboard with sample data
    pub fn initialize_with_sample_data(&mut self) {
        self.initialize_sample_weather();
        self.initialize_sample_calendar();
        self.initialize_sample_contacts();
        self.initialize_sample_startup();
    }

    /// Initialize sample weather data
    fn initialize_sample_weather(&mut self) {
        self.weather_widget.current_weather = Some(CurrentWeather {
            location: "San Francisco, CA".to_string(),
            temperature: 22.5,
            feels_like: 24.0,
            condition: WeatherCondition::PartlyCloudy,
            humidity: 65,
            wind_speed: 12.5,
            wind_direction: 245,
            pressure: 1013.25,
            visibility: 10.0,
            uv_index: 6,
        });

        self.weather_widget.forecast = vec![
            WeatherForecast {
                date: Local::now() + std::time::Duration::from_secs(24 * 60 * 60),
                high_temp: 25.0,
                low_temp: 18.0,
                condition: WeatherCondition::Clear,
                precipitation_chance: 10,
                wind_speed: 8.0,
            },
            WeatherForecast {
                date: Local::now() + std::time::Duration::from_secs(2 * 24 * 60 * 60),
                high_temp: 23.0,
                low_temp: 16.0,
                condition: WeatherCondition::Rain,
                precipitation_chance: 80,
                wind_speed: 15.0,
            },
            WeatherForecast {
                date: Local::now() + ChronoDuration::days(3),
                high_temp: 20.0,
                low_temp: 14.0,
                condition: WeatherCondition::Cloudy,
                precipitation_chance: 40,
                wind_speed: 10.0,
            },
        ];

        self.weather_widget.update_time = Some(Local::now());
    }

    /// Initialize sample calendar events
    fn initialize_sample_calendar(&mut self) {
        let now = Local::now();
        
        self.calendar_widget.events = vec![
            CalendarEvent {
                id: "1".to_string(),
                title: "Team Standup".to_string(),
                description: Some("Daily team synchronization meeting".to_string()),
                start_time: now + ChronoDuration::hours(2),
                end_time: now + ChronoDuration::hours(2) + ChronoDuration::minutes(30),
                event_type: EventType::Meeting,
                location: Some("Conference Room A".to_string()),
                attendees: vec!["Alice".to_string(), "Bob".to_string(), "Charlie".to_string()],
                reminder: Some(std::time::Duration::from_secs(15 * 60)),
                color: EventColor::Blue,
            },
            CalendarEvent {
                id: "2".to_string(),
                title: "Doctor Appointment".to_string(),
                description: Some("Annual checkup".to_string()),
                start_time: now + std::time::Duration::from_secs(24 * 60 * 60) + ChronoDuration::hours(10),
                end_time: now + std::time::Duration::from_secs(24 * 60 * 60) + ChronoDuration::hours(11),
                event_type: EventType::Appointment,
                location: Some("Medical Center".to_string()),
                attendees: vec![],
                reminder: Some(std::time::Duration::from_secs(60 * 60)),
                color: EventColor::Red,
            },
            CalendarEvent {
                id: "3".to_string(),
                title: "Project Deadline".to_string(),
                description: Some("Final submission for Q1 project".to_string()),
                start_time: now + ChronoDuration::days(3),
                end_time: now + ChronoDuration::days(3),
                event_type: EventType::Work,
                location: None,
                attendees: vec![],
                reminder: Some(std::time::Duration::from_secs(24 * 60 * 60)),
                color: EventColor::Orange,
            },
            CalendarEvent {
                id: "4".to_string(),
                title: "Sarah's Birthday".to_string(),
                description: Some("Birthday celebration".to_string()),
                start_time: now + ChronoDuration::days(5),
                end_time: now + ChronoDuration::days(5),
                event_type: EventType::Birthday,
                location: None,
                attendees: vec!["Sarah".to_string()],
                reminder: Some(std::time::Duration::from_secs(24 * 60 * 60)),
                color: EventColor::Pink,
            },
            CalendarEvent {
                id: "5".to_string(),
                title: "Weekend Trip".to_string(),
                description: Some("Mountain hiking trip".to_string()),
                start_time: now + ChronoDuration::days(7),
                end_time: now + ChronoDuration::days(9),
                event_type: EventType::Travel,
                location: Some("Yosemite National Park".to_string()),
                attendees: vec!["Family".to_string()],
                reminder: Some(std::time::Duration::from_secs(2 * 24 * 60 * 60)),
                color: EventColor::Green,
            },
        ];
    }

    /// Initialize sample contacts
    fn initialize_sample_contacts(&mut self) {
        let now = Local::now();
        
        self.contacts_widget.recent_contacts = vec![
            Contact {
                id: "1".to_string(),
                name: "Alice Johnson".to_string(),
                email: "alice.johnson@company.com".to_string(),
                phone: Some("+1 (555) 123-4567".to_string()),
                avatar: None,
                last_contact: Some(now - ChronoDuration::hours(2)),
                contact_frequency: 25,
                is_favorite: true,
                status: ContactStatus::Online,
            },
            Contact {
                id: "2".to_string(),
                name: "Bob Smith".to_string(),
                email: "bob.smith@email.com".to_string(),
                phone: Some("+1 (555) 234-5678".to_string()),
                avatar: None,
                last_contact: Some(now - std::time::Duration::from_secs(24 * 60 * 60)),
                contact_frequency: 18,
                is_favorite: false,
                status: ContactStatus::Away,
            },
            Contact {
                id: "3".to_string(),
                name: "Charlie Brown".to_string(),
                email: "charlie@example.org".to_string(),
                phone: None,
                avatar: None,
                last_contact: Some(now - ChronoDuration::days(3)),
                contact_frequency: 12,
                is_favorite: false,
                status: ContactStatus::Offline,
            },
            Contact {
                id: "4".to_string(),
                name: "Diana Prince".to_string(),
                email: "diana.prince@corp.com".to_string(),
                phone: Some("+1 (555) 345-6789".to_string()),
                avatar: None,
                last_contact: Some(now - ChronoDuration::hours(6)),
                contact_frequency: 30,
                is_favorite: true,
                status: ContactStatus::Busy,
            },
        ];

        self.contacts_widget.favorite_contacts = vec![
            Contact {
                id: "5".to_string(),
                name: "Emily Davis".to_string(),
                email: "emily@personal.com".to_string(),
                phone: Some("+1 (555) 456-7890".to_string()),
                avatar: None,
                last_contact: Some(now - ChronoDuration::minutes(30)),
                contact_frequency: 45,
                is_favorite: true,
                status: ContactStatus::Online,
            },
            Contact {
                id: "6".to_string(),
                name: "Frank Wilson".to_string(),
                email: "frank.wilson@business.net".to_string(),
                phone: Some("+1 (555) 567-8901".to_string()),
                avatar: None,
                last_contact: Some(now - ChronoDuration::hours(4)),
                contact_frequency: 22,
                is_favorite: true,
                status: ContactStatus::Away,
            },
        ];

        self.contacts_widget.contact_count = 
            self.contacts_widget.recent_contacts.len() + 
            self.contacts_widget.favorite_contacts.len();
    }

    /// Initialize sample startup phases
    fn initialize_sample_startup(&mut self) {
        self.startup_widget.phases = vec![
            StartupPhase {
                name: "Core Initialization".to_string(),
                description: "Loading essential components".to_string(),
                progress: 100.0,
                status: PhaseStatus::Completed,
                start_time: Some(std::time::Instant::now() - std::time::Duration::from_secs(10)),
                duration: Some(std::time::Duration::from_secs(2)),
                substeps: vec![
                    "Loading configuration".to_string(),
                    "Initializing storage".to_string(),
                    "Setting up logging".to_string(),
                ],
            },
            StartupPhase {
                name: "Database Connection".to_string(),
                description: "Connecting to email database".to_string(),
                progress: 100.0,
                status: PhaseStatus::Completed,
                start_time: Some(std::time::Instant::now() - std::time::Duration::from_secs(8)),
                duration: Some(std::time::Duration::from_secs(3)),
                substeps: vec![
                    "Establishing connection".to_string(),
                    "Running migrations".to_string(),
                    "Verifying schema".to_string(),
                ],
            },
            StartupPhase {
                name: "Email Systems".to_string(),
                description: "Initializing email components".to_string(),
                progress: 85.0,
                status: PhaseStatus::InProgress,
                start_time: Some(std::time::Instant::now() - std::time::Duration::from_secs(5)),
                duration: None,
                substeps: vec![
                    "Loading accounts".to_string(),
                    "Starting IMAP connections".to_string(),
                    "Initializing sync engine".to_string(),
                ],
            },
            StartupPhase {
                name: "Flash Fast Integration".to_string(),
                description: "Activating performance systems".to_string(),
                progress: 60.0,
                status: PhaseStatus::InProgress,
                start_time: Some(std::time::Instant::now() - std::time::Duration::from_secs(3)),
                duration: None,
                substeps: vec![
                    "Starting background processor".to_string(),
                    "Initializing cache system".to_string(),
                    "Enabling precaching".to_string(),
                ],
            },
            StartupPhase {
                name: "UI Initialization".to_string(),
                description: "Setting up user interface".to_string(),
                progress: 0.0,
                status: PhaseStatus::Pending,
                start_time: None,
                duration: None,
                substeps: vec![
                    "Loading themes".to_string(),
                    "Initializing components".to_string(),
                    "Setting up keybindings".to_string(),
                ],
            },
        ];

        self.startup_widget.current_phase = 2; // Currently on Email Systems
        self.startup_widget.overall_progress = 75.0;
        self.startup_widget.estimated_time_remaining = Some(std::time::Duration::from_secs(15));
    }

    /// Update startup progress simulation
    pub fn update_startup_simulation(&mut self) {
        if self.startup_widget.overall_progress >= 100.0 {
            return;
        }

        // Update current phase progress
        if let Some(phase) = self.startup_widget.phases.get_mut(self.startup_widget.current_phase) {
            if phase.status == PhaseStatus::InProgress {
                phase.progress += 1.0;
                
                if phase.progress >= 100.0 {
                    phase.progress = 100.0;
                    phase.status = PhaseStatus::Completed;
                    phase.duration = phase.start_time.map(|start| start.elapsed());
                    
                    // Move to next phase
                    if self.startup_widget.current_phase < self.startup_widget.phases.len() - 1 {
                        self.startup_widget.current_phase += 1;
                        
                        if let Some(next_phase) = self.startup_widget.phases.get_mut(self.startup_widget.current_phase) {
                            next_phase.status = PhaseStatus::InProgress;
                            next_phase.start_time = Some(std::time::Instant::now());
                        }
                    }
                }
            }
        }

        // Update overall progress
        let total_progress: f64 = self.startup_widget.phases
            .iter()
            .map(|phase| phase.progress)
            .sum();
        
        self.startup_widget.overall_progress = total_progress / self.startup_widget.phases.len() as f64;
        
        // Update estimated time remaining
        if self.startup_widget.overall_progress < 100.0 {
            let remaining_phases = self.startup_widget.phases.len() - self.startup_widget.current_phase - 1;
            let estimated_seconds = remaining_phases * 5; // 5 seconds per phase estimate
            self.startup_widget.estimated_time_remaining = Some(std::time::Duration::from_secs(estimated_seconds as u64));
        } else {
            self.startup_widget.estimated_time_remaining = None;
        }
    }

    /// Set weather data from external source
    pub fn set_weather(&mut self, weather: CurrentWeather) {
        self.weather_widget.current_weather = Some(weather);
        self.weather_widget.update_time = Some(Local::now());
    }

    /// Set weather forecast
    pub fn set_weather_forecast(&mut self, forecast: Vec<WeatherForecast>) {
        self.weather_widget.forecast = forecast;
    }

    /// Add calendar event
    pub fn add_calendar_event(&mut self, event: CalendarEvent) {
        self.calendar_widget.events.push(event);
        // Sort events by start time
        self.calendar_widget.events.sort_by(|a, b| a.start_time.cmp(&b.start_time));
    }

    /// Update calendar events
    pub fn set_calendar_events(&mut self, events: Vec<CalendarEvent>) {
        self.calendar_widget.events = events;
        self.calendar_widget.events.sort_by(|a, b| a.start_time.cmp(&b.start_time));
    }

    /// Add contact
    pub fn add_contact(&mut self, contact: Contact, to_favorites: bool) {
        if to_favorites {
            self.contacts_widget.favorite_contacts.push(contact);
        } else {
            self.contacts_widget.recent_contacts.push(contact);
        }
        self.contacts_widget.contact_count = 
            self.contacts_widget.recent_contacts.len() + 
            self.contacts_widget.favorite_contacts.len();
    }

    /// Update contacts
    pub fn set_contacts(&mut self, recent: Vec<Contact>, favorites: Vec<Contact>) {
        self.contacts_widget.recent_contacts = recent;
        self.contacts_widget.favorite_contacts = favorites;
        self.contacts_widget.contact_count = 
            self.contacts_widget.recent_contacts.len() + 
            self.contacts_widget.favorite_contacts.len();
    }

    /// Update system stats
    pub fn update_system_stats(&mut self, cpu: f64, memory: f64, disk: f64) {
        self.system_monitor.cpu_usage = cpu.clamp(0.0, 100.0);
        self.system_monitor.memory_usage = memory.clamp(0.0, 100.0);
        self.system_monitor.disk_usage = disk.clamp(0.0, 100.0);
    }

    /// Update network activity
    pub fn update_network_activity(&mut self, upload: f64, download: f64) {
        self.system_monitor.network_activity.upload_speed = upload;
        self.system_monitor.network_activity.download_speed = download;
        
        // Update totals (simplified)
        self.system_monitor.network_activity.total_upload += upload as u64;
        self.system_monitor.network_activity.total_download += download as u64;
    }

    /// Set startup phases
    pub fn set_startup_phases(&mut self, phases: Vec<StartupPhase>) {
        self.startup_widget.phases = phases;
        self.startup_widget.current_phase = 0;
        
        // Calculate overall progress
        let total_progress: f64 = self.startup_widget.phases
            .iter()
            .map(|phase| phase.progress)
            .sum();
        
        self.startup_widget.overall_progress = total_progress / self.startup_widget.phases.len() as f64;
    }

    /// Get current startup phase
    pub fn get_current_startup_phase(&self) -> Option<&StartupPhase> {
        self.startup_widget.phases.get(self.startup_widget.current_phase)
    }

    /// Check if startup is complete
    pub fn is_startup_complete(&self) -> bool {
        self.startup_widget.overall_progress >= 100.0
    }

    /// Toggle contacts view mode
    pub fn cycle_contacts_view(&mut self) {
        self.contacts_widget.view_mode = match self.contacts_widget.view_mode {
            ContactViewMode::Recent => ContactViewMode::Favorites,
            ContactViewMode::Favorites => ContactViewMode::All,
            ContactViewMode::All => ContactViewMode::Recent,
        };
    }

    /// Toggle calendar view mode
    pub fn cycle_calendar_view(&mut self) {
        self.calendar_widget.view_mode = match self.calendar_widget.view_mode {
            CalendarViewMode::Month => CalendarViewMode::Week,
            CalendarViewMode::Week => CalendarViewMode::Day,
            CalendarViewMode::Day => CalendarViewMode::Agenda,
            CalendarViewMode::Agenda => CalendarViewMode::Month,
        };
    }

    /// Set time format
    pub fn set_time_format(&mut self, format: TimeFormat) {
        self.clock_state.time_format = format;
    }

    /// Set date format
    pub fn set_date_format(&mut self, format: DateFormat) {
        self.clock_state.date_format = format;
    }

    /// Toggle seconds display
    pub fn toggle_seconds_display(&mut self) {
        self.clock_state.show_seconds = !self.clock_state.show_seconds;
    }

    /// Toggle timezone display
    pub fn toggle_timezone_display(&mut self) {
        self.clock_state.timezone_display = !self.clock_state.timezone_display;
    }
}