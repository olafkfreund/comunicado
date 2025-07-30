use crate::keyboard::{KeyboardAction, KeyboardManager};
use crate::ui::{ComposeAction, DraftAction, FocusedPane, StartPageNavigation, UIMode, UI};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use chrono::Datelike;

pub struct EventHandler {
    should_quit: bool,
    keyboard_manager: KeyboardManager,
}

/// Result of handling a key event
#[derive(Debug, Clone)]
pub enum EventResult {
    Continue,
    ComposeAction(ComposeAction),
    DraftAction(DraftAction),
    AccountSwitch(String),  // Account ID to switch to
    AddAccount,             // Launch account setup wizard
    RemoveAccount(String),  // Account ID to remove
    RefreshAccount(String), // Account ID to refresh connection
    SyncAccount(String),    // Account ID to manually sync
    FolderSelect(String),   // Folder path to load messages from
    FolderForceRefresh(String), // Folder path to force refresh from IMAP
    FolderOperation(crate::ui::folder_tree::FolderOperation), // Folder operation to execute
    ContactsPopup,          // Open contacts popup
    ContactsAction(crate::contacts::ContactPopupAction), // Contact popup action
    AddToContacts(String, String), // Add email address and name to contacts
    EmailViewerStarted(String), // Email address of sender for contact lookup
    ViewSenderContact(String), // View contact details for email address
    EditSenderContact(String), // Edit contact for email address
    RemoveSenderFromContacts(String), // Remove sender from contacts
    ContactQuickActions(String), // Show quick actions menu for email address
    ReplyToMessage(uuid::Uuid), // Message ID to reply to
    ReplyAllToMessage(uuid::Uuid), // Message ID to reply all to
    ForwardMessage(uuid::Uuid), // Message ID to forward
}

impl EventHandler {
    /// Create a new event handler with default keyboard configuration
    pub fn new() -> Self {
        Self {
            should_quit: false,
            keyboard_manager: KeyboardManager::default(),
        }
    }

    /// Get the keyboard manager for configuration
    pub fn keyboard_manager(&self) -> &KeyboardManager {
        &self.keyboard_manager
    }

    /// Get mutable keyboard manager for configuration
    pub fn keyboard_manager_mut(&mut self) -> &mut KeyboardManager {
        &mut self.keyboard_manager
    }

    /// Get help text for keyboard shortcuts
    pub fn get_keyboard_help(&self) -> String {
        self.keyboard_manager.get_help_text()
    }

    /// Handle a key event using the configurable keyboard system
    pub async fn handle_key_event_with_config(
        &mut self,
        key: KeyEvent,
        ui: &mut UI,
    ) -> EventResult {
        // Handle compose mode separately (these use different input handling)
        if ui.mode() == &UIMode::Compose {
            if let Some(action) = ui.handle_compose_key(key.code).await {
                return EventResult::ComposeAction(action);
            }
            return EventResult::Continue;
        }

        // Handle draft list mode
        if ui.mode() == &UIMode::DraftList {
            if let Some(action) = ui.handle_draft_list_key(key.code).await {
                return EventResult::DraftAction(action);
            }
            return EventResult::Continue;
        }

        // Handle attachment viewer mode
        if ui.focused_pane() == FocusedPane::ContentPreview
            && ui.content_preview().is_viewing_attachment()
        {
            return self.handle_attachment_viewer_keys(key, ui).await;
        }

        // Handle text input modes (search, folder search)
        if self.handle_text_input_modes(key, ui) {
            return EventResult::Continue;
        }

        // Get the action from keyboard manager
        if let Some(action) = self.keyboard_manager.get_action(key.code, key.modifiers) {
            return self.execute_keyboard_action(action.clone(), ui).await;
        }

        // Handle mode-specific keys that don't have actions
        match ui.mode() {
            UIMode::StartPage => self.handle_start_page_keys(key, ui).await,
            UIMode::EmailViewer => self.handle_email_viewer_keys(key, ui).await,
            UIMode::KeyboardShortcuts => self.handle_keyboard_shortcuts_keys(key, ui).await,
            UIMode::ContactsPopup => self.handle_contacts_popup_keys(key, ui).await,
            _ => EventResult::Continue,
        }
    }

    /// Handle text input modes (search, folder search, etc.)
    fn handle_text_input_modes(&mut self, key: KeyEvent, ui: &mut UI) -> bool {
        // Handle search input mode for folder tree
        if ui.focused_pane() == FocusedPane::FolderTree && ui.folder_tree().is_in_search_mode() {
            match key.code {
                KeyCode::Char(c) => {
                    ui.folder_tree_mut().handle_search_input(c);
                    return true;
                }
                KeyCode::Backspace => {
                    ui.folder_tree_mut().handle_search_backspace();
                    return true;
                }
                _ => {}
            }
        }

        // Handle search input mode for message list
        if ui.focused_pane() == FocusedPane::MessageList && ui.message_list().is_search_active() {
            match key.code {
                KeyCode::Char(c) => {
                    let mut current_query = ui.message_list().search_query().to_string();
                    current_query.push(c);
                    ui.message_list_mut().update_search(current_query);
                    return true;
                }
                KeyCode::Backspace => {
                    let mut current_query = ui.message_list().search_query().to_string();
                    current_query.pop();
                    ui.message_list_mut().update_search(current_query);
                    return true;
                }
                _ => {}
            }
        }

        false
    }

