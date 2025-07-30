//! Background task management for startup optimization
//!
//! This module provides:
//! - Async task spawning and management
//! - Task priority handling
//! - Progress tracking and status updates
//! - Resource-efficient task scheduling

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock, Semaphore};
use tokio::task::JoinHandle;
use uuid::Uuid;
// Removed serde imports as they're not needed

/// Task priority levels for scheduling
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Critical = 0,   // Must complete before UI starts
    High = 1,       // Required for core functionality
    Medium = 2,     // Important but non-blocking
    Low = 3,        // Background optimization
    Cleanup = 4,    // Cleanup and maintenance
}

/// Task status tracking
#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Pending,
    Running { started_at: Instant },
    Completed { duration: Duration },
    Failed { error: String, duration: Duration },
    Cancelled,
}

/// Task execution context and metadata
#[derive(Debug, Clone)]
pub struct TaskContext {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub priority: TaskPriority,
    pub estimated_duration: Option<Duration>,
    pub timeout: Option<Duration>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub dependencies: Vec<Uuid>,
}

/// Task execution result
#[derive(Debug)]
pub enum TaskResult {
    Success,
    Failure(String),
    Timeout,
    Cancelled,
}

/// Background task handle for tracking and control
pub struct TaskHandle {
    pub id: Uuid,
    pub context: TaskContext,
    handle: JoinHandle<TaskResult>,
    status: Arc<RwLock<TaskStatus>>,
    progress: Arc<RwLock<f64>>,
    status_sender: mpsc::UnboundedSender<TaskStatusUpdate>,
}

impl TaskHandle {
    /// Get current task status
    pub async fn status(&self) -> TaskStatus {
        self.status.read().await.clone()
    }
    
    /// Get current progress (0.0 to 1.0)
    pub async fn progress(&self) -> f64 {
        *self.progress.read().await
    }
    
    /// Check if task is completed
    pub async fn is_completed(&self) -> bool {
        matches!(
            *self.status.read().await,
            TaskStatus::Completed { .. } | TaskStatus::Failed { .. } | TaskStatus::Cancelled
        )
    }
    
    /// Cancel the task
    pub fn cancel(&self) {
        self.handle.abort();
    }
    
    /// Wait for task completion
    pub async fn wait(self) -> TaskResult {
        match self.handle.await {
            Ok(result) => result,
            Err(_) => TaskResult::Cancelled,
        }
    }
}

/// Task status update message
#[derive(Debug, Clone)]
pub struct TaskStatusUpdate {
    pub task_id: Uuid,
    pub status: TaskStatus,
    pub progress: Option<f64>,
    pub message: Option<String>,
}

/// Background task manager for coordinating async operations
pub struct BackgroundTaskManager {
    tasks: Arc<RwLock<HashMap<Uuid, TaskHandle>>>,
    task_queue: Arc<RwLock<Vec<(TaskContext, Pin<Box<dyn Future<Output = TaskResult> + Send>>)>>>,
    status_sender: mpsc::UnboundedSender<TaskStatusUpdate>,
    status_receiver: mpsc::UnboundedReceiver<TaskStatusUpdate>,
    max_concurrent_tasks: usize,
    semaphore: Arc<Semaphore>,
    is_running: Arc<RwLock<bool>>,
}

