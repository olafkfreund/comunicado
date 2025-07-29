use crate::imap::protocol::ImapProtocol;
use crate::imap::{ImapConnection, ImapError, ImapResult};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::{interval, timeout, Instant};
use tracing::{debug, error, info, warn};

/// Types of IDLE notifications
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdleNotification {
    /// New message exists in the folder
    Exists { count: u32 },
    /// Recent messages count changed
    Recent { count: u32 },
    /// Message flags changed
    Expunge { sequence: u32 },
    /// Fetch notification for updated message
    Fetch { sequence: u32, uid: Option<u32> },
    /// IDLE connection was lost
    ConnectionLost,
    /// IDLE timeout occurred
    Timeout,
}

/// IDLE response parser
pub struct IdleResponseParser;

impl IdleResponseParser {
    /// Parse IDLE notification responses
    pub fn parse_idle_response(response: &str) -> Vec<IdleNotification> {
        let mut notifications = Vec::new();

        for line in response.lines() {
            let line = line.trim();

            if line.starts_with("* ") {
                if let Some(notification) = Self::parse_untagged_response(line) {
                    notifications.push(notification);
                }
            }
        }

        notifications
    }

    fn parse_untagged_response(line: &str) -> Option<IdleNotification> {
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.len() < 3 {
            return None;
        }

        // Skip the "*" prefix
        let number_str = parts[1];
        let command = parts[2];

        match command {
            "EXISTS" => {
                if let Ok(count) = number_str.parse::<u32>() {
                    Some(IdleNotification::Exists { count })
                } else {
                    None
                }
            }
            "RECENT" => {
                if let Ok(count) = number_str.parse::<u32>() {
                    Some(IdleNotification::Recent { count })
                } else {
                    None
                }
            }
            "EXPUNGE" => {
                if let Ok(sequence) = number_str.parse::<u32>() {
                    Some(IdleNotification::Expunge { sequence })
                } else {
                    None
                }
            }
            "FETCH" => {
                if let Ok(sequence) = number_str.parse::<u32>() {
                    let uid = Self::extract_uid_from_fetch_response(line);
                    Some(IdleNotification::Fetch { sequence, uid })
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn extract_uid_from_fetch_response(line: &str) -> Option<u32> {
        // Look for UID in FETCH response
        // Example: * 1 FETCH (UID 1234 FLAGS (\Seen))
        if let Some(uid_start) = line.find("UID ") {
            let uid_part = &line[uid_start + 4..];
            if let Some(space_pos) = uid_part.find(' ') {
                uid_part[..space_pos].parse().ok()
            } else {
                uid_part.trim_end_matches(')').parse().ok()
            }
        } else {
            None
        }
    }
}

/// IDLE connection manager for real-time updates
pub struct IdleManager {
    connection: Arc<Mutex<ImapConnection>>,
    notification_sender: mpsc::UnboundedSender<IdleNotification>,
    is_idle: Arc<RwLock<bool>>,
    selected_folder: Arc<RwLock<Option<String>>>,
    last_heartbeat: Arc<RwLock<Instant>>,
    idle_timeout: Duration,
    heartbeat_interval: Duration,
}

impl IdleManager {
    /// Create a new IDLE manager
    pub fn new(
        connection: Arc<Mutex<ImapConnection>>,
        notification_sender: mpsc::UnboundedSender<IdleNotification>,
    ) -> Self {
        Self {
            connection,
            notification_sender,
            is_idle: Arc::new(RwLock::new(false)),
            selected_folder: Arc::new(RwLock::new(None)),
            last_heartbeat: Arc::new(RwLock::new(Instant::now())),
            idle_timeout: Duration::from_secs(29 * 60), // 29 minutes (RFC recommends < 30 min)
            heartbeat_interval: Duration::from_secs(60), // Check every minute
        }
    }

    /// Start IDLE mode for the selected folder
    pub async fn start_idle(&self, folder_name: String) -> ImapResult<()> {
        let mut is_idle = self.is_idle.write().await;
        if *is_idle {
            return Err(ImapError::invalid_state("Already in IDLE mode"));
        }

        // Update selected folder
        {
            let mut selected = self.selected_folder.write().await;
            *selected = Some(folder_name.clone());
        }

        // Start IDLE command
        let mut connection = self.connection.lock().await;
        let command = ImapProtocol::format_idle();
        let initial_response = connection.send_command(&command).await?;

        // Check if IDLE started successfully
        if !initial_response.contains("+ idling") && !initial_response.contains("+ IDLE") {
            return Err(ImapError::protocol("Failed to start IDLE mode"));
        }

        *is_idle = true;
        drop(connection); // Release the connection lock

        // Update heartbeat
        {
            let mut heartbeat = self.last_heartbeat.write().await;
            *heartbeat = Instant::now();
        }

        info!("Started IDLE mode for folder: {}", folder_name);

        // Start background tasks
        self.start_idle_listener().await?;
        self.start_heartbeat_monitor().await?;

        Ok(())
    }

    /// Stop IDLE mode
    pub async fn stop_idle(&self) -> ImapResult<()> {
        let mut is_idle = self.is_idle.write().await;
        if !*is_idle {
            return Ok(()); // Already stopped
        }

        // Send DONE command to exit IDLE
        let mut connection = self.connection.lock().await;
        let command = ImapProtocol::format_done();
        let _response = connection.send_command(&command).await?;

        *is_idle = false;

        // Clear selected folder
        {
            let mut selected = self.selected_folder.write().await;
            *selected = None;
        }

        info!("Stopped IDLE mode");
        Ok(())
    }

    /// Check if currently in IDLE mode
    pub async fn is_idle(&self) -> bool {
        *self.is_idle.read().await
    }

    /// Get the current IDLE folder
    pub async fn get_idle_folder(&self) -> Option<String> {
        self.selected_folder.read().await.clone()
    }

    /// Start the IDLE response listener
    async fn start_idle_listener(&self) -> ImapResult<()> {
        let connection = Arc::clone(&self.connection);
        let sender = self.notification_sender.clone();
        let is_idle = Arc::clone(&self.is_idle);
        let heartbeat = Arc::clone(&self.last_heartbeat);

        tokio::spawn(async move {
            while *is_idle.read().await {
                // Try to read response with timeout
                let response_result = {
                    let mut conn = connection.lock().await;
                    timeout(Duration::from_secs(30), conn.read_response()).await
                };

                match response_result {
                    Ok(Ok(response)) => {
                        // Update heartbeat on successful response
                        {
                            let mut last_heartbeat = heartbeat.write().await;
                            *last_heartbeat = Instant::now();
                        }

                        // Parse and send notifications
                        let notifications = IdleResponseParser::parse_idle_response(&response);
                        for notification in notifications {
                            debug!("IDLE notification: {:?}", notification);
                            if sender.send(notification).is_err() {
                                warn!("Failed to send IDLE notification - receiver dropped");
                                break;
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        error!("IDLE connection error: {}", e);
                        let _ = sender.send(IdleNotification::ConnectionLost);
                        break;
                    }
                    Err(_) => {
                        // Timeout occurred - this is normal for IDLE
                        debug!("IDLE timeout occurred - connection still alive");
                    }
                }

                // Small delay to prevent busy loop
                tokio::time::sleep(Duration::from_millis(100)).await;
            }

            debug!("IDLE listener task ended");
        });

        Ok(())
    }

    /// Start the heartbeat monitor
    async fn start_heartbeat_monitor(&self) -> ImapResult<()> {
        let is_idle = Arc::clone(&self.is_idle);
        let heartbeat = Arc::clone(&self.last_heartbeat);
        let sender = self.notification_sender.clone();
        let heartbeat_interval = self.heartbeat_interval;
        let idle_timeout = self.idle_timeout;

        tokio::spawn(async move {
            let mut interval = interval(heartbeat_interval);

            while *is_idle.read().await {
                interval.tick().await;

                let last_heartbeat = *heartbeat.read().await;
                let elapsed = last_heartbeat.elapsed();

                if elapsed > idle_timeout {
                    warn!("IDLE timeout exceeded: {:?}", elapsed);
                    let _ = sender.send(IdleNotification::Timeout);
                    break;
                }

                debug!("IDLE heartbeat check: {:?} elapsed", elapsed);
            }

            debug!("IDLE heartbeat monitor ended");
        });

        Ok(())
    }

    /// Force refresh the IDLE connection
    pub async fn refresh_idle(&self) -> ImapResult<()> {
        if !self.is_idle().await {
            return Ok(());
        }

        let folder = if let Some(folder) = self.get_idle_folder().await {
            folder
        } else {
            return Err(ImapError::invalid_state("No folder selected for IDLE"));
        };

        // Stop and restart IDLE
        self.stop_idle().await?;
        tokio::time::sleep(Duration::from_millis(100)).await; // Brief pause
        self.start_idle(folder).await?;

        info!("IDLE connection refreshed");
        Ok(())
    }
}

/// High-level IDLE notification service
pub struct IdleNotificationService {
    idle_manager: Arc<IdleManager>,
    notification_receiver: Arc<Mutex<mpsc::UnboundedReceiver<IdleNotification>>>,
    callbacks: Arc<RwLock<Vec<Box<dyn Fn(IdleNotification) + Send + Sync>>>>,
}

impl IdleNotificationService {
    /// Create a new notification service
    pub fn new(connection: Arc<Mutex<ImapConnection>>) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let idle_manager = Arc::new(IdleManager::new(connection, sender));

        Self {
            idle_manager,
            notification_receiver: Arc::new(Mutex::new(receiver)),
            callbacks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Start monitoring a folder for changes
    pub async fn start_monitoring(&self, folder_name: String) -> ImapResult<()> {
        self.idle_manager.start_idle(folder_name).await?;
        self.start_notification_dispatcher().await;
        Ok(())
    }

    /// Stop monitoring
    pub async fn stop_monitoring(&self) -> ImapResult<()> {
        self.idle_manager.stop_idle().await
    }

    /// Add a callback for notifications
    pub async fn add_callback<F>(&self, callback: F)
    where
        F: Fn(IdleNotification) + Send + Sync + 'static,
    {
        let mut callbacks = self.callbacks.write().await;
        callbacks.push(Box::new(callback));
    }

    /// Start the notification dispatcher
    async fn start_notification_dispatcher(&self) {
        let receiver = Arc::clone(&self.notification_receiver);
        let callbacks = Arc::clone(&self.callbacks);
        let idle_manager = Arc::clone(&self.idle_manager);

        tokio::spawn(async move {
            loop {
                let notification = {
                    let mut receiver = receiver.lock().await;
                    receiver.recv().await
                };

                match notification {
                    Some(notification) => {
                        // Handle special notifications
                        match &notification {
                            IdleNotification::ConnectionLost => {
                                warn!("IDLE connection lost - attempting to reconnect");
                                if let Err(e) = idle_manager.refresh_idle().await {
                                    error!("Failed to refresh IDLE connection: {}", e);
                                }
                            }
                            IdleNotification::Timeout => {
                                info!("IDLE timeout - refreshing connection");
                                if let Err(e) = idle_manager.refresh_idle().await {
                                    error!(
                                        "Failed to refresh IDLE connection after timeout: {}",
                                        e
                                    );
                                }
                            }
                            _ => {}
                        }

                        // Notify all callbacks
                        let callbacks = callbacks.read().await;
                        for callback in callbacks.iter() {
                            callback(notification.clone());
                        }
                    }
                    None => {
                        debug!("Notification channel closed");
                        break;
                    }
                }
            }
        });
    }

    /// Get statistics about IDLE operations
    pub async fn get_stats(&self) -> IdleStats {
        IdleStats {
            is_active: self.idle_manager.is_idle().await,
            monitored_folder: self.idle_manager.get_idle_folder().await,
            callback_count: self.callbacks.read().await.len(),
        }
    }
}

/// IDLE statistics
#[derive(Debug, Clone)]
pub struct IdleStats {
    pub is_active: bool,
    pub monitored_folder: Option<String>,
    pub callback_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_idle_response_parsing() {
        let response = "* 1 EXISTS\n* 0 RECENT\n* 1 FETCH (UID 1234 FLAGS (\\Seen))";
        let notifications = IdleResponseParser::parse_idle_response(response);

        assert_eq!(notifications.len(), 3);

        match &notifications[0] {
            IdleNotification::Exists { count } => assert_eq!(*count, 1),
            _ => panic!("Expected Exists notification"),
        }

        match &notifications[1] {
            IdleNotification::Recent { count } => assert_eq!(*count, 0),
            _ => panic!("Expected Recent notification"),
        }

        match &notifications[2] {
            IdleNotification::Fetch { sequence, uid } => {
                assert_eq!(*sequence, 1);
                assert_eq!(*uid, Some(1234));
            }
            _ => panic!("Expected Fetch notification"),
        }
    }

    #[test]
    fn test_expunge_parsing() {
        let response = "* 5 EXPUNGE";
        let notifications = IdleResponseParser::parse_idle_response(response);

        assert_eq!(notifications.len(), 1);
        match &notifications[0] {
            IdleNotification::Expunge { sequence } => assert_eq!(*sequence, 5),
            _ => panic!("Expected Expunge notification"),
        }
    }

    #[test]
    fn test_invalid_response_parsing() {
        let response = "* INVALID RESPONSE\n+ OK IDLE";
        let notifications = IdleResponseParser::parse_idle_response(response);

        // Should ignore invalid responses
        assert_eq!(notifications.len(), 0);
    }
}
