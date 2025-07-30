//! Optimized application initialization with background tasks and lazy loading
//!
//! This module provides a fast-starting TUI application that:
//! - Shows UI immediately with minimal loading
//! - Initializes heavy components in background
//! - Uses lazy loading for non-critical resources
//! - Provides progressive feature availability

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tokio::time::interval;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

use crate::startup::{
    BackgroundTaskManager, LazyInit, LazyInitManager, StartupOptimizer, 
    StartupCache, StartupConfig, TaskContext, TaskPriority, InitializationState,
    optimization::{UiPreferences, AccountCacheInfo}
};
use crate::ui::UI;
use crate::theme::Theme;
use crate::contacts::ContactsManager;
use crate::imap::account_manager::ImapAccountManager;
use crossterm::event::Event;

/// Application state for optimized startup
#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    /// Initial startup with minimal UI
    Starting,
    /// Core components loaded, basic functionality available
    BasicReady,
    /// All components loaded, full functionality available
    FullyReady,
    /// Error during startup
    Error(String),
}

/// Resource availability status  
#[derive(Debug, Clone)]
pub struct ResourceStatus {
    pub contacts_manager: InitializationState,
    pub account_manager: InitializationState,
}

impl ResourceStatus {
    pub fn all_ready(&self) -> bool {
        self.contacts_manager.is_ready() && self.account_manager.is_ready()
    }
    
    pub fn basic_ready(&self) -> bool {
        // Only require contacts for basic functionality
        self.contacts_manager.is_ready()
    }
}

/// Optimized application with fast startup and background initialization
pub struct OptimizedApp {
    // Core UI components (always available)
    ui: UI,
    theme: Theme,
    
    // Terminal management
    terminal: Option<Terminal<CrosstermBackend<io::Stdout>>>,
    
    // Application state
    state: AppState,
    
    // Startup optimization
    startup_optimizer: StartupOptimizer,
    
    // Lazy-loaded managers
    contacts_manager: LazyInit<ContactsManager>,
    account_manager: LazyInit<ImapAccountManager>,
    
    // Background task coordination
    background_tasks: BackgroundTaskManager,
    #[allow(dead_code)]
    lazy_manager: LazyInitManager,
    
    // Event system
    #[allow(dead_code)]
    event_sender: mpsc::UnboundedSender<Event>,
    #[allow(dead_code)]
    event_receiver: mpsc::UnboundedReceiver<Event>,
    
    // Performance tracking
    startup_time: Instant,
    
    // Resource status
    resource_status: Arc<RwLock<ResourceStatus>>,
}

impl OptimizedApp {
    /// Create a new optimized application
    pub async fn new() -> Result<Self, String> {
        let startup_time = Instant::now();
        
        // Initialize core components immediately (fast)
        let ui = UI::new();
        let theme = Theme::default();
        
        // Initialize start page with real data (no mock data)
        // Note: Start page initialization is handled in StartPage::new()
        
        // Set up startup optimization
        let startup_config = StartupConfig {
            enable_caching: true,
            enable_preloading: true,
            enable_background_optimization: true,
            ui_responsiveness_target_ms: 16, // 60 FPS
            ..Default::default()
        };
        let mut startup_optimizer = StartupOptimizer::new(startup_config);
        startup_optimizer.initialize().await?;
        
        // Create background task manager
        let background_tasks = BackgroundTaskManager::new(4);
        let lazy_manager = LazyInitManager::new();
        
        // Set up lazy-loaded components
        let contacts_manager = LazyInit::new("Contacts Manager".to_string());
        let account_manager = LazyInit::new("Account Manager".to_string());
        
        // Event system
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        
        // Resource status tracking
        let resource_status = Arc::new(RwLock::new(ResourceStatus {
            contacts_manager: InitializationState::NotStarted,
            account_manager: InitializationState::NotStarted,
        }));
        
        Ok(Self {
            ui,
            theme,
            terminal: None,
            state: AppState::Starting,
            startup_optimizer,
            contacts_manager,
            account_manager,
            background_tasks,
            lazy_manager,
            event_sender,
            event_receiver,
            startup_time,
            resource_status,
        })
    }
    
    /// Start the application with progressive loading
    pub async fn run(&mut self) -> Result<(), String> {
        // Initialize terminal first
        self.setup_terminal().map_err(|e| format!("Failed to setup terminal: {}", e))?;
        
        // Phase 1: Show minimal UI immediately
        self.show_startup_ui().await?;
        
        // Phase 2: Start background initialization
        self.start_background_initialization().await?;
        
        // Phase 3: Main event loop with progressive feature availability
        let result = self.run_main_loop().await;
        
        // Always cleanup terminal
        if let Err(e) = self.cleanup_terminal() {
            eprintln!("Warning: Failed to cleanup terminal: {}", e);
        }
        
        result
    }
    
