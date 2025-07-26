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

use crate::events::{EventHandler, EventResult};
use crate::ui::{UI, ComposeAction};
use crate::email::{EmailDatabase, EmailNotificationManager};
use crate::oauth2::{SetupWizard, SecureStorage, AccountConfig, TokenManager};
use crate::imap::ImapAccountManager;
use crate::smtp::{SmtpService, SmtpServiceBuilder};
use crate::contacts::ContactsManager;

pub struct App {
    should_quit: bool,
    ui: UI,
    event_handler: EventHandler,
    database: Option<Arc<EmailDatabase>>,
    notification_manager: Option<Arc<EmailNotificationManager>>,
    storage: SecureStorage,
    imap_manager: Option<ImapAccountManager>,
    token_manager: Option<TokenManager>,
    smtp_service: Option<SmtpService>,
    contacts_manager: Option<Arc<ContactsManager>>,
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
            smtp_service: None,
            contacts_manager: None,
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
            
            tracing::info!("Account setup completed successfully!");
            tracing::info!("Account: {} ({})", account_config.display_name, account_config.email_address);
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
        
        // Convert AccountConfig to AccountItem for the UI
        let account_items: Vec<crate::ui::AccountItem> = accounts.iter()
            .map(|config| crate::ui::AccountItem::from_config(config))
            .collect();
        
        // Set accounts in the UI
        self.ui.set_accounts(account_items);
        
        // Create all accounts in the database
        for account in &accounts {
            self.create_account_from_config(account).await?;
        }
        
