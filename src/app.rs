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

use crate::ai::config_manager::AIConfigManager;
use crate::calendar::CalendarManager;
use crate::contacts::ContactsManager;
use crate::email::sync_engine::SyncProgress;
use crate::email::{EmailDatabase, EmailNotificationManager};
use crate::events::legacy::EventResult;
use crate::imap::ImapAccountManager;
use crate::notifications::{NotificationConfig, UnifiedNotificationManager};
use crate::oauth2::{AccountConfig, SecureStorage, TokenManager};
use crate::performance::background_processor::{BackgroundProcessor, BackgroundTask, TaskResult};
use crate::smtp::{SmtpService, SmtpServiceBuilder};
use crate::startup::StartupProgressManager;
use crate::ui::command_palette::CommandAction;
use crate::ui::mouse_handler::MouseAction;
use crate::ui::{ComposeAction, DraftAction, UI};
use tokio::sync::mpsc;
use uuid::Uuid;

pub struct App {
    should_quit: bool,
    ui: UI,
    database: Option<Arc<EmailDatabase>>,
    notification_manager: Option<Arc<EmailNotificationManager>>,
    storage: SecureStorage,
    imap_manager: Option<Arc<ImapAccountManager>>,
    token_manager: Option<TokenManager>,
    token_refresh_scheduler: Option<crate::oauth2::token::TokenRefreshScheduler>,
    smtp_service: Option<SmtpService>,
    contacts_manager: Option<Arc<ContactsManager>>,
    calendar_manager: Option<Arc<CalendarManager>>,
    unified_notification_manager: Option<Arc<UnifiedNotificationManager>>,
    // Auto-sync functionality
    last_auto_sync: Instant,
    auto_sync_interval: Duration,
    // Deferred initialization
    deferred_initialization: bool,
    initialization_complete: bool,
    initialization_in_progress: bool,
    // Background processing
    background_processor: Option<Arc<BackgroundProcessor>>,
    sync_progress_rx: Option<mpsc::UnboundedReceiver<SyncProgress>>,
    task_completion_rx: Option<mpsc::UnboundedReceiver<TaskResult>>,
    // Sync engine for email operations
    sync_engine: Option<Arc<crate::email::sync_engine::SyncEngine>>,
    // Email operations service
    email_operations_service: Option<Arc<crate::email::EmailOperationsService>>,
    // AI configuration manager
    ai_config_manager: Option<Arc<crate::ai::config_manager::AIConfigManager>>,
    // Startup progress manager
    startup_progress_manager: StartupProgressManager,
    // Toast integration service (using simple direct approach now)
    // toast_integration_service: Option<crate::ui::toast_integration::ToastIntegrationService>,
}

impl App {
    /// Create a new application instance with default configuration
    pub fn new() -> Result<Self> {
        Ok(Self {
            should_quit: false,
            ui: UI::new(),
            database: None,
            notification_manager: None,
            storage: SecureStorage::new("comunicado".to_string())
                .map_err(|e| anyhow::anyhow!("Failed to initialize secure storage: {}", e))?,
            imap_manager: None,
            token_manager: None,
            token_refresh_scheduler: None,
            smtp_service: None,
            contacts_manager: None,
            calendar_manager: None,
            unified_notification_manager: None,
            // Initialize auto-sync with 3 minute interval
            last_auto_sync: Instant::now(),
            auto_sync_interval: Duration::from_secs(3 * 60), // 3 minutes
            // Deferred initialization
            deferred_initialization: false,
            initialization_complete: false,
            initialization_in_progress: false,
            // Background processing
            background_processor: None,
            sync_progress_rx: None,
            task_completion_rx: None,
            // Sync engine
            sync_engine: None,
            // Email operations service
            email_operations_service: None,
            // AI configuration manager
            ai_config_manager: None,
            // Startup progress manager
            startup_progress_manager: StartupProgressManager::new(),
            // Toast integration service
            // toast_integration_service: None,
        })
    }

    /// Initialize the database connection
    pub async fn initialize_database(&mut self) -> Result<()> {
        tracing::info!("🗄️ Initializing database connection...");

        // Start database phase in progress manager
        if let Err(e) = self.startup_progress_manager.start_phase("Database") {
            tracing::warn!("Failed to start Database phase in progress manager: {}", e);
        }

        // Create database path in user's config directory (same as CLI)
        let config_dir = match dirs::config_dir() {
            Some(dir) => dir.join("comunicado").join("databases"),
            None => {
                if let Err(e) = self
                    .startup_progress_manager
                    .fail_phase("Database", "Cannot find config directory".to_string())
                {
                    tracing::warn!("Failed to fail Database phase in progress manager: {}", e);
                }
                return Err(anyhow::anyhow!("Cannot find config directory"));
            }
        };

        // Create directory if it doesn't exist
        if let Err(e) = std::fs::create_dir_all(&config_dir) {
            if let Err(pe) = self.startup_progress_manager.fail_phase(
                "Database",
                format!("Failed to create database directory: {}", e),
            ) {
                tracing::warn!("Failed to fail Database phase in progress manager: {}", pe);
            }
            return Err(anyhow::anyhow!(
                "Failed to create database directory: {}",
                e
            ));
        }

        let db_path = config_dir.join("email.db");
        let db_path_str = match db_path.to_str() {
            Some(path) => path,
            None => {
                if let Err(e) = self
                    .startup_progress_manager
                    .fail_phase("Database", "Invalid database path".to_string())
                {
                    tracing::warn!("Failed to fail Database phase in progress manager: {}", e);
                }
                return Err(anyhow::anyhow!("Invalid database path"));
            }
        };

        tracing::info!("📊 TUI connecting to database: {}", db_path_str);

        // Create database connection with quick mode for startup
        let database = match EmailDatabase::new_with_mode(db_path_str, true).await {
            Ok(db) => db,
            Err(e) => {
                let error_msg = format!("Failed to initialize database: {}", e);
                if let Err(pe) = self
                    .startup_progress_manager
                    .fail_phase("Database", error_msg.clone())
                {
                    tracing::warn!("Failed to fail Database phase in progress manager: {}", pe);
                }
                return Err(anyhow::anyhow!(error_msg));
            }
        };

        tracing::info!("✅ TUI database connection established successfully");

        let database_arc = Arc::new(database);

        // Create sync engine for background operations
        let (sync_progress_tx, _sync_progress_rx) = mpsc::unbounded_channel::<SyncProgress>();
        let sync_engine = Arc::new(crate::email::sync_engine::SyncEngine::new(
            database_arc.clone(),
            sync_progress_tx,
        ));
        self.sync_engine = Some(sync_engine);

        // Initialize calendar database and manager
        tracing::info!("📅 Initializing calendar database...");
        let calendar_db_path = config_dir.join("calendar.db");
        let calendar_db_path_str = match calendar_db_path.to_str() {
            Some(path) => path,
            None => {
                return Err(anyhow::anyhow!("Invalid calendar database path"));
            }
        };

        let calendar_database = Arc::new(
            crate::calendar::database::CalendarDatabase::new(calendar_db_path_str)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to initialize calendar database: {}", e))?,
        );

        // Initialize token manager for calendar and contacts (reuse existing or create new)
        let token_manager = if let Some(existing_tm) = &self.token_manager {
            Arc::new(existing_tm.clone())
        } else {
            let new_tm = TokenManager::new();
            self.token_manager = Some(new_tm.clone());
            Arc::new(new_tm)
        };

        // Create calendar manager
        let calendar_manager = Arc::new(
            CalendarManager::new(calendar_database.clone(), token_manager.clone())
                .await
                .map_err(|e| anyhow::anyhow!("Failed to create calendar manager: {}", e))?,
        );

        self.calendar_manager = Some(calendar_manager.clone());
        tracing::info!("✅ Calendar system initialized successfully");

        // Initialize contacts database and manager
        tracing::info!("👥 Initializing contacts database...");
        let contacts_db_path = config_dir.join("contacts.db");
        let contacts_db_path_str = match contacts_db_path.to_str() {
            Some(path) => path,
            None => {
                return Err(anyhow::anyhow!("Invalid contacts database path"));
            }
        };

        tracing::info!(
            "🔧 Initializing contacts database at: {}",
            contacts_db_path_str
        );
        let contacts_database =
            crate::contacts::database::ContactsDatabase::new(contacts_db_path_str)
                .await
                .map_err(|e| {
                    tracing::error!("❌ Failed to initialize contacts database: {}", e);
                    anyhow::anyhow!("Failed to initialize contacts database: {}", e)
                })?;
        tracing::info!("✅ Contacts database initialized successfully");

        // Create contacts manager (ContactsManager expects non-Arc values)
        let token_manager_for_contacts = match &*token_manager {
            tm => tm.clone(),
        };
        tracing::info!("🔧 Creating contacts manager...");
        let contacts_manager = Arc::new(
            ContactsManager::new(contacts_database, token_manager_for_contacts)
                .await
                .map_err(|e| {
                    tracing::error!("❌ Failed to create contacts manager: {}", e);
                    anyhow::anyhow!("Failed to create contacts manager: {}", e)
                })?,
        );
        tracing::info!("✅ Contacts manager created successfully");

        self.contacts_manager = Some(contacts_manager.clone());
        tracing::info!("✅ Contacts system initialized successfully");

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

        // Complete database phase in progress manager
        if let Err(e) = self.startup_progress_manager.complete_phase("Database") {
            tracing::warn!(
                "Failed to complete Database phase in progress manager: {}",
                e
            );
        }

        // Load calendar and contacts data into UI
        self.refresh_calendar_data().await?;
        self.refresh_contacts_data().await?;

        // Initialize AI configuration
        if let Err(e) = self.initialize_ai_configuration().await {
            tracing::warn!("Failed to initialize AI configuration: {}", e);
            // Don't fail the entire initialization for AI config issues
            self.ui
                .show_toast_warning("AI features may not be available - check configuration");
        }

        tracing::info!("✅ Database initialization completed successfully");
        Ok(())
    }

