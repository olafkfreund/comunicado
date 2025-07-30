//! User interface for keyboard shortcuts customization
//!
//! This module provides a comprehensive UI for managing keyboard shortcuts including:
//! - Browsing and searching shortcuts by context
//! - Creating and editing key bindings
//! - Conflict resolution interface
//! - Import/export functionality
//! - Real-time binding testing

use crate::keyboard::customization::{
    KeyboardCustomizationManager, KeyboardAction, KeyBinding, KeyCombination, 
    KeyboardContext, KeyBindingPriority, ConflictResolution, KeyboardCustomizationError,
};
use crate::theme::Theme;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, 
        Table, Row, Cell, Tabs, Wrap,
    },
    Frame,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Keyboard shortcuts UI state
pub struct KeyboardShortcutsUI {
    manager: KeyboardCustomizationManager,
    
    // UI navigation
    current_tab: ShortcutsTab,
    focused_area: FocusedArea,
    
    // Shortcuts browsing
    current_context: KeyboardContext,
    context_list_state: ListState,
    shortcuts_list_state: ListState,
    shortcuts_filter: String,
    
    // Editing state
    edit_mode: EditMode,
    selected_action: Option<String>,
    selected_binding: Option<Uuid>,
    new_binding_state: NewBindingState,
    
    // Conflict resolution
    pending_conflicts: Vec<ConflictInfo>,
    conflict_resolution_state: ConflictResolutionState,
    
    // Settings
    conflict_resolution_mode: ConflictResolution,
    show_disabled_bindings: bool,
    group_by_category: bool,
    
    // Status and messages
    status_message: String,
    error_message: Option<String>,
    show_help: bool,
    
    // Import/Export
    import_export_state: ImportExportState,
}

/// Main tabs in shortcuts UI
#[derive(Debug, Clone, PartialEq)]
pub enum ShortcutsTab {
    Browse,
    Edit,
    Conflicts,
    Settings,
    ImportExport,
}

/// Focus areas within each tab
#[derive(Debug, Clone, PartialEq)]
pub enum FocusedArea {
    TabBar,
    ContextList,
    ShortcutsList,
    ActionDetails,
    EditForm,
    ConflictList,
    ConflictResolution,
    SettingsForm,
    ImportExportForm,
}

/// Edit modes for shortcuts
#[derive(Debug, Clone, PartialEq)]
pub enum EditMode {
    None,
    CreateBinding,
    EditBinding,
    DeleteBinding,
    TestBinding,
}

/// State for creating new key bindings
#[derive(Debug, Clone)]
pub struct NewBindingState {
    action_id: String,
    context: KeyboardContext,
    priority: KeyBindingPriority,
    capturing_key: bool,
    captured_key: Option<KeyCombination>,
    step: BindingCreationStep,
}

/// Steps in binding creation process
#[derive(Debug, Clone, PartialEq)]
pub enum BindingCreationStep {
    SelectAction,
    SelectContext,
    CaptureKey,
    SetPriority,
    Confirm,
}

/// Conflict information for UI display
#[derive(Debug, Clone)]
pub struct ConflictInfo {
    pub new_binding: KeyBinding,
    pub existing_binding: KeyBinding,
    pub action_name: String,
    pub existing_action_name: String,
    pub resolution: Option<ConflictResolutionChoice>,
}

/// User's choice for conflict resolution
#[derive(Debug, Clone, PartialEq)]
pub enum ConflictResolutionChoice {
    KeepExisting,
    UseNew,
    DisableBoth,
    ChangePriority,
    ChangeKey,
}

/// State for conflict resolution UI
#[derive(Debug, Clone)]
pub struct ConflictResolutionState {
    selected_conflict: usize,
    resolution_choice: Option<ConflictResolutionChoice>,
    alternative_key: Option<KeyCombination>,
    new_priority: Option<KeyBindingPriority>,
}

