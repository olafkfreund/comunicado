//! Sync protocol for mobile companion app

use super::{MobileError, Result, EmailSummary, CalendarEventSummary, NotificationSettings};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Sync protocol manager
pub struct SyncProtocol {
    protocol_version: String,
    supported_features: Vec<String>,
    compression_enabled: bool,
    encryption_enabled: bool,
}

/// Sync message envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMessage {
    pub id: Uuid,
    pub protocol_version: String,
    pub timestamp: DateTime<Utc>,
    pub device_id: Uuid,
    pub message_type: SyncMessageType,
    pub payload: SyncPayload,
    pub checksum: Option<String>,
    pub compressed: bool,
    pub encrypted: bool,
}

/// Sync message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncMessageType {
    Request,
    Response,
    Notification,
    Heartbeat,
    Error,
}

/// Sync payload containing the actual data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum SyncPayload {
    // Request types
    Handshake(HandshakeRequest),
    Command(SyncCommand),
    
    // Response types
    HandshakeResponse(HandshakeResponse),
    CommandResponse(SyncCommandResponse),
    
    // Data types
    Emails(Vec<EmailSummary>),
    CalendarEvents(Vec<CalendarEventSummary>),
    Settings(AppSettings),
    Contacts(Vec<ContactSummary>),
    
    // Status types
    SettingsUpdated,
    SyncComplete,
    Heartbeat(HeartbeatData),
    Error(ErrorData),
    Unsupported,
}

/// Handshake request to establish sync session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeRequest {
    pub client_version: String,
    pub supported_protocols: Vec<String>,
    pub device_capabilities: DeviceCapabilities,
    pub preferred_features: Vec<String>,
}

/// Handshake response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeResponse {
    pub server_version: String,
    pub protocol_version: String,
    pub session_id: Uuid,
    pub supported_features: Vec<String>,
    pub sync_interval_seconds: u64,
    pub max_message_size: usize,
}

/// Device capabilities for sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    pub max_emails_per_sync: usize,
    pub max_events_per_sync: usize,
    pub supports_incremental_sync: bool,
    pub supports_compression: bool,
    pub supports_encryption: bool,
    pub offline_storage_mb: Option<u32>,
}

/// Sync commands from mobile device
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", content = "params")]
pub enum SyncCommand {
    // Email commands
    RequestEmails { 
        folder: String, 
        count: usize,
        since: Option<DateTime<Utc>>,
    },
    MarkEmailRead { 
        email_id: String 
    },
    MarkEmailArchived { 
        email_id: String 
    },
    
    // Calendar commands
    RequestCalendarEvents { 
        days_ahead: u32,
        calendar_ids: Option<Vec<String>>,
    },
    UpdateEventResponse { 
        event_id: String, 
        response: EventResponse 
    },
    
    // Settings commands
    UpdateSettings { 
        settings: NotificationSettings 
    },
    RequestSettings,
    
    // Sync commands
    RequestFullSync,
    RequestIncrementalSync { 
        last_sync: DateTime<Utc> 
    },
    
    // Contact commands
    RequestContacts { 
        count: Option<usize> 
    },
    UpdateContact { 
        contact: ContactSummary 
    },
}

/// Sync command responses
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", content = "data")]
pub enum SyncCommandResponse {
    Success(SuccessData),
    Error(ErrorData),
    Partial(PartialData),
}

/// Success response data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessData {
    pub message: String,
    pub items_count: Option<usize>,
    pub next_sync_recommended: Option<DateTime<Utc>>,
}

/// Error response data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorData {
    pub error_code: String,
    pub error_message: String,
    pub retry_after_seconds: Option<u64>,
    pub details: HashMap<String, String>,
}

/// Partial response data (for large datasets)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialData {
    pub message: String,
    pub items_count: usize,
    pub total_items: usize,
    pub continuation_token: String,
}