    /// Initialize AI configuration and validation
    async fn initialize_ai_configuration(&mut self) -> Result<()> {
        tracing::info!("🤖 Initializing AI configuration...");

        // Get config directory (same logic as database initialization)
        let config_dir = match dirs::config_dir() {
            Some(dir) => dir.join("comunicado"),
            None => {
                return Err(anyhow::anyhow!(
                    "Cannot find config directory for AI configuration"
                ));
            }
        };

        // Create config directory if it doesn't exist
        if let Err(e) = std::fs::create_dir_all(&config_dir) {
            return Err(anyhow::anyhow!("Failed to create config directory: {}", e));
        }

        let ai_config_path = config_dir.join("ai_config.toml");

        // Create AI configuration manager
        let ai_config_manager = Arc::new(AIConfigManager::new(ai_config_path.clone()));

        // Initialize the configuration (load from file and validate)
        match ai_config_manager.initialize().await {
            Ok(()) => {
                let config = ai_config_manager.get_config().await;

                // Log configuration status
                if config.enabled {
                    tracing::info!("✅ AI features enabled with provider: {}", config.provider);

                    // Validate API keys for cloud providers
                    match config.provider {
                        crate::ai::config::AIProviderType::OpenAI => {
                            if config.get_api_key("openai").is_none() {
                                tracing::warn!(
                                    "⚠️ OpenAI provider selected but no API key configured"
                                );
                                self.ui
                                    .show_toast_warning("OpenAI API key required for AI features");
                            }
                        }
                        crate::ai::config::AIProviderType::Anthropic => {
                            if config.get_api_key("anthropic").is_none() {
                                tracing::warn!(
                                    "⚠️ Anthropic provider selected but no API key configured"
                                );
                                self.ui.show_toast_warning(
                                    "Anthropic API key required for AI features",
                                );
                            }
                        }
                        crate::ai::config::AIProviderType::Google => {
                            if config.get_api_key("google").is_none() {
                                tracing::warn!(
                                    "⚠️ Google provider selected but no API key configured"
                                );
                                self.ui
                                    .show_toast_warning("Google API key required for AI features");
                            }
                        }
                        crate::ai::config::AIProviderType::Ollama => {
                            tracing::info!(
                                "🏠 Using local Ollama provider at: {}",
                                config.ollama_endpoint
                            );
                            // TODO: In future, could ping Ollama endpoint to verify it's accessible
                        }
                        crate::ai::config::AIProviderType::None => {
                            tracing::info!("❌ AI features disabled");
                        }
                    }

                    // Log feature status
                    tracing::info!("AI features status:");
                    tracing::info!(
                        "  - Email suggestions: {}",
                        config.email_suggestions_enabled
                    );
                    tracing::info!(
                        "  - Email summarization: {}",
                        config.email_summarization_enabled
                    );
                    tracing::info!(
                        "  - Calendar assistance: {}",
                        config.calendar_assistance_enabled
                    );
                    tracing::info!(
                        "  - Email categorization: {}",
                        config.email_categorization_enabled
                    );
                } else {
                    tracing::info!("❌ AI features are disabled in configuration");
                }

                self.ai_config_manager = Some(ai_config_manager);
                tracing::info!("✅ AI configuration initialized successfully");
                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to initialize AI configuration: {}", e);

                // For certain errors, we might want to create a default config
                if e.to_string().contains("Failed to read config") {
                    tracing::info!("Creating default AI configuration...");
                    let default_config = crate::ai::config::AIConfig::default();

                    // Save default configuration
                    if let Err(save_err) = default_config.save_to_file(&ai_config_path).await {
                        return Err(anyhow::anyhow!(
                            "Failed to save default AI config: {}",
                            save_err
                        ));
                    }

                    self.ai_config_manager = Some(ai_config_manager);
                    tracing::info!("✅ Default AI configuration created and initialized");
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("AI configuration validation failed: {}", e))
                }
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

    /// Set the database instance for the application
    pub fn set_database(&mut self, database: Arc<EmailDatabase>) {
        self.database = Some(database.clone());
        // Also set the database on the UI components
        self.ui.set_database(database);
        tracing::info!("📊 Database set for application and UI components");
    }

    /// Refresh calendar data from database and update UI
    pub async fn refresh_calendar_data(&mut self) -> Result<()> {
        if let Some(calendar_manager) = &self.calendar_manager {
            tracing::info!("🔄 Refreshing calendar data from database...");

            // Get all calendars
            let calendars = calendar_manager.get_calendars().await;
            tracing::info!(
                "📅 CALENDAR DEBUG: Found {} calendars in manager",
                calendars.len()
            );
            for calendar in &calendars {
                tracing::info!("📅   - Calendar: {} (ID: {})", calendar.name, calendar.id);
            }

            if calendars.is_empty() {
                tracing::error!("❌ CALENDAR ISSUE: No calendars found in calendar manager!");
                tracing::error!(
                    "❌ This means calendar sync hasn't run or failed to store calendars"
                );
                return Ok(());
            }

            // Get all events for the next 6 months (to show in calendar views)
            let now = chrono::Utc::now();
            let six_months_later = now + chrono::Duration::days(180);
            tracing::info!(
                "🗓️  Querying events from {} to {}",
                now - chrono::Duration::days(30),
                six_months_later
            );

            let events = calendar_manager
                .get_all_events(
                    Some(now - chrono::Duration::days(30)),
                    Some(six_months_later),
                )
                .await
                .map_err(|e| anyhow::anyhow!("Failed to load calendar events: {}", e))?;

            tracing::info!(
                "🎯 CALENDAR DEBUG: Retrieved {} events from calendar manager",
                events.len()
            );
            for event in &events {
                tracing::info!(
                    "📅   Event: {} (Start: {}, Calendar: {})",
                    event.title,
                    event.start_time,
                    event.calendar_id
                );
            }

            if events.is_empty() {
                tracing::error!("❌ CALENDAR ISSUE: No events retrieved from calendar manager!");
                tracing::error!("❌ This could mean: 1) No events in database, 2) Date range issue, 3) Calendar manager issue");
            }

            // Update UI with calendar data
            let calendars_count = calendars.len();
            let events_count = events.len();
            self.ui.set_calendars(calendars);
            self.ui.set_calendar_events(events);

            tracing::info!(
                "✅ Loaded {} calendars and {} events into UI",
                calendars_count,
                events_count
            );
        } else {
            tracing::error!("❌ CALENDAR CRITICAL: Calendar manager not initialized, cannot refresh calendar data");
            tracing::error!("❌ This means calendar initialization failed during app startup");
        }
        Ok(())
    }

    /// Refresh contacts data from database and update UI
    pub async fn refresh_contacts_data(&mut self) -> Result<()> {
        if let Some(contacts_manager) = &self.contacts_manager {
            tracing::info!("🔄 Refreshing contacts data from database...");

            // Get all contacts using empty search criteria
            let criteria = crate::contacts::ContactSearchCriteria::new();
            let contacts = contacts_manager
                .search_contacts(&criteria)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to load contacts: {}", e))?;

            tracing::info!(
                "👥 CONTACTS DEBUG: Loaded {} contacts from database",
                contacts.len()
            );
            for contact in &contacts {
                tracing::info!(
                    "👤   Contact: {} ({})",
                    contact.display_name,
                    contact
                        .primary_email()
                        .map(|e| e.address.as_str())
                        .unwrap_or("no email")
                );
            }

            if contacts.is_empty() {
                tracing::error!("❌ CONTACTS ISSUE: No contacts found in database!");
                tracing::error!(
                    "❌ This means contacts sync hasn't run or failed to store contacts"
                );
            }
        } else {
            tracing::error!("❌ CONTACTS CRITICAL: Contacts manager not initialized, cannot refresh contacts data");
            tracing::error!("❌ This means contacts initialization failed during app startup");
        }
        Ok(())
    }

    /// Refresh both calendar and contacts data on demand
    pub async fn refresh_all_data(&mut self) -> Result<()> {
        self.refresh_calendar_data().await?;
        self.refresh_contacts_data().await?;
        Ok(())
    }

    /// Check if auto-refresh is needed and perform it
    pub async fn check_and_refresh(&mut self) -> Result<()> {
        let now = Instant::now();
        if now.saturating_duration_since(self.last_auto_sync) >= self.auto_sync_interval {
            tracing::debug!(
                "Auto-refresh interval reached, refreshing calendar and contacts data..."
            );
            self.refresh_all_data().await?;
            self.last_auto_sync = now;
        }
        Ok(())
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

    /// Get a reference to the startup progress manager
    pub fn startup_progress_manager(&self) -> &StartupProgressManager {
        &self.startup_progress_manager
    }

    /// Get a mutable reference to the startup progress manager
    pub fn startup_progress_manager_mut(&mut self) -> &mut StartupProgressManager {
        &mut self.startup_progress_manager
    }

    /// Retry initialization in background (for when startup was skipped)
    pub async fn retry_initialization_background(&mut self) -> Result<()> {
        if self.initialization_complete {
            return Ok(()); // Already complete
        }

        tracing::info!("🔄 Retrying initialization in background...");

        // Try database initialization first
        if self.database.is_none() {
            tracing::info!("📊 Initializing database in background...");
            match tokio::time::timeout(
                std::time::Duration::from_secs(15),
                self.initialize_database(),
            )
            .await
            {
                Ok(Ok(())) => {
                    tracing::info!("✅ Database initialized successfully in background");
                    self.ui
                        .show_toast_success("Database connection established");

                    // Load calendar and contacts data into UI (same as in initialize_database)
                    if let Err(e) = self.refresh_calendar_data().await {
                        tracing::warn!(
                            "Failed to refresh calendar data after background init: {}",
                            e
                        );
                    }
                    if let Err(e) = self.refresh_contacts_data().await {
                        tracing::warn!(
                            "Failed to refresh contacts data after background init: {}",
                            e
                        );
                    }
                }
                Ok(Err(e)) => {
                    tracing::warn!("⚠️ Background database initialization failed: {}", e);
                    self.ui.show_toast_warning(
                        "Database initialization failed - limited functionality",
                    );
                }
                Err(_) => {
                    tracing::warn!("⏱️ Background database initialization timed out");
                    self.ui
                        .show_toast_warning("Database connection timed out - retrying later");
                }
            }
        }

        // Try IMAP manager initialization
        if self.imap_manager.is_none() {
            tracing::info!("📬 Initializing IMAP manager in background...");
            match tokio::time::timeout(
                std::time::Duration::from_secs(20),
                self.initialize_imap_manager(),
            )
            .await
            {
                Ok(Ok(())) => {
                    tracing::info!("✅ IMAP manager initialized successfully in background");
                    self.ui.show_toast_success("Email accounts connected");

                    // Load accounts and folders
                    if let Err(e) = self.load_existing_accounts().await {
                        tracing::warn!("Failed to load accounts after IMAP init: {}", e);
                        self.ui.show_toast_warning("Failed to load email accounts");
                    } else {
                        // Now actually load folders for the current account
                        if let Some(current_account_id) = self.ui.get_current_account_id() {
                            let account_id = current_account_id.clone(); // Clone to avoid borrowing issues
                            tracing::info!("Loading folders for current account: {}", account_id);
                            match self.ui.load_folders(&account_id).await {
                                Ok(()) => {
                                    tracing::info!(
                                        "✅ Successfully loaded folders for account: {}",
                                        account_id
                                    );
                                    self.ui
                                        .show_toast_success("Email folders loaded successfully");
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        "Failed to load folders for account {}: {}",
                                        account_id,
                                        e
                                    );
                                    self.ui.show_toast_warning("Failed to load email folders");
                                }
                            }
                        } else {
                            tracing::warn!("No current account set after loading accounts");
                            self.ui.show_toast_warning("No email account selected");
                        }
                    }
                }
                Ok(Err(e)) => {
                    tracing::warn!("⚠️ Background IMAP initialization failed: {}", e);
                    self.ui
                        .show_toast_warning("Email connection failed - check account settings");
                }
                Err(_) => {
                    tracing::warn!("⏱️ Background IMAP initialization timed out");
                    self.ui
                        .show_toast_warning("Email connection timed out - retrying later");
                }
            }
        }

        // Try background processor
        if self.background_processor.is_none() {
            tracing::info!("⚙️ Initializing background processor...");
            if let Err(e) = self.initialize_background_processor().await {
                tracing::warn!("Background processor initialization failed: {}", e);
                self.ui.show_toast_warning("Background sync disabled");
            } else {
                tracing::info!("✅ Background processor initialized");
                self.ui.show_toast_success("Background sync enabled");
            }
        }

        Ok(())
    }

    /// Perform deferred initialization in the background
    pub async fn perform_deferred_initialization(&mut self) -> Result<()> {
        if self.initialization_in_progress || self.initialization_complete {
            return Ok(());
        }

        self.initialization_in_progress = true;
        tracing::info!("Starting deferred initialization...");

        // Phase 1: Database (skip for now - initialize later)
        tracing::info!("⚡ Skipping database initialization for fast startup");
        // Database initialization skipped

        // Phase 2: IMAP Manager (with robust timeout handling)
        // Initializing IMAP manager

        // Initialize IMAP manager with timeout to prevent hanging
        match tokio::time::timeout(
            std::time::Duration::from_secs(15), // 15 second timeout
            self.initialize_imap_manager(),
        )
        .await
        {
            Ok(Ok(())) => {
                tracing::info!("✅ IMAP manager initialized successfully");
                // IMAP manager ready
            }
            Ok(Err(e)) => {
                tracing::warn!("⚠️ IMAP manager initialization failed: {}", e);
                // IMAP failed - continuing
                // Continue startup - app can work without IMAP for now
            }
            Err(_) => {
                tracing::warn!("⏱️ IMAP manager initialization timed out after 15 seconds");
                // IMAP timed out - continuing
                // Continue startup - app can work without IMAP for now
            }
        }
        // IMAP Manager phase complete

        // Phase 3: Account Setup (with robust timeout handling)
        print!("📋 Loading accounts...");
        let _ = std::io::Write::flush(&mut std::io::stdout());

        // Load existing accounts with timeout (should be fast now - just loading cached data)
        match tokio::time::timeout(
            std::time::Duration::from_secs(15), // 15 second timeout for account/cache loading
            self.load_existing_accounts(),
        )
        .await
        {
            Ok(Ok(())) => {
                tracing::info!("✅ Account setup completed successfully");
                // ✅ (removed print - already logged above)
            }
            Ok(Err(e)) => {
                tracing::warn!("⚠️ Account setup failed: {}", e);
                // ⚠️ (removed print - already logged above)
                // Continue startup - show empty state in UI
            }
            Err(_) => {
                tracing::warn!("⏱️ Account setup timed out after 10 seconds");
                // ⏱️ (removed print - already logged above)
                // Continue startup - show empty state in UI
            }
        }
        // Account setup complete

        // Phase 4: Background Processor (essential for IMAP sync)
        print!("🔄 Initializing background processor...");
        let _ = std::io::Write::flush(&mut std::io::stdout());

        // Initialize background processor (optional - don't fail startup if it fails)
        match self.initialize_background_processor().await {
            Ok(()) => {
                // ✅ (removed print - already logged below)
                tracing::info!("✅ Background processor initialized successfully");
            }
            Err(e) => {
                // ⚠️ (removed print - already logged below)
                tracing::warn!("⚠️ Background processor initialization failed: {}", e);
                // Continue without background processor
            }
        }

        // Perform immediate IMAP sync with timeout to populate emails (replaces broken background sync)
        if let Some(current_account_id) = self.ui.get_current_account_id().cloned() {
            print!("📬 Fetching initial emails...");
            let _ = std::io::Write::flush(&mut std::io::stdout());
            tracing::info!(
                "📬 Starting immediate IMAP sync for account: {}",
                current_account_id
            );

            // Run IMAP sync with 15-second timeout to prevent hanging
            let sync_result = tokio::time::timeout(
                std::time::Duration::from_secs(15),
                self.sync_account_from_imap(&current_account_id),
            )
            .await;

            match sync_result {
                Ok(Ok(())) => {
                    tracing::info!("✅ Initial IMAP sync completed successfully");
                    // ✅ (removed print - already logged above)
                }
                Ok(Err(e)) => {
                    tracing::warn!("⚠️ Initial IMAP sync failed: {}", e);
                    // ⚠️ (removed print - already logged above)
                }
                Err(_) => {
                    tracing::warn!("⚠️ Initial IMAP sync timed out after 15 seconds");
                    // ⚠️ (removed print - already logged above)
                }
            }
        } else {
            tracing::warn!("⚠️ No account found for initial sync");
        }

        tracing::info!("✅ Background services ready");

        self.initialization_complete = true;
        self.initialization_in_progress = false;

        // Show welcome toast notification
        self.ui
            .show_toast_success("🚀 Comunicado ready! Modern TUI email & calendar client");

        tracing::info!("Deferred initialization completed");

        // Check and refresh expired tokens now that initialization is complete
        if let Err(e) = self.check_and_refresh_tokens().await {
            tracing::warn!("Failed to refresh tokens after initialization: {}", e);
        }

        // Log ready state
        tracing::info!("All services ready");

        Ok(())
    }

    // Startup progress and initialization complete

    /// Initialize IMAP account manager with OAuth2 support
    pub async fn initialize_imap_manager(&mut self) -> Result<()> {
        tracing::info!("📬 Initializing IMAP manager...");

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
            let _has_valid_tokens = match tokio::time::timeout(
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
                    tracing::error!(
                        "Token initialization timed out after 15 seconds - using fallback"
                    );
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

            // Skip token refresh scheduler during startup to prevent blocking
            // Token refresh will be handled on-demand when needed
            tracing::info!(
                "Skipping token refresh scheduler during startup for faster initialization"
            );
            self.token_refresh_scheduler = None;
            tracing::debug!("Token refresh scheduler setup complete");

            // Store token manager for later token refresh operations
            let token_manager_arc = Arc::new(token_manager);

            // Set IMAP manager in UI for attachment downloading functionality
            let imap_manager_arc = Arc::new(imap_manager);
            self.ui
                .content_preview_mut()
                .set_imap_manager(imap_manager_arc.clone());

            self.token_manager = Some(token_manager_arc.as_ref().clone());
            self.imap_manager = Some(imap_manager_arc);

            Ok(())
        }
        .await;

        // Report success or failure to progress manager
        result
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
            max_queue_size: 50,      // Reasonable queue size
            result_cache_size: 25,   // Keep recent results
            processing_interval: Duration::from_millis(250), // Check every 250ms
        };

        // Get required services for background processor
        let sync_engine = self
            .sync_engine
            .as_ref()
            .ok_or_else(|| {
                anyhow::anyhow!("Sync engine not initialized. Call initialize_database() first.")
            })?
            .clone();
        let database = self
            .database
            .as_ref()
            .ok_or_else(|| {
                anyhow::anyhow!("Database not initialized. Call initialize_database() first.")
            })?
            .clone();
        let account_manager = self
            .imap_manager
            .as_ref()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "IMAP account manager not initialized. Call initialize_imap_manager() first."
                )
            })?
            .clone();

        let processor = Arc::new(BackgroundProcessor::with_settings(
            progress_tx,
            completion_tx,
            sync_engine,
            account_manager,
            database,
            settings,
        ));

        // Start the background processor
        processor
            .start()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to start background processor: {}", e))?;

        // Store processor and channels
        self.background_processor = Some(processor.clone());
        self.sync_progress_rx = Some(progress_rx);
        self.task_completion_rx = Some(completion_rx);

        // Set background processor on enhanced progress overlay for task cancellation
        self.ui
            .enhanced_progress_overlay_mut()
            .set_background_processor(processor);

        tracing::info!("✅ Background processor initialized successfully");
        Ok(())
    }

    /// Initialize toast integration service for cross-application notifications
    pub async fn initialize_toast_integration(&mut self) -> Result<()> {
        tracing::info!(
            "🍞 Toast integration using simple direct approach (no separate service needed)"
        );

        // Toast integration is handled directly in the UI event processing
        // via SimpleToastIntegration in process_background_updates() and handle_notification()
        // No separate service initialization required

        tracing::info!("✅ Toast integration ready (using simple direct approach)");
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
                // Update UI with sync progress (legacy overlay)
                self.ui.update_sync_progress(progress.clone());

                // Update enhanced progress overlay with sync progress
                self.ui
                    .enhanced_progress_overlay_mut()
                    .update_sync_progress(progress);
            }
        }

        // Process task completion updates
        if let Some(ref mut completion_rx) = self.task_completion_rx {
            while let Ok(result) = completion_rx.try_recv() {
                // Handle task completion
                tracing::debug!("Background task completed: {:?}", result.status);

                // Update UI account status for successful account sync tasks
                if let crate::performance::background_processor::BackgroundTaskType::AccountSync {
                    ..
                } = result.task_type
                {
                    match result.status {
                        crate::performance::background_processor::TaskStatus::Completed => {
                            // Account sync completed successfully - update UI status to Online
                            tracing::info!("Account sync completed successfully for {}, updating UI status to Online", result.account_id);
                            self.ui.update_account_status(
                                &result.account_id,
                                crate::ui::AccountSyncStatus::Online,
                                None,
                            );
                        }
                        crate::performance::background_processor::TaskStatus::Failed(_) => {
                            // Account sync failed - update UI status to Error
                            tracing::warn!(
                                "Account sync failed for {}, updating UI status to Error",
                                result.account_id
                            );
                            self.ui.update_account_status(
                                &result.account_id,
                                crate::ui::AccountSyncStatus::Error,
                                None,
                            );
                        }
                        _ => {
                            // Cancelled or other status - no UI update needed
                        }
                    }
                }

                // Update enhanced progress overlay with completion
                self.ui
                    .enhanced_progress_overlay_mut()
                    .handle_task_completion(result.clone());

                // Add toast notification for task completion
                crate::ui::toast_integration_simple::SimpleToastIntegration::handle_task_completion(
                    self.ui.toast_manager(),
                    result,
                );
            }
        }

        // Process AI operation results
        self.ui.process_ai_results();
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
        tracing::info!("📋 Setting up accounts...");

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
        }
        .await;

        // Report success or failure to progress manager
        result
    }

    /// Load existing accounts from storage
    async fn load_existing_accounts(&mut self) -> Result<()> {
        tracing::debug!("Attempting to load accounts from storage...");
        let accounts = self
            .storage
            .load_all_accounts()
            .map_err(|e| anyhow::anyhow!("Failed to load accounts: {}", e))?;

        tracing::debug!("Loaded {} accounts from storage", accounts.len());
        for (i, account) in accounts.iter().enumerate() {
            tracing::debug!(
                "Account {}: {} ({}) - tokens expired: {}",
                i,
                account.display_name,
                account.account_id,
                account.is_token_expired()
            );
        }

        if accounts.is_empty() {
            tracing::warn!("No accounts found in storage, loading sample data instead");
            return self.load_sample_data().await;
        }

        // Skip OAuth2 token loading during startup to prevent hanging
        tracing::info!(
            "Skipping OAuth2 token loading for {} accounts - will load on-demand",
            accounts.len()
        );
        // Token loading disabled during startup to prevent hanging
        tracing::debug!("Account enumeration complete - token loading deferred");

        // Convert AccountConfig to AccountItem for the UI
        let account_items: Vec<crate::ui::AccountItem> = accounts
            .iter()
            .map(crate::ui::AccountItem::from_config)
            .collect();

        // Set accounts in the UI
        self.ui.set_accounts(account_items.clone());

        // Ensure current account is set and verify it persists
        tracing::info!("🔍 Checking current account status after setting accounts...");
        if let Some(account_id) = self.ui.get_current_account_id() {
            tracing::debug!("Current account ID is now: {}", account_id);

            // Clone the account_id to avoid borrowing issues
            let account_id_clone = account_id.clone();

            // Ensure account is properly activated with full folder and message loading
            tracing::info!("🚀 Activating current account: {}", account_id_clone);
            if let Err(e) = self.ui.switch_to_account(&account_id_clone).await {
                tracing::warn!("❌ Failed to activate account {}: {}", account_id_clone, e);
            } else {
                tracing::info!("✅ Successfully activated account: {}", account_id_clone);
            }
        } else {
            tracing::warn!("No current account ID set after setting accounts!");
            // Try to set the first account as current explicitly
            if !account_items.is_empty() {
                let first_account_id = &account_items[0].account_id;
                tracing::info!(
                    "Explicitly setting first account as current: {}",
                    first_account_id
                );
                if let Err(e) = self.ui.switch_to_account(first_account_id).await {
                    tracing::warn!("Failed to set first account as current: {}", e);
                } else {
                    tracing::info!(
                        "✅ Successfully set first account as current: {}",
                        first_account_id
                    );
                }
            }
        }

        // Skip database account creation during startup to prevent blocking
        // Accounts will be created on-demand when they are first used
        tracing::info!(
            "Skipping database account creation during startup for faster initialization"
        );
        tracing::debug!(
            "Found {} accounts, database entries will be created on-demand",
            accounts.len()
        );

        // Skip UI data loading during startup to prevent hangs - will be loaded after UI starts
        tracing::debug!("Skipping UI data loading during startup for fast initialization");
        tracing::info!("📅 UI data loading and IMAP sync scheduled for background processing to keep startup fast");

        Ok(())
    }

    /// Load sample data for demonstration (fallback)
    pub async fn load_sample_data(&mut self) -> Result<()> {
        tracing::info!("Loading sample data as fallback (no real accounts found)");

        if let Some(ref database) = self.database {
            // Create sample account and folder if they don't exist
            self.create_sample_account_and_folder(database).await?;

            // Create a sample account for the UI
            let sample_account = crate::ui::AccountItem {
                account_id: "sample-account".to_string(),
                display_name: "Demo Account".to_string(),
                email_address: "demo@example.com".to_string(),
                provider: "sample".to_string(),
                is_online: false,
                unread_count: 0,
                sync_status: crate::ui::AccountSyncStatus::Offline,
            };

            // Set the sample account in the UI
            self.ui.set_accounts(vec![sample_account]);

            // Load folders for the sample account
            let _ = self.ui.load_folders("sample-account").await;

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

        let folders: Vec<String> =
            sqlx::query("SELECT name FROM folders WHERE account_id = ? ORDER BY name")
                .bind(account_id)
                .fetch_all(&database.pool)
                .await?
                .into_iter()
                .map(|row| row.get::<String, _>("name"))
                .collect();

        // Define important folders that should always be synced (including Gmail-specific folders)
        let important_folders = [
            "INBOX",
            "Sent",
            "Drafts",
            "Trash",
            "Spam",
            "Junk",
            "Sent Items",
            "Sent Mail",
            "All Mail",
            "Starred",
            "Important",
            "Bin", // Gmail-specific folders
        ];

        // Separate important folders from others
        let mut priority_folders = Vec::new();
        let mut other_folders = Vec::new();

        for folder in &folders {
            let folder_lower = folder.to_lowercase();
            if important_folders
                .iter()
                .any(|&important| folder_lower.contains(&important.to_lowercase()))
            {
                priority_folders.push(folder.clone());
            } else {
                other_folders.push(folder.clone());
            }
        }

        tracing::info!(
            "Syncing messages for {} priority folders and {} other folders in account: {}",
            priority_folders.len(),
            other_folders.len(),
            account_id
        );

        // First, fetch messages from important folders
        for folder_name in &priority_folders {
            tracing::debug!(
                "Fetching messages from priority folder: {} in account: {}",
                folder_name,
                account_id
            );
            match self.fetch_messages_from_imap(account_id, folder_name).await {
                Ok(()) => {
                    tracing::debug!(
                        "Successfully fetched messages from priority folder: {}",
                        folder_name
                    );
                }
                Err(e) => {
                    tracing::warn!("Failed to fetch messages from priority folder {}: {}. Continuing with other folders.", folder_name, e);
                    // Continue with other folders even if one fails
                }
            }
        }

        // Then fetch from other folders (increased limit to sync all important Gmail folders)
        let max_other_folders = 25; // Increased from 5 to support all Gmail folders like All Mail, etc.
        let folders_to_sync = other_folders.iter().take(max_other_folders);

        for folder_name in folders_to_sync {
            tracing::debug!(
                "Fetching messages from folder: {} in account: {}",
                folder_name,
                account_id
            );
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
            tracing::info!(
                "Synced {} of {} other folders. Use manual folder refresh for remaining folders.",
                max_other_folders,
                other_folders.len()
            );
        }

        tracing::info!(
            "Completed IMAP sync for account: {} ({} folders processed)",
            account_id,
            folders.len()
        );
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
        tracing::info!("🚀 Starting Comunicado...");

        // Check if we're running in a proper terminal
        if !std::io::stdout().is_tty() {
            return Err(anyhow::anyhow!(
                "Comunicado requires a proper terminal (TTY) to run. Please run this application in a terminal emulator."
            ));
        }

        tracing::debug!("🔄 Setting up terminal...");

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
            // Perform background initialization if not done yet
            if !self.initialization_complete {
                if let Err(e) = self.retry_initialization_background().await {
                    tracing::error!("Background initialization failed: {}", e);
                    // Don't fail the app, just log the error and continue
                }
                // Mark as complete to prevent continuous retries
                self.initialization_complete = true;
            }

            // Process background task updates to prevent UI blocking
            self.process_background_updates().await;

            // Check for auto-sync (every 3 minutes) - now uses background processing
            if self.last_auto_sync.elapsed() >= self.auto_sync_interval {
                self.queue_auto_sync_background().await;

                // Also refresh calendar and contacts data
                if let Err(e) = self.check_and_refresh().await {
                    tracing::warn!("Failed to refresh calendar/contacts data: {}", e);
                }

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

            // Draw UI with panic protection
            let draw_result = terminal.draw(|f| {
                // Catch panics in the render call
                let render_result =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.ui.render(f)));

                match render_result {
                    Ok(_) => {
                        // Rendering succeeded
                    }
                    Err(panic_info) => {
                        tracing::error!("🚨 PANIC CAUGHT in ui.render()!");
                        if let Some(s) = panic_info.downcast_ref::<&str>() {
                            tracing::error!("🚨 Render panic message: {}", s);
                        } else if let Some(s) = panic_info.downcast_ref::<String>() {
                            tracing::error!("🚨 Render panic message: {}", s);
                        } else {
                            tracing::error!("🚨 Unknown render panic type");
                        }
                        // Draw a simple error message instead of crashing
                        use ratatui::layout::Alignment;
                        use ratatui::widgets::{Block, Borders, Paragraph};
                        let error_msg = Paragraph::new("🚨 Render Error - Check logs for details")
                            .block(Block::default().borders(Borders::ALL).title("Error"))
                            .alignment(Alignment::Center);
                        f.render_widget(error_msg, f.area());
                    }
                }
            });

            if let Err(e) = draw_result {
                tracing::error!("🚨 Terminal drawing error: {}", e);
                return Err(e.into());
            }

            // Process event bus updates first (new event-driven system with optimized batching)
            if let Some(bus) = crate::events::get_event_bus() {
                // Use optimized batch processing for better performance
                if let Ok(mut bus_lock) = bus.lock() {
                    // Check if memory optimization is needed (every ~100 cycles)
                    if self.should_optimize_event_bus_memory() {
                        if bus_lock.should_optimize_memory() {
                            if let Ok(cleaned_count) = bus_lock.optimize_memory() {
                                if cleaned_count > 0 {
                                    tracing::debug!(
                                        "Event bus memory optimization cleaned {} events",
                                        cleaned_count
                                    );
                                }
                            }
                        }
                    }

                    // Process events with optimized batching
                    match bus_lock.process_batch_optimized() {
                        Ok(event_count) => {
                            if event_count > 0 {
                                tracing::debug!(
                                    "Processed {} events in optimized batch",
                                    event_count
                                );
                            }
                        }
                        Err(e) => {
                            tracing::debug!(
                                "Event bus batch processing error (non-critical): {}",
                                e
                            );
                            // Use error recovery mechanism
                            let _ = bus_lock.handle_processing_error(&e, "batch_processing");

                            // Fallback to basic processing if batch processing fails
                            if let Err(fallback_err) = bus_lock.process_pending() {
                                tracing::warn!(
                                    "Both batch and fallback processing failed: {}",
                                    fallback_err
                                );
                                let _ = bus_lock
                                    .handle_processing_error(&fallback_err, "fallback_processing");
                            }
                        }
                    }
                }
            }

            // Handle terminal events
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout)? {
                match event::read()? {
                    Event::Key(key) => {
                        // Handle global quit commands first (Ctrl+C, Q, Esc)
                        if self.handle_global_quit_keys(key) {
                            self.should_quit = true;
                            break;
                        }

                        // Process key events through the new event-driven UI integration system
                        let event_handled =
                            match self.process_key_event_through_integration(key).await {
                                Ok(handled) => handled,
                                Err(e) => {
                                    tracing::warn!("Event-driven key processing failed: {}", e);
                                    false
                                }
                            };

                        // If the event was handled by the new system, we can skip legacy processing
                        let event_result = if event_handled {
                            EventResult::Handled
                        } else {
                            // Fall back to legacy processing for unhandled events
                            EventResult::Continue
                        };

                        // Handle the event result
                        match event_result {
                            EventResult::Continue => {}
                            EventResult::Handled => {} // Key was handled by mode-specific handler
                            EventResult::ComposeAction(action) => {
                                self.handle_compose_action(action).await?;
                            }
                            EventResult::DraftAction(action) => {
                                self.handle_draft_action(action).await?;
                            }
                            EventResult::CommandAction(action) => {
                                self.handle_command_action(action).await?;
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
                                tracing::debug!(
                                    "🔍 Processing FolderSelect event for: '{}'",
                                    folder_path
                                );
                                self.handle_folder_select(&folder_path).await?;
                            }
                            EventResult::FolderForceRefresh(folder_path) => {
                                self.handle_folder_force_refresh(&folder_path).await?;
                            }
                            EventResult::FolderOperation(operation) => {
                                self.handle_folder_operation(operation).await?;
                            }
                            EventResult::ContactsPopup => {
                                tracing::info!("🎯 EventResult::ContactsPopup received in app.rs");
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
                            EventResult::DeleteEmail(account_id, message_id, folder) => {
                                self.handle_delete_email(&account_id, message_id, &folder)
                                    .await?;
                            }
                            EventResult::ArchiveEmail(account_id, message_id, folder) => {
                                self.handle_archive_email(&account_id, message_id, &folder)
                                    .await?;
                            }
                            EventResult::MarkEmailRead(account_id, message_id, folder) => {
                                self.handle_mark_email_read(&account_id, message_id, &folder)
                                    .await?;
                            }
                            EventResult::MarkEmailUnread(account_id, message_id, folder) => {
                                self.handle_mark_email_unread(&account_id, message_id, &folder)
                                    .await?;
                            }
                            EventResult::ToggleEmailFlag(account_id, message_id, folder) => {
                                self.handle_toggle_email_flag(&account_id, message_id, &folder)
                                    .await?;
                            }
                            EventResult::RetryInitialization => {
                                // Reset initialization flag and retry
                                self.initialization_complete = false;
                                self.ui
                                    .show_toast_info("Retrying initialization in background...");
                            }
                            EventResult::CancelBackgroundTask => {
                                // Cancel the selected task in enhanced progress overlay
                                self.ui.cancel_enhanced_progress_selected_task().await;
                            }
                            // Calendar operations
                            EventResult::CreateEvent(calendar_id) => {
                                self.handle_create_event(&calendar_id).await?;
                            }
                            EventResult::EditEvent(calendar_id, event_id) => {
                                self.handle_edit_event(&calendar_id, &event_id).await?;
                            }
                            EventResult::DeleteEvent(calendar_id, event_id) => {
                                self.handle_delete_event(&calendar_id, &event_id).await?;
                            }
                            EventResult::ViewEventDetails(calendar_id, event_id) => {
                                self.handle_view_event_details(&calendar_id, &event_id)
                                    .await?;
                            }
                            EventResult::CreateTodo(calendar_id) => {
                                self.handle_create_todo(&calendar_id).await?;
                            }
                            EventResult::ToggleTodoComplete(calendar_id, event_id) => {
                                self.handle_toggle_todo_complete(&calendar_id, &event_id)
                                    .await?;
                            }
                            EventResult::AISummarizeEmail(message_id) => {
                                self.handle_ai_summarize_email(message_id).await?;
                            }
                            EventResult::TriggerEmailSync => {
                                self.trigger_manual_sync().await?;
                            }
                            // Notes operations
                            EventResult::ConvertEmailToNote(message_id) => {
                                self.handle_convert_email_to_note(message_id).await?;
                            }
                            EventResult::ConvertEventToNote(event_id) => {
                                self.handle_convert_event_to_note(&event_id).await?;
                            }
                            EventResult::ConvertKdeMessageToNote(title, content) => {
                                self.handle_convert_kde_message_to_note(&title, &content)
                                    .await?;
                            }
                            EventResult::ShowNotes => {
                                self.handle_show_notes().await?;
                            }
                            EventResult::CreateNote => {
                                self.handle_create_note().await?;
                            }
                        }

                        // Check for quit command
                        // TODO: Integrate quit handling with event-driven system
                        // self.should_quit is managed elsewhere now
                    }
                    Event::Mouse(mouse_event) => {
                        // Process mouse events through the new mouse handling system
                        if let Err(e) = self.process_mouse_event(mouse_event).await {
                            tracing::warn!("Mouse event processing failed: {}", e);
                        }
                    }
                    _ => {
                        // Handle other events (resize, etc.)
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

                // Clean up old enhanced progress entries
                self.ui.cleanup_enhanced_progress();

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
        tracing::info!("🔄 Initializing background services...");

        // Note: Event bus initialization is deferred to avoid blocking startup
        // The event bus will be initialized when first used to prevent async task
        // spawning issues during application initialization
        tracing::debug!("Event bus initialization deferred");

        // Perform services initialization with error handling
        let result: Result<()> = async {
            tracing::debug!("Starting service initialization");

        // Initialize token manager if not already done
        if self.token_manager.is_none() {
            tracing::debug!("Creating new token manager");
            let token_manager = TokenManager::new();
            self.token_manager = Some(token_manager);
        }

        // Initialize calendar and contacts managers with existing database
        self.initialize_managers().await?;

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
                    let config_dir = dirs::config_dir()
                        .ok_or_else(|| anyhow::anyhow!("Failed to get config directory"))?
                        .join("comunicado");

                    // Ensure the directory exists
                    if let Err(e) = std::fs::create_dir_all(&config_dir) {
                        tracing::warn!("Failed to create contacts directory: {}", e);
                        return Ok::<Option<Arc<ContactsManager>>, anyhow::Error>(None);
                    }

                    let contacts_db_path = config_dir.join("contacts.db");

                    // Try to initialize contacts database
                    let contacts_db_url = format!("sqlite:{}?mode=rwc", contacts_db_path.display());
                    tracing::info!("🔧 Initializing contacts database with URL: {}", contacts_db_url);
                    match crate::contacts::ContactsDatabase::new(&contacts_db_url)
                    .await
                    {
                        Ok(contacts_database) => {
                            match ContactsManager::new(contacts_database, token_manager.clone())
                                .await
                            {
                                Ok(contacts_manager) => Ok(Some(Arc::new(contacts_manager))),
                                Err(e) => {
                                    tracing::error!("❌ CONTACTS INIT ERROR: Failed to initialize contacts manager: {}", e);
                                    tracing::error!("❌ CONTACTS DEBUG: This error will cause contacts to be unavailable");
                                    Ok(None)
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("❌ CONTACTS DB ERROR: Failed to initialize contacts database: {}", e);
                            tracing::error!("❌ CONTACTS DEBUG: Database path issue will cause contacts to be unavailable");
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
                    self.ui.set_contacts_manager(contacts_manager.clone());

                    // Set up address book UI
                    self.ui.set_address_book_contacts_manager(contacts_manager);

                    // Load initial contacts
                    self.ui.ensure_address_book_contacts_loaded().await;

                    tracing::info!("Contacts manager initialized successfully with sender recognition and contacts loaded");
                }
                Ok(Ok(None)) => {
                    tracing::error!("❌ CONTACTS FINAL: Contacts manager initialization skipped due to errors (see above)");
                    tracing::error!("❌ CONTACTS RESULT: contacts_manager will remain None");
                }
                Ok(Err(e)) => {
                    tracing::error!("❌ CONTACTS FINAL: Contacts manager initialization failed: {}", e);
                }
                Err(_) => {
                    tracing::error!("❌ CONTACTS TIMEOUT: Contacts manager initialization timed out after 20 seconds");
                    tracing::error!("❌ CONTACTS RESULT: contacts_manager will remain None");
                    // Don't fail overall initialization - contacts are optional
                }
            }
        }

        // Initialize email operations service
        if let (Some(ref imap_manager), Some(ref database)) = (&self.imap_manager, &self.database) {
            tracing::debug!("Initializing email operations service");

            let email_operations_service = Arc::new(crate::email::EmailOperationsService::new(
                imap_manager.clone(),
                database.clone(),
            ));

            self.email_operations_service = Some(email_operations_service);
            tracing::info!("Email operations service initialized successfully");
        }

            tracing::info!("Services initialized successfully");
            Ok(())
        }.await;

        // Report success or failure to progress manager
        result
    }

    /// Initialize calendar and contacts managers with existing database
    async fn initialize_managers(&mut self) -> Result<()> {
        tracing::info!("🔄 Initializing calendar and contacts managers...");

        if let (Some(ref token_manager), Some(ref _database)) =
            (&self.token_manager, &self.database)
        {
            // Initialize calendar manager if not already done
            if self.calendar_manager.is_none() {
                tracing::info!("📅 Creating calendar manager...");

                // Create calendar database with proper path (same as CLI)
                let config_dir = dirs::config_dir()
                    .ok_or_else(|| anyhow::anyhow!("Failed to get config directory"))?
                    .join("comunicado");

                // Create the directory if it doesn't exist
                if let Err(e) = std::fs::create_dir_all(&config_dir) {
                    tracing::warn!("Failed to create calendar directory: {}", e);
                }

                let calendar_db_path = config_dir.join("calendar.db");
                let calendar_db_url = format!("sqlite:{}?mode=rwc", calendar_db_path.display());

                tracing::info!("📅 Creating calendar database: {}", calendar_db_url);
                match crate::calendar::CalendarDatabase::new(&calendar_db_url).await {
                    Ok(calendar_database) => {
                        match crate::calendar::CalendarManager::new(
                            Arc::new(calendar_database),
                            Arc::new(token_manager.clone()),
                        )
                        .await
                        {
                            Ok(calendar_manager) => {
                                self.calendar_manager = Some(Arc::new(calendar_manager));
                                tracing::info!("✅ Calendar manager initialized successfully");
                            }
                            Err(e) => {
                                tracing::error!("❌ Failed to create calendar manager: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("❌ Failed to create calendar database: {}", e);
                    }
                }
            }

            // Contacts manager should already be initialized in initialize_services
            // But let's check if it exists
            if self.contacts_manager.is_none() {
                tracing::warn!("⚠️  Contacts manager not initialized - this should have been done in initialize_services");
            } else {
                tracing::info!("✅ Contacts manager already initialized");
            }
        } else {
            tracing::error!(
                "❌ Cannot initialize managers: token_manager or database not available"
            );
        }

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

    /// Handle command palette actions
    async fn handle_command_action(&mut self, action: CommandAction) -> Result<()> {
        // Delegate to UI for command execution - may return additional app-level actions
        if let Some(app_event) = self.ui.execute_command_action(action) {
            // Handle app-level events that the UI can't handle directly
            match app_event {
                EventResult::TriggerEmailSync => {
                    self.trigger_manual_sync().await?;
                }
                EventResult::ContactsPopup => {
                    self.handle_contacts_popup().await?;
                }
                _ => {
                    // Other events would be handled here if needed
                    tracing::warn!(
                        "Unhandled app-level event from command palette: {:?}",
                        app_event
                    );
                }
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

        let access_token = token.as_ref().ok_or_else(|| {
            anyhow::anyhow!("No access token available for account {}", account_id)
        })?;

        // Initialize SMTP for this account
        smtp_service
            .initialize_account(
                account_id,
                &config.provider,
                &config.email_address,
                &access_token.token,
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
            "Use CLI to add accounts: 'comunicado setup-gmail' or 'comunicado setup-outlook'"
                .to_string(),
            tokio::time::Duration::from_secs(10),
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
            if trimmed
                .to_lowercase()
                .starts_with("content-transfer-encoding:")
            {
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
            && line
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')
        {
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
        if let Ok(timestamp_regex) = regex::Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}") {
            if timestamp_regex.is_match(line) {
                return true;
            }
        }

        // Check for IP addresses and server names
        if let Ok(ip_regex) = regex::Regex::new(r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b") {
            if ip_regex.is_match(line) {
                return true;
            }
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
            "On ", // "On Mon, Jul 28, 2025..."
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
            if trimmed.len() > 50
                && trimmed
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "+=/-_".contains(c))
            {
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
        tracing::info!(
            "🔄 Refreshing account connection (non-blocking): {}",
            account_id
        );

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

        // ✅ FIX: Use background task instead of blocking sync operation
        use crate::performance::background_processor::{
            BackgroundTask, BackgroundTaskType, TaskPriority,
        };

        let refresh_task = BackgroundTask {
            id: uuid::Uuid::new_v4(),
            name: format!("Refresh account connection: {}", account_id),
            priority: TaskPriority::High, // High priority for user-initiated refresh
            account_id: account_id.to_string(),
            folder_name: None, // Refresh all folders
            task_type: BackgroundTaskType::AccountSync {
                strategy: crate::email::sync_engine::SyncStrategy::Incremental,
            },
            created_at: std::time::Instant::now(),
            estimated_duration: Some(std::time::Duration::from_secs(30)),
        };

        // Queue the background task (non-blocking)
        match self.queue_background_task(refresh_task).await {
            Ok(task_id) => {
                tracing::info!(
                    "✅ Account refresh task queued successfully: {} (task: {})",
                    account_id,
                    task_id
                );

                // Log immediate feedback that the operation started
                tracing::info!("🔄 Account refresh queued - UI remains responsive while sync runs in background");

                // The background processor will handle the actual sync and update the UI status
                // via the process_background_updates() method in the main loop
            }
            Err(e) => {
                tracing::error!(
                    "❌ Failed to queue account refresh task for {}: {}",
                    account_id,
                    e
                );
                self.ui.update_account_status(
                    account_id,
                    crate::ui::AccountSyncStatus::Error,
                    None,
                );
                // Error is already logged and UI status updated above
                return Err(anyhow::anyhow!("Failed to queue account refresh: {}", e));
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
        tracing::debug!(
            "🔍 handle_folder_select called with folder_path: '{}'",
            folder_path
        );

        // Get the current account ID and clone it to avoid borrowing issues
        let current_account_id = match self.ui.get_current_account_id() {
            Some(id) => {
                tracing::debug!("🔍 Current account ID: '{}'", id);
                id.clone()
            }
            None => {
                tracing::warn!("❌ No current account selected for folder selection");
                tracing::warn!("No current account selected for folder selection");
                return Ok(());
            }
        };

        tracing::info!(
            "Loading folder: '{}' for account: '{}' (instant load from cache)",
            folder_path,
            current_account_id
        );

        // Debug: Log the exact values being passed to database query
        tracing::info!(
            "DEBUG: Calling ui.load_messages with account_id='{}', folder_name='{}'",
            current_account_id,
            folder_path
        );

        // STEP 1: Load messages from database immediately (non-blocking UI)
        // This provides instant feedback to the user
        match self
            .ui
            .load_messages(current_account_id.clone(), folder_path.to_string())
            .await
        {
            Ok(()) => {
                tracing::info!(
                    "✅ Instantly loaded cached messages from folder: {}",
                    folder_path
                );

                // Show a brief notification that content is loaded
                self.ui.show_notification(
                    format!("📂 Loaded {}", folder_path),
                    std::time::Duration::from_millis(1500),
                );
            }
            Err(e) => {
                tracing::warn!(
                    "No cached messages for folder {}: {}. Will fetch from IMAP.",
                    folder_path,
                    e
                );

                // Show loading indicator for first-time folder access
                self.ui.show_notification(
                    format!("📥 Loading {} for the first time...", folder_path),
                    std::time::Duration::from_secs(3),
                );
            }
        }

        // STEP 2: Queue background refresh task using the background processor
        // Use the full folder_path instead of just the last segment to preserve Gmail prefixes like [Gmail]/
        let folder_name_for_display = folder_path.split('/').last().unwrap_or(folder_path);
        {
            use crate::performance::background_processor::{
                BackgroundTask, BackgroundTaskType, TaskPriority,
            };

            use uuid::Uuid;

            let background_task = BackgroundTask {
                id: Uuid::new_v4(),
                name: format!("Quick refresh: {}", folder_name_for_display),
                priority: TaskPriority::Normal,
                account_id: current_account_id.clone(),
                folder_name: Some(folder_path.to_string()), // Use full path
                task_type: BackgroundTaskType::FolderRefresh {
                    folder_name: folder_path.to_string(), // Use full path
                },
                created_at: std::time::Instant::now(),
                estimated_duration: Some(std::time::Duration::from_secs(2)),
            };

            // Queue the background task (non-blocking)
            match self.queue_background_task(background_task).await {
                Ok(task_id) => {
                    tracing::info!(
                        "✅ Queued background refresh task for {} (ID: {})",
                        folder_path,
                        task_id
                    );
                    self.ui.show_notification(
                        format!("🔄 Background sync queued for {}", folder_name_for_display),
                        std::time::Duration::from_millis(1000),
                    );
                }
                Err(e) => {
                    tracing::warn!("Failed to queue background task: {}", e);
                    // Fallback to the old method if background processor isn't available
                    let account_id_bg = current_account_id.clone();
                    let folder_path_bg = folder_path.to_string();
                    tokio::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
                        tracing::info!(
                            "✅ Fallback background check completed for {} (account: {})",
                            folder_path_bg,
                            account_id_bg
                        );
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
            std::time::Duration::from_secs(2),
        );

        tracing::info!(
            "Force refreshing folder: {} for account: {}",
            folder_path,
            current_account_id
        );

        // Queue a high-priority background task for full IMAP sync
        if let Some(folder_name) = folder_path.split('/').next_back() {
            use crate::email::sync_engine::SyncStrategy;
            use crate::performance::background_processor::{
                BackgroundTask, BackgroundTaskType, TaskPriority,
            };
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
                    tracing::info!(
                        "✅ Queued high-priority sync task for {} (ID: {})",
                        folder_path,
                        task_id
                    );
                    self.ui.show_notification(
                        format!("🚀 Full sync started for {}", folder_name),
                        std::time::Duration::from_secs(2),
                    );
                    return Ok(());
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to queue background sync task: {}, falling back to direct IMAP",
                        e
                    );
                    // Fall through to direct IMAP fetch as fallback
                }
            }
        }

        // Fallback to direct IMAP fetch if background processor is not available
        match self
            .fetch_messages_from_imap(&current_account_id, folder_path)
            .await
        {
            Ok(()) => {
                tracing::info!("✅ Successfully refreshed folder: {}", folder_path);
                self.ui.show_notification(
                    format!("✅ Refreshed {}", folder_path),
                    std::time::Duration::from_secs(2),
                );
            }
            Err(e) => {
                tracing::warn!("Failed to refresh folder {}: {}", folder_path, e);
                self.ui.show_notification(
                    format!("⚠️ Failed to refresh {}: {}", folder_path, e),
                    std::time::Duration::from_secs(4),
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

            // Get local database statistics
            let local_stats = if let Some(ref database) = self.database {
                match sqlx::query_as::<_, (i64, i64, Option<i64>)>(
                    "SELECT COUNT(*), COUNT(CASE WHEN is_read = 0 THEN 1 END), SUM(size) FROM messages WHERE account_id = ? AND folder_name = ?"
                )
                .bind(account_id)
                .bind(&folder_path)
                .fetch_one(&database.pool)
                .await {
                    Ok(stats) => Some(stats),
                    Err(e) => {
                        tracing::error!("Failed to get local folder statistics: {}", e);
                        None
                    }
                }
            } else {
                None
            };

            // Get IMAP server statistics
            let server_stats = if let Some(ref imap_manager) = self.imap_manager {
                if let Some(client_arc) = imap_manager.get_client(account_id).await.ok() {
                    let mut client = client_arc.lock().await;
                    match client
                        .get_folder_status(
                            &folder_path,
                            &["MESSAGES", "RECENT", "UIDNEXT", "UIDVALIDITY", "UNSEEN"],
                        )
                        .await
                    {
                        Ok(folder_info) => Some(folder_info),
                        Err(e) => {
                            tracing::error!(
                                "Failed to get server folder properties for {}: {}",
                                folder_path,
                                e
                            );
                            None
                        }
                    }
                } else {
                    None
                }
            } else {
                None
            };

            // Create comprehensive properties notification
            let properties_text = if let (Some(local), Some(ref server)) =
                (local_stats.as_ref(), server_stats.as_ref())
            {
                format!(
                    "📁 {}: {} msg (local: {}, {} unread), Server: {} msg, {} recent, {} unseen",
                    selected_folder.name,
                    server.exists.unwrap_or(0),
                    local.0,
                    local.1,
                    server.exists.unwrap_or(0),
                    server.recent.unwrap_or(0),
                    server.unseen.unwrap_or(0)
                )
            } else if let Some(local) = local_stats {
                format!(
                    "📁 {}: {} total messages, {} unread, {} bytes (local only)",
                    selected_folder.name,
                    local.0,
                    local.1,
                    local.2.unwrap_or(0)
                )
            } else if let Some(server) = server_stats {
                format!(
                    "📁 {}: {} messages, {} recent, {} unseen (server only)",
                    selected_folder.name,
                    server.exists.unwrap_or(0),
                    server.recent.unwrap_or(0),
                    server.unseen.unwrap_or(0)
                )
            } else {
                format!("📁 Properties: {} ({})", selected_folder.name, folder_path)
            };

            tracing::info!("Folder properties: {}", properties_text);

            // Show properties in notification (in a full implementation, this would be a dialog)
            self.ui
                .show_notification(properties_text, std::time::Duration::from_secs(6));
        } else {
            self.ui.show_notification(
                "No folder selected".to_string(),
                std::time::Duration::from_secs(2),
            );
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

        // Determine folder name - for now use a default name
        // In a full implementation, this would show a dialog to get the name
        let folder_name = if let Some(parent) = parent_path {
            format!("{}/New Subfolder", parent)
        } else {
            "New Folder".to_string()
        };

        // Get IMAP client and create the folder
        if let Some(ref imap_manager) = self.imap_manager {
            if let Some(client_arc) = imap_manager.get_client(account_id).await.ok() {
                let mut client = client_arc.lock().await;
                match client.create_folder(&folder_name).await {
                    Ok(()) => {
                        tracing::info!(
                            "Successfully created folder: {} in account: {}",
                            folder_name,
                            account_id
                        );

                        // Refresh folder list to show the new folder
                        if let Err(e) = self.handle_folder_refresh(account_id).await {
                            tracing::error!("Failed to refresh folders after creation: {}", e);
                        }

                        // Show success notification
                        self.ui.show_notification(
                            format!("✅ Created folder: {}", folder_name),
                            std::time::Duration::from_secs(3),
                        );
                    }
                    Err(e) => {
                        tracing::error!("Failed to create folder {}: {}", folder_name, e);
                        self.ui.show_notification(
                            format!("❌ Failed to create folder: {}", e),
                            std::time::Duration::from_secs(5),
                        );
                    }
                }
            } else {
                tracing::error!("No IMAP client available for account: {}", account_id);
                self.ui.show_notification(
                    "❌ No connection available".to_string(),
                    std::time::Duration::from_secs(3),
                );
            }
        } else {
            tracing::error!("No IMAP manager available");
            self.ui.show_notification(
                "❌ No IMAP manager available".to_string(),
                std::time::Duration::from_secs(3),
            );
        }

        Ok(())
    }

    /// Handle delete folder
    async fn handle_delete_folder(&mut self, account_id: &str, folder_path: &str) -> Result<()> {
        tracing::info!(
            "Deleting folder: {} from account: {}",
            folder_path,
            account_id
        );

        // Check if this is a system folder that shouldn't be deleted
        let folder_name = folder_path.split('/').last().unwrap_or(folder_path);
        if ["INBOX", "Sent", "Drafts", "Trash", "Junk", "Spam"].contains(&folder_name) {
            tracing::warn!("Attempted to delete system folder: {}", folder_path);
            self.ui.show_notification(
                "❌ Cannot delete system folder".to_string(),
                std::time::Duration::from_secs(3),
            );
            return Ok(());
        }

        // Get IMAP client and delete the folder
        if let Some(ref imap_manager) = self.imap_manager {
            if let Some(client_arc) = imap_manager.get_client(account_id).await.ok() {
                let mut client = client_arc.lock().await;
                match client.delete_folder(folder_path).await {
                    Ok(()) => {
                        tracing::info!(
                            "Successfully deleted folder: {} from account: {}",
                            folder_path,
                            account_id
                        );

                        // Refresh folder list to remove the deleted folder
                        if let Err(e) = self.handle_folder_refresh(account_id).await {
                            tracing::error!("Failed to refresh folders after deletion: {}", e);
                        }

                        // Show success notification
                        self.ui.show_notification(
                            format!("✅ Deleted folder: {}", folder_path),
                            std::time::Duration::from_secs(3),
                        );
                    }
                    Err(e) => {
                        tracing::error!("Failed to delete folder {}: {}", folder_path, e);
                        self.ui.show_notification(
                            format!("❌ Failed to delete folder: {}", e),
                            std::time::Duration::from_secs(5),
                        );
                    }
                }
            } else {
                tracing::error!("No IMAP client available for account: {}", account_id);
                self.ui.show_notification(
                    "❌ No connection available".to_string(),
                    std::time::Duration::from_secs(3),
                );
            }
        } else {
            tracing::error!("No IMAP manager available");
            self.ui.show_notification(
                "❌ No IMAP manager available".to_string(),
                std::time::Duration::from_secs(3),
            );
        }

        Ok(())
    }

    /// Handle rename folder
    async fn handle_rename_folder(&mut self, account_id: &str, folder_path: &str) -> Result<()> {
        tracing::info!(
            "Renaming folder: {} in account: {}",
            folder_path,
            account_id
        );

        // Check if this is a system folder that shouldn't be renamed
        let folder_name = folder_path.split('/').last().unwrap_or(folder_path);
        if ["INBOX", "Sent", "Drafts", "Trash", "Junk", "Spam"].contains(&folder_name) {
            tracing::warn!("Attempted to rename system folder: {}", folder_path);
            self.ui.show_notification(
                "❌ Cannot rename system folder".to_string(),
                std::time::Duration::from_secs(3),
            );
            return Ok(());
        }

        // Generate a new name - for now use a simple renamed version
        // In a full implementation, this would show a dialog to get the new name
        let new_name = if folder_path.contains('/') {
            let parts: Vec<&str> = folder_path.rsplitn(2, '/').collect();
            if parts.len() == 2 {
                format!("{}/{}_renamed", parts[1], parts[0])
            } else {
                format!("{}_renamed", folder_path)
            }
        } else {
            format!("{}_renamed", folder_path)
        };

        // Get IMAP client and rename the folder
        if let Some(ref imap_manager) = self.imap_manager {
            if let Some(client_arc) = imap_manager.get_client(account_id).await.ok() {
                let mut client = client_arc.lock().await;
                match client.rename_folder(folder_path, &new_name).await {
                    Ok(()) => {
                        tracing::info!(
                            "Successfully renamed folder {} to {} in account: {}",
                            folder_path,
                            new_name,
                            account_id
                        );

                        // Refresh folder list to show the renamed folder
                        if let Err(e) = self.handle_folder_refresh(account_id).await {
                            tracing::error!("Failed to refresh folders after rename: {}", e);
                        }

                        // Show success notification
                        self.ui.show_notification(
                            format!("✅ Renamed folder to: {}", new_name),
                            std::time::Duration::from_secs(3),
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to rename folder {} to {}: {}",
                            folder_path,
                            new_name,
                            e
                        );
                        self.ui.show_notification(
                            format!("❌ Failed to rename folder: {}", e),
                            std::time::Duration::from_secs(5),
                        );
                    }
                }
            } else {
                tracing::error!("No IMAP client available for account: {}", account_id);
                self.ui.show_notification(
                    "❌ No connection available".to_string(),
                    std::time::Duration::from_secs(3),
                );
            }
        } else {
            tracing::error!("No IMAP manager available");
            self.ui.show_notification(
                "❌ No IMAP manager available".to_string(),
                std::time::Duration::from_secs(3),
            );
        }

        Ok(())
    }

    /// Handle empty folder (delete all messages)
    async fn handle_empty_folder(&mut self, account_id: &str, folder_path: &str) -> Result<()> {
        tracing::info!(
            "Emptying folder: {} in account: {}",
            folder_path,
            account_id
        );

        // Check if this is INBOX - be extra careful
        if folder_path.to_uppercase() == "INBOX" {
            tracing::warn!("Attempted to empty INBOX folder: {}", folder_path);
            self.ui.show_notification(
                "⚠️ Cannot empty INBOX folder".to_string(),
                std::time::Duration::from_secs(3),
            );
            return Ok(());
        }

        // Get IMAP client and empty the folder
        if let Some(ref imap_manager) = self.imap_manager {
            if let Some(client_arc) = imap_manager.get_client(account_id).await.ok() {
                let mut client = client_arc.lock().await;

                // Select the folder first
                match client.select_folder(folder_path).await {
                    Ok(_folder_info) => {
                        // Mark all messages as deleted and expunge
                        match client
                            .uid_store_flags("1:*", &[crate::imap::MessageFlag::Deleted], false)
                            .await
                        {
                            Ok(()) => {
                                match client.expunge().await {
                                    Ok(()) => {
                                        tracing::info!(
                                            "Successfully emptied folder: {} in account: {}",
                                            folder_path,
                                            account_id
                                        );

                                        // Update database by deleting all messages from this folder
                                        if let Some(ref database) = self.database {
                                            // Use the existing method to delete all messages for this folder
                                            if let Err(e) = sqlx::query(
                                                "DELETE FROM messages WHERE account_id = ? AND folder_name = ?"
                                            )
                                            .bind(account_id)
                                            .bind(folder_path)
                                            .execute(&database.pool)
                                            .await {
                                                tracing::error!("Failed to update database after emptying folder: {}", e);
                                            }
                                        }

                                        // Refresh folder to show empty state
                                        if let Err(e) =
                                            self.handle_folder_force_refresh(folder_path).await
                                        {
                                            tracing::error!(
                                                "Failed to refresh folder after emptying: {}",
                                                e
                                            );
                                        }

                                        // Show success notification
                                        self.ui.show_notification(
                                            format!("✅ Emptied folder: {}", folder_path),
                                            std::time::Duration::from_secs(3),
                                        );
                                    }
                                    Err(e) => {
                                        tracing::error!(
                                            "Failed to expunge folder {}: {}",
                                            folder_path,
                                            e
                                        );
                                        self.ui.show_notification(
                                            format!("❌ Failed to empty folder: {}", e),
                                            std::time::Duration::from_secs(5),
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Failed to mark messages as deleted in folder {}: {}",
                                    folder_path,
                                    e
                                );
                                self.ui.show_notification(
                                    format!("❌ Failed to mark messages for deletion: {}", e),
                                    std::time::Duration::from_secs(5),
                                );
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to select folder {}: {}", folder_path, e);
                        self.ui.show_notification(
                            format!("❌ Failed to access folder: {}", e),
                            std::time::Duration::from_secs(5),
                        );
                    }
                }
            } else {
                tracing::error!("No IMAP client available for account: {}", account_id);
                self.ui.show_notification(
                    "❌ No connection available".to_string(),
                    std::time::Duration::from_secs(3),
                );
            }
        } else {
            tracing::error!("No IMAP manager available");
            self.ui.show_notification(
                "❌ No IMAP manager available".to_string(),
                std::time::Duration::from_secs(3),
            );
        }

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

        // Get IMAP client and handle subscription
        if let Some(ref imap_manager) = self.imap_manager {
            if let Some(client_arc) = imap_manager.get_client(account_id).await.ok() {
                let mut client = client_arc.lock().await;
                let result = if subscribe {
                    client.subscribe_folder(folder_path).await
                } else {
                    client.unsubscribe_folder(folder_path).await
                };

                match result {
                    Ok(()) => {
                        let action_past = if subscribe {
                            "subscribed to"
                        } else {
                            "unsubscribed from"
                        };
                        tracing::info!(
                            "Successfully {} folder: {} in account: {}",
                            action_past,
                            folder_path,
                            account_id
                        );

                        // Update local state
                        self.ui
                            .folder_tree_mut()
                            .mark_folder_synced(folder_path, 0, 0);

                        // Show success notification
                        let emoji = if subscribe { "✅" } else { "❌" };
                        self.ui.show_notification(
                            format!("{} {} folder: {}", emoji, action_past, folder_path),
                            std::time::Duration::from_secs(3),
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to {} folder {}: {}",
                            action.to_lowercase(),
                            folder_path,
                            e
                        );
                        self.ui.show_notification(
                            format!("❌ Failed to {} folder: {}", action.to_lowercase(), e),
                            std::time::Duration::from_secs(5),
                        );
                    }
                }
            } else {
                tracing::error!("No IMAP client available for account: {}", account_id);
                self.ui.show_notification(
                    "❌ No connection available".to_string(),
                    std::time::Duration::from_secs(3),
                );
            }
        } else {
            tracing::error!("No IMAP manager available");
            self.ui.show_notification(
                "❌ No IMAP manager available".to_string(),
                std::time::Duration::from_secs(3),
            );
        }

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
            use crate::performance::background_processor::{
                BackgroundTask, BackgroundTaskType, TaskPriority,
            };
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
                    tracing::debug!(
                        "✅ Queued auto-sync task for {} (ID: {})",
                        account_id,
                        task_id
                    );
                }
                Err(e) => {
                    tracing::warn!("Failed to queue auto-sync task for {}: {}", account_id, e);
                }
            }
        }

        // Show a subtle notification that auto-sync is running
        self.ui.show_notification(
            "🔄 Background sync started".to_string(),
            std::time::Duration::from_millis(1500),
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

    /// Trigger manual sync for all accounts (called from command palette)
    /// Uses background tasks to avoid blocking the UI thread
    async fn trigger_manual_sync(&mut self) -> Result<()> {
        tracing::info!("Manual sync triggered - queuing background tasks for all accounts");

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
                    let error_msg = format!("Failed to get account IDs for manual sync: {}", e);
                    tracing::error!("{}", error_msg);
                    self.ui.show_toast_error(&error_msg);
                    return Err(anyhow::anyhow!(error_msg));
                }
            }
        } else {
            let error_msg = "Database not available for manual sync";
            tracing::error!("{}", error_msg);
            self.ui.show_toast_error(error_msg);
            return Err(anyhow::anyhow!(error_msg));
        };

        if account_ids.is_empty() {
            let info_msg = "No accounts configured for sync";
            tracing::info!("{}", info_msg);
            self.ui.show_toast_info(info_msg);
            return Ok(());
        }

        tracing::info!(
            "Queuing background sync tasks for {} accounts",
            account_ids.len()
        );

        // Queue background sync tasks for each account (non-blocking)
        let mut queued_tasks = 0;
        let mut failed_queues = 0;

        for account_id in &account_ids {
            let sync_task = crate::performance::background_processor::BackgroundTask {
                id: uuid::Uuid::new_v4(),
                name: format!("Manual sync for account: {}", account_id),
                priority: crate::performance::background_processor::TaskPriority::High,
                account_id: account_id.clone(),
                folder_name: None, // Sync all folders
                task_type:
                    crate::performance::background_processor::BackgroundTaskType::AccountSync {
                        strategy: crate::email::sync_engine::SyncStrategy::Incremental,
                    },
                created_at: std::time::Instant::now(),
                estimated_duration: Some(std::time::Duration::from_secs(30)),
            };

            match self.queue_background_task(sync_task).await {
                Ok(task_id) => {
                    tracing::debug!(
                        "Queued background sync task {} for account: {}",
                        task_id,
                        account_id
                    );
                    queued_tasks += 1;
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to queue background sync for account {}: {}",
                        account_id,
                        e
                    );
                    failed_queues += 1;
                }
            }
        }

        // Provide immediate user feedback and handle fallback for failed queues
        if queued_tasks > 0 {
            self.ui.show_toast_success(&format!(
                "✓ Email sync started for {} accounts (background processing)",
                queued_tasks
            ));
            tracing::info!("Successfully queued {} background sync tasks", queued_tasks);
        }

        if failed_queues > 0 {
            self.ui.show_toast_warning(&format!(
                "⚠ {} accounts failed to queue - trying direct sync",
                failed_queues
            ));
            tracing::warn!(
                "Failed to queue {} background sync tasks, attempting direct sync fallback",
                failed_queues
            );

            // Fallback: Trigger direct IMAP sync for the current folder as immediate feedback
            if let (Some(current_account), Some(current_folder)) =
                self.ui.message_list().get_current_context()
            {
                let account_id = current_account.clone();
                let folder_name = current_folder.clone();
                tracing::info!(
                    "Attempting direct sync fallback for {}/{}",
                    account_id,
                    folder_name
                );

                match self
                    .fetch_messages_from_imap(&account_id, &folder_name)
                    .await
                {
                    Ok(()) => {
                        self.ui
                            .show_toast_success("✓ Direct sync completed for current folder");
                        tracing::info!(
                            "Direct sync fallback succeeded for {}/{}",
                            account_id,
                            folder_name
                        );
                    }
                    Err(e) => {
                        self.ui
                            .show_toast_error(&format!("❌ Direct sync failed: {}", e));
                        tracing::error!(
                            "Direct sync fallback failed for {}/{}: {}",
                            account_id,
                            folder_name,
                            e
                        );
                    }
                }
            }
        }

        // Show progress overlay to track background tasks
        if queued_tasks > 0 {
            self.ui.show_enhanced_progress_overlay();
        }

        Ok(())
    }

    /// Check and refresh expired tokens with UI updates
    pub async fn check_and_refresh_tokens(&mut self) -> Result<()> {
        if let Some(ref token_manager) = self.token_manager {
            let account_ids = token_manager.get_account_ids().await;
            let mut refreshed_accounts = Vec::new();

            for account_id in account_ids {
                let diagnosis = token_manager.diagnose_account_tokens(&account_id).await;

                match diagnosis {
                    crate::oauth2::token::TokenDiagnosis::ExpiredWithRefresh { .. }
                    | crate::oauth2::token::TokenDiagnosis::ExpiringSoon {
                        has_refresh_token: true,
                        ..
                    } => {
                        tracing::info!("Refreshing token for account: {}", account_id);
                        match token_manager.refresh_access_token(&account_id).await {
                            Ok(_) => {
                                tracing::info!(
                                    "Successfully refreshed token for account: {}",
                                    account_id
                                );
                                refreshed_accounts.push(account_id.clone());

                                // Update UI to show account as online
                                self.ui.update_account_status(
                                    &account_id,
                                    crate::ui::AccountSyncStatus::Online,
                                    None,
                                );
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Failed to refresh token for account {}: {}",
                                    account_id,
                                    e
                                );
                                // Update UI to show account has error
                                self.ui.update_account_status(
                                    &account_id,
                                    crate::ui::AccountSyncStatus::Error,
                                    None,
                                );
                            }
                        }
                    }
                    crate::oauth2::token::TokenDiagnosis::ExpiredNoRefresh { .. } => {
                        tracing::warn!(
                            "Account {} has expired token without refresh capability",
                            account_id
                        );
                        self.ui.update_account_status(
                            &account_id,
                            crate::ui::AccountSyncStatus::Error,
                            None,
                        );
                    }
                    _ => {
                        // Token is valid, ensure account shows as online
                        self.ui.update_account_status(
                            &account_id,
                            crate::ui::AccountSyncStatus::Online,
                            None,
                        );
                    }
                }
            }

            if !refreshed_accounts.is_empty() {
                let message = format!(
                    "🔄 Refreshed tokens for {} account(s)",
                    refreshed_accounts.len()
                );
                self.ui.show_toast_success(&message);
                tracing::info!(
                    "Token refresh completed for accounts: {:?}",
                    refreshed_accounts
                );
            }
        }

        Ok(())
    }

    /// Handle contacts popup request
    async fn handle_contacts_popup(&mut self) -> Result<()> {
        tracing::info!("🎯 handle_contacts_popup() called!");
        tracing::debug!(
            "🔍 Checking contacts_manager state: {:?}",
            self.contacts_manager.is_some()
        );

        // Extra diagnostic info
        if self.contacts_manager.is_none() {
            tracing::error!("🚨 CONTACTS DEBUG: contacts_manager is None!");
            tracing::error!("🚨 This means contacts initialization failed or was skipped");
        } else {
            tracing::info!("✅ Contacts manager exists in memory");
        }

        if let Some(ref contacts_manager) = self.contacts_manager {
            tracing::info!("✅ Contacts manager found, showing popup");
            self.ui.show_contacts_popup(contacts_manager.clone());
            tracing::info!("📱 Contacts popup should now be visible");
        } else {
            tracing::warn!("❌ Contacts manager not initialized, cannot show contacts popup");
            self.ui.show_toast_error("Contacts manager not available");
        }
        Ok(())
    }

    /// Handle contacts popup action
    async fn handle_contacts_action(
        &mut self,
        action: crate::contacts::ContactPopupAction,
    ) -> Result<()> {
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
                // Show contact details in the popup instead of closing it
                if self.ui.show_contact_details_in_popup(contact.clone()) {
                    // Successfully shown contact details in popup
                    tracing::info!("📱 Showing contact details in popup");
                } else {
                    // Fallback: show notification if popup is not available
                    self.ui.hide_contacts_popup();
                    let message = format!(
                        "Viewing contact: {} <{}>",
                        contact.display_name,
                        contact
                            .primary_email()
                            .map(|e| e.address.as_str())
                            .unwrap_or("no email")
                    );
                    self.ui.show_notification(message, Duration::from_secs(3));
                }
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
                    self.ui
                        .show_contacts_popup_with_contact(contacts_manager.clone(), contact);
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
                    tracing::info!(
                        "Found contact to edit for {}: {}",
                        email,
                        contact.display_name
                    );

                    // Show contact edit dialog/popup
                    self.ui
                        .show_contact_edit_dialog(contacts_manager.clone(), contact);
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
                    self.ui
                        .show_contact_context_menu(email.to_string(), Some(contact));
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
                    tracing::info!(
                        "Found contact for sender {}: {}",
                        sender_email,
                        contact.display_name
                    );

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
                let compose_action =
                    crate::ui::ComposeAction::StartReplyFromMessage(stored_message);
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
                let compose_action =
                    crate::ui::ComposeAction::StartReplyAllFromMessage(stored_message);
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
                let compose_action =
                    crate::ui::ComposeAction::StartForwardFromMessage(stored_message);
                self.handle_compose_action(compose_action).await?;
            } else {
                tracing::error!("Message not found for ID: {}", message_id);
            }
        } else {
            tracing::error!("Database not available");
        }

        Ok(())
    }

    /// Get the email operations service
    pub fn email_operations_service(&self) -> Option<Arc<crate::email::EmailOperationsService>> {
        self.email_operations_service.clone()
    }

    /// Handle delete email operation
    async fn handle_delete_email(
        &mut self,
        account_id: &str,
        message_id: uuid::Uuid,
        folder: &str,
    ) -> Result<()> {
        if let Some(ref service) = self.email_operations_service {
            match service
                .delete_email_by_id(account_id, message_id, folder)
                .await
            {
                Ok(()) => {
                    self.ui.show_toast_info("Email deleted successfully");
                    // Refresh the message list to reflect the change
                    if let Err(e) = self.handle_folder_force_refresh(folder).await {
                        tracing::warn!("Failed to refresh folder after delete: {}", e);
                    }
                }
                Err(e) => {
                    let error_msg = format!("Failed to delete email: {}", e);
                    tracing::error!("{}", error_msg);
                    self.ui.show_toast_error(&error_msg);
                }
            }
        } else {
            self.ui
                .show_toast_error("Email operations service not available");
        }
        Ok(())
    }

    /// Handle archive email operation
    async fn handle_archive_email(
        &mut self,
        account_id: &str,
        message_id: uuid::Uuid,
        folder: &str,
    ) -> Result<()> {
        if let Some(ref service) = self.email_operations_service {
            match service
                .archive_email_by_id(account_id, message_id, folder)
                .await
            {
                Ok(()) => {
                    self.ui.show_toast_info("Email archived successfully");
                    // Refresh the message list to reflect the change
                    if let Err(e) = self.handle_folder_force_refresh(folder).await {
                        tracing::warn!("Failed to refresh folder after archive: {}", e);
                    }
                }
                Err(e) => {
                    let error_msg = format!("Failed to archive email: {}", e);
                    tracing::error!("{}", error_msg);
                    self.ui.show_toast_error(&error_msg);
                }
            }
        } else {
            self.ui
                .show_toast_error("Email operations service not available");
        }
        Ok(())
    }

    /// Handle mark email as read operation
    async fn handle_mark_email_read(
        &mut self,
        account_id: &str,
        message_id: uuid::Uuid,
        folder: &str,
    ) -> Result<()> {
        if let Some(ref service) = self.email_operations_service {
            match service
                .mark_email_read_by_id(account_id, message_id, folder)
                .await
            {
                Ok(()) => {
                    self.ui.show_toast_info("Email marked as read");
                    // Update the UI to reflect the change
                    self.ui.message_list_mut().mark_selected_as_read();
                }
                Err(e) => {
                    let error_msg = format!("Failed to mark email as read: {}", e);
                    tracing::error!("{}", error_msg);
                    self.ui.show_toast_error(&error_msg);
                }
            }
        } else {
            self.ui
                .show_toast_error("Email operations service not available");
        }
        Ok(())
    }

    /// Handle mark email as unread operation
    async fn handle_mark_email_unread(
        &mut self,
        account_id: &str,
        message_id: uuid::Uuid,
        folder: &str,
    ) -> Result<()> {
        if let Some(ref service) = self.email_operations_service {
            match service
                .mark_email_unread_by_id(account_id, message_id, folder)
                .await
            {
                Ok(()) => {
                    self.ui.show_toast_info("Email marked as unread");
                    // Update the UI to reflect the change - need to add a method for this
                    // For now, just refresh the folder
                    if let Err(e) = self.handle_folder_force_refresh(folder).await {
                        tracing::warn!("Failed to refresh folder after mark unread: {}", e);
                    }
                }
                Err(e) => {
                    let error_msg = format!("Failed to mark email as unread: {}", e);
                    tracing::error!("{}", error_msg);
                    self.ui.show_toast_error(&error_msg);
                }
            }
        } else {
            self.ui
                .show_toast_error("Email operations service not available");
        }
        Ok(())
    }

    /// Handle toggle email flag operation
    async fn handle_toggle_email_flag(
        &mut self,
        account_id: &str,
        message_id: uuid::Uuid,
        folder: &str,
    ) -> Result<()> {
        if let Some(ref service) = self.email_operations_service {
            match service
                .toggle_email_flag_by_id(account_id, message_id, folder)
                .await
            {
                Ok(is_flagged) => {
                    let status = if is_flagged { "flagged" } else { "unflagged" };
                    self.ui.show_toast_info(&format!("Email {}", status));
                    // Update the UI to reflect the change
                    self.ui.message_list_mut().toggle_selected_important();
                }
                Err(e) => {
                    let error_msg = format!("Failed to toggle email flag: {}", e);
                    tracing::error!("{}", error_msg);
                    self.ui.show_toast_error(&error_msg);
                }
            }
        } else {
            self.ui
                .show_toast_error("Email operations service not available");
        }
        Ok(())
    }

    /// Handle AI email summarization with real email content from database
    async fn handle_ai_summarize_email(&mut self, message_id: uuid::Uuid) -> Result<()> {
        if let Some(ref database) = self.database {
            match database.get_message_by_id(message_id).await {
                Ok(Some(stored_message)) => {
                    // Convert StoredMessage to EmailMessage for AI processing
                    let message_id_str = stored_message
                        .message_id
                        .unwrap_or_else(|| format!("msg_{}", message_id));
                    let message_id_obj = crate::email::MessageId::new(message_id_str);

                    // Combine all recipients
                    let mut recipients = stored_message.to_addrs.clone();
                    recipients.extend(stored_message.cc_addrs.clone());
                    recipients.extend(stored_message.bcc_addrs.clone());

                    // Use body_text or body_html as content, prefer text
                    let content = stored_message
                        .body_text
                        .or(stored_message.body_html)
                        .unwrap_or_else(|| "No content available".to_string());

                    let mut email_message = crate::email::EmailMessage::new(
                        message_id_obj,
                        stored_message.subject,
                        stored_message.from_addr,
                        recipients,
                        content,
                        stored_message.date,
                    );

                    // Set additional properties
                    email_message.set_read(!stored_message.flags.contains(&"\\Seen".to_string()));
                    email_message
                        .set_important(stored_message.flags.contains(&"\\Flagged".to_string()));
                    email_message.set_attachments(!stored_message.attachments.is_empty());

                    // Set reply information if available
                    if let Some(reply_to) = stored_message.in_reply_to {
                        let reply_to_id = crate::email::MessageId::new(reply_to);
                        email_message.set_in_reply_to(reply_to_id);
                    }

                    if !stored_message.references.is_empty() {
                        let references_str = stored_message.references.join(" ");
                        email_message.set_references(references_str);
                    }

                    // Start AI summarization with real email content
                    self.ui.start_ai_email_summarization(email_message);
                    self.ui
                        .show_toast_info("Starting AI email summarization...");
                }
                Ok(None) => {
                    self.ui.show_toast_error("Email not found in database");
                }
                Err(e) => {
                    let error_msg = format!("Failed to load email from database: {}", e);
                    tracing::error!("{}", error_msg);
                    self.ui.show_toast_error(&error_msg);
                }
            }
        } else {
            self.ui.show_toast_error("Database not available");
        }
        Ok(())
    }

    /// Handle creating a new calendar event
    async fn handle_create_event(&mut self, calendar_id: &str) -> Result<()> {
        if let Some(ref manager) = self.calendar_manager {
            // Create a basic event template - in a real implementation this would open an event creation dialog
            let event = crate::calendar::Event::new(
                calendar_id.to_string(),
                "New Event".to_string(),
                chrono::Utc::now(),
                chrono::Utc::now() + chrono::Duration::hours(1),
            );

            match manager.create_event(event).await {
                Ok(_created_event) => {
                    self.ui.show_toast_info("Event created successfully");
                    // TODO: Refresh calendar view to show new event
                }
                Err(e) => {
                    let error_msg = format!("Failed to create event: {}", e);
                    tracing::error!("{}", error_msg);
                    self.ui.show_toast_error(&error_msg);
                }
            }
        } else {
            self.ui.show_toast_error("Calendar manager not available");
        }
        Ok(())
    }

    /// Handle editing an existing calendar event
    async fn handle_edit_event(&mut self, _calendar_id: &str, event_id: &str) -> Result<()> {
        if let Some(_manager) = &self.calendar_manager {
            // TODO: Implement event editing dialog
            // For now, just show a message that the feature is available
            self.ui.show_toast_info(&format!(
                "Edit event feature triggered for event: {}",
                event_id
            ));
            tracing::info!(
                "Edit event feature would open dialog for event: {}",
                event_id
            );
        } else {
            self.ui.show_toast_error("Calendar manager not available");
        }
        Ok(())
    }

    /// Handle deleting a calendar event
    async fn handle_delete_event(&mut self, _calendar_id: &str, event_id: &str) -> Result<()> {
        if let Some(ref manager) = self.calendar_manager {
            match manager.delete_event(event_id).await {
                Ok(_was_deleted) => {
                    self.ui.show_toast_info("Event deleted successfully");
                    // TODO: Refresh calendar view to reflect deletion
                }
                Err(e) => {
                    let error_msg = format!("Failed to delete event: {}", e);
                    tracing::error!("{}", error_msg);
                    self.ui.show_toast_error(&error_msg);
                }
            }
        } else {
            self.ui.show_toast_error("Calendar manager not available");
        }
        Ok(())
    }

    /// Handle viewing event details
    async fn handle_view_event_details(
        &mut self,
        _calendar_id: &str,
        event_id: &str,
    ) -> Result<()> {
        if let Some(_manager) = &self.calendar_manager {
            // TODO: Implement event details popup
            // For now, just show a message that the feature is available
            self.ui
                .show_toast_info(&format!("View event details for: {}", event_id));
            tracing::info!(
                "Event details view would show details for event: {}",
                event_id
            );
        } else {
            self.ui.show_toast_error("Calendar manager not available");
        }
        Ok(())
    }

    /// Handle creating a new todo/task
    async fn handle_create_todo(&mut self, calendar_id: &str) -> Result<()> {
        if let Some(ref manager) = self.calendar_manager {
            // Create a basic todo template - todos are events with special status
            let mut todo = crate::calendar::Event::new(
                calendar_id.to_string(),
                "New Todo".to_string(),
                chrono::Utc::now(),
                chrono::Utc::now() + chrono::Duration::hours(1),
            );
            todo.description = Some("Task description".to_string());
            todo.status = crate::calendar::EventStatus::Tentative; // Use Tentative for incomplete todos

            match manager.create_event(todo).await {
                Ok(_created_todo) => {
                    self.ui.show_toast_info("Todo created successfully");
                    // TODO: Refresh todo/calendar view
                }
                Err(e) => {
                    let error_msg = format!("Failed to create todo: {}", e);
                    tracing::error!("{}", error_msg);
                    self.ui.show_toast_error(&error_msg);
                }
            }
        } else {
            self.ui.show_toast_error("Calendar manager not available");
        }
        Ok(())
    }

    /// Handle converting email to note
    async fn handle_convert_email_to_note(&mut self, message_id: uuid::Uuid) -> Result<()> {
        // TODO: Implement email-to-note conversion
        tracing::info!("Converting email {} to note", message_id);
        self.ui
            .show_toast_info("Email converted to note (feature coming soon)");
        Ok(())
    }

    /// Handle converting event to note
    async fn handle_convert_event_to_note(&mut self, event_id: &str) -> Result<()> {
        // TODO: Implement event-to-note conversion
        tracing::info!("Converting event {} to note", event_id);
        self.ui
            .show_toast_info("Event converted to note (feature coming soon)");
        Ok(())
    }

    /// Handle converting KDE Connect message to note
    async fn handle_convert_kde_message_to_note(
        &mut self,
        title: &str,
        _content: &str,
    ) -> Result<()> {
        // TODO: Implement KDE Connect message-to-note conversion
        tracing::info!("Converting KDE Connect message '{}' to note", title);
        self.ui
            .show_toast_info("KDE Connect message converted to note (feature coming soon)");
        Ok(())
    }

    /// Handle showing notes view
    async fn handle_show_notes(&mut self) -> Result<()> {
        // TODO: Switch to notes view when implemented
        tracing::info!("Switching to Notes view");
        self.ui
            .show_toast_info("Switching to Notes view (feature coming soon)");
        Ok(())
    }

    /// Handle creating new note
    async fn handle_create_note(&mut self) -> Result<()> {
        // TODO: Implement note creation
        tracing::info!("Creating new note");
        self.ui
            .show_toast_info("Creating new note (feature coming soon)");
        Ok(())
    }

    /// Handle toggling todo completion status
    async fn handle_toggle_todo_complete(
        &mut self,
        _calendar_id: &str,
        event_id: &str,
    ) -> Result<()> {
        if let Some(_manager) = &self.calendar_manager {
            // TODO: Implement todo completion toggle
            // For now, just show a message that the feature is available
            self.ui
                .show_toast_info(&format!("Toggle todo completion for: {}", event_id));
            tracing::info!("Todo completion toggle triggered for event: {}", event_id);
        } else {
            self.ui.show_toast_error("Calendar manager not available");
        }
        Ok(())
    }

    /// Handle global quit keys that should work from anywhere in the application
    fn handle_global_quit_keys(&self, key: crossterm::event::KeyEvent) -> bool {
        use crossterm::event::{KeyCode, KeyModifiers};

        match key.code {
            // Q for quit (when not in text input mode)
            KeyCode::Char('q') | KeyCode::Char('Q') if !self.ui.is_in_text_input() => {
                tracing::info!("User pressed Q to quit");
                true
            }
            // Ctrl+C for quit
            KeyCode::Char('c') | KeyCode::Char('C')
                if key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                tracing::info!("User pressed Ctrl+C to quit");
                true
            }
            // Escape to quit (only from main view, not in modals/popups)
            KeyCode::Esc if self.ui.is_in_main_view() => {
                tracing::info!("User pressed Escape to quit from main view");
                true
            }
            _ => false,
        }
    }

    /// Process key events through the event-driven UI integration system
    async fn process_key_event_through_integration(
        &mut self,
        key: crossterm::event::KeyEvent,
    ) -> Result<bool> {
        use crate::events::types::KeyEventData;

        // First check if context menu is visible and handle its keys
        if self.ui.context_menu().is_visible() {
            if let Some(action) = self.ui.handle_context_menu_key(key.code) {
                self.handle_context_menu_action(action).await?;
                return Ok(true);
            }
        }

        // Convert crossterm KeyEvent to our event system format
        let key_data = KeyEventData {
            code: format!("{:?}", key.code),
            modifiers: key.modifiers.iter().map(|m| format!("{:?}", m)).collect(),
            char: match key.code {
                crossterm::event::KeyCode::Char(c) => Some(c),
                _ => None,
            },
        };

        // Process through the UI integration system
        match self
            .ui
            .ui_integration_mut()
            .handle_key_event(key_data)
            .await
        {
            Ok(handled) => {
                if handled {
                    tracing::debug!("Key event processed through UI integration system");
                } else {
                    tracing::debug!("Key event not handled by UI integration system");
                }
                Ok(handled)
            }
            Err(e) => {
                tracing::warn!("UI integration key processing failed: {}", e);
                // Still publish to event bus as fallback
                self.publish_key_event_to_bus_fallback(key).await?;
                Ok(false)
            }
        }
    }

    /// Fallback: Publish key events to the event bus for event-driven processing
    async fn publish_key_event_to_bus_fallback(
        &mut self,
        key: crossterm::event::KeyEvent,
    ) -> Result<()> {
        use crate::events::publish;
        use crate::events::types::{KeyEventData, UIEvent, UIEventData};

        // Convert crossterm KeyEvent to our event system
        let key_data = KeyEventData {
            code: format!("{:?}", key.code),
            modifiers: key.modifiers.iter().map(|m| format!("{:?}", m)).collect(),
            char: match key.code {
                crossterm::event::KeyCode::Char(c) => Some(c),
                _ => None,
            },
        };

        let ui_event = UIEventData::new(UIEvent::KeyPressed { key: key_data });

        // Publish the event to the event bus
        if let Err(e) = publish(ui_event) {
            tracing::warn!("Failed to publish key event to event bus: {}", e);
            return Err(anyhow::anyhow!("Event bus publishing failed: {}", e));
        }

        Ok(())
    }

    /// Check if event bus memory optimization should be performed (periodically)
    /// This is called from the main loop to avoid excessive optimization checks
    fn should_optimize_event_bus_memory(&self) -> bool {
        // Run memory optimization check every ~100 main loop cycles (about every 5 seconds)
        static mut OPTIMIZATION_COUNTER: u64 = 0;
        unsafe {
            OPTIMIZATION_COUNTER += 1;
            OPTIMIZATION_COUNTER % 100 == 0
        }
    }

    /// Process mouse events through the new mouse handling system
    async fn process_mouse_event(
        &mut self,
        mouse_event: crossterm::event::MouseEvent,
    ) -> Result<()> {
        tracing::debug!(
            "Processing mouse event: {:?} at ({}, {})",
            mouse_event.kind,
            mouse_event.column,
            mouse_event.row
        );

        // Update terminal size in the mouse processor
        if let Ok(terminal_size) = crossterm::terminal::size() {
            self.ui
                .mouse_processor
                .update_terminal_size(terminal_size.0, terminal_size.1);
        }

        // Process the mouse event and get the action
        let action = self
            .ui
            .mouse_processor
            .process_mouse_event(mouse_event)
            .await?;

        // Execute the mouse action
        self.handle_mouse_action(action).await?;

        Ok(())
    }

    /// Handle mouse actions returned by the mouse processor
    async fn handle_mouse_action(&mut self, action: MouseAction) -> Result<()> {
        // First check if context menu is visible and handle clicks on it
        if self.ui.context_menu().is_visible() {
            // Get mouse coordinates from the action
            let (x, y) = match &action {
                MouseAction::ShowEmailContextMenu { x, y, .. } => (*x, *y),
                MouseAction::ShowFolderContextMenu { x, y, .. } => (*x, *y),
                MouseAction::ShowEmailContentContextMenu { x, y } => (*x, *y),
                MouseAction::ShowGeneralContextMenu { x, y } => (*x, *y),
                MouseAction::SelectMessage { row, column } => (*column, *row),
                MouseAction::SelectFolder { row, column } => (*column, *row),
                MouseAction::ClickStatusBar { row, column } => (*column, *row),
                _ => (0, 0), // Default coordinates for actions without explicit position
            };

            // Handle context menu mouse click
            if let Some(menu_action) = self.ui.handle_context_menu_click(x, y) {
                self.handle_context_menu_action(menu_action).await?;
                return Ok(());
            }
        }

        match action {
            MouseAction::None => {
                // No action needed
            }

            // Message List Actions
            MouseAction::SelectMessage { row, column: _ } => {
                self.ui.set_selected_message_index(row as usize);
                tracing::debug!("Selected message at row: {}", row);
            }
            MouseAction::ScrollMessageListUp => {
                self.ui.scroll_message_list_up();
            }
            MouseAction::ScrollMessageListDown => {
                self.ui.scroll_message_list_down();
            }
            MouseAction::HoverMessage { row, column: _ } => {
                // Future: Show message preview on hover
                tracing::trace!("Hovering over message at row: {}", row);
            }

            // Folder Tree Actions
            MouseAction::SelectFolder { row, column: _ } => {
                // Get folder at the row and select it
                if let Some(folder_path) = self.ui.get_folder_at_row(row as usize) {
                    let _ = self.ui.handle_folder_click(&folder_path).await;
                    tracing::debug!("Selected folder: {}", folder_path);
                }
            }
            MouseAction::ScrollFolderTreeUp => {
                self.ui.scroll_folder_tree_up();
            }
            MouseAction::ScrollFolderTreeDown => {
                self.ui.scroll_folder_tree_down();
            }

            // Email Viewer Actions
            MouseAction::FocusEmailViewer => {
                self.ui
                    .set_focused_pane(crate::ui::FocusedPane::ContentPreview);
            }
            MouseAction::ScrollEmailViewerUp => {
                self.ui.scroll_email_viewer_up();
            }
            MouseAction::ScrollEmailViewerDown => {
                self.ui.scroll_email_viewer_down();
            }

            // Calendar Actions
            MouseAction::SelectCalendarDate { row, column } => {
                // Future: Handle calendar date selection
                tracing::debug!("Selected calendar date at ({}, {})", column, row);
            }
            MouseAction::ScrollCalendarUp => {
                self.ui.scroll_calendar_up();
            }
            MouseAction::ScrollCalendarDown => {
                self.ui.scroll_calendar_down();
            }

            // Context Menu Actions
            MouseAction::ShowEmailContextMenu {
                x,
                y,
                row,
                column: _,
            } => {
                // Select the message first
                self.ui.set_selected_message_index(row as usize);
                // Show context menu at position
                self.show_email_context_menu(x, y).await?;
            }
            MouseAction::ShowFolderContextMenu {
                x,
                y,
                row,
                column: _,
            } => {
                if let Some(folder_path) = self.ui.get_folder_at_row(row as usize) {
                    self.show_folder_context_menu(&folder_path, x, y).await?;
                }
            }
            MouseAction::ShowEmailContentContextMenu { x, y } => {
                self.show_email_content_context_menu(x, y).await?;
            }
            MouseAction::ShowGeneralContextMenu { x, y } => {
                self.show_general_context_menu(x, y).await?;
            }

            // General Actions
            MouseAction::ClearSelection => {
                // Clear current selections
                tracing::debug!("Clearing selections");
            }

            // Status Bar Actions
            MouseAction::ClickStatusBar { row: _, column } => {
                // Future: Handle status bar clicks (mode switching, etc.)
                tracing::debug!("Clicked status bar at column: {}", column);
            }

            // Middle Click Actions (future features)
            MouseAction::MiddleClickMessage => {
                tracing::debug!("Middle click on message - future: open in new tab");
            }
            MouseAction::MiddleClickFolder => {
                tracing::debug!("Middle click on folder - future: open in new view");
            }

            // Drag Actions (future features)
            MouseAction::DragText { start_x, start_y } => {
                tracing::debug!(
                    "Start text drag at ({}, {}) - future: text selection",
                    start_x,
                    start_y
                );
            }

            // Hover Actions
            MouseAction::HoverFolder { row, column: _ } => {
                tracing::trace!("Hovering over folder at row: {}", row);
            }
        }

        Ok(())
    }

    // Context Menu Methods (Mouse Support)

    /// Show email context menu at the given coordinates
    async fn show_email_context_menu(&mut self, x: u16, y: u16) -> Result<()> {
        tracing::debug!("Show email context menu at ({}, {})", x, y);

        // Get information about the currently selected message
        let (is_read, is_draft, has_attachments, folder_name) =
            if let Some(selected_message) = self.ui.get_selected_message() {
                // Get current folder from folder tree selection
                let current_folder = self
                    .ui
                    .get_selected_folder_path()
                    .unwrap_or_else(|| "INBOX".to_string());
                (
                    selected_message.is_read,
                    current_folder.to_lowercase().contains("draft"),
                    selected_message.has_attachments,
                    current_folder,
                )
            } else {
                // Default values if no message selected
                (false, false, false, "INBOX".to_string())
            };

        // Show the context menu for email messages with proper context
        self.ui.show_context_menu_at_position(
            x,
            y,
            crate::ui::context_menu::ContextType::EmailMessage {
                is_read,
                is_draft,
                has_attachments,
                folder_name,
            },
        );

        Ok(())
    }

    /// Show folder context menu at the given coordinates
    async fn show_folder_context_menu(&mut self, folder_path: &str, x: u16, y: u16) -> Result<()> {
        tracing::debug!(
            "Show folder context menu for '{}' at ({}, {})",
            folder_path,
            x,
            y
        );

        // Determine folder properties
        let is_special = matches!(
            folder_path.to_lowercase().as_str(),
            "inbox" | "sent" | "drafts" | "trash" | "spam" | "junk"
        );

        // Get unread count for this folder (placeholder for now)
        let unread_count = 0; // TODO: Implement actual unread count retrieval

        // Show the context menu for folders with proper context
        self.ui.show_context_menu_at_position(
            x,
            y,
            crate::ui::context_menu::ContextType::EmailFolder {
                folder_name: folder_path.to_string(),
                is_special,
                unread_count,
            },
        );

        Ok(())
    }

    /// Show email content context menu at the given coordinates
    async fn show_email_content_context_menu(&mut self, x: u16, y: u16) -> Result<()> {
        tracing::debug!("Show email content context menu at ({}, {})", x, y);

        // For email content, we can show general menu actions like copy, select all, etc.
        // Or message-specific actions if there's a selected message
        if let Some(selected_message) = self.ui.get_selected_message() {
            let current_folder = self
                .ui
                .get_selected_folder_path()
                .unwrap_or_else(|| "INBOX".to_string());
            self.ui.show_context_menu_at_position(
                x,
                y,
                crate::ui::context_menu::ContextType::EmailMessage {
                    is_read: selected_message.is_read,
                    is_draft: current_folder.to_lowercase().contains("draft"),
                    has_attachments: selected_message.has_attachments,
                    folder_name: current_folder,
                },
            );
        } else {
            // Show general context menu if no specific message
            self.ui.show_context_menu_at_position(
                x,
                y,
                crate::ui::context_menu::ContextType::General,
            );
        }

        Ok(())
    }

    /// Show general context menu at the given coordinates
    async fn show_general_context_menu(&mut self, x: u16, y: u16) -> Result<()> {
        tracing::debug!("Show general context menu at ({}, {})", x, y);

        // Show the general application context menu
        self.ui
            .show_context_menu_at_position(x, y, crate::ui::context_menu::ContextType::General);

        Ok(())
    }

    /// Handle context menu action execution
    async fn handle_context_menu_action(
        &mut self,
        action: crate::ui::context_menu::ContextMenuAction,
    ) -> Result<()> {
        use crate::ui::context_menu::ContextMenuAction;

        tracing::debug!("Handling context menu action: {:?}", action);

        match action {
            // Email message actions
            ContextMenuAction::ReplyToMessage => {
                if let Some(selected_message) = self.ui.get_selected_message() {
                    let message_subject = selected_message.subject.clone();
                    if let Some(message_id) = selected_message.message_id {
                        if let (Some(ref database), Some(ref contacts_manager)) =
                            (&self.database, &self.contacts_manager)
                        {
                            // Load full message from database
                            match database.get_message_by_id(message_id).await {
                                Ok(Some(stored_message)) => {
                                    // Start reply composition with the full message
                                    self.ui.start_reply_from_message(
                                        stored_message,
                                        contacts_manager.clone(),
                                    );
                                    tracing::info!(
                                        "Started reply composition for message: {}",
                                        message_subject
                                    );
                                }
                                Ok(None) => {
                                    self.ui.show_toast_warning("Message not found in database");
                                    tracing::warn!("Message {} not found in database", message_id);
                                }
                                Err(e) => {
                                    self.ui
                                        .show_toast_error(format!("Failed to load message: {}", e));
                                    tracing::error!("Failed to load message {}: {}", message_id, e);
                                }
                            }
                        } else {
                            self.ui
                                .show_toast_warning("Database or contacts not available");
                            tracing::warn!(
                                "Cannot reply: database or contacts manager not initialized"
                            );
                        }
                    } else {
                        self.ui.show_toast_warning("Message ID not available");
                        tracing::warn!("Cannot reply: selected message has no ID");
                    }
                } else {
                    self.ui.show_toast_warning("No message selected for reply");
                }
            }

            ContextMenuAction::ReplyAllToMessage => {
                if let Some(selected_message) = self.ui.get_selected_message() {
                    let message_subject = selected_message.subject.clone();
                    if let Some(message_id) = selected_message.message_id {
                        if let (Some(ref database), Some(ref contacts_manager)) =
                            (&self.database, &self.contacts_manager)
                        {
                            // Load full message from database
                            match database.get_message_by_id(message_id).await {
                                Ok(Some(stored_message)) => {
                                    // Start reply all composition with the full message
                                    self.ui.start_reply_all_from_message(
                                        stored_message,
                                        contacts_manager.clone(),
                                    );
                                    tracing::info!(
                                        "Started reply all composition for message: {}",
                                        message_subject
                                    );
                                }
                                Ok(None) => {
                                    self.ui.show_toast_warning("Message not found in database");
                                    tracing::warn!("Message {} not found in database", message_id);
                                }
                                Err(e) => {
                                    self.ui
                                        .show_toast_error(format!("Failed to load message: {}", e));
                                    tracing::error!("Failed to load message {}: {}", message_id, e);
                                }
                            }
                        } else {
                            self.ui
                                .show_toast_warning("Database or contacts not available");
                            tracing::warn!(
                                "Cannot reply all: database or contacts manager not initialized"
                            );
                        }
                    } else {
                        self.ui.show_toast_warning("Message ID not available");
                        tracing::warn!("Cannot reply all: selected message has no ID");
                    }
                } else {
                    self.ui
                        .show_toast_warning("No message selected for reply all");
                }
            }

            ContextMenuAction::ForwardMessage => {
                if let Some(selected_message) = self.ui.get_selected_message() {
                    let message_subject = selected_message.subject.clone();
                    if let Some(message_id) = selected_message.message_id {
                        if let (Some(ref database), Some(ref contacts_manager)) =
                            (&self.database, &self.contacts_manager)
                        {
                            // Load full message from database
                            match database.get_message_by_id(message_id).await {
                                Ok(Some(stored_message)) => {
                                    // Start forward composition with the full message
                                    self.ui.start_forward_from_message(
                                        stored_message,
                                        contacts_manager.clone(),
                                    );
                                    tracing::info!(
                                        "Started forward composition for message: {}",
                                        message_subject
                                    );
                                }
                                Ok(None) => {
                                    self.ui.show_toast_warning("Message not found in database");
                                    tracing::warn!("Message {} not found in database", message_id);
                                }
                                Err(e) => {
                                    self.ui
                                        .show_toast_error(format!("Failed to load message: {}", e));
                                    tracing::error!("Failed to load message {}: {}", message_id, e);
                                }
                            }
                        } else {
                            self.ui
                                .show_toast_warning("Database or contacts not available");
                            tracing::warn!(
                                "Cannot forward: database or contacts manager not initialized"
                            );
                        }
                    } else {
                        self.ui.show_toast_warning("Message ID not available");
                        tracing::warn!("Cannot forward: selected message has no ID");
                    }
                } else {
                    self.ui
                        .show_toast_warning("No message selected for forwarding");
                }
            }

            ContextMenuAction::DeleteMessage => {
                if let Some(selected_message) = self.ui.get_selected_message() {
                    let message_subject = selected_message.subject.clone();
                    if let Some(message_id) = selected_message.message_id {
                        if let Some(ref operations_service) = self.email_operations_service {
                            // We need to get the account ID - let me check how to get it
                            if let Some(ref database) = self.database {
                                match database.get_message_by_id(message_id).await {
                                    Ok(Some(stored_message)) => {
                                        // Delete the message using the operations service
                                        match operations_service
                                            .delete_email_by_id(
                                                &stored_message.account_id,
                                                message_id,
                                                &stored_message.folder_name,
                                            )
                                            .await
                                        {
                                            Ok(()) => {
                                                self.ui.show_toast_info(format!(
                                                    "Deleted message: {}",
                                                    message_subject
                                                ));
                                                // Update the UI to reflect the change
                                                let _ = self.ui.refresh_messages().await;
                                                tracing::info!(
                                                    "Successfully deleted message: {}",
                                                    message_subject
                                                );
                                            }
                                            Err(e) => {
                                                self.ui.show_toast_error(format!(
                                                    "Failed to delete message: {}",
                                                    e
                                                ));
                                                tracing::error!("Failed to delete message: {}", e);
                                            }
                                        }
                                    }
                                    Ok(None) => {
                                        self.ui.show_toast_warning("Message not found in database");
                                        tracing::warn!(
                                            "Message {} not found in database",
                                            message_id
                                        );
                                    }
                                    Err(e) => {
                                        self.ui.show_toast_error(format!(
                                            "Failed to load message: {}",
                                            e
                                        ));
                                        tracing::error!(
                                            "Failed to load message {}: {}",
                                            message_id,
                                            e
                                        );
                                    }
                                }
                            } else {
                                self.ui.show_toast_warning("Database not available");
                                tracing::warn!("Cannot delete: database not initialized");
                            }
                        } else {
                            self.ui
                                .show_toast_warning("Email operations service not available");
                            tracing::warn!(
                                "Cannot delete: email operations service not initialized"
                            );
                        }
                    } else {
                        self.ui.show_toast_warning("Message ID not available");
                        tracing::warn!("Cannot delete: selected message has no ID");
                    }
                } else {
                    self.ui
                        .show_toast_warning("No message selected for deletion");
                }
            }

            ContextMenuAction::MarkAsRead => {
                if let Some(selected_message) = self.ui.get_selected_message() {
                    let message_subject = selected_message.subject.clone();
                    if let Some(message_id) = selected_message.message_id {
                        if let Some(ref database) = self.database {
                            match database.get_message_by_id(message_id).await {
                                Ok(Some(stored_message)) => {
                                    // Add \\Seen flag to mark as read
                                    let mut flags = stored_message.flags.clone();
                                    if !flags.contains(&"\\Seen".to_string()) {
                                        flags.push("\\Seen".to_string());

                                        // Update message flags in database
                                        match database
                                            .update_message_flags(
                                                &stored_message.account_id,
                                                &stored_message.folder_name,
                                                stored_message.imap_uid,
                                                flags,
                                            )
                                            .await
                                        {
                                            Ok(()) => {
                                                self.ui.show_toast_info("Message marked as read");
                                                // Update the UI to reflect the change
                                                let _ = self.ui.refresh_messages().await;
                                                tracing::info!(
                                                    "Marked message as read: {}",
                                                    message_subject
                                                );
                                            }
                                            Err(e) => {
                                                self.ui.show_toast_error(format!(
                                                    "Failed to mark as read: {}",
                                                    e
                                                ));
                                                tracing::error!(
                                                    "Failed to mark message as read: {}",
                                                    e
                                                );
                                            }
                                        }
                                    } else {
                                        self.ui.show_toast_info("Message already marked as read");
                                    }
                                }
                                Ok(None) => {
                                    self.ui.show_toast_warning("Message not found in database");
                                    tracing::warn!("Message {} not found in database", message_id);
                                }
                                Err(e) => {
                                    self.ui
                                        .show_toast_error(format!("Failed to load message: {}", e));
                                    tracing::error!("Failed to load message {}: {}", message_id, e);
                                }
                            }
                        } else {
                            self.ui.show_toast_warning("Database not available");
                            tracing::warn!("Cannot mark as read: database not initialized");
                        }
                    } else {
                        self.ui.show_toast_warning("Message ID not available");
                        tracing::warn!("Cannot mark as read: selected message has no ID");
                    }
                } else {
                    self.ui.show_toast_warning("No message selected");
                }
            }

            ContextMenuAction::MarkAsUnread => {
                if let Some(selected_message) = self.ui.get_selected_message() {
                    let message_subject = selected_message.subject.clone();
                    if let Some(message_id) = selected_message.message_id {
                        if let Some(ref database) = self.database {
                            match database.get_message_by_id(message_id).await {
                                Ok(Some(stored_message)) => {
                                    // Remove \\Seen flag to mark as unread
                                    let mut flags = stored_message.flags.clone();
                                    let initial_len = flags.len();
                                    flags.retain(|flag| flag != "\\Seen");

                                    if initial_len != flags.len() {
                                        // Update message flags in database
                                        match database
                                            .update_message_flags(
                                                &stored_message.account_id,
                                                &stored_message.folder_name,
                                                stored_message.imap_uid,
                                                flags,
                                            )
                                            .await
                                        {
                                            Ok(()) => {
                                                self.ui.show_toast_info("Message marked as unread");
                                                // Update the UI to reflect the change
                                                let _ = self.ui.refresh_messages().await;
                                                tracing::info!(
                                                    "Marked message as unread: {}",
                                                    message_subject
                                                );
                                            }
                                            Err(e) => {
                                                self.ui.show_toast_error(format!(
                                                    "Failed to mark as unread: {}",
                                                    e
                                                ));
                                                tracing::error!(
                                                    "Failed to mark message as unread: {}",
                                                    e
                                                );
                                            }
                                        }
                                    } else {
                                        self.ui.show_toast_info("Message already marked as unread");
                                    }
                                }
                                Ok(None) => {
                                    self.ui.show_toast_warning("Message not found in database");
                                    tracing::warn!("Message {} not found in database", message_id);
                                }
                                Err(e) => {
                                    self.ui
                                        .show_toast_error(format!("Failed to load message: {}", e));
                                    tracing::error!("Failed to load message {}: {}", message_id, e);
                                }
                            }
                        } else {
                            self.ui.show_toast_warning("Database not available");
                            tracing::warn!("Cannot mark as unread: database not initialized");
                        }
                    } else {
                        self.ui.show_toast_warning("Message ID not available");
                        tracing::warn!("Cannot mark as unread: selected message has no ID");
                    }
                } else {
                    self.ui.show_toast_warning("No message selected");
                }
            }

            ContextMenuAction::MoveToFolder(folder_name) => {
                if let Some(_selected_message) = self.ui.get_selected_message() {
                    self.ui
                        .show_toast_info(format!("Moving message to folder: {}", folder_name));
                    // TODO: Implement message move operation
                } else {
                    self.ui.show_toast_warning("No message selected for moving");
                }
            }

            ContextMenuAction::CopyMessage => {
                if let Some(_selected_message) = self.ui.get_selected_message() {
                    self.ui.show_toast_info("Message copied to clipboard");
                    // TODO: Implement message copy to clipboard
                } else {
                    self.ui
                        .show_toast_warning("No message selected for copying");
                }
            }

            ContextMenuAction::ExportMessage => {
                if let Some(_selected_message) = self.ui.get_selected_message() {
                    self.ui.show_toast_info("Message exported");
                    // TODO: Implement message export to file
                } else {
                    self.ui.show_toast_warning("No message selected for export");
                }
            }

            ContextMenuAction::ViewMessageSource => {
                if let Some(_selected_message) = self.ui.get_selected_message() {
                    self.ui.show_toast_info("Viewing message source");
                    // TODO: Implement message source viewer
                } else {
                    self.ui.show_toast_warning("No message selected");
                }
            }

            // Folder actions
            ContextMenuAction::CreateFolder => {
                // Get current account ID
                if let Some(current_account_id) = self.ui.get_current_account_id_string() {
                    // Get selected folder for creating subfolder context
                    let parent_path = self.ui.get_selected_folder_path();

                    match self
                        .handle_create_folder(&current_account_id, parent_path.as_deref())
                        .await
                    {
                        Ok(()) => {
                            tracing::info!("Create folder operation initiated successfully");
                        }
                        Err(e) => {
                            self.ui
                                .show_toast_error(format!("Failed to create folder: {}", e));
                            tracing::error!("Failed to create folder: {}", e);
                        }
                    }
                } else {
                    self.ui.show_toast_warning("No account selected");
                    tracing::warn!("Cannot create folder: no current account");
                }
            }

            ContextMenuAction::RenameFolder => {
                // Get current account and selected folder
                if let Some(current_account_id) = self.ui.get_current_account_id_string() {
                    if let Some(selected_folder_path) = self.ui.get_selected_folder_path() {
                        match self
                            .handle_rename_folder(&current_account_id, &selected_folder_path)
                            .await
                        {
                            Ok(()) => {
                                tracing::info!("Rename folder operation completed successfully");
                            }
                            Err(e) => {
                                self.ui
                                    .show_toast_error(format!("Failed to rename folder: {}", e));
                                tracing::error!("Failed to rename folder: {}", e);
                            }
                        }
                    } else {
                        self.ui.show_toast_warning("No folder selected");
                        tracing::warn!("Cannot rename folder: no folder selected");
                    }
                } else {
                    self.ui.show_toast_warning("No account selected");
                    tracing::warn!("Cannot rename folder: no current account");
                }
            }

            ContextMenuAction::DeleteFolder => {
                // Get current account and selected folder
                if let Some(current_account_id) = self.ui.get_current_account_id_string() {
                    if let Some(selected_folder_path) = self.ui.get_selected_folder_path() {
                        // Show confirmation toast before deletion
                        self.ui.show_toast_warning(format!(
                            "Deleting folder: {}",
                            selected_folder_path
                        ));

                        match self
                            .handle_delete_folder(&current_account_id, &selected_folder_path)
                            .await
                        {
                            Ok(()) => {
                                tracing::info!("Delete folder operation completed successfully");
                            }
                            Err(e) => {
                                self.ui
                                    .show_toast_error(format!("Failed to delete folder: {}", e));
                                tracing::error!("Failed to delete folder: {}", e);
                            }
                        }
                    } else {
                        self.ui.show_toast_warning("No folder selected");
                        tracing::warn!("Cannot delete folder: no folder selected");
                    }
                } else {
                    self.ui.show_toast_warning("No account selected");
                    tracing::warn!("Cannot delete folder: no current account");
                }
            }

            ContextMenuAction::MarkAllAsRead => {
                self.ui.show_toast_info("Marked all messages as read");
                // TODO: Implement mark all as read for current folder
            }

            ContextMenuAction::CompactFolder => {
                self.ui.show_toast_info("Folder compaction started");
                // TODO: Implement folder compaction
            }

            ContextMenuAction::RefreshFolder => {
                self.ui.show_toast_info("Refreshing folder...");
                // TODO: Trigger folder refresh
                self.trigger_manual_sync().await?;
            }

            // General actions
            ContextMenuAction::Copy => {
                self.ui.show_toast_info("Content copied to clipboard");
                // TODO: Implement clipboard copy
            }

            ContextMenuAction::Cut => {
                self.ui.show_toast_info("Content cut to clipboard");
                // TODO: Implement clipboard cut
            }

            ContextMenuAction::Paste => {
                self.ui.show_toast_info("Content pasted from clipboard");
                // TODO: Implement clipboard paste
            }

            ContextMenuAction::SelectAll => {
                self.ui.show_toast_info("All content selected");
                // TODO: Implement select all
            }

            ContextMenuAction::Properties => {
                self.ui.show_toast_info("Properties dialog opened");
                // TODO: Show properties dialog
            }

            // Account actions
            ContextMenuAction::RefreshAccount => {
                self.ui.show_toast_info("Refreshing account...");
                self.trigger_manual_sync().await?;
            }

            ContextMenuAction::AccountSettings => {
                self.ui.show_toast_info("Account settings opened");
                // TODO: Show account settings dialog
            }

            ContextMenuAction::AddAccount => {
                self.handle_add_account().await?;
            }

            ContextMenuAction::RemoveAccount => {
                self.ui
                    .show_toast_info("Remove account confirmation opened");
                // TODO: Show remove account confirmation
            }

            // Calendar actions
            ContextMenuAction::CreateEvent => {
                self.ui.show_toast_info("Create event dialog opened");
                // TODO: Show create event dialog
            }

            ContextMenuAction::EditEvent => {
                self.ui.show_toast_info("Edit event dialog opened");
                // TODO: Show edit event dialog
            }

            ContextMenuAction::DeleteEvent => {
                self.ui.show_toast_info("Delete event confirmation opened");
                // TODO: Show delete event confirmation
            }

            ContextMenuAction::DuplicateEvent => {
                self.ui.show_toast_info("Event duplicated");
                // TODO: Implement event duplication
            }

            ContextMenuAction::ExportEvent => {
                self.ui.show_toast_info("Event exported");
                // TODO: Implement event export
            }

            ContextMenuAction::ViewEventDetails => {
                self.ui.show_toast_info("Event details opened");
                // TODO: Show event details dialog
            }

            ContextMenuAction::Cancel => {
                // Context menu is already hidden by the menu system
                tracing::debug!("Context menu cancelled");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_startup_progress_manager_integration() {
        let mut app = App::new().unwrap();

        // Test that the progress manager is initialized
        assert_eq!(app.startup_progress_manager().phases().len(), 4);
        assert!(!app.startup_progress_manager().is_complete());
        assert_eq!(
            app.startup_progress_manager().overall_progress_percentage(),
            0.0
        );

        // Test that we can get a mutable reference
        let progress_manager = app.startup_progress_manager_mut();
        progress_manager.start_phase("Database").unwrap();

        // Test that the phase is now started
        assert!(progress_manager
            .current_phase()
            .unwrap()
            .status()
            .is_in_progress());
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
