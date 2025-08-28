//! Legacy Event System Compatibility
//!
//! This module provides compatibility layer between the old EventResult-based
//! system and the new event bus architecture, allowing for gradual migration.

use crate::events::bus::{Event, EventMetadata, EventPriority};
use crate::events::types::*;
use crate::ui::{ComposeAction, DraftAction, FocusedPane, UIMode};
use crate::ui::command_palette::CommandAction;
use crate::ui::folder_tree::FolderOperation;
use crate::contacts::ContactPopupAction;
use uuid::Uuid;

/// Legacy EventResult enum for backward compatibility
#[derive(Debug, Clone)]
pub enum EventResult {
    Continue,
    Handled,
    ComposeAction(ComposeAction),
    DraftAction(DraftAction),
    CommandAction(CommandAction),
    TriggerEmailSync,
    AccountSwitch(String),
    AddAccount,
    RemoveAccount(String),
    RefreshAccount(String),
    SyncAccount(String),
    FolderSelect(String),
    FolderForceRefresh(String),
    FolderOperation(FolderOperation),
    ContactsPopup,
    ContactsAction(ContactPopupAction),
    AddToContacts(String, String),
    EmailViewerStarted(String),
    ViewSenderContact(String),
    EditSenderContact(String),
    RemoveSenderFromContacts(String),
    ContactQuickActions(String),
    ReplyToMessage(Uuid),
    ReplyAllToMessage(Uuid),
    ForwardMessage(Uuid),
    DeleteEmail(String, Uuid, String),
    ArchiveEmail(String, Uuid, String),
    MarkEmailRead(String, Uuid, String),
    MarkEmailUnread(String, Uuid, String),
    ToggleEmailFlag(String, Uuid, String),
    CreateEvent(String),
    EditEvent(String, String),
    DeleteEvent(String, String),
    ViewEventDetails(String, String),
    CreateTodo(String),
    ToggleTodoComplete(String, String),
    RetryInitialization,
    CancelBackgroundTask,
    AISummarizeEmail(Uuid),
    ConvertEmailToNote(Uuid),
    ConvertEventToNote(String),
    ConvertKdeMessageToNote(String, String),
    ShowNotes,
    CreateNote,
}

/// Wrapper for legacy EventResult to make it compatible with the new Event system
#[derive(Debug)]
pub struct LegacyEventResultWrapper {
    metadata: EventMetadata,
    result: EventResult,
}

impl LegacyEventResultWrapper {
    pub fn new(result: EventResult) -> Self {
        let priority = match &result {
            EventResult::Continue | EventResult::Handled => EventPriority::Low,
            EventResult::TriggerEmailSync | EventResult::AddAccount | 
            EventResult::RemoveAccount(_) | EventResult::RefreshAccount(_) => EventPriority::High,
            _ => EventPriority::Normal,
        };
        
        Self {
            metadata: EventMetadata::new(priority, "legacy".to_string()),
            result,
        }
    }
    
    pub fn result(&self) -> &EventResult {
        &self.result
    }
}

impl Event for LegacyEventResultWrapper {
    fn event_type(&self) -> &'static str {
        "LegacyEventResult"
    }
    
    fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }
}

/// Converter from legacy EventResult to modern event types
pub struct EventResultConverter;

