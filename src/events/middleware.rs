//! Event Middleware for Cross-Cutting Concerns
//!
//! This module provides middleware implementations for logging, debugging,
//! performance monitoring, and other cross-cutting concerns that apply
//! to all events in the system.

use crate::events::bus::{Event, EventError, EventMiddleware, MiddlewarePriority};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use uuid::Uuid;

// =============================================================================
// Logging Middleware
// =============================================================================

/// Logs all events passing through the system
pub struct LoggingMiddleware {
    priority: MiddlewarePriority,
    log_level: LogLevel,
    include_payload: bool,
    filter_events: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LoggingMiddleware {
    pub fn new(log_level: LogLevel) -> Self {
        Self {
            priority: MiddlewarePriority::High,
            log_level,
            include_payload: false,
            filter_events: Vec::new(),
        }
    }

    pub fn with_payload(mut self) -> Self {
        self.include_payload = true;
        self
    }

    pub fn with_filter(mut self, event_types: Vec<String>) -> Self {
        self.filter_events = event_types;
        self
    }

    fn should_log(&self, event: &dyn Event) -> bool {
        if self.filter_events.is_empty() {
            return true;
        }

        !self.filter_events.contains(&event.event_type().to_string())
    }
}

impl EventMiddleware for LoggingMiddleware {
    fn before_handle(&mut self, event: &mut dyn Event) -> Result<(), EventError> {
        if !self.should_log(event) {
            return Ok(());
        }

        let metadata = event.metadata();

        match self.log_level {
            LogLevel::Trace => {
                tracing::trace!(
                    "Processing event: {} [{}] from {} at {:?}",
                    event.event_type(),
                    metadata.id,
                    metadata.source,
                    metadata.timestamp
                );
            }
            LogLevel::Debug => {
                tracing::debug!(
                    "Event: {} [{}] priority: {:?}",
                    event.event_type(),
                    metadata.id,
                    metadata.priority
                );
            }
            LogLevel::Info => {
                tracing::info!("Processing {}", event.event_type());
            }
            _ => {}
        }

        if self.include_payload {
            tracing::debug!("Event payload: {:?}", event);
        }

        Ok(())
    }

    fn after_handle(&mut self, event: &dyn Event, result: &Result<(), EventError>) {
        if !self.should_log(event) {
            return;
        }

        match result {
            Ok(()) => {
                tracing::debug!("Event {} processed successfully", event.event_type());
            }
            Err(e) => {
                tracing::error!("Event {} failed: {}", event.event_type(), e);
            }
        }
    }

    fn priority(&self) -> MiddlewarePriority {
        self.priority
    }
}

// =============================================================================
// Performance Monitoring Middleware
// =============================================================================

/// Monitors event processing performance and collects metrics
pub struct PerformanceMiddleware {
    metrics: Arc<Mutex<PerformanceMetrics>>,
    slow_threshold: Duration,
    active_events: Arc<Mutex<HashMap<Uuid, Instant>>>,
}

#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    pub total_events: u64,
    pub total_processing_time: Duration,
    pub average_processing_time: Duration,
    pub slowest_event_time: Duration,
    pub slowest_event_type: Option<String>,
    pub event_counts: HashMap<String, u64>,
    pub event_times: HashMap<String, Duration>,
    pub slow_events: Vec<SlowEvent>,
}

#[derive(Debug, Clone)]
pub struct SlowEvent {
    pub event_type: String,
    pub event_id: Uuid,
    pub processing_time: Duration,
    pub timestamp: Instant,
}

