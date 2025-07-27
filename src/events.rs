use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::ui::{FocusedPane, UI, UIMode, ComposeAction, DraftAction, StartPageNavigation};

pub struct EventHandler {
    should_quit: bool,
}

/// Result of handling a key event
#[derive(Debug, Clone)]
pub enum EventResult {
    Continue,
    ComposeAction(ComposeAction),
    DraftAction(DraftAction),
    AccountSwitch(String), // Account ID to switch to
    AddAccount, // Launch account setup wizard
    RemoveAccount(String), // Account ID to remove
    RefreshAccount(String), // Account ID to refresh connection
    SyncAccount(String), // Account ID to manually sync
    FolderSelect(String), // Folder path to load messages from
    FolderOperation(crate::ui::folder_tree::FolderOperation), // Folder operation to execute
}

impl EventHandler {
    pub fn new() -> Self {
        Self {
            should_quit: false,
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
        
        // Handle attachment viewer mode when it's active in content preview
        if ui.focused_pane() == FocusedPane::ContentPreview && ui.content_preview().is_viewing_attachment() {
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
                    ui.content_preview_mut().scroll_to_bottom();
                    return EventResult::Continue;
                }
                KeyCode::Char(c) => {
                    if let Err(e) = ui.content_preview_mut().handle_attachment_viewer_key(c).await {
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
            KeyCode::Char(c) if ui.focused_pane() == FocusedPane::FolderTree && ui.folder_tree().is_in_search_mode() => {
                ui.folder_tree_mut().handle_search_input(c);
                return EventResult::Continue;
            }
            KeyCode::Backspace if ui.focused_pane() == FocusedPane::FolderTree && ui.folder_tree().is_in_search_mode() => {
                ui.folder_tree_mut().handle_search_backspace();
                return EventResult::Continue;
            }
            
            // Handle search input mode for message list
            KeyCode::Char(c) if ui.focused_pane() == FocusedPane::MessageList && ui.message_list().is_search_active() => {
                let mut current_query = ui.message_list().search_query().to_string();
                current_query.push(c);
                ui.message_list_mut().update_search(current_query);
                return EventResult::Continue;
            }
            KeyCode::Backspace if ui.focused_pane() == FocusedPane::MessageList && ui.message_list().is_search_active() => {
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
                        if ui.content_preview().has_attachments() && 
                           ui.content_preview().get_selected_attachment().is_some() &&
                           key.modifiers.contains(KeyModifiers::CONTROL) {
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
                        if ui.content_preview().has_attachments() && 
                           ui.content_preview().get_selected_attachment().is_some() &&
                           key.modifiers.contains(KeyModifiers::CONTROL) {
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
                        if let Some(operation) = ui.folder_tree_mut().handle_function_key(key.code) {
                            return EventResult::FolderOperation(operation);
                        }
                    }
                    _ => {
                        // Global F5 refresh
                        // TODO: Add global refresh functionality
                    }
                }
            }
            KeyCode::F(2) => {
                // F2 - Rename
                match ui.focused_pane() {
                    FocusedPane::FolderTree => {
                        if let Some(operation) = ui.folder_tree_mut().handle_function_key(key.code) {
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
                        if let Some(operation) = ui.folder_tree_mut().handle_function_key(key.code) {
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
                        ui.message_list_mut().set_sort_criteria(SortCriteria::Date(SortOrder::Descending));
                    }
                    FocusedPane::ContentPreview => {
                        // Save selected attachment
                        if ui.content_preview().has_attachments() {
                            if let Some(_attachment) = ui.content_preview().get_selected_attachment() {
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
                            if let Err(e) = ui.content_preview_mut().view_selected_attachment().await {
                                tracing::error!("Failed to view attachment: {}", e);
                            }
                        }
                    }
                }
            }
            KeyCode::Char('r') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Sort by sender (only when Ctrl is not pressed)
                if let FocusedPane::MessageList = ui.focused_pane() {
                    use crate::email::{SortCriteria, SortOrder};
                    ui.message_list_mut().set_sort_criteria(SortCriteria::Sender(SortOrder::Ascending));
                }
            }
            KeyCode::Char('u') => {
                // Sort by subject
                if let FocusedPane::MessageList = ui.focused_pane() {
                    use crate::email::{SortCriteria, SortOrder};
                    ui.message_list_mut().set_sort_criteria(SortCriteria::Subject(SortOrder::Ascending));
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
                        let _ = ui.folder_tree_mut().create_folder(Some(&parent_path), "New Folder".to_string());
                    }
                }
            }
            KeyCode::Char('d') => {
                // Delete folder (when folder tree focused, non-Ctrl)
                if matches!(ui.focused_pane(), FocusedPane::FolderTree) && !key.modifiers.contains(KeyModifiers::CONTROL) {
                    let folder_path = ui.folder_tree().selected_folder().map(|f| f.path.clone());
                    if let Some(path) = folder_path {
                        let _ = ui.folder_tree_mut().delete_folder(&path);
                    }
                }
            }
            KeyCode::Char('R') => {
                // Refresh folder (capital R)
                if let FocusedPane::FolderTree = ui.focused_pane() {
                    let folder_path = ui.folder_tree().selected_folder().map(|f| f.path.clone());
                    if let Some(path) = folder_path {
                        ui.folder_tree_mut().refresh_folder(&path);
                        // Simulate sync completion after a moment (in production this would be async)
                        ui.folder_tree_mut().mark_folder_synced(&path, 0, 42);
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
                    ui.content_preview_mut().scroll_to_bottom();
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
            KeyCode::Tab => {
                ui.handle_start_page_navigation(StartPageNavigation::Next);
            }
            KeyCode::BackTab => {
                ui.handle_start_page_navigation(StartPageNavigation::Previous);
            }
            
            // Switch to email interface
            KeyCode::Enter | KeyCode::Char('e') => {
                ui.show_email_interface();
            }
            
            // Quick actions from start page
            KeyCode::Char('c') => {
                // Compose email - switch to email interface and start compose
                ui.show_email_interface();
                return EventResult::ComposeAction(crate::ui::ComposeAction::StartCompose);
            }
            KeyCode::Char('/') => {
                // Search - switch to email interface and focus search
                ui.show_email_interface();
                // TODO: Focus search when implemented
            }
            KeyCode::Char('a') => {
                // Address book - switch to email interface and show contacts
                ui.show_email_interface();
                // TODO: Show contacts when implemented
            }
            KeyCode::Char('C') => {
                // Calendar - switch to email interface and show calendar
                ui.show_email_interface();
                // TODO: Show calendar when implemented
            }
            
            // Task management on start page
            KeyCode::Char('t') => {
                // TODO: Add new task functionality
            }
            KeyCode::Char('x') => {
                // TODO: Mark task as complete
            }
            
            // Refresh data (when not in folder tree - handled above)
            KeyCode::Char('r') if ui.focused_pane() != FocusedPane::FolderTree => {
                // TODO: Add refresh functionality to app events
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

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new()
    }
}