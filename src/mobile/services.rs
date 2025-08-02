use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock, Mutex};
use tokio::time::Interval;
use chrono::{DateTime, Utc};
use tracing::{info, debug, warn, error};
use serde::{Deserialize, Serialize};

use crate::mobile::{
    KdeConnectClient, MessageStore, MobileConfig, Result,
    kde_connect::types::MobileNotification
};

/// Background service that synchronizes SMS/MMS messages and notifications
/// between mobile devices via KDE Connect and local storage
pub struct MobileSyncService {
    config: Arc<RwLock<MobileConfig>>,
    kde_connect: Arc<Mutex<KdeConnectClient>>,
    message_store: Arc<MessageStore>,
    stats: Arc<RwLock<MobileSyncStats>>,
    control_tx: mpsc::UnboundedSender<ServiceControl>,
    control_rx: Arc<Mutex<mpsc::UnboundedReceiver<ServiceControl>>>,
    is_running: Arc<RwLock<bool>>,
}

/// Statistics and status information for the mobile sync service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileSyncStats {
    pub is_running: bool,
    pub total_syncs: u64,
    pub total_errors: u64,
    pub last_sync_time: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub conversation_count: usize,
    pub message_count: usize,
    pub unread_count: usize,
    pub notification_count: usize,
    pub sync_interval_seconds: u64,
    pub connected_device_name: Option<String>,
    pub bytes_synced: u64,
    pub uptime_seconds: u64,
}

/// Control commands for the mobile sync service
#[derive(Debug, Clone)]
pub enum ServiceControl {
    Start,
    Stop,
    Pause,
    Resume,
    ForceSync,
    Reconnect,
    UpdateConfig(MobileConfig),
}

/// Sync operation results
#[derive(Debug, Clone)]
pub struct SyncResult {
    pub messages_processed: u32,
    pub notifications_processed: u32,
    pub errors: Vec<String>,
    pub duration_ms: u64,
}

