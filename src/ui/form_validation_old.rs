//! Real-time form validation system with comprehensive feedback
//!
//! This module provides a robust form validation system that offers immediate
//! feedback to users as they type, with visual indicators, helpful suggestions,
//! and accessibility features for better user experience.

use crate::theme::Theme;
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, List, ListItem},
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    Frame,
};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use regex::Regex;

/// Validation result for a single field
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationResult {
    Valid,
    Warning(ValidationMessage),
    Error(ValidationMessage),
    Pending, // For async validations
}

/// Validation message with context and suggestions
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationMessage {
    pub message: String,
    pub suggestions: Vec<String>,
    pub error_type: ValidationErrorType,
}

/// Types of validation errors for better categorization
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationErrorType {
    Required,
    Format,
    Length,
    Pattern,
    Custom(String),
    Network, // For async validations like email existence
}

/// Field validation rule
#[derive(Debug, Clone)]
pub struct ValidationRule {
    pub name: String,
    pub validator: ValidationFunction,
    pub trigger: ValidationTrigger,
    pub debounce_ms: u64,
}

/// When to trigger validation
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationTrigger {
    OnChange,     // As user types
    OnBlur,       // When field loses focus
    OnSubmit,     // Only when form is submitted
    Manual,       // Manually triggered
}

/// Validation function type
#[derive(Debug, Clone)]
pub enum ValidationFunction {
    Required,
    Email,
    Length { min: usize, max: Option<usize> },
    Pattern { regex: String, message: String },
    Custom { name: String, description: String },
}

/// Form field with validation state
#[derive(Debug, Clone)]
pub struct ValidatedField {
    pub value: String,
    pub validation_result: ValidationResult,
    pub rules: Vec<ValidationRule>,
    pub is_focused: bool,
    pub last_validation: Option<Instant>,
    pub debounce_timer: Option<Instant>,
    pub is_dirty: bool,
    pub show_suggestions: bool,
}

/// Form validation manager
pub struct FormValidationSystem {
    fields: HashMap<String, ValidatedField>,
    form_is_valid: bool,
    show_validation_summary: bool,
    validation_delay: Duration,
    email_regex: Regex,
}

impl FormValidationSystem {
    /// Create a new form validation system
    pub fn new() -> Self {
        let email_regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")
            .expect("Invalid email regex");

        Self {
            fields: HashMap::new(),
            form_is_valid: false,
            show_validation_summary: true,
            validation_delay: Duration::from_millis(300), // Debounce delay
            email_regex,
        }
    }

    /// Add a field to the form with validation rules
    pub fn add_field(&mut self, field_name: String, rules: Vec<ValidationRule>) -> &mut Self {
        self.fields.insert(field_name, ValidatedField {
            value: String::new(),
            validation_result: ValidationResult::Valid,
            rules,
            is_focused: false,
            last_validation: None,
            debounce_timer: None,
            is_dirty: false,
            show_suggestions: false,
        });
        self
    }

    /// Update field value and trigger validation if appropriate
    pub fn update_field_value(&mut self, field_name: &str, value: String) {
        if let Some(field) = self.fields.get_mut(field_name) {
            field.value = value;
            field.is_dirty = true;
            
            // Set debounce timer for on-change validations
            let now = Instant::now();
            field.debounce_timer = Some(now);
            
            // Immediate validation for certain rules (like required)
            self.validate_field_immediate(field_name);
        }
    }

    /// Set field focus state
    pub fn set_field_focus(&mut self, field_name: &str, is_focused: bool) {
        if let Some(field) = self.fields.get_mut(field_name) {
            let was_focused = field.is_focused;
            field.is_focused = is_focused;
            
            // Trigger on-blur validation if field lost focus and is dirty
            if was_focused && !is_focused && field.is_dirty {
                self.validate_field(field_name);
            }
        }
    }

