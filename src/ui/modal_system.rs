//! Standardized modal dialog system for consistent UX
//!
//! This module provides a unified modal dialog system with consistent styling,
//! behavior, and accessibility features. It includes various modal types like
//! confirmation dialogs, input forms, information displays, and custom modals.

use crate::theme::Theme;
use crate::ui::form_validation::{FormValidationSystem, ValidationRule};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect, Margin},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap, Gauge},
    Frame,
};
use std::collections::HashMap;

/// Modal dialog types with different behaviors and layouts
#[derive(Debug, Clone, PartialEq)]
pub enum ModalType {
    Confirmation,   // Yes/No confirmation
    Information,    // Information display with OK button
    Warning,        // Warning message with acknowledgment
    Error,          // Error message with acknowledgment
    Input,          // Input form with validation
    Choice,         // Multiple choice selection
    Progress,       // Progress indicator
    Custom,         // Custom content
}

/// Modal size presets
#[derive(Debug, Clone, PartialEq)]
pub enum ModalSize {
    Small,    // 40x8
    Medium,   // 60x12
    Large,    // 80x20
    FullScreen, // 95% of screen
    Custom { width: u16, height: u16 },
}

/// Modal button configuration
#[derive(Debug, Clone)]
pub struct ModalButton {
    pub label: String,
    pub action: String,
    pub style: ButtonStyle,
    pub is_default: bool,
    pub shortcut: Option<char>,
}

/// Button styling options
#[derive(Debug, Clone, PartialEq)]
pub enum ButtonStyle {
    Primary,    // Accent color, for main actions
    Secondary,  // Muted color, for cancel/back actions
    Danger,     // Warning color, for destructive actions
    Success,    // Success color, for positive actions
}

/// Modal dialog state and configuration
pub struct Modal {
    pub id: String,
    pub modal_type: ModalType,
    pub size: ModalSize,
    pub title: String,
    pub content: ModalContent,
    pub buttons: Vec<ModalButton>,
    pub is_visible: bool,
    pub is_closable: bool, // Can be closed with Escape
    pub auto_close_delay: Option<std::time::Duration>,
    pub created_at: std::time::Instant,
    
    // Navigation state
    pub selected_button: usize,
    pub selected_item: usize,
    pub list_state: ListState,
    
    // Form state (for input modals)
    pub form_validation: Option<FormValidationSystem>,
    pub input_values: HashMap<String, String>,
    pub focused_field: Option<String>,
    
    // Custom state
    pub custom_data: HashMap<String, String>,
}

/// Modal content types
#[derive(Debug, Clone)]
pub enum ModalContent {
    Text(String),
    RichText(Vec<Line<'static>>),
    List(Vec<String>),
    Form(Vec<FormField>),
    Progress { current: u64, total: u64, message: String },
    Custom(String), // Identifier for custom renderer
}

/// Form field definition for input modals
#[derive(Debug, Clone)]
pub struct FormField {
    pub name: String,
    pub label: String,
    pub field_type: FormFieldType,
    pub validation_rules: Vec<ValidationRule>,
    pub placeholder: Option<String>,
    pub help_text: Option<String>,
}

/// Form field types
#[derive(Debug, Clone)]
pub enum FormFieldType {
    Text,
    Email,
    Password,
    Number,
    TextArea,
    Select(Vec<String>),
    Checkbox,
}

/// Modal system manager
pub struct ModalSystem {
    modals: Vec<Modal>,
    z_index_counter: usize,
    theme_override: Option<Theme>,
    global_shortcuts_enabled: bool,
}

impl ModalSystem {
    /// Create a new modal system
    pub fn new() -> Self {
        Self {
            modals: Vec::new(),
            z_index_counter: 1000,
            theme_override: None,
            global_shortcuts_enabled: true,
        }
    }

    /// Show a confirmation dialog
    pub fn show_confirmation(
        &mut self,
        id: String,
        title: String,
        message: String,
        confirm_label: Option<String>,
        cancel_label: Option<String>,
    ) -> &mut Modal {
        let modal = Modal::new(
            id,
            ModalType::Confirmation,
            ModalSize::Medium,
            title,
            ModalContent::Text(message),
        )
        .with_buttons(vec![
            ModalButton {
                label: cancel_label.unwrap_or_else(|| "Cancel".to_string()),
                action: "cancel".to_string(),
                style: ButtonStyle::Secondary,
                is_default: false,
                shortcut: Some('c'),
            },
            ModalButton {
                label: confirm_label.unwrap_or_else(|| "Confirm".to_string()),
                action: "confirm".to_string(),
                style: ButtonStyle::Primary,
                is_default: true,
                shortcut: Some('y'),
            },
        ])
        .closable(true);

        self.add_modal(modal)
    }

