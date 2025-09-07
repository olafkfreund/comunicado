//! Builder patterns for Modal construction
//!
//! This module provides fluent builder patterns for creating Modal dialogs
//! with complex configurations in an intuitive and type-safe way.

use crate::ui::modal_system::{
    ButtonStyle, FormField, Modal, ModalButton, ModalContent, ModalSize, ModalType,
};
use std::collections::HashMap;
use std::time::Duration;

/// Builder for Modal instances with fluent API
#[derive(Debug)]
pub struct ModalBuilder {
    id: String,
    modal_type: ModalType,
    size: ModalSize,
    title: String,
    content: ModalContent,
    buttons: Vec<ModalButton>,
    is_closable: bool,
    auto_close_delay: Option<Duration>,
    custom_data: HashMap<String, String>,
}

impl ModalBuilder {
    /// Create a new modal builder with required fields
    pub fn new<S: Into<String>>(id: S, title: S) -> Self {
        Self {
            id: id.into(),
            modal_type: ModalType::Information,
            size: ModalSize::Medium,
            title: title.into(),
            content: ModalContent::Text(String::new()),
            buttons: Vec::new(),
            is_closable: true,
            auto_close_delay: None,
            custom_data: HashMap::new(),
        }
    }

    /// Set the modal type
    pub fn modal_type(mut self, modal_type: ModalType) -> Self {
        self.modal_type = modal_type;
        self
    }

    /// Set the modal size
    pub fn size(mut self, size: ModalSize) -> Self {
        self.size = size;
        self
    }

    /// Set small size (40x8)
    pub fn small(mut self) -> Self {
        self.size = ModalSize::Small;
        self
    }

    /// Set medium size (60x12)
    pub fn medium(mut self) -> Self {
        self.size = ModalSize::Medium;
        self
    }

    /// Set large size (80x20)
    pub fn large(mut self) -> Self {
        self.size = ModalSize::Large;
        self
    }

    /// Set full screen size
    pub fn fullscreen(mut self) -> Self {
        self.size = ModalSize::FullScreen;
        self
    }

    /// Set custom size
    pub fn custom_size(mut self, width: u16, height: u16) -> Self {
        self.size = ModalSize::Custom { width, height };
        self
    }

    /// Set modal content as plain text
    pub fn text<S: Into<String>>(mut self, text: S) -> Self {
        self.content = ModalContent::Text(text.into());
        self
    }

    /// Set modal content as rich text (lines)
    pub fn rich_text(mut self, lines: Vec<ratatui::text::Line<'static>>) -> Self {
        self.content = ModalContent::RichText(lines);
        self
    }

    /// Set modal content as list
    pub fn list(mut self, items: Vec<String>) -> Self {
        self.content = ModalContent::List(items);
        self
    }

    /// Set modal content as form
    pub fn form(mut self, fields: Vec<FormField>) -> Self {
        self.content = ModalContent::Form(fields);
        self
    }

    /// Set modal content as progress
    pub fn progress_content<S: Into<String>>(
        mut self,
        current: u64,
        total: u64,
        message: S,
    ) -> Self {
        self.content = ModalContent::Progress {
            current,
            total,
            message: message.into(),
        };
        self
    }

    /// Set modal content as custom
    pub fn custom<S: Into<String>>(mut self, identifier: S) -> Self {
        self.content = ModalContent::Custom(identifier.into());
        self
    }

    /// Add a button to the modal
    pub fn button(mut self, button: ModalButton) -> Self {
        self.buttons.push(button);
        self
    }

    /// Add multiple buttons to the modal
    pub fn buttons(mut self, buttons: Vec<ModalButton>) -> Self {
        self.buttons.extend(buttons);
        self
    }

    /// Set whether the modal is closable with Escape
    pub fn closable(mut self, closable: bool) -> Self {
        self.is_closable = closable;
        self
    }

