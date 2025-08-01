#[cfg(test)]
mod keyboard_shortcut_tests {
    use comunicado::keyboard::{KeyboardAction, KeyboardManager, KeyboardShortcut};
    use crossterm::event::{KeyCode, KeyModifiers};

    #[test]
    fn test_keyboard_manager_initialization() {
        let manager = KeyboardManager::default();
        println!("✅ KeyboardManager initialized successfully");
        
        // Test that basic shortcuts are registered
        let shortcuts = manager.get_all_shortcuts();
        println!("📊 Total shortcuts registered: {}", shortcuts.len());
        
        // Print first 10 shortcuts for debugging
        println!("🔍 First 10 shortcuts:");
        for (i, (shortcut, action)) in shortcuts.iter().take(10).enumerate() {
            println!("  {}: {:?} → {:?}", i + 1, shortcut, action);
        }
    }

    #[test]
    fn test_critical_shortcuts() {
        let manager = KeyboardManager::default();
        
        // Test the shortcuts that were reported as not working
        let test_cases = vec![
            (KeyCode::Char('q'), KeyModifiers::NONE, "Quit"),
            (KeyCode::Char('V'), KeyModifiers::NONE, "OpenEmailViewer"),
            (KeyCode::Char('A'), KeyModifiers::NONE, "ViewAttachment"),
            (KeyCode::Enter, KeyModifiers::NONE, "Select"),
            (KeyCode::Char('?'), KeyModifiers::NONE, "ShowKeyboardShortcuts"),
            (KeyCode::Char('t'), KeyModifiers::NONE, "ToggleThreadedView"),
            (KeyCode::Char('r'), KeyModifiers::CONTROL, "ReplyEmail"),
        ];

        println!("🧪 Testing critical shortcuts:");
        for (key_code, modifiers, expected_desc) in test_cases {
            match manager.get_action(key_code, modifiers) {
                Some(action) => {
                    println!("  ✅ {:?}+{:?} → {:?}", key_code, modifiers, action);
                }
                None => {
                    println!("  ❌ {:?}+{:?} → NOT FOUND (expected: {})", key_code, modifiers, expected_desc);
                }
            }
        }
    }

    #[test] 
    fn test_v_shortcut_specifically() {
        let manager = KeyboardManager::default();
        
        println!("🔍 Debugging V shortcut specifically:");
        
        // Test both uppercase and lowercase V
        let v_uppercase = manager.get_action(KeyCode::Char('V'), KeyModifiers::NONE);
        let v_lowercase = manager.get_action(KeyCode::Char('v'), KeyModifiers::NONE);
        
        println!("  V (uppercase): {:?}", v_uppercase);
        println!("  v (lowercase): {:?}", v_lowercase);
        
        // Check if V is registered in shortcuts
        let all_shortcuts = manager.get_all_shortcuts();
        let v_shortcuts: Vec<_> = all_shortcuts.iter()
            .filter(|(shortcut, _)| matches!(shortcut.key, KeyCode::Char('V') | KeyCode::Char('v')))
            .collect();
            
        println!("  All V-related shortcuts found: {}", v_shortcuts.len());
        for (shortcut, action) in v_shortcuts {
            println!("    {:?} → {:?}", shortcut, action);
        }
        
        // Test the shortcut creation directly
        let v_shortcut = KeyboardShortcut::simple(KeyCode::Char('V'));
        println!("  Direct V shortcut test: {:?}", manager.config().get_action(&v_shortcut));
    }

    #[test]
    fn test_shortcut_conflicts() {
        let manager = KeyboardManager::default();
        let all_shortcuts = manager.get_all_shortcuts();
        
        // Group shortcuts by key combination to find conflicts
        use std::collections::HashMap;
        let mut key_map: HashMap<(KeyCode, KeyModifiers), Vec<KeyboardAction>> = HashMap::new();
        
        for (shortcut, action) in all_shortcuts {
            let key = (shortcut.key, shortcut.modifiers);
            key_map.entry(key).or_insert_with(Vec::new).push(action);
        }
        
        println!("🔍 Checking for shortcut conflicts:");
        let mut conflicts_found = 0;
        for ((key_code, modifiers), actions) in key_map {
            if actions.len() > 1 {
                conflicts_found += 1;
                println!("  ⚠️  CONFLICT: {:?}+{:?} → {:?}", key_code, modifiers, actions);
            }
        }
        
        if conflicts_found == 0 {
            println!("  ✅ No conflicts found!");
        } else {
            println!("  ❌ Found {} conflicts", conflicts_found);
        }
    }

    #[test]
    fn test_keyboard_config_loading() {
        let manager = KeyboardManager::default();
        
        println!("🔍 Testing keyboard configuration:");
        let all_shortcuts = manager.get_all_shortcuts();
        println!("  Config shortcuts count: {}", all_shortcuts.len());
        
        // Test if the setup methods were called
        let has_quit = manager.get_action(KeyCode::Char('q'), KeyModifiers::NONE).is_some();
        let has_help = manager.get_action(KeyCode::Char('?'), KeyModifiers::NONE).is_some();
        
        println!("  Has quit shortcut (q): {}", has_quit);
        println!("  Has help shortcut (?): {}", has_help);
        
        if !has_quit || !has_help {
            println!("  ❌ Basic shortcuts missing - setup_default_shortcuts() may not have been called");
        } else {
            println!("  ✅ Basic shortcuts present - setup appears to have worked");
        }
    }
}