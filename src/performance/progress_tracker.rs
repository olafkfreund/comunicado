//! Progress tracking and status reporting system
//!
//! This module provides real-time progress tracking for background operations,
//! allowing the UI to display meaningful status updates and progress bars
//! instead of freezing during long-running operations.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

/// Progress status for operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgressStatus {
    /// Operation is starting
    Starting,
    /// Operation is in progress
    InProgress,
    /// Operation completed successfully
    Completed,
    /// Operation failed
    Failed(String),
    /// Operation was cancelled
    Cancelled,
    /// Operation is paused
    Paused,
}

/// Progress update with detailed information
#[derive(Debug, Clone)]
pub struct ProgressUpdate {
    /// Unique operation ID
    pub operation_id: Uuid,
    /// Human-readable operation name
    pub operation_name: String,
    /// Current status
    pub status: ProgressStatus,
    /// Progress percentage (0-100)
    pub progress_percent: f64,
    /// Current step description
    pub current_step: String,
    /// Current item being processed
    pub current_item: Option<String>,
    /// Items processed so far
    pub items_processed: u64,
    /// Total items to process (if known)
    pub total_items: Option<u64>,
    /// Bytes processed (for file operations)
    pub bytes_processed: u64,
    /// Total bytes (if known)
    pub total_bytes: Option<u64>,
    /// Time when operation started
    pub started_at: Instant,
    /// Estimated completion time
    pub estimated_completion: Option<Instant>,
    /// Processing rate (items per second)
    pub processing_rate: Option<f64>,
    /// Additional context information
    pub context: HashMap<String, String>,
}

impl ProgressUpdate {
    /// Create a new progress update
    pub fn new(operation_id: Uuid, operation_name: String) -> Self {
        Self {
            operation_id,
            operation_name,
            status: ProgressStatus::Starting,
            progress_percent: 0.0,
            current_step: "Initializing...".to_string(),
            current_item: None,
            items_processed: 0,
            total_items: None,
            bytes_processed: 0,
            total_bytes: None,
            started_at: Instant::now(),
            estimated_completion: None,
            processing_rate: None,
            context: HashMap::new(),
        }
    }

    /// Update progress with new values
    pub fn update_progress(&mut self, processed: u64, total: Option<u64>) {
        self.items_processed = processed;
        self.total_items = total;
        
        if let Some(total) = total {
            if total > 0 {
                self.progress_percent = (processed as f64 / total as f64) * 100.0;
            }
        }

        self.update_rate_and_eta();
    }

    /// Update current step
    pub fn update_step(&mut self, step: String) {
        self.current_step = step;
    }

    /// Update current item being processed
    pub fn update_current_item(&mut self, item: String) {
        self.current_item = Some(item);
    }

    /// Update bytes processed
    pub fn update_bytes(&mut self, processed: u64, total: Option<u64>) {
        self.bytes_processed = processed;
        self.total_bytes = total;
    }

    /// Set operation as completed
    pub fn set_completed(&mut self) {
        self.status = ProgressStatus::Completed;
        self.progress_percent = 100.0;
        self.current_step = "Completed".to_string();
        self.estimated_completion = Some(Instant::now());
    }

    /// Set operation as failed
    pub fn set_failed(&mut self, error: String) {
        self.status = ProgressStatus::Failed(error.clone());
        self.current_step = format!("Failed: {}", error);
        self.estimated_completion = Some(Instant::now());
    }

    /// Set operation as cancelled
    pub fn set_cancelled(&mut self) {
        self.status = ProgressStatus::Cancelled;
        self.current_step = "Cancelled".to_string();
        self.estimated_completion = Some(Instant::now());
    }

    /// Add context information
    pub fn add_context(&mut self, key: String, value: String) {
        self.context.insert(key, value);
    }

    /// Calculate processing rate and estimated completion time
    fn update_rate_and_eta(&mut self) {
        let elapsed = self.started_at.elapsed();
        let elapsed_secs = elapsed.as_secs_f64();
        
        if elapsed_secs > 0.0 && self.items_processed > 0 {
            self.processing_rate = Some(self.items_processed as f64 / elapsed_secs);
            
            if let (Some(total), Some(rate)) = (self.total_items, self.processing_rate) {
                if rate > 0.0 && self.items_processed < total {
                    let remaining_items = total - self.items_processed;
                    let remaining_secs = remaining_items as f64 / rate;
                    self.estimated_completion = Some(Instant::now() + Duration::from_secs_f64(remaining_secs));
                }
            }
        }
    }

