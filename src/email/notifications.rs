use tokio::sync::{mpsc, broadcast};
use std::sync::Arc;
use std::collections::HashMap;
use crate::email::{EmailDatabase, StoredMessage};
use uuid::Uuid;

/// Types of email notifications
#[derive(Debug, Clone)]
pub enum EmailNotification {
    /// New message received
    NewMessage {
        account_id: String,
        folder_name: String,
        message: StoredMessage,
    },
    /// Message updated (flags changed, etc.)
    MessageUpdated {
        account_id: String,
        folder_name: String,
        message_id: Uuid,
        message: StoredMessage,
    },
    /// Message deleted
    MessageDeleted {
        account_id: String,
        folder_name: String,
        message_id: Uuid,
    },
    /// Folder synchronization started
    SyncStarted {
        account_id: String,
        folder_name: String,
    },
    /// Folder synchronization completed
    SyncCompleted {
        account_id: String,
        folder_name: String,
        new_count: u32,
        updated_count: u32,
    },
    /// Folder synchronization failed
    SyncFailed {
        account_id: String,
        folder_name: String,
        error: String,
    },
}

impl EmailNotification {
    /// Get the account ID from the notification
    pub fn account_id(&self) -> &str {
        match self {
            EmailNotification::NewMessage { account_id, .. } => account_id,
            EmailNotification::MessageUpdated { account_id, .. } => account_id,
            EmailNotification::MessageDeleted { account_id, .. } => account_id,
            EmailNotification::SyncStarted { account_id, .. } => account_id,
            EmailNotification::SyncCompleted { account_id, .. } => account_id,
            EmailNotification::SyncFailed { account_id, .. } => account_id,
        }
    }
    
    /// Get the folder name from the notification
    pub fn folder_name(&self) -> &str {
        match self {
            EmailNotification::NewMessage { folder_name, .. } => folder_name,
            EmailNotification::MessageUpdated { folder_name, .. } => folder_name,
            EmailNotification::MessageDeleted { folder_name, .. } => folder_name,
            EmailNotification::SyncStarted { folder_name, .. } => folder_name,
            EmailNotification::SyncCompleted { folder_name, .. } => folder_name,
            EmailNotification::SyncFailed { folder_name, .. } => folder_name,
        }
    }
}

/// Email notification manager
pub struct EmailNotificationManager {
    /// Broadcast sender for sending notifications to all subscribers
    sender: broadcast::Sender<EmailNotification>,
    /// Channel for receiving notifications from email sync processes
    notification_receiver: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<EmailNotification>>>,
    /// Sender for internal notification publishing
    notification_sender: mpsc::UnboundedSender<EmailNotification>,
    /// Database reference for message operations
    database: Arc<EmailDatabase>,
}

impl EmailNotificationManager {
    /// Create a new email notification manager
    pub fn new(database: Arc<EmailDatabase>) -> Self {
        let (sender, _) = broadcast::channel(1000); // Buffer for 1000 notifications
        let (notification_sender, notification_receiver) = mpsc::unbounded_channel();
        
        Self {
            sender,
            notification_receiver: Arc::new(tokio::sync::Mutex::new(notification_receiver)),
            notification_sender,
            database,
        }
    }
    
    /// Subscribe to email notifications
    pub fn subscribe(&self) -> broadcast::Receiver<EmailNotification> {
        self.sender.subscribe()
    }
    
    /// Get a sender for publishing notifications
    pub fn get_sender(&self) -> mpsc::UnboundedSender<EmailNotification> {
        self.notification_sender.clone()
    }
    
    /// Start the notification processing loop
    pub async fn start(&self) {
        let receiver = self.notification_receiver.clone();
        let sender = self.sender.clone();
        
        tokio::spawn(async move {
            let mut receiver = receiver.lock().await;
            
            while let Some(notification) = receiver.recv().await {
                // Broadcast the notification to all subscribers
                if let Err(e) = sender.send(notification.clone()) {
                    tracing::warn!("Failed to broadcast notification: {}", e);
                }
                
                // Log the notification for debugging
                tracing::debug!("Processed notification: {:?}", notification);
            }
        });
    }
    
    /// Publish a new message notification
    pub async fn notify_new_message(&self, account_id: String, folder_name: String, message: StoredMessage) {
        let notification = EmailNotification::NewMessage {
            account_id,
            folder_name,
            message,
        };
        
        if let Err(e) = self.notification_sender.send(notification) {
            tracing::error!("Failed to send new message notification: {}", e);
        }
    }
    
