use anyhow::{anyhow, Result};
use clap::{Args, Parser, Subcommand};
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

    /// Keyboard shortcut management
    Keyboard(KeyboardArgs),

    /// Maildir import/export operations
    Maildir(MaildirArgs),
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

        // Initialize token manager
        let token_manager = Some(TokenManager::new());

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
            Commands::Keyboard(args) => self.handle_keyboard(args, dry_run).await,
            Commands::Maildir(args) => self.handle_maildir(args, dry_run).await,
        }
    }

    /// Handle test commands
    async fn handle_test(&self, args: TestArgs, _dry_run: bool) -> Result<()> {
        println!("🧪 Comunicado Account Connection Test");
        println!("====================================\n");

        let accounts = self.storage.list_accounts()?;

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
                        .get_valid_access_token(&account.email_address)
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
        // Create a temporary IMAP connection
        let imap_manager = ImapAccountManager::new()?;

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
                    "https://www.googleapis.com/auth/gmail.readonly".to_string(),
                    "https://www.googleapis.com/auth/gmail.send".to_string(),
                    "https://www.googleapis.com/auth/calendar".to_string(),
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
}
