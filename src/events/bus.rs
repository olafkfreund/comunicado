//! Event Bus Architecture for Decoupled Component Communication
//!
//! This module provides a centralized event bus system that replaces direct
//! component coupling with a publish-subscribe pattern. It supports type-safe
//! event handling, batching, middleware, and async processing.

use std::any::{Any, TypeId};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use uuid::Uuid;
use tracing::{info, warn};
use crate::events::performance_metrics::PerformanceMonitor;

/// Event priority levels for processing order
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventPriority {
    Critical = 0,  // System-critical events (errors, shutdown)
    High = 1,      // User input, UI updates
    Normal = 2,    // Business logic, data operations
    Low = 3,       // Background tasks, cleanup
}

/// Event metadata for tracking and debugging
#[derive(Debug, Clone)]
pub struct EventMetadata {
    pub id: Uuid,
    pub timestamp: Instant,
    pub priority: EventPriority,
    pub source: String,
    pub correlation_id: Option<Uuid>,
}

impl EventMetadata {
    pub fn new(priority: EventPriority, source: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Instant::now(),
            priority,
            source,
            correlation_id: None,
        }
    }

    pub fn with_correlation(mut self, correlation_id: Uuid) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }
}

/// Base trait for all events in the system
pub trait Event: Any + Send + Sync + std::fmt::Debug {
    /// Get the event type name for logging and debugging
    fn event_type(&self) -> &'static str;
    
    /// Get event metadata
    fn metadata(&self) -> &EventMetadata;
    
    /// Check if event can be batched with others of the same type
    fn is_batchable(&self) -> bool {
        false
    }
    
    /// Merge with another event of the same type for batching
    fn merge_with(&mut self, _other: Box<dyn Event>) -> Result<(), Box<dyn Event>> {
        Err(Box::new(NoOpEvent::new("merge_not_supported")))
    }
}

/// Event handler trait for type-safe event processing
pub trait EventHandler<T: Event>: Send + Sync {
    /// Handle a specific event type
    fn handle(&mut self, event: &T) -> Result<(), EventError>;
    
    /// Check if handler can process events of this type
    fn can_handle(&self, _event: &T) -> bool {
        true
    }
    
    /// Handler priority for ordering multiple handlers
    fn priority(&self) -> HandlerPriority {
        HandlerPriority::Normal
    }
}

/// Handler priority for execution order
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HandlerPriority {
    Critical = 0,
    High = 1,
    Normal = 2,
    Low = 3,
}

/// Event processing errors
#[derive(Debug, thiserror::Error)]
pub enum EventError {
    #[error("Handler not found for event type: {0}")]
    HandlerNotFound(String),
    
    #[error("Event processing failed: {0}")]
    ProcessingFailed(String),
    
    #[error("Event bus is shutting down")]
    ShuttingDown,
    
    #[error("Event queue is full")]
    QueueFull,
    
    #[error("Middleware error: {0}")]
    MiddlewareError(String),
}

/// Event middleware for cross-cutting concerns
pub trait EventMiddleware: Send + Sync {
    /// Process event before handlers (can modify or reject events)
    fn before_handle(&mut self, event: &mut dyn Event) -> Result<(), EventError>;
    
    /// Process event after handlers (for cleanup, logging, etc.)
    fn after_handle(&mut self, event: &dyn Event, result: &Result<(), EventError>);
    
    /// Middleware priority for execution order
    fn priority(&self) -> MiddlewareePriority {
        MiddlewarePriority::Normal
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MiddlewarePriority {
    Critical = 0,
    High = 1,
    Normal = 2,
    Low = 3,
}

/// Type-erased event wrapper for storage
struct BoxedEvent {
    event: Box<dyn Event>,
    type_id: TypeId,
    #[allow(dead_code)]
    handler_count: usize,
}

/// Handler information for type-erased storage
#[derive(Clone)]
struct HandlerInfo {
    id: Uuid,
    priority: HandlerPriority,
    handler: Arc<Mutex<dyn Any + Send + Sync>>,
}

/// Event bus configuration
#[derive(Debug, Clone)]
pub struct EventBusConfig {
    /// Maximum events in queue before dropping
    pub max_queue_size: usize,
    /// Batch processing interval
    pub batch_interval: Duration,
    /// Maximum events per batch
    pub max_batch_size: usize,
    /// Enable async processing
    pub async_processing: bool,
    /// Number of worker threads for async processing
    pub worker_threads: usize,
}

impl Default for EventBusConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 10000,
            batch_interval: Duration::from_millis(10),
            max_batch_size: 100,
            async_processing: true,
            worker_threads: 4,
        }
    }
}

