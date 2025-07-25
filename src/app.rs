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
use crate::oauth2::{SetupWizard, SecureStorage, AccountConfig, TokenManager};
use crate::imap::ImapAccountManager;

pub struct App {
    should_quit: bool,
    ui: UI,
    event_handler: EventHandler,
    database: Option<Arc<EmailDatabase>>,
    notification_manager: Option<Arc<EmailNotificationManager>>,
    storage: SecureStorage,
    imap_manager: Option<ImapAccountManager>,
    token_manager: Option<TokenManager>,
}

impl App {
    pub fn new() -> Result<Self> {
        Ok(Self {
            should_quit: false,
            ui: UI::new(),
            event_handler: EventHandler::new(),
            database: None,
            notification_manager: None,
            storage: SecureStorage::new("comunicado".to_string())
                .map_err(|e| anyhow::anyhow!("Failed to initialize secure storage: {}", e))?,
            imap_manager: None,
            token_manager: None,
        })
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
    
    /// Initialize IMAP account manager with OAuth2 support
    pub async fn initialize_imap_manager(&mut self) -> Result<()> {
        // Create token manager for OAuth2 authentication
        let token_manager = TokenManager::new();
        
        // Create IMAP account manager with OAuth2 support
        let mut imap_manager = ImapAccountManager::new_with_oauth2(token_manager.clone())
            .map_err(|e| anyhow::anyhow!("Failed to create IMAP account manager: {}", e))?;
        
        // Load existing accounts from OAuth2 storage
        imap_manager.load_accounts().await
            .map_err(|e| anyhow::anyhow!("Failed to load IMAP accounts: {}", e))?;
        
        self.token_manager = Some(token_manager);
        self.imap_manager = Some(imap_manager);
        
        Ok(())
    }
    
    /// Check for existing accounts and run setup wizard if needed
    pub async fn check_accounts_and_setup(&mut self) -> Result<()> {
        let account_ids = self.storage.list_account_ids()
            .map_err(|e| anyhow::anyhow!("Failed to list accounts: {}", e))?;
        
        if account_ids.is_empty() {
            // No accounts found, run setup wizard
            self.run_setup_wizard().await?;
        } else {
            // Load existing accounts
            self.load_existing_accounts().await?;
        }
        
        Ok(())
    }
    
    /// Run the OAuth2 setup wizard
    async fn run_setup_wizard(&mut self) -> Result<()> {
        let mut wizard = SetupWizard::new()
            .map_err(|e| anyhow::anyhow!("Failed to create setup wizard: {}", e))?;
        
        if let Some(account_config) = wizard.run().await
            .map_err(|e| anyhow::anyhow!("Setup wizard failed: {}", e))? {
            
            // Store the account configuration in database
            self.create_account_from_config(&account_config).await?;
            
            println!("Account setup completed successfully!");
            println!("Account: {} ({})", account_config.display_name, account_config.email_address);
        } else {
            return Err(anyhow::anyhow!("Account setup was cancelled"));
        }
        
        Ok(())
    }
    
    /// Load existing accounts from storage
    async fn load_existing_accounts(&mut self) -> Result<()> {
        let accounts = self.storage.load_all_accounts()
            .map_err(|e| anyhow::anyhow!("Failed to load accounts: {}", e))?;
        
        if accounts.is_empty() {
            return self.load_sample_data().await;
        }
        
        // For now, load the first account
        if let Some(account) = accounts.first() {
            self.create_account_from_config(account).await?;
            
            // Try to fetch real messages from IMAP
            if let Some(ref mut _imap_manager) = self.imap_manager {
                match self.fetch_messages_from_imap(&account.account_id, "INBOX").await {
                    Ok(_) => {
                        // Successfully fetched messages via IMAP
                        let _ = self.ui.load_messages(account.account_id.clone(), "INBOX".to_string()).await;
                    }
                    Err(e) => {
                        tracing::warn!("Failed to fetch messages via IMAP: {}, falling back to sample data", e);
                        // Fall back to sample data if IMAP fails
                        let _ = self.load_sample_data().await;
                    }
                }
            } else {
                // Fall back to sample data if IMAP manager not initialized
                let _ = self.load_sample_data().await;
            }
        }
        
        Ok(())
    }
    
    /// Create account and folder in database from OAuth2 config
    async fn create_account_from_config(&self, config: &AccountConfig) -> Result<()> {
        if let Some(ref database) = self.database {
            // Check if account already exists
            let account_exists = sqlx::query("SELECT id FROM accounts WHERE id = ?")
                .bind(&config.account_id)
                .fetch_optional(&database.pool)
                .await?
                .is_some();
            
            if !account_exists {
                // Create account
                sqlx::query("INSERT INTO accounts (id, name, email, provider, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
                    .bind(&config.account_id)
                    .bind(&config.display_name)
                    .bind(&config.email_address)
                    .bind(&config.provider)
                    .bind(chrono::Utc::now().to_rfc3339())
                    .bind(chrono::Utc::now().to_rfc3339())
                    .execute(&database.pool)
                    .await?;
                
                // Create INBOX folder (always create this for email accounts)
                sqlx::query("INSERT INTO folders (account_id, name, full_name, delimiter, attributes, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)")
                    .bind(&config.account_id)
                    .bind("INBOX")
                    .bind("INBOX")
                    .bind(".")
                    .bind("[]")
                    .bind(chrono::Utc::now().to_rfc3339())
                    .bind(chrono::Utc::now().to_rfc3339())
                    .execute(&database.pool)
                    .await?;
            }
        }
        
        Ok(())
    }
    
    /// Load sample data for demonstration (fallback)
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
    
    /// Fetch messages from IMAP and store in database
    async fn fetch_messages_from_imap(&mut self, account_id: &str, folder_name: &str) -> Result<()> {
        let imap_manager = self.imap_manager.as_mut()
            .ok_or_else(|| anyhow::anyhow!("IMAP manager not initialized"))?;
        
        let _database = self.database.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database not initialized"))?;
        
        // Test connection first
        match imap_manager.test_connection(account_id).await {
            Ok(true) => {
                tracing::info!("IMAP connection test successful for account: {}", account_id);
            }
            Ok(false) => {
                return Err(anyhow::anyhow!("IMAP connection test failed for account: {}", account_id));
            }
            Err(e) => {
                return Err(anyhow::anyhow!("IMAP connection error for account {}: {}", account_id, e));
            }
        }
        
        // Get IMAP client
        let client_arc = imap_manager.get_client(account_id).await
            .map_err(|e| anyhow::anyhow!("Failed to get IMAP client: {}", e))?;
        
        {
            let mut client = client_arc.lock().await;
            
            // Select the folder (typically INBOX)
            let folder = client.select_folder(folder_name).await
                .map_err(|e| anyhow::anyhow!("Failed to select folder {}: {}", folder_name, e))?;
            
            tracing::info!("Selected folder '{}': {:?} messages exist, {:?} recent", 
                         folder_name, folder.exists, folder.recent);
            
            // Fetch recent messages (limit to 50 for now)
            let message_count = std::cmp::min(folder.exists.unwrap_or(0) as usize, 50);
            if message_count > 0 {
                let sequence_set = format!("1:{}", message_count);
                let fetch_items = vec!["UID", "FLAGS", "ENVELOPE", "BODY.PEEK[HEADER]"];
                
                let messages = client.fetch_messages(&sequence_set, &fetch_items).await
                    .map_err(|e| anyhow::anyhow!("Failed to fetch messages: {}", e))?;
                
                tracing::info!("Fetched {} messages from IMAP", messages.len());
                
                // Store messages in database
                for message in messages {
                    // Convert IMAP message to database format and store
                    // For now, just log the message info
                    tracing::debug!("Message UID: {:?}, Subject: {:?}", 
                                  message.uid, message.envelope.as_ref().map(|e| &e.subject));
                    
                    // TODO: Convert and store message in database
                    // This would involve parsing the envelope and headers,
                    // storing in the messages table, etc.
                }
            }
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
        Self::new().expect("Failed to create default App")
    }
}