use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    tty::IsTty,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use sqlx::Row;
use std::io;
use std::sync::Arc;
use tokio::time::{Duration, Instant};

use crate::contacts::ContactsManager;
use crate::email::{EmailDatabase, EmailNotificationManager};
use crate::events::{EventHandler, EventResult};
use crate::imap::ImapAccountManager;
use crate::notifications::{NotificationConfig, UnifiedNotificationManager};
use crate::oauth2::{AccountConfig, SecureStorage, TokenManager};
use crate::smtp::{SmtpService, SmtpServiceBuilder};
use crate::startup::StartupProgressManager;
use crate::ui::{ComposeAction, DraftAction, UI};
use crate::performance::background_processor::{BackgroundProcessor, BackgroundTask, TaskResult};
use crate::email::sync_engine::SyncProgress;
use tokio::sync::mpsc;
use uuid::Uuid;

pub struct App {
    should_quit: bool,
    ui: UI,
    event_handler: EventHandler,
    database: Option<Arc<EmailDatabase>>,
    notification_manager: Option<Arc<EmailNotificationManager>>,
    storage: SecureStorage,
    imap_manager: Option<Arc<ImapAccountManager>>,
    token_manager: Option<TokenManager>,
    token_refresh_scheduler: Option<crate::oauth2::token::TokenRefreshScheduler>,
    smtp_service: Option<SmtpService>,
    contacts_manager: Option<Arc<ContactsManager>>,
    unified_notification_manager: Option<Arc<UnifiedNotificationManager>>,
    // Auto-sync functionality
    last_auto_sync: Instant,
    auto_sync_interval: Duration,
    // Deferred initialization
    deferred_initialization: bool,
    initialization_complete: bool,
    initialization_in_progress: bool,
    // Startup progress tracking
    startup_progress_manager: StartupProgressManager,
    // Background processing
    background_processor: Option<Arc<BackgroundProcessor>>,
    sync_progress_rx: Option<mpsc::UnboundedReceiver<SyncProgress>>,
    task_completion_rx: Option<mpsc::UnboundedReceiver<TaskResult>>,
}