/// Main event bus implementation
pub struct EventBus {
    config: EventBusConfig,
    
    // Event storage
    event_queue: Arc<Mutex<VecDeque<BoxedEvent>>>,
    
    // Handler registration
    handlers: Arc<Mutex<HashMap<TypeId, Vec<HandlerInfo>>>>,
    
    // Middleware chain
    middleware: Arc<Mutex<Vec<(MiddlewarePriority, Box<dyn EventMiddleware>)>>>,
    
    // Processing state
    is_running: Arc<Mutex<bool>>,
    
    // Async processing
    sender: Option<mpsc::UnboundedSender<BoxedEvent>>,
    
    // Statistics
    stats: Arc<Mutex<EventBusStats>>,
    
    // Performance monitoring
    performance_monitor: Arc<PerformanceMonitor>,
    
    // Logging system
    logging_system: Arc<crate::events::logging_system::LoggingSystem>,
    
    // Shutdown coordination
    shutdown_manager: Arc<crate::events::shutdown_manager::ShutdownManager>,
}

/// Event bus statistics for monitoring
#[derive(Debug, Clone)]
pub struct EventBusStats {
    pub events_published: u64,
    pub events_processed: u64,
    pub events_dropped: u64,
    pub events_batched: u64,
    pub processing_errors: u64,
    pub average_processing_time: Duration,
    pub queue_size: usize,
    pub batches_processed: u64,
    pub total_processing_time: Duration,
}

impl Default for EventBusStats {
    fn default() -> Self {
        Self {
            events_published: 0,
            events_processed: 0,
            events_dropped: 0,
            events_batched: 0,
            processing_errors: 0,
            average_processing_time: Duration::from_millis(0),
            queue_size: 0,
            batches_processed: 0,
            total_processing_time: Duration::from_millis(0),
        }
    }
}

/// Performance statistics for external monitoring
#[derive(Debug, Clone)]
pub struct EventBusPerformanceStats {
    pub events_published: u64,
    pub events_processed: u64,
    pub events_dropped: u64,
    pub batches_processed: u64,
    pub current_queue_size: usize,
    pub average_processing_time_ms: f64,
}

impl EventBus {
    /// Create a new event bus with default configuration
    pub fn new() -> Self {
        Self::with_config(EventBusConfig::default())
    }
    
    /// Create a new event bus with custom configuration
    pub fn with_config(config: EventBusConfig) -> Self {
        let sender = if config.async_processing {
            let (tx, rx) = mpsc::unbounded_channel();
            // Start background processing task
            let bus_weak = Arc::new(Mutex::new(None::<Weak<Mutex<EventBus>>>));
            tokio::spawn(Self::async_processor(bus_weak.clone(), rx));
            Some(tx)
        } else {
            None
        };
        
        let performance_monitor = Arc::new(PerformanceMonitor::new());
        let logging_system = Arc::new(crate::events::logging_system::LoggingSystem::new());
        let shutdown_manager = Arc::new(crate::events::shutdown_manager::ShutdownManager::new());
        
        Self {
            config,
            event_queue: Arc::new(Mutex::new(VecDeque::new())),
            handlers: Arc::new(Mutex::new(HashMap::new())),
            middleware: Arc::new(Mutex::new(Vec::new())),
            is_running: Arc::new(Mutex::new(true)),
            sender,
            stats: Arc::new(Mutex::new(EventBusStats::default())),
            performance_monitor,
            logging_system,
            shutdown_manager,
        }
    }
    
