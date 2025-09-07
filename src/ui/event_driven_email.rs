//! Event-Driven Email Operations
//!
//! This module provides event-driven implementations for email operations,
//! replacing direct method calls with event publishing to decouple components.

use crate::events::types::{
    AccountEvent, AccountEventData, EmailEvent, EmailEventData, SearchScope, UIEvent, UIEventData,
    UIMode as EventUIMode,
};
use crate::events::{publish, EventError};
use crate::ui::{FocusedPane, UIMode};
use uuid::Uuid;

/// Event-driven email operations handler
pub struct EventDrivenEmailHandler {
    current_account_id: Option<String>,
    selected_email_id: Option<Uuid>,
    current_folder: Option<String>,
}

impl EventDrivenEmailHandler {
    pub fn new() -> Self {
        Self {
            current_account_id: None,
            selected_email_id: None,
            current_folder: None,
        }
    }

    /// Update the current context (account, email, folder)
    pub fn update_context(
        &mut self,
        account_id: Option<String>,
        email_id: Option<Uuid>,
        folder: Option<String>,
    ) {
        self.current_account_id = account_id;
        self.selected_email_id = email_id;
        self.current_folder = folder;
    }

    /// Delete the currently selected email using events
    pub fn delete_current_email(&self) -> Result<(), EventError> {
        if let (Some(account_id), Some(email_id), Some(_folder)) = (
            &self.current_account_id,
            &self.selected_email_id,
            &self.current_folder,
        ) {
            let event = EmailEventData::new(EmailEvent::EmailDeleted {
                account_id: account_id.clone(),
                email_id: *email_id,
            });

            publish(event)?;

            // Also publish UI event for immediate feedback
            let ui_event = UIEventData::new(UIEvent::ComponentFocused {
                component_id: "message_list".to_string(),
            });
            publish(ui_event)?;

            tracing::info!(
                "Published email deletion event for email {} in account {}",
                email_id,
                account_id
            );
        } else {
            return Err(EventError::ProcessingFailed(
                "No email selected for deletion".to_string(),
            ));
        }

        Ok(())
    }

    /// Archive the currently selected email using events
    pub fn archive_current_email(&self) -> Result<(), EventError> {
        if let (Some(account_id), Some(email_id), Some(_folder)) = (
            &self.current_account_id,
            &self.selected_email_id,
            &self.current_folder,
        ) {
            let event = EmailEventData::new(EmailEvent::EmailArchived {
                account_id: account_id.clone(),
                email_id: *email_id,
            });

            publish(event)?;

            tracing::info!(
                "Published email archive event for email {} in account {}",
                email_id,
                account_id
            );
        } else {
            return Err(EventError::ProcessingFailed(
                "No email selected for archiving".to_string(),
            ));
        }

        Ok(())
    }

    /// Mark the currently selected email as read using events
    pub fn mark_current_email_read(&self) -> Result<(), EventError> {
        if let (Some(account_id), Some(email_id), Some(_folder)) = (
            &self.current_account_id,
            &self.selected_email_id,
            &self.current_folder,
        ) {
            let event = EmailEventData::new(EmailEvent::EmailMarkedRead {
                account_id: account_id.clone(),
                email_id: *email_id,
            });

            publish(event)?;

            tracing::info!(
                "Published email mark read event for email {} in account {}",
                email_id,
                account_id
            );
        } else {
            return Err(EventError::ProcessingFailed(
                "No email selected for marking read".to_string(),
            ));
        }

        Ok(())
    }

    /// Mark the currently selected email as unread using events
    pub fn mark_current_email_unread(&self) -> Result<(), EventError> {
        if let (Some(account_id), Some(email_id), Some(_folder)) = (
            &self.current_account_id,
            &self.selected_email_id,
            &self.current_folder,
        ) {
            let event = EmailEventData::new(EmailEvent::EmailMarkedUnread {
                account_id: account_id.clone(),
                email_id: *email_id,
            });

            publish(event)?;

            tracing::info!(
                "Published email mark unread event for email {} in account {}",
                email_id,
                account_id
            );
        } else {
            return Err(EventError::ProcessingFailed(
                "No email selected for marking unread".to_string(),
            ));
        }

        Ok(())
    }

