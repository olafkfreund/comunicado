//! Core synchronization engine for cloud data sync

use super::{CloudSyncResult, SyncDataType, SyncMetadata};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Synchronization engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    pub auto_sync_enabled: bool,
    pub sync_interval_seconds: u64,
    pub batch_size: usize,
    pub max_retry_attempts: u32,
    pub retry_backoff_ms: u64,
    pub conflict_resolution_timeout_ms: u64,
    pub parallel_syncs: usize,
}

/// Synchronization status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SyncStatus {
    /// Not yet started
    Pending,
    /// Currently synchronizing
    InProgress,
    /// Successfully completed
    Success,
    /// Failed with error
    Failed,
    /// Disabled for this data type
    Disabled,
    /// Skipped (no changes detected)
    Skipped,
    /// Requires manual conflict resolution
    ConflictRequiresResolution,
}

/// Detailed synchronization result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub data_type: SyncDataType,
    pub status: SyncStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub items_synchronized: u32,
    pub bytes_transferred: u64,
    pub conflicts_resolved: u32,
    pub error_message: Option<String>,
    pub sync_metadata: Option<SyncMetadata>,
}

/// Synchronization statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct SyncStatistics {
    pub total_syncs: u64,
    pub successful_syncs: u64,
    pub failed_syncs: u64,
    pub total_bytes_transferred: u64,
    pub average_sync_time_ms: u64,
    pub last_sync_time: Option<DateTime<Utc>>,
    pub sync_frequency: HashMap<SyncDataType, u32>,
}

/// Core synchronization engine
pub struct SyncEngine {
    config: SyncConfig,
    sync_queue: RwLock<Vec<SyncTask>>,
    active_syncs: RwLock<HashMap<SyncDataType, SyncProgress>>,
    sync_history: RwLock<Vec<SyncResult>>,
    statistics: RwLock<SyncStatistics>,
    scheduler: SyncScheduler,
}

/// Synchronization task
#[derive(Debug, Clone)]
pub struct SyncTask {
    pub id: Uuid,
    pub data_type: SyncDataType,
    pub priority: SyncPriority,
    pub created_at: DateTime<Utc>,
    pub scheduled_at: DateTime<Utc>,
    pub retry_count: u32,
    pub operation: SyncOperation,
}

/// Synchronization priority levels
#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum SyncPriority {
    Low = 1,
    Normal = 5,
    High = 10,
    Critical = 20,
}

/// Synchronization operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncOperation {
    /// Upload local changes to cloud
    Upload { local_data: Vec<u8> },
    /// Download remote changes from cloud
    Download { remote_metadata: SyncMetadata },
    /// Bidirectional sync with conflict resolution
    Bidirectional,
    /// Full refresh (download all)
    FullRefresh,
}

/// Progress tracking for active syncs
#[derive(Debug, Clone)]
pub struct SyncProgress {
    pub task_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub current_phase: SyncPhase,
    pub progress_percentage: f32,
    pub items_processed: u32,
    pub total_items: u32,
    pub bytes_transferred: u64,
    pub estimated_completion: Option<DateTime<Utc>>,
}

/// Phases of synchronization process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncPhase {
    Initializing,
    CheckingLocalChanges,
    CheckingRemoteChanges,
    ComparingVersions,
    ResolvingConflicts,
    Uploading,
    Downloading,
    Finalizing,
    Complete,
    Error(String),
}

/// Synchronization scheduler
pub struct SyncScheduler {
    scheduled_tasks: RwLock<HashMap<SyncDataType, DateTime<Utc>>>,
    auto_sync_enabled: bool,
}

impl SyncEngine {
    pub fn new() -> CloudSyncResult<Self> {
        Ok(Self {
            config: SyncConfig::default(),
            sync_queue: RwLock::new(Vec::new()),
            active_syncs: RwLock::new(HashMap::new()),
            sync_history: RwLock::new(Vec::new()),
            statistics: RwLock::new(SyncStatistics::new()),
            scheduler: SyncScheduler::new(),
        })
    }

    /// Configure the sync engine
    pub fn configure(&mut self, config: SyncConfig) {
        self.config = config;
        self.scheduler.auto_sync_enabled = self.config.auto_sync_enabled;
    }