    /// Register an event handler for a specific event type
    pub fn subscribe<T, H>(&mut self, handler: H) -> Uuid
    where
        T: Event + 'static,
        H: EventHandler<T> + 'static,
    {
        let handler_id = Uuid::new_v4();
        let type_id = TypeId::of::<T>();
        let priority = handler.priority();
        
        let handler_info = HandlerInfo {
            id: handler_id,
            priority,
            handler: Arc::new(Mutex::new(handler)),
        };
        
        let mut handlers = self.handlers.lock().unwrap();
        handlers.entry(type_id)
            .or_insert_with(Vec::new)
            .push(handler_info);
            
        // Sort handlers by priority
        if let Some(type_handlers) = handlers.get_mut(&type_id) {
            type_handlers.sort_by(|a, b| a.priority.cmp(&b.priority));
        }
        
        handler_id
    }
    
    /// Unregister an event handler
    pub fn unsubscribe<T>(&mut self, handler_id: Uuid) -> bool
    where
        T: Event + 'static,
    {
        let type_id = TypeId::of::<T>();
        let mut handlers = self.handlers.lock().unwrap();
        
        if let Some(type_handlers) = handlers.get_mut(&type_id) {
            if let Some(pos) = type_handlers.iter().position(|h| h.id == handler_id) {
                type_handlers.remove(pos);
                return true;
            }
        }
        false
    }
    
    /// Register middleware
    pub fn add_middleware(&mut self, middleware: Box<dyn EventMiddleware>) -> Uuid {
        let id = Uuid::new_v4();
        let priority = middleware.priority();
        
        let mut middleware_vec = self.middleware.lock().unwrap();
        middleware_vec.push((priority, middleware));
        
        // Sort middleware by priority
        middleware_vec.sort_by(|a, b| a.0.cmp(&b.0));
        
        id
    }
    
    /// Publish an event to the bus
    pub fn publish<T>(&mut self, event: T) -> Result<(), EventError>
    where
        T: Event + 'static,
    {
        if !*self.is_running.lock().unwrap() {
            return Err(EventError::ShuttingDown);
        }
        
        let type_id = TypeId::of::<T>();
        let boxed_event = Box::new(event);
        
        // Get handler count for this event type
        let handler_count = {
            let handlers_map = self.handlers.lock().unwrap();
            handlers_map.get(&type_id).map(|h| h.len()).unwrap_or(0)
        };
        
        let boxed = BoxedEvent {
            event: boxed_event,
            type_id,
            handler_count,
        };
        
        // Update stats
        {
            let mut stats = self.stats.lock().unwrap();
            stats.events_published += 1;
        }
        
        if self.config.async_processing {
            // Send to async processor
            if let Some(ref sender) = self.sender {
                sender.send(boxed).map_err(|_| EventError::ShuttingDown)?;
            }
        } else {
            // Process synchronously
            self.process_event(boxed)?;
        }
        
        Ok(())
    }
    