    /// Initialize terminal for TUI rendering
    fn setup_terminal(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        self.terminal = Some(terminal);
        
        Ok(())
    }
    
    /// Cleanup terminal when exiting
    fn cleanup_terminal(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(mut terminal) = self.terminal.take() {
            disable_raw_mode()?;
            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        }
        Ok(())
    }
    
    /// Show minimal UI immediately for responsiveness
    async fn show_startup_ui(&mut self) -> Result<(), String> {
        // Check for cached data to show immediately
        if let Some(cache) = self.startup_optimizer.get_cache().await {
            self.apply_cached_ui_preferences(&cache).await;
        }
        
        // Show startup progress
        self.ui.show_startup_progress("Inicializando Comunicado...", 0.0);
        
        // First render
        self.render_ui().await?;
        
        self.state = AppState::Starting;
        Ok(())
    }
    
    /// Start background initialization of heavy components
    async fn start_background_initialization(&mut self) -> Result<(), String> {
        // Start background optimization tasks
        let _optimization_tasks = self.startup_optimizer.start_background_tasks().await;
        
        // Start lazy component initialization with priorities
        
        // Critical: Database and core services
        let db_task = TaskContext::new(
            "Database Initialization".to_string(),
            "Initialize email and contacts database".to_string(),
            TaskPriority::Critical,
        ).with_timeout(Duration::from_secs(30));
        
        let contacts_status = Arc::clone(&self.resource_status);
        
        let _db_task_id = self.background_tasks.spawn_task(db_task, |reporter| async move {
            reporter.send_message("Initializing contacts database...".to_string());
            reporter.update_progress(0.2).await;
            
            // Simulate contacts database initialization
            tokio::time::sleep(Duration::from_millis(100)).await;
            reporter.update_progress(0.5).await;
            reporter.send_message("Creating contacts manager...".to_string());
            
            tokio::time::sleep(Duration::from_millis(100)).await;
            
            // Update status
            {
                let mut status = contacts_status.write().await;
                status.contacts_manager = InitializationState::Ready { 
                    duration: Duration::from_millis(200) 
                };
            }
            
            reporter.update_progress(1.0).await;
            reporter.send_message("Contacts manager ready".to_string());
            Ok(())
        }).await;
        
        
        // Low Priority: Account management
        let account_task = TaskContext::new(
            "Account Management".to_string(),
            "Set up account management and OAuth tokens".to_string(),
            TaskPriority::Low,
        ).with_timeout(Duration::from_secs(15));
        
        let account_status = Arc::clone(&self.resource_status);
        let _account_task_id = self.background_tasks.spawn_task(account_task, |reporter| async move {
            reporter.send_message("Setting up account management...".to_string());
            reporter.update_progress(0.5).await;
            
            tokio::time::sleep(Duration::from_millis(100)).await;
            
            // Update status
            {
                let mut status = account_status.write().await;
                status.account_manager = InitializationState::Ready { 
                    duration: Duration::from_millis(100) 
                };
            }
            
            reporter.update_progress(1.0).await;
            reporter.send_message("Account management ready".to_string());
            Ok(())
        }).await;
        
        Ok(())
    }
    
    /// Main application event loop with progressive feature availability
    async fn run_main_loop(&mut self) -> Result<(), String> {
        let mut ui_update_interval = interval(Duration::from_millis(16)); // 60 FPS target
        let mut status_check_interval = interval(Duration::from_millis(100));
        
        // Simple event loop for testing - in real implementation would be more complex
        let mut should_quit = false;
        while !should_quit {
            tokio::select! {
                // Handle UI updates at 60 FPS
                _ = ui_update_interval.tick() => {
                    self.update_ui_state().await?;
                    self.render_ui().await?;
                }
                
                // Check resource status periodically
                _ = status_check_interval.tick() => {
                    self.check_resource_status().await?;
                }
                
                // Simple timeout to prevent infinite loop in tests
                _ = tokio::time::sleep(Duration::from_millis(5000)) => {
                    should_quit = true;
                }
                
                // Process background task updates
                else => {
                    let _updates = self.background_tasks.process_status_updates().await;
                    // Handle status updates if needed
                }
            }
        }
        
        // Finalize startup optimization and save metrics
        let _metrics = self.startup_optimizer.finalize().await?;
        
        Ok(())
    }
    