    /// Show an information dialog
    pub fn show_info(&mut self, id: String, title: String, message: String) -> &mut Modal {
        let modal = Modal::new(
            id,
            ModalType::Information,
            ModalSize::Medium,
            title,
            ModalContent::Text(message),
        )
        .with_buttons(vec![ModalButton {
            label: "OK".to_string(),
            action: "ok".to_string(),
            style: ButtonStyle::Primary,
            is_default: true,
            shortcut: Some('o'),
        }])
        .closable(true);

        self.add_modal(modal)
    }

    /// Show a warning dialog
    pub fn show_warning(&mut self, id: String, title: String, message: String) -> &mut Modal {
        let modal = Modal::new(
            id,
            ModalType::Warning,
            ModalSize::Medium,
            title,
            ModalContent::Text(message),
        )
        .with_buttons(vec![ModalButton {
            label: "Understood".to_string(),
            action: "acknowledge".to_string(),
            style: ButtonStyle::Primary,
            is_default: true,
            shortcut: Some('u'),
        }])
        .closable(true);

        self.add_modal(modal)
    }

    /// Show an error dialog
    pub fn show_error(&mut self, id: String, title: String, message: String) -> &mut Modal {
        let modal = Modal::new(
            id,
            ModalType::Error,
            ModalSize::Medium,
            title,
            ModalContent::Text(message),
        )
        .with_buttons(vec![ModalButton {
            label: "OK".to_string(),
            action: "ok".to_string(),
            style: ButtonStyle::Danger,
            is_default: true,
            shortcut: Some('o'),
        }])
        .closable(true);

        self.add_modal(modal)
    }

    /// Show an input form dialog
    pub fn show_input_form(
        &mut self,
        id: String,
        title: String,
        fields: Vec<FormField>,
    ) -> &mut Modal {
        let modal = Modal::new(
            id,
            ModalType::Input,
            ModalSize::Large,
            title,
            ModalContent::Form(fields),
        )
        .with_buttons(vec![
            ModalButton {
                label: "Cancel".to_string(),
                action: "cancel".to_string(),
                style: ButtonStyle::Secondary,
                is_default: false,
                shortcut: Some('c'),
            },
            ModalButton {
                label: "Submit".to_string(),
                action: "submit".to_string(),
                style: ButtonStyle::Primary,
                is_default: true,
                shortcut: Some('s'),
            },
        ])
        .closable(true);

        self.add_modal(modal)
    }

    /// Show a choice selection dialog
    pub fn show_choice(
        &mut self,
        id: String,
        title: String,
        message: String,
        choices: Vec<String>,
    ) -> &mut Modal {
        let modal = Modal::new(
            id,
            ModalType::Choice,
            ModalSize::Medium,
            title,
            ModalContent::List(choices),
        )
        .with_buttons(vec![
            ModalButton {
                label: "Cancel".to_string(),
                action: "cancel".to_string(),
                style: ButtonStyle::Secondary,
                is_default: false,
                shortcut: Some('c'),
            },
            ModalButton {
                label: "Select".to_string(),
                action: "select".to_string(),
                style: ButtonStyle::Primary,
                is_default: true,
                shortcut: Some('s'),
            },
        ])
        .closable(true);

        self.add_modal(modal)
    }

    /// Show a progress dialog
    pub fn show_progress(
        &mut self,
        id: String,
        title: String,
        message: String,
        current: u64,
        total: u64,
    ) -> &mut Modal {
        let modal = Modal::new(
            id,
            ModalType::Progress,
            ModalSize::Medium,
            title,
            ModalContent::Progress { current, total, message },
        )
        .closable(false); // Progress dialogs are not typically closable

        self.add_modal(modal)
    }

    /// Add a modal to the system
    fn add_modal(&mut self, modal: Modal) -> &mut Modal {
        self.z_index_counter += 1;
        self.modals.push(modal);
        self.modals.last_mut().unwrap()
    }

    /// Close a modal by ID
    pub fn close_modal(&mut self, id: &str) {
        self.modals.retain(|modal| modal.id != id);
    }

    /// Close the top-most modal
    pub fn close_top_modal(&mut self) {
        if !self.modals.is_empty() {
            self.modals.pop();
        }
    }

    /// Get the top-most modal
    pub fn get_top_modal(&mut self) -> Option<&mut Modal> {
        self.modals.last_mut()
    }