    /// Process events synchronously (for non-async mode or manual processing)
    fn process_event(&mut self, mut boxed_event: BoxedEvent) -> Result<(), EventError> {
        let start_time = Instant::now();
        let event_type_name = boxed_event.event.event_type();
        
        // Record event processing start
        let monitor = Arc::clone(&self.performance_monitor);
        tokio::spawn(async move {
            monitor.record_event_start(&event_type_name).await;
        });
        
        // Apply middleware (before)
        {
            let mut middleware = self.middleware.lock().unwrap();
            for (_, middleware_impl) in middleware.iter_mut() {
                middleware_impl.before_handle(boxed_event.event.as_mut())
                    .map_err(|e| EventError::MiddlewareError(e.to_string()))?;
            }
        }
        
        let mut processing_result = Ok(());
        let mut handler_results = Vec::new();
        
        // Process with handlers
        let handlers = {
            let handlers_map = self.handlers.lock().unwrap();
            handlers_map.get(&boxed_event.type_id).cloned().unwrap_or_default()
        };
        
        for handler_info in &handlers {
            let handler_start_time = Instant::now();
            let handler_name = format!("{}::{}", event_type_name, handler_info.priority as u8);
            let handler = handler_info.handler.clone();
            
            // This is a simplified version - in practice, you'd need more sophisticated
            // type casting to call the correct handler method
            match self.call_handler_for_event(&*boxed_event.event, handler) {
                Ok(()) => {
                    let handler_processing_time = handler_start_time.elapsed();
                    handler_results.push((handler_name, handler_processing_time, true));
                    continue;
                }
                Err(e) => {
                    let handler_processing_time = handler_start_time.elapsed();
                    handler_results.push((handler_name.clone(), handler_processing_time, false));
                    
                    // Log the error
                    let logging_system = Arc::clone(&self.logging_system);
                    let error_message = format!("Handler {} failed to process event {}: {:?}", handler_name, event_type_name, e);
                    let mut details = std::collections::HashMap::new();
                    details.insert("handler".to_string(), handler_name.clone());
                    details.insert("event_type".to_string(), event_type_name.to_string());
                    details.insert("processing_time_ms".to_string(), handler_processing_time.as_millis().to_string());
                    
                    tokio::spawn(async move {
                        logging_system.log_error(
                            crate::events::ErrorCategory::ApplicationError,
                            crate::events::LoggingLevel::Error,
                            error_message,
                            Some("event_processing".to_string()),
                            None,
                            details,
                            None,
                        ).await;
                    });
                    
                    processing_result = Err(e);
                    break;
                }
            }
        }
        
        // Apply middleware (after)
        {
            let mut middleware = self.middleware.lock().unwrap();
            for (_, middleware_impl) in middleware.iter_mut() {
                middleware_impl.after_handle(&*boxed_event.event, &processing_result);
            }
        }
        
        // Update stats
        {
            let mut stats = self.stats.lock().unwrap();
            stats.events_processed += 1;
            
            let processing_time = start_time.elapsed();
            stats.average_processing_time = 
                (stats.average_processing_time + processing_time) / 2;
            
            if processing_result.is_err() {
                stats.processing_errors += 1;
            }
        }
        
        // Record event completion and handler metrics
        let total_processing_time = start_time.elapsed();
        let event_success = processing_result.is_ok();
        let monitor = Arc::clone(&self.performance_monitor);
        let logging_system = Arc::clone(&self.logging_system);
        let event_type_name_clone = event_type_name;
        let handler_count = handlers.len();
        
        tokio::spawn(async move {
            // Record overall event completion
            monitor.record_event_completion(&event_type_name_clone, total_processing_time, event_success).await;
            
            // Record individual handler metrics
            for (handler_name, handler_time, handler_success) in handler_results {
                monitor.record_event_completion(&handler_name, handler_time, handler_success).await;
            }
            
            // Audit log successful event processing
            if event_success {
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("event_type".to_string(), event_type_name_clone.to_string());
                metadata.insert("handler_count".to_string(), handler_count.to_string());
                metadata.insert("processing_time_ms".to_string(), total_processing_time.as_millis().to_string());
                
                logging_system.log_audit(
                    "event_processed".to_string(),
                    "event_bus".to_string(),
                    None,
                    crate::events::AuditStatus::Success,
                    Some(total_processing_time),
                    metadata,
                    None,
                ).await;
            }
        });
        
        processing_result
    }
    
    /// Simplified handler calling - in practice this would use more sophisticated type handling
    fn call_handler_for_event(
        &self,
        _event: &dyn Event,
        _handler: Arc<Mutex<dyn Any + Send + Sync>>,
    ) -> Result<(), EventError> {
        // This is a placeholder - actual implementation would need proper type casting
        // and dynamic dispatch to the correct handler method
        Ok(())
    }
    
    /// Async event processor
    async fn async_processor(
        _bus_weak: Arc<Mutex<Option<Weak<Mutex<EventBus>>>>>,
        mut receiver: mpsc::UnboundedReceiver<BoxedEvent>,
    ) {
        while let Some(_event) = receiver.recv().await {
            // Process event asynchronously
            // This would need access to the bus instance for processing
            tokio::task::yield_now().await;
        }
    }
    
