//! Compatibility layer for the old FormValidationSystem API
//!
//! This module provides backward compatibility while the codebase migrates
//! to the new separated components.

use super::*;
use crate::theme::Theme;
use ratatui::{layout::Rect, Frame};
use regex::Regex;
use std::collections::HashMap;
use std::time::Duration;

/// Legacy FormValidationSystem API - provides backward compatibility
///
/// This is a drop-in replacement for the old monolithic FormValidationSystem
/// that delegates to the new separated components internally.
pub struct LegacyFormValidationSystem {
    unified: UnifiedFormValidationSystem,
    // Keep the old email regex for API compatibility
    email_regex: Regex,
}

impl LegacyFormValidationSystem {
    /// Create a new form validation system (legacy API)
    pub fn new() -> Self {
        let email_regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")
            .expect("Invalid email regex");

        Self {
            unified: UnifiedFormValidationSystem::new(),
            email_regex,
        }
    }

    /// Add a field to the form with validation rules (legacy API)
    pub fn add_field(&mut self, field_name: String, rules: Vec<ValidationRule>) -> &mut Self {
        self.unified.add_field(field_name, rules);
        self
    }

    /// Update field value (legacy API)
    pub fn update_field_value(&mut self, field_name: &str, value: String) {
        self.unified.update_field_value(field_name, value);
    }

    /// Validate a specific field (legacy API)
    pub fn validate_field(&mut self, field_name: &str) {
        if let Some(field) = self.unified.state().get_field(field_name) {
            let result = self.unified.validator().validate_field(field);
            self.unified
                .state_mut()
                .set_field_validation_result(field_name, result);
        }
    }

    /// Validate all fields (legacy API)
    pub fn validate_all(&mut self) {
        self.unified.validate_all();
    }

    /// Check if form is valid (legacy API)
    pub fn is_valid(&self) -> bool {
        self.unified.is_valid()
    }

    /// Update field immediately (legacy API compatibility)
    pub fn validate_field_immediate(&mut self, field_name: &str) {
        self.validate_field(field_name);
    }

    /// Set field focus (legacy API)
    pub fn set_field_focus(&mut self, field_name: &str, focused: bool) {
        self.unified
            .state_mut()
            .set_field_focus(field_name, focused);
    }

    /// Check if field exists (legacy API)
    pub fn has_field(&self, field_name: &str) -> bool {
        self.unified.state().get_field(field_name).is_some()
    }

    /// Get field value (legacy API)
    pub fn get_field_value(&self, field_name: &str) -> Option<String> {
        self.unified
            .state()
            .get_field(field_name)
            .map(|f| f.value.clone())
    }

    /// Get field validation result (legacy API)
    pub fn get_field_validation(&self, field_name: &str) -> Option<&ValidationResult> {
        self.unified
            .state()
            .get_field(field_name)
            .map(|f| &f.validation_result)
    }

    /// Clear field validation (legacy API)
    pub fn clear_field_validation(&mut self, field_name: &str) {
        self.unified
            .state_mut()
            .set_field_validation_result(field_name, ValidationResult::Valid);
    }

    /// Clear all validation (legacy API)
    pub fn clear_all_validation(&mut self) {
        self.unified.state_mut().clear_validation();
    }

    /// Reset form (legacy API)
    pub fn reset(&mut self) {
        self.unified.state_mut().reset();
    }

    /// Get form data (legacy API)
    pub fn get_form_data(&self) -> HashMap<String, String> {
        self.unified.state().get_form_data()
    }

    /// Set form data (legacy API)
    pub fn set_form_data(&mut self, data: HashMap<String, String>) {
        self.unified.state_mut().set_form_data(data);
    }

    /// Render validated field (legacy API)
    pub fn render_validated_field(
        &self,
        frame: &mut Frame,
        area: Rect,
        field_name: &str,
        title: &str,
        theme: &Theme,
    ) {
        self.unified
            .render_validated_field(frame, area, field_name, title, theme);
    }

    /// Render validation summary (legacy API)
    pub fn render_validation_summary(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.unified.render_validation_summary(frame, area, theme);
    }

    /// Access to email regex for compatibility
    pub fn email_regex(&self) -> &Regex {
        &self.email_regex
    }

    /// Check validation delay for compatibility
    pub fn validation_delay(&self) -> Duration {
        Duration::from_millis(300) // Default debounce delay
    }

    /// Legacy show_validation_summary flag
    pub fn show_validation_summary(&self) -> bool {
        self.unified.state().show_validation_summary()
    }

    /// Set show validation summary flag
    pub fn set_show_validation_summary(&mut self, show: bool) {
        self.unified.state_mut().set_show_validation_summary(show);
    }

    /// Get fields with errors (legacy API)
    pub fn fields_with_errors(&self) -> Vec<&String> {
        self.unified.state().fields_with_errors()
    }

    /// Check if field has error (legacy API)
    pub fn field_has_error(&self, field_name: &str) -> bool {
        self.unified
            .state()
            .get_field(field_name)
            .map(|f| f.has_error())
            .unwrap_or(false)
    }

    /// Check if field is dirty (legacy API)
    pub fn field_is_dirty(&self, field_name: &str) -> bool {
        self.unified
            .state()
            .get_field(field_name)
            .map(|f| f.is_dirty)
            .unwrap_or(false)
    }

    /// Check if field is touched (legacy API)
    pub fn field_is_touched(&self, field_name: &str) -> bool {
        self.unified
            .state()
            .get_field(field_name)
            .map(|f| f.is_touched)
            .unwrap_or(false)
    }

    /// Get access to the new unified system (for migration)
    pub fn unified(&self) -> &UnifiedFormValidationSystem {
        &self.unified
    }

    /// Get mutable access to the new unified system (for migration)
    pub fn unified_mut(&mut self) -> &mut UnifiedFormValidationSystem {
        &mut self.unified
    }
}

impl Default for LegacyFormValidationSystem {
    fn default() -> Self {
        Self::new()
    }
}

// Re-export the legacy system as the default FormValidationSystem for compatibility
pub use LegacyFormValidationSystem as FormValidationSystemCompat;
