//! Device management for mobile companion app

use super::{MobileError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Device manager
pub struct DeviceManager {
    devices: RwLock<HashMap<Uuid, MobileDevice>>,
    device_sessions: RwLock<HashMap<Uuid, DeviceSession>>,
    max_devices: usize,
    stats: RwLock<DeviceStats>,
}

/// Mobile device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileDevice {
    pub id: Uuid,
    pub name: String,
    pub device_type: DeviceType,
    pub os_version: String,
    pub app_version: String,
    pub manufacturer: String,
    pub model: String,
    pub screen_resolution: Option<ScreenResolution>,
    pub timezone: String,
    pub language: String,
    pub registered_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub status: DeviceStatus,
    pub capabilities: DeviceCapabilities,
    pub settings: DeviceSettings,
}

/// Device types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeviceType {
    Android,
    IOs,
    Web,
    Desktop,
    Other(String),
}

/// Device status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeviceStatus {
    Online,
    Offline,
    Away,
    DoNotDisturb,
    Inactive,
}

/// Screen resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenResolution {
    pub width: u32,
    pub height: u32,
    pub density: f32,
}

/// Device capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    pub push_notifications: bool,
    pub websocket_support: bool,
    pub biometric_auth: bool,
    pub background_sync: bool,
    pub file_sharing: bool,
    pub camera_access: bool,
    pub location_services: bool,
    pub contacts_access: bool,
    pub calendar_integration: bool,
}

/// Device settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceSettings {
    pub notification_preferences: NotificationPreferences,
    pub sync_preferences: SyncPreferences,
    pub privacy_settings: PrivacySettings,
    pub ui_preferences: UiPreferences,
}

/// Notification preferences per device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPreferences {
    pub enabled: bool,
    pub sound_enabled: bool,
    pub vibration_enabled: bool,
    pub led_enabled: bool,
    pub show_preview: bool,
    pub quiet_hours: Option<QuietHours>,
    pub category_filters: HashMap<String, bool>,
}

/// Sync preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPreferences {
    pub auto_sync: bool,
    pub sync_frequency_minutes: u32,
    pub wifi_only: bool,
    pub background_sync: bool,
    pub data_types: HashMap<String, bool>,
}

/// Privacy settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacySettings {
    pub analytics_enabled: bool,
    pub crash_reporting: bool,
    pub location_sharing: bool,
    pub contact_sync: bool,
    pub usage_stats: bool,
}

/// UI preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiPreferences {
    pub theme: String,
    pub font_size: FontSize,
    pub language: String,
    pub time_format: TimeFormat,
    pub date_format: String,
}

/// Font size options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FontSize {
    Small,
    Medium,
    Large,
    ExtraLarge,
}

/// Time format options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeFormat {
    Hour12,
    Hour24,
}

/// Quiet hours configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuietHours {
    pub enabled: bool,
    pub start_hour: u8,
    pub start_minute: u8,
    pub end_hour: u8,
    pub end_minute: u8,
    pub days_of_week: Vec<u8>, // 0 = Sunday, 1 = Monday, etc.
}

/// Device session information
#[derive(Debug, Clone)]
pub struct DeviceSession {
    pub device_id: Uuid,
    pub session_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub connection_type: ConnectionType,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub active_features: Vec<String>,
}

/// Connection types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionType {
    WebSocket,
    RestApi,
    PushOnly,
    KdeConnect,
}

/// Device statistics
#[derive(Debug, Clone, Default)]
pub struct DeviceStats {
    pub total_devices: usize,
    pub online_devices: usize,
    pub by_type: HashMap<String, usize>,
    pub by_status: HashMap<String, usize>,
    pub average_session_duration: f64,
    pub total_sessions: u64,
}

impl DeviceManager {
    pub fn new(max_devices: usize) -> Result<Self> {
        Ok(Self {
            devices: RwLock::new(HashMap::new()),
            device_sessions: RwLock::new(HashMap::new()),
            max_devices,
            stats: RwLock::new(DeviceStats::default()),
        })
    }

    /// Register a new device
    pub async fn register_device(&self, mut device_info: MobileDevice) -> Result<Uuid> {
        let devices = self.devices.read().await;
        if devices.len() >= self.max_devices {
            return Err(MobileError::ConfigurationError(
                format!("Maximum device limit ({}) reached", self.max_devices)
            ));
        }
        drop(devices);

        device_info.id = Uuid::new_v4();
        device_info.registered_at = Utc::now();
        device_info.last_seen = Utc::now();
        device_info.status = DeviceStatus::Online;

        let device_id = device_info.id;

        let mut devices = self.devices.write().await;
        devices.insert(device_id, device_info.clone());
        drop(devices);

        // Update statistics
        self.update_device_stats().await;

        Ok(device_id)
    }

