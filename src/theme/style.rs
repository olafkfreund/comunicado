use crate::theme::color::ThemeColors;
use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};

/// Component-specific styling definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentStyle {
    pub normal: StyleDefinition,
    pub focused: StyleDefinition,
    pub selected: StyleDefinition,
    pub disabled: StyleDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleDefinition {
    pub fg: Option<String>, // Color name reference
    pub bg: Option<String>,
    pub modifiers: Vec<String>, // Modifier names
}

/// Complete style set for all UI components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleSet {
    pub folder_tree: ComponentStyle,
    pub message_list: ComponentStyle,
    pub content_preview: ComponentStyle,
    pub status_bar: ComponentStyle,
    pub border: ComponentStyle,
    pub button: ComponentStyle,
    pub input: ComponentStyle,
}

impl StyleSet {
    /// Get ratatui Style for a component in a specific state
    pub fn get_style(&self, component: &str, focused: bool, colors: &ThemeColors) -> Style {
        let component_style = match component {
            "folder_tree" => &self.folder_tree,
            "message_list" => &self.message_list,
            "content_preview" => &self.content_preview,
            "status_bar" => &self.status_bar,
            "border" => &self.border,
            "button" => &self.button,
            "input" => &self.input,
            _ => &self.folder_tree, // Default fallback
        };

        let style_def = if focused {
            &component_style.focused
        } else {
            &component_style.normal
        };

        self.apply_style_definition(style_def, colors)
    }

    /// Get style for selected state
    pub fn get_selected_style(&self, component: &str, colors: &ThemeColors) -> Style {
        let component_style = match component {
            "folder_tree" => &self.folder_tree,
            "message_list" => &self.message_list,
            "content_preview" => &self.content_preview,
            "status_bar" => &self.status_bar,
            "border" => &self.border,
            "button" => &self.button,
            "input" => &self.input,
            _ => &self.folder_tree,
        };

        self.apply_style_definition(&component_style.selected, colors)
    }

    /// Get style for disabled state
    pub fn get_disabled_style(&self, component: &str, colors: &ThemeColors) -> Style {
        let component_style = match component {
            "folder_tree" => &self.folder_tree,
            "message_list" => &self.message_list,
            "content_preview" => &self.content_preview,
            "status_bar" => &self.status_bar,
            "border" => &self.border,
            "button" => &self.button,
            "input" => &self.input,
            _ => &self.folder_tree,
        };

        self.apply_style_definition(&component_style.disabled, colors)
    }

    fn apply_style_definition(&self, style_def: &StyleDefinition, colors: &ThemeColors) -> Style {
        let mut style = Style::default();

        // Apply foreground color
        if let Some(fg_name) = &style_def.fg {
            if let Some(color) = self.resolve_color_name(fg_name, colors) {
                style = style.fg(color);
            }
        }

        // Apply background color
        if let Some(bg_name) = &style_def.bg {
            if let Some(color) = self.resolve_color_name(bg_name, colors) {
                style = style.bg(color);
            }
        }

        // Apply modifiers
        for modifier_name in &style_def.modifiers {
            if let Some(modifier) = self.resolve_modifier_name(modifier_name) {
                style = style.add_modifier(modifier);
            }
        }

        style
    }

    fn resolve_color_name(&self, color_name: &str, colors: &ThemeColors) -> Option<Color> {
        match color_name {
            // Palette colors
            "background" => Some(colors.palette.background),
            "foreground" => Some(colors.palette.foreground),
            "surface" => Some(colors.palette.surface),
            "overlay" => Some(colors.palette.overlay),
            "text_primary" => Some(colors.palette.text_primary),
            "text_secondary" => Some(colors.palette.text_secondary),
            "text_muted" => Some(colors.palette.text_muted),
            "text_inverse" => Some(colors.palette.text_inverse),
            "border" => Some(colors.palette.border),
            "border_focused" => Some(colors.palette.border_focused),
            "selection" => Some(colors.palette.selection),
            "selection_text" => Some(colors.palette.selection_text),
            "success" => Some(colors.palette.success),
            "warning" => Some(colors.palette.warning),
            "error" => Some(colors.palette.error),
            "info" => Some(colors.palette.info),
            "accent" => Some(colors.palette.accent),
            "highlight" => Some(colors.palette.highlight),
            "disabled" => Some(colors.palette.disabled),

            // Component-specific colors
            "folder_normal" => Some(colors.folder_tree.folder_normal),
            "folder_selected" => Some(colors.folder_tree.folder_selected),
            "folder_unread" => Some(colors.folder_tree.folder_unread),
            "count_badge" => Some(colors.folder_tree.count_badge),
            "expand_icon" => Some(colors.folder_tree.expand_icon),

            "sender" => Some(colors.message_list.sender),
            "subject_read" => Some(colors.message_list.subject_read),
            "subject_unread" => Some(colors.message_list.subject_unread),
            "date" => Some(colors.message_list.date),
            "thread_indicator" => Some(colors.message_list.thread_indicator),
            "attachment_icon" => Some(colors.message_list.attachment_icon),
            "priority_high" => Some(colors.message_list.priority_high),

            "header" => Some(colors.content_preview.header),
            "body" => Some(colors.content_preview.body),
            "quote" => Some(colors.content_preview.quote),
            "link" => Some(colors.content_preview.link),
            "code" => Some(colors.content_preview.code),

            "status_bg" => Some(colors.status_bar.background),
            "status_text" => Some(colors.status_bar.text),
            "status_separator" => Some(colors.status_bar.section_separator),
            "status_active" => Some(colors.status_bar.active_indicator),

            _ => None,
        }
    }