/// State for import/export functionality
#[derive(Debug, Clone)]
pub struct ImportExportState {
    mode: ImportExportMode,
    file_path: String,
    merge_on_import: bool,
    selected_actions: Vec<String>,
    export_format: ExportFormat,
}

/// Import/export modes
#[derive(Debug, Clone, PartialEq)]
pub enum ImportExportMode {
    None,
    Import,
    Export,
    Reset,
}

/// Export formats
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExportFormat {
    Json,
    Toml,
    Yaml,
    Csv,
}

/// Actions that can be performed in the shortcuts UI
#[derive(Debug, Clone)]
pub enum ShortcutsAction {
    SwitchTab(ShortcutsTab),
    SwitchContext(KeyboardContext),
    SelectShortcut(String),
    CreateBinding(String),
    EditBinding(Uuid),
    DeleteBinding(Uuid),
    TestBinding(KeyCombination),
    ResolveConflict(usize, ConflictResolutionChoice),
    ImportConfig(String, bool),
    ExportConfig(String, ExportFormat),
    ResetToDefaults,
    ToggleSetting(String),
    SaveChanges,
    CancelEdit,
    ShowHelp,
    Back,
}

impl KeyboardShortcutsUI {
    /// Create a new keyboard shortcuts UI
    pub fn new(manager: KeyboardCustomizationManager) -> Self {
        Self {
            manager,
            current_tab: ShortcutsTab::Browse,
            focused_area: FocusedArea::ContextList,
            current_context: KeyboardContext::Global,
            context_list_state: ListState::default(),
            shortcuts_list_state: ListState::default(),
            shortcuts_filter: String::new(),
            edit_mode: EditMode::None,
            selected_action: None,
            selected_binding: None,
            new_binding_state: NewBindingState {
                action_id: String::new(),
                context: KeyboardContext::Global,
                priority: KeyBindingPriority::User,
                capturing_key: false,
                captured_key: None,
                step: BindingCreationStep::SelectAction,
            },
            pending_conflicts: Vec::new(),
            conflict_resolution_state: ConflictResolutionState {
                selected_conflict: 0,
                resolution_choice: None,
                alternative_key: None,
                new_priority: None,
            },
            conflict_resolution_mode: ConflictResolution::Priority,
            show_disabled_bindings: false,
            group_by_category: true,
            status_message: "Ready".to_string(),
            error_message: None,
            show_help: false,
            import_export_state: ImportExportState {
                mode: ImportExportMode::None,
                file_path: String::new(),
                merge_on_import: true,
                selected_actions: Vec::new(),
                export_format: ExportFormat::Json,
            },
        }
    }
    
    /// Handle keyboard input
    pub async fn handle_key(
        &mut self,
        key: KeyCode,
        modifiers: KeyModifiers,
    ) -> Option<ShortcutsAction> {
        // Check if we're capturing a key for binding creation
        if self.new_binding_state.capturing_key {
            let captured_key = KeyCombination::new(key, modifiers);
            self.new_binding_state.captured_key = Some(captured_key.clone());
            self.new_binding_state.capturing_key = false;
            self.new_binding_state.step = BindingCreationStep::SetPriority;
            return Some(ShortcutsAction::TestBinding(captured_key));
        }
        
        // Global shortcuts
        match key {
            KeyCode::F(1) => return Some(ShortcutsAction::ShowHelp),
            KeyCode::Tab => {
                self.next_tab();
                return None;
            }
            KeyCode::BackTab => {
                self.previous_tab();
                return None;
            }
            KeyCode::Esc => {
                if self.show_help {
                    self.show_help = false;
                } else if self.error_message.is_some() {
                    self.error_message = None;
                } else if self.edit_mode != EditMode::None {
                    return Some(ShortcutsAction::CancelEdit);
                } else {
                    return Some(ShortcutsAction::Back);
                }
                return None;
            }
            _ => {}
        }
        
        // Tab-specific shortcuts
        match self.current_tab {
            ShortcutsTab::Browse => self.handle_browse_key(key, modifiers),
            ShortcutsTab::Edit => self.handle_edit_key(key, modifiers),
            ShortcutsTab::Conflicts => self.handle_conflicts_key(key, modifiers),
            ShortcutsTab::Settings => self.handle_settings_key(key, modifiers),
            ShortcutsTab::ImportExport => self.handle_import_export_key(key, modifiers),
        }
    }
    
