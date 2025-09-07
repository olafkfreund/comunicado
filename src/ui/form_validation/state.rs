//! Form state management
//!
//! This module handles form state, field data, and validation status tracking.

use crate::ui::form_validation::types::*;
use std::collections::HashMap;

/// Form state manager handles field data and form-level state
pub struct FormState {
    /// Field data keyed by field name
    fields: HashMap<String, ValidatedField>,
    /// Overall form validity
    form_is_valid: bool,
    /// Whether to show validation summary
    show_validation_summary: bool,
    /// Form submission state
    is_submitting: bool,
    /// Form submission attempts counter
    submit_attempts: u32,
}

impl FormState {
    /// Create a new form state
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
            form_is_valid: false,
            show_validation_summary: true,
            is_submitting: false,
            submit_attempts: 0,
        }
    }

    /// Add a field to the form with validation rules
    pub fn add_field(&mut self, field_name: String, rules: Vec<ValidationRule>) {
        self.fields.insert(field_name, ValidatedField::new(rules));
        self.update_form_validity();
    }

    /// Remove a field from the form
    pub fn remove_field(&mut self, field_name: &str) -> Option<ValidatedField> {
        let field = self.fields.remove(field_name);
        self.update_form_validity();
        field
    }

    /// Get a field by name
    pub fn get_field(&self, field_name: &str) -> Option<&ValidatedField> {
        self.fields.get(field_name)
    }

    /// Get a mutable field by name
    pub fn get_field_mut(&mut self, field_name: &str) -> Option<&mut ValidatedField> {
        self.fields.get_mut(field_name)
    }

    /// Update field value
    pub fn update_field_value(&mut self, field_name: &str, value: String) {
        if let Some(field) = self.fields.get_mut(field_name) {
            field.set_value(value);
        }
    }

    /// Set field focus state
    pub fn set_field_focus(&mut self, field_name: &str, focused: bool) {
        if let Some(field) = self.fields.get_mut(field_name) {
            field.set_focused(focused);
        }
    }

    /// Set field validation result
    pub fn set_field_validation_result(&mut self, field_name: &str, result: ValidationResult) {
        if let Some(field) = self.fields.get_mut(field_name) {
            field.set_validation_result(result);
        }
        self.update_form_validity();
    }

    /// Get all field names
    pub fn field_names(&self) -> impl Iterator<Item = String> + '_ {
        self.fields.keys().cloned()
    }

    /// Get all fields
    pub fn fields(&self) -> &HashMap<String, ValidatedField> {
        &self.fields
    }

    /// Get fields that need validation (due to debounce timers)
    pub fn fields_needing_validation(&self) -> Vec<(&String, &ValidatedField)> {
        self.fields
            .iter()
            .filter(|(_, field)| field.needs_validation())
            .collect()
    }

    /// Update overall form validity
    pub fn update_form_validity(&mut self) {
        self.form_is_valid = self.fields.values().all(|field| field.is_valid());
    }

    /// Check if the form is valid
    pub fn is_form_valid(&self) -> bool {
        self.form_is_valid
    }

    /// Get validation errors for summary display
    pub fn get_validation_errors(&self) -> Vec<(&String, &ValidationMessage)> {
        self.fields
            .iter()
            .filter_map(|(name, field)| match &field.validation_result {
                ValidationResult::Error(msg) => Some((name, msg)),
                _ => None,
            })
            .collect()
    }

    /// Get validation warnings
    pub fn get_validation_warnings(&self) -> Vec<(&String, &ValidationMessage)> {
        self.fields
            .iter()
            .filter_map(|(name, field)| match &field.validation_result {
                ValidationResult::Warning(msg) => Some((name, msg)),
                _ => None,
            })
            .collect()
    }

    /// Check if validation summary should be shown
    pub fn show_validation_summary(&self) -> bool {
        self.show_validation_summary && !self.form_is_valid
    }

    /// Set whether to show validation summary
    pub fn set_show_validation_summary(&mut self, show: bool) {
        self.show_validation_summary = show;
    }

    /// Mark form as submitting
    pub fn set_submitting(&mut self, submitting: bool) {
        self.is_submitting = submitting;
        if submitting {
            self.submit_attempts += 1;
        }
    }

    /// Check if form is currently being submitted
    pub fn is_submitting(&self) -> bool {
        self.is_submitting
    }

    /// Get number of submission attempts
    pub fn submit_attempts(&self) -> u32 {
        self.submit_attempts
    }

    /// Reset form state
    pub fn reset(&mut self) {
        for field in self.fields.values_mut() {
            field.set_value(String::new());
            field.set_validation_result(ValidationResult::Valid);
            field.is_dirty = false;
            field.is_touched = false;
            field.debounce_timer = None;
            field.last_validation = None;
        }
        self.form_is_valid = false;
        self.is_submitting = false;
        self.submit_attempts = 0;
    }

    /// Clear validation results
    pub fn clear_validation(&mut self) {
        for field in self.fields.values_mut() {
            field.set_validation_result(ValidationResult::Valid);
        }
        self.update_form_validity();
    }

    /// Mark all fields as touched (useful for showing validation after submit attempt)
    pub fn touch_all_fields(&mut self) {
        for field in self.fields.values_mut() {
            field.is_touched = true;
        }
    }

    /// Get form data as key-value pairs
    pub fn get_form_data(&self) -> HashMap<String, String> {
        self.fields
            .iter()
            .map(|(name, field)| (name.clone(), field.value.clone()))
            .collect()
    }

    /// Set form data from key-value pairs
    pub fn set_form_data(&mut self, data: HashMap<String, String>) {
        for (field_name, value) in data {
            self.update_field_value(&field_name, value);
        }
    }

    /// Get fields with errors
    pub fn fields_with_errors(&self) -> Vec<&String> {
        self.fields
            .iter()
            .filter(|(_, field)| field.has_error())
            .map(|(name, _)| name)
            .collect()
    }

    /// Get fields with warnings
    pub fn fields_with_warnings(&self) -> Vec<&String> {
        self.fields
            .iter()
            .filter(|(_, field)| field.has_warning())
            .map(|(name, _)| name)
            .collect()
    }

    /// Count valid fields
    pub fn valid_field_count(&self) -> usize {
        self.fields
            .values()
            .filter(|field| field.is_valid())
            .count()
    }

    /// Count fields with errors
    pub fn error_field_count(&self) -> usize {
        self.fields
            .values()
            .filter(|field| field.has_error())
            .count()
    }

    /// Count fields with warnings
    pub fn warning_field_count(&self) -> usize {
        self.fields
            .values()
            .filter(|field| field.has_warning())
            .count()
    }

    /// Get form completion percentage (0.0 to 1.0)
    pub fn completion_percentage(&self) -> f32 {
        if self.fields.is_empty() {
            return 1.0;
        }

        let completed_fields = self
            .fields
            .values()
            .filter(|field| !field.value.trim().is_empty())
            .count();

        completed_fields as f32 / self.fields.len() as f32
    }

    /// Check if form has been modified
    pub fn is_dirty(&self) -> bool {
        self.fields.values().any(|field| field.is_dirty)
    }

    /// Check if any field has been touched
    pub fn is_touched(&self) -> bool {
        self.fields.values().any(|field| field.is_touched)
    }
}

