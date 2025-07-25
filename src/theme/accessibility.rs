use serde::{Deserialize, Serialize};

/// Accessibility options for theme customization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilityOptions {
    pub high_contrast: bool,
    pub reduce_motion: bool,
    pub color_blindness: Option<ColorBlindness>,
    pub large_text: bool,
    pub screen_reader_compatible: bool,
}

/// Types of color blindness to account for in theme design
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorBlindness {
    /// Red-green color blindness (most common)
    Protanopia,
    /// Red-green color blindness (second most common)
    Deuteranopia,
    /// Blue-yellow color blindness (rare)
    Tritanopia,
}

impl Default for AccessibilityOptions {
    fn default() -> Self {
        Self {
            high_contrast: false,
            reduce_motion: false,
            color_blindness: None,
            large_text: false,
            screen_reader_compatible: false,
        }
    }
}

impl AccessibilityOptions {
    /// Create high contrast accessibility options
    pub fn high_contrast() -> Self {
        Self {
            high_contrast: true,
            reduce_motion: true,
            color_blindness: None,
            large_text: true,
            screen_reader_compatible: true,
        }
    }

    /// Create options for protanopia (red-green color blindness)
    pub fn protanopia() -> Self {
        Self {
            high_contrast: false,
            reduce_motion: false,
            color_blindness: Some(ColorBlindness::Protanopia),
            large_text: false,
            screen_reader_compatible: false,
        }
    }

    /// Create options for deuteranopia (red-green color blindness)
    pub fn deuteranopia() -> Self {
        Self {
            high_contrast: false,
            reduce_motion: false,
            color_blindness: Some(ColorBlindness::Deuteranopia),
            large_text: false,
            screen_reader_compatible: false,
        }
    }

    /// Create options for tritanopia (blue-yellow color blindness)
    pub fn tritanopia() -> Self {
        Self {
            high_contrast: false,
            reduce_motion: false,
            color_blindness: Some(ColorBlindness::Tritanopia),
            large_text: false,
            screen_reader_compatible: false,
        }
    }

    /// Check if any accessibility features are enabled
    pub fn has_accessibility_features(&self) -> bool {
        self.high_contrast || 
        self.reduce_motion || 
        self.color_blindness.is_some() || 
        self.large_text || 
        self.screen_reader_compatible
    }

    /// Get accessibility description for user display
    pub fn get_description(&self) -> String {
        let mut features = Vec::new();

        if self.high_contrast {
            features.push("High Contrast");
        }
        if self.reduce_motion {
            features.push("Reduced Motion");
        }
        if let Some(color_blindness) = self.color_blindness {
            features.push(match color_blindness {
                ColorBlindness::Protanopia => "Protanopia Support",
                ColorBlindness::Deuteranopia => "Deuteranopia Support", 
                ColorBlindness::Tritanopia => "Tritanopia Support",
            });
        }
        if self.large_text {
            features.push("Large Text");
        }
        if self.screen_reader_compatible {
            features.push("Screen Reader Compatible");
        }

        if features.is_empty() {
            "No accessibility features enabled".to_string()
        } else {
            features.join(", ")
        }
    }

    /// Validate accessibility options
    pub fn validate(&self) -> Result<(), String> {
        // Currently no validation needed, but could add checks for
        // conflicting options or unsupported combinations
        Ok(())
    }

    /// Merge with another accessibility options set
    pub fn merge_with(&self, other: &AccessibilityOptions) -> AccessibilityOptions {
        AccessibilityOptions {
            high_contrast: self.high_contrast || other.high_contrast,
            reduce_motion: self.reduce_motion || other.reduce_motion,
            color_blindness: other.color_blindness.or(self.color_blindness),
            large_text: self.large_text || other.large_text,
            screen_reader_compatible: self.screen_reader_compatible || other.screen_reader_compatible,
        }
    }
}

impl ColorBlindness {
    /// Get all available color blindness types
    pub fn all_types() -> Vec<ColorBlindness> {
        vec![
            ColorBlindness::Protanopia,
            ColorBlindness::Deuteranopia,
            ColorBlindness::Tritanopia,
        ]
    }

    /// Get human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            ColorBlindness::Protanopia => "Protanopia",
            ColorBlindness::Deuteranopia => "Deuteranopia",
            ColorBlindness::Tritanopia => "Tritanopia",
        }
    }

    /// Get description of the color blindness type
    pub fn description(&self) -> &'static str {
        match self {
            ColorBlindness::Protanopia => "Red-green color blindness (missing red cones)",
            ColorBlindness::Deuteranopia => "Red-green color blindness (missing green cones)",
            ColorBlindness::Tritanopia => "Blue-yellow color blindness (missing blue cones)",
        }
    }

    /// Get prevalence information
    pub fn prevalence(&self) -> &'static str {
        match self {
            ColorBlindness::Protanopia => "~1% of men, <0.1% of women",
            ColorBlindness::Deuteranopia => "~1% of men, <0.1% of women", 
            ColorBlindness::Tritanopia => "~0.01% of population",
        }
    }
}