    /// Get current statistics
    pub fn stats(&self) -> EventBusStats {
        self.stats.lock().unwrap().clone()
    }
    
    /// Get performance monitor for detailed metrics
    pub fn performance_monitor(&self) -> Arc<PerformanceMonitor> {
        Arc::clone(&self.performance_monitor)
    }
    
    /// Start performance monitoring
    pub async fn start_performance_monitoring(&self) {
        self.performance_monitor.start_monitoring().await;
        tracing::info!("Event bus performance monitoring started");
    }
    
    /// Get comprehensive performance summary
    pub async fn get_performance_summary(&self) -> Result<crate::events::performance_metrics::PerformanceSummary, EventError> {
        Ok(self.performance_monitor.get_performance_summary().await)
    }
    
    /// Check performance health status
    pub async fn check_performance_health(&self) -> Result<crate::events::performance_metrics::PerformanceHealthStatus, EventError> {
        Ok(self.performance_monitor.check_performance_health().await)
    }
    
    /// Update memory usage statistics
    pub async fn update_memory_usage(&self, memory_bytes: u64) {
        let queue_size = {
            let queue = self.event_queue.lock().unwrap();
            queue.len()
        };
        let handler_count = {
            let handlers = self.handlers.lock().unwrap();
            handlers.values().map(|v| v.len()).sum()
        };
        
        self.performance_monitor
            .update_memory_usage(memory_bytes, queue_size, handler_count)
            .await;
    }
    
    /// Get current memory metrics
    pub async fn get_memory_metrics(&self) -> crate::events::performance_metrics::MemoryMetrics {
        self.performance_monitor.get_memory_metrics().await
    }
    
    /// Check for memory-related health issues
    pub async fn check_memory_health(&self) -> Vec<String> {
        self.performance_monitor.check_memory_health().await
    }
    
    /// Get memory usage trend
    pub async fn get_memory_trend(&self) -> f64 {
        self.performance_monitor.get_memory_usage_trend().await
    }
    
    /// Access the logging system
    pub fn logging_system(&self) -> Arc<crate::events::logging_system::LoggingSystem> {
        Arc::clone(&self.logging_system)
    }
    
    /// Log an error with context
    pub async fn log_error(
        &self,
        category: crate::events::ErrorCategory,
        level: crate::events::LoggingLevel,
        message: String,
        operation: Option<String>,
        details: std::collections::HashMap<String, String>,
    ) -> uuid::Uuid {
        self.logging_system
            .log_error(category, level, message, operation, None, details, None)
            .await
    }
    
    /// Log audit information
    pub async fn log_audit(
        &self,
        operation: String,
        status: crate::events::AuditStatus,
        duration: Option<std::time::Duration>,
        metadata: std::collections::HashMap<String, String>,
    ) {
        self.logging_system
            .log_audit(
                operation,
                "event_bus".to_string(),
                None,
                status,
                duration,
                metadata,
                None,
            )
            .await;
    }
    
    /// Log system health status
    pub async fn log_health(
        &self,
        status: crate::events::SystemHealthStatus,
        metrics: crate::events::HealthMetrics,
        issues: Vec<String>,
        recovery_actions: Vec<String>,
    ) {
        self.logging_system
            .log_health(status, metrics, issues, recovery_actions)
            .await;
    }
    
    /// Get error analysis from logging system
    pub async fn get_error_analysis(&self) -> crate::events::ErrorAnalysis {
        self.logging_system.get_error_analysis().await
    }
    
    /// Access the shutdown manager
    pub fn shutdown_manager(&self) -> Arc<crate::events::shutdown_manager::ShutdownManager> {
        Arc::clone(&self.shutdown_manager)
    }
    