impl Default for FormState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_form_state_management() {
        let mut state = FormState::new();

        // Add a field
        state.add_field(
            "email".to_string(),
            vec![
                ValidationRule::new("required", ValidationFunction::Required),
                ValidationRule::new("email", ValidationFunction::Email),
            ],
        );

        assert_eq!(state.field_names().count(), 1);
        assert!(!state.is_form_valid());

        // Update field value
        state.update_field_value("email", "test@example.com".to_string());

        let field = state.get_field("email").unwrap();
        assert_eq!(field.value, "test@example.com");
        assert!(field.is_dirty);
    }

    #[test]
    fn test_form_data_operations() {
        let mut state = FormState::new();

        state.add_field("name".to_string(), vec![]);
        state.add_field("email".to_string(), vec![]);

        let mut data = HashMap::new();
        data.insert("name".to_string(), "John Doe".to_string());
        data.insert("email".to_string(), "john@example.com".to_string());

        state.set_form_data(data.clone());

        let retrieved_data = state.get_form_data();
        assert_eq!(retrieved_data, data);
    }

    #[test]
    fn test_completion_percentage() {
        let mut state = FormState::new();

        state.add_field("field1".to_string(), vec![]);
        state.add_field("field2".to_string(), vec![]);

        // 0% completion
        assert_eq!(state.completion_percentage(), 0.0);

        // 50% completion
        state.update_field_value("field1", "value".to_string());
        assert_eq!(state.completion_percentage(), 0.5);

        // 100% completion
        state.update_field_value("field2", "value".to_string());
        assert_eq!(state.completion_percentage(), 1.0);
    }
}
