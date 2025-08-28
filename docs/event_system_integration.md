# Event System Integration Guide

This guide demonstrates how to integrate and use the new Event Bus Architecture in Comunicado, replacing direct component coupling with a publish-subscribe pattern.

## Overview

The new event system provides:
- **Type-safe event handling** with compile-time guarantees
- **Decoupled architecture** reducing component dependencies
- **Middleware support** for cross-cutting concerns (logging, performance monitoring)
- **Async processing** for non-blocking operations
- **Legacy compatibility** for gradual migration
- **Event batching and queuing** for performance optimization

## Quick Start

### 1. Initialize the Event Bus

```rust
use comunicado::events;

// Initialize the global event bus (typically in main.rs or app initialization)
let event_bus = events::initialize_event_bus();

// The event bus is now available globally
```

### 2. Publishing Events

```rust
use comunicado::events::{publish, events};
use uuid::Uuid;

// UI Events
publish(events::pane_changed(
    events::FocusedPane::MessageList,
    events::FocusedPane::Calendar
))?;

// Email Events
publish(events::email_received(
    "account1".to_string(),
    Uuid::new_v4()
))?;

// System Events
publish(events::account_connected("account1".to_string()))?;
```

### 3. Creating Event Handlers

```rust
use comunicado::events::{EventHandler, EventError, UIEventData, subscribe};

struct MyUIHandler {
    name: String,
}

impl EventHandler<UIEventData> for MyUIHandler {
    fn handle(&mut self, event: &UIEventData) -> Result<(), EventError> {
        match &event.event {
            UIEvent::PaneChanged { from, to } => {
                println!("Pane changed from {:?} to {:?}", from, to);
                // Update local state, trigger UI refresh, etc.
            }
            UIEvent::ThemeChanged { theme_name } => {
                println!("Theme changed to: {}", theme_name);
                // Apply new theme, refresh components
            }
            _ => {} // Handle other events as needed
        }
        Ok(())
    }
}

// Register the handler
let handler = MyUIHandler { name: "MyHandler".to_string() };
let handler_id = subscribe::<UIEventData, _>(handler)?;
```

## Architecture Overview

### Event Flow

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Component A   │    │   Event Bus     │    │   Component B   │
│   (Publisher)   │───▶│                 │───▶│  (Subscriber)   │
└─────────────────┘    │  ┌───────────┐  │    └─────────────────┘
                       │  │Middleware │  │
                       │  │  Chain    │  │    ┌─────────────────┐
┌─────────────────┐    │  └───────────┘  │    │   Component C   │
│   Component D   │───▶│                 │───▶│  (Subscriber)   │
│   (Publisher)   │    │  ┌───────────┐  │    └─────────────────┘
└─────────────────┘    │  │Event Queue│  │
                       │  │& Batching │  │
                       │  └───────────┘  │
                       └─────────────────┘
```

### Event Types Hierarchy

```
Event (trait)
├── UIEventData
│   ├── PaneChanged
│   ├── ModeChanged
│   ├── ThemeChanged
│   └── KeyPressed
├── EmailEventData
│   ├── EmailReceived
│   ├── EmailSent
│   ├── EmailDeleted
│   └── FolderSynced
├── CalendarEventData
│   ├── EventCreated
│   ├── InvitationReceived
│   └── ReminderTriggered
├── AccountEventData
│   ├── AccountConnected
│   ├── AccountSyncStarted
│   └── AccountAuthFailed
└── SystemEventData
    ├── NetworkConnected
    ├── AppStarted
    └── PerformanceAlert
```

## Integration Examples

### 1. Email Component Integration

Replace direct method calls with event publishing:

**Before (Direct Coupling):**
```rust
// Old way - tight coupling
impl EmailComponent {
    fn delete_email(&mut self, email_id: Uuid) {
        // Delete email logic
        self.message_list.remove_email(email_id);
        self.notification_service.show_notification("Email deleted");
        self.analytics.track_action("email_delete");
    }
}
```

**After (Event-Driven):**
```rust
// New way - event publishing
impl EmailComponent {
    fn delete_email(&mut self, email_id: Uuid) -> Result<(), EventError> {
        // Delete email logic
        
        // Publish event - other components will handle UI updates, notifications, analytics
        publish(events::email_deleted("current_account".to_string(), email_id))?;
        
        Ok(())
    }
}

// Separate handlers for different concerns
struct MessageListHandler;
impl EventHandler<EmailEventData> for MessageListHandler {
    fn handle(&mut self, event: &EmailEventData) -> Result<(), EventError> {
        if let EmailEvent::EmailDeleted { email_id, .. } = &event.event {
            // Update message list UI
            self.remove_email_from_list(*email_id);
        }
        Ok(())
    }
}

