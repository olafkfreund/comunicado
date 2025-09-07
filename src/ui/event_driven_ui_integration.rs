//! Event-Driven UI Integration System
//!
//! This module connects UI events to actual application functionality through
//! the event bus, creating a truly reactive user interface.

use crate::events::types::{
    AccountEvent, AccountEventData, CalendarEvent, CalendarEventData, EmailEvent, EmailEventData,
    FocusedPane, KeyEventData, UIEvent, UIEventData, UIMode,
};
use crate::events::{publish, EventError};
use crate::ui::event_driven_account::AccountAction;
use crate::ui::event_driven_calendar::CalendarAction;
use std::collections::HashMap;
use uuid::Uuid;

/// Email actions for UI integration
#[derive(Debug, Clone)]
pub enum EmailAction {
    DeleteCurrentEmail,
    ArchiveCurrentEmail,
    MarkCurrentEmailAsRead,
    MarkCurrentEmailAsUnread,
    ReplyToCurrentEmail,
    ForwardCurrentEmail,
    ComposeNewEmail,
}

/// UI Integration Manager that handles real-time event processing
pub struct EventDrivenUIIntegration {
    /// Current UI state
    current_pane: FocusedPane,
    current_mode: UIMode,
    /// Selected items for context-aware operations
    selected_email_id: Option<Uuid>,
    selected_calendar_id: Option<String>,
    selected_account_id: Option<String>,
    /// Key binding mappings
    key_bindings: HashMap<String, UIAction>,
}

/// UI Actions that can be triggered by events
#[derive(Debug, Clone)]
pub enum UIAction {
    // Email actions
    DeleteCurrentEmail,
    ArchiveCurrentEmail,
    MarkCurrentEmailRead,
    MarkCurrentEmailUnread,
    ReplyToCurrentEmail,
    ForwardCurrentEmail,
    ComposeNewEmail,

    // Calendar actions
    CreateCalendarEvent,
    EditCurrentEvent,
    DeleteCurrentEvent,
    ViewCurrentEvent,

    // Account actions
    SyncCurrentAccount,
    SwitchAccount,
    AddNewAccount,

    // Navigation actions
    SwitchPane(FocusedPane),
    SwitchMode(UIMode),
    NextItem,
    PreviousItem,

    // Search and filter
    StartSearch,
    ClearSearch,

    // System actions
    ShowHelp,
    Quit,
}

impl EventDrivenUIIntegration {
    pub fn new() -> Self {
        let mut integration = Self {
            current_pane: FocusedPane::MessageList,
            current_mode: UIMode::Normal,
            selected_email_id: None,
            selected_calendar_id: None,
            selected_account_id: None,
            key_bindings: HashMap::new(),
        };

        integration.initialize_default_keybindings();
        integration
    }

    /// Initialize default key bindings for common operations
    fn initialize_default_keybindings(&mut self) {
        // Email operations
        self.key_bindings
            .insert("d".to_string(), UIAction::DeleteCurrentEmail);
        self.key_bindings
            .insert("a".to_string(), UIAction::ArchiveCurrentEmail);
        self.key_bindings
            .insert("r".to_string(), UIAction::ReplyToCurrentEmail);
        self.key_bindings
            .insert("f".to_string(), UIAction::ForwardCurrentEmail);
        self.key_bindings
            .insert("c".to_string(), UIAction::ComposeNewEmail);
        self.key_bindings
            .insert("m".to_string(), UIAction::MarkCurrentEmailRead);
        self.key_bindings
            .insert("u".to_string(), UIAction::MarkCurrentEmailUnread);

        // Calendar operations
        self.key_bindings
            .insert("n".to_string(), UIAction::CreateCalendarEvent);
        self.key_bindings
            .insert("e".to_string(), UIAction::EditCurrentEvent);
        self.key_bindings
            .insert("Delete".to_string(), UIAction::DeleteCurrentEvent);
        self.key_bindings
            .insert("Enter".to_string(), UIAction::ViewCurrentEvent);

        // Account operations
        self.key_bindings
            .insert("s".to_string(), UIAction::SyncCurrentAccount);
        self.key_bindings
            .insert("Ctrl+a".to_string(), UIAction::AddNewAccount);

        // Navigation
        self.key_bindings
            .insert("j".to_string(), UIAction::NextItem);
        self.key_bindings
            .insert("k".to_string(), UIAction::PreviousItem);
        self.key_bindings.insert(
            "1".to_string(),
            UIAction::SwitchPane(FocusedPane::AccountList),
        );
        self.key_bindings.insert(
            "2".to_string(),
            UIAction::SwitchPane(FocusedPane::FolderTree),
        );
        self.key_bindings.insert(
            "3".to_string(),
            UIAction::SwitchPane(FocusedPane::MessageList),
        );
        self.key_bindings.insert(
            "4".to_string(),
            UIAction::SwitchPane(FocusedPane::ContentPreview),
        );

        // Search
        self.key_bindings
            .insert("/".to_string(), UIAction::StartSearch);
        self.key_bindings
            .insert("Escape".to_string(), UIAction::ClearSearch);

        // System
        self.key_bindings
            .insert("?".to_string(), UIAction::ShowHelp);
        self.key_bindings.insert("q".to_string(), UIAction::Quit);
    }

