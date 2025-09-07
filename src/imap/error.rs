use std::fmt;
use std::io;
use thiserror::Error;

pub type ImapResult<T> = Result<T, ImapError>;

/// IMAP client errors
#[derive(Error, Debug)]
pub enum ImapError {
    /// IO error (network, file system, etc.)
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// Connection error
    #[error("Connection error: {0}")]
    Connection(String),

    /// Authentication error
    #[error("Authentication failed: {0}")]
    Authentication(String),

    /// Protocol error (invalid response, etc.)
    #[error("Protocol error: {0}")]
    Protocol(String),

    /// Server error (NO/BAD response)
    #[error("Server error: {0}")]
    Server(String),

    /// TLS/SSL error
    #[error("TLS error: {0}")]
    Tls(String),

    /// Timeout error
    #[error("Operation timed out")]
    Timeout,

    /// Parse error
    #[error("Parse error: {0}")]
    Parse(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Feature not supported
    #[error("Feature not supported: {0}")]
    NotSupported(String),

    /// Folder not found
    #[error("Folder not found: {0}")]
    FolderNotFound(String),

    /// Message not found
    #[error("Message not found: {0}")]
    MessageNotFound(String),

    /// Invalid state for operation
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// Generic error with custom message
    #[error("Error: {0}")]
    Generic(String),
}

impl ImapError {
    /// Create a new connection error
    pub fn connection<S: Into<String>>(msg: S) -> Self {
        ImapError::Connection(msg.into())
    }

    /// Create a new authentication error
    pub fn authentication<S: Into<String>>(msg: S) -> Self {
        ImapError::Authentication(msg.into())
    }

    /// Create a new protocol error
    pub fn protocol<S: Into<String>>(msg: S) -> Self {
        ImapError::Protocol(msg.into())
    }

    /// Create a new server error
    pub fn server<S: Into<String>>(msg: S) -> Self {
        ImapError::Server(msg.into())
    }

    /// Create a new TLS error
    pub fn tls<S: Into<String>>(msg: S) -> Self {
        ImapError::Tls(msg.into())
    }

    /// Create a new parse error
    pub fn parse<S: Into<String>>(msg: S) -> Self {
        ImapError::Parse(msg.into())
    }

    /// Create a new invalid configuration error
    pub fn invalid_config<S: Into<String>>(msg: S) -> Self {
        ImapError::InvalidConfig(msg.into())
    }

    /// Create a new not supported error
    pub fn not_supported<S: Into<String>>(msg: S) -> Self {
        ImapError::NotSupported(msg.into())
    }

    /// Create a new folder not found error
    pub fn folder_not_found<S: Into<String>>(folder: S) -> Self {
        ImapError::FolderNotFound(folder.into())
    }

    /// Create a new message not found error
    pub fn message_not_found<S: Into<String>>(msg: S) -> Self {
        ImapError::MessageNotFound(msg.into())
    }

    /// Create a new invalid state error
    pub fn invalid_state<S: Into<String>>(msg: S) -> Self {
        ImapError::InvalidState(msg.into())
    }

    /// Create a generic error
    pub fn generic<S: Into<String>>(msg: S) -> Self {
        ImapError::Generic(msg.into())
    }

    /// Create a storage error (alias for generic)
    pub fn storage<S: Into<String>>(msg: S) -> Self {
        ImapError::Generic(msg.into())
    }

    /// Create a not found error (alias for generic)
    pub fn not_found<S: Into<String>>(msg: S) -> Self {
        ImapError::Generic(msg.into())
    }

    /// Check if this is a recoverable error
    pub fn is_recoverable(&self) -> bool {
        match self {
            ImapError::Io(_) => true,
            ImapError::Connection(_) => true,
            ImapError::Timeout => true,
            ImapError::Tls(_) => false, // TLS errors usually require reconfiguration
            ImapError::Authentication(_) => false, // Auth errors require new credentials
            ImapError::Protocol(_) => false, // Protocol errors indicate bugs
            ImapError::Server(_) => true, // Server errors might be temporary
            ImapError::Parse(_) => false, // Parse errors indicate bugs
            ImapError::InvalidConfig(_) => false, // Config errors require fixes
            ImapError::NotSupported(_) => false, // Feature not available
            ImapError::FolderNotFound(_) => false, // Folder doesn't exist
            ImapError::MessageNotFound(_) => false, // Message doesn't exist
            ImapError::InvalidState(_) => true, // State might be fixable
            ImapError::Generic(_) => true, // Unknown, assume recoverable
        }
    }

    /// Check if this is a connection-related error
    pub fn is_connection_error(&self) -> bool {
        matches!(
            self,
            ImapError::Io(_) | ImapError::Connection(_) | ImapError::Timeout | ImapError::Tls(_)
        )
    }

    /// Check if this is an authentication error
    pub fn is_auth_error(&self) -> bool {
        matches!(self, ImapError::Authentication(_))
    }

    /// Get user-friendly recovery suggestions
    pub fn recovery_suggestions(&self) -> Vec<String> {
        match self {
            ImapError::Io(_) => vec![
                "Check your network connection".to_string(),
                "Verify firewall settings aren't blocking IMAP".to_string(),
                "Try again in a moment".to_string(),
                "Check if the email server is accessible".to_string(),
            ],
            ImapError::Connection(_) => vec![
                "Verify IMAP server address and port".to_string(),
                "Check network connectivity".to_string(),
                "Ensure IMAP server is running".to_string(),
                "Try using a different port (143 for non-SSL, 993 for SSL)".to_string(),
            ],
            ImapError::Authentication(_) => vec![
                "Verify your email credentials".to_string(),
                "Check if 2FA/app passwords are required".to_string(),
                "Try re-authenticating: comunicado account update <email>".to_string(),
                "Check account settings with your email provider".to_string(),
            ],
            ImapError::Protocol(_) => vec![
                "This may be a bug in the IMAP implementation".to_string(),
                "Try with a different email client to test".to_string(),
                "Report this issue with debug logs".to_string(),
                "Check if the server supports required IMAP extensions".to_string(),
            ],
            ImapError::Server(_) => vec![
                "The email server reported an error".to_string(),
                "Wait a moment and try again".to_string(),
                "Check server status with your email provider".to_string(),
                "Try with fewer concurrent operations".to_string(),
            ],
            ImapError::Tls(_) => vec![
                "Check TLS/SSL configuration".to_string(),
                "Verify server certificates are valid".to_string(),
                "Try disabling SSL verification temporarily for testing".to_string(),
                "Update system certificates: sudo apt update && sudo apt install ca-certificates"
                    .to_string(),
            ],
            ImapError::Timeout => vec![
                "Operation timed out - server may be slow".to_string(),
                "Try again with increased timeout".to_string(),
                "Check network connectivity".to_string(),
                "Try during off-peak hours".to_string(),
            ],
            ImapError::Parse(_) => vec![
                "Server response couldn't be parsed".to_string(),
                "This may indicate server compatibility issues".to_string(),
                "Try with a different email client to test".to_string(),
                "Report this issue with debug logs".to_string(),
            ],
            ImapError::InvalidConfig(_) => vec![
                "Check your email account configuration".to_string(),
                "Verify server addresses and ports".to_string(),
                "Run: comunicado config --validate".to_string(),
                "Reset configuration: comunicado config --reset".to_string(),
            ],
            ImapError::NotSupported(_) => vec![
                "Feature not supported by this IMAP server".to_string(),
                "Try a different approach or disable this feature".to_string(),
                "Check server capabilities".to_string(),
                "Consider switching to a more modern email provider".to_string(),
            ],
            ImapError::FolderNotFound(_) => vec![
                "Folder doesn't exist on the server".to_string(),
                "Check folder name spelling and case".to_string(),
                "List available folders: comunicado folders list".to_string(),
                "Try syncing folders: comunicado sync folders".to_string(),
            ],
            ImapError::MessageNotFound(_) => vec![
                "Email message no longer exists".to_string(),
                "It may have been deleted or moved".to_string(),
                "Try refreshing the folder".to_string(),
                "Sync messages: comunicado sync messages".to_string(),
            ],
            ImapError::InvalidState(_) => vec![
                "IMAP connection is in an invalid state".to_string(),
                "Try reconnecting to the server".to_string(),
                "Restart the application".to_string(),
                "Check if another client is interfering".to_string(),
            ],
            ImapError::Generic(_) => vec![
                "An unexpected error occurred".to_string(),
                "Try the operation again".to_string(),
                "Check logs for more details".to_string(),
                "Report this issue if it persists".to_string(),
            ],
        }
    }
}

/// IMAP response status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResponseStatus {
    Ok,
    No,
    Bad,
    Preauth,
    Bye,
}

impl fmt::Display for ResponseStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResponseStatus::Ok => write!(f, "OK"),
            ResponseStatus::No => write!(f, "NO"),
            ResponseStatus::Bad => write!(f, "BAD"),
            ResponseStatus::Preauth => write!(f, "PREAUTH"),
            ResponseStatus::Bye => write!(f, "BYE"),
        }
    }
}

impl ResponseStatus {
    pub fn from_str(status: &str) -> Option<Self> {
        match status.to_uppercase().as_str() {
            "OK" => Some(ResponseStatus::Ok),
            "NO" => Some(ResponseStatus::No),
            "BAD" => Some(ResponseStatus::Bad),
            "PREAUTH" => Some(ResponseStatus::Preauth),
            "BYE" => Some(ResponseStatus::Bye),
            _ => None,
        }
    }

    pub fn is_success(&self) -> bool {
        matches!(self, ResponseStatus::Ok | ResponseStatus::Preauth)
    }

    pub fn is_error(&self) -> bool {
        matches!(self, ResponseStatus::No | ResponseStatus::Bad)
    }
}