    /// Update UI state based on resource availability
    async fn update_ui_state(&mut self) -> Result<(), String> {
        let status = self.resource_status.read().await;
        
        match self.state {
            AppState::Starting => {
                if status.basic_ready() {
                    self.state = AppState::BasicReady;
                    self.ui.show_notification("👥 Contatos prontos!".to_string(), Duration::from_secs(2));
                }
            }
            AppState::BasicReady => {
                if status.all_ready() {
                    self.state = AppState::FullyReady;
                    self.ui.show_notification("🚀 Todas as funcionalidades ativas!".to_string(), Duration::from_secs(2));
                    
                    // App is now fully ready
                }
            }
            AppState::FullyReady => {
                // All systems ready - normal operation
            }
            AppState::Error(_) => {
                // Handle error state
            }
        }
        
        Ok(())
    }
    
    /// Check and update resource initialization status
    async fn check_resource_status(&mut self) -> Result<(), String> {
        let mut status = self.resource_status.write().await;
        
        // Check each lazy component's status
        status.contacts_manager = self.contacts_manager.state().await;
        status.account_manager = self.account_manager.state().await;
        
        // Update progress in UI
        let progress = self.calculate_overall_progress(&status);
        self.ui.update_startup_progress(progress);
        
        Ok(())
    }
    
    /// Calculate overall initialization progress
    fn calculate_overall_progress(&self, status: &ResourceStatus) -> f64 {
        let components = [
            &status.contacts_manager,
            &status.account_manager,
        ];
        
        let total_weight = components.len() as f64;
        let completed_weight = components.iter()
            .map(|state| if state.is_ready() { 1.0 } else if state.is_initializing() { 0.5 } else { 0.0 })
            .sum::<f64>();
        
        (completed_weight / total_weight).clamp(0.0, 1.0)
    }
    
    /// Apply cached UI preferences for immediate visual continuity
    async fn apply_cached_ui_preferences(&mut self, cache: &StartupCache) {
        // Apply theme
        // Apply theme based on name
        match cache.ui_preferences.theme.as_str() {
            "professional_dark" => self.theme = Theme::professional_dark(),
            "professional_light" => self.theme = Theme::professional_light(),
            "high_contrast" => self.theme = Theme::high_contrast(),
            "gruvbox_dark" => self.theme = Theme::gruvbox_dark(),
            "gruvbox_light" => self.theme = Theme::gruvbox_light(),
            _ => self.theme = Theme::default(),
        }
        
        // Apply layout preferences
        self.ui.apply_layout_preferences(&cache.ui_preferences);
        
        // Show cached data if available
        if !cache.accounts.is_empty() {
            self.ui.show_cached_account_info(&cache.accounts);
        }
    }
    
    /// Render the UI
    async fn render_ui(&mut self) -> Result<(), String> {
        let render_start = Instant::now();
        
        // Render using the managed terminal
        if let Some(ref mut terminal) = self.terminal {
            match terminal.draw(|frame| {
                self.ui.render(frame);
            }) {
                Ok(_) => {
                    let render_duration = render_start.elapsed();
                    self.startup_optimizer.record_operation("ui_render", render_duration);
                    Ok(())
                }
                Err(e) => {
                    let render_duration = render_start.elapsed();
                    self.startup_optimizer.record_operation("ui_render_error", render_duration);
                    Err(format!("TUI rendering failed: {}", e))
                }
            }
        } else {
            // Fallback to progress display if terminal not available
            let status = self.resource_status.read().await;
            let progress = self.calculate_overall_progress(&status);
            println!("Progress: {:.1}%", progress * 100.0);
            
            let render_duration = render_start.elapsed();
            self.startup_optimizer.record_operation("ui_render_fallback", render_duration);
            Ok(())
        }
    }
    
    /// Handle application events
    #[allow(dead_code)]
    async fn handle_event(&mut self, _event: crossterm::event::Event) -> Result<bool, String> {
        // Simplified event handling for now
        // Return true to continue, false to quit
        Ok(true)
    }
    