impl MobileSyncService {
    /// Create a new mobile sync service
    pub async fn new(
        config: MobileConfig,
        kde_connect: Arc<Mutex<KdeConnectClient>>,
        message_store: Arc<MessageStore>,
    ) -> Result<Self> {
        let (control_tx, control_rx) = mpsc::unbounded_channel();
        
        let stats = MobileSyncStats {
            is_running: false,
            total_syncs: 0,
            total_errors: 0,
            last_sync_time: None,
            last_error: None,
            conversation_count: 0,
            message_count: 0,
            unread_count: 0,
            notification_count: 0,
            sync_interval_seconds: config.sms.sync_interval_seconds,
            connected_device_name: None,
            bytes_synced: 0,
            uptime_seconds: 0,
        };

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            kde_connect,
            message_store,
            stats: Arc::new(RwLock::new(stats)),
            control_tx,
            control_rx: Arc::new(Mutex::new(control_rx)),
            is_running: Arc::new(RwLock::new(false)),
        })
    }

    /// Get a handle to send control commands to the service
    pub fn control_handle(&self) -> mpsc::UnboundedSender<ServiceControl> {
        self.control_tx.clone()
    }

    /// Get current service statistics
    pub async fn get_stats(&self) -> MobileSyncStats {
        let mut stats = self.stats.read().await.clone();
        stats.is_running = *self.is_running.read().await;
        
        // Update storage statistics
        if let Ok(store_stats) = self.message_store.get_stats().await {
            stats.conversation_count = store_stats.conversation_count;
            stats.message_count = store_stats.message_count;
            stats.unread_count = store_stats.unread_conversation_count;
        }

        // Update connected device info
        let kde_connect = self.kde_connect.lock().await;
        if kde_connect.is_connected() {
            if let Ok(device_details) = kde_connect.get_device_details() {
                stats.connected_device_name = Some(device_details.name);
            }
        }

        stats
    }

    /// Start the background sync service
    pub async fn start(&self) -> Result<()> {
        if *self.is_running.read().await {
            warn!("Mobile sync service is already running");
            return Ok(());
        }

        info!("Starting mobile sync service");
        *self.is_running.write().await = true;
        
        let config = self.config.clone();
        let kde_connect = self.kde_connect.clone();
        let message_store = self.message_store.clone();
        let stats = self.stats.clone();
        let control_rx = self.control_rx.clone();
        let is_running = self.is_running.clone();

        // Spawn the main service loop
        tokio::spawn(async move {
            Self::service_loop(config, kde_connect, message_store, stats, control_rx, is_running).await;
        });

        info!("Mobile sync service started successfully");
        Ok(())
    }

    /// Stop the sync service
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping mobile sync service");
        self.control_tx.send(ServiceControl::Stop).map_err(|_| {
            crate::mobile::MobileError::NotificationFailed("Failed to send stop command".to_string())
        })?;
        Ok(())
    }

    /// Force an immediate sync
    pub async fn force_sync(&self) -> Result<()> {
        debug!("Requesting immediate sync");
        self.control_tx.send(ServiceControl::ForceSync).map_err(|_| {
            crate::mobile::MobileError::NotificationFailed("Failed to send force sync command".to_string())
        })?;
        Ok(())
    }

    /// Update service configuration
    pub async fn update_config(&self, new_config: MobileConfig) -> Result<()> {
        debug!("Updating mobile sync service configuration");
        *self.config.write().await = new_config.clone();
        self.control_tx.send(ServiceControl::UpdateConfig(new_config)).map_err(|_| {
            crate::mobile::MobileError::ConfigurationError("Failed to send config update".to_string())
        })?;
        Ok(())
    }

    /// Main service loop that handles syncing and control commands
    async fn service_loop(
        config: Arc<RwLock<MobileConfig>>,
        kde_connect: Arc<Mutex<KdeConnectClient>>,
        message_store: Arc<MessageStore>,
        stats: Arc<RwLock<MobileSyncStats>>,
        control_rx: Arc<Mutex<mpsc::UnboundedReceiver<ServiceControl>>>,
        is_running: Arc<RwLock<bool>>,
    ) {
        let mut sync_interval = Self::create_sync_interval(&config).await;
        let mut paused = false;
        let start_time = Utc::now();

        info!("Mobile sync service loop started");

        loop {
            tokio::select! {
                // Handle control commands
                command = async {
                    let mut rx = control_rx.lock().await;
                    rx.recv().await
                } => {
                    match command {
                        Some(ServiceControl::Stop) => {
                            info!("Received stop command, shutting down sync service");
                            break;
                        }
                        Some(ServiceControl::Pause) => {
                            info!("Pausing sync service");
                            paused = true;
                        }
                        Some(ServiceControl::Resume) => {
                            info!("Resuming sync service");
                            paused = false;
                        }
                        Some(ServiceControl::ForceSync) => {
                            if !paused {
                                debug!("Performing forced sync");
                                Self::perform_sync(&kde_connect, &message_store, &stats).await;
                            }
                        }
                        Some(ServiceControl::Reconnect) => {
                            info!("Reconnecting to KDE Connect device");
                            if let Err(e) = Self::reconnect_device(&kde_connect).await {
                                error!("Failed to reconnect: {}", e);
                                Self::update_error_stats(&stats, &format!("Reconnect failed: {}", e)).await;
                            }
                        }
                        Some(ServiceControl::UpdateConfig(new_config)) => {
                            debug!("Updating service configuration");
                            sync_interval = Self::create_sync_interval_from_config(&new_config).await;
                        }
                        Some(ServiceControl::Start) => {
                            debug!("Received start command (already running)");
                        }
                        None => {
                            warn!("Control channel closed, stopping service");
                            break;
                        }
                    }
                }

                // Handle periodic sync
                _ = sync_interval.tick(), if !paused => {
                    debug!("Performing scheduled sync");
                    Self::perform_sync(&kde_connect, &message_store, &stats).await;
                }

                // Update uptime stats every minute
                _ = tokio::time::sleep(Duration::from_secs(60)) => {
                    let uptime = (Utc::now() - start_time).num_seconds() as u64;
                    stats.write().await.uptime_seconds = uptime;
                }
            }
        }

        *is_running.write().await = false;
        info!("Mobile sync service stopped");
    }

    /// Create sync interval timer from current configuration
    async fn create_sync_interval(config: &Arc<RwLock<MobileConfig>>) -> Interval {
        let interval_seconds = config.read().await.sms.sync_interval_seconds;
        tokio::time::interval(Duration::from_secs(interval_seconds.max(5))) // Minimum 5 seconds
    }

    /// Create sync interval timer from provided configuration
    async fn create_sync_interval_from_config(config: &MobileConfig) -> Interval {
        let interval_seconds = config.sms.sync_interval_seconds;
        tokio::time::interval(Duration::from_secs(interval_seconds.max(5))) // Minimum 5 seconds
    }

    /// Perform a complete sync operation
    async fn perform_sync(
        kde_connect: &Arc<Mutex<KdeConnectClient>>,
        message_store: &Arc<MessageStore>,
        stats: &Arc<RwLock<MobileSyncStats>>,
    ) {
        let sync_start = Utc::now();
        let mut result = SyncResult {
            messages_processed: 0,
            notifications_processed: 0,
            errors: Vec::new(),
            duration_ms: 0,
        };

        // Sync SMS messages
        match Self::sync_messages(kde_connect, message_store).await {
            Ok(message_count) => {
                result.messages_processed = message_count;
                debug!("Synced {} messages", message_count);
            }
            Err(e) => {
                let error_msg = format!("Message sync failed: {}", e);
                error!("{}", error_msg);
                result.errors.push(error_msg.clone());
                Self::update_error_stats(stats, &error_msg).await;
            }
        }

        // Sync notifications
        match Self::sync_notifications(kde_connect).await {
            Ok(notification_count) => {
                result.notifications_processed = notification_count;
                debug!("Processed {} notifications", notification_count);
            }
            Err(e) => {
                let error_msg = format!("Notification sync failed: {}", e);
                error!("{}", error_msg);
                result.errors.push(error_msg.clone());
                Self::update_error_stats(stats, &error_msg).await;
            }
        }

        // Update statistics
        let duration = (Utc::now() - sync_start).num_milliseconds() as u64;
        result.duration_ms = duration;

        let mut stats_guard = stats.write().await;
        stats_guard.total_syncs += 1;
        stats_guard.last_sync_time = Some(sync_start);
        if !result.errors.is_empty() {
            stats_guard.total_errors += 1;
            stats_guard.last_error = result.errors.first().cloned();
        }

        if result.messages_processed > 0 || result.notifications_processed > 0 {
            info!(
                "Sync completed: {} messages, {} notifications in {}ms", 
                result.messages_processed, 
                result.notifications_processed, 
                duration
            );
        }
    }

    /// Sync SMS/MMS messages from mobile device
    async fn sync_messages(
        kde_connect: &Arc<Mutex<KdeConnectClient>>,
        message_store: &Arc<MessageStore>,
    ) -> Result<u32> {
        let mut client = kde_connect.lock().await;
        
        if !client.is_connected() {
            return Err(crate::mobile::MobileError::DeviceNotPaired("Device not connected".to_string()));
        }

        // Request latest conversations from device
        client.request_conversations()?;

        // Listen for incoming messages
        let mut message_receiver = client.listen_for_messages().await?;
        let mut messages_processed = 0u32;

        // Process messages with a timeout to avoid blocking indefinitely
        let timeout_duration = Duration::from_secs(10);
        let start_time = std::time::Instant::now();

        while start_time.elapsed() < timeout_duration {
            tokio::select! {
                maybe_message = message_receiver.recv() => {
                    match maybe_message {
                        Some(message) => {
                            debug!("Received message: ID={}, Thread={}", message.id, message.thread_id);
                            
                            match message_store.store_message(message).await {
                                Ok(conversation_id) => {
                                    messages_processed += 1;
                                    debug!("Stored message in conversation {}", conversation_id);
                                }
                                Err(e) => {
                                    error!("Failed to store message: {}", e);
                                    return Err(e);
                                }
                            }
                        }
                        None => {
                            debug!("Message channel closed");
                            break;
                        }
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(100)) => {
                    // Continue polling for messages
                }
            }
        }

        Ok(messages_processed)
    }

    /// Sync mobile notifications
    async fn sync_notifications(kde_connect: &Arc<Mutex<KdeConnectClient>>) -> Result<u32> {
        let mut client = kde_connect.lock().await;
        
        if !client.is_connected() {
            return Err(crate::mobile::MobileError::DeviceNotPaired("Device not connected".to_string()));
        }

        let mut notification_receiver = client.listen_for_notifications().await?;
        let mut notifications_processed = 0u32;

        // Process notifications with a timeout
        let timeout_duration = Duration::from_secs(5);
        let start_time = std::time::Instant::now();

        while start_time.elapsed() < timeout_duration {
            tokio::select! {
                maybe_notification = notification_receiver.recv() => {
                    match maybe_notification {
                        Some(notification) => {
                            debug!("Received notification: {} from {}", notification.title, notification.app_name);
                            
                            // Process notification (could integrate with Comunicado's notification system)
                            Self::process_mobile_notification(notification).await;
                            notifications_processed += 1;
                        }
                        None => {
                            debug!("Notification channel closed");
                            break;
                        }
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(100)) => {
                    // Continue polling for notifications
                }
            }
        }

        Ok(notifications_processed)
    }

    /// Process a mobile notification
    async fn process_mobile_notification(notification: MobileNotification) {
        debug!(
            "Processing notification: {} - {} ({})", 
            notification.app_name, 
            notification.title,
            notification.text
        );

        // Here you could integrate with Comunicado's notification system
        // For now, just log the notification
        if notification.has_reply_action {
            debug!("Notification supports reply action: {:?}", notification.reply_id);
        }
    }

    /// Reconnect to KDE Connect device
    async fn reconnect_device(kde_connect: &Arc<Mutex<KdeConnectClient>>) -> Result<()> {
        let mut client = kde_connect.lock().await;
        
        // Discover available devices
        let devices = client.discover_devices()?;
        debug!("Found {} KDE Connect devices", devices.len());

        // Try to connect to first available device with SMS support
        for device in devices {
            if device.is_reachable && device.has_sms_plugin {
                info!("Attempting to connect to device: {}", device.name);
                match client.connect_device(device.id.clone()) {
                    Ok(_) => {
                        info!("Successfully connected to device: {}", device.name);
                        return Ok(());
                    }
                    Err(e) => {
                        warn!("Failed to connect to device {}: {}", device.name, e);
                        continue;
                    }
                }
            }
        }

        Err(crate::mobile::MobileError::DeviceNotPaired("No suitable devices found".to_string()))
    }

    /// Update error statistics
    async fn update_error_stats(stats: &Arc<RwLock<MobileSyncStats>>, error_msg: &str) {
        let mut stats_guard = stats.write().await;
        stats_guard.total_errors += 1;
        stats_guard.last_error = Some(error_msg.to_string());
    }

    /// Send SMS message through connected device
    pub async fn send_sms(&self, message: &str, addresses: &[String]) -> Result<()> {
        let client = self.kde_connect.lock().await;
        
        if !client.is_connected() {
            return Err(crate::mobile::MobileError::DeviceNotPaired("Device not connected".to_string()));
        }

        info!("Sending SMS to {:?}: {}", addresses, message);
        
        match client.send_sms(message, addresses) {
            Ok(_) => {
                info!("SMS sent successfully");
                
                // Update stats
                let mut stats = self.stats.write().await;
                stats.bytes_synced += message.len() as u64;
                
                Ok(())
            }
            Err(e) => {
                error!("Failed to send SMS: {}", e);
                Self::update_error_stats(&self.stats, &format!("SMS send failed: {}", e)).await;
                Err(e)
            }
        }
    }

    /// Reply to a mobile notification
    pub async fn reply_to_notification(&self, reply_id: &str, message: &str) -> Result<()> {
        let client = self.kde_connect.lock().await;
        
        if !client.is_connected() {
            return Err(crate::mobile::MobileError::DeviceNotPaired("Device not connected".to_string()));
        }

        debug!("Replying to notification {}: {}", reply_id, message);
        
        match client.send_notification_reply(reply_id, message) {
            Ok(_) => {
                info!("Notification reply sent successfully");
                Ok(())
            }
            Err(e) => {
                error!("Failed to send notification reply: {}", e);
                Err(e)
            }
        }
    }

    /// Check if the service is running
    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }

    /// Get connected device information
    pub async fn get_connected_device(&self) -> Option<String> {
        let client = self.kde_connect.lock().await;
        client.get_connected_device().cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    async fn create_test_service() -> MobileSyncService {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_mobile.db");
        let message_store = Arc::new(MessageStore::new(db_path).await.unwrap());
        
        let kde_connect = Arc::new(Mutex::new(KdeConnectClient::new().unwrap()));
        let config = MobileConfig::default();

        MobileSyncService::new(config, kde_connect, message_store).await.unwrap()
    }

    #[tokio::test]
    async fn test_service_creation() {
        let service = create_test_service().await;
        
        assert!(!service.is_running().await);
        
        let stats = service.get_stats().await;
        assert!(!stats.is_running);
        assert_eq!(stats.total_syncs, 0);
        assert_eq!(stats.total_errors, 0);
    }

    #[tokio::test]
    async fn test_service_start_stop() {
        let service = create_test_service().await;
        
        // Start the service
        service.start().await.unwrap();
        
        // Give it a moment to start
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(service.is_running().await);
        
        let stats = service.get_stats().await;
        assert!(stats.is_running);
        
        // Stop the service
        service.stop().await.unwrap();
        
        // Give it a moment to stop
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(!service.is_running().await);
    }

    #[tokio::test]
    async fn test_control_commands() {
        let service = create_test_service().await;
        let control_handle = service.control_handle();
        
        // Start the service
        service.start().await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        // Test force sync command
        control_handle.send(ServiceControl::ForceSync).unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        // Test pause/resume commands
        control_handle.send(ServiceControl::Pause).unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        control_handle.send(ServiceControl::Resume).unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        // Stop the service
        service.stop().await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        assert!(!service.is_running().await);
    }

    #[tokio::test]
    async fn test_configuration_update() {
        let service = create_test_service().await;
        
        // Update configuration
        let mut new_config = MobileConfig::default();
        new_config.sms.sync_interval_seconds = 120; // 2 minutes
        new_config.enabled = true;
        
        service.update_config(new_config.clone()).await.unwrap();
        
        // Verify configuration was updated
        let updated_config = service.config.read().await.clone();
        assert_eq!(updated_config.sms.sync_interval_seconds, 120);
        assert!(updated_config.enabled);
    }

    #[tokio::test]
    async fn test_stats_tracking() {
        let service = create_test_service().await;
        
        // Start service to initialize stats tracking
        service.start().await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        let stats = service.get_stats().await;
        assert!(stats.is_running);
        assert_eq!(stats.sync_interval_seconds, 30); // Default from config
        assert!(stats.uptime_seconds >= 0);
        
        service.stop().await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    #[tokio::test]
    async fn test_error_handling() {
        let service = create_test_service().await;
        
        // Try to send SMS without starting service or connecting device
        let result = service.send_sms("Test message", &["+1234567890".to_string()]).await;
        assert!(result.is_err());
        
        // Try to reply to notification without connection
        let result = service.reply_to_notification("test-reply", "Test reply").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_device_connection_status() {
        let service = create_test_service().await;
        
        // Initially no device connected
        assert!(service.get_connected_device().await.is_none());
        
        // Connect to mock device
        {
            let mut client = service.kde_connect.lock().await;
            let devices = client.discover_devices().unwrap();
            if let Some(device) = devices.iter().find(|d| d.is_reachable) {
                client.connect_device(device.id.clone()).unwrap();
            }
        }
        
        // Should now have a connected device
        let connected_device = service.get_connected_device().await;
        assert!(connected_device.is_some());
    }

    #[tokio::test]
    async fn test_sync_result_tracking() {
        let service = create_test_service().await;
        
        service.start().await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        // Force a sync to test result tracking
        service.force_sync().await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        let stats = service.get_stats().await;
        // The sync should have run at least once
        assert!(stats.total_syncs >= 1);
        
        service.stop().await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        let service = Arc::new(create_test_service().await);
        
        service.start().await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        // Spawn multiple concurrent operations
        let handles = (0..5).map(|i| {
            let service = service.clone();
            tokio::spawn(async move {
                if i % 2 == 0 {
                    service.force_sync().await
                } else {
                    service.get_stats().await;
                    Ok(())
                }
            })
        }).collect::<Vec<_>>();
        
        // Wait for all operations to complete
        for handle in handles {
            handle.await.unwrap().unwrap();
        }
        
        service.stop().await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}