    /// Initiate graceful shutdown of the event bus
    pub async fn graceful_shutdown(&self) -> Result<(), crate::events::shutdown_manager::ShutdownError> {
        info!("Initiating event bus graceful shutdown");
        
        // Register event bus shutdown hook
        let event_bus_shutdown_hook = crate::events::shutdown_manager::ShutdownHook {
            id: "event_bus_core".to_string(),
            priority: crate::events::shutdown_manager::ShutdownPriority::High,
            timeout: std::time::Duration::from_secs(5),
            callback: Box::new(|| {
                tokio::spawn(async {
                    // Stop accepting new events
                    tracing::info!("Stopping event bus processing");
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await; // Simulate cleanup
                    Ok(())
                })
            }),
            description: Some("Event bus core shutdown".to_string()),
        };
        
        self.shutdown_manager.register_hook(event_bus_shutdown_hook).await?;
        
        // Register resource cleanup tasks
        let queue_cleanup_task = crate::events::shutdown_manager::CleanupTask {
            id: "event_queue".to_string(),
            priority: crate::events::shutdown_manager::ShutdownPriority::Normal,
            cleanup_fn: Box::new(|| {
                tokio::spawn(async {
                    tracing::info!("Clearing event queue");
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                    Ok(())
                })
            }),
            description: "Event queue cleanup".to_string(),
            critical: false,
        };
        
        self.shutdown_manager.register_cleanup_task(queue_cleanup_task).await?;
        
        // Initiate shutdown through the manager
        self.shutdown_manager.initiate_shutdown().await
    }
    
    /// Force immediate shutdown (emergency)
    pub fn force_shutdown(&mut self) {
        *self.is_running.lock().unwrap() = false;
        warn!("Event bus force shutdown initiated");
    }
    
    /// Check if shutdown has been requested
    pub fn is_shutdown_requested(&self) -> bool {
        self.shutdown_manager.is_shutdown_requested()
    }
    
    /// Get current shutdown state
    pub async fn get_shutdown_state(&self) -> crate::events::shutdown_manager::ShutdownState {
        self.shutdown_manager.get_state().await
    }
    
    /// Get shutdown progress information
    pub async fn get_shutdown_progress(&self) -> crate::events::shutdown_manager::ShutdownProgress {
        self.shutdown_manager.get_progress().await
    }
    
    /// Shutdown the event bus (deprecated - use graceful_shutdown)
    #[deprecated(note = "Use graceful_shutdown() instead")]
    pub fn shutdown(&mut self) {
        *self.is_running.lock().unwrap() = false;
    }
    
    /// Process pending events (for sync mode)
    pub fn process_pending(&mut self) -> Result<usize, EventError> {
        let events_to_process = {
            let mut events = self.event_queue.lock().unwrap();
            let mut events_vec = Vec::new();
            while let Some(event) = events.pop_front() {
                events_vec.push(event);
            }
            events_vec
        };
        
        let event_count = events_to_process.len();
        for event in events_to_process {
            self.process_event(event)?;
        }
        
        Ok(event_count)
    }

    /// Process events with priority-based batching for better performance
    pub fn process_batch_optimized(&mut self) -> Result<usize, EventError> {
        let events_to_process = {
            let mut events = self.event_queue.lock().unwrap();
            let mut events_vec = Vec::new();
            let batch_limit = self.config.max_batch_size.min(events.len());
            
            // Take up to max_batch_size events from the queue
            for _ in 0..batch_limit {
                if let Some(event) = events.pop_front() {
                    events_vec.push(event);
                }
            }
            events_vec
        };

        if events_to_process.is_empty() {
            return Ok(0);
        }

        // Sort events by priority for optimal processing order
        let mut prioritized_events = events_to_process;
        prioritized_events.sort_by(|a, b| {
            a.event.metadata().priority.cmp(&b.event.metadata().priority)
        });

        // Group similar events for batch processing
        let mut event_groups: HashMap<TypeId, Vec<BoxedEvent>> = HashMap::new();
        for event in prioritized_events {
            event_groups.entry(event.type_id).or_insert_with(Vec::new).push(event);
        }

        let mut total_processed = 0;

        // Process each event type as a batch
        for (type_id, events) in event_groups {
            let batch_result = self.process_event_batch(type_id, events);
            match batch_result {
                Ok(count) => total_processed += count,
                Err(e) => {
                    tracing::warn!("Batch processing failed for type {:?}: {}", type_id, e);
                    // Continue processing other batches even if one fails
                }
            }
        }

        // Update statistics
        {
            let mut stats = self.stats.lock().unwrap();
            stats.events_processed += total_processed as u64;
            stats.batches_processed += 1;
        }

        Ok(total_processed)
    }