    /// Toggle flag on the currently selected email using events
    pub fn toggle_current_email_flag(&self) -> Result<(), EventError> {
        if let (Some(account_id), Some(email_id), Some(_folder)) = (
            &self.current_account_id,
            &self.selected_email_id,
            &self.current_folder,
        ) {
            let event = EmailEventData::new(EmailEvent::EmailFlagged {
                account_id: account_id.clone(),
                email_id: *email_id,
            });

            publish(event)?;

            tracing::info!(
                "Published email flag toggle event for email {} in account {}",
                email_id,
                account_id
            );
        } else {
            return Err(EventError::ProcessingFailed(
                "No email selected for flag toggle".to_string(),
            ));
        }

        Ok(())
    }

    /// Start reply to the currently selected email using events
    pub fn reply_to_current_email(&self) -> Result<(), EventError> {
        if let (Some(account_id), Some(email_id)) =
            (&self.current_account_id, &self.selected_email_id)
        {
            let reply_id = Uuid::new_v4();
            let event = EmailEventData::new(EmailEvent::EmailReplied {
                original_id: *email_id,
                reply_id,
            });

            publish(event)?;

            // Also publish UI mode change event
            let ui_event = UIEventData::new(UIEvent::ModeChanged {
                from: EventUIMode::Normal,
                to: EventUIMode::Compose,
            });
            publish(ui_event)?;

            tracing::info!(
                "Published email reply event for email {} in account {}",
                email_id,
                account_id
            );
        } else {
            return Err(EventError::ProcessingFailed(
                "No email selected for reply".to_string(),
            ));
        }

        Ok(())
    }

    /// Start reply all to the currently selected email using events
    pub fn reply_all_to_current_email(&self) -> Result<(), EventError> {
        if let (Some(account_id), Some(email_id)) =
            (&self.current_account_id, &self.selected_email_id)
        {
            let reply_id = Uuid::new_v4();
            let event = EmailEventData::new(EmailEvent::EmailReplied {
                original_id: *email_id,
                reply_id,
            });

            publish(event)?;

            // Also publish UI mode change event
            let ui_event = UIEventData::new(UIEvent::ModeChanged {
                from: EventUIMode::Normal,
                to: EventUIMode::Compose,
            });
            publish(ui_event)?;

            tracing::info!(
                "Published email reply all event for email {} in account {}",
                email_id,
                account_id
            );
        } else {
            return Err(EventError::ProcessingFailed(
                "No email selected for reply all".to_string(),
            ));
        }

        Ok(())
    }

    /// Start forward of the currently selected email using events
    pub fn forward_current_email(&self) -> Result<(), EventError> {
        if let (Some(account_id), Some(email_id)) =
            (&self.current_account_id, &self.selected_email_id)
        {
            let forward_id = Uuid::new_v4();
            let event = EmailEventData::new(EmailEvent::EmailForwarded {
                original_id: *email_id,
                forward_id,
            });

            publish(event)?;

            // Also publish UI mode change event
            let ui_event = UIEventData::new(UIEvent::ModeChanged {
                from: EventUIMode::Normal,
                to: EventUIMode::Compose,
            });
            publish(ui_event)?;

            tracing::info!(
                "Published email forward event for email {} in account {}",
                email_id,
                account_id
            );
        } else {
            return Err(EventError::ProcessingFailed(
                "No email selected for forwarding".to_string(),
            ));
        }

        Ok(())
    }

    /// Switch to a different folder using events
    pub fn switch_folder(&mut self, folder_path: String) -> Result<(), EventError> {
        if let Some(account_id) = &self.current_account_id {
            let event = EmailEventData::new(EmailEvent::FolderChanged {
                account_id: account_id.clone(),
                folder_path: folder_path.clone(),
            });

            publish(event)?;

            // Update local state
            self.current_folder = Some(folder_path.clone());
            self.selected_email_id = None; // Clear selection when changing folders

            tracing::info!(
                "Published folder change event to {} in account {}",
                folder_path,
                account_id
            );
        } else {
            return Err(EventError::ProcessingFailed(
                "No account selected for folder switch".to_string(),
            ));
        }

        Ok(())
    }