/// Heartbeat data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatData {
    pub timestamp: DateTime<Utc>,
    pub battery_level: Option<u8>,
    pub network_type: Option<String>,
    pub app_state: AppState,
}

/// Mobile app state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppState {
    Foreground,
    Background,
    Inactive,
    Terminated,
}

/// Event response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventResponse {
    Accept,
    Decline,
    Tentative,
}

/// Application settings for sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub notification_settings: NotificationSettings,
    pub sync_settings: SyncSettings,
    pub ui_settings: UiSettings,
    pub account_settings: AccountSettings,
}

/// Sync settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSettings {
    pub auto_sync_enabled: bool,
    pub sync_frequency_minutes: u32,
    pub wifi_only: bool,
    pub sync_in_background: bool,
    pub data_types: HashMap<String, bool>,
}

/// UI settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSettings {
    pub theme: String,
    pub font_size: String,
    pub show_previews: bool,
    pub compact_mode: bool,
    pub animation_enabled: bool,
}

/// Account settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountSettings {
    pub email_accounts: Vec<AccountInfo>,
    pub calendar_accounts: Vec<AccountInfo>,
    pub default_account: Option<String>,
}

/// Account information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    pub id: String,
    pub name: String,
    pub email: String,
    pub account_type: String,
    pub enabled: bool,
}

/// Contact summary for sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactSummary {
    pub id: String,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub avatar_url: Option<String>,
    pub last_updated: DateTime<Utc>,
}

/// Sync session state
#[derive(Debug, Clone)]
pub struct SyncSession {
    pub session_id: Uuid,
    pub device_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub protocol_version: String,
    pub capabilities: DeviceCapabilities,
    pub state: SyncSessionState,
    pub pending_commands: Vec<SyncCommand>,
}

/// Sync session states
#[derive(Debug, Clone, PartialEq)]
pub enum SyncSessionState {
    Handshaking,
    Active,
    Syncing,
    Idle,
    Error,
    Terminated,
}

impl SyncProtocol {
    pub fn new() -> Self {
        Self {
            protocol_version: "1.0".to_string(),
            supported_features: vec![
                "incremental_sync".to_string(),
                "compression".to_string(),
                "encryption".to_string(),
                "heartbeat".to_string(),
                "batch_operations".to_string(),
            ],
            compression_enabled: true,
            encryption_enabled: true,
        }
    }

    /// Create handshake request
    pub fn create_handshake_request(&self, capabilities: DeviceCapabilities) -> SyncMessage {
        let handshake = HandshakeRequest {
            client_version: "1.0.0".to_string(),
            supported_protocols: vec!["sync/1.0".to_string()],
            device_capabilities: capabilities,
            preferred_features: self.supported_features.clone(),
        };

        SyncMessage {
            id: Uuid::new_v4(),
            protocol_version: self.protocol_version.clone(),
            timestamp: Utc::now(),
            device_id: Uuid::new_v4(), // Would be actual device ID
            message_type: SyncMessageType::Request,
            payload: SyncPayload::Handshake(handshake),
            checksum: None,
            compressed: false,
            encrypted: false,
        }
    }

    /// Create handshake response
    pub fn create_handshake_response(&self, session_id: Uuid) -> SyncMessage {
        let response = HandshakeResponse {
            server_version: "1.0.0".to_string(),
            protocol_version: self.protocol_version.clone(),
            session_id,
            supported_features: self.supported_features.clone(),
            sync_interval_seconds: 300,
            max_message_size: 1024 * 1024, // 1MB
        };

        SyncMessage {
            id: Uuid::new_v4(),
            protocol_version: self.protocol_version.clone(),
            timestamp: Utc::now(),
            device_id: Uuid::new_v4(),
            message_type: SyncMessageType::Response,
            payload: SyncPayload::HandshakeResponse(response),
            checksum: None,
            compressed: false,
            encrypted: false,
        }
    }