    /// Process a key event and trigger appropriate actions through the event system
    pub async fn handle_key_event(&mut self, key_data: KeyEventData) -> Result<bool, EventError> {
        // Convert key data to string representation
        let key_string = self.key_data_to_string(&key_data);

        // Look up action for this key
        if let Some(action) = self.key_bindings.get(&key_string).cloned() {
            tracing::debug!(
                "Processing UI action: {:?} from key: {}",
                action,
                key_string
            );

            // Execute the action through the event system
            self.execute_ui_action(action).await?;
            return Ok(true); // Event was handled
        }

        // Special handling for alphanumeric keys in search mode
        if self.current_mode == UIMode::Search && key_data.char.is_some() {
            self.handle_search_input(key_data.char.unwrap()).await?;
            return Ok(true);
        }

        Ok(false) // Event was not handled
    }

    /// Execute a UI action through the event system
    async fn execute_ui_action(&mut self, action: UIAction) -> Result<(), EventError> {
        match action {
            // Email operations
            UIAction::DeleteCurrentEmail => {
                self.execute_email_action(EmailAction::DeleteCurrentEmail)
                    .await?;
            }
            UIAction::ArchiveCurrentEmail => {
                self.execute_email_action(EmailAction::ArchiveCurrentEmail)
                    .await?;
            }
            UIAction::MarkCurrentEmailRead => {
                self.execute_email_action(EmailAction::MarkCurrentEmailAsRead)
                    .await?;
            }
            UIAction::MarkCurrentEmailUnread => {
                self.execute_email_action(EmailAction::MarkCurrentEmailAsUnread)
                    .await?;
            }
            UIAction::ReplyToCurrentEmail => {
                self.execute_email_action(EmailAction::ReplyToCurrentEmail)
                    .await?;
            }
            UIAction::ForwardCurrentEmail => {
                self.execute_email_action(EmailAction::ForwardCurrentEmail)
                    .await?;
            }
            UIAction::ComposeNewEmail => {
                self.execute_email_action(EmailAction::ComposeNewEmail)
                    .await?;
            }

            // Calendar operations
            UIAction::CreateCalendarEvent => {
                if let Some(calendar_id) = &self.selected_calendar_id {
                    self.execute_calendar_action(CalendarAction::CreateEvent {
                        calendar_id: calendar_id.clone(),
                        event_id: Uuid::new_v4().to_string(),
                    })
                    .await?;
                }
            }
            UIAction::EditCurrentEvent => {
                if let (Some(calendar_id), Some(event_id)) =
                    (&self.selected_calendar_id, &self.selected_email_id)
                {
                    self.execute_calendar_action(CalendarAction::UpdateEvent {
                        calendar_id: calendar_id.clone(),
                        event_id: event_id.to_string(),
                    })
                    .await?;
                }
            }
            UIAction::DeleteCurrentEvent => {
                if let (Some(calendar_id), Some(event_id)) =
                    (&self.selected_calendar_id, &self.selected_email_id)
                {
                    self.execute_calendar_action(CalendarAction::DeleteEvent {
                        calendar_id: calendar_id.clone(),
                        event_id: event_id.to_string(),
                    })
                    .await?;
                }
            }
            UIAction::ViewCurrentEvent => {
                // View current calendar event details
                tracing::debug!("View current event action triggered");
            }

            // Account operations
            UIAction::SyncCurrentAccount => {
                if let Some(account_id) = &self.selected_account_id {
                    self.execute_account_action(AccountAction::SyncAccount {
                        account_id: account_id.clone(),
                    })
                    .await?;
                }
            }
            UIAction::AddNewAccount => {
                // Trigger account addition workflow
                self.publish_ui_mode_change(UIMode::Settings).await?;
            }
            UIAction::SwitchAccount => {
                // Switch to next available account
                tracing::debug!("Switch account action triggered");
            }

            // Navigation actions
            UIAction::SwitchPane(new_pane) => {
                self.switch_pane(new_pane).await?;
            }
            UIAction::SwitchMode(new_mode) => {
                self.publish_ui_mode_change(new_mode).await?;
            }
            UIAction::NextItem => {
                self.navigate_items(1).await?;
            }
            UIAction::PreviousItem => {
                self.navigate_items(-1).await?;
            }

            // Search actions
            UIAction::StartSearch => {
                self.publish_ui_mode_change(UIMode::Search).await?;
            }
            UIAction::ClearSearch => {
                if self.current_mode == UIMode::Search {
                    self.publish_ui_mode_change(UIMode::Normal).await?;
                }
            }

            // System actions
            UIAction::ShowHelp => {
                self.publish_ui_mode_change(UIMode::Help).await?;
            }
            UIAction::Quit => {
                // Publish application shutdown event
                self.publish_app_shutdown().await?;
            }
        }

        Ok(())
    }

