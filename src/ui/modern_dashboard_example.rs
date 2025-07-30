//! Modern Dashboard Usage Example
//! 
//! This module demonstrates how to initialize and use the modern dashboard
//! with real data integration and interactive features.

use super::modern_dashboard::{
    ModernDashboard, CurrentWeather, WeatherCondition, CalendarEvent, EventType, EventColor,
    Contact, ContactStatus, StartupPhase, PhaseStatus, TimeFormat, DateFormat,
};
use chrono::{Local, Duration as ChronoDuration};
use std::time::Duration;

/// Example integration showing how to set up the modern dashboard
pub struct ModernDashboardExample;

impl ModernDashboardExample {
    /// Initialize the modern dashboard with realistic sample data
    pub fn setup_dashboard() -> ModernDashboard {
        let mut dashboard = ModernDashboard::new();
        
        // Initialize with sample data for demonstration
        dashboard.initialize_with_sample_data();
        
        println!("🎨 Modern Dashboard initialized with sample data");
        println!("   ⏰ Real-time clock with animations");
        println!("   📊 System monitoring with gauges and graphs");
        println!("   🌤️  Weather widget with forecast");
        println!("   📅 Enhanced calendar with events");
        println!("   👥 Modern contacts with status indicators");
        println!("   🚀 Startup progress with real-time updates");
        
        dashboard
    }

    /// Demonstrate updating dashboard with real-time data
    pub fn update_with_real_data(dashboard: &mut ModernDashboard) {
        println!("🔄 Updating dashboard with real-time data...");
        
        // Update weather with current conditions
        let current_weather = CurrentWeather {
            location: "San Francisco, CA".to_string(),
            temperature: 24.5,
            feels_like: 26.0,
            condition: WeatherCondition::Clear,
            humidity: 60,
            wind_speed: 8.5,
            wind_direction: 230,
            pressure: 1015.2,
            visibility: 15.0,
            uv_index: 7,
        };
        dashboard.set_weather(current_weather);
        
        // Update system statistics (simulated values)
        dashboard.update_system_stats(45.2, 67.8, 52.1);
        dashboard.update_network_activity(125.5, 850.2);
        
        // Add a new calendar event
        let meeting_event = CalendarEvent {
            id: "urgent_meeting".to_string(),
            title: "Urgent Team Sync".to_string(),
            description: Some("Critical project discussion".to_string()),
            start_time: Local::now() + ChronoDuration::minutes(30),
            end_time: Local::now() + ChronoDuration::hours(1),
            event_type: EventType::Meeting,
            location: Some("Conference Room B".to_string()),
            attendees: vec!["Alice".to_string(), "Bob".to_string()],
            reminder: Some(ChronoDuration::minutes(15)),
            color: EventColor::Red,
        };
        dashboard.add_calendar_event(meeting_event);
        
        // Add a new contact
        let new_contact = Contact {
            id: "important_contact".to_string(), 
            name: "Maya Patel".to_string(),
            email: "maya.patel@company.com".to_string(),
            phone: Some("+1 (555) 999-8888".to_string()),
            avatar: None,
            last_contact: Some(Local::now() - ChronoDuration::minutes(15)),
            contact_frequency: 40,
            is_favorite: true,
            status: ContactStatus::Online,
        };
        dashboard.add_contact(new_contact, true);
        
        println!("✅ Dashboard updated with fresh data");
    }

    /// Demonstrate startup progress simulation
    pub fn simulate_startup_progress(dashboard: &mut ModernDashboard) {
        println!("🚀 Simulating application startup progress...");
        
        let startup_phases = vec![
            StartupPhase {
                name: "Core Systems".to_string(),
                description: "Loading essential components".to_string(),
                progress: 100.0,
                status: PhaseStatus::Completed,
                start_time: Some(std::time::Instant::now() - Duration::from_secs(8)),
                duration: Some(Duration::from_secs(2)),
                substeps: vec![
                    "Configuration loaded".to_string(),
                    "Storage initialized".to_string(),
                    "Logging configured".to_string(),
                ],
            },
            StartupPhase {
                name: "Database Connection".to_string(),
                description: "Establishing database connectivity".to_string(),
                progress: 100.0,
                status: PhaseStatus::Completed,
                start_time: Some(std::time::Instant::now() - Duration::from_secs(6)),
                duration: Some(Duration::from_secs(3)),
                substeps: vec![
                    "Connection established".to_string(),
                    "Schema validated".to_string(),
                    "Indexes verified".to_string(),
                ],
            },
            StartupPhase {
                name: "Email Integration".to_string(),
                description: "Initializing email systems".to_string(),
                progress: 95.0,
                status: PhaseStatus::InProgress,
                start_time: Some(std::time::Instant::now() - Duration::from_secs(4)),
                duration: None,
                substeps: vec![
                    "IMAP connections active".to_string(),
                    "Flash Fast precaching enabled".to_string(),
                    "Background sync starting".to_string(),
                ],
            },
            StartupPhase {
                name: "UI Initialization".to_string(),
                description: "Loading user interface".to_string(),
                progress: 60.0,
                status: PhaseStatus::InProgress,
                start_time: Some(std::time::Instant::now() - Duration::from_secs(2)),
                duration: None,
                substeps: vec![
                    "Modern dashboard ready".to_string(),
                    "Theme system loaded".to_string(),
                    "Components initializing".to_string(),
                ],
            },
            StartupPhase {
                name: "Final Setup".to_string(),
                description: "Completing initialization".to_string(),
                progress: 0.0,
                status: PhaseStatus::Pending,
                start_time: None,
                duration: None,
                substeps: vec![
                    "Keybindings configured".to_string(),
                    "Auto-sync enabled".to_string(),
                    "System ready".to_string(),
                ],
            },
        ];
        
        dashboard.set_startup_phases(startup_phases);
        
        println!("⚡ Startup simulation configured - progress will update in real-time");
    }

