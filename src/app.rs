use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    tty::IsTty,
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
use crate::services::ServiceManager;

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
    services: Option<ServiceManager>,
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
            services: None,
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
    
    /// Initialize dashboard services for start page
    pub async fn initialize_dashboard_services(&mut self) -> Result<()> {
        let services = ServiceManager::new()
            .map_err(|e| anyhow::anyhow!("Failed to initialize dashboard services: {}", e))?;
        
        self.services = Some(services);
        
        // Start background service updates
        self.update_start_page_data().await?;
        
        Ok(())
    }
    
    /// Update start page with fresh data
    pub async fn update_start_page_data(&mut self) -> Result<()> {
        if let Some(ref mut services) = self.services {
            // Update weather
            match services.weather.get_weather(None).await {
                Ok(weather) => {
                    self.ui.start_page_mut().set_weather(weather);
                }
                Err(e) => {
                    tracing::warn!("Failed to get weather data: {}", e);
                }
            }
            
            // Update system stats
            let stats = services.system_stats.get_stats();
            self.ui.start_page_mut().set_system_stats(stats);
            
            // Update tasks
            let tasks = services.tasks.get_pending_tasks()
                .into_iter()
                .take(8) // Limit for display
                .cloned()
                .collect();
            self.ui.start_page_mut().set_tasks(tasks);
            
            // TODO: Update calendar events when CalDAV is implemented
            // For now, set empty calendar events
            self.ui.start_page_mut().set_calendar_events(vec![]);
        }
        
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
        tracing::info!("Starting OAuth2 setup wizard");
        
        // Try TUI wizard first, fallback to file-based setup
        match self.try_tui_setup_wizard().await {
            Ok(Some(account_config)) => {
                self.setup_oauth2_account(&account_config).await?;
                tracing::info!("Account setup completed successfully!");
                tracing::info!("Account: {} ({})", account_config.display_name, account_config.email_address);
                Ok(())
            }
            Ok(None) => {
                Err(anyhow::anyhow!("Account setup was cancelled"))
            }
            Err(e) => {
                tracing::warn!("TUI setup wizard failed: {}", e);
                tracing::info!("Falling back to file-based setup...");
                self.try_file_based_setup().await
            }
        }
    }
    
    /// Try to run the TUI-based setup wizard
    async fn try_tui_setup_wizard(&mut self) -> Result<Option<crate::oauth2::AccountConfig>> {
        let mut wizard = crate::oauth2::SetupWizard::new()
            .map_err(|e| anyhow::anyhow!("Failed to create setup wizard: {}", e))?;
        tracing::info!("Setup wizard created successfully");
        
        let account_config = wizard.run().await
            .map_err(|e| anyhow::anyhow!("Setup wizard failed: {}", e))?;
        
        Ok(account_config)
    }
    
    /// Try file-based OAuth2 setup when TUI fails
    async fn try_file_based_setup(&mut self) -> Result<()> {
        use crate::oauth2::OAuth2FileImporter;
        
        println!("\n🔧 OAuth2 Setup Required");
        println!("═══════════════════════════════════════════════════════════════");
        println!("The interactive setup wizard is not available in this environment.");
        println!("Please use file-based setup instead:");
        println!();
        println!("📁 Step 1: Download OAuth2 credentials from Google Cloud Console");
        println!("   • Go to: https://console.cloud.google.com/apis/credentials");
        println!("   • Create OAuth2 Client ID (Desktop Application)");
        println!("   • Download the JSON file");
        println!();
        println!("✉️  Step 2: Enter your email address and credentials file path");
        println!();
        
        // Get email address
        print!("📧 Email address: ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        let mut email = String::new();
        std::io::stdin().read_line(&mut email)?;
        let email = email.trim();
        
        if !OAuth2FileImporter::validate_email(email) {
            return Err(anyhow::anyhow!("Invalid email address format"));
        }
        
        // Get display name
        print!("👤 Display name (optional): ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        let mut display_name = String::new();
        std::io::stdin().read_line(&mut display_name)?;
        let display_name = display_name.trim();
        let display_name = if display_name.is_empty() { None } else { Some(display_name.to_string()) };
        
        // Get credentials file path
        print!("📄 Path to credentials JSON file: ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        let mut file_path = String::new();
        std::io::stdin().read_line(&mut file_path)?;
        let file_path = file_path.trim();
        
        // Import credentials and set up account
        tracing::info!("Starting file-based OAuth2 setup for {}", email);
        let account_config = OAuth2FileImporter::import_google_credentials(
            file_path,
            email,
            display_name,
        ).await.map_err(|e| anyhow::anyhow!("Failed to import credentials: {}", e))?;
        
        // Setup account using the OAuth2 account setup method
        self.setup_oauth2_account(&account_config).await?;
        
        println!();
        println!("✅ Account setup completed successfully!");
        println!("   Account: {} ({})", account_config.display_name, account_config.email_address);
        println!();
        
        Ok(())
    }
    
    /// Setup OAuth2 account (shared between TUI and file setup)
    async fn setup_oauth2_account(&mut self, account_config: &crate::oauth2::AccountConfig) -> Result<()> {
        // Store the account configuration in secure storage
        self.storage.store_account(account_config)
            .map_err(|e| anyhow::anyhow!("Failed to store account: {}", e))?;
        
        // Store OAuth2 tokens in TokenManager for IMAP/SMTP authentication
        if let Some(ref token_manager) = self.token_manager {
            // Create TokenResponse from AccountConfig for TokenManager storage
            let token_response = crate::oauth2::TokenResponse {
                access_token: account_config.access_token.clone(),
                refresh_token: account_config.refresh_token.clone(),
                token_type: "Bearer".to_string(),
                expires_in: account_config.token_expires_at.map(|expires_at| {
                    let now = chrono::Utc::now();
                    let duration = expires_at.signed_duration_since(now);
                    duration.num_seconds().max(0) as u64
                }),
                scope: Some(account_config.scopes.join(" ")),
            };
            
            token_manager.store_tokens(
                account_config.account_id.clone(),
                account_config.provider.clone(),
                &token_response,
            ).await.map_err(|e| anyhow::anyhow!("Failed to store tokens in TokenManager: {}", e))?;
            
            tracing::info!("OAuth2 tokens stored in TokenManager for account: {}", account_config.account_id);
        } else {
            tracing::warn!("TokenManager not initialized, OAuth2 tokens not stored for IMAP/SMTP");
        }
        
        // Create account in database using existing method
        self.create_account_from_config(account_config).await?;
        
        // Convert AccountConfig to AccountItem for the UI
        let account_item = crate::ui::AccountItem::from_config(account_config);
        
        // Add account to the UI
        let accounts = vec![account_item];
        self.ui.set_accounts(accounts);
        
        Ok(())
    }
    
    /// Load existing accounts from storage
    async fn load_existing_accounts(&mut self) -> Result<()> {
        let accounts = self.storage.load_all_accounts()
            .map_err(|e| anyhow::anyhow!("Failed to load accounts: {}", e))?;
        
        println!("DEBUG: Loaded {} accounts from storage", accounts.len());
        for (i, account) in accounts.iter().enumerate() {
            println!("DEBUG: Account {}: {} ({})", i, account.display_name, account.account_id);
        }
        
        if accounts.is_empty() {
            return self.load_sample_data().await;
        }
        
        // Load OAuth2 tokens into TokenManager for existing accounts
        if let Some(ref token_manager) = self.token_manager {
            for account in &accounts {
                if !account.access_token.is_empty() {
                    // Create TokenResponse from AccountConfig for TokenManager storage
                    let token_response = crate::oauth2::TokenResponse {
                        access_token: account.access_token.clone(),
                        refresh_token: account.refresh_token.clone(),
                        token_type: "Bearer".to_string(),
                        expires_in: account.token_expires_at.map(|expires_at| {
                            let now = chrono::Utc::now();
                            let duration = expires_at.signed_duration_since(now);
                            duration.num_seconds().max(0) as u64
                        }),
                        scope: Some(account.scopes.join(" ")),
                    };
                    
                    if let Err(e) = token_manager.store_tokens(
                        account.account_id.clone(),
                        account.provider.clone(),
                        &token_response,
                    ).await {
                        tracing::warn!("Failed to load tokens for account {}: {}", account.account_id, e);
                    } else {
                        tracing::info!("Loaded OAuth2 tokens for existing account: {}", account.account_id);
                    }
                }
            }
        } else {
            tracing::warn!("TokenManager not initialized, OAuth2 tokens not loaded for existing accounts");
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
        println!("DEBUG: App.load_existing_accounts() - Loading messages for accounts");
        println!("DEBUG: Checking for current account...");
        if let Some(current_account_id) = self.ui.get_current_account_id().cloned() {
            println!("DEBUG: Found current account: {}", current_account_id);
            // Try to sync folders and messages from IMAP
            if let Some(ref mut _imap_manager) = self.imap_manager {
                println!("DEBUG: Starting IMAP sync for account: {}", current_account_id);
                match self.sync_account_from_imap(&current_account_id).await {
                    Ok(_) => {
                        println!("DEBUG: IMAP sync completed successfully, loading folders and messages");
                        // Successfully synced from IMAP, load folders first
                        match self.ui.load_folders(&current_account_id).await {
                            Ok(_) => println!("DEBUG: Successfully loaded folders from database"),
                            Err(e) => println!("DEBUG: Failed to load folders from database: {}", e),
                        }
                        
                        // Then load messages for INBOX
                        match self.ui.load_messages(current_account_id.clone(), "INBOX".to_string()).await {
                            Ok(_) => println!("DEBUG: Successfully loaded messages for INBOX"),
                            Err(e) => println!("DEBUG: Failed to load messages for INBOX: {}", e),
                        }
                    }
                    Err(e) => {
                        tracing::error!("IMAP sync failed for account {}: {}", current_account_id, e);
                        // Show the actual error instead of falling back to sample data
                        // This helps debug OAuth2 authentication issues
                        return Err(anyhow::anyhow!("Failed to sync IMAP data: {}", e));
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
        println!("DEBUG: fetch_messages_from_imap called for account: {}, folder: {}", account_id, folder_name);
        let imap_manager = self.imap_manager.as_mut()
            .ok_or_else(|| anyhow::anyhow!("IMAP manager not initialized"))?;
        
        let database = self.database.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database not initialized"))?;
        
        tracing::info!("Starting message fetch for account: {}, folder: {}", account_id, folder_name);
        
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
            } else {
                tracing::info!("No messages found in folder: {}", folder_name);
            }
        }
        
        tracing::info!("Message fetch completed successfully for account: {}", account_id);
        Ok(())
    }
    
    /// Sync account data from IMAP (folders and messages)
    async fn sync_account_from_imap(&mut self, account_id: &str) -> Result<()> {
        println!("DEBUG: sync_account_from_imap called for: {}", account_id);
        tracing::info!("Starting IMAP sync for account: {}", account_id);
        
        // First sync folders
        self.sync_folders_from_imap(account_id).await?;
        
        // Then sync messages for INBOX (or first available folder)
        self.fetch_messages_from_imap(account_id, "INBOX").await?;
        
        Ok(())
    }
    
    /// Sync folders from IMAP and store in database
    async fn sync_folders_from_imap(&mut self, account_id: &str) -> Result<()> {
        println!("DEBUG: sync_folders_from_imap called for: {}", account_id);
        let imap_manager = self.imap_manager.as_mut()
            .ok_or_else(|| anyhow::anyhow!("IMAP manager not initialized"))?;
        
        let database = self.database.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database not initialized"))?;
        
        tracing::info!("Syncing folders for account: {}", account_id);
        
        // Get IMAP client
        println!("DEBUG: About to call imap_manager.get_client() for: {}", account_id);
        let client_arc = imap_manager.get_client(account_id).await
            .map_err(|e| anyhow::anyhow!("Failed to get IMAP client: {}", e))?;
        println!("DEBUG: Successfully got IMAP client for: {}", account_id);
        
        {
            let mut client = client_arc.lock().await;
            
            // List all folders from IMAP server
            println!("DEBUG: About to call client.list_folders()");
            let folders = client.list_folders("", "*").await
                .map_err(|e| {
                    println!("DEBUG: Failed to list folders: {}", e);
                    anyhow::anyhow!("Failed to list folders: {}", e)
                })?;
            println!("DEBUG: Successfully listed {} folders", folders.len());
            
            let folder_count = folders.len();
            tracing::info!("Found {} folders from IMAP", folder_count);
            
            // Delete existing folders for this account to refresh the list
            sqlx::query("DELETE FROM folders WHERE account_id = ?")
                .bind(account_id)
                .execute(&database.pool)
                .await?;
            
            // Store each folder in database
            println!("DEBUG: About to store {} folders in database", folders.len());
            for folder in folders {
                println!("DEBUG: Storing folder: {} ({})", folder.name, folder.full_name);
                tracing::debug!("Storing folder: {} ({})", folder.name, folder.full_name);
                
                let attributes_json = serde_json::to_string(&folder.attributes)
                    .unwrap_or_else(|_| "[]".to_string());
                
                sqlx::query("INSERT INTO folders (account_id, name, full_name, delimiter, attributes, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)")
                    .bind(account_id)
                    .bind(&folder.name)
                    .bind(&folder.full_name)
                    .bind(folder.delimiter.as_deref().unwrap_or("."))
                    .bind(&attributes_json)
                    .bind(chrono::Utc::now().to_rfc3339())
                    .bind(chrono::Utc::now().to_rfc3339())
                    .execute(&database.pool)
                    .await?;
            }
            
            tracing::info!("Successfully synced {} folders for account {}", folder_count, account_id);
        }
        
        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        // Check if we're running in a proper terminal
        if !std::io::stdout().is_tty() {
            return Err(anyhow::anyhow!(
                "Comunicado requires a proper terminal (TTY) to run. Please run this application in a terminal emulator."
            ));
        }
        
        // Setup terminal
        enable_raw_mode().map_err(|e| anyhow::anyhow!("Failed to enable raw mode: {}. Make sure you're running in a proper terminal.", e))?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
            .map_err(|e| anyhow::anyhow!("Failed to setup terminal: {}. Make sure your terminal supports these features.", e))?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)
            .map_err(|e| anyhow::anyhow!("Failed to create terminal: {}", e))?;

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
                        EventResult::RefreshAccount(account_id) => {
                            self.handle_refresh_account(&account_id).await?;
                        }
                        EventResult::SyncAccount(account_id) => {
                            self.handle_sync_account(&account_id).await?;
                        }
                        EventResult::FolderSelect(folder_path) => {
                            self.handle_folder_select(&folder_path).await?;
                        }
                        EventResult::FolderOperation(operation) => {
                            self.handle_folder_operation(operation).await?;
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
        
        // First sync folders and messages from IMAP, then switch to account
        match self.sync_account_from_imap(account_id).await {
            Ok(()) => {
                tracing::info!("Successfully synced account: {}", account_id);
                
                // Now switch to the account in UI (which will load from local database)
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
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to sync account {}: {}", account_id, e);
                // Update status to error if failed
                self.ui.update_account_status(account_id, crate::ui::AccountSyncStatus::Error, None);
                
                // Still try to switch to account with local data as fallback
                if let Err(e) = self.ui.switch_to_account(account_id).await {
                    tracing::error!("Failed to switch to account {} even with local data: {}", account_id, e);
                }
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
    
    /// Handle account refresh (Ctrl+R) - reconnect and update status
    async fn handle_refresh_account(&mut self, account_id: &str) -> Result<()> {
        tracing::info!("Refreshing account connection: {}", account_id);
        
        // Update status to show we're refreshing
        self.ui.update_account_status(account_id, crate::ui::AccountSyncStatus::Syncing, None);
        
        // Force reconnection by clearing any cached connections
        if let Some(ref mut imap_manager) = self.imap_manager {
            // Disconnect all connections to force fresh connection (no per-account disconnect available)
            if let Err(e) = imap_manager.disconnect_all().await {
                tracing::warn!("Error disconnecting connections: {}", e);
            }
        }
        
        // Try to sync account to test connection
        match self.sync_account_from_imap(account_id).await {
            Ok(()) => {
                tracing::info!("Successfully refreshed account: {}", account_id);
                self.ui.update_account_status(account_id, crate::ui::AccountSyncStatus::Online, None);
                
                // Reload messages for current folder if this is the active account
                if let Some(current_id) = self.ui.get_current_account_id() {
                    if current_id == account_id {
                        let _ = self.ui.load_messages(account_id.to_string(), "INBOX".to_string()).await;
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to refresh account {}: {}", account_id, e);
                self.ui.update_account_status(account_id, crate::ui::AccountSyncStatus::Error, None);
                return Err(anyhow::anyhow!("Failed to refresh account: {}", e));
            }
        }
        
        Ok(())
    }
    
    /// Handle manual IMAP sync (F5) - sync folders and messages
    async fn handle_sync_account(&mut self, account_id: &str) -> Result<()> {
        tracing::info!("Manual IMAP sync requested for account: {}", account_id);
        
        // Update status to show we're syncing
        self.ui.update_account_status(account_id, crate::ui::AccountSyncStatus::Syncing, None);
        
        // Perform full sync
        match self.sync_account_from_imap(account_id).await {
            Ok(()) => {
                tracing::info!("Successfully synced account: {}", account_id);
                self.ui.update_account_status(account_id, crate::ui::AccountSyncStatus::Online, None);
                
                // If this is the current account, reload the message list
                if let Some(current_id) = self.ui.get_current_account_id() {
                    if current_id == account_id {
                        // Get current folder
                        let current_folder = self.ui.folder_tree()
                            .selected_folder()
                            .map(|f| f.name.clone())
                            .unwrap_or_else(|| "INBOX".to_string());
                        
                        // Reload messages
                        if let Err(e) = self.ui.load_messages(account_id.to_string(), current_folder).await {
                            tracing::error!("Failed to reload messages after sync: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to sync account {}: {}", account_id, e);
                self.ui.update_account_status(account_id, crate::ui::AccountSyncStatus::Error, None);
                return Err(anyhow::anyhow!("Failed to sync account: {}", e));
            }
        }
        
        Ok(())
    }
    
    /// Handle folder selection event - load messages from the selected folder
    async fn handle_folder_select(&mut self, folder_path: &str) -> Result<()> {
        // Get the current account ID and clone it to avoid borrowing issues
        let current_account_id = match self.ui.get_current_account_id() {
            Some(id) => id.clone(),
            None => {
                tracing::warn!("No current account selected for folder selection");
                return Ok(());
            }
        };
        
        tracing::info!("Loading messages from folder: {} for account: {}", folder_path, current_account_id);
        
        // Load messages from the selected folder
        match self.ui.load_messages(current_account_id.clone(), folder_path.to_string()).await {
            Ok(()) => {
                tracing::info!("Successfully loaded messages from folder: {}", folder_path);
            }
            Err(e) => {
                tracing::error!("Failed to load messages from folder {}: {}", folder_path, e);
                // Try to fetch fresh messages from IMAP if database load fails
                if let Err(fetch_error) = self.fetch_messages_from_imap(&current_account_id, folder_path).await {
                    tracing::error!("Failed to fetch messages from IMAP for folder {}: {}", folder_path, fetch_error);
                } else {
                    // Retry loading from database after fetch
                    if let Err(retry_error) = self.ui.load_messages(current_account_id.clone(), folder_path.to_string()).await {
                        tracing::error!("Failed to load messages even after IMAP fetch for folder {}: {}", folder_path, retry_error);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Handle folder operation events
    async fn handle_folder_operation(&mut self, operation: crate::ui::folder_tree::FolderOperation) -> Result<()> {
        use crate::ui::folder_tree::FolderOperation;
        
        // Get current account and selected folder
        let current_account_id = match self.ui.get_current_account_id() {
            Some(id) => id.clone(),
            None => {
                tracing::warn!("No current account selected for folder operation");
                return Ok(());
            }
        };
        
        let selected_folder = self.ui.folder_tree().selected_folder().map(|f| f.clone());
        
        match operation {
            FolderOperation::Refresh => {
                self.handle_folder_refresh(&current_account_id).await?;
            }
            FolderOperation::MarkAllRead => {
                self.handle_mark_all_read(&current_account_id).await?;
            }
            FolderOperation::Properties => {
                self.handle_folder_properties(&current_account_id).await?;
            }
            FolderOperation::Create => {
                self.handle_create_folder(&current_account_id, None).await?;
            }
            FolderOperation::CreateSubfolder => {
                if let Some(folder) = selected_folder {
                    self.handle_create_folder(&current_account_id, Some(&folder.path)).await?;
                }
            }
            FolderOperation::Delete => {
                if let Some(folder) = selected_folder {
                    self.handle_delete_folder(&current_account_id, &folder.path).await?;
                }
            }
            FolderOperation::Rename => {
                if let Some(folder) = selected_folder {
                    self.handle_rename_folder(&current_account_id, &folder.path).await?;
                }
            }
            FolderOperation::EmptyFolder => {
                if let Some(folder) = selected_folder {
                    self.handle_empty_folder(&current_account_id, &folder.path).await?;
                }
            }
            FolderOperation::Subscribe => {
                if let Some(folder) = selected_folder {
                    self.handle_folder_subscription(&current_account_id, &folder.path, true).await?;
                }
            }
            FolderOperation::Unsubscribe => {
                if let Some(folder) = selected_folder {
                    self.handle_folder_subscription(&current_account_id, &folder.path, false).await?;
                }
            }
            FolderOperation::Move => {
                // TODO: Implement move folder functionality
                tracing::info!("Move folder operation not yet implemented");
            }
        }
        
        Ok(())
    }
    
    /// Handle folder refresh
    async fn handle_folder_refresh(&mut self, account_id: &str) -> Result<()> {
        tracing::info!("Refreshing folder for account: {}", account_id);
        
        // Sync folders first
        self.sync_folders_from_imap(account_id).await?;
        
        // Reload folders in UI
        if let Err(e) = self.ui.load_folders(account_id).await {
            tracing::error!("Failed to reload folders: {}", e);
        }
        
        // If a folder is currently selected, refresh its messages
        if let Some(selected_folder) = self.ui.folder_tree().selected_folder() {
            let folder_path = selected_folder.path.clone();
            self.fetch_messages_from_imap(account_id, &folder_path).await?;
            if let Err(e) = self.ui.load_messages(account_id.to_string(), folder_path).await {
                tracing::error!("Failed to reload messages: {}", e);
            }
        }
        
        tracing::info!("Folder refresh completed for account: {}", account_id);
        Ok(())
    }
    
    /// Handle mark all messages as read in current folder
    async fn handle_mark_all_read(&mut self, account_id: &str) -> Result<()> {
        if let Some(selected_folder) = self.ui.folder_tree().selected_folder() {
            let folder_path = selected_folder.path.clone();
            tracing::info!("Marking all messages as read in folder: {} for account: {}", folder_path, account_id);
            
            // TODO: Implement IMAP STORE command to mark messages as read
            // For now, just update the database
            if let Some(ref database) = self.database {
                let result = sqlx::query("UPDATE messages SET is_read = 1, flags = flags || ',\\Seen' WHERE account_id = ? AND folder_name = ? AND is_read = 0")
                    .bind(account_id)
                    .bind(&folder_path)
                    .execute(&database.pool)
                    .await?;
                
                tracing::info!("Marked {} messages as read in folder: {}", result.rows_affected(), folder_path);
                
                // Reload messages to update UI
                if let Err(e) = self.ui.load_messages(account_id.to_string(), folder_path).await {
                    tracing::error!("Failed to reload messages after marking as read: {}", e);
                }
            }
        }
        
        Ok(())
    }
    
    /// Handle folder properties display
    async fn handle_folder_properties(&mut self, account_id: &str) -> Result<()> {
        if let Some(selected_folder) = self.ui.folder_tree().selected_folder() {
            let folder_path = selected_folder.path.clone();
            tracing::info!("Showing properties for folder: {} in account: {}", folder_path, account_id);
            
            // TODO: Implement folder properties dialog
            // For now, just log the information
            if let Some(ref database) = self.database {
                let stats = sqlx::query_as::<_, (i64, i64, Option<i64>)>(
                    "SELECT COUNT(*), COUNT(CASE WHEN is_read = 0 THEN 1 END), SUM(size) FROM messages WHERE account_id = ? AND folder_name = ?"
                )
                .bind(account_id)
                .bind(&folder_path)
                .fetch_one(&database.pool)
                .await?;
                
                tracing::info!("Folder {} statistics: {} total messages, {} unread, {} bytes", 
                             folder_path, stats.0, stats.1, stats.2.unwrap_or(0));
            }
        }
        
        Ok(())
    }
    
    /// Handle create new folder
    async fn handle_create_folder(&mut self, account_id: &str, parent_path: Option<&str>) -> Result<()> {
        tracing::info!("Creating new folder in account: {}, parent: {:?}", account_id, parent_path);
        
        // TODO: Implement folder creation dialog
        // For now, create a default folder name
        let folder_name = if parent_path.is_some() {
            "New Subfolder"
        } else {
            "New Folder"
        };
        
        // TODO: Implement IMAP CREATE command
        tracing::info!("Would create folder: {} in account: {}", folder_name, account_id);
        
        Ok(())
    }
    
    /// Handle delete folder
    async fn handle_delete_folder(&mut self, account_id: &str, folder_path: &str) -> Result<()> {
        tracing::info!("Deleting folder: {} from account: {}", folder_path, account_id);
        
        // TODO: Implement confirmation dialog
        // TODO: Implement IMAP DELETE command
        tracing::info!("Would delete folder: {} from account: {}", folder_path, account_id);
        
        Ok(())
    }
    
    /// Handle rename folder
    async fn handle_rename_folder(&mut self, account_id: &str, folder_path: &str) -> Result<()> {
        tracing::info!("Renaming folder: {} in account: {}", folder_path, account_id);
        
        // TODO: Implement rename dialog
        // TODO: Implement IMAP RENAME command
        tracing::info!("Would rename folder: {} in account: {}", folder_path, account_id);
        
        Ok(())
    }
    
    /// Handle empty folder (delete all messages)
    async fn handle_empty_folder(&mut self, account_id: &str, folder_path: &str) -> Result<()> {
        tracing::info!("Emptying folder: {} in account: {}", folder_path, account_id);
        
        // TODO: Implement confirmation dialog
        // TODO: Implement IMAP STORE/EXPUNGE commands to delete all messages
        tracing::info!("Would empty folder: {} in account: {}", folder_path, account_id);
        
        Ok(())
    }
    
    /// Handle folder subscription management
    async fn handle_folder_subscription(&mut self, account_id: &str, folder_path: &str, subscribe: bool) -> Result<()> {
        let action = if subscribe { "Subscribing to" } else { "Unsubscribing from" };
        tracing::info!("{} folder: {} in account: {}", action, folder_path, account_id);
        
        // TODO: Implement IMAP SUBSCRIBE/UNSUBSCRIBE commands
        // For now, just update local state
        self.ui.folder_tree_mut().mark_folder_synced(folder_path, 0, 0);
        
        Ok(())
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new().expect("Failed to create default App")
    }
}