    /// Create command message
    pub fn create_command(
        &self,
        device_id: Uuid,
        command: SyncCommand,
    ) -> SyncMessage {
        SyncMessage {
            id: Uuid::new_v4(),
            protocol_version: self.protocol_version.clone(),
            timestamp: Utc::now(),
            device_id,
            message_type: SyncMessageType::Request,
            payload: SyncPayload::Command(command),
            checksum: None,
            compressed: self.compression_enabled,
            encrypted: self.encryption_enabled,
        }
    }

    /// Create command response
    pub fn create_command_response(
        &self,
        device_id: Uuid,
        response: SyncCommandResponse,
    ) -> SyncMessage {
        SyncMessage {
            id: Uuid::new_v4(),
            protocol_version: self.protocol_version.clone(),
            timestamp: Utc::now(),
            device_id,
            message_type: SyncMessageType::Response,
            payload: SyncPayload::CommandResponse(response),
            checksum: None,
            compressed: self.compression_enabled,
            encrypted: self.encryption_enabled,
        }
    }

    /// Create heartbeat message
    pub fn create_heartbeat(
        &self,
        device_id: Uuid,
        battery_level: Option<u8>,
        network_type: Option<String>,
        app_state: AppState,
    ) -> SyncMessage {
        let heartbeat = HeartbeatData {
            timestamp: Utc::now(),
            battery_level,
            network_type,
            app_state,
        };

        SyncMessage {
            id: Uuid::new_v4(),
            protocol_version: self.protocol_version.clone(),
            timestamp: Utc::now(),
            device_id,
            message_type: SyncMessageType::Heartbeat,
            payload: SyncPayload::Heartbeat(heartbeat),
            checksum: None,
            compressed: false,
            encrypted: false,
        }
    }

    /// Serialize message to bytes
    pub fn serialize_message(&self, message: &SyncMessage) -> Result<Vec<u8>> {
        let mut data = serde_json::to_vec(message)
            .map_err(|e| MobileError::SerializationError(e))?;

        // Apply compression if enabled
        if message.compressed && self.compression_enabled {
            data = self.compress_data(&data)?;
        }

        // Apply encryption if enabled
        if message.encrypted && self.encryption_enabled {
            data = self.encrypt_data(&data)?;
        }

        Ok(data)
    }

    /// Deserialize message from bytes
    pub fn deserialize_message(&self, data: &[u8]) -> Result<SyncMessage> {
        let mut processed_data = data.to_vec();

        // First, try to parse as JSON to check if it's encrypted/compressed
        if let Ok(message) = serde_json::from_slice::<SyncMessage>(&processed_data) {
            // Apply reverse processing if needed
            if message.encrypted && self.encryption_enabled {
                processed_data = self.decrypt_data(&processed_data)?;
            }

            if message.compressed && self.compression_enabled {
                processed_data = self.decompress_data(&processed_data)?;
            }

            // Re-parse the processed data
            serde_json::from_slice(&processed_data)
                .map_err(|e| MobileError::SerializationError(e))
        } else {
            // Try to decrypt/decompress and then parse
            if self.encryption_enabled {
                processed_data = self.decrypt_data(&processed_data)?;
            }

            if self.compression_enabled {
                processed_data = self.decompress_data(&processed_data)?;
            }

            serde_json::from_slice(&processed_data)
                .map_err(|e| MobileError::SerializationError(e))
        }
    }