    /// Check if any modal is visible
    pub fn has_visible_modals(&self) -> bool {
        self.modals.iter().any(|modal| modal.is_visible)
    }

    /// Update progress for a progress modal
    pub fn update_progress(&mut self, id: &str, current: u64, total: u64, message: Option<String>) {
        if let Some(modal) = self.modals.iter_mut().find(|m| m.id == id) {
            if let ModalContent::Progress { current: c, total: t, message: msg } = &mut modal.content {
                *c = current;
                *t = total;
                if let Some(new_message) = message {
                    *msg = new_message;
                }
            }
        }
    }

    /// Render all visible modals
    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        for modal in &mut self.modals {
            if modal.is_visible {
                self.render_modal(frame, area, modal, theme);
            }
        }
    }

    /// Render a single modal
    fn render_modal(&self, frame: &mut Frame, area: Rect, modal: &mut Modal, theme: &Theme) {
        let modal_area = self.calculate_modal_area(area, &modal.size);
        
        // Clear the modal area
        frame.render_widget(Clear, modal_area);
        
        // Create modal layout
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title bar
                Constraint::Min(0),    // Content
                Constraint::Length(3), // Button bar
            ])
            .split(modal_area);

        // Render modal background and title
        self.render_modal_title(frame, layout[0], modal, theme);
        
        // Render content based on modal type
        self.render_modal_content(frame, layout[1], modal, theme);
        
        // Render buttons
        self.render_modal_buttons(frame, layout[2], modal, theme);
    }

    /// Calculate modal area based on size
    fn calculate_modal_area(&self, area: Rect, size: &ModalSize) -> Rect {
        let (width, height) = match size {
            ModalSize::Small => (40, 8),
            ModalSize::Medium => (60, 12),
            ModalSize::Large => (80, 20),
            ModalSize::FullScreen => (
                (area.width as f32 * 0.95) as u16,
                (area.height as f32 * 0.95) as u16,
            ),
            ModalSize::Custom { width, height } => (*width, *height),
        };

        let x = area.width.saturating_sub(width) / 2;
        let y = area.height.saturating_sub(height) / 2;

        Rect {
            x: area.x + x,
            y: area.y + y,
            width: width.min(area.width),
            height: height.min(area.height),
        }
    }

    /// Render modal title bar
    fn render_modal_title(&self, frame: &mut Frame, area: Rect, modal: &Modal, theme: &Theme) {
        let title_style = match modal.modal_type {
            ModalType::Error => Style::default().fg(theme.colors.palette.error),
            ModalType::Warning => Style::default().fg(theme.colors.palette.warning),
            ModalType::Information => Style::default().fg(theme.colors.palette.info),
            ModalType::Progress => Style::default().fg(theme.colors.palette.info),
            _ => Style::default().fg(theme.colors.palette.accent),
        };

        let title_icon = match modal.modal_type {
            ModalType::Confirmation => "❓",
            ModalType::Information => "ℹ️",
            ModalType::Warning => "⚠️",
            ModalType::Error => "❌",
            ModalType::Input => "✏️",
            ModalType::Choice => "📋",
            ModalType::Progress => "⏳",
            ModalType::Custom => "🔧",
        };

        let title = format!("{} {}", title_icon, modal.title);
        
        let title_block = Block::default()
            .borders(Borders::ALL)
            .border_style(title_style)
            .title(title);

        let title_paragraph = Paragraph::new("")
            .block(title_block)
            .alignment(Alignment::Center);

        frame.render_widget(title_paragraph, area);
    }

    /// Render modal content
    fn render_modal_content(&self, frame: &mut Frame, area: Rect, modal: &mut Modal, theme: &Theme) {
        match &modal.content {
            ModalContent::Text(text) => {
                let paragraph = Paragraph::new(text.clone())
                    .block(Block::default().borders(Borders::ALL))
                    .wrap(Wrap { trim: true })
                    .alignment(Alignment::Left);
                frame.render_widget(paragraph, area);
            }

            ModalContent::RichText(lines) => {
                let paragraph = Paragraph::new(lines.clone())
                    .block(Block::default().borders(Borders::ALL))
                    .wrap(Wrap { trim: true })
                    .alignment(Alignment::Left);
                frame.render_widget(paragraph, area);
            }

            ModalContent::List(items) => {
                let list_items: Vec<ListItem> = items
                    .iter()
                    .enumerate()
                    .map(|(i, item)| {
                        let style = if i == modal.selected_item {
                            Style::default().bg(theme.colors.palette.selection)
                        } else {
                            Style::default()
                        };
                        ListItem::new(item.clone()).style(style)
                    })
                    .collect();

                let list = List::new(list_items)
                    .block(Block::default().borders(Borders::ALL))
                    .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                    .highlight_symbol("► ");

                frame.render_stateful_widget(list, area, &mut modal.list_state);
            }

            ModalContent::Form(fields) => {
                self.render_form_content(frame, area, modal, fields, theme);
            }

            ModalContent::Progress { current, total, message } => {
                let progress_area = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3), // Message
                        Constraint::Length(3), // Progress bar
                        Constraint::Min(0),    // Remaining space
                    ])
                    .split(area);

                // Message
                let message_paragraph = Paragraph::new(message.clone())
                    .block(Block::default().borders(Borders::ALL))
                    .alignment(Alignment::Center);
                frame.render_widget(message_paragraph, progress_area[0]);

                // Progress bar
                let percentage = if *total > 0 {
                    (*current * 100 / *total) as u16
                } else {
                    0
                };

                let progress_bar = Gauge::default()
                    .block(Block::default().borders(Borders::ALL))
                    .gauge_style(Style::default().fg(theme.colors.palette.accent))
                    .percent(percentage)
                    .label(format!("{}/{} ({}%)", current, total, percentage));

                frame.render_widget(progress_bar, progress_area[1]);
            }

            ModalContent::Custom(_) => {
                // Custom content would be handled by the caller
                let placeholder = Paragraph::new("Custom content placeholder")
                    .block(Block::default().borders(Borders::ALL))
                    .alignment(Alignment::Center);
                frame.render_widget(placeholder, area);
            }
        }
    }

    /// Render form content for input modals
    fn render_form_content(&self, frame: &mut Frame, area: Rect, modal: &mut Modal, fields: &[FormField], theme: &Theme) {
        if fields.is_empty() {
            return;
        }

        let field_height = 3;
        let total_height = fields.len() as u16 * field_height;
        
        if total_height <= area.height {
            // All fields fit, render normally
            let constraints: Vec<Constraint> = (0..fields.len())
                .map(|_| Constraint::Length(field_height))
                .collect();

            let field_areas = Layout::default()
                .direction(Direction::Vertical)
                .constraints(constraints)
                .split(area);

            for (i, field) in fields.iter().enumerate() {
                self.render_form_field(frame, field_areas[i], modal, field, i, theme);
            }
        } else {
            // Need scrolling - simplified implementation
            let visible_fields = (area.height / field_height) as usize;
            let start_index = modal.selected_item.saturating_sub(visible_fields / 2);
            let end_index = (start_index + visible_fields).min(fields.len());

            let constraints: Vec<Constraint> = (0..(end_index - start_index))
                .map(|_| Constraint::Length(field_height))
                .collect();

            let field_areas = Layout::default()
                .direction(Direction::Vertical)
                .constraints(constraints)
                .split(area);

            for (i, field) in fields.iter().enumerate().skip(start_index).take(end_index - start_index) {
                let area_index = i - start_index;
                self.render_form_field(frame, field_areas[area_index], modal, field, i, theme);
            }
        }
    }

    /// Render a single form field
    fn render_form_field(&self, frame: &mut Frame, area: Rect, modal: &mut Modal, field: &FormField, field_index: usize, theme: &Theme) {
        let is_focused = modal.focused_field.as_ref().map_or(false, |f| f == &field.name);
        let value = modal.input_values.get(&field.name).cloned().unwrap_or_default();

        let border_style = if is_focused {
            Style::default().fg(theme.colors.palette.accent)
        } else {
            Style::default().fg(theme.colors.palette.border)
        };

        let content = if is_focused {
            format!("{}█", value) // Add cursor
        } else {
            value
        };

        let paragraph = Paragraph::new(content)
            .block(Block::default()
                .title(field.label.clone())
                .borders(Borders::ALL)
                .border_style(border_style))
            .style(Style::default().fg(theme.colors.palette.text_primary));

        frame.render_widget(paragraph, area);
    }

    /// Render modal buttons
    fn render_modal_buttons(&self, frame: &mut Frame, area: Rect, modal: &Modal, theme: &Theme) {
        if modal.buttons.is_empty() {
            return;
        }

        let button_width = area.width / modal.buttons.len() as u16;
        let constraints: Vec<Constraint> = modal.buttons
            .iter()
            .map(|_| Constraint::Length(button_width))
            .collect();

        let button_areas = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints)
            .split(area.inner(&Margin { horizontal: 1, vertical: 1 }));

        for (i, button) in modal.buttons.iter().enumerate() {
            let is_selected = i == modal.selected_button;
            let button_style = self.get_button_style(button, is_selected, theme);
            
            let button_text = if let Some(shortcut) = button.shortcut {
                format!("[{}] {}", shortcut.to_uppercase(), button.label)
            } else {
                button.label.clone()
            };

            let button_paragraph = Paragraph::new(button_text)
                .block(Block::default().borders(Borders::ALL).border_style(button_style))
                .style(button_style)
                .alignment(Alignment::Center);

            frame.render_widget(button_paragraph, button_areas[i]);
        }
    }

    /// Get button styling based on type and state
    fn get_button_style(&self, button: &ModalButton, is_selected: bool, theme: &Theme) -> Style {
        let base_color = match button.style {
            ButtonStyle::Primary => theme.colors.palette.accent,
            ButtonStyle::Secondary => theme.colors.palette.text_muted,
            ButtonStyle::Danger => theme.colors.palette.error,
            ButtonStyle::Success => theme.colors.palette.success,
        };

        let mut style = Style::default().fg(base_color);
        
        if is_selected {
            style = style.add_modifier(Modifier::BOLD).bg(theme.colors.palette.selection);
        }
        
        if button.is_default {
            style = style.add_modifier(Modifier::UNDERLINED);
        }

        style
    }
}