    /// Update validation system (call in main loop)
    pub fn update(&mut self) {
        let now = Instant::now();
        let field_names: Vec<String> = self.fields.keys().cloned().collect();
        
        for field_name in field_names {
            if let Some(field) = self.fields.get(&field_name) {
                // Check if debounce timer has expired
                if let Some(debounce_time) = field.debounce_timer {
                    if now.duration_since(debounce_time) >= self.validation_delay {
                        // Remove debounce timer and validate
                        self.fields.get_mut(&field_name).unwrap().debounce_timer = None;
                        self.validate_field(&field_name);
                    }
                }
            }
        }
        
        self.update_form_validity();
    }

    /// Validate a specific field immediately (no debounce)
    fn validate_field_immediate(&mut self, field_name: &str) {
        if let Some(field) = self.fields.get(field_name) {
            // Only validate rules that should trigger immediately
            for rule in &field.rules {
                if matches!(rule.validator, ValidationFunction::Required) 
                    && rule.trigger == ValidationTrigger::OnChange {
                    self.validate_field(field_name);
                    break;
                }
            }
        }
    }

    /// Validate a specific field
    pub fn validate_field(&mut self, field_name: &str) {
        let (value, rules) = if let Some(field) = self.fields.get(field_name) {
            (field.value.clone(), field.rules.clone())
        } else {
            return;
        };
        
        let mut result = ValidationResult::Valid;
        
        // Apply validation rules in order
        for rule in &rules {
            match self.apply_validation_rule(&value, rule) {
                ValidationResult::Error(_) => {
                    result = self.apply_validation_rule(&value, rule);
                    break; // Stop at first error
                }
                ValidationResult::Warning(msg) if result == ValidationResult::Valid => {
                    result = ValidationResult::Warning(msg);
                    // Continue to check for errors
                }
                _ => {} // Continue validation
            }
        }
        
        if let Some(field) = self.fields.get_mut(field_name) {
            field.validation_result = result;
            field.last_validation = Some(Instant::now());
        }
    }

    /// Apply a single validation rule
    fn apply_validation_rule(&self, value: &str, rule: &ValidationRule) -> ValidationResult {
        match &rule.validator {
            ValidationFunction::Required => {
                if value.trim().is_empty() {
                    ValidationResult::Error(ValidationMessage {
                        message: "This field is required".to_string(),
                        suggestions: vec!["Please enter a value".to_string()],
                        error_type: ValidationErrorType::Required,
                    })
                } else {
                    ValidationResult::Valid
                }
            }
            
            ValidationFunction::Email => {
                if value.trim().is_empty() {
                    ValidationResult::Valid // Let required rule handle empty values
                } else if !self.email_regex.is_match(value.trim()) {
                    ValidationResult::Error(ValidationMessage {
                        message: "Invalid email address".to_string(),
                        suggestions: vec![
                            "Check for typos in the email address".to_string(),
                            "Make sure it includes @ and a domain".to_string(),
                            "Example: user@example.com".to_string(),
                        ],
                        error_type: ValidationErrorType::Format,
                    })
                } else if self.check_email_warnings(value.trim()) {
                    ValidationResult::Warning(ValidationMessage {
                        message: "Email domain might be misspelled".to_string(),
                        suggestions: vec![
                            "Did you mean gmail.com?".to_string(),
                            "Did you mean outlook.com?".to_string(),
                        ],
                        error_type: ValidationErrorType::Format,
                    })
                } else {
                    ValidationResult::Valid
                }
            }
            
            ValidationFunction::Length { min, max } => {
                let len = value.chars().count();
                
                if len < *min {
                    ValidationResult::Error(ValidationMessage {
                        message: format!("Must be at least {} characters", min),
                        suggestions: vec![
                            format!("Add {} more characters", min - len),
                        ],
                        error_type: ValidationErrorType::Length,
                    })
                } else if let Some(max_len) = max {
                    if len > *max_len {
                        ValidationResult::Error(ValidationMessage {
                            message: format!("Must be no more than {} characters", max_len),
                            suggestions: vec![
                                format!("Remove {} characters", len - max_len),
                            ],
                            error_type: ValidationErrorType::Length,
                        })
                    } else {
                        ValidationResult::Valid
                    }
                } else {
                    ValidationResult::Valid
                }
            }
            
            ValidationFunction::Pattern { regex, message } => {
                if value.trim().is_empty() {
                    ValidationResult::Valid // Let other rules handle empty values
                } else {
                    match Regex::new(regex) {
                        Ok(pattern) => {
                            if pattern.is_match(value.trim()) {
                                ValidationResult::Valid
                            } else {
                                ValidationResult::Error(ValidationMessage {
                                    message: message.clone(),
                                    suggestions: vec!["Please check the format".to_string()],
                                    error_type: ValidationErrorType::Pattern,
                                })
                            }
                        }
                        Err(_) => ValidationResult::Valid, // Invalid regex, skip validation
                    }
                }
            }
            
            ValidationFunction::Custom { name: _, description } => {
                // Custom validation would be implemented by the calling code
                // For now, return a placeholder
                ValidationResult::Warning(ValidationMessage {
                    message: format!("Custom validation: {}", description),
                    suggestions: vec!["Custom validation needs implementation".to_string()],
                    error_type: ValidationErrorType::Custom("placeholder".to_string()),
                })
            }
        }
    }

