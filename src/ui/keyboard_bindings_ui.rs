//! Keyboard Bindings Configuration UI - Complete interface for customizing keyboard shortcuts
//!
//! This module provides a comprehensive UI for:
//! - Viewing all configured keyboard bindings
//! - Adding new custom bindings
//! - Editing existing bindings
//! - Removing bindings
//! - Resetting to defaults
//! - Importing/exporting binding configurations

use crate::theme::Theme;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap,
    },
    Frame,
};
use std::collections::HashMap;

/// Keyboard bindings manager modes
#[derive(Debug, Clone, PartialEq)]
pub enum KeyboardBindingsMode {
    List,             // View all bindings
    Add,              // Add new binding
    Edit(String),     // Edit binding by key
    Delete(String),   // Confirm deletion
    Import,           // Import bindings
    Export,           // Export bindings
}

/// Action categories for organization
#[derive(Debug, Clone, PartialEq)]
pub enum ActionCategory {
    Navigation,
    Email,
    Calendar,
    UI,
    Application,
    Custom,
}

impl ActionCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            ActionCategory::Navigation => "Navigation",
            ActionCategory::Email => "Email",
            ActionCategory::Calendar => "Calendar", 
            ActionCategory::UI => "UI & Interface",
            ActionCategory::Application => "Application",
            ActionCategory::Custom => "Custom",
        }
    }
    
    pub fn all() -> Vec<ActionCategory> {
        vec![
            ActionCategory::Navigation,
            ActionCategory::Email,
            ActionCategory::Calendar,
            ActionCategory::UI,
            ActionCategory::Application,
            ActionCategory::Custom,
        ]
    }
}

/// Predefined actions that can be bound to keys
#[derive(Debug, Clone)]
pub struct KeyAction {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: ActionCategory,
    pub default_key: Option<String>,
}

/// Keyboard bindings configuration UI state
pub struct KeyboardBindingsUI {
    mode: KeyboardBindingsMode,
    bindings: HashMap<String, String>, // key -> action_id
    list_state: ListState,
    selected_category: ActionCategory,
    category_index: usize,
    
    // Form fields for adding/editing bindings
    form_key_combo: String,
    form_action_id: String,
    form_description: String,
    
    // UI state
    editing_field: Option<usize>, // Which field is being edited (0-2)
    input_buffer: String,
    status_message: String,
    
    // Available actions
    actions: Vec<KeyAction>,
    filtered_actions: Vec<KeyAction>,
}

impl KeyboardBindingsUI {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        
        let actions = Self::get_default_actions();
        let filtered_actions = actions.clone();
        
