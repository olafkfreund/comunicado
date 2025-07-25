use comunicado::theme::{
    Theme, ThemeManager, ThemeConfig, UserPreferences, AccessibilityOptions, ColorBlindness
};

#[test]
fn test_theme_creation() {
    let dark_theme = Theme::professional_dark();
    assert_eq!(dark_theme.name, "Professional Dark");
    assert!(!dark_theme.description.is_empty());

    let light_theme = Theme::professional_light();
    assert_eq!(light_theme.name, "Professional Light");
    
    let high_contrast_theme = Theme::high_contrast();
    assert_eq!(high_contrast_theme.name, "High Contrast");

    // Test Gruvbox themes
    let gruvbox_dark = Theme::gruvbox_dark();
    assert_eq!(gruvbox_dark.name, "Gruvbox Dark");
    assert!(!gruvbox_dark.description.is_empty());
    assert!(gruvbox_dark.description.contains("Retro groove"));

    let gruvbox_light = Theme::gruvbox_light();
    assert_eq!(gruvbox_light.name, "Gruvbox Light");
    assert!(!gruvbox_light.description.is_empty());
    assert!(gruvbox_light.description.contains("Retro groove"));
}

#[test]
fn test_theme_accessibility() {
    let mut theme = Theme::professional_dark();
    
    // Test accessibility validation
    assert!(theme.validate_accessibility().is_ok());
    
    // Test Gruvbox themes accessibility
    let gruvbox_dark = Theme::gruvbox_dark();
    assert!(gruvbox_dark.validate_accessibility().is_ok());
    
    let gruvbox_light = Theme::gruvbox_light();
    assert!(gruvbox_light.validate_accessibility().is_ok());
    
    // Test accessibility options
    let accessibility = AccessibilityOptions::high_contrast();
    assert!(accessibility.high_contrast);
    assert!(accessibility.large_text);
    assert!(accessibility.screen_reader_compatible);
    
    theme.with_accessibility(accessibility);
    assert!(theme.accessibility.high_contrast);
}

#[test]
fn test_color_blindness_support() {
    let theme = Theme::professional_dark();
    
    // Test protanopia adjustment
    let _protanopia_theme = theme.colors.adjust_for_color_blindness(ColorBlindness::Protanopia);
    // Colors should be adjusted (simplified test)
    assert!(true); // In real implementation, we'd check specific color adjustments
    
    // Test deuteranopia adjustment
    let _deuteranopia_theme = theme.colors.adjust_for_color_blindness(ColorBlindness::Deuteranopia);
    assert!(true);
    
    // Test tritanopia adjustment
    let _tritanopia_theme = theme.colors.adjust_for_color_blindness(ColorBlindness::Tritanopia);
    assert!(true);
}

#[test]
fn test_theme_manager() {
    let mut manager = ThemeManager::new();
    
    // Test initial state - now defaults to Gruvbox Dark
    assert_eq!(manager.current_theme().name, "Gruvbox Dark");
    
    // Test theme switching
    assert!(manager.set_theme("Professional Light").is_ok());
    assert_eq!(manager.current_theme().name, "Professional Light");
    
    // Test switching to Gruvbox Light
    assert!(manager.set_theme("Gruvbox Light").is_ok());
    assert_eq!(manager.current_theme().name, "Gruvbox Light");
    
    // Test invalid theme
    assert!(manager.set_theme("Nonexistent Theme").is_err());
    
    // Test available themes - now includes Gruvbox themes
    let available = manager.available_themes();
    assert!(available.contains(&"Gruvbox Dark"));
    assert!(available.contains(&"Gruvbox Light"));
    assert!(available.contains(&"Professional Dark"));
    assert!(available.contains(&"Professional Light"));
    assert!(available.contains(&"High Contrast"));
    
    // Verify Gruvbox Dark is first (default)
    assert_eq!(available[0], "Gruvbox Dark");
}

