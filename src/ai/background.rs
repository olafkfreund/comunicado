//! Background processing system for AI operations
//! 
//! This module provides queuing, streaming, and non-blocking AI operations
//! to optimize performance and user experience.

use crate::ai::{AIResult, AIError, AIService};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock, Semaphore};
use tokio::time::timeout;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Configuration for background processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundConfig {
    /// Maximum number of concurrent AI operations
    pub max_concurrent_operations: usize,
    /// Queue size limit
    pub max_queue_size: usize,
    /// Default timeout for AI operations
    pub operation_timeout: Duration,
    /// Batch size for processing multiple items
    pub batch_size: usize,
    /// Interval for progress updates
    pub progress_update_interval: Duration,
    /// Enable streaming responses
    pub enable_streaming: bool,
    /// Maximum retry attempts for failed operations
    pub max_retries: usize,
}

impl Default for BackgroundConfig {
    fn default() -> Self {
        Self {
            max_concurrent_operations: 4,
            max_queue_size: 100,
            operation_timeout: Duration::from_secs(30),
            batch_size: 10,
            progress_update_interval: Duration::from_millis(500),
            enable_streaming: true,
            max_retries: 2,
        }
    }
}

/// Types of AI operations that can be queued
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AIOperationType {
    /// Email summarization
    EmailSummarization {
        email_id: String,
        content: String,
        max_length: Option<usize>,
    },
    /// Email reply suggestions
    EmailReply {
        email_id: String,
        content: String,
        context: String,
    },
    /// Email categorization
    EmailCategorization {
        email_id: String,
        content: String,
    },
    /// Calendar event parsing
    CalendarParsing {
        text: String,
        context: Option<String>,
    },
    /// Batch email processing
    BatchEmailProcessing {
        email_ids: Vec<String>,
        operation: String,
    },
    /// Custom AI operation
    Custom {
        operation_name: String,
        prompt: String,
        context: Option<String>,
    },
}

/// Priority levels for AI operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum OperationPriority {
    /// Low priority - background processing
    Low = 0,
    /// Normal priority - default
    Normal = 1,
    /// High priority - user-initiated
    High = 2,
    /// Critical priority - immediate response required
    Critical = 3,
}

/// Status of an AI operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationStatus {
    /// Queued and waiting to be processed
    Queued,
    /// Currently being processed
    Processing { progress: f32 },
    /// Completed successfully
    Completed { result: String },
    /// Failed with error
    Failed { error: String, retries_left: usize },
    /// Cancelled by user
    Cancelled,
    /// Timed out
    TimedOut,
}

/// A queued AI operation
#[derive(Debug, Clone)]
pub struct AIOperation {
    /// Unique operation ID
    pub id: Uuid,
    /// Type of operation
    pub operation_type: AIOperationType,
    /// Operation priority
    pub priority: OperationPriority,
    /// Creation timestamp
    pub created_at: Instant,
    /// Current status
    pub status: OperationStatus,
    /// Estimated duration (if known)
    pub estimated_duration: Option<Duration>,
    /// User-defined metadata
    pub metadata: HashMap<String, String>,
}

impl AIOperation {
    /// Create a new AI operation
    pub fn new(operation_type: AIOperationType, priority: OperationPriority) -> Self {
        Self {
            id: Uuid::new_v4(),
            operation_type,
            priority,
            created_at: Instant::now(),
            status: OperationStatus::Queued,
            estimated_duration: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a high-priority operation
    pub fn high_priority(operation_type: AIOperationType) -> Self {
        Self::new(operation_type, OperationPriority::High)
    }

    /// Create a critical operation
    pub fn critical(operation_type: AIOperationType) -> Self {
        Self::new(operation_type, OperationPriority::Critical)
    }

    /// Get the age of this operation
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }

    /// Check if this operation is time-sensitive
    pub fn is_time_sensitive(&self) -> bool {
        matches!(self.priority, OperationPriority::High | OperationPriority::Critical)
    }

    /// Add metadata to the operation
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Set estimated duration
    pub fn with_estimated_duration(mut self, duration: Duration) -> Self {
        self.estimated_duration = Some(duration);
        self
    }
}

/// Progress update for streaming operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressUpdate {
    /// Operation ID
    pub operation_id: Uuid,
    /// Progress percentage (0.0 to 1.0)
    pub progress: f32,
    /// Status message
    pub message: String,
    /// Partial results (for streaming)
    pub partial_result: Option<String>,
    /// Estimated time remaining
    pub eta: Option<Duration>,
}