    /// Check for common email domain typos
    fn check_email_warnings(&self, email: &str) -> bool {
        let common_domains = ["gmail.com", "outlook.com", "yahoo.com", "hotmail.com"];
        let domain = email.split('@').nth(1).unwrap_or("");
        
        // Simple typo detection (this could be more sophisticated)
        for correct_domain in &common_domains {
            if domain != *correct_domain && self.is_similar_domain(domain, correct_domain) {
                return true;
            }
        }
        
        false
    }

    /// Simple domain similarity check
    fn is_similar_domain(&self, domain: &str, correct: &str) -> bool {
        // Very basic similarity check - could use Levenshtein distance
        if domain.len() == correct.len() {
            let mut diff_count = 0;
            for (a, b) in domain.chars().zip(correct.chars()) {
                if a != b {
                    diff_count += 1;
                    if diff_count > 2 {
                        return false;
                    }
                }
            }
            diff_count <= 2
        } else {
            false
        }
    }

    /// Validate entire form
    pub fn validate_form(&mut self) {
        let field_names: Vec<String> = self.fields.keys().cloned().collect();
        for field_name in field_names {
            self.validate_field(&field_name);
        }
        self.update_form_validity();
    }

    /// Update overall form validity
    fn update_form_validity(&mut self) {
        self.form_is_valid = self.fields.values().all(|field| {
            matches!(field.validation_result, ValidationResult::Valid | ValidationResult::Warning(_))
        });
    }

    /// Check if form is valid
    pub fn is_valid(&self) -> bool {
        self.form_is_valid
    }

    /// Get field validation state
    pub fn get_field_validation(&self, field_name: &str) -> Option<&ValidationResult> {
        self.fields.get(field_name).map(|field| &field.validation_result)
    }

    /// Get all validation errors
    pub fn get_validation_errors(&self) -> HashMap<String, ValidationMessage> {
        let mut errors = HashMap::new();
        
        for (field_name, field) in &self.fields {
            if let ValidationResult::Error(msg) = &field.validation_result {
                errors.insert(field_name.clone(), msg.clone());
            }
        }
        
        errors
    }

