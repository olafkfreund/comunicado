pub mod config;
pub mod kde_connect;
pub mod services;
pub mod storage;
pub mod ui;

// Mobile companion app modules
pub mod auth_manager;
pub mod device_manager;
pub mod mobile_api;
pub mod notification_bridge;
pub mod push_service;
pub mod sync_protocol;
pub mod websocket_server;

// Re-export main types for easier access
pub use config::{MobileConfig, NotificationSettings, SmsSettings};
pub use kde_connect::{DeviceInfo, KdeConnectClient, MobileNotification, SmsMessage};
pub use services::{MobileSyncService, MobileSyncStats, ServiceControl};
pub use storage::{MessageStore, MessageStoreStats};
pub use ui::{SmsColorScheme, SmsComposition, SmsRenderConfig, SmsUi, SmsViewMode};

// Mobile companion app exports
pub use device_manager::{DeviceManager, DeviceStatus, MobileDevice};
pub use notification_bridge::{NotificationBridge, NotificationPayload, NotificationPriority};
pub use push_service::{PushProvider, PushService, PushToken};
pub use sync_protocol::{SyncCommand, SyncMessage, SyncProtocol};

// Add missing types for push_service and sync_protocol
pub use push_service::PushProviderConfigReal as PushProviderConfig;

/// Push provider types
#[derive(Debug, Clone, Hash, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PushProviderType {
    FCM,
    APNS,
    WebPush,
    Custom(String),
}
pub type EmailSummary = String; // Placeholder
pub type CalendarEventSummary = String; // Placeholder

// Unused imports commented out for now
// use serde::{Deserialize, Serialize};
// use std::collections::HashMap;
// use tokio::sync::RwLock;
// use uuid::Uuid;

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

    #[error("Push service error: {0}")]
    PushService(String),

    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    #[error("Network error: {0}")]
    Network(String),
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
            Self::PushService(_) => "Push service error",
            Self::DeviceNotFound(_) => "Mobile device not found",
            Self::Network(_) => "Network error",
        }
    }

    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::DeviceNotReachable(_)
                | Self::MessageSendFailed(_)
                | Self::NotificationFailed(_)
                | Self::IoError(_)
        )
    }
}

pub type Result<T> = std::result::Result<T, MobileError>;