    /// Queue a synchronization task
    pub async fn queue_sync(
        &self,
        data_type: SyncDataType,
        operation: SyncOperation,
        priority: SyncPriority,
    ) -> CloudSyncResult<Uuid> {
        let task_id = Uuid::new_v4();
        let now = Utc::now();
        
        let task = SyncTask {
            id: task_id,
            data_type: data_type.clone(),
            priority,
            created_at: now,
            scheduled_at: now,
            retry_count: 0,
            operation,
        };

        {
            let mut queue = self.sync_queue.write().await;
            queue.push(task);
            
            // Sort by priority and scheduled time
            queue.sort_by(|a, b| {
                b.priority.partial_cmp(&a.priority)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.scheduled_at.cmp(&b.scheduled_at))
            });
        }

        Ok(task_id)
    }

    /// Process the sync queue
    pub async fn process_queue(&self) -> CloudSyncResult<Vec<SyncResult>> {
        let mut results = Vec::new();
        let max_parallel = self.config.parallel_syncs;
        
        // Get tasks to process
        let tasks_to_process = {
            let mut queue = self.sync_queue.write().await;
            let available_count = max_parallel.saturating_sub(self.active_syncs.read().await.len());
            let mut tasks = Vec::new();
            
            for _ in 0..available_count.min(queue.len()) {
                if let Some(task) = queue.drain(0..1).next() {
                    tasks.push(task);
                }
            }
            
            tasks
        };

        // Process tasks concurrently
        let mut task_handles = Vec::new();
        
        for task in tasks_to_process {
            let task_handle = self.process_sync_task(task);
            task_handles.push(task_handle);
        }

        // Wait for all tasks to complete
        for handle in task_handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => {
                    // Log error but continue processing other tasks
                    eprintln!("Sync task failed: {}", e);
                }
            }
        }

        // Update statistics
        self.update_statistics(&results).await;

        Ok(results)
    }

    /// Process a single sync task
    async fn process_sync_task(&self, mut task: SyncTask) -> CloudSyncResult<SyncResult> {
        let start_time = Utc::now();
        
        // Create progress tracker
        let progress = SyncProgress {
            task_id: task.id,
            started_at: start_time,
            current_phase: SyncPhase::Initializing,
            progress_percentage: 0.0,
            items_processed: 0,
            total_items: 1, // Will be updated based on operation
            bytes_transferred: 0,
            estimated_completion: None,
        };

        // Register active sync
        {
            let mut active = self.active_syncs.write().await;
            active.insert(task.data_type.clone(), progress);
        }

        let result = self.execute_sync_operation(&mut task).await;

        // Remove from active syncs
        {
            let mut active = self.active_syncs.write().await;
            active.remove(&task.data_type);
        }

        // Create result
        let sync_result = match result {
            Ok((status, metadata, items_synced, bytes_transferred, conflicts_resolved)) => {
                SyncResult {
                    data_type: task.data_type,
                    status,
                    started_at: start_time,
                    completed_at: Some(Utc::now()),
                    items_synchronized: items_synced,
                    bytes_transferred,
                    conflicts_resolved,
                    error_message: None,
                    sync_metadata: metadata,
                }
            }
            Err(e) => {
                // Handle retry logic
                if task.retry_count < self.config.max_retry_attempts {
                    task.retry_count += 1;
                    task.scheduled_at = Utc::now() + 
                        chrono::Duration::milliseconds(
                            (self.config.retry_backoff_ms * (1 << task.retry_count) as u64) as i64
                        );
                    
                    // Re-queue the task
                    let mut queue = self.sync_queue.write().await;
                    queue.push(task.clone());
                }

                SyncResult {
                    data_type: task.data_type,
                    status: SyncStatus::Failed,
                    started_at: start_time,
                    completed_at: Some(Utc::now()),
                    items_synchronized: 0,
                    bytes_transferred: 0,
                    conflicts_resolved: 0,
                    error_message: Some(e.to_string()),
                    sync_metadata: None,
                }
            }
        };

        // Add to history
        {
            let mut history = self.sync_history.write().await;
            history.push(sync_result.clone());
            
            // Keep only recent history (last 1000 results)
            if history.len() > 1000 {
                history.drain(0..100);
            }
        }

        Ok(sync_result)
    }

    /// Execute the actual sync operation
    async fn execute_sync_operation(
        &self,
        task: &mut SyncTask,
    ) -> CloudSyncResult<(SyncStatus, Option<SyncMetadata>, u32, u64, u32)> {
        self.update_progress(task.data_type.clone(), SyncPhase::CheckingLocalChanges, 10.0).await;

        match &task.operation {
            SyncOperation::Upload { local_data } => {
                self.update_progress(task.data_type.clone(), SyncPhase::Uploading, 50.0).await;
                
                // Simulate upload operation
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                let bytes_transferred = local_data.len() as u64;
                
                self.update_progress(task.data_type.clone(), SyncPhase::Complete, 100.0).await;
                
                Ok((SyncStatus::Success, None, 1, bytes_transferred, 0))
            }
            SyncOperation::Download { remote_metadata } => {
                self.update_progress(task.data_type.clone(), SyncPhase::Downloading, 50.0).await;
                
                // Simulate download operation
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                let bytes_transferred = remote_metadata.size_bytes;
                
                self.update_progress(task.data_type.clone(), SyncPhase::Complete, 100.0).await;
                
                Ok((SyncStatus::Success, Some(remote_metadata.clone()), 1, bytes_transferred, 0))
            }
            SyncOperation::Bidirectional => {
                self.update_progress(task.data_type.clone(), SyncPhase::CheckingRemoteChanges, 25.0).await;
                self.update_progress(task.data_type.clone(), SyncPhase::ComparingVersions, 40.0).await;
                
                // Check for conflicts
                let has_conflicts = self.detect_conflicts(task.data_type.clone()).await?;
                
                if has_conflicts {
                    self.update_progress(task.data_type.clone(), SyncPhase::ResolvingConflicts, 60.0).await;
                    
                    // Simulate conflict resolution
                    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                    
                    self.update_progress(task.data_type.clone(), SyncPhase::Complete, 100.0).await;
                    
                    Ok((SyncStatus::Success, None, 1, 1024, 1))
                } else {
                    self.update_progress(task.data_type.clone(), SyncPhase::Complete, 100.0).await;
                    
                    Ok((SyncStatus::Skipped, None, 0, 0, 0))
                }
            }
            SyncOperation::FullRefresh => {
                self.update_progress(task.data_type.clone(), SyncPhase::Downloading, 30.0).await;
                self.update_progress(task.data_type.clone(), SyncPhase::Finalizing, 80.0).await;
                
                // Simulate full refresh
                tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                
                self.update_progress(task.data_type.clone(), SyncPhase::Complete, 100.0).await;
                
                Ok((SyncStatus::Success, None, 5, 5120, 0))
            }
        }
    }

    /// Update sync progress
    async fn update_progress(
        &self,
        data_type: SyncDataType,
        phase: SyncPhase,
        progress: f32,
    ) {
        let mut active_syncs = self.active_syncs.write().await;
        if let Some(sync_progress) = active_syncs.get_mut(&data_type) {
            sync_progress.current_phase = phase;
            sync_progress.progress_percentage = progress;
            
            // Estimate completion time based on progress
            if progress > 0.0 && progress < 100.0 {
                let elapsed = Utc::now().signed_duration_since(sync_progress.started_at);
                let total_estimated = elapsed.num_milliseconds() as f32 * (100.0 / progress);
                let remaining_ms = total_estimated - elapsed.num_milliseconds() as f32;
                
                if remaining_ms > 0.0 {
                    sync_progress.estimated_completion = Some(
                        Utc::now() + chrono::Duration::milliseconds(remaining_ms as i64)
                    );
                }
            }
        }
    }

    /// Detect conflicts for a data type
    async fn detect_conflicts(&self, _data_type: SyncDataType) -> CloudSyncResult<bool> {
        // Simulate conflict detection
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        Ok(rand::random::<f32>() < 0.1) // 10% chance of conflict
    }

    /// Get current sync status for all data types
    pub async fn get_sync_status(&self) -> HashMap<SyncDataType, SyncStatus> {
        let mut status_map = HashMap::new();
        let active_syncs = self.active_syncs.read().await;
        
        for data_type in SyncDataType::all() {
            let status = if active_syncs.contains_key(&data_type) {
                SyncStatus::InProgress
            } else {
                // Get last status from history
                let history = self.sync_history.read().await;
                history.iter()
                    .filter(|result| result.data_type == data_type)
                    .last()
                    .map(|result| result.status.clone())
                    .unwrap_or(SyncStatus::Pending)
            };
            
            status_map.insert(data_type, status);
        }
        
        status_map
    }

    /// Get detailed progress for active syncs
    pub async fn get_active_progress(&self) -> HashMap<SyncDataType, SyncProgress> {
        self.active_syncs.read().await.clone()
    }

    /// Get sync history
    pub async fn get_sync_history(&self, limit: Option<usize>) -> Vec<SyncResult> {
        let history = self.sync_history.read().await;
        let limit = limit.unwrap_or(100);
        
        history.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get synchronization statistics
    pub async fn get_statistics(&self) -> SyncStatistics {
        (*self.statistics.read().await).clone()
    }

    /// Update statistics based on completed syncs
    async fn update_statistics(&self, results: &[SyncResult]) {
        let mut stats = self.statistics.write().await;
        
        for result in results {
            stats.total_syncs += 1;
            
            match result.status {
                SyncStatus::Success | SyncStatus::Skipped => {
                    stats.successful_syncs += 1;
                }
                SyncStatus::Failed => {
                    stats.failed_syncs += 1;
                }
                _ => {}
            }
            
            stats.total_bytes_transferred += result.bytes_transferred;
            
            if let Some(completed_at) = result.completed_at {
                let sync_time = completed_at.signed_duration_since(result.started_at);
                let sync_time_ms = sync_time.num_milliseconds().max(0) as u64;
                
                // Update average sync time
                if stats.total_syncs > 1 {
                    stats.average_sync_time_ms = 
                        (stats.average_sync_time_ms * (stats.total_syncs - 1) + sync_time_ms) / stats.total_syncs;
                } else {
                    stats.average_sync_time_ms = sync_time_ms;
                }
                
                stats.last_sync_time = Some(completed_at);
            }
            
            *stats.sync_frequency.entry(result.data_type.clone()).or_insert(0) += 1;
        }
    }

    /// Schedule automatic sync for a data type
    pub async fn schedule_auto_sync(&self, data_type: SyncDataType) -> CloudSyncResult<()> {
        if self.scheduler.auto_sync_enabled {
            let next_sync = Utc::now() + 
                chrono::Duration::seconds(self.config.sync_interval_seconds as i64);
            
            self.scheduler.schedule_sync(data_type.clone(), next_sync).await;
            
            // Queue the sync task
            self.queue_sync(
                data_type,
                SyncOperation::Bidirectional,
                SyncPriority::Normal,
            ).await?;
        }
        
        Ok(())
    }
}