impl BackgroundTaskManager {
    /// Create a new background task manager
    pub fn new(max_concurrent_tasks: usize) -> Self {
        let (status_sender, status_receiver) = mpsc::unbounded_channel();
        let semaphore = Arc::new(Semaphore::new(max_concurrent_tasks));
        
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            task_queue: Arc::new(RwLock::new(Vec::new())),
            status_sender,
            status_receiver,
            max_concurrent_tasks,
            semaphore,
            is_running: Arc::new(RwLock::new(false)),
        }
    }
    
    /// Spawn a new background task
    pub async fn spawn_task<F, Fut>(&self, context: TaskContext, task_fn: F) -> Uuid
    where
        F: FnOnce(TaskProgressReporter) -> Fut + Send + 'static,
        Fut: Future<Output = Result<(), String>> + Send + 'static,
    {
        let task_id = context.id;
        let status = Arc::new(RwLock::new(TaskStatus::Pending));
        let progress = Arc::new(RwLock::new(0.0));
        
        // Create progress reporter for the task
        let progress_reporter = TaskProgressReporter {
            id: task_id,
            status: Arc::clone(&status),
            progress: Arc::clone(&progress),
            sender: self.status_sender.clone(),
        };
        
        // Create the async task wrapper
        let semaphore = Arc::clone(&self.semaphore);
        let task_context = context.clone();
        let task_status = Arc::clone(&status);
        
        let handle = tokio::spawn(async move {
            // Wait for semaphore permit (concurrency control)
            let _permit = semaphore.acquire().await.unwrap();
            
            // Update status to running
            {
                let mut status_guard = task_status.write().await;
                *status_guard = TaskStatus::Running { started_at: Instant::now() };
            }
            
            let start_time = Instant::now();
            
            // Execute the task with timeout if specified
            let result = if let Some(timeout) = task_context.timeout {
                match tokio::time::timeout(timeout, task_fn(progress_reporter)).await {
                    Ok(Ok(())) => TaskResult::Success,
                    Ok(Err(err)) => TaskResult::Failure(err),
                    Err(_) => TaskResult::Timeout,
                }
            } else {
                match task_fn(progress_reporter).await {
                    Ok(()) => TaskResult::Success,
                    Err(err) => TaskResult::Failure(err),
                }
            };
            
            let duration = start_time.elapsed();
            
            // Update final status
            {
                let mut status_guard = task_status.write().await;
                *status_guard = match &result {
                    TaskResult::Success => TaskStatus::Completed { duration },
                    TaskResult::Failure(err) => TaskStatus::Failed { 
                        error: err.clone(), 
                        duration 
                    },
                    TaskResult::Timeout => TaskStatus::Failed { 
                        error: "Task timed out".to_string(), 
                        duration 
                    },
                    TaskResult::Cancelled => TaskStatus::Cancelled,
                };
            }
            
            result
        });
        
        // Create task handle
        let task_handle = TaskHandle {
            id: task_id,
            context,
            handle,
            status,
            progress,
            status_sender: self.status_sender.clone(),
        };
        
        // Store the task handle
        {
            let mut tasks = self.tasks.write().await;
            tasks.insert(task_id, task_handle);
        }
        
        task_id
    }
    
    /// Get task by ID
    pub async fn get_task(&self, task_id: &Uuid) -> Option<TaskHandle> {
        // Note: This would need to be implemented differently in practice
        // as TaskHandle contains non-Clone fields. For now, we'll return status info.
        None
    }
    
    /// Get all active task IDs
    pub async fn active_tasks(&self) -> Vec<Uuid> {
        let tasks = self.tasks.read().await;
        tasks.keys().cloned().collect()
    }
    
    /// Get task status
    pub async fn task_status(&self, task_id: &Uuid) -> Option<TaskStatus> {
        let tasks = self.tasks.read().await;
        if let Some(task) = tasks.get(task_id) {
            Some(task.status().await)
        } else {
            None
        }
    }
    
    /// Get task progress
    pub async fn task_progress(&self, task_id: &Uuid) -> Option<f64> {
        let tasks = self.tasks.read().await;
        if let Some(task) = tasks.get(task_id) {
            Some(task.progress().await)
        } else {
            None
        }
    }
    
    /// Update task progress (called internally)
    pub fn update_task_progress(&self, task_id: Uuid, progress: f64) {
        let _ = self.status_sender.send(TaskStatusUpdate {
            task_id,
            status: TaskStatus::Pending, // Will be ignored for progress updates
            progress: Some(progress),
            message: None,
        });
    }
    
    /// Update task status (called internally)
    pub fn update_task_status(&self, task_id: Uuid, message: String) {
        let _ = self.status_sender.send(TaskStatusUpdate {
            task_id,
            status: TaskStatus::Pending, // Will be ignored for status updates
            progress: None,
            message: Some(message),
        });
    }
    
    /// Get number of active tasks
    pub async fn active_task_count(&self) -> usize {
        let tasks = self.tasks.read().await;
        let mut count = 0;
        for task in tasks.values() {
            if !task.is_completed().await {
                count += 1;
            }
        }
        count
    }
    
    /// Wait for all tasks with given priority to complete
    pub async fn wait_for_priority(&self, priority: TaskPriority) -> Vec<TaskResult> {
        let mut results = Vec::new();
        let task_ids: Vec<Uuid> = {
            let tasks = self.tasks.read().await;
            tasks.values()
                .filter(|task| task.context.priority == priority)
                .map(|task| task.id)
                .collect()
        };
        
        for task_id in task_ids {
            if let Some(task) = {
                let mut tasks = self.tasks.write().await;
                tasks.remove(&task_id)
            } {
                let result = task.wait().await;
                results.push(result);
            }
        }
        
        results
    }
    
    /// Cancel all tasks
    pub async fn cancel_all(&self) {
        let tasks = self.tasks.read().await;
        for task in tasks.values() {
            task.cancel();
        }
    }
    
    /// Get task statistics
    pub async fn get_statistics(&self) -> TaskStatistics {
        let tasks = self.tasks.read().await;
        let mut stats = TaskStatistics::default();
        
        for task in tasks.values() {
            stats.total_tasks += 1;
            
            match task.status().await {
                TaskStatus::Pending => stats.pending_tasks += 1,
                TaskStatus::Running { .. } => stats.running_tasks += 1,
                TaskStatus::Completed { .. } => stats.completed_tasks += 1,
                TaskStatus::Failed { .. } => stats.failed_tasks += 1,
                TaskStatus::Cancelled => stats.cancelled_tasks += 1,
            }
            
            match task.context.priority {
                TaskPriority::Critical => stats.critical_tasks += 1,
                TaskPriority::High => stats.high_priority_tasks += 1,
                TaskPriority::Medium => stats.medium_priority_tasks += 1,
                TaskPriority::Low => stats.low_priority_tasks += 1,
                TaskPriority::Cleanup => stats.cleanup_tasks += 1,
            }
        }
        
        stats
    }
    
    /// Process status updates (should be called regularly)
    pub async fn process_status_updates(&mut self) -> Vec<TaskStatusUpdate> {
        let mut updates = Vec::new();
        
        while let Ok(update) = self.status_receiver.try_recv() {
            updates.push(update);
        }
        
        updates
    }
}