    /// Set auto-close delay
    pub fn auto_close_after(mut self, delay: Duration) -> Self {
        self.auto_close_delay = Some(delay);
        self
    }

    /// Add custom data
    pub fn custom_data<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.custom_data.insert(key.into(), value.into());
        self
    }

    /// Build the modal
    pub fn build(self) -> Modal {
        let mut modal = Modal::new(
            self.id,
            self.modal_type,
            self.size,
            self.title,
            self.content,
        );

        modal.buttons = self.buttons;
        modal.is_closable = self.is_closable;
        modal.auto_close_delay = self.auto_close_delay;
        modal.custom_data = self.custom_data;

        // Set default button selection
        if !modal.buttons.is_empty() {
            if let Some(default_index) = modal.buttons.iter().position(|b| b.is_default) {
                modal.selected_button = default_index;
            }
        }

        modal
    }
}

/// Builder for ModalButton instances
#[derive(Debug)]
pub struct ButtonBuilder {
    label: String,
    action: String,
    style: ButtonStyle,
    is_default: bool,
    shortcut: Option<char>,
}

impl ButtonBuilder {
    /// Create a new button builder
    pub fn new<L, A>(label: L, action: A) -> Self
    where
        L: Into<String>,
        A: Into<String>,
    {
        Self {
            label: label.into(),
            action: action.into(),
            style: ButtonStyle::Secondary,
            is_default: false,
            shortcut: None,
        }
    }

    /// Set button style to primary
    pub fn primary(mut self) -> Self {
        self.style = ButtonStyle::Primary;
        self
    }

    /// Set button style to secondary
    pub fn secondary(mut self) -> Self {
        self.style = ButtonStyle::Secondary;
        self
    }

    /// Set button style to danger
    pub fn danger(mut self) -> Self {
        self.style = ButtonStyle::Danger;
        self
    }

    /// Set button style to success
    pub fn success(mut self) -> Self {
        self.style = ButtonStyle::Success;
        self
    }

    /// Set button style directly
    pub fn style(mut self, style: ButtonStyle) -> Self {
        self.style = style;
        self
    }

    /// Mark this button as default
    pub fn default(mut self) -> Self {
        self.is_default = true;
        self
    }

    /// Set keyboard shortcut
    pub fn shortcut(mut self, key: char) -> Self {
        self.shortcut = Some(key);
        self
    }

    /// Build the button
    pub fn build(self) -> ModalButton {
        ModalButton {
            label: self.label,
            action: self.action,
            style: self.style,
            is_default: self.is_default,
            shortcut: self.shortcut,
        }
    }
}

/// Builder for FormField instances
#[derive(Debug)]
pub struct FormFieldBuilder {
    name: String,
    label: String,
    field_type: crate::ui::modal_system::FormFieldType,
    validation_rules: Vec<crate::ui::form_validation::ValidationRule>,
    placeholder: Option<String>,
    help_text: Option<String>,
}

impl FormFieldBuilder {
    /// Create a new form field builder
    pub fn new<N, L>(name: N, label: L) -> Self
    where
        N: Into<String>,
        L: Into<String>,
    {
        Self {
            name: name.into(),
            label: label.into(),
            field_type: crate::ui::modal_system::FormFieldType::Text,
            validation_rules: Vec::new(),
            placeholder: None,
            help_text: None,
        }
    }

    /// Set field type to text
    pub fn text(mut self) -> Self {
        self.field_type = crate::ui::modal_system::FormFieldType::Text;
        self
    }

    /// Set field type to email
    pub fn email(mut self) -> Self {
        self.field_type = crate::ui::modal_system::FormFieldType::Email;
        self
    }

    /// Set field type to password
    pub fn password(mut self) -> Self {
        self.field_type = crate::ui::modal_system::FormFieldType::Password;
        self
    }

    /// Set field type to number
    pub fn number(mut self) -> Self {
        self.field_type = crate::ui::modal_system::FormFieldType::Number;
        self
    }

