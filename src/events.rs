use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::ui::{FocusedPane, UI};

pub struct EventHandler {
    should_quit: bool,
}

impl EventHandler {
    pub fn new() -> Self {
        Self {
            should_quit: false,
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent, ui: &mut UI) {
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
                    FocusedPane::FolderTree => {
                        ui.folder_tree_mut().handle_down();
                    }
                    FocusedPane::MessageList => {
                        ui.message_list_mut().handle_down();
                    }
                    FocusedPane::ContentPreview => {
                        ui.content_preview_mut().handle_down();
                    }
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                // Move up in current pane
                match ui.focused_pane() {
                    FocusedPane::FolderTree => {
                        ui.folder_tree_mut().handle_up();
                    }
                    FocusedPane::MessageList => {
                        ui.message_list_mut().handle_up();
                    }
                    FocusedPane::ContentPreview => {
                        ui.content_preview_mut().handle_up();
                    }
                }
            }
            
            // Enter key for selection
            KeyCode::Enter => {
                match ui.focused_pane() {
                    FocusedPane::FolderTree => {
                        ui.folder_tree_mut().handle_enter();
                    }
                    FocusedPane::MessageList => {
                        ui.message_list_mut().handle_enter();
                    }
                    FocusedPane::ContentPreview => {
                        // Maybe handle links or attachments in the future
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
                // Toggle thread expansion/collapse (Space key)
                if let FocusedPane::MessageList = ui.focused_pane() {
                    ui.message_list_mut().toggle_selected_thread();
                }
            }
            KeyCode::Char('o') => {
                // Open/expand thread
                if let FocusedPane::MessageList = ui.focused_pane() {
                    ui.message_list_mut().expand_selected_thread();
                }
            }
            KeyCode::Char('c') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Close/collapse thread (c without Ctrl)
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
            
            _ => {}
        }
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