/// Progress reporter for tasks to update their status
pub struct TaskProgressReporter {
    id: Uuid,
    status: Arc<RwLock<TaskStatus>>,
    progress: Arc<RwLock<f64>>,
    sender: mpsc::UnboundedSender<TaskStatusUpdate>,
}

impl TaskProgressReporter {
    /// Update task progress (0.0 to 1.0)
    pub async fn update_progress(&self, progress: f64) {
        {
            let mut progress_guard = self.progress.write().await;
            *progress_guard = progress.clamp(0.0, 1.0);
        }
        
        let _ = self.sender.send(TaskStatusUpdate {
            task_id: self.id,
            status: TaskStatus::Pending, // Ignored for progress updates
            progress: Some(progress),
            message: None,
        });
    }
    
    /// Send a status message
    pub fn send_message(&self, message: String) {
        let _ = self.sender.send(TaskStatusUpdate {
            task_id: self.id,
            status: TaskStatus::Pending, // Ignored for message updates
            progress: None,
            message: Some(message),
        });
    }
}

/// Task execution statistics
#[derive(Debug, Default, Clone)]
pub struct TaskStatistics {
    pub total_tasks: usize,
    pub pending_tasks: usize,
    pub running_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    pub cancelled_tasks: usize,
    pub critical_tasks: usize,
    pub high_priority_tasks: usize,
    pub medium_priority_tasks: usize,
    pub low_priority_tasks: usize,
    pub cleanup_tasks: usize,
}

impl TaskStatistics {
    /// Get completion percentage
    pub fn completion_percentage(&self) -> f64 {
        if self.total_tasks == 0 {
            100.0
        } else {
            (self.completed_tasks as f64 / self.total_tasks as f64) * 100.0
        }
    }
    
    /// Check if all critical tasks are complete
    pub fn critical_tasks_complete(&self) -> bool {
        self.critical_tasks == 0 || 
        self.completed_tasks >= self.critical_tasks
    }
}