    /// Render field with validation feedback
    pub fn render_validated_field(
        &self,
        frame: &mut Frame,
        area: Rect,
        field_name: &str,
        title: &str,
        theme: &Theme,
    ) {
        if let Some(field) = self.fields.get(field_name) {
            // Determine field style based on validation state
            let (border_style, help_text, help_color) = match &field.validation_result {
                ValidationResult::Valid => (
                    if field.is_focused {
                        Style::default().fg(theme.colors.palette.accent)
                    } else {
                        Style::default().fg(theme.colors.palette.border)
                    },
                    None,
                    theme.colors.palette.text_muted,
                ),
                ValidationResult::Warning(msg) => (
                    Style::default().fg(theme.colors.palette.warning),
                    Some(&msg.message),
                    theme.colors.palette.warning,
                ),
                ValidationResult::Error(msg) => (
                    Style::default().fg(theme.colors.palette.error),
                    Some(&msg.message),
                    theme.colors.palette.error,
                ),
                ValidationResult::Pending => (
                    Style::default().fg(theme.colors.palette.info),
                    None, // No help text for pending
                    theme.colors.palette.info,
                ),
            };

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
            let content = if field.is_focused {
                format!("{}█", field.value) // Add cursor indicator
            } else {
                field.value.clone()
            };

            let paragraph = Paragraph::new(content)
                .block(Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(border_style))
                .style(Style::default().fg(theme.colors.palette.text_primary));

            frame.render_widget(paragraph, layout[0]);

            // Render help text if present
            if let (Some(help), true) = (help_text, layout.len() > 1) {
                let help_paragraph = Paragraph::new(help.as_str())
                    .style(Style::default().fg(help_color).add_modifier(Modifier::ITALIC))
                    .alignment(Alignment::Left);

                frame.render_widget(help_paragraph, layout[1]);
            }

            // Render suggestions if field is focused and has suggestions
            if field.is_focused && field.show_suggestions {
                if let ValidationResult::Error(msg) | ValidationResult::Warning(msg) = &field.validation_result {
                    if !msg.suggestions.is_empty() && area.height > 6 {
                        self.render_suggestions(frame, area, &msg.suggestions, theme);
                    }
                }
            }
        }
    }

    /// Render validation suggestions popup
    fn render_suggestions(&self, frame: &mut Frame, area: Rect, suggestions: &[String], theme: &Theme) {
        let popup_height = (suggestions.len() + 2).min(area.height as usize / 2);
        let popup_area = Rect {
            x: area.x,
            y: area.y + area.height - popup_height as u16,
            width: area.width,
            height: popup_height as u16,
        };

        // Clear the area
        frame.render_widget(ratatui::widgets::Clear, popup_area);

        let suggestions_items: Vec<ListItem> = suggestions
            .iter()
            .map(|suggestion| {
                ListItem::new(Line::from(vec![
                    Span::styled("💡 ", Style::default().fg(theme.colors.palette.info)),
                    Span::styled(suggestion.clone(), Style::default().fg(theme.colors.palette.text_primary)),
                ]))
            })
            .collect();

        let suggestions_list = List::new(suggestions_items)
            .block(Block::default()
                .title("Suggestions")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.colors.palette.info)));

        frame.render_widget(suggestions_list, popup_area);
    }

    /// Render form validation summary
    pub fn render_validation_summary(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.show_validation_summary {
            return;
        }

        let errors = self.get_validation_errors();
        if errors.is_empty() && self.form_is_valid {
            // Show success state
            let success_text = Line::from(vec![
                Span::styled("✅ ", Style::default().fg(theme.colors.palette.success)),
                Span::styled("All fields are valid", Style::default().fg(theme.colors.palette.success)),
            ]);

            let paragraph = Paragraph::new(success_text)
                .block(Block::default()
                    .title("Validation Status")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.colors.palette.success)))
                .alignment(Alignment::Center);

            frame.render_widget(paragraph, area);
        } else if !errors.is_empty() {
            // Show error summary
            let mut lines = Vec::new();
            lines.push(Line::from(vec![
                Span::styled("❌ ", Style::default().fg(theme.colors.palette.error)),
                Span::styled(
                    format!("{} field(s) need attention:", errors.len()),
                    Style::default().fg(theme.colors.palette.error).add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::from(""));

            for (field, error) in errors {
                lines.push(Line::from(vec![
                    Span::styled("• ", Style::default().fg(theme.colors.palette.error)),
                    Span::styled(format!("{}: ", field), Style::default().fg(theme.colors.palette.text_primary).add_modifier(Modifier::BOLD)),
                    Span::styled(error.message, Style::default().fg(theme.colors.palette.text_secondary)),
                ]));
            }

            let paragraph = Paragraph::new(lines)
                .block(Block::default()
                    .title("Validation Errors")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.colors.palette.error)))
                .wrap(ratatui::widgets::Wrap { trim: true });

            frame.render_widget(paragraph, area);
        }
    }

    /// Toggle field suggestions
    pub fn toggle_field_suggestions(&mut self, field_name: &str) {
        if let Some(field) = self.fields.get_mut(field_name) {
            field.show_suggestions = !field.show_suggestions;
        }
    }
}

