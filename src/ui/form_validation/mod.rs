//! Form validation system with separated concerns
//!
//! This module implements a form validation system following the Single Responsibility
//! Principle by separating validation logic, state management, and rendering.

pub mod compat;
pub mod renderer;
pub mod state;
pub mod types;
pub mod validator;

pub use compat::FormValidationSystemCompat;
pub use renderer::FormRenderer;
pub use state::FormState;
pub use types::*;
pub use validator::FormValidator;

// For backward compatibility, re-export the old API
pub use compat::LegacyFormValidationSystem as FormValidationSystem;

/// Unified form validation system that orchestrates the separated components
pub struct UnifiedFormValidationSystem {
    validator: FormValidator,
    state: FormState,
    renderer: FormRenderer,
}

impl UnifiedFormValidationSystem {
    /// Create a new form validation system
    pub fn new() -> Self {
        Self {
            validator: FormValidator::new(),
            state: FormState::new(),
            renderer: FormRenderer::new(),
        }
    }

    /// Create with custom validator
    pub fn with_validator(validator: FormValidator) -> Self {
        Self {
            validator,
            state: FormState::new(),
            renderer: FormRenderer::new(),
        }
    }

    /// Get reference to the validator
    pub fn validator(&self) -> &FormValidator {
        &self.validator
    }

    /// Get mutable reference to the validator
    pub fn validator_mut(&mut self) -> &mut FormValidator {
        &mut self.validator
    }

    /// Get reference to the state
    pub fn state(&self) -> &FormState {
        &self.state
    }

    /// Get mutable reference to the state
    pub fn state_mut(&mut self) -> &mut FormState {
        &mut self.state
    }

    /// Get reference to the renderer
    pub fn renderer(&self) -> &FormRenderer {
        &self.renderer
    }

    /// Get mutable reference to the renderer
    pub fn renderer_mut(&mut self) -> &mut FormRenderer {
        &mut self.renderer
    }

    /// Add a field with validation rules
    pub fn add_field(&mut self, field_name: String, rules: Vec<ValidationRule>) -> &mut Self {
        self.state.add_field(field_name, rules);
        self
    }

    /// Update field value and trigger validation
    pub fn update_field_value(&mut self, field_name: &str, value: String) {
        self.state.update_field_value(field_name, value);
        if let Some(field) = self.state.get_field(field_name) {
            let result = self.validator.validate_field(field);
            self.state.set_field_validation_result(field_name, result);
        }
    }

    /// Validate all fields
    pub fn validate_all(&mut self) {
        let field_names: Vec<String> = self.state.field_names().collect();

        for field_name in field_names {
            if let Some(field) = self.state.get_field(&field_name) {
                let result = self.validator.validate_field(field);
                self.state.set_field_validation_result(&field_name, result);
            }
        }

        self.state.update_form_validity();
    }

    /// Check if the form is valid
    pub fn is_valid(&self) -> bool {
        self.state.is_form_valid()
    }

    /// Render a validated field
    pub fn render_validated_field(
        &self,
        frame: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
        field_name: &str,
        title: &str,
        theme: &crate::theme::Theme,
    ) {
        if let Some(field) = self.state.get_field(field_name) {
            self.renderer.render_field(frame, area, field, title, theme);
        }
    }

    /// Render validation summary
    pub fn render_validation_summary(
        &self,
        frame: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
        theme: &crate::theme::Theme,
    ) {
        if self.state.show_validation_summary() {
            let errors = self.state.get_validation_errors();
            self.renderer.render_summary(frame, area, &errors, theme);
        }
    }
}

impl Default for UnifiedFormValidationSystem {
    fn default() -> Self {
        Self::new()
    }
}