    /// Process sync command
    pub async fn process_command(
        &self,
        command: SyncCommand,
    ) -> Result<SyncCommandResponse> {
        match command {
            SyncCommand::RequestEmails { folder, count, since } => {
                // This would integrate with email service
                let _ = (folder, count, since); // Suppress warnings
                Ok(SyncCommandResponse::Success(SuccessData {
                    message: "Emails retrieved successfully".to_string(),
                    items_count: Some(0),
                    next_sync_recommended: Some(Utc::now() + chrono::Duration::minutes(15)),
                }))
            }
            SyncCommand::RequestCalendarEvents { days_ahead, calendar_ids } => {
                // This would integrate with calendar service
                let _ = (days_ahead, calendar_ids);
                Ok(SyncCommandResponse::Success(SuccessData {
                    message: "Calendar events retrieved successfully".to_string(),
                    items_count: Some(0),
                    next_sync_recommended: Some(Utc::now() + chrono::Duration::hours(1)),
                }))
            }
            SyncCommand::UpdateSettings { settings: _ } => {
                Ok(SyncCommandResponse::Success(SuccessData {
                    message: "Settings updated successfully".to_string(),
                    items_count: None,
                    next_sync_recommended: None,
                }))
            }
            _ => {
                Ok(SyncCommandResponse::Error(ErrorData {
                    error_code: "UNSUPPORTED_COMMAND".to_string(),
                    error_message: "Command not supported".to_string(),
                    retry_after_seconds: None,
                    details: HashMap::new(),
                }))
            }
        }
    }

    /// Validate message integrity
    pub fn validate_message(&self, message: &SyncMessage) -> Result<bool> {
        // Check protocol version compatibility
        if !self.is_compatible_version(&message.protocol_version) {
            return Ok(false);
        }

        // Check message timestamp (not too old or in future)
        let now = Utc::now();
        let age = now.signed_duration_since(message.timestamp);
        
        if age.num_minutes() > 60 || age.num_minutes() < -5 {
            return Ok(false);
        }

        // Validate checksum if present
        if let Some(expected_checksum) = &message.checksum {
            let calculated_checksum = self.calculate_checksum(message)?;
            if expected_checksum != &calculated_checksum {
                return Ok(false);
            }
        }

        Ok(true)
    }

    // Private helper methods
    
    fn is_compatible_version(&self, version: &str) -> bool {
        // Simple version compatibility check
        version.starts_with("1.")
    }

    fn calculate_checksum(&self, message: &SyncMessage) -> Result<String> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        message.id.hash(&mut hasher);
        message.timestamp.hash(&mut hasher);
        message.device_id.hash(&mut hasher);
        
        Ok(format!("{:x}", hasher.finish()))
    }

    fn compress_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        // Placeholder compression (would use actual compression library)
        let mut compressed = b"COMPRESSED:".to_vec();
        compressed.extend_from_slice(data);
        Ok(compressed)
    }

    fn decompress_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        // Placeholder decompression
        if data.starts_with(b"COMPRESSED:") {
            Ok(data[11..].to_vec())
        } else {
            Ok(data.to_vec())
        }
    }

    fn encrypt_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        // Placeholder encryption (would use actual encryption)
        let mut encrypted = b"ENCRYPTED:".to_vec();
        encrypted.extend_from_slice(data);
        Ok(encrypted)
    }

    fn decrypt_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        // Placeholder decryption
        if data.starts_with(b"ENCRYPTED:") {
            Ok(data[10..].to_vec())
        } else {
            Ok(data.to_vec())
        }
    }
}

impl Default for DeviceCapabilities {
    fn default() -> Self {
        Self {
            max_emails_per_sync: 50,
            max_events_per_sync: 100,
            supports_incremental_sync: true,
            supports_compression: true,
            supports_encryption: true,
            offline_storage_mb: Some(100),
        }
    }
}

impl Default for SyncSettings {
    fn default() -> Self {
        let mut data_types = HashMap::new();
        data_types.insert("emails".to_string(), true);
        data_types.insert("calendar".to_string(), true);
        data_types.insert("contacts".to_string(), false);
        data_types.insert("settings".to_string(), true);

        Self {
            auto_sync_enabled: true,
            sync_frequency_minutes: 15,
            wifi_only: false,
            sync_in_background: true,
            data_types,
        }
    }
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            theme: "system".to_string(),
            font_size: "medium".to_string(),
            show_previews: true,
            compact_mode: false,
            animation_enabled: true,
        }
    }
}