    /// Execute a keyboard action
    async fn execute_keyboard_action(
        &mut self,
        action: KeyboardAction,
        ui: &mut UI,
    ) -> EventResult {
        match action {
            // Global actions
            KeyboardAction::Quit => {
                self.should_quit = true;
                EventResult::Continue
            }
            KeyboardAction::ForceQuit => {
                self.should_quit = true;
                EventResult::Continue
            }
            KeyboardAction::ShowStartPage => {
                ui.show_start_page();
                EventResult::Continue
            }
            KeyboardAction::ShowKeyboardShortcuts => {
                ui.show_keyboard_shortcuts();
                EventResult::Continue
            }

            // Navigation
            KeyboardAction::NextPane => {
                ui.next_pane();
                EventResult::Continue
            }
            KeyboardAction::PreviousPane => {
                ui.previous_pane();
                EventResult::Continue
            }
            KeyboardAction::VimMoveLeft => {
                match ui.focused_pane() {
                    FocusedPane::FolderTree => {
                        ui.folder_tree_mut().handle_left();
                    }
                    _ => {
                        ui.previous_pane();
                    }
                }
                EventResult::Continue
            }
            KeyboardAction::VimMoveRight => {
                match ui.focused_pane() {
                    FocusedPane::FolderTree => {
                        ui.folder_tree_mut().handle_right();
                    }
                    _ => {
                        ui.next_pane();
                    }
                }
                EventResult::Continue
            }
            KeyboardAction::VimMoveDown | KeyboardAction::MoveDown => {
                self.handle_move_down(ui);
                EventResult::Continue
            }
            KeyboardAction::VimMoveUp | KeyboardAction::MoveUp => {
                self.handle_move_up(ui);
                EventResult::Continue
            }

            // Selection and interaction
            KeyboardAction::Select => self.handle_select(ui),
            KeyboardAction::Escape => {
                self.handle_escape(ui);
                EventResult::Continue
            }
            KeyboardAction::ToggleExpanded => {
                match ui.focused_pane() {
                    FocusedPane::AccountSwitcher => {
                        ui.account_switcher_mut().toggle_expanded();
                    }
                    FocusedPane::MessageList => {
                        ui.message_list_mut().toggle_selected_thread();
                    }
                    _ => {}
                }
                EventResult::Continue
            }

            // Email actions
            KeyboardAction::ComposeEmail => {
                if !ui.is_composing() {
                    EventResult::ComposeAction(ComposeAction::StartCompose)
                } else {
                    EventResult::Continue
                }
            }
            KeyboardAction::ShowDraftList => {
                if !ui.is_draft_list_visible() && !ui.is_composing() {
                    EventResult::DraftAction(DraftAction::RefreshDrafts)
                } else {
                    EventResult::Continue
                }
            }
            KeyboardAction::ReplyEmail => {
                if matches!(ui.focused_pane(), FocusedPane::MessageList | FocusedPane::ContentPreview) {
                    if let Some(message) = ui.message_list().selected_message() {
                        if let Some(message_id) = message.message_id {
                            EventResult::ReplyToMessage(message_id)
                        } else {
                            EventResult::Continue
                        }
                    } else {
                        EventResult::Continue
                    }
                } else {
                    EventResult::Continue
                }
            }
            KeyboardAction::ReplyAllEmail => {
                if matches!(ui.focused_pane(), FocusedPane::MessageList | FocusedPane::ContentPreview) {
                    if let Some(message) = ui.message_list().selected_message() {
                        if let Some(message_id) = message.message_id {
                            EventResult::ReplyAllToMessage(message_id)
                        } else {
                            EventResult::Continue
                        }
                    } else {
                        EventResult::Continue
                    }
                } else {
                    EventResult::Continue
                }
            }
            KeyboardAction::ForwardEmail => {
                if matches!(ui.focused_pane(), FocusedPane::MessageList | FocusedPane::ContentPreview) {
                    if let Some(message) = ui.message_list().selected_message() {
                        if let Some(message_id) = message.message_id {
                            EventResult::ForwardMessage(message_id)
                        } else {
                            EventResult::Continue
                        }
                    } else {
                        EventResult::Continue
                    }
                } else {
                    EventResult::Continue
                }
            }
            KeyboardAction::DeleteEmail => {
                if matches!(ui.focused_pane(), FocusedPane::MessageList | FocusedPane::ContentPreview) {
                    if let Some(message) = ui.message_list().selected_message() {
                        // Mark message for deletion - this would need to be handled by the main app loop
                        tracing::info!("Delete email action triggered for message: {}", message.subject);
                        // TODO: Implement actual email deletion
                        EventResult::Continue
                    } else {
                        EventResult::Continue
                    }
                } else {
                    EventResult::Continue
                }
            }
            KeyboardAction::ArchiveEmail => {
                if matches!(ui.focused_pane(), FocusedPane::MessageList | FocusedPane::ContentPreview) {
                    if let Some(message) = ui.message_list().selected_message() {
                        tracing::info!("Archive email action triggered for message: {}", message.subject);
                        // TODO: Implement actual email archiving
                        EventResult::Continue
                    } else {
                        EventResult::Continue
                    }
                } else {
                    EventResult::Continue
                }
            }
            KeyboardAction::MarkAsRead => {
                if matches!(ui.focused_pane(), FocusedPane::MessageList | FocusedPane::ContentPreview) {
                    if let Some(message) = ui.message_list().selected_message() {
                        tracing::info!("Mark as read action triggered for message: {}", message.subject);
                        // TODO: Implement mark as read
                        EventResult::Continue
                    } else {
                        EventResult::Continue
                    }
                } else {
                    EventResult::Continue
                }
            }
            KeyboardAction::MarkAsUnread => {
                if matches!(ui.focused_pane(), FocusedPane::MessageList | FocusedPane::ContentPreview) {
                    if let Some(message) = ui.message_list().selected_message() {
                        tracing::info!("Mark as unread action triggered for message: {}", message.subject);
                        // TODO: Implement mark as unread
                        EventResult::Continue
                    } else {
                        EventResult::Continue
                    }
                } else {
                    EventResult::Continue
                }
            }
            KeyboardAction::NextMessage => {
                if matches!(ui.focused_pane(), FocusedPane::MessageList | FocusedPane::ContentPreview) {
                    ui.message_list_mut().handle_down();
                }
                EventResult::Continue
            }
            KeyboardAction::PreviousMessage => {
                if matches!(ui.focused_pane(), FocusedPane::MessageList | FocusedPane::ContentPreview) {
                    ui.message_list_mut().handle_up();
                }
                EventResult::Continue
            }

            // Account management
            KeyboardAction::AddAccount => EventResult::AddAccount,
            KeyboardAction::RemoveAccount => {
                if matches!(ui.focused_pane(), FocusedPane::AccountSwitcher) {
                    if let Some(account_id) = ui.account_switcher().get_current_account_id() {
                        EventResult::RemoveAccount(account_id.clone())
                    } else {
                        EventResult::Continue
                    }
                } else {
                    EventResult::Continue
                }
            }
            KeyboardAction::RefreshAccount => {
                if matches!(ui.focused_pane(), FocusedPane::AccountSwitcher) {
                    if let Some(account_id) = ui.account_switcher().get_current_account_id() {
                        EventResult::RefreshAccount(account_id.clone())
                    } else {
                        EventResult::Continue
                    }
                } else {
                    EventResult::Continue
                }
            }

            // Search
            KeyboardAction::StartSearch => {
                if let FocusedPane::MessageList = ui.focused_pane() {
                    if !ui.message_list().is_search_active() {
                        ui.message_list_mut().start_search();
                    }
                }
                EventResult::Continue
            }
            KeyboardAction::StartFolderSearch => {
                if let FocusedPane::FolderTree = ui.focused_pane() {
                    if !ui.folder_tree().is_in_search_mode() {
                        ui.folder_tree_mut().enter_search_mode();
                    }
                }
                EventResult::Continue
            }

            // View controls
            KeyboardAction::ToggleThreadedView => {
                if let FocusedPane::MessageList = ui.focused_pane() {
                    ui.message_list_mut().toggle_view_mode();
                }
                EventResult::Continue
            }
            KeyboardAction::ExpandThread => {
                if let FocusedPane::MessageList = ui.focused_pane() {
                    ui.message_list_mut().expand_selected_thread();
                }
                EventResult::Continue
            }
            KeyboardAction::CollapseThread => {
                if let FocusedPane::MessageList = ui.focused_pane() {
                    ui.message_list_mut().collapse_selected_thread();
                }
                EventResult::Continue
            }
            KeyboardAction::ToggleViewMode => {
                if let FocusedPane::ContentPreview = ui.focused_pane() {
                    ui.content_preview_mut().toggle_view_mode();
                }
                EventResult::Continue
            }
            KeyboardAction::ToggleHeaders => {
                if let FocusedPane::ContentPreview = ui.focused_pane() {
                    ui.content_preview_mut().toggle_headers();
                }
                EventResult::Continue
            }

            // Sorting
            KeyboardAction::SortByDate => {
                if let FocusedPane::MessageList = ui.focused_pane() {
                    use crate::email::{SortCriteria, SortOrder};
                    ui.message_list_mut()
                        .set_sort_criteria(SortCriteria::Date(SortOrder::Descending));
                }
                EventResult::Continue
            }
            KeyboardAction::SortBySender => {
                if let FocusedPane::MessageList = ui.focused_pane() {
                    use crate::email::{SortCriteria, SortOrder};
                    ui.message_list_mut()
                        .set_sort_criteria(SortCriteria::Sender(SortOrder::Ascending));
                }
                EventResult::Continue
            }
            KeyboardAction::SortBySubject => {
                if let FocusedPane::MessageList = ui.focused_pane() {
                    use crate::email::{SortCriteria, SortOrder};
                    ui.message_list_mut()
                        .set_sort_criteria(SortCriteria::Subject(SortOrder::Ascending));
                }
                EventResult::Continue
            }

            // Content preview
            KeyboardAction::ScrollToTop => {
                if let FocusedPane::ContentPreview = ui.focused_pane() {
                    ui.content_preview_mut().scroll_to_top();
                }
                EventResult::Continue
            }
            KeyboardAction::ScrollToBottom => {
                if let FocusedPane::ContentPreview = ui.focused_pane() {
                    ui.content_preview_mut().scroll_to_bottom(20);
                }
                EventResult::Continue
            }
            KeyboardAction::SelectFirstAttachment => {
                if let FocusedPane::ContentPreview = ui.focused_pane() {
                    if ui.content_preview().has_attachments() {
                        ui.content_preview_mut().select_first_attachment();
                    }
                }
                EventResult::Continue
            }
            KeyboardAction::ViewAttachment => {
                if let FocusedPane::ContentPreview = ui.focused_pane() {
                    if ui.content_preview().has_attachments() {
                        if let Some(_attachment) = ui.content_preview().get_selected_attachment() {
                            if let Err(e) =
                                ui.content_preview_mut().view_selected_attachment().await
                            {
                                tracing::error!("Failed to view attachment: {}", e);
                            }
                        }
                    }
                }
                EventResult::Continue
            }
            KeyboardAction::OpenAttachmentWithSystem => {
                if let FocusedPane::ContentPreview = ui.focused_pane() {
                    if ui.content_preview().has_attachments() {
                        if let Some(_attachment) = ui.content_preview().get_selected_attachment() {
                            if let Err(e) =
                                ui.content_preview_mut().open_attachment_with_system().await
                            {
                                tracing::error!(
                                    "Failed to open attachment with system application: {}",
                                    e
                                );
                            }
                        }
                    }
                }
                EventResult::Continue
            }

            // Folder operations
            KeyboardAction::CreateFolder => {
                if let FocusedPane::FolderTree = ui.focused_pane() {
                    let parent_path = ui.folder_tree().selected_folder().map(|f| f.path.clone());
                    if let Some(parent_path) = parent_path {
                        let _ = ui
                            .folder_tree_mut()
                            .create_folder(Some(&parent_path), "New Folder".to_string());
                    }
                }
                EventResult::Continue
            }
            KeyboardAction::DeleteFolder => {
                if matches!(ui.focused_pane(), FocusedPane::FolderTree) {
                    let folder_path = ui.folder_tree().selected_folder().map(|f| f.path.clone());
                    if let Some(path) = folder_path {
                        let _ = ui.folder_tree_mut().delete_folder(&path);
                    }
                }
                EventResult::Continue
            }
            KeyboardAction::RefreshFolder => {
                if let FocusedPane::FolderTree = ui.focused_pane() {
                    let folder_path = ui.folder_tree().selected_folder().map(|f| f.path.clone());
                    if let Some(path) = folder_path {
                        ui.folder_tree_mut().refresh_folder(&path);
                        ui.folder_tree_mut().mark_folder_synced(&path, 0, 42);
                    }
                }
                EventResult::Continue
            }
            KeyboardAction::FolderRefresh => match ui.focused_pane() {
                FocusedPane::FolderTree => {
                    if let Some(operation) = ui.folder_tree_mut().handle_function_key(KeyCode::F(5))
                    {
                        EventResult::FolderOperation(operation)
                    } else {
                        EventResult::Continue
                    }
                }
                _ => EventResult::Continue,
            },
            KeyboardAction::FolderRename => match ui.focused_pane() {
                FocusedPane::FolderTree => {
                    if let Some(operation) = ui.folder_tree_mut().handle_function_key(KeyCode::F(2))
                    {
                        EventResult::FolderOperation(operation)
                    } else {
                        EventResult::Continue
                    }
                }
                _ => EventResult::Continue,
            },
            KeyboardAction::FolderDelete => match ui.focused_pane() {
                FocusedPane::FolderTree => {
                    if let Some(operation) =
                        ui.folder_tree_mut().handle_function_key(KeyCode::Delete)
                    {
                        EventResult::FolderOperation(operation)
                    } else {
                        EventResult::Continue
                    }
                }
                _ => EventResult::Continue,
            },

            // Copy operations
            KeyboardAction::CopyEmailContent => {
                if matches!(ui.focused_pane(), FocusedPane::ContentPreview) {
                    if let Err(e) = ui.content_preview_mut().copy_email_content() {
                        tracing::error!("Failed to copy email content: {}", e);
                    }
                }
                EventResult::Continue
            }
            KeyboardAction::CopyAttachmentInfo => {
                if matches!(ui.focused_pane(), FocusedPane::ContentPreview) {
                    if let Err(e) = ui.content_preview_mut().copy_attachment_info() {
                        tracing::error!("Failed to copy attachment info: {}", e);
                    }
                }
                EventResult::Continue
            }

            // Attachment navigation
            KeyboardAction::NextAttachment => {
                if ui.focused_pane() == FocusedPane::ContentPreview
                    && ui.content_preview().has_attachments()
                {
                    ui.content_preview_mut().next_attachment();
                }
                EventResult::Continue
            }
            KeyboardAction::PreviousAttachment => {
                if ui.focused_pane() == FocusedPane::ContentPreview
                    && ui.content_preview().has_attachments()
                {
                    ui.content_preview_mut().previous_attachment();
                }
                EventResult::Continue
            }

            // Contacts actions
            KeyboardAction::ContactsPopup => {
                EventResult::ContactsPopup
            }
            KeyboardAction::ViewSenderContact => {
                self.handle_sender_contact_action(ui, |email| EventResult::ViewSenderContact(email))
            }
            KeyboardAction::EditSenderContact => {
                self.handle_sender_contact_action(ui, |email| EventResult::EditSenderContact(email))
            }
            KeyboardAction::AddSenderToContacts => {
                self.handle_sender_contact_action(ui, |email| {
                    // Extract name from the sender field if available
                    let name = Self::extract_name_from_sender(&email).unwrap_or_else(|| email.clone());
                    EventResult::AddToContacts(email, name)
                })
            }
            KeyboardAction::RemoveSenderFromContacts => {
                self.handle_sender_contact_action(ui, |email| EventResult::RemoveSenderFromContacts(email))
            }
            KeyboardAction::ContactQuickActions => {
                self.handle_sender_contact_action(ui, |email| EventResult::ContactQuickActions(email))
            }

            // Calendar actions
            KeyboardAction::ShowCalendar => {
                ui.show_calendar();
                EventResult::Continue
            }
            KeyboardAction::CreateEvent => {
                if ui.mode() == &UIMode::Calendar {
                    // TODO: Implement event creation - for now just show a notification
                    tracing::info!("Create event action triggered in calendar mode");
                } else {
                    ui.show_calendar();
                    tracing::info!("Switched to calendar and triggered create event");
                }
                EventResult::Continue
            }
            KeyboardAction::EditEvent => {
                if ui.mode() == &UIMode::Calendar {
                    // TODO: Implement event editing
                    tracing::info!("Edit event action triggered in calendar mode");
                }
                EventResult::Continue
            }
            KeyboardAction::DeleteEvent => {
                if ui.mode() == &UIMode::Calendar {
                    // TODO: Implement event deletion
                    tracing::info!("Delete event action triggered in calendar mode");
                }
                EventResult::Continue
            }
            KeyboardAction::ViewEventDetails => {
                if ui.mode() == &UIMode::Calendar {
                    // TODO: Implement event details view
                    tracing::info!("View event details action triggered in calendar mode");
                }
                EventResult::Continue
            }
            KeyboardAction::CreateTodo => {
                if ui.mode() == &UIMode::Calendar {
                    // TODO: Implement todo creation
                    tracing::info!("Create todo action triggered in calendar mode");
                } else {
                    ui.show_calendar();
                    tracing::info!("Switched to calendar and triggered create todo");
                }
                EventResult::Continue
            }
            KeyboardAction::ToggleTodoComplete => {
                if ui.mode() == &UIMode::Calendar {
                    // TODO: Implement todo toggle
                    tracing::info!("Toggle todo complete action triggered in calendar mode");
                }
                EventResult::Continue
            }
            KeyboardAction::ViewTodos => {
                if ui.mode() == &UIMode::Calendar {
                    // TODO: Implement todos view
                    tracing::info!("View todos action triggered in calendar mode");
                } else {
                    ui.show_calendar();
                    tracing::info!("Switched to calendar and triggered view todos");
                }
                EventResult::Continue
            }
            KeyboardAction::CalendarNextMonth => {
                if ui.mode() == &UIMode::Calendar {
                    // Navigate to next month using existing date logic
                    let current_date = ui.calendar_ui_mut().selected_date();
                    let next_month = if current_date.month() == 12 {
                        chrono::NaiveDate::from_ymd_opt(current_date.year() + 1, 1, 1).unwrap_or(current_date)
                    } else {
                        chrono::NaiveDate::from_ymd_opt(current_date.year(), current_date.month() + 1, 1).unwrap_or(current_date)
                    };
                    ui.calendar_ui_mut().set_selected_date(next_month);
                    tracing::info!("Calendar navigated to next month: {}", next_month);
                }
                EventResult::Continue
            }
            KeyboardAction::CalendarPrevMonth => {
                if ui.mode() == &UIMode::Calendar {
                    // Navigate to previous month using existing date logic
                    let current_date = ui.calendar_ui_mut().selected_date();
                    let prev_month = if current_date.month() == 1 {
                        chrono::NaiveDate::from_ymd_opt(current_date.year() - 1, 12, 1).unwrap_or(current_date)
                    } else {
                        chrono::NaiveDate::from_ymd_opt(current_date.year(), current_date.month() - 1, 1).unwrap_or(current_date)
                    };
                    ui.calendar_ui_mut().set_selected_date(prev_month);
                    tracing::info!("Calendar navigated to previous month: {}", prev_month);
                }
                EventResult::Continue
            }
            KeyboardAction::CalendarToday => {
                if ui.mode() == &UIMode::Calendar {
                    ui.calendar_ui_mut().navigate_to_today();
                }
                EventResult::Continue
            }
            KeyboardAction::CalendarWeekView => {
                if ui.mode() == &UIMode::Calendar {
                    ui.calendar_ui_mut().set_view_mode(crate::ui::CalendarViewMode::Week);
                }
                EventResult::Continue
            }
            KeyboardAction::CalendarMonthView => {
                if ui.mode() == &UIMode::Calendar {
                    ui.calendar_ui_mut().set_view_mode(crate::ui::CalendarViewMode::Month);
                }
                EventResult::Continue
            }
            KeyboardAction::CalendarDayView => {
                if ui.mode() == &UIMode::Calendar {
                    ui.calendar_ui_mut().set_view_mode(crate::ui::CalendarViewMode::Day);
                }
                EventResult::Continue
            }
            KeyboardAction::CalendarAgendaView => {
                if ui.mode() == &UIMode::Calendar {
                    ui.calendar_ui_mut().set_view_mode(crate::ui::CalendarViewMode::Agenda);
                }
                EventResult::Continue
            }

            // Other actions that need specific handling
            _ => {
                tracing::debug!("Unhandled keyboard action: {:?}", action);
                EventResult::Continue
            }
        }
    }