/// Result of a background AI operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResult {
    /// Operation ID
    pub operation_id: Uuid,
    /// Final status
    pub status: OperationStatus,
    /// Processing duration
    pub duration: Duration,
    /// Result data (if successful)
    pub result: Option<String>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Performance metrics
    pub metrics: OperationMetrics,
}

/// Performance metrics for an operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationMetrics {
    /// Time spent queued
    pub queue_time: Duration,
    /// Actual processing time
    pub processing_time: Duration,
    /// Number of retry attempts
    pub retry_attempts: usize,
    /// Cache hit (if applicable)
    pub cache_hit: bool,
    /// Memory usage estimate
    pub memory_usage_mb: f32,
    /// Tokens processed (if applicable)
    pub tokens_processed: Option<usize>,
}

/// Background AI processor
pub struct AIBackgroundProcessor {
    /// Configuration
    config: Arc<RwLock<BackgroundConfig>>,
    /// AI service reference
    ai_service: Arc<AIService>,
    /// Operation queue (priority queue)
    operation_queue: Arc<RwLock<Vec<AIOperation>>>,
    /// Active operations
    active_operations: Arc<RwLock<HashMap<Uuid, AIOperation>>>,
    /// Semaphore for limiting concurrent operations
    concurrency_limiter: Arc<Semaphore>,
    /// Progress update sender
    progress_sender: Arc<RwLock<Option<mpsc::UnboundedSender<ProgressUpdate>>>>,
    /// Result sender
    result_sender: Arc<RwLock<Option<mpsc::UnboundedSender<OperationResult>>>>,
    /// Processing statistics
    stats: Arc<RwLock<ProcessingStats>>,
    /// Shutdown signal
    shutdown_signal: Arc<RwLock<bool>>,
}

/// Processing statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ProcessingStats {
    /// Total operations processed
    pub total_operations: usize,
    /// Successful operations
    pub successful_operations: usize,
    /// Failed operations
    pub failed_operations: usize,
    /// Cancelled operations
    pub cancelled_operations: usize,
    /// Average processing time
    pub avg_processing_time: Duration,
    /// Current queue size
    pub current_queue_size: usize,
    /// Active operations count
    pub active_operations_count: usize,
    /// Cache hit rate
    pub cache_hit_rate: f32,
    /// Operations by type
    pub operations_by_type: HashMap<String, usize>,
    /// Operations by priority
    pub operations_by_priority: HashMap<String, usize>,
}