impl EventResultConverter {
    /// Convert legacy EventResult to modern event types and publish to event bus
    pub fn convert_and_publish(result: &EventResult) -> Vec<Box<dyn Event>> {
        let mut events: Vec<Box<dyn Event>> = Vec::new();
        
        match result {
            // Email operations
            EventResult::DeleteEmail(account_id, email_id, _folder) => {
                events.push(Box::new(EmailEventData::new(EmailEvent::EmailDeleted {
                    account_id: account_id.clone(),
                    email_id: *email_id,
                })));
            }
            EventResult::ArchiveEmail(account_id, email_id, _folder) => {
                events.push(Box::new(EmailEventData::new(EmailEvent::EmailArchived {
                    account_id: account_id.clone(),
                    email_id: *email_id,
                })));
            }
            EventResult::MarkEmailRead(account_id, email_id, _folder) => {
                events.push(Box::new(EmailEventData::new(EmailEvent::EmailMarkedRead {
                    account_id: account_id.clone(),
                    email_id: *email_id,
                })));
            }
            EventResult::MarkEmailUnread(account_id, email_id, _folder) => {
                events.push(Box::new(EmailEventData::new(EmailEvent::EmailMarkedUnread {
                    account_id: account_id.clone(),
                    email_id: *email_id,
                })));
            }
            EventResult::ToggleEmailFlag(account_id, email_id, _folder) => {
                events.push(Box::new(EmailEventData::new(EmailEvent::EmailFlagged {
                    account_id: account_id.clone(),
                    email_id: *email_id,
                })));
            }
            EventResult::ReplyToMessage(email_id) => {
                events.push(Box::new(EmailEventData::new(EmailEvent::EmailReplied {
                    original_id: *email_id,
                    reply_id: Uuid::new_v4(),
                })));
            }
            EventResult::ForwardMessage(email_id) => {
                events.push(Box::new(EmailEventData::new(EmailEvent::EmailForwarded {
                    original_id: *email_id,
                    forward_id: Uuid::new_v4(),
                })));
            }
            
            // Account operations
            EventResult::AccountSwitch(account_id) => {
                events.push(Box::new(AccountEventData::new(AccountEvent::AccountConnected {
                    account_id: account_id.clone(),
                })));
            }
            EventResult::AddAccount => {
                // This would need the actual account details
                events.push(Box::new(AccountEventData::new(AccountEvent::AccountAdded {
                    account_id: "new_account".to_string(),
                    provider: "unknown".to_string(),
                })));
            }
            EventResult::RemoveAccount(account_id) => {
                events.push(Box::new(AccountEventData::new(AccountEvent::AccountRemoved {
                    account_id: account_id.clone(),
                })));
            }
            EventResult::SyncAccount(account_id) => {
                events.push(Box::new(AccountEventData::new(AccountEvent::AccountSyncStarted {
                    account_id: account_id.clone(),
                })));
            }
            
            // Folder operations
            EventResult::FolderSelect(folder_path) => {
                events.push(Box::new(EmailEventData::new(EmailEvent::FolderChanged {
                    account_id: "current_account".to_string(),
                    folder_path: folder_path.clone(),
                })));
            }
            
            // Calendar operations
            EventResult::CreateEvent(calendar_id) => {
                events.push(Box::new(CalendarEventData::new(CalendarEvent::EventCreated {
                    calendar_id: calendar_id.clone(),
                    event_id: Uuid::new_v4().to_string(),
                })));
            }
            EventResult::EditEvent(calendar_id, event_id) => {
                events.push(Box::new(CalendarEventData::new(CalendarEvent::EventUpdated {
                    calendar_id: calendar_id.clone(),
                    event_id: event_id.clone(),
                })));
            }
            EventResult::DeleteEvent(calendar_id, event_id) => {
                events.push(Box::new(CalendarEventData::new(CalendarEvent::EventDeleted {
                    calendar_id: calendar_id.clone(),
                    event_id: event_id.clone(),
                })));
            }
            
            // Contact operations
            EventResult::AddToContacts(email, name) => {
                events.push(Box::new(ContactEventData::new(ContactEvent::ContactAdded {
                    contact_id: Uuid::new_v4(),
                    name: name.clone(),
                    email: email.clone(),
                })));
            }
            
            // UI state changes would be handled by translating to UI events
            EventResult::Continue | EventResult::Handled => {
                // These don't map to specific events
            }
            
            _ => {
                // For unhandled cases, create a generic legacy event
                tracing::debug!("Unhandled legacy event result: {:?}", result);
            }
        }
        
        events
    }
    
    /// Convert modern events back to legacy EventResult format (for gradual migration)
    pub fn event_to_result(event: &dyn Event) -> Option<EventResult> {
        match event.event_type() {
            "EmailEvent" => {
                // This is a simplified conversion - in practice you'd need more sophisticated
                // pattern matching based on the actual event content
                Some(EventResult::Handled)
            }
            "CalendarEvent" => {
                Some(EventResult::Handled)
            }
            "UIEvent" => {
                Some(EventResult::Continue)
            }
            _ => None,
        }
    }
}

