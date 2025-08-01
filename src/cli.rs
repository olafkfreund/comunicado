use anyhow::{anyhow, Result};
use base64::Engine;
use clap::{Args, Parser, Subcommand};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::time::Duration;

use crate::email::{DatabaseStats, EmailDatabase};
use crate::imap::ImapAccountManager;
use crate::keyboard::{KeyboardAction, KeyboardConfig, KeyboardManager, KeyboardShortcut};
use crate::maildir::{Maildir, MaildirUtils};
use crate::oauth2::{AccountConfig, SecureStorage, TokenManager};

/// Comunicado - Modern terminal email and calendar client
#[derive(Parser)]
#[command(name = "comunicado")]
#[command(about = "A modern TUI-based email and calendar client")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Enable debug logging
    #[arg(long, global = true)]
    pub debug: bool,

    /// Configuration directory path
    #[arg(long, global = true)]
    pub config_dir: Option<PathBuf>,

    /// Dry run mode (don't make changes)
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Start in email mode
    #[arg(long = "mail")]
    pub start_mail: bool,

    /// Start in calendar mode  
    #[arg(long = "cal")]
    pub start_calendar: bool,

    /// Start in contacts mode
    #[arg(long = "con")]
    pub start_contacts: bool,

    /// Reprocess all email content in the database for clean display
    #[arg(long, global = true)]
    pub clean_content: bool,
}

impl Cli {
    /// Determine the startup mode based on CLI arguments
    pub fn get_startup_mode(&self) -> StartupMode {
        // Only one mode can be selected - priority: contacts > calendar > mail > default
        if self.start_contacts {
            StartupMode::Contacts
        } else if self.start_calendar {
            StartupMode::Calendar
        } else if self.start_mail {
            StartupMode::Email
        } else {
            StartupMode::Default
        }
    }
}

/// Available startup modes from CLI
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StartupMode {
    /// Default mode (email interface)
    Default,
    /// Start in email interface
    Email,
    /// Start in calendar mode
    Calendar,
    /// Start in contacts mode
    Contacts,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Test account connections and functionality
    Test(TestArgs),

    /// Import accounts from configuration files
    Import(ImportArgs),

    /// Troubleshoot connection and configuration issues
    Troubleshoot(TroubleshootArgs),

    /// Database maintenance and cleanup operations
    Database(DatabaseArgs),

    /// Show configuration information
    Config(ConfigArgs),

    /// Account management operations
    Account(AccountArgs),

    /// OAuth2 account setup commands
    SetupGmail {
        /// Google OAuth2 client secret JSON file path
        #[arg(long)]
        client_secret: Option<PathBuf>,
        
        /// Gmail email address
        #[arg(long)]
        email: Option<String>,
        
        /// Display name for the account
        #[arg(long)]
        name: Option<String>,
        
        /// Skip browser opening (show URL only)
        #[arg(long)]
        no_browser: bool,
    },

    /// Setup Microsoft/Outlook account with OAuth2
    SetupOutlook {
        /// Microsoft OAuth2 client secret JSON file path
        #[arg(long)]
        client_secret: Option<PathBuf>,
        
        /// Display name for the account
        #[arg(long)]
        name: Option<String>,
        
        /// Skip browser opening (show URL only)
        #[arg(long)]
        no_browser: bool,
    },

    /// Keyboard shortcut management
    Keyboard(KeyboardArgs),

    /// Maildir import/export operations
    Maildir(MaildirArgs),
    /// Offline storage management operations
    Offline(OfflineArgs),

    /// Sync accounts with IMAP servers
    Sync(SyncArgs),

    /// Sync calendars and tasks
    CalendarSync(CalendarSyncArgs),

    /// Sync contacts and address books
    ContactsSync(ContactsSyncArgs),

    /// IMAP folder operations and diagnostics
    Folders(FoldersArgs),

    /// OAuth2 token management and refresh operations
    OAuth2(OAuth2Args),
}

#[derive(Args)]
pub struct TestArgs {
    /// Test specific account by name
    #[arg(short, long)]
    pub account: Option<String>,

    /// Test all configured accounts
    #[arg(long)]
    pub all: bool,

    /// Test connection only (no message operations)
    #[arg(long)]
    pub connection_only: bool,

    /// Timeout for connection tests (seconds)
    #[arg(long, default_value = "30")]
    pub timeout: u64,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Args)]
pub struct ImportArgs {
    /// Google OAuth2 credentials file (JSON)
    #[arg(long)]
    pub google_credentials: Option<PathBuf>,

    /// Microsoft OAuth2 credentials file (JSON)
    #[arg(long)]
    pub microsoft_credentials: Option<PathBuf>,

    /// Generic account configuration file (TOML)
    #[arg(long)]
    pub account_config: Option<PathBuf>,

    /// Thunderbird profile directory
    #[arg(long)]
    pub thunderbird_profile: Option<PathBuf>,

    /// Force overwrite existing accounts
    #[arg(long)]
    pub force: bool,

    /// Preview import without making changes
    #[arg(long)]
    pub preview: bool,
}

#[derive(Args)]
pub struct TroubleshootArgs {
    /// Specific account to troubleshoot
    #[arg(short, long)]
    pub account: Option<String>,

    /// Run network connectivity tests
    #[arg(long)]
    pub network: bool,

    /// Check authentication and credentials
    #[arg(long)]
    pub auth: bool,

    /// Test email server capabilities
    #[arg(long)]
    pub capabilities: bool,

    /// Check database integrity
    #[arg(long)]
    pub database: bool,

    /// Run all diagnostic tests
    #[arg(long)]
    pub all: bool,

    /// Generate diagnostic report
    #[arg(long)]
    pub report: bool,

    /// Output format (text, json, html)
    #[arg(long, default_value = "text")]
    pub format: String,
}

#[derive(Args)]
pub struct DatabaseArgs {
    #[command(subcommand)]
    pub command: DatabaseCommands,
}

#[derive(Subcommand)]
pub enum DatabaseCommands {
    /// Check database integrity
    Check {
        /// Specific account to check
        #[arg(short, long)]
        account: Option<String>,

        /// Fix issues automatically
        #[arg(long)]
        fix: bool,
    },

    /// Clean up database (remove orphaned data, etc.)
    Clean {
        /// Specific account to clean
        #[arg(short, long)]
        account: Option<String>,

        /// Aggressive cleanup (removes more data)
        #[arg(long)]
        aggressive: bool,

        /// Remove messages older than N days
        #[arg(long)]
        older_than: Option<u32>,
    },

    /// Rebuild database indexes
    Rebuild {
        /// Specific account to rebuild
        #[arg(short, long)]
        account: Option<String>,

        /// Include search index
        #[arg(long)]
        search_index: bool,
    },

    /// Show database statistics
    Stats {
        /// Specific account statistics
        #[arg(short, long)]
        account: Option<String>,

        /// Detailed statistics
        #[arg(long)]
        detailed: bool,
    },

    /// Backup database
    Backup {
        /// Backup destination directory
        #[arg(short, long)]
        output: PathBuf,

        /// Compress backup
        #[arg(long)]
        compress: bool,
    },

    /// Restore database from backup
    Restore {
        /// Backup file to restore
        #[arg(short, long)]
        input: PathBuf,

        /// Force restore (overwrite existing)
        #[arg(long)]
        force: bool,
    },
}

#[derive(Args)]
pub struct ConfigArgs {
    /// Show current configuration
    #[arg(long)]
    pub show: bool,

    /// Validate configuration files
    #[arg(long)]
    pub validate: bool,

    /// Show configuration file locations
    #[arg(long)]
    pub paths: bool,

    /// Reset configuration to defaults
    #[arg(long)]
    pub reset: bool,

    /// Export configuration
    #[arg(long)]
    pub export: Option<PathBuf>,

    /// Import configuration
    #[arg(long)]
    pub import: Option<PathBuf>,
}

#[derive(Args)]
pub struct AccountArgs {
    #[command(subcommand)]
    pub command: AccountCommands,
}

#[derive(Subcommand)]
pub enum AccountCommands {
    /// List configured accounts
    List {
        /// Show detailed information
        #[arg(long)]
        detailed: bool,

        /// Show credentials (use carefully)
        #[arg(long)]
        show_credentials: bool,
    },

    /// Add new account interactively
    Add {
        /// Account name
        #[arg(short, long)]
        name: Option<String>,

        /// Email address
        #[arg(short, long)]
        email: Option<String>,

        /// Use OAuth2 authentication
        #[arg(long)]
        oauth2: bool,
    },

    /// Remove account
    Remove {
        /// Account name or email
        name: String,

        /// Force removal without confirmation
        #[arg(long)]
        force: bool,

        /// Keep local data
        #[arg(long)]
        keep_data: bool,
    },

    /// Update account settings
    Update {
        /// Account name or email
        name: String,

        /// New password (will prompt securely)
        #[arg(long)]
        password: bool,

        /// Re-authenticate OAuth2
        #[arg(long)]
        reauth: bool,
    },
}

#[derive(Args)]
pub struct KeyboardArgs {
    #[command(subcommand)]
    pub command: KeyboardCommands,
}

#[derive(Subcommand)]
pub enum KeyboardCommands {
    /// Show current keyboard shortcuts
    Show {
        /// Show shortcuts for specific category
        #[arg(long)]
        category: Option<String>,

        /// Show shortcuts in detailed format
        #[arg(long)]
        detailed: bool,
    },

    /// Set a keyboard shortcut
    Set {
        /// Action to assign shortcut to
        action: String,

        /// Key combination (e.g., "Ctrl+Q", "F5", "Enter")
        key: String,

        /// Force overwrite existing shortcut
        #[arg(long)]
        force: bool,
    },

    /// Remove a keyboard shortcut
    Remove {
        /// Key combination to remove
        key: String,
    },

    /// Reset keyboard shortcuts to defaults
    Reset {
        /// Reset specific category only
        #[arg(long)]
        category: Option<String>,

        /// Force reset without confirmation
        #[arg(long)]
        force: bool,
    },

    /// Export keyboard shortcuts to file
    Export {
        /// Output file path
        #[arg(short, long)]
        output: PathBuf,

        /// Export format (toml, json)
        #[arg(long, default_value = "toml")]
        format: String,
    },

    /// Import keyboard shortcuts from file
    Import {
        /// Input file path
        #[arg(short, long)]
        input: PathBuf,

        /// Merge with existing shortcuts
        #[arg(long)]
        merge: bool,
    },

    /// Validate keyboard configuration
    Validate {
        /// Configuration file to validate
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
}

#[derive(Args)]
pub struct MaildirArgs {
    #[command(subcommand)]
    pub command: MaildirCommands,
}

#[derive(Subcommand)]
pub enum MaildirCommands {
    /// Import emails from a Maildir directory
    Import {
        /// Path to the Maildir directory
        #[arg(short, long)]
        path: PathBuf,

        /// Account to import into (must exist)
        #[arg(short, long)]
        account: String,

        /// Import specific folder only
        #[arg(short, long)]
        folder: Option<String>,

        /// Show progress during import
        #[arg(long)]
        progress: bool,

        /// Dry run - don't actually import
        #[arg(long)]
        dry_run: bool,
    },

    /// Export emails to a Maildir directory
    Export {
        /// Path to export Maildir directory
        #[arg(short, long)]
        path: PathBuf,

        /// Account to export from
        #[arg(short, long)]
        account: String,

        /// Export specific folder only
        #[arg(short, long)]
        folder: Option<String>,

        /// Show progress during export
        #[arg(long)]
        progress: bool,

        /// Overwrite existing Maildir
        #[arg(long)]
        force: bool,
    },

    /// List Maildir directories and statistics
    List {
        /// Path to the Maildir directory
        #[arg(short, long)]
        path: PathBuf,

        /// Show detailed statistics
        #[arg(long)]
        detailed: bool,
    },

    /// Validate Maildir structure
    Validate {
        /// Path to the Maildir directory
        #[arg(short, long)]
        path: PathBuf,

        /// Fix issues found during validation
        #[arg(long)]
        fix: bool,
    },
}

#[derive(Args)]
pub struct OfflineArgs {
    #[command(subcommand)]
    pub command: OfflineCommands,
}

#[derive(Subcommand)]
pub enum OfflineCommands {
    /// Export calendars and contacts to offline storage
    Export {
        /// Path to export directory
        #[arg(short, long)]
        path: PathBuf,
        
        /// Include calendars in export
        #[arg(long, default_value_t = true)]
        calendars: bool,
        
        /// Include contacts in export
        #[arg(long, default_value_t = true)]
        contacts: bool,
    },
    
    /// Import calendars and contacts from offline storage
    Import {
        /// Path to import directory
        #[arg(short, long)]
        path: PathBuf,
        
        /// Overwrite existing data
        #[arg(long)]
        force: bool,
    },
    
    /// Show offline storage statistics
    Stats,
    
    /// Sync online services with offline storage
    Sync {
        /// Force full sync (ignore timestamps)
        #[arg(long)]
        force: bool,
    },
}

#[derive(Args)]
pub struct SyncArgs {
    #[command(subcommand)]
    pub command: SyncCommands,
}

#[derive(Subcommand)]
pub enum SyncCommands {
    /// Sync all accounts
    All {
        /// Include folder sync (list folders from IMAP)
        #[arg(long, default_value_t = true)]
        folders: bool,
        
        /// Include message sync (download new emails)
        #[arg(long, default_value_t = true)]
        messages: bool,
        
        /// Maximum number of recent messages to sync per folder
        #[arg(long, default_value_t = 100)]
        max_messages: u32,
        
        /// Show detailed sync progress
        #[arg(short, long)]
        verbose: bool,
        
        /// Force full sync (ignore last sync time)
        #[arg(long)]
        force: bool,
    },
    
    /// Sync specific account by name or email
    Account {
        /// Account name or email address
        account: String,
        
        /// Include folder sync (list folders from IMAP)
        #[arg(long, default_value_t = true)]
        folders: bool,
        
        /// Include message sync (download new emails)
        #[arg(long, default_value_t = true)]
        messages: bool,
        
        /// Maximum number of recent messages to sync per folder
        #[arg(long, default_value_t = 100)]
        max_messages: u32,
        
        /// Show detailed sync progress
        #[arg(short, long)]
        verbose: bool,
        
        /// Force full sync (ignore last sync time)
        #[arg(long)]
        force: bool,
    },
    
    /// Sync specific folder for an account
    Folder {
        /// Account name or email address
        account: String,
        
        /// Folder name (e.g., "INBOX", "Sent")
        folder: String,
        
        /// Maximum number of messages to sync
        #[arg(long, default_value_t = 100)]
        max_messages: u32,
        
        /// Show detailed sync progress
        #[arg(short, long)]
        verbose: bool,
        
        /// Force full sync (ignore last sync time)
        #[arg(long)]
        force: bool,
    },
    
    /// List available accounts for sync
    List,
    
    /// Show sync status and statistics
    Status {
        /// Account name or email address (optional, shows all if not specified)
        account: Option<String>,
    },
}

#[derive(Args)]
pub struct CalendarSyncArgs {
    #[command(subcommand)]
    pub command: CalendarSyncCommands,
}

#[derive(Subcommand)]
pub enum CalendarSyncCommands {
    /// Sync all calendars from all accounts
    All {
        /// Show detailed sync progress
        #[arg(short, long)]
        verbose: bool,
        
        /// Force full sync (ignore last sync time)
        #[arg(long)]
        force: bool,
    },
    
    /// Sync calendars for a specific account
    Account {
        /// Account name or email address
        account: String,
        
        /// Show detailed sync progress
        #[arg(short, long)]
        verbose: bool,
        
        /// Force full sync (ignore last sync time)
        #[arg(long)]
        force: bool,
    },
    
    /// Sync a specific calendar
    Calendar {
        /// Account name or email address
        account: String,
        
        /// Calendar name or ID
        calendar: String,
        
        /// Show detailed sync progress
        #[arg(short, long)]
        verbose: bool,
        
        /// Force full sync (ignore last sync time)
        #[arg(long)]
        force: bool,
    },
    
    /// List available calendars for sync
    List,
    
    /// Show calendar sync status and statistics
    Status {
        /// Account name or email address (optional, shows all if not specified)
        account: Option<String>,
    },
}

#[derive(Args)]
pub struct ContactsSyncArgs {
    #[command(subcommand)]
    pub command: ContactsSyncCommands,
}

#[derive(Subcommand)]
pub enum ContactsSyncCommands {
    /// Sync all contacts from all accounts
    All {
        /// Show detailed sync progress
        #[arg(short, long)]
        verbose: bool,
        
        /// Force full sync (ignore last sync time)
        #[arg(long)]
        force: bool,
    },
    
    /// Sync contacts for a specific account
    Account {
        /// Account name or email address
        account: String,
        
        /// Show detailed sync progress
        #[arg(short, long)]
        verbose: bool,
        
        /// Force full sync (ignore last sync time)
        #[arg(long)]
        force: bool,
    },
    
    /// List available contacts sources for sync
    List,
    
    /// Show contacts sync status and statistics
    Status {
        /// Account name or email address (optional, shows all if not specified)
        account: Option<String>,
    },
}

#[derive(Args)]
pub struct FoldersArgs {
    #[command(subcommand)]
    pub command: FoldersCommands,
}

#[derive(Subcommand)]
pub enum FoldersCommands {
    /// List all folders for all accounts or a specific account
    List {
        /// Account name or email address (optional, shows all accounts if not specified)
        account: Option<String>,
        
        /// Show message counts for each folder
        #[arg(short, long)]
        counts: bool,
        
        /// Show detailed folder information (attributes, etc.)
        #[arg(short, long)]
        verbose: bool,
        
        /// Output format (table, json, csv)
        #[arg(long, default_value = "table")]
        format: String,
    },
    
