/// Contextual help overlay system for providing context-aware assistance
/// 
/// Displays help information based on current view mode with keyboard shortcuts,
/// feature descriptions, and navigation tips. Uses Ctrl+H and '?' for universal compatibility.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use crate::theme::Theme;
use crate::ui::typography::{TypographySystem, TypographyLevel, VisualHierarchy};
use crate::tea::message::ViewMode;

/// Help overlay component with context-aware content
#[derive(Debug, Clone)]
pub struct HelpOverlay {
    visible: bool,
    current_view: ViewMode,
    help_content: HelpContent,
}

/// Help content categorized by sections
#[derive(Debug, Clone)]
pub struct HelpContent {
    pub title: String,
    pub description: String,
    pub sections: Vec<HelpSection>,
    pub global_shortcuts: Vec<KeyBinding>,
}

/// Help section with related keyboard shortcuts and descriptions
#[derive(Debug, Clone)]
pub struct HelpSection {
    pub title: String,
    pub description: Option<String>,
    pub shortcuts: Vec<KeyBinding>,
}

/// Keyboard shortcut binding with description
#[derive(Debug, Clone)]
pub struct KeyBinding {
    pub keys: String,
    pub description: String,
    pub category: KeyBindingCategory,
}

/// Category for organizing keyboard shortcuts
#[derive(Debug, Clone, PartialEq)]
pub enum KeyBindingCategory {
    Navigation,
    Actions,
    View,
    System,
}

impl HelpOverlay {
    /// Create new help overlay
    pub fn new() -> Self {
        Self {
            visible: false,
            current_view: ViewMode::Email,
            help_content: Self::create_default_help(),
        }
    }

    /// Show help overlay for current view
    pub fn show(&mut self, view_mode: ViewMode) {
        self.visible = true;
        self.current_view = view_mode;
        self.help_content = Self::create_help_for_view(view_mode);
    }

    /// Hide help overlay
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Toggle help overlay visibility
    pub fn toggle(&mut self, view_mode: ViewMode) {
        if self.visible {
            self.hide();
        } else {
            self.show(view_mode);
        }
    }

