use crate::theme::Theme;
use crate::ui::UIMode;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

/// Context-aware keyboard shortcuts popup that shows shortcuts relevant to the current UI mode
pub struct ContextShortcutsPopup {
    visible: bool,
    scroll_offset: usize,
}

impl ContextShortcutsPopup {
    pub fn new() -> Self {
        Self {
            visible: false,
            scroll_offset: 0,
        }
    }

    /// Show the shortcuts popup
    pub fn show(&mut self) {
        self.visible = true;
        self.scroll_offset = 0;
    }

    /// Hide the shortcuts popup
    pub fn hide(&mut self) {
        self.visible = false;
        self.scroll_offset = 0;
    }

    /// Toggle visibility
    pub fn toggle(&mut self) {
        if self.visible {
            self.hide();
        } else {
            self.show();
        }
    }

    /// Check if the popup is visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Scroll up in the shortcuts list
    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    /// Scroll down in the shortcuts list
    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(1);
    }

    /// Render the context-aware shortcuts popup
    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme, current_mode: &UIMode) {
        if !self.visible {
            return;
        }

        // Clear the background
        frame.render_widget(Clear, area);

        // Create the main popup area (centered, taking 70% of screen)
        let popup_area = self.center_popup(area, 70, 80);

        // Create the main block
        let title = format!("Keyboard Shortcuts - {} Mode", self.mode_display_name(current_mode));
        let main_block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(theme.get_component_style("border", true));

        // Get inner area before rendering
        let inner_area = main_block.inner(popup_area);
        frame.render_widget(main_block, popup_area);

        // Split into sections
        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Description
                Constraint::Min(1),    // Shortcuts list
                Constraint::Length(3), // Help text
            ])
            .split(inner_area);

        // Description
        let description = self.get_mode_description(current_mode);
        let description_widget = Paragraph::new(description)
            .style(theme.get_component_style("text", false))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::BOTTOM));
        frame.render_widget(description_widget, sections[0]);

        // Get shortcuts for current mode
        let shortcuts = self.get_shortcuts_for_mode(current_mode);

        // Create list items
        let mut items = Vec::new();
        for (category, category_shortcuts) in shortcuts {
            // Category header
            items.push(ListItem::new(Line::from(vec![Span::styled(
                format!(" {} ", category),
                theme
                    .get_component_style("border", true)
                    .add_modifier(Modifier::BOLD),
            )])));

            // Shortcuts in this category
            for (shortcut, description) in category_shortcuts {
                items.push(ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("  {:15}", shortcut),
                        theme
                            .get_component_style("button", true)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" - ", theme.get_component_style("text", false)),
                    Span::styled(description, theme.get_component_style("text", false)),
                ])));
            }

            // Add spacing between categories
            items.push(ListItem::new(Line::from("")));
        }

        // Calculate visible items
        let total_items = items.len();
        let max_visible = sections[1].height as usize - 2; // Account for borders

        // Adjust scroll if needed
        if self.scroll_offset >= total_items.saturating_sub(max_visible) {
            self.scroll_offset = total_items.saturating_sub(max_visible);
        }

        // Create the list
        let shortcuts_list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Shortcuts"))
            .style(theme.get_component_style("text", false))
            .highlight_style(
                theme
                    .get_component_style("text", true)
                    .add_modifier(Modifier::BOLD),
            );

        frame.render_widget(shortcuts_list, sections[1]);

        // Help text
        let help_text = "Use ↑/↓ or j/k to scroll, ? or Esc to close";
        let help_widget = Paragraph::new(help_text)
            .style(theme.get_component_style("status_bar", false))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::TOP));
        frame.render_widget(help_widget, sections[2]);
    }

    /// Get the display name for a UI mode
    fn mode_display_name(&self, mode: &UIMode) -> &'static str {
        match mode {
            UIMode::Normal => "Email",
            UIMode::Compose => "Compose",
            UIMode::DraftList => "Drafts",
            UIMode::Calendar => "Calendar",
            UIMode::ContextAware => "Context-Aware",
            UIMode::EventCreate => "Create Event",
            UIMode::EventEdit => "Edit Event",
            UIMode::EventView => "View Event",
            UIMode::EmailViewer => "Email Viewer",
            UIMode::InvitationViewer => "Meeting Invitation",
            UIMode::Search => "Search",
            UIMode::KeyboardShortcuts => "Help",
            UIMode::ContactsPopup => "Contacts",
        }
    }

    /// Get the description for a UI mode
    fn get_mode_description(&self, mode: &UIMode) -> &'static str {
        match mode {
            UIMode::Normal => "Navigate between folders, messages, and content preview",
            UIMode::Compose => "Compose and send new emails",
            UIMode::DraftList => "Manage saved email drafts",
            UIMode::Calendar => "View and manage calendar events",
            UIMode::ContextAware => "Integrated email and calendar view with smart context",
            UIMode::EventCreate => "Create new calendar events",
            UIMode::EventEdit => "Edit existing calendar events",
            UIMode::EventView => "View calendar event details",
            UIMode::EmailViewer => "Read emails in full-screen mode",
            UIMode::InvitationViewer => "Handle meeting invitations",
            UIMode::Search => "Search through emails and calendar events",
            UIMode::KeyboardShortcuts => "View all available keyboard shortcuts",
            UIMode::ContactsPopup => "Browse and manage contacts",
        }
    }

    /// Get shortcuts organized by category for the current mode
    fn get_shortcuts_for_mode(&self, mode: &UIMode) -> Vec<(&'static str, Vec<(&'static str, &'static str)>)> {
        match mode {
            UIMode::Normal => vec![
                ("AI Assistant", vec![
                    ("Ctrl+Alt+I", "Toggle AI assistant panel"),
                    ("Ctrl+Alt+S", "AI email suggestions"),
                    ("Ctrl+Alt+U", "Summarize email with AI"),
                    ("Ctrl+Alt+C", "AI compose assistance"),
                    ("Ctrl+Alt+G", "AI configuration"),
                ]),
                ("Navigation", vec![
                    ("Tab", "Switch between panes"),
                    ("Shift+Tab", "Switch panes backward"),
                    ("j/k", "Navigate up/down in lists"),
                    ("h/l", "Collapse/expand folders"),
                    ("Enter", "Select item or open email"),
                    ("Esc", "Go back or cancel"),
                ]),
                ("Email Actions", vec![
                    ("c", "Compose new email"),
                    ("Ctrl+R", "Reply to email"),
                    ("Shift+R", "Reply to all"),
                    ("Ctrl+F", "Forward email"),
                    ("Shift+Delete", "Delete email"),
                    ("Shift+U", "Mark as unread"),
                    ("Shift+A", "Archive email"),
                ]),
                ("View Modes", vec![
                    ("C", "Switch to calendar view"),
                    ("E", "Switch to email view"),
                    ("m", "Toggle email view mode"),
                    ("H", "Toggle headers"),
                    ("/", "Search"),
                    ("?", "Show this help"),
                ]),
                ("System", vec![
                    ("F", "Force refresh folder"),
                    ("Ctrl+A", "Add account"),
                    ("q", "Quit application"),
                ]),
            ],
            
            UIMode::Compose => vec![
                ("AI Assistant", vec![
                    ("Ctrl+Alt+C", "AI compose assistance"),
                    ("Ctrl+Alt+R", "Generate quick reply suggestions"),
                    ("Ctrl+Alt+E", "Generate email content with AI"),
                ]),
                ("Navigation", vec![
                    ("Tab", "Next field"),
                    ("Shift+Tab", "Previous field"),
                    ("Enter", "Edit current field"),
                    ("Esc", "Cancel composition"),
                ]),
                ("Actions", vec![
                    ("Ctrl+S", "Send email"),
                    ("Ctrl+D", "Save as draft"),
                    ("@", "Contact lookup"),
                ]),
                ("Spell Checking", vec![
                    ("Ctrl+Z", "Toggle spell checking"),
                    ("Ctrl+N", "Next spelling error"),
                    ("Ctrl+P", "Previous spelling error"),
                    ("Ctrl+,", "Spell check configuration"),
                ]),
                ("Formatting", vec![
                    ("Ctrl+A", "Select all"),
                    ("Ctrl+C", "Copy"),
                    ("Ctrl+V", "Paste"),
                ]),
            ],

            UIMode::Calendar => vec![
                ("AI Assistant", vec![
                    ("Ctrl+Alt+I", "Toggle AI assistant panel"),
                    ("Ctrl+Alt+L", "AI calendar assistance"),
                    ("Ctrl+Alt+T", "Parse scheduling requests with AI"),
                ]),
                ("Navigation", vec![
                    ("←→", "Previous/Next month"),
                    ("↑↓", "Navigate weeks"),
                    ("j/k", "Navigate days"),
                    (".", "Go to today"),
                    ("Enter", "View event details"),
                ]),
                ("View Modes", vec![
                    ("1", "Day view"),
                    ("2", "Week view"),
                    ("3", "Month view"),
                    ("4", "Agenda view"),
                ]),
                ("Event Actions", vec![
                    ("e", "Create new event"),
                    ("E", "Create recurring event"),
                    ("Ctrl+e", "Edit selected event"),
                    ("Del", "Delete event"),
                    ("Space", "Toggle event completion"),
                ]),
                ("Calendar Management", vec![
                    ("c", "Show calendar list"),
                    ("t", "Create todo/task"),
                    ("T", "View todos"),
                    ("Esc", "Return to email view"),
                ]),
            ],

            UIMode::Search => vec![
                ("Search", vec![
                    ("Type", "Enter search query"),
                    ("Enter", "Execute search"),
                    ("↑↓/j/k", "Navigate results"),
                    ("Enter", "Open selected result"),
                ]),
                ("Search Modes", vec![
                    ("Tab", "Cycle search modes"),
                    ("F1", "Search all"),
                    ("F2", "Search subject"),
                    ("F3", "Search sender"),
                    ("F4", "Search content"),
                ]),
                ("Actions", vec![
                    ("Esc", "Close search"),
                    ("Ctrl+C", "Clear search"),
                    ("/", "Focus search box"),
                ]),
            ],

            UIMode::EmailViewer => vec![
                ("AI Assistant", vec![
                    ("Ctrl+Alt+U", "Summarize email with AI"),
                    ("Ctrl+Alt+A", "Analyze email content with AI"),
                    ("Ctrl+Alt+R", "Generate quick reply suggestions"),
                ]),
                ("Navigation", vec![
                    ("j/k", "Scroll up/down"),
                    ("↑↓", "Scroll line by line"),
                    ("Page Up/Down", "Scroll page"),
                    ("Home/End", "Go to top/bottom"),
                ]),
                ("Actions", vec![
                    ("Ctrl+R", "Reply"),
                    ("Shift+R", "Reply all"),
                    ("Ctrl+F", "Forward"),
                    ("c", "Add sender to contacts"),
                    ("Space", "Show actions menu"),
                ]),
                ("View Options", vec![
                    ("m", "Toggle view mode"),
                    ("H", "Show/hide headers"),
                    ("a", "View attachments"),
                    ("v", "View selected attachment"),
                ]),
                ("Exit", vec![
                    ("q", "Close viewer"),
                    ("Esc", "Close viewer"),
                ]),
            ],

            UIMode::ContextAware => vec![
                ("Navigation", vec![
                    ("Tab", "Switch between panes"),
                    ("j/k", "Navigate items"),
                    ("Enter", "Select/Open"),
                    ("Esc", "Hide context panel"),
                ]),
                ("Context Actions", vec![
                    ("c", "Create event from email"),
                    ("s", "Schedule meeting"),
                    ("a", "Accept invitation"),
                    ("d", "Decline invitation"),
                    ("t", "Tentative response"),
                ]),
                ("Email Actions", vec![
                    ("r", "Reply to email"),
                    ("f", "Forward email"),
                    ("v", "View details"),
                ]),
            ],

            UIMode::InvitationViewer => vec![
                ("Navigation", vec![
                    ("j/k", "Navigate options"),
                    ("Enter", "Select action"),
                    ("Tab", "Switch sections"),
                ]),
                ("Response Actions", vec![
                    ("a", "Accept invitation"),
                    ("d", "Decline invitation"),
                    ("t", "Tentative response"),
                    ("r", "Reply with message"),
                ]),
                ("View Options", vec![
                    ("v", "View full details"),
                    ("c", "Add to calendar"),
                    ("e", "Edit before accepting"),
                ]),
                ("Navigation", vec![
                    ("q", "Close viewer"),
                    ("Esc", "Close viewer"),
                ]),
            ],

            _ => vec![
                ("General", vec![
                    ("?", "Show help"),
                    ("Esc", "Go back"),
                    ("Ctrl+Q", "Quit"),
                ]),
            ],
        }
    }

    /// Create a centered popup area
    fn center_popup(&self, area: Rect, percent_x: u16, percent_y: u16) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(area);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }
}

impl Default for ContextShortcutsPopup {
    fn default() -> Self {
        Self::new()
    }
}