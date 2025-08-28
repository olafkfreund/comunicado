//! Form validation logic
//!
//! This module handles the pure validation logic, separated from state management
//! and rendering concerns.

use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;
use crate::ui::form_validation::types::*;

/// Cached regex patterns for performance
static EMAIL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")
        .expect("Invalid email regex")
});

static URL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^https?://[^\s/$.?#].[^\s]*$")
        .expect("Invalid URL regex")
});

static PHONE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\+?[1-9]\d{1,14}$")
        .expect("Invalid phone regex")
});

/// Form validator handles pure validation logic
pub struct FormValidator {
    /// Cached compiled regex patterns for custom validations
    custom_patterns: HashMap<String, Regex>,
}

impl FormValidator {
    /// Create a new form validator
    pub fn new() -> Self {
        Self {
            custom_patterns: HashMap::new(),
        }
    }
    
    /// Validate a field against all its rules
    pub fn validate_field(&self, field: &ValidatedField) -> ValidationResult {
        // Skip validation if field is empty and not required
        if field.value.is_empty() && !self.has_required_rule(&field.rules) {
            return ValidationResult::Valid;
        }
        
        // Apply validation rules in order of priority
        for rule in &field.rules {
            match &rule.validator {
                ValidationFunction::Required => {
                    if let Some(error) = self.validate_required(&field.value) {
                        return ValidationResult::Error(error);
                    }
                },
                ValidationFunction::Email => {
                    if let Some(error) = self.validate_email(&field.value) {
                        return ValidationResult::Error(error);
                    }
                },
                ValidationFunction::Length { min, max } => {
                    if let Some(error) = self.validate_length(&field.value, *min, *max) {
                        return ValidationResult::Error(error);
                    }
                },
                ValidationFunction::Pattern { regex, message } => {
                    if let Some(error) = self.validate_pattern(&field.value, regex, message) {
                        return ValidationResult::Error(error);
                    }
                },
                ValidationFunction::MinValue(min) => {
                    if let Some(error) = self.validate_min_value(&field.value, *min) {
                        return ValidationResult::Error(error);
                    }
                },
                ValidationFunction::MaxValue(max) => {
                    if let Some(error) = self.validate_max_value(&field.value, *max) {
                        return ValidationResult::Error(error);
                    }
                },
                ValidationFunction::Url => {
                    if let Some(error) = self.validate_url(&field.value) {
                        return ValidationResult::Error(error);
                    }
                },
                ValidationFunction::Phone => {
                    if let Some(error) = self.validate_phone(&field.value) {
                        return ValidationResult::Error(error);
                    }
                },
                ValidationFunction::Custom { .. } => {
                    // Custom validation would be handled by external validator
                    // For now, we'll skip it
                },
                ValidationFunction::MatchField(_) => {
                    // Field matching requires access to other fields
                    // This would be handled at a higher level
                },
            }
        }
        
        ValidationResult::Valid
    }
    
    /// Validate field matching (requires access to other field values)
    pub fn validate_field_match(
        &self,
        field: &ValidatedField,
        other_field_value: &str,
        match_field_name: &str
    ) -> ValidationResult {
        for rule in &field.rules {
            if let ValidationFunction::MatchField(target_field) = &rule.validator {
                if target_field == match_field_name && field.value != other_field_value {
                    return ValidationResult::Error(
                        ValidationMessage::new(format!("Must match {}", match_field_name))
                            .with_suggestion("Ensure both fields have the same value".to_string())
                            .with_code("FIELD_MISMATCH".to_string())
                    );
                }
            }
        }
        ValidationResult::Valid
    }
    
    /// Add a custom regex pattern
    pub fn add_custom_pattern(&mut self, name: String, pattern: &str) -> Result<(), regex::Error> {
        let regex = Regex::new(pattern)?;
        self.custom_patterns.insert(name, regex);
        Ok(())
    }
    
    // Private validation methods
    
    fn has_required_rule(&self, rules: &[ValidationRule]) -> bool {
        rules.iter().any(|rule| matches!(rule.validator, ValidationFunction::Required))
    }
    
    fn validate_required(&self, value: &str) -> Option<ValidationMessage> {
        if value.trim().is_empty() {
            Some(
                ValidationMessage::new("This field is required")
                    .with_suggestion("Please enter a value".to_string())
                    .with_code("REQUIRED".to_string())
            )
        } else {
            None
        }
    }
    
    fn validate_email(&self, value: &str) -> Option<ValidationMessage> {
        if !value.is_empty() && !EMAIL_REGEX.is_match(value) {
            Some(
                ValidationMessage::new("Invalid email format")
                    .with_suggestion("Enter a valid email address (e.g., user@example.com)".to_string())
                    .with_code("INVALID_EMAIL".to_string())
            )
        } else {
            None
        }
    }
    