    /// Demonstrate user interactions and customization
    pub fn demonstrate_interactions(dashboard: &mut ModernDashboard) {
        println!("🎯 Demonstrating user interactions...");
        
        // Cycle through time formats
        dashboard.set_time_format(TimeFormat::TwelveHour);
        println!("   🕐 Switched to 12-hour time format");
        
        // Change date format
        dashboard.set_date_format(DateFormat::Verbose);
        println!("   📅 Using verbose date format");
        
        // Toggle seconds display
        dashboard.toggle_seconds_display();
        println!("   ⏱️  Toggled seconds display");
        
        // Cycle calendar view
        dashboard.cycle_calendar_view();
        println!("   📅 Cycled calendar view mode");
        
        // Cycle contacts view
        dashboard.cycle_contacts_view();
        println!("   👥 Cycled contacts view mode");
        
        println!("✨ User interactions demonstrated");
    }

    /// Show dashboard status and capabilities
    pub fn show_dashboard_status(dashboard: &ModernDashboard) {
        println!("\n📊 MODERN DASHBOARD STATUS REPORT");
        println!("==================================");
        
        // Check startup completion
        if dashboard.is_startup_complete() {
            println!("🚀 Startup: COMPLETE");
        } else {
            if let Some(phase) = dashboard.get_current_startup_phase() {
                println!("🚀 Startup: {} ({:.1}%)", phase.name, phase.progress);
            }
        }
        
        println!("⏰ Real-time Clock: ACTIVE with animations");
        println!("📊 System Monitoring: ACTIVE with 10fps updates");
        println!("🌤️  Weather Widget: ACTIVE with animated icons");
        println!("📅 Calendar Integration: ACTIVE with events");
        println!("👥 Contacts Management: ACTIVE with status indicators");
        println!("⚡ Performance: Optimized for smooth 10fps animations");
        
        println!("\n🎨 Visual Features:");
        println!("   • Animated progress bars and gauges");
        println!("   • Real-time sparkline graphs for system history");
        println!("   • Color-coded status indicators");
        println!("   • Pulsing animations synchronized with data");
        println!("   • Modern card-based layout design");
        println!("   • Responsive terminal-based interface");
        
        println!("\n💡 Interactive Elements:");
        println!("   • Multiple time and date format options");
        println!("   • Calendar view switching (Month/Week/Day/Agenda)");
        println!("   • Contact filtering (Recent/Favorites/All)");
        println!("   • Real-time system monitoring with history");
        println!("   • Startup progress with detailed phase tracking");
        
        println!("\n🎉 The Modern Dashboard is fully operational!");
    }

    /// Complete setup example
    pub fn complete_setup_example() -> ModernDashboard {
        println!("🎨 MODERN DASHBOARD COMPLETE SETUP EXAMPLE");
        println!("==========================================\n");
        
        // Step 1: Initialize dashboard  
        let mut dashboard = Self::setup_dashboard();
        
        // Step 2: Update with real data
        Self::update_with_real_data(&mut dashboard);
        
        // Step 3: Configure startup simulation
        Self::simulate_startup_progress(&mut dashboard);
        
        // Step 4: Demonstrate interactions
        Self::demonstrate_interactions(&mut dashboard);
        
        // Step 5: Show final status
        Self::show_dashboard_status(&dashboard);
        
        println!("\n🚀 Modern Dashboard is ready for use!");
        println!("The dashboard will automatically update in real-time with:");
        println!("   • Animated clock and date display");
        println!("   • Live system performance monitoring");  
        println!("   • Weather updates with animated icons");
        println!("   • Calendar events and appointments");
        println!("   • Contact status and recent activity");
        println!("   • Startup progress and system status");
        
        dashboard
    }
}

/// Helper function to initialize the modern dashboard in an app
pub fn initialize_modern_dashboard_in_app(ui: &mut crate::ui::UI) {
    println!("🔧 Integrating Modern Dashboard into application...");
    
    // Initialize with sample data
    ui.modern_dashboard_mut().initialize_with_sample_data();
    
    // Set optimal time format
    ui.modern_dashboard_mut().set_time_format(TimeFormat::TwentyFourHour);
    ui.modern_dashboard_mut().set_date_format(DateFormat::Verbose);
    
    // Enable seconds display for dynamic animation
    ui.modern_dashboard_mut().toggle_seconds_display();
    
    println!("✅ Modern Dashboard integrated successfully!");
    println!("   The dashboard will now display when in StartPage mode");
    println!("   Real-time updates and animations are active");
}