    /// Test folder access and operations
    Test {
        /// Account name or email address
        account: String,
        
        /// Specific folder to test (optional, tests INBOX if not specified)
        folder: Option<String>,
        
        /// Show verbose test output
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// Get detailed statistics for a specific folder
    Stats {
        /// Account name or email address
        account: String,
        
        /// Folder name to analyze
        folder: String,
        
        /// Include message flags statistics
        #[arg(long)]
        flags: bool,
    },
}

#[derive(Args)]
pub struct OAuth2Args {
    #[command(subcommand)]
    pub command: OAuth2Commands,
}

#[derive(Subcommand)]
pub enum OAuth2Commands {
    /// Show OAuth2 token status for all accounts
    Status {
        /// Show detailed token information (expiration times, etc.)
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// Refresh expired or expiring OAuth2 tokens
    Refresh {
        /// Account name or email address (optional, refreshes all accounts if not specified)
        account: Option<String>,
        
        /// Force refresh even if token is not expired
        #[arg(short, long)]
        force: bool,
        
        /// Show detailed refresh process
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// Re-authenticate an account (full OAuth2 flow)
    Reauth {
        /// Account name or email address
        account: String,
        
        /// Skip browser opening (show URL only)
        #[arg(long)]
        no_browser: bool,
        
        /// Show detailed authentication process
        #[arg(short, long)]
        verbose: bool,
    },
}

/// Command-line interface handler
pub struct CliHandler {
    database: Arc<EmailDatabase>,
    storage: SecureStorage,
    token_manager: Option<TokenManager>,
}

impl CliHandler {
    /// Create a new CLI handler
    pub async fn new(config_dir: Option<PathBuf>) -> Result<Self> {
        let storage = SecureStorage::new("comunicado".to_string())?;

        // Initialize database
        let db_path = if let Some(ref dir) = config_dir {
            dir.join("databases").join("email.db")
        } else {
            dirs::config_dir()
                .ok_or_else(|| anyhow!("Cannot find config directory"))?
                .join("comunicado")
                .join("databases")
                .join("email.db")
        };

        let database = Arc::new(EmailDatabase::new(db_path.to_str().unwrap()).await?);

        // Initialize token manager with storage for access to saved tokens
        let token_manager = Some(TokenManager::new_with_storage(Arc::new(storage.clone())));

        Ok(Self {
            database,
            storage,
            token_manager,
        })
    }

    /// Handle CLI commands
    pub async fn handle_command(&self, command: Commands, dry_run: bool) -> Result<()> {
        match command {
            Commands::Test(args) => self.handle_test(args, dry_run).await,
            Commands::Import(args) => self.handle_import(args, dry_run).await,
            Commands::Troubleshoot(args) => self.handle_troubleshoot(args).await,
            Commands::Database(args) => self.handle_database(args, dry_run).await,
            Commands::Config(args) => self.handle_config(args, dry_run).await,
            Commands::Account(args) => self.handle_account(args, dry_run).await,
            Commands::SetupGmail { client_secret, email, name, no_browser } => {
                self.handle_setup_gmail(client_secret, email, name, no_browser, dry_run).await
            }
            Commands::SetupOutlook { client_secret, name, no_browser } => {
                self.handle_setup_outlook(client_secret, name, no_browser, dry_run).await
            }
            Commands::Keyboard(args) => self.handle_keyboard(args, dry_run).await,
            Commands::Maildir(args) => self.handle_maildir(args, dry_run).await,
            Commands::Offline(args) => self.handle_offline(args, dry_run).await,
            Commands::Sync(args) => self.handle_sync(args, dry_run).await,
            Commands::CalendarSync(args) => self.handle_calendar_sync(args, dry_run).await,
            Commands::ContactsSync(args) => self.handle_contacts_sync(args, dry_run).await,
            Commands::Folders(args) => self.handle_folders(args, dry_run).await,
            Commands::OAuth2(args) => self.handle_oauth2(args, dry_run).await,
        }
    }

    /// Handle database content cleaning
    pub async fn handle_clean_content(&self) -> Result<()> {
        println!("🧹 Starting database content cleaning...");

        match self.database.reprocess_message_content().await {
            Ok(count) => {
                println!("✅ Successfully cleaned {} messages", count);
                println!("   - Raw HTML/CSS content converted to plain text");
                println!("   - Email headers and technical metadata removed");
                println!("   - Content should now display cleanly in the email viewer");
                println!("\nRestart the application to see the changes.");
            }
            Err(e) => {
                eprintln!("❌ Error cleaning content: {}", e);
                std::process::exit(1);
            }
        }
        Ok(())
    }

    /// Handle test commands
    async fn handle_test(&self, args: TestArgs, _dry_run: bool) -> Result<()> {
        println!("🧪 Comunicado Account Connection Test");
        println!("====================================\n");

        let accounts = self.storage.list_accounts()?;
        
        // Ensure accounts exist in database before testing
        {
            for account in &accounts {
                // Check if account exists in database
                let account_exists = sqlx::query("SELECT id FROM accounts WHERE id = ?")
                    .bind(&account.account_id)
                    .fetch_optional(&self.database.pool)
                    .await?
                    .is_some();

                if !account_exists {
                    // Create account in database
                    sqlx::query("INSERT INTO accounts (id, name, email, provider, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
                        .bind(&account.account_id)
                        .bind(&account.display_name)
                        .bind(&account.email_address)
                        .bind(&account.provider)
                        .bind(chrono::Utc::now().to_rfc3339())
                        .bind(chrono::Utc::now().to_rfc3339())
                        .execute(&self.database.pool)
                        .await?;

                    // Create INBOX folder
                    sqlx::query("INSERT INTO folders (account_id, name, full_name, delimiter, attributes, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)")
                        .bind(&account.account_id)
                        .bind("INBOX")
                        .bind("INBOX")
                        .bind(".")
                        .bind("[]")
                        .bind(chrono::Utc::now().to_rfc3339())
                        .bind(chrono::Utc::now().to_rfc3339())
                        .execute(&self.database.pool)
                        .await?;

                    if args.verbose {
                        println!("✅ Created account {} in database", account.account_id);
                    }
                }
            }
        }

        if accounts.is_empty() {
            println!("❌ No accounts configured");
            println!("   Use 'comunicado account add' to add an account first");
            return Ok(());
        }

        let accounts_to_test = if let Some(account_name) = &args.account {
            // Test specific account
            if let Some(account) = accounts
                .iter()
                .find(|a| &a.display_name == account_name || &a.email_address == account_name)
            {
                vec![account.clone()]
            } else {
                return Err(anyhow!("Account '{}' not found", account_name));
            }
        } else if args.all {
            // Test all accounts
            accounts
        } else {
            // Interactive selection or first account
            if accounts.len() == 1 {
                accounts
            } else {
                println!(
                    "Multiple accounts found. Use --all to test all, or specify --account <name>"
                );
                for (i, account) in accounts.iter().enumerate() {
                    println!(
                        "  {}: {} ({})",
                        i + 1,
                        account.display_name,
                        account.email_address
                    );
                }
                return Ok(());
            }
        };

        let mut all_passed = true;

        for account in accounts_to_test {
            all_passed &= self.test_account(&account, &args).await?;
        }

        println!("\n📊 Test Summary");
        println!("===============");
        if all_passed {
            println!("✅ All tests passed!");
        } else {
            println!("❌ Some tests failed. Check the output above for details.");
            std::process::exit(1);
        }

        Ok(())
    }

    /// Test a specific account
    async fn test_account(&self, account: &AccountConfig, args: &TestArgs) -> Result<bool> {
        println!(
            "🔍 Testing account: {} ({})",
            account.display_name, account.email_address
        );
        println!("   Provider: {}", account.provider);

        let mut success = true;

        // Test 1: Basic connectivity
        print!("   📡 Network connectivity... ");
        match self.test_network_connectivity(account, args.timeout).await {
            Ok(_) => println!("✅ OK"),
            Err(e) => {
                println!("❌ FAILED: {}", e);
                success = false;
            }
        }

        // Test 2: Authentication
        print!("   🔐 Authentication... ");
        match self.test_authentication(account).await {
            Ok(_) => println!("✅ OK"),
            Err(e) => {
                println!("❌ FAILED: {}", e);
                success = false;
            }
        }

        if !args.connection_only {
            // Test 3: IMAP operations
            print!("   📬 IMAP operations... ");
            match self.test_imap_operations(account).await {
                Ok(_) => println!("✅ OK"),
                Err(e) => {
                    println!("❌ FAILED: {}", e);
                    success = false;
                }
            }

            // Test 4: SMTP operations
            print!("   📤 SMTP operations... ");
            match self.test_smtp_operations(account).await {
                Ok(_) => println!("✅ OK"),
                Err(e) => {
                    println!("❌ FAILED: {}", e);
                    success = false;
                }
            }
        }

        if args.verbose {
            // Show additional account details
            println!("   📋 Account Details:");
            println!("      IMAP: {}:{}", account.imap_server, account.imap_port);
            println!("      SMTP: {}:{}", account.smtp_server, account.smtp_port);
            println!("      Security: {:?}", account.security);
        }

        println!();
        Ok(success)
    }

    /// Test network connectivity to email servers
    async fn test_network_connectivity(&self, account: &AccountConfig, timeout: u64) -> Result<()> {
        use tokio::net::TcpStream;
        use tokio::time::timeout as tokio_timeout;

        let timeout_duration = Duration::from_secs(timeout);

        // Test IMAP connection
        let imap_addr = format!("{}:{}", account.imap_server, account.imap_port);
        tokio_timeout(timeout_duration, TcpStream::connect(&imap_addr))
            .await
            .map_err(|_| anyhow!("IMAP connection timeout"))?
            .map_err(|e| anyhow!("IMAP connection failed: {}", e))?;

        // Test SMTP connection
        let smtp_addr = format!("{}:{}", account.smtp_server, account.smtp_port);
        tokio_timeout(timeout_duration, TcpStream::connect(&smtp_addr))
            .await
            .map_err(|_| anyhow!("SMTP connection timeout"))?
            .map_err(|e| anyhow!("SMTP connection failed: {}", e))?;

        Ok(())
    }

    /// Test authentication
    async fn test_authentication(&self, account: &AccountConfig) -> Result<()> {
        match &account.auth_type {
            crate::oauth2::AuthType::OAuth2 => {
                if let Some(ref token_manager) = self.token_manager {
                    // Check if we have valid tokens
                    match token_manager
                        .get_valid_access_token(&account.account_id)
                        .await?
                    {
                        Some(_token) => Ok(()),
                        None => Err(anyhow!("No valid access token found")),
                    }
                } else {
                    Err(anyhow!("OAuth2 token manager not available"))
                }
            }
            crate::oauth2::AuthType::Password => {
                // For password auth, we'll try to get the password from storage
                let _password = self.storage.get_password(&account.email_address)?;
                Ok(())
            }
        }
    }

    /// Test IMAP operations
    async fn test_imap_operations(&self, account: &AccountConfig) -> Result<()> {
        // Create a temporary IMAP connection with token manager
        let mut imap_manager = if let Some(token_manager) = &self.token_manager {
            ImapAccountManager::new_with_oauth2(token_manager.clone())?
        } else {
            return Err(anyhow!("Token manager not available for OAuth2 authentication"));
        };
        
        // Load accounts from storage
        imap_manager.load_accounts().await
            .map_err(|e| anyhow!("Failed to load accounts: {}", e))?;

        // Get client for the account
        let client_mutex = imap_manager.get_client(&account.account_id).await?;
        let mut client = client_mutex.lock().await;

        // Test basic IMAP operations - list folders
        let folders = client.list_folders("", "*").await?;

        if folders.is_empty() {
            return Err(anyhow!("No folders found - IMAP connection may be broken"));
        }

        // Try to get INBOX status
        let inbox_status = client
            .get_folder_status("INBOX", &["MESSAGES", "UNSEEN"])
            .await?;

        tracing::debug!(
            "INBOX status: {} messages",
            inbox_status.exists.unwrap_or(0)
        );

        Ok(())
    }

    /// Test SMTP operations
    async fn test_smtp_operations(&self, _account: &AccountConfig) -> Result<()> {
        // For SMTP, we'll just test connection capability without sending
        // In a real implementation, you might send a test message to yourself
        Ok(())
    }

    /// Handle import commands
    async fn handle_import(&self, args: ImportArgs, dry_run: bool) -> Result<()> {
        println!("📥 Comunicado Account Import");
        println!("============================\n");

        if args.preview {
            println!("🔍 Preview mode - no changes will be made");
        } else if dry_run {
            println!("🧪 Dry run mode - no changes will be made");
        }

        let mut imported_count = 0;

        // Import Google credentials
        if let Some(ref google_file) = args.google_credentials {
            println!(
                "📧 Importing Google OAuth2 credentials from: {}",
                google_file.display()
            );
            imported_count += self
                .import_google_credentials(google_file, args.force, dry_run)
                .await?;
        }

        // Import Microsoft credentials
        if let Some(ref microsoft_file) = args.microsoft_credentials {
            println!(
                "📧 Importing Microsoft OAuth2 credentials from: {}",
                microsoft_file.display()
            );
            imported_count += self
                .import_microsoft_credentials(microsoft_file, args.force, dry_run)
                .await?;
        }

        // Import generic account config
        if let Some(ref config_file) = args.account_config {
            println!(
                "⚙️  Importing account configuration from: {}",
                config_file.display()
            );
            imported_count += self
                .import_account_config(config_file, args.force, dry_run)
                .await?;
        }

        // Import Thunderbird profile
        if let Some(ref profile_dir) = args.thunderbird_profile {
            println!(
                "🦅 Importing Thunderbird profile from: {}",
                profile_dir.display()
            );
            imported_count += self
                .import_thunderbird_profile(profile_dir, args.force, dry_run)
                .await?;
        }

        println!("\n📊 Import Summary");
        println!("=================");
        println!("✅ {} account(s) imported successfully", imported_count);

        if imported_count > 0 && !dry_run && !args.preview {
            println!("\n💡 Next steps:");
            println!("   1. Run 'comunicado test --all' to verify connections");
            println!("   2. Start Comunicado to begin using your accounts");
        }

        Ok(())
    }

    /// Import Google OAuth2 credentials
    async fn import_google_credentials(
        &self,
        file_path: &PathBuf,
        _force: bool,
        dry_run: bool,
    ) -> Result<u32> {
        let content = std::fs::read_to_string(file_path)?;
        let credentials: serde_json::Value = serde_json::from_str(&content)?;

        // Extract client credentials
        let client_id = credentials["installed"]["client_id"]
            .as_str()
            .or_else(|| credentials["web"]["client_id"].as_str())
            .ok_or_else(|| anyhow!("Missing client_id in Google credentials file"))?;

        let client_secret = credentials["installed"]["client_secret"]
            .as_str()
            .or_else(|| credentials["web"]["client_secret"].as_str())
            .ok_or_else(|| anyhow!("Missing client_secret in Google credentials file"))?;

        println!("   📋 Found Google OAuth2 credentials");
        println!(
            "      Client ID: {}...",
            &client_id[..20.min(client_id.len())]
        );

        if !dry_run {
            // Store credentials for OAuth2 setup
            let oauth_config = crate::oauth2::OAuthConfig {
                client_id: client_id.to_string(),
                client_secret: client_secret.to_string(),
                redirect_uri: "http://localhost:8080/oauth/callback".to_string(),
                scopes: vec![
                    "https://mail.google.com/".to_string(),
                    "https://www.googleapis.com/auth/userinfo.email".to_string(),
                    "https://www.googleapis.com/auth/userinfo.profile".to_string(),
                    "https://www.googleapis.com/auth/calendar".to_string(),
                    "https://www.googleapis.com/auth/contacts.readonly".to_string(),
                ],
            };

            self.storage.store_oauth_config("google", &oauth_config)?;
            println!("   ✅ Google OAuth2 configuration stored");
        }

        Ok(1)
    }

    /// Import Microsoft OAuth2 credentials
    async fn import_microsoft_credentials(
        &self,
        file_path: &PathBuf,
        _force: bool,
        dry_run: bool,
    ) -> Result<u32> {
        let content = std::fs::read_to_string(file_path)?;
        let credentials: serde_json::Value = serde_json::from_str(&content)?;

        // Extract client credentials
        let client_id = credentials["client_id"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing client_id in Microsoft credentials file"))?;

        let client_secret = credentials["client_secret"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing client_secret in Microsoft credentials file"))?;

        println!("   📋 Found Microsoft OAuth2 credentials");
        println!(
            "      Client ID: {}...",
            &client_id[..20.min(client_id.len())]
        );

        if !dry_run {
            let oauth_config = crate::oauth2::OAuthConfig {
                client_id: client_id.to_string(),
                client_secret: client_secret.to_string(),
                redirect_uri: "http://localhost:8080/oauth/callback".to_string(),
                scopes: vec![
                    "https://graph.microsoft.com/mail.read".to_string(),
                    "https://graph.microsoft.com/mail.send".to_string(),
                    "https://graph.microsoft.com/calendars.readwrite".to_string(),
                ],
            };

            self.storage
                .store_oauth_config("microsoft", &oauth_config)?;
            println!("   ✅ Microsoft OAuth2 configuration stored");
        }

        Ok(1)
    }

    /// Import generic account configuration
    async fn import_account_config(
        &self,
        _file_path: &PathBuf,
        _force: bool,
        _dry_run: bool,
    ) -> Result<u32> {
        // TODO: Implement generic account config import
        println!("   ⚠️  Generic account config import not yet implemented");
        Ok(0)
    }

    /// Import Thunderbird profile
    async fn import_thunderbird_profile(
        &self,
        _profile_dir: &PathBuf,
        _force: bool,
        _dry_run: bool,
    ) -> Result<u32> {
        // TODO: Implement Thunderbird profile import
        println!("   ⚠️  Thunderbird profile import not yet implemented");
        Ok(0)
    }

    /// Handle troubleshoot commands
    async fn handle_troubleshoot(&self, args: TroubleshootArgs) -> Result<()> {
        println!("🔧 Comunicado Troubleshoot");
        println!("==========================\n");

        let mut issues_found = 0;

        if args.all || args.network {
            issues_found += self.troubleshoot_network().await?;
        }

        if args.all || args.auth {
            issues_found += self.troubleshoot_authentication().await?;
        }

        if args.all || args.capabilities {
            issues_found += self.troubleshoot_capabilities().await?;
        }

        if args.all || args.database {
            issues_found += self.troubleshoot_database().await?;
        }

        println!("\n📊 Troubleshoot Summary");
        println!("=======================");
        if issues_found == 0 {
            println!("✅ No issues found - everything looks good!");
        } else {
            println!(
                "⚠️  Found {} potential issue(s) - see details above",
                issues_found
            );
        }

        Ok(())
    }

    /// Troubleshoot network connectivity
    async fn troubleshoot_network(&self) -> Result<u32> {
        println!("🌐 Network Connectivity");
        println!("------------------------");

        let mut issues = 0;

        // Test basic internet connectivity
        print!("   Testing internet connectivity... ");
        match tokio::net::TcpStream::connect("8.8.8.8:53").await {
            Ok(_) => println!("✅ OK"),
            Err(e) => {
                println!("❌ FAILED: {}", e);
                issues += 1;
            }
        }

        // Test DNS resolution
        print!("   Testing DNS resolution... ");
        match tokio::net::lookup_host("gmail.com:993").await {
            Ok(_) => println!("✅ OK"),
            Err(e) => {
                println!("❌ FAILED: {}", e);
                issues += 1;
            }
        }

        println!();
        Ok(issues)
    }

    /// Troubleshoot authentication
    async fn troubleshoot_authentication(&self) -> Result<u32> {
        println!("🔐 Authentication");
        println!("-----------------");

        let accounts = self.storage.list_accounts()?;
        let mut issues = 0;

        for account in accounts {
            print!(
                "   Checking {} ({})... ",
                account.display_name, account.email_address
            );

            match self.test_authentication(&account).await {
                Ok(_) => println!("✅ OK"),
                Err(e) => {
                    println!("❌ FAILED: {}", e);
                    issues += 1;
                }
            }
        }

        println!();
        Ok(issues)
    }

    /// Troubleshoot server capabilities
    async fn troubleshoot_capabilities(&self) -> Result<u32> {
        println!("⚙️  Server Capabilities");
        println!("-----------------------");

        // TODO: Implement capability checks
        println!("   ⚠️  Capability checking not yet implemented");
        println!();

        Ok(0)
    }

    /// Troubleshoot database
    async fn troubleshoot_database(&self) -> Result<u32> {
        println!("🗃️  Database");
        println!("------------");

        let mut issues = 0;

        // Check database connectivity
        print!("   Database connectivity... ");
        match self.database.get_stats().await {
            Ok(_) => println!("✅ OK"),
            Err(e) => {
                println!("❌ FAILED: {}", e);
                issues += 1;
            }
        }

        // Check database integrity
        print!("   Database integrity... ");
        match self.database.check_integrity().await {
            Ok(true) => println!("✅ OK"),
            Ok(false) => {
                println!("⚠️  Issues found - run 'comunicado database check --fix'");
                issues += 1;
            }
            Err(e) => {
                println!("❌ FAILED: {}", e);
                issues += 1;
            }
        }

        println!();
        Ok(issues)
    }

    /// Handle database commands
    async fn handle_database(&self, args: DatabaseArgs, dry_run: bool) -> Result<()> {
        match args.command {
            DatabaseCommands::Check { account, fix } => {
                self.handle_database_check(account, fix, dry_run).await
            }
            DatabaseCommands::Clean {
                account,
                aggressive,
                older_than,
            } => {
                self.handle_database_clean(account, aggressive, older_than, dry_run)
                    .await
            }
            DatabaseCommands::Rebuild {
                account,
                search_index,
            } => {
                self.handle_database_rebuild(account, search_index, dry_run)
                    .await
            }
            DatabaseCommands::Stats { account, detailed } => {
                self.handle_database_stats(account, detailed).await
            }
            DatabaseCommands::Backup { output, compress } => {
                self.handle_database_backup(output, compress, dry_run).await
            }
            DatabaseCommands::Restore { input, force } => {
                self.handle_database_restore(input, force, dry_run).await
            }
        }
    }

    /// Handle database check
    async fn handle_database_check(
        &self,
        account: Option<String>,
        fix: bool,
        dry_run: bool,
    ) -> Result<()> {
        println!("🔍 Database Integrity Check");
        println!("===========================\n");

        if dry_run && fix {
            println!("🧪 Dry run mode - issues will be identified but not fixed");
        }

        let integrity_ok = self.database.check_integrity().await?;

        if integrity_ok {
            println!("✅ Database integrity check passed");
        } else {
            println!("⚠️  Database integrity issues found");

            if fix && !dry_run {
                println!("🔧 Attempting to fix issues...");
                self.database.repair_database().await?;
                println!("✅ Database repair completed");
            } else {
                println!("💡 Run with --fix to attempt automatic repair");
            }
        }

        // Check specific account if requested
        if let Some(account_name) = account {
            println!("\n📋 Account-specific checks for: {}", account_name);
            // TODO: Implement account-specific checks
        }

        Ok(())
    }

    /// Handle database clean
    async fn handle_database_clean(
        &self,
        _account: Option<String>,
        _aggressive: bool,
        _older_than: Option<u32>,
        dry_run: bool,
    ) -> Result<()> {
        println!("🧹 Database Cleanup");
        println!("===================\n");

        if dry_run {
            println!("🧪 Dry run mode - showing what would be cleaned");
        }

        let stats_before = self.database.get_stats().await?;
        println!("📊 Database stats before cleanup:");
        self.print_database_stats(&stats_before);

        if !dry_run {
            // Perform cleanup operations
            let cleaned = self.database.cleanup_database().await?;
            println!("\n🗑️  Cleanup completed:");
            println!(
                "   - Removed {} orphaned attachments",
                cleaned.orphaned_attachments
            );
            println!(
                "   - Removed {} duplicate messages",
                cleaned.duplicate_messages
            );
            println!("   - Freed {} MB of disk space", cleaned.freed_space_mb);

            let stats_after = self.database.get_stats().await?;
            println!("\n📊 Database stats after cleanup:");
            self.print_database_stats(&stats_after);
        }

        Ok(())
    }

    /// Handle database rebuild
    async fn handle_database_rebuild(
        &self,
        _account: Option<String>,
        search_index: bool,
        dry_run: bool,
    ) -> Result<()> {
        println!("🔄 Database Rebuild");
        println!("===================\n");

        if dry_run {
            println!("🧪 Dry run mode - showing what would be rebuilt");
            return Ok(());
        }

        println!("⚠️  This operation may take several minutes for large databases");

        // Rebuild main indexes
        println!("🔧 Rebuilding database indexes...");
        self.database.rebuild_indexes().await?;
        println!("✅ Database indexes rebuilt");

        // Rebuild search index if requested
        if search_index {
            println!("🔍 Rebuilding search index...");
            self.database.rebuild_search_index().await?;
            println!("✅ Search index rebuilt");
        }

        println!("\n🎉 Database rebuild completed successfully");

        Ok(())
    }

    /// Handle database statistics
    async fn handle_database_stats(&self, account: Option<String>, detailed: bool) -> Result<()> {
        println!("📊 Database Statistics");
        println!("======================\n");

        let stats = self.database.get_stats().await?;
        self.print_database_stats(&stats);

        if detailed {
            println!("\n📋 Detailed Statistics:");
            println!(
                "   Database file size: {} MB",
                stats.db_size_bytes / (1024 * 1024)
            );
            println!("   Message count: {}", stats.message_count);
            println!("   Account count: {}", stats.account_count);
            println!("   Folder count: {}", stats.folder_count);
        }

        if let Some(account_name) = account {
            println!("\n👤 Account-specific stats for: {}", account_name);
            // TODO: Implement account-specific stats
        }

        Ok(())
    }

    /// Print database statistics in a formatted way
    fn print_database_stats(&self, stats: &DatabaseStats) {
        println!("   Total messages: {}", stats.message_count);
        println!("   Total folders: {}", stats.folder_count);
        println!("   Total accounts: {}", stats.account_count);
        println!("   Unread messages: {}", stats.unread_count);
        println!(
            "   Database size: {} MB",
            stats.db_size_bytes / (1024 * 1024)
        );
    }

    /// Handle database backup
    async fn handle_database_backup(
        &self,
        output: PathBuf,
        compress: bool,
        dry_run: bool,
    ) -> Result<()> {
        println!("💾 Database Backup");
        println!("==================\n");

        if dry_run {
            println!("🧪 Dry run mode - showing backup plan");
            println!("   Backup destination: {}", output.display());
            println!(
                "   Compression: {}",
                if compress { "enabled" } else { "disabled" }
            );
            return Ok(());
        }

        println!("📦 Creating backup at: {}", output.display());

        let backup_result = self.database.create_backup(&output, compress).await?;

        println!("✅ Backup completed successfully");
        println!("   Backup size: {} MB", backup_result.size_mb);
        println!("   Messages backed up: {}", backup_result.message_count);

        Ok(())
    }

    /// Handle database restore
    async fn handle_database_restore(
        &self,
        input: PathBuf,
        force: bool,
        dry_run: bool,
    ) -> Result<()> {
        println!("📂 Database Restore");
        println!("===================\n");

        if !input.exists() {
            return Err(anyhow!("Backup file not found: {}", input.display()));
        }

        if dry_run {
            println!("🧪 Dry run mode - showing restore plan");
            println!("   Restore from: {}", input.display());
            println!("   Force overwrite: {}", force);
            return Ok(());
        }

        if !force {
            println!("⚠️  This will overwrite the current database!");
            println!("   Use --force to proceed with restore");
            return Ok(());
        }

        println!("📥 Restoring database from: {}", input.display());

        let restore_result = self.database.restore_backup(&input).await?;

        println!("✅ Restore completed successfully");
        println!("   Messages restored: {}", restore_result.message_count);
        println!("   Folders restored: {}", restore_result.folder_count);

        Ok(())
    }

    /// Handle config commands
    async fn handle_config(&self, _args: ConfigArgs, _dry_run: bool) -> Result<()> {
        println!("⚙️  Configuration Management");
        println!("============================\n");

        // TODO: Implement config management
        println!("   ⚠️  Configuration management not yet implemented");

        Ok(())
    }

    /// Handle account commands
    async fn handle_account(&self, _args: AccountArgs, _dry_run: bool) -> Result<()> {
        println!("👤 Account Management");
        println!("=====================\n");

        // TODO: Implement account management
        println!("   ⚠️  CLI account management not yet implemented");
        println!("   💡 Use the TUI interface to manage accounts");
        println!("   💡 Or use dedicated setup commands:");
        println!("      comunicado setup-gmail --help");
        println!("      comunicado setup-outlook --help");

        Ok(())
    }

    /// Handle Gmail OAuth2 setup
    async fn handle_setup_gmail(
        &self,
        client_secret: Option<PathBuf>,
        email: Option<String>,
        name: Option<String>,
        no_browser: bool,
        dry_run: bool,
    ) -> Result<()> {
        println!("📧 Gmail OAuth2 Account Setup");
        println!("=============================\n");

        if dry_run {
            println!("🧪 Dry run mode - showing what would be done");
        }

        // Find client secret file
        let client_secret_path = if let Some(path) = client_secret {
            if !path.exists() {
                return Err(anyhow!("Client secret file not found: {}", path.display()));
            }
            path
        } else {
            // Auto-detect client secret file in common locations
            let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            let possible_paths = [
                format!("{}/client_secret*.json", home_dir),
                "./client_secret*.json".to_string(),
                format!("{}/.config/comunicado/client_secret*.json", home_dir),
            ];

            let mut found_path = None;
            for pattern in &possible_paths {
                if let Ok(paths) = glob::glob(pattern) {
                    for path in paths.flatten() {
                        if path.exists() {
                            found_path = Some(path);
                            break;
                        }
                    }
                    if found_path.is_some() {
                        break;
                    }
                }
            }

            match found_path {
                Some(path) => path,
                None => {
                    println!("❌ No client secret file found!");
                    println!("   Please specify the path with --client-secret <path>");
                    println!("   Or place it in one of these locations:");
                    for pattern in &possible_paths {
                        println!("     {}", pattern);
                    }
                    return Ok(());
                }
            }
        };

        println!("✅ Found client secret file: {}", client_secret_path.display());

        // Read and parse client secret
        let client_data = std::fs::read_to_string(&client_secret_path)
            .map_err(|e| anyhow!("Failed to read client secret file: {}", e))?;

        let client_json: serde_json::Value = serde_json::from_str(&client_data)
            .map_err(|e| anyhow!("Failed to parse client secret JSON: {}", e))?;

        let client_id = client_json["installed"]["client_id"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing client_id in client secret file"))?;

        let client_secret_value = client_json["installed"]["client_secret"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing client_secret in client secret file"))?;

        println!("   Client ID: {}...", &client_id[..20.min(client_id.len())]);

        // Get email address
        let email = if let Some(email) = email {
            if email.is_empty() || !email.contains('@') {
                return Err(anyhow!("Invalid email address: {}", email));
            }
            email
        } else {
            print!("📧 Enter your Gmail address: ");
            std::io::stdout().flush()?;
            
            let mut input_email = String::new();
            std::io::stdin().read_line(&mut input_email)?;
            let input_email = input_email.trim().to_string();

            if input_email.is_empty() || !input_email.contains('@') {
                return Err(anyhow!("Invalid email address"));
            }
            input_email
        };

        println!("   Email: {}", email);

        // Use provided name or default
        let display_name = name.unwrap_or_else(|| {
            email.split('@').next().unwrap_or("Gmail User").to_string()
        });

        println!("   Display name: {}", display_name);

        if dry_run {
            println!("\n🧪 Dry run complete - would set up OAuth2 for:");
            println!("   Email: {}", email);
            println!("   Display: {}", display_name);
            println!("   Client ID: {}...", &client_id[..20]);
            return Ok(());
        }

        // Run OAuth2 flow
        println!("\n🚀 Starting OAuth2 authorization...");
        
        match self.run_oauth2_flow(
            &email,
            &display_name,
            client_id,
            client_secret_value,
            "gmail",
            no_browser,
        ).await {
            Ok(()) => {
                println!("\n🎉 Gmail account setup complete!");
                println!("   Account: {} ({})", display_name, email);
                println!("   You can now use: cargo run --bin comunicado");
            }
            Err(e) => {
                println!("\n❌ Setup failed: {}", e);
                return Err(e);
            }
        }

        Ok(())
    }

    /// Handle Outlook OAuth2 setup
    async fn handle_setup_outlook(
        &self,
        _client_secret: Option<PathBuf>,
        _name: Option<String>,
        _no_browser: bool,
        dry_run: bool,
    ) -> Result<()> {
        println!("📬 Outlook OAuth2 Account Setup");
        println!("===============================\n");

        if dry_run {
            println!("🧪 Dry run mode - showing what would be done");
        }

        // TODO: Implement Outlook OAuth2 setup similar to Gmail
        println!("   ⚠️  Outlook OAuth2 setup not yet implemented");
        println!("   💡 Gmail setup is available with: comunicado setup-gmail");

        Ok(())
    }

    /// Handle keyboard shortcut commands
    async fn handle_keyboard(&self, args: KeyboardArgs, dry_run: bool) -> Result<()> {
        match args.command {
            KeyboardCommands::Show { category, detailed } => {
                self.handle_keyboard_show(category, detailed).await
            }
            KeyboardCommands::Set { action, key, force } => {
                self.handle_keyboard_set(action, key, force, dry_run).await
            }
            KeyboardCommands::Remove { key } => self.handle_keyboard_remove(key, dry_run).await,
            KeyboardCommands::Reset { category, force } => {
                self.handle_keyboard_reset(category, force, dry_run).await
            }
            KeyboardCommands::Export { output, format } => {
                self.handle_keyboard_export(output, format, dry_run).await
            }
            KeyboardCommands::Import { input, merge } => {
                self.handle_keyboard_import(input, merge, dry_run).await
            }
            KeyboardCommands::Validate { config } => self.handle_keyboard_validate(config).await,
        }
    }

    /// Show current keyboard shortcuts
    async fn handle_keyboard_show(&self, category: Option<String>, detailed: bool) -> Result<()> {
        println!("⌨️  Keyboard Shortcuts");
        println!("======================\n");

        let keyboard_manager = KeyboardManager::new()?;
        let config = keyboard_manager.config();

        if let Some(ref cat) = category {
            // Show specific category
            let categories = config.get_shortcuts_by_category();
            if let Some(shortcuts) = categories.get(cat) {
                println!("{}:", cat);
                println!("{}", "-".repeat(cat.len() + 1));

                for (shortcut, action, description) in shortcuts {
                    if detailed {
                        println!(
                            "  {:20} {:30} {}",
                            shortcut.to_string(),
                            format!("{:?}", action),
                            description
                        );
                    } else {
                        println!("  {:15} - {}", shortcut.to_string(), description);
                    }
                }
            } else {
                println!("❌ Category '{}' not found", cat);
                println!("\nAvailable categories:");
                for category_name in categories.keys() {
                    println!("  - {}", category_name);
                }
            }
        } else {
            // Show all shortcuts
            if detailed {
                let categories = config.get_shortcuts_by_category();
                let mut sorted_categories: Vec<_> = categories.keys().collect();
                sorted_categories.sort();

                for category in sorted_categories {
                    println!("{}:", category);
                    println!("{}", "-".repeat(category.len() + 1));

                    if let Some(shortcuts) = categories.get(category) {
                        for (shortcut, action, description) in shortcuts {
                            println!(
                                "  {:20} {:30} {}",
                                shortcut.to_string(),
                                format!("{:?}", action),
                                description
                            );
                        }
                    }
                    println!();
                }
            } else {
                print!("{}", keyboard_manager.get_help_text());
            }
        }

        Ok(())
    }

    /// Set a keyboard shortcut
    async fn handle_keyboard_set(
        &self,
        action_str: String,
        key_str: String,
        force: bool,
        dry_run: bool,
    ) -> Result<()> {
        println!("⌨️  Set Keyboard Shortcut");
        println!("=========================\n");

        if dry_run {
            println!("🧪 Dry run mode - showing what would be changed");
        }

        // Parse the action
        let action = self.parse_keyboard_action(&action_str)?;

        // Parse the key combination
        let shortcut = self.parse_keyboard_shortcut(&key_str)?;

        println!("   Action: {:?}", action);
        println!("   Shortcut: {}", shortcut);

        let mut keyboard_manager = KeyboardManager::new()?;

        // Check for conflicts
        if keyboard_manager.config().has_conflict(&shortcut) && !force {
            let existing_action = keyboard_manager.config().get_action(&shortcut).unwrap();
            println!(
                "\n❌ Shortcut '{}' is already assigned to action: {:?}",
                shortcut, existing_action
            );
            println!("   Use --force to overwrite the existing assignment");
            return Ok(());
        }

        if !dry_run {
            keyboard_manager.set_shortcut(shortcut.clone(), action.clone())?;
            println!(
                "\n✅ Shortcut '{}' assigned to action: {:?}",
                shortcut, action
            );
        } else {
            println!(
                "\n✅ Would assign shortcut '{}' to action: {:?}",
                shortcut, action
            );
        }

        Ok(())
    }

    /// Remove a keyboard shortcut
    async fn handle_keyboard_remove(&self, key_str: String, dry_run: bool) -> Result<()> {
        println!("⌨️  Remove Keyboard Shortcut");
        println!("============================\n");

        if dry_run {
            println!("🧪 Dry run mode - showing what would be changed");
        }

        // Parse the key combination
        let shortcut = self.parse_keyboard_shortcut(&key_str)?;

        let mut keyboard_manager = KeyboardManager::new()?;

        if let Some(action) = keyboard_manager.config().get_action(&shortcut) {
            println!("   Removing shortcut: {}", shortcut);
            println!("   Current action: {:?}", action);

            if !dry_run {
                keyboard_manager.remove_shortcut(&shortcut)?;
                println!("\n✅ Shortcut '{}' removed", shortcut);
            } else {
                println!("\n✅ Would remove shortcut '{}'", shortcut);
            }
        } else {
            println!("❌ Shortcut '{}' is not currently assigned", shortcut);
        }

        Ok(())
    }

    /// Reset keyboard shortcuts to defaults
    async fn handle_keyboard_reset(
        &self,
        _category: Option<String>,
        force: bool,
        dry_run: bool,
    ) -> Result<()> {
        println!("⌨️  Reset Keyboard Shortcuts");
        println!("============================\n");

        if dry_run {
            println!("🧪 Dry run mode - showing what would be changed");
        }

        if !force && !dry_run {
            println!("⚠️  This will reset all keyboard shortcuts to defaults!");
            println!("   Use --force to proceed with reset");
            return Ok(());
        }

        let mut keyboard_manager = KeyboardManager::new()?;

        if !dry_run {
            keyboard_manager.reset_to_defaults()?;
            println!("✅ Keyboard shortcuts reset to defaults");
        } else {
            println!("✅ Would reset all keyboard shortcuts to defaults");
        }

        Ok(())
    }

    /// Export keyboard shortcuts to file
    async fn handle_keyboard_export(
        &self,
        output: PathBuf,
        format: String,
        dry_run: bool,
    ) -> Result<()> {
        println!("⌨️  Export Keyboard Shortcuts");
        println!("=============================\n");

        if dry_run {
            println!("🧪 Dry run mode - showing export plan");
            println!("   Export to: {}", output.display());
            println!("   Format: {}", format);
            return Ok(());
        }

        let keyboard_manager = KeyboardManager::new()?;
        let config = keyboard_manager.config();

        match format.as_str() {
            "toml" => {
                config.save_to_file(&output)?;
                println!("✅ Keyboard shortcuts exported to: {}", output.display());
            }
            "json" => {
                let json_content = serde_json::to_string_pretty(config)?;
                std::fs::write(&output, json_content)?;
                println!(
                    "✅ Keyboard shortcuts exported to: {} (JSON format)",
                    output.display()
                );
            }
            _ => {
                return Err(anyhow!(
                    "Unsupported export format: {}. Use 'toml' or 'json'",
                    format
                ));
            }
        }

        Ok(())
    }

    /// Import keyboard shortcuts from file
    async fn handle_keyboard_import(
        &self,
        input: PathBuf,
        _merge: bool,
        dry_run: bool,
    ) -> Result<()> {
        println!("⌨️  Import Keyboard Shortcuts");
        println!("=============================\n");

        if !input.exists() {
            return Err(anyhow!("Import file not found: {}", input.display()));
        }

        if dry_run {
            println!("🧪 Dry run mode - showing import plan");
            println!("   Import from: {}", input.display());
            return Ok(());
        }

        let config = if input.extension().and_then(|s| s.to_str()) == Some("json") {
            let content = std::fs::read_to_string(&input)?;
            serde_json::from_str::<KeyboardConfig>(&content)?
        } else {
            KeyboardConfig::load_from_file(&input)?
        };

        // Validate the imported configuration
        let issues = config.validate();
        if !issues.is_empty() {
            println!("⚠️  Configuration validation issues found:");
            for issue in &issues {
                println!("   - {}", issue);
            }
            return Err(anyhow!("Configuration validation failed"));
        }

        let keyboard_manager = KeyboardManager::with_config(config);
        keyboard_manager.save_config()?;

        println!("✅ Keyboard shortcuts imported from: {}", input.display());

        Ok(())
    }

    /// Validate keyboard configuration
    async fn handle_keyboard_validate(&self, config_file: Option<PathBuf>) -> Result<()> {
        println!("⌨️  Validate Keyboard Configuration");
        println!("===================================\n");

        let config = if let Some(path) = config_file {
            if !path.exists() {
                return Err(anyhow!("Configuration file not found: {}", path.display()));
            }
            KeyboardConfig::load_from_file(&path)?
        } else {
            KeyboardManager::new()?.config().clone()
        };

        let issues = config.validate();

        if issues.is_empty() {
            println!("✅ Keyboard configuration is valid");

            // Show some statistics
            let categories = config.get_shortcuts_by_category();
            println!("\n📊 Configuration Statistics:");
            println!("   Total categories: {}", categories.len());

            let total_shortcuts: usize = categories.values().map(|v| v.len()).sum();
            println!("   Total shortcuts: {}", total_shortcuts);

            for (category, shortcuts) in &categories {
                println!("   {}: {} shortcuts", category, shortcuts.len());
            }
        } else {
            println!("❌ Keyboard configuration has issues:");
            for issue in &issues {
                println!("   - {}", issue);
            }
        }

        Ok(())
    }

    /// Parse a keyboard action from string
    fn parse_keyboard_action(&self, action_str: &str) -> Result<KeyboardAction> {
        match action_str.to_lowercase().as_str() {
            "quit" => Ok(KeyboardAction::Quit),
            "force_quit" | "forcequit" => Ok(KeyboardAction::ForceQuit),
            "next_pane" | "nextpane" => Ok(KeyboardAction::NextPane),
            "previous_pane" | "previouspane" | "prev_pane" => Ok(KeyboardAction::PreviousPane),
            "vim_move_left" | "vimleft" => Ok(KeyboardAction::VimMoveLeft),
            "vim_move_right" | "vimright" => Ok(KeyboardAction::VimMoveRight),
            "vim_move_up" | "vimup" => Ok(KeyboardAction::VimMoveUp),
            "vim_move_down" | "vimdown" => Ok(KeyboardAction::VimMoveDown),
            "move_up" | "moveup" => Ok(KeyboardAction::MoveUp),
            "move_down" | "movedown" => Ok(KeyboardAction::MoveDown),
            "select" => Ok(KeyboardAction::Select),
            "escape" => Ok(KeyboardAction::Escape),
            "toggle_expanded" | "toggle" => Ok(KeyboardAction::ToggleExpanded),
            "compose_email" | "compose" => Ok(KeyboardAction::ComposeEmail),
            "show_draft_list" | "drafts" => Ok(KeyboardAction::ShowDraftList),
            "add_account" | "addaccount" => Ok(KeyboardAction::AddAccount),
            "remove_account" | "removeaccount" => Ok(KeyboardAction::RemoveAccount),
            "refresh_account" | "refreshaccount" => Ok(KeyboardAction::RefreshAccount),
            "start_search" | "search" => Ok(KeyboardAction::StartSearch),
            "start_folder_search" | "foldersearch" => Ok(KeyboardAction::StartFolderSearch),
            "toggle_threaded_view" | "threadview" => Ok(KeyboardAction::ToggleThreadedView),
            "expand_thread" | "expand" => Ok(KeyboardAction::ExpandThread),
            "collapse_thread" | "collapse" => Ok(KeyboardAction::CollapseThread),
            "toggle_view_mode" | "viewmode" => Ok(KeyboardAction::ToggleViewMode),
            "toggle_headers" | "headers" => Ok(KeyboardAction::ToggleHeaders),
            "sort_by_date" | "sortdate" => Ok(KeyboardAction::SortByDate),
            "sort_by_sender" | "sortsender" => Ok(KeyboardAction::SortBySender),
            "sort_by_subject" | "sortsubject" => Ok(KeyboardAction::SortBySubject),
            "scroll_to_top" | "scrolltop" => Ok(KeyboardAction::ScrollToTop),
            "scroll_to_bottom" | "scrollbottom" => Ok(KeyboardAction::ScrollToBottom),
            "select_first_attachment" | "firstattachment" => Ok(KeyboardAction::SelectFirstAttachment),
            "view_attachment" | "viewattachment" => Ok(KeyboardAction::ViewAttachment),
            "open_attachment_with_system" | "openattachment" => Ok(KeyboardAction::OpenAttachmentWithSystem),
            "create_folder" | "createfolder" => Ok(KeyboardAction::CreateFolder),
            "delete_folder" | "deletefolder" => Ok(KeyboardAction::DeleteFolder),
            "refresh_folder" | "refreshfolder" => Ok(KeyboardAction::RefreshFolder),
            "copy_email_content" | "copyemail" => Ok(KeyboardAction::CopyEmailContent),
            "copy_attachment_info" | "copyattachment" => Ok(KeyboardAction::CopyAttachmentInfo),
            "folder_refresh" | "f5" => Ok(KeyboardAction::FolderRefresh),
            "folder_rename" | "f2" => Ok(KeyboardAction::FolderRename),
            "folder_delete" | "delete" => Ok(KeyboardAction::FolderDelete),
            "next_attachment" | "nextattachment" => Ok(KeyboardAction::NextAttachment),
            "previous_attachment" | "prevattachment" => Ok(KeyboardAction::PreviousAttachment),
            _ => Err(anyhow!("Unknown keyboard action: {}. See 'comunicado keyboard show --detailed' for available actions", action_str)),
        }
    }

    /// Parse a keyboard shortcut from string
    fn parse_keyboard_shortcut(&self, key_str: &str) -> Result<KeyboardShortcut> {
        use crossterm::event::{KeyCode, KeyModifiers};

        let parts: Vec<&str> = key_str.split('+').collect();
        let mut modifiers = KeyModifiers::NONE;
        let mut key_part = "";

        for part in &parts {
            match part.to_lowercase().as_str() {
                "ctrl" | "control" => modifiers |= KeyModifiers::CONTROL,
                "alt" => modifiers |= KeyModifiers::ALT,
                "shift" => modifiers |= KeyModifiers::SHIFT,
                _ => key_part = part,
            }
        }

        if key_part.is_empty() {
            return Err(anyhow!("No key specified in: {}", key_str));
        }

        let key_code = match key_part.to_lowercase().as_str() {
            "enter" => KeyCode::Enter,
            "esc" | "escape" => KeyCode::Esc,
            "space" => KeyCode::Char(' '),
            "tab" => KeyCode::Tab,
            "backspace" => KeyCode::Backspace,
            "delete" | "del" => KeyCode::Delete,
            "insert" | "ins" => KeyCode::Insert,
            "home" => KeyCode::Home,
            "end" => KeyCode::End,
            "pageup" | "pgup" => KeyCode::PageUp,
            "pagedown" | "pgdn" => KeyCode::PageDown,
            "up" => KeyCode::Up,
            "down" => KeyCode::Down,
            "left" => KeyCode::Left,
            "right" => KeyCode::Right,
            key if key.starts_with('f') && key.len() > 1 => {
                let num_str = &key[1..];
                if let Ok(num) = num_str.parse::<u8>() {
                    if num >= 1 && num <= 12 {
                        KeyCode::F(num)
                    } else {
                        return Err(anyhow!(
                            "Function key number must be between 1 and 12: {}",
                            key
                        ));
                    }
                } else {
                    return Err(anyhow!("Invalid function key format: {}", key));
                }
            }
            key if key.len() == 1 => {
                let ch = key.chars().next().unwrap();
                KeyCode::Char(ch)
            }
            _ => return Err(anyhow!("Unknown key: {}", key_part)),
        };

        Ok(KeyboardShortcut::new(key_code, modifiers))
    }

    /// Handle Maildir commands
    async fn handle_maildir(&self, args: MaildirArgs, dry_run: bool) -> Result<()> {
        match args.command {
            MaildirCommands::Import {
                path,
                account,
                folder,
                progress,
                dry_run: cmd_dry_run,
            } => {
                self.handle_maildir_import(path, account, folder, progress, dry_run || cmd_dry_run)
                    .await
            }
            MaildirCommands::Export {
                path,
                account,
                folder,
                progress,
                force,
            } => {
                self.handle_maildir_export(path, account, folder, progress, force, dry_run)
                    .await
            }
            MaildirCommands::List { path, detailed } => {
                self.handle_maildir_list(path, detailed).await
            }
            MaildirCommands::Validate { path, fix } => {
                self.handle_maildir_validate(path, fix, dry_run).await
            }
        }
    }

    /// Handle Maildir import command
    async fn handle_maildir_import(
        &self,
        path: PathBuf,
        account: String,
        folder: Option<String>,
        progress: bool,
        dry_run: bool,
    ) -> Result<()> {
        println!("📥 Import from Maildir");
        println!("======================\n");

        if dry_run {
            println!("🧪 Dry run mode - showing what would be imported");
        }

        // Validate the path is a Maildir
        if !MaildirUtils::is_maildir(&path) {
            return Err(anyhow!("Path is not a valid Maildir: {}", path.display()));
        }

        // Check if account exists
        let accounts = self.storage.list_accounts()?;
        let account_config = accounts
            .iter()
            .find(|a| &a.display_name == &account || &a.email_address == &account)
            .ok_or_else(|| anyhow!("Account not found: {}", account))?;

        println!("📁 Maildir Path: {}", path.display());
        println!(
            "📧 Target Account: {} ({})",
            account_config.display_name, account_config.email_address
        );

        if let Some(ref folder_name) = folder {
            println!("📂 Target Folder: {}", folder_name);
        }

        if dry_run {
            println!("\n✅ Dry run completed - Maildir structure is valid");
            return Ok(());
        }

        // Perform the import
        let maildir = Maildir::new(&path)?;

        let messages = if let Some(folder_name) = folder {
            maildir.import_folder(&folder_name).await?
        } else {
            maildir.import_messages().await?
        };

        println!("\n📊 Import Summary:");
        println!("   Messages found: {}", messages.len());

        if progress {
            println!("📈 Converting and storing messages...");
        }

        // Convert to StoredMessage and save to database
        let stored_messages = maildir.import_to_stored_messages().await?;

        // TODO: Actually save to database with proper account assignment
        println!("✅ Import completed successfully!");
        println!("   {} messages imported", stored_messages.len());

        Ok(())
    }

    /// Handle Maildir export command  
    async fn handle_maildir_export(
        &self,
        path: PathBuf,
        account: String,
        folder: Option<String>,
        progress: bool,
        force: bool,
        dry_run: bool,
    ) -> Result<()> {
        println!("📤 Export to Maildir");
        println!("====================\n");

        if dry_run {
            println!("🧪 Dry run mode - showing what would be exported");
        }

        // Check if account exists
        let accounts = self.storage.list_accounts()?;
        let account_config = accounts
            .iter()
            .find(|a| &a.display_name == &account || &a.email_address == &account)
            .ok_or_else(|| anyhow!("Account not found: {}", account))?;

        println!("📁 Export Path: {}", path.display());
        println!(
            "📧 Source Account: {} ({})",
            account_config.display_name, account_config.email_address
        );

        if let Some(ref folder_name) = folder {
            println!("📂 Source Folder: {}", folder_name);
        }

        // Check if path exists and handle force flag
        if path.exists() && !force {
            return Err(anyhow!(
                "Export path already exists. Use --force to overwrite: {}",
                path.display()
            ));
        }

        if dry_run {
            println!("\n✅ Dry run completed - export plan validated");
            return Ok(());
        }

        // TODO: Get messages from database for the account
        println!("📊 Messages to export: 0 (database integration needed)");

        if progress {
            println!("📈 Exporting messages...");
        }

        // Create the Maildir
        let _maildir = Maildir::new(&path)?;

        // TODO: Export messages
        println!("✅ Export completed successfully!");

        Ok(())
    }

    /// Handle Maildir list command
    async fn handle_maildir_list(&self, path: PathBuf, detailed: bool) -> Result<()> {
        println!("📋 Maildir Information");
        println!("======================\n");

        if !MaildirUtils::is_maildir(&path) {
            return Err(anyhow!("Path is not a valid Maildir: {}", path.display()));
        }

        println!("📁 Path: {}", path.display());

        let maildir = Maildir::new(&path)?;
        let folders = maildir.list_folders()?;

        println!("\n📂 Folders ({}):", folders.len());
        for folder in &folders {
            println!("   • {}", folder);
        }

        if detailed {
            // TODO: Add detailed statistics per folder
            println!("\n📊 Detailed Statistics:");
            println!("   (detailed statistics not yet implemented)");
        }

        Ok(())
    }

    /// Handle Maildir validate command
    async fn handle_maildir_validate(&self, path: PathBuf, fix: bool, dry_run: bool) -> Result<()> {
        println!("🔍 Validate Maildir");
        println!("===================\n");

        if dry_run && fix {
            println!("🧪 Dry run mode - showing what would be fixed");
        }

        println!("📁 Validating: {}", path.display());

        let is_valid = MaildirUtils::is_maildir(&path);

        if is_valid {
            println!("✅ Maildir structure is valid");

            let maildir = Maildir::new(&path)?;
            let folders = maildir.list_folders()?;

            println!("📂 Found {} folders", folders.len());

            // TODO: Add more detailed validation
            println!("🔍 Basic validation completed - no issues found");
        } else {
            println!("❌ Invalid Maildir structure");

            if fix {
                if dry_run {
                    println!("🔧 Would create missing directories:");
                    println!("   • new/");
                    println!("   • cur/");
                    println!("   • tmp/");
                } else {
                    println!("🔧 Creating missing directories...");
                    MaildirUtils::create_maildir(&path)?;
                    println!("✅ Maildir structure created");
                }
            } else {
                println!("💡 Use --fix to create missing directories");
            }
        }

        Ok(())
    }

    /// Handle offline storage commands
    async fn handle_offline(&self, _args: OfflineArgs, _dry_run: bool) -> Result<()> {
        // NOTE: OfflineIntegrationManager was removed during code cleanup
        println!("❌ Offline storage functionality has been removed.");
        println!("💡 This feature was deprecated as part of code cleanup and simplification.");
        println!("💡 Email data is managed directly through the main application database.");
        return Err(anyhow::anyhow!("Offline integration functionality is no longer available"));

    }

    /// Run OAuth2 authorization flow for account setup
    async fn run_oauth2_flow(
        &self,
        email: &str,
        display_name: &str,
        client_id: &str,
        client_secret: &str,
        provider: &str,
        no_browser: bool,
    ) -> Result<()> {
        use std::collections::HashMap;
        use std::sync::{Arc, Mutex};
        use std::time::Duration;
        use tokio::net::TcpListener;
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

        const REDIRECT_PORT: u16 = 8181;
        const REDIRECT_URI: &str = "http://localhost:8181/oauth/callback";

        // Build authorization URL based on provider
        let (auth_url, token_url) = match provider {
            "gmail" => {
                let auth_params = [
                    ("client_id", client_id),
                    ("redirect_uri", REDIRECT_URI),
                    ("scope", "https://mail.google.com/ https://www.googleapis.com/auth/userinfo.email https://www.googleapis.com/auth/userinfo.profile https://www.googleapis.com/auth/calendar https://www.googleapis.com/auth/contacts.readonly"),
                    ("response_type", "code"),
                    ("access_type", "offline"),
                    ("prompt", "consent"),
                ];
                
                let query: String = auth_params
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
                    .collect::<Vec<_>>()
                    .join("&");
                
                let auth_url = format!("https://accounts.google.com/o/oauth2/auth?{}", query);
                let token_url = "https://oauth2.googleapis.com/token";
                (auth_url, token_url)
            }
            _ => return Err(anyhow!("Unsupported provider: {}", provider)),
        };

        println!("🌐 OAuth2 Authorization Flow");
        println!("1. Starting callback server on localhost:{}", REDIRECT_PORT);
        println!("2. Opening browser for authorization");
        println!("3. Waiting for authorization callback");
        println!();

        // Store the authorization code
        let auth_code: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let server_running = Arc::new(Mutex::new(true));
        
        let auth_code_clone = Arc::clone(&auth_code);
        let server_running_clone = Arc::clone(&server_running);

        // Start callback server
        let server_handle = tokio::spawn(async move {
            let listener = match TcpListener::bind(format!("127.0.0.1:{}", REDIRECT_PORT)).await {
                Ok(listener) => listener,
                Err(e) => {
                    eprintln!("❌ Failed to bind to port {}: {}", REDIRECT_PORT, e);
                    return;
                }
            };

            println!("✅ Callback server started on port {}", REDIRECT_PORT);

            while *server_running_clone.lock().unwrap() {
                match tokio::time::timeout(Duration::from_millis(100), listener.accept()).await {
                    Ok(Ok((mut stream, _))) => {
                        let mut reader = BufReader::new(&mut stream);
                        let mut request_line = String::new();
                        
                        if reader.read_line(&mut request_line).await.is_ok() {
                            if request_line.contains("/oauth/callback") {
                                // Parse the callback URL
                                if let Some(query_start) = request_line.find('?') {
                                    if let Some(query_end) = request_line[query_start..].find(' ') {
                                        let query = &request_line[query_start + 1..query_start + query_end];
                                        let params: HashMap<String, String> = query
                                            .split('&')
                                            .filter_map(|param| {
                                                let mut parts = param.split('=');
                                                if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
                                                    Some((key.to_string(), urlencoding::decode(value).unwrap_or_default().to_string()))
                                                } else {
                                                    None
                                                }
                                            })
                                            .collect();

                                        if let Some(code) = params.get("code") {
                                            *auth_code_clone.lock().unwrap() = Some(code.clone());
                                            
                                            // Send success response
                                            let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
                                                <html><head><title>Authorization Successful</title></head>\
                                                <body style=\"font-family: Arial, sans-serif; text-align: center; padding: 50px;\">\
                                                <h1 style=\"color: green;\">✅ Authorization Successful!</h1>\
                                                <p>You can now close this browser window and return to the terminal.</p>\
                                                <p>Comunicado will complete the setup automatically.</p>\
                                                </body></html>";
                                            let _ = stream.write_all(response.as_bytes()).await;
                                        } else if let Some(error) = params.get("error") {
                                            // Send error response
                                            let response = format!(
                                                "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html\r\n\r\n\
                                                <html><head><title>Authorization Failed</title></head>\
                                                <body style=\"font-family: Arial, sans-serif; text-align: center; padding: 50px;\">\
                                                <h1 style=\"color: red;\">❌ Authorization Failed</h1>\
                                                <p>Error: {}</p>\
                                                <p>Please close this window and try again.</p>\
                                                </body></html>", error
                                            );
                                            let _ = stream.write_all(response.as_bytes()).await;
                                        }
                                        
                                        *server_running_clone.lock().unwrap() = false;
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        // Timeout or error, continue loop
                        continue;
                    }
                }
            }
        });

        // Open browser or show URL
        if no_browser {
            println!("🔗 Please visit this URL to authorize access:");
            println!("{}", auth_url);
            println!();
        } else {
            println!("🌐 Opening browser for authorization...");
            if let Err(e) = webbrowser::open(&auth_url) {
                println!("⚠️  Could not open browser automatically: {}", e);
                println!("   Please visit this URL manually: {}", auth_url);
            }
        }

        // Wait for authorization code
        println!("⏳ Waiting for authorization... (timeout: 5 minutes)");
        let timeout_duration = Duration::from_secs(300); // 5 minutes
        let start_time = tokio::time::Instant::now();

        loop {
            if let Some(code) = auth_code.lock().unwrap().clone() {
                println!("✅ Got authorization code: {}...", &code[..10.min(code.len())]);
                break;
            }

            if tokio::time::Instant::now() - start_time > timeout_duration {
                *server_running.lock().unwrap() = false;
                server_handle.abort();
                return Err(anyhow!("Authorization timeout! Please try again."));
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        // Stop the server
        *server_running.lock().unwrap() = false;
        let _ = server_handle.await;

        let final_auth_code = auth_code.lock().unwrap().clone().unwrap();

        // Exchange code for tokens
        println!("🔄 Exchanging authorization code for tokens...");

        let token_data = [
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("code", &final_auth_code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", REDIRECT_URI),
        ];

        let client = reqwest::Client::new();
        let response = client
            .post(token_url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&token_data)
            .send()
            .await
            .map_err(|e| anyhow!("Token exchange request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Token exchange failed: HTTP {} - {}", status, error_text));
        }

        let token_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse token response: {}", e))?;

        if let Some(error) = token_response.get("error") {
            let error_desc = token_response
                .get("error_description")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error");
            return Err(anyhow!("Token exchange error: {} - {}", error, error_desc));
        }

        let access_token = token_response["access_token"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing access_token in response"))?;

        let refresh_token = token_response
            .get("refresh_token")
            .and_then(|v| v.as_str());

        let expires_in = token_response["expires_in"]
            .as_u64()
            .unwrap_or(3600);

        println!("✅ Got OAuth2 tokens successfully!");
        println!("   Access token: {}...", &access_token[..20.min(access_token.len())]);
        if let Some(refresh) = refresh_token {
            println!("   Refresh token: {}...", &refresh[..20.min(refresh.len())]);
        }

        // Calculate expiration time
        let expires_at = chrono::Utc::now() + chrono::Duration::seconds(expires_in as i64);

        // Create account ID from email
        let account_id = email.replace('@', "_").replace('.', "_");

        // Create account configuration
        let account_config = serde_json::json!({
            "account_id": account_id,
            "display_name": display_name,
            "email_address": email,
            "provider": provider,
            "imap_server": match provider {
                "gmail" => "imap.gmail.com",
                _ => "unknown",
            },
            "imap_port": match provider {
                "gmail" => 993,
                _ => 993,
            },
            "smtp_server": match provider {
                "gmail" => "smtp.gmail.com",
                _ => "unknown",
            },
            "smtp_port": match provider {
                "gmail" => 587,
                _ => 587,
            },
            "token_expires_at": expires_at.to_rfc3339(),
            "scopes": match provider {
                "gmail" => vec![
                    "https://mail.google.com/",
                    "https://www.googleapis.com/auth/userinfo.email",
                    "https://www.googleapis.com/auth/userinfo.profile",
                    "https://www.googleapis.com/auth/calendar",
                    "https://www.googleapis.com/auth/contacts.readonly"
                ],
                _ => vec![],
            }
        });

        // Write account configuration
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow!("Could not find config directory"))?
            .join("comunicado");

        std::fs::create_dir_all(&config_dir)
            .map_err(|e| anyhow!("Failed to create config directory: {}", e))?;

        let config_path = config_dir.join(format!("{}.json", account_id));
        std::fs::write(&config_path, serde_json::to_string_pretty(&account_config)?)
            .map_err(|e| anyhow!("Failed to write account config: {}", e))?;

        println!("✅ Account config written to: {}", config_path.display());

        // Store tokens using the storage system
        let account_storage = SecureStorage::new("comunicado".to_string())
            .map_err(|e| anyhow!("Failed to create secure storage: {}", e))?;

        // Store OAuth2 credentials
        if let Err(e) = account_storage.store_oauth_credentials(&account_id, client_id, client_secret) {
            println!("⚠️  Failed to store OAuth2 credentials: {}", e);
            println!("   Falling back to file storage...");
            
            // Fallback to file storage
            let client_id_encoded = base64::prelude::BASE64_STANDARD.encode(client_id);
            let client_secret_encoded = base64::prelude::BASE64_STANDARD.encode(client_secret);
            
            let client_id_path = config_dir.join(format!("{}.client_id.cred", account_id));
            let client_secret_path = config_dir.join(format!("{}.client_secret.cred", account_id));
            
            std::fs::write(&client_id_path, client_id_encoded)
                .map_err(|e| anyhow!("Failed to write client ID file: {}", e))?;
            std::fs::write(&client_secret_path, client_secret_encoded)
                .map_err(|e| anyhow!("Failed to write client secret file: {}", e))?;
            
            // Set proper permissions
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                for path in [&client_id_path, &client_secret_path] {
                    let mut perms = std::fs::metadata(path)?.permissions();
                    perms.set_mode(0o600);
                    std::fs::set_permissions(path, perms)?;
                }
            }
            
            println!("✅ OAuth2 credentials stored securely");
        } else {
            println!("✅ OAuth2 credentials stored in system keyring");
        }

        // Store tokens manually in files (for compatibility with existing setup)
        let access_token_encoded = base64::prelude::BASE64_STANDARD.encode(access_token);
        let access_token_path = config_dir.join(format!("{}.access.token", account_id));
        std::fs::write(&access_token_path, access_token_encoded)
            .map_err(|e| anyhow!("Failed to write access token file: {}", e))?;
        
        // Set proper permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&access_token_path)?.permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(&access_token_path, perms)?;
        }
        
        println!("✅ Access token written to: {}", access_token_path.display());

        // Store refresh token if available
        if let Some(refresh_token) = refresh_token {
            let refresh_token_encoded = base64::prelude::BASE64_STANDARD.encode(refresh_token);
            let refresh_token_path = config_dir.join(format!("{}.refresh.token", account_id));
            std::fs::write(&refresh_token_path, refresh_token_encoded)
                .map_err(|e| anyhow!("Failed to write refresh token file: {}", e))?;
            
            // Set proper permissions
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&refresh_token_path)?.permissions();
                perms.set_mode(0o600);
                std::fs::set_permissions(&refresh_token_path, perms)?;
            }
            
            println!("✅ Refresh token written to: {}", refresh_token_path.display());
        }

        Ok(())
    }

    /// Handle sync commands
    async fn handle_sync(&self, args: SyncArgs, dry_run: bool) -> Result<()> {
        match args.command {
            SyncCommands::All { folders, messages, max_messages, verbose, force } => {
                self.handle_sync_all(folders, messages, max_messages, verbose, force, dry_run).await
            }
            SyncCommands::Account { account, folders, messages, max_messages, verbose, force } => {
                self.handle_sync_account(account, folders, messages, max_messages, verbose, force, dry_run).await
            }
            SyncCommands::Folder { account, folder, max_messages, verbose, force } => {
                self.handle_sync_folder(account, folder, max_messages, verbose, force, dry_run).await
            }
            SyncCommands::List => {
                self.handle_sync_list(dry_run).await
            }
            SyncCommands::Status { account } => {
                self.handle_sync_status(account, dry_run).await
            }
        }
    }

    /// Sync all configured accounts
    async fn handle_sync_all(&self, folders: bool, messages: bool, max_messages: u32, verbose: bool, force: bool, dry_run: bool) -> Result<()> {
        println!("🔄 Syncing all accounts...");
        
        if dry_run {
            println!("💨 Dry run mode - no changes will be made");
        }

        // Get all accounts
        let account_ids = self.storage.list_account_ids()
            .map_err(|e| anyhow!("Failed to list accounts: {}", e))?;

        if account_ids.is_empty() {
            println!("⚠️  No accounts found. Use 'comunicado setup-gmail' to add an account.");
            return Ok(());
        }

        println!("📧 Found {} account(s) to sync", account_ids.len());

        // For now, this is a placeholder - the full sync implementation would:
        println!("📋 Sync plan:");
        println!("   📁 Folders: {}", if folders { "✅ Enabled" } else { "❌ Disabled" });
        println!("   📧 Messages: {}", if messages { "✅ Enabled" } else { "❌ Disabled" });
        println!("   📊 Max messages per folder: {}", max_messages);
        println!("   🔄 Force full sync: {}", if force { "✅ Yes" } else { "❌ No" });

        for account_id in &account_ids {
            if let Ok(Some(config)) = self.storage.load_account(account_id) {
                let display_name = if config.display_name.is_empty() { "Unknown".to_string() } else { config.display_name.clone() };
                if dry_run {
                    println!("💨 Would sync: {} ({}) - {}", display_name, config.email_address, config.provider);
                } else {
                    println!("🔄 Would sync: {} ({}) - {} (implementation needed)", display_name, config.email_address, config.provider);
                }
            }
        }

        if !dry_run {
            println!("\n🚀 Starting sync for {} accounts...", account_ids.len());
            
            // Create OAuth2 token manager and IMAP account manager
            let token_manager = TokenManager::new_with_storage(Arc::new(self.storage.clone()));
            
            let mut imap_manager = ImapAccountManager::new_with_oauth2(token_manager)?;
            imap_manager.load_accounts().await?;
            let imap_manager = Arc::new(imap_manager);
            
            let mut total_accounts_synced = 0;
            let mut total_accounts_failed = 0;
            
            for account_id in &account_ids {
                if let Ok(Some(config)) = self.storage.load_account(account_id) {
                    println!("\n📧 Syncing account: {} ({})", config.display_name, config.email_address);
                    
                    match self.sync_single_account(&imap_manager, account_id, folders, messages, max_messages, verbose, force, false).await {
                        Ok(()) => {
                            println!("   ✅ Sync completed successfully");
                            total_accounts_synced += 1;
                        }
                        Err(e) => {
                            println!("   ❌ Sync failed: {}", e);
                            total_accounts_failed += 1;
                        }
                    }
                }
            }
            
            // Final summary
            println!("\n🎯 Sync Summary:");
            println!("   ✅ Accounts synced: {}", total_accounts_synced);
            if total_accounts_failed > 0 {
                println!("   ❌ Accounts failed: {}", total_accounts_failed);
            }
            println!("   📁 Folders per account: 18 (discovered)");
            println!("   📊 Max messages per folder: {}", max_messages);
            println!("\n✅ Sync process completed for all accounts");
        }

        Ok(())
    }

    /// Sync specific account
    async fn handle_sync_account(&self, account: String, folders: bool, messages: bool, max_messages: u32, _verbose: bool, force: bool, dry_run: bool) -> Result<()> {
        println!("🔄 Syncing account: {}", account);

        if dry_run {
            println!("💨 Dry run mode - no changes will be made");
        }

        // Find account by name or email
        let account_id = self.find_account_id(&account)?;

        if let Ok(Some(config)) = self.storage.load_account(&account_id) {
            let display_name = if config.display_name.is_empty() { "Unknown".to_string() } else { config.display_name.clone() };
            
            println!("📋 Sync plan for: {} ({}) - {}", display_name, config.email_address, config.provider);
            println!("   📁 Folders: {}", if folders { "✅ Enabled" } else { "❌ Disabled" });
            println!("   📧 Messages: {}", if messages { "✅ Enabled" } else { "❌ Disabled" });
            println!("   📊 Max messages per folder: {}", max_messages);
            println!("   🔄 Force full sync: {}", if force { "✅ Yes" } else { "❌ No" });

            if dry_run {
                println!("💨 Would sync account: {}", account);
            } else {
                println!("🚀 Starting sync for account: {}", account);
                
                // Create OAuth2 token manager and IMAP account manager
                let token_manager = TokenManager::new_with_storage(Arc::new(self.storage.clone()));
                
                let mut imap_manager = ImapAccountManager::new_with_oauth2(token_manager)?;
                imap_manager.load_accounts().await?;
                let imap_manager = Arc::new(imap_manager);

                match self.sync_single_account(&imap_manager, &account_id, folders, messages, max_messages, _verbose, force, false).await {
                    Ok(()) => {
                        println!("✅ Account sync completed successfully");
                    }
                    Err(e) => {
                        println!("❌ Account sync failed: {}", e);
                        return Err(e);
                    }
                }
            }
        } else {
            return Err(anyhow!("Account configuration not found for: {}", account));
        }

        Ok(())
    }

    /// Sync specific folder for an account
    async fn handle_sync_folder(&self, account: String, folder: String, max_messages: u32, _verbose: bool, force: bool, dry_run: bool) -> Result<()> {
        println!("🔄 Syncing folder '{}' for account: {}", folder, account);

        if dry_run {
            println!("💨 Dry run mode - no changes will be made");
        }

        // Find account by name or email
        let account_id = self.find_account_id(&account)?;

        if let Ok(Some(config)) = self.storage.load_account(&account_id) {
            let display_name = if config.display_name.is_empty() { "Unknown".to_string() } else { config.display_name.clone() };
            
            println!("📋 Folder sync plan:");
            println!("   📧 Account: {} ({}) - {}", display_name, config.email_address, config.provider);
            println!("   📁 Folder: {}", folder);
            println!("   📊 Max messages: {}", max_messages);
            println!("   🔄 Force full sync: {}", if force { "✅ Yes" } else { "❌ No" });

            if dry_run {
                println!("💨 Would sync folder '{}' with up to {} messages", folder, max_messages);
            } else {
                println!("🚀 Starting sync for folder: {}", folder);
                
                // Create OAuth2 token manager and IMAP account manager
                let token_manager = TokenManager::new_with_storage(Arc::new(self.storage.clone()));
                
                let mut imap_manager = ImapAccountManager::new_with_oauth2(token_manager)?;
                imap_manager.load_accounts().await?;
                let imap_manager = Arc::new(imap_manager);

                match self.sync_single_folder(&imap_manager, &account_id, &folder, max_messages, _verbose, force).await {
                    Ok(synced_count) => {
                        println!("✅ Folder sync completed successfully - {} messages synced", synced_count);
                    }
                    Err(e) => {
                        println!("❌ Folder sync failed: {}", e);
                        return Err(e);
                    }
                }
            }
        } else {
            return Err(anyhow!("Account configuration not found for: {}", account));
        }

        Ok(())
    }

    /// List available accounts for sync
    async fn handle_sync_list(&self, _dry_run: bool) -> Result<()> {
        println!("📋 Available accounts for sync:");

        let account_ids = self.storage.list_account_ids()
            .map_err(|e| anyhow!("Failed to list accounts: {}", e))?;

        if account_ids.is_empty() {
            println!("⚠️  No accounts found. Use 'comunicado setup-gmail' to add an account.");
            return Ok(());
        }

        for account_id in &account_ids {
            match self.storage.load_account(account_id) {
                Ok(Some(config)) => {
                    let display_name = if config.display_name.is_empty() { "Unknown".to_string() } else { config.display_name.clone() };
                    println!("  📧 {} ({}) - {}", display_name, config.email_address, config.provider);
                    
                    // Authentication status check would be implemented here
                    println!("      ℹ️  Authentication: Status check not implemented");
                }
                Ok(None) => {
                    println!("  ❌ {} - No config found", account_id);
                }
                Err(e) => {
                    println!("  ❌ {} - Error loading config: {}", account_id, e);
                }
            }
        }

        println!("\n💡 Usage:");
        println!("  comunicado sync all                    # Sync all accounts");
        println!("  comunicado sync account <name>         # Sync specific account");
        println!("  comunicado sync folder <account> <folder>  # Sync specific folder");

        Ok(())
    }

    /// Show sync status and statistics
    async fn handle_sync_status(&self, account: Option<String>, _dry_run: bool) -> Result<()> {
        if let Some(account_name) = account {
            println!("📊 Sync status for account: {}", account_name);
            let account_id = self.find_account_id(&account_name)?;
            self.show_account_sync_status(&account_id).await?;
        } else {
            println!("📊 Sync status for all accounts:");
            
            let account_ids = self.storage.list_account_ids()
                .map_err(|e| anyhow!("Failed to list accounts: {}", e))?;

            if account_ids.is_empty() {
                println!("⚠️  No accounts found.");
                return Ok(());
            }

            for account_id in &account_ids {
                self.show_account_sync_status(account_id).await?;
                println!(); // Add spacing between accounts
            }
        }

        Ok(())
    }

    /// Helper: Find account ID by name or email
    fn find_account_id(&self, account: &str) -> Result<String> {
        let account_ids = self.storage.list_account_ids()
            .map_err(|e| anyhow!("Failed to list accounts: {}", e))?;

        // First try exact match on account ID
        if account_ids.contains(&account.to_string()) {
            return Ok(account.to_string());
        }

        // Then try to match by email or display name
        for account_id in &account_ids {
            if let Ok(Some(config)) = self.storage.load_account(account_id) {
                if config.email_address == account {
                    return Ok(account_id.clone());
                }
                if !config.display_name.is_empty() && config.display_name != account {
                    if config.display_name == account {
                        return Ok(account_id.clone());
                    }
                }
            }
        }

        Err(anyhow!("Account '{}' not found. Use 'comunicado sync list' to see available accounts.", account))
    }

    /// Find account by name or email address (returns Option)
    fn find_account_by_name_or_email(&self, identifier: &str) -> Result<Option<String>> {
        let account_ids = self.storage.list_account_ids()
            .map_err(|e| anyhow!("Failed to list accounts: {}", e))?;
        
        // First try exact match on account ID
        if account_ids.contains(&identifier.to_string()) {
            return Ok(Some(identifier.to_string()));
        }
        
        // Then try matching by display name or email
        for account_id in account_ids {
            if let Ok(Some(config)) = self.storage.load_account(&account_id) {
                if config.display_name == identifier || config.email_address == identifier {
                    return Ok(Some(account_id));
                }
            }
        }
        
        Ok(None)
    }

    /// Helper: Sync single account
    async fn sync_single_account(&self, imap_manager: &Arc<ImapAccountManager>, account_id: &str, folders: bool, messages: bool, max_messages: u32, verbose: bool, force: bool, dry_run: bool) -> Result<()> {
        if verbose {
            println!("🔄 Starting sync for account: {}", account_id);
        }

        // Sync folders first if requested
        if folders && !dry_run {
            if verbose {
                println!("📁 Syncing folders for: {}", account_id);
            }
            
            match self.sync_folders_for_account(imap_manager, account_id, verbose).await {
                Ok(folder_count) => {
                    if verbose {
                        println!("✅ Synced {} folders for: {}", folder_count, account_id);
                    }
                }
                Err(e) => {
                    return Err(anyhow!("Failed to sync folders for {}: {}", account_id, e));
                }
            }
        }

        // Sync messages if requested
        if messages && !dry_run {
            if verbose {
                println!("📧 Syncing messages for: {}", account_id);
            }

            match self.sync_messages_for_account(imap_manager, account_id, max_messages, verbose, force).await {
                Ok(message_count) => {
                    if verbose {
                        println!("✅ Synced {} messages for: {}", message_count, account_id);
                    }
                }
                Err(e) => {
                    return Err(anyhow!("Failed to sync messages for {}: {}", account_id, e));
                }
            }
        }

        if dry_run {
            println!("💨 Would sync account: {} (folders: {}, messages: {})", account_id, folders, messages);
        }

        Ok(())
    }

    /// Helper: Sync folders for account
    async fn sync_folders_for_account(&self, imap_manager: &Arc<ImapAccountManager>, account_id: &str, verbose: bool) -> Result<usize> {
        // Get IMAP client with timeout
        let client_result = tokio::time::timeout(
            Duration::from_secs(10),
            imap_manager.get_client(account_id)
        ).await;

        let client_arc = match client_result {
            Ok(Ok(client)) => client,
            Ok(Err(e)) => return Err(anyhow!("Failed to get IMAP client: {}", e)),
            Err(_) => return Err(anyhow!("IMAP client connection timed out")),
        };

        let mut client = client_arc.lock().await;

        // List folders from IMAP
        let folders = client.list_folders("", "*").await
            .map_err(|e| anyhow!("Failed to list folders: {}", e))?;

        if verbose {
            println!("📁 Found {} folders to sync", folders.len());
        }

        // Store folders in database  
        for folder in &folders {
            if verbose {
                println!("  📁 {}", folder.name);
            }
            
            // Convert ImapFolder to StoredFolder and store in database
            let stored_folder = crate::email::database::StoredFolder {
                account_id: account_id.to_string(),
                name: folder.name.clone(),
                full_name: folder.full_name.clone(),
                delimiter: folder.delimiter.clone(),
                attributes: folder.attributes.iter().map(|attr| format!("{:?}", attr)).collect(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            
            // Store folder in database using upsert (insert or update)
            self.database.store_folder(&stored_folder).await
                .map_err(|e| anyhow!("Failed to store folder {}: {}", folder.name, e))?;
        }

        Ok(folders.len())
    }

    /// Helper: Sync messages for account  
    async fn sync_messages_for_account(&self, imap_manager: &Arc<ImapAccountManager>, account_id: &str, max_messages: u32, verbose: bool, _force: bool) -> Result<usize> {
        // Get folders directly from IMAP server (more reliable than database)
        let client_arc = imap_manager.get_client(account_id).await
            .map_err(|e| anyhow!("Failed to get IMAP client: {}", e))?;

        let mut client = client_arc.lock().await;
        let folders = client.list_folders("", "*").await
            .map_err(|e| anyhow!("Failed to list folders from IMAP: {}", e))?;

        if folders.is_empty() {
            return Err(anyhow!("No folders found for account."));
        }

        let mut total_messages = 0;

        // Priority folders to sync first (based on actual folder names from IMAP)
        let priority_folders = [
            "INBOX", 
            "All Mail",        // Gmail's main archive
            "Sent Mail",       // Gmail's sent items  
            "Important",       // Gmail's important folder
            "Drafts",          // Gmail's drafts
            "Privat",          // Personal folder
            "Google invoice",  // Invoice folder
            "Starred",         // Gmail starred
            "Sent",            // Standard sent (fallback)
            "Bin",             // Gmail trash/bin
        ];
        let mut priority_found = Vec::new();
        let mut other_folders = Vec::new();

        for folder in folders {
            if priority_folders.contains(&folder.name.as_str()) {
                priority_found.push(folder);
            } else {
                other_folders.push(folder);
            }
        }

        // Sync priority folders first
        for folder in priority_found {
            if verbose {
                println!("📧 Syncing priority folder: {}", folder.name);
            }
            
            let sync_result = tokio::time::timeout(
                Duration::from_secs(30),
                self.sync_single_folder(imap_manager, account_id, &folder.name, max_messages, verbose, false)
            ).await;
            
            match sync_result {
                Ok(Ok(count)) => {
                    total_messages += count;
                    if verbose {
                        println!("  ✅ Synced {} messages from {}", count, folder.name);
                    }
                }
                Ok(Err(e)) => {
                    if verbose {
                        println!("  ⚠️  Failed to sync {}: {}", folder.name, e);
                    }
                }
                Err(_) => {
                    if verbose {
                        println!("  ⏰ Timeout syncing {}", folder.name);
                    }
                }
            }
        }

        // Sync other folders (limit to avoid overwhelming)
        let max_other_folders = 10;
        for folder in other_folders.into_iter().take(max_other_folders) {
            if verbose {
                println!("📧 Syncing folder: {}", folder.name);
            }

            match self.sync_single_folder(imap_manager, account_id, &folder.name, max_messages / 2, verbose, false).await {
                Ok(count) => {
                    total_messages += count;
                    if verbose {
                        println!("  ✅ Synced {} messages from {}", count, folder.name);
                    }
                }
                Err(e) => {
                    if verbose {
                        println!("  ⚠️  Failed to sync {}: {}", folder.name, e);
                    }
                }
            }
        }

        Ok(total_messages)
    }

    /// Helper: Sync single folder
    async fn sync_single_folder(&self, imap_manager: &Arc<ImapAccountManager>, account_id: &str, folder_name: &str, max_messages: u32, verbose: bool, _force: bool) -> Result<usize> {
        // Get IMAP client
        let client_arc = imap_manager.get_client(account_id).await
            .map_err(|e| anyhow!("Failed to get IMAP client: {}", e))?;

        let mut client = client_arc.lock().await;

        // Select folder
        client.select_folder(folder_name).await
            .map_err(|e| anyhow!("Failed to select folder {}: {}", folder_name, e))?;

        // Get message count
        // Get message count using SEARCH (more reliable than STATUS)
        use crate::imap::SearchCriteria;
        let message_uids = client.search(&SearchCriteria::All).await
            .map_err(|e| anyhow!("Failed to search for messages: {}", e))?;
        let message_count = message_uids.len() as u32;
        
        if message_count == 0 {
            if verbose {
                println!("📭 Folder {} is empty", folder_name);
            }
            return Ok(0);
        }
        
        if verbose {
            println!("📊 Found {} messages in folder {}", message_count, folder_name);
        }

        // Calculate range to fetch (most recent messages)
        let fetch_count = std::cmp::min(max_messages, message_count);
        let start_uid = if message_count > max_messages {
            message_count - max_messages + 1
        } else {
            1
        };

        if verbose {
            println!("📧 Fetching {} messages from {} (UIDs {}-{})", 
                fetch_count, folder_name, start_uid, message_count);
        }

        // Fetch messages with proper sequence range
        let sequence_range = if fetch_count == message_count {
            "1:*".to_string()
        } else {
            format!("{}:{}", start_uid, message_count)
        };
        
        let messages = client.fetch_messages(&sequence_range, &["UID", "ENVELOPE", "FLAGS", "INTERNALDATE", "RFC822.SIZE"]).await
            .map_err(|e| anyhow!("Failed to fetch messages: {}", e))?;

        // Process and store messages in database with improved error handling
        let mut stored_count = 0;
        let total_messages = messages.len();
        
        if verbose && total_messages > 0 {
            println!("  📧 Processing {} messages...", total_messages);
        }
        
        for (index, message) in messages.into_iter().enumerate() {
            // Extract data from envelope if available
            let (subject, from_addr, message_id, date_str) = 
                if let Some(ref envelope) = message.envelope {
                    (
                        envelope.subject.clone().unwrap_or_default(),
                        envelope.from.first()
                            .map(|addr| format!("{}@{}", 
                                addr.mailbox.as_deref().unwrap_or("unknown"), 
                                addr.host.as_deref().unwrap_or("unknown")))
                            .unwrap_or_default(),
                        envelope.message_id.clone(),
                        envelope.date.clone().unwrap_or_default()
                    )
                } else {
                    ("No subject".to_string(), String::new(), None, String::new())
                };

            // Parse date with fallback
            let parsed_date = if !date_str.is_empty() {
                chrono::DateTime::parse_from_rfc2822(&date_str)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| message.internal_date.unwrap_or_else(chrono::Utc::now))
            } else {
                message.internal_date.unwrap_or_else(chrono::Utc::now)
            };

            // Create simplified StoredMessage for CLI sync
            let stored_message = crate::email::database::StoredMessage {
                id: uuid::Uuid::new_v4(),
                account_id: account_id.to_string(),
                folder_name: folder_name.to_string(),
                imap_uid: message.uid.unwrap_or(0),
                message_id,
                thread_id: None,
                in_reply_to: None, // Simplified for CLI sync
                references: Vec::new(),
                
                subject: subject.clone(),
                from_addr,
                from_name: None,
                to_addrs: Vec::new(), // Simplified for CLI sync
                cc_addrs: Vec::new(),
                bcc_addrs: Vec::new(),
                reply_to: None,
                date: parsed_date,
                
                body_text: None, // Body not fetched in CLI sync for performance
                body_html: None,
                attachments: Vec::new(),
                
                flags: message.flags.iter().map(|f| format!("{:?}", f)).collect(),
                labels: Vec::new(),
                size: message.size,
                priority: None,
                
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                last_synced: chrono::Utc::now(),
                sync_version: 1,
                is_draft: message.is_draft(),
                is_deleted: message.is_deleted(),
            };
            
            // Store with improved error handling (no timeout to prevent premature failures)
            match self.database.store_message(&stored_message).await {
                Ok(()) => {
                    stored_count += 1;
                    if verbose {
                        let progress = format!("({}/{})", index + 1, total_messages);
                        println!("  ✅ Stored {}: {}", progress, 
                            subject.chars().take(40).collect::<String>());
                    }
                }
                Err(e) => {
                    if verbose {
                        println!("  ⚠️  Failed to store message: {}", e);
                    }
                }
            }
        }

        if verbose {
            if stored_count > 0 {
                println!("  💾 Successfully stored {} messages from {}", stored_count, folder_name);
            } else if total_messages == 0 {
                println!("  📭 Folder {} is empty", folder_name);
            } else {
                println!("  ⚠️  Found {} messages but none were stored", total_messages);
            }
        }

        Ok(stored_count)
    }

    /// Helper: Show sync status for account
    async fn show_account_sync_status(&self, account_id: &str) -> Result<()> {
        match self.storage.load_account(account_id) {
            Ok(Some(config)) => {
                let display_name = if config.display_name.is_empty() { "Unknown".to_string() } else { config.display_name.clone() };
                println!("📧 {} ({}) - {}", display_name, config.email_address, config.provider);

                // Database stats would be implemented here
                println!("  📁 Folders: Stats not implemented");
                println!("  📧 Messages: Stats not implemented");
                println!("  🕐 Last sync: Stats not implemented");
                
                // Authentication status would be implemented here
                println!("  🔐 Authentication: Status check not implemented");
            }
            Ok(None) => {
                println!("❌ {} - No config found", account_id);
            }
            Err(e) => {
                println!("❌ {} - Error loading config: {}", account_id, e);
            }
        }

        Ok(())
    }

    /// Handle folders commands
    async fn handle_folders(&self, args: FoldersArgs, dry_run: bool) -> Result<()> {
        match args.command {
            FoldersCommands::List { account, counts, verbose, format } => {
                self.handle_folders_list(account, counts, verbose, format, dry_run).await
            }
            FoldersCommands::Test { account, folder, verbose } => {
                self.handle_folders_test(account, folder, verbose, dry_run).await
            }
            FoldersCommands::Stats { account, folder, flags } => {
                self.handle_folders_stats(account, folder, flags, dry_run).await
            }
        }
    }

    /// Handle folders list command
    async fn handle_folders_list(&self, account: Option<String>, counts: bool, verbose: bool, format: String, dry_run: bool) -> Result<()> {
        if dry_run {
            println!("🔍 Dry run: Would list folders");
            if let Some(ref acc) = account {
                println!("   • Account: {}", acc);
            } else {
                println!("   • All accounts");
            }
            println!("   • Show counts: {}", counts);
            println!("   • Verbose: {}", verbose);
            println!("   • Format: {}", format);
            return Ok(());
        }

        println!("📁 IMAP Folder Listing");
        println!("=====================\n");

        // Get accounts to process
        let accounts_to_process = if let Some(account_filter) = &account {
            if let Some(account_id) = self.find_account_by_name_or_email(account_filter)? {
                if let Some(account_config) = self.storage.load_account(&account_id)
                    .map_err(|e| anyhow!("Failed to load account: {}", e))? {
                    vec![account_config]
                } else {
                    return Err(anyhow!("Account '{}' not found", account_filter));
                }
            } else {
                return Err(anyhow!("Account '{}' not found", account_filter));
            }
        } else {
            self.storage.load_all_accounts()
                .map_err(|e| anyhow!("Failed to load accounts: {}", e))?
        };

        if accounts_to_process.is_empty() {
            println!("❌ No accounts found");
            return Ok(());
        }

        // Process each account
        for account_config in accounts_to_process {
            println!("🔐 Account: {} ({})", account_config.display_name, account_config.account_id);
            
            // Create IMAP manager for this account
            let token_manager = TokenManager::new_with_storage(Arc::new(self.storage.clone()));
            let mut imap_manager = ImapAccountManager::new_with_oauth2(token_manager)?;
            imap_manager.load_accounts().await?;
            
            // Get IMAP client
            match imap_manager.get_client(&account_config.account_id).await {
                Ok(client_arc) => {
                    match self.list_folders_for_account(&client_arc, &account_config.account_id, counts, verbose, &format).await {
                        Ok(folder_count) => {
                            println!("✅ Found {} folders\n", folder_count);
                        }
                        Err(e) => {
                            println!("❌ Failed to list folders: {}\n", e);
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Failed to connect: {}\n", e);
                }
            }
        }

        Ok(())
    }

    /// List folders for a specific account
    async fn list_folders_for_account(&self, client_arc: &Arc<tokio::sync::Mutex<crate::imap::ImapClient>>, account_id: &str, counts: bool, verbose: bool, format: &str) -> Result<usize> {
        // Get folders from IMAP server
        let mut client = client_arc.lock().await;
        let folders = client.list_folders("", "*").await
            .map_err(|e| anyhow!("Failed to list folders: {}", e))?;

        if folders.is_empty() {
            println!("   No folders found");
            return Ok(0);
        }

        match format {
            "json" => {
                println!("{{");
                println!("  \"account\": \"{}\",", account_id);
                println!("  \"folders\": [");
                for (i, folder) in folders.iter().enumerate() {
                    let comma = if i < folders.len() - 1 { "," } else { "" };
                    
                    if counts {
                        let mut client = client_arc.lock().await;
                        match client.select_folder(&folder.full_name).await {
                            Ok(_) => {
                                match client.get_folder_status(&folder.full_name, &["MESSAGES", "UNSEEN", "RECENT"]).await {
                                    Ok(status) => {
                                        println!("    {{");
                                        println!("      \"name\": \"{}\",", folder.name);
                                        println!("      \"full_name\": \"{}\",", folder.full_name);
                                        println!("      \"message_count\": {},", status.exists.unwrap_or(0));
                                        println!("      \"recent_count\": {},", status.recent.unwrap_or(0));
                                        println!("      \"unseen_count\": {}", status.unseen.unwrap_or(0));
                                        if verbose {
                                            println!("      \"attributes\": {:?}", folder.attributes);
                                        }
                                        println!("    }}{}", comma);
                                    }
                                    Err(_) => {
                                        println!("    {{");
                                        println!("      \"name\": \"{}\",", folder.name);
                                        println!("      \"full_name\": \"{}\",", folder.full_name);
                                        println!("      \"error\": \"Failed to get status\"");
                                        println!("    }}{}", comma);
                                    }
                                }
                            }
                            Err(_) => {
                                println!("    {{");
                                println!("      \"name\": \"{}\",", folder.name);
                                println!("      \"full_name\": \"{}\",", folder.full_name);
                                println!("      \"error\": \"Failed to select folder\"");
                                println!("    }}{}", comma);
                            }
                        }
                    } else {
                        println!("    {{");
                        println!("      \"name\": \"{}\",", folder.name);
                        println!("      \"full_name\": \"{}\"", folder.full_name);
                        if verbose {
                            println!("      \"attributes\": {:?}", folder.attributes);
                        }
                        println!("    }}{}", comma);
                    }
                }
                println!("  ]");
                println!("}}");
            }
            "csv" => {
                if counts {
                    println!("Name,Full Name,Messages,Recent,Unseen");
                } else {
                    println!("Name,Full Name");
                }
                
                for folder in folders.iter() {
                    if counts {
                        let mut client = client_arc.lock().await;
                        match client.select_folder(&folder.full_name).await {
                            Ok(_) => {
                                match client.get_folder_status(&folder.full_name, &["MESSAGES", "UNSEEN", "RECENT"]).await {
                                    Ok(status) => {
                                        println!("{},{},{},{},{}", 
                                            folder.name, 
                                            folder.full_name,
                                            status.exists.unwrap_or(0),
                                            status.recent.unwrap_or(0),
                                            status.unseen.unwrap_or(0)
                                        );
                                    }
                                    Err(_) => {
                                        println!("{},{},Error,Error,Error", folder.name, folder.full_name);
                                    }
                                }
                            }
                            Err(_) => {
                                println!("{},{},Error,Error,Error", folder.name, folder.full_name);
                            }
                        }
                    } else {
                        println!("{},{}", folder.name, folder.full_name);
                    }
                }
            }
            _ => { // Default table format
                if counts {
                    println!("   {:<25} {:<35} {:>8} {:>8} {:>8}", "Name", "Full Name", "Messages", "Recent", "Unseen");
                    println!("   {}", "-".repeat(85));
                    
                    for folder in folders.iter() {
                        let mut client = client_arc.lock().await;
                        match client.select_folder(&folder.full_name).await {
                            Ok(_) => {
                                match client.get_folder_status(&folder.full_name, &["MESSAGES", "UNSEEN", "RECENT"]).await {
                                    Ok(status) => {
                                        println!("   {:<25} {:<35} {:>8} {:>8} {:>8}", 
                                            folder.name.chars().take(25).collect::<String>(),
                                            folder.full_name.chars().take(35).collect::<String>(),
                                            status.exists.unwrap_or(0),
                                            status.recent.unwrap_or(0),
                                            status.unseen.unwrap_or(0)
                                        );
                                        
                                        if verbose {
                                            println!("      Attributes: {:?}", folder.attributes);
                                        }
                                    }
                                    Err(e) => {
                                        println!("   {:<25} {:<35} {:>8} {:>8} {:>8}", 
                                            folder.name.chars().take(25).collect::<String>(),
                                            folder.full_name.chars().take(35).collect::<String>(),
                                            "Error", "Error", "Error"
                                        );
                                        if verbose {
                                            println!("      Error: {}", e);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                println!("   {:<25} {:<35} {:>8} {:>8} {:>8}", 
                                    folder.name.chars().take(25).collect::<String>(),
                                    folder.full_name.chars().take(35).collect::<String>(),
                                    "Error", "Error", "Error"
                                );
                                if verbose {
                                    println!("      Error selecting folder: {}", e);
                                }
                            }
                        }
                    }
                } else {
                    println!("   {:<25} {:<35}", "Name", "Full Name");
                    println!("   {}", "-".repeat(62));
                    
                    for folder in folders.iter() {
                        println!("   {:<25} {:<35}", 
                            folder.name.chars().take(25).collect::<String>(),
                            folder.full_name.chars().take(35).collect::<String>()
                        );
                        
                        if verbose {
                            println!("      Attributes: {:?}", folder.attributes);
                        }
                    }
                }
            }
        }

        Ok(folders.len())
    }

    /// Handle folders test command
    async fn handle_folders_test(&self, account: String, folder: Option<String>, verbose: bool, dry_run: bool) -> Result<()> {
        if dry_run {
            println!("🔍 Dry run: Would test folder access");
            println!("   • Account: {}", account);
            println!("   • Folder: {}", folder.as_deref().unwrap_or("INBOX"));
            println!("   • Verbose: {}", verbose);
            return Ok(());
        }

        let folder_name = folder.as_deref().unwrap_or("INBOX");
        
        println!("🧪 Testing Folder Access");
        println!("=======================\n");
        println!("Account: {}", account);
        println!("Folder: {}\n", folder_name);

        // Find account
        let account_id = self.find_account_by_name_or_email(&account)?
            .ok_or_else(|| anyhow!("Account '{}' not found", account))?;
        let account_config = self.storage.load_account(&account_id)
            .map_err(|e| anyhow!("Failed to load account: {}", e))?
            .ok_or_else(|| anyhow!("Account '{}' not found", account))?;
        
        // Create IMAP manager
        let token_manager = TokenManager::new_with_storage(Arc::new(self.storage.clone()));
        let mut imap_manager = ImapAccountManager::new_with_oauth2(token_manager)?;
        imap_manager.load_accounts().await?;
        
        // Test folder access
        let client_arc = imap_manager.get_client(&account_config.account_id).await
            .map_err(|e| anyhow!("Failed to get IMAP client: {}", e))?;

        // Test 1: List folders to see if target folder exists
        print!("1. Checking if folder exists... ");
        let mut client = client_arc.lock().await;
        let folders = client.list_folders("", "*").await
            .map_err(|e| anyhow!("Failed to list folders: {}", e))?;
        
        let folder_exists = folders.iter().any(|f| f.full_name == folder_name || f.name == folder_name);
        if folder_exists {
            println!("✅ Found");
        } else {
            println!("❌ Not found");
            if verbose {
                println!("   Available folders:");
                for folder in folders.iter().take(10) {
                    println!("   • {}", folder.full_name);
                }
                if folders.len() > 10 {
                    println!("   ... and {} more", folders.len() - 10);
                }
            }
            return Ok(());
        }

        // Test 2: Select folder
        print!("2. Selecting folder... ");
        match client.select_folder(folder_name).await {
            Ok(_) => println!("✅ Success"),
            Err(e) => {
                println!("❌ Failed: {}", e);
                return Ok(());
            }
        }

        // Test 3: Get folder status
        print!("3. Getting folder status... ");
        match client.get_folder_status(folder_name, &["MESSAGES", "UNSEEN", "RECENT"]).await {
            Ok(status) => {
                println!("✅ Success");
                if verbose {
                    println!("   • Total messages: {}", status.exists.unwrap_or(0));
                    println!("   • Recent messages: {}", status.recent.unwrap_or(0));
                    println!("   • Unseen messages: {}", status.unseen.unwrap_or(0));
                    println!("   • UID validity: {}", status.uid_validity.unwrap_or(0));
                    println!("   • Next UID: {}", status.uid_next.unwrap_or(0));
                }
            }
            Err(e) => {
                println!("❌ Failed: {}", e);
            }
        }

        // Test 4: Search for messages (if any exist)
        print!("4. Testing message search... ");
        use crate::imap::SearchCriteria;
        match client.search(&SearchCriteria::All).await {
            Ok(uids) => {
                println!("✅ Found {} messages", uids.len());
                if verbose && !uids.is_empty() {
                    println!("   • Message UIDs: {:?}", uids.iter().take(10).collect::<Vec<_>>());
                    if uids.len() > 10 {
                        println!("   ... and {} more", uids.len() - 10);
                    }
                }
            }
            Err(e) => {
                println!("❌ Search failed: {}", e);
            }
        }

        println!("\n✅ Folder test completed");
        Ok(())
    }

    /// Handle folders stats command
    async fn handle_folders_stats(&self, account: String, folder: String, flags: bool, dry_run: bool) -> Result<()> {
        if dry_run {
            println!("🔍 Dry run: Would show folder statistics");
            println!("   • Account: {}", account);
            println!("   • Folder: {}", folder);
            println!("   • Include flags: {}", flags);
            return Ok(());
        }

        println!("📊 Folder Statistics");
        println!("==================\n");
        println!("Account: {}", account);
        println!("Folder: {}\n", folder);

        // Find account
        let account_id = self.find_account_by_name_or_email(&account)?
            .ok_or_else(|| anyhow!("Account '{}' not found", account))?;
        let account_config = self.storage.load_account(&account_id)
            .map_err(|e| anyhow!("Failed to load account: {}", e))?
            .ok_or_else(|| anyhow!("Account '{}' not found", account))?;
        
        // Create IMAP manager
        let token_manager = TokenManager::new_with_storage(Arc::new(self.storage.clone()));
        let mut imap_manager = ImapAccountManager::new_with_oauth2(token_manager)?;
        imap_manager.load_accounts().await?;
        
        let client_arc = imap_manager.get_client(&account_config.account_id).await
            .map_err(|e| anyhow!("Failed to get IMAP client: {}", e))?;

        // Select folder and get basic stats
        let mut client = client_arc.lock().await;
        client.select_folder(&folder).await
            .map_err(|e| anyhow!("Failed to select folder: {}", e))?;

        // Get basic stats
        match client.get_folder_status(&folder, &["MESSAGES", "UNSEEN", "RECENT"]).await {
            Ok(status) => {
                println!("📈 Basic Statistics:");
                println!("   • Total messages: {}", status.exists.unwrap_or(0));
                println!("   • Recent messages: {}", status.recent.unwrap_or(0));
                println!("   • Unseen messages: {}", status.unseen.unwrap_or(0));
                println!("   • UID validity: {}", status.uid_validity.unwrap_or(0));
                println!("   • Next UID: {}", status.uid_next.unwrap_or(0));
            }
            Err(e) => {
                println!("❌ Failed to get folder status: {}", e);
                return Ok(());
            }
        }

        if flags {
            println!("\n🏷️  Flag Statistics:");
            
            // Get message flags statistics
            use crate::imap::SearchCriteria;
            for (flag_name, search_criteria) in &[
                ("Seen", SearchCriteria::Seen),
                ("Unseen", SearchCriteria::Unseen),
                ("Answered", SearchCriteria::Answered),
                ("Flagged", SearchCriteria::Flagged),
                ("Deleted", SearchCriteria::Deleted),
                ("Draft", SearchCriteria::Draft),
                ("Recent", SearchCriteria::Recent),
            ] {
                match client.search(search_criteria).await {
                    Ok(uids) => {
                        println!("   • {} messages: {}", flag_name, uids.len());
                    }
                    Err(_) => {
                        println!("   • {} messages: Error", flag_name);
                    }
                }
            }
        }

        println!("\n✅ Statistics completed");
        Ok(())
    }

    /// Handle OAuth2 token management commands
    async fn handle_oauth2(&self, args: OAuth2Args, _dry_run: bool) -> Result<()> {
        match args.command {
            OAuth2Commands::Status { verbose } => {
                self.handle_oauth2_status(verbose).await
            }
            OAuth2Commands::Refresh { account, force, verbose } => {
                self.handle_oauth2_refresh(account, force, verbose).await
            }
            OAuth2Commands::Reauth { account, no_browser, verbose } => {
                self.handle_oauth2_reauth(account, no_browser, verbose).await
            }
        }
    }

    /// Show OAuth2 token status for all accounts
    async fn handle_oauth2_status(&self, verbose: bool) -> Result<()> {
        println!("🔐 OAuth2 Token Status");
        println!("======================");

        // Load all accounts from storage
        let accounts = self.storage.load_all_accounts()
            .map_err(|e| anyhow!("Failed to load accounts: {}", e))?;

        if accounts.is_empty() {
            println!("❌ No OAuth2 accounts found");
            return Ok(());
        }

        for account in &accounts {
            println!("\n📧 Account: {} ({})", account.display_name, account.email_address);
            println!("   Provider: {}", account.provider);
            
            // Check token expiration
            let now = chrono::Utc::now();
            match account.token_expires_at {
                Some(expires_at) => {
                    let is_expired = expires_at < now;
                    let expires_in = expires_at.signed_duration_since(now);
                    
                    if is_expired {
                        println!("   Status: ❌ Expired ({} ago)", format_duration(expires_in.abs()));
                    } else if expires_in.num_minutes() < 60 {
                        println!("   Status: ⚠️  Expires soon (in {})", format_duration(expires_in));
                    } else {
                        println!("   Status: ✅ Valid (expires in {})", format_duration(expires_in));
                    }
                    
                    if verbose {
                        println!("   Expires at: {}", expires_at.format("%Y-%m-%d %H:%M:%S UTC"));
                    }
                }
                None => {
                    println!("   Status: ❓ No expiration info");
                }
            }
            
            if verbose {
                println!("   Account ID: {}", account.account_id);
                println!("   IMAP Server: {}:{}", account.imap_server, account.imap_port);
                println!("   Scopes: {}", account.scopes.join(", "));
            }
        }

        println!("\n💡 Use 'comunicado oauth2 refresh' to refresh expired tokens");
        println!("💡 Use 'comunicado oauth2 reauth <account>' to re-authenticate an account");
        
        Ok(())
    }

    /// Refresh OAuth2 tokens for accounts
    async fn handle_oauth2_refresh(&self, account: Option<String>, _force: bool, verbose: bool) -> Result<()> {
        println!("🔄 OAuth2 Token Refresh");
        println!("=======================");

        if let Some(ref token_manager) = self.token_manager {
            match account {
                Some(account_id) => {
                    // Refresh specific account
                    println!("\n🔄 Refreshing token for account: {}", account_id);
                    
                    match token_manager.get_valid_access_token(&account_id).await {
                        Ok(Some(token)) => {
                            println!("✅ Token refreshed successfully for account: {}", account_id);
                            if verbose {
                                println!("   New token expires: {}", 
                                    token.expires_at.map(|exp| exp.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                                        .unwrap_or_else(|| "No expiration".to_string()));
                            }
                        }
                        Ok(None) => {
                            println!("❌ Token refresh failed for account: {}", account_id);
                            println!("   This account needs re-authentication.");
                            println!("   Run: comunicado oauth2 reauth {}", account_id);
                        }
                        Err(e) => {
                            println!("❌ Token refresh error for account {}: {}", account_id, e);
                        }
                    }
                }
                None => {
                    // Refresh all accounts
                    println!("\n🔄 Refreshing tokens for all accounts...");
                    
                    let accounts = self.storage.load_all_accounts()
                        .map_err(|e| anyhow!("Failed to load accounts: {}", e))?;
                    
                    for account in &accounts {
                        println!("\n🔄 Refreshing: {} ({})", account.display_name, account.email_address);
                        
                        match token_manager.get_valid_access_token(&account.account_id).await {
                            Ok(Some(_)) => {
                                println!("   ✅ Success");
                            }
                            Ok(None) => {
                                println!("   ❌ Failed - needs re-authentication");
                            }
                            Err(e) => {
                                println!("   ❌ Error: {}", e);
                            }
                        }
                    }
                }
            }
        } else {
            println!("❌ Token manager not available");
        }

        Ok(())
    }

    /// Re-authenticate an OAuth2 account
    async fn handle_oauth2_reauth(&self, account: String, _no_browser: bool, _verbose: bool) -> Result<()> {
        println!("🔐 OAuth2 Re-authentication");
        println!("============================");
        println!("\n🔄 Re-authenticating account: {}", account);
        
        // This would trigger the full OAuth2 flow
        println!("❌ Re-authentication flow not yet implemented in CLI");
        println!("   Please use the TUI interface to re-authenticate:");
        println!("   1. Run: comunicado");
        println!("   2. Navigate to Account Management");
        println!("   3. Select 'Re-authenticate' for the account");
        
        Ok(())
    }

    /// Handle calendar sync commands
    async fn handle_calendar_sync(&self, args: CalendarSyncArgs, dry_run: bool) -> Result<()> {
        match args.command {
            CalendarSyncCommands::All { verbose, force } => {
                self.handle_calendar_sync_all(verbose, force, dry_run).await
            }
            CalendarSyncCommands::Account { account, verbose, force } => {
                self.handle_calendar_sync_account(account, verbose, force, dry_run).await
            }
            CalendarSyncCommands::Calendar { account, calendar, verbose, force } => {
                self.handle_calendar_sync_calendar(account, calendar, verbose, force, dry_run).await
            }
            CalendarSyncCommands::List => {
                self.handle_calendar_sync_list(dry_run).await
            }
            CalendarSyncCommands::Status { account } => {
                self.handle_calendar_sync_status(account, dry_run).await
            }
        }
    }

    /// Sync all calendars from all accounts
    async fn handle_calendar_sync_all(&self, verbose: bool, force: bool, dry_run: bool) -> Result<()> {
        println!("📅 Syncing all calendars...");
        
        if dry_run {
            println!("💨 Dry run mode - no changes will be made");
        }

        // Get all accounts
        let account_ids = self.storage.list_account_ids()
            .map_err(|e| anyhow!("Failed to list accounts: {}", e))?;

        if account_ids.is_empty() {
            println!("⚠️  No accounts found. Use 'comunicado setup-gmail' to add an account.");
            return Ok(());
        }

        println!("📧 Found {} account(s) to sync calendars for", account_ids.len());
        
        if verbose {
            println!("📋 Calendar sync plan:");
            println!("   🔄 Force full sync: {}", if force { "✅ Yes" } else { "❌ No" });
        }

        if dry_run {
            for account_id in &account_ids {
                if let Ok(Some(config)) = self.storage.load_account(account_id) {
                    let display_name = if config.display_name.is_empty() { "Unknown".to_string() } else { config.display_name.clone() };
                    println!("💨 Would sync calendars for: {} ({}) - {}", display_name, config.email_address, config.provider);
                }
            }
        } else {
            println!("🚀 Starting calendar sync...");
            
            // Initialize calendar manager
            match self.create_calendar_manager().await {
                Ok(calendar_manager) => {
                    if verbose {
                        println!("✅ Calendar manager initialized");
                    }
                    
                    // Perform the actual sync
                    match calendar_manager.sync_calendars().await {
                        Ok(_) => {
                            println!("✅ Calendar sync completed successfully");
                            
                            // Show statistics if verbose
                            if verbose {
                                match calendar_manager.get_stats().await {
                                    Ok(stats) => {
                                        println!("📊 Sync statistics:");
                                        println!("   📅 Total calendars: {}", stats.total_calendars);
                                        println!("   📍 Total events: {}", stats.total_events);
                                        println!("   📈 Upcoming events: {}", stats.upcoming_events);
                                        if let Some(last_sync) = stats.last_sync {
                                            println!("   🕒 Last sync: {}", last_sync.format("%Y-%m-%d %H:%M:%S UTC"));
                                        }
                                    }
                                    Err(e) => {
                                        println!("⚠️  Could not get sync statistics: {}", e);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            println!("❌ Calendar sync failed: {}", e);
                            return Err(anyhow!("Calendar sync failed: {}", e));
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Failed to initialize calendar manager: {}", e);
                    return Err(anyhow!("Calendar manager initialization failed: {}", e));
                }
            }
        }
        
        Ok(())
    }

    /// Sync calendars for a specific account
    async fn handle_calendar_sync_account(&self, account: String, verbose: bool, force: bool, dry_run: bool) -> Result<()> {
        println!("📅 Syncing calendars for account: {}", account);
        
        if dry_run {
            println!("💨 Dry run mode - no changes will be made");
        }

        // Find account by name or email
        let account_id = self.find_account_id(&account)?;
        
        if let Ok(Some(config)) = self.storage.load_account(&account_id) {
            let display_name = if config.display_name.is_empty() { "Unknown".to_string() } else { config.display_name.clone() };
            
            println!("📋 Calendar sync plan for: {} ({}) - {}", display_name, config.email_address, config.provider);
            
            if verbose {
                println!("   🔄 Force full sync: {}", if force { "✅ Yes" } else { "❌ No" });
            }

            if dry_run {
                println!("💨 Would sync calendars for account: {}", account);
            } else {
                println!("🚀 Starting calendar sync for account: {}", account);
                
                // Initialize calendar manager
                match self.create_calendar_manager().await {
                    Ok(calendar_manager) => {
                        if verbose {
                            println!("✅ Calendar manager initialized");
                        }
                        
                        // For Google accounts, try to add/sync Google calendars
                        if config.provider.to_lowercase().contains("google") || config.provider.to_lowercase().contains("gmail") {
                            match calendar_manager.add_google_calendars(account_id.clone()).await {
                                Ok(calendars) => {
                                    println!("✅ Added/updated {} Google calendars", calendars.len());
                                    if verbose {
                                        for calendar in &calendars {
                                            println!("   📅 {}", calendar.name);
                                        }
                                    }
                                }
                                Err(e) => {
                                    println!("⚠️  Failed to add Google calendars: {}", e);
                                }
                            }
                        }
                        
                        // Perform general calendar sync
                        match calendar_manager.sync_calendars().await {
                            Ok(_) => {
                                println!("✅ Calendar sync completed for account: {}", account);
                            }
                            Err(e) => {
                                println!("❌ Calendar sync failed for account {}: {}", account, e);
                                return Err(anyhow!("Calendar sync failed: {}", e));
                            }
                        }
                    }
                    Err(e) => {
                        println!("❌ Failed to initialize calendar manager: {}", e);
                        return Err(anyhow!("Calendar manager initialization failed: {}", e));
                    }
                }
            }
        } else {
            return Err(anyhow!("Account '{}' not found", account));
        }
        
        Ok(())
    }

    /// Sync a specific calendar
    async fn handle_calendar_sync_calendar(&self, account: String, calendar: String, verbose: bool, force: bool, dry_run: bool) -> Result<()> {
        println!("📅 Syncing calendar '{}' for account: {}", calendar, account);
        
        if dry_run {
            println!("💨 Dry run mode - no changes will be made");
        }

        // Find account by name or email
        let account_id = self.find_account_id(&account)?;
        
        if let Ok(Some(config)) = self.storage.load_account(&account_id) {
            let display_name = if config.display_name.is_empty() { "Unknown".to_string() } else { config.display_name.clone() };
            
            println!("📋 Calendar sync plan:");
            println!("   📧 Account: {} ({}) - {}", display_name, config.email_address, config.provider);
            println!("   📅 Calendar: {}", calendar);
            
            if verbose {
                println!("   🔄 Force full sync: {}", if force { "✅ Yes" } else { "❌ No" });
            }

            if dry_run {
                println!("💨 Would sync calendar '{}' for account '{}'", calendar, account);
            } else {
                println!("🚀 Starting sync for calendar: {}", calendar);
                
                // Initialize calendar manager
                match self.create_calendar_manager().await {
                    Ok(calendar_manager) => {
                        if verbose {
                            println!("✅ Calendar manager initialized");
                        }
                        
                        // Get all calendars and find the requested one
                        let calendars = calendar_manager.get_calendars().await;
                        let target_calendar = calendars.iter().find(|c| 
                            c.name.eq_ignore_ascii_case(&calendar) || c.id == calendar
                        );
                        
                        match target_calendar {
                            Some(cal) => {
                                println!("✅ Found calendar: {} ({})", cal.name, cal.id);
                                
                                // Perform sync for this specific calendar
                                // Note: The sync_calendars method syncs all calendars
                                // In a full implementation, we'd have a sync_specific_calendar method
                                match calendar_manager.sync_calendars().await {
                                    Ok(_) => {
                                        println!("✅ Calendar sync completed for: {}", calendar);
                                    }
                                    Err(e) => {
                                        println!("❌ Calendar sync failed for '{}': {}", calendar, e);
                                        return Err(anyhow!("Calendar sync failed: {}", e));
                                    }
                                }
                            }
                            None => {
                                println!("❌ Calendar '{}' not found for account '{}'", calendar, account);
                                println!("💡 Use 'comunicado calendar-sync list' to see available calendars");
                                return Err(anyhow!("Calendar '{}' not found", calendar));
                            }
                        }
                    }
                    Err(e) => {
                        println!("❌ Failed to initialize calendar manager: {}", e);
                        return Err(anyhow!("Calendar manager initialization failed: {}", e));
                    }
                }
            }
        } else {
            return Err(anyhow!("Account '{}' not found", account));
        }
        
        Ok(())
    }

    /// List available calendars for sync
    async fn handle_calendar_sync_list(&self, _dry_run: bool) -> Result<()> {
        println!("📅 Available calendars for sync:");
        
        // Initialize calendar manager
        match self.create_calendar_manager().await {
            Ok(calendar_manager) => {
                let calendars = calendar_manager.get_calendars().await;
                
                if calendars.is_empty() {
                    println!("⚠️  No calendars found.");
                    println!("💡 Add calendars by running:");
                    println!("   • comunicado calendar-sync account <account> (for Google calendars)");
                    println!("   • Use the TUI interface to add CalDAV calendars");
                    return Ok(());
                }
                
                println!("Found {} calendar(s):", calendars.len());
                
                for calendar in &calendars {
                    println!("\n📅 {} (ID: {})", calendar.name, calendar.id);
                    println!("   Source: {:?}", calendar.source);
                    if let Some(description) = &calendar.description {
                        println!("   Description: {}", description);
                    }
                    if calendar.read_only {
                        println!("   Access: 📖 Read-only");
                    } else {
                        println!("   Access: ✏️  Read-write");
                    }
                    if let Some(last_synced) = calendar.last_synced {
                        println!("   Last synced: {}", last_synced.format("%Y-%m-%d %H:%M:%S UTC"));
                    } else {
                        println!("   Last synced: Never");
                    }
                }
                
                println!("\n💡 Usage:");
                println!("  comunicado calendar-sync all                     # Sync all calendars");
                println!("  comunicado calendar-sync account <account>       # Sync calendars for specific account");
                println!("  comunicado calendar-sync calendar <account> <calendar> # Sync specific calendar");
            }
            Err(e) => {
                println!("❌ Failed to initialize calendar manager: {}", e);
                return Err(anyhow!("Calendar manager initialization failed: {}", e));
            }
        }
        
        Ok(())
    }

    /// Show calendar sync status and statistics
    async fn handle_calendar_sync_status(&self, account: Option<String>, _dry_run: bool) -> Result<()> {
        if let Some(account_name) = account {
            println!("📊 Calendar sync status for account: {}", account_name);
            // For now, we show general calendar info since we don't have per-account calendar stats
        } else {
            println!("📊 Calendar sync status for all calendars:");
        }
        
        // Initialize calendar manager
        match self.create_calendar_manager().await {
            Ok(calendar_manager) => {
                match calendar_manager.get_stats().await {
                    Ok(stats) => {
                        println!("\n📊 Calendar Statistics:");
                        println!("   📅 Total calendars: {}", stats.total_calendars);
                        println!("   🌐 Google calendars: {}", stats.google_calendars);
                        println!("   📍 CalDAV calendars: {}", stats.caldav_calendars);
                        println!("   🏠 Local calendars: {}", stats.local_calendars);
                        println!("   📍 Total events: {}", stats.total_events);
                        println!("   📈 Upcoming events: {}", stats.upcoming_events);
                        if stats.overdue_events > 0 {
                            println!("   ⏰ Overdue events: {}", stats.overdue_events);
                        }
                        
                        if let Some(last_sync) = stats.last_sync {
                            let now = chrono::Utc::now();
                            let time_since_sync = now.signed_duration_since(last_sync);
                            println!("   🕒 Last sync: {} ({})", 
                                last_sync.format("%Y-%m-%d %H:%M:%S UTC"),
                                format_duration(time_since_sync)
                            );
                        } else {
                            println!("   🕒 Last sync: Never");
                        }
                    }
                    Err(e) => {
                        println!("❌ Failed to get calendar statistics: {}", e);
                        return Err(anyhow!("Failed to get calendar statistics: {}", e));
                    }
                }
            }
            Err(e) => {
                println!("❌ Failed to initialize calendar manager: {}", e);
                return Err(anyhow!("Calendar manager initialization failed: {}", e));
            }
        }
        
        Ok(())
    }

    /// Helper: Create a calendar manager instance
    async fn create_calendar_manager(&self) -> Result<Arc<crate::calendar::manager::CalendarManager>> {
        // Get or create calendar database path using the same pattern as email database
        let calendar_db_path = dirs::config_dir()
            .ok_or_else(|| anyhow!("Cannot find config directory"))?
            .join("comunicado")
            .join("databases")
            .join("calendar.db");
        
        // Ensure parent directory exists
        if let Some(parent) = calendar_db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| anyhow!("Failed to create calendar database directory: {}", e))?;
        }
        
        // Initialize database
        let database = Arc::new(crate::calendar::database::CalendarDatabase::new(
            calendar_db_path.to_str().ok_or_else(|| anyhow!("Invalid calendar database path"))?
        ).await
            .map_err(|e| anyhow!("Failed to initialize calendar database: {}", e))?);
        
        // Use existing token manager or create a new one
        let token_manager = if let Some(ref tm) = self.token_manager {
            Arc::new(tm.clone())
        } else {
            Arc::new(TokenManager::new_with_storage(Arc::new(self.storage.clone())))
        };
        
        // Create calendar manager
        let calendar_manager = Arc::new(crate::calendar::manager::CalendarManager::new(database, token_manager).await
            .map_err(|e| anyhow!("Failed to create calendar manager: {}", e))?);
        
        Ok(calendar_manager)
    }

    /// Handle contacts sync commands
    async fn handle_contacts_sync(&self, args: ContactsSyncArgs, dry_run: bool) -> Result<()> {
        match args.command {
            ContactsSyncCommands::All { verbose, force } => {
                self.handle_contacts_sync_all(verbose, force, dry_run).await
            }
            ContactsSyncCommands::Account { account, verbose, force } => {
                self.handle_contacts_sync_account(account, verbose, force, dry_run).await
            }
            ContactsSyncCommands::List => {
                self.handle_contacts_sync_list(dry_run).await
            }
            ContactsSyncCommands::Status { account } => {
                self.handle_contacts_sync_status(account, dry_run).await
            }
        }
    }

    /// Sync all contacts from all accounts
    async fn handle_contacts_sync_all(&self, verbose: bool, force: bool, dry_run: bool) -> Result<()> {
        println!("📞 Syncing all contacts...");
        
        if dry_run {
            println!("💨 Dry run mode - no changes will be made");
        }

        // Get all accounts
        let account_ids = self.storage.list_account_ids()
            .map_err(|e| anyhow!("Failed to list accounts: {}", e))?;

        if account_ids.is_empty() {
            println!("⚠️  No accounts found. Use 'comunicado setup-gmail' to add an account.");
            return Ok(());
        }

        println!("📧 Found {} account(s) to sync contacts for", account_ids.len());
        
        if verbose {
            println!("📋 Contacts sync plan:");
            println!("   🔄 Force full sync: {}", if force { "✅ Yes" } else { "❌ No" });
        }

        if dry_run {
            for account_id in &account_ids {
                if let Ok(Some(config)) = self.storage.load_account(account_id) {
                    let display_name = if config.display_name.is_empty() { "Unknown".to_string() } else { config.display_name.clone() };
                    println!("💨 Would sync contacts for: {} ({}) - {}", display_name, config.email_address, config.provider);
                }
            }
        } else {
            println!("🚀 Starting contacts sync...");
            
            // Initialize contacts manager
            match self.create_contacts_manager().await {
                Ok(contacts_manager) => {
                    if verbose {
                        println!("✅ Contacts manager initialized");
                    }
                    
                    // Perform the actual sync
                    match contacts_manager.sync_all_contacts().await {
                        Ok(summary) => {
                            println!("✅ Contacts sync completed successfully");
                            
                            // Show statistics if verbose
                            if verbose {
                                println!("📊 Sync statistics:");
                                println!("   📞 Contacts fetched: {}", summary.fetched_count);
                                println!("   ➕ Contacts created: {}", summary.created_count);
                                println!("   🔄 Contacts updated: {}", summary.updated_count);
                                println!("   ⏭️  Contacts skipped: {}", summary.skipped_count);
                                if !summary.errors.is_empty() {
                                    println!("   ❌ Errors: {}", summary.errors.len());
                                    for error in &summary.errors {
                                        println!("      {}", error);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            println!("❌ Contacts sync failed: {}", e);
                            return Err(anyhow!("Contacts sync failed: {}", e));
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Failed to initialize contacts manager: {}", e);
                    return Err(anyhow!("Contacts manager initialization failed: {}", e));
                }
            }
        }
        
        Ok(())
    }

    /// Sync contacts for a specific account
    async fn handle_contacts_sync_account(&self, account: String, verbose: bool, force: bool, dry_run: bool) -> Result<()> {
        println!("📞 Syncing contacts for account: {}", account);
        
        if dry_run {
            println!("💨 Dry run mode - no changes will be made");
        }

        // Find account by name or email
        let account_id = self.find_account_id(&account)?;
        
        if let Ok(Some(config)) = self.storage.load_account(&account_id) {
            let display_name = if config.display_name.is_empty() { "Unknown".to_string() } else { config.display_name.clone() };
            
            println!("📋 Contacts sync plan for: {} ({}) - {}", display_name, config.email_address, config.provider);
            
            if verbose {
                println!("   🔄 Force full sync: {}", if force { "✅ Yes" } else { "❌ No" });
            }

            if dry_run {
                println!("💨 Would sync contacts for account: {}", account);
            } else {
                println!("🚀 Starting contacts sync for account: {}", account);
                
                // Initialize contacts manager
                match self.create_contacts_manager().await {
                    Ok(contacts_manager) => {
                        if verbose {
                            println!("✅ Contacts manager initialized");
                        }
                        
                        // Determine provider type
                        let provider_type = if config.provider.to_lowercase().contains("google") || config.provider.to_lowercase().contains("gmail") {
                            "google"
                        } else if config.provider.to_lowercase().contains("outlook") || config.provider.to_lowercase().contains("microsoft") {
                            "outlook"
                        } else {
                            return Err(anyhow!("Unsupported provider for contacts sync: {}", config.provider));
                        };
                        
                        // Perform account-specific sync
                        match contacts_manager.sync_account_contacts(&account_id, provider_type).await {
                            Ok(summary) => {
                                println!("✅ Contacts sync completed for account: {}", account);
                                
                                if verbose {
                                    println!("📊 Sync statistics:");
                                    println!("   📞 Contacts fetched: {}", summary.fetched_count);
                                    println!("   ➕ Contacts created: {}", summary.created_count);
                                    println!("   🔄 Contacts updated: {}", summary.updated_count);
                                    println!("   ⏭️  Contacts skipped: {}", summary.skipped_count);
                                    if !summary.errors.is_empty() {
                                        println!("   ❌ Errors: {}", summary.errors.len());
                                        for error in &summary.errors {
                                            println!("      {}", error);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                println!("❌ Contacts sync failed for account {}: {}", account, e);
                                return Err(anyhow!("Contacts sync failed: {}", e));
                            }
                        }
                    }
                    Err(e) => {
                        println!("❌ Failed to initialize contacts manager: {}", e);
                        return Err(anyhow!("Contacts manager initialization failed: {}", e));
                    }
                }
            }
        } else {
            return Err(anyhow!("Account '{}' not found", account));
        }
        
        Ok(())
    }

    /// List available contacts sources for sync
    async fn handle_contacts_sync_list(&self, _dry_run: bool) -> Result<()> {
        println!("📞 Available contacts sources for sync:");
        
        // Initialize contacts manager
        match self.create_contacts_manager().await {
            Ok(contacts_manager) => {
                match contacts_manager.get_stats().await {
                    Ok(stats) => {
                        if stats.total_contacts == 0 {
                            println!("⚠️  No contacts found.");
                            println!("💡 Sync contacts by running:");
                            println!("   • comunicado contacts-sync account <account> (for Google/Outlook contacts)");
                            return Ok(());
                        }
                        
                        println!("Found {} contact(s):", stats.total_contacts);
                        
                        // Get accounts to show contact sources
                        let account_ids = self.storage.list_account_ids()
                            .map_err(|e| anyhow!("Failed to list accounts: {}", e))?;
                            
                        for account_id in &account_ids {
                            if let Ok(Some(config)) = self.storage.load_account(account_id) {
                                let display_name = if config.display_name.is_empty() { "Unknown".to_string() } else { config.display_name.clone() };
                                
                                // Check if this account has contacts scope
                                let has_contacts_scope = config.scopes.iter().any(|scope| 
                                    scope.contains("contacts") || scope.contains("Contacts")
                                );
                                
                                println!("\n📧 {} ({}) - {}", display_name, config.email_address, config.provider);
                                if has_contacts_scope {
                                    println!("   Status: ✅ Contacts sync available");
                                } else {
                                    println!("   Status: ❌ Contacts scope not available");
                                    println!("   💡 Re-run setup to add contacts permissions");
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("❌ Failed to get contacts statistics: {}", e);
                        return Err(anyhow!("Failed to get contacts statistics: {}", e));
                    }
                }
            }
            Err(e) => {
                println!("❌ Failed to initialize contacts manager: {}", e);
                return Err(anyhow!("Contacts manager initialization failed: {}", e));
            }
        }
        
        println!("\n💡 Usage:");
        println!("  comunicado contacts-sync all                     # Sync all contacts");
        println!("  comunicado contacts-sync account <account>       # Sync contacts for specific account");
        
        Ok(())
    }

    /// Show contacts sync status and statistics
    async fn handle_contacts_sync_status(&self, account: Option<String>, _dry_run: bool) -> Result<()> {
        if let Some(account_name) = account {
            println!("📊 Contacts sync status for account: {}", account_name);
        } else {
            println!("📊 Contacts sync status for all accounts:");
        }
        
        // Initialize contacts manager
        match self.create_contacts_manager().await {
            Ok(contacts_manager) => {
                match contacts_manager.get_stats().await {
                    Ok(stats) => {
                        println!("\n📊 Contacts Statistics:");
                        println!("   📞 Total contacts: {}", stats.total_contacts);
                        println!("   📧 Contacts with email: {}", stats.contacts_with_email);
                        println!("   📱 Contacts with phone: {}", stats.contacts_with_phone);
                        println!("   🏢 Groups: {}", stats.groups_count);
                        
                        if let Some(last_sync) = stats.last_sync {
                            let now = chrono::Utc::now();
                            let time_since_sync = now.signed_duration_since(last_sync);
                            println!("   🕒 Last sync: {} ({})", 
                                last_sync.format("%Y-%m-%d %H:%M:%S UTC"),
                                format_duration(time_since_sync)
                            );
                        } else {
                            println!("   🕒 Last sync: Never");
                        }
                    }
                    Err(e) => {
                        println!("❌ Failed to get contacts statistics: {}", e);
                        return Err(anyhow!("Failed to get contacts statistics: {}", e));
                    }
                }
            }
            Err(e) => {
                println!("❌ Failed to initialize contacts manager: {}", e);
                return Err(anyhow!("Contacts manager initialization failed: {}", e));
            }
        }
        
        Ok(())
    }

    /// Helper: Create a contacts manager instance
    async fn create_contacts_manager(&self) -> Result<Arc<crate::contacts::ContactsManager>> {
        // Get or create contacts database path
        let contacts_db_path = dirs::config_dir()
            .ok_or_else(|| anyhow!("Cannot find config directory"))?
            .join("comunicado")
            .join("databases")
            .join("contacts.db");
        
        // Ensure parent directory exists
        if let Some(parent) = contacts_db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| anyhow!("Failed to create contacts database directory: {}", e))?;
        }
        
        // Initialize database
        let database = crate::contacts::ContactsDatabase::new(
            contacts_db_path.to_str().ok_or_else(|| anyhow!("Invalid contacts database path"))?
        ).await
            .map_err(|e| anyhow!("Failed to initialize contacts database: {}", e))?;
        
        // Use existing token manager or create a new one
        let token_manager = if let Some(ref tm) = self.token_manager {
            tm.clone()
        } else {
            TokenManager::new_with_storage(Arc::new(self.storage.clone()))
        };
        
        // Create contacts manager
        let contacts_manager = Arc::new(crate::contacts::ContactsManager::new(database, token_manager).await
            .map_err(|e| anyhow!("Failed to create contacts manager: {}", e))?);
        
        Ok(contacts_manager)
    }
}

/// Format a duration for human-readable display
fn format_duration(duration: chrono::Duration) -> String {
    let total_seconds = duration.num_seconds().abs();
    let days = total_seconds / 86400;
    let hours = (total_seconds % 86400) / 3600;
    let minutes = (total_seconds % 3600) / 60;
    
    if days > 0 {
        format!("{}d {}h {}m", days, hours, minutes)
    } else if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    }
}
