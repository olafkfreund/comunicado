//! Common types for form validation system

use std::time::{Duration, Instant};

/// Validation result for a field
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationResult {
    /// Field is valid
    Valid,
    /// Field has an error
    Error(ValidationMessage),
    /// Field has a warning
    Warning(ValidationMessage),
    /// Field validation is pending (async validation)
    Pending,
}

/// Validation message with context
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationMessage {
    /// The validation message
    pub message: String,
    /// Suggested fix or help text
    pub suggestion: Option<String>,
    /// Error code for programmatic handling
    pub code: Option<String>,
}

impl ValidationMessage {
    /// Create a new validation message
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            suggestion: None,
            code: None,
        }
    }
    
    /// Add a suggestion to the message
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
    
    /// Add an error code to the message
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }
}

/// Form validation rule
#[derive(Debug, Clone)]
pub struct ValidationRule {
    /// Rule name for identification
    pub name: String,
    /// Validation function to apply
    pub validator: ValidationFunction,
    /// When to trigger this validation
    pub trigger: ValidationTrigger,
    /// Debounce delay in milliseconds
    pub debounce_ms: u64,
}

impl ValidationRule {
    /// Create a new validation rule
    pub fn new(name: impl Into<String>, validator: ValidationFunction) -> Self {
        Self {
            name: name.into(),
            validator,
            trigger: ValidationTrigger::OnChange,
            debounce_ms: 300,
        }
    }
    
    /// Set the trigger for this rule
    pub fn with_trigger(mut self, trigger: ValidationTrigger) -> Self {
        self.trigger = trigger;
        self
    }
    
    /// Set the debounce delay
    pub fn with_debounce(mut self, debounce_ms: u64) -> Self {
        self.debounce_ms = debounce_ms;
        self
    }
}

/// When to trigger validation
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationTrigger {
    /// As user types
    OnChange,
    /// When field loses focus  
    OnBlur,
    /// Only when form is submitted
    OnSubmit,
    /// Manually triggered
    Manual,
}

/// Validation function type
#[derive(Debug, Clone)]
pub enum ValidationFunction {
    /// Field must not be empty
    Required,
    /// Must be valid email format
    Email,
    /// Length constraints
    Length { min: usize, max: Option<usize> },
    /// Must match regex pattern
    Pattern { regex: String, message: String },
    /// Custom validation function
    Custom { name: String, description: String },
    /// Minimum numeric value
    MinValue(f64),
    /// Maximum numeric value  
    MaxValue(f64),
    /// Must match another field
    MatchField(String),
    /// URL validation
    Url,
    /// Phone number validation
    Phone,
}

/// Form field with validation state
#[derive(Debug, Clone)]
pub struct ValidatedField {
    /// Current field value
    pub value: String,
    /// Current validation result
    pub validation_result: ValidationResult,
    /// Validation rules for this field
    pub rules: Vec<ValidationRule>,
    /// Whether field is currently focused
    pub is_focused: bool,
    /// Whether field has been modified
    pub is_dirty: bool,
    /// Whether field has been touched (focused and blurred)
    pub is_touched: bool,
    /// Timer for debounced validation
    pub debounce_timer: Option<Instant>,
    /// Last validation timestamp
    pub last_validation: Option<Instant>,
    /// Whether to show validation suggestions
    pub show_suggestions: bool,
}

impl ValidatedField {
    /// Create a new validated field
    pub fn new(rules: Vec<ValidationRule>) -> Self {
        Self {
            value: String::new(),
            validation_result: ValidationResult::Valid,
            rules,
            is_focused: false,
            is_dirty: false,
            is_touched: false,
            debounce_timer: None,
            last_validation: None,
            show_suggestions: true,
        }
    }
    
    /// Check if field needs validation based on debounce
    pub fn needs_validation(&self) -> bool {
        if let Some(timer) = self.debounce_timer {
            timer.elapsed() >= Duration::from_millis(
                self.rules.first().map(|r| r.debounce_ms).unwrap_or(300)
            )
        } else {
            false
        }
    }
    
    /// Mark field as focused
    pub fn set_focused(&mut self, focused: bool) {
        if self.is_focused && !focused {
            self.is_touched = true;
        }
        self.is_focused = focused;
    }
    
    /// Update field value
    pub fn set_value(&mut self, value: String) {
        if self.value != value {
            self.value = value;
            self.is_dirty = true;
            self.debounce_timer = Some(Instant::now());
        }
    }
    
    /// Set validation result
    pub fn set_validation_result(&mut self, result: ValidationResult) {
        self.validation_result = result;
        self.last_validation = Some(Instant::now());
        self.debounce_timer = None;
    }
    
    /// Check if field has errors
    pub fn has_error(&self) -> bool {
        matches!(self.validation_result, ValidationResult::Error(_))
    }
    
    /// Check if field has warnings
    pub fn has_warning(&self) -> bool {
        matches!(self.validation_result, ValidationResult::Warning(_))
    }
    
    /// Check if field is valid
    pub fn is_valid(&self) -> bool {
        matches!(self.validation_result, ValidationResult::Valid)
    }
}

impl Default for ValidatedField {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}