    /// Check if help overlay is visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Render help overlay
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme, typography: &TypographySystem) {
        if !self.visible {
            return;
        }

        // Calculate overlay size (80% of terminal)
        let overlay_area = Self::centered_rect(80, 80, area);

        // Clear the background
        frame.render_widget(Clear, overlay_area);

        // Create help block with enhanced styling
        let help_block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} Help ", self.help_content.title))
            .title_style(typography.get_typography_style(TypographyLevel::Heading2, theme))
            .border_style(theme.get_component_style("border", true))
            .style(Style::default().bg(theme.colors.palette.overlay));

        let inner_area = help_block.inner(overlay_area);
        frame.render_widget(help_block, overlay_area);

        // Split content area
        let content_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Description
                Constraint::Min(10),   // Content
                Constraint::Length(2), // Footer
            ])
            .split(inner_area);

        // Render description
        self.render_description(frame, content_chunks[0], theme, typography);

        // Render help content
        self.render_content(frame, content_chunks[1], theme, typography);

        // Render footer
        self.render_footer(frame, content_chunks[2], theme, typography);
    }

    /// Render help description
    fn render_description(&self, frame: &mut Frame, area: Rect, theme: &Theme, typography: &TypographySystem) {
        let description_text = typography.create_text(
            &self.help_content.description,
            TypographyLevel::Body,
            theme,
        );

        let description_paragraph = Paragraph::new(description_text)
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Center);

        frame.render_widget(description_paragraph, area);
    }

    /// Render help content with sections
    fn render_content(&self, frame: &mut Frame, area: Rect, theme: &Theme, typography: &TypographySystem) {
        let mut lines = Vec::new();

        // Add sections
        for (i, section) in self.help_content.sections.iter().enumerate() {
            if i > 0 {
                lines.push(Line::from(""));
            }

            // Section title
            lines.push(Line::from(vec![
                typography.create_span(
                    section.title.clone(),
                    TypographyLevel::Heading3,
                    theme,
                )
            ]));

            // Section description
            if let Some(desc) = &section.description {
                lines.push(Line::from(vec![
                    typography.create_span(desc.clone(), TypographyLevel::Caption, theme)
                ]));
                lines.push(Line::from(""));
            }

            // Section shortcuts
            for shortcut in &section.shortcuts {
                lines.push(self.create_shortcut_line(shortcut, theme, typography));
            }
        }

        // Add global shortcuts if not empty
        if !self.help_content.global_shortcuts.is_empty() {
            lines.push(Line::from(""));
            lines.push(VisualHierarchy::subtle_divider(theme));
            lines.push(Line::from(""));

            lines.push(Line::from(vec![
                typography.create_span(
                    "Global Shortcuts".to_string(),
                    TypographyLevel::Heading3,
                    theme,
                )
            ]));

            for shortcut in &self.help_content.global_shortcuts {
                lines.push(self.create_shortcut_line(shortcut, theme, typography));
            }
        }

        let content_text = Text::from(lines);
        let content_paragraph = Paragraph::new(content_text)
            .wrap(Wrap { trim: true })
            .scroll((0, 0));

        frame.render_widget(content_paragraph, area);
    }

    /// Create a formatted line for a keyboard shortcut
    fn create_shortcut_line(&self, shortcut: &KeyBinding, theme: &Theme, typography: &TypographySystem) -> Line {
        let mut spans = Vec::new();

        // Add spacing based on density
        let spacing = typography.spacing();
        spans.push(Span::raw(" ".repeat(spacing.sm as usize)));

        // Key binding (highlighted)
        spans.push(typography.create_emphasis(&shortcut.keys, theme));

        // Separator
        spans.push(typography.create_span(" → ".to_string(), TypographyLevel::Metadata, theme));

        // Description
        spans.push(typography.create_span(
            shortcut.description.clone(),
            TypographyLevel::Body,
            theme,
        ));

        Line::from(spans)
    }

    /// Render footer with close instructions
    fn render_footer(&self, frame: &mut Frame, area: Rect, theme: &Theme, typography: &TypographySystem) {
        let footer_text = Line::from(vec![
            typography.create_span("Press ".to_string(), TypographyLevel::Caption, theme),
            typography.create_emphasis("Ctrl+H", theme),
            typography.create_span(" or ".to_string(), TypographyLevel::Caption, theme),
            typography.create_emphasis("?", theme),
            typography.create_span(" to close help, ".to_string(), TypographyLevel::Caption, theme),
            typography.create_emphasis("Esc", theme),
            typography.create_span(" to close".to_string(), TypographyLevel::Caption, theme),
        ]);

        let footer_paragraph = Paragraph::new(footer_text)
            .alignment(Alignment::Center)
            .style(typography.get_typography_style(TypographyLevel::Metadata, theme));

        frame.render_widget(footer_paragraph, area);
    }

    /// Create centered rectangle for overlay
    fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }

    /// Create default help content
    fn create_default_help() -> HelpContent {
        HelpContent {
            title: "General".to_string(),
            description: "General help and keyboard shortcuts".to_string(),
            sections: vec![],
            global_shortcuts: Self::get_global_shortcuts(),
        }
    }

    /// Create help content for specific view mode
    fn create_help_for_view(view_mode: ViewMode) -> HelpContent {
        match view_mode {
            ViewMode::Email => Self::create_email_help(),
            ViewMode::Calendar => Self::create_calendar_help(),
            ViewMode::Contacts => Self::create_contacts_help(),
            ViewMode::Settings => Self::create_settings_help(),
        }
    }

    /// Create email view help content
    fn create_email_help() -> HelpContent {
        HelpContent {
            title: "Email Client".to_string(),
            description: "Navigate and manage your emails efficiently with these keyboard shortcuts.".to_string(),
            sections: vec![
                HelpSection {
                    title: "Email Navigation".to_string(),
                    description: Some("Move between folders and messages".to_string()),
                    shortcuts: vec![
                        KeyBinding {
                            keys: "↑/↓, j/k".to_string(),
                            description: "Navigate message list".to_string(),
                            category: KeyBindingCategory::Navigation,
                        },
                        KeyBinding {
                            keys: "Enter".to_string(),
                            description: "Open selected message".to_string(),
                            category: KeyBindingCategory::Actions,
                        },
                        KeyBinding {
                            keys: "Tab".to_string(),
                            description: "Switch between folder tree and message list".to_string(),
                            category: KeyBindingCategory::Navigation,
                        },
                        KeyBinding {
                            keys: "h/l".to_string(),
                            description: "Collapse/expand folder tree".to_string(),
                            category: KeyBindingCategory::View,
                        },
                    ],
                },
                HelpSection {
                    title: "Email Actions".to_string(),
                    description: Some("Compose, reply, and manage messages".to_string()),
                    shortcuts: vec![
                        KeyBinding {
                            keys: "c".to_string(),
                            description: "Compose new email".to_string(),
                            category: KeyBindingCategory::Actions,
                        },
                        KeyBinding {
                            keys: "r".to_string(),
                            description: "Reply to selected message".to_string(),
                            category: KeyBindingCategory::Actions,
                        },
                        KeyBinding {
                            keys: "R".to_string(),
                            description: "Reply all to selected message".to_string(),
                            category: KeyBindingCategory::Actions,
                        },
                        KeyBinding {
                            keys: "f".to_string(),
                            description: "Forward selected message".to_string(),
                            category: KeyBindingCategory::Actions,
                        },
                        KeyBinding {
                            keys: "d, Delete".to_string(),
                            description: "Delete selected message".to_string(),
                            category: KeyBindingCategory::Actions,
                        },
                        KeyBinding {
                            keys: "m".to_string(),
                            description: "Mark as read/unread".to_string(),
                            category: KeyBindingCategory::Actions,
                        },
                    ],
                },
                HelpSection {
                    title: "Search & Filter".to_string(),
                    description: Some("Find messages and apply filters".to_string()),
                    shortcuts: vec![
                        KeyBinding {
                            keys: "/".to_string(),
                            description: "Start search".to_string(),
                            category: KeyBindingCategory::Actions,
                        },
                        KeyBinding {
                            keys: "n".to_string(),
                            description: "Next search result".to_string(),
                            category: KeyBindingCategory::Navigation,
                        },
                        KeyBinding {
                            keys: "N".to_string(),
                            description: "Previous search result".to_string(),
                            category: KeyBindingCategory::Navigation,
                        },
                        KeyBinding {
                            keys: "Esc".to_string(),
                            description: "Clear search".to_string(),
                            category: KeyBindingCategory::Actions,
                        },
                    ],
                },
            ],
            global_shortcuts: Self::get_global_shortcuts(),
        }
    }

    /// Create calendar view help content
    fn create_calendar_help() -> HelpContent {
        HelpContent {
            title: "Calendar".to_string(),
            description: "Manage your calendar events and appointments with these shortcuts.".to_string(),
            sections: vec![
                HelpSection {
                    title: "Calendar Navigation".to_string(),
                    description: Some("Navigate between dates and views".to_string()),
                    shortcuts: vec![
                        KeyBinding {
                            keys: "←/→, h/l".to_string(),
                            description: "Previous/next day".to_string(),
                            category: KeyBindingCategory::Navigation,
                        },
                        KeyBinding {
                            keys: "↑/↓, j/k".to_string(),
                            description: "Previous/next week".to_string(),
                            category: KeyBindingCategory::Navigation,
                        },
                        KeyBinding {
                            keys: "PageUp/PageDown".to_string(),
                            description: "Previous/next month".to_string(),
                            category: KeyBindingCategory::Navigation,
                        },
                        KeyBinding {
                            keys: "t".to_string(),
                            description: "Go to today".to_string(),
                            category: KeyBindingCategory::Navigation,
                        },
                        KeyBinding {
                            keys: "g".to_string(),
                            description: "Go to specific date".to_string(),
                            category: KeyBindingCategory::Navigation,
                        },
                    ],
                },
                HelpSection {
                    title: "Event Management".to_string(),
                    description: Some("Create, edit, and manage calendar events".to_string()),
                    shortcuts: vec![
                        KeyBinding {
                            keys: "n".to_string(),
                            description: "Create new event".to_string(),
                            category: KeyBindingCategory::Actions,
                        },
                        KeyBinding {
                            keys: "Enter".to_string(),
                            description: "View/edit selected event".to_string(),
                            category: KeyBindingCategory::Actions,
                        },
                        KeyBinding {
                            keys: "d, Delete".to_string(),
                            description: "Delete selected event".to_string(),
                            category: KeyBindingCategory::Actions,
                        },
                        KeyBinding {
                            keys: "c".to_string(),
                            description: "Copy event".to_string(),
                            category: KeyBindingCategory::Actions,
                        },
                        KeyBinding {
                            keys: "e".to_string(),
                            description: "Edit event".to_string(),
                            category: KeyBindingCategory::Actions,
                        },
                    ],
                },
                HelpSection {
                    title: "Calendar Views".to_string(),
                    description: Some("Switch between different calendar view modes".to_string()),
                    shortcuts: vec![
                        KeyBinding {
                            keys: "1".to_string(),
                            description: "Day view".to_string(),
                            category: KeyBindingCategory::View,
                        },
                        KeyBinding {
                            keys: "2".to_string(),
                            description: "Week view".to_string(),
                            category: KeyBindingCategory::View,
                        },
                        KeyBinding {
                            keys: "3".to_string(),
                            description: "Month view".to_string(),
                            category: KeyBindingCategory::View,
                        },
                        KeyBinding {
                            keys: "4".to_string(),
                            description: "Agenda view".to_string(),
                            category: KeyBindingCategory::View,
                        },
                    ],
                },
            ],
            global_shortcuts: Self::get_global_shortcuts(),
        }
    }

    /// Create contacts view help content
    fn create_contacts_help() -> HelpContent {
        HelpContent {
            title: "Contacts".to_string(),
            description: "Manage your contacts and address book with these keyboard shortcuts.".to_string(),
            sections: vec![
                HelpSection {
                    title: "Contact Navigation".to_string(),
                    description: Some("Browse and search through your contacts".to_string()),
                    shortcuts: vec![
                        KeyBinding {
                            keys: "↑/↓, j/k".to_string(),
                            description: "Navigate contact list".to_string(),
                            category: KeyBindingCategory::Navigation,
                        },
                        KeyBinding {
                            keys: "Enter".to_string(),
                            description: "View selected contact".to_string(),
                            category: KeyBindingCategory::Actions,
                        },
                        KeyBinding {
                            keys: "Page Up/Down".to_string(),
                            description: "Scroll contact list".to_string(),
                            category: KeyBindingCategory::Navigation,
                        },
                    ],
                },
                HelpSection {
                    title: "Contact Management".to_string(),
                    description: Some("Add, edit, and organize your contacts".to_string()),
                    shortcuts: vec![
                        KeyBinding {
                            keys: "n".to_string(),
                            description: "Add new contact".to_string(),
                            category: KeyBindingCategory::Actions,
                        },
                        KeyBinding {
                            keys: "e".to_string(),
                            description: "Edit selected contact".to_string(),
                            category: KeyBindingCategory::Actions,
                        },
                        KeyBinding {
                            keys: "d, Delete".to_string(),
                            description: "Delete selected contact".to_string(),
                            category: KeyBindingCategory::Actions,
                        },
                        KeyBinding {
                            keys: "c".to_string(),
                            description: "Copy contact details".to_string(),
                            category: KeyBindingCategory::Actions,
                        },
                    ],
                },
                HelpSection {
                    title: "Contact Actions".to_string(),
                    description: Some("Communicate with your contacts".to_string()),
                    shortcuts: vec![
                        KeyBinding {
                            keys: "m".to_string(),
                            description: "Send email to contact".to_string(),
                            category: KeyBindingCategory::Actions,
                        },
                        KeyBinding {
                            keys: "i".to_string(),
                            description: "Import contacts".to_string(),
                            category: KeyBindingCategory::Actions,
                        },
                        KeyBinding {
                            keys: "x".to_string(),
                            description: "Export contacts".to_string(),
                            category: KeyBindingCategory::Actions,
                        },
                    ],
                },
            ],
            global_shortcuts: Self::get_global_shortcuts(),
        }
    }

    /// Create settings view help content
    fn create_settings_help() -> HelpContent {
        HelpContent {
            title: "Settings".to_string(),
            description: "Configure Comunicado preferences and account settings.".to_string(),
            sections: vec![
                HelpSection {
                    title: "Settings Navigation".to_string(),
                    description: Some("Navigate through settings categories".to_string()),
                    shortcuts: vec![
                        KeyBinding {
                            keys: "↑/↓, j/k".to_string(),
                            description: "Navigate settings options".to_string(),
                            category: KeyBindingCategory::Navigation,
                        },
                        KeyBinding {
                            keys: "Enter".to_string(),
                            description: "Select/edit setting".to_string(),
                            category: KeyBindingCategory::Actions,
                        },
                        KeyBinding {
                            keys: "Tab".to_string(),
                            description: "Switch between categories and options".to_string(),
                            category: KeyBindingCategory::Navigation,
                        },
                    ],
                },
                HelpSection {
                    title: "Account Management".to_string(),
                    description: Some("Manage email accounts and authentication".to_string()),
                    shortcuts: vec![
                        KeyBinding {
                            keys: "a".to_string(),
                            description: "Add new account".to_string(),
                            category: KeyBindingCategory::Actions,
                        },
                        KeyBinding {
                            keys: "e".to_string(),
                            description: "Edit selected account".to_string(),
                            category: KeyBindingCategory::Actions,
                        },
                        KeyBinding {
                            keys: "d".to_string(),
                            description: "Delete selected account".to_string(),
                            category: KeyBindingCategory::Actions,
                        },
                        KeyBinding {
                            keys: "t".to_string(),
                            description: "Test account connection".to_string(),
                            category: KeyBindingCategory::Actions,
                        },
                    ],
                },
                HelpSection {
                    title: "Preferences".to_string(),
                    description: Some("Customize appearance and behavior".to_string()),
                    shortcuts: vec![
                        KeyBinding {
                            keys: "Space".to_string(),
                            description: "Toggle boolean settings".to_string(),
                            category: KeyBindingCategory::Actions,
                        },
                        KeyBinding {
                            keys: "Enter".to_string(),
                            description: "Edit text/number settings".to_string(),
                            category: KeyBindingCategory::Actions,
                        },
                        KeyBinding {
                            keys: "r".to_string(),
                            description: "Reset to default value".to_string(),
                            category: KeyBindingCategory::Actions,
                        },
                    ],
                },
            ],
            global_shortcuts: Self::get_global_shortcuts(),
        }
    }

    /// Get global keyboard shortcuts that work across all views
    fn get_global_shortcuts() -> Vec<KeyBinding> {
        vec![
            KeyBinding {
                keys: "Ctrl+H, ?".to_string(),
                description: "Show/hide help".to_string(),
                category: KeyBindingCategory::System,
            },
            KeyBinding {
                keys: "Ctrl+Q, q".to_string(),
                description: "Quit application".to_string(),
                category: KeyBindingCategory::System,
            },
            KeyBinding {
                keys: "F1".to_string(),
                description: "Switch to Email view".to_string(),
                category: KeyBindingCategory::View,
            },
            KeyBinding {
                keys: "F2".to_string(),
                description: "Switch to Calendar view".to_string(),
                category: KeyBindingCategory::View,
            },
            KeyBinding {
                keys: "Ctrl+K".to_string(),
                description: "Open Contacts popup".to_string(),
                category: KeyBindingCategory::View,
            },
            KeyBinding {
                keys: "F4".to_string(),
                description: "Switch to Settings view".to_string(),
                category: KeyBindingCategory::View,
            },
            KeyBinding {
                keys: "Ctrl+S".to_string(),
                description: "Sync all accounts".to_string(),
                category: KeyBindingCategory::Actions,
            },
            KeyBinding {
                keys: "Ctrl+R".to_string(),
                description: "Refresh current view".to_string(),
                category: KeyBindingCategory::Actions,
            },
            KeyBinding {
                keys: "Ctrl+D".to_string(),
                description: "Cycle information density".to_string(),
                category: KeyBindingCategory::View,
            },
            KeyBinding {
                keys: "Esc".to_string(),
                description: "Cancel current action/close dialogs".to_string(),
                category: KeyBindingCategory::System,
            },
            
            // Contacts popup shortcuts (when Ctrl+K popup is open)
            KeyBinding {
                keys: "f".to_string(),
                description: "Show All contacts (in contacts popup)".to_string(),
                category: KeyBindingCategory::Actions,
            },
            KeyBinding {
                keys: "s".to_string(),
                description: "Sync contacts (in contacts popup)".to_string(),
                category: KeyBindingCategory::Actions,
            },
            KeyBinding {
                keys: "r".to_string(),
                description: "Switch to Recent contacts (in contacts popup)".to_string(),
                category: KeyBindingCategory::Actions,
            },
            KeyBinding {
                keys: "/".to_string(),
                description: "Start search (in contacts popup)".to_string(),
                category: KeyBindingCategory::Actions,
            },
            KeyBinding {
                keys: "Tab".to_string(),
                description: "Toggle contact details (in contacts popup)".to_string(),
                category: KeyBindingCategory::View,
            },
        ]
    }
}