impl Default for FormValidationSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidatedField {
    pub fn new(rules: Vec<ValidationRule>) -> Self {
        Self {
            value: String::new(),
            validation_result: ValidationResult::Valid,
            rules,
            is_focused: false,
            last_validation: None,
            debounce_timer: None,
            is_dirty: false,
            show_suggestions: false,
        }
    }
}

impl ValidationMessage {
    pub fn new(message: String, error_type: ValidationErrorType) -> Self {
        Self {
            message,
            suggestions: Vec::new(),
            error_type,
        }
    }

    pub fn with_suggestions(mut self, suggestions: Vec<String>) -> Self {
        self.suggestions = suggestions;
        self
    }
}

/// Helper function to create common validation rules
pub mod rules {
    use super::*;

    pub fn required() -> ValidationRule {
        ValidationRule {
            name: "required".to_string(),
            validator: ValidationFunction::Required,
            trigger: ValidationTrigger::OnChange,
            debounce_ms: 100,
        }
    }

    pub fn email() -> ValidationRule {
        ValidationRule {
            name: "email".to_string(),
            validator: ValidationFunction::Email,
            trigger: ValidationTrigger::OnChange,
            debounce_ms: 300,
        }
    }

    pub fn length(min: usize, max: Option<usize>) -> ValidationRule {
        ValidationRule {
            name: "length".to_string(),
            validator: ValidationFunction::Length { min, max },
            trigger: ValidationTrigger::OnChange,
            debounce_ms: 200,
        }
    }

    pub fn pattern(regex: String, message: String) -> ValidationRule {
        ValidationRule {
            name: "pattern".to_string(),
            validator: ValidationFunction::Pattern { regex, message },
            trigger: ValidationTrigger::OnBlur,
            debounce_ms: 100,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_form_validation_system() {
        let mut form = FormValidationSystem::new();
        form.add_field("email".to_string(), vec![rules::email()]);
        
        form.update_field_value("email", "invalid-email".to_string());
        form.validate_field("email");
        
        assert!(!form.is_valid());
        assert!(matches!(form.get_field_validation("email"), Some(ValidationResult::Error(_))));
    }

    #[test]
    fn test_email_validation() {
        let form = FormValidationSystem::new();
        let rule = rules::email();
        
        let valid_result = form.apply_validation_rule("test@example.com", &rule);
        assert_eq!(valid_result, ValidationResult::Valid);
        
        let invalid_result = form.apply_validation_rule("invalid-email", &rule);
        assert!(matches!(invalid_result, ValidationResult::Error(_)));
    }

    #[test]
    fn test_length_validation() {
        let form = FormValidationSystem::new();
        let rule = rules::length(3, Some(10));
        
        let valid_result = form.apply_validation_rule("hello", &rule);
        assert_eq!(valid_result, ValidationResult::Valid);
        
        let too_short_result = form.apply_validation_rule("hi", &rule);
        assert!(matches!(too_short_result, ValidationResult::Error(_)));
    }
}