impl AIBackgroundProcessor {
    /// Create a new background processor
    pub fn new(ai_service: Arc<AIService>, config: BackgroundConfig) -> Self {
        let concurrency_limiter = Arc::new(Semaphore::new(config.max_concurrent_operations));
        
        Self {
            config: Arc::new(RwLock::new(config)),
            ai_service,
            operation_queue: Arc::new(RwLock::new(Vec::new())),
            active_operations: Arc::new(RwLock::new(HashMap::new())),
            concurrency_limiter,
            progress_sender: Arc::new(RwLock::new(None)),
            result_sender: Arc::new(RwLock::new(None)),
            stats: Arc::new(RwLock::new(ProcessingStats::default())),
            shutdown_signal: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the background processor
    pub async fn start(&self) -> (mpsc::UnboundedReceiver<ProgressUpdate>, mpsc::UnboundedReceiver<OperationResult>) {
        let (progress_tx, progress_rx) = mpsc::unbounded_channel();
        let (result_tx, result_rx) = mpsc::unbounded_channel();

        {
            let mut progress_sender = self.progress_sender.write().await;
            *progress_sender = Some(progress_tx);
        }

        {
            let mut result_sender = self.result_sender.write().await;
            *result_sender = Some(result_tx);
        }

        // Start the main processing loop
        self.start_processing_loop().await;

        // Start periodic statistics updates
        self.start_stats_updater().await;

        info!("AI background processor started");

        (progress_rx, result_rx)
    }

    /// Stop the background processor
    pub async fn stop(&self) {
        let mut shutdown = self.shutdown_signal.write().await;
        *shutdown = true;
        info!("AI background processor shutdown initiated");
    }

    /// Queue an AI operation
    pub async fn queue_operation(&self, mut operation: AIOperation) -> AIResult<Uuid> {
        let config = self.config.read().await;
        let mut queue = self.operation_queue.write().await;

        // Check queue size limits
        if queue.len() >= config.max_queue_size {
            return Err(AIError::internal_error("AI operation queue is full"));
        }

        operation.status = OperationStatus::Queued;
        let operation_id = operation.id;

        // Insert operation maintaining priority order
        let insert_pos = queue
            .iter()
            .position(|op| op.priority <= operation.priority)
            .unwrap_or(queue.len());

        queue.insert(insert_pos, operation);

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.current_queue_size = queue.len();
        stats.total_operations += 1;

        // Update operation type statistics
        let op_type = self.get_operation_type_name(&queue[insert_pos].operation_type);
        *stats.operations_by_type.entry(op_type).or_insert(0) += 1;

        // Update priority statistics
        let priority_name = format!("{:?}", queue[insert_pos].priority);
        *stats.operations_by_priority.entry(priority_name).or_insert(0) += 1;

        debug!("Queued AI operation {} with priority {:?}", operation_id, queue[insert_pos].priority);

        Ok(operation_id)
    }

    /// Cancel an operation
    pub async fn cancel_operation(&self, operation_id: Uuid) -> AIResult<bool> {
        // Try to remove from queue first
        {
            let mut queue = self.operation_queue.write().await;
            if let Some(pos) = queue.iter().position(|op| op.id == operation_id) {
                queue.remove(pos);
                let mut stats = self.stats.write().await;
                stats.cancelled_operations += 1;
                stats.current_queue_size = queue.len();
                return Ok(true);
            }
        }

        // Check if it's currently being processed
        {
            let mut active = self.active_operations.write().await;
            if let Some(operation) = active.get_mut(&operation_id) {
                operation.status = OperationStatus::Cancelled;
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Get operation status
    pub async fn get_operation_status(&self, operation_id: Uuid) -> Option<OperationStatus> {
        // Check active operations first
        {
            let active = self.active_operations.read().await;
            if let Some(operation) = active.get(&operation_id) {
                return Some(operation.status.clone());
            }
        }

        // Check queue
        {
            let queue = self.operation_queue.read().await;
            if let Some(operation) = queue.iter().find(|op| op.id == operation_id) {
                return Some(operation.status.clone());
            }
        }

        None
    }

    /// Get current processing statistics
    pub async fn get_stats(&self) -> ProcessingStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Start the main processing loop
    async fn start_processing_loop(&self) {
        let processor = Arc::new(self.clone());
        
        tokio::spawn(async move {
            loop {
                // Check shutdown signal
                if *processor.shutdown_signal.read().await {
                    break;
                }

                // Get next operation from queue
                let operation = {
                    let mut queue = processor.operation_queue.write().await;
                    if queue.is_empty() {
                        None
                    } else {
                        // Get highest priority operation (queue is sorted by priority)
                        Some(queue.remove(0))
                    }
                };

                if let Some(operation) = operation {
                    // Update queue size statistics
                    {
                        let mut stats = processor.stats.write().await;
                        stats.current_queue_size = processor.operation_queue.read().await.len();
                    }

                    // Acquire concurrency permit
                    let permit = processor.concurrency_limiter.clone().acquire_owned().await;
                    
                    if permit.is_ok() {
                        let processor_clone = processor.clone();
                        
                        // Process operation in background
                        tokio::spawn(async move {
                            processor_clone.process_operation(operation).await;
                            // Permit is automatically released when dropped
                        });
                    }
                } else {
                    // No operations in queue, sleep briefly
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }

            info!("AI background processor stopped");
        });
    }

    /// Start periodic statistics updater
    async fn start_stats_updater(&self) {
        let processor = Arc::new(self.clone());
        
        tokio::spawn(async move {
            loop {
                if *processor.shutdown_signal.read().await {
                    break;
                }

                // Update active operations count
                {
                    let mut stats = processor.stats.write().await;
                    stats.active_operations_count = processor.active_operations.read().await.len();
                }

                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });
    }

    /// Process a single operation
    async fn process_operation(&self, mut operation: AIOperation) {
        let start_time = Instant::now();
        let queue_time = operation.age();

        // Move to active operations
        {
            let mut active = self.active_operations.write().await;
            operation.status = OperationStatus::Processing { progress: 0.0 };
            active.insert(operation.id, operation.clone());
        }

        // Send initial progress update
        self.send_progress_update(ProgressUpdate {
            operation_id: operation.id,
            progress: 0.0,
            message: "Starting operation...".to_string(),
            partial_result: None,
            eta: operation.estimated_duration,
        }).await;

        let config = self.config.read().await.clone();
        let result = self.execute_operation_with_timeout(&operation, &config).await;

        let processing_time = start_time.elapsed();

        // Remove from active operations
        {
            let mut active = self.active_operations.write().await;
            active.remove(&operation.id);
        }

        // Create result
        let operation_result = OperationResult {
            operation_id: operation.id,
            status: match &result {
                Ok(result) => OperationStatus::Completed { result: result.clone() },
                Err(error) => OperationStatus::Failed { 
                    error: error.to_string(), 
                    retries_left: 0 
                },
            },
            duration: processing_time,
            result: result.as_ref().ok().cloned(),
            error: result.as_ref().err().map(|e| e.to_string()),
            metrics: OperationMetrics {
                queue_time,
                processing_time,
                retry_attempts: 0,
                cache_hit: false, // Could be enhanced to check cache
                memory_usage_mb: 0.0, // Could be enhanced with actual memory tracking
                tokens_processed: None,
            },
        };

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            if result.is_ok() {
                stats.successful_operations += 1;
            } else {
                stats.failed_operations += 1;
            }
            
            // Update average processing time
            let total_completed = stats.successful_operations + stats.failed_operations;
            if total_completed > 1 {
                stats.avg_processing_time = Duration::from_millis(
                    ((stats.avg_processing_time.as_millis() * (total_completed - 1) as u128 
                     + processing_time.as_millis()) / total_completed as u128)
                     .try_into().unwrap_or(0)
                );
            } else {
                stats.avg_processing_time = processing_time;
            }
        }

        // Send final result
        self.send_result(operation_result).await;
    }

    /// Execute operation with timeout
    async fn execute_operation_with_timeout(
        &self, 
        operation: &AIOperation, 
        config: &BackgroundConfig
    ) -> AIResult<String> {
        let operation_future = self.execute_operation(operation, config);
        
        match timeout(config.operation_timeout, operation_future).await {
            Ok(result) => result,
            Err(_) => {
                warn!("AI operation {} timed out after {:?}", operation.id, config.operation_timeout);
                Err(AIError::timeout(config.operation_timeout))
            }
        }
    }

    /// Execute the actual AI operation
    async fn execute_operation(&self, operation: &AIOperation, config: &BackgroundConfig) -> AIResult<String> {
        match &operation.operation_type {
            AIOperationType::EmailSummarization { content, max_length, .. } => {
                self.ai_service.summarize_email(content, *max_length).await
            }
            
            AIOperationType::EmailReply { content, context, .. } => {
                let suggestions = self.ai_service.suggest_email_reply(content, context).await?;
                Ok(suggestions.join("\n"))
            }
            
            AIOperationType::EmailCategorization { content, .. } => {
                let category = self.ai_service.categorize_email(content).await?;
                Ok(category.to_string())
            }
            
            AIOperationType::CalendarParsing { text, context: _ } => {
                let intent = self.ai_service.parse_scheduling_intent(text).await?;
                serde_json::to_string(&intent)
                    .map_err(|e| AIError::internal_error(format!("Failed to serialize scheduling intent: {}", e)))
            }
            
            AIOperationType::BatchEmailProcessing { email_ids, operation } => {
                self.process_batch_emails(email_ids, operation, config).await
            }
            
            AIOperationType::Custom { prompt, context, .. } => {
                let ai_context = context.as_ref().map(|c| crate::ai::AIContext {
                    user_preferences: std::collections::HashMap::new(),
                    email_thread: Some(c.clone()),
                    calendar_context: None,
                    max_length: None,
                    creativity: Some(0.7),
                });
                
                self.ai_service.complete_text(prompt, ai_context.as_ref()).await
            }
        }
    }

    /// Process multiple emails in batch
    async fn process_batch_emails(
        &self,
        email_ids: &[String],
        operation: &str,
        _config: &BackgroundConfig,
    ) -> AIResult<String> {
        let mut results = Vec::new();
        let total_emails = email_ids.len();

        for (index, email_id) in email_ids.iter().enumerate() {
            // Update progress
            let _progress = index as f32 / total_emails as f32;
            // Note: In a real implementation, you'd send progress updates here
            
            // Simulate email processing based on operation type
            let result = match operation {
                "summarize" => format!("Summary for email {}", email_id),
                "categorize" => format!("Category for email {}", email_id),
                _ => format!("Processed email {} with operation {}", email_id, operation),
            };
            
            results.push(result);

            // Add small delay to prevent overwhelming the AI service
            if index < total_emails - 1 {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }

        Ok(results.join("\n"))
    }

    /// Send progress update
    async fn send_progress_update(&self, update: ProgressUpdate) {
        if let Some(sender) = self.progress_sender.read().await.as_ref() {
            if sender.send(update).is_err() {
                warn!("Failed to send progress update");
            }
        }
    }

    /// Send operation result
    async fn send_result(&self, result: OperationResult) {
        if let Some(sender) = self.result_sender.read().await.as_ref() {
            if sender.send(result).is_err() {
                warn!("Failed to send operation result");
            }
        }
    }

    /// Get operation type name for statistics
    fn get_operation_type_name(&self, op_type: &AIOperationType) -> String {
        match op_type {
            AIOperationType::EmailSummarization { .. } => "EmailSummarization".to_string(),
            AIOperationType::EmailReply { .. } => "EmailReply".to_string(),
            AIOperationType::EmailCategorization { .. } => "EmailCategorization".to_string(),
            AIOperationType::CalendarParsing { .. } => "CalendarParsing".to_string(),
            AIOperationType::BatchEmailProcessing { .. } => "BatchEmailProcessing".to_string(),
            AIOperationType::Custom { operation_name, .. } => operation_name.clone(),
        }
    }
}

// Clone implementation for Arc sharing
impl Clone for AIBackgroundProcessor {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            ai_service: self.ai_service.clone(),
            operation_queue: self.operation_queue.clone(),
            active_operations: self.active_operations.clone(),
            concurrency_limiter: self.concurrency_limiter.clone(),
            progress_sender: self.progress_sender.clone(),
            result_sender: self.result_sender.clone(),
            stats: self.stats.clone(),
            shutdown_signal: self.shutdown_signal.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_operation_creation() {
        let operation = AIOperation::new(
            AIOperationType::EmailSummarization {
                email_id: "test".to_string(),
                content: "Test email content".to_string(),
                max_length: Some(100),
            },
            OperationPriority::Normal,
        );

        assert!(matches!(operation.status, OperationStatus::Queued));
        assert_eq!(operation.priority, OperationPriority::Normal);
        assert!(operation.age() < Duration::from_millis(10));
    }

    #[test]
    fn test_operation_priority_ordering() {
        let low = OperationPriority::Low;
        let normal = OperationPriority::Normal;
        let high = OperationPriority::High;
        let critical = OperationPriority::Critical;

        assert!(low < normal);
        assert!(normal < high);
        assert!(high < critical);
    }

    #[test]
    fn test_background_config_defaults() {
        let config = BackgroundConfig::default();
        
        assert_eq!(config.max_concurrent_operations, 4);
        assert_eq!(config.max_queue_size, 100);
        assert_eq!(config.operation_timeout, Duration::from_secs(30));
        assert_eq!(config.batch_size, 10);
        assert!(config.enable_streaming);
    }

    #[test]
    fn test_operation_metadata() {
        let operation = AIOperation::new(
            AIOperationType::Custom {
                operation_name: "test".to_string(),
                prompt: "test prompt".to_string(),
                context: None,
            },
            OperationPriority::Normal,
        )
        .with_metadata("user_id".to_string(), "123".to_string())
        .with_estimated_duration(Duration::from_secs(5));

        assert_eq!(operation.metadata.get("user_id"), Some(&"123".to_string()));
        assert_eq!(operation.estimated_duration, Some(Duration::from_secs(5)));
    }
}