/// Legacy event handler that maintains old behavior while using new event system
pub struct LegacyEventHandler {
    // Legacy handler callback for backward compatibility
    legacy_callback: Option<Box<dyn Fn(&EventResult) -> bool + Send + Sync>>,
}

impl LegacyEventHandler {
    pub fn new() -> Self {
        Self {
            legacy_callback: None,
        }
    }
    
    pub fn with_callback<F>(mut self, callback: F) -> Self 
    where
        F: Fn(&EventResult) -> bool + Send + Sync + 'static,
    {
        self.legacy_callback = Some(Box::new(callback));
        self
    }
    
    /// Process legacy EventResult using the new event system
    pub fn process_legacy_result(&self, result: EventResult) -> bool {
        // TODO: Convert to modern events and publish
        // This is a placeholder during migration - specific events will be published directly
        tracing::debug!("Legacy event result processing: {:?}", result);
        
        // Call legacy callback if present
        if let Some(ref callback) = self.legacy_callback {
            callback(&result)
        } else {
            true
        }
    }
}

impl Default for LegacyEventHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Bridge between old and new event systems for UI components
pub struct EventBridge {
    legacy_handler: LegacyEventHandler,
}

impl EventBridge {
    pub fn new() -> Self {
        Self {
            legacy_handler: LegacyEventHandler::new(),
        }
    }
    
    /// Process a legacy EventResult through the legacy handler before converting to modern events
    pub fn process_legacy_result(&self, result: &EventResult) -> bool {
        self.legacy_handler.process_legacy_result(result.clone())
    }
    
    /// Handle legacy keyboard events and convert to modern events
    pub fn handle_key_event(&self, key_event: &crossterm::event::KeyEvent, ui_mode: UIMode, _focused_pane: FocusedPane) -> EventResult {
        // Create UI event for key press
        let key_data = KeyEventData {
            code: format!("{:?}", key_event.code),
            modifiers: vec![], // Would extract actual modifiers
            char: match key_event.code {
                crossterm::event::KeyCode::Char(c) => Some(c),
                _ => None,
            },
        };
        
        let ui_event = UIEventData::new(UIEvent::KeyPressed { key: key_data });
        
        // Publish the modern event
        if let Err(e) = crate::events::publish(ui_event) {
            tracing::error!("Failed to publish UI event: {}", e);
        }
        
        // Return legacy result for backward compatibility
        match key_event.code {
            crossterm::event::KeyCode::Esc => EventResult::Continue,
            crossterm::event::KeyCode::Enter => match ui_mode {
                UIMode::Compose => EventResult::ComposeAction(ComposeAction::Send),
                _ => EventResult::Handled,
            },
            _ => EventResult::Continue,
        }
    }
    
    /// Handle application shutdown with both old and new systems
    pub async fn handle_shutdown(&self) {
        // Publish modern shutdown event
        let app_event = AppEventData::new(AppEvent::AppShuttingDown);
        if let Err(e) = crate::events::publish(app_event) {
            tracing::error!("Failed to publish shutdown event: {}", e);
        }
        
        // Shutdown the event bus
        crate::events::shutdown_event_bus().await;
    }
}

impl Default for EventBridge {
    fn default() -> Self {
        Self::new()
    }
}

/// Utility functions for creating event factories from legacy results
pub mod legacy_events {
    use super::*;
    
    pub fn email_deleted(account_id: String, email_id: Uuid) -> EmailEventData {
        EmailEventData::new(EmailEvent::EmailDeleted { account_id, email_id })
    }
    
    pub fn email_archived(account_id: String, email_id: Uuid) -> EmailEventData {
        EmailEventData::new(EmailEvent::EmailArchived { account_id, email_id })
    }
    
    pub fn email_marked_read(account_id: String, email_id: Uuid) -> EmailEventData {
        EmailEventData::new(EmailEvent::EmailMarkedRead { account_id, email_id })
    }
    