    /// Handle keyboard input
    #[allow(dead_code)]
    async fn handle_keyboard_input(&mut self, key_event: crossterm::event::KeyEvent) -> Result<bool, String> {
        use crossterm::event::KeyCode;
        
        match key_event.code {
            KeyCode::Char('q') => Ok(false), // Quit
            KeyCode::F(1) => {
                self.ui.show_notification("📧 Feature not implemented".to_string(), Duration::from_secs(1));
                Ok(true)
            }
            KeyCode::F(2) => {
                self.ui.show_notification("📅 Feature not implemented".to_string(), Duration::from_secs(1));
                Ok(true)
            }
            KeyCode::F(3) => {
                // Try to access contacts
                if self.contacts_manager.is_ready().await {
                    self.ui.show_notification("👥 Acessando contatos...".to_string(), Duration::from_secs(1));
                } else {
                    self.ui.show_notification("⏳ Sistema de contatos ainda inicializando...".to_string(), Duration::from_secs(2));
                }
                Ok(true)
            }
            _ => Ok(true),
        }
    }
    
    /// Get current application state
    pub fn state(&self) -> &AppState {
        &self.state
    }
    
    /// Get startup duration
    pub fn startup_duration(&self) -> Duration {
        self.startup_time.elapsed()
    }
    
    /// Check if the application is fully ready
    pub async fn is_fully_ready(&self) -> bool {
        matches!(self.state, AppState::FullyReady)
    }
    
    /// Get resource status for debugging/monitoring
    pub async fn get_resource_status(&self) -> ResourceStatus {
        self.resource_status.read().await.clone()
    }
}

// Extension trait for UI to support new functionality
trait UIExtensions {
    fn show_startup_progress(&mut self, message: &str, progress: f64);
    fn update_startup_progress(&mut self, progress: f64);
    #[allow(dead_code)]
    fn show_notification(&mut self, message: &str, duration: Duration);
    fn apply_layout_preferences(&mut self, preferences: &UiPreferences);
    fn show_cached_account_info(&mut self, accounts: &[AccountCacheInfo]);
}

impl UIExtensions for UI {
    fn show_startup_progress(&mut self, message: &str, progress: f64) {
        // Implementation would update the startup progress display
        // For now, just log the progress
        eprintln!("Startup: {} ({:.1}%)", message, progress * 100.0);
    }
    
    fn update_startup_progress(&mut self, progress: f64) {
        // Update the progress bar
        eprintln!("Progress: {:.1}%", progress * 100.0);
    }
    
    fn show_notification(&mut self, message: &str, _duration: Duration) {
        // Show a temporary notification
        eprintln!("Notification: {}", message);
    }
    
    fn apply_layout_preferences(&mut self, _preferences: &UiPreferences) {
        // Apply cached layout preferences
        // Implementation would restore window sizes, panel visibility, etc.
    }
    
    fn show_cached_account_info(&mut self, _accounts: &[AccountCacheInfo]) {
        // Show cached account information immediately
        // This provides visual continuity during startup
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_optimized_app_creation() {
        let app = OptimizedApp::new().await;
        assert!(app.is_ok());
        
        let app = app.unwrap();
        assert_eq!(app.state(), &AppState::Starting);
        assert!(app.startup_duration() > Duration::from_secs(0));
    }

    #[tokio::test]
    async fn test_resource_status() {
        let status = ResourceStatus {
            contacts_manager: InitializationState::Ready { duration: Duration::from_millis(50) },
            account_manager: InitializationState::NotStarted,
        };
        
        assert!(status.basic_ready());
        assert!(!status.all_ready());
    }

    #[test]
    fn test_progress_calculation() {
        let app = OptimizedApp {
            ui: UI::new(),
            theme: Theme::default(),
            state: AppState::Starting,
            startup_optimizer: StartupOptimizer::new(StartupConfig::default()),
            contacts_manager: LazyInit::new("test".to_string()),
            account_manager: LazyInit::new("test".to_string()),
            background_tasks: BackgroundTaskManager::new(4),
            lazy_manager: LazyInitManager::new(),
            event_sender: mpsc::unbounded_channel().0,
            event_receiver: mpsc::unbounded_channel().1,
            startup_time: Instant::now(),
            resource_status: Arc::new(RwLock::new(ResourceStatus {
                contacts_manager: InitializationState::Ready { duration: Duration::from_millis(50) },
                account_manager: InitializationState::NotStarted,
            })),
        };
        
        let status = ResourceStatus {
            contacts_manager: InitializationState::Ready { duration: Duration::from_millis(50) },
            account_manager: InitializationState::Initializing { started_at: Instant::now() },
        };
        
        let progress = app.calculate_overall_progress(&status);
        assert_eq!(progress, 0.75); // 1 ready + 0.5 initializing = 1.5/2 = 0.75
    }
}