impl PerformanceMiddleware {
    pub fn new(slow_threshold: Duration) -> Self {
        Self {
            metrics: Arc::new(Mutex::new(PerformanceMetrics::default())),
            slow_threshold,
            active_events: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn get_metrics(&self) -> PerformanceMetrics {
        self.metrics.lock().unwrap().clone()
    }

    pub fn reset_metrics(&mut self) {
        *self.metrics.lock().unwrap() = PerformanceMetrics::default();
    }
}

impl EventMiddleware for PerformanceMiddleware {
    fn before_handle(&mut self, event: &mut dyn Event) -> Result<(), EventError> {
        let event_id = event.metadata().id;
        self.active_events
            .lock()
            .unwrap()
            .insert(event_id, Instant::now());
        Ok(())
    }

    fn after_handle(&mut self, event: &dyn Event, _result: &Result<(), EventError>) {
        let event_id = event.metadata().id;
        let event_type = event.event_type();

        if let Some(start_time) = self.active_events.lock().unwrap().remove(&event_id) {
            let processing_time = start_time.elapsed();

            let mut metrics = self.metrics.lock().unwrap();
            metrics.total_events += 1;
            metrics.total_processing_time += processing_time;

            // Update average
            metrics.average_processing_time =
                metrics.total_processing_time / metrics.total_events as u32;

            // Update per-event-type metrics
            *metrics
                .event_counts
                .entry(event_type.to_string())
                .or_insert(0) += 1;
            let event_total_time = metrics
                .event_times
                .entry(event_type.to_string())
                .or_insert(Duration::ZERO);
            *event_total_time += processing_time;

            // Track slowest event
            if processing_time > metrics.slowest_event_time {
                metrics.slowest_event_time = processing_time;
                metrics.slowest_event_type = Some(event_type.to_string());
            }

            // Track slow events
            if processing_time > self.slow_threshold {
                metrics.slow_events.push(SlowEvent {
                    event_type: event_type.to_string(),
                    event_id,
                    processing_time,
                    timestamp: start_time,
                });

                // Keep only last 100 slow events
                if metrics.slow_events.len() > 100 {
                    metrics.slow_events.remove(0);
                }

                tracing::warn!(
                    "Slow event detected: {} took {:?} (threshold: {:?})",
                    event_type,
                    processing_time,
                    self.slow_threshold
                );
            }
        }
    }

    fn priority(&self) -> MiddlewarePriority {
        MiddlewarePriority::High
    }
}

// =============================================================================
// Debug Tracing Middleware
// =============================================================================

/// Provides detailed tracing and debugging information for events
pub struct DebugMiddleware {
    trace_buffer: Arc<Mutex<VecDeque<TraceEntry>>>,
    max_buffer_size: usize,
    trace_enabled: bool,
    detailed_logging: bool,
}

#[derive(Debug, Clone)]
pub struct TraceEntry {
    pub timestamp: Instant,
    pub event_id: Uuid,
    pub event_type: String,
    pub event_source: String,
    pub correlation_id: Option<Uuid>,
    pub processing_time: Option<Duration>,
    pub result: TraceResult,
    pub context: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub enum TraceResult {
    Started,
    Completed,
    Failed(String),
}

impl DebugMiddleware {
    pub fn new() -> Self {
        Self {
            trace_buffer: Arc::new(Mutex::new(VecDeque::new())),
            max_buffer_size: 1000,
            trace_enabled: cfg!(debug_assertions),
            detailed_logging: false,
        }
    }

    pub fn enable_trace(mut self) -> Self {
        self.trace_enabled = true;
        self
    }

    pub fn with_detailed_logging(mut self) -> Self {
        self.detailed_logging = true;
        self
    }

    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.max_buffer_size = size;
        self
    }

    pub fn get_trace_entries(&self) -> Vec<TraceEntry> {
        self.trace_buffer.lock().unwrap().iter().cloned().collect()
    }

    pub fn clear_trace(&mut self) {
        self.trace_buffer.lock().unwrap().clear();
    }

    fn add_trace_entry(&self, entry: TraceEntry) {
        let mut buffer = self.trace_buffer.lock().unwrap();

        if buffer.len() >= self.max_buffer_size {
            buffer.pop_front();
        }

        buffer.push_back(entry);
    }
}

impl Default for DebugMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl EventMiddleware for DebugMiddleware {
    fn before_handle(&mut self, event: &mut dyn Event) -> Result<(), EventError> {
        if !self.trace_enabled {
            return Ok(());
        }

        let metadata = event.metadata();

        let entry = TraceEntry {
            timestamp: Instant::now(),
            event_id: metadata.id,
            event_type: event.event_type().to_string(),
            event_source: metadata.source.clone(),
            correlation_id: metadata.correlation_id,
            processing_time: None,
            result: TraceResult::Started,
            context: HashMap::new(),
        };

        self.add_trace_entry(entry);

        if self.detailed_logging {
            tracing::trace!(
                "Event trace: {} [{}] started, correlation: {:?}",
                event.event_type(),
                metadata.id,
                metadata.correlation_id
            );
        }

        Ok(())
    }

    fn after_handle(&mut self, event: &dyn Event, result: &Result<(), EventError>) {
        if !self.trace_enabled {
            return;
        }

        let metadata = event.metadata();
        let processing_time = metadata.timestamp.elapsed();

        let trace_result = match result {
            Ok(()) => TraceResult::Completed,
            Err(e) => TraceResult::Failed(e.to_string()),
        };

        let entry = TraceEntry {
            timestamp: Instant::now(),
            event_id: metadata.id,
            event_type: event.event_type().to_string(),
            event_source: metadata.source.clone(),
            correlation_id: metadata.correlation_id,
            processing_time: Some(processing_time),
            result: trace_result,
            context: HashMap::new(),
        };

        self.add_trace_entry(entry);

        if self.detailed_logging {
            tracing::trace!(
                "Event trace: {} [{}] completed in {:?}, result: {:?}",
                event.event_type(),
                metadata.id,
                processing_time,
                result.is_ok()
            );
        }
    }

    fn priority(&self) -> MiddlewarePriority {
        MiddlewarePriority::Normal
    }
}

// =============================================================================
// Validation Middleware
// =============================================================================

/// Validates events before processing and can reject invalid events
pub struct ValidationMiddleware {
    validators: HashMap<String, Box<dyn EventValidator>>,
    strict_mode: bool,
}

pub trait EventValidator: Send + Sync {
    fn validate(&self, event: &dyn Event) -> Result<(), String>;
}

impl ValidationMiddleware {
    pub fn new() -> Self {
        Self {
            validators: HashMap::new(),
            strict_mode: false,
        }
    }

    pub fn strict(mut self) -> Self {
        self.strict_mode = true;
        self
    }

    pub fn add_validator<V>(&mut self, event_type: String, validator: V)
    where
        V: EventValidator + 'static,
    {
        self.validators.insert(event_type, Box::new(validator));
    }
}

impl Default for ValidationMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl EventMiddleware for ValidationMiddleware {
    fn before_handle(&mut self, event: &mut dyn Event) -> Result<(), EventError> {
        let event_type = event.event_type();

        if let Some(validator) = self.validators.get(event_type) {
            if let Err(validation_error) = validator.validate(event) {
                let error_msg =
                    format!("Validation failed for {}: {}", event_type, validation_error);

                if self.strict_mode {
                    return Err(EventError::MiddlewareError(error_msg));
                } else {
                    tracing::warn!("{}", error_msg);
                }
            }
        }

        Ok(())
    }

    fn after_handle(&mut self, _event: &dyn Event, _result: &Result<(), EventError>) {
        // No post-processing needed for validation
    }

    fn priority(&self) -> MiddlewarePriority {
        MiddlewarePriority::Critical
    }
}

// =============================================================================
// Rate Limiting Middleware
// =============================================================================

/// Prevents event flooding by rate limiting events per source
pub struct RateLimitMiddleware {
    rate_limits: HashMap<String, RateLimit>,
    default_limit: RateLimit,
}

#[derive(Debug, Clone)]
struct RateLimit {
    max_events: u32,
    window: Duration,
    current_count: u32,
    window_start: Instant,
}

impl RateLimit {
    fn new(max_events: u32, window: Duration) -> Self {
        Self {
            max_events,
            window,
            current_count: 0,
            window_start: Instant::now(),
        }
    }

    fn check_and_update(&mut self) -> bool {
        let now = Instant::now();

        // Reset if window has passed
        if now.duration_since(self.window_start) >= self.window {
            self.current_count = 0;
            self.window_start = now;
        }

        if self.current_count >= self.max_events {
            false // Rate limit exceeded
        } else {
            self.current_count += 1;
            true // Allow
        }
    }
}

impl RateLimitMiddleware {
    pub fn new(default_max_events: u32, default_window: Duration) -> Self {
        Self {
            rate_limits: HashMap::new(),
            default_limit: RateLimit::new(default_max_events, default_window),
        }
    }

    pub fn add_source_limit(&mut self, source: String, max_events: u32, window: Duration) {
        self.rate_limits
            .insert(source, RateLimit::new(max_events, window));
    }
}

impl EventMiddleware for RateLimitMiddleware {
    fn before_handle(&mut self, event: &mut dyn Event) -> Result<(), EventError> {
        let source = event.metadata().source.clone();

        let allowed = if let Some(rate_limit) = self.rate_limits.get_mut(&source) {
            rate_limit.check_and_update()
        } else {
            self.default_limit.check_and_update()
        };

        if !allowed {
            tracing::warn!("Rate limit exceeded for source: {}", source);
            return Err(EventError::MiddlewareError(format!(
                "Rate limit exceeded for source: {}",
                source
            )));
        }

        Ok(())
    }

    fn after_handle(&mut self, _event: &dyn Event, _result: &Result<(), EventError>) {
        // No post-processing needed for rate limiting
    }

    fn priority(&self) -> MiddlewarePriority {
        MiddlewarePriority::High
    }
}

// =============================================================================
// Middleware Chain Builder
// =============================================================================

/// Builder for creating middleware chains with common configurations
pub struct MiddlewareChain {
    middleware: Vec<Box<dyn EventMiddleware>>,
}

impl MiddlewareChain {
    pub fn new() -> Self {
        Self {
            middleware: Vec::new(),
        }
    }

    pub fn with_logging(mut self, log_level: LogLevel) -> Self {
        self.middleware
            .push(Box::new(LoggingMiddleware::new(log_level)));
        self
    }

    pub fn with_performance_monitoring(mut self, slow_threshold: Duration) -> Self {
        self.middleware
            .push(Box::new(PerformanceMiddleware::new(slow_threshold)));
        self
    }

    pub fn with_debug_tracing(mut self) -> Self {
        self.middleware
            .push(Box::new(DebugMiddleware::new().enable_trace()));
        self
    }

    pub fn with_validation(mut self) -> Self {
        self.middleware.push(Box::new(ValidationMiddleware::new()));
        self
    }

    pub fn with_rate_limiting(mut self, max_events: u32, window: Duration) -> Self {
        self.middleware
            .push(Box::new(RateLimitMiddleware::new(max_events, window)));
        self
    }

    pub fn build(self) -> Vec<Box<dyn EventMiddleware>> {
        self.middleware
    }
}

impl Default for MiddlewareChain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::bus::NoOpEvent;

    #[test]
    fn test_logging_middleware() {
        let mut middleware = LoggingMiddleware::new(LogLevel::Debug);
        let mut event = NoOpEvent::new("test");

        assert!(middleware.before_handle(&mut event).is_ok());
        middleware.after_handle(&event, &Ok(()));
    }

    #[test]
    fn test_performance_middleware() {
        let mut middleware = PerformanceMiddleware::new(Duration::from_millis(100));
        let mut event = NoOpEvent::new("test");

        assert!(middleware.before_handle(&mut event).is_ok());

        // Simulate some processing time
        std::thread::sleep(Duration::from_millis(10));

        middleware.after_handle(&event, &Ok(()));

        let metrics = middleware.get_metrics();
        assert_eq!(metrics.total_events, 1);
        assert!(metrics.average_processing_time >= Duration::from_millis(10));
    }

    #[test]
    fn test_middleware_chain() {
        let chain = MiddlewareChain::new()
            .with_logging(LogLevel::Debug)
            .with_performance_monitoring(Duration::from_millis(50))
            .build();

        assert_eq!(chain.len(), 2);
    }
}