    pub fn email_marked_unread(account_id: String, email_id: Uuid) -> EmailEventData {
        EmailEventData::new(EmailEvent::EmailMarkedUnread { account_id, email_id })
    }
    
    pub fn email_flagged(account_id: String, email_id: Uuid) -> EmailEventData {
        EmailEventData::new(EmailEvent::EmailFlagged { account_id, email_id })
    }
    
    pub fn email_replied(original_id: Uuid, reply_id: Uuid) -> EmailEventData {
        EmailEventData::new(EmailEvent::EmailReplied { original_id, reply_id })
    }
    
    pub fn email_forwarded(original_id: Uuid, forward_id: Uuid) -> EmailEventData {
        EmailEventData::new(EmailEvent::EmailForwarded { original_id, forward_id })
    }
    
    pub fn account_connected(account_id: String) -> AccountEventData {
        AccountEventData::new(AccountEvent::AccountConnected { account_id })
    }
    
    pub fn account_added(account_id: String, provider: String) -> AccountEventData {
        AccountEventData::new(AccountEvent::AccountAdded { account_id, provider })
    }
    
    pub fn account_removed(account_id: String) -> AccountEventData {
        AccountEventData::new(AccountEvent::AccountRemoved { account_id })
    }
    
    pub fn account_sync_started(account_id: String) -> AccountEventData {
        AccountEventData::new(AccountEvent::AccountSyncStarted { account_id })
    }
    
    pub fn folder_changed(account_id: String, folder_path: String) -> EmailEventData {
        EmailEventData::new(EmailEvent::FolderChanged { account_id, folder_path })
    }
    
    pub fn event_created(calendar_id: String, event_id: String) -> CalendarEventData {
        CalendarEventData::new(CalendarEvent::EventCreated { calendar_id, event_id })
    }
    
    pub fn event_updated(calendar_id: String, event_id: String) -> CalendarEventData {
        CalendarEventData::new(CalendarEvent::EventUpdated { calendar_id, event_id })
    }
    
    pub fn event_deleted(calendar_id: String, event_id: String) -> CalendarEventData {
        CalendarEventData::new(CalendarEvent::EventDeleted { calendar_id, event_id })
    }
    
    pub fn contact_added(contact_id: Uuid, name: String, email: String) -> ContactEventData {
        ContactEventData::new(ContactEvent::ContactAdded { contact_id, name, email })
    }
    
    pub fn app_shutting_down() -> AppEventData {
        AppEventData::new(AppEvent::AppShuttingDown)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_legacy_event_wrapper() {
        let result = EventResult::TriggerEmailSync;
        let wrapper = LegacyEventResultWrapper::new(result);
        
        assert_eq!(wrapper.event_type(), "LegacyEventResult");
        assert_eq!(wrapper.metadata().priority, EventPriority::High);
        assert_eq!(wrapper.metadata().source, "legacy");
    }
    
    #[test]
    fn test_event_result_converter() {
        let result = EventResult::DeleteEmail(
            "account1".to_string(),
            Uuid::new_v4(),
            "INBOX".to_string()
        );
        
        let events = EventResultConverter::convert_and_publish(&result);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type(), "EmailEvent");
    }
    
    #[test]
    fn test_event_bridge() {
        let bridge = EventBridge::new();
        let key_event = crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::NONE
        );
        
        let result = bridge.handle_key_event(&key_event, UIMode::Normal, FocusedPane::MessageList);
        assert!(matches!(result, EventResult::Handled));
    }
    
    #[test]
    fn test_legacy_event_factories() {
        let email_event = EmailEventData::new(EmailEvent::EmailDeleted {
            account_id: "account1".to_string(),
            email_id: Uuid::new_v4(),
        });
        assert_eq!(email_event.event_type(), "EmailEvent");
        
        let account_event = AccountEventData::new(AccountEvent::AccountConnected {
            account_id: "account1".to_string(),
        });
        assert_eq!(account_event.event_type(), "AccountEvent");
    }
}