//! Enhanced AI service with background processing and streaming capabilities

use crate::ai::{
    AIService, AIBackgroundProcessor, AIStreamingManager, AIOperation, AIOperationType,
    BackgroundConfig, StreamingConfig, OperationPriority, OperationResult, ProgressUpdate,
    StreamChunk, AIResult, AIError
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Configuration for the enhanced AI service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedAIConfig {
    /// Background processing configuration
    pub background_config: BackgroundConfig,
    /// Streaming configuration
    pub streaming_config: StreamingConfig,
    /// Enable automatic cache warming
    pub enable_cache_warming: bool,
    /// Enable performance monitoring
    pub enable_performance_monitoring: bool,
    /// Batch processing preferences
    pub batch_processing_enabled: bool,
    /// Default operation timeout
    pub default_timeout: Duration,
}

impl Default for EnhancedAIConfig {
    fn default() -> Self {
        Self {
            background_config: BackgroundConfig::default(),
            streaming_config: StreamingConfig::default(),
            enable_cache_warming: true,
            enable_performance_monitoring: true,
            batch_processing_enabled: true,
            default_timeout: Duration::from_secs(30),
        }
    }
}

/// Enhanced AI service with background processing and streaming
pub struct EnhancedAIService {
    /// Base AI service
    ai_service: Arc<AIService>,
    /// Background processor
    background_processor: Arc<AIBackgroundProcessor>,
    /// Streaming manager
    streaming_manager: Arc<AIStreamingManager>,
    /// Configuration
    config: Arc<RwLock<EnhancedAIConfig>>,
    /// Progress update receiver
    progress_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<ProgressUpdate>>>>,
    /// Result receiver
    result_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<OperationResult>>>>,
    /// Performance metrics
    metrics: Arc<RwLock<PerformanceMetrics>>,
}

/// Performance metrics for the enhanced AI service
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Total operations processed
    pub total_operations: usize,
    /// Operations processed per minute
    pub operations_per_minute: f64,
    /// Average response time
    pub avg_response_time: Duration,
    /// Cache hit rate
    pub cache_hit_rate: f64,
    /// Background processing utilization
    pub background_utilization: f64,
    /// Active streaming sessions
    pub active_streaming_sessions: usize,
    /// Memory usage estimate (MB)
    pub memory_usage_mb: f64,
    /// Error rate (percentage)
    pub error_rate: f64,
}

/// AI operation request with enhanced options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedAIRequest {
    /// Operation type
    pub operation_type: AIOperationType,
    /// Priority level
    pub priority: OperationPriority,
    /// Enable streaming for this operation
    pub enable_streaming: bool,
    /// Enable background processing
    pub enable_background: bool,
    /// Custom timeout
    pub timeout: Option<Duration>,
    /// Request metadata
    pub metadata: std::collections::HashMap<String, String>,
}

