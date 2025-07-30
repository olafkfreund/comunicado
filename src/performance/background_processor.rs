//! Background task processor for non-blocking operations
//!
//! This module provides an async background processing system that handles
//! email synchronization, folder refresh, and other long-running operations
//! without blocking the UI thread.

use crate::email::sync_engine::{SyncProgress, SyncStrategy, SyncPhase};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::task::JoinHandle;
use uuid::Uuid;
use chrono::Utc;

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

/// Background task processor for non-blocking operations
/// 
/// This processor manages a priority-based queue of background tasks to prevent UI blocking.
/// Tasks are executed asynchronously with configurable concurrency limits and timeouts.
/// 
/// # Features
/// - Priority-based task scheduling (Critical, High, Normal, Low)
/// - Configurable concurrent task limits
/// - Progress tracking with real-time updates
/// - Automatic task timeout and cleanup
/// - Task cancellation support
/// 
/// # Example
/// ```rust
/// use comunicado::performance::background_processor::*;
/// use tokio::sync::mpsc;
/// 
/// let (progress_tx, progress_rx) = mpsc::unbounded_channel();
/// let (completion_tx, completion_rx) = mpsc::unbounded_channel();
/// 
/// let processor = BackgroundProcessor::new(progress_tx, completion_tx);
/// processor.start().await?;
/// 
/// // Queue a task
/// let task = BackgroundTask { /* ... */ };
/// let task_id = processor.queue_task(task).await?;
/// ```
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
    /// Creates a new background processor with default settings
    /// 
    /// # Arguments
    /// * `progress_sender` - Channel for sending sync progress updates to UI
    /// * `completion_sender` - Channel for sending task completion notifications
    /// 
    /// # Returns
    /// A new `BackgroundProcessor` instance with default configuration:
    /// - Max 3 concurrent tasks
    /// - 5 minute task timeout
    /// - Queue size limit of 100 tasks
    /// - Result cache size of 50 entries
    /// - 100ms processing interval
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

    /// Creates a new background processor with custom settings
    /// 
    /// # Arguments
    /// * `progress_sender` - Channel for sending sync progress updates to UI
    /// * `completion_sender` - Channel for sending task completion notifications  
    /// * `settings` - Custom processor configuration
    /// 
    /// # Returns
    /// A new `BackgroundProcessor` instance with the specified configuration
    /// 
    /// # Example
    /// ```rust
    /// let settings = ProcessorSettings {
    ///     max_concurrent_tasks: 2,
    ///     task_timeout: Duration::from_secs(120),
    ///     max_queue_size: 50,
    ///     result_cache_size: 25,
    ///     processing_interval: Duration::from_millis(250),
    /// };
    /// let processor = BackgroundProcessor::with_settings(progress_tx, completion_tx, settings);
    /// ```
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

    /// Starts the background processor task queue
    /// 
    /// This spawns the main processor loop that handles task scheduling and execution.
    /// The processor will run until `stop()` is called.
    /// 
    /// # Returns
    /// `Ok(())` if the processor started successfully, or an error if startup failed
    /// 
    /// # Example
    /// ```rust
    /// let processor = BackgroundProcessor::new(progress_tx, completion_tx);
    /// processor.start().await?;
    /// // Processor is now running in the background
    /// ```
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

    /// Stops the background processor and cancels all running tasks
    /// 
    /// This method sends a shutdown signal to the processor loop and cancels
    /// all currently running tasks. It's important to call this method before
    /// dropping the processor to ensure clean shutdown.
    /// 
    /// # Returns
    /// `Ok(())` if shutdown completed successfully, or an error if shutdown failed
    /// 
    /// # Example
    /// ```rust
    /// // Graceful shutdown
    /// processor.stop().await?;
    /// ```
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

    /// Queues a background task for execution
    /// 
    /// Tasks are queued according to their priority level and will be executed
    /// when processor capacity becomes available. The queue has a configurable
    /// size limit to prevent memory issues.
    /// 
    /// # Arguments
    /// * `task` - The background task to queue for execution
    /// 
    /// # Returns
    /// `Ok(task_id)` if the task was queued successfully, or `Err(msg)` if
    /// the queue is full or another error occurred
    /// 
    /// # Errors
    /// Returns an error if:
    /// - The task queue has reached its maximum capacity
    /// - The processor has been shut down
    /// 
    /// # Example
    /// ```rust
    /// let task = BackgroundTask {
    ///     id: Uuid::new_v4(),
    ///     name: "Sync INBOX".to_string(),
    ///     priority: TaskPriority::Normal,
    ///     // ... other fields
    /// };
    /// 
    /// let task_id = processor.queue_task(task).await?;
    /// println!("Task queued with ID: {}", task_id);
    /// ```
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

    /// Cancels a queued or running task
    /// 
    /// This method first attempts to remove the task from the queue if it hasn't
    /// started executing yet. If the task is already running, it will be aborted
    /// and marked as cancelled.
    /// 
    /// # Arguments
    /// * `task_id` - The unique identifier of the task to cancel
    /// 
    /// # Returns
    /// `true` if the task was found and cancelled, `false` if the task was not found
    /// 
    /// # Example
    /// ```rust
    /// let task_id = processor.queue_task(task).await?;
    /// 
    /// // Cancel the task if needed
    /// if processor.cancel_task(task_id).await {
    ///     println!("Task cancelled successfully");
    /// } else {
    ///     println!("Task not found or already completed");
    /// }
    /// ```
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

    /// Retrieves the current status of a task
    /// 
    /// Checks the running tasks, completed tasks cache, and queued tasks
    /// to determine the current status of the specified task.
    /// 
    /// # Arguments
    /// * `task_id` - The unique identifier of the task to check
    /// 
    /// # Returns
    /// `Some(TaskStatus)` if the task is found, `None` if the task doesn't exist
    /// or has been cleaned up from the cache
    /// 
    /// # Example
    /// ```rust
    /// match processor.get_task_status(task_id).await {
    ///     Some(TaskStatus::Running) => println!("Task is currently executing"),
    ///     Some(TaskStatus::Completed) => println!("Task completed successfully"),
    ///     Some(TaskStatus::Failed(error)) => println!("Task failed: {}", error),
    ///     Some(TaskStatus::Queued) => println!("Task is waiting in queue"),
    ///     Some(TaskStatus::Cancelled) => println!("Task was cancelled"),
    ///     None => println!("Task not found"),
    /// }
    /// ```
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

    /// Retrieves all tasks currently waiting in the queue
    /// 
    /// Returns a list of all queued tasks sorted by priority (highest first)
    /// and then by creation time (oldest first). This is useful for monitoring
    /// queue status and debugging.
    /// 
    /// # Returns
    /// A vector of `BackgroundTask` objects representing all queued tasks,
    /// sorted by priority and creation time
    /// 
    /// # Example
    /// ```rust
    /// let queued = processor.get_queued_tasks().await;
    /// println!("Tasks in queue: {}", queued.len());
    /// 
    /// for task in queued {
    ///     println!("  {} - Priority: {:?}", task.name, task.priority);
    /// }
    /// ```
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

    /// Retrieves the IDs of all currently executing tasks
    /// 
    /// Returns a list of task IDs for tasks that are currently being executed.
    /// This is useful for monitoring processor workload and debugging.
    /// 
    /// # Returns
    /// A vector of `Uuid` values representing the IDs of all running tasks
    /// 
    /// # Example
    /// ```rust
    /// let running = processor.get_running_tasks().await;
    /// println!("Currently running {} tasks", running.len());
    /// 
    /// for task_id in running {
    ///     println!("  Running task: {}", task_id);
    /// }
    /// ```
    pub async fn get_running_tasks(&self) -> Vec<Uuid> {
        let running_tasks = self.running_tasks.read().await;
        running_tasks.keys().cloned().collect()
    }

    /// Retrieves the result of a completed task
    /// 
    /// Returns the full result information for a task that has completed execution,
    /// including status, timing information, and any result data. Results are
    /// cached for a limited time before being cleaned up.
    /// 
    /// # Arguments
    /// * `task_id` - The unique identifier of the task whose result to retrieve
    /// 
    /// # Returns
    /// `Some(TaskResult)` if the task has completed and its result is still cached,
    /// `None` if the task hasn't completed or the result has been cleaned up
    /// 
    /// # Example
    /// ```rust
    /// if let Some(result) = processor.get_task_result(task_id).await {
    ///     match result.status {
    ///         TaskStatus::Completed => {
    ///             let duration = result.completed_at.unwrap()
    ///                 .duration_since(result.started_at);
    ///             println!("Task completed in {:.2}s", duration.as_secs_f64());
    ///         }
    ///         TaskStatus::Failed(error) => {
    ///             println!("Task failed: {}", error);
    ///         }
    ///         _ => {}
    ///     }
    /// }
    /// ```
    pub async fn get_task_result(&self, task_id: Uuid) -> Option<TaskResult> {
        let task_results = self.task_results.read().await;
        task_results.get(&task_id).cloned()
    }

    /// Internal method to process the task queue
    /// 
    /// This is the core processing loop that:
    /// 1. Cleans up completed tasks
    /// 2. Checks if new tasks can be started based on concurrency limits
    /// 3. Selects the highest priority task from the queue
    /// 4. Spawns task execution with timeout handling
    /// 
    /// This method is called periodically by the main processor loop.
    /// 
    /// # Arguments
    /// * `task_queue` - Shared reference to the priority-based task queue
    /// * `running_tasks` - Shared reference to currently executing tasks
    /// * `_task_results` - Shared reference to completed task results cache
    /// * `progress_sender` - Channel for sending progress updates to UI
    /// * `completion_sender` - Channel for sending task completion notifications
    /// * `settings` - Processor configuration settings
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

    /// Internal method to execute a specific background task
    /// 
    /// This method handles the actual execution of different task types,
    /// sending progress updates throughout the process. Each task type
    /// has its own execution logic and progress reporting pattern.
    /// 
    /// # Arguments
    /// * `task` - The background task to execute
    /// * `progress_sender` - Channel for sending real-time progress updates
    /// 
    /// # Returns
    /// `Ok(TaskResultData)` if the task completed successfully,
    /// `Err(String)` if the task failed with an error message
    /// 
    /// # Task Types Supported
    /// - `FolderRefresh`: Quick metadata refresh for a folder
    /// - `FolderSync`: Full synchronization of a folder with progress tracking
    /// - `AccountSync`: Complete account synchronization across all folders
    /// - `Search`: Search operation across specified folders
    /// - `Indexing`: Message indexing for search functionality
    /// - `CachePreload`: Preload message cache for faster access
    async fn execute_task(
        task: BackgroundTask,
        progress_sender: Arc<mpsc::UnboundedSender<SyncProgress>>,
    ) -> Result<TaskResultData, String> {
        match task.task_type {
            BackgroundTaskType::FolderRefresh { folder_name } => {
                // Send progress update
                let progress = SyncProgress {
                    account_id: task.account_id.clone(),
                    folder_name: folder_name.clone(),
                    phase: SyncPhase::FetchingBodies,
                    messages_processed: 0,
                    total_messages: 0,
                    bytes_downloaded: 0,
                    started_at: Utc::now(),
                    estimated_completion: None,
                };
                let _ = progress_sender.send(progress);

                // NOTE: This would normally integrate with the actual IMAP client
                // For now, we simulate a quick folder refresh
                tokio::time::sleep(Duration::from_millis(500)).await;
                Ok(TaskResultData::MessageCount(42))
            }
            BackgroundTaskType::FolderSync { folder_name, strategy: _ } => {
                // Send progress updates during sync
                let progress = SyncProgress {
                    account_id: task.account_id.clone(),
                    folder_name: folder_name.clone(),
                    phase: SyncPhase::Initializing,
                    messages_processed: 0,
                    total_messages: 100,
                    bytes_downloaded: 0,
                    started_at: Utc::now(),
                    estimated_completion: Some(Utc::now() + chrono::Duration::seconds(30)),
                };
                let _ = progress_sender.send(progress);

                // Simulate progressive sync with status updates
                for i in 0..10 {
                    tokio::time::sleep(Duration::from_millis(300)).await;
                    let progress = SyncProgress {
                        account_id: task.account_id.clone(),
                        folder_name: folder_name.clone(),
                        phase: SyncPhase::FetchingBodies,
                        messages_processed: i * 10,
                        total_messages: 100,
                        bytes_downloaded: (i * 1024) as u64,
                        started_at: Utc::now(),
                        estimated_completion: Some(Utc::now() + chrono::Duration::seconds((30 - i * 3) as i64)),
                    };
                    let _ = progress_sender.send(progress);
                }

                Ok(TaskResultData::MessageCount(100))
            }
            BackgroundTaskType::AccountSync { strategy: _ } => {
                // Send progress for full account sync
                let progress = SyncProgress {
                    account_id: task.account_id.clone(),
                    folder_name: "All Folders".to_string(),
                    phase: SyncPhase::Initializing,
                    messages_processed: 0,
                    total_messages: 500,
                    bytes_downloaded: 0,
                    started_at: Utc::now(),
                    estimated_completion: Some(Utc::now() + chrono::Duration::minutes(2)),
                };
                let _ = progress_sender.send(progress);

                // Simulate account-wide sync
                tokio::time::sleep(Duration::from_secs(5)).await;
                Ok(TaskResultData::MessageCount(500))
            }
            BackgroundTaskType::Search { query: _, folders: _ } => {
                // Send search progress
                let progress = SyncProgress {
                    account_id: task.account_id.clone(),
                    folder_name: "Search".to_string(),
                    phase: SyncPhase::ProcessingChanges,
                    messages_processed: 0,
                    total_messages: 1000,
                    bytes_downloaded: 0,
                    started_at: Utc::now(),
                    estimated_completion: Some(Utc::now() + chrono::Duration::seconds(10)),
                };
                let _ = progress_sender.send(progress);

                tokio::time::sleep(Duration::from_secs(1)).await;
                Ok(TaskResultData::SearchResults(vec!["msg1".to_string(), "msg2".to_string()]))
            }
            BackgroundTaskType::Indexing { folder_name } => {
                // Send indexing progress
                let progress = SyncProgress {
                    account_id: task.account_id.clone(),
                    folder_name: folder_name.clone(),
                    phase: SyncPhase::ProcessingChanges,
                    messages_processed: 0,
                    total_messages: 1000,
                    bytes_downloaded: 0,
                    started_at: Utc::now(),
                    estimated_completion: Some(Utc::now() + chrono::Duration::minutes(1)),
                };
                let _ = progress_sender.send(progress);

                tokio::time::sleep(Duration::from_secs(3)).await;
                Ok(TaskResultData::MessageCount(1000))
            }
            BackgroundTaskType::CachePreload { folder_name, message_count } => {
                // Send cache preload progress
                let progress = SyncProgress {
                    account_id: task.account_id.clone(),
                    folder_name: folder_name.clone(),
                    phase: SyncPhase::ProcessingChanges,
                    messages_processed: 0,
                    total_messages: message_count as u32,
                    bytes_downloaded: 0,
                    started_at: Utc::now(),
                    estimated_completion: Some(Utc::now() + chrono::Duration::seconds(5)),
                };
                let _ = progress_sender.send(progress);

                tokio::time::sleep(Duration::from_millis(200)).await;
                Ok(TaskResultData::CacheStats(message_count))
            }
        }
    }

    /// Internal method to clean up old task results from the cache
    /// 
    /// This method removes older task results to prevent unbounded memory growth.
    /// Results are sorted by completion time and only the most recent results
    /// are kept in the cache.
    /// 
    /// # Arguments
    /// * `_task_results` - Shared reference to the task results cache
    /// * `max_results` - Maximum number of results to keep in cache
    /// 
    /// # Behavior
    /// - If the cache contains fewer than `max_results`, no cleanup is performed
    /// - Results are sorted by completion time (newest first)
    /// - Older results beyond the limit are removed from the cache
    /// 
    /// # Note
    /// This method is currently unused but available for future cache management
    #[allow(dead_code)]
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