    /// Execute email-related actions through the event system
    async fn execute_email_action(&self, action: EmailAction) -> Result<(), EventError> {
        // This would use the EmailAction enum from event_driven_email.rs
        // For now, publish corresponding email events directly
        match action {
            EmailAction::DeleteCurrentEmail => {
                if let (Some(account_id), Some(email_id)) =
                    (&self.selected_account_id, &self.selected_email_id)
                {
                    let event = EmailEventData::new(EmailEvent::EmailDeleted {
                        account_id: account_id.clone(),
                        email_id: *email_id,
                    });
                    publish(event)?;
                }
            }
            EmailAction::MarkCurrentEmailAsRead => {
                if let (Some(account_id), Some(email_id)) =
                    (&self.selected_account_id, &self.selected_email_id)
                {
                    let event = EmailEventData::new(EmailEvent::EmailMarkedRead {
                        account_id: account_id.clone(),
                        email_id: *email_id,
                    });
                    publish(event)?;
                }
            }
            EmailAction::ComposeNewEmail => {
                let event = EmailEventData::new(EmailEvent::EmailComposed {
                    draft_id: Uuid::new_v4(),
                });
                publish(event)?;
            }
            // Add more email actions as needed
            _ => {
                tracing::debug!("Email action {:?} not yet implemented", action);
            }
        }

        Ok(())
    }

    /// Execute calendar-related actions through the event system
    async fn execute_calendar_action(&self, action: CalendarAction) -> Result<(), EventError> {
        match action {
            CalendarAction::CreateEvent {
                calendar_id,
                event_id,
            } => {
                let event = CalendarEventData::new(CalendarEvent::EventCreated {
                    calendar_id,
                    event_id,
                });
                publish(event)?;
            }
            CalendarAction::UpdateEvent {
                calendar_id,
                event_id,
            } => {
                let event = CalendarEventData::new(CalendarEvent::EventUpdated {
                    calendar_id,
                    event_id,
                });
                publish(event)?;
            }
            CalendarAction::DeleteEvent {
                calendar_id,
                event_id,
            } => {
                let event = CalendarEventData::new(CalendarEvent::EventDeleted {
                    calendar_id,
                    event_id,
                });
                publish(event)?;
            }
            _ => {
                tracing::debug!("Calendar action {:?} not yet implemented", action);
            }
        }

        Ok(())
    }

    /// Execute account-related actions through the event system
    async fn execute_account_action(&self, action: AccountAction) -> Result<(), EventError> {
        match action {
            AccountAction::SyncAccount { account_id } => {
                let event = AccountEventData::new(AccountEvent::AccountSyncStarted { account_id });
                publish(event)?;
            }
            AccountAction::AddAccount {
                account_id,
                provider,
            } => {
                let event = AccountEventData::new(AccountEvent::AccountAdded {
                    account_id,
                    provider,
                });
                publish(event)?;
            }
            _ => {
                tracing::debug!("Account action {:?} not yet implemented", action);
            }
        }

        Ok(())
    }

