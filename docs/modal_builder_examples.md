# Modal Builder Pattern Examples

This document demonstrates how to use the new builder patterns for creating Modal dialogs in Comunicado.

## Overview

The Modal builder pattern provides a fluent API for constructing complex Modal dialogs with type safety and improved usability. It eliminates the need for complex constructors with many parameters.

## Basic Usage

### Simple Information Dialog

```rust
use crate::ui::modal_builder::ModalBuilder;

let modal = ModalBuilder::info("info-dialog", "Information", "This is an info message").build();
```

### Confirmation Dialog

```rust
let modal = ModalBuilder::confirmation(
    "confirm-delete", 
    "Delete File", 
    "Are you sure you want to delete this file?"
).build();
```

### Custom Confirmation with Custom Button Labels

```rust
let modal = ModalBuilder::new("custom-confirm", "Save Changes?")
    .modal_type(ModalType::Confirmation)
    .text("You have unsaved changes. Would you like to save them before continuing?")
    .buttons(vec![
        ButtonBuilder::new("Discard", "discard").danger().shortcut('d').build(),
        ButtonBuilder::new("Cancel", "cancel").secondary().shortcut('c').build(),
        ButtonBuilder::new("Save", "save").primary().default().shortcut('s').build(),
    ])
    .build();
```

## Advanced Usage

### Complex Form Modal

```rust
use crate::ui::modal_builder::{ModalBuilder, FormFieldBuilder};

let fields = vec![
    FormFieldBuilder::new("name", "Full Name")
        .text()
        .placeholder("Enter your full name")
        .help("This will be displayed in your profile")
        .build(),
    FormFieldBuilder::new("email", "Email Address") 
        .email()
        .placeholder("user@example.com")
        .validation_rules(vec![
            ValidationRule::new("required", ValidationFunction::Required),
            ValidationRule::new("email", ValidationFunction::Email),
        ])
        .build(),
    FormFieldBuilder::new("role", "Role")
        .select(vec!["Admin".to_string(), "User".to_string(), "Guest".to_string()])
        .help("Select your role in the organization")
        .build(),
];

let modal = ModalBuilder::input_form("user-registration", "User Registration", fields)
    .large()
    .custom_data("context", "onboarding")
    .custom_data("version", "1.2")
    .auto_close_after(Duration::from_secs(300))
    .build();
```

### Progress Dialog

```rust
let modal = ModalBuilder::progress(
    "file-download",
    "Downloading File", 
    "Downloading config.json...",
    45, // current
    100 // total
)
.medium()
.build();
```

### Custom Modal with Rich Content

```rust
use ratatui::text::{Line, Span};
use ratatui::style::{Color, Style};

let rich_content = vec![
    Line::from(vec![
        Span::styled("Welcome ", Style::default().fg(Color::White)),
        Span::styled("to Comunicado!", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
    ]),
    Line::from(""),
    Line::from("This is a modern terminal email client with:"),
    Line::from("• HTML email rendering"),
    Line::from("• Calendar integration"), 
    Line::from("• OAuth2 authentication"),
];

let modal = ModalBuilder::new("welcome", "Welcome!")
    .modal_type(ModalType::Information)
    .rich_text(rich_content)
    .fullscreen()
    .button(ButtonBuilder::new("Get Started", "start").success().default().build())
    .custom_data("tour_step", "1")
    .build();
```

### Choice Selection Dialog

```rust
let choices = vec![
    "Gmail (OAuth2)".to_string(),
    "Outlook (OAuth2)".to_string(), 
    "IMAP Server".to_string(),
    "Exchange".to_string(),
];

let modal = ModalBuilder::choice(
    "email-provider",
    "Choose Email Provider",
    "Select your email provider type:",
    choices
)
.medium()
.build();
```

## Builder Pattern Benefits

### Before (Traditional Constructor)

```rust
// Old way - difficult to read and remember parameter order
let modal = Modal::new(
    "test-modal".to_string(),
    ModalType::Confirmation,
    ModalSize::Medium,
    "Delete File".to_string(),
    ModalContent::Text("Are you sure?".to_string()),
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
        label: "Delete".to_string(),
        action: "delete".to_string(),
        style: ButtonStyle::Danger,
        is_default: true,
        shortcut: Some('d'),
    },
])
.closable(true);
```

### After (Builder Pattern)

```rust
// New way - fluent, readable, self-documenting
let modal = ModalBuilder::new("test-modal", "Delete File")
    .modal_type(ModalType::Confirmation)
    .text("Are you sure you want to delete this file?")
    .medium()
    .buttons(vec![
        ButtonBuilder::new("Cancel", "cancel").secondary().shortcut('c').build(),
        ButtonBuilder::new("Delete", "delete").danger().default().shortcut('d').build(),
    ])
    .closable(true)
    .build();
```

## Integration with ModalSystem

The builder pattern integrates seamlessly with the existing ModalSystem:

```rust
let mut modal_system = ModalSystem::new();

// Using convenience methods (uses builders internally)
modal_system.show_confirmation(
    "delete-confirm".to_string(),
    "Delete File".to_string(), 
    "This action cannot be undone.".to_string(),
    Some("Delete".to_string()),
    Some("Cancel".to_string()),
);

// Or building directly and adding to system
let modal = ModalBuilder::warning("disk-space", "Low Disk Space", "Less than 1GB remaining")
    .auto_close_after(Duration::from_secs(10))
    .build();
modal_system.add_modal(modal);
```

## Best Practices

1. **Use convenience methods for common cases**: `ModalBuilder::info()`, `ModalBuilder::confirmation()`, etc.
2. **Chain method calls for readability**: Build modals in a logical flow
3. **Set sensible defaults**: The builder provides good defaults for most cases
4. **Use type-safe builders**: `ButtonBuilder` and `FormFieldBuilder` provide type safety
5. **Leverage custom data**: Store contextual information for complex workflows
6. **Consider auto-close for non-critical modals**: Improves user experience

## Migration Guide

For existing code using the old Modal constructor:

1. Replace `Modal::new()` with `ModalBuilder::new()`
2. Chain configuration methods instead of calling setters
3. Use `ButtonBuilder` for complex button configurations
4. Call `.build()` at the end to create the Modal
5. Update any method calls that expect the old Modal interface

The builder pattern maintains full backward compatibility through the convenience methods in ModalSystem.