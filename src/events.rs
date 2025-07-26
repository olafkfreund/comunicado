use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::ui::{FocusedPane, UI, UIMode, ComposeAction};

pub struct EventHandler {
    should_quit: bool,
}

/// Result of handling a key event
#[derive(Debug, Clone)]
pub enum EventResult {
    Continue,
    ComposeAction(ComposeAction),
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
        
        match key.code {
            // Global quit commands
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
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
                        ui.content_preview_mut().handle_down();
                    }
                    FocusedPane::Compose => {
                        // Handled separately in compose mode
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
                        ui.content_preview_mut().handle_up();
                    }
                    FocusedPane::Compose => {
                        // Handled separately in compose mode
                    }
                }
            }
            
            // Enter key for selection
            KeyCode::Enter => {
                match ui.focused_pane() {
                    FocusedPane::AccountSwitcher => {
                        if let Some(account_id) = ui.account_switcher_mut().select_current() {
                            // Switch to the selected account asynchronously
                            // For now, we'll set the account and let the main loop handle loading
                            tracing::info!("Account selected: {}", account_id);
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
            
            // Sorting controls
            KeyCode::Char('s') => {
                // Cycle through sort modes (date, sender, subject)
                if let FocusedPane::MessageList = ui.focused_pane() {
                    // For now, just demonstrate different sort criteria
                    use crate::email::{SortCriteria, SortOrder};
                    ui.message_list_mut().set_sort_criteria(SortCriteria::Date(SortOrder::Descending));
                }
            }
            KeyCode::Char('r') => {
                // Sort by sender
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