impl Modal {
    /// Create a new modal
    pub fn new(
        id: String,
        modal_type: ModalType,
        size: ModalSize,
        title: String,
        content: ModalContent,
    ) -> Self {
        let mut list_state = ListState::default();
        if matches!(content, ModalContent::List(_)) {
            list_state.select(Some(0));
        }

        Self {
            id,
            modal_type,
            size,
            title,
            content,
            buttons: Vec::new(),
            is_visible: true,
            is_closable: true,
            auto_close_delay: None,
            created_at: std::time::Instant::now(),
            selected_button: 0,
            selected_item: 0,
            list_state,
            form_validation: None,
            input_values: HashMap::new(),
            focused_field: None,
            custom_data: HashMap::new(),
        }
    }

    /// Add buttons to the modal
    pub fn with_buttons(mut self, buttons: Vec<ModalButton>) -> Self {
        self.buttons = buttons;
        if !self.buttons.is_empty() {
            // Set default button as selected
            if let Some(default_index) = self.buttons.iter().position(|b| b.is_default) {
                self.selected_button = default_index;
            }
        }
        self
    }

    /// Set whether modal is closable with Escape
    pub fn closable(mut self, closable: bool) -> Self {
        self.is_closable = closable;
        self
    }

    /// Set auto-close delay
    pub fn auto_close_after(mut self, delay: std::time::Duration) -> Self {
        self.auto_close_delay = Some(delay);
        self
    }