    /// Set field type to text area
    pub fn textarea(mut self) -> Self {
        self.field_type = crate::ui::modal_system::FormFieldType::TextArea;
        self
    }

    /// Set field type to select with options
    pub fn select(mut self, options: Vec<String>) -> Self {
        self.field_type = crate::ui::modal_system::FormFieldType::Select(options);
        self
    }

    /// Set field type to checkbox
    pub fn checkbox(mut self) -> Self {
        self.field_type = crate::ui::modal_system::FormFieldType::Checkbox;
        self
    }

    /// Add a validation rule
    pub fn validation_rule(mut self, rule: crate::ui::form_validation::ValidationRule) -> Self {
        self.validation_rules.push(rule);
        self
    }

    /// Add multiple validation rules
    pub fn validation_rules(
        mut self,
        rules: Vec<crate::ui::form_validation::ValidationRule>,
    ) -> Self {
        self.validation_rules.extend(rules);
        self
    }

    /// Set placeholder text
    pub fn placeholder<S: Into<String>>(mut self, placeholder: S) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Set help text
    pub fn help<S: Into<String>>(mut self, help: S) -> Self {
        self.help_text = Some(help.into());
        self
    }

    /// Build the form field
    pub fn build(self) -> FormField {
        FormField {
            name: self.name,
            label: self.label,
            field_type: self.field_type,
            validation_rules: self.validation_rules,
            placeholder: self.placeholder,
            help_text: self.help_text,
        }
    }
}

/// Convenience functions for creating common modal types
impl ModalBuilder {
    /// Create a confirmation dialog
    pub fn confirmation<S: Into<String>>(id: S, title: S, message: S) -> Self {
        Self::new(id, title)
            .modal_type(ModalType::Confirmation)
            .text(message)
            .buttons(vec![
                ButtonBuilder::new("Cancel", "cancel")
                    .secondary()
                    .shortcut('c')
                    .build(),
                ButtonBuilder::new("Confirm", "confirm")
                    .primary()
                    .default()
                    .shortcut('y')
                    .build(),
            ])
    }

    /// Create an information dialog
    pub fn info<S: Into<String>>(id: S, title: S, message: S) -> Self {
        Self::new(id, title)
            .modal_type(ModalType::Information)
            .text(message)
            .button(
                ButtonBuilder::new("OK", "ok")
                    .primary()
                    .default()
                    .shortcut('o')
                    .build(),
            )
    }

    /// Create a warning dialog
    pub fn warning<S: Into<String>>(id: S, title: S, message: S) -> Self {
        Self::new(id, title)
            .modal_type(ModalType::Warning)
            .text(message)
            .button(
                ButtonBuilder::new("Understood", "acknowledge")
                    .primary()
                    .default()
                    .shortcut('u')
                    .build(),
            )
    }

    /// Create an error dialog
    pub fn error<S: Into<String>>(id: S, title: S, message: S) -> Self {
        Self::new(id, title)
            .modal_type(ModalType::Error)
            .text(message)
            .button(
                ButtonBuilder::new("OK", "ok")
                    .danger()
                    .default()
                    .shortcut('o')
                    .build(),
            )
    }

    /// Create a choice selection dialog
    pub fn choice<S: Into<String>>(id: S, title: S, _message: S, choices: Vec<String>) -> Self {
        Self::new(id, title)
            .modal_type(ModalType::Choice)
            .list(choices)
            .buttons(vec![
                ButtonBuilder::new("Cancel", "cancel")
                    .secondary()
                    .shortcut('c')
                    .build(),
                ButtonBuilder::new("Select", "select")
                    .primary()
                    .default()
                    .shortcut('s')
                    .build(),
            ])
    }

    /// Create a progress dialog
    pub fn progress<S: Into<String>>(
        id: S,
        title: S,
        message: S,
        current: u64,
        total: u64,
    ) -> Self {
        Self::new(id, title)
            .modal_type(ModalType::Progress)
            .progress_content(current, total, message)
            .closable(false)
    }