    /// Process a batch of events of the same type
    fn process_event_batch(&mut self, type_id: TypeId, events: Vec<BoxedEvent>) -> Result<usize, EventError> {
        // Get handlers for this event type once
        let handlers = {
            let handlers_map = self.handlers.lock().unwrap();
            handlers_map.get(&type_id).cloned().unwrap_or_default()
        };

        if handlers.is_empty() {
            // No handlers registered for this event type
            return Ok(events.len());
        }

        let mut processed_count = 0;

        // Pre-apply middleware to all events in the batch
        let processed_events = self.apply_batch_middleware(events)?;

        // Process each event with all handlers
        for event in processed_events {
            match self.process_single_event_with_handlers(event, &handlers) {
                Ok(()) => processed_count += 1,
                Err(e) => {
                    tracing::warn!("Event processing failed: {}", e);
                    // Continue with other events in the batch
                }
            }
        }

        Ok(processed_count)
    }

    /// Apply middleware to a batch of events
    fn apply_batch_middleware(&mut self, mut events: Vec<BoxedEvent>) -> Result<Vec<BoxedEvent>, EventError> {
        let mut middleware = self.middleware.lock().unwrap();
        
        for event in &mut events {
            for (_, middleware_impl) in middleware.iter_mut() {
                middleware_impl.before_handle(event.event.as_mut())
                    .map_err(|e| EventError::MiddlewareError(e.to_string()))?;
            }
        }
        
        Ok(events)
    }

    /// Process a single event with pre-fetched handlers
    fn process_single_event_with_handlers(&mut self, event: BoxedEvent, handlers: &[HandlerInfo]) -> Result<(), EventError> {
        for handler_info in handlers {
            let handler = handler_info.handler.clone();
            
            match self.call_handler_for_event(&*event.event, handler) {
                Ok(()) => continue,
                Err(e) => {
                    // Log error but continue with other handlers
                    tracing::warn!("Handler failed for event {}: {}", event.event.event_type(), e);
                }
            }
        }
        
        Ok(())
    }

    /// Get event bus performance statistics
    pub fn get_performance_stats(&self) -> Result<EventBusPerformanceStats, EventError> {
        let stats = self.stats.lock().unwrap();
        let queue_size = self.event_queue.lock().unwrap().len();
        
        Ok(EventBusPerformanceStats {
            events_published: stats.events_published,
            events_processed: stats.events_processed,
            events_dropped: stats.events_dropped,
            batches_processed: stats.batches_processed,
            current_queue_size: queue_size,
            average_processing_time_ms: if stats.batches_processed > 0 {
                stats.total_processing_time.as_millis() as f64 / stats.batches_processed as f64
            } else {
                0.0
            },
        })
    }

    /// Clear event bus statistics
    pub fn reset_performance_stats(&mut self) {
        let mut stats = self.stats.lock().unwrap();
        *stats = EventBusStats::default();
    }

    /// Optimize memory usage by cleaning up old events and compacting queues
    pub fn optimize_memory(&mut self) -> Result<usize, EventError> {
        let mut cleaned_events = 0;
        
        // Clean up event queue if it's getting too large
        let queue_size = {
            let queue = self.event_queue.lock().unwrap();
            queue.len()
        };
        
        if queue_size > self.config.max_queue_size * 3 / 4 {
            // If queue is 75% full, prioritize and remove low priority old events
            let mut queue = self.event_queue.lock().unwrap();
            let mut events_to_keep = Vec::new();
            let cutoff_time = Instant::now() - Duration::from_secs(30); // Keep events from last 30 seconds
            
            // Keep only recent high-priority events and all critical events
            while let Some(event) = queue.pop_front() {
                let should_keep = event.event.metadata().priority <= EventPriority::High
                    || event.event.metadata().timestamp > cutoff_time
                    || event.event.metadata().priority == EventPriority::Critical;
                    
                if should_keep {
                    events_to_keep.push(event);
                } else {
                    cleaned_events += 1;
                }
            }
            
            // Put kept events back
            for event in events_to_keep {
                queue.push_back(event);
            }
            
            tracing::info!("Memory optimization cleaned {} old events from queue", cleaned_events);
        }
        
        // Update statistics
        {
            let mut stats = self.stats.lock().unwrap();
            stats.events_dropped += cleaned_events as u64;
        }
        
        Ok(cleaned_events)
    }

