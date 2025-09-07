//! Comprehensive Settings UI Tests
//!
//! This test suite verifies all keyboard shortcuts and commands available in the application settings UI.
//! It ensures proper keyboard handling, navigation, and command execution across all settings tabs.

use comunicado::ui::settings_ui::{SettingsTab, SettingsUI, SettingsUIState};
use crossterm::event::{KeyCode, KeyModifiers};

#[cfg(test)]
mod settings_ui_tests {
    use super::*;

    /// Helper function to create a new SettingsUI instance for testing
    fn create_test_settings_ui() -> SettingsUI {
        let mut ui = SettingsUI::new();
        ui.show(); // Make sure it's visible for testing
        ui
    }

    /// Test all basic navigation keyboard shortcuts
    #[test]
    fn test_basic_navigation_shortcuts() {
        let mut settings_ui = create_test_settings_ui();

        // Test Tab navigation (next tab)
        let initial_tab = settings_ui.state().current_tab;
        assert!(settings_ui.handle_key(KeyCode::Tab, KeyModifiers::NONE));
        let after_tab = settings_ui.state().current_tab;
        assert_ne!(initial_tab, after_tab, "Tab should switch to next tab");

        // Test Shift+Tab navigation (previous tab)
        assert!(settings_ui.handle_key(KeyCode::BackTab, KeyModifiers::NONE));
        let after_back_tab = settings_ui.state().current_tab;
        assert_eq!(
            initial_tab, after_back_tab,
            "Shift+Tab should return to previous tab"
        );

        // Test Up/Down arrow keys
        let initial_index = settings_ui.state().selected_index;
        assert!(settings_ui.handle_key(KeyCode::Down, KeyModifiers::NONE));
        let after_down = settings_ui.state().selected_index;
        assert_ne!(
            initial_index, after_down,
            "Down arrow should change selection"
        );

        assert!(settings_ui.handle_key(KeyCode::Up, KeyModifiers::NONE));
        let after_up = settings_ui.state().selected_index;
        assert_eq!(
            initial_index, after_up,
            "Up arrow should return to original selection"
        );

        // Test Vim-style navigation (j/k)
        assert!(settings_ui.handle_key(KeyCode::Char('j'), KeyModifiers::NONE));
        let after_j = settings_ui.state().selected_index;
        assert_ne!(initial_index, after_j, "'j' should move down");

        assert!(settings_ui.handle_key(KeyCode::Char('k'), KeyModifiers::NONE));
        let after_k = settings_ui.state().selected_index;
        assert_eq!(initial_index, after_k, "'k' should move up");
    }