    /// Create an input form dialog
    pub fn input_form<S: Into<String>>(id: S, title: S, fields: Vec<FormField>) -> Self {
        Self::new(id, title)
            .modal_type(ModalType::Input)
            .large()
            .form(fields)
            .buttons(vec![
                ButtonBuilder::new("Cancel", "cancel")
                    .secondary()
                    .shortcut('c')
                    .build(),
                ButtonBuilder::new("Submit", "submit")
                    .primary()
                    .default()
                    .shortcut('s')
                    .build(),
            ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modal_builder_basic() {
        let modal = ModalBuilder::new("test", "Test Modal")
            .text("Test message")
            .small()
            .closable(false)
            .build();

        assert_eq!(modal.id, "test");
        assert_eq!(modal.title, "Test Modal");
        assert_eq!(modal.size, ModalSize::Small);
        assert!(!modal.is_closable);
    }

    #[test]
    fn test_button_builder() {
        let button = ButtonBuilder::new("Save", "save")
            .primary()
            .default()
            .shortcut('s')
            .build();

        assert_eq!(button.label, "Save");
        assert_eq!(button.action, "save");
        assert_eq!(button.style, ButtonStyle::Primary);
        assert!(button.is_default);
        assert_eq!(button.shortcut, Some('s'));
    }

    #[test]
    fn test_form_field_builder() {
        let field = FormFieldBuilder::new("email", "Email Address")
            .email()
            .placeholder("user@example.com")
            .help("Enter your email address")
            .build();

        assert_eq!(field.name, "email");
        assert_eq!(field.label, "Email Address");
        assert!(matches!(
            field.field_type,
            crate::ui::modal_system::FormFieldType::Email
        ));
        assert_eq!(field.placeholder, Some("user@example.com".to_string()));
        assert_eq!(
            field.help_text,
            Some("Enter your email address".to_string())
        );
    }

    #[test]
    fn test_confirmation_dialog_builder() {
        let modal =
            ModalBuilder::confirmation("confirm-delete", "Delete File", "Are you sure?").build();

        assert_eq!(modal.id, "confirm-delete");
        assert_eq!(modal.modal_type, ModalType::Confirmation);
        assert_eq!(modal.buttons.len(), 2);
        assert_eq!(modal.buttons[1].action, "confirm");
        assert!(modal.buttons[1].is_default);
    }

    #[test]
    fn test_complex_modal_building() {
        let fields = vec![
            FormFieldBuilder::new("name", "Name")
                .text()
                .placeholder("Enter your name")
                .build(),
            FormFieldBuilder::new("email", "Email")
                .email()
                .placeholder("user@example.com")
                .build(),
        ];

        let modal = ModalBuilder::new("user-form", "User Registration")
            .modal_type(ModalType::Input)
            .large()
            .form(fields)
            .custom_data("version", "1.0")
            .custom_data("context", "onboarding")
            .auto_close_after(Duration::from_secs(300))
            .buttons(vec![
                ButtonBuilder::new("Cancel", "cancel")
                    .secondary()
                    .shortcut('c')
                    .build(),
                ButtonBuilder::new("Register", "register")
                    .success()
                    .default()
                    .shortcut('r')
                    .build(),
            ])
            .build();

        assert_eq!(modal.id, "user-form");
        assert_eq!(modal.modal_type, ModalType::Input);
        assert_eq!(modal.size, ModalSize::Large);
        assert_eq!(modal.custom_data.get("version"), Some(&"1.0".to_string()));
        assert_eq!(
            modal.custom_data.get("context"),
            Some(&"onboarding".to_string())
        );
        assert!(modal.auto_close_delay.is_some());
        assert_eq!(modal.buttons.len(), 2);
        assert_eq!(modal.selected_button, 1); // Default button should be selected
    }
}
