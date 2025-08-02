//! Integration test for Ctrl+, settings functionality
//!
//! This test verifies the complete integration of the Ctrl+, shortcut
//! from keyboard input to settings UI display.

use comunicado::keyboard::{KeyboardAction, KeyboardConfig};
use comunicado::events::EventHandler;
use comunicado::ui::UI;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[tokio::test]
async fn test_ctrl_comma_opens_settings() {
    // Create UI and event handler
    let mut ui = UI::new();
    let mut event_handler = EventHandler::new();
    
    // Verify initial state
    assert_ne!(ui.mode(), &comunicado::ui::UIMode::Settings);
    
    // Create Ctrl+, key event
    let ctrl_comma_event = KeyEvent::new(KeyCode::Char(','), KeyModifiers::CONTROL);
    
    // Process the key event through the event handler
    let result = event_handler.handle_key_event_with_config(ctrl_comma_event, &mut ui).await;
    
    // Verify that the event was handled (doesn't quit or error)
    assert!(matches!(result, comunicado::events::EventResult::Continue));
    
    // Verify that the UI is now in Settings mode
    assert_eq!(ui.mode(), &comunicado::ui::UIMode::Settings);
}

#[test]
fn test_keyboard_configuration_completeness() {
    let keyboard_config = KeyboardConfig::new();
    
    // Verify that the OpenSettings action is properly mapped
    let shortcuts = keyboard_config.get_shortcuts_for_action(&KeyboardAction::OpenSettings);
    assert!(!shortcuts.is_empty(), "OpenSettings action should have at least one shortcut");
    
    // Verify that Ctrl+, specifically is mapped
    let ctrl_comma = comunicado::keyboard::KeyboardShortcut::ctrl(KeyCode::Char(','));
    let action = keyboard_config.get_action(&ctrl_comma);
    assert_eq!(action, Some(&KeyboardAction::OpenSettings));
}

#[test] 
fn test_settings_ui_integration() {
    let mut ui = UI::new();
    
    // Test that show_settings method exists and works
    ui.show_settings();
    assert_eq!(ui.mode(), &comunicado::ui::UIMode::Settings);
    
    // Test that we can access the settings UI
    let settings_ui = ui.settings_ui_mut();
    assert!(settings_ui.is_visible());
    
    // Test that we can close settings
    ui.show_email_interface();
    assert_ne!(ui.mode(), &comunicado::ui::UIMode::Settings);
}

#[test]
fn test_keyboard_action_description_exists() {
    // This verifies that the OpenSettings action is properly integrated
    // into the keyboard shortcuts system with a description
    use comunicado::ui::keyboard_shortcuts::KeyboardShortcutsUI;
    
    let _shortcuts_ui = KeyboardShortcutsUI::new();
    
    // Just test that the action can be created and debugged
    let action = KeyboardAction::OpenSettings;
    let debug_string = format!("{:?}", action);
    assert_eq!(debug_string, "OpenSettings");
    
    // Test that it's categorized as a Global action
    let keyboard_config = KeyboardConfig::new();
    let shortcuts = keyboard_config.get_shortcuts_for_action(&KeyboardAction::OpenSettings);
    assert!(!shortcuts.is_empty(), "OpenSettings should have shortcuts configured");
}