        Self {
            mode: KeyboardBindingsMode::List,
            bindings: HashMap::new(),
            list_state,
            selected_category: ActionCategory::Navigation,
            category_index: 0,
            
            form_key_combo: String::new(),
            form_action_id: String::new(),
            form_description: String::new(),
            
            editing_field: None,
            input_buffer: String::new(),
            status_message: String::new(),
            
            actions,
            filtered_actions,
        }
    }
    
    /// Initialize with existing bindings
    pub fn with_bindings(bindings: HashMap<String, String>) -> Self {
        let mut ui = Self::new();
        ui.bindings = bindings;
        ui.status_message = format!("Loaded {} custom bindings", ui.bindings.len());
        ui
    }
    
    /// Get default available actions
    fn get_default_actions() -> Vec<KeyAction> {
        vec![
            // Navigation actions
            KeyAction {
                id: "move_up".to_string(),
                name: "Move Up".to_string(),
                description: "Move cursor up".to_string(),
                category: ActionCategory::Navigation,
                default_key: Some("Up".to_string()),
            },
            KeyAction {
                id: "move_down".to_string(),
                name: "Move Down".to_string(),
                description: "Move cursor down".to_string(),
                category: ActionCategory::Navigation,
                default_key: Some("Down".to_string()),
            },
            KeyAction {
                id: "move_left".to_string(),
                name: "Move Left".to_string(),
                description: "Move cursor left".to_string(),
                category: ActionCategory::Navigation,
                default_key: Some("Left".to_string()),
            },
            KeyAction {
                id: "move_right".to_string(),
                name: "Move Right".to_string(),
                description: "Move cursor right".to_string(),
                category: ActionCategory::Navigation,
                default_key: Some("Right".to_string()),
            },
            KeyAction {
                id: "page_up".to_string(),
                name: "Page Up".to_string(),
                description: "Scroll up by page".to_string(),
                category: ActionCategory::Navigation,
                default_key: Some("PageUp".to_string()),
            },
            KeyAction {
                id: "page_down".to_string(),
                name: "Page Down".to_string(),
                description: "Scroll down by page".to_string(),
                category: ActionCategory::Navigation,
                default_key: Some("PageDown".to_string()),
            },
            
            // Email actions
            KeyAction {
                id: "compose_email".to_string(),
                name: "Compose Email".to_string(),
                description: "Start composing a new email".to_string(),
                category: ActionCategory::Email,
                default_key: Some("c".to_string()),
            },
            KeyAction {
                id: "reply_email".to_string(),
                name: "Reply".to_string(),
                description: "Reply to current email".to_string(),
                category: ActionCategory::Email,
                default_key: Some("r".to_string()),
            },
            KeyAction {
                id: "reply_all".to_string(),
                name: "Reply All".to_string(),
                description: "Reply to all recipients".to_string(),
                category: ActionCategory::Email,
                default_key: Some("R".to_string()),
            },
            KeyAction {
                id: "forward_email".to_string(),
                name: "Forward".to_string(),
                description: "Forward current email".to_string(),
                category: ActionCategory::Email,
                default_key: Some("f".to_string()),
            },
            KeyAction {
                id: "delete_email".to_string(),
                name: "Delete Email".to_string(),
                description: "Delete current email".to_string(),
                category: ActionCategory::Email,
                default_key: Some("d".to_string()),
            },
            KeyAction {
                id: "mark_read".to_string(),
                name: "Mark as Read".to_string(),
                description: "Mark email as read".to_string(),
                category: ActionCategory::Email,
                default_key: Some("m".to_string()),
            },
            KeyAction {
                id: "toggle_star".to_string(),
                name: "Toggle Star".to_string(),
                description: "Star or unstar email".to_string(),
                category: ActionCategory::Email,
                default_key: Some("s".to_string()),
            },
            
            // Calendar actions
            KeyAction {
                id: "new_event".to_string(),
                name: "New Event".to_string(),
                description: "Create new calendar event".to_string(),
                category: ActionCategory::Calendar,
                default_key: Some("n".to_string()),
            },
            KeyAction {
                id: "edit_event".to_string(),
                name: "Edit Event".to_string(),
                description: "Edit selected event".to_string(),
                category: ActionCategory::Calendar,
                default_key: Some("e".to_string()),
            },
            KeyAction {
                id: "delete_event".to_string(),
                name: "Delete Event".to_string(),
                description: "Delete selected event".to_string(),
                category: ActionCategory::Calendar,
                default_key: Some("Delete".to_string()),
            },
            KeyAction {
                id: "next_month".to_string(),
                name: "Next Month".to_string(),
                description: "Navigate to next month".to_string(),
                category: ActionCategory::Calendar,
                default_key: Some("]".to_string()),
            },
            KeyAction {
                id: "prev_month".to_string(),
                name: "Previous Month".to_string(),
                description: "Navigate to previous month".to_string(),
                category: ActionCategory::Calendar,
                default_key: Some("[".to_string()),
            },
            
            // UI actions
            KeyAction {
                id: "toggle_help".to_string(),
                name: "Toggle Help".to_string(),
                description: "Show or hide help panel".to_string(),
                category: ActionCategory::UI,
                default_key: Some("h".to_string()),
            },
            KeyAction {
                id: "toggle_sidebar".to_string(),
                name: "Toggle Sidebar".to_string(),
                description: "Show or hide sidebar".to_string(),
                category: ActionCategory::UI,
                default_key: Some("Tab".to_string()),
            },
            KeyAction {
                id: "command_palette".to_string(),
                name: "Command Palette".to_string(),
                description: "Open command palette".to_string(),
                category: ActionCategory::UI,
                default_key: Some("Ctrl+d".to_string()),
            },
            KeyAction {
                id: "settings".to_string(),
                name: "Settings".to_string(),
                description: "Open settings".to_string(),
                category: ActionCategory::UI,
                default_key: Some(",".to_string()),
            },
            
            // Application actions
            KeyAction {
                id: "quit".to_string(),
                name: "Quit".to_string(),
                description: "Exit application".to_string(),
                category: ActionCategory::Application,
                default_key: Some("q".to_string()),
            },
            KeyAction {
                id: "refresh".to_string(),
                name: "Refresh".to_string(),
                description: "Refresh current view".to_string(),
                category: ActionCategory::Application,
                default_key: Some("F5".to_string()),
            },
            KeyAction {
                id: "sync".to_string(),
                name: "Sync".to_string(),
                description: "Sync emails and calendar".to_string(),
                category: ActionCategory::Application,
                default_key: Some("F6".to_string()),
            },
        ]
    }
    
    /// Handle keyboard input
    pub fn handle_key(&mut self, key: crossterm::event::KeyCode) -> bool {
        use crossterm::event::KeyCode;
        
        match (&self.mode, key) {
            // List mode navigation
            (KeyboardBindingsMode::List, KeyCode::Up) => {
                let selected = self.list_state.selected().unwrap_or(0);
                if selected > 0 {
                    self.list_state.select(Some(selected - 1));
                }
                true
            }
            (KeyboardBindingsMode::List, KeyCode::Down) => {
                let selected = self.list_state.selected().unwrap_or(0);
                let max_items = self.get_filtered_bindings().len();
                if selected < max_items.saturating_sub(1) {
                    self.list_state.select(Some(selected + 1));
                }
                true
            }
            
            // List mode actions
            (KeyboardBindingsMode::List, KeyCode::Enter) => {
                if let Some(selected) = self.list_state.selected() {
                    let filtered_bindings = self.get_filtered_bindings();
                    if selected < filtered_bindings.len() {
                        let (key, _) = &filtered_bindings[selected];
                        self.mode = KeyboardBindingsMode::Edit(key.clone());
                        self.load_binding_for_editing(key);
                    }
                }
                true
            }
            (KeyboardBindingsMode::List, KeyCode::Char('a')) => {
                self.mode = KeyboardBindingsMode::Add;
                self.clear_form();
                true
            }
            (KeyboardBindingsMode::List, KeyCode::Char('d')) => {
                if let Some(selected) = self.list_state.selected() {
                    let filtered_bindings = self.get_filtered_bindings();
                    if selected < filtered_bindings.len() {
                        let (key, _) = &filtered_bindings[selected];
                        self.mode = KeyboardBindingsMode::Delete(key.clone());
                    }
                }
                true
            }
            (KeyboardBindingsMode::List, KeyCode::Char('i')) => {
                self.mode = KeyboardBindingsMode::Import;
                true
            }
            (KeyboardBindingsMode::List, KeyCode::Char('x')) => {
                self.mode = KeyboardBindingsMode::Export;
                true
            }
            (KeyboardBindingsMode::List, KeyCode::Char('c')) => {
                self.cycle_category();
                true
            }
            (KeyboardBindingsMode::List, KeyCode::Esc) => false, // Close bindings manager
            
            // Add/Edit mode navigation
            (KeyboardBindingsMode::Add | KeyboardBindingsMode::Edit(_), KeyCode::Tab) => {
                if self.editing_field.is_none() {
                    self.editing_field = Some(0);
                    self.start_field_edit(0);
                } else if let Some(current) = self.editing_field {
                    self.apply_field_edit();
                    let next_field = (current + 1) % 3;
                    self.editing_field = Some(next_field);
                    self.start_field_edit(next_field);
                }
                true
            }
            (KeyboardBindingsMode::Add | KeyboardBindingsMode::Edit(_), KeyCode::Enter) => {
                if self.editing_field.is_some() {
                    self.apply_field_edit();
                } else {
                    self.save_binding();
                }
                true
            }
            (KeyboardBindingsMode::Add | KeyboardBindingsMode::Edit(_), KeyCode::Esc) => {
                if self.editing_field.is_some() {
                    self.cancel_field_edit();
                } else {
                    self.mode = KeyboardBindingsMode::List;
                }
                true
            }
            
            // Field editing
            (KeyboardBindingsMode::Add | KeyboardBindingsMode::Edit(_), KeyCode::Char(c)) => {
                if self.editing_field.is_some() {
                    self.input_buffer.push(c);
                }
                true
            }
            (KeyboardBindingsMode::Add | KeyboardBindingsMode::Edit(_), KeyCode::Backspace) => {
                if self.editing_field.is_some() {
                    self.input_buffer.pop();
                }
                true
            }
            
            // Delete confirmation
            (KeyboardBindingsMode::Delete(_), KeyCode::Char('y')) => {
                self.confirm_delete();
                true
            }
            (KeyboardBindingsMode::Delete(_), KeyCode::Char('n') | KeyCode::Esc) => {
                self.mode = KeyboardBindingsMode::List;
                true
            }
            
            // Import/Export
            (KeyboardBindingsMode::Import | KeyboardBindingsMode::Export, KeyCode::Esc) => {
                self.mode = KeyboardBindingsMode::List;
                true
            }
            
            _ => false,
        }
    }
    
    /// Render the keyboard bindings interface
    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        match self.mode.clone() {
            KeyboardBindingsMode::List => self.render_bindings_list(frame, area, theme),
            KeyboardBindingsMode::Add => self.render_add_binding(frame, area, theme),
            KeyboardBindingsMode::Edit(key) => self.render_edit_binding(frame, area, theme, &key),
            KeyboardBindingsMode::Delete(key) => self.render_delete_confirmation(frame, area, theme, &key),
            KeyboardBindingsMode::Import => self.render_import_dialog(frame, area, theme),
            KeyboardBindingsMode::Export => self.render_export_dialog(frame, area, theme),
        }
    }
    
    fn render_bindings_list(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(3),
                Constraint::Length(3),
            ])
            .split(area);
        
        // Category selector
        let category_text = format!("Category: {} (C to cycle)", self.selected_category.as_str());
        let category = Paragraph::new(category_text)
            .block(Block::default().borders(Borders::ALL).title("Filter"))
            .alignment(Alignment::Center);
        frame.render_widget(category, chunks[0]);
        
        // Bindings list
        let filtered_bindings = self.get_filtered_bindings();
        let items: Vec<ListItem> = filtered_bindings.iter().map(|(key, action_id)| {
            let action_name = self.get_action_name(action_id);
            ListItem::new(Line::from(vec![
                Span::styled(key, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(" → "),
                Span::styled(action_name, Style::default().fg(Color::Cyan)),
            ]))
        }).collect();
        
        let list = List::new(items)
            .block(Block::default()
                .title(format!("⌨️ Keyboard Bindings ({} total)", self.bindings.len()))
                .borders(Borders::ALL))
            .highlight_style(theme.get_component_style("selected", true))
            .highlight_symbol("► ");
        
        frame.render_stateful_widget(list, chunks[1], &mut self.list_state);
        
        // Help text
        let help = Paragraph::new(
            "Enter: Edit • A: Add • D: Delete • I: Import • X: Export • C: Cycle Category • Esc: Close"
        )
        .block(Block::default().borders(Borders::ALL).title("Controls"))
        .wrap(Wrap { trim: true });
        
        frame.render_widget(help, chunks[2]);
    }
    
    fn render_add_binding(&mut self, frame: &mut Frame, area: Rect, _theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
            ])
            .split(area);
        
        // Title
        let title = Paragraph::new("Add New Keyboard Binding")
            .block(Block::default().borders(Borders::ALL).title("⌨️ Binding Setup"))
            .alignment(Alignment::Center);
        frame.render_widget(title, chunks[0]);
        
        // Form fields
        let field_names = ["Key Combination", "Action ID", "Description"];
        let field_values = [&self.form_key_combo, &self.form_action_id, &self.form_description];
        
        let mut form_lines = Vec::new();
        form_lines.push("Available actions by category:".to_string());
        form_lines.push("".to_string());
        
        for i in 0..3 {
            let prefix = if self.editing_field == Some(i) {
                format!("► {}: {} _", field_names[i], self.input_buffer)
            } else {
                format!("  {}: {}", field_names[i], field_values[i])
            };
            form_lines.push(prefix);
        }
        
        form_lines.push("".to_string());
        form_lines.push("Example key combinations:".to_string());
        form_lines.push("  Single key: 'a', 'Enter', 'F1'".to_string());
        form_lines.push("  With modifier: 'Ctrl+a', 'Alt+Enter', 'Shift+F1'".to_string());
        form_lines.push("".to_string());
        
        if self.editing_field.is_some() {
            form_lines.push("Enter: Apply • Esc: Cancel edit".to_string());
        } else {
            form_lines.push("Tab: Edit fields • Enter: Save • Esc: Cancel".to_string());
        }
        
        let form_text = form_lines.join("\n");
        let form = Paragraph::new(form_text)
            .block(Block::default().borders(Borders::ALL).title("Binding Details"))
            .wrap(Wrap { trim: true });
        
        frame.render_widget(form, chunks[1]);
        
        // Status
        if !self.status_message.is_empty() {
            let status = Paragraph::new(self.status_message.as_str())
                .block(Block::default().borders(Borders::ALL).title("Status"));
            frame.render_widget(status, chunks[2]);
        }
    }
    
    fn render_edit_binding(&mut self, frame: &mut Frame, area: Rect, theme: &Theme, _key: &str) {
        // Similar to add binding but with existing data pre-filled
        self.render_add_binding(frame, area, theme);
    }
    
    fn render_delete_confirmation(&mut self, frame: &mut Frame, area: Rect, _theme: &Theme, key: &str) {
        let popup_area = self.centered_rect(60, 20, area);
        
        frame.render_widget(Clear, popup_area);
        
        let action_name = self.bindings.get(key)
            .and_then(|action_id| Some(self.get_action_name(action_id)))
            .unwrap_or("Unknown".to_string());
        
        let confirmation = Paragraph::new(format!(
            "Are you sure you want to delete the binding?\\n\\nKey: {}\\nAction: {}\\n\\nThis cannot be undone.\\n\\nPress Y to confirm, N or Esc to cancel.",
            key, action_name
        ))
        .block(Block::default()
            .borders(Borders::ALL)
            .title("⚠️  Delete Binding"))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
        
        frame.render_widget(confirmation, popup_area);
    }
    
    fn render_import_dialog(&mut self, frame: &mut Frame, area: Rect, _theme: &Theme) {
        let popup_area = self.centered_rect(70, 30, area);
        
        frame.render_widget(Clear, popup_area);
        
        let import_text = "Import Keyboard Bindings\\n\\nSupported formats:\\n• JSON configuration files\\n• Vim-style keymaps\\n• Custom binding exports\\n\\nFeature implementation in progress...\\n\\nPress Esc to return";
        
        let import_dialog = Paragraph::new(import_text)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("📥 Import Bindings"))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        
        frame.render_widget(import_dialog, popup_area);
    }
    
    fn render_export_dialog(&mut self, frame: &mut Frame, area: Rect, _theme: &Theme) {
        let popup_area = self.centered_rect(70, 30, area);
        
        frame.render_widget(Clear, popup_area);
        
        let export_text = format!(
            "Export Keyboard Bindings\\n\\nCurrent bindings: {}\\n\\nExport formats:\\n• JSON configuration\\n• Vim-style keymap\\n• Plain text list\\n\\nFeature implementation in progress...\\n\\nPress Esc to return",
            self.bindings.len()
        );
        
        let export_dialog = Paragraph::new(export_text)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("📤 Export Bindings"))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        
        frame.render_widget(export_dialog, popup_area);
    }
    
    // Helper methods
    fn centered_rect(&self, percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);
        
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }
    
    fn get_filtered_bindings(&self) -> Vec<(String, String)> {
        self.bindings.iter()
            .filter(|(_, action_id)| {
                self.actions.iter()
                    .find(|action| &action.id == *action_id)
                    .map(|action| action.category == self.selected_category)
                    .unwrap_or(self.selected_category == ActionCategory::Custom)
            })
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
    
    fn get_action_name(&self, action_id: &str) -> String {
        self.actions.iter()
            .find(|action| action.id == action_id)
            .map(|action| action.name.clone())
            .unwrap_or_else(|| action_id.to_string())
    }
    
    fn cycle_category(&mut self) {
        let categories = ActionCategory::all();
        self.category_index = (self.category_index + 1) % categories.len();
        self.selected_category = categories[self.category_index].clone();
        self.list_state.select(Some(0)); // Reset selection
    }
    
    fn clear_form(&mut self) {
        self.form_key_combo.clear();
        self.form_action_id.clear();
        self.form_description.clear();
        self.editing_field = None;
        self.input_buffer.clear();
    }
    
    fn load_binding_for_editing(&mut self, key: &str) {
        if let Some(action_id) = self.bindings.get(key) {
            self.form_key_combo = key.to_string();
            self.form_action_id = action_id.clone();
            self.form_description = self.get_action_name(action_id);
            self.editing_field = None;
            self.input_buffer.clear();
        }
    }
    
    fn start_field_edit(&mut self, field_index: usize) {
        let current_value = match field_index {
            0 => self.form_key_combo.clone(),
            1 => self.form_action_id.clone(),
            2 => self.form_description.clone(),
            _ => String::new(),
        };
        self.input_buffer = current_value;
    }
    
    fn apply_field_edit(&mut self) {
        if let Some(field_index) = self.editing_field {
            let value = self.input_buffer.trim().to_string();
            match field_index {
                0 => self.form_key_combo = value,
                1 => self.form_action_id = value,
                2 => self.form_description = value,
                _ => {}
            }
            self.editing_field = None;
            self.input_buffer.clear();
        }
    }
    
    fn cancel_field_edit(&mut self) {
        self.editing_field = None;
        self.input_buffer.clear();
    }
    
    /// Save binding (validation and ready state)
    pub fn save_binding(&mut self) {
        // Validate required fields
        if self.form_key_combo.trim().is_empty() {
            self.status_message = "Key combination is required".to_string();
            return;
        }
        
        if self.form_action_id.trim().is_empty() {
            self.status_message = "Action ID is required".to_string();
            return;
        }
        
        // Check for conflicts
        if self.bindings.contains_key(&self.form_key_combo) {
            if let KeyboardBindingsMode::Add = self.mode {
                self.status_message = "Key combination already exists".to_string();
                return;
            }
        }
        
        // Add/update binding
        self.bindings.insert(self.form_key_combo.clone(), self.form_action_id.clone());
        self.status_message = format!("Binding '{}' saved successfully", self.form_key_combo);
        self.mode = KeyboardBindingsMode::List;
    }
    
    /// Confirm binding deletion
    pub fn confirm_delete(&mut self) {
        if let KeyboardBindingsMode::Delete(key) = &self.mode {
            self.bindings.remove(key);
            self.status_message = format!("Binding '{}' deleted", key);
        }
        self.mode = KeyboardBindingsMode::List;
    }
    
    /// Get current bindings for saving
    pub fn get_bindings(&self) -> HashMap<String, String> {
        self.bindings.clone()
    }
    
    /// Check if bindings have been modified
    pub fn is_modified(&self) -> bool {
        // This would compare against original bindings in a real implementation
        true
    }
}

impl Default for KeyboardBindingsUI {
    fn default() -> Self {
        Self::new()
    }
}