    /// Handle sender contact actions - get sender email from current context
    fn handle_sender_contact_action<F>(&self, ui: &UI, action_fn: F) -> EventResult
    where
        F: FnOnce(String) -> EventResult,
    {
        // Check if we're in email viewer mode with sender info
        if ui.mode() == &UIMode::EmailViewer {
            if let Some(sender_email) = ui.email_viewer().get_sender_email() {
                return action_fn(sender_email);
            }
        }

        // Check if we're in message list with a selected message
        if matches!(ui.focused_pane(), FocusedPane::MessageList | FocusedPane::ContentPreview) {
            if let Some(message) = ui.message_list().selected_message() {
                // Extract email from sender field
                let sender_email = Self::extract_email_from_sender(&message.sender);
                return action_fn(sender_email);
            }
        }

        tracing::debug!("No sender context available for contact action");
        EventResult::Continue
    }

    /// Extract email address from sender field (handles "Name <email>" format)
    fn extract_email_from_sender(sender: &str) -> String {
        if let Some(start) = sender.find('<') {
            if let Some(end) = sender.find('>') {
                return sender[start + 1..end].to_string();
            }
        }
        sender.to_string()
    }

    /// Extract name from sender field (handles "Name <email>" format)
    fn extract_name_from_sender(sender: &str) -> Option<String> {
        if sender.contains('<') {
            let parts: Vec<&str> = sender.split('<').collect();
            if !parts.is_empty() {
                let name = parts[0].trim().trim_matches('"');
                if !name.is_empty() && name != sender {
                    return Some(name.to_string());
                }
            }
        }
        None
    }