impl EnhancedAIRequest {
    /// Create a new AI request
    pub fn new(operation_type: AIOperationType) -> Self {
        Self {
            operation_type,
            priority: OperationPriority::Normal,
            enable_streaming: false,
            enable_background: true,
            timeout: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Create a high-priority request
    pub fn high_priority(operation_type: AIOperationType) -> Self {
        Self {
            operation_type,
            priority: OperationPriority::High,
            enable_streaming: true,
            enable_background: false, // High priority should be immediate
            timeout: Some(Duration::from_secs(10)),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Create a background request
    pub fn background(operation_type: AIOperationType) -> Self {
        Self {
            operation_type,
            priority: OperationPriority::Low,
            enable_streaming: false,
            enable_background: true,
            timeout: Some(Duration::from_secs(60)),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Create a streaming request
    pub fn streaming(operation_type: AIOperationType) -> Self {
        Self {
            operation_type,
            priority: OperationPriority::Normal,
            enable_streaming: true,
            enable_background: true,
            timeout: Some(Duration::from_secs(30)),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Add metadata to the request
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Set custom timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
}

/// Response from an enhanced AI operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedAIResponse {
    /// Operation ID
    pub operation_id: Uuid,
    /// Response content
    pub content: String,
    /// Whether response was served from cache
    pub from_cache: bool,
    /// Processing time
    pub processing_time: Duration,
    /// Whether response was streamed
    pub was_streamed: bool,
    /// Response metadata
    pub metadata: std::collections::HashMap<String, String>,
}

impl EnhancedAIService {
    /// Create a new enhanced AI service
    pub async fn new(
        ai_service: Arc<AIService>,
        config: EnhancedAIConfig,
    ) -> AIResult<Self> {
        // Create background processor
        let background_processor = Arc::new(AIBackgroundProcessor::new(
            ai_service.clone(),
            config.background_config.clone(),
        ));

        // Create streaming manager
        let streaming_manager = Arc::new(AIStreamingManager::new(
            config.streaming_config.clone(),
        ));

        // Start background processor
        let (progress_rx, result_rx) = background_processor.start().await;

        // Start streaming cleanup task
        streaming_manager.start_cleanup_task().await;

        let service = Self {
            ai_service,
            background_processor,
            streaming_manager,
            config: Arc::new(RwLock::new(config)),
            progress_receiver: Arc::new(RwLock::new(Some(progress_rx))),
            result_receiver: Arc::new(RwLock::new(Some(result_rx))),
            metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
        };

        // Start background tasks
        service.start_background_tasks().await;

        info!("Enhanced AI service initialized successfully");

        Ok(service)
    }

    /// Process an AI request
    pub async fn process_request(
        &self,
        request: EnhancedAIRequest,
    ) -> AIResult<EnhancedAIResponse> {
        let operation_id = Uuid::new_v4();
        let start_time = std::time::Instant::now();

        debug!("Processing AI request {} with priority {:?}", operation_id, request.priority);

        // Create AI operation
        let mut operation = AIOperation::new(request.operation_type.clone(), request.priority);
        
        // Add metadata
        for (key, value) in &request.metadata {
            operation = operation.with_metadata(key.clone(), value.clone());
        }

        // Set timeout if specified
        if let Some(timeout) = request.timeout {
            operation = operation.with_estimated_duration(timeout);
        }

        let result = if request.enable_background {
            // Process in background
            self.process_background_request(operation, request.enable_streaming).await
        } else {
            // Process immediately
            self.process_immediate_request(&request).await
        };

        let processing_time = start_time.elapsed();

        // Update metrics
        self.update_metrics(processing_time, result.is_ok()).await;

        match result {
            Ok(content) => {
                Ok(EnhancedAIResponse {
                    operation_id,
                    content,
                    from_cache: false, // Could be enhanced to check cache
                    processing_time,
                    was_streamed: request.enable_streaming,
                    metadata: request.metadata,
                })
            }
            Err(error) => Err(error),
        }
    }

    /// Process request in background
    async fn process_background_request(
        &self,
        operation: AIOperation,
        enable_streaming: bool,
    ) -> AIResult<String> {
        let operation_id = operation.id;

        // Start streaming if enabled
        let stream_rx = if enable_streaming {
            Some(self.streaming_manager.start_stream(operation_id).await?)
        } else {
            None
        };

        // Queue the operation
        self.background_processor.queue_operation(operation).await?;

        // Wait for result
        let mut result_content = String::new();
        
        if let Some(mut rx) = stream_rx {
            // Collect streaming chunks
            let mut accumulated_content = String::new();
            
            while let Some(chunk) = rx.recv().await {
                accumulated_content.push_str(&chunk.content);
                
                if chunk.is_final {
                    result_content = accumulated_content;
                    break;
                }
            }
        } else {
            // Wait for operation to complete via result receiver
            // This is a simplified implementation - in practice you'd match operation IDs
            result_content = "Background operation completed".to_string();
        }

        Ok(result_content)
    }

    /// Process request immediately
    async fn process_immediate_request(&self, request: &EnhancedAIRequest) -> AIResult<String> {
        match &request.operation_type {
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
            
            AIOperationType::CalendarParsing { text, .. } => {
                let intent = self.ai_service.parse_scheduling_intent(text).await?;
                serde_json::to_string(&intent)
                    .map_err(|e| AIError::internal_error(format!("Serialization failed: {}", e)))
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
            
            AIOperationType::BatchEmailProcessing { .. } => {
                Err(AIError::internal_error("Batch processing requires background mode"))
            }
        }
    }

    /// Start background tasks for monitoring and management
    async fn start_background_tasks(&self) {
        // Start progress monitoring task
        self.start_progress_monitor().await;
        
        // Start result processing task
        self.start_result_processor().await;
        
        // Start performance monitoring task
        self.start_performance_monitor().await;
    }

    /// Start progress monitoring task
    async fn start_progress_monitor(&self) {
        let progress_receiver = self.progress_receiver.clone();
        let streaming_manager = self.streaming_manager.clone();
        
        tokio::spawn(async move {
            let rx = {
                let mut receiver_guard = progress_receiver.write().await;
                receiver_guard.take()
            };
            
            if let Some(mut receiver) = rx {
                while let Some(progress) = receiver.recv().await {
                    debug!("Progress update for operation {}: {:.1}% - {}", 
                           progress.operation_id, progress.progress * 100.0, progress.message);
                    
                    // Convert progress to streaming chunk if applicable
                    if let Some(partial_result) = &progress.partial_result {
                        let chunk = StreamChunk {
                            operation_id: progress.operation_id,
                            sequence: 0, // Would need proper sequence tracking
                            content: partial_result.clone(),
                            is_final: progress.progress >= 1.0,
                            timestamp: std::time::SystemTime::now(),
                            metadata: std::collections::HashMap::new(),
                        };
                        
                        if let Err(e) = streaming_manager.send_chunk(chunk).await {
                            warn!("Failed to send streaming chunk: {}", e);
                        }
                    }
                }
            }
        });
    }

    /// Start result processing task
    async fn start_result_processor(&self) {
        let result_receiver = self.result_receiver.clone();
        let metrics = self.metrics.clone();
        
        tokio::spawn(async move {
            let rx = {
                let mut receiver_guard = result_receiver.write().await;
                receiver_guard.take()
            };
            
            if let Some(mut receiver) = rx {
                while let Some(result) = receiver.recv().await {
                    debug!("Operation {} completed in {:?}", 
                           result.operation_id, result.duration);
                    
                    // Update metrics
                    let mut metrics_guard = metrics.write().await;
                    metrics_guard.total_operations += 1;
                    
                    // Update average response time
                    if metrics_guard.total_operations > 1 {
                        let total = metrics_guard.total_operations as u64;
                        metrics_guard.avg_response_time = Duration::from_millis(
                            ((metrics_guard.avg_response_time.as_millis() * (total - 1) as u128
                             + result.duration.as_millis()) / total as u128)
                             .try_into().unwrap_or(0)
                        );
                    } else {
                        metrics_guard.avg_response_time = result.duration;
                    }
                }
            }
        });
    }

    /// Start performance monitoring task
    async fn start_performance_monitor(&self) {
        let config = self.config.clone();
        let metrics = self.metrics.clone();
        let background_processor = self.background_processor.clone();
        let streaming_manager = self.streaming_manager.clone();
        
        tokio::spawn(async move {
            loop {
                let config_guard = config.read().await;
                if !config_guard.enable_performance_monitoring {
                    drop(config_guard);
                    tokio::time::sleep(Duration::from_secs(60)).await;
                    continue;
                }
                drop(config_guard);

                // Update performance metrics
                let mut metrics_guard = metrics.write().await;
                
                // Get background processor stats
                let bg_stats = background_processor.get_stats().await;
                metrics_guard.background_utilization = 
                    bg_stats.active_operations_count as f64 / 4.0; // Assuming max 4 concurrent
                
                // Get streaming stats
                let streaming_stats = streaming_manager.get_stats().await;
                metrics_guard.active_streaming_sessions = streaming_stats.active_streams;
                
                // Calculate operations per minute
                if metrics_guard.total_operations > 0 {
                    // This is a simplified calculation - would need proper time tracking
                    metrics_guard.operations_per_minute = metrics_guard.total_operations as f64;
                }
                
                drop(metrics_guard);
                
                tokio::time::sleep(Duration::from_secs(30)).await;
            }
        });
    }

    /// Update performance metrics
    async fn update_metrics(&self, _processing_time: Duration, success: bool) {
        let mut metrics = self.metrics.write().await;
        metrics.total_operations += 1;
        
        if !success {
            // Update error rate
            let error_count = (metrics.error_rate * (metrics.total_operations - 1) as f64 / 100.0) + 1.0;
            metrics.error_rate = (error_count / metrics.total_operations as f64) * 100.0;
        }
    }

    /// Get performance metrics
    pub async fn get_performance_metrics(&self) -> PerformanceMetrics {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }

    /// Get background processing statistics
    pub async fn get_background_stats(&self) -> crate::ai::ProcessingStats {
        self.background_processor.get_stats().await
    }

    /// Get streaming statistics
    pub async fn get_streaming_stats(&self) -> crate::ai::StreamingStats {
        self.streaming_manager.get_stats().await
    }

    /// Cancel a background operation
    pub async fn cancel_operation(&self, operation_id: Uuid) -> AIResult<bool> {
        self.background_processor.cancel_operation(operation_id).await
    }

    /// Get operation status
    pub async fn get_operation_status(&self, operation_id: Uuid) -> Option<crate::ai::OperationStatus> {
        self.background_processor.get_operation_status(operation_id).await
    }

    /// Shutdown the enhanced AI service
    pub async fn shutdown(&self) {
        info!("Shutting down enhanced AI service");
        self.background_processor.stop().await;
        // Streaming manager cleanup happens automatically through Drop
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::{AIService, AIResponseCache, AIConfig, AIProviderManager, AIProviderType};

    async fn create_test_enhanced_service() -> EnhancedAIService {
        // This is a simplified test setup
        // In practice, you'd need to properly initialize the AI service with providers
        
        let config = AIConfig {
            enabled: true,
            provider: AIProviderType::None, // Mock provider for testing
            ..Default::default()
        };
        
        let provider_manager = Arc::new(tokio::sync::RwLock::new(
            AIProviderManager::new(Arc::new(tokio::sync::RwLock::new(config.clone())))
        ));
        
        let cache = Arc::new(AIResponseCache::default());
        let ai_config = Arc::new(tokio::sync::RwLock::new(config));
        
        let ai_service = Arc::new(AIService::new(provider_manager, cache, ai_config));
        let enhanced_config = EnhancedAIConfig::default();
        
        EnhancedAIService::new(ai_service, enhanced_config).await.unwrap()
    }

    #[tokio::test]
    async fn test_enhanced_ai_request_creation() {
        let request = EnhancedAIRequest::new(AIOperationType::Custom {
            operation_name: "test".to_string(),
            prompt: "Test prompt".to_string(),
            context: None,
        });
        
        assert_eq!(request.priority, OperationPriority::Normal);
        assert!(!request.enable_streaming);
        assert!(request.enable_background);
    }

    #[tokio::test]
    async fn test_high_priority_request() {
        let request = EnhancedAIRequest::high_priority(AIOperationType::Custom {
            operation_name: "urgent".to_string(),
            prompt: "Urgent prompt".to_string(),
            context: None,
        });
        
        assert_eq!(request.priority, OperationPriority::High);
        assert!(request.enable_streaming);
        assert!(!request.enable_background); // High priority should be immediate
    }

    #[tokio::test]
    async fn test_enhanced_service_initialization() {
        let service = create_test_enhanced_service().await;
        let metrics = service.get_performance_metrics().await;
        
        assert_eq!(metrics.total_operations, 0);
        assert_eq!(metrics.active_streaming_sessions, 0);
    }
}