struct NotificationHandler;
impl EventHandler<EmailEventData> for NotificationHandler {
    fn handle(&mut self, event: &EmailEventData) -> Result<(), EventError> {
        if let EmailEvent::EmailDeleted { .. } = &event.event {
            self.show_notification("Email deleted successfully");
        }
        Ok(())
    }
}
```

### 2. UI State Management

**Before (Scattered State Updates):**
```rust
impl UI {
    fn switch_pane(&mut self, new_pane: FocusedPane) {
        self.focused_pane = new_pane;
        self.update_title_bar();
        self.refresh_shortcuts();
        self.update_help_text();
        self.save_user_preference();
    }
}
```

**After (Event-Driven State):**
```rust
impl UI {
    fn switch_pane(&mut self, new_pane: FocusedPane) -> Result<(), EventError> {
        let old_pane = self.focused_pane.clone();
        self.focused_pane = new_pane.clone();
        
        // Single event triggers all necessary updates
        publish(events::pane_changed(old_pane, new_pane))?;
        
        Ok(())
    }
}

// Separate handlers for different UI concerns
struct TitleBarHandler;
impl EventHandler<UIEventData> for TitleBarHandler {
    fn handle(&mut self, event: &UIEventData) -> Result<(), EventError> {
        if let UIEvent::PaneChanged { to, .. } = &event.event {
            self.update_title_for_pane(to);
        }
        Ok(())
    }
}

struct ShortcutHandler;
impl EventHandler<UIEventData> for ShortcutHandler {
    fn handle(&mut self, event: &UIEventData) -> Result<(), EventError> {
        if let UIEvent::PaneChanged { to, .. } = &event.event {
            self.refresh_shortcuts_for_pane(to);
        }
        Ok(())
    }
}
```

### 3. Cross-Component Communication

**Calendar and Email Integration:**
```rust
// Email component publishes calendar-related events
impl EmailViewer {
    fn handle_calendar_invitation(&mut self, email_id: Uuid) -> Result<(), EventError> {
        // Parse invitation from email
        let invitation = self.parse_calendar_invitation(email_id)?;
        
        // Publish event for calendar component to handle
        publish(events::invitation_received(
            invitation.event_id,
            invitation.from
        ))?;
        
        Ok(())
    }
}

// Calendar component handles the invitation
struct CalendarInvitationHandler {
    calendar_service: CalendarService,
}

impl EventHandler<CalendarEventData> for CalendarInvitationHandler {
    fn handle(&mut self, event: &CalendarEventData) -> Result<(), EventError> {
        if let CalendarEvent::InvitationReceived { event_id, from } = &event.event {
            // Show invitation UI
            self.show_invitation_popup(event_id, from);
            
            // Add to pending invitations
            self.calendar_service.add_pending_invitation(event_id.clone(), from.clone());
        }
        Ok(())
    }
}
```

## Middleware Integration

### 1. Adding Custom Middleware

```rust
use comunicado::events::middleware::{EventMiddleware, MiddlewarePriority, EventError};

struct CustomSecurityMiddleware {
    security_service: SecurityService,
}

impl EventMiddleware for CustomSecurityMiddleware {
    fn before_handle(&mut self, event: &mut dyn Event) -> Result<(), EventError> {
        // Check if event is authorized
        if !self.security_service.is_event_authorized(event) {
            return Err(EventError::MiddlewareError(
                "Event not authorized".to_string()
            ));
        }
        
        // Log security-relevant events
        if event.metadata().priority == EventPriority::Critical {
            self.security_service.log_critical_event(event);
        }
        
        Ok(())
    }
    
    fn after_handle(&mut self, event: &dyn Event, result: &Result<(), EventError>) {
        if result.is_err() {
            self.security_service.log_event_failure(event);
        }
    }
    
    fn priority(&self) -> MiddlewarePriority {
        MiddlewarePriority::Critical  // Run first
    }
}

// Add to event bus
let bus = initialize_event_bus();
let mut bus = bus.lock().unwrap();
bus.add_middleware(CustomSecurityMiddleware::new(security_service));
```

### 2. Performance Monitoring

```rust
use comunicado::events::middleware::{PerformanceMiddleware, PerformanceMetrics};
use std::time::Duration;

// Add performance monitoring middleware
let bus = initialize_event_bus();
let mut bus = bus.lock().unwrap();
let perf_middleware = PerformanceMiddleware::new(Duration::from_millis(100));
bus.add_middleware(perf_middleware);

// Get performance metrics
let stats = get_stats().unwrap();
println!("Average event processing time: {:?}", stats.average_processing_time);
println!("Total events processed: {}", stats.events_processed);
```

## Legacy System Migration

### Gradual Migration Strategy

1. **Phase 1**: Add event bus alongside existing system
2. **Phase 2**: New features use event bus only
3. **Phase 3**: Migrate high-traffic paths to events
4. **Phase 4**: Migrate remaining components
5. **Phase 5**: Remove legacy event handling

### Migration Example

```rust
use comunicado::events::legacy::{EventBridge, EventResult};