    /// Get elapsed time
    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// Get estimated remaining time
    pub fn estimated_remaining(&self) -> Option<Duration> {
        self.estimated_completion.map(|eta| {
            let now = Instant::now();
            if eta > now {
                eta - now
            } else {
                Duration::from_secs(0)
            }
        })
    }

    /// Format progress as human-readable string
    pub fn format_progress(&self) -> String {
        match (&self.total_items, self.items_processed) {
            (Some(total), processed) => {
                format!("{}/{} ({:.1}%)", processed, total, self.progress_percent)
            }
            (None, processed) => {
                format!("{} items", processed)
            }
        }
    }

    /// Format processing rate as human-readable string
    pub fn format_rate(&self) -> Option<String> {
        self.processing_rate.map(|rate| {
            if rate >= 1.0 {
                format!("{:.1} items/sec", rate)
            } else {
                format!("{:.1} sec/item", 1.0 / rate)
            }
        })
    }

    /// Format estimated time remaining
    pub fn format_eta(&self) -> Option<String> {
        self.estimated_remaining().map(|remaining| {
            let secs = remaining.as_secs();
            if secs < 60 {
                format!("{}s remaining", secs)
            } else if secs < 3600 {
                format!("{}m {}s remaining", secs / 60, secs % 60)  
            } else {
                format!("{}h {}m remaining", secs / 3600, (secs % 3600) / 60)
            }
        })
    }
}

/// Progress tracker for managing multiple concurrent operations
pub struct ProgressTracker {
    /// Active progress updates
    active_operations: Arc<RwLock<HashMap<Uuid, ProgressUpdate>>>,
    /// Broadcast sender for progress updates
    progress_sender: broadcast::Sender<ProgressUpdate>,
    /// Operation history (limited size)
    operation_history: Arc<RwLock<Vec<ProgressUpdate>>>,
    /// Maximum history size
    max_history_size: usize,
}

impl ProgressTracker {
    /// Create a new progress tracker
    pub fn new() -> Self {
        let (progress_sender, _) = broadcast::channel(1000);
        
        Self {
            active_operations: Arc::new(RwLock::new(HashMap::new())),
            progress_sender,
            operation_history: Arc::new(RwLock::new(Vec::new())),
            max_history_size: 100,
        }
    }

    /// Start tracking a new operation
    pub async fn start_operation(&self, operation_id: Uuid, operation_name: String) -> ProgressUpdate {
        let mut progress = ProgressUpdate::new(operation_id, operation_name);
        progress.status = ProgressStatus::Starting;
        
        {
            let mut operations = self.active_operations.write().await;
            operations.insert(operation_id, progress.clone());
        }

        // Broadcast initial progress
        let _ = self.progress_sender.send(progress.clone());
        
        progress
    }

    /// Update progress for an operation
    pub async fn update_progress(&self, mut progress: ProgressUpdate) {
        progress.status = ProgressStatus::InProgress;
        
        {
            let mut operations = self.active_operations.write().await;
            operations.insert(progress.operation_id, progress.clone());
        }

        // Broadcast progress update
        let _ = self.progress_sender.send(progress);
    }

    /// Complete an operation
    pub async fn complete_operation(&self, operation_id: Uuid) {
        if let Some(mut progress) = self.get_operation(operation_id).await {
            progress.set_completed();
            
            {
                let mut operations = self.active_operations.write().await;
                operations.remove(&operation_id);
            }

            // Add to history
            self.add_to_history(progress.clone()).await;
            
            // Broadcast completion
            let _ = self.progress_sender.send(progress);
        }
    }

    /// Fail an operation
    pub async fn fail_operation(&self, operation_id: Uuid, error: String) {
        if let Some(mut progress) = self.get_operation(operation_id).await {
            progress.set_failed(error);
            
            {
                let mut operations = self.active_operations.write().await;
                operations.remove(&operation_id);
            }

            // Add to history
            self.add_to_history(progress.clone()).await;
            
            // Broadcast failure
            let _ = self.progress_sender.send(progress);
        }
    }

    /// Cancel an operation
    pub async fn cancel_operation(&self, operation_id: Uuid) {
        if let Some(mut progress) = self.get_operation(operation_id).await {
            progress.set_cancelled();
            
            {
                let mut operations = self.active_operations.write().await;
                operations.remove(&operation_id);
            }

            // Add to history
            self.add_to_history(progress.clone()).await;
            
            // Broadcast cancellation
            let _ = self.progress_sender.send(progress);
        }
    }

    /// Get current progress for an operation
    pub async fn get_operation(&self, operation_id: Uuid) -> Option<ProgressUpdate> {
        let operations = self.active_operations.read().await;
        operations.get(&operation_id).cloned()
    }