    /// Publish a message updated notification
    pub async fn notify_message_updated(&self, account_id: String, folder_name: String, message: StoredMessage) {
        let notification = EmailNotification::MessageUpdated {
            account_id,
            folder_name,
            message_id: message.id,
            message,
        };
        
        if let Err(e) = self.notification_sender.send(notification) {
            tracing::error!("Failed to send message updated notification: {}", e);
        }
    }
    
    /// Publish a message deleted notification
    pub async fn notify_message_deleted(&self, account_id: String, folder_name: String, message_id: Uuid) {
        let notification = EmailNotification::MessageDeleted {
            account_id,
            folder_name,
            message_id,
        };
        
        if let Err(e) = self.notification_sender.send(notification) {
            tracing::error!("Failed to send message deleted notification: {}", e);
        }
    }
    
    /// Publish a sync started notification
    pub async fn notify_sync_started(&self, account_id: String, folder_name: String) {
        let notification = EmailNotification::SyncStarted {
            account_id,
            folder_name,
        };
        
        if let Err(e) = self.notification_sender.send(notification) {
            tracing::error!("Failed to send sync started notification: {}", e);
        }
    }
    
    /// Publish a sync completed notification
    pub async fn notify_sync_completed(&self, account_id: String, folder_name: String, new_count: u32, updated_count: u32) {
        let notification = EmailNotification::SyncCompleted {
            account_id,
            folder_name,
            new_count,
            updated_count,
        };
        
        if let Err(e) = self.notification_sender.send(notification) {
            tracing::error!("Failed to send sync completed notification: {}", e);
        }
    }
    
    /// Publish a sync failed notification
    pub async fn notify_sync_failed(&self, account_id: String, folder_name: String, error: String) {
        let notification = EmailNotification::SyncFailed {
            account_id,
            folder_name,
            error,
        };
        
        if let Err(e) = self.notification_sender.send(notification) {
            tracing::error!("Failed to send sync failed notification: {}", e);
        }
    }
}

/// Email update handler for the UI
pub struct UIEmailUpdater {
    /// Notification receiver
    notification_receiver: broadcast::Receiver<EmailNotification>,
    /// Current subscriptions (account_id, folder_name)
    subscriptions: HashMap<String, Vec<String>>,
}

impl UIEmailUpdater {
    /// Create a new UI email updater
    pub fn new(notification_manager: &EmailNotificationManager) -> Self {
        Self {
            notification_receiver: notification_manager.subscribe(),
            subscriptions: HashMap::new(),
        }
    }
    
    /// Subscribe to updates for a specific account and folder
    pub fn subscribe_to_folder(&mut self, account_id: String, folder_name: String) {
        self.subscriptions
            .entry(account_id)
            .or_default()
            .push(folder_name);
    }
    
    /// Unsubscribe from updates for a specific account and folder
    pub fn unsubscribe_from_folder(&mut self, account_id: &str, folder_name: &str) {
        if let Some(folders) = self.subscriptions.get_mut(account_id) {
            folders.retain(|f| f != folder_name);
            if folders.is_empty() {
                self.subscriptions.remove(account_id);
            }
        }
    }
    
    /// Check for new notifications (non-blocking)
    pub fn try_recv_notification(&mut self) -> Option<EmailNotification> {
        match self.notification_receiver.try_recv() {
            Ok(notification) => {
                // Check if we're subscribed to this notification
                if self.is_subscribed_to(&notification) {
                    Some(notification)
                } else {
                    None
                }
            }
            Err(broadcast::error::TryRecvError::Empty) => None,
            Err(broadcast::error::TryRecvError::Lagged(count)) => {
                tracing::warn!("Notification receiver lagged by {} messages", count);
                None
            }
            Err(broadcast::error::TryRecvError::Closed) => {
                tracing::error!("Notification channel closed");
                None
            }
        }
    }
    
    /// Check if we're subscribed to a notification
    fn is_subscribed_to(&self, notification: &EmailNotification) -> bool {
        let account_id = notification.account_id();
        let folder_name = notification.folder_name();
        
        if let Some(folders) = self.subscriptions.get(account_id) {
            folders.contains(&folder_name.to_string())
        } else {
            false
        }
    }
    
    /// Get all current subscriptions
    pub fn get_subscriptions(&self) -> &HashMap<String, Vec<String>> {
        &self.subscriptions
    }
    
    /// Clear all subscriptions
    pub fn clear_subscriptions(&mut self) {
        self.subscriptions.clear();
    }
}

