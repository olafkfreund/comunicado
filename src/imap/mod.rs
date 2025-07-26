pub mod client;
pub mod connection;
pub mod protocol;
pub mod types;
pub mod error;
pub mod account_manager;
pub mod idle;

pub use client::ImapClient;
pub use connection::ImapConnection;
pub use account_manager::{ImapAccountManager, ImapAccount, AccountManagerStats};
pub use idle::{IdleNotification, IdleNotificationService, IdleManager, IdleStats};
pub use types::*;
pub use error::{ImapError, ImapResult};

/// IMAP capability flags
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImapCapability {
    Imap4Rev1,
    StartTls,
    LoginDisabled,
    SaslIr,
    AuthPlain,
    AuthLogin,
    AuthXOAuth2,
    Idle,
    Namespace,
    Unselect,
    Children,
    UidPlus,
    CondStore,
    QResync,
    Move,
    Special,
    Custom(String),
}

impl ImapCapability {
    /// Parse capability string into ImapCapability enum
    pub fn from_str(capability: &str) -> Self {
        match capability.to_uppercase().as_str() {
            "IMAP4REV1" => ImapCapability::Imap4Rev1,
            "STARTTLS" => ImapCapability::StartTls,
            "LOGINDISABLED" => ImapCapability::LoginDisabled,
            "SASL-IR" => ImapCapability::SaslIr,
            "AUTH=PLAIN" => ImapCapability::AuthPlain,
            "AUTH=LOGIN" => ImapCapability::AuthLogin,
            "AUTH=XOAUTH2" => ImapCapability::AuthXOAuth2,
            "IDLE" => ImapCapability::Idle,
            "NAMESPACE" => ImapCapability::Namespace,
            "UNSELECT" => ImapCapability::Unselect,
            "CHILDREN" => ImapCapability::Children,
            "UIDPLUS" => ImapCapability::UidPlus,
            "CONDSTORE" => ImapCapability::CondStore,
            "QRESYNC" => ImapCapability::QResync,
            "MOVE" => ImapCapability::Move,
            "SPECIAL-USE" => ImapCapability::Special,
            _ => ImapCapability::Custom(capability.to_string()),
        }
    }
}

/// IMAP authentication method
#[derive(Debug, Clone)]
pub enum ImapAuthMethod {
    Password(String),
    OAuth2 { account_id: String },
}

/// IMAP server configuration
#[derive(Debug, Clone)]
pub struct ImapConfig {
    pub hostname: String,
    pub port: u16,
    pub username: String,
    pub auth_method: ImapAuthMethod,
    pub use_tls: bool,
    pub use_starttls: bool,
    pub timeout_seconds: u64,
    pub validate_certificates: bool,
}

impl ImapConfig {
    /// Create a new IMAP configuration with password auth
    pub fn new(hostname: String, port: u16, username: String, password: String) -> Self {
        Self {
            hostname,
            port,
            username,
            auth_method: ImapAuthMethod::Password(password),
            use_tls: port == 993, // Default to TLS for port 993
            use_starttls: port == 143, // Default to STARTTLS for port 143
            timeout_seconds: 60, // Increased timeout for better reliability
            validate_certificates: true,
        }
    }
    
    /// Create a new IMAP configuration with OAuth2 auth
    pub fn new_oauth2(hostname: String, port: u16, username: String, account_id: String) -> Self {
        Self {
            hostname,
            port,
            username,
            auth_method: ImapAuthMethod::OAuth2 { account_id },
            use_tls: port == 993,
            use_starttls: port == 143,
            timeout_seconds: 60, // Increased timeout for better reliability
            validate_certificates: true,
        }
    }
    
    /// Create configuration for Gmail with password
    pub fn gmail(username: String, password: String) -> Self {
        Self::new("imap.gmail.com".to_string(), 993, username, password)
    }
    
    /// Create configuration for Gmail with OAuth2
    pub fn gmail_oauth2(username: String, account_id: String) -> Self {
        Self::new_oauth2("imap.gmail.com".to_string(), 993, username, account_id)
    }
    
    /// Create configuration for Outlook/Hotmail with password
    pub fn outlook(username: String, password: String) -> Self {
        Self::new("outlook.office365.com".to_string(), 993, username, password)
    }
    
    /// Create configuration for Outlook/Hotmail with OAuth2
    pub fn outlook_oauth2(username: String, account_id: String) -> Self {
        Self::new_oauth2("outlook.office365.com".to_string(), 993, username, account_id)
    }
    
    /// Create configuration for Yahoo with password
    pub fn yahoo(username: String, password: String) -> Self {
        Self::new("imap.mail.yahoo.com".to_string(), 993, username, password)
    }
    
    /// Create configuration for Yahoo with OAuth2
    pub fn yahoo_oauth2(username: String, account_id: String) -> Self {
        Self::new_oauth2("imap.mail.yahoo.com".to_string(), 993, username, account_id)
    }
    
    /// Enable TLS encryption
    pub fn with_tls(mut self, use_tls: bool) -> Self {
        self.use_tls = use_tls;
        self
    }
    
    /// Enable STARTTLS upgrade
    pub fn with_starttls(mut self, use_starttls: bool) -> Self {
        self.use_starttls = use_starttls;
        self
    }
    
    /// Set connection timeout
    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = timeout_seconds;
        self
    }
    
    /// Set certificate validation
    pub fn with_certificate_validation(mut self, validate: bool) -> Self {
        self.validate_certificates = validate;
        self
    }
}