    /// Trigger manual email sync using events
    pub fn trigger_email_sync(&self) -> Result<(), EventError> {
        if let Some(account_id) = &self.current_account_id {
            let event = AccountEventData::new(AccountEvent::AccountSyncStarted {
                account_id: account_id.clone(),
            });

            publish(event)?;

            tracing::info!("Published manual sync event for account {}", account_id);
        } else {
            return Err(EventError::ProcessingFailed(
                "No account selected for sync".to_string(),
            ));
        }

        Ok(())
    }

    /// Handle search operation using events
    pub fn search_emails(&self, query: String, scope: SearchScope) -> Result<(), EventError> {
        let event = EmailEventData::new(EmailEvent::SearchStarted { query, scope });

        publish(event)?;

        // Also publish UI state change
        let ui_event = UIEventData::new(UIEvent::ModeChanged {
            from: EventUIMode::Normal,
            to: EventUIMode::Search,
        });
        publish(ui_event)?;

        tracing::info!("Published email search event");
        Ok(())
    }
}

impl Default for EventDrivenEmailHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Event-driven UI state management
pub struct EventDrivenUIState {
    current_pane: FocusedPane,
    current_mode: UIMode,
    pane_history: Vec<FocusedPane>,
}

impl EventDrivenUIState {
    pub fn new() -> Self {
        Self {
            current_pane: FocusedPane::MessageList,
            current_mode: UIMode::Normal,
            pane_history: vec![FocusedPane::MessageList],
        }
    }

    /// Change the focused pane using events
    pub fn change_pane(&mut self, new_pane: FocusedPane) -> Result<(), EventError> {
        let old_pane = self.current_pane.clone();

        if old_pane != new_pane {
            // Update state
            self.pane_history.push(old_pane.clone());
            if self.pane_history.len() > 10 {
                self.pane_history.remove(0); // Keep history limited
            }
            self.current_pane = new_pane.clone();

            // TODO: Publish event with proper type conversion from UI types to Event types

            // tracing::// debug!("Published pane change event from {:?} to {:?}", old_pane, new_pane);
        }

        Ok(())
    }

    /// Change the UI mode using events
    pub fn change_mode(&mut self, new_mode: UIMode) -> Result<(), EventError> {
        let old_mode = self.current_mode.clone();

        if old_mode != new_mode {
            self.current_mode = new_mode.clone();

            // TODO: Publish event with proper type conversion from UI types to Event types

            // tracing::// debug!("Published mode change event from {:?} to {:?}", old_mode, new_mode);
        }

        Ok(())
    }

    /// Go back to the previous pane using events
    pub fn go_back(&mut self) -> Result<(), EventError> {
        if let Some(previous_pane) = self.pane_history.pop() {
            let _current_pane = self.current_pane.clone();
            self.current_pane = previous_pane.clone();

            // TODO: Publish event with proper type conversion
            // let event = UIEventData::new(UIEvent::PaneChanged {
            //     from: convert_ui_to_event_pane(current_pane),
            //     to: convert_ui_to_event_pane(previous_pane),
            // });
            // publish(event)?;

            // tracing::// debug!("Published back navigation event to {:?}", previous_pane);
        }

        Ok(())
    }

    /// Handle theme change using events
    pub fn change_theme(&self, theme_name: String) -> Result<(), EventError> {
        let event = UIEventData::new(UIEvent::ThemeChanged { theme_name });
        publish(event)?;

        tracing::info!("Published theme change event");
        Ok(())
    }

    /// Handle window resize using events
    pub fn handle_window_resize(&self, new_size: (u16, u16)) -> Result<(), EventError> {
        let event = UIEventData::new(UIEvent::WindowResized { new_size });
        publish(event)?;

        // tracing::// debug!("Published window resize event: {:?}", new_size);
        Ok(())
    }

    /// Get current state
    pub fn current_pane(&self) -> &FocusedPane {
        &self.current_pane
    }

    pub fn current_mode(&self) -> &UIMode {
        &self.current_mode
    }
}

impl Default for EventDrivenUIState {
    fn default() -> Self {
        Self::new()
    }
}

/// Integration helper for migrating from legacy event results to new event system
pub struct EventMigrationHelper;