    /// Handle keys in browse tab
    fn handle_browse_key(&mut self, key: KeyCode, _modifiers: KeyModifiers) -> Option<ShortcutsAction> {
        match self.focused_area {
            FocusedArea::ContextList => {
                match key {
                    KeyCode::Up => {
                        self.previous_context();
                        None
                    }
                    KeyCode::Down => {
                        self.next_context();
                        None
                    }
                    KeyCode::Enter | KeyCode::Right => {
                        self.focused_area = FocusedArea::ShortcutsList;
                        None
                    }
                    _ => None,
                }
            }
            FocusedArea::ShortcutsList => {
                match key {
                    KeyCode::Up => {
                        self.previous_shortcut();
                        None
                    }
                    KeyCode::Down => {
                        self.next_shortcut();
                        None
                    }
                    KeyCode::Left => {
                        self.focused_area = FocusedArea::ContextList;
                        None
                    }
                    KeyCode::Enter => {
                        if let Some(action_id) = self.get_selected_action() {
                            Some(ShortcutsAction::SelectShortcut(action_id))
                        } else {
                            None
                        }
                    }
                    KeyCode::Char('e') => {
                        if let Some(binding_id) = self.get_selected_binding_id() {
                            Some(ShortcutsAction::EditBinding(binding_id))
                        } else {
                            None
                        }
                    }
                    KeyCode::Char('d') => {
                        if let Some(binding_id) = self.get_selected_binding_id() {
                            Some(ShortcutsAction::DeleteBinding(binding_id))
                        } else {
                            None
                        }
                    }
                    KeyCode::Char('n') => {
                        if let Some(action_id) = self.get_selected_action() {
                            Some(ShortcutsAction::CreateBinding(action_id))
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }
    
    /// Handle keys in edit tab
    fn handle_edit_key(&mut self, key: KeyCode, _modifiers: KeyModifiers) -> Option<ShortcutsAction> {
        match self.new_binding_state.step {
            BindingCreationStep::CaptureKey => {
                match key {
                    KeyCode::Char(' ') => {
                        self.new_binding_state.capturing_key = true;
                        self.status_message = "Press the key combination to bind...".to_string();
                        None
                    }
                    KeyCode::Enter => {
                        if self.new_binding_state.captured_key.is_some() {
                            self.new_binding_state.step = BindingCreationStep::SetPriority;
                        }
                        None
                    }
                    _ => None,
                }
            }
            BindingCreationStep::SetPriority => {
                match key {
                    KeyCode::Char('1') => {
                        self.new_binding_state.priority = KeyBindingPriority::Default;
                        None
                    }
                    KeyCode::Char('2') => {
                        self.new_binding_state.priority = KeyBindingPriority::Plugin;
                        None
                    }
                    KeyCode::Char('3') => {
                        self.new_binding_state.priority = KeyBindingPriority::User;
                        None
                    }
                    KeyCode::Char('4') => {
                        self.new_binding_state.priority = KeyBindingPriority::System;
                        None
                    }
                    KeyCode::Enter => {
                        self.new_binding_state.step = BindingCreationStep::Confirm;
                        None
                    }
                    _ => None,
                }
            }
            BindingCreationStep::Confirm => {
                match key {
                    KeyCode::Char('y') | KeyCode::Enter => {
                        Some(ShortcutsAction::SaveChanges)
                    }
                    KeyCode::Char('n') => {
                        Some(ShortcutsAction::CancelEdit)
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }
    
    /// Handle keys in conflicts tab
    fn handle_conflicts_key(&mut self, key: KeyCode, _modifiers: KeyModifiers) -> Option<ShortcutsAction> {
        match key {
            KeyCode::Up => {
                if self.conflict_resolution_state.selected_conflict > 0 {
                    self.conflict_resolution_state.selected_conflict -= 1;
                }
                None
            }
            KeyCode::Down => {
                if self.conflict_resolution_state.selected_conflict < self.pending_conflicts.len().saturating_sub(1) {
                    self.conflict_resolution_state.selected_conflict += 1;
                }
                None
            }
            KeyCode::Char('1') => {
                Some(ShortcutsAction::ResolveConflict(
                    self.conflict_resolution_state.selected_conflict,
                    ConflictResolutionChoice::KeepExisting,
                ))
            }
            KeyCode::Char('2') => {
                Some(ShortcutsAction::ResolveConflict(
                    self.conflict_resolution_state.selected_conflict,
                    ConflictResolutionChoice::UseNew,
                ))
            }
            KeyCode::Char('3') => {
                Some(ShortcutsAction::ResolveConflict(
                    self.conflict_resolution_state.selected_conflict,
                    ConflictResolutionChoice::DisableBoth,
                ))
            }
            _ => None,
        }
    }
    
    /// Handle keys in settings tab
    fn handle_settings_key(&mut self, key: KeyCode, _modifiers: KeyModifiers) -> Option<ShortcutsAction> {
        match key {
            KeyCode::Char('1') => Some(ShortcutsAction::ToggleSetting("show_disabled".to_string())),
            KeyCode::Char('2') => Some(ShortcutsAction::ToggleSetting("group_by_category".to_string())),
            KeyCode::Char('3') => Some(ShortcutsAction::ToggleSetting("conflict_resolution".to_string())),
            _ => None,
        }
    }
    
    /// Handle keys in import/export tab
    fn handle_import_export_key(&mut self, key: KeyCode, _modifiers: KeyModifiers) -> Option<ShortcutsAction> {
        match key {
            KeyCode::Char('i') => {
                self.import_export_state.mode = ImportExportMode::Import;
                None
            }
            KeyCode::Char('e') => {
                self.import_export_state.mode = ImportExportMode::Export;
                None
            }
            KeyCode::Char('r') => {
                Some(ShortcutsAction::ResetToDefaults)
            }
            KeyCode::Enter => {
                match self.import_export_state.mode {
                    ImportExportMode::Import => {
                        Some(ShortcutsAction::ImportConfig(
                            self.import_export_state.file_path.clone(),
                            self.import_export_state.merge_on_import,
                        ))
                    }
                    ImportExportMode::Export => {
                        Some(ShortcutsAction::ExportConfig(
                            self.import_export_state.file_path.clone(),
                            self.import_export_state.export_format.clone(),
                        ))
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }
    
    /// Render the shortcuts UI
    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if self.show_help {
            self.render_help_overlay(frame, area, theme);
            return;
        }
        
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Tab bar
                Constraint::Min(0),    // Content
                Constraint::Length(3), // Status bar
            ])
            .split(area);
        
        // Render tab bar
        self.render_tab_bar(frame, chunks[0], theme);
        
        // Render content based on current tab
        match self.current_tab {
            ShortcutsTab::Browse => self.render_browse_tab(frame, chunks[1], theme),
            ShortcutsTab::Edit => self.render_edit_tab(frame, chunks[1], theme),
            ShortcutsTab::Conflicts => self.render_conflicts_tab(frame, chunks[1], theme),
            ShortcutsTab::Settings => self.render_settings_tab(frame, chunks[1], theme),
            ShortcutsTab::ImportExport => self.render_import_export_tab(frame, chunks[1], theme),
        }
        
        // Render status bar
        self.render_status_bar(frame, chunks[2], theme);
        
        // Render error overlay if needed
        if self.error_message.is_some() {
            self.render_error_overlay(frame, area, theme);
        }
    }
    
    /// Render tab bar
    fn render_tab_bar(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let tab_titles = vec!["Browse", "Edit", "Conflicts", "Settings", "Import/Export"];
        
        let selected_tab = match self.current_tab {
            ShortcutsTab::Browse => 0,
            ShortcutsTab::Edit => 1,
            ShortcutsTab::Conflicts => 2,
            ShortcutsTab::Settings => 3,
            ShortcutsTab::ImportExport => 4,
        };
        
        let tabs = Tabs::new(tab_titles)
            .block(Block::default().borders(Borders::ALL).title("Keyboard Shortcuts"))
            .style(theme.get_component_style("default", false))
            .highlight_style(theme.get_component_style("selected", true))
            .select(selected_tab);
        
        frame.render_widget(tabs, area);
    }
    
    /// Render browse tab
    fn render_browse_tab(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25), // Context list
                Constraint::Percentage(75), // Shortcuts list
            ])
            .split(area);
        
        // Render context list
        self.render_context_list(frame, chunks[0], theme);
        
        // Render shortcuts list
        self.render_shortcuts_list(frame, chunks[1], theme);
    }
    
    /// Render context list
    fn render_context_list(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let contexts = vec![
            KeyboardContext::Global,
            KeyboardContext::Email,
            KeyboardContext::Compose,
            KeyboardContext::Calendar,
            KeyboardContext::Search,
            KeyboardContext::DraftList,
        ];
        
        let items: Vec<ListItem> = contexts
            .iter()
            .map(|context| {
                let name = context.to_string();
                let count = self.manager.get_actions_for_context(context).len();
                let content = format!("{} ({})", name, count);
                
                let style = if *context == self.current_context {
                    theme.get_component_style("selected", true)
                } else {
                    theme.get_component_style("default", false)
                };
                
                ListItem::new(content).style(style)
            })
            .collect();
        
        let context_list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Contexts")
                    .border_style(if self.focused_area == FocusedArea::ContextList {
                        theme.get_component_style("border", true)
                    } else {
                        theme.get_component_style("border", false)
                    }),
            )
            .highlight_style(theme.get_component_style("highlight", true))
            .highlight_symbol("→ ");
        
        frame.render_stateful_widget(context_list, area, &mut self.context_list_state);
    }
    
    /// Render shortcuts list
    fn render_shortcuts_list(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let actions = self.manager.get_actions_for_context(&self.current_context);
        
        // Create table data
        let mut rows = Vec::new();
        for action in actions {
            let bindings = self.manager.get_bindings_for_action(&action.id);
            
            if bindings.is_empty() {
                // Show action without binding
                rows.push(Row::new(vec![
                    Cell::from(action.name.clone()),
                    Cell::from("None"),
                    Cell::from(action.description.clone()),
                ]));
            } else {
                for binding in bindings {
                    if binding.enabled || self.show_disabled_bindings {
                        let key_display = binding.key_combination.to_string();
                        let status = if binding.enabled { "✓" } else { "✗" };
                        
                        rows.push(Row::new(vec![
                            Cell::from(format!("{} {}", status, action.name)),
                            Cell::from(key_display),
                            Cell::from(action.description.clone()),
                        ]));
                    }
                }
            }
        }
        
        let shortcuts_table = Table::new(
            rows,
            [
                Constraint::Percentage(30), // Action
                Constraint::Percentage(20), // Key
                Constraint::Percentage(50), // Description
            ],
        )
        .header(
            Row::new(vec!["Action", "Key", "Description"])
                .style(theme.get_component_style("header", false))
                .height(1),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Shortcuts")
                .border_style(if self.focused_area == FocusedArea::ShortcutsList {
                    theme.get_component_style("border", true)
                } else {
                    theme.get_component_style("border", false)
                }),
        )
        .highlight_style(theme.get_component_style("selected", true))
        .highlight_symbol("→ ");
        
        frame.render_stateful_widget(shortcuts_table, area, &mut self.shortcuts_list_state);
    }
    
    /// Render edit tab
    fn render_edit_tab(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        match self.new_binding_state.step {
            BindingCreationStep::CaptureKey => {
                self.render_key_capture(frame, area, theme);
            }
            BindingCreationStep::SetPriority => {
                self.render_priority_selection(frame, area, theme);
            }
            BindingCreationStep::Confirm => {
                self.render_binding_confirmation(frame, area, theme);
            }
            _ => {
                self.render_edit_instructions(frame, area, theme);
            }
        }
    }
    
    /// Render key capture interface
    fn render_key_capture(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let text = if self.new_binding_state.capturing_key {
            vec![
                Line::from("Press the key combination you want to bind..."),
                Line::from("(Press Escape to cancel)"),
            ]
        } else {
            vec![
                Line::from("Press Space to start capturing a key combination"),
                Line::from(""),
                Line::from("Or press Enter to proceed if you've already captured a key"),
            ]
        };
        
        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("Capture Key Combination"))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        
        frame.render_widget(paragraph, area);
    }
    
    /// Render priority selection
    fn render_priority_selection(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let text = vec![
            Line::from("Select binding priority:"),
            Line::from(""),
            Line::from("1 - Default (lowest priority)"),
            Line::from("2 - Plugin"),
            Line::from("3 - User (recommended)"),
            Line::from("4 - System (highest priority)"),
            Line::from(""),
            Line::from("Press Enter to continue"),
        ];
        
        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("Set Priority"))
            .wrap(Wrap { trim: true });
        
        frame.render_widget(paragraph, area);
    }
    
    /// Render binding confirmation
    fn render_binding_confirmation(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let key_str = self.new_binding_state.captured_key
            .as_ref()
            .map(|k| k.to_string())
            .unwrap_or_else(|| "None".to_string());
        
        let text = vec![
            Line::from("Confirm new key binding:"),
            Line::from(""),
            Line::from(format!("Action: {}", self.new_binding_state.action_id)),
            Line::from(format!("Key: {}", key_str)),
            Line::from(format!("Context: {}", self.new_binding_state.context)),
            Line::from(format!("Priority: {:?}", self.new_binding_state.priority)),
            Line::from(""),
            Line::from("Press Y to confirm, N to cancel"),
        ];
        
        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("Confirm Binding"))
            .wrap(Wrap { trim: true });
        
        frame.render_widget(paragraph, area);
    }
    
    /// Render edit instructions
    fn render_edit_instructions(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let text = vec![
            Line::from("Keyboard Shortcut Editor"),
            Line::from(""),
            Line::from("Select an action from the Browse tab,"),
            Line::from("then press 'n' to create a new binding"),
            Line::from("or 'e' to edit an existing binding."),
        ];
        
        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("Edit Shortcuts"))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        
        frame.render_widget(paragraph, area);
    }
    
    /// Render conflicts tab
    fn render_conflicts_tab(&self, frame: &mut Frame, _area: Rect, _theme: &Theme) {
        // TODO: Implement conflicts rendering
    }
    
    /// Render settings tab
    fn render_settings_tab(&self, frame: &mut Frame, _area: Rect, _theme: &Theme) {
        // TODO: Implement settings rendering
    }
    
    /// Render import/export tab
    fn render_import_export_tab(&self, frame: &mut Frame, _area: Rect, _theme: &Theme) {
        // TODO: Implement import/export rendering
    }
    
    /// Render status bar
    fn render_status_bar(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let status = Paragraph::new(self.status_message.clone())
            .block(Block::default().borders(Borders::ALL))
            .style(theme.get_component_style("status", false));
        
        frame.render_widget(status, area);
    }
    
    /// Render help overlay
    fn render_help_overlay(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let popup_area = centered_rect(80, 60, area);
        
        frame.render_widget(Clear, popup_area);
        
        let help_text = vec![
            Line::from(Span::styled("Keyboard Shortcuts Help", 
                Style::default().add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from("Global:"),
            Line::from("  F1 - Show this help"),
            Line::from("  Tab/Shift+Tab - Switch tabs"),
            Line::from("  Esc - Go back/Cancel"),
            Line::from(""),
            Line::from("Browse:"),
            Line::from("  ↑/↓ - Navigate"),
            Line::from("  Enter - Select"),
            Line::from("  n - New binding"),
            Line::from("  e - Edit binding"),
            Line::from("  d - Delete binding"),
        ];
        
        let help = Paragraph::new(help_text)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Help")
                .title_alignment(Alignment::Center))
            .wrap(Wrap { trim: true });
        
        frame.render_widget(help, popup_area);
    }
    
    /// Render error overlay
    fn render_error_overlay(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if let Some(error_msg) = &self.error_message {
            let popup_area = centered_rect(60, 20, area);
            
            frame.render_widget(Clear, popup_area);
            
            let error_text = vec![
                Line::from(Span::styled("Error", 
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))),
                Line::from(""),
                Line::from(error_msg.as_str()),
                Line::from(""),
                Line::from("Press Esc to dismiss."),
            ];
            
            let error = Paragraph::new(error_text)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Red)))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });
            
            frame.render_widget(error, popup_area);
        }
    }
    
    /// Navigation methods
    fn next_tab(&mut self) {
        self.current_tab = match self.current_tab {
            ShortcutsTab::Browse => ShortcutsTab::Edit,
            ShortcutsTab::Edit => ShortcutsTab::Conflicts,
            ShortcutsTab::Conflicts => ShortcutsTab::Settings,
            ShortcutsTab::Settings => ShortcutsTab::ImportExport,
            ShortcutsTab::ImportExport => ShortcutsTab::Browse,
        };
    }
    
    fn previous_tab(&mut self) {
        self.current_tab = match self.current_tab {
            ShortcutsTab::Browse => ShortcutsTab::ImportExport,
            ShortcutsTab::Edit => ShortcutsTab::Browse,
            ShortcutsTab::Conflicts => ShortcutsTab::Edit,
            ShortcutsTab::Settings => ShortcutsTab::Conflicts,
            ShortcutsTab::ImportExport => ShortcutsTab::Settings,
        };
    }
    
    fn next_context(&mut self) {
        // TODO: Implement context navigation
    }
    
    fn previous_context(&mut self) {
        // TODO: Implement context navigation
    }
    
    fn next_shortcut(&mut self) {
        // TODO: Implement shortcut navigation
    }
    
    fn previous_shortcut(&mut self) {
        // TODO: Implement shortcut navigation
    }
    
    fn get_selected_action(&self) -> Option<String> {
        // TODO: Implement get selected action
        None
    }
    
    fn get_selected_binding_id(&self) -> Option<Uuid> {
        // TODO: Implement get selected binding ID
        None
    }
}

/// Create a centered rectangle
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_shortcuts_ui_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("keyboard.json");
        let manager = KeyboardCustomizationManager::new(config_path).unwrap();
        
        let ui = KeyboardShortcutsUI::new(manager);
        
        assert_eq!(ui.current_tab, ShortcutsTab::Browse);
        assert_eq!(ui.focused_area, FocusedArea::ContextList);
        assert_eq!(ui.current_context, KeyboardContext::Global);
    }

    #[test]
    fn test_tab_navigation() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("keyboard.json");
        let manager = KeyboardCustomizationManager::new(config_path).unwrap();
        
        let mut ui = KeyboardShortcutsUI::new(manager);
        
        assert_eq!(ui.current_tab, ShortcutsTab::Browse);
        
        ui.next_tab();
        assert_eq!(ui.current_tab, ShortcutsTab::Edit);
        
        ui.previous_tab();
        assert_eq!(ui.current_tab, ShortcutsTab::Browse);
    }
}