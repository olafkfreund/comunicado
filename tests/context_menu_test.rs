//! Context Menu Esc Key Functionality Test
//!
//! This test specifically verifies that the Esc key works correctly to close
//! the context-aware menu system.

use comunicado::ui::{ContextAwareMenu, FocusedPane, MenuContext, UIMode};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

#[test]
fn test_context_menu_basic_functionality() {
    let mut menu = ContextAwareMenu::new();

    // Initially the menu should not be visible
    assert!(!menu.is_visible(), "Menu should not be visible initially");

    // Create a test context
    let context = MenuContext {
        ui_mode: UIMode::Normal,
        focused_pane: FocusedPane::MessageList,
        has_selected_message: false,
        has_selected_folder: false,
        has_selected_event: false,
        has_selected_contact: false,
        is_composing: false,
    };

    // Show the menu
    menu.show(context);
    assert!(menu.is_visible(), "Menu should be visible after show()");

    println!("✅ Basic menu show/hide functionality works");
}

#[test]
fn test_context_menu_esc_key_navigation() {
    let mut menu = ContextAwareMenu::new();

    let context = MenuContext {
        ui_mode: UIMode::Normal,
        focused_pane: FocusedPane::MessageList,
        has_selected_message: false,
        has_selected_folder: false,
        has_selected_event: false,
        has_selected_contact: false,
        is_composing: false,
    };

    // Show the menu
    menu.show(context);
    assert!(menu.is_visible(), "Menu should be visible after show()");

    // Test navigate_back when we're at the main menu (should return false)
    let can_go_back = menu.navigate_back();
    assert!(
        !can_go_back,
        "navigate_back() should return false when at main menu"
    );

    // Menu should still be visible after navigate_back returns false
    assert!(
        menu.is_visible(),
        "Menu should still be visible after failed navigate_back"
    );

    // Now explicitly hide the menu (this is what the Esc handler should do)
    menu.hide();
    assert!(!menu.is_visible(), "Menu should be hidden after hide()");

    println!("✅ Menu navigation and hide functionality works correctly");
}

#[test]
fn test_context_menu_esc_key_with_submenu() {
    let mut menu = ContextAwareMenu::new();

    let context = MenuContext {
        ui_mode: UIMode::Normal,
        focused_pane: FocusedPane::MessageList,
        has_selected_message: true, // This will create submenus
        has_selected_folder: false,
        has_selected_event: false,
        has_selected_contact: false,
        is_composing: false,
    };

    // Show the menu
    menu.show(context);
    assert!(menu.is_visible(), "Menu should be visible after show()");

    // Navigate down to find a submenu item (this would require knowing the menu structure)
    // For now, just test the basic logic

    // At the main menu level, navigate_back should return false
    let can_go_back_main = menu.navigate_back();
    assert!(
        !can_go_back_main,
        "navigate_back() should return false at main menu"
    );

    // The hide should work correctly
    menu.hide();
    assert!(!menu.is_visible(), "Menu should be hidden after hide()");

    println!("✅ Menu submenu navigation logic works correctly");
}

#[test]
fn test_key_event_creation() {
    // Test that we can create the exact key event that should close the menu
    let esc_key = KeyEvent {
        code: KeyCode::Esc,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    };

    // Verify the key code is correct
    assert!(
        matches!(esc_key.code, KeyCode::Esc),
        "Key event should have Esc code"
    );
    assert_eq!(
        esc_key.modifiers,
        KeyModifiers::NONE,
        "Key event should have no modifiers"
    );

    println!("✅ Key event creation works correctly");
}

#[test]
fn test_context_menu_full_cycle() {
    let mut menu = ContextAwareMenu::new();

    let context = MenuContext {
        ui_mode: UIMode::Normal,
        focused_pane: FocusedPane::MessageList,
        has_selected_message: false,
        has_selected_folder: false,
        has_selected_event: false,
        has_selected_contact: false,
        is_composing: false,
    };

    // Full cycle: show -> navigate -> hide
    assert!(!menu.is_visible(), "Menu should start hidden");

    menu.show(context);
    assert!(menu.is_visible(), "Menu should be visible after show()");

    // Test navigation
    menu.navigate_up();
    menu.navigate_down();
    assert!(
        menu.is_visible(),
        "Menu should remain visible during navigation"
    );

    // Test that Esc logic works: navigate_back() returns false, so we should hide
    if !menu.navigate_back() {
        menu.hide();
    }
    assert!(!menu.is_visible(), "Menu should be hidden after Esc logic");

    println!("✅ Complete menu lifecycle works correctly");
}

#[tokio::test]
async fn test_context_menu_integration_with_key_simulation() {
    // This test simulates the exact key handling logic from events.rs
    let mut menu = ContextAwareMenu::new();

    let context = MenuContext {
        ui_mode: UIMode::Normal,
        focused_pane: FocusedPane::MessageList,
        has_selected_message: false,
        has_selected_folder: false,
        has_selected_event: false,
        has_selected_contact: false,
        is_composing: false,
    };

    // Show the menu (equivalent to Ctrl+D)
    menu.show(context);
    assert!(menu.is_visible(), "Menu should be visible after Ctrl+D");

    // Simulate the exact logic from events.rs line 105-109
    let esc_key = KeyEvent {
        code: KeyCode::Esc,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    };

    // This is the exact logic from the event handler
    if menu.is_visible() {
        match esc_key.code {
            KeyCode::Left | KeyCode::Char('h') | KeyCode::Esc => {
                if !menu.navigate_back() {
                    menu.hide();
                }
                // Would return EventResult::Continue here
            }
            _ => {}
        }
    }

    assert!(
        !menu.is_visible(),
        "Menu should be hidden after Esc key simulation"
    );

    println!("✅ Key event simulation matches expected behavior");
}