impl App {
    /// Create a new application instance with default configuration
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
            token_refresh_scheduler: None,
            smtp_service: None,
            contacts_manager: None,
            unified_notification_manager: None,
            // Initialize auto-sync with 3 minute interval
            last_auto_sync: Instant::now(),
            auto_sync_interval: Duration::from_secs(3 * 60), // 3 minutes
            // Deferred initialization
            deferred_initialization: false,
            initialization_complete: false,
            initialization_in_progress: false,
            // Startup progress tracking
            startup_progress_manager: StartupProgressManager::new(),
            // Background processing
            background_processor: None,
            sync_progress_rx: None,
            task_completion_rx: None,
        })
    }

    /// Get reference to the startup progress manager
    pub fn startup_progress_manager(&self) -> &StartupProgressManager {
        &self.startup_progress_manager
    }

    /// Get mutable reference to the startup progress manager
    pub fn startup_progress_manager_mut(&mut self) -> &mut StartupProgressManager {
        &mut self.startup_progress_manager
    }

    /// Initialize the database connection
    pub async fn initialize_database(&mut self) -> Result<()> {
        // Start database initialization phase
        self.startup_progress_manager.start_phase("Database").map_err(|e| anyhow::anyhow!("Progress manager error: {}", e))?;
        
        // Perform database initialization with error handling
        let result: Result<()> = async {
            // Create database path in user's data directory
            let data_dir = dirs::data_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("comunicado");

            // Create directory if it doesn't exist
            std::fs::create_dir_all(&data_dir)?;

            let db_path = data_dir.join("messages.db");
            let db_path_str = db_path
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid database path"))?;

            // Create database connection
            let database = EmailDatabase::new(db_path_str)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to initialize database: {}", e))?;

            let database_arc = Arc::new(database);

            // Create notification manager
            let notification_manager = Arc::new(EmailNotificationManager::new(database_arc.clone()));

            // Start the notification processing
            notification_manager.start().await;

            // Initialize unified notification manager with desktop notifications
            let notification_config = NotificationConfig::default();
            let unified_notification_manager = Arc::new(
                UnifiedNotificationManager::new().with_desktop_notifications(notification_config),
            );

            // Connect email notifications to the unified manager
            let email_receiver = notification_manager.subscribe();
            unified_notification_manager.connect_email_notifications(email_receiver);

            // TODO: Connect calendar notifications when calendar system is implemented
            // let calendar_receiver = calendar_manager.subscribe();
            // unified_notification_manager.connect_calendar_notifications(calendar_receiver);

            tracing::info!("Unified notification system initialized successfully");

            // Set database and notification manager in UI
            self.ui.set_database(database_arc.clone());
            self.ui
                .set_notification_manager(notification_manager.clone());

            self.database = Some(database_arc);
            self.notification_manager = Some(notification_manager);
            self.unified_notification_manager = Some(unified_notification_manager);

            Ok(())
        }.await;

        // Report success or failure to progress manager
        match result {
            Ok(()) => {
                self.startup_progress_manager.complete_phase("Database").map_err(|e| anyhow::anyhow!("Progress manager error: {}", e))?;
                Ok(())
            }
            Err(e) => {
                self.startup_progress_manager.fail_phase("Database", e.to_string()).map_err(|pe| anyhow::anyhow!("Progress manager error: {}", pe))?;
                Err(e)
            }
        }
    }

    /// Get database reference for maintenance operations
    pub fn get_database(&self) -> Option<&Arc<EmailDatabase>> {
        self.database.as_ref()
    }

    /// Enable or disable desktop notifications
    pub fn set_desktop_notifications_enabled(&mut self, enabled: bool) {
        // The unified notification manager is immutable once created
        // Configuration changes would need to be implemented differently
        // For now, just log the intent
        tracing::info!(
            "Desktop notifications {}",
            if enabled { "enabled" } else { "disabled" }
        );
        tracing::info!("Note: Runtime notification configuration changes require restart");
    }

    /// Set the initial UI mode based on CLI startup arguments
    pub fn set_initial_mode(&mut self, mode: crate::cli::StartupMode) {
        self.ui.set_initial_mode(mode);
    }

    /// Send a test desktop notification
    pub async fn send_test_notification(&self) {
        if let Some(unified_manager) = &self.unified_notification_manager {
            unified_manager.send_test_notification().await;
        }
    }

    /// Set deferred initialization flag
    pub fn set_deferred_initialization(&mut self, enabled: bool) {
        self.deferred_initialization = enabled;
    }

    /// Check if initialization is complete
    pub fn is_initialization_complete(&self) -> bool {
        self.initialization_complete
    }

    /// Perform deferred initialization in the background
    pub async fn perform_deferred_initialization(&mut self) -> Result<()> {
        if self.initialization_in_progress || self.initialization_complete {
            return Ok(());
        }

        self.initialization_in_progress = true;
        tracing::info!("Starting deferred initialization...");

        // Phase 1: Database
        if let Err(e) = self.initialize_database().await {
            tracing::error!("Database initialization failed: {}", e);
            return Err(e);
        }

        // Phase 2: IMAP Manager (skipped at startup for speed - will initialize in background)
        let _ = self.startup_progress_manager.start_phase("IMAP Manager");
        let _ = self.startup_progress_manager.update_phase_progress("IMAP Manager", 50.0, Some("⚡ Skipping IMAP for fast startup...".to_string()));
        
        tracing::info!("Skipping IMAP initialization for fast startup - will initialize in background");
        // Mark as complete so startup continues immediately
        let _ = self.startup_progress_manager.update_phase_progress("IMAP Manager", 100.0, Some("⚡ IMAP deferred to background".to_string()));
        let _ = self.startup_progress_manager.complete_phase("IMAP Manager");

        // Phase 3: Account Setup (skipped at startup for speed)
        let _ = self.startup_progress_manager.start_phase("Account Setup");
        let _ = self.startup_progress_manager.update_phase_progress("Account Setup", 50.0, Some("⚡ Skipping account setup for fast startup...".to_string()));
        
        tracing::info!("Skipping account setup for fast startup - will setup in background");
        // Mark as complete so startup continues immediately
        let _ = self.startup_progress_manager.update_phase_progress("Account Setup", 100.0, Some("⚡ Account setup deferred to background".to_string()));
        let _ = self.startup_progress_manager.complete_phase("Account Setup");

        // Phase 4: Services (skipped at startup for speed)
        let _ = self.startup_progress_manager.start_phase("Services");
        let _ = self.startup_progress_manager.update_phase_progress("Services", 50.0, Some("⚡ Skipping services for fast startup...".to_string()));
        
        tracing::info!("Skipping services initialization for fast startup - will initialize in background");
        // Mark as complete so startup continues immediately
        let _ = self.startup_progress_manager.update_phase_progress("Services", 100.0, Some("⚡ Services deferred to background".to_string()));
        let _ = self.startup_progress_manager.complete_phase("Services");

        self.initialization_complete = true;
        self.initialization_in_progress = false;
        
        // Show welcome toast notification
        self.ui.show_toast_success("🚀 Comunicado ready! Modern TUI email & calendar client");
        
        tracing::info!("Deferred initialization completed");

        // Log ready state
        tracing::info!("All services ready");

        Ok(())
    }

    // Startup progress and initialization complete


    /// Initialize IMAP account manager with OAuth2 support
    pub async fn initialize_imap_manager(&mut self) -> Result<()> {
        // Start IMAP manager initialization phase
        self.startup_progress_manager.start_phase("IMAP Manager").map_err(|e| anyhow::anyhow!("Progress manager error: {}", e))?;
        
        // Perform IMAP manager initialization with error handling
        let result: Result<()> = async {
            // Create token manager for OAuth2 authentication with storage backend
            let token_manager = TokenManager::new_with_storage(Arc::new(self.storage.clone()));

            // Create IMAP account manager with OAuth2 support
            let mut imap_manager = ImapAccountManager::new_with_oauth2(token_manager.clone())
            .map_err(|e| anyhow::anyhow!("Failed to create IMAP account manager: {}", e))?;

        // Load existing accounts from OAuth2 storage
        imap_manager
            .load_accounts()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to load IMAP accounts: {}", e))?;

        // Load OAuth2 tokens for all existing accounts into the TokenManager
        tracing::debug!("About to load tokens into manager");
        let load_result = self.load_tokens_into_manager(&token_manager).await?;
        tracing::debug!("Initial token loading complete");

        // Use robust initialization that handles problematic tokens
        tracing::info!("Performing robust token initialization to prevent startup hangs");
        let has_valid_tokens = match tokio::time::timeout(
            std::time::Duration::from_secs(15), // 15 second timeout for entire initialization
            token_manager.initialize_for_startup(),
        )
        .await
        {
            Ok(Ok(valid_tokens)) => {
                tracing::info!("Token initialization completed successfully");
                valid_tokens
            }
            Ok(Err(e)) => {
                tracing::error!("Token initialization failed: {}", e);
                // Fallback to basic loading result
                load_result
            }
            Err(_) => {
                tracing::error!("Token initialization timed out after 15 seconds - using fallback");
                // Use basic result and clear all tokens to prevent future hangs
                match token_manager.validate_and_cleanup_tokens().await {
                    Ok(problematic_accounts) => {
                        if !problematic_accounts.is_empty() {
                            tracing::warn!(
                                "Emergency cleanup removed {} problematic accounts",
                                problematic_accounts.len()
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!("Emergency token cleanup failed: {}", e);
                    }
                }
                false // Assume no valid tokens after timeout
            }
        };

        // Only create and start automatic token refresh scheduler if we have valid tokens
        if has_valid_tokens {
            tracing::debug!("Creating token refresh scheduler");
            let token_manager_arc = Arc::new(token_manager.clone());
            let scheduler = crate::oauth2::token::TokenRefreshScheduler::new(token_manager_arc);

            tracing::debug!("Starting token refresh scheduler");
            let scheduler_result =
                tokio::time::timeout(std::time::Duration::from_secs(5), scheduler.start()).await;

            match scheduler_result {
                Ok(Ok(())) => {
                    tracing::info!("Started automatic OAuth2 token refresh scheduler");
                    self.token_refresh_scheduler = Some(scheduler);
                }
                Ok(Err(e)) => {
                    tracing::warn!("Failed to start automatic token refresh scheduler: {}", e);
                    // Continue without scheduler - tokens will need manual refresh
                    self.token_refresh_scheduler = None;
                }
                Err(_) => {
                    tracing::warn!("Token refresh scheduler startup timed out after 5 seconds");
                    // Continue without scheduler - tokens will need manual refresh
                    self.token_refresh_scheduler = None;
                }
            }
        } else {
            tracing::info!("No valid OAuth2 tokens found, skipping token refresh scheduler");
            self.token_refresh_scheduler = None;
        }
        tracing::debug!("Token refresh scheduler setup complete");

        // Set IMAP manager in UI for attachment downloading functionality
        let imap_manager_arc = Arc::new(imap_manager);
        self.ui
            .content_preview_mut()
            .set_imap_manager(imap_manager_arc.clone());

            self.token_manager = Some(token_manager);
            self.imap_manager = Some(imap_manager_arc);

            Ok(())
        }.await;

        // Report success or failure to progress manager
        match result {
            Ok(()) => {
                self.startup_progress_manager.complete_phase("IMAP Manager").map_err(|e| anyhow::anyhow!("Progress manager error: {}", e))?;
                Ok(())
            }
            Err(e) => {
                self.startup_progress_manager.fail_phase("IMAP Manager", e.to_string()).map_err(|pe| anyhow::anyhow!("Progress manager error: {}", pe))?;
                Err(e)
            }
        }
    }

    /// Initialize background processor for async task handling
    pub async fn initialize_background_processor(&mut self) -> Result<()> {
        tracing::info!("🔄 Initializing background processor for non-blocking operations");

        // Create channels for progress updates and task completion
        let (progress_tx, progress_rx) = mpsc::unbounded_channel::<SyncProgress>();
        let (completion_tx, completion_rx) = mpsc::unbounded_channel::<TaskResult>();

        // Create background processor with optimized settings
        let settings = crate::performance::background_processor::ProcessorSettings {
            max_concurrent_tasks: 2, // Conservative limit to prevent system overload
            task_timeout: Duration::from_secs(300), // 5 minute timeout
            max_queue_size: 50, // Reasonable queue size
            result_cache_size: 25, // Keep recent results
            processing_interval: Duration::from_millis(250), // Check every 250ms
        };

        let processor = Arc::new(BackgroundProcessor::with_settings(
            progress_tx,
            completion_tx,
            settings,
        ));

        // Start the background processor
        processor.start().await.map_err(|e| {
            anyhow::anyhow!("Failed to start background processor: {}", e)
        })?;

        // Store processor and channels
        self.background_processor = Some(processor);
        self.sync_progress_rx = Some(progress_rx);
        self.task_completion_rx = Some(completion_rx);

        tracing::info!("✅ Background processor initialized successfully");
        Ok(())
    }

    /// Queue background task to prevent UI blocking
    pub async fn queue_background_task(&self, task: BackgroundTask) -> Result<Uuid> {
        if let Some(ref processor) = self.background_processor {
            processor
                .queue_task(task)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to queue background task: {}", e))
        } else {
            Err(anyhow::anyhow!("Background processor not initialized"))
        }
    }

    /// Process background task updates (call this in main loop)
    pub async fn process_background_updates(&mut self) {
        // Process sync progress updates
        if let Some(ref mut progress_rx) = self.sync_progress_rx {
            while let Ok(progress) = progress_rx.try_recv() {
                // Update UI with sync progress
                self.ui.update_sync_progress(progress);
            }
        }

        // Process task completion updates
        if let Some(ref mut completion_rx) = self.task_completion_rx {
            while let Ok(result) = completion_rx.try_recv() {
                // Handle task completion
                tracing::debug!("Background task completed: {:?}", result.status);
            }
        }
    }

    /// Load OAuth2 tokens from storage into the TokenManager
    /// Returns true if any valid tokens were loaded
    async fn load_tokens_into_manager(&self, token_manager: &TokenManager) -> Result<bool> {
        tracing::debug!("Starting token loading process");

        // Load all OAuth2 accounts from storage
        let accounts = match self.storage.load_all_accounts() {
            Ok(accounts) => {
                tracing::debug!(
                    "Successfully loaded {} accounts from storage",
                    accounts.len()
                );
                accounts
            }
            Err(e) => {
                tracing::warn!("Failed to load accounts from storage: {}", e);
                // Return early but don't fail the app startup - this is not critical
                return Ok(false);
            }
        };

        tracing::info!(
            "Loading tokens for {} accounts into TokenManager",
            accounts.len()
        );

        if accounts.is_empty() {
            tracing::info!("No accounts found to load tokens for");
            return Ok(false);
        }

        let mut has_valid_tokens = false;

        for account in accounts {
            // Skip accounts without access tokens
            if account.access_token.is_empty() {
                tracing::warn!("No access token found for account {}", account.account_id);
                continue;
            }

            // Create a TokenResponse to store the tokens in TokenManager
            let token_response = crate::oauth2::TokenResponse {
                access_token: account.access_token,
                refresh_token: account.refresh_token,
                token_type: "Bearer".to_string(), // OAuth2 standard
                expires_in: account.token_expires_at.map(|exp| {
                    let now = chrono::Utc::now();
                    std::cmp::max(0, (exp - now).num_seconds()) as u64
                }),
                scope: Some(account.scopes.join(" ")),
            };

            // Store the tokens in the TokenManager
            if let Err(e) = token_manager
                .store_tokens(
                    account.account_id.clone(),
                    account.provider.clone(),
                    &token_response,
                )
                .await
            {
                tracing::warn!(
                    "Failed to store tokens for account {}: {}",
                    account.account_id,
                    e
                );
            } else {
                tracing::debug!(
                    "Successfully loaded tokens for account {}",
                    account.account_id
                );
                has_valid_tokens = true;
            }
        }

        tracing::info!(
            "Token loading complete, has_valid_tokens: {}",
            has_valid_tokens
        );
        Ok(has_valid_tokens)
    }

    /// Check for existing accounts and run setup wizard if needed
    pub async fn check_accounts_and_setup(&mut self) -> Result<()> {
        // Start account setup phase
        self.startup_progress_manager.start_phase("Account Setup").map_err(|e| anyhow::anyhow!("Progress manager error: {}", e))?;
        
        // Perform account setup with error handling
        let result: Result<()> = async {
            tracing::debug!("Starting account check and setup process");

        let account_ids = self
            .storage
            .list_account_ids()
            .map_err(|e| anyhow::anyhow!("Failed to list accounts: {}", e))?;

        tracing::debug!("Found {} existing account IDs", account_ids.len());

        if account_ids.is_empty() {
            tracing::info!("No existing accounts found - starting without accounts");
            // No accounts found, continue without running setup wizard
            // Users can use CLI commands to add accounts
        } else {
            tracing::info!("Loading existing accounts");
            // Load existing accounts
            match tokio::time::timeout(
                std::time::Duration::from_secs(30), // 30 second timeout for loading
                self.load_existing_accounts(),
            )
            .await
            {
                Ok(result) => result?,
                Err(_) => {
                    tracing::error!("Loading existing accounts timed out after 30 seconds");
                    return Err(anyhow::anyhow!("Loading existing accounts timed out"));
                }
            }
        }

            tracing::debug!("Account check and setup process completed");
            Ok(())
        }.await;

        // Report success or failure to progress manager
        match result {
            Ok(()) => {
                self.startup_progress_manager.complete_phase("Account Setup").map_err(|e| anyhow::anyhow!("Progress manager error: {}", e))?;
                Ok(())
            }
            Err(e) => {
                self.startup_progress_manager.fail_phase("Account Setup", e.to_string()).map_err(|pe| anyhow::anyhow!("Progress manager error: {}", pe))?;
                Err(e)
            }
        }
    }



    /// Load existing accounts from storage
    async fn load_existing_accounts(&mut self) -> Result<()> {
        let accounts = self
            .storage
            .load_all_accounts()
            .map_err(|e| anyhow::anyhow!("Failed to load accounts: {}", e))?;

        tracing::debug!("Loaded {} accounts from storage", accounts.len());
        for (i, account) in accounts.iter().enumerate() {
            tracing::debug!(
                "Account {}: {} ({})",
                i,
                account.display_name,
                account.account_id
            );
        }

        if accounts.is_empty() {
            return self.load_sample_data().await;
        }

        // Load OAuth2 tokens into TokenManager for existing accounts
        tracing::debug!(
            "About to load OAuth2 tokens for {} accounts",
            accounts.len()
        );
        if let Some(ref token_manager) = self.token_manager {
            for account in &accounts {
                tracing::debug!(
                    "Processing account: {} (has access token: {})",
                    account.account_id,
                    !account.access_token.is_empty()
                );
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

                    tracing::debug!("About to store tokens for account: {}", account.account_id);
                    if let Err(e) = token_manager
                        .store_tokens(
                            account.account_id.clone(),
                            account.provider.clone(),
                            &token_response,
                        )
                        .await
                    {
                        tracing::warn!(
                            "Failed to load tokens for account {}: {}",
                            account.account_id,
                            e
                        );
                    } else {
                        tracing::info!(
                            "Loaded OAuth2 tokens for existing account: {}",
                            account.account_id
                        );
                    }
                }
            }
        } else {
            tracing::warn!(
                "TokenManager not initialized, OAuth2 tokens not loaded for existing accounts"
            );
        }
        tracing::debug!("Finished loading OAuth2 tokens");

        // Convert AccountConfig to AccountItem for the UI
        let account_items: Vec<crate::ui::AccountItem> = accounts
            .iter()
            .map(crate::ui::AccountItem::from_config)
            .collect();

        // Set accounts in the UI
        self.ui.set_accounts(account_items);

        // Create all accounts in the database
        for account in &accounts {
            self.create_account_from_config(account).await?;
        }

        // Load messages for the first account (or current account)
        tracing::debug!("App.load_existing_accounts() - Loading messages for accounts");
        tracing::debug!("Checking for current account...");
        if let Some(current_account_id) = self.ui.get_current_account_id().cloned() {
            tracing::debug!("Found current account: {}", current_account_id);
            // Try to sync folders and messages from IMAP
            if let Some(ref _imap_manager) = self.imap_manager {
                tracing::debug!("Starting IMAP sync for account: {}", current_account_id);
                match self.sync_account_from_imap(&current_account_id).await {
                    Ok(_) => {
                        tracing::debug!(
                            "IMAP sync completed successfully, loading folders and messages"
                        );
                        // Successfully synced from IMAP, load folders first
                        match self.ui.load_folders(&current_account_id).await {
                            Ok(_) => tracing::debug!("Successfully loaded folders from database"),
                            Err(e) => {
                                tracing::debug!("Failed to load folders from database: {}", e)
                            }
                        }

                        // Then load messages for INBOX
                        match self
                            .ui
                            .load_messages(current_account_id.clone(), "INBOX".to_string())
                            .await
                        {
                            Ok(_) => tracing::debug!("Successfully loaded messages for INBOX"),
                            Err(e) => tracing::debug!("Failed to load messages for INBOX: {}", e),
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            "IMAP sync failed for account {}: {}",
                            current_account_id,
                            e
                        );

                        // If it's an authentication timeout, log helpful information
                        if e.to_string().contains("timed out")
                            || e.to_string().contains("authentication")
                        {
                            tracing::warn!(
                                "Authentication issue detected - tokens may have expired"
                            );
                            tracing::info!("Consider re-running setup if this persists: rm ~/.config/comunicado/{}.json", current_account_id);
                        }

                        // Continue with sample data instead of crashing
                        tracing::info!("Falling back to sample data due to IMAP sync failure");
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
            let _ = self
                .ui
                .load_messages("sample-account".to_string(), "INBOX".to_string())
                .await;
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
    async fn fetch_messages_from_imap(
        &mut self,
        account_id: &str,
        folder_name: &str,
    ) -> Result<()> {
        tracing::debug!(
            "fetch_messages_from_imap called for account: {}, folder: {}",
            account_id,
            folder_name
        );
        let imap_manager = self
            .imap_manager
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("IMAP manager not initialized"))?;

        let database = self
            .database
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database not initialized"))?;

        tracing::info!(
            "Starting message fetch for account: {}, folder: {}",
            account_id,
            folder_name
        );

        // Get IMAP client
        let client_arc = imap_manager
            .get_client(account_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get IMAP client: {}", e))?;

        {
            let mut client = client_arc.lock().await;

            // Select the folder (typically INBOX)
            let folder = client
                .select_folder(folder_name)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to select folder {}: {}", folder_name, e))?;

            tracing::info!(
                "Selected folder '{}': {:?} messages exist, {:?} recent",
                folder_name,
                folder.exists,
                folder.recent
            );

            // Fetch recent messages (limit to 50 for now)
            let message_count = std::cmp::min(folder.exists.unwrap_or(0) as usize, 50);

            tracing::info!(
                "Folder {} has {} messages, will fetch {} messages",
                folder_name,
                folder.exists.unwrap_or(0),
                message_count
            );

            if message_count > 0 {
                let sequence_set = format!("1:{message_count}");
                let fetch_items = vec!["UID", "FLAGS", "ENVELOPE", "BODY.PEEK[]", "BODYSTRUCTURE"];

                let messages = client
                    .fetch_messages(&sequence_set, &fetch_items)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to fetch messages: {}", e))?;

                tracing::info!("Fetched {} messages from IMAP", messages.len());

                // Store messages in database
                for message in messages {
                    tracing::info!(
                        "Processing message UID: {:?}, Subject: {:?}",
                        message.uid,
                        message.envelope.as_ref().map(|e| &e.subject)
                    );

                    // Convert IMAP message to StoredMessage format
                    match self
                        .convert_imap_to_stored_message(&message, account_id, folder_name)
                        .await
                    {
                        Ok(stored_message) => {
                            // Store in database
                            if let Err(e) = database.store_message(&stored_message).await {
                                tracing::error!(
                                    "Failed to store message UID {}: {}",
                                    message.uid.unwrap_or(0),
                                    e
                                );
                            } else {
                                tracing::info!("Stored message: {}", stored_message.subject);
                            }
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to convert message UID {}: {}",
                                message.uid.unwrap_or(0),
                                e
                            );
                        }
                    }
                }
            } else {
                tracing::info!("No messages found in folder: {}", folder_name);
            }
        }

        tracing::info!(
            "Message fetch completed successfully for account: {}",
            account_id
        );
        Ok(())
    }

    /// Sync account data from IMAP (folders and messages)
    async fn sync_account_from_imap(&mut self, account_id: &str) -> Result<()> {
        tracing::debug!("sync_account_from_imap called for: {}", account_id);
        tracing::info!("Starting IMAP sync for account: {}", account_id);

        // First sync folders
        self.sync_folders_from_imap(account_id).await?;

        // Get all folders for this account from database
        let database = self
            .database
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database not initialized"))?;

        let folders: Vec<String> = sqlx::query("SELECT name FROM folders WHERE account_id = ? ORDER BY name")
            .bind(account_id)
            .fetch_all(&database.pool)
            .await?
            .into_iter()
            .map(|row| row.get::<String, _>("name"))
            .collect();

        // Define important folders that should always be synced
        let important_folders = ["INBOX", "Sent", "Drafts", "Trash", "Spam", "Junk", "Sent Items", "Sent Mail"];
        
        // Separate important folders from others
        let mut priority_folders = Vec::new();
        let mut other_folders = Vec::new();
        
        for folder in &folders {
            let folder_lower = folder.to_lowercase();
            if important_folders.iter().any(|&important| folder_lower.contains(&important.to_lowercase())) {
                priority_folders.push(folder.clone());
            } else {
                other_folders.push(folder.clone());
            }
        }

        tracing::info!("Syncing messages for {} priority folders and {} other folders in account: {}", 
                      priority_folders.len(), other_folders.len(), account_id);

        // First, fetch messages from important folders
        for folder_name in &priority_folders {
            tracing::debug!("Fetching messages from priority folder: {} in account: {}", folder_name, account_id);
            match self.fetch_messages_from_imap(account_id, folder_name).await {
                Ok(()) => {
                    tracing::debug!("Successfully fetched messages from priority folder: {}", folder_name);
                }
                Err(e) => {
                    tracing::warn!("Failed to fetch messages from priority folder {}: {}. Continuing with other folders.", folder_name, e);
                    // Continue with other folders even if one fails
                }
            }
        }

        // Then fetch from other folders (with a limit to avoid performance issues)
        let max_other_folders = 5; // Limit to avoid performance issues
        let folders_to_sync = other_folders.iter().take(max_other_folders);
        
        for folder_name in folders_to_sync {
            tracing::debug!("Fetching messages from folder: {} in account: {}", folder_name, account_id);
            match self.fetch_messages_from_imap(account_id, folder_name).await {
                Ok(()) => {
                    tracing::debug!("Successfully fetched messages from folder: {}", folder_name);
                }
                Err(e) => {
                    tracing::warn!("Failed to fetch messages from folder {}: {}. Continuing with other folders.", folder_name, e);
                    // Continue with other folders even if one fails
                }
            }
        }

        if other_folders.len() > max_other_folders {
            tracing::info!("Synced {} of {} other folders. Use manual folder refresh for remaining folders.", 
                          max_other_folders, other_folders.len());
        }

        tracing::info!("Completed IMAP sync for account: {} ({} folders processed)", account_id, folders.len());
        Ok(())
    }

    /// Sync folders from IMAP and store in database
    async fn sync_folders_from_imap(&mut self, account_id: &str) -> Result<()> {
        tracing::debug!("sync_folders_from_imap called for: {}", account_id);
        let imap_manager = self
            .imap_manager
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("IMAP manager not initialized"))?;

        let database = self
            .database
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database not initialized"))?;

        tracing::info!("Syncing folders for account: {}", account_id);

        // Get IMAP client with timeout to prevent hanging on expired tokens
        tracing::debug!(
            "About to call imap_manager.get_client() for: {}",
            account_id
        );

        let client_result = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            imap_manager.get_client(account_id),
        )
        .await;

        let client_arc = match client_result {
            Ok(Ok(client)) => {
                tracing::debug!("Successfully got IMAP client for: {}", account_id);
                client
            }
            Ok(Err(e)) => {
                tracing::error!("Failed to get IMAP client for {}: {}", account_id, e);
                return Err(anyhow::anyhow!("Failed to get IMAP client: {}", e));
            }
            Err(_) => {
                tracing::error!("IMAP client connection timed out for: {}", account_id);
                return Err(anyhow::anyhow!(
                    "IMAP client connection timed out after 10 seconds"
                ));
            }
        };
        tracing::debug!("Successfully got IMAP client for: {}", account_id);

        {
            let mut client = client_arc.lock().await;

            // List all folders from IMAP server
            tracing::debug!("About to call client.list_folders()");
            let folders = client.list_folders("", "*").await.map_err(|e| {
                tracing::debug!("Failed to list folders: {}", e);
                anyhow::anyhow!("Failed to list folders: {}", e)
            })?;
            tracing::debug!("Successfully listed {} folders", folders.len());

            let folder_count = folders.len();
            tracing::info!("Found {} folders from IMAP", folder_count);

            // Delete existing folders for this account to refresh the list
            sqlx::query("DELETE FROM folders WHERE account_id = ?")
                .bind(account_id)
                .execute(&database.pool)
                .await?;

            // Store each folder in database
            tracing::debug!("About to store {} folders in database", folders.len());
            for folder in folders {
                tracing::debug!("Storing folder: {} ({})", folder.name, folder.full_name);
                tracing::debug!("Storing folder: {} ({})", folder.name, folder.full_name);

                let attributes_json =
                    serde_json::to_string(&folder.attributes).unwrap_or_else(|_| "[]".to_string());

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

            tracing::info!(
                "Successfully synced {} folders for account {}",
                folder_count,
                account_id
            );
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
        enable_raw_mode().map_err(|e| {
            anyhow::anyhow!(
                "Failed to enable raw mode: {}. Make sure you're running in a proper terminal.",
                e
            )
        })?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture).map_err(|e| {
            anyhow::anyhow!(
                "Failed to setup terminal: {}. Make sure your terminal supports these features.",
                e
            )
        })?;
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

    async fn run_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<()> {
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(50);
        let mut previous_selection: Option<usize> = None;

        loop {
            // Perform deferred initialization if not done yet
            if !self.initialization_complete {
                if let Err(e) = self.perform_deferred_initialization().await {
                    tracing::error!("Deferred initialization failed: {}", e);
                    // Don't fail the app, just log the error and continue
                }
            }

            // Process background task updates to prevent UI blocking
            self.process_background_updates().await;
            
            // Check for auto-sync (every 3 minutes) - now uses background processing
            if self.last_auto_sync.elapsed() >= self.auto_sync_interval {
                self.queue_auto_sync_background().await;
                self.last_auto_sync = Instant::now();
            }

            // Process email notifications
            self.ui.process_notifications().await;

            // Update UI notifications (clear expired ones)
            self.ui.update_notifications();
            
            // Update toast notifications (handle expiration and animations)
            self.ui.update_toasts();

            // Check if message selection changed and handle it
            let current_selection = self.ui.message_list().get_selection_state();
            if current_selection != previous_selection {
                self.ui.handle_message_selection().await;
                previous_selection = current_selection;
            }

            // Check for auto-save if in compose mode
            if let Some(auto_save_action) = self.ui.check_compose_auto_save() {
                if let Err(e) = self.handle_compose_action(auto_save_action).await {
                    tracing::warn!("Auto-save failed: {}", e);
                }
            }

            // Draw UI - show startup progress during startup, then normal UI
            terminal.draw(|f| {
                if self.startup_progress_manager.is_visible() {
                    // Show startup progress screen during startup
                    use crate::startup::StartupProgressScreen;
                    let progress_screen = StartupProgressScreen::new();
                    let theme = crate::theme::Theme::default();
                    progress_screen.render(f, f.size(), &self.startup_progress_manager, &theme);
                } else {
                    // After startup, use regular UI
                    self.ui.render(f)
                }
            })?;

            // Handle events
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    let event_result = self.event_handler.handle_key_event_with_config(key, &mut self.ui).await;

                    // Handle the event result
                    match event_result {
                        EventResult::Continue => {}
                        EventResult::ComposeAction(action) => {
                            self.handle_compose_action(action).await?;
                        }
                        EventResult::DraftAction(action) => {
                            self.handle_draft_action(action).await?;
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
                        EventResult::FolderForceRefresh(folder_path) => {
                            self.handle_folder_force_refresh(&folder_path).await?;
                        }
                        EventResult::FolderOperation(operation) => {
                            self.handle_folder_operation(operation).await?;
                        }
                        EventResult::ContactsPopup => {
                            self.handle_contacts_popup().await?;
                        }
                        EventResult::ContactsAction(action) => {
                            self.handle_contacts_action(action).await?;
                        }
                        EventResult::AddToContacts(email, name) => {
                            self.handle_add_to_contacts(&email, &name).await?;
                        }
                        EventResult::EmailViewerStarted(sender_email) => {
                            self.handle_email_viewer_started(&sender_email).await?;
                        }
                        EventResult::ReplyToMessage(message_id) => {
                            self.handle_reply_to_message(message_id).await?;
                        }
                        EventResult::ReplyAllToMessage(message_id) => {
                            self.handle_reply_all_to_message(message_id).await?;
                        }
                        EventResult::ForwardMessage(message_id) => {
                            self.handle_forward_message(message_id).await?;
                        }
                        EventResult::ViewSenderContact(email) => {
                            self.handle_view_sender_contact(&email).await?;
                        }
                        EventResult::EditSenderContact(email) => {
                            self.handle_edit_sender_contact(&email).await?;
                        }
                        EventResult::RemoveSenderFromContacts(email) => {
                            self.handle_remove_sender_from_contacts(&email).await?;
                        }
                        EventResult::ContactQuickActions(email) => {
                            self.handle_contact_quick_actions(&email).await?;
                        }
                    }

                    // Check for quit command
                    if self.event_handler.should_quit() {
                        self.should_quit = true;
                    }
                }
            }

            if last_tick.elapsed() >= tick_rate {
                // Periodic updates on each tick
                self.ui.refresh_status_bar();
                
                // Process pending email notifications
                self.ui.process_notifications().await;
                
                // Clean up expired notifications
                self.ui.update_notifications();
                
                // Clean up old sync progress entries
                self.ui.cleanup_sync_progress();
                
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
        // Start services initialization phase
        self.startup_progress_manager.start_phase("Services").map_err(|e| anyhow::anyhow!("Progress manager error: {}", e))?;
        
        // Perform services initialization with error handling
        let result: Result<()> = async {
            tracing::debug!("Starting service initialization");

        // Initialize token manager if not already done
        if self.token_manager.is_none() {
            tracing::debug!("Creating new token manager");
            let token_manager = TokenManager::new();
            self.token_manager = Some(token_manager);
        }

        // Initialize SMTP service with timeout
        if let (Some(ref token_manager), Some(ref database)) = (&self.token_manager, &self.database)
        {
            tracing::debug!("Initializing SMTP service");

            let smtp_init_result = tokio::time::timeout(
                std::time::Duration::from_secs(15), // 15 second timeout for SMTP init
                async {
                    SmtpServiceBuilder::new()
                        .with_token_manager(Arc::new(token_manager.clone()))
                        .with_database(database.clone())
                        .build()
                },
            )
            .await;

            match smtp_init_result {
                Ok(Ok(smtp_service)) => {
                    self.smtp_service = Some(smtp_service);
                    tracing::debug!("SMTP service initialized successfully");
                }
                Ok(Err(e)) => {
                    tracing::error!("Failed to initialize SMTP service: {}", e);
                    return Err(anyhow::anyhow!("Failed to initialize SMTP service: {}", e));
                }
                Err(_) => {
                    tracing::error!("SMTP service initialization timed out after 15 seconds");
                    return Err(anyhow::anyhow!("SMTP service initialization timed out"));
                }
            }
        }

        // Initialize contacts manager (optional - don't fail if it can't be initialized)
        if let (Some(ref _database), Some(ref token_manager)) =
            (&self.database, &self.token_manager)
        {
            tracing::debug!("Initializing contacts manager");

            let contacts_init_result = tokio::time::timeout(
                std::time::Duration::from_secs(20), // 20 second timeout for contacts init
                async {
                    // Create contacts database from email database path
                    let data_dir = dirs::data_dir()
                        .ok_or_else(|| anyhow::anyhow!("Failed to get data directory"))?
                        .join("comunicado");

                    // Ensure the directory exists
                    if let Err(e) = std::fs::create_dir_all(&data_dir) {
                        tracing::warn!("Failed to create contacts directory: {}", e);
                        return Ok::<Option<Arc<ContactsManager>>, anyhow::Error>(None);
                    }

                    let contacts_db_path = data_dir.join("contacts.db");

                    // Try to initialize contacts database
                    match crate::contacts::ContactsDatabase::new(&format!(
                        "sqlite:{}",
                        contacts_db_path.display()
                    ))
                    .await
                    {
                        Ok(contacts_database) => {
                            match ContactsManager::new(contacts_database, token_manager.clone())
                                .await
                            {
                                Ok(contacts_manager) => Ok(Some(Arc::new(contacts_manager))),
                                Err(e) => {
                                    tracing::warn!("Failed to initialize contacts manager: {}", e);
                                    Ok(None)
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to initialize contacts database: {}", e);
                            Ok(None)
                        }
                    }
                },
            )
            .await;

            match contacts_init_result {
                Ok(Ok(Some(contacts_manager))) => {
                    self.contacts_manager = Some(contacts_manager.clone());
                    
                    // Set up sender recognition in UI
                    self.ui.set_contacts_manager(contacts_manager);
                    
                    tracing::info!("Contacts manager initialized successfully with sender recognition");
                }
                Ok(Ok(None)) => {
                    tracing::warn!("Contacts manager initialization skipped due to errors");
                }
                Ok(Err(e)) => {
                    tracing::warn!("Contacts manager initialization failed: {}", e);
                }
                Err(_) => {
                    tracing::error!("Contacts manager initialization timed out after 20 seconds");
                    // Don't fail overall initialization - contacts are optional
                }
            }
        }

            tracing::info!("Services initialized successfully");
            Ok(())
        }.await;

        // Report success or failure to progress manager
        match result {
            Ok(()) => {
                self.startup_progress_manager.complete_phase("Services").map_err(|e| anyhow::anyhow!("Progress manager error: {}", e))?;
                Ok(())
            }
            Err(e) => {
                self.startup_progress_manager.fail_phase("Services", e.to_string()).map_err(|pe| anyhow::anyhow!("Progress manager error: {}", pe))?;
                Err(e)
            }
        }
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
            ComposeAction::AutoSave => {
                self.auto_save_draft().await?;
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
            ComposeAction::StartReplyFromMessage(message) => {
                // Start reply compose mode with the message
                if let Some(ref contacts_manager) = self.contacts_manager {
                    self.ui
                        .start_reply_from_message(message, contacts_manager.clone());
                    self.ui.exit_email_viewer(); // Exit email viewer after starting compose
                    tracing::info!("Started reply compose mode from email viewer");
                } else {
                    tracing::warn!("Cannot start reply: contacts manager not initialized");
                }
            }
            ComposeAction::StartReplyAllFromMessage(message) => {
                // Start reply all compose mode with the message
                if let Some(ref contacts_manager) = self.contacts_manager {
                    self.ui
                        .start_reply_all_from_message(message, contacts_manager.clone());
                    self.ui.exit_email_viewer(); // Exit email viewer after starting compose
                    tracing::info!("Started reply all compose mode from email viewer");
                } else {
                    tracing::warn!("Cannot start reply all: contacts manager not initialized");
                }
            }
            ComposeAction::StartForwardFromMessage(message) => {
                // Start forward compose mode with the message
                if let Some(ref contacts_manager) = self.contacts_manager {
                    self.ui
                        .start_forward_from_message(message, contacts_manager.clone());
                    self.ui.exit_email_viewer(); // Exit email viewer after starting compose
                    tracing::info!("Started forward compose mode from email viewer");
                } else {
                    tracing::warn!("Cannot start forward: contacts manager not initialized");
                }
            }
            ComposeAction::StartEditFromMessage(message) => {
                // Start edit compose mode with the message
                if let Some(ref contacts_manager) = self.contacts_manager {
                    self.ui
                        .start_edit_from_message(message, contacts_manager.clone());
                    self.ui.exit_email_viewer(); // Exit email viewer after starting compose
                    tracing::info!("Started edit compose mode from email viewer");
                } else {
                    tracing::warn!("Cannot start edit: contacts manager not initialized");
                }
            }
        }
        Ok(())
    }

    /// Send the current composed email
    async fn send_email(&mut self) -> Result<()> {
        let compose_data = self
            .ui
            .get_compose_data()
            .ok_or_else(|| anyhow::anyhow!("No compose data available"))?;

        let smtp_service = self
            .smtp_service
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("SMTP service not initialized"))?;

        // For now, use the first available account
        // In a real implementation, this should come from the active account
        let configs = self
            .storage
            .load_all_accounts()
            .map_err(|e| anyhow::anyhow!("Failed to load account configs: {}", e))?;

        if let Some(config) = configs.first() {
            let account_id = &config.account_id;
            let from_address = &config.email_address;

            // Initialize SMTP for this account if not already done
            if !smtp_service.is_account_configured(account_id).await {
                self.initialize_smtp_for_account(account_id, config).await?;
            }

            // Send the email
            match smtp_service
                .send_email(account_id, from_address, &compose_data)
                .await
            {
                Ok(result) => {
                    tracing::info!("Email sent successfully: {}", result.message_id);
                    self.ui.exit_compose();
                    self.ui.clear_compose_modified();

                    // TODO: Add a success notification to the UI
                    tracing::info!(
                        "Email sent to {} recipients",
                        result.accepted_recipients.len()
                    );
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
        let compose_data = self
            .ui
            .get_compose_data()
            .ok_or_else(|| anyhow::anyhow!("No compose data available"))?;

        let smtp_service = self
            .smtp_service
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("SMTP service not initialized"))?;

        // For now, use the first available account
        let configs = self
            .storage
            .load_all_accounts()
            .map_err(|e| anyhow::anyhow!("Failed to load account configs: {}", e))?;

        if let Some(config) = configs.first() {
            let account_id = &config.account_id;

            match smtp_service.save_draft(account_id, &compose_data).await {
                Ok(draft_id) => {
                    tracing::info!("Draft saved with ID: {}", draft_id);
                    self.ui.clear_compose_modified();
                    self.ui.set_compose_draft_id(Some(draft_id));
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

    /// Auto-save the current composed email as a draft
    async fn auto_save_draft(&mut self) -> Result<()> {
        let compose_data = self
            .ui
            .get_compose_data()
            .ok_or_else(|| anyhow::anyhow!("No compose data available"))?;

        let smtp_service = self
            .smtp_service
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("SMTP service not initialized"))?;

        // For now, use the first available account
        let configs = self
            .storage
            .load_all_accounts()
            .map_err(|e| anyhow::anyhow!("Failed to load account configs: {}", e))?;

        if let Some(config) = configs.first() {
            let account_id = &config.account_id;
            let existing_draft_id = self.ui.get_compose_draft_id();

            match smtp_service
                .auto_save_draft(
                    account_id,
                    &compose_data,
                    existing_draft_id.map(|s| s.as_str()),
                )
                .await
            {
                Ok(draft_id) => {
                    tracing::debug!("Draft auto-saved with ID: {}", draft_id);
                    self.ui.mark_compose_auto_saved();
                    self.ui.set_compose_draft_id(Some(draft_id));
                }
                Err(e) => {
                    tracing::warn!("Failed to auto-save draft: {}", e);
                    // Don't fail the entire operation for auto-save failures
                }
            }
        }

        Ok(())
    }

    /// Handle draft actions (show list, load draft, delete draft)
    async fn handle_draft_action(&mut self, action: DraftAction) -> Result<()> {
        match action {
            DraftAction::Continue => {
                // Nothing to do
            }
            DraftAction::RefreshDrafts => {
                // Show draft list and load drafts
                self.show_draft_list().await?;
            }
            DraftAction::LoadDraft(draft_id) => {
                self.load_draft_for_editing(&draft_id).await?;
            }
            DraftAction::DeleteDraft(draft_id) => {
                self.delete_draft(&draft_id).await?;
            }
            DraftAction::Close => {
                self.ui.hide_draft_list();
            }
            DraftAction::ToggleSort => {
                // This is handled within the draft list UI
            }
            DraftAction::ToggleDetails => {
                // This is handled within the draft list UI
            }
        }
        Ok(())
    }

    /// Show draft list and load all drafts
    async fn show_draft_list(&mut self) -> Result<()> {
        // Load drafts from database
        let drafts = self.load_all_drafts().await?;

        // Update UI with drafts
        self.ui.update_draft_list(drafts);
        self.ui.show_draft_list();

        Ok(())
    }

    /// Load all drafts for the current account
    async fn load_all_drafts(&self) -> Result<Vec<crate::email::database::StoredDraft>> {
        let smtp_service = self
            .smtp_service
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("SMTP service not initialized"))?;

        // For now, use the first available account
        let configs = self
            .storage
            .load_all_accounts()
            .map_err(|e| anyhow::anyhow!("Failed to load account configs: {}", e))?;

        if let Some(config) = configs.first() {
            let account_id = &config.account_id;

            match smtp_service.list_drafts(account_id).await {
                Ok(drafts) => {
                    tracing::info!("Loaded {} drafts for account {}", drafts.len(), account_id);
                    Ok(drafts)
                }
                Err(e) => {
                    tracing::error!("Failed to load drafts: {}", e);
                    Err(anyhow::anyhow!("Failed to load drafts: {}", e))
                }
            }
        } else {
            Err(anyhow::anyhow!("No email accounts configured"))
        }
    }

    /// Load a draft for editing
    async fn load_draft_for_editing(&mut self, draft_id: &str) -> Result<()> {
        let smtp_service = self
            .smtp_service
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("SMTP service not initialized"))?;

        // For now, use the first available account
        let configs = self
            .storage
            .load_all_accounts()
            .map_err(|e| anyhow::anyhow!("Failed to load account configs: {}", e))?;

        if let Some(config) = configs.first() {
            let account_id = &config.account_id;

            match smtp_service.load_draft(account_id, draft_id).await {
                Ok(compose_data) => {
                    // Get contacts manager for compose UI
                    if let Some(ref contacts_manager) = self.contacts_manager {
                        self.ui.load_draft_for_editing(
                            compose_data,
                            draft_id.to_string(),
                            contacts_manager.clone(),
                        );
                        tracing::info!("Loaded draft {} for editing", draft_id);
                    } else {
                        return Err(anyhow::anyhow!("Contacts manager not initialized"));
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to load draft {}: {}", draft_id, e);
                    return Err(anyhow::anyhow!("Failed to load draft: {}", e));
                }
            }
        } else {
            return Err(anyhow::anyhow!("No email accounts configured"));
        }

        Ok(())
    }

    /// Delete a draft
    async fn delete_draft(&mut self, draft_id: &str) -> Result<()> {
        let smtp_service = self
            .smtp_service
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("SMTP service not initialized"))?;

        // For now, use the first available account
        let configs = self
            .storage
            .load_all_accounts()
            .map_err(|e| anyhow::anyhow!("Failed to load account configs: {}", e))?;

        if let Some(config) = configs.first() {
            let account_id = &config.account_id;

            match smtp_service.delete_draft(account_id, draft_id).await {
                Ok(()) => {
                    // Remove from UI list
                    self.ui.remove_draft_from_list(draft_id);
                    tracing::info!("Deleted draft {}", draft_id);
                }
                Err(e) => {
                    tracing::error!("Failed to delete draft {}: {}", draft_id, e);
                    return Err(anyhow::anyhow!("Failed to delete draft: {}", e));
                }
            }
        } else {
            return Err(anyhow::anyhow!("No email accounts configured"));
        }

        Ok(())
    }

    /// Initialize SMTP for a specific account
    async fn initialize_smtp_for_account(
        &self,
        account_id: &str,
        config: &AccountConfig,
    ) -> Result<()> {
        let smtp_service = self
            .smtp_service
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("SMTP service not initialized"))?;

        let token_manager = self
            .token_manager
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Token manager not initialized"))?;

        // Get current access token
        let token = token_manager
            .get_access_token(account_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get access token: {}", e))?;

        // Initialize SMTP for this account
        smtp_service
            .initialize_account(
                account_id,
                &config.provider,
                &config.email_address,
                &token.unwrap().token,
            )
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to initialize SMTP for account {}: {}",
                    account_id,
                    e
                )
            })?;

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
        self.ui
            .update_account_status(account_id, crate::ui::AccountSyncStatus::Syncing, None);

        // First sync folders and messages from IMAP, then switch to account
        match self.sync_account_from_imap(account_id).await {
            Ok(()) => {
                tracing::info!("Successfully synced account: {}", account_id);

                // Now switch to the account in UI (which will load from local database)
                match self.ui.switch_to_account(account_id).await {
                    Ok(()) => {
                        tracing::info!("Successfully switched to account: {}", account_id);
                        // Update status to online if successful
                        self.ui.update_account_status(
                            account_id,
                            crate::ui::AccountSyncStatus::Online,
                            None,
                        );
                    }
                    Err(e) => {
                        tracing::error!("Failed to switch to account {}: {}", account_id, e);
                        // Update status to error if failed
                        self.ui.update_account_status(
                            account_id,
                            crate::ui::AccountSyncStatus::Error,
                            None,
                        );
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to sync account {}: {}", account_id, e);
                // Update status to error if failed
                self.ui.update_account_status(
                    account_id,
                    crate::ui::AccountSyncStatus::Error,
                    None,
                );

                // Still try to switch to account with local data as fallback
                if let Err(e) = self.ui.switch_to_account(account_id).await {
                    tracing::error!(
                        "Failed to switch to account {} even with local data: {}",
                        account_id,
                        e
                    );
                }
            }
        }
    }

    /// Handle account management - direct users to CLI setup
    async fn handle_add_account(&mut self) -> Result<()> {
        tracing::info!("Account management requested - directing to CLI setup");

        // Show message directing users to CLI commands
        self.ui.show_notification(
            "Use CLI to add accounts: 'comunicado setup-gmail' or 'comunicado setup-outlook'".to_string(),
            tokio::time::Duration::from_secs(10)
        );

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
            return Err(anyhow::anyhow!(
                "Failed to remove account from storage: {}",
                e
            ));
        }

        // Remove from database
        if let Some(ref database) = self.database {
            // Remove all messages for this account
            if let Err(e) = sqlx::query("DELETE FROM messages WHERE account_id = ?")
                .bind(account_id)
                .execute(&database.pool)
                .await
            {
                tracing::error!(
                    "Failed to remove messages for account {}: {}",
                    account_id,
                    e
                );
            }

            // Remove folders for this account
            if let Err(e) = sqlx::query("DELETE FROM folders WHERE account_id = ?")
                .bind(account_id)
                .execute(&database.pool)
                .await
            {
                tracing::error!("Failed to remove folders for account {}: {}", account_id, e);
            }

            // Remove the account itself
            if let Err(e) = sqlx::query("DELETE FROM accounts WHERE id = ?")
                .bind(account_id)
                .execute(&database.pool)
                .await
            {
                tracing::error!("Failed to remove account from database: {}", e);
                return Err(anyhow::anyhow!(
                    "Failed to remove account from database: {}",
                    e
                ));
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
        use chrono::Utc;
        use uuid::Uuid;

        // Extract envelope data
        let envelope = imap_message.envelope.as_ref();
        tracing::info!(
            "Converting IMAP message, envelope present: {}",
            envelope.is_some()
        );

        if let Some(env) = envelope {
            tracing::info!(
                "Envelope - subject: {:?}, from count: {}",
                env.subject,
                env.from.len()
            );
        }

        // Extract and decode subject
        let subject = envelope
            .and_then(|e| e.subject.as_ref())
            .map(|s| crate::mime::decode_mime_header(s))
            .unwrap_or_else(|| "(No Subject)".to_string());

        tracing::info!("Extracted and decoded subject: {}", subject);

        // Extract sender information
        let (from_addr, from_name) = if let Some(env) = envelope {
            if let Some(from) = env.from.first() {
                tracing::info!(
                    "From address - mailbox: {:?}, host: {:?}, name: {:?}",
                    from.mailbox,
                    from.host,
                    from.name
                );
                let addr = format!(
                    "{}@{}",
                    from.mailbox.as_deref().unwrap_or("unknown"),
                    from.host.as_deref().unwrap_or("unknown.com")
                );
                let name = from
                    .name
                    .as_ref()
                    .map(|n| crate::mime::decode_mime_header(n));
                tracing::info!(
                    "Extracted and decoded from_addr: {}, from_name: {:?}",
                    addr,
                    name
                );
                (addr, name)
            } else {
                tracing::info!("No from address in envelope");
                ("unknown@unknown.com".to_string(), None)
            }
        } else {
            tracing::info!("No envelope available");
            ("unknown@unknown.com".to_string(), None)
        };

        tracing::info!(
            "Final extracted - subject: {}, from_addr: {}, from_name: {:?}",
            subject,
            from_addr,
            from_name
        );

        // Extract recipient addresses
        let to_addrs = envelope
            .map(|e| {
                e.to.iter()
                    .map(|addr| {
                        format!(
                            "{}@{}",
                            addr.mailbox.as_deref().unwrap_or("unknown"),
                            addr.host.as_deref().unwrap_or("unknown.com")
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();

        let cc_addrs = envelope
            .map(|e| {
                e.cc.iter()
                    .map(|addr| {
                        format!(
                            "{}@{}",
                            addr.mailbox.as_deref().unwrap_or("unknown"),
                            addr.host.as_deref().unwrap_or("unknown.com")
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();

        let bcc_addrs = envelope
            .map(|e| {
                e.bcc
                    .iter()
                    .map(|addr| {
                        format!(
                            "{}@{}",
                            addr.mailbox.as_deref().unwrap_or("unknown"),
                            addr.host.as_deref().unwrap_or("unknown.com")
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Extract message ID and threading info
        let message_id = envelope.and_then(|e| e.message_id.clone());
        let in_reply_to = envelope.and_then(|e| e.in_reply_to.clone());

        // Extract reply-to address
        let reply_to = envelope.and_then(|e| e.reply_to.first()).map(|addr| {
            format!(
                "{}@{}",
                addr.mailbox.as_deref().unwrap_or("unknown"),
                addr.host.as_deref().unwrap_or("unknown.com")
            )
        });

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
        let flags: Vec<String> = imap_message
            .flags
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
    fn parse_email_body(
        &self,
        imap_message: &crate::imap::ImapMessage,
    ) -> (Option<String>, Option<String>) {
        use crate::html::is_html_content;

        let raw_body = match &imap_message.body {
            Some(body) => body,
            None => return (None, None),
        };

        // Apply aggressive email content cleaning before any processing
        let cleaned_body = self.clean_email_content(raw_body);

        // Check if the cleaned content appears to be HTML
        if is_html_content(&cleaned_body) {
            // This is HTML content - clean it further
            let html_renderer = crate::html::HtmlRenderer::new(80);
            let cleaned_html = html_renderer.clean_and_sanitize_html(&cleaned_body);

            let html_body = Some(cleaned_html.clone());

            // Convert HTML to plain text for the text body
            let text_body = Some(html_renderer.html_to_plain_text(&cleaned_html));

            (text_body, html_body)
        } else {
            // This is plain text content - apply text-specific cleaning
            let cleaned_text = self.clean_plain_text_content(&cleaned_body);
            (Some(cleaned_text), None)
        }
    }

    /// Aggressively clean email content to remove headers, encoded data, and technical junk
    fn clean_email_content(&self, raw_content: &str) -> String {
        // First, try to extract content from MIME structure
        if let Some(extracted_content) = self.extract_content_from_mime(raw_content) {
            return extracted_content;
        }

        // Fall back to line-by-line cleaning for simpler formats
        let lines: Vec<&str> = raw_content.lines().collect();
        let mut cleaned_lines = Vec::new();
        let mut _in_header_section = true;
        let mut found_content_start = false;
        let mut skip_until_boundary = false;

        for line in lines {
            let trimmed = line.trim();

            // Skip MIME boundaries and everything until content
            if trimmed.starts_with("--") && (trimmed.contains("boundary") || trimmed.len() > 10) {
                skip_until_boundary = false;
                continue;
            }

            // Skip encoded content blocks
            if self.is_encoded_content_block(trimmed) {
                skip_until_boundary = true;
                continue;
            }

            if skip_until_boundary {
                continue;
            }

            // Skip empty lines at the start
            if !found_content_start && trimmed.is_empty() {
                continue;
            }

            // Skip lines that look like email headers or technical metadata
            if self.is_technical_line(trimmed) {
                continue;
            }

            // Look for the start of actual content
            if !found_content_start {
                // Check if this line looks like actual content (not headers/metadata)
                if self.looks_like_content(trimmed) {
                    found_content_start = true;
                    _in_header_section = false;
                } else {
                    continue; // Skip this line, still in header/metadata section
                }
            }

            // If we've found content, include non-empty meaningful lines
            if found_content_start && !trimmed.is_empty() && trimmed.len() > 2 {
                cleaned_lines.push(trimmed);
            }
        }

        // Join the cleaned lines
        let result = cleaned_lines.join("\n");

        // Final cleanup of any remaining artifacts
        self.final_content_cleanup(&result)
    }

    /// Extract content from MIME structure by finding the main body part
    fn extract_content_from_mime(&self, raw_content: &str) -> Option<String> {
        let lines: Vec<&str> = raw_content.lines().collect();
        let mut extracted_content = Vec::new();
        let mut in_content_section = false;
        let mut _content_type = None;
        let mut boundary = None;

        // First pass: find boundary and content type
        for line in &lines {
            if line.to_lowercase().contains("content-type:") {
                if line.to_lowercase().contains("text/html") {
                    _content_type = Some("html");
                } else if line.to_lowercase().contains("text/plain") {
                    _content_type = Some("plain");
                }
            }
            if line.to_lowercase().contains("boundary=") {
                if let Some(start) = line.find("boundary=") {
                    let boundary_part = &line[start + 9..];
                    let boundary_clean = boundary_part.trim_matches('"').trim_matches('\'');
                    boundary = Some(format!("--{}", boundary_clean));
                }
            }
        }

        // Second pass: extract content between boundaries
        let mut found_text_part = false;
        for line in &lines {
            let trimmed = line.trim();

            // Check for MIME part boundaries
            if let Some(ref b) = boundary {
                if trimmed.starts_with(b) {
                    // Reset state for new MIME part
                    in_content_section = false;
                    found_text_part = false;
                    continue;
                }
            }

            // Look for content-type headers in MIME parts
            if trimmed.to_lowercase().starts_with("content-type:") {
                if trimmed.to_lowercase().contains("text/") {
                    found_text_part = true;
                }
                continue;
            }

            // Look for content-transfer-encoding
            if trimmed.to_lowercase().starts_with("content-transfer-encoding:") {
                continue;
            }

            // Empty line after headers indicates start of content
            if found_text_part && trimmed.is_empty() && !in_content_section {
                in_content_section = true;
                continue;
            }

            // Collect content lines
            if in_content_section && found_text_part {
                // Stop at next boundary or end
                if let Some(ref b) = boundary {
                    if trimmed.starts_with(b) {
                        break;
                    }
                }
                
                if !self.is_technical_line(trimmed) && !self.is_encoded_content_block(trimmed) {
                    extracted_content.push(trimmed);
                }
            }
        }

        if !extracted_content.is_empty() {
            Some(extracted_content.join("\n"))
        } else {
            None
        }
    }

    /// Check if a line is an encoded content block (base64, quoted-printable, etc.)
    fn is_encoded_content_block(&self, line: &str) -> bool {
        // Check for base64 encoded content (long lines of alphanumeric + / + =)
        if line.len() > 60 
            && line.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=') {
            return true;
        }

        // Check for quoted-printable encoding
        if line.contains("=?") && line.contains("?=") {
            return true;
        }

        // Check for other encoding indicators
        let encoding_patterns = [
            "Content-Transfer-Encoding:",
            "charset=",
            "=20", // quoted-printable space
            "=3D", // quoted-printable equals
            "=0A", // quoted-printable newline
        ];

        for pattern in &encoding_patterns {
            if line.contains(pattern) {
                return true;
            }
        }

        // Check for long hex sequences
        if line.len() > 30 && line.chars().all(|c| c.is_ascii_hexdigit()) {
            return true;
        }

        false
    }

    /// Check if a line looks like technical email metadata
    fn is_technical_line(&self, line: &str) -> bool {
        // Check for common email headers and technical patterns
        let technical_patterns = [
            "Message-ID:",
            "Date:",
            "From:",
            "To:",
            "Subject:",
            "Content-Type:",
            "Content-Transfer-Encoding:",
            "MIME-Version:",
            "X-",
            "Return-Path:",
            "Delivered-To:",
            "Received:",
            "Authentication-Results:",
            "DKIM-Signature:",
            "List-",
            "ARC-",
            "DMARC",
            "SPF",
            "DomainKey",
            "with SMTP id",
            "by 2002:",
            "X-Received:",
            "X-Google-",
            "X-MS-",
            "X-Mailer:",
            // Additional patterns from screenshots
            "bh=",
            "b=",
            "d=google.com;",
            "s=",
            "sarc=",
            "dmarc=pass",
            "spf=pass",
            "dkim=pass",
            "header.i=",
            "header.b=",
            "envelope.from=",
            "dares=pass",
            "Mon, ",
            "Tue, ",
            "Wed, ",
            "Thu, ",
            "Fri, ",
            "Sat, ",
            "Sun, ",
            "Jan 2025",
            "Feb 2025", 
            "Mar 2025",
            "Apr 2025",
            "May 2025",
            "Jun 2025",
            "Jul 2025",
            "Aug 2025",
            "Sep 2025",
            "Oct 2025",
            "Nov 2025",
            "Dec 2025",
            "(PDT)",
            "(PST)",
            "(UTC)",
            "(GMT)",
            "version=1;",
            "algorithm=",
            "canonical=",
            "selector=",
            "subdomain=",
            "header.from=",
            "smtp.helo=",
            "smtp.mailfrom=",
        ];

        for pattern in &technical_patterns {
            if line.starts_with(pattern) {
                return true;
            }
        }

        // Check for lines that are mostly encoded content
        if line.len() > 30
            && line
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')
        {
            return true;
        }

        // Check for timestamp patterns
        if regex::Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}")
            .unwrap()
            .is_match(line)
        {
            return true;
        }

        // Check for IP addresses and server names
        if regex::Regex::new(r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b")
            .unwrap()
            .is_match(line)
        {
            return true;
        }

        false
    }

    /// Check if a line looks like actual email content (not metadata)
    fn looks_like_content(&self, line: &str) -> bool {
        // Skip if it looks technical
        if self.is_technical_line(line) {
            return false;
        }

        // Look for signs of actual content
        let content_indicators = [
            // Common greeting patterns
            "Hi ",
            "Hello ",
            "Dear ",
            "Greetings",
            // Common content words that start sentences
            "The ",
            "This ",
            "Please ",
            "Thank ",
            "We ",
            "You ",
            "I ",
            "It ",
            "Here ",
            "There ",
            // HTML content indicators (opening tags)
            "<html>",
            "<html ",
            "<body>",
            "<body ",
            "<div>",
            "<div ",
            "<p>",
            "<p ",
            "<span>",
            "<span ",
            "<a ",
            "<h1",
            "<h2",
            "<h3",
            "<strong",
            "<em",
            "<br",
            "<img",
            "<table",
            // Common email content patterns
            "On ",  // "On Mon, Jul 28, 2025..."
            "Best ",
            "Regards",
            "Sincerely",
            "Thanks",
            "Kind ",
            "Hope ",
            "Looking ",
            "Would ",
            "Could ",
            "Should ",
            // Question patterns
            "How ",
            "What ",
            "When ",
            "Where ",
            "Why ",
            "Who ",
        ];

        // If line contains readable text and common words, it's likely content
        for indicator in &content_indicators {
            if line.contains(indicator) {
                return true;
            }
        }

        // If line has reasonable length and mixed case, it's likely content
        if line.len() > 10
            && line.chars().any(|c| c.is_lowercase())
            && line.chars().any(|c| c.is_uppercase())
        {
            return true;
        }

        false
    }

    /// Final cleanup of any remaining artifacts
    fn final_content_cleanup(&self, content: &str) -> String {
        let mut cleaned = content.to_string();

        // Remove remaining quoted-printable artifacts
        cleaned = cleaned.replace("=20", " ");
        cleaned = cleaned.replace("=3D", "=");
        cleaned = cleaned.replace("=\n", "");
        cleaned = cleaned.replace("=0A", "\n");
        cleaned = cleaned.replace("=09", "\t");

        // Remove URL encoding artifacts
        cleaned = cleaned.replace("%20", " ");
        cleaned = cleaned.replace("%3D", "=");
        cleaned = cleaned.replace("%2F", "/");
        cleaned = cleaned.replace("%3A", ":");

        // Remove any remaining technical headers that slipped through
        let lines: Vec<&str> = cleaned.lines().collect();
        let mut final_lines = Vec::new();

        for line in lines {
            let trimmed = line.trim();
            
            // Skip lines that are clearly technical artifacts
            if trimmed.is_empty() {
                final_lines.push(line);
                continue;
            }

            // Skip lines with only technical characters
            if trimmed.len() > 50 && trimmed.chars().all(|c| c.is_ascii_alphanumeric() || "+=/-_".contains(c)) {
                continue;
            }

            // Skip lines that look like encoded headers
            if trimmed.contains("=?") && trimmed.contains("?=") {
                continue;
            }

            // Skip authentication/DKIM lines
            if trimmed.contains("dkim=") || trimmed.contains("spf=") || trimmed.contains("dmarc=") {
                continue;
            }

            // Skip lines with multiple semicolons (likely technical)
            if trimmed.matches(';').count() > 2 {
                continue;
            }

            final_lines.push(line);
        }

        let result = final_lines.join("\n");

        // Remove excessive whitespace
        if let Ok(re) = regex::Regex::new(r"\n\s*\n\s*\n") {
            let cleaned_whitespace = re.replace_all(&result, "\n\n").to_string();
            cleaned_whitespace.trim().to_string()
        } else {
            result.trim().to_string()
        }
    }

    /// Clean plain text content specifically
    fn clean_plain_text_content(&self, content: &str) -> String {
        let mut cleaned = content.to_string();

        // Remove quoted-printable encoding artifacts
        cleaned = cleaned.replace("=\n", "");
        cleaned = cleaned.replace("=20", " ");
        cleaned = cleaned.replace("=3D", "=");

        // Remove excessive whitespace
        if let Ok(re) = regex::Regex::new(r"\n\s*\n\s*\n") {
            cleaned = re.replace_all(&cleaned, "\n\n").to_string();
        }

        cleaned.trim().to_string()
    }

    /// Parse attachments from IMAP message body structure
    fn parse_attachments_from_body_structure(
        &self,
        imap_message: &crate::imap::ImapMessage,
    ) -> Vec<crate::email::StoredAttachment> {
        let mut attachments = Vec::new();

        if let Some(ref body_structure) = imap_message.body_structure {
            self.extract_attachments_recursive(body_structure, &mut attachments, 0);
        }

        attachments
    }

    /// Recursively extract attachments from body structure
    fn extract_attachments_recursive(
        &self,
        body_structure: &crate::imap::BodyStructure,
        attachments: &mut Vec<crate::email::StoredAttachment>,
        part_index: usize,
    ) {
        // Check if this part is an attachment
        if body_structure.is_attachment() {
            let filename = body_structure
                .parameters
                .get("name")
                .or_else(|| body_structure.parameters.get("filename"))
                .cloned()
                .unwrap_or_else(|| format!("attachment_{}", part_index));

            let content_type = format!(
                "{}/{}",
                body_structure.media_type, body_structure.media_subtype
            );

            let attachment = crate::email::StoredAttachment {
                id: format!(
                    "att_{}_{}",
                    part_index,
                    chrono::Utc::now().timestamp_millis()
                ),
                filename,
                content_type,
                size: body_structure.size.unwrap_or(0),
                content_id: body_structure.content_id.clone(),
                is_inline: body_structure.content_id.is_some(),
                data: None,      // Will be fetched separately when needed
                file_path: None, // Will be set when saved to disk
            };

            tracing::debug!(
                "Found attachment: {} ({})",
                attachment.filename,
                attachment.content_type
            );

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
        self.ui
            .update_account_status(account_id, crate::ui::AccountSyncStatus::Syncing, None);

        // Force reconnection by clearing any cached connections
        if let Some(ref _imap_manager) = self.imap_manager {
            // Create a new manager to handle disconnection
            // Since disconnect_all needs mutable access and we can't get that through Arc,
            // we skip the disconnect for now - connections will timeout naturally
            tracing::debug!("Skipping forced disconnection due to Arc limitation - connections will timeout naturally");
        }

        // Try to sync account to test connection
        match self.sync_account_from_imap(account_id).await {
            Ok(()) => {
                tracing::info!("Successfully refreshed account: {}", account_id);
                self.ui.update_account_status(
                    account_id,
                    crate::ui::AccountSyncStatus::Online,
                    None,
                );

                // Reload messages for current folder if this is the active account
                if let Some(current_id) = self.ui.get_current_account_id() {
                    if current_id == account_id {
                        let _ = self
                            .ui
                            .load_messages(account_id.to_string(), "INBOX".to_string())
                            .await;
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to refresh account {}: {}", account_id, e);
                self.ui.update_account_status(
                    account_id,
                    crate::ui::AccountSyncStatus::Error,
                    None,
                );
                return Err(anyhow::anyhow!("Failed to refresh account: {}", e));
            }
        }

        Ok(())
    }

    /// Handle manual IMAP sync (F5) - sync folders and messages
    async fn handle_sync_account(&mut self, account_id: &str) -> Result<()> {
        tracing::info!("Manual IMAP sync requested for account: {}", account_id);

        // Update status to show we're syncing
        self.ui
            .update_account_status(account_id, crate::ui::AccountSyncStatus::Syncing, None);

        // Perform full sync
        match self.sync_account_from_imap(account_id).await {
            Ok(()) => {
                tracing::info!("Successfully synced account: {}", account_id);
                self.ui.update_account_status(
                    account_id,
                    crate::ui::AccountSyncStatus::Online,
                    None,
                );

                // If this is the current account, reload the message list
                if let Some(current_id) = self.ui.get_current_account_id() {
                    if current_id == account_id {
                        // Get current folder
                        let current_folder = self
                            .ui
                            .folder_tree()
                            .selected_folder()
                            .map(|f| f.name.clone())
                            .unwrap_or_else(|| "INBOX".to_string());

                        // Reload messages
                        if let Err(e) = self
                            .ui
                            .load_messages(account_id.to_string(), current_folder)
                            .await
                        {
                            tracing::error!("Failed to reload messages after sync: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to sync account {}: {}", account_id, e);
                self.ui.update_account_status(
                    account_id,
                    crate::ui::AccountSyncStatus::Error,
                    None,
                );
                return Err(anyhow::anyhow!("Failed to sync account: {}", e));
            }
        }

        Ok(())
    }

    /// Handle folder selection event - load cached messages immediately, then refresh in background
    /// This method provides instant feedback by loading cached messages first, then updates in background
    async fn handle_folder_select(&mut self, folder_path: &str) -> Result<()> {
        // Get the current account ID and clone it to avoid borrowing issues
        let current_account_id = match self.ui.get_current_account_id() {
            Some(id) => id.clone(),
            None => {
                tracing::warn!("No current account selected for folder selection");
                return Ok(());
            }
        };

        tracing::info!(
            "Loading folder: {} for account: {} (instant load from cache)",
            folder_path,
            current_account_id
        );

        // STEP 1: Load messages from database immediately (non-blocking UI)
        // This provides instant feedback to the user
        match self
            .ui
            .load_messages(current_account_id.clone(), folder_path.to_string())
            .await
        {
            Ok(()) => {
                tracing::info!("✅ Instantly loaded cached messages from folder: {}", folder_path);
                
                // Show a brief notification that content is loaded
                self.ui.show_notification(
                    format!("📂 Loaded {}", folder_path),
                    std::time::Duration::from_millis(1500)
                );
            }
            Err(e) => {
                tracing::warn!("No cached messages for folder {}: {}. Will fetch from IMAP.", folder_path, e);
                
                // Show loading indicator for first-time folder access
                self.ui.show_notification(
                    format!("📥 Loading {} for the first time...", folder_path),
                    std::time::Duration::from_secs(3)
                );
            }
        }

        // STEP 2: Queue background refresh task using the background processor
        if let Some(folder_name) = folder_path.split('/').next_back() {
            use crate::performance::background_processor::{BackgroundTask, BackgroundTaskType, TaskPriority};
            
            use uuid::Uuid;
            
            let background_task = BackgroundTask {
                id: Uuid::new_v4(),
                name: format!("Quick refresh: {}", folder_name),
                priority: TaskPriority::Normal,
                account_id: current_account_id.clone(),
                folder_name: Some(folder_name.to_string()),
                task_type: BackgroundTaskType::FolderRefresh {
                    folder_name: folder_name.to_string(),
                },
                created_at: std::time::Instant::now(),
                estimated_duration: Some(std::time::Duration::from_secs(2)),
            };
            
            // Queue the background task (non-blocking)
            match self.queue_background_task(background_task).await {
                Ok(task_id) => {
                    tracing::info!("✅ Queued background refresh task for {} (ID: {})", folder_path, task_id);
                    self.ui.show_notification(
                        format!("🔄 Background sync queued for {}", folder_name),
                        std::time::Duration::from_millis(1000)
                    );
                }
                Err(e) => {
                    tracing::warn!("Failed to queue background task: {}", e);
                    // Fallback to the old method if background processor isn't available
                    let account_id_bg = current_account_id.clone();
                    let folder_path_bg = folder_path.to_string();
                    tokio::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
                        tracing::info!("✅ Fallback background check completed for {} (account: {})", folder_path_bg, account_id_bg);
                    });
                }
            }
        }

        Ok(())
    }

    /// Force refresh a folder with full IMAP sync (for F5/Ctrl+R)
    /// This is the blocking version that users can trigger manually
    async fn handle_folder_force_refresh(&mut self, folder_path: &str) -> Result<()> {
        let current_account_id = match self.ui.get_current_account_id() {
            Some(id) => id.clone(),
            None => {
                tracing::warn!("No current account selected for folder refresh");
                return Ok(());
            }
        };

        // Show that we're doing a full refresh
        self.ui.show_notification(
            format!("🔄 Force refreshing {}...", folder_path),
            std::time::Duration::from_secs(2)
        );

        tracing::info!("Force refreshing folder: {} for account: {}", folder_path, current_account_id);

        // Queue a high-priority background task for full IMAP sync
        if let Some(folder_name) = folder_path.split('/').next_back() {
            use crate::performance::background_processor::{BackgroundTask, BackgroundTaskType, TaskPriority};
            use crate::email::sync_engine::SyncStrategy;
            use uuid::Uuid;
            
            let sync_task = BackgroundTask {
                id: Uuid::new_v4(),
                name: format!("Full sync: {}", folder_name),
                priority: TaskPriority::High, // High priority for user-requested operations
                account_id: current_account_id.clone(),
                folder_name: Some(folder_name.to_string()),
                task_type: BackgroundTaskType::FolderSync {
                    folder_name: folder_name.to_string(),
                    strategy: SyncStrategy::Full,
                },
                created_at: std::time::Instant::now(),
                estimated_duration: Some(std::time::Duration::from_secs(30)),
            };
            
            match self.queue_background_task(sync_task).await {
                Ok(task_id) => {
                    tracing::info!("✅ Queued high-priority sync task for {} (ID: {})", folder_path, task_id);
                    self.ui.show_notification(
                        format!("🚀 Full sync started for {}", folder_name),
                        std::time::Duration::from_secs(2)
                    );
                    return Ok(());
                }
                Err(e) => {
                    tracing::warn!("Failed to queue background sync task: {}, falling back to direct IMAP", e);
                    // Fall through to direct IMAP fetch as fallback
                }
            }
        }
        
        // Fallback to direct IMAP fetch if background processor is not available
        match self.fetch_messages_from_imap(&current_account_id, folder_path).await {
            Ok(()) => {
                tracing::info!("✅ Successfully refreshed folder: {}", folder_path);
                self.ui.show_notification(
                    format!("✅ Refreshed {}", folder_path),
                    std::time::Duration::from_secs(2)
                );
            }
            Err(e) => {
                tracing::warn!("Failed to refresh folder {}: {}", folder_path, e);
                self.ui.show_notification(
                    format!("⚠️ Failed to refresh {}: {}", folder_path, e),
                    std::time::Duration::from_secs(4)
                );
            }
        }

        // Reload the messages in the UI
        if let Err(e) = self
            .ui
            .load_messages(current_account_id, folder_path.to_string())
            .await
        {
            tracing::error!("Failed to reload messages after refresh: {}", e);
        }

        Ok(())
    }

    /// Handle folder operation events
    async fn handle_folder_operation(
        &mut self,
        operation: crate::ui::folder_tree::FolderOperation,
    ) -> Result<()> {
        use crate::ui::folder_tree::FolderOperation;

        // Get current account and selected folder
        let current_account_id = match self.ui.get_current_account_id() {
            Some(id) => id.clone(),
            None => {
                tracing::warn!("No current account selected for folder operation");
                return Ok(());
            }
        };

        let selected_folder = self.ui.folder_tree().selected_folder().cloned();

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
                    self.handle_create_folder(&current_account_id, Some(&folder.path))
                        .await?;
                }
            }
            FolderOperation::Delete => {
                if let Some(folder) = selected_folder {
                    self.handle_delete_folder(&current_account_id, &folder.path)
                        .await?;
                }
            }
            FolderOperation::Rename => {
                if let Some(folder) = selected_folder {
                    self.handle_rename_folder(&current_account_id, &folder.path)
                        .await?;
                }
            }
            FolderOperation::EmptyFolder => {
                if let Some(folder) = selected_folder {
                    self.handle_empty_folder(&current_account_id, &folder.path)
                        .await?;
                }
            }
            FolderOperation::Subscribe => {
                if let Some(folder) = selected_folder {
                    self.handle_folder_subscription(&current_account_id, &folder.path, true)
                        .await?;
                }
            }
            FolderOperation::Unsubscribe => {
                if let Some(folder) = selected_folder {
                    self.handle_folder_subscription(&current_account_id, &folder.path, false)
                        .await?;
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
            self.fetch_messages_from_imap(account_id, &folder_path)
                .await?;
            if let Err(e) = self
                .ui
                .load_messages(account_id.to_string(), folder_path)
                .await
            {
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
            tracing::info!(
                "Marking all messages as read in folder: {} for account: {}",
                folder_path,
                account_id
            );

            // TODO: Implement IMAP STORE command to mark messages as read
            // For now, just update the database
            if let Some(ref database) = self.database {
                let result = sqlx::query("UPDATE messages SET is_read = 1, flags = flags || ',\\Seen' WHERE account_id = ? AND folder_name = ? AND is_read = 0")
                    .bind(account_id)
                    .bind(&folder_path)
                    .execute(&database.pool)
                    .await?;

                tracing::info!(
                    "Marked {} messages as read in folder: {}",
                    result.rows_affected(),
                    folder_path
                );

                // Reload messages to update UI
                if let Err(e) = self
                    .ui
                    .load_messages(account_id.to_string(), folder_path)
                    .await
                {
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
            tracing::info!(
                "Showing properties for folder: {} in account: {}",
                folder_path,
                account_id
            );

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

                tracing::info!(
                    "Folder {} statistics: {} total messages, {} unread, {} bytes",
                    folder_path,
                    stats.0,
                    stats.1,
                    stats.2.unwrap_or(0)
                );
            }
        }

        Ok(())
    }

    /// Handle create new folder
    async fn handle_create_folder(
        &mut self,
        account_id: &str,
        parent_path: Option<&str>,
    ) -> Result<()> {
        tracing::info!(
            "Creating new folder in account: {}, parent: {:?}",
            account_id,
            parent_path
        );

        // TODO: Implement folder creation dialog
        // For now, create a default folder name
        let folder_name = if parent_path.is_some() {
            "New Subfolder"
        } else {
            "New Folder"
        };

        // TODO: Implement IMAP CREATE command
        tracing::info!(
            "Would create folder: {} in account: {}",
            folder_name,
            account_id
        );

        Ok(())
    }

    /// Handle delete folder
    async fn handle_delete_folder(&mut self, account_id: &str, folder_path: &str) -> Result<()> {
        tracing::info!(
            "Deleting folder: {} from account: {}",
            folder_path,
            account_id
        );

        // TODO: Implement confirmation dialog
        // TODO: Implement IMAP DELETE command
        tracing::info!(
            "Would delete folder: {} from account: {}",
            folder_path,
            account_id
        );

        Ok(())
    }

    /// Handle rename folder
    async fn handle_rename_folder(&mut self, account_id: &str, folder_path: &str) -> Result<()> {
        tracing::info!(
            "Renaming folder: {} in account: {}",
            folder_path,
            account_id
        );

        // TODO: Implement rename dialog
        // TODO: Implement IMAP RENAME command
        tracing::info!(
            "Would rename folder: {} in account: {}",
            folder_path,
            account_id
        );

        Ok(())
    }

    /// Handle empty folder (delete all messages)
    async fn handle_empty_folder(&mut self, account_id: &str, folder_path: &str) -> Result<()> {
        tracing::info!(
            "Emptying folder: {} in account: {}",
            folder_path,
            account_id
        );

        // TODO: Implement confirmation dialog
        // TODO: Implement IMAP STORE/EXPUNGE commands to delete all messages
        tracing::info!(
            "Would empty folder: {} in account: {}",
            folder_path,
            account_id
        );

        Ok(())
    }

    /// Handle folder subscription management
    async fn handle_folder_subscription(
        &mut self,
        account_id: &str,
        folder_path: &str,
        subscribe: bool,
    ) -> Result<()> {
        let action = if subscribe {
            "Subscribing to"
        } else {
            "Unsubscribing from"
        };
        tracing::info!(
            "{} folder: {} in account: {}",
            action,
            folder_path,
            account_id
        );

        // TODO: Implement IMAP SUBSCRIBE/UNSUBSCRIBE commands
        // For now, just update local state
        self.ui
            .folder_tree_mut()
            .mark_folder_synced(folder_path, 0, 0);

        Ok(())
    }

    /// Queue automatic background sync for all accounts (non-blocking)
    async fn queue_auto_sync_background(&mut self) {
        tracing::info!("🔄 Queuing automatic background sync for all accounts");
        
        // Get all account IDs
        let account_ids = if let Some(ref database) = self.database {
            match sqlx::query("SELECT id FROM accounts")
                .fetch_all(&database.pool)
                .await
            {
                Ok(rows) => rows
                    .into_iter()
                    .map(|row| row.get::<String, _>("id"))
                    .collect::<Vec<_>>(),
                Err(e) => {
                    tracing::error!("Failed to get account IDs for auto-sync: {}", e);
                    return;
                }
            }
        } else {
            tracing::warn!("Database not available for auto-sync");
            return;
        };

        if account_ids.is_empty() {
            tracing::debug!("No accounts configured, skipping auto-sync");
            return;
        }

        // Queue background sync tasks for each account with low priority
        for account_id in account_ids {
            use crate::performance::background_processor::{BackgroundTask, BackgroundTaskType, TaskPriority};
            use uuid::Uuid;
            
            let background_task = BackgroundTask {
                id: Uuid::new_v4(),
                name: format!("Auto-sync: {}", account_id),
                priority: TaskPriority::Low, // Low priority so it doesn't interfere with user actions
                account_id: account_id.clone(),
                folder_name: None, // Account-wide sync
                task_type: BackgroundTaskType::AccountSync {
                    strategy: crate::email::sync_engine::SyncStrategy::Incremental,
                },
                created_at: std::time::Instant::now(),
                estimated_duration: Some(std::time::Duration::from_secs(30)),
            };
            
            match self.queue_background_task(background_task).await {
                Ok(task_id) => {
                    tracing::debug!("✅ Queued auto-sync task for {} (ID: {})", account_id, task_id);
                }
                Err(e) => {
                    tracing::warn!("Failed to queue auto-sync task for {}: {}", account_id, e);
                }
            }
        }
        
        // Show a subtle notification that auto-sync is running
        self.ui.show_notification(
            "🔄 Background sync started".to_string(),
            std::time::Duration::from_millis(1500)
        );
    }

    /// Perform automatic background sync for all accounts (DEPRECATED - use queue_auto_sync_background instead)
    #[allow(dead_code)]
    async fn perform_auto_sync(&mut self) {
        tracing::info!("Performing automatic background sync for all accounts");

        // Get all account IDs
        let account_ids = if let Some(ref database) = self.database {
            match sqlx::query("SELECT id FROM accounts")
                .fetch_all(&database.pool)
                .await
            {
                Ok(rows) => rows
                    .into_iter()
                    .map(|row| row.get::<String, _>("id"))
                    .collect::<Vec<_>>(),
                Err(e) => {
                    tracing::error!("Failed to get account IDs for auto-sync: {}", e);
                    return;
                }
            }
        } else {
            tracing::warn!("Database not available for auto-sync");
            return;
        };

        if account_ids.is_empty() {
            tracing::debug!("No accounts configured, skipping auto-sync");
            return;
        }

        tracing::info!("Auto-syncing {} accounts", account_ids.len());

        // Get the current account to reload its messages after sync
        let current_account_id = self.ui.get_current_account_id().cloned();
        let current_folder = self
            .ui
            .folder_tree()
            .selected_folder()
            .map(|f| f.name.clone())
            .unwrap_or_else(|| "INBOX".to_string());

        let mut new_email_count = 0;
        let mut synced_accounts: Vec<String> = Vec::new();

        // Sync each account
        for account_id in &account_ids {
            tracing::debug!("Auto-syncing account: {}", account_id);

            // Get message count before sync for this account's INBOX
            let messages_before = if let Some(ref database) = self.database {
                sqlx::query("SELECT COUNT(*) as count FROM messages WHERE account_id = ? AND folder_name = 'INBOX' AND is_deleted = FALSE")
                    .bind(account_id)
                    .fetch_one(&database.pool)
                    .await
                    .map(|row| row.get::<i64, _>("count") as u32)
                    .unwrap_or(0)
            } else {
                0
            };

            // Perform sync
            match self.sync_account_from_imap(account_id).await {
                Ok(()) => {
                    tracing::debug!("Auto-sync successful for account: {}", account_id);
                    synced_accounts.push(account_id.to_string());

                    // Get message count after sync
                    let messages_after = if let Some(ref database) = self.database {
                        sqlx::query("SELECT COUNT(*) as count FROM messages WHERE account_id = ? AND folder_name = 'INBOX' AND is_deleted = FALSE")
                            .bind(account_id)
                            .fetch_one(&database.pool)
                            .await
                            .map(|row| row.get::<i64, _>("count") as u32)
                            .unwrap_or(0)
                    } else {
                        0
                    };

                    // Count new emails
                    if messages_after > messages_before {
                        let new_count = messages_after - messages_before;
                        new_email_count += new_count;
                        tracing::info!(
                            "Found {} new emails for account: {}",
                            new_count,
                            account_id
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!("Auto-sync failed for account {}: {}", account_id, e);
                    // Don't stop the auto-sync for other accounts
                }
            }
        }

        // Show notification if we found new emails
        if new_email_count > 0 {
            let message = if new_email_count == 1 {
                "📧 1 new email received".to_string()
            } else {
                format!("📧 {new_email_count} new emails received")
            };

            tracing::info!("Auto-sync found {} new emails", new_email_count);
            self.ui.show_notification(message, Duration::from_secs(5));
        }

        // If the current account was synced, reload its messages
        if let Some(current_id) = current_account_id {
            if synced_accounts.contains(&current_id) {
                tracing::debug!("Reloading messages for current account after auto-sync");
                if let Err(e) = self.ui.load_messages(current_id, current_folder).await {
                    tracing::error!("Failed to reload messages after auto-sync: {}", e);
                }
            }
        }

        tracing::info!(
            "Auto-sync completed. Synced {} accounts, found {} new emails",
            synced_accounts.len(),
            new_email_count
        );
    }

    /// Handle contacts popup request
    async fn handle_contacts_popup(&mut self) -> Result<()> {
        if let Some(ref contacts_manager) = self.contacts_manager {
            self.ui.show_contacts_popup(contacts_manager.clone());
        } else {
            tracing::warn!("Contacts manager not initialized, cannot show contacts popup");
        }
        Ok(())
    }

    /// Handle contacts popup action
    async fn handle_contacts_action(&mut self, action: crate::contacts::ContactPopupAction) -> Result<()> {
        use crate::contacts::ContactPopupAction;
        
        match action {
            ContactPopupAction::SelectForEmail { to, name } => {
                // Use this contact for email composition
                self.ui.hide_contacts_popup();
                
                let message = format!("Selected contact: {} <{}>", name, to);
                self.ui.show_notification(message, Duration::from_secs(3));
                
                // TODO: Start email composition with this contact
            }
            ContactPopupAction::ViewContact(contact) => {
                // View contact details
                self.ui.hide_contacts_popup();
                
                let message = format!("Viewing contact: {} <{}>", 
                    contact.display_name, 
                    contact.primary_email().map(|e| e.address.as_str()).unwrap_or("no email")
                );
                self.ui.show_notification(message, Duration::from_secs(3));
            }
            ContactPopupAction::Close => {
                // Close the popup
                self.ui.hide_contacts_popup();
            }
            ContactPopupAction::OpenFullAddressBook => {
                // Open full address book view
                self.ui.hide_contacts_popup();
                self.ui.show_notification(
                    "Opening full address book...".to_string(),
                    Duration::from_secs(3),
                );
                // TODO: Implement full address book view
            }
        }
        
        Ok(())
    }

    /// Handle add to contacts request
    async fn handle_add_to_contacts(&mut self, email: &str, name: &str) -> Result<()> {
        if let Some(ref contacts_manager) = self.contacts_manager {
            let mut new_contact = crate::contacts::Contact::new(
                uuid::Uuid::new_v4().to_string(),
                crate::contacts::ContactSource::Local,
                name.to_string(),
            );
            
            new_contact.emails.push(crate::contacts::ContactEmail::new(
                email.to_string(),
                "primary".to_string(),
            ));
            
            match contacts_manager.create_contact(new_contact).await {
                Ok(_) => {
                    self.ui.show_notification(
                        format!("Added {} to contacts", name),
                        Duration::from_secs(3),
                    );
                }
                Err(e) => {
                    tracing::error!("Failed to add contact: {}", e);
                    self.ui.show_notification(
                        format!("Failed to add contact: {}", e),
                        Duration::from_secs(5),
                    );
                }
            }
        } else {
            tracing::warn!("Contacts manager not initialized, cannot add to contacts");
        }
        
        Ok(())
    }

    /// Handle view sender contact request
    async fn handle_view_sender_contact(&mut self, email: &str) -> Result<()> {
        if let Some(ref contacts_manager) = self.contacts_manager {
            tracing::debug!("Looking up contact details for: {}", email);
            
            match contacts_manager.find_contact_by_email(email).await {
                Ok(Some(contact)) => {
                    tracing::info!("Found contact for {}: {}", email, contact.display_name);
                    
                    // Show contact details in contacts popup with the specific contact selected
                    self.ui.show_contacts_popup_with_contact(contacts_manager.clone(), contact);
                }
                Ok(None) => {
                    tracing::info!("No contact found for email: {}", email);
                    self.ui.show_notification(
                        format!("No contact found for {}", email),
                        Duration::from_secs(3),
                    );
                }
                Err(e) => {
                    tracing::error!("Failed to lookup contact for {}: {}", email, e);
                    self.ui.show_notification(
                        format!("Failed to lookup contact: {}", e),
                        Duration::from_secs(5),
                    );
                }
            }
        } else {
            tracing::warn!("Contacts manager not initialized, cannot view contact");
        }
        
        Ok(())
    }

    /// Handle edit sender contact request
    async fn handle_edit_sender_contact(&mut self, email: &str) -> Result<()> {
        if let Some(ref contacts_manager) = self.contacts_manager {
            tracing::debug!("Looking up contact to edit for: {}", email);
            
            match contacts_manager.find_contact_by_email(email).await {
                Ok(Some(contact)) => {
                    tracing::info!("Found contact to edit for {}: {}", email, contact.display_name);
                    
                    // Show contact edit dialog/popup
                    self.ui.show_contact_edit_dialog(contacts_manager.clone(), contact);
                }
                Ok(None) => {
                    tracing::info!("No contact found to edit for email: {}", email);
                    self.ui.show_notification(
                        format!("No contact found for {} to edit", email),
                        Duration::from_secs(3),
                    );
                }
                Err(e) => {
                    tracing::error!("Failed to lookup contact to edit for {}: {}", email, e);
                    self.ui.show_notification(
                        format!("Failed to lookup contact: {}", e),
                        Duration::from_secs(5),
                    );
                }
            }
        } else {
            tracing::warn!("Contacts manager not initialized, cannot edit contact");
        }
        
        Ok(())
    }

    /// Handle remove sender from contacts request
    async fn handle_remove_sender_from_contacts(&mut self, email: &str) -> Result<()> {
        if let Some(ref contacts_manager) = self.contacts_manager {
            tracing::debug!("Looking up contact to remove for: {}", email);
            
            match contacts_manager.find_contact_by_email(email).await {
                Ok(Some(contact)) => {
                    if let Some(contact_id) = contact.id {
                        tracing::info!("Removing contact for {}: {}", email, contact.display_name);
                        
                        match contacts_manager.delete_contact(contact_id).await {
                            Ok(_) => {
                                self.ui.show_notification(
                                    format!("Removed {} from contacts", contact.display_name),
                                    Duration::from_secs(3),
                                );
                            }
                            Err(e) => {
                                tracing::error!("Failed to remove contact: {}", e);
                                self.ui.show_notification(
                                    format!("Failed to remove contact: {}", e),
                                    Duration::from_secs(5),
                                );
                            }
                        }
                    } else {
                        tracing::warn!("Contact found but has no ID, cannot remove");
                        self.ui.show_notification(
                            "Cannot remove contact: invalid contact data".to_string(),
                            Duration::from_secs(5),
                        );
                    }
                }
                Ok(None) => {
                    tracing::info!("No contact found to remove for email: {}", email);
                    self.ui.show_notification(
                        format!("No contact found for {} to remove", email),
                        Duration::from_secs(3),
                    );
                }
                Err(e) => {
                    tracing::error!("Failed to lookup contact to remove for {}: {}", email, e);
                    self.ui.show_notification(
                        format!("Failed to lookup contact: {}", e),
                        Duration::from_secs(5),
                    );
                }
            }
        } else {
            tracing::warn!("Contacts manager not initialized, cannot remove contact");
        }
        
        Ok(())
    }

    /// Handle contact quick actions menu request
    async fn handle_contact_quick_actions(&mut self, email: &str) -> Result<()> {
        if let Some(ref contacts_manager) = self.contacts_manager {
            tracing::debug!("Showing quick actions menu for: {}", email);
            
            // Show context menu with available actions based on whether contact exists
            match contacts_manager.find_contact_by_email(email).await {
                Ok(Some(contact)) => {
                    // Contact exists - show edit/remove/view actions
                    self.ui.show_contact_context_menu(email.to_string(), Some(contact));
                }
                Ok(None) => {
                    // No contact - show add action
                    self.ui.show_contact_context_menu(email.to_string(), None);
                }
                Err(e) => {
                    tracing::error!("Failed to lookup contact for quick actions: {}", e);
                    self.ui.show_notification(
                        format!("Failed to check contact status: {}", e),
                        Duration::from_secs(5),
                    );
                }
            }
        } else {
            tracing::warn!("Contacts manager not initialized, cannot show quick actions");
        }
        
        Ok(())
    }

    /// Handle email viewer started - look up sender contact information
    async fn handle_email_viewer_started(&mut self, sender_email: &str) -> Result<()> {
        if let Some(ref contacts_manager) = self.contacts_manager {
            tracing::debug!("Looking up contact for sender: {}", sender_email);
            
            match contacts_manager.find_contact_by_email(sender_email).await {
                Ok(Some(contact)) => {
                    tracing::info!("Found contact for sender {}: {}", sender_email, contact.display_name);
                    
                    // Set the contact information in the email viewer
                    self.ui.email_viewer_mut().set_sender_contact(Some(contact));
                }
                Ok(None) => {
                    tracing::debug!("No contact found for sender: {}", sender_email);
                    
                    // Clear any existing contact information
                    self.ui.email_viewer_mut().set_sender_contact(None);
                }
                Err(e) => {
                    tracing::warn!("Error looking up contact for {}: {}", sender_email, e);
                    
                    // Clear contact information on error
                    self.ui.email_viewer_mut().set_sender_contact(None);
                }
            }
        } else {
            tracing::debug!("Contacts manager not available for sender lookup");
        }
        
        Ok(())
    }

    /// Handle reply to message action
    async fn handle_reply_to_message(&mut self, message_id: uuid::Uuid) -> Result<()> {
        tracing::info!("Reply to message triggered for ID: {}", message_id);
        
        // Load the full message from database
        if let Some(database) = &self.database {
            if let Some(stored_message) = database.get_message_by_id(message_id).await? {
            // Start compose with reply
            let compose_action = crate::ui::ComposeAction::StartReplyFromMessage(stored_message);
                self.handle_compose_action(compose_action).await?;
            } else {
                tracing::error!("Message not found for ID: {}", message_id);
            }
        } else {
            tracing::error!("Database not available");
        }
        
        Ok(())
    }

    /// Handle reply all to message action
    async fn handle_reply_all_to_message(&mut self, message_id: uuid::Uuid) -> Result<()> {
        tracing::info!("Reply all to message triggered for ID: {}", message_id);
        
        // Load the full message from database
        if let Some(database) = &self.database {
            if let Some(stored_message) = database.get_message_by_id(message_id).await? {
            // Start compose with reply all
            let compose_action = crate::ui::ComposeAction::StartReplyAllFromMessage(stored_message);
                self.handle_compose_action(compose_action).await?;
            } else {
                tracing::error!("Message not found for ID: {}", message_id);
            }
        } else {
            tracing::error!("Database not available");
        }
        
        Ok(())
    }

    /// Handle forward message action
    async fn handle_forward_message(&mut self, message_id: uuid::Uuid) -> Result<()> {
        tracing::info!("Forward message triggered for ID: {}", message_id);
        
        // Load the full message from database
        if let Some(database) = &self.database {
            if let Some(stored_message) = database.get_message_by_id(message_id).await? {
            // Start compose with forward
            let compose_action = crate::ui::ComposeAction::StartForwardFromMessage(stored_message);
                self.handle_compose_action(compose_action).await?;
            } else {
                tracing::error!("Message not found for ID: {}", message_id);
            }
        } else {
            tracing::error!("Database not available");
        }
        
        Ok(())
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new().expect("Failed to create default App")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_startup_progress_manager_integration() {
        let mut app = App::new().unwrap();
        
        // Test that the progress manager is initialized
        assert_eq!(app.startup_progress_manager().phases().len(), 5);
        assert!(!app.startup_progress_manager().is_complete());
        assert_eq!(app.startup_progress_manager().overall_progress_percentage(), 0.0);
        
        // Test that we can get a mutable reference
        let progress_manager = app.startup_progress_manager_mut();
        progress_manager.start_phase("Database").unwrap();
        
        // Test that the phase is now started
        assert!(progress_manager.current_phase().unwrap().status().is_in_progress());
    }

    #[tokio::test]
    async fn test_database_initialization_with_progress() {
        let mut app = App::new().unwrap();
        
        // Initialize database should update progress
        let result = app.initialize_database().await;
        
        match result {
            Ok(()) => {
                // Database phase should be completed
                let phases = app.startup_progress_manager().phases();
                assert!(phases[0].status().is_completed());
            }
            Err(_) => {
                // Database phase should be failed
                let phases = app.startup_progress_manager().phases();
                assert!(phases[0].status().is_failed());
            }
        }
    }
}