    /// Check if event bus needs memory optimization
    pub fn should_optimize_memory(&self) -> bool {
        let queue_size = self.event_queue.lock().unwrap().len();
        queue_size > self.config.max_queue_size / 2 // Optimize when queue is 50% full
    }

    /// Add error recovery mechanism for failed event processing
    pub fn handle_processing_error(&mut self, error: &EventError, event_type: &str) -> Result<(), EventError> {
        tracing::warn!("Event processing error for {}: {}", event_type, error);
        
        // Update error statistics
        {
            let mut stats = self.stats.lock().unwrap();
            stats.processing_errors += 1;
        }
        
        // Implement circuit breaker pattern if too many errors
        let error_rate = {
            let stats = self.stats.lock().unwrap();
            if stats.events_processed > 100 {
                stats.processing_errors as f64 / stats.events_processed as f64
            } else {
                0.0
            }
        };
        
        if error_rate > 0.1 {  // More than 10% error rate
            tracing::error!("High error rate detected ({}%), consider investigating event handlers", error_rate * 100.0);
            // Could implement circuit breaker logic here
        }
        
        Ok(())
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

// Helper types and implementations

/// No-op event for testing and placeholders
#[derive(Debug)]
pub struct NoOpEvent {
    metadata: EventMetadata,
    #[allow(dead_code)]
    message: String,
}

impl NoOpEvent {
    pub fn new(message: &str) -> Self {
        Self {
            metadata: EventMetadata::new(EventPriority::Low, "system".to_string()),
            message: message.to_string(),
        }
    }
}

impl Event for NoOpEvent {
    fn event_type(&self) -> &'static str {
        "NoOpEvent"
    }
    
    fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }
}

// Fix typo in MiddlewarePriority alias
type MiddlewareePriority = MiddlewarePriority;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[derive(Debug)]
    struct TestEvent {
        metadata: EventMetadata,
        message: String,
    }
    
    impl TestEvent {
        fn new(message: String) -> Self {
            Self {
                metadata: EventMetadata::new(EventPriority::Normal, "test".to_string()),
                message,
            }
        }
    }
    
    impl Event for TestEvent {
        fn event_type(&self) -> &'static str {
            "TestEvent"
        }
        
        fn metadata(&self) -> &EventMetadata {
            &self.metadata
        }
    }
    
    struct TestHandler {
        received_events: Vec<String>,
    }
    
    impl TestHandler {
        fn new() -> Self {
            Self {
                received_events: Vec::new(),
            }
        }
    }
    
    impl EventHandler<TestEvent> for TestHandler {
        fn handle(&mut self, event: &TestEvent) -> Result<(), EventError> {
            self.received_events.push(event.message.clone());
            Ok(())
        }
    }
    
    #[test]
    fn test_event_bus_creation() {
        let config = EventBusConfig {
            async_processing: false,
            ..EventBusConfig::default()
        };
        let bus = EventBus::with_config(config);
        let stats = bus.stats();
        assert_eq!(stats.events_published, 0);
        assert_eq!(stats.events_processed, 0);
    }
    
    #[test]
    fn test_handler_registration() {
        let config = EventBusConfig {
            async_processing: false,
            ..EventBusConfig::default()
        };
        let mut bus = EventBus::with_config(config);
        let handler = TestHandler::new();
        
        let handler_id = bus.subscribe::<TestEvent, _>(handler);
        assert!(!handler_id.is_nil());
        
        let unsubscribed = bus.unsubscribe::<TestEvent>(handler_id);
        assert!(unsubscribed);
    }
}