impl EventMigrationHelper {
    /// Convert legacy command actions to event-driven operations
    pub fn handle_command_action(
        action: &crate::ui::command_palette::CommandAction,
        email_handler: &EventDrivenEmailHandler,
        _ui_state: &mut EventDrivenUIState,
    ) -> Result<(), EventError> {
        use crate::ui::command_palette::CommandAction;

        match action {
            CommandAction::DeleteEmail => {
                email_handler.delete_current_email()?;
            }
            CommandAction::ArchiveEmail => {
                email_handler.archive_current_email()?;
            }
            CommandAction::ReplyEmail => {
                email_handler.reply_to_current_email()?;
            }
            CommandAction::ReplyAllEmail => {
                email_handler.reply_all_to_current_email()?;
            }
            CommandAction::ForwardEmail => {
                email_handler.forward_current_email()?;
            }
            CommandAction::MarkAsRead => {
                email_handler.mark_current_email_read()?;
            }
            CommandAction::MarkAsUnread => {
                email_handler.mark_current_email_unread()?;
            }
            CommandAction::ShowKeyboardShortcuts => {
                // TODO: Fix type conversion - EventUIMode::Help to UIMode
                // ui_state.change_mode(UIMode::Help)?;
            }
            CommandAction::ToggleContacts => {
                // TODO: Fix type conversion and check if Contacts pane exists
                // ui_state.change_pane(FocusedPane::Contacts)?;
            }
            CommandAction::ChangeTheme => {
                // This would need the actual theme name from user selection
                // TODO: Fix theme change implementation
                // _ui_state.change_theme("default".to_string())?;
            }
            _ => {
                // tracing::// debug!("Command action {:?} not yet migrated to event system", action);
            }
        }

        Ok(())
    }

    /// Convert legacy keyboard actions to event-driven operations
    pub fn handle_keyboard_action(
        action: &crate::keyboard::KeyboardAction,
        email_handler: &EventDrivenEmailHandler,
        _ui_state: &mut EventDrivenUIState,
    ) -> Result<(), EventError> {
        use crate::keyboard::KeyboardAction;

        match action {
            KeyboardAction::DeleteEmail => {
                email_handler.delete_current_email()?;
            }
            KeyboardAction::ArchiveEmail => {
                email_handler.archive_current_email()?;
            }
            KeyboardAction::ReplyEmail => {
                email_handler.reply_to_current_email()?;
            }
            KeyboardAction::ForwardEmail => {
                email_handler.forward_current_email()?;
            }
            // Note: Focus actions and ToggleCalendar don't exist in current KeyboardAction enum
            // They would need to be added or handled through NextPane/PreviousPane
            // Note: These keyboard actions don't exist in the current KeyboardAction enum
            // They would need to be added or handled differently
            _ if matches!(action, _) => {
                // Placeholder for actions that need to be properly mapped
                // tracing::// debug!("Keyboard action {:?} not yet supported in event system", action);
            }
            _ => {
                // tracing::// debug!("Keyboard action {:?} not yet migrated to event system", action);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::initialize_event_bus;

    #[test]
    fn test_event_driven_email_handler() {
        let _bus = initialize_event_bus();

        let mut handler = EventDrivenEmailHandler::new();
        handler.update_context(
            Some("test_account".to_string()),
            Some(Uuid::new_v4()),
            Some("INBOX".to_string()),
        );

        assert!(handler.delete_current_email().is_ok());
        assert!(handler.archive_current_email().is_ok());
    }

    #[test]
    fn test_event_driven_ui_state() {
        let _bus = initialize_event_bus();

        let mut ui_state = EventDrivenUIState::new();
        assert_eq!(ui_state.current_pane(), &FocusedPane::MessageList);

        assert!(ui_state.change_pane(FocusedPane::Calendar).is_ok());
        assert_eq!(ui_state.current_pane(), &FocusedPane::Calendar);

        assert!(ui_state.go_back().is_ok());
        assert_eq!(ui_state.current_pane(), &FocusedPane::MessageList);
    }

    #[test]
    fn test_migration_helper() {
        let _bus = initialize_event_bus();

        let email_handler = EventDrivenEmailHandler::new();
        let mut ui_state = EventDrivenUIState::new();

        let action = crate::ui::command_palette::CommandAction::ShowKeyboardShortcuts;
        assert!(EventMigrationHelper::handle_command_action(
            &action,
            &email_handler,
            &mut ui_state
        )
        .is_ok());
        assert_eq!(ui_state.current_mode(), &UIMode::KeyboardShortcuts);
    }
}
