pub mod client;
pub mod message;
pub mod providers;
pub mod service;

pub use client::{SmtpClient, SmtpConfig};
pub use message::{EmailMessage, MessageBuilder};
pub use providers::{SmtpProviderRegistry, SmtpProviderConfig};
pub use service::{SmtpService, SmtpServiceBuilder};

use thiserror::Error;

/// SMTP-related errors
#[derive(Error, Debug)]
pub enum SmtpError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    
    #[error("Message send failed: {0}")]
    SendFailed(String),
    
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    
    #[error("Invalid email address: {0}")]
    InvalidAddress(String),
    
    #[error("Message formatting error: {0}")]
    MessageFormatError(String),
    
    #[error("OAuth2 error: {0}")]
    OAuth2Error(String),
    
    #[error("Network error: {0}")]
    NetworkError(#[from] lettre::transport::smtp::Error),
    
    #[error("Address parse error: {0}")]
    AddressParseError(#[from] lettre::address::AddressError),
    
    #[error("Message build error: {0}")]
    MessageBuildError(#[from] lettre::error::Error),
}

pub type SmtpResult<T> = Result<T, SmtpError>;

/// SMTP authentication methods
#[derive(Debug, Clone)]
pub enum SmtpAuth {
    /// OAuth2 authentication with access token
    OAuth2 {
        username: String,
        access_token: String,
    },
    /// Plain username/password authentication
    Plain {
        username: String,
        password: String,
    },
    /// Login authentication
    Login {
        username: String,
        password: String,
    },
}

/// SMTP connection security
#[derive(Debug, Clone, PartialEq)]
pub enum SmtpSecurity {
    /// No encryption (not recommended)
    None,
    /// STARTTLS (opportunistic encryption)
    StartTls,
    /// Direct TLS connection
    Tls,
}

impl Default for SmtpSecurity {
    fn default() -> Self {
        SmtpSecurity::StartTls
    }
}

/// Send result information
#[derive(Debug, Clone)]
pub struct SendResult {
    pub message_id: String,
    pub accepted_recipients: Vec<String>,
    pub rejected_recipients: Vec<String>,
    pub sent_at: chrono::DateTime<chrono::Utc>,
}

impl SendResult {
    pub fn is_success(&self) -> bool {
        !self.accepted_recipients.is_empty() && self.rejected_recipients.is_empty()
    }
    
    pub fn is_partial_success(&self) -> bool {
        !self.accepted_recipients.is_empty() && !self.rejected_recipients.is_empty()
    }
    
    pub fn is_failure(&self) -> bool {
        self.accepted_recipients.is_empty()
    }
}