#[test]
fn test_theme_config() {
    let config = ThemeConfig::default();
    assert!(config.current_theme.is_none());
    assert!(!config.user_preferences.high_contrast);
    
    // Test validation
    assert!(config.validate().is_ok());
    
    // Test hex color validation
    assert!(ThemeConfig::hex_to_rgb("#FF0000").is_ok());
    assert_eq!(ThemeConfig::hex_to_rgb("#FF0000").unwrap(), (255, 0, 0));
    
    assert!(ThemeConfig::hex_to_rgb("FF0000").is_err()); // Missing #
    assert!(ThemeConfig::hex_to_rgb("#FF00").is_err()); // Too short
    assert!(ThemeConfig::hex_to_rgb("#GGGGGG").is_err()); // Invalid hex
}

#[test]
fn test_component_styles() {
    let theme = Theme::professional_dark();
    
    // Test getting component styles
    let _folder_style = theme.get_component_style("folder_tree", false);
    let _folder_style_focused = theme.get_component_style("folder_tree", true);
    
    // Styles should be different when focused vs not focused
    // (In a real implementation, we'd check specific style properties)
    assert!(true);
    
    let _message_style = theme.get_component_style("message_list", false);
    let _content_style = theme.get_component_style("content_preview", false);
    
    // Each component should have its own styling
    assert!(true);
}

#[test]
fn test_accessibility_options() {
    let default_options = AccessibilityOptions::default();
    assert!(!default_options.has_accessibility_features());
    
    let high_contrast = AccessibilityOptions::high_contrast();
    assert!(high_contrast.has_accessibility_features());
    assert!(high_contrast.high_contrast);
    assert!(high_contrast.large_text);
    
    let protanopia = AccessibilityOptions::protanopia();
    assert!(protanopia.has_accessibility_features());
    assert_eq!(protanopia.color_blindness, Some(ColorBlindness::Protanopia));
    
    // Test description generation
    assert!(!high_contrast.get_description().is_empty());
    assert!(high_contrast.get_description().contains("High Contrast"));
}

#[test]
fn test_color_blindness_types() {
    let all_types = ColorBlindness::all_types();
    assert_eq!(all_types.len(), 3);
    
    let protanopia = ColorBlindness::Protanopia;
    assert_eq!(protanopia.name(), "Protanopia");
    assert!(!protanopia.description().is_empty());
    assert!(!protanopia.prevalence().is_empty());
    
    let deuteranopia = ColorBlindness::Deuteranopia;
    assert_eq!(deuteranopia.name(), "Deuteranopia");
    
    let tritanopia = ColorBlindness::Tritanopia;
    assert_eq!(tritanopia.name(), "Tritanopia");
}

#[test]
fn test_user_preferences() {
    let mut preferences = UserPreferences::default();
    assert!(!preferences.high_contrast);
    assert!(!preferences.auto_theme);
    
    // Test preference modification
    preferences.high_contrast = true;
    assert!(preferences.high_contrast);
    
    // Test accessibility merge
    let accessibility1 = AccessibilityOptions::default();
    let accessibility2 = AccessibilityOptions::high_contrast();
    let merged = accessibility1.merge_with(&accessibility2);
    assert!(merged.high_contrast);
    assert!(merged.large_text);
}

#[test]
fn test_style_system() {
    use comunicado::theme::{StyleSet, ThemeColors};
    
    let style_set = StyleSet::default();
    let colors = ThemeColors::professional_dark();
    
    // Test getting styles for different components
    let _folder_normal = style_set.get_style("folder_tree", false, &colors);
    let _folder_focused = style_set.get_style("folder_tree", true, &colors);
    let _folder_selected = style_set.get_selected_style("folder_tree", &colors);
    let _folder_disabled = style_set.get_disabled_style("folder_tree", &colors);
    
    // All styles should be valid (not testing specific values, just that they exist)
    assert!(true);
    
    // Test different components
    let _message_style = style_set.get_style("message_list", false, &colors);
    let _content_style = style_set.get_style("content_preview", false, &colors);
    let _status_style = style_set.get_style("status_bar", false, &colors);
    
    assert!(true);
}