// Legacy component using EventResult
impl LegacyComponent {
    fn handle_key(&mut self, key: KeyEvent) -> EventResult {
        // Bridge converts legacy results to modern events
        let bridge = EventBridge::new();
        bridge.handle_key_event(&key, self.ui_mode, self.focused_pane)
    }
}

// Modern component using events directly
impl ModernComponent {
    fn handle_key(&mut self, key: KeyEvent) -> Result<(), EventError> {
        let key_data = KeyEventData::from(key);
        publish(UIEventData::new(UIEvent::KeyPressed { key: key_data }))?;
        Ok(())
    }
}
```

## Testing with the Event System

### Unit Testing Event Handlers

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use comunicado::events::bus::NoOpEvent;
    
    #[test]
    fn test_email_deletion_handler() {
        let mut handler = EmailDeletionHandler::new();
        let event = events::email_deleted("account1".to_string(), Uuid::new_v4());
        
        let result = handler.handle(&event);
        assert!(result.is_ok());
        assert_eq!(handler.deleted_emails.len(), 1);
    }
    
    #[test]
    fn test_event_publishing() {
        let _bus = initialize_event_bus();
        
        let event = events::pane_changed(
            FocusedPane::MessageList,
            FocusedPane::Calendar
        );
        
        assert!(publish(event).is_ok());
    }
}
```

### Integration Testing

```rust
#[test]
fn test_email_to_calendar_workflow() {
    let _bus = initialize_event_bus();
    
    // Set up handlers
    let calendar_handler = CalendarInvitationHandler::new();
    let notification_handler = NotificationHandler::new();
    
    subscribe::<CalendarEventData, _>(calendar_handler).unwrap();
    subscribe::<CalendarEventData, _>(notification_handler).unwrap();
    
    // Simulate email invitation
    let invitation_event = events::invitation_received(
        "event123".to_string(),
        "user@example.com".to_string()
    );
    
    publish(invitation_event).unwrap();
    
    // Verify handlers were called (would need mock verification)
    // assert!(calendar_handler.received_invitation("event123"));
    // assert!(notification_handler.showed_notification());
}
```

## Best Practices

### 1. Event Design

- **Keep events immutable**: Events should be snapshots of what happened
- **Make events specific**: Prefer specific events over generic ones
- **Include necessary context**: Events should have all data handlers need
- **Use descriptive names**: Event names should clearly indicate what happened

```rust
// Good: Specific, descriptive event
EmailEvent::EmailMarkedAsRead { account_id, email_id, timestamp }

// Bad: Generic, unclear event  
EmailEvent::EmailChanged { email_id, change_type, data }
```

### 2. Handler Organization

- **Single responsibility**: Each handler should handle one concern
- **Stateless when possible**: Prefer stateless handlers for easier testing
- **Error handling**: Always handle errors gracefully
- **Performance conscious**: Keep handlers fast to avoid blocking

```rust
// Good: Focused handler
struct EmailNotificationHandler {
    notification_service: NotificationService,
}

impl EventHandler<EmailEventData> for EmailNotificationHandler {
    fn handle(&mut self, event: &EmailEventData) -> Result<(), EventError> {
        match &event.event {
            EmailEvent::EmailReceived { .. } => {
                self.notification_service.show_email_notification(event);
            }
            _ => {} // Only handle notification-relevant events
        }
        Ok(())
    }
}
```

### 3. Middleware Usage

- **Order matters**: Critical middleware should run first
- **Keep it fast**: Middleware runs for all events
- **Avoid side effects**: Middleware should be observational when possible
- **Error handling**: Failed middleware can block event processing

### 4. Performance Considerations

- **Batch related events**: Use event batching for high-frequency events
- **Async for heavy work**: Use async handlers for I/O or heavy processing
- **Monitor performance**: Use performance middleware to identify bottlenecks
- **Limit handler count**: Too many handlers can slow down event processing

## Debugging and Monitoring

### Enable Debug Tracing

```rust
use comunicado::events::middleware::{DebugMiddleware, LoggingMiddleware, LogLevel};

let debug_middleware = DebugMiddleware::new()
    .enable_trace()
    .with_detailed_logging();

let logging_middleware = LoggingMiddleware::new(LogLevel::Debug)
    .with_payload();

let bus = initialize_event_bus();
let mut bus = bus.lock().unwrap();
bus.add_middleware(debug_middleware);
bus.add_middleware(logging_middleware);
```

### Performance Monitoring

```rust
use comunicado::events::get_stats;

// Get event bus statistics
if let Some(stats) = get_stats() {
    println!("Event Bus Statistics:");
    println!("  Total events: {}", stats.events_processed);
    println!("  Average processing time: {:?}", stats.average_processing_time);
    println!("  Events dropped: {}", stats.events_dropped);
    println!("  Current queue size: {}", stats.queue_size);
}
```

This event system provides a robust foundation for decoupled, maintainable, and testable component communication in Comunicado. The migration can be done gradually while maintaining full backward compatibility.