        // Load messages for the first account (or current account)
        if let Some(current_account_id) = self.ui.get_current_account_id().cloned() {
            // Try to fetch real messages from IMAP
            if let Some(ref mut _imap_manager) = self.imap_manager {
                match self.fetch_messages_from_imap(&current_account_id, "INBOX").await {
                    Ok(_) => {
                        // Successfully fetched messages via IMAP
                        let _ = self.ui.load_messages(current_account_id.clone(), "INBOX".to_string()).await;
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
        
        let database = self.database.as_ref()
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
                let fetch_items = vec!["UID", "FLAGS", "ENVELOPE", "BODY.PEEK[]", "BODYSTRUCTURE"];
                
                let messages = client.fetch_messages(&sequence_set, &fetch_items).await
                    .map_err(|e| anyhow::anyhow!("Failed to fetch messages: {}", e))?;
                
                tracing::info!("Fetched {} messages from IMAP", messages.len());
                
                // Store messages in database
                for message in messages {
                    tracing::debug!("Processing message UID: {:?}, Subject: {:?}", 
                                  message.uid, message.envelope.as_ref().map(|e| &e.subject));
                    
                    // Convert IMAP message to StoredMessage format
                    match self.convert_imap_to_stored_message(&message, account_id, folder_name).await {
                        Ok(stored_message) => {
                            // Store in database
                            if let Err(e) = database.store_message(&stored_message).await {
                                tracing::error!("Failed to store message UID {}: {}", 
                                              message.uid.unwrap_or(0), e);
                            } else {
                                tracing::debug!("Stored message: {}", stored_message.subject);
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to convert message UID {}: {}", 
                                          message.uid.unwrap_or(0), e);
                        }
                    }
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
                    let event_result = self.event_handler.handle_key_event(key, &mut self.ui).await;
                    
                    // Handle the event result
                    match event_result {
                        EventResult::Continue => {},
                        EventResult::ComposeAction(action) => {
                            self.handle_compose_action(action).await?;
                        }
                        EventResult::AccountSwitch(account_id) => {
                            self.handle_account_switch(&account_id).await;
                        }
                        EventResult::AddAccount => {
                            self.handle_add_account().await?;
                        }
                        EventResult::RemoveAccount(account_id) => {
                            self.handle_remove_account(&account_id).await?;
                        }
                    }
                    
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
    
    /// Initialize SMTP service and contacts manager
    pub async fn initialize_services(&mut self) -> Result<()> {
        // Initialize token manager if not already done
        if self.token_manager.is_none() {
            let token_manager = TokenManager::new();
            self.token_manager = Some(token_manager);
        }
        
        // Initialize SMTP service
        if let Some(ref token_manager) = self.token_manager {
            let smtp_service = SmtpServiceBuilder::new()
                .with_token_manager(Arc::new(token_manager.clone()))
                .build()
                .map_err(|e| anyhow::anyhow!("Failed to initialize SMTP service: {}", e))?;
            
            self.smtp_service = Some(smtp_service);
        }
        
        // Initialize contacts manager (optional - don't fail if it can't be initialized)
        if let (Some(ref _database), Some(ref token_manager)) = (&self.database, &self.token_manager) {
            // Create contacts database from email database path  
            let data_dir = dirs::data_dir()
                .ok_or_else(|| anyhow::anyhow!("Failed to get data directory"))?
                .join("comunicado");
            
            // Ensure the directory exists
            if let Err(e) = std::fs::create_dir_all(&data_dir) {
                tracing::warn!("Failed to create contacts directory: {}", e);
                return Ok(()); // Don't fail initialization
            }
            
            let contacts_db_path = data_dir.join("contacts.db");
            
            // Try to initialize contacts database, but don't fail if it can't be created
            match crate::contacts::ContactsDatabase::new(
                &format!("sqlite:{}", contacts_db_path.display())
            ).await {
                Ok(contacts_database) => {
                    match ContactsManager::new(contacts_database, token_manager.clone()).await {
                        Ok(contacts_manager) => {
                            self.contacts_manager = Some(Arc::new(contacts_manager));
                            tracing::info!("Contacts manager initialized successfully");
                        }
                        Err(e) => {
                            tracing::warn!("Failed to initialize contacts manager: {}", e);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to initialize contacts database: {}", e);
                }
            }
        }
        
        tracing::info!("Services initialized successfully");
        Ok(())
    }
    
    /// Handle compose actions (send, save draft, cancel)
    async fn handle_compose_action(&mut self, action: ComposeAction) -> Result<()> {
        match action {
            ComposeAction::Send => {
                self.send_email().await?;
            }
            ComposeAction::SaveDraft => {
                self.save_draft().await?;
            }
            ComposeAction::Cancel => {
                // Check if there are unsaved changes and potentially warn the user
                if self.ui.is_compose_modified() {
                    tracing::info!("Compose cancelled with unsaved changes");
                }
                self.ui.exit_compose();
            }
            ComposeAction::Continue => {
                // Nothing to do, continue composing
            }
            ComposeAction::StartCompose => {
                // Start compose mode
                self.start_compose_mode();
            }
        }
        Ok(())
    }
    
    /// Send the current composed email
    async fn send_email(&mut self) -> Result<()> {
        let compose_data = self.ui.get_compose_data()
            .ok_or_else(|| anyhow::anyhow!("No compose data available"))?;
        
        let smtp_service = self.smtp_service.as_ref()
            .ok_or_else(|| anyhow::anyhow!("SMTP service not initialized"))?;
        
        // For now, use the first available account
        // In a real implementation, this should come from the active account
        let configs = self.storage.load_all_accounts()
            .map_err(|e| anyhow::anyhow!("Failed to load account configs: {}", e))?;
        
        if let Some(config) = configs.first() {
            let account_id = &config.account_id;
            let from_address = &config.email_address;
            
            // Initialize SMTP for this account if not already done
            if !smtp_service.is_account_configured(account_id).await {
                self.initialize_smtp_for_account(account_id, config).await?;
            }
            
            // Send the email
            match smtp_service.send_email(account_id, from_address, &compose_data).await {
                Ok(result) => {
                    tracing::info!("Email sent successfully: {}", result.message_id);
                    self.ui.exit_compose();
                    self.ui.clear_compose_modified();
                    
                    // TODO: Add a success notification to the UI
                    tracing::info!("Email sent to {} recipients", result.accepted_recipients.len());
                }
                Err(e) => {
                    tracing::error!("Failed to send email: {}", e);
                    // TODO: Show error message in UI
                    return Err(anyhow::anyhow!("Failed to send email: {}", e));
                }
            }
        } else {
            return Err(anyhow::anyhow!("No email accounts configured"));
        }
        
        Ok(())
    }
    
    /// Save the current compose as a draft
    async fn save_draft(&mut self) -> Result<()> {
        let compose_data = self.ui.get_compose_data()
            .ok_or_else(|| anyhow::anyhow!("No compose data available"))?;
        
        let smtp_service = self.smtp_service.as_ref()
            .ok_or_else(|| anyhow::anyhow!("SMTP service not initialized"))?;
        
        // For now, use the first available account
        let configs = self.storage.load_all_accounts()
            .map_err(|e| anyhow::anyhow!("Failed to load account configs: {}", e))?;
        
        if let Some(config) = configs.first() {
            let account_id = &config.account_id;
            
            match smtp_service.save_draft(account_id, &compose_data).await {
                Ok(draft_id) => {
                    tracing::info!("Draft saved with ID: {}", draft_id);
                    self.ui.clear_compose_modified();
                    // TODO: Add a success notification to the UI
                }
                Err(e) => {
                    tracing::error!("Failed to save draft: {}", e);
                    // TODO: Show error message in UI
                    return Err(anyhow::anyhow!("Failed to save draft: {}", e));
                }
            }
        } else {
            return Err(anyhow::anyhow!("No email accounts configured"));
        }
        
        Ok(())
    }
    
    /// Initialize SMTP for a specific account
    async fn initialize_smtp_for_account(&self, account_id: &str, config: &AccountConfig) -> Result<()> {
        let smtp_service = self.smtp_service.as_ref()
            .ok_or_else(|| anyhow::anyhow!("SMTP service not initialized"))?;
        
        let token_manager = self.token_manager.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Token manager not initialized"))?;
        
        // Get current access token
        let token = token_manager.get_access_token(account_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get access token: {}", e))?;
        
        // Initialize SMTP for this account
        smtp_service.initialize_account(
            account_id,
            &config.provider,
            &config.email_address,
            &token.unwrap().token,
        ).await
        .map_err(|e| anyhow::anyhow!("Failed to initialize SMTP for account {}: {}", account_id, e))?;
        
        tracing::info!("SMTP initialized for account: {}", account_id);
        Ok(())
    }
    
    /// Start compose mode with contacts support
    pub fn start_compose_mode(&mut self) {
        if let Some(ref contacts_manager) = self.contacts_manager {
            self.ui.start_compose(contacts_manager.clone());
            tracing::info!("Started compose mode with contacts support");
        } else {
            // Create a minimal contacts manager for compose mode without contacts support
            tracing::info!("Started compose mode without contacts support");
            // For now, just skip compose mode if no contacts manager
            // TODO: Implement a basic compose mode without contacts
            tracing::warn!("Compose mode requires contacts manager - skipping for now");
        }
    }
    
    /// Handle account switching with proper error handling
    async fn handle_account_switch(&mut self, account_id: &str) {
        tracing::info!("Switching to account: {}", account_id);
        
        // Update account status to show we're attempting to connect
        self.ui.update_account_status(account_id, crate::ui::AccountSyncStatus::Syncing, None);
        
        // Try to switch account - catch any errors gracefully
        match self.ui.switch_to_account(account_id).await {
            Ok(()) => {
                tracing::info!("Successfully switched to account: {}", account_id);
                // Update status to online if successful
                self.ui.update_account_status(account_id, crate::ui::AccountSyncStatus::Online, None);
            }
            Err(e) => {
                tracing::error!("Failed to switch to account {}: {}", account_id, e);
                // Update status to error if failed
                self.ui.update_account_status(account_id, crate::ui::AccountSyncStatus::Error, None);
                
                // Don't crash the app - just log the error and continue
                // The user can try again or switch to a different account
            }
        }
    }
    
    /// Handle adding a new account by launching the setup wizard
    async fn handle_add_account(&mut self) -> Result<()> {
        tracing::info!("Launching account setup wizard");
        
        // Create and run the setup wizard
        let mut wizard = SetupWizard::new()
            .map_err(|e| anyhow::anyhow!("Failed to create setup wizard: {}", e))?;
        
        if let Some(account_config) = wizard.run().await
            .map_err(|e| anyhow::anyhow!("Setup wizard failed: {}", e))? {
            
            // Create the account in the database
            self.create_account_from_config(&account_config).await?;
            
            // Convert to AccountItem for the UI
            let account_item = crate::ui::AccountItem::from_config(&account_config);
            
            // Add the account to the UI
            self.ui.add_account(account_item);
            
            tracing::info!("New account added successfully: {} ({})", 
                          account_config.display_name, account_config.email_address);
            
            // Optionally switch to the new account immediately
            self.handle_account_switch(&account_config.account_id).await;
            
        } else {
            tracing::info!("Account setup was cancelled by user");
        }
        
        Ok(())
    }
    
    /// Handle removing an account
    async fn handle_remove_account(&mut self, account_id: &str) -> Result<()> {
        tracing::info!("Removing account: {}", account_id);
        
        // Check if this is the last account - don't allow removal if it is
        if self.ui.account_switcher().accounts().len() <= 1 {
            tracing::warn!("Cannot remove the last remaining account");
            return Ok(());
        }
        
        // Remove from secure storage first
        if let Err(e) = self.storage.remove_account(account_id) {
            tracing::error!("Failed to remove account from secure storage: {}", e);
            return Err(anyhow::anyhow!("Failed to remove account from storage: {}", e));
        }
        
        // Remove from database
        if let Some(ref database) = self.database {
            // Remove all messages for this account
            if let Err(e) = sqlx::query("DELETE FROM messages WHERE account_id = ?")
                .bind(account_id)
                .execute(&database.pool)
                .await {
                tracing::error!("Failed to remove messages for account {}: {}", account_id, e);
            }
            
            // Remove folders for this account
            if let Err(e) = sqlx::query("DELETE FROM folders WHERE account_id = ?")
                .bind(account_id)
                .execute(&database.pool)
                .await {
                tracing::error!("Failed to remove folders for account {}: {}", account_id, e);
            }
            
            // Remove the account itself
            if let Err(e) = sqlx::query("DELETE FROM accounts WHERE id = ?")
                .bind(account_id)
                .execute(&database.pool)
                .await {
                tracing::error!("Failed to remove account from database: {}", e);
                return Err(anyhow::anyhow!("Failed to remove account from database: {}", e));
            }
        }
        
        // Remove from UI
        self.ui.remove_account(account_id);
        
        tracing::info!("Account {} removed successfully", account_id);
        
        Ok(())
    }
    
    /// Convert IMAP message to StoredMessage format for database storage
    async fn convert_imap_to_stored_message(
        &self,
        imap_message: &crate::imap::ImapMessage,
        account_id: &str,
        folder_name: &str,
    ) -> Result<crate::email::StoredMessage> {
        use uuid::Uuid;
        use chrono::Utc;
        
        // Extract envelope data
        let envelope = imap_message.envelope.as_ref();
        
        // Extract subject
        let subject = envelope
            .and_then(|e| e.subject.as_ref())
            .map(|s| s.clone())
            .unwrap_or_else(|| "(No Subject)".to_string());
        
        // Extract sender information
        let (from_addr, from_name) = if let Some(env) = envelope {
            if let Some(from) = env.from.first() {
                let addr = format!("{}@{}", 
                    from.mailbox.as_deref().unwrap_or("unknown"),
                    from.host.as_deref().unwrap_or("unknown.com")
                );
                let name = from.name.clone();
                (addr, name)
            } else {
                ("unknown@unknown.com".to_string(), None)
            }
        } else {
            ("unknown@unknown.com".to_string(), None)
        };
        
        // Extract recipient addresses
        let to_addrs = envelope
            .map(|e| e.to.iter().map(|addr| {
                format!("{}@{}", 
                    addr.mailbox.as_deref().unwrap_or("unknown"),
                    addr.host.as_deref().unwrap_or("unknown.com")
                )
            }).collect())
            .unwrap_or_default();
            
        let cc_addrs = envelope
            .map(|e| e.cc.iter().map(|addr| {
                format!("{}@{}", 
                    addr.mailbox.as_deref().unwrap_or("unknown"),
                    addr.host.as_deref().unwrap_or("unknown.com")
                )
            }).collect())
            .unwrap_or_default();
            
        let bcc_addrs = envelope
            .map(|e| e.bcc.iter().map(|addr| {
                format!("{}@{}", 
                    addr.mailbox.as_deref().unwrap_or("unknown"),
                    addr.host.as_deref().unwrap_or("unknown.com")
                )
            }).collect())
            .unwrap_or_default();
        
        // Extract message ID and threading info
        let message_id = envelope.and_then(|e| e.message_id.clone());
        let in_reply_to = envelope.and_then(|e| e.in_reply_to.clone());
        
        // Extract reply-to address
        let reply_to = envelope
            .and_then(|e| e.reply_to.first())
            .map(|addr| format!("{}@{}", 
                addr.mailbox.as_deref().unwrap_or("unknown"),
                addr.host.as_deref().unwrap_or("unknown.com")
            ));
        
        // Parse date
        let date = if let Some(env) = envelope {
            if let Some(date_str) = &env.date {
                // Try to parse RFC 2822 date format
                match chrono::DateTime::parse_from_rfc2822(date_str) {
                    Ok(parsed) => parsed.with_timezone(&Utc),
                    Err(_) => {
                        // Fall back to internal date or current time
                        imap_message.internal_date.unwrap_or_else(Utc::now)
                    }
                }
            } else {
                imap_message.internal_date.unwrap_or_else(Utc::now)
            }
        } else {
            imap_message.internal_date.unwrap_or_else(Utc::now)
        };
        
        // Convert IMAP flags to string format
        let flags: Vec<String> = imap_message.flags
            .iter()
            .map(|flag| match flag {
                crate::imap::MessageFlag::Seen => "\\Seen".to_string(),
                crate::imap::MessageFlag::Answered => "\\Answered".to_string(),
                crate::imap::MessageFlag::Flagged => "\\Flagged".to_string(),
                crate::imap::MessageFlag::Deleted => "\\Deleted".to_string(),
                crate::imap::MessageFlag::Draft => "\\Draft".to_string(),
                crate::imap::MessageFlag::Recent => "\\Recent".to_string(),
                crate::imap::MessageFlag::Custom(s) => s.clone(),
            })
            .collect();
        
        // Extract and parse email body content
        let (body_text, body_html) = self.parse_email_body(imap_message);
        
        // Parse attachments from body structure
        let attachments = self.parse_attachments_from_body_structure(imap_message);
        
        // Create StoredMessage
        let stored_message = crate::email::StoredMessage {
            id: Uuid::new_v4(),
            account_id: account_id.to_string(),
            folder_name: folder_name.to_string(),
            imap_uid: imap_message.uid.unwrap_or(0),
            message_id,
            thread_id: None, // Will be populated by threading engine later
            in_reply_to,
            references: Vec::new(), // TODO: Parse References header
            
            // Headers
            subject,
            from_addr,
            from_name,
            to_addrs,
            cc_addrs,
            bcc_addrs,
            reply_to,
            date,
            
            // Content
            body_text,
            body_html,
            attachments,
            
            // Metadata
            flags,
            labels: Vec::new(),
            size: imap_message.size,
            priority: None,
            
            // Timestamps
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_synced: Utc::now(),
            sync_version: 1,
            is_draft: false,
            is_deleted: false,
        };
        
        Ok(stored_message)
    }
    
    /// Parse email body content to extract both text and HTML parts
    fn parse_email_body(&self, imap_message: &crate::imap::ImapMessage) -> (Option<String>, Option<String>) {
        use crate::html::is_html_content;
        
        let raw_body = match &imap_message.body {
            Some(body) => body,
            None => return (None, None),
        };
        
        // Check if the content appears to be HTML
        if is_html_content(raw_body) {
            // This is HTML content
            let html_body = Some(raw_body.clone());
            
            // Convert HTML to plain text for the text body
            let html_renderer = crate::html::HtmlRenderer::new(80);
            let text_body = Some(html_renderer.html_to_plain_text(raw_body));
            
            (text_body, html_body)
        } else {
            // This is plain text content
            (Some(raw_body.clone()), None)
        }
    }
    
    /// Parse attachments from IMAP message body structure
    fn parse_attachments_from_body_structure(&self, imap_message: &crate::imap::ImapMessage) -> Vec<crate::email::StoredAttachment> {
        let mut attachments = Vec::new();
        
        if let Some(ref body_structure) = imap_message.body_structure {
            self.extract_attachments_recursive(body_structure, &mut attachments, 0);
        }
        
        attachments
    }
    
    /// Recursively extract attachments from body structure
    fn extract_attachments_recursive(&self, body_structure: &crate::imap::BodyStructure, attachments: &mut Vec<crate::email::StoredAttachment>, part_index: usize) {
        // Check if this part is an attachment
        if body_structure.is_attachment() {
            let filename = body_structure.parameters.get("name")
                .or_else(|| body_structure.parameters.get("filename"))
                .cloned()
                .unwrap_or_else(|| format!("attachment_{}", part_index));
            
            let content_type = format!("{}/{}", body_structure.media_type, body_structure.media_subtype);
            
            let attachment = crate::email::StoredAttachment {
                id: format!("att_{}_{}", part_index, chrono::Utc::now().timestamp_millis()),
                filename,
                content_type,
                size: body_structure.size.unwrap_or(0),
                content_id: body_structure.content_id.clone(),
                is_inline: body_structure.content_id.is_some(),
                data: None, // Will be fetched separately when needed
                file_path: None, // Will be set when saved to disk
            };
            
            tracing::debug!("Found attachment: {} ({})", attachment.filename, attachment.content_type);
            
            attachments.push(attachment);
        }
        
        // Process multipart structures recursively
        if body_structure.is_multipart() {
            for (index, part) in body_structure.parts.iter().enumerate() {
                self.extract_attachments_recursive(part, attachments, part_index * 10 + index + 1);
            }
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new().expect("Failed to create default App")
    }
}