    fn validate_length(&self, value: &str, min: usize, max: Option<usize>) -> Option<ValidationMessage> {
        let len = value.chars().count();
        
        if len < min {
            Some(
                ValidationMessage::new(format!("Must be at least {} characters", min))
                    .with_suggestion(format!("Add {} more characters", min - len))
                    .with_code("TOO_SHORT".to_string())
            )
        } else if let Some(max_len) = max {
            if len > max_len {
                Some(
                    ValidationMessage::new(format!("Must be no more than {} characters", max_len))
                        .with_suggestion(format!("Remove {} characters", len - max_len))
                        .with_code("TOO_LONG".to_string())
                )
            } else {
                None
            }
        } else {
            None
        }
    }
    
    fn validate_pattern(&self, value: &str, pattern: &str, message: &str) -> Option<ValidationMessage> {
        // Try to use cached pattern first
        if let Some(regex) = self.custom_patterns.values().find(|r| r.as_str() == pattern) {
            if !value.is_empty() && !regex.is_match(value) {
                return Some(
                    ValidationMessage::new(message.to_string())
                        .with_code("PATTERN_MISMATCH".to_string())
                );
            }
        } else {
            // Compile on-demand (not ideal for performance)
            if let Ok(regex) = Regex::new(pattern) {
                if !value.is_empty() && !regex.is_match(value) {
                    return Some(
                        ValidationMessage::new(message.to_string())
                            .with_code("PATTERN_MISMATCH".to_string())
                    );
                }
            }
        }
        None
    }
    
    fn validate_min_value(&self, value: &str, min: f64) -> Option<ValidationMessage> {
        if let Ok(num) = value.parse::<f64>() {
            if num < min {
                Some(
                    ValidationMessage::new(format!("Must be at least {}", min))
                        .with_suggestion(format!("Enter a value >= {}", min))
                        .with_code("VALUE_TOO_LOW".to_string())
                )
            } else {
                None
            }
        } else if !value.is_empty() {
            Some(
                ValidationMessage::new("Must be a valid number")
                    .with_suggestion("Enter a numeric value".to_string())
                    .with_code("INVALID_NUMBER".to_string())
            )
        } else {
            None
        }
    }
    
    fn validate_max_value(&self, value: &str, max: f64) -> Option<ValidationMessage> {
        if let Ok(num) = value.parse::<f64>() {
            if num > max {
                Some(
                    ValidationMessage::new(format!("Must be no more than {}", max))
                        .with_suggestion(format!("Enter a value <= {}", max))
                        .with_code("VALUE_TOO_HIGH".to_string())
                )
            } else {
                None
            }
        } else if !value.is_empty() {
            Some(
                ValidationMessage::new("Must be a valid number")
                    .with_suggestion("Enter a numeric value".to_string())
                    .with_code("INVALID_NUMBER".to_string())
            )
        } else {
            None
        }
    }
    
    fn validate_url(&self, value: &str) -> Option<ValidationMessage> {
        if !value.is_empty() && !URL_REGEX.is_match(value) {
            Some(
                ValidationMessage::new("Invalid URL format")
                    .with_suggestion("Enter a valid URL (e.g., https://example.com)".to_string())
                    .with_code("INVALID_URL".to_string())
            )
        } else {
            None
        }
    }
    
    fn validate_phone(&self, value: &str) -> Option<ValidationMessage> {
        if !value.is_empty() && !PHONE_REGEX.is_match(value) {
            Some(
                ValidationMessage::new("Invalid phone number format")
                    .with_suggestion("Enter a valid phone number (e.g., +1234567890)".to_string())
                    .with_code("INVALID_PHONE".to_string())
            )
        } else {
            None
        }
    }
}

impl Default for FormValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_email_validation() {
        let validator = FormValidator::new();
        
        let mut field = ValidatedField::new(vec![
            ValidationRule::new("email", ValidationFunction::Email)
        ]);
        
        // Valid email
        field.set_value("test@example.com".to_string());
        assert!(matches!(validator.validate_field(&field), ValidationResult::Valid));
        
        // Invalid email
        field.set_value("invalid-email".to_string());
        assert!(matches!(validator.validate_field(&field), ValidationResult::Error(_)));
    }
    
    #[test]
    fn test_required_validation() {
        let validator = FormValidator::new();
        
        let mut field = ValidatedField::new(vec![
            ValidationRule::new("required", ValidationFunction::Required)
        ]);
        
        // Empty value should fail
        field.set_value("".to_string());
        assert!(matches!(validator.validate_field(&field), ValidationResult::Error(_)));
        
        // Non-empty value should pass
        field.set_value("value".to_string());
        assert!(matches!(validator.validate_field(&field), ValidationResult::Valid));
    }
    
    #[test]
    fn test_length_validation() {
        let validator = FormValidator::new();
        
        let mut field = ValidatedField::new(vec![
            ValidationRule::new("length", ValidationFunction::Length { min: 3, max: Some(10) })
        ]);
        
        // Too short
        field.set_value("ab".to_string());
        assert!(matches!(validator.validate_field(&field), ValidationResult::Error(_)));
        
        // Valid length
        field.set_value("abcdef".to_string());
        assert!(matches!(validator.validate_field(&field), ValidationResult::Valid));
        
        // Too long
        field.set_value("abcdefghijk".to_string());
        assert!(matches!(validator.validate_field(&field), ValidationResult::Error(_)));
    }
}