    /// Get device by ID
    pub async fn get_device(&self, device_id: Uuid) -> Result<MobileDevice> {
        let devices = self.devices.read().await;
        devices.get(&device_id)
            .cloned()
            .ok_or_else(|| MobileError::DeviceNotFound(device_id.to_string()))
    }

    /// Update device status
    pub async fn update_device_status(&self, device_id: Uuid, status: DeviceStatus) -> Result<()> {
        let mut devices = self.devices.write().await;
        if let Some(device) = devices.get_mut(&device_id) {
            device.status = status;
            device.last_seen = Utc::now();
            
            // Update statistics
            self.update_device_stats().await;
            
            Ok(())
        } else {
            Err(MobileError::DeviceNotFound(device_id.to_string()))
        }
    }

    /// Update device settings
    pub async fn update_device_settings(
        &self,
        device_id: Uuid,
        settings: DeviceSettings,
    ) -> Result<()> {
        let mut devices = self.devices.write().await;
        if let Some(device) = devices.get_mut(&device_id) {
            device.settings = settings;
            device.last_seen = Utc::now();
            Ok(())
        } else {
            Err(MobileError::DeviceNotFound(device_id.to_string()))
        }
    }

    /// Start device session
    pub async fn start_session(
        &self,
        device_id: Uuid,
        connection_type: ConnectionType,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<Uuid> {
        // Verify device exists
        let devices = self.devices.read().await;
        if !devices.contains_key(&device_id) {
            return Err(MobileError::DeviceNotFound(device_id.to_string()));
        }
        drop(devices);

        let session_id = Uuid::new_v4();
        let session = DeviceSession {
            device_id,
            session_id,
            started_at: Utc::now(),
            last_activity: Utc::now(),
            connection_type,
            ip_address,
            user_agent,
            active_features: Vec::new(),
        };

        let mut sessions = self.device_sessions.write().await;
        sessions.insert(session_id, session);

        // Update device status to online
        self.update_device_status(device_id, DeviceStatus::Online).await?;

        Ok(session_id)
    }

    /// End device session
    pub async fn end_session(&self, session_id: Uuid) -> Result<()> {
        let mut sessions = self.device_sessions.write().await;
        if let Some(session) = sessions.remove(&session_id) {
            // Update device status to offline
            self.update_device_status(session.device_id, DeviceStatus::Offline).await?;
            
            // Update session statistics
            let mut stats = self.stats.write().await;
            stats.total_sessions += 1;
            
            let session_duration = Utc::now().signed_duration_since(session.started_at);
            let duration_seconds = session_duration.num_seconds() as f64;
            
            if stats.total_sessions == 1 {
                stats.average_session_duration = duration_seconds;
            } else {
                stats.average_session_duration = 
                    (stats.average_session_duration * (stats.total_sessions - 1) as f64 + duration_seconds) 
                    / stats.total_sessions as f64;
            }

            Ok(())
        } else {
            Err(MobileError::ConfigurationError("Session not found".to_string()))
        }
    }

    /// Update session activity
    pub async fn update_session_activity(&self, session_id: Uuid) -> Result<()> {
        let mut sessions = self.device_sessions.write().await;
        if let Some(session) = sessions.get_mut(&session_id) {
            session.last_activity = Utc::now();
            Ok(())
        } else {
            Err(MobileError::ConfigurationError("Session not found".to_string()))
        }
    }

    /// Get all devices
    pub async fn get_all_devices(&self) -> Result<Vec<MobileDevice>> {
        let devices = self.devices.read().await;
        Ok(devices.values().cloned().collect())
    }

    /// Get devices by status
    pub async fn get_devices_by_status(&self, status: DeviceStatus) -> Result<Vec<MobileDevice>> {
        let devices = self.devices.read().await;
        Ok(devices.values()
            .filter(|device| device.status == status)
            .cloned()
            .collect())
    }

    /// Get online devices
    pub async fn get_online_devices(&self) -> Result<Vec<MobileDevice>> {
        self.get_devices_by_status(DeviceStatus::Online).await
    }

    /// Get device count
    pub async fn get_device_count(&self) -> usize {
        let devices = self.devices.read().await;
        devices.len()
    }

    /// Remove device
    pub async fn remove_device(&self, device_id: Uuid) -> Result<()> {
        let mut devices = self.devices.write().await;
        if devices.remove(&device_id).is_some() {
            // Remove any active sessions
            let mut sessions = self.device_sessions.write().await;
            sessions.retain(|_, session| session.device_id != device_id);
            
            // Update statistics
            self.update_device_stats().await;
            
            Ok(())
        } else {
            Err(MobileError::DeviceNotFound(device_id.to_string()))
        }
    }

    /// Get device statistics
    pub async fn get_statistics(&self) -> DeviceStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Clean up inactive devices
    pub async fn cleanup_inactive_devices(&self, inactive_hours: u64) -> Result<u32> {
        let cutoff_time = Utc::now() - chrono::Duration::hours(inactive_hours as i64);
        let mut devices = self.devices.write().await;
        
        let initial_count = devices.len();
        devices.retain(|_, device| {
            device.last_seen > cutoff_time || device.status == DeviceStatus::Online
        });
        
        let removed_count = initial_count - devices.len();
        
        if removed_count > 0 {
            // Update statistics
            self.update_device_stats().await;
        }

        Ok(removed_count as u32)
    }

    /// Get devices requiring attention
    pub async fn get_devices_requiring_attention(&self) -> Result<Vec<DeviceIssue>> {
        let devices = self.devices.read().await;
        let mut issues = Vec::new();

        for device in devices.values() {
            let inactive_duration = Utc::now().signed_duration_since(device.last_seen);
            
            // Check for inactive devices
            if inactive_duration.num_hours() > 24 && device.status == DeviceStatus::Online {
                issues.push(DeviceIssue {
                    device_id: device.id,
                    issue_type: IssueType::StaleOnlineStatus,
                    description: format!("Device {} shows online but hasn't been seen for {} hours", 
                                       device.name, inactive_duration.num_hours()),
                    severity: IssueSeverity::Medium,
                });
            }

            // Check for devices without push capabilities
            if !device.capabilities.push_notifications && device.device_type != DeviceType::Desktop {
                issues.push(DeviceIssue {
                    device_id: device.id,
                    issue_type: IssueType::MissingCapability,
                    description: format!("Device {} lacks push notification capability", device.name),
                    severity: IssueSeverity::Low,
                });
            }
        }

        Ok(issues)
    }

    // Private methods
    async fn update_device_stats(&self) {
        let devices = self.devices.read().await;
        let mut stats = self.stats.write().await;

        stats.total_devices = devices.len();
        stats.online_devices = devices.values()
            .filter(|device| device.status == DeviceStatus::Online)
            .count();

        // Count by type
        stats.by_type.clear();
        for device in devices.values() {
            let type_name = format!("{:?}", device.device_type);
            *stats.by_type.entry(type_name).or_insert(0) += 1;
        }

        // Count by status
        stats.by_status.clear();
        for device in devices.values() {
            let status_name = format!("{:?}", device.status);
            *stats.by_status.entry(status_name).or_insert(0) += 1;
        }
    }
}

/// Device issue information
#[derive(Debug, Clone)]
pub struct DeviceIssue {
    pub device_id: Uuid,
    pub issue_type: IssueType,
    pub description: String,
    pub severity: IssueSeverity,
}

/// Issue types
#[derive(Debug, Clone)]
pub enum IssueType {
    StaleOnlineStatus,
    MissingCapability,
    ConfigurationError,
    ConnectivityIssue,
}

/// Issue severity levels
#[derive(Debug, Clone)]
pub enum IssueSeverity {
    Low,
    Medium,
    High,
    Critical,
}

// Default implementations
impl Default for DeviceCapabilities {
    fn default() -> Self {
        Self {
            push_notifications: true,
            websocket_support: true,
            biometric_auth: false,
            background_sync: true,
            file_sharing: false,
            camera_access: false,
            location_services: false,
            contacts_access: false,
            calendar_integration: false,
        }
    }
}

impl Default for DeviceSettings {
    fn default() -> Self {
        Self {
            notification_preferences: NotificationPreferences::default(),
            sync_preferences: SyncPreferences::default(),
            privacy_settings: PrivacySettings::default(),
            ui_preferences: UiPreferences::default(),
        }
    }
}

impl Default for NotificationPreferences {
    fn default() -> Self {
        Self {
            enabled: true,
            sound_enabled: true,
            vibration_enabled: true,
            led_enabled: true,
            show_preview: true,
            quiet_hours: None,
            category_filters: HashMap::new(),
        }
    }
}

impl Default for SyncPreferences {
    fn default() -> Self {
        let mut data_types = HashMap::new();
        data_types.insert("email".to_string(), true);
        data_types.insert("calendar".to_string(), true);
        data_types.insert("contacts".to_string(), false);
        data_types.insert("settings".to_string(), true);

        Self {
            auto_sync: true,
            sync_frequency_minutes: 15,
            wifi_only: false,
            background_sync: true,
            data_types,
        }
    }
}

impl Default for PrivacySettings {
    fn default() -> Self {
        Self {
            analytics_enabled: false,
            crash_reporting: true,
            location_sharing: false,
            contact_sync: false,
            usage_stats: false,
        }
    }
}

impl Default for UiPreferences {
    fn default() -> Self {
        Self {
            theme: "system".to_string(),
            font_size: FontSize::Medium,
            language: "en".to_string(),
            time_format: TimeFormat::Hour12,
            date_format: "MM/dd/yyyy".to_string(),
        }
    }
}