pub mod color;
pub mod style;
pub mod config;
pub mod accessibility;

use ratatui::style::Style;
use serde::{Deserialize, Serialize};

pub use color::{ColorPalette, ThemeColors};
pub use style::{ComponentStyle, StyleSet};
pub use config::{ThemeConfig, UserPreferences};
pub use accessibility::{AccessibilityOptions, ColorBlindness};

/// Main theme management structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub description: String,
    pub colors: ThemeColors,
    pub styles: StyleSet,
    pub accessibility: AccessibilityOptions,
}

impl Theme {
    /// Create a new professional dark theme
    pub fn professional_dark() -> Self {
        Self {
            name: "Professional Dark".to_string(),
            description: "Clean, minimalistic dark theme for professional use".to_string(),
            colors: ThemeColors::professional_dark(),
            styles: StyleSet::default(),
            accessibility: AccessibilityOptions::default(),
        }
    }

    /// Create a new professional light theme
    pub fn professional_light() -> Self {
        Self {
            name: "Professional Light".to_string(),
            description: "Clean, minimalistic light theme for professional use".to_string(),
            colors: ThemeColors::professional_light(),
            styles: StyleSet::default(),
            accessibility: AccessibilityOptions::default(),
        }
    }

    /// Create a high contrast theme for accessibility
    pub fn high_contrast() -> Self {
        Self {
            name: "High Contrast".to_string(),
            description: "High contrast theme for better accessibility".to_string(),
            colors: ThemeColors::high_contrast(),
            styles: StyleSet::default(),
            accessibility: AccessibilityOptions::high_contrast(),
        }
    }

    /// Create a Gruvbox dark theme
    pub fn gruvbox_dark() -> Self {
        Self {
            name: "Gruvbox Dark".to_string(),
            description: "Retro groove dark theme with warm, earthy colors".to_string(),
            colors: ThemeColors::gruvbox_dark(),
            styles: StyleSet::default(),
            accessibility: AccessibilityOptions::default(),
        }
    }

    /// Create a Gruvbox light theme
    pub fn gruvbox_light() -> Self {
        Self {
            name: "Gruvbox Light".to_string(),
            description: "Retro groove light theme with warm, earthy colors".to_string(),
            colors: ThemeColors::gruvbox_light(),
            styles: StyleSet::default(),
            accessibility: AccessibilityOptions::default(),
        }
    }

    /// Get style for a specific UI component
    pub fn get_component_style(&self, component: &str, focused: bool) -> Style {
        self.styles.get_style(component, focused, &self.colors)
    }

    /// Apply accessibility adjustments to the theme
    pub fn with_accessibility(&mut self, options: AccessibilityOptions) -> &mut Self {
        if options.high_contrast {
            self.colors = self.colors.to_high_contrast();
        }
        if let Some(color_blindness) = options.color_blindness {
            self.colors = self.colors.adjust_for_color_blindness(color_blindness);
        }
        self.accessibility = options;
        self
    }

    /// Validate theme colors for accessibility compliance
    pub fn validate_accessibility(&self) -> Result<(), String> {
        self.colors.validate_contrast_ratios()
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::gruvbox_dark()
    }
}

/// Theme manager for handling multiple themes and user preferences
#[derive(Debug)]
pub struct ThemeManager {
    themes: Vec<Theme>,
    current_theme: String,
    user_preferences: UserPreferences,
}

impl ThemeManager {
    pub fn new() -> Self {
        let themes = vec![
            Theme::gruvbox_dark(),
            Theme::gruvbox_light(),
            Theme::professional_dark(),
            Theme::professional_light(),
            Theme::high_contrast(),
        ];

        Self {
            current_theme: themes[0].name.clone(),
            themes,
            user_preferences: UserPreferences::default(),
        }
    }

    /// Get the currently active theme
    pub fn current_theme(&self) -> &Theme {
        self.themes
            .iter()
            .find(|t| t.name == self.current_theme)
            .unwrap_or(&self.themes[0])
    }

    /// Switch to a different theme
    pub fn set_theme(&mut self, theme_name: &str) -> Result<(), String> {
        if self.themes.iter().any(|t| t.name == theme_name) {
            self.current_theme = theme_name.to_string();
            Ok(())
        } else {
            Err(format!("Theme '{}' not found", theme_name))
        }
    }

    /// Get list of available themes
    pub fn available_themes(&self) -> Vec<&str> {
        self.themes.iter().map(|t| t.name.as_str()).collect()
    }

    /// Load user preferences from configuration
    pub fn load_preferences(&mut self, config: &ThemeConfig) {
        self.user_preferences = config.user_preferences.clone();
        if let Some(theme_name) = &config.current_theme {
            let _ = self.set_theme(theme_name);
        }
    }

    /// Save current configuration
    pub fn save_config(&self) -> ThemeConfig {
        ThemeConfig {
            current_theme: Some(self.current_theme.clone()),
            user_preferences: self.user_preferences.clone(),
        }
    }
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}