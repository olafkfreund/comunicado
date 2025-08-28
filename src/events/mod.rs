//! Event System Module
//!
//! This module provides a comprehensive event-driven architecture for Comunicado,
//! replacing direct coupling with a publish-subscribe pattern using an event bus.

pub mod bus;
pub mod types;
pub mod middleware;
pub mod handlers;
pub mod application_handlers;
pub mod notification_bridge;
pub mod performance_metrics;
pub mod logging_system;
pub mod shutdown_manager;
pub mod legacy;

// Re-export core types for convenience
pub use bus::{
    EventBus, EventBusConfig, EventBusStats, Event, EventHandler, EventMiddleware,
    EventError, EventPriority, EventMetadata, HandlerPriority, MiddlewarePriority
};

pub use types::{
    // Event data types
    UIEventData, EmailEventData, CalendarEventData, ContactEventData,
    AccountEventData, NetworkEventData, AppEventData, PerformanceEventData,
    
    // Event enums
    UIEvent, EmailEvent, CalendarEvent, ContactEvent,
    AccountEvent, NetworkEvent, AppEvent, PerformanceEvent,
    
    // Supporting types
    FocusedPane, UIMode, ViewType, KeyEventData, MouseButton,
    ColorScheme, SearchScope,
    
    // Factory functions
    events,
};

pub use middleware::{
    LoggingMiddleware, PerformanceMiddleware, DebugMiddleware, ValidationMiddleware,
    RateLimitMiddleware, MiddlewareChain, LogLevel, PerformanceMetrics,
};

pub use application_handlers::{
    EmailEventHandler, CalendarEventHandler, AccountEventHandler, UIEventHandler,
    ApplicationEventHandler, EventHandlerRegistry, register_all_handlers,
};

pub use notification_bridge::{
    NotificationBridge, EmailNotificationBridge, CalendarNotificationBridge,
    AccountNotificationBridge, UINotificationBridge, AppNotificationBridge,
    NotificationBridgeRegistry,
};

pub use logging_system::{
    LoggingSystem, LoggingConfig, LogLevel as LoggingLevel, ErrorCategory, TrackedError, 
    ErrorTracker, AuditLogger, AuditLogEntry, AuditStatus, HealthLogger,
    SystemHealthStatus, HealthMetrics, LogStream, LogEntry, ErrorAnalysis,
};

pub use shutdown_manager::{
    ShutdownManager, ShutdownConfig, ShutdownState, ShutdownHook, ShutdownPriority,
    OperationInfo, ShutdownError, ShutdownPhase, ShutdownProgress, CleanupTask,
};

use std::sync::{Arc, Mutex};
use std::time::Duration;
use uuid::Uuid;

/// Global event bus instance for the application
static GLOBAL_EVENT_BUS: std::sync::OnceLock<Arc<Mutex<EventBus>>> = std::sync::OnceLock::new();

/// Initialize the global event bus with default configuration and register all handlers
pub fn initialize_event_bus() -> Arc<Mutex<EventBus>> {
    GLOBAL_EVENT_BUS.get_or_init(|| {
        let config = EventBusConfig {
            max_queue_size: 5000,
            batch_interval: Duration::from_millis(5),
            max_batch_size: 50,
            async_processing: true,
            worker_threads: 2,
        };
        
        let mut bus = EventBus::with_config(config);
        
        // Add default middleware
        let middleware_chain = MiddlewareChain::new()
            .with_logging(LogLevel::Info)
            .with_performance_monitoring(Duration::from_millis(50))
            .with_debug_tracing()
            .with_rate_limiting(1000, Duration::from_secs(1))
            .build();
            
        for middleware in middleware_chain {
            bus.add_middleware(middleware);
        }
        
        let bus_arc = Arc::new(Mutex::new(bus));
        
        // Register all application event handlers
        if let Err(e) = register_all_handlers() {
            tracing::error!("Failed to register event handlers: {}", e);
        }
        
        bus_arc
    }).clone()
}

/// Initialize event bus with notification system integration
pub fn initialize_event_bus_with_notifications(
    notification_manager: &crate::notifications::UnifiedNotificationManager
) -> Arc<Mutex<EventBus>> {
    let bus_arc = initialize_event_bus();
    
    // Register notification bridges
    if let Err(e) = register_notification_bridges(notification_manager) {
        tracing::error!("Failed to register notification bridges: {}", e);
    }
    
    bus_arc
}

/// Register notification bridges with the global event bus
pub fn register_notification_bridges(
    notification_manager: &crate::notifications::UnifiedNotificationManager
) -> Result<(), EventError> {
    let bus_arc = get_event_bus().ok_or_else(|| {
        EventError::HandlerNotFound("Global event bus not initialized".to_string())
    })?;
    
    let mut bus = bus_arc.lock().map_err(|_| {
        EventError::ProcessingFailed("Failed to lock event bus".to_string())
    })?;
    
    // Create and register notification bridge registry
    let bridge_registry = NotificationBridgeRegistry::new(notification_manager);
    bridge_registry.register_all_bridges(&mut bus);
    
    tracing::info!("Notification bridges registered with event bus");
    Ok(())
}

/// Get the global event bus instance
pub fn get_event_bus() -> Option<Arc<Mutex<EventBus>>> {
    GLOBAL_EVENT_BUS.get().cloned()
}

/// Publish an event to the global event bus
pub fn publish<T>(event: T) -> Result<(), EventError>
where
    T: Event + 'static,
{
    if let Some(bus_arc) = get_event_bus() {
        let mut bus = bus_arc.lock().unwrap();
        bus.publish(event)
    } else {
        Err(EventError::HandlerNotFound("Global event bus not initialized".to_string()))
    }
}

/// Subscribe to events on the global event bus
pub fn subscribe<T, H>(handler: H) -> Result<Uuid, EventError>
where
    T: Event + 'static,
    H: EventHandler<T> + 'static,
{
    if let Some(bus_arc) = get_event_bus() {
        let mut bus = bus_arc.lock().unwrap();
        Ok(bus.subscribe::<T, H>(handler))
    } else {
        Err(EventError::HandlerNotFound("Global event bus not initialized".to_string()))
    }
}

/// Get statistics from the global event bus
pub fn get_stats() -> Option<EventBusStats> {
    if let Some(bus_arc) = get_event_bus() {
        let bus = bus_arc.lock().unwrap();
        Some(bus.stats())
    } else {
        None
    }
}

/// Shutdown the global event bus
pub async fn shutdown_event_bus() {
    if let Some(bus_arc) = get_event_bus() {
        let bus = bus_arc.lock().unwrap();
        let _ = bus.graceful_shutdown().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::bus::NoOpEvent;
    
    #[test]
    fn test_global_event_bus_initialization() {
        let _bus = initialize_event_bus();
        assert!(get_event_bus().is_some());
        
        let stats = get_stats();
        assert!(stats.is_some());
    }
    
    #[test]
    fn test_event_publishing() {
        let _bus = initialize_event_bus();
        
        let event = NoOpEvent::new("test event");
        let result = publish(event);
        
        // Should succeed even with no handlers
        assert!(result.is_ok());
    }
}