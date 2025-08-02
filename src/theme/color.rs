use crate::theme::accessibility::ColorBlindness;
use ratatui::style::Color;
use serde::{Deserialize, Serialize};

/// Professional color palette for clean, minimalistic design
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorPalette {
    // Base colors
    pub background: Color,
    pub foreground: Color,
    pub surface: Color,
    pub overlay: Color,

    // Text colors
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_muted: Color,
    pub text_inverse: Color,

    // UI element colors
    pub border: Color,
    pub border_focused: Color,
    pub selection: Color,
    pub selection_text: Color,

    // Status colors
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,

    // Special purpose colors
    pub accent: Color,
    pub highlight: Color,
    pub disabled: Color,
}

/// Complete theme color scheme
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeColors {
    pub palette: ColorPalette,

    // Component-specific colors
    pub folder_tree: FolderTreeColors,
    pub message_list: MessageListColors,
    pub content_preview: ContentPreviewColors,
    pub status_bar: StatusBarColors,
    pub ai_assistant: AIAssistantColors,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderTreeColors {
    pub folder_normal: Color,
    pub folder_selected: Color,
    pub folder_unread: Color,
    pub count_badge: Color,
    pub expand_icon: Color,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageListColors {
    pub sender: Color,
    pub subject_read: Color,
    pub subject_unread: Color,
    pub date: Color,
    pub thread_indicator: Color,
    pub attachment_icon: Color,
    pub priority_high: Color,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentPreviewColors {
    pub header: Color,
    pub body: Color,
    pub quote: Color,
    pub link: Color,
    pub code: Color,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusBarColors {
    pub background: Color,
    pub text: Color,
    pub section_separator: Color,
    pub active_indicator: Color,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIAssistantColors {
    pub border: Color,
    pub title: Color,
    pub text: Color,
    pub selected: Color,
    pub section: Color,
    pub context: Color,
    pub help: Color,
    pub loading: Color,
    pub error: Color,
}

impl ThemeColors {
    /// Professional dark theme colors
    pub fn professional_dark() -> Self {
        let palette = ColorPalette {
            background: Color::Rgb(16, 16, 20),
            foreground: Color::Rgb(224, 224, 230),
            surface: Color::Rgb(24, 24, 28),
            overlay: Color::Rgb(32, 32, 36),

            text_primary: Color::Rgb(224, 224, 230),
            text_secondary: Color::Rgb(160, 160, 168),
            text_muted: Color::Rgb(112, 112, 120),
            text_inverse: Color::Rgb(16, 16, 20),

            border: Color::Rgb(64, 64, 72),
            border_focused: Color::Rgb(88, 166, 255),
            selection: Color::Rgb(88, 166, 255),
            selection_text: Color::Rgb(16, 16, 20),

            success: Color::Rgb(76, 175, 80),
            warning: Color::Rgb(255, 193, 7),
            error: Color::Rgb(244, 67, 54),
            info: Color::Rgb(33, 150, 243),

            accent: Color::Rgb(88, 166, 255),
            highlight: Color::Rgb(255, 235, 59),
            disabled: Color::Rgb(96, 96, 104),
        };

        Self {
            palette: palette.clone(),
            folder_tree: FolderTreeColors {
                folder_normal: palette.text_secondary,
                folder_selected: palette.selection_text,
                folder_unread: palette.text_primary,
                count_badge: palette.warning,
                expand_icon: palette.text_muted,
            },
            message_list: MessageListColors {
                sender: palette.info,
                subject_read: palette.text_secondary,
                subject_unread: palette.text_primary,
                date: palette.text_muted,
                thread_indicator: palette.accent,
                attachment_icon: palette.text_muted,
                priority_high: palette.error,
            },
            content_preview: ContentPreviewColors {
                header: palette.accent,
                body: palette.text_primary,
                quote: palette.text_muted,
                link: palette.info,
                code: Color::Rgb(152, 195, 121),
            },
            status_bar: StatusBarColors {
                background: palette.surface,
                text: palette.text_primary,
                section_separator: palette.border,
                active_indicator: palette.accent,
            },
            ai_assistant: AIAssistantColors {
                border: palette.accent,
                title: palette.accent,
                text: palette.text_primary,
                selected: palette.selection,
                section: palette.highlight,
                context: palette.text_secondary,
                help: palette.text_muted,
                loading: palette.info,
                error: palette.error,
            },
        }
    }

    /// Professional light theme colors
    pub fn professional_light() -> Self {
        let palette = ColorPalette {
            background: Color::Rgb(250, 250, 252),
            foreground: Color::Rgb(32, 32, 40),
            surface: Color::Rgb(242, 242, 245),
            overlay: Color::Rgb(234, 234, 238),

            text_primary: Color::Rgb(32, 32, 40),
            text_secondary: Color::Rgb(96, 96, 104),
            text_muted: Color::Rgb(144, 144, 152),
            text_inverse: Color::Rgb(250, 250, 252),

            border: Color::Rgb(208, 208, 216),
            border_focused: Color::Rgb(0, 122, 255),
            selection: Color::Rgb(0, 122, 255),
            selection_text: Color::Rgb(250, 250, 252),

            success: Color::Rgb(52, 199, 89),
            warning: Color::Rgb(255, 149, 0),
            error: Color::Rgb(255, 59, 48),
            info: Color::Rgb(0, 122, 255),

            accent: Color::Rgb(0, 122, 255),
            highlight: Color::Rgb(255, 204, 0),
            disabled: Color::Rgb(174, 174, 178),
        };

        Self {
            palette: palette.clone(),
            folder_tree: FolderTreeColors {
                folder_normal: palette.text_secondary,
                folder_selected: palette.selection_text,
                folder_unread: palette.text_primary,
                count_badge: palette.warning,
                expand_icon: palette.text_muted,
            },
            message_list: MessageListColors {
                sender: palette.info,
                subject_read: palette.text_secondary,
                subject_unread: palette.text_primary,
                date: palette.text_muted,
                thread_indicator: palette.accent,
                attachment_icon: palette.text_muted,
                priority_high: palette.error,
            },
            content_preview: ContentPreviewColors {
                header: palette.accent,
                body: palette.text_primary,
                quote: palette.text_muted,
                link: palette.info,
                code: Color::Rgb(108, 113, 196),
            },
            status_bar: StatusBarColors {
                background: palette.surface,
                text: palette.text_primary,
                section_separator: palette.border,
                active_indicator: palette.accent,
            },
            ai_assistant: AIAssistantColors {
                border: palette.accent,
                title: palette.accent,
                text: palette.text_primary,
                selected: palette.selection,
                section: palette.highlight,
                context: palette.text_secondary,
                help: palette.text_muted,
                loading: palette.info,
                error: palette.error,
            },
        }
    }

    /// High contrast theme for accessibility
    pub fn high_contrast() -> Self {
        let palette = ColorPalette {
            background: Color::Black,
            foreground: Color::White,
            surface: Color::Rgb(32, 32, 32),
            overlay: Color::Rgb(48, 48, 48),

            text_primary: Color::White,
            text_secondary: Color::Rgb(200, 200, 200),
            text_muted: Color::Rgb(160, 160, 160),
            text_inverse: Color::Black,

            border: Color::Rgb(128, 128, 128),
            border_focused: Color::Yellow,
            selection: Color::Yellow,
            selection_text: Color::Black,

            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            info: Color::Cyan,

            accent: Color::Yellow,
            highlight: Color::Magenta,
            disabled: Color::Rgb(80, 80, 80),
        };

        Self {
            palette: palette.clone(),
            folder_tree: FolderTreeColors {
                folder_normal: palette.text_secondary,
                folder_selected: palette.selection_text,
                folder_unread: palette.text_primary,
                count_badge: palette.warning,
                expand_icon: palette.text_muted,
            },
            message_list: MessageListColors {
                sender: palette.info,
                subject_read: palette.text_secondary,
                subject_unread: palette.text_primary,
                date: palette.text_muted,
                thread_indicator: palette.accent,
                attachment_icon: palette.text_muted,
                priority_high: palette.error,
            },
            content_preview: ContentPreviewColors {
                header: palette.accent,
                body: palette.text_primary,
                quote: palette.text_muted,
                link: palette.info,
                code: palette.success,
            },
            status_bar: StatusBarColors {
                background: palette.surface,
                text: palette.text_primary,
                section_separator: palette.border,
                active_indicator: palette.accent,
            },
            ai_assistant: AIAssistantColors {
                border: palette.accent,
                title: palette.accent,
                text: palette.text_primary,
                selected: palette.selection,
                section: palette.highlight,
                context: palette.text_secondary,
                help: palette.text_muted,
                loading: palette.info,
                error: palette.error,
            },
        }
    }

    /// Gruvbox dark theme - authentic retro groove colors
    pub fn gruvbox_dark() -> Self {
        let palette = ColorPalette {
            // Gruvbox dark background colors
            background: Color::Rgb(40, 40, 40), // #282828 - dark0
            foreground: Color::Rgb(235, 219, 178), // #ebdbb2 - light1
            surface: Color::Rgb(60, 56, 54),    // #3c3836 - dark1
            overlay: Color::Rgb(80, 73, 69),    // #504945 - dark2

            // Gruvbox text colors
            text_primary: Color::Rgb(235, 219, 178), // #ebdbb2 - light1
            text_secondary: Color::Rgb(213, 196, 161), // #d5c4a1 - light2
            text_muted: Color::Rgb(189, 174, 147),   // #bdae93 - light3
            text_inverse: Color::Rgb(40, 40, 40),    // #282828 - dark0

            // Gruvbox UI element colors
            border: Color::Rgb(102, 92, 84), // #665c54 - dark4
            border_focused: Color::Rgb(131, 165, 152), // #83a598 - bright_blue
            selection: Color::Rgb(131, 165, 152), // #83a598 - bright_blue
            selection_text: Color::Rgb(40, 40, 40), // #282828 - dark0

            // Gruvbox status colors
            success: Color::Rgb(152, 151, 26), // #98971a - bright_green
            warning: Color::Rgb(215, 153, 33), // #d79921 - bright_yellow
            error: Color::Rgb(204, 36, 29),    // #cc241d - bright_red
            info: Color::Rgb(131, 165, 152),   // #83a598 - bright_blue

            // Gruvbox accent colors
            accent: Color::Rgb(250, 189, 47), // #fabd2f - bright_yellow (more vibrant)
            highlight: Color::Rgb(211, 134, 155), // #d3869b - bright_purple
            disabled: Color::Rgb(146, 131, 116), // #928374 - gray
        };

        Self {
            palette: palette.clone(),
            folder_tree: FolderTreeColors {
                folder_normal: palette.text_secondary,
                folder_selected: palette.selection_text,
                folder_unread: Color::Rgb(250, 189, 47), // #fabd2f - bright_yellow for unread
                count_badge: Color::Rgb(215, 153, 33),   // #d79921 - bright_yellow
                expand_icon: palette.text_muted,
            },
            message_list: MessageListColors {
                sender: Color::Rgb(131, 165, 152), // #83a598 - bright_blue
                subject_read: palette.text_secondary,
                subject_unread: Color::Rgb(235, 219, 178), // #ebdbb2 - light1 (bright for unread)
                date: palette.text_muted,
                thread_indicator: Color::Rgb(142, 192, 124), // #8ec07c - bright_aqua
                attachment_icon: Color::Rgb(211, 134, 155),  // #d3869b - bright_purple
                priority_high: Color::Rgb(251, 73, 52),      // #fb4934 - bright_red (more vibrant)
            },
            content_preview: ContentPreviewColors {
                header: Color::Rgb(250, 189, 47), // #fabd2f - bright_yellow
                body: palette.text_primary,
                quote: Color::Rgb(146, 131, 116), // #928374 - gray for quotes
                link: Color::Rgb(131, 165, 152),  // #83a598 - bright_blue
                code: Color::Rgb(184, 187, 38),   // #b8bb26 - bright_green
            },
            status_bar: StatusBarColors {
                background: Color::Rgb(50, 48, 47), // #32302f - dark0_soft
                text: palette.text_primary,
                section_separator: palette.border,
                active_indicator: Color::Rgb(250, 189, 47), // #fabd2f - bright_yellow
            },
            ai_assistant: AIAssistantColors {
                border: Color::Rgb(250, 189, 47), // #fabd2f - bright_yellow
                title: Color::Rgb(250, 189, 47),  // #fabd2f - bright_yellow
                text: palette.text_primary,
                selected: Color::Rgb(131, 165, 152), // #83a598 - bright_blue
                section: Color::Rgb(184, 187, 38),   // #b8bb26 - bright_green
                context: palette.text_secondary,
                help: palette.text_muted,
                loading: Color::Rgb(131, 165, 152),  // #83a598 - bright_blue
                error: Color::Rgb(251, 73, 52),      // #fb4934 - bright_red
            },
        }
    }

    /// Gruvbox light theme - authentic retro groove light variant
    pub fn gruvbox_light() -> Self {
        let palette = ColorPalette {
            // Gruvbox light background colors
            background: Color::Rgb(251, 241, 199), // #fbf1c7 - light0
            foreground: Color::Rgb(60, 56, 54),    // #3c3836 - dark1
            surface: Color::Rgb(242, 229, 188),    // #f2e5bc - light1
            overlay: Color::Rgb(235, 219, 178),    // #ebdbb2 - light2

            // Gruvbox light text colors
            text_primary: Color::Rgb(60, 56, 54), // #3c3836 - dark1
            text_secondary: Color::Rgb(80, 73, 69), // #504945 - dark2
            text_muted: Color::Rgb(102, 92, 84),  // #665c54 - dark4
            text_inverse: Color::Rgb(251, 241, 199), // #fbf1c7 - light0

            // Gruvbox light UI element colors
            border: Color::Rgb(189, 174, 147), // #bdae93 - light3
            border_focused: Color::Rgb(7, 102, 120), // #076678 - dark_blue
            selection: Color::Rgb(7, 102, 120), // #076678 - dark_blue
            selection_text: Color::Rgb(251, 241, 199), // #fbf1c7 - light0

            // Gruvbox light status colors
            success: Color::Rgb(121, 116, 14), // #79740e - dark_green
            warning: Color::Rgb(181, 118, 20), // #b57614 - dark_yellow
            error: Color::Rgb(157, 0, 6),      // #9d0006 - dark_red
            info: Color::Rgb(7, 102, 120),     // #076678 - dark_blue

            // Gruvbox light accent colors
            accent: Color::Rgb(181, 118, 20), // #b57614 - dark_yellow
            highlight: Color::Rgb(143, 63, 113), // #8f3f71 - dark_purple
            disabled: Color::Rgb(146, 131, 116), // #928374 - gray
        };

        Self {
            palette: palette.clone(),
            folder_tree: FolderTreeColors {
                folder_normal: palette.text_secondary,
                folder_selected: palette.selection_text,
                folder_unread: Color::Rgb(181, 118, 20), // #b57614 - dark_yellow for unread
                count_badge: Color::Rgb(181, 118, 20),   // #b57614 - dark_yellow
                expand_icon: palette.text_muted,
            },
            message_list: MessageListColors {
                sender: Color::Rgb(7, 102, 120), // #076678 - dark_blue
                subject_read: palette.text_secondary,
                subject_unread: Color::Rgb(40, 40, 40), // #282828 - dark0 (darker for unread)
                date: palette.text_muted,
                thread_indicator: Color::Rgb(66, 123, 88), // #427b58 - dark_aqua
                attachment_icon: Color::Rgb(143, 63, 113), // #8f3f71 - dark_purple
                priority_high: Color::Rgb(204, 36, 29),    // #cc241d - bright_red
            },
            content_preview: ContentPreviewColors {
                header: Color::Rgb(181, 118, 20), // #b57614 - dark_yellow
                body: palette.text_primary,
                quote: Color::Rgb(146, 131, 116), // #928374 - gray for quotes
                link: Color::Rgb(7, 102, 120),    // #076678 - dark_blue
                code: Color::Rgb(121, 116, 14),   // #79740e - dark_green
            },
            status_bar: StatusBarColors {
                background: Color::Rgb(245, 234, 193), // #f5eac3 - light0_soft
                text: palette.text_primary,
                section_separator: palette.border,
                active_indicator: Color::Rgb(181, 118, 20), // #b57614 - dark_yellow
            },
            ai_assistant: AIAssistantColors {
                border: Color::Rgb(181, 118, 20), // #b57614 - dark_yellow
                title: Color::Rgb(181, 118, 20),  // #b57614 - dark_yellow
                text: palette.text_primary,
                selected: Color::Rgb(7, 102, 120),   // #076678 - dark_blue
                section: Color::Rgb(121, 116, 14),   // #79740e - dark_green
                context: palette.text_secondary,
                help: palette.text_muted,
                loading: Color::Rgb(7, 102, 120),    // #076678 - dark_blue
                error: Color::Rgb(157, 0, 6),        // #9d0006 - dark_red
            },
        }
    }

    /// Convert to high contrast version
    pub fn to_high_contrast(&self) -> Self {
        Self::high_contrast()
    }

    /// Adjust colors for color blindness
    pub fn adjust_for_color_blindness(&self, color_blindness: ColorBlindness) -> Self {
        // This is a simplified implementation
        // In a real application, you'd use proper color blindness simulation algorithms
        match color_blindness {
            ColorBlindness::Protanopia | ColorBlindness::Deuteranopia => {
                // Adjust red-green colors to blue-yellow spectrum
                let mut adjusted = self.clone();
                adjusted.palette.error = Color::Rgb(255, 140, 0); // Orange instead of red
                adjusted.palette.success = Color::Rgb(0, 162, 232); // Blue instead of green
                adjusted
            }
            ColorBlindness::Tritanopia => {
                // Adjust blue-yellow colors
                let mut adjusted = self.clone();
                adjusted.palette.info = Color::Rgb(150, 150, 150); // Gray instead of blue
                adjusted.palette.warning = Color::Rgb(255, 100, 100); // Red-orange instead of yellow
                adjusted
            }
        }
    }

    /// Validate contrast ratios for accessibility compliance
    pub fn validate_contrast_ratios(&self) -> Result<(), String> {
        // Simplified contrast validation
        // In a real implementation, you'd calculate actual WCAG contrast ratios

        // Check that there's sufficient contrast between text and background
        if self.is_similar_color(self.palette.text_primary, self.palette.background) {
            return Err("Insufficient contrast between primary text and background".to_string());
        }

        if self.is_similar_color(self.palette.text_secondary, self.palette.background) {
            return Err("Insufficient contrast between secondary text and background".to_string());
        }

        Ok(())
    }

    /// Check if two colors are too similar (simplified)
    fn is_similar_color(&self, color1: Color, color2: Color) -> bool {
        // This is a very simplified color similarity check
        // In a real implementation, you'd use proper color space calculations
        match (color1, color2) {
            (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => {
                let diff = ((r1 as i32 - r2 as i32).abs()
                    + (g1 as i32 - g2 as i32).abs()
                    + (b1 as i32 - b2 as i32).abs()) as f32;
                diff < 150.0 // Arbitrary threshold
            }
            _ => false,
        }
    }
}
