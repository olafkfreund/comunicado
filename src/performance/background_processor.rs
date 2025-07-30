//! Background task processor for non-blocking operations
//!
//! This module provides an async background processing system that handles
//! email synchronization, folder refresh, and other long-running operations
//! without blocking the UI thread.

use crate::email::sync_engine::{SyncProgress, SyncStrategy};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::task::JoinHandle;
use uuid::Uuid;

/// Priority levels for background tasks
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TaskPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Background task definition
#[derive(Debug, Clone)]
pub struct BackgroundTask {
    pub id: Uuid,
    pub name: String,
    pub priority: TaskPriority,
    pub account_id: String,
    pub folder_name: Option<String>,
    pub task_type: BackgroundTaskType,
    pub created_at: Instant,
    pub estimated_duration: Option<Duration>,
}

/// Types of background tasks
#[derive(Debug, Clone)]
pub enum BackgroundTaskType {
    /// Sync all folders for an account
    AccountSync {
        strategy: SyncStrategy,
    },
    /// Sync a specific folder
    FolderSync {
        folder_name: String,
        strategy: SyncStrategy,
    },
    /// Refresh folder metadata
    FolderRefresh {
        folder_name: String,
    },
    /// Search across folders
    Search {
        query: String,
        folders: Vec<String>,
    },
    /// Index messages for search
    Indexing {
        folder_name: String,
    },
    /// Cache preloading
    CachePreload {
        folder_name: String,
        message_count: usize,
    },
}

/// Task execution status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskStatus {
    Queued,
    Running,
    Completed,
    Failed(String),
    Cancelled,
}

/// Background task result
#[derive(Debug, Clone)]
pub struct TaskResult {
    pub task_id: Uuid,
    pub status: TaskStatus,
    pub started_at: Instant,
    pub completed_at: Option<Instant>,
    pub error: Option<String>,
    pub result_data: Option<TaskResultData>,
}

/// Task result data for different task types
#[derive(Debug, Clone)]
pub enum TaskResultData {
    SyncProgress(SyncProgress),
    MessageCount(usize),
    SearchResults(Vec<String>), // Message IDs
    CacheStats(usize), // Number of cached items
}

/// Background task processor
pub struct BackgroundProcessor {
    /// Task queue organized by priority
    task_queue: Arc<RwLock<HashMap<TaskPriority, Vec<BackgroundTask>>>>,
    /// Currently running tasks
    running_tasks: Arc<RwLock<HashMap<Uuid, JoinHandle<TaskResult>>>>,
    /// Task results cache
    task_results: Arc<RwLock<HashMap<Uuid, TaskResult>>>,
    /// Progress sender for UI updates
    progress_sender: Arc<mpsc::UnboundedSender<SyncProgress>>,
    /// Task completion sender
    completion_sender: Arc<mpsc::UnboundedSender<TaskResult>>,
    /// Processor settings
    settings: ProcessorSettings,
    /// Shutdown signal
    shutdown_tx: Arc<Mutex<Option<mpsc::Sender<()>>>>,
}

/// Background processor settings
#[derive(Debug, Clone)]
pub struct ProcessorSettings {
    /// Maximum number of concurrent tasks
    pub max_concurrent_tasks: usize,
    /// Task timeout duration
    pub task_timeout: Duration,
    /// Queue size limit
    pub max_queue_size: usize,
    /// Result cache size
    pub result_cache_size: usize,
    /// Processing interval
    pub processing_interval: Duration,
}

impl Default for ProcessorSettings {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 3,
            task_timeout: Duration::from_secs(300), // 5 minutes
            max_queue_size: 100,
            result_cache_size: 50,
            processing_interval: Duration::from_millis(100),
        }
    }
}

impl BackgroundProcessor {
    /// Create a new background processor
    pub fn new(
        progress_sender: mpsc::UnboundedSender<SyncProgress>,
        completion_sender: mpsc::UnboundedSender<TaskResult>,
    ) -> Self {
        Self {
            task_queue: Arc::new(RwLock::new(HashMap::new())),
            running_tasks: Arc::new(RwLock::new(HashMap::new())),
            task_results: Arc::new(RwLock::new(HashMap::new())),
            progress_sender: Arc::new(progress_sender),
            completion_sender: Arc::new(completion_sender),
            settings: ProcessorSettings::default(),
            shutdown_tx: Arc::new(Mutex::new(None)),
        }
    }

    /// Create with custom settings
    pub fn with_settings(
        progress_sender: mpsc::UnboundedSender<SyncProgress>,
        completion_sender: mpsc::UnboundedSender<TaskResult>,
        settings: ProcessorSettings,
    ) -> Self {
        Self {
            task_queue: Arc::new(RwLock::new(HashMap::new())),
            running_tasks: Arc::new(RwLock::new(HashMap::new())),
            task_results: Arc::new(RwLock::new(HashMap::new())),
            progress_sender: Arc::new(progress_sender),
            completion_sender: Arc::new(completion_sender),
            settings,
            shutdown_tx: Arc::new(Mutex::new(None)),
        }
    }

