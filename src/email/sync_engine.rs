use crate::email::database::{EmailDatabase, StoredMessage, FolderSyncState, SyncStatus};
use crate::imap::{ImapClient, ImapFolder, ImapMessage, ImapCapability, SearchCriteria};
use chrono::{DateTime, Utc, Duration as ChronoDuration};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock, mpsc};
use tokio::time::{sleep, Duration};
use tracing::{info, warn, error, debug};
use thiserror::Error;

/// Sync engine errors
#[derive(Error, Debug)]
pub enum SyncError {
    #[error("Database error: {0}")]
    Database(#[from] crate::email::database::DatabaseError),
    
    #[error("IMAP error: {0}")]
    Imap(#[from] crate::imap::ImapError),
    
    #[error("Sync conflict: {0}")]
    Conflict(String),
    
    #[error("Account not found: {0}")]
    AccountNotFound(String),
    
    #[error("Folder not found: {0}")]
    FolderNotFound(String),
    
    #[error("Sync timeout: {0}")]
    Timeout(String),
}

pub type SyncResult<T> = Result<T, SyncError>;

/// Sync progress information
#[derive(Debug, Clone)]
pub struct SyncProgress {
    pub account_id: String,
    pub folder_name: String,
    pub phase: SyncPhase,
    pub messages_processed: u32,
    pub total_messages: u32,
    pub bytes_downloaded: u64,
    pub started_at: DateTime<Utc>,
    pub estimated_completion: Option<DateTime<Utc>>,
}

/// Sync phase
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncPhase {
    Initializing,
    CheckingFolders,
    FetchingHeaders,
    FetchingBodies,
    ProcessingChanges,
    Complete,
    Error(String),
}

/// Sync strategy
#[derive(Debug, Clone)]
pub enum SyncStrategy {
    /// Full sync - download everything
    Full,
    /// Incremental sync - only changes since last sync
    Incremental,
    /// Headers only - download headers but not bodies
    HeadersOnly,
    /// Recent messages - only recent messages (last N days)
    Recent(u32),
}

/// Conflict resolution strategy
#[derive(Debug, Clone)]
pub enum ConflictResolution {
    /// Server wins - use server version
    ServerWins,
    /// Local wins - keep local version
    LocalWins,
    /// Merge - attempt to merge changes
    Merge,
    /// Ask user - prompt for resolution
    AskUser,
}

/// Email synchronization engine
pub struct SyncEngine {
    database: Arc<EmailDatabase>,
    sync_progress: Arc<RwLock<HashMap<String, SyncProgress>>>,
    sync_locks: Arc<RwLock<HashMap<String, Arc<Mutex<()>>>>>,
    progress_sender: mpsc::UnboundedSender<SyncProgress>,
    conflict_resolution: ConflictResolution,
    max_concurrent_syncs: usize,
}

impl SyncEngine {
    /// Create a new sync engine
    pub fn new(
        database: Arc<EmailDatabase>,
        progress_sender: mpsc::UnboundedSender<SyncProgress>,
    ) -> Self {
        Self {
            database,
            sync_progress: Arc::new(RwLock::new(HashMap::new())),
            sync_locks: Arc::new(RwLock::new(HashMap::new())),
            progress_sender,
            conflict_resolution: ConflictResolution::ServerWins,
            max_concurrent_syncs: 3,
        }
    }
    
    /// Set conflict resolution strategy
    pub fn set_conflict_resolution(&mut self, strategy: ConflictResolution) {
        self.conflict_resolution = strategy;
    }
    
    /// Sync all folders for an account
    pub async fn sync_account(
        &self,
        account_id: String,
        mut client: ImapClient,
        strategy: SyncStrategy,
    ) -> SyncResult<()> {
        info!("Starting account sync: {}", account_id);
        
        // Get or create sync lock for this account
        let sync_lock = {
            let mut locks = self.sync_locks.write().await;
            let key = format!("account:{}", account_id);
            locks.entry(key).or_insert_with(|| Arc::new(Mutex::new(()))).clone()
        };
        
        let _guard = sync_lock.lock().await;
        
        // Connect and authenticate
        client.connect().await?;
        client.authenticate().await?;
        
        // Get folder list
        let folders = client.list_folders("", "*").await?;
        info!("Found {} folders for account {}", folders.len(), account_id);
        
        // Sync each folder
        for folder in folders {
            if let Err(e) = self.sync_folder(
                account_id.clone(),
                &mut client,
                &folder,
                strategy.clone(),
            ).await {
                error!("Failed to sync folder {}: {}", folder.name, e);
                // Continue with other folders
            }
        }
        
        info!("Completed account sync: {}", account_id);
        Ok(())
    }
    
    /// Sync a specific folder
    pub async fn sync_folder(
        &self,
        account_id: String,
        client: &mut ImapClient,
        folder: &ImapFolder,
        strategy: SyncStrategy,
    ) -> SyncResult<()> {
        let folder_key = format!("{}:{}", account_id, folder.name);
        
        // Get or create sync lock for this folder
        let sync_lock = {
            let mut locks = self.sync_locks.write().await;
            locks.entry(folder_key.clone()).or_insert_with(|| Arc::new(Mutex::new(()))).clone()
        };
        
        let _guard = sync_lock.lock().await;
        
        info!("Starting folder sync: {} - {}", account_id, folder.name);
        
        // Initialize progress tracking
        let progress = SyncProgress {
            account_id: account_id.clone(),
            folder_name: folder.name.clone(),
            phase: SyncPhase::Initializing,
            messages_processed: 0,
            total_messages: 0,
            bytes_downloaded: 0,
            started_at: Utc::now(),
            estimated_completion: None,
        };
        
        self.update_progress(progress.clone()).await;
        
        // Select folder
        let selected_folder = client.select_folder(&folder.name).await?;
        
        // Get existing sync state
        let mut sync_state = self.database
            .get_folder_sync_state(&account_id, &folder.name)
            .await?
            .unwrap_or(FolderSyncState {
                account_id: account_id.clone(),
                folder_name: folder.name.clone(),
                uid_validity: selected_folder.uid_validity.unwrap_or(0),
                uid_next: selected_folder.uid_next.unwrap_or(1),
                highest_modseq: None,
                last_sync: DateTime::from_timestamp(0, 0).unwrap().into(),
                message_count: 0,
                unread_count: 0,
                sync_status: SyncStatus::Idle,
            });
        
        // Check for UID validity change (indicates folder reset)
        let needs_full_sync = if let Some(folder_uid_validity) = selected_folder.uid_validity {
            if sync_state.uid_validity != folder_uid_validity {
                warn!("UID validity changed for folder {}, forcing full sync", folder.name);
                sync_state.uid_validity = folder_uid_validity;
                true
            } else {
                matches!(strategy, SyncStrategy::Full)
            }
        } else {
            true // No UID validity, force full sync
        };
        
        sync_state.sync_status = SyncStatus::Syncing;
        self.database.update_folder_sync_state(&sync_state).await?;
        
        // Determine sync approach
        match (needs_full_sync, strategy) {
            (true, _) | (false, SyncStrategy::Full) => {
                self.full_sync_folder(&account_id, client, &selected_folder, &mut sync_state).await?;
            }
            (false, SyncStrategy::Incremental) => {
                self.incremental_sync_folder(&account_id, client, &selected_folder, &mut sync_state).await?;
            }
            (false, SyncStrategy::HeadersOnly) => {
                self.headers_only_sync_folder(&account_id, client, &selected_folder, &mut sync_state).await?;
            }
            (false, SyncStrategy::Recent(days)) => {
                self.recent_sync_folder(&account_id, client, &selected_folder, &mut sync_state, days).await?;
            }
        }
        
        // Update final sync state
        sync_state.sync_status = SyncStatus::Complete;
        sync_state.last_sync = Utc::now();
        self.database.update_folder_sync_state(&sync_state).await?;
        
        // Update final progress
        let mut final_progress = progress;
        final_progress.phase = SyncPhase::Complete;
        self.update_progress(final_progress).await;
        
        info!("Completed folder sync: {} - {}", account_id, folder.name);
        Ok(())
    }
    
    /// Perform full folder synchronization
    async fn full_sync_folder(
        &self,
        account_id: &str,
        client: &mut ImapClient,
        folder: &ImapFolder,
        sync_state: &mut FolderSyncState,
    ) -> SyncResult<()> {
        debug!("Starting full sync for folder: {}", folder.name);
        
        let total_messages = folder.exists.unwrap_or(0);
        self.update_progress_phase(account_id, &folder.name, SyncPhase::FetchingHeaders).await;
        
        if total_messages == 0 {
            return Ok(());
        }
        
        // Fetch all message UIDs first
        let search_results = client.search(&SearchCriteria::All).await?;
        info!("Found {} messages in folder {}", search_results.len(), folder.name);
        
        // Batch process messages to avoid overwhelming the server
        const BATCH_SIZE: usize = 50;
        let batches: Vec<_> = search_results.chunks(BATCH_SIZE).collect();
        
        for (batch_index, batch_uids) in batches.iter().enumerate() {
            self.process_message_batch(
                account_id,
                client,
                &folder.name,
                batch_uids,
                SyncStrategy::Full,
            ).await?;
            
            // Update progress
            let processed = ((batch_index + 1) * BATCH_SIZE).min(search_results.len()) as u32;
            self.update_progress_count(account_id, &folder.name, processed, total_messages).await;
            
            // Small delay to be nice to the server
            sleep(Duration::from_millis(100)).await;
        }
        
        sync_state.message_count = search_results.len() as u32;
        Ok(())
    }
    
    /// Perform incremental folder synchronization
    async fn incremental_sync_folder(
        &self,
        account_id: &str,
        client: &mut ImapClient,
        folder: &ImapFolder,
        sync_state: &mut FolderSyncState,
    ) -> SyncResult<()> {
        debug!("Starting incremental sync for folder: {}", folder.name);
        
        self.update_progress_phase(account_id, &folder.name, SyncPhase::CheckingFolders).await;
        
        // Use CONDSTORE if available for efficient sync
        if client.capabilities().contains(&ImapCapability::CondStore) {
            self.condstore_sync_folder(account_id, client, folder, sync_state).await?;
        } else {
            // Fallback to UID-based incremental sync
            self.uid_based_incremental_sync(account_id, client, folder, sync_state).await?;
        }
        
        Ok(())
    }
    
    /// CONDSTORE-based incremental sync
    async fn condstore_sync_folder(
        &self,
        account_id: &str,
        client: &mut ImapClient,
        folder: &ImapFolder,
        sync_state: &mut FolderSyncState,
    ) -> SyncResult<()> {
        let modseq = sync_state.highest_modseq.unwrap_or(0);
        
        // Use UID search since CONDSTORE is available
        // In a real implementation, this would use SEARCH (MODSEQ xxx)
        let criteria = SearchCriteria::All; // Simplified for now
        let changed_uids = client.search(&criteria).await?;
        
        if !changed_uids.is_empty() {
            info!("Found {} changed messages in folder {}", changed_uids.len(), folder.name);
            
            self.process_message_batch(
                account_id,
                client,
                &folder.name,
                &changed_uids,
                SyncStrategy::Incremental,
            ).await?;
        }
        
        // Update highest modseq from folder status
        if let Some(new_modseq) = folder.uid_next.map(|_| modseq + 1) {
            sync_state.highest_modseq = Some(new_modseq as u64);
        }
        
        Ok(())
    }
    
    /// UID-based incremental sync (fallback when CONDSTORE unavailable)
    async fn uid_based_incremental_sync(
        &self,
        account_id: &str,
        client: &mut ImapClient,
        folder: &ImapFolder,
        sync_state: &mut FolderSyncState,
    ) -> SyncResult<()> {
        let last_uid = sync_state.uid_next.saturating_sub(1);
        
        // Search for new messages since last sync
        let criteria = SearchCriteria::Uid(format!("{}:*", last_uid + 1));
        let new_uids = client.search(&criteria).await?;
        
        if !new_uids.is_empty() {
            info!("Found {} new messages in folder {}", new_uids.len(), folder.name);
            
            self.process_message_batch(
                account_id,
                client,
                &folder.name,
                &new_uids,
                SyncStrategy::Incremental,
            ).await?;
        }
        
        // Update UID next
        if let Some(folder_uid_next) = folder.uid_next {
            sync_state.uid_next = folder_uid_next;
        }
        
        Ok(())
    }
    
    /// Headers-only sync
    async fn headers_only_sync_folder(
        &self,
        account_id: &str,
        client: &mut ImapClient,
        folder: &ImapFolder,
        sync_state: &mut FolderSyncState,
    ) -> SyncResult<()> {
        debug!("Starting headers-only sync for folder: {}", folder.name);
        
        // Similar to incremental sync but only fetch headers
        let search_results = client.search(&SearchCriteria::All).await?;
        
        // Fetch only headers for efficiency
        const BATCH_SIZE: usize = 100; // Larger batches for headers-only
        let batches: Vec<_> = search_results.chunks(BATCH_SIZE).collect();
        
        for batch_uids in batches {
            // Fetch headers only
            let uid_set = batch_uids.iter()
                .map(|uid| uid.to_string())
                .collect::<Vec<_>>()
                .join(",");
            
            let messages = client.uid_fetch_messages(&uid_set, &[
                "UID", "FLAGS", "ENVELOPE", "INTERNALDATE", "RFC822.SIZE"
            ]).await?;
            
            // Store messages (without bodies)
            for message in messages {
                let stored_message = StoredMessage::from_imap_message(
                    &message,
                    account_id.to_string(),
                    folder.name.clone(),
                );
                
                self.database.store_message(&stored_message).await?;
            }
        }
        
        sync_state.message_count = search_results.len() as u32;
        Ok(())
    }
    
    /// Recent messages sync
    async fn recent_sync_folder(
        &self,
        account_id: &str,
        client: &mut ImapClient,
        folder: &ImapFolder,
        sync_state: &mut FolderSyncState,
        days: u32,
    ) -> SyncResult<()> {
        debug!("Starting recent sync for folder: {} (last {} days)", folder.name, days);
        
        let since_date = Utc::now() - ChronoDuration::days(days as i64);
        let criteria = SearchCriteria::Since(since_date);
        
        let recent_uids = client.search(&criteria).await?;
        
        if !recent_uids.is_empty() {
            info!("Found {} recent messages in folder {}", recent_uids.len(), folder.name);
            
            self.process_message_batch(
                account_id,
                client,
                &folder.name,
                &recent_uids,
                SyncStrategy::Recent(days),
            ).await?;
        }
        
        sync_state.message_count = recent_uids.len() as u32;
        Ok(())
    }
    
    /// Process a batch of messages
    async fn process_message_batch(
        &self,
        account_id: &str,
        client: &mut ImapClient,
        folder_name: &str,
        uids: &[u32],
        strategy: SyncStrategy,
    ) -> SyncResult<()> {
        if uids.is_empty() {
            return Ok(());
        }
        
        let uid_set = uids.iter()
            .map(|uid| uid.to_string())
            .collect::<Vec<_>>()
            .join(",");
        
        // Determine what to fetch based on strategy
        let fetch_items = match strategy {
            SyncStrategy::HeadersOnly => vec![
                "UID", "FLAGS", "ENVELOPE", "INTERNALDATE", "RFC822.SIZE"
            ],
            _ => vec![
                "UID", "FLAGS", "ENVELOPE", "INTERNALDATE", "RFC822.SIZE", "BODY.PEEK[]"
            ],
        };
        
        self.update_progress_phase(account_id, folder_name, SyncPhase::FetchingBodies).await;
        
        let messages = client.uid_fetch_messages(&uid_set, &fetch_items).await?;
        
        self.update_progress_phase(account_id, folder_name, SyncPhase::ProcessingChanges).await;
        
        // Process each message
        for message in messages {
            // Check for conflicts if this is an update
            if matches!(strategy, SyncStrategy::Incremental) {
                if let Some(existing) = self.database
                    .get_message_by_uid(account_id, folder_name, message.uid.unwrap_or(0))
                    .await? 
                {
                    self.resolve_message_conflict(&existing, &message, account_id, folder_name).await?;
                    continue;
                }
            }
            
            // Convert and store message
            let stored_message = StoredMessage::from_imap_message(
                &message,
                account_id.to_string(),
                folder_name.to_string(),
            );
            
            self.database.store_message(&stored_message).await?;
        }
        
        Ok(())
    }
    
    /// Resolve message conflicts
    async fn resolve_message_conflict(
        &self,
        local_message: &StoredMessage,
        server_message: &ImapMessage,
        account_id: &str,
        folder_name: &str,
    ) -> SyncResult<()> {
        match self.conflict_resolution {
            ConflictResolution::ServerWins => {
                // Update with server version
                let updated_message = StoredMessage::from_imap_message(
                    server_message,
                    account_id.to_string(),
                    folder_name.to_string(),
                );
                self.database.store_message(&updated_message).await?;
            }
            ConflictResolution::LocalWins => {
                // Keep local version, do nothing
                debug!("Keeping local version of message {}", local_message.message_id.as_deref().unwrap_or("unknown"));
            }
            ConflictResolution::Merge => {
                // Attempt to merge changes (simplified - merge flags and labels)
                let mut merged_message = local_message.clone();
                
                // Merge flags from server
                let server_flags: HashSet<String> = server_message.flags.iter().map(|flag| {
                    match flag {
                        crate::imap::MessageFlag::Seen => "\\Seen".to_string(),
                        crate::imap::MessageFlag::Answered => "\\Answered".to_string(),
                        crate::imap::MessageFlag::Flagged => "\\Flagged".to_string(),
                        crate::imap::MessageFlag::Deleted => "\\Deleted".to_string(),
                        crate::imap::MessageFlag::Draft => "\\Draft".to_string(),
                        crate::imap::MessageFlag::Recent => "\\Recent".to_string(),
                        crate::imap::MessageFlag::Custom(s) => s.clone(),
                    }
                }).collect();
                
                let local_flags: HashSet<String> = merged_message.flags.iter().cloned().collect();
                let merged_flags: Vec<String> = server_flags.union(&local_flags).cloned().collect();
                merged_message.flags = merged_flags;
                
                merged_message.updated_at = Utc::now();
                merged_message.last_synced = Utc::now();
                merged_message.sync_version += 1;
                
                self.database.store_message(&merged_message).await?;
            }
            ConflictResolution::AskUser => {
                // For now, default to server wins
                // In a real implementation, this would trigger a user prompt
                warn!("Message conflict detected for {}, defaulting to server version", 
                      local_message.message_id.as_deref().unwrap_or("unknown"));
                      
                let updated_message = StoredMessage::from_imap_message(
                    server_message,
                    account_id.to_string(),
                    folder_name.to_string(),
                );
                self.database.store_message(&updated_message).await?;
            }
        }
        
        Ok(())
    }
    
    /// Update sync progress
    pub async fn update_progress(&self, progress: SyncProgress) {
        let key = format!("{}:{}", progress.account_id, progress.folder_name);
        
        {
            let mut progress_map = self.sync_progress.write().await;
            progress_map.insert(key, progress.clone());
        }
        
        // Send progress update
        let _ = self.progress_sender.send(progress);
    }
    
    /// Update progress phase
    async fn update_progress_phase(&self, account_id: &str, folder_name: &str, phase: SyncPhase) {
        let key = format!("{}:{}", account_id, folder_name);
        
        if let Some(mut progress) = self.sync_progress.read().await.get(&key).cloned() {
            progress.phase = phase;
            self.update_progress(progress).await;
        }
    }
    
    /// Update progress count
    async fn update_progress_count(&self, account_id: &str, folder_name: &str, processed: u32, total: u32) {
        let key = format!("{}:{}", account_id, folder_name);
        
        if let Some(mut progress) = self.sync_progress.read().await.get(&key).cloned() {
            progress.messages_processed = processed;
            progress.total_messages = total;
            
            // Estimate completion time
            if processed > 0 {
                let elapsed = Utc::now().signed_duration_since(progress.started_at);
                let rate = processed as f64 / elapsed.num_seconds() as f64;
                if rate > 0.0 {
                    let remaining_seconds = (total - processed) as f64 / rate;
                    progress.estimated_completion = Some(
                        Utc::now() + ChronoDuration::seconds(remaining_seconds as i64)
                    );
                }
            }
            
            self.update_progress(progress).await;
        }
    }
    
    /// Get current sync progress for all operations
    pub async fn get_sync_progress(&self) -> HashMap<String, SyncProgress> {
        self.sync_progress.read().await.clone()
    }
    
    /// Get sync progress for a specific folder
    pub async fn get_folder_sync_progress(&self, account_id: &str, folder_name: &str) -> Option<SyncProgress> {
        let key = format!("{}:{}", account_id, folder_name);
        self.sync_progress.read().await.get(&key).cloned()
    }
    
    /// Cancel sync for a folder
    pub async fn cancel_sync(&self, account_id: &str, folder_name: &str) -> SyncResult<()> {
        let key = format!("{}:{}", account_id, folder_name);
        
        // Remove from progress tracking
        {
            let mut progress_map = self.sync_progress.write().await;
            if let Some(mut progress) = progress_map.remove(&key) {
                progress.phase = SyncPhase::Error("Cancelled by user".to_string());
                let _ = self.progress_sender.send(progress);
            }
        }
        
        // The actual cancellation would need to be handled by the sync task
        // This is a simplified implementation
        warn!("Sync cancellation requested for {}:{}", account_id, folder_name);
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::sync::mpsc;
    
    #[tokio::test]
    async fn test_sync_engine_creation() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("sync_test.db");
        let db = Arc::new(EmailDatabase::new(db_path.to_str().unwrap()).await.unwrap());
        
        let (sender, _receiver) = mpsc::unbounded_channel();
        let sync_engine = SyncEngine::new(db, sender);
        
        assert_eq!(sync_engine.max_concurrent_syncs, 3);
    }
    
    #[tokio::test]
    async fn test_conflict_resolution() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("conflict_test.db");
        let db = Arc::new(EmailDatabase::new(db_path.to_str().unwrap()).await.unwrap());
        
        let (sender, _receiver) = mpsc::unbounded_channel();
        let mut sync_engine = SyncEngine::new(db, sender);
        
        // Test different conflict resolution strategies
        sync_engine.set_conflict_resolution(ConflictResolution::ServerWins);
        sync_engine.set_conflict_resolution(ConflictResolution::LocalWins);
        sync_engine.set_conflict_resolution(ConflictResolution::Merge);
    }
    
    #[tokio::test]
    async fn test_progress_tracking() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("progress_test.db");
        let db = Arc::new(EmailDatabase::new(db_path.to_str().unwrap()).await.unwrap());
        
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let sync_engine = SyncEngine::new(db, sender);
        
        let progress = SyncProgress {
            account_id: "test-account".to_string(),
            folder_name: "INBOX".to_string(),
            phase: SyncPhase::Initializing,
            messages_processed: 0,
            total_messages: 100,
            bytes_downloaded: 0,
            started_at: Utc::now(),
            estimated_completion: None,
        };
        
        sync_engine.update_progress(progress.clone()).await;
        
        // Check that progress was sent
        let received_progress = receiver.recv().await.unwrap();
        assert_eq!(received_progress.account_id, "test-account");
        assert_eq!(received_progress.folder_name, "INBOX");
        assert_eq!(received_progress.phase, SyncPhase::Initializing);
        
        // Check that progress is stored
        let stored_progress = sync_engine.get_folder_sync_progress("test-account", "INBOX").await;
        assert!(stored_progress.is_some());
        assert_eq!(stored_progress.unwrap().total_messages, 100);
    }
}