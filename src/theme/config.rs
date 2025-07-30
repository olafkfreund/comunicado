use crate::theme::accessibility::AccessibilityOptions;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Theme configuration for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub current_theme: Option<String>,
    pub user_preferences: UserPreferences,
}

/// User preferences for theme customization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub auto_theme: bool,
    pub high_contrast: bool,
    pub reduce_motion: bool,
    pub accessibility: AccessibilityOptions,
    pub custom_colors: Option<CustomColorOverrides>,
    pub font_size: FontSize,
    pub spacing: SpacingPreferences,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomColorOverrides {
    pub background: Option<String>,
    pub foreground: Option<String>,
    pub accent: Option<String>,
    pub success: Option<String>,
    pub warning: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FontSize {
    Small,
    Medium,
    Large,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpacingPreferences {
    pub compact: bool,
    pub padding: u16,
    pub margin: u16,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            current_theme: None,
            user_preferences: UserPreferences::default(),
        }
    }
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            auto_theme: false,
            high_contrast: false,
            reduce_motion: false,
            accessibility: AccessibilityOptions::default(),
            custom_colors: None,
            font_size: FontSize::Medium,
            spacing: SpacingPreferences::default(),
        }
    }
}

impl Default for SpacingPreferences {
    fn default() -> Self {
        Self {
            compact: false,
            padding: 1,
            margin: 1,
        }
    }
}

impl ThemeConfig {
    /// Load theme configuration from file
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = Self::config_file_path()?;

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(config_path)?;
        let config: ThemeConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save theme configuration to file
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = Self::config_file_path()?;

        // Create parent directories if they don't exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        fs::write(config_path, content)?;
        Ok(())
    }

    /// Get the path to the theme configuration file
    fn config_file_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let config_dir = if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(xdg_config)
        } else if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".config")
        } else {
            return Err("Unable to determine config directory".into());
        };

        Ok(config_dir.join("comunicado").join("theme.toml"))
    }

    /// Reset to default configuration
    pub fn reset_to_default(&mut self) {
        *self = Self::default();
    }

    /// Validate configuration settings
    pub fn validate(&self) -> Result<(), String> {
        // Validate custom colors if provided
        if let Some(custom_colors) = &self.user_preferences.custom_colors {
            if let Some(bg) = &custom_colors.background {
                Self::validate_color_string(bg)?;
            }
            if let Some(fg) = &custom_colors.foreground {
                Self::validate_color_string(fg)?;
            }
            if let Some(accent) = &custom_colors.accent {
                Self::validate_color_string(accent)?;
            }
        }

        // Validate spacing preferences
        if self.user_preferences.spacing.padding > 10 {
            return Err("Padding cannot exceed 10".to_string());
        }
        if self.user_preferences.spacing.margin > 10 {
            return Err("Margin cannot exceed 10".to_string());
        }

        Ok(())
    }

    /// Validate a color string (hex format)
    fn validate_color_string(color: &str) -> Result<(), String> {
        if !color.starts_with('#') || color.len() != 7 {
            return Err(format!("Invalid color format: {}. Expected #RRGGBB", color));
        }

        // Check if all characters after # are valid hex
        for c in color.chars().skip(1) {
            if !c.is_ascii_hexdigit() {
                return Err(format!("Invalid hex character in color: {}", color));
            }
        }

        Ok(())
    }

    /// Convert hex color string to RGB values
    pub fn hex_to_rgb(hex: &str) -> Result<(u8, u8, u8), String> {
        if !hex.starts_with('#') || hex.len() != 7 {
            return Err("Invalid hex color format".to_string());
        }

        let r = u8::from_str_radix(&hex[1..3], 16).map_err(|_| "Invalid red component")?;
        let g = u8::from_str_radix(&hex[3..5], 16).map_err(|_| "Invalid green component")?;
        let b = u8::from_str_radix(&hex[5..7], 16).map_err(|_| "Invalid blue component")?;

        Ok((r, g, b))
    }

    /// Apply user preferences to modify a theme
    pub fn apply_preferences(&self, theme: &mut crate::theme::Theme) {
        // Apply high contrast if requested
        if self.user_preferences.high_contrast {
            theme.accessibility.high_contrast = true;
            *theme = theme
                .clone()
                .with_accessibility(theme.accessibility.clone())
                .clone();
        }

        // Apply custom color overrides
        if let Some(custom_colors) = &self.user_preferences.custom_colors {
            if let Some(bg_hex) = &custom_colors.background {
                if let Ok((r, g, b)) = Self::hex_to_rgb(bg_hex) {
                    theme.colors.palette.background = ratatui::style::Color::Rgb(r, g, b);
                }
            }
            if let Some(fg_hex) = &custom_colors.foreground {
                if let Ok((r, g, b)) = Self::hex_to_rgb(fg_hex) {
                    theme.colors.palette.foreground = ratatui::style::Color::Rgb(r, g, b);
                }
            }
            if let Some(accent_hex) = &custom_colors.accent {
                if let Ok((r, g, b)) = Self::hex_to_rgb(accent_hex) {
                    theme.colors.palette.accent = ratatui::style::Color::Rgb(r, g, b);
                }
            }
        }
    }
}
