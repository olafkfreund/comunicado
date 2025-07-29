use crate::keyboard::{KeyboardAction, KeyboardManager, KeyboardShortcut};
use crate::theme::Theme;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

pub struct KeyboardShortcutsUI {
    scroll_offset: usize,
}

impl KeyboardShortcutsUI {
    pub fn new() -> Self {
        Self { scroll_offset: 0 }
    }

    pub fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        theme: &Theme,
        keyboard_manager: &KeyboardManager,
    ) {
        // Clear the background
        frame.render_widget(Clear, area);

        // Create the main popup area
        let popup_area = self.center_popup(area, 80, 80);

        // Create the main block
        let main_block = Block::default()
            .title("Keyboard Shortcuts (Press ? to close)")
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
        let description = Paragraph::new("All available keyboard shortcuts:")
            .style(theme.get_component_style("text", false))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::BOTTOM));
        frame.render_widget(description, sections[0]);

        // Get shortcuts organized by category
        let shortcuts = self.organize_shortcuts(keyboard_manager);

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
            for (shortcut, action) in category_shortcuts {
                let shortcut_display = shortcut.to_string();
                let action_description = self.get_action_description(&action);

                items.push(ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("  {:15}", shortcut_display),
                        theme
                            .get_component_style("button", true)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" - ", theme.get_component_style("text", false)),
                    Span::styled(action_description, theme.get_component_style("text", false)),
                ])));
            }

            // Add spacing between categories
            items.push(ListItem::new(Line::from("")));
        }

        // Calculate visible items
        let total_items = items.len();
        let max_visible = sections[1].height as usize - 2; // Account for borders

        // Adjust scroll if needed
        if self.scroll_offset + max_visible > total_items {
            self.scroll_offset = total_items.saturating_sub(max_visible);
        }

        let visible_items = items
            .into_iter()
            .skip(self.scroll_offset)
            .take(max_visible)
            .collect::<Vec<_>>();

        // Render the shortcuts list
        let list = List::new(visible_items)
            .block(Block::default().borders(Borders::ALL).title("Shortcuts"))
            .style(theme.get_component_style("text", false));

        frame.render_widget(list, sections[1]);

        // Help text
        let help_text = Paragraph::new("Use ↑/↓ to scroll, ? or Esc to close")
            .style(theme.get_component_style("status_bar", false))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::TOP));
        frame.render_widget(help_text, sections[2]);
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(1);
    }

    fn center_popup(&self, area: Rect, width_percent: u16, height_percent: u16) -> Rect {
        let popup_width = (area.width * width_percent) / 100;
        let popup_height = (area.height * height_percent) / 100;

        let x = (area.width.saturating_sub(popup_width)) / 2;
        let y = (area.height.saturating_sub(popup_height)) / 2;

        Rect {
            x: area.x + x,
            y: area.y + y,
            width: popup_width,
            height: popup_height,
        }
    }

    fn organize_shortcuts(
        &self,
        keyboard_manager: &KeyboardManager,
    ) -> Vec<(&str, Vec<(KeyboardShortcut, KeyboardAction)>)> {
        let mut categories: Vec<(&str, Vec<(KeyboardShortcut, KeyboardAction)>)> = vec![
            ("Global Actions", Vec::new()),
            ("Navigation", Vec::new()),
            ("Selection & Interaction", Vec::new()),
            ("Pane Management", Vec::new()),
            ("Email Actions", Vec::new()),
            ("Account Management", Vec::new()),
            ("Calendar", Vec::new()),
            ("Folder Management", Vec::new()),
            ("Vim-style Movement", Vec::new()),
        ];

        // Get all shortcuts and organize them
        for (shortcut, action) in keyboard_manager.get_all_shortcuts() {
            let category_index = match action {
                KeyboardAction::Quit
                | KeyboardAction::ForceQuit
                | KeyboardAction::ShowStartPage
                | KeyboardAction::ShowKeyboardShortcuts => 0,
                KeyboardAction::NextPane
                | KeyboardAction::PreviousPane
                | KeyboardAction::MoveUp
                | KeyboardAction::MoveDown
                | KeyboardAction::MoveLeft
                | KeyboardAction::MoveRight => 1,
                KeyboardAction::Select
                | KeyboardAction::Escape
                | KeyboardAction::ToggleExpanded => 2,
                KeyboardAction::VimMoveLeft
                | KeyboardAction::VimMoveRight
                | KeyboardAction::VimMoveUp
                | KeyboardAction::VimMoveDown => 8,
                KeyboardAction::ComposeEmail
                | KeyboardAction::ReplyEmail
                | KeyboardAction::ReplyAllEmail
                | KeyboardAction::ForwardEmail
                | KeyboardAction::DeleteEmail
                | KeyboardAction::MarkAsRead
                | KeyboardAction::MarkAsUnread => 4,
                KeyboardAction::AddAccount
                | KeyboardAction::RemoveAccount
                | KeyboardAction::RefreshAccount
                | KeyboardAction::SwitchAccount => 5,
                KeyboardAction::StartSearch
                | KeyboardAction::StartFolderSearch
                | KeyboardAction::EndSearch => 7,
                _ => 0, // Default to global actions
            };

            categories[category_index]
                .1
                .push((shortcut.clone(), action.clone()));
        }

        // Filter out empty categories
        categories
            .into_iter()
            .filter(|(_, shortcuts)| !shortcuts.is_empty())
            .collect()
    }

    fn get_action_description(&self, action: &KeyboardAction) -> &'static str {
        match action {
            KeyboardAction::Quit => "Quit application",
            KeyboardAction::ForceQuit => "Force quit application",
            KeyboardAction::ShowStartPage => "Show start page",
            KeyboardAction::ShowKeyboardShortcuts => "Show this shortcuts dialog",
            KeyboardAction::NextPane => "Move to next pane",
            KeyboardAction::PreviousPane => "Move to previous pane",
            KeyboardAction::VimMoveLeft => "Move cursor left (vim-style)",
            KeyboardAction::VimMoveRight => "Move cursor right (vim-style)",
            KeyboardAction::VimMoveUp => "Move cursor up (vim-style)",
            KeyboardAction::VimMoveDown => "Move cursor down (vim-style)",
            KeyboardAction::MoveUp => "Move up",
            KeyboardAction::MoveDown => "Move down",
            KeyboardAction::MoveLeft => "Move left",
            KeyboardAction::MoveRight => "Move right",
            KeyboardAction::Select => "Select current item",
            KeyboardAction::Escape => "Cancel or go back",
            KeyboardAction::ToggleExpanded => "Toggle expanded/collapsed",
            KeyboardAction::ComposeEmail => "Compose new email",
            KeyboardAction::ReplyEmail => "Reply to email",
            KeyboardAction::ReplyAllEmail => "Reply all to email",
            KeyboardAction::ForwardEmail => "Forward email",
            KeyboardAction::DeleteEmail => "Delete email",
            KeyboardAction::MarkAsRead => "Mark as read",
            KeyboardAction::MarkAsUnread => "Mark as unread",
            KeyboardAction::ShowDraftList => "Show draft list",
            KeyboardAction::ArchiveEmail => "Archive email",
            KeyboardAction::AddAccount => "Add new account",
            KeyboardAction::RemoveAccount => "Remove account",
            KeyboardAction::RefreshAccount => "Refresh account",
            KeyboardAction::SwitchAccount => "Switch account",
            KeyboardAction::StartSearch => "Start search",
            KeyboardAction::StartFolderSearch => "Search folders",
            KeyboardAction::EndSearch => "End search",
            KeyboardAction::ToggleThreadedView => "Toggle threaded view",
            KeyboardAction::ExpandThread => "Expand thread",
            KeyboardAction::CollapseThread => "Collapse thread",
            _ => "Unknown action",
        }
    }
}

impl Default for KeyboardShortcutsUI {
    fn default() -> Self {
        Self::new()
    }
}