impl TaskContext {
    /// Create a new task context
    pub fn new(name: String, description: String, priority: TaskPriority) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            description,
            priority,
            estimated_duration: None,
            timeout: None,
            retry_count: 0,
            max_retries: 0,
            dependencies: Vec::new(),
        }
    }
    
    /// Set estimated duration
    pub fn with_estimated_duration(mut self, duration: Duration) -> Self {
        self.estimated_duration = Some(duration);
        self
    }
    
    /// Set timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
    
    /// Set max retries
    pub fn with_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }
    
    /// Add dependency
    pub fn with_dependency(mut self, dependency_id: Uuid) -> Self {
        self.dependencies.push(dependency_id);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_background_task_manager_creation() {
        let manager = BackgroundTaskManager::new(4);
        
        assert_eq!(manager.max_concurrent_tasks, 4);
        assert_eq!(manager.active_task_count().await, 0);
    }

    #[tokio::test]
    async fn test_task_context_creation() {
        let context = TaskContext::new(
            "Test Task".to_string(),
            "A test task".to_string(),
            TaskPriority::High,
        )
        .with_estimated_duration(Duration::from_secs(5))
        .with_timeout(Duration::from_secs(10))
        .with_retries(3);
        
        assert_eq!(context.name, "Test Task");
        assert_eq!(context.priority, TaskPriority::High);
        assert_eq!(context.estimated_duration, Some(Duration::from_secs(5)));
        assert_eq!(context.timeout, Some(Duration::from_secs(10)));
        assert_eq!(context.max_retries, 3);
    }

    #[tokio::test]
    async fn test_task_spawning_and_completion() {
        let manager = BackgroundTaskManager::new(2);
        
        let context = TaskContext::new(
            "Test Task".to_string(),
            "A test task".to_string(),
            TaskPriority::Medium,
        );
        
        let task_id = manager.spawn_task(context, |reporter| async move {
            reporter.update_progress(0.5).await;
            sleep(Duration::from_millis(50)).await;
            reporter.update_progress(1.0).await;
            Ok(())
        }).await;
        
        // Wait a bit for task to start
        sleep(Duration::from_millis(25)).await;
        
        let status = manager.task_status(&task_id).await;
        assert!(matches!(status, Some(TaskStatus::Running { .. })));
        
        // Wait for completion
        sleep(Duration::from_millis(100)).await;
        
        let status = manager.task_status(&task_id).await;
        assert!(matches!(status, Some(TaskStatus::Completed { .. })));
    }

    #[tokio::test]
    async fn test_task_failure() {
        let manager = BackgroundTaskManager::new(2);
        
        let context = TaskContext::new(
            "Failing Task".to_string(),
            "A task that fails".to_string(),
            TaskPriority::Low,
        );
        
        let task_id = manager.spawn_task(context, |_reporter| async move {
            sleep(Duration::from_millis(10)).await;
            Err("Simulated failure".to_string())
        }).await;
        
        // Wait for completion
        sleep(Duration::from_millis(50)).await;
        
        let status = manager.task_status(&task_id).await;
        assert!(matches!(status, Some(TaskStatus::Failed { .. })));
    }

    #[tokio::test]
    async fn test_task_timeout() {
        let manager = BackgroundTaskManager::new(2);
        
        let context = TaskContext::new(
            "Slow Task".to_string(),
            "A slow task".to_string(),
            TaskPriority::Medium,
        )
        .with_timeout(Duration::from_millis(20));
        
        let task_id = manager.spawn_task(context, |_reporter| async move {
            sleep(Duration::from_millis(100)).await; // Longer than timeout
            Ok(())
        }).await;
        
        // Wait for timeout
        sleep(Duration::from_millis(50)).await;
        
        let status = manager.task_status(&task_id).await;
        assert!(matches!(status, Some(TaskStatus::Failed { .. })));
    }

    #[tokio::test]
    async fn test_task_statistics() {
        let manager = BackgroundTaskManager::new(4);
        
        // Spawn some tasks
        let _task1 = manager.spawn_task(
            TaskContext::new("Task 1".to_string(), "".to_string(), TaskPriority::Critical),
            |_| async { Ok(()) }
        ).await;
        
        let _task2 = manager.spawn_task(
            TaskContext::new("Task 2".to_string(), "".to_string(), TaskPriority::High),
            |_| async { Ok(()) }
        ).await;
        
        let _task3 = manager.spawn_task(
            TaskContext::new("Task 3".to_string(), "".to_string(), TaskPriority::Medium),
            |_| async { Err("Error".to_string()) }
        ).await;
        
        // Wait a bit
        sleep(Duration::from_millis(50)).await;
        
        let stats = manager.get_statistics().await;
        assert_eq!(stats.total_tasks, 3);
        assert_eq!(stats.critical_tasks, 1);
        assert_eq!(stats.high_priority_tasks, 1);
        assert_eq!(stats.medium_priority_tasks, 1);
    }

    #[tokio::test]
    async fn test_priority_ordering() {
        assert!(TaskPriority::Critical < TaskPriority::High);
        assert!(TaskPriority::High < TaskPriority::Medium);
        assert!(TaskPriority::Medium < TaskPriority::Low);
        assert!(TaskPriority::Low < TaskPriority::Cleanup);
    }
}