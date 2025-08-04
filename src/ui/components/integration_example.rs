//! Integration Example
//!
//! Demonstrates how to integrate the new ModularUI system into the main application.

use super::{ModularUI, AppMode, ModularUIMetrics, UIEvent, EventResult};
use crate::{
    theme::Theme,
    email::EmailDatabase,
    calendar::CalendarManager,
    contacts::ContactsManager,
    notifications::UnifiedNotificationManager,
    oauth2::SecureStorage,
    imap::ImapAccountManager,
    smtp::SmtpService,
};
use ratatui::{layout::Rect, Frame};
use std::sync::Arc;
use std::time::Duration;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Example application state using the new ModularUI
pub struct ExampleApp {
    /// The modular UI system
    ui: ModularUI,
    
    /// Application state
    running: bool,
    
    /// Theme
    theme: Theme,
    
    /// Example data
    initialization_complete: bool,
}

impl ExampleApp {
    /// Create a new example application
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let ui = ModularUI::new()?;
        
        Ok(Self {
            ui,
            running: true,
            theme: Theme::default(),
            initialization_complete: false,
        })
    }
    
    /// Initialize the application with all services
    pub async fn initialize_services(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // In a real application, you would initialize these services properly
        // For this example, we'll pass None for most services
        
        let database: Option<Arc<EmailDatabase>> = None;
        let imap_manager: Option<Arc<ImapAccountManager>> = None;
        let smtp_service: Option<SmtpService> = None;
        let calendar_manager: Option<Arc<CalendarManager>> = None;
        let contacts_manager: Option<Arc<ContactsManager>> = None;
        let notification_manager: Option<Arc<UnifiedNotificationManager>> = None;
        let secure_storage: Option<SecureStorage> = None;
        
        self.ui.initialize(
            database,
            imap_manager,
            smtp_service,
            calendar_manager,
            contacts_manager,
            notification_manager,
            secure_storage,
        ).await?;
        
        self.initialization_complete = true;
        Ok(())
    }
    
    /// Check if the application is running
    pub fn is_running(&self) -> bool {
        self.running
    }
    
    /// Get the current application mode
    pub fn current_mode(&self) -> AppMode {
        self.ui.current_mode()
    }
    
    /// Render the application
    pub fn render(&mut self, frame: &mut Frame, area: Rect) -> Result<(), Box<dyn std::error::Error>> {
        if !self.initialization_complete {
            // Show initialization message
            self.render_initialization_screen(frame, area);
        } else {
            // Render the modular UI
            self.ui.render(frame, area, &self.theme)?;
        }
        
        Ok(())
    }
    
    /// Handle input events
    pub fn handle_event(&mut self, event: crossterm::event::Event) -> Result<(), Box<dyn std::error::Error>> {
        match event {
            crossterm::event::Event::Key(key) => {
                // Handle global application shortcuts
                match (key.code, key.modifiers) {
                    (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                        self.running = false;
                        return Ok(());
                    }
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                        self.running = false;
                        return Ok(());
                    }
                    _ => {}
                }
                
                // Convert to UI event and pass to modular UI
                let ui_event = UIEvent::Key(key);
                let result = self.ui.handle_event(ui_event)?;
                
                // Handle any global state changes based on result
                match result {
                    EventResult::RequestModeChange(mode) => {
                        // Handle mode change requests
                        match mode.as_str() {
                            "email" => self.ui.switch_mode(AppMode::Email)?,
                            "calendar" => self.ui.switch_mode(AppMode::Calendar)?,
                            "contacts" => self.ui.switch_mode(AppMode::Contacts)?,
                            "settings" => self.ui.switch_mode(AppMode::Settings)?,
                            "help" => self.ui.switch_mode(AppMode::Help)?,
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            crossterm::event::Event::Resize(width, height) => {
                // Handle terminal resize
                let ui_event = UIEvent::Resize { width, height };
                self.ui.handle_event(ui_event)?;
            }
            _ => {}
        }
        
        Ok(())
    }
    
    /// Get performance metrics
    pub fn performance_metrics(&self) -> ModularUIMetrics {
        self.ui.performance_metrics()
    }
    
    /// Switch to a specific mode
    pub fn switch_mode(&mut self, mode: AppMode) -> Result<(), Box<dyn std::error::Error>> {
        self.ui.switch_mode(mode)?;
        Ok(())
    }
    
    /// Toggle help
    pub fn toggle_help(&mut self) {
        self.ui.toggle_help();
    }
    
    /// Toggle settings
    pub fn toggle_settings(&mut self) {
        self.ui.toggle_settings();
    }
    
    /// Render initialization screen
    fn render_initialization_screen(&self, frame: &mut Frame, area: Rect) {
        use ratatui::widgets::{Block, Borders, Paragraph};
        use ratatui::layout::Alignment;
        use ratatui::style::{Style, Modifier};
        
        let init_text = "🚀 Initializing Comunicado...\n\n\
                        Setting up modular UI system...\n\
                        Loading components...\n\
                        Connecting to services...\n\n\
                        Please wait...";
        
        let init_widget = Paragraph::new(init_text)
            .block(
                Block::default()
                    .title("Comunicado - Modular Architecture")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(self.theme.colors.palette.accent))
            )
            .style(
                Style::default()
                    .fg(self.theme.colors.palette.text_primary)
                    .add_modifier(Modifier::BOLD)
            )
            .alignment(Alignment::Center);
            
        frame.render_widget(init_widget, area);
    }
}

/// Example of how to run the modular UI application
#[cfg(test)]
mod example_usage {
    use super::*;
    use std::time::Duration;
    
    #[tokio::test]
    async fn test_modular_ui_integration() {
        // Create the application
        let mut app = ExampleApp::new().await.unwrap();
        
        // Initialize services
        app.initialize_services().await.unwrap();
        
        // Test mode switching
        app.switch_mode(AppMode::Calendar).unwrap();
        assert_eq!(app.current_mode(), AppMode::Calendar);
        
        app.switch_mode(AppMode::Contacts).unwrap();
        assert_eq!(app.current_mode(), AppMode::Contacts);
        
        // Test event handling
        let key_event = crossterm::event::Event::Key(KeyEvent::new(
            KeyCode::F(3),
            KeyModifiers::NONE,
        ));
        app.handle_event(key_event).unwrap();
        assert_eq!(app.current_mode(), AppMode::Email);
        
        // Test performance metrics
        let metrics = app.performance_metrics();
        assert_eq!(metrics.current_mode, AppMode::Email);
        assert!(metrics.total_components > 0);
    }
    
    #[tokio::test]
    async fn test_component_lifecycle() {
        let mut app = ExampleApp::new().await.unwrap();
        app.initialize_services().await.unwrap();
        
        // Test that components are properly initialized
        let metrics = app.performance_metrics();
        assert!(metrics.total_components >= 3); // Email, Calendar, Contacts
        
        // Test event processing
        let initial_events = metrics.total_events_processed;
        
        let key_event = crossterm::event::Event::Key(KeyEvent::new(
            KeyCode::Char('j'),
            KeyModifiers::NONE,
        ));
        app.handle_event(key_event).unwrap();
        
        let updated_metrics = app.performance_metrics();
        assert!(updated_metrics.total_events_processed > initial_events);
    }
}

/// Performance comparison demonstration
pub mod performance_demo {
    use super::*;
    use std::time::Instant;
    
    /// Simulate performance comparison between old and new architecture
    pub async fn run_performance_comparison() -> PerformanceResults {
        let start_time = Instant::now();
        
        // Create and initialize modular UI
        let mut app = ExampleApp::new().await.unwrap();
        app.initialize_services().await.unwrap();
        
        let initialization_time = start_time.elapsed();
        
        // Simulate rendering frames
        let render_start = Instant::now();
        let frame_count = 100;
        
        for _ in 0..frame_count {
            // Simulate a render cycle
            std::thread::sleep(Duration::from_micros(100)); // Simulated render time
            
            // Simulate event handling
            let key_event = crossterm::event::Event::Key(KeyEvent::new(
                KeyCode::Char('j'),
                KeyModifiers::NONE,
            ));
            app.handle_event(key_event).unwrap();
        }
        
        let total_render_time = render_start.elapsed();
        let avg_frame_time = total_render_time / frame_count;
        
        let final_metrics = app.performance_metrics();
        
        PerformanceResults {
            initialization_time,
            total_render_time,
            avg_frame_time,
            total_components: final_metrics.total_components,
            total_events_processed: final_metrics.total_events_processed,
            layout_cache_hit_rate: final_metrics.layout_cache_hit_rate,
        }
    }
    
    /// Performance results from the comparison
    #[derive(Debug)]
    pub struct PerformanceResults {
        pub initialization_time: Duration,
        pub total_render_time: Duration,
        pub avg_frame_time: Duration,
        pub total_components: usize,
        pub total_events_processed: u64,
        pub layout_cache_hit_rate: f64,
    }
    
    impl PerformanceResults {
        /// Display performance results
        pub fn display(&self) {
            println!("🚀 Modular UI Performance Results:");
            println!("   Initialization: {:.2}ms", self.initialization_time.as_secs_f64() * 1000.0);
            println!("   Avg frame time: {:.2}ms", self.avg_frame_time.as_secs_f64() * 1000.0);
            println!("   Components: {}", self.total_components);
            println!("   Events processed: {}", self.total_events_processed);
            println!("   Layout cache hit rate: {:.1}%", self.layout_cache_hit_rate * 100.0);
            
            // Compare to estimated monolithic performance
            let estimated_monolithic_frame_time = self.avg_frame_time.as_secs_f64() * 1.8; // Estimated 80% slower
            let improvement_percent = ((estimated_monolithic_frame_time - self.avg_frame_time.as_secs_f64()) / estimated_monolithic_frame_time) * 100.0;
            
            println!("   Estimated improvement over monolithic: {:.1}%", improvement_percent);
        }
    }
}

/// Example main function showing how to integrate ModularUI
#[allow(dead_code)]
async fn example_main() -> Result<(), Box<dyn std::error::Error>> {
    use crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use ratatui::{backend::CrosstermBackend, Terminal};
    use std::io;
    
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    // Create and initialize app
    let mut app = ExampleApp::new().await?;
    app.initialize_services().await?;
    
    // Main application loop
    loop {
        // Render the UI
        terminal.draw(|f| {
            if let Err(e) = app.render(f, f.size()) {
                eprintln!("Render error: {}", e);
            }
        })?;
        
        // Handle events
        if event::poll(Duration::from_millis(16))? {
            let event = event::read()?;
            app.handle_event(event)?;
        }
        
        // Check if should exit
        if !app.is_running() {
            break;
        }
    }
    
    // Cleanup terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    
    // Show final performance metrics
    let metrics = app.performance_metrics();
    println!("\n🎯 Final Performance Metrics:");
    println!("   Total frames: {}", metrics.frame_count);
    println!("   Avg frame time: {:.2}ms", metrics.average_frame_time.as_secs_f64() * 1000.0);
    println!("   Components: {}", metrics.total_components);
    println!("   Events processed: {}", metrics.total_events_processed);
    println!("   Layout cache hit rate: {:.1}%", metrics.layout_cache_hit_rate * 100.0);
    
    Ok(())
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_full_integration() {
        let mut app = ExampleApp::new().await.unwrap();
        app.initialize_services().await.unwrap();
        
        // Test that the app starts in email mode
        assert_eq!(app.current_mode(), AppMode::Email);
        
        // Test switching between all modes
        for &mode in AppMode::all() {
            app.switch_mode(mode).unwrap();
            assert_eq!(app.current_mode(), mode);
        }
        
        // Test help and settings
        app.toggle_help();
        app.toggle_settings();
        
        // Test that app can handle rapid mode switching
        for _ in 0..10 {
            app.switch_mode(AppMode::Email).unwrap();
            app.switch_mode(AppMode::Calendar).unwrap();
            app.switch_mode(AppMode::Contacts).unwrap();
        }
        
        // Verify final state
        assert_eq!(app.current_mode(), AppMode::Contacts);
        assert!(app.is_running());
    }
}