/// Helper trait for components that can handle email notifications
pub trait EmailNotificationHandler {
    /// Handle a new message notification
    fn handle_new_message(&mut self, account_id: &str, folder_name: &str, message: &StoredMessage);
    
    /// Handle a message updated notification
    fn handle_message_updated(&mut self, account_id: &str, folder_name: &str, message: &StoredMessage);
    
    /// Handle a message deleted notification
    fn handle_message_deleted(&mut self, account_id: &str, folder_name: &str, message_id: Uuid);
    
    /// Handle a sync started notification
    fn handle_sync_started(&mut self, account_id: &str, folder_name: &str);
    
    /// Handle a sync completed notification
    fn handle_sync_completed(&mut self, account_id: &str, folder_name: &str, new_count: u32, updated_count: u32);
    
    /// Handle a sync failed notification
    fn handle_sync_failed(&mut self, account_id: &str, folder_name: &str, error: &str);
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use crate::email::{EmailDatabase, StoredMessage};
    use chrono::Utc;
    
    #[tokio::test]
    async fn test_notification_manager_creation() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db_path_str = db_path.to_str().unwrap();
        
        let database = Arc::new(EmailDatabase::new(db_path_str).await.unwrap());
        let manager = EmailNotificationManager::new(database);
        
        // Should be able to create subscribers
        let _receiver = manager.subscribe();
        let _sender = manager.get_sender();
    }
    
    #[tokio::test]
    async fn test_notification_publishing() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db_path_str = db_path.to_str().unwrap();
        
        let database = Arc::new(EmailDatabase::new(db_path_str).await.unwrap());
        let manager = EmailNotificationManager::new(database);
        
        // Start the notification processing
        manager.start().await;
        
        // Subscribe to notifications
        let mut receiver = manager.subscribe();
        
        // Create a sample message
        let message = StoredMessage {
            id: Uuid::new_v4(),
            account_id: "test-account".to_string(),
            folder_name: "INBOX".to_string(),
            imap_uid: 1,
            message_id: Some("test@example.com".to_string()),
            thread_id: None,
            in_reply_to: None,
            references: vec![],
            subject: "Test Subject".to_string(),
            from_addr: "sender@example.com".to_string(),
            from_name: Some("Test Sender".to_string()),
            to_addrs: vec!["recipient@example.com".to_string()],
            cc_addrs: vec![],
            bcc_addrs: vec![],
            reply_to: None,
            date: Utc::now(),
            body_text: Some("Test body".to_string()),
            body_html: None,
            attachments: vec![],
            flags: vec![],
            labels: vec![],
            size: Some(100),
            priority: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_synced: Utc::now(),
            sync_version: 1,
            is_draft: false,
            is_deleted: false,
        };
        
        // Publish a notification
        manager.notify_new_message("test-account".to_string(), "INBOX".to_string(), message.clone()).await;
        
        // Should receive the notification
        tokio::select! {
            result = receiver.recv() => {
                match result {
                    Ok(EmailNotification::NewMessage { account_id, folder_name, message: received_message }) => {
                        assert_eq!(account_id, "test-account");
                        assert_eq!(folder_name, "INBOX");
                        assert_eq!(received_message.subject, "Test Subject");
                    }
                    Ok(other) => panic!("Received unexpected notification: {:?}", other),
                    Err(e) => panic!("Failed to receive notification: {}", e),
                }
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                panic!("Timeout waiting for notification");
            }
        }
    }
    
    #[tokio::test]
    async fn test_ui_updater_subscriptions() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db_path_str = db_path.to_str().unwrap();
        
        let database = Arc::new(EmailDatabase::new(db_path_str).await.unwrap());
        let manager = EmailNotificationManager::new(database);
        
        let mut updater = UIEmailUpdater::new(&manager);
        
        // Subscribe to a folder
        updater.subscribe_to_folder("test-account".to_string(), "INBOX".to_string());
        updater.subscribe_to_folder("test-account".to_string(), "Sent".to_string());
        
        // Check subscriptions
        let subscriptions = updater.get_subscriptions();
        assert!(subscriptions.contains_key("test-account"));
        assert_eq!(subscriptions.get("test-account").unwrap().len(), 2);
        
        // Unsubscribe
        updater.unsubscribe_from_folder("test-account", "Sent");
        let subscriptions = updater.get_subscriptions();
        assert_eq!(subscriptions.get("test-account").unwrap().len(), 1);
        
        // Clear all subscriptions
        updater.clear_subscriptions();
        assert!(updater.get_subscriptions().is_empty());
    }
}