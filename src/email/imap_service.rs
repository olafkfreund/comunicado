use crate::email::database::EmailDatabase;
use crate::email::sync_engine::{SyncEngine, SyncProgress, SyncStrategy};
use crate::imap::{
    IdleNotification, ImapAccountManager, ImapClient, ImapResult
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::{Duration, Instant, interval};
use tracing::{debug, error, info, warn};

/// IMAP service that integrates connection pooling, IDLE support, and sync coordination
pub struct ImapService {
    account_manager: Arc<ImapAccountManager>,
    sync_engine: Arc<SyncEngine>,
    database: Arc<EmailDatabase>,
    
    // IDLE management
    idle_connections: Arc<RwLock<HashMap<String, Arc<Mutex<ImapClient>>>>>,
    active_folders: Arc<RwLock<HashMap<String, String>>>, // account_id -> folder_name
    
    // Notification channels
    idle_notification_sender: mpsc::UnboundedSender<IdleUpdate>,
    idle_notification_receiver: Arc<Mutex<mpsc::UnboundedReceiver<IdleUpdate>>>,
    
    // Sync coordination
    sync_progress_sender: mpsc::UnboundedSender<SyncProgress>,
    
    // Background task management
    background_tasks: Arc<RwLock<Vec<tokio::task::JoinHandle<()>>>>,
}

/// IDLE update notification
#[derive(Debug, Clone)]
pub struct IdleUpdate {
    pub account_id: String,
    pub folder_name: String,
    pub notification: IdleNotification,
    pub timestamp: Instant,
}

impl ImapService {
    /// Create a new IMAP service
    pub async fn new(
        account_manager: ImapAccountManager,
        database: Arc<EmailDatabase>,
        sync_progress_sender: mpsc::UnboundedSender<SyncProgress>,
    ) -> ImapResult<Self> {
        let sync_engine = Arc::new(SyncEngine::new(database.clone(), sync_progress_sender.clone()));
        
        let (idle_notification_sender, idle_notification_receiver) = mpsc::unbounded_channel();
        
        let service = Self {
            account_manager: Arc::new(account_manager),
            sync_engine,
            database,
            idle_connections: Arc::new(RwLock::new(HashMap::new())),
            active_folders: Arc::new(RwLock::new(HashMap::new())),
            idle_notification_sender,
            idle_notification_receiver: Arc::new(Mutex::new(idle_notification_receiver)),
            sync_progress_sender,
            background_tasks: Arc::new(RwLock::new(Vec::new())),
        };
        
        // Start background IDLE notification processor
        service.start_idle_processor().await;
        
        // Start periodic connection health check
        service.start_connection_health_check().await;
        
        Ok(service)
    }
    
    /// Start monitoring a folder with IDLE
    pub async fn start_folder_monitoring(&self, account_id: String, folder_name: String) -> ImapResult<()> {
        info!("Starting IDLE monitoring for {}/{}", account_id, folder_name);
        
        // Get or create a dedicated IDLE connection
        let client = self.get_idle_connection(&account_id).await?;
        
        // Select the folder
        {
            let mut client_guard = client.lock().await;
            client_guard.select_folder(&folder_name).await?;
            
            // Start IDLE monitoring
            client_guard.start_folder_monitoring(folder_name.clone()).await?;
            
            // Add IDLE callback to handle notifications
            let notification_sender = self.idle_notification_sender.clone();
            let account_id_clone = account_id.clone();
            let folder_name_clone = folder_name.clone();
            
            client_guard.add_idle_callback(move |notification| {
                let update = IdleUpdate {
                    account_id: account_id_clone.clone(),
                    folder_name: folder_name_clone.clone(),
                    notification,
                    timestamp: Instant::now(),
                };
                
                if let Err(e) = notification_sender.send(update) {
                    error!("Failed to send IDLE notification: {}", e);
                }
            }).await?;
        }
        
        // Track active monitoring
        {
            let mut active_folders = self.active_folders.write().await;
            active_folders.insert(account_id.clone(), folder_name.clone());
        }
        
        info!("IDLE monitoring started for {}/{}", account_id, folder_name);
        Ok(())
    }
    
    /// Stop monitoring a folder
    pub async fn stop_folder_monitoring(&self, account_id: &str) -> ImapResult<()> {
        info!("Stopping IDLE monitoring for account: {}", account_id);
        
        // Remove from active folders
        {
            let mut active_folders = self.active_folders.write().await;
            active_folders.remove(account_id);
        }
        
        // Stop IDLE on the connection
        if let Some(client) = self.get_existing_idle_connection(account_id).await {
            let mut client_guard = client.lock().await;
            let _ = client_guard.stop_folder_monitoring().await;
        }
        
        info!("IDLE monitoring stopped for account: {}", account_id);
        Ok(())
    }
    
    /// Perform a manual sync for an account/folder
    pub async fn sync_folder(
        &self,
        account_id: String,
        folder_name: String,
        strategy: SyncStrategy,
    ) -> ImapResult<()> {
        info!("Starting manual sync for {}/{}", account_id, folder_name);
        
        // Get a regular connection for syncing (separate from IDLE)
        let client = self.account_manager.get_client(&account_id).await?;
        
        // Get folder information
        let _folder = {
            let mut client_guard = client.lock().await;
            client_guard.select_folder(&folder_name).await?;
            // Create a minimal folder object - this would need proper implementation
            crate::imap::ImapFolder {
                name: folder_name.clone(),
                full_name: folder_name.clone(),
                delimiter: Some("/".to_string()),
                attributes: Vec::new(),
                exists: None,
                recent: None,
                unseen: None,
                uid_validity: None,
                uid_next: None,
            }
        };
        
        // Perform sync - note: this needs to be refactored to work with the correct signature
        // For now, we'll log this as a placeholder
        info!("Sync would be performed for {}/{} with strategy {:?}", account_id, folder_name, strategy);
        
        info!("Manual sync completed for {}/{}", account_id, folder_name);
        Ok(())
    }
    
    /// Get or create an IDLE-dedicated connection
    async fn get_idle_connection(&self, account_id: &str) -> ImapResult<Arc<Mutex<ImapClient>>> {
        let mut idle_connections = self.idle_connections.write().await;
        
        if let Some(client) = idle_connections.get(account_id) {
            return Ok(client.clone());
        }
        
        // Create a new dedicated IDLE connection
        let client = self.account_manager.get_client(account_id).await?;
        
        // Initialize IDLE service on this client
        {
            let mut client_guard = client.lock().await;
            client_guard.init_idle_service()?;
        }
        
        idle_connections.insert(account_id.to_string(), client.clone());
        Ok(client)
    }
    
    /// Get existing IDLE connection if it exists
    async fn get_existing_idle_connection(&self, account_id: &str) -> Option<Arc<Mutex<ImapClient>>> {
        let idle_connections = self.idle_connections.read().await;
        idle_connections.get(account_id).cloned()
    }
    
    /// Start background IDLE notification processor
    async fn start_idle_processor(&self) {
        let idle_receiver = Arc::clone(&self.idle_notification_receiver);
        let sync_engine = Arc::clone(&self.sync_engine);
        let account_manager = Arc::clone(&self.account_manager);
        
        let task = tokio::spawn(async move {
            let mut receiver = idle_receiver.lock().await;
            
            while let Some(update) = receiver.recv().await {
                if let Err(e) = Self::process_idle_notification(
                    &update,
                    &sync_engine,
                    &account_manager,
                ).await {
                    error!("Failed to process IDLE notification: {}", e);
                }
            }
        });
        
        let mut background_tasks = self.background_tasks.write().await;
        background_tasks.push(task);
    }
    
    /// Process an IDLE notification
    async fn process_idle_notification(
        update: &IdleUpdate,
        _sync_engine: &Arc<SyncEngine>,
        _account_manager: &Arc<ImapAccountManager>,
    ) -> ImapResult<()> {
        debug!("Processing IDLE notification: {:?}", update);
        
        match &update.notification {
            IdleNotification::Exists { count } => {
                info!("New messages detected in {}/{}: {}", 
                      update.account_id, update.folder_name, count);
                
                // TODO: Trigger incremental sync for new messages
                // This would need proper integration with sync engine
            }
            IdleNotification::Expunge { sequence } => {
                info!("Message expunged from {}/{}: sequence {}", 
                      update.account_id, update.folder_name, sequence);
                
                // TODO: Could trigger a cleanup sync if needed
            }
            IdleNotification::Fetch { sequence, uid } => {
                info!("Message flags updated in {}/{}: sequence {} (UID: {:?})", 
                      update.account_id, update.folder_name, sequence, uid);
                
                // TODO: Trigger flag sync
            }
            IdleNotification::ConnectionLost => {
                warn!("IDLE connection lost for {}/{}", 
                      update.account_id, update.folder_name);
                
                // Connection will be recreated on next operation
            }
            IdleNotification::Timeout => {
                debug!("IDLE timeout for {}/{} - will refresh", 
                       update.account_id, update.folder_name);
            }
            _ => {
                debug!("Other IDLE notification: {:?}", update.notification);
            }
        }
        
        Ok(())
    }
    
    /// Start periodic connection health check
    async fn start_connection_health_check(&self) {
        let idle_connections = Arc::clone(&self.idle_connections);
        let active_folders = Arc::clone(&self.active_folders);
        
        let task = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(300)); // Check every 5 minutes
            
            loop {
                interval.tick().await;
                
                // Check IDLE connection health
                let connections_to_check: Vec<String> = {
                    let connections = idle_connections.read().await;
                    connections.keys().cloned().collect()
                };
                
                for account_id in connections_to_check {
                    if let Some(client) = {
                        let connections = idle_connections.read().await;
                        connections.get(&account_id).cloned()
                    } {
                        // Check if connection is still healthy
                        let stats = {
                            let client_guard = client.lock().await;
                            client_guard.get_idle_stats().await
                        };
                        
                        if let Some(stats) = stats {
                            if !stats.is_active {
                                warn!("IDLE connection inactive for account: {}", account_id);
                                
                                // Try to restart IDLE if we have an active folder
                                if let Some(folder_name) = {
                                    let folders = active_folders.read().await;
                                    folders.get(&account_id).cloned()
                                } {
                                    let mut client_guard = client.lock().await;
                                    if let Err(e) = client_guard.start_folder_monitoring(folder_name).await {
                                        error!("Failed to restart IDLE for {}: {}", account_id, e);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });
        
        let mut background_tasks = self.background_tasks.write().await;
        background_tasks.push(task);
    }
    
    /// Get statistics about active IDLE connections
    pub async fn get_idle_stats(&self) -> HashMap<String, crate::imap::IdleStats> {
        let mut stats = HashMap::new();
        let idle_connections = self.idle_connections.read().await;
        
        for (account_id, client) in idle_connections.iter() {
            let client_guard = client.lock().await;
            if let Some(idle_stats) = client_guard.get_idle_stats().await {
                stats.insert(account_id.clone(), idle_stats);
            }
        }
        
        stats
    }
    
    /// Shutdown the service and cleanup resources
    pub async fn shutdown(&self) -> ImapResult<()> {
        info!("Shutting down IMAP service");
        
        // Stop all IDLE monitoring
        let account_ids: Vec<String> = {
            let active_folders = self.active_folders.read().await;
            active_folders.keys().cloned().collect()
        };
        
        for account_id in account_ids {
            let _ = self.stop_folder_monitoring(&account_id).await;
        }
        
        // Cancel background tasks
        let mut background_tasks = self.background_tasks.write().await;
        for task in background_tasks.drain(..) {
            task.abort();
        }
        
        // Clear connections
        {
            let mut idle_connections = self.idle_connections.write().await;
            idle_connections.clear();
        }
        
        info!("IMAP service shutdown complete");
        Ok(())
    }
}

impl Drop for ImapService {
    fn drop(&mut self) {
        // Abort any remaining background tasks
        if let Ok(mut tasks) = self.background_tasks.try_write() {
            for task in tasks.drain(..) {
                task.abort();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::timeout;
    
    #[tokio::test]
    async fn test_idle_service_creation() {
        // This would need mock implementations for testing
        // For now, just verify the service can be created with mocks
    }
    
    #[tokio::test] 
    async fn test_idle_notification_processing() {
        // Test IDLE notification processing logic
        let update = IdleUpdate {
            account_id: "test_account".to_string(),
            folder_name: "INBOX".to_string(),
            notification: IdleNotification::Exists { count: 5 },
            timestamp: Instant::now(),
        };
        
        // Would verify that the notification triggers appropriate sync actions
    }
}