    fn resolve_modifier_name(&self, modifier_name: &str) -> Option<Modifier> {
        match modifier_name {
            "bold" => Some(Modifier::BOLD),
            "italic" => Some(Modifier::ITALIC),
            "underlined" => Some(Modifier::UNDERLINED),
            "crossed_out" => Some(Modifier::CROSSED_OUT),
            "reversed" => Some(Modifier::REVERSED),
            "dim" => Some(Modifier::DIM),
            "slow_blink" => Some(Modifier::SLOW_BLINK),
            "rapid_blink" => Some(Modifier::RAPID_BLINK),
            _ => None,
        }
    }
}

impl Default for StyleSet {
    fn default() -> Self {
        Self {
            folder_tree: ComponentStyle {
                normal: StyleDefinition {
                    fg: Some("folder_normal".to_string()),
                    bg: None,
                    modifiers: vec![],
                },
                focused: StyleDefinition {
                    fg: Some("folder_normal".to_string()),
                    bg: None,
                    modifiers: vec![],
                },
                selected: StyleDefinition {
                    fg: Some("folder_selected".to_string()),
                    bg: Some("selection".to_string()),
                    modifiers: vec!["bold".to_string()],
                },
                disabled: StyleDefinition {
                    fg: Some("disabled".to_string()),
                    bg: None,
                    modifiers: vec!["dim".to_string()],
                },
            },
            message_list: ComponentStyle {
                normal: StyleDefinition {
                    fg: Some("subject_read".to_string()),
                    bg: None,
                    modifiers: vec![],
                },
                focused: StyleDefinition {
                    fg: Some("subject_read".to_string()),
                    bg: None,
                    modifiers: vec![],
                },
                selected: StyleDefinition {
                    fg: Some("selection_text".to_string()),
                    bg: Some("selection".to_string()),
                    modifiers: vec!["bold".to_string()],
                },
                disabled: StyleDefinition {
                    fg: Some("disabled".to_string()),
                    bg: None,
                    modifiers: vec!["dim".to_string()],
                },
            },
            content_preview: ComponentStyle {
                normal: StyleDefinition {
                    fg: Some("body".to_string()),
                    bg: None,
                    modifiers: vec![],
                },
                focused: StyleDefinition {
                    fg: Some("body".to_string()),
                    bg: None,
                    modifiers: vec![],
                },
                selected: StyleDefinition {
                    fg: Some("selection_text".to_string()),
                    bg: Some("selection".to_string()),
                    modifiers: vec![],
                },
                disabled: StyleDefinition {
                    fg: Some("disabled".to_string()),
                    bg: None,
                    modifiers: vec!["dim".to_string()],
                },
            },
            status_bar: ComponentStyle {
                normal: StyleDefinition {
                    fg: Some("status_text".to_string()),
                    bg: Some("status_bg".to_string()),
                    modifiers: vec![],
                },
                focused: StyleDefinition {
                    fg: Some("status_text".to_string()),
                    bg: Some("status_bg".to_string()),
                    modifiers: vec!["bold".to_string()],
                },
                selected: StyleDefinition {
                    fg: Some("status_active".to_string()),
                    bg: Some("status_bg".to_string()),
                    modifiers: vec!["bold".to_string()],
                },
                disabled: StyleDefinition {
                    fg: Some("disabled".to_string()),
                    bg: Some("status_bg".to_string()),
                    modifiers: vec!["dim".to_string()],
                },
            },
            border: ComponentStyle {
                normal: StyleDefinition {
                    fg: Some("border".to_string()),
                    bg: None,
                    modifiers: vec![],
                },
                focused: StyleDefinition {
                    fg: Some("border_focused".to_string()),
                    bg: None,
                    modifiers: vec!["bold".to_string()],
                },
                selected: StyleDefinition {
                    fg: Some("border_focused".to_string()),
                    bg: None,
                    modifiers: vec!["bold".to_string()],
                },
                disabled: StyleDefinition {
                    fg: Some("disabled".to_string()),
                    bg: None,
                    modifiers: vec!["dim".to_string()],
                },
            },
            button: ComponentStyle {
                normal: StyleDefinition {
                    fg: Some("text_primary".to_string()),
                    bg: Some("surface".to_string()),
                    modifiers: vec![],
                },
                focused: StyleDefinition {
                    fg: Some("text_inverse".to_string()),
                    bg: Some("accent".to_string()),
                    modifiers: vec!["bold".to_string()],
                },
                selected: StyleDefinition {
                    fg: Some("selection_text".to_string()),
                    bg: Some("selection".to_string()),
                    modifiers: vec!["bold".to_string()],
                },
                disabled: StyleDefinition {
                    fg: Some("disabled".to_string()),
                    bg: Some("surface".to_string()),
                    modifiers: vec!["dim".to_string()],
                },
            },
            input: ComponentStyle {
                normal: StyleDefinition {
                    fg: Some("text_primary".to_string()),
                    bg: Some("surface".to_string()),
                    modifiers: vec![],
                },
                focused: StyleDefinition {
                    fg: Some("text_primary".to_string()),
                    bg: Some("surface".to_string()),
                    modifiers: vec![],
                },
                selected: StyleDefinition {
                    fg: Some("selection_text".to_string()),
                    bg: Some("selection".to_string()),
                    modifiers: vec![],
                },
                disabled: StyleDefinition {
                    fg: Some("disabled".to_string()),
                    bg: Some("surface".to_string()),
                    modifiers: vec!["dim".to_string()],
                },
            },
        }
    }
}
