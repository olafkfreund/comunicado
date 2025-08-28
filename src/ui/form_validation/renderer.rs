//! Form rendering logic
//!
//! This module handles the rendering of form fields and validation feedback,
//! separated from validation logic and state management.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Modifier, Style},
    widgets::{Block, Borders, Paragraph, List, ListItem},
    Frame,
};
use crate::theme::Theme;
use crate::ui::form_validation::types::*;

/// Form renderer handles UI presentation of form fields and validation feedback
pub struct FormRenderer {
    /// Whether to show field suggestions
    show_suggestions: bool,
    /// Whether to show field labels
    show_labels: bool,
    /// Whether to show validation icons
    show_icons: bool,
}

impl FormRenderer {
    /// Create a new form renderer
    pub fn new() -> Self {
        Self {
            show_suggestions: true,
            show_labels: true,
            show_icons: true,
        }
    }
    
    /// Create renderer with custom settings
    pub fn with_settings(show_suggestions: bool, show_labels: bool, show_icons: bool) -> Self {
        Self {
            show_suggestions,
            show_labels,
            show_icons,
        }
    }
    
    /// Render a validated field
    pub fn render_field(
        &self,
        frame: &mut Frame,
        area: Rect,
        field: &ValidatedField,
        title: &str,
        theme: &Theme,
    ) {
        // Determine styling based on validation state
        let (border_style, help_text, help_color) = self.get_field_styling(field, theme);
        
        // Calculate layout
        let layout = if help_text.is_some() && area.height > 3 {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(3), // Input field
                    Constraint::Length(2), // Help text
                ])
                .split(area)
        } else {
            std::rc::Rc::from([area].as_slice())
        };

        // Render input field
        let content = self.get_field_content(field);
        let field_title = self.get_field_title(title, field);
        
        let paragraph = Paragraph::new(content)
            .style(Style::default().fg(theme.colors.palette.text_primary))
            .block(
                Block::default()
                    .title(field_title)
                    .borders(Borders::ALL)
                    .border_style(border_style)
            )
            .alignment(Alignment::Left);

        frame.render_widget(paragraph, layout[0]);

        // Render help text if present and space available
        if let (Some(help), true) = (help_text, layout.len() > 1) {
            let help_paragraph = Paragraph::new(help.as_str())
                .style(Style::default().fg(help_color).add_modifier(Modifier::ITALIC))
                .alignment(Alignment::Left);

            frame.render_widget(help_paragraph, layout[1]);
        }
    }
    
    /// Render validation summary
    pub fn render_summary(
        &self,
        frame: &mut Frame,
        area: Rect,
        errors: &[(&String, &ValidationMessage)],
        theme: &Theme,
    ) {
        if errors.is_empty() {
            return;
        }

        let items: Vec<ListItem> = errors
            .iter()
            .map(|(field_name, message)| {
                let icon = if self.show_icons { "❌ " } else { "• " };
                let content = format!("{}{}: {}", icon, field_name, message.message);
                ListItem::new(content)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title(" Validation Errors ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.colors.palette.error))
            )
            .style(Style::default().fg(theme.colors.palette.error));

        frame.render_widget(list, area);
    }
    
    /// Render form progress indicator
    pub fn render_progress(
        &self,
        frame: &mut Frame,
        area: Rect,
        completion_percentage: f32,
        theme: &Theme,
    ) {
        let progress_text = format!("Form Progress: {:.0}%", completion_percentage * 100.0);
        
        let paragraph = Paragraph::new(progress_text)
            .style(Style::default().fg(theme.colors.palette.text_muted))
            .alignment(Alignment::Right);
            
        frame.render_widget(paragraph, area);
    }
    
    /// Render field validation status indicator
    pub fn render_field_status(
        &self,
        frame: &mut Frame,
        area: Rect,
        field: &ValidatedField,
        theme: &Theme,
    ) {
        if !self.show_icons {
            return;
        }
        
        let (icon, color) = match &field.validation_result {
            ValidationResult::Valid => ("✓", theme.colors.palette.accent),
            ValidationResult::Error(_) => ("❌", theme.colors.palette.error),
            ValidationResult::Warning(_) => ("⚠", theme.colors.palette.warning),
            ValidationResult::Pending => ("⏳", theme.colors.palette.info),
        };
        
        let paragraph = Paragraph::new(icon)
            .style(Style::default().fg(color))
            .alignment(Alignment::Center);
            
        frame.render_widget(paragraph, area);
    }
    
    // Private helper methods
    
    fn get_field_styling(&self, field: &ValidatedField, theme: &Theme) -> (Style, Option<String>, ratatui::style::Color) {
        match &field.validation_result {
            ValidationResult::Valid => (
                if field.is_focused {
                    Style::default().fg(theme.colors.palette.accent)
                } else {
                    Style::default().fg(theme.colors.palette.text_muted)
                },
                None,
                theme.colors.palette.text_muted,
            ),
            ValidationResult::Warning(msg) => (
                Style::default().fg(theme.colors.palette.warning),
                if self.show_suggestions {
                    Some(self.format_help_text(&msg.message, &msg.suggestion))
                } else {
                    None
                },
                theme.colors.palette.warning,
            ),
            ValidationResult::Error(msg) => (
                Style::default().fg(theme.colors.palette.error),
                if self.show_suggestions {
                    Some(self.format_help_text(&msg.message, &msg.suggestion))
                } else {
                    None
                },
                theme.colors.palette.error,
            ),
            ValidationResult::Pending => (
                Style::default().fg(theme.colors.palette.info),
                None,
                theme.colors.palette.info,
            ),
        }
    }
    
    fn get_field_content(&self, field: &ValidatedField) -> String {
        if field.is_focused {
            format!("{}█", field.value) // Add cursor indicator
        } else {
            field.value.clone()
        }
    }
    
    fn get_field_title(&self, base_title: &str, field: &ValidatedField) -> String {
        if !self.show_labels {
            return base_title.to_string();
        }
        
        let mut title = format!(" {} ", base_title);
        
        if self.show_icons {
            let icon = match &field.validation_result {
                ValidationResult::Valid if !field.value.is_empty() => " ✓",
                ValidationResult::Error(_) => " ❌",
                ValidationResult::Warning(_) => " ⚠",
                ValidationResult::Pending => " ⏳",
                _ => "",
            };
            title.push_str(icon);
        }
        
        // Add required indicator
        let is_required = field.rules.iter()
            .any(|rule| matches!(rule.validator, ValidationFunction::Required));
            
        if is_required {
            title.push_str(" *");
        }
        
        title
    }
    
    fn format_help_text(&self, message: &str, suggestion: &Option<String>) -> String {
        if let Some(suggestion) = suggestion {
            format!("{} - {}", message, suggestion)
        } else {
            message.to_string()
        }
    }
    
    /// Set whether to show field suggestions
    pub fn set_show_suggestions(&mut self, show: bool) {
        self.show_suggestions = show;
    }
    
    /// Set whether to show field labels
    pub fn set_show_labels(&mut self, show: bool) {
        self.show_labels = show;
    }
    
    /// Set whether to show validation icons
    pub fn set_show_icons(&mut self, show: bool) {
        self.show_icons = show;
    }
    
    /// Get current renderer settings
    pub fn settings(&self) -> (bool, bool, bool) {
        (self.show_suggestions, self.show_labels, self.show_icons)
    }
}

impl Default for FormRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_renderer_settings() {
        let mut renderer = FormRenderer::new();
        assert_eq!(renderer.settings(), (true, true, true));
        
        renderer.set_show_suggestions(false);
        renderer.set_show_labels(false);
        renderer.set_show_icons(false);
        
        assert_eq!(renderer.settings(), (false, false, false));
    }
    
    #[test]
    fn test_field_title_formatting() {
        let renderer = FormRenderer::new();
        
        let mut field = ValidatedField::new(vec![
            ValidationRule::new("required", ValidationFunction::Required)
        ]);
        
        field.set_validation_result(ValidationResult::Valid);
        let title = renderer.get_field_title("Email", &field);
        
        // Should include required indicator
        assert!(title.contains('*'));
    }
    
    #[test]  
    fn test_help_text_formatting() {
        let renderer = FormRenderer::new();
        
        // With suggestion
        let help = renderer.format_help_text(
            "Invalid email", 
            &Some("Please enter a valid email address".to_string())
        );
        assert!(help.contains(" - "));
        
        // Without suggestion
        let help = renderer.format_help_text("Invalid email", &None);
        assert_eq!(help, "Invalid email");
    }
}