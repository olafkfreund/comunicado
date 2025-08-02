use crate::keyboard::{KeyboardAction, KeyboardManager};
use crate::tea::message::ViewMode;
use crate::ui::{ComposeAction, DraftAction, FocusedPane, UIMode, UI};
use crossterm::event::{KeyCode, KeyEvent};
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
    DeleteEmail(String, uuid::Uuid, String), // Account ID, Message ID, Folder
    ArchiveEmail(String, uuid::Uuid, String), // Account ID, Message ID, Folder
    MarkEmailRead(String, uuid::Uuid, String), // Account ID, Message ID, Folder
    MarkEmailUnread(String, uuid::Uuid, String), // Account ID, Message ID, Folder
    ToggleEmailFlag(String, uuid::Uuid, String), // Account ID, Message ID, Folder
    CreateEvent(String), // Calendar ID
    EditEvent(String, String), // Calendar ID, Event ID
    DeleteEvent(String, String), // Calendar ID, Event ID
    ViewEventDetails(String, String), // Calendar ID, Event ID
    CreateTodo(String), // Calendar ID
    ToggleTodoComplete(String, String), // Calendar ID, Event ID
    RetryInitialization, // Retry failed initialization
    CancelBackgroundTask, // Cancel selected background task
    AISummarizeEmail(uuid::Uuid), // Message ID to summarize with AI
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
        
        // Handle AI popup first if it's visible and interactive
        if ui.ai_popup().is_interactive() {
            return self.handle_ai_popup_keys(key, ui);
        }
        
        // Handle global help overlay first (works in all modes)
        if self.handle_help_keys(key, ui) {
            return EventResult::Continue;
        }

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

        // Handle enhanced progress overlay keys with high priority (before keyboard manager)
        if ui.enhanced_progress_overlay().is_visible() {
            match key.code {
                KeyCode::Esc => {
                    // If cancel dialog is already showing, actually cancel the task
                    if ui.enhanced_progress_overlay().is_cancel_dialog_showing() {
                        return EventResult::CancelBackgroundTask;
                    }
                    // If there's a cancellable task selected, show cancel dialog
                    else if ui.enhanced_progress_overlay().has_cancellable_task() {
                        ui.show_enhanced_progress_cancel_dialog();
                        return EventResult::Continue;
                    }
                    // Otherwise just hide the overlay
                    else {
                        ui.hide_enhanced_progress_overlay();
                        return EventResult::Continue;
                    }
                }
                KeyCode::Enter => {
                    return self.handle_select(ui);
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.handle_move_up(ui);
                    return EventResult::Continue;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.handle_move_down(ui);
                    return EventResult::Continue;
                }
                KeyCode::Char('q') | KeyCode::Char('Q') => {
                    ui.hide_enhanced_progress_overlay();
                    return EventResult::Continue;
                }
                _ => {} // Fall through to keyboard manager for other keys
            }
        }

        // Handle mode-specific keys BEFORE keyboard manager to avoid conflicts
        let mode_result = match ui.mode() {
            UIMode::EmailViewer => self.handle_email_viewer_keys(key, ui).await,
            UIMode::KeyboardShortcuts => self.handle_keyboard_shortcuts_keys(key, ui).await,
            UIMode::Settings => self.handle_settings_keys(key, ui).await,
            UIMode::ContactsPopup => self.handle_contacts_popup_keys(key, ui).await,
            _ => EventResult::Continue,
        };
        
        // If mode-specific handler processed the key, return its result
        if !matches!(mode_result, EventResult::Continue) {
            return mode_result;
        }

        // Get the action from keyboard manager (only if mode-specific handler didn't handle it)
        if let Some(action) = self.keyboard_manager.get_action(key.code, key.modifiers) {
            tracing::debug!("Found keyboard action for {:?}: {:?}", key.code, action);
            return self.execute_keyboard_action(action.clone(), ui).await;
        } else {
            tracing::debug!("No keyboard action found for key: {:?} with modifiers: {:?}", key.code, key.modifiers);
        }

        EventResult::Continue
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

    /// Handle help overlay keyboard shortcuts (Ctrl+H and ? key)
    fn handle_help_keys(&mut self, key: KeyEvent, ui: &mut UI) -> bool {
        // Check for help toggle keys: Ctrl+H or ? (question mark)
        let is_help_key = match key.code {
            KeyCode::Char('h') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => true,
            KeyCode::Char('?') => true,
            KeyCode::Esc if ui.is_help_visible() => true, // Allow Esc to close help
            _ => false,
        };

        if is_help_key {
            // Determine current view mode based on UI mode
            let current_view = match ui.mode() {
                UIMode::Calendar => ViewMode::Calendar,
                UIMode::ContactsPopup => ViewMode::Contacts,
                _ => ViewMode::Email, // Default to Email for normal/compose/draft modes
            };

            ui.toggle_help(current_view);
            return true;
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
            KeyboardAction::ShowKeyboardShortcuts => {
                ui.show_keyboard_shortcuts();
                EventResult::Continue
            }
            KeyboardAction::OpenSettings => {
                ui.show_settings();
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
                        tracing::info!("🔍 About to call handle_right() on folder tree");
                        
                        // Add comprehensive error handling around the entire call
                        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            ui.folder_tree_mut().handle_right();
                        })) {
                            Ok(_) => {
                                tracing::info!("✅ handle_right() completed successfully");
                            }
                            Err(panic_info) => {
                                tracing::error!("🚨 PANIC CAUGHT in handle_right()!");
                                if let Some(s) = panic_info.downcast_ref::<&str>() {
                                    tracing::error!("🚨 Panic message: {}", s);
                                } else if let Some(s) = panic_info.downcast_ref::<String>() {
                                    tracing::error!("🚨 Panic message: {}", s);
                                } else {
                                    tracing::error!("🚨 Unknown panic type");
                                }
                            }
                        }
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
                        if let (Some(message_id), Some(account_id), Some(folder)) = (
                            &message.message_id,
                            ui.message_list().current_account(),
                            ui.message_list().current_folder(),
                        ) {
                            tracing::info!("Delete email action triggered for message: {}", message.subject);
                            // Return event result for App to handle with email operations service
                            EventResult::DeleteEmail(account_id.clone(), *message_id, folder.clone())
                        } else {
                            tracing::warn!("Missing required information for delete email operation");
                            EventResult::Continue
                        }
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
                        if let (Some(message_id), Some(account_id), Some(folder)) = (
                            &message.message_id,
                            ui.message_list().current_account(),
                            ui.message_list().current_folder(),
                        ) {
                            tracing::info!("Archive email action triggered for message: {}", message.subject);
                            EventResult::ArchiveEmail(account_id.clone(), *message_id, folder.clone())
                        } else {
                            tracing::warn!("Missing required information for archive email operation");
                            EventResult::Continue
                        }
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
                        if let (Some(message_id), Some(account_id), Some(folder)) = (
                            &message.message_id,
                            ui.message_list().current_account(),
                            ui.message_list().current_folder(),
                        ) {
                            tracing::info!("Mark as read action triggered for message: {}", message.subject);
                            EventResult::MarkEmailRead(account_id.clone(), *message_id, folder.clone())
                        } else {
                            tracing::warn!("Missing required information for mark as read operation");
                            EventResult::Continue
                        }
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
                        if let (Some(message_id), Some(account_id), Some(folder)) = (
                            &message.message_id,
                            ui.message_list().current_account(),
                            ui.message_list().current_folder(),
                        ) {
                            tracing::info!("Mark as unread action triggered for message: {}", message.subject);
                            EventResult::MarkEmailUnread(account_id.clone(), *message_id, folder.clone())
                        } else {
                            tracing::warn!("Missing required information for mark as unread operation");
                            EventResult::Continue
                        }
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
            KeyboardAction::OpenEmailViewer => {
                tracing::debug!("🔍 OpenEmailViewer action triggered! Current pane: {:?}", ui.focused_pane());
                // Open email popup viewer for reply/forward/edit actions
                if matches!(ui.focused_pane(), FocusedPane::MessageList | FocusedPane::ContentPreview) {
                    tracing::debug!("✅ Pane check passed for OpenEmailViewer");
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
                if matches!(ui.focused_pane(), FocusedPane::MessageList | FocusedPane::ContentPreview) {
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
                if matches!(ui.focused_pane(), FocusedPane::MessageList | FocusedPane::ContentPreview) {
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
                tracing::info!("🔥 ContactsPopup action triggered!");
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
            KeyboardAction::ShowEmail => {
                ui.show_email();
                EventResult::Continue
            }
            KeyboardAction::CreateEvent => {
                if ui.mode() == &UIMode::Calendar {
                    // Use default calendar ID for creating events
                    let calendar_id = "primary".to_string();
                    EventResult::CreateEvent(calendar_id)
                } else {
                    ui.show_calendar();
                    tracing::info!("Switched to calendar and triggered create event");
                    EventResult::Continue
                }
            }
            KeyboardAction::EditEvent => {
                if ui.mode() == &UIMode::Calendar {
                    // Get selected event ID - use default calendar
                    if let Some(event_id) = ui.calendar_ui().get_selected_event_id() {
                        let calendar_id = "primary".to_string();
                        EventResult::EditEvent(calendar_id, event_id)
                    } else {
                        tracing::warn!("No event selected for editing");
                        EventResult::Continue
                    }
                } else {
                    EventResult::Continue
                }
            }
            KeyboardAction::DeleteEvent => {
                if ui.mode() == &UIMode::Calendar {
                    // Get selected event ID - use default calendar
                    if let Some(event_id) = ui.calendar_ui().get_selected_event_id() {
                        let calendar_id = "primary".to_string();
                        EventResult::DeleteEvent(calendar_id, event_id)
                    } else {
                        tracing::warn!("No event selected for deletion");
                        EventResult::Continue
                    }
                } else {
                    EventResult::Continue
                }
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
                    // Use default calendar ID for creating todos
                    let calendar_id = "primary".to_string();
                    EventResult::CreateTodo(calendar_id)
                } else {
                    ui.show_calendar();
                    tracing::info!("Switched to calendar and triggered create todo");
                    EventResult::Continue
                }
            }
            KeyboardAction::ToggleTodoComplete => {
                if ui.mode() == &UIMode::Calendar {
                    // Get selected event/todo ID - use default calendar
                    if let Some(event_id) = ui.calendar_ui().get_selected_event_id() {
                        let calendar_id = "primary".to_string();
                        EventResult::ToggleTodoComplete(calendar_id, event_id)
                    } else {
                        tracing::warn!("No todo selected for toggle completion");
                        EventResult::Continue
                    }
                } else {
                    EventResult::Continue
                }
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

            // AI Assistant actions
            KeyboardAction::AIToggleAssistant => {
                ui.toggle_ai_assistant();
                EventResult::Continue
            }
            KeyboardAction::AIEmailSuggestions => {
                if matches!(ui.focused_pane(), FocusedPane::MessageList | FocusedPane::ContentPreview) {
                    if let Some(message) = ui.message_list().selected_message() {
                        let subject = message.subject.clone();
                        let sender = message.sender.clone();
                        ui.show_ai_email_suggestions(&subject, &sender);
                    }
                }
                EventResult::Continue
            }
            KeyboardAction::AIComposeSuggestions => {
                if ui.mode() == &UIMode::Compose || ui.is_composing() {
                    ui.show_ai_compose_suggestions();
                }
                EventResult::Continue
            }
            KeyboardAction::AISummarizeEmail => {
                if matches!(ui.focused_pane(), FocusedPane::MessageList | FocusedPane::ContentPreview) {
                    if let Some(message_item) = ui.message_list().selected_message() {
                        if let Some(message_id) = message_item.message_id {
                            return EventResult::AISummarizeEmail(message_id);
                        }
                    }
                }
                EventResult::Continue
            }
            KeyboardAction::AICalendarAssist => {
                if ui.mode() == &UIMode::Calendar {
                    ui.show_ai_calendar_assistance();
                } else {
                    // Switch to calendar first, then show AI assistance
                    ui.show_calendar();
                    ui.show_ai_calendar_assistance();
                }
                EventResult::Continue
            }
            KeyboardAction::AIConfigureSettings => {
                ui.show_ai_configuration();
                EventResult::Continue
            }
            KeyboardAction::AIQuickReply => {
                if matches!(ui.focused_pane(), FocusedPane::MessageList | FocusedPane::ContentPreview) {
                    if let Some(message) = ui.message_list().selected_message() {
                        if let Some(email_content) = ui.content_preview().get_email_content() {
                            let sender = message.sender.clone();
                            let body = email_content.body.clone();
                            ui.show_ai_quick_reply(&sender, &body);
                        }
                    }
                }
                EventResult::Continue
            }
            KeyboardAction::AIEmailAnalysis => {
                if matches!(ui.focused_pane(), FocusedPane::MessageList | FocusedPane::ContentPreview) {
                    if let Some(_message) = ui.message_list().selected_message() {
                        if let Some(email_content) = ui.content_preview().get_email_content() {
                            let body = email_content.body.clone();
                            let subject = email_content.headers.subject.clone();
                            ui.show_ai_email_analysis(&body, &subject);
                        }
                    }
                }
                EventResult::Continue
            }
            KeyboardAction::AIScheduleRequest => {
                if matches!(ui.focused_pane(), FocusedPane::MessageList | FocusedPane::ContentPreview) {
                    if let Some(_message) = ui.message_list().selected_message() {
                        if let Some(email_content) = ui.content_preview().get_email_content() {
                            let body = email_content.body.clone();
                            ui.show_ai_schedule_parsing(&body);
                        }
                    }
                }
                EventResult::Continue
            }
            KeyboardAction::AIContentGeneration => {
                if ui.mode() == &UIMode::Compose || ui.is_composing() {
                    ui.show_ai_content_generation();
                } else {
                    // Start compose mode and then show AI content generation
                    return EventResult::ComposeAction(ComposeAction::StartCompose);
                }
                EventResult::Continue
            }

            // Email Viewer actions - only work in email viewer mode
            KeyboardAction::EmailViewerReply => {
                if ui.mode() == &UIMode::EmailViewer {
                    if let Some(message_id) = ui.email_viewer().get_message_id() {
                        EventResult::ReplyToMessage(message_id)
                    } else {
                        EventResult::Continue
                    }
                } else {
                    EventResult::Continue
                }
            }
            KeyboardAction::EmailViewerReplyAll => {
                if ui.mode() == &UIMode::EmailViewer {
                    if let Some(message_id) = ui.email_viewer().get_message_id() {
                        EventResult::ReplyAllToMessage(message_id)
                    } else {
                        EventResult::Continue
                    }
                } else {
                    EventResult::Continue
                }
            }
            KeyboardAction::EmailViewerForward => {
                if ui.mode() == &UIMode::EmailViewer {
                    if let Some(message_id) = ui.email_viewer().get_message_id() {
                        EventResult::ForwardMessage(message_id)
                    } else {
                        EventResult::Continue
                    }
                } else {
                    EventResult::Continue
                }
            }
            KeyboardAction::EmailViewerEdit => {
                if ui.mode() == &UIMode::EmailViewer {
                    // Start compose mode to edit the current email
                    EventResult::ComposeAction(ComposeAction::StartCompose)
                } else {
                    EventResult::Continue
                }
            }
            KeyboardAction::EmailViewerDelete => {
                if ui.mode() == &UIMode::EmailViewer {
                    if let Some(message_id) = ui.email_viewer().get_message_id() {
                        // Get account and folder info from email viewer
                        if let (Some(account_id), Some(folder)) = (
                            ui.email_viewer().get_account_id(),
                            ui.email_viewer().get_folder_name(),
                        ) {
                            EventResult::DeleteEmail(account_id, message_id, folder)
                        } else {
                            tracing::warn!("Missing account/folder info in email viewer for delete");
                            EventResult::Continue
                        }
                    } else {
                        EventResult::Continue
                    }
                } else {
                    EventResult::Continue
                }
            }
            KeyboardAction::EmailViewerArchive => {
                if ui.mode() == &UIMode::EmailViewer {
                    if let Some(message_id) = ui.email_viewer().get_message_id() {
                        if let (Some(account_id), Some(folder)) = (
                            ui.email_viewer().get_account_id(),
                            ui.email_viewer().get_folder_name(),
                        ) {
                            EventResult::ArchiveEmail(account_id, message_id, folder)
                        } else {
                            tracing::warn!("Missing account/folder info in email viewer for archive");
                            EventResult::Continue
                        }
                    } else {
                        EventResult::Continue
                    }
                } else {
                    EventResult::Continue
                }
            }
            KeyboardAction::EmailViewerMarkRead => {
                if ui.mode() == &UIMode::EmailViewer {
                    if let Some(message_id) = ui.email_viewer().get_message_id() {
                        if let (Some(account_id), Some(folder)) = (
                            ui.email_viewer().get_account_id(),
                            ui.email_viewer().get_folder_name(),
                        ) {
                            EventResult::MarkEmailRead(account_id, message_id, folder)
                        } else {
                            tracing::warn!("Missing account/folder info in email viewer for mark read");
                            EventResult::Continue
                        }
                    } else {
                        EventResult::Continue
                    }
                } else {
                    EventResult::Continue
                }
            }
            KeyboardAction::EmailViewerMarkUnread => {
                if ui.mode() == &UIMode::EmailViewer {
                    if let Some(message_id) = ui.email_viewer().get_message_id() {
                        if let (Some(account_id), Some(folder)) = (
                            ui.email_viewer().get_account_id(),
                            ui.email_viewer().get_folder_name(),
                        ) {
                            EventResult::MarkEmailUnread(account_id, message_id, folder)
                        } else {
                            tracing::warn!("Missing account/folder info in email viewer for mark unread");
                            EventResult::Continue
                        }
                    } else {
                        EventResult::Continue
                    }
                } else {
                    EventResult::Continue
                }
            }
            KeyboardAction::EmailViewerClose => {
                if ui.mode() == &UIMode::EmailViewer {
                    // Close email viewer by switching back to normal mode
                    self.handle_escape(ui);
                }
                EventResult::Continue
            }

            // Arrow navigation actions
            KeyboardAction::MoveLeft => {
                match ui.focused_pane() {
                    FocusedPane::AccountSwitcher => {
                        ui.previous_pane();
                    }
                    FocusedPane::FolderTree => {
                        ui.folder_tree_mut().handle_left();
                    }
                    FocusedPane::MessageList => {
                        ui.previous_pane();
                    }
                    FocusedPane::ContentPreview => {
                        ui.previous_pane();
                    }
                    FocusedPane::Compose | FocusedPane::DraftList | FocusedPane::Calendar => {
                        // For these special modes, just go to previous pane
                        ui.previous_pane();
                    }
                }
                EventResult::Continue
            }
            KeyboardAction::MoveRight => {
                match ui.focused_pane() {
                    FocusedPane::AccountSwitcher => {
                        ui.next_pane();
                    }
                    FocusedPane::FolderTree => {
                        ui.folder_tree_mut().handle_right();
                    }
                    FocusedPane::MessageList => {
                        ui.next_pane();
                    }
                    FocusedPane::ContentPreview => {
                        // At rightmost pane, cycle back to beginning
                        ui.previous_pane();
                        ui.previous_pane();
                        ui.previous_pane();
                    }
                    FocusedPane::Compose | FocusedPane::DraftList | FocusedPane::Calendar => {
                        // For these special modes, just go to next pane
                        ui.next_pane();
                    }
                }
                EventResult::Continue
            }

            // Search actions
            KeyboardAction::EndSearch => {
                // End search based on current focused pane
                match ui.focused_pane() {
                    FocusedPane::MessageList => {
                        if ui.message_list().is_search_active() {
                            ui.message_list_mut().end_search();
                        }
                    }
                    FocusedPane::FolderTree => {
                        if ui.folder_tree().is_in_search_mode() {
                            ui.folder_tree_mut().exit_search_mode(true);
                        }
                    }
                    _ => {}
                }
                EventResult::Continue
            }

            // Attachment actions
            KeyboardAction::SaveAttachment => {
                if ui.focused_pane() == FocusedPane::ContentPreview {
                    if ui.content_preview().is_viewing_attachment() {
                        // TODO: Implement save attachment functionality
                        tracing::info!("Save attachment action triggered");
                        // For now, show a toast message
                        ui.show_toast_info("Save attachment functionality coming soon");
                    }
                }
                EventResult::Continue
            }

            // Account management actions
            KeyboardAction::SwitchAccount => {
                if ui.focused_pane() == FocusedPane::AccountSwitcher {
                    ui.account_switcher_mut().next_account();
                } else {
                    // Cycle to account switcher pane
                    while ui.focused_pane() != FocusedPane::AccountSwitcher {
                        ui.next_pane();
                    }
                    ui.account_switcher_mut().next_account();
                }
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
        // First check if enhanced progress overlay is visible and handle navigation there
        if ui.enhanced_progress_overlay().is_visible() {
            ui.enhanced_progress_next();
            return;
        }

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
        // First check if enhanced progress overlay is visible and handle navigation there
        if ui.enhanced_progress_overlay().is_visible() {
            ui.enhanced_progress_previous();
            return;
        }

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
        // First check if enhanced progress overlay is visible
        if ui.enhanced_progress_overlay().is_visible() {
            if ui.enhanced_progress_overlay().is_cancel_dialog_showing() {
                // Confirm cancellation - return event that can be handled async by main app
                return EventResult::CancelBackgroundTask;
            } else {
                // Show cancel dialog for selected task
                ui.show_enhanced_progress_cancel_dialog();
                return EventResult::Continue;
            }
        }

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
                        tracing::debug!("Folder selection: returning FolderSelect({})", folder_path);
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

    /// Handle escape action for different panes and modes
    fn handle_escape(&mut self, ui: &mut UI) {
        // First handle enhanced progress overlay
        if ui.enhanced_progress_overlay().is_visible() {
            // If cancel dialog is shown, hide it
            if ui.enhanced_progress_overlay().is_cancel_dialog_showing() {
                ui.hide_enhanced_progress_cancel_dialog();
                return;
            }
            // Otherwise hide the entire overlay
            ui.hide_enhanced_progress_overlay();
            return;
        }

        // First check UI mode for mode-specific escape handling
        match ui.mode() {
            UIMode::Calendar => {
                // Return to email view from calendar
                ui.show_email();
                return;
            }
            _ => {}
        }

        // Then handle pane-specific escape actions
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

    /// Handle settings keys when in Settings mode
    async fn handle_settings_keys(&mut self, key: KeyEvent, ui: &mut UI) -> EventResult {
        // First try to handle key with the settings UI
        if ui.settings_ui_mut().handle_key(key.code, key.modifiers) {
            return EventResult::Continue;
        }

        // If settings UI didn't handle it, check for global close keys
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                // Close settings and return to normal interface
                ui.show_email_interface();
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
            // Navigation shortcuts from contacts to other views
            KeyCode::Char('e') => {
                // e - Go to email interface
                ui.hide_contacts_popup();
                ui.show_email_interface();
                EventResult::Continue
            }
            KeyCode::Char('g') => {
                // g - Go to calendar
                ui.hide_contacts_popup();
                ui.show_calendar();
                EventResult::Continue
            }
            KeyCode::Char('K') => {
                // K - Stay in contacts (no-op, but documented for consistency)
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

    /// Handle AI popup keyboard input
    fn handle_ai_popup_keys(&mut self, key: KeyEvent, ui: &mut UI) -> EventResult {
        use crossterm::event::{KeyCode, KeyModifiers};
        
        // Check for global AI toggle shortcut (Ctrl+Alt+I) even when popup is visible
        if key.modifiers.contains(KeyModifiers::CONTROL | KeyModifiers::ALT) 
            && key.code == KeyCode::Char('i') {
            ui.ai_popup_mut().hide();
            return EventResult::Continue;
        }
        
        match key.code {
            KeyCode::Esc => {
                ui.ai_popup_mut().hide();
                EventResult::Continue
            }
            KeyCode::Tab => {
                ui.ai_popup_mut().next_tab();
                EventResult::Continue
            }
            KeyCode::BackTab => {
                ui.ai_popup_mut().previous_tab();
                EventResult::Continue
            }
            KeyCode::Up => {
                ui.ai_popup_mut().move_up();
                EventResult::Continue
            }
            KeyCode::Down => {
                ui.ai_popup_mut().move_down();
                EventResult::Continue
            }
            KeyCode::Enter => {
                if let Some(action) = ui.ai_popup_mut().select_current() {
                    ui.handle_ai_popup_action(action);
                }
                EventResult::Continue
            }
            _ => EventResult::Continue,
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