    /// Handle attachment viewer key events
    async fn handle_attachment_viewer_keys(&mut self, key: KeyEvent, ui: &mut UI) -> EventResult {
        match key.code {
            KeyCode::Esc => {
                ui.content_preview_mut().close_attachment_viewer();
                EventResult::Continue
            }
            KeyCode::Up | KeyCode::Char('k') => {
                ui.content_preview_mut().handle_up();
                EventResult::Continue
            }
            KeyCode::Down | KeyCode::Char('j') => {
                ui.content_preview_mut().handle_down();
                EventResult::Continue
            }
            KeyCode::Home => {
                ui.content_preview_mut().scroll_to_top();
                EventResult::Continue
            }
            KeyCode::End => {
                ui.content_preview_mut().scroll_to_bottom(20);
                EventResult::Continue
            }
            KeyCode::Char(c) => {
                if let Err(e) = ui
                    .content_preview_mut()
                    .handle_attachment_viewer_key(c)
                    .await
                {
                    tracing::error!("Error handling attachment viewer key: {}", e);
                }
                EventResult::Continue
            }
            _ => EventResult::Continue,
        }
    }

    /// Handle move down action for different panes
    fn handle_move_down(&mut self, ui: &mut UI) {
        match ui.focused_pane() {
            FocusedPane::AccountSwitcher => {
                ui.account_switcher_mut().next_account();
            }
            FocusedPane::FolderTree => {
                ui.folder_tree_mut().handle_down();
            }
            FocusedPane::MessageList => {
                ui.message_list_mut().handle_down();
            }
            FocusedPane::ContentPreview => {
                ui.content_preview_mut().handle_down();
            }
            _ => {}
        }
    }

    /// Handle move up action for different panes
    fn handle_move_up(&mut self, ui: &mut UI) {
        match ui.focused_pane() {
            FocusedPane::AccountSwitcher => {
                ui.account_switcher_mut().previous_account();
            }
            FocusedPane::FolderTree => {
                ui.folder_tree_mut().handle_up();
            }
            FocusedPane::MessageList => {
                ui.message_list_mut().handle_up();
            }
            FocusedPane::ContentPreview => {
                ui.content_preview_mut().handle_up();
            }
            _ => {}
        }
    }

    /// Handle select action for different panes
    fn handle_select(&mut self, ui: &mut UI) -> EventResult {
        match ui.focused_pane() {
            FocusedPane::AccountSwitcher => {
                if let Some(account_id) = ui.account_switcher_mut().select_current() {
                    tracing::info!("Account selected: {}", account_id);
                    EventResult::AccountSwitch(account_id)
                } else {
                    EventResult::Continue
                }
            }
            FocusedPane::FolderTree => {
                if ui.folder_tree().is_in_search_mode() {
                    ui.folder_tree_mut().exit_search_mode(true);
                    EventResult::Continue
                } else {
                    if let Some(folder_path) = ui.folder_tree_mut().handle_enter() {
                        EventResult::FolderSelect(folder_path)
                    } else {
                        EventResult::Continue
                    }
                }
            }
            FocusedPane::MessageList => {
                ui.message_list_mut().handle_enter();
                EventResult::Continue
            }
            _ => EventResult::Continue,
        }
    }

    /// Handle escape action for different panes
    fn handle_escape(&mut self, ui: &mut UI) {
        match ui.focused_pane() {
            FocusedPane::FolderTree => {
                if ui.folder_tree().is_in_search_mode() {
                    ui.folder_tree_mut().exit_search_mode(false);
                } else {
                    ui.folder_tree_mut().handle_escape();
                }
            }
            FocusedPane::MessageList => {
                if ui.message_list().is_search_active() {
                    ui.message_list_mut().end_search();
                }
            }
            FocusedPane::ContentPreview => {
                if ui.content_preview().is_viewing_attachment() {
                    ui.content_preview_mut().close_attachment_viewer();
                }
            }
            _ => {}
        }
    }