    /// Switch focus to a different pane
    async fn switch_pane(&mut self, new_pane: FocusedPane) -> Result<(), EventError> {
        let old_pane = self.current_pane.clone();
        self.current_pane = new_pane.clone();

        let event = UIEventData::new(UIEvent::PaneChanged {
            from: old_pane,
            to: new_pane,
        });
        publish(event)?;

        Ok(())
    }

    /// Change UI mode and publish the change
    async fn publish_ui_mode_change(&mut self, new_mode: UIMode) -> Result<(), EventError> {
        let old_mode = self.current_mode.clone();
        self.current_mode = new_mode.clone();

        let event = UIEventData::new(UIEvent::ModeChanged {
            from: old_mode,
            to: new_mode,
        });
        publish(event)?;

        Ok(())
    }

    /// Navigate through items in the current pane
    async fn navigate_items(&self, direction: i32) -> Result<(), EventError> {
        // This would trigger navigation events based on current pane
        // For now, just log the navigation
        tracing::debug!("Navigating {} items in {:?}", direction, self.current_pane);
        Ok(())
    }

    /// Handle search input
    async fn handle_search_input(&self, character: char) -> Result<(), EventError> {
        tracing::debug!("Search input: {}", character);
        // This would trigger search events
        Ok(())
    }

    /// Publish application shutdown event
    async fn publish_app_shutdown(&self) -> Result<(), EventError> {
        use crate::events::types::{AppEvent, AppEventData};
        let event = AppEventData::new(AppEvent::AppShuttingDown);
        publish(event)?;
        Ok(())
    }

    /// Convert KeyEventData to string representation for key binding lookup
    fn key_data_to_string(&self, key_data: &KeyEventData) -> String {
        let mut key_string = String::new();

        // Add modifiers
        for modifier in &key_data.modifiers {
            if !key_string.is_empty() {
                key_string.push_str("+");
            }
            key_string.push_str(modifier);
        }

        // Add the main key
        if !key_string.is_empty() {
            key_string.push_str("+");
        }

        // Use character if available, otherwise use code
        if let Some(c) = key_data.char {
            key_string.push(c);
        } else {
            // Parse key code from string representation
            let code_str = &key_data.code;
            if code_str.starts_with("Char(") && code_str.ends_with(")") {
                let char_part = &code_str[5..code_str.len() - 1];
                if char_part.len() == 3 && char_part.starts_with("'") && char_part.ends_with("'") {
                    key_string.push_str(&char_part[1..2]);
                } else {
                    key_string.push_str(code_str);
                }
            } else {
                key_string.push_str(code_str);
            }
        }

        key_string
    }

    /// Update current selections for context-aware operations
    pub fn set_selected_email(&mut self, email_id: Option<Uuid>) {
        self.selected_email_id = email_id;
    }

    pub fn set_selected_calendar(&mut self, calendar_id: Option<String>) {
        self.selected_calendar_id = calendar_id;
    }

    pub fn set_selected_account(&mut self, account_id: Option<String>) {
        self.selected_account_id = account_id;
    }

    /// Get current UI state
    pub fn current_pane(&self) -> &FocusedPane {
        &self.current_pane
    }

    pub fn current_mode(&self) -> &UIMode {
        &self.current_mode
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::initialize_event_bus;

    #[tokio::test]
    async fn test_ui_integration_key_handling() {
        let _bus = initialize_event_bus();
        let mut integration = EventDrivenUIIntegration::new();

        // Test key binding lookup
        let key_data = KeyEventData {
            code: "Char('d')".to_string(),
            modifiers: vec![],
            char: Some('d'),
        };

        let result = integration.handle_key_event(key_data).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_pane_switching() {
        let _bus = initialize_event_bus();
        let mut integration = EventDrivenUIIntegration::new();

        let result = integration.switch_pane(FocusedPane::Calendar).await;
        assert!(result.is_ok());
        assert_eq!(*integration.current_pane(), FocusedPane::Calendar);
    }
}