    /// Test the edit mode functionality
    #[test]
    fn test_edit_mode_shortcuts() {
        let mut settings_ui = create_test_settings_ui();

        // Test entering edit mode with 'e'
        assert!(
            !settings_ui.state().edit_mode,
            "Should not be in edit mode initially"
        );
        assert!(settings_ui.handle_key(KeyCode::Char('e'), KeyModifiers::NONE));
        assert!(
            settings_ui.state().edit_mode,
            "Should enter edit mode after pressing 'e'"
        );

        // Test typing in edit mode
        assert!(settings_ui.handle_key(KeyCode::Char('t'), KeyModifiers::NONE));
        assert!(settings_ui.handle_key(KeyCode::Char('e'), KeyModifiers::NONE));
        assert!(settings_ui.handle_key(KeyCode::Char('s'), KeyModifiers::NONE));
        assert!(settings_ui.handle_key(KeyCode::Char('t'), KeyModifiers::NONE));
        assert_eq!(
            settings_ui.state().input_buffer,
            "test",
            "Should capture input in edit mode"
        );

        // Test backspace in edit mode
        assert!(settings_ui.handle_key(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(
            settings_ui.state().input_buffer,
            "tes",
            "Backspace should remove last character"
        );

        // Test canceling edit mode with Esc
        assert!(settings_ui.handle_key(KeyCode::Esc, KeyModifiers::NONE));
        assert!(!settings_ui.state().edit_mode, "Esc should exit edit mode");
        assert!(
            settings_ui.state().input_buffer.is_empty(),
            "Input buffer should be cleared"
        );

        // Test entering edit mode again and confirming with Enter
        assert!(settings_ui.handle_key(KeyCode::Char('e'), KeyModifiers::NONE));
        assert!(settings_ui.handle_key(KeyCode::Char('n'), KeyModifiers::NONE));
        assert!(settings_ui.handle_key(KeyCode::Char('e'), KeyModifiers::NONE));
        assert!(settings_ui.handle_key(KeyCode::Char('w'), KeyModifiers::NONE));
        assert!(settings_ui.handle_key(KeyCode::Enter, KeyModifiers::NONE));
        assert!(
            !settings_ui.state().edit_mode,
            "Enter should exit edit mode and apply changes"
        );
    }

    /// Test control key shortcuts
    #[test]
    fn test_control_key_shortcuts() {
        let mut settings_ui = create_test_settings_ui();

        // Test Ctrl+R (Reset current setting)
        assert!(settings_ui.handle_key(KeyCode::Char('r'), KeyModifiers::CONTROL));
        // Note: We can't easily test the actual reset functionality without mocking,
        // but we can verify the key is handled

        // Test Ctrl+S (Save settings)
        assert!(settings_ui.handle_key(KeyCode::Char('s'), KeyModifiers::CONTROL));
        // Note: Similar to reset, we verify the key is handled

        // Test that regular 'r' and 's' are not handled when not in edit mode
        assert!(!settings_ui.handle_key(KeyCode::Char('r'), KeyModifiers::NONE));
        assert!(!settings_ui.handle_key(KeyCode::Char('s'), KeyModifiers::NONE));
    }

    /// Test closing settings
    #[test]
    fn test_close_settings_shortcuts() {
        let mut settings_ui = create_test_settings_ui();

        // Test Esc closes settings
        assert!(
            settings_ui.is_visible(),
            "Settings should be visible initially"
        );
        assert!(settings_ui.handle_key(KeyCode::Esc, KeyModifiers::NONE));
        assert!(!settings_ui.is_visible(), "Esc should close settings");

        // Re-open for next test
        settings_ui.show();
        assert!(settings_ui.is_visible(), "Settings should be visible again");

        // Test 'q' closes settings
        assert!(settings_ui.handle_key(KeyCode::Char('q'), KeyModifiers::NONE));
        assert!(!settings_ui.is_visible(), "'q' should close settings");
    }

    /// Test all settings tabs can be accessed
    #[test]
    fn test_all_settings_tabs() {
        let _settings_ui = create_test_settings_ui();

        let all_tabs = SettingsTab::all();
        assert_eq!(all_tabs.len(), 8, "Should have 8 settings tabs");

        // Test that we can navigate through all tabs
        for (i, expected_tab) in all_tabs.iter().enumerate() {
            // Navigate to specific tab by pressing Tab multiple times
            let mut current_ui = create_test_settings_ui();

            for _ in 0..i {
                assert!(current_ui.handle_key(KeyCode::Tab, KeyModifiers::NONE));
            }

            assert_eq!(
                current_ui.state().current_tab,
                *expected_tab,
                "Should be able to navigate to {:?} tab",
                expected_tab
            );
        }
    }

    /// Test Enter/Space selection functionality
    #[test]
    fn test_selection_shortcuts() {
        let mut settings_ui = create_test_settings_ui();

        // Test Enter key selection
        assert!(settings_ui.handle_key(KeyCode::Enter, KeyModifiers::NONE));
        // The key should be handled (we can't easily test the actual selection logic without mocking)

        // Test Space key selection
        assert!(settings_ui.handle_key(KeyCode::Char(' '), KeyModifiers::NONE));
        // Similar to Enter, the key should be handled
    }

    /// Test that unknown keys are not handled
    #[test]
    fn test_unknown_keys_not_handled() {
        let mut settings_ui = create_test_settings_ui();

        // Test various keys that should NOT be handled
        assert!(!settings_ui.handle_key(KeyCode::Char('x'), KeyModifiers::NONE));
        assert!(!settings_ui.handle_key(KeyCode::Char('z'), KeyModifiers::NONE));
        assert!(!settings_ui.handle_key(KeyCode::F(1), KeyModifiers::NONE));
        assert!(!settings_ui.handle_key(KeyCode::Insert, KeyModifiers::NONE));
        assert!(!settings_ui.handle_key(KeyCode::Delete, KeyModifiers::NONE));
    }

    /// Test tab-specific functionality for General tab
    #[test]
    fn test_general_tab_items() {
        let mut settings_ui = create_test_settings_ui();

        // Ensure we're on General tab
        while settings_ui.state().current_tab != SettingsTab::Core {
            assert!(settings_ui.handle_key(KeyCode::Tab, KeyModifiers::NONE));
        }

        // Test navigation through all General tab items
        let max_items = 10; // As defined in get_max_items_for_tab()

        for i in 0..max_items {
            // Navigate to specific item
            let mut test_ui = create_test_settings_ui();
            while test_ui.state().current_tab != SettingsTab::Core {
                assert!(test_ui.handle_key(KeyCode::Tab, KeyModifiers::NONE));
            }

            for _ in 0..i {
                assert!(test_ui.handle_key(KeyCode::Down, KeyModifiers::NONE));
            }

            assert_eq!(
                test_ui.state().selected_index,
                i,
                "Should be able to navigate to item {}",
                i
            );

            // Test selection works for this item
            assert!(test_ui.handle_key(KeyCode::Enter, KeyModifiers::NONE));
        }
    }

    /// Test tab-specific functionality for AI tab
    #[test]
    fn test_ai_tab_items() {
        let mut settings_ui = create_test_settings_ui();

        // Navigate to AI tab
        while settings_ui.state().current_tab != SettingsTab::Privacy {
            assert!(settings_ui.handle_key(KeyCode::Tab, KeyModifiers::NONE));
        }

        // Test that we can navigate through AI tab items
        let max_items = 8; // As defined in get_max_items_for_tab()

        for i in 0..max_items {
            let mut test_ui = create_test_settings_ui();
            while test_ui.state().current_tab != SettingsTab::Privacy {
                assert!(test_ui.handle_key(KeyCode::Tab, KeyModifiers::NONE));
            }

            for _ in 0..i {
                assert!(test_ui.handle_key(KeyCode::Down, KeyModifiers::NONE));
            }

            assert_eq!(
                test_ui.state().selected_index,
                i,
                "Should be able to navigate to AI item {}",
                i
            );

            // Test that 'e' key works for editing (this was the original bug)
            if i == 1 {
                // Provider setting - should be editable
                assert!(test_ui.handle_key(KeyCode::Char('e'), KeyModifiers::NONE));
                assert!(
                    test_ui.state().edit_mode,
                    "Should enter edit mode for AI provider"
                );
            }
        }
    }

    /// Test that settings UI doesn't handle keys when not visible
    #[test]
    fn test_invisible_ui_doesnt_handle_keys() {
        let mut settings_ui = SettingsUI::new();
        assert!(
            !settings_ui.is_visible(),
            "Settings should not be visible initially"
        );

        // Test that no keys are handled when UI is not visible
        assert!(!settings_ui.handle_key(KeyCode::Tab, KeyModifiers::NONE));
        assert!(!settings_ui.handle_key(KeyCode::Char('e'), KeyModifiers::NONE));
        assert!(!settings_ui.handle_key(KeyCode::Enter, KeyModifiers::NONE));
        assert!(!settings_ui.handle_key(KeyCode::Esc, KeyModifiers::NONE));
    }

    /// Test status message functionality
    #[test]
    fn test_status_message_handling() {
        let mut settings_ui = create_test_settings_ui();

        // Initially no status message
        assert!(settings_ui.state().status_message.is_none());

        // Simulate actions that would set status messages
        // (We can't directly test the internal methods without exposing them,
        // but we can test the state management)
        settings_ui
            .state_mut()
            .set_status("Test message".to_string());
        assert!(settings_ui.state().status_message.is_some());
        assert_eq!(
            settings_ui.state().status_message.as_ref().unwrap(),
            "Test message"
        );

        settings_ui.state_mut().clear_status();
        assert!(settings_ui.state().status_message.is_none());
    }

    /// Integration test: Complete workflow simulation
    #[test]
    fn test_complete_settings_workflow() {
        let mut settings_ui = create_test_settings_ui();

        // 1. Navigate to AI tab
        while settings_ui.state().current_tab != SettingsTab::Privacy {
            assert!(settings_ui.handle_key(KeyCode::Tab, KeyModifiers::NONE));
        }

        // 2. Navigate to AI provider item (index 1)
        assert!(settings_ui.handle_key(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(settings_ui.state().selected_index, 1);

        // 3. Enter edit mode with 'e' (this was the bug we fixed)
        assert!(settings_ui.handle_key(KeyCode::Char('e'), KeyModifiers::NONE));
        assert!(settings_ui.state().edit_mode, "Should enter edit mode");

        // 4. Type some text
        assert!(settings_ui.handle_key(KeyCode::Char('G'), KeyModifiers::NONE));
        assert!(settings_ui.handle_key(KeyCode::Char('P'), KeyModifiers::NONE));
        assert!(settings_ui.handle_key(KeyCode::Char('T'), KeyModifiers::NONE));
        assert_eq!(settings_ui.state().input_buffer, "GPT");

        // 5. Save with Enter
        assert!(settings_ui.handle_key(KeyCode::Enter, KeyModifiers::NONE));
        assert!(!settings_ui.state().edit_mode, "Should exit edit mode");

        // 6. Use Ctrl+S to save settings
        assert!(settings_ui.handle_key(KeyCode::Char('s'), KeyModifiers::CONTROL));

        // 7. Close settings with Esc
        assert!(settings_ui.handle_key(KeyCode::Esc, KeyModifiers::NONE));
        assert!(!settings_ui.is_visible(), "Settings should be closed");
    }

    /// Test keyboard shortcuts documentation matches implementation
    #[test]
    fn test_documented_shortcuts_work() {
        let mut settings_ui = create_test_settings_ui();

        // Based on the footer text in render_footer():
        // "Tab/Shift+Tab: Switch tabs | ↑↓: Navigate | Enter/Space: Select | E: Edit | Ctrl+R: Reset | Ctrl+S: Save | Q/Esc: Close"

        // Tab/Shift+Tab: Switch tabs
        let initial_tab = settings_ui.state().current_tab;
        assert!(settings_ui.handle_key(KeyCode::Tab, KeyModifiers::NONE));
        assert_ne!(settings_ui.state().current_tab, initial_tab);
        assert!(settings_ui.handle_key(KeyCode::BackTab, KeyModifiers::NONE));
        assert_eq!(settings_ui.state().current_tab, initial_tab);

        // ↑↓: Navigate
        let initial_index = settings_ui.state().selected_index;
        assert!(settings_ui.handle_key(KeyCode::Down, KeyModifiers::NONE));
        assert_ne!(settings_ui.state().selected_index, initial_index);
        assert!(settings_ui.handle_key(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(settings_ui.state().selected_index, initial_index);

        // Enter/Space: Select
        assert!(settings_ui.handle_key(KeyCode::Enter, KeyModifiers::NONE));
        assert!(settings_ui.handle_key(KeyCode::Char(' '), KeyModifiers::NONE));

        // E: Edit
        assert!(settings_ui.handle_key(KeyCode::Char('e'), KeyModifiers::NONE));
        assert!(settings_ui.state().edit_mode);
        assert!(settings_ui.handle_key(KeyCode::Esc, KeyModifiers::NONE)); // Exit edit mode

        // Ctrl+R: Reset
        assert!(settings_ui.handle_key(KeyCode::Char('r'), KeyModifiers::CONTROL));

        // Ctrl+S: Save
        assert!(settings_ui.handle_key(KeyCode::Char('s'), KeyModifiers::CONTROL));

        // Q/Esc: Close
        assert!(settings_ui.handle_key(KeyCode::Char('q'), KeyModifiers::NONE));
        assert!(!settings_ui.is_visible());

        settings_ui.show();
        assert!(settings_ui.handle_key(KeyCode::Esc, KeyModifiers::NONE));
        assert!(!settings_ui.is_visible());
    }
}

/// Test specific settings tab navigation and item counts
#[cfg(test)]
mod settings_tab_tests {
    use super::*;

    #[test]
    fn test_settings_tab_enum() {
        let all_tabs = SettingsTab::all();

        // Verify all expected tabs exist (using consolidated tabs)
        assert!(all_tabs.contains(&SettingsTab::Core));
        assert!(all_tabs.contains(&SettingsTab::Interface));
        assert!(all_tabs.contains(&SettingsTab::Privacy));
        assert!(all_tabs.contains(&SettingsTab::Advanced));

        // Test tab titles
        assert_eq!(SettingsTab::Core.title(), "Core Settings");
        assert_eq!(SettingsTab::Interface.title(), "Interface & Input");
        assert_eq!(SettingsTab::Privacy.title(), "Privacy & AI");
        assert_eq!(SettingsTab::Advanced.title(), "Advanced & System");
    }

    #[test]
    fn test_tab_navigation() {
        let general = SettingsTab::Core;
        let next = general.next();
        assert_eq!(next, SettingsTab::Core);

        let previous = next.previous();
        assert_eq!(previous, SettingsTab::Core);

        // Test wraparound
        let advanced = SettingsTab::Advanced;
        let wrapped_next = advanced.next();
        assert_eq!(wrapped_next, SettingsTab::Core);

        let wrapped_previous = general.previous();
        assert_eq!(wrapped_previous, SettingsTab::Advanced);
    }
}

/// Test settings UI state management
#[cfg(test)]
mod settings_state_tests {
    use super::*;

    #[test]
    fn test_settings_ui_state_creation() {
        let state = SettingsUIState::new();

        assert!(!state.visible);
        assert_eq!(state.current_tab, SettingsTab::Core);
        assert_eq!(state.selected_index, 0);
        assert!(!state.edit_mode);
        assert!(state.input_buffer.is_empty());
        assert!(!state.modified);
        assert!(state.status_message.is_none());
    }

    #[test]
    fn test_settings_ui_state_show_hide() {
        let mut state = SettingsUIState::new();

        // Test show
        state.show();
        assert!(state.visible);
        assert_eq!(state.current_tab, SettingsTab::Core);
        assert_eq!(state.selected_index, 0);
        assert!(!state.edit_mode);
        assert!(state.input_buffer.is_empty());
        assert!(state.status_message.is_none());

        // Test hide
        state.edit_mode = true;
        state.input_buffer = "test".to_string();
        state.set_status("test status".to_string());

        state.hide();
        assert!(!state.visible);
        assert!(!state.edit_mode);
        assert!(state.input_buffer.is_empty());
        assert!(state.status_message.is_none());
    }

    #[test]
    fn test_settings_ui_state_edit_mode() {
        let mut state = SettingsUIState::new();

        // Test start edit
        state.start_edit();
        assert!(state.edit_mode);
        assert!(state.input_buffer.is_empty());

        // Test handle input
        state.handle_input('h');
        state.handle_input('e');
        state.handle_input('l');
        state.handle_input('l');
        state.handle_input('o');
        assert_eq!(state.input_buffer, "hello");

        // Test backspace
        state.handle_backspace();
        assert_eq!(state.input_buffer, "hell");

        // Test cancel edit
        state.cancel_edit();
        assert!(!state.edit_mode);
        assert!(state.input_buffer.is_empty());
    }

    #[test]
    fn test_tab_max_items() {
        let mut state = SettingsUIState::new();

        // Test max items for each tab matches the render functions
        state.current_tab = SettingsTab::Core;
        assert_eq!(state.get_max_items_for_tab(), 10);

        state.current_tab = SettingsTab::Core;
        assert_eq!(state.get_max_items_for_tab(), 6);

        state.current_tab = SettingsTab::Interface;
        assert_eq!(state.get_max_items_for_tab(), 7);

        state.current_tab = SettingsTab::Interface;
        assert_eq!(state.get_max_items_for_tab(), 5);

        state.current_tab = SettingsTab::Core;
        assert_eq!(state.get_max_items_for_tab(), 6);

        state.current_tab = SettingsTab::Privacy;
        assert_eq!(state.get_max_items_for_tab(), 5);

        state.current_tab = SettingsTab::Privacy;
        assert_eq!(state.get_max_items_for_tab(), 8);

        state.current_tab = SettingsTab::Advanced;
        assert_eq!(state.get_max_items_for_tab(), 6);
    }
}