    pub async fn handle_key_event(&mut self, key: KeyEvent, ui: &mut UI) -> EventResult {
        // Handle compose mode separately
        if ui.mode() == &UIMode::Compose {
            if let Some(action) = ui.handle_compose_key(key.code).await {
                return EventResult::ComposeAction(action);
            }
            return EventResult::Continue;
        }

        // Handle draft list mode
        if ui.mode() == &UIMode::DraftList {
            if let Some(action) = ui.handle_draft_list_key(key.code).await {
                return EventResult::DraftAction(action);
            }
            return EventResult::Continue;
        }

        // Handle start page mode
        if ui.mode() == &UIMode::StartPage {
            return self.handle_start_page_keys(key, ui).await;
        }

        // Handle email viewer mode
        if ui.mode() == &UIMode::EmailViewer {
            return self.handle_email_viewer_keys(key, ui).await;
        }

        // Handle attachment viewer mode when it's active in content preview
        if ui.focused_pane() == FocusedPane::ContentPreview
            && ui.content_preview().is_viewing_attachment()
        {
            // Route key handling to attachment viewer
            match key.code {
                KeyCode::Esc => {
                    ui.content_preview_mut().close_attachment_viewer();
                    return EventResult::Continue;
                }
                KeyCode::Up => {
                    ui.content_preview_mut().handle_up();
                    return EventResult::Continue;
                }
                KeyCode::Down => {
                    ui.content_preview_mut().handle_down();
                    return EventResult::Continue;
                }
                KeyCode::Char('k') => {
                    ui.content_preview_mut().handle_up();
                    return EventResult::Continue;
                }
                KeyCode::Char('j') => {
                    ui.content_preview_mut().handle_down();
                    return EventResult::Continue;
                }
                KeyCode::Home => {
                    ui.content_preview_mut().scroll_to_top();
                    return EventResult::Continue;
                }
                KeyCode::End => {
                    ui.content_preview_mut().scroll_to_bottom(20); // Default height
                    return EventResult::Continue;
                }
                KeyCode::Char(c) => {
                    if let Err(e) = ui
                        .content_preview_mut()
                        .handle_attachment_viewer_key(c)
                        .await
                    {
                        tracing::error!("Error handling attachment viewer key: {}", e);
                    }
                    return EventResult::Continue;
                }
                _ => {
                    // Other keys are ignored in attachment viewer mode
                    return EventResult::Continue;
                }
            }
        }

        match key.code {
            // Global quit commands
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }

            // Handle search input mode for folder tree
            KeyCode::Char(c)
                if ui.focused_pane() == FocusedPane::FolderTree
                    && ui.folder_tree().is_in_search_mode() =>
            {
                ui.folder_tree_mut().handle_search_input(c);
                return EventResult::Continue;
            }
            KeyCode::Backspace
                if ui.focused_pane() == FocusedPane::FolderTree
                    && ui.folder_tree().is_in_search_mode() =>
            {
                ui.folder_tree_mut().handle_search_backspace();
                return EventResult::Continue;
            }

            // Handle search input mode for message list
            KeyCode::Char(c)
                if ui.focused_pane() == FocusedPane::MessageList
                    && ui.message_list().is_search_active() =>
            {
                let mut current_query = ui.message_list().search_query().to_string();
                current_query.push(c);
                ui.message_list_mut().update_search(current_query);
                return EventResult::Continue;
            }
            KeyCode::Backspace
                if ui.focused_pane() == FocusedPane::MessageList
                    && ui.message_list().is_search_active() =>
            {
                let mut current_query = ui.message_list().search_query().to_string();
                current_query.pop();
                ui.message_list_mut().update_search(current_query);
                return EventResult::Continue;
            }

            // Go back to start page
            KeyCode::Char('~') => {
                ui.show_start_page();
            }

            // Navigation between panes
            KeyCode::Tab => {
                ui.next_pane();
            }
            KeyCode::BackTab => {
                ui.previous_pane();
            }

            // Vim-style navigation
            KeyCode::Char('h') => {
                match ui.focused_pane() {
                    FocusedPane::FolderTree => {
                        // Handle left movement in folder tree (collapse)
                        ui.folder_tree_mut().handle_left();
                    }
                    _ => {
                        // Move to previous pane
                        ui.previous_pane();
                    }
                }
            }
            KeyCode::Char('l') => {
                match ui.focused_pane() {
                    FocusedPane::FolderTree => {
                        // Handle right movement in folder tree (expand)
                        ui.folder_tree_mut().handle_right();
                    }
                    _ => {
                        // Move to next pane
                        ui.next_pane();
                    }
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                // Move down in current pane
                match ui.focused_pane() {
                    FocusedPane::AccountSwitcher => {
                        ui.account_switcher_mut().next_account();
                    }
                    FocusedPane::FolderTree => {
                        ui.folder_tree_mut().handle_down();
                    }
                    FocusedPane::MessageList => {
                        ui.message_list_mut().handle_down();
                    }
                    FocusedPane::ContentPreview => {
                        // Check if there are attachments and if any are selected
                        if ui.content_preview().has_attachments()
                            && ui.content_preview().get_selected_attachment().is_some()
                            && key.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            // Ctrl+J: Navigate to next attachment
                            ui.content_preview_mut().next_attachment();
                        } else {
                            // Regular scroll down
                            ui.content_preview_mut().handle_down();
                        }
                    }
                    FocusedPane::Compose => {
                        // Handled separately in compose mode
                    }
                    FocusedPane::StartPage => {
                        // Should not happen in normal mode
                    }
                    FocusedPane::DraftList => {
                        // Draft list navigation handled in draft list mode
                    }
                    FocusedPane::Calendar => {
                        // Calendar navigation handled in calendar mode
                    }
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                // Move up in current pane
                match ui.focused_pane() {
                    FocusedPane::AccountSwitcher => {
                        ui.account_switcher_mut().previous_account();
                    }
                    FocusedPane::FolderTree => {
                        ui.folder_tree_mut().handle_up();
                    }
                    FocusedPane::MessageList => {
                        ui.message_list_mut().handle_up();
                    }
                    FocusedPane::ContentPreview => {
                        // Check if there are attachments and if any are selected
                        if ui.content_preview().has_attachments()
                            && ui.content_preview().get_selected_attachment().is_some()
                            && key.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            // Ctrl+K: Navigate to previous attachment
                            ui.content_preview_mut().previous_attachment();
                        } else {
                            // Regular scroll up
                            ui.content_preview_mut().handle_up();
                        }
                    }
                    FocusedPane::Compose => {
                        // Handled separately in compose mode
                    }
                    FocusedPane::StartPage => {
                        // Should not happen in normal mode
                    }
                    FocusedPane::DraftList => {
                        // Draft list navigation handled in draft list mode
                    }
                    FocusedPane::Calendar => {
                        // Calendar navigation handled in calendar mode
                    }
                }
            }

            // Escape key handling
            KeyCode::Esc => {
                match ui.focused_pane() {
                    FocusedPane::FolderTree => {
                        // Check if in search mode first
                        if ui.folder_tree().is_in_search_mode() {
                            ui.folder_tree_mut().exit_search_mode(false); // Cancel search
                        } else {
                            ui.folder_tree_mut().handle_escape();
                        }
                    }
                    FocusedPane::MessageList => {
                        // Check if in search mode first
                        if ui.message_list().is_search_active() {
                            ui.message_list_mut().end_search(); // Cancel search
                        }
                    }
                    FocusedPane::ContentPreview => {
                        // Close attachment viewer if open
                        if ui.content_preview().is_viewing_attachment() {
                            ui.content_preview_mut().close_attachment_viewer();
                        }
                    }
                    _ => {}
                }
            }

            // Function keys for folder operations
            KeyCode::F(5) => {
                // F5 - Refresh
                match ui.focused_pane() {
                    FocusedPane::FolderTree => {
                        if let Some(operation) = ui.folder_tree_mut().handle_function_key(key.code)
                        {
                            return EventResult::FolderOperation(operation);
                        }
                    }
                    _ => {
                        // Global F5 refresh
                        // TODO: Add global refresh functionality
                    }
                }
            }
            KeyCode::F(3) => {
                // F3 - Calendar (global access)
                ui.show_calendar();
            }
            KeyCode::F(2) => {
                // F2 - Rename
                match ui.focused_pane() {
                    FocusedPane::FolderTree => {
                        if let Some(operation) = ui.folder_tree_mut().handle_function_key(key.code)
                        {
                            return EventResult::FolderOperation(operation);
                        }
                    }
                    _ => {}
                }
            }
            KeyCode::Delete => {
                // Delete key
                match ui.focused_pane() {
                    FocusedPane::FolderTree => {
                        if let Some(operation) = ui.folder_tree_mut().handle_function_key(key.code)
                        {
                            return EventResult::FolderOperation(operation);
                        }
                    }
                    _ => {}
                }
            }

            // Enter key for selection
            KeyCode::Enter => {
                match ui.focused_pane() {
                    FocusedPane::AccountSwitcher => {
                        if let Some(account_id) = ui.account_switcher_mut().select_current() {
                            // Return account switch event to be handled by main loop
                            tracing::info!("Account selected: {}", account_id);
                            return EventResult::AccountSwitch(account_id);
                        }
                    }
                    FocusedPane::FolderTree => {
                        // Check if in search mode first
                        if ui.folder_tree().is_in_search_mode() {
                            ui.folder_tree_mut().exit_search_mode(true); // Apply search
                        } else {
                            if let Some(folder_path) = ui.folder_tree_mut().handle_enter() {
                                return EventResult::FolderSelect(folder_path);
                            }
                        }
                    }
                    FocusedPane::MessageList => {
                        ui.message_list_mut().handle_enter();
                    }
                    FocusedPane::ContentPreview => {
                        // Maybe handle links or attachments in the future
                    }
                    FocusedPane::Compose => {
                        // Handled separately in compose mode
                    }
                    FocusedPane::StartPage => {
                        // Should not happen in normal mode
                    }
                    FocusedPane::DraftList => {
                        // Draft list navigation handled in draft list mode
                    }
                    FocusedPane::Calendar => {
                        // Calendar navigation handled in calendar mode
                    }
                }
            }

            // Threading and view mode controls
            KeyCode::Char('t') => {
                // Toggle threaded view
                if let FocusedPane::MessageList = ui.focused_pane() {
                    ui.message_list_mut().toggle_view_mode();
                }
            }
            KeyCode::Char(' ') => {
                // Toggle expansion/collapse (Space key)
                match ui.focused_pane() {
                    FocusedPane::AccountSwitcher => {
                        ui.account_switcher_mut().toggle_expanded();
                    }
                    FocusedPane::MessageList => {
                        ui.message_list_mut().toggle_selected_thread();
                    }
                    _ => {}
                }
            }
            KeyCode::Char('o') => {
                // Open/expand thread
                if let FocusedPane::MessageList = ui.focused_pane() {
                    ui.message_list_mut().expand_selected_thread();
                }
            }
            KeyCode::Char('p' | 'P') => {
                // Open email popup viewer for reply/forward/edit actions
                if ui.focused_pane() == FocusedPane::ContentPreview {
                    if let Some(selected_message_item) = ui.message_list().selected_message() {
                        // We need the email content to start the viewer
                        if let Some(email_content) = ui.content_preview().get_email_content() {
                            // Create a minimal StoredMessage from MessageItem and EmailContent
                            // TODO: This should be improved to fetch the full StoredMessage from database
                            if let Some(message_id) = selected_message_item.message_id {
                                let stored_message = crate::email::StoredMessage {
                                    id: message_id,
                                    account_id: "default".to_string(), // TODO: Get actual account ID
                                    folder_name: "INBOX".to_string(),  // TODO: Get actual folder
                                    imap_uid: 0,                       // TODO: Get actual UID
                                    subject: email_content.headers.subject.clone(),
                                    from_name: Some(email_content.headers.from.clone()),
                                    from_addr: email_content.headers.from.clone(),
                                    to_addrs: email_content.headers.to.clone(),
                                    cc_addrs: email_content.headers.cc.clone(),
                                    bcc_addrs: email_content.headers.bcc.clone(),
                                    date: chrono::Utc::now(), // TODO: Parse actual date
                                    body_text: Some(email_content.body.clone()),
                                    body_html: if email_content.content_type
                                        == crate::ui::content_preview::ContentType::Html
                                    {
                                        Some(email_content.body.clone())
                                    } else {
                                        None
                                    },
                                    attachments: Vec::new(), // TODO: Convert attachments
                                    flags: if selected_message_item.is_read {
                                        vec!["\\Seen".to_string()]
                                    } else {
                                        Vec::new()
                                    },
                                    labels: Vec::new(),
                                    size: None,
                                    priority: None,
                                    is_draft: false,
                                    is_deleted: false,
                                    reply_to: email_content.headers.reply_to.clone(),
                                    message_id: Some(email_content.headers.message_id.clone()),
                                    thread_id: None,
                                    in_reply_to: email_content.headers.in_reply_to.clone(),
                                    references: Vec::new(),
                                    created_at: chrono::Utc::now(),
                                    updated_at: chrono::Utc::now(),
                                    last_synced: chrono::Utc::now(),
                                    sync_version: 1,
                                };
                                // Extract sender email for contact lookup before starting viewer
                                let sender_email = Self::extract_email_from_address(&email_content.headers.from);
                                
                                ui.start_email_viewer(stored_message, email_content.clone());
                                
                                return EventResult::EmailViewerStarted(sender_email);
                            }
                        }
                    }
                }
            }
            KeyCode::Char('C') => {
                // Close/collapse thread (capital C)
                if let FocusedPane::MessageList = ui.focused_pane() {
                    ui.message_list_mut().collapse_selected_thread();
                }
            }

            // Sorting controls / Attachment save
            KeyCode::Char('s') => {
                match ui.focused_pane() {
                    FocusedPane::MessageList => {
                        // Cycle through sort modes (date, sender, subject)
                        use crate::email::{SortCriteria, SortOrder};
                        ui.message_list_mut()
                            .set_sort_criteria(SortCriteria::Date(SortOrder::Descending));
                    }
                    FocusedPane::ContentPreview => {
                        // Save selected attachment
                        if ui.content_preview().has_attachments() {
                            if let Some(_attachment) =
                                ui.content_preview().get_selected_attachment()
                            {
                                // In a full implementation, this would trigger an async save operation
                                // For now, we'll just indicate that save was attempted
                                tracing::info!("Attachment save requested");
                                // TODO: Implement async save operation
                                // This would require adding a save operation to EventResult
                            }
                        }
                    }
                    _ => {}
                }
            }
            KeyCode::Char('v') => {
                // View selected attachment
                if let FocusedPane::ContentPreview = ui.focused_pane() {
                    if ui.content_preview().has_attachments() {
                        if let Some(_attachment) = ui.content_preview().get_selected_attachment() {
                            // Open attachment viewer
                            if let Err(e) =
                                ui.content_preview_mut().view_selected_attachment().await
                            {
                                tracing::error!("Failed to view attachment: {}", e);
                            }
                        }
                    }
                }
            }
            KeyCode::Char('O') => {
                // Open selected attachment with system default application (xdg-open)
                if let FocusedPane::ContentPreview = ui.focused_pane() {
                    if ui.content_preview().has_attachments() {
                        if let Some(_attachment) = ui.content_preview().get_selected_attachment() {
                            // Open attachment with xdg-open
                            if let Err(e) =
                                ui.content_preview_mut().open_attachment_with_system().await
                            {
                                tracing::error!(
                                    "Failed to open attachment with system application: {}",
                                    e
                                );
                            }
                        }
                    }
                }
            }
            KeyCode::Char('r') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Sort by sender (only when Ctrl is not pressed)
                if let FocusedPane::MessageList = ui.focused_pane() {
                    use crate::email::{SortCriteria, SortOrder};
                    ui.message_list_mut()
                        .set_sort_criteria(SortCriteria::Sender(SortOrder::Ascending));
                }
            }
            KeyCode::Char('u') => {
                // Sort by subject
                if let FocusedPane::MessageList = ui.focused_pane() {
                    use crate::email::{SortCriteria, SortOrder};
                    ui.message_list_mut()
                        .set_sort_criteria(SortCriteria::Subject(SortOrder::Ascending));
                }
            }