impl SyncScheduler {
    fn new() -> Self {
        Self {
            scheduled_tasks: RwLock::new(HashMap::new()),
            auto_sync_enabled: false,
        }
    }

    async fn schedule_sync(&self, data_type: SyncDataType, scheduled_time: DateTime<Utc>) {
        let mut tasks = self.scheduled_tasks.write().await;
        tasks.insert(data_type, scheduled_time);
    }

    pub async fn get_next_scheduled_sync(&self, data_type: &SyncDataType) -> Option<DateTime<Utc>> {
        let tasks = self.scheduled_tasks.read().await;
        tasks.get(data_type).copied()
    }
}

impl SyncStatistics {
    fn new() -> Self {
        Self {
            total_syncs: 0,
            successful_syncs: 0,
            failed_syncs: 0,
            total_bytes_transferred: 0,
            average_sync_time_ms: 0,
            last_sync_time: None,
            sync_frequency: HashMap::new(),
        }
    }
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            auto_sync_enabled: true,
            sync_interval_seconds: 300, // 5 minutes
            batch_size: 10,
            max_retry_attempts: 3,
            retry_backoff_ms: 1000, // 1 second base
            conflict_resolution_timeout_ms: 30000, // 30 seconds
            parallel_syncs: 3,
        }
    }
}