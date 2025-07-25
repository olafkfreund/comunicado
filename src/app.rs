use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io;
use std::sync::Arc;
use tokio::time::{Duration, Instant};

use crate::events::EventHandler;
use crate::ui::UI;
use crate::email::{EmailDatabase, EmailNotificationManager};

pub struct App {
    should_quit: bool,
    ui: UI,
    event_handler: EventHandler,
    database: Option<Arc<EmailDatabase>>,
    notification_manager: Option<Arc<EmailNotificationManager>>,
}

impl App {
    pub fn new() -> Self {
        Self {
            should_quit: false,
            ui: UI::new(),
            event_handler: EventHandler::new(),
            database: None,
            notification_manager: None,
        }
    }
    
    /// Initialize the database connection
    pub async fn initialize_database(&mut self) -> Result<()> {
        // Create database path in user's data directory
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("comunicado");
        
        // Create directory if it doesn't exist
        std::fs::create_dir_all(&data_dir)?;
        
        let db_path = data_dir.join("messages.db");
        let db_path_str = db_path.to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid database path"))?;
        
        // Create database connection
        let database = EmailDatabase::new(db_path_str).await
            .map_err(|e| anyhow::anyhow!("Failed to initialize database: {}", e))?;
        
        let database_arc = Arc::new(database);
        
        // Create notification manager
        let notification_manager = Arc::new(EmailNotificationManager::new(database_arc.clone()));
        
        // Start the notification processing
        notification_manager.start().await;
        
        // Set database and notification manager in UI
        self.ui.set_database(database_arc.clone());
        self.ui.set_notification_manager(notification_manager.clone());
        
        self.database = Some(database_arc);
        self.notification_manager = Some(notification_manager);
        
        Ok(())
    }
    
    /// Load sample data for demonstration
    pub async fn load_sample_data(&mut self) -> Result<()> {
        if let Some(ref database) = self.database {
            // Create sample account and folder if they don't exist
            self.create_sample_account_and_folder(database).await?;
            
            // Try to load messages from database
            let _ = self.ui.load_messages("sample-account".to_string(), "INBOX".to_string()).await;
        }
        Ok(())
    }
    
    /// Create sample account and folder for demonstration
    async fn create_sample_account_and_folder(&self, database: &EmailDatabase) -> Result<()> {
        
        // Check if sample account exists
        let account_exists = sqlx::query("SELECT id FROM accounts WHERE id = ?")
            .bind("sample-account")
            .fetch_optional(&database.pool)
            .await?
            .is_some();
            
        if !account_exists {
            // Create sample account
            sqlx::query("INSERT INTO accounts (id, name, email, provider, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
                .bind("sample-account")
                .bind("Sample Account")
                .bind("user@example.com")
                .bind("sample")
                .bind(chrono::Utc::now().to_rfc3339())
                .bind(chrono::Utc::now().to_rfc3339())
                .execute(&database.pool)
                .await?;
                
            // Create INBOX folder
            sqlx::query("INSERT INTO folders (account_id, name, full_name, delimiter, attributes, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)")
                .bind("sample-account")
                .bind("INBOX")
                .bind("INBOX")
                .bind(".")
                .bind("[]")
                .bind(chrono::Utc::now().to_rfc3339())
                .bind(chrono::Utc::now().to_rfc3339())
                .execute(&database.pool)
                .await?;
        }
        
        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Run the main loop
        let result = self.run_loop(&mut terminal).await;

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        result
    }

    async fn run_loop(&mut self, terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<()> {
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(50);
        let mut previous_selection: Option<usize> = None;

        loop {
            // Process email notifications
            self.ui.process_notifications().await;
            
            // Check if message selection changed and handle it
            let current_selection = self.ui.message_list().get_selection_state();
            if current_selection != previous_selection {
                self.ui.handle_message_selection().await;
                previous_selection = current_selection;
            }

            // Draw UI
            terminal.draw(|f| self.ui.render(f))?;

            // Handle events
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    self.event_handler.handle_key_event(key, &mut self.ui);
                    
                    // Check for quit command
                    if self.event_handler.should_quit() {
                        self.should_quit = true;
                    }
                }
            }

            if last_tick.elapsed() >= tick_rate {
                last_tick = Instant::now();
            }

            if self.should_quit {
                break;
            }
        }

        Ok(())
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}