    /// Navigate to next button
    pub fn next_button(&mut self) {
        if !self.buttons.is_empty() {
            self.selected_button = (self.selected_button + 1) % self.buttons.len();
        }
    }

    /// Navigate to previous button
    pub fn previous_button(&mut self) {
        if !self.buttons.is_empty() {
            self.selected_button = (self.selected_button + self.buttons.len() - 1) % self.buttons.len();
        }
    }

    /// Get selected button action
    pub fn get_selected_button_action(&self) -> Option<&str> {
        self.buttons.get(self.selected_button).map(|b| b.action.as_str())
    }
}

impl Default for ModalSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modal_system_creation() {
        let modal_system = ModalSystem::new();
        assert!(!modal_system.has_visible_modals());
    }

    #[test]
    fn test_confirmation_dialog() {
        let mut modal_system = ModalSystem::new();
        modal_system.show_confirmation(
            "test".to_string(),
            "Test".to_string(),
            "Are you sure?".to_string(),
            None,
            None,
        );
        assert!(modal_system.has_visible_modals());
    }

    #[test]
    fn test_modal_button_navigation() {
        let mut modal = Modal::new(
            "test".to_string(),
            ModalType::Confirmation,
            ModalSize::Medium,
            "Test".to_string(),
            ModalContent::Text("Test".to_string()),
        ).with_buttons(vec![
            ModalButton {
                label: "Cancel".to_string(),
                action: "cancel".to_string(),
                style: ButtonStyle::Secondary,
                is_default: false,
                shortcut: None,
            },
            ModalButton {
                label: "OK".to_string(),
                action: "ok".to_string(),
                style: ButtonStyle::Primary,
                is_default: true,
                shortcut: None,
            },
        ]);

        assert_eq!(modal.selected_button, 1); // Default button selected
        modal.next_button();
        assert_eq!(modal.selected_button, 0); // Wraps around
    }
}