    /// Get all active operations
    pub async fn get_active_operations(&self) -> Vec<ProgressUpdate> {
        let operations = self.active_operations.read().await;
        operations.values().cloned().collect()
    }

    /// Get operation history
    pub async fn get_operation_history(&self) -> Vec<ProgressUpdate> {
        let history = self.operation_history.read().await;
        history.clone()
    }

    /// Subscribe to progress updates
    pub fn subscribe(&self) -> broadcast::Receiver<ProgressUpdate> {
        self.progress_sender.subscribe()
    }

    /// Get number of active operations
    pub async fn active_operation_count(&self) -> usize {
        let operations = self.active_operations.read().await;
        operations.len()
    }

    /// Clear all completed operations from history
    pub async fn clear_history(&self) {
        let mut history = self.operation_history.write().await;
        history.clear();
    }

    /// Add operation to history with size limiting
    async fn add_to_history(&self, progress: ProgressUpdate) {
        let mut history = self.operation_history.write().await;
        
        history.push(progress);
        
        // Limit history size
        if history.len() > self.max_history_size {
            history.remove(0);
        }
    }

    /// Clean up stale operations (running for too long without updates)
    pub async fn cleanup_stale_operations(&self, max_age: Duration) {
        let mut operations = self.active_operations.write().await;
        let now = Instant::now();
        
        let stale_operations: Vec<_> = operations
            .iter()
            .filter(|(_, progress)| now.duration_since(progress.started_at) > max_age)
            .map(|(id, _)| *id)
            .collect();

        for operation_id in stale_operations {
            if let Some(mut progress) = operations.remove(&operation_id) {
                progress.set_failed("Operation timed out".to_string());
                
                // Add to history
                {
                    let mut history = self.operation_history.write().await;
                    history.push(progress.clone());
                    
                    if history.len() > self.max_history_size {
                        history.remove(0);
                    }
                }
                
                // Broadcast timeout
                let _ = self.progress_sender.send(progress);
            }
        }
    }
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper struct for creating common progress updates
pub struct ProgressBuilder {
    operation_id: Uuid,
    operation_name: String,
    total_items: Option<u64>,
}

impl ProgressBuilder {
    pub fn new(operation_name: String) -> Self {
        Self {
            operation_id: Uuid::new_v4(),
            operation_name,
            total_items: None,
        }
    }

    pub fn with_id(mut self, operation_id: Uuid) -> Self {
        self.operation_id = operation_id;
        self
    }

    pub fn with_total_items(mut self, total: u64) -> Self {
        self.total_items = Some(total);
        self
    }

    pub fn build(self) -> ProgressUpdate {
        let mut progress = ProgressUpdate::new(self.operation_id, self.operation_name);
        progress.total_items = self.total_items;
        progress
    }
}

/// Convenience functions for common operations
impl ProgressTracker {
    /// Start folder sync operation
    pub async fn start_folder_sync(&self, account_id: &str, folder_name: &str) -> ProgressUpdate {
        let operation_name = format!("Syncing {} - {}", account_id, folder_name);
        let operation_id = Uuid::new_v4();
        
        let mut progress = self.start_operation(operation_id, operation_name).await;
        progress.add_context("account_id".to_string(), account_id.to_string());
        progress.add_context("folder_name".to_string(), folder_name.to_string());
        progress.add_context("operation_type".to_string(), "folder_sync".to_string());
        
        self.update_progress(progress.clone()).await;
        progress
    }

    /// Start message search operation
    pub async fn start_search(&self, query: &str, folder_count: usize) -> ProgressUpdate {
        let operation_name = format!("Searching: {}", query);
        let operation_id = Uuid::new_v4();
        
        let mut progress = self.start_operation(operation_id, operation_name).await;
        progress.total_items = Some(folder_count as u64);
        progress.add_context("query".to_string(), query.to_string());
        progress.add_context("operation_type".to_string(), "search".to_string());
        
        self.update_progress(progress.clone()).await;
        progress
    }

    /// Start cache preload operation
    pub async fn start_cache_preload(&self, folder_name: &str, message_count: usize) -> ProgressUpdate {
        let operation_name = format!("Preloading cache: {}", folder_name);
        let operation_id = Uuid::new_v4();
        
        let mut progress = self.start_operation(operation_id, operation_name).await;
        progress.total_items = Some(message_count as u64);
        progress.add_context("folder_name".to_string(), folder_name.to_string());
        progress.add_context("operation_type".to_string(), "cache_preload".to_string());
        
        self.update_progress(progress.clone()).await;
        progress
    }
}