impl Default for HelpOverlay {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_help_overlay_creation() {
        let help = HelpOverlay::new();
        assert!(!help.is_visible());
        assert_eq!(help.current_view, ViewMode::Email);
    }

    #[test]
    fn test_help_overlay_toggle() {
        let mut help = HelpOverlay::new();
        
        help.toggle(ViewMode::Calendar);
        assert!(help.is_visible());
        assert_eq!(help.current_view, ViewMode::Calendar);
        
        help.toggle(ViewMode::Calendar);
        assert!(!help.is_visible());
    }

    #[test]
    fn test_help_content_creation() {
        let email_help = HelpOverlay::create_email_help();
        assert_eq!(email_help.title, "Email Client");
        assert!(!email_help.sections.is_empty());
        assert!(!email_help.global_shortcuts.is_empty());

        let calendar_help = HelpOverlay::create_calendar_help();
        assert_eq!(calendar_help.title, "Calendar");
        assert!(!calendar_help.sections.is_empty());
    }

    #[test]
    fn test_global_shortcuts() {
        let global_shortcuts = HelpOverlay::get_global_shortcuts();
        assert!(!global_shortcuts.is_empty());
        
        // Check that help shortcut exists
        let help_shortcut = global_shortcuts.iter()
            .find(|s| s.keys.contains("Ctrl+H"));
        assert!(help_shortcut.is_some());
    }
}