    /// Start the background processor
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);
        
        {
            let mut shutdown_handle = self.shutdown_tx.lock().await;
            *shutdown_handle = Some(shutdown_tx);
        }

        let task_queue = self.task_queue.clone();
        let running_tasks = self.running_tasks.clone();
        let task_results = self.task_results.clone();
        let progress_sender = self.progress_sender.clone();
        let completion_sender = self.completion_sender.clone();
        let settings = self.settings.clone();

        tokio::spawn(async move {
            let mut processing_interval = tokio::time::interval(settings.processing_interval);

            loop {
                tokio::select! {
                    _ = processing_interval.tick() => {
                        Self::process_queue(
                            &task_queue,
                            &running_tasks,
                            &task_results,
                            &progress_sender,
                            &completion_sender,
                            &settings,
                        ).await;
                    }
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop the background processor
    pub async fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(shutdown_tx) = self.shutdown_tx.lock().await.take() {
            let _ = shutdown_tx.send(()).await;
        }

        // Cancel all running tasks
        {
            let mut running_tasks = self.running_tasks.write().await;
            for (_, handle) in running_tasks.drain() {
                handle.abort();
            }
        }

        Ok(())
    }

    /// Queue a background task
    pub async fn queue_task(&self, task: BackgroundTask) -> Result<Uuid, String> {
        {
            let task_queue = self.task_queue.read().await;
            let total_queued: usize = task_queue.values().map(|v| v.len()).sum();
            
            if total_queued >= self.settings.max_queue_size {
                return Err("Task queue is full".to_string());
            }
        }

        let task_id = task.id;
        
        {
            let mut task_queue = self.task_queue.write().await;
            let priority_queue = task_queue.entry(task.priority).or_insert_with(Vec::new);
            priority_queue.push(task);
        }

        Ok(task_id)
    }

    /// Cancel a queued or running task
    pub async fn cancel_task(&self, task_id: Uuid) -> bool {
        // Try to remove from queue first
        {
            let mut task_queue = self.task_queue.write().await;
            for priority_queue in task_queue.values_mut() {
                if let Some(pos) = priority_queue.iter().position(|t| t.id == task_id) {
                    priority_queue.remove(pos);
                    return true;
                }
            }
        }

        // Try to cancel running task
        {
            let mut running_tasks = self.running_tasks.write().await;
            if let Some(handle) = running_tasks.remove(&task_id) {
                handle.abort();
                
                // Add cancelled result
                let result = TaskResult {
                    task_id,
                    status: TaskStatus::Cancelled,
                    started_at: Instant::now(),
                    completed_at: Some(Instant::now()),
                    error: None,
                    result_data: None,
                };
                
                let _ = self.completion_sender.send(result);
                return true;
            }
        }

        false
    }

    /// Get task status
    pub async fn get_task_status(&self, task_id: Uuid) -> Option<TaskStatus> {
        // Check running tasks
        {
            let running_tasks = self.running_tasks.read().await;
            if running_tasks.contains_key(&task_id) {
                return Some(TaskStatus::Running);
            }
        }

        // Check completed tasks
        {
            let task_results = self.task_results.read().await;
            if let Some(result) = task_results.get(&task_id) {
                return Some(result.status.clone());
            }
        }

        // Check queued tasks
        {
            let task_queue = self.task_queue.read().await;
            for priority_queue in task_queue.values() {
                if priority_queue.iter().any(|t| t.id == task_id) {
                    return Some(TaskStatus::Queued);
                }
            }
        }

        None
    }

    /// Get all queued tasks
    pub async fn get_queued_tasks(&self) -> Vec<BackgroundTask> {
        let task_queue = self.task_queue.read().await;
        let mut tasks = Vec::new();
        
        for priority_queue in task_queue.values() {
            tasks.extend(priority_queue.iter().cloned());
        }
        
        // Sort by priority (high to low) then by creation time
        tasks.sort_by(|a, b| {
            b.priority.cmp(&a.priority).then(a.created_at.cmp(&b.created_at))
        });
        
        tasks
    }

    /// Get all running tasks
    pub async fn get_running_tasks(&self) -> Vec<Uuid> {
        let running_tasks = self.running_tasks.read().await;
        running_tasks.keys().cloned().collect()
    }

    /// Get task result
    pub async fn get_task_result(&self, task_id: Uuid) -> Option<TaskResult> {
        let task_results = self.task_results.read().await;
        task_results.get(&task_id).cloned()
    }

    /// Process the task queue
    async fn process_queue(
        task_queue: &Arc<RwLock<HashMap<TaskPriority, Vec<BackgroundTask>>>>,
        running_tasks: &Arc<RwLock<HashMap<Uuid, JoinHandle<TaskResult>>>>,
        _task_results: &Arc<RwLock<HashMap<Uuid, TaskResult>>>,
        progress_sender: &Arc<mpsc::UnboundedSender<SyncProgress>>,
        completion_sender: &Arc<mpsc::UnboundedSender<TaskResult>>,
        settings: &ProcessorSettings,
    ) {
        // Clean up completed tasks
        {
            let mut running = running_tasks.write().await;
            let mut completed_tasks = Vec::new();
            
            running.retain(|&task_id, handle| {
                if handle.is_finished() {
                    completed_tasks.push(task_id);
                    false
                } else {
                    true
                }
            });
        }

        // Check if we can start new tasks
        let current_running = {
            let running = running_tasks.read().await;
            running.len()
        };

        if current_running >= settings.max_concurrent_tasks {
            return;
        }

        // Get next task to execute (highest priority first)
        let next_task = {
            let mut queue = task_queue.write().await;
            let mut selected_task = None;
            
            // Process priorities from high to low
            for priority in [TaskPriority::Critical, TaskPriority::High, TaskPriority::Normal, TaskPriority::Low] {
                if let Some(priority_queue) = queue.get_mut(&priority) {
                    if !priority_queue.is_empty() {
                        selected_task = Some(priority_queue.remove(0));
                        break;
                    }
                }
            }
            
            selected_task
        };

        if let Some(task) = next_task {
            // Start task execution
            let task_id = task.id;
            let progress_sender_clone = progress_sender.clone();
            let completion_sender_clone = completion_sender.clone();
            let task_timeout = settings.task_timeout;
            
            let handle = tokio::spawn(async move {
                let started_at = Instant::now();
                
                // Execute task with timeout
                let result = tokio::time::timeout(
                    task_timeout,
                    Self::execute_task(task, progress_sender_clone)
                ).await;

                let task_result = match result {
                    Ok(Ok(result_data)) => TaskResult {
                        task_id,
                        status: TaskStatus::Completed,
                        started_at,
                        completed_at: Some(Instant::now()),
                        error: None,
                        result_data: Some(result_data),
                    },
                    Ok(Err(error)) => TaskResult {
                        task_id,
                        status: TaskStatus::Failed(error.clone()),
                        started_at,
                        completed_at: Some(Instant::now()),
                        error: Some(error),
                        result_data: None,
                    },
                    Err(_) => TaskResult {
                        task_id,
                        status: TaskStatus::Failed("Task timeout".to_string()),
                        started_at,
                        completed_at: Some(Instant::now()),
                        error: Some("Task execution timed out".to_string()),
                        result_data: None,
                    },
                };

                // Send completion notification
                let _ = completion_sender_clone.send(task_result.clone());
                
                task_result
            });

            // Store running task
            {
                let mut running = running_tasks.write().await;
                running.insert(task_id, handle);
            }
        }
    }

    /// Execute a specific task
    async fn execute_task(
        task: BackgroundTask,
        _progress_sender: Arc<mpsc::UnboundedSender<SyncProgress>>,
    ) -> Result<TaskResultData, String> {
        match task.task_type {
            BackgroundTaskType::FolderRefresh { folder_name: _ } => {
                // Simulate folder refresh
                tokio::time::sleep(Duration::from_millis(500)).await;
                Ok(TaskResultData::MessageCount(42)) // Placeholder
            }
            BackgroundTaskType::FolderSync { folder_name: _, strategy: _ } => {
                // This would integrate with the actual sync engine
                tokio::time::sleep(Duration::from_secs(2)).await;
                Ok(TaskResultData::MessageCount(10)) // Placeholder
            }
            BackgroundTaskType::AccountSync { strategy: _ } => {
                // This would sync all folders for an account
                tokio::time::sleep(Duration::from_secs(5)).await;
                Ok(TaskResultData::MessageCount(50)) // Placeholder
            }
            BackgroundTaskType::Search { query: _, folders: _ } => {
                // Simulate search operation
                tokio::time::sleep(Duration::from_secs(1)).await;
                Ok(TaskResultData::SearchResults(vec!["msg1".to_string(), "msg2".to_string()]))
            }
            BackgroundTaskType::Indexing { folder_name: _ } => {
                // Simulate indexing
                tokio::time::sleep(Duration::from_secs(3)).await;
                Ok(TaskResultData::MessageCount(100))
            }
            BackgroundTaskType::CachePreload { folder_name: _, message_count } => {
                // Simulate cache preloading
                tokio::time::sleep(Duration::from_millis(200)).await;
                Ok(TaskResultData::CacheStats(message_count))
            }
        }
    }

    /// Clean up old task results
    async fn cleanup_old_results(
        _task_results: &Arc<RwLock<HashMap<Uuid, TaskResult>>>,
        max_results: usize,
    ) {
        let mut results = _task_results.write().await;
        
        if results.len() <= max_results {
            return;
        }

        // Sort by completion time and keep only the newest results
        let mut sorted_results: Vec<_> = results.iter().collect();
        sorted_results.sort_by(|a, b| {
            b.1.completed_at.cmp(&a.1.completed_at)
        });

        let to_remove: Vec<_> = sorted_results.iter()
            .skip(max_results)
            .map(|(id, _)| **id)
            .collect();

        for id in to_remove {
            results.remove(&id);
        }
    }
}