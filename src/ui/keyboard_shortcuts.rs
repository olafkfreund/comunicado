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
            ("AI Assistant", Vec::new()),
            ("Navigation", Vec::new()),
            ("Selection & Interaction", Vec::new()),
            ("Email Actions", Vec::new()),
            ("Calendar & Events", Vec::new()),
            ("View & Attachments", Vec::new()),
            ("Sorting & Search", Vec::new()),
            ("Account Management", Vec::new()),
            ("Folder Management", Vec::new()),
            ("Vim-style Movement", Vec::new()),
        ];

        // Get all shortcuts and organize them
        for (shortcut, action) in keyboard_manager.get_all_shortcuts() {
            let category_index = match action {
                // Global Actions (0)
                KeyboardAction::Quit
                | KeyboardAction::ForceQuit
                | KeyboardAction::ShowKeyboardShortcuts => 0,
                
                // AI Assistant (1)
                KeyboardAction::AIToggleAssistant
                | KeyboardAction::AIEmailSuggestions
                | KeyboardAction::AIComposeSuggestions
                | KeyboardAction::AISummarizeEmail
                | KeyboardAction::AICalendarAssist
                | KeyboardAction::AIConfigureSettings
                | KeyboardAction::AIQuickReply
                | KeyboardAction::AIEmailAnalysis
                | KeyboardAction::AIScheduleRequest
                | KeyboardAction::AIContentGeneration => 1,
                
                // Navigation (2)
                KeyboardAction::NextPane
                | KeyboardAction::PreviousPane
                | KeyboardAction::MoveUp
                | KeyboardAction::MoveDown
                | KeyboardAction::MoveLeft
                | KeyboardAction::MoveRight
                | KeyboardAction::NextMessage
                | KeyboardAction::PreviousMessage => 2,
                
                // Selection & Interaction (3)
                KeyboardAction::Select
                | KeyboardAction::Escape
                | KeyboardAction::ToggleExpanded => 3,
                
                // Email Actions (4)
                KeyboardAction::ComposeEmail
                | KeyboardAction::ReplyEmail
                | KeyboardAction::ReplyAllEmail
                | KeyboardAction::ForwardEmail
                | KeyboardAction::DeleteEmail
                | KeyboardAction::MarkAsRead
                | KeyboardAction::MarkAsUnread
                | KeyboardAction::ArchiveEmail
                | KeyboardAction::ShowDraftList => 4,
                
                // Calendar & Events (5)
                KeyboardAction::ShowCalendar
                | KeyboardAction::ShowEmail
                | KeyboardAction::CreateEvent
                | KeyboardAction::EditEvent
                | KeyboardAction::DeleteEvent
                | KeyboardAction::ViewEventDetails
                | KeyboardAction::CreateTodo
                | KeyboardAction::ToggleTodoComplete
                | KeyboardAction::ViewTodos
                | KeyboardAction::CalendarNextMonth
                | KeyboardAction::CalendarPrevMonth
                | KeyboardAction::CalendarToday
                | KeyboardAction::CalendarWeekView
                | KeyboardAction::CalendarMonthView
                | KeyboardAction::CalendarDayView
                | KeyboardAction::CalendarAgendaView => 5,
                
                // View & Attachments (6)
                KeyboardAction::OpenEmailViewer
                | KeyboardAction::ViewAttachment
                | KeyboardAction::SelectFirstAttachment
                | KeyboardAction::OpenAttachmentWithSystem
                | KeyboardAction::ToggleViewMode
                | KeyboardAction::ToggleHeaders
                | KeyboardAction::ScrollToTop
                | KeyboardAction::ScrollToBottom
                | KeyboardAction::ToggleThreadedView
                | KeyboardAction::ExpandThread
                | KeyboardAction::CollapseThread => 6,
                
                // Sorting & Search (7)
                KeyboardAction::SortByDate
                | KeyboardAction::SortBySender
                | KeyboardAction::SortBySubject
                | KeyboardAction::StartSearch
                | KeyboardAction::StartFolderSearch
                | KeyboardAction::EndSearch => 7,
                
                // Account Management (8)
                KeyboardAction::AddAccount
                | KeyboardAction::RemoveAccount
                | KeyboardAction::RefreshAccount
                | KeyboardAction::SwitchAccount => 8,
                
                // Folder Management (9)
                KeyboardAction::RefreshFolder
                | KeyboardAction::CreateFolder
                | KeyboardAction::DeleteFolder => 9,
                
                // Vim-style Movement (10)
                KeyboardAction::VimMoveLeft
                | KeyboardAction::VimMoveRight
                | KeyboardAction::VimMoveUp
                | KeyboardAction::VimMoveDown => 10,
                
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
            KeyboardAction::ReplyEmail => "Reply to email (message list/preview)",
            KeyboardAction::ReplyAllEmail => "Reply all to email (message list/preview)",
            KeyboardAction::ForwardEmail => "Forward email (message list/preview)",
            KeyboardAction::DeleteEmail => "Delete email (message list/preview)",
            KeyboardAction::MarkAsRead => "Mark as read (message list/preview)",
            KeyboardAction::MarkAsUnread => "Mark as unread (message list/preview)",
            KeyboardAction::ShowDraftList => "Show draft list",
            KeyboardAction::ArchiveEmail => "Archive email (message list/preview)",
            KeyboardAction::AddAccount => "Add new account",
            KeyboardAction::RemoveAccount => "Remove account (account switcher)",
            KeyboardAction::RefreshAccount => "Refresh account (account switcher)",
            KeyboardAction::SwitchAccount => "Switch account",
            KeyboardAction::StartSearch => "Start search (message list)",
            KeyboardAction::StartFolderSearch => "Search folders (folder tree)",
            KeyboardAction::EndSearch => "End search",
            KeyboardAction::ToggleThreadedView => "Toggle threaded view (message list)",
            KeyboardAction::ExpandThread => "Expand thread (message list)",
            KeyboardAction::CollapseThread => "Collapse thread (message list)",
            KeyboardAction::OpenEmailViewer => "Open email in full-screen viewer",
            KeyboardAction::ViewAttachment => "View selected attachment (content preview)",
            KeyboardAction::SelectFirstAttachment => "Select first attachment (content preview)",
            KeyboardAction::OpenAttachmentWithSystem => "Open attachment with system app (content preview)",
            KeyboardAction::ToggleViewMode => "Toggle view mode (content preview)",
            KeyboardAction::ToggleHeaders => "Toggle extended headers (content preview)",
            KeyboardAction::ScrollToTop => "Scroll to top (content preview)",
            KeyboardAction::ScrollToBottom => "Scroll to bottom (content preview)",
            KeyboardAction::SortByDate => "Sort by date (message list)",
            KeyboardAction::SortBySender => "Sort by sender (message list)",
            KeyboardAction::SortBySubject => "Sort by subject (message list)",
            KeyboardAction::NextMessage => "Next message (message list/preview)",
            KeyboardAction::PreviousMessage => "Previous message (message list/preview)",
            KeyboardAction::RefreshFolder => "Refresh current folder (folder tree)",
            KeyboardAction::CreateFolder => "Create new folder (folder tree)",
            KeyboardAction::DeleteFolder => "Delete folder (folder tree)",
            KeyboardAction::ViewTodos => "View todos (calendar mode)",
            KeyboardAction::CreateTodo => "Create new todo (calendar mode)",
            KeyboardAction::ToggleTodoComplete => "Toggle todo complete (calendar mode)",
            
            // AI Assistant actions
            KeyboardAction::AIToggleAssistant => "Toggle AI assistant panel",
            KeyboardAction::AIEmailSuggestions => "Get AI suggestions for current email (message list/preview)",
            KeyboardAction::AIComposeSuggestions => "AI assistance for email composition (compose mode)",
            KeyboardAction::AISummarizeEmail => "Generate AI summary of current email (message list/preview)",
            KeyboardAction::AICalendarAssist => "AI calendar assistance (calendar mode)",
            KeyboardAction::AIConfigureSettings => "Open AI configuration and settings",
            KeyboardAction::AIQuickReply => "Generate quick reply suggestions (message list/preview)",
            KeyboardAction::AIEmailAnalysis => "Analyze email content with AI (message list/preview)",
            KeyboardAction::AIScheduleRequest => "Parse scheduling requests with AI (message list/preview)",
            KeyboardAction::AIContentGeneration => "Generate email content with AI (compose mode)",
            
            // Calendar and Event actions
            KeyboardAction::ShowCalendar => "Switch to calendar view",
            KeyboardAction::ShowEmail => "Switch to email view",
            KeyboardAction::CreateEvent => "Create new calendar event",
            KeyboardAction::EditEvent => "Edit selected event",
            KeyboardAction::DeleteEvent => "Delete selected event",
            KeyboardAction::ViewEventDetails => "View event details",
            KeyboardAction::CalendarNextMonth => "Next month in calendar",
            KeyboardAction::CalendarPrevMonth => "Previous month in calendar",
            KeyboardAction::CalendarToday => "Jump to today in calendar",
            KeyboardAction::CalendarWeekView => "Switch to week view",
            KeyboardAction::CalendarMonthView => "Switch to month view",
            KeyboardAction::CalendarDayView => "Switch to day view",
            KeyboardAction::CalendarAgendaView => "Switch to agenda view",
            
            _ => "Unknown action",
        }
    }
}

impl Default for KeyboardShortcutsUI {
    fn default() -> Self {
        Self::new()
    }
}