            // Search functionality
            KeyCode::Char('/') => {
                // Enter message search mode
                if let FocusedPane::MessageList = ui.focused_pane() {
                    if !ui.message_list().is_search_active() {
                        ui.message_list_mut().start_search();
                    }
                }
            }

            // Search interface F-keys (when search is active)
            // TODO: These are documented but not yet implemented - the current search
            // is a simple text filter, not the advanced SearchUI with different modes
            KeyCode::F(1 | 4) => {
                if ui.focused_pane() == FocusedPane::MessageList
                    && ui.message_list().is_search_active()
                {
                    // F1, F4: Search mode switching (not yet implemented)
                    // Note: F2 is handled above for folder rename operations
                    // Note: F3 is handled above for calendar access
                    tracing::info!("Search mode switching F-keys not yet implemented");
                }
            }

            // Folder management shortcuts (when folder tree is focused)
            KeyCode::Char('f') => {
                // Enter folder search mode
                if let FocusedPane::FolderTree = ui.focused_pane() {
                    if !ui.folder_tree().is_in_search_mode() {
                        ui.folder_tree_mut().enter_search_mode();
                    }
                }
            }
            KeyCode::Char('n') => {
                // Create new folder (when folder tree focused)
                if let FocusedPane::FolderTree = ui.focused_pane() {
                    // In production, this would open a dialog for folder name input
                    // For demo, create a sample folder
                    let parent_path = ui.folder_tree().selected_folder().map(|f| f.path.clone());
                    if let Some(parent_path) = parent_path {
                        let _ = ui
                            .folder_tree_mut()
                            .create_folder(Some(&parent_path), "New Folder".to_string());
                    }
                }
            }
            KeyCode::Char('d') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Delete folder (when folder tree focused, non-Ctrl)
                // Note: Ctrl+D is handled separately below for draft list
                if matches!(ui.focused_pane(), FocusedPane::FolderTree) {
                    let folder_path = ui.folder_tree().selected_folder().map(|f| f.path.clone());
                    if let Some(path) = folder_path {
                        let _ = ui.folder_tree_mut().delete_folder(&path);
                    }
                }
            }
            KeyCode::Char('R') => {
                // Force refresh folder (capital R) - full IMAP sync
                if let FocusedPane::FolderTree = ui.focused_pane() {
                    let folder_path = ui.folder_tree().selected_folder().map(|f| f.path.clone());
                    if let Some(path) = folder_path {
                        return EventResult::FolderForceRefresh(path);
                    }
                }
            }

            // Content preview controls (when content preview is focused)
            KeyCode::Char('m') => {
                // Toggle view mode (Raw, Formatted, Headers)
                if let FocusedPane::ContentPreview = ui.focused_pane() {
                    ui.content_preview_mut().toggle_view_mode();
                }
            }
            KeyCode::Char('a') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Select first attachment (when content preview is focused and has attachments)
                if let FocusedPane::ContentPreview = ui.focused_pane() {
                    if ui.content_preview().has_attachments() {
                        ui.content_preview_mut().select_first_attachment();
                        tracing::info!("First attachment selected");
                    }
                }
            }
            KeyCode::Char('H') => {
                // Toggle expanded headers (capital H)
                if let FocusedPane::ContentPreview = ui.focused_pane() {
                    ui.content_preview_mut().toggle_headers();
                }
            }
            KeyCode::Home => {
                // Jump to top of content
                if let FocusedPane::ContentPreview = ui.focused_pane() {
                    ui.content_preview_mut().scroll_to_top();
                }
            }
            KeyCode::End => {
                // Jump to bottom of content
                if let FocusedPane::ContentPreview = ui.focused_pane() {
                    ui.content_preview_mut().scroll_to_bottom(20); // Default height
                }
            }

            // Compose email shortcut
            KeyCode::Char('c') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Start compose mode only if we're not already in compose mode
                if !ui.is_composing() {
                    // Return a special compose action to signal the app to start compose mode
                    return EventResult::ComposeAction(ComposeAction::StartCompose);
                }
            }

            // Show draft list shortcut
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Ctrl+D to show draft list
                if !ui.is_draft_list_visible() && !ui.is_composing() {
                    return EventResult::DraftAction(DraftAction::RefreshDrafts);
                }
            }

            // Add account shortcut
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Ctrl+A to add a new account
                return EventResult::AddAccount;
            }

            // Remove account shortcut (when account switcher is focused)
            KeyCode::Char('x') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Ctrl+X to remove the currently selected account (only when account switcher is focused)
                if matches!(ui.focused_pane(), FocusedPane::AccountSwitcher) {
                    if let Some(account_id) = ui.account_switcher().get_current_account_id() {
                        return EventResult::RemoveAccount(account_id.clone());
                    }
                }
            }

            // Refresh account connection shortcut
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Ctrl+R to refresh account connection and sync folders/emails
                if matches!(ui.focused_pane(), FocusedPane::AccountSwitcher) {
                    if let Some(account_id) = ui.account_switcher().get_current_account_id() {
                        tracing::info!("Refreshing account connection: {}", account_id);
                        return EventResult::RefreshAccount(account_id.clone());
                    }
                }
            }

            // Copy functionality
            KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Ctrl+Y to copy email content to clipboard
                if matches!(ui.focused_pane(), FocusedPane::ContentPreview) {
                    if let Err(e) = ui.content_preview_mut().copy_email_content() {
                        tracing::error!("Failed to copy email content: {}", e);
                    }
                }
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::ALT) => {
                // Alt+C to copy attachment info to clipboard
                if matches!(ui.focused_pane(), FocusedPane::ContentPreview) {
                    if let Err(e) = ui.content_preview_mut().copy_attachment_info() {
                        tracing::error!("Failed to copy attachment info: {}", e);
                    }
                }
            }

            _ => {}
        }

        EventResult::Continue
    }

    /// Handle key events for start page mode
    async fn handle_start_page_keys(&mut self, key: KeyEvent, ui: &mut UI) -> EventResult {
        match key.code {
            // Global quit commands
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }

            // Navigate between widgets on start page
            KeyCode::Left | KeyCode::Char('h') => {
                ui.handle_start_page_navigation(StartPageNavigation::Previous);
            }
            KeyCode::Right | KeyCode::Char('l') => {
                ui.handle_start_page_navigation(StartPageNavigation::Next);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                ui.handle_start_page_navigation(StartPageNavigation::Previous);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                ui.handle_start_page_navigation(StartPageNavigation::Next);
            }
            KeyCode::Tab => {
                ui.handle_start_page_navigation(StartPageNavigation::Next);
            }
            KeyCode::BackTab => {
                ui.handle_start_page_navigation(StartPageNavigation::Previous);
            }

            // Function keys for direct access
            KeyCode::F(1) => {
                // F1: Help/Keyboard shortcuts
                ui.show_keyboard_shortcuts();
            }
            KeyCode::F(3) => {
                // F3: Calendar mode
                ui.show_calendar();
            }
            KeyCode::F(4) => {
                // F4: Settings/Configuration
                // TODO: Implement settings mode
            }

            // Primary actions
            KeyCode::Enter | KeyCode::Char('e') => {
                // Enter/e: Switch to email interface
                ui.show_email_interface();
            }
            KeyCode::Char('c') => {
                // c: Compose email
                ui.show_email_interface();
                return EventResult::ComposeAction(crate::ui::ComposeAction::StartCompose);
            }

            // Navigation and view modes
            KeyCode::Char('E') if ui.focused_pane() != FocusedPane::FolderTree => {
                // E: Email interface
                ui.show_email_interface();
            }
            KeyCode::Char('C') => {
                // C: Calendar view
                ui.show_calendar();
            }
            KeyCode::Char('?') if ui.focused_pane() != FocusedPane::FolderTree => {
                // ?: Help/keyboard shortcuts
                ui.show_keyboard_shortcuts();
            }

            // Search and filtering
            KeyCode::Char('/') => {
                // /: Search - switch to email interface and activate search
                ui.show_email_interface();
                // TODO: Focus search when implemented
            }
            KeyCode::Char('f') => {
                // f: Filter/Find
                ui.show_email_interface();
                // TODO: Activate filter mode
            }

            // Quick productivity actions
            KeyCode::Char('n') if ui.focused_pane() != FocusedPane::FolderTree => {
                // n: New (compose email)
                ui.show_email_interface();
                return EventResult::ComposeAction(crate::ui::ComposeAction::StartCompose);
            }
            KeyCode::Char('r') => {
                // r: Refresh all data
                // TODO: Add refresh functionality to app events
            }
            KeyCode::Char('s') => {
                // s: Sync/Synchronize accounts
                // TODO: Trigger account synchronization
            }

            // Dashboard-specific actions
            KeyCode::Char('t') if ui.focused_pane() != FocusedPane::FolderTree => {
                // t: Tasks/Todos (switch to calendar for todo management)
                ui.show_calendar();
            }
            KeyCode::Char('w') if ui.focused_pane() != FocusedPane::FolderTree => {
                // w: Weather refresh
                // TODO: Refresh weather data
            }
            KeyCode::Char('M') if ui.focused_pane() != FocusedPane::FolderTree => {
                // M: Monitor system resources (uppercase to avoid conflict)
                // TODO: Toggle detailed system monitor view
            }

            // Space and Escape for common actions
            KeyCode::Char(' ') => {
                // Space: Quick action (same as Enter)
                ui.show_email_interface();
            }
            KeyCode::Esc => {
                // Esc: Nothing to escape from on dashboard, but could be used for other actions
                // For now, just continue
            }

            // Folder-specific character keys (only when folder tree is focused)
            KeyCode::Char('m') if ui.focused_pane() == FocusedPane::FolderTree => {
                if let Some(operation) = ui.folder_tree_mut().handle_char_key('m') {
                    return EventResult::FolderOperation(operation);
                }
            }
            KeyCode::Char('n') if ui.focused_pane() == FocusedPane::FolderTree => {
                if let Some(operation) = ui.folder_tree_mut().handle_char_key('n') {
                    return EventResult::FolderOperation(operation);
                }
            }
            KeyCode::Char('N') if ui.focused_pane() == FocusedPane::FolderTree => {
                if let Some(operation) = ui.folder_tree_mut().handle_char_key('N') {
                    return EventResult::FolderOperation(operation);
                }
            }
            KeyCode::Char('d') if ui.focused_pane() == FocusedPane::FolderTree => {
                if let Some(operation) = ui.folder_tree_mut().handle_char_key('d') {
                    return EventResult::FolderOperation(operation);
                }
            }
            KeyCode::Char('R') if ui.focused_pane() == FocusedPane::FolderTree => {
                if let Some(operation) = ui.folder_tree_mut().handle_char_key('R') {
                    return EventResult::FolderOperation(operation);
                }
            }
            KeyCode::Char('E') if ui.focused_pane() == FocusedPane::FolderTree => {
                if let Some(operation) = ui.folder_tree_mut().handle_char_key('E') {
                    return EventResult::FolderOperation(operation);
                }
            }
            KeyCode::Char('p') if ui.focused_pane() == FocusedPane::FolderTree => {
                if let Some(operation) = ui.folder_tree_mut().handle_char_key('p') {
                    return EventResult::FolderOperation(operation);
                }
            }
            KeyCode::Char('?') if ui.focused_pane() == FocusedPane::FolderTree => {
                ui.folder_tree_mut().handle_char_key('?'); // Show context menu
            }

            _ => {}
        }

        EventResult::Continue
    }

    /// Handle email viewer mode key events
    async fn handle_email_viewer_keys(&mut self, key: KeyEvent, ui: &mut UI) -> EventResult {
        if let Some(action) = ui.handle_email_viewer_key(key.code) {
            match action {
                crate::ui::email_viewer::EmailViewerAction::Reply => {
                    // Start reply composition
                    self.handle_email_reply(ui).await
                }
                crate::ui::email_viewer::EmailViewerAction::ReplyAll => {
                    // Start reply all composition
                    self.handle_email_reply_all(ui).await
                }
                crate::ui::email_viewer::EmailViewerAction::Forward => {
                    // Start forward composition
                    self.handle_email_forward(ui).await
                }
                crate::ui::email_viewer::EmailViewerAction::Edit => {
                    // Edit email as draft (if it's a draft)
                    self.handle_email_edit(ui).await
                }
                crate::ui::email_viewer::EmailViewerAction::Delete => {
                    // Delete email
                    self.handle_email_delete(ui).await
                }
                crate::ui::email_viewer::EmailViewerAction::Archive => {
                    // Archive email
                    self.handle_email_archive(ui).await
                }
                crate::ui::email_viewer::EmailViewerAction::MarkAsRead => {
                    // Mark email as read
                    self.handle_email_mark_read(ui).await
                }
                crate::ui::email_viewer::EmailViewerAction::MarkAsUnread => {
                    // Mark email as unread
                    self.handle_email_mark_unread(ui).await
                }
                crate::ui::email_viewer::EmailViewerAction::AddToContacts => {
                    // Add sender to contacts
                    self.handle_email_add_to_contacts(ui).await
                }
                crate::ui::email_viewer::EmailViewerAction::Close => {
                    // Exit email viewer
                    ui.exit_email_viewer();
                    EventResult::Continue
                }
            }
        } else {
            EventResult::Continue
        }
    }

    /// Handle reply action from email viewer
    async fn handle_email_reply(&mut self, ui: &mut UI) -> EventResult {
        // Get current email data from email viewer
        if let Some(message) = ui.email_viewer_mut().current_message.clone() {
            // For now, return a specific action that the main loop can handle with contacts_manager
            // TODO: This should be handled by the main app with proper contacts_manager access
            return EventResult::ComposeAction(crate::ui::ComposeAction::StartReplyFromMessage(
                message,
            ));
        }
        EventResult::Continue
    }

    /// Handle reply all action from email viewer
    async fn handle_email_reply_all(&mut self, ui: &mut UI) -> EventResult {
        // Get current email data from email viewer
        if let Some(message) = ui.email_viewer_mut().current_message.clone() {
            // For now, return a specific action that the main loop can handle with contacts_manager
            // TODO: This should be handled by the main app with proper contacts_manager access
            return EventResult::ComposeAction(crate::ui::ComposeAction::StartReplyAllFromMessage(
                message,
            ));
        }
        EventResult::Continue
    }

    /// Handle forward action from email viewer
    async fn handle_email_forward(&mut self, ui: &mut UI) -> EventResult {
        // Get current email data from email viewer
        if let Some(message) = ui.email_viewer_mut().current_message.clone() {
            // For now, return a specific action that the main loop can handle with contacts_manager
            // TODO: This should be handled by the main app with proper contacts_manager access
            return EventResult::ComposeAction(crate::ui::ComposeAction::StartForwardFromMessage(
                message,
            ));
        }
        EventResult::Continue
    }

    /// Handle edit action from email viewer (for drafts)
    async fn handle_email_edit(&mut self, ui: &mut UI) -> EventResult {
        // Get current email data from email viewer
        if let Some(message) = ui.email_viewer_mut().current_message.clone() {
            // For now, return a specific action that the main loop can handle with contacts_manager
            // TODO: This should be handled by the main app with proper contacts_manager access
            return EventResult::ComposeAction(crate::ui::ComposeAction::StartEditFromMessage(
                message,
            ));
        }
        EventResult::Continue
    }

    /// Handle delete action from email viewer
    async fn handle_email_delete(&mut self, ui: &mut UI) -> EventResult {
        // Get current email data from email viewer
        if let Some(message) = ui.email_viewer_mut().current_message.clone() {
            // TODO: Implement actual email deletion
            tracing::info!(
                "Delete email action requested for message: {}",
                message.subject
            );
            // For now, just exit the viewer
            ui.exit_email_viewer();
            EventResult::Continue
        } else {
            EventResult::Continue
        }
    }

    /// Handle archive action from email viewer
    async fn handle_email_archive(&mut self, ui: &mut UI) -> EventResult {
        // Get current email data from email viewer
        if let Some(message) = ui.email_viewer_mut().current_message.clone() {
            // TODO: Implement actual email archiving
            tracing::info!(
                "Archive email action requested for message: {}",
                message.subject
            );
            // For now, just exit the viewer
            ui.exit_email_viewer();
            EventResult::Continue
        } else {
            EventResult::Continue
        }
    }

    /// Handle mark as read action from email viewer
    async fn handle_email_mark_read(&mut self, ui: &mut UI) -> EventResult {
        // Get current email data from email viewer
        if let Some(message) = ui.email_viewer_mut().current_message.clone() {
            // TODO: Implement actual mark as read
            tracing::info!(
                "Mark as read action requested for message: {}",
                message.subject
            );
            EventResult::Continue
        } else {
            EventResult::Continue
        }
    }

    /// Handle mark as unread action from email viewer
    async fn handle_email_mark_unread(&mut self, ui: &mut UI) -> EventResult {
        // Get current email data from email viewer
        if let Some(message) = ui.email_viewer_mut().current_message.clone() {
            // TODO: Implement actual mark as unread
            tracing::info!(
                "Mark as unread action requested for message: {}",
                message.subject
            );
            EventResult::Continue
        } else {
            EventResult::Continue
        }
    }

    /// Handle add to contacts action from email viewer
    async fn handle_email_add_to_contacts(&mut self, ui: &mut UI) -> EventResult {
        // Get current email data from email viewer
        if let Some(message) = ui.email_viewer_mut().current_message.clone() {
            // Extract sender name and email address
            let email_address = message.from_addr.clone();
            
            // Extract name from "Name <email>" format or use email as fallback
            let sender_name = if message.from_name.as_ref().map_or(true, |name| name.is_empty()) {
                // If no from_name, try to extract from from_addr if it contains < >
                if email_address.contains('<') && email_address.contains('>') {
                    email_address
                        .split('<')
                        .next()
                        .unwrap_or(&email_address)
                        .trim()
                        .trim_matches('"')
                        .to_string()
                } else {
                    // Use the email address before @ as the name
                    email_address
                        .split('@')
                        .next()
                        .unwrap_or(&email_address)
                        .to_string()
                }
            } else {
                message.from_name.clone().unwrap_or_default()
            };

            // Clean email address from "Name <email>" format
            let clean_email = if email_address.contains('<') && email_address.contains('>') {
                email_address
                    .split('<')
                    .nth(1)
                    .unwrap_or(&email_address)
                    .trim_end_matches('>')
                    .to_string()
            } else {
                email_address
            };

            tracing::info!(
                "Adding contact: {} <{}> from message: {}",
                sender_name,
                clean_email,
                message.subject
            );

            return EventResult::AddToContacts(clean_email, sender_name);
        }
        EventResult::Continue
    }

    /// Check if the application should quit
    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    /// Handle keyboard shortcuts popup mode keys
    async fn handle_keyboard_shortcuts_keys(&mut self, key: KeyEvent, ui: &mut UI) -> EventResult {
        match key.code {
            KeyCode::Char('?') | KeyCode::Esc => {
                // Close keyboard shortcuts popup
                ui.show_email_interface();
                EventResult::Continue
            }
            KeyCode::Up | KeyCode::Char('k') => {
                // Scroll up in shortcuts list
                ui.keyboard_shortcuts_ui_mut().scroll_up();
                EventResult::Continue
            }
            KeyCode::Down | KeyCode::Char('j') => {
                // Scroll down in shortcuts list
                ui.keyboard_shortcuts_ui_mut().scroll_down();
                EventResult::Continue
            }
            _ => EventResult::Continue,
        }
    }

    /// Handle contacts popup key events
    async fn handle_contacts_popup_keys(&mut self, key: KeyEvent, ui: &mut UI) -> EventResult {
        match key.code {
            KeyCode::Esc => {
                // Close contacts popup
                ui.hide_contacts_popup();
                EventResult::Continue
            }
            _ => {
                // Delegate to contacts popup for other keys
                if let Some(action) = ui.handle_contacts_popup_key(key.code).await {
                    return EventResult::ContactsAction(action);
                }
                EventResult::Continue
            }
        }
    }

    /// Extract email address from "Name <email@domain.com>" format
    fn extract_email_from_address(address: &str) -> String {
        if address.contains('<') && address.contains('>') {
            address
                .split('<')
                .nth(1)
                .unwrap_or(address)
                .trim_end_matches('>')
                .to_string()
        } else {
            address.to_string()
        }
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new()
    }
}
