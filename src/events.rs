use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::ui::{FocusedPane, UI, UIMode, ComposeAction, StartPageNavigation};

pub struct EventHandler {
    should_quit: bool,
}

/// Result of handling a key event
#[derive(Debug, Clone)]
pub enum EventResult {
    Continue,
    ComposeAction(ComposeAction),
    AccountSwitch(String), // Account ID to switch to
    AddAccount, // Launch account setup wizard
    RemoveAccount(String), // Account ID to remove
    RefreshAccount(String), // Account ID to refresh connection
    SyncAccount(String), // Account ID to manually sync
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
        
        // Handle start page mode
        if ui.mode() == &UIMode::StartPage {
            return self.handle_start_page_keys(key, ui).await;
        }
        
        match key.code {
            // Global quit commands
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
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
                        ui.folder_tree_mut().handle_enter();
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
            
            // Folder management shortcuts (when folder tree is focused)
            KeyCode::Char('f') => {
                // Toggle folder search/filter
                if let FocusedPane::FolderTree = ui.focused_pane() {
                    // For now, just clear search - in production this would open search input
                    ui.folder_tree_mut().clear_search();
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
            KeyCode::Char('/') => {
                // Start folder search
                if let FocusedPane::FolderTree = ui.focused_pane() {
                    // In production, this would open search input
                    // For demo, toggle showing unsubscribed folders
                    ui.folder_tree_mut().toggle_show_unsubscribed();
                }
            }
            
            // Content preview controls (when content preview is focused)
            KeyCode::Char('v') => {
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
            
            // Manual IMAP sync shortcut
            KeyCode::F(5) => {
                // F5 to manually trigger IMAP sync for current account
                if let Some(account_id) = ui.account_switcher().get_current_account_id() {
                    tracing::info!("Manual IMAP sync requested: {}", account_id);
                    return EventResult::SyncAccount(account_id.clone());
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
            
            // Refresh data
            KeyCode::F(5) | KeyCode::Char('r') => {
                // TODO: Add refresh functionality to app events
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