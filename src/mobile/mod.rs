pub mod kde_connect;
pub mod storage;
pub mod services;
pub mod ui;
pub mod config;

// Re-export main types for easier access
pub use kde_connect::{KdeConnectClient, SmsMessage, MobileNotification, DeviceInfo};
pub use storage::{MessageStore, MessageStoreStats};
pub use services::{MobileSyncService, MobileSyncStats, ServiceControl};
pub use config::{MobileConfig, SmsSettings, NotificationSettings};
pub use ui::{SmsUi, SmsViewMode, SmsComposition, SmsRenderConfig, SmsColorScheme};

// Module-level error type
#[derive(Debug, thiserror::Error)]
pub enum MobileError {
    #[error("KDE Connect not available: {0}")]
    KdeConnectNotAvailable(String),
    
    #[error("Device not paired: {0}")]
    DeviceNotPaired(String),
    
    #[error("Device not reachable: {0}")]
    DeviceNotReachable(String),
    
    #[error("D-Bus connection failed: {0}")]
    #[cfg(feature = "kde-connect")]
    DbusConnectionFailed(#[from] dbus::Error),
    
    #[error("D-Bus connection failed: {0}")]
    #[cfg(not(feature = "kde-connect"))]
    DbusConnectionFailed(String),
    
    #[error("Message send failed: {0}")]
    MessageSendFailed(String),
    
    #[error("Notification failed: {0}")]
    NotificationFailed(String),
    
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

impl MobileError {
    pub fn user_message(&self) -> &str {
        match self {
            Self::KdeConnectNotAvailable(_) => "KDE Connect is not installed or running",
            Self::DeviceNotPaired(_) => "Mobile device is not paired with KDE Connect",
            Self::DeviceNotReachable(_) => "Mobile device is not reachable on the network",
            Self::DbusConnectionFailed(_) => "Failed to connect to D-Bus service",
            Self::MessageSendFailed(_) => "Failed to send SMS message",
            Self::NotificationFailed(_) => "Failed to process mobile notification",
            Self::DatabaseError(_) => "Database operation failed",
            Self::ConfigurationError(_) => "Mobile integration configuration error",
            Self::SerializationError(_) => "Data serialization error",
            Self::IoError(_) => "File system operation failed",
        }
    }

    pub fn is_recoverable(&self) -> bool {
        matches!(self, 
            Self::DeviceNotReachable(_) | 
            Self::MessageSendFailed(_) |
            Self::NotificationFailed(_) |
            Self::IoError(_)
        )
    }
}

pub type Result<T> = std::result::Result<T, MobileError>;