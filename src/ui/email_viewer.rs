use chrono::{DateTime, Local};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::email::StoredMessage;
use crate::theme::Theme;
use crate::ui::content_preview::{EmailContent, EmailHeader, ViewMode};

/// Email viewer actions
#[derive(Debug, Clone, PartialEq)]
pub enum EmailViewerAction {
    Reply,
    ReplyAll,
    Forward,
    Edit,
    Delete,
    Archive,
    MarkAsRead,
    MarkAsUnread,
    Close,
}

/// Full-screen email viewer with action buttons
pub struct EmailViewer {
    email_content: Option<EmailContent>,
    pub current_message: Option<StoredMessage>,
    view_mode: ViewMode,
    scroll_position: usize,
    show_raw_headers: bool,
    show_actions: bool,
    selected_action: usize,
    actions: Vec<EmailViewerAction>,
}

impl EmailViewer {
    pub fn new() -> Self {
        Self {
            email_content: None,
            current_message: None,
            view_mode: ViewMode::Formatted,
            scroll_position: 0,
            show_raw_headers: false,
            show_actions: false,
            selected_action: 0,
            actions: vec![
                EmailViewerAction::Reply,
                EmailViewerAction::ReplyAll,
                EmailViewerAction::Forward,
                EmailViewerAction::Edit,
                EmailViewerAction::Delete,
                EmailViewerAction::Archive,
                EmailViewerAction::MarkAsRead,
                EmailViewerAction::Close,
            ],
        }
    }

    /// Set email content to display
    pub fn set_email(&mut self, message: StoredMessage, email_content: EmailContent) {
        self.current_message = Some(message);
        self.email_content = Some(email_content);
        self.scroll_position = 0;
        self.show_actions = false;
        self.selected_action = 0;
    }

    /// Toggle view mode
    pub fn toggle_view_mode(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::Formatted => ViewMode::Raw,
            ViewMode::Raw => ViewMode::Html,
            ViewMode::Html => ViewMode::Headers,
            ViewMode::Headers => ViewMode::Formatted,
        };
    }

    /// Toggle raw headers display
    pub fn toggle_headers(&mut self) {
        self.show_raw_headers = !self.show_raw_headers;
    }

    /// Toggle actions panel
    pub fn toggle_actions(&mut self) {
        self.show_actions = !self.show_actions;
    }

    /// Scroll up with bounds checking
    pub fn scroll_up(&mut self, lines: usize) {
        self.scroll_position = self.scroll_position.saturating_sub(lines);
    }

    /// Scroll down with bounds checking (bounds will be applied in render)
    pub fn scroll_down(&mut self, lines: usize) {
        self.scroll_position = self.scroll_position.saturating_add(lines);
    }

    /// Scroll to top of content
    pub fn scroll_to_top(&mut self) {
        self.scroll_position = 0;
    }

    /// Scroll to bottom of content (will be clamped in render)
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_position = usize::MAX;
    }

    /// Get current scroll information for display
    pub fn get_scroll_info(&self, total_lines: usize, visible_height: usize) -> (usize, usize, bool, bool) {
        let max_scroll = total_lines.saturating_sub(visible_height);
        let current_scroll = self.scroll_position.min(max_scroll);
        let can_scroll_up = current_scroll > 0;
        let can_scroll_down = current_scroll < max_scroll;
        (current_scroll, max_scroll, can_scroll_up, can_scroll_down)
    }

    /// Select next action
    pub fn next_action(&mut self) {
        if self.show_actions {
            self.selected_action = (self.selected_action + 1) % self.actions.len();
        }
    }

    /// Select previous action
    pub fn previous_action(&mut self) {
        if self.show_actions {
            self.selected_action = if self.selected_action == 0 {
                self.actions.len() - 1
            } else {
                self.selected_action - 1
            };
        }
    }

    /// Get current selected action
    pub fn get_selected_action(&self) -> Option<EmailViewerAction> {
        if self.show_actions {
            self.actions.get(self.selected_action).cloned()
        } else {
            None
        }
    }

    /// Set the viewport height for proper page scrolling
    pub fn set_viewport_height(&mut self, height: usize) {
        // Store viewport height for page scrolling calculations
        // This will be called from the render method
    }

    /// Handle key input
    pub fn handle_key(&mut self, key: crossterm::event::KeyCode) -> Option<EmailViewerAction> {
        self.handle_key_with_viewport(key, 20) // Default viewport height
    }

    /// Handle key input with viewport height for proper page scrolling
    pub fn handle_key_with_viewport(&mut self, key: crossterm::event::KeyCode, viewport_height: usize) -> Option<EmailViewerAction> {
        use crossterm::event::KeyCode;

        match key {
            KeyCode::Esc => {
                if self.show_actions {
                    self.show_actions = false;
                    None
                } else {
                    Some(EmailViewerAction::Close)
                }
            }
            KeyCode::Char('q') => Some(EmailViewerAction::Close),
            KeyCode::Char('r') => Some(EmailViewerAction::Reply),
            KeyCode::Char('R') => Some(EmailViewerAction::ReplyAll),
            KeyCode::Char('f') => Some(EmailViewerAction::Forward),
            KeyCode::Char('e') => Some(EmailViewerAction::Edit),
            KeyCode::Char('d') => Some(EmailViewerAction::Delete),
            KeyCode::Char('a') => Some(EmailViewerAction::Archive),
            KeyCode::Char('m') => Some(EmailViewerAction::MarkAsRead),
            KeyCode::Char('u') => Some(EmailViewerAction::MarkAsUnread),
            KeyCode::Char('v') => {
                self.toggle_view_mode();
                None
            }
            KeyCode::Char('h') => {
                self.toggle_headers();
                None
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                if self.show_actions {
                    self.get_selected_action()
                } else {
                    self.toggle_actions();
                    None
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.show_actions {
                    self.previous_action();
                } else {
                    self.scroll_up(1);
                }
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.show_actions {
                    self.next_action();
                } else {
                    self.scroll_down(1);
                }
                None
            }
            KeyCode::PageUp => {
                // Scroll by a full page based on viewport height
                let page_size = viewport_height.saturating_sub(2); // Leave some overlap
                self.scroll_up(page_size);
                None
            }
            KeyCode::PageDown => {
                // Scroll by a full page based on viewport height
                let page_size = viewport_height.saturating_sub(2); // Leave some overlap
                self.scroll_down(page_size);
                None
            }
            KeyCode::Home => {
                self.scroll_to_top();
                None
            }
            KeyCode::End => {
                self.scroll_to_bottom();
                None
            }
            _ => None,
        }
    }

    /// Render the email viewer
    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Clear the background
        frame.render_widget(Clear, area);

        // Create main layout with header, content, and footer
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Header bar
                Constraint::Min(0),     // Email content
                Constraint::Length(3),  // Status/instructions bar
            ])
            .split(area);

        // Render header bar
        self.render_header_bar(frame, chunks[0], theme);

        // Render email content area
        if self.show_actions {
            // Split content area for actions panel
            let content_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(70),
                    Constraint::Percentage(30),
                ])
                .split(chunks[1]);

            self.render_email_content(frame, content_chunks[0], theme);
            self.render_actions_panel(frame, content_chunks[1], theme);
        } else {
            self.render_email_content(frame, chunks[1], theme);
        }

        // Render footer bar
        self.render_footer_bar(frame, chunks[2], theme);
    }

    fn render_header_bar(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let title = if let Some(ref email) = self.email_content {
            format!("📧 {}", email.headers.subject)
        } else {
            "Email Viewer".to_string()
        };

        let view_mode_text = match self.view_mode {
            ViewMode::Formatted => "[Formatted]",
            ViewMode::Raw => "[Raw]",
            ViewMode::Html => "[HTML]",
            ViewMode::Headers => "[Headers]",
        };

        let header_text = format!("{} {}", title, view_mode_text);

        let header = Paragraph::new(header_text)
            .block(Block::default()
                .title("Email Viewer")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.colors.palette.accent)))
            .style(Style::default().fg(theme.colors.palette.accent).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center);

        frame.render_widget(header, area);
    }

    fn render_email_content(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let content_height = area.height.saturating_sub(2) as usize;

        let lines = if let Some(ref email) = self.email_content {
            match self.view_mode {
                ViewMode::Formatted => Self::render_formatted_email_static(email, theme),
                ViewMode::Raw => Self::render_raw_email_static(email, theme),
                ViewMode::Html => Self::render_html_email_static(email, theme),
                ViewMode::Headers => Self::render_headers_email_static(email, theme),
            }
        } else {
            vec![Line::from("No email content available")]
        };

        // Calculate proper scroll bounds
        let max_scroll = if lines.len() > content_height {
            lines.len().saturating_sub(content_height)
        } else {
            0
        };

        // Clamp scroll position to valid bounds
        self.scroll_position = self.scroll_position.min(max_scroll);

        // Apply scrolling
        let start_line = self.scroll_position;
        let end_line = (start_line + content_height).min(lines.len());
        let visible_lines = if end_line > start_line {
            lines[start_line..end_line].to_vec()
        } else if lines.is_empty() {
            vec![Line::from("No content")]
        } else {
            // Show the last available content
            let adjusted_start = lines.len().saturating_sub(content_height);
            let adjusted_end = lines.len();
            lines[adjusted_start..adjusted_end].to_vec()
        };

        // Create scroll indicator in the title
        let scroll_info = if lines.len() > content_height {
            let (current, max, can_up, can_down) = self.get_scroll_info(lines.len(), content_height);
            let percentage = if max > 0 {
                ((current as f64 / max as f64) * 100.0).round() as usize
            } else {
                0
            };
            
            let up_arrow = if can_up { "↑" } else { " " };
            let down_arrow = if can_down { "↓" } else { " " };
            format!(" [{}{} {}%]", up_arrow, down_arrow, percentage)
        } else {
            String::new()
        };

        let title = format!("Email Content{}", scroll_info);
        
        let content_paragraph = Paragraph::new(visible_lines)
            .block(Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.colors.palette.border)))
            .wrap(Wrap { trim: true });

        frame.render_widget(content_paragraph, area);
    }

    fn render_actions_panel(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let mut action_lines = Vec::new();

        action_lines.push(Line::from(vec![
            Span::styled("Actions", Style::default()
                .fg(theme.colors.palette.accent)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED))
        ]));
        action_lines.push(Line::from(""));

        for (i, action) in self.actions.iter().enumerate() {
            let is_selected = i == self.selected_action;
            let prefix = if is_selected { "► " } else { "  " };
            
            let style = if is_selected {
                Style::default().fg(theme.colors.palette.accent).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.colors.content_preview.body)
            };

            let action_text = match action {
                EmailViewerAction::Reply => "Reply",
                EmailViewerAction::ReplyAll => "Reply All",
                EmailViewerAction::Forward => "Forward",
                EmailViewerAction::Edit => "Edit",
                EmailViewerAction::Delete => "Delete",
                EmailViewerAction::Archive => "Archive",
                EmailViewerAction::MarkAsRead => "Mark Read",
                EmailViewerAction::MarkAsUnread => "Mark Unread",
                EmailViewerAction::Close => "Close",
            };

            action_lines.push(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(action_text, style),
            ]));
        }

        let actions_paragraph = Paragraph::new(action_lines)
            .block(Block::default()
                .title("Actions")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.colors.palette.accent)))
            .wrap(Wrap { trim: true });

        frame.render_widget(actions_paragraph, area);
    }

    fn render_footer_bar(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let instructions = if self.show_actions {
            "↑↓: Select Action | Enter: Execute | Esc: Hide Actions | r: Reply | f: Forward | q: Quit"
        } else {
            "j/k/↑↓: Scroll | PgUp/PgDn: Page | Home/End: Top/Bottom | Space: Actions | v: View | q: Quit"
        };

        let footer = Paragraph::new(instructions)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(theme.colors.palette.text_muted))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        frame.render_widget(footer, area);
    }

    fn render_formatted_email_static<'a>(email: &'a EmailContent, theme: &'a Theme) -> Vec<Line<'a>> {
        let mut lines = Vec::new();

        // Modern sender box
        lines.extend(Self::render_sender_box_static(&email.headers, theme));
        lines.push(Line::from(""));

        // Subject
        if !email.headers.subject.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("📧 Subject: ", Style::default().fg(theme.colors.palette.accent).add_modifier(Modifier::BOLD)),
                Span::styled(email.headers.subject.clone(), Style::default().fg(theme.colors.palette.accent)),
            ]));
            lines.push(Line::from(""));
        }

        // Content separator
        lines.push(Line::from(vec![
            Span::styled("─".repeat(80), Style::default().fg(theme.colors.palette.border))
        ]));
        lines.push(Line::from(""));

        // Email body
        for line in email.body.lines() {
            lines.push(Line::from(line.to_string()));
        }

        lines
    }

    fn render_raw_email_static<'a>(email: &'a EmailContent, _theme: &'a Theme) -> Vec<Line<'a>> {
        let mut lines = Vec::new();
        
        // Raw headers
        lines.push(Line::from(format!("From: {}", email.headers.from)));
        lines.push(Line::from(format!("To: {}", email.headers.to.join(", "))));
        if !email.headers.cc.is_empty() {
            lines.push(Line::from(format!("CC: {}", email.headers.cc.join(", "))));
        }
        lines.push(Line::from(format!("Subject: {}", email.headers.subject)));
        lines.push(Line::from(format!("Date: {}", email.headers.date)));
        lines.push(Line::from(""));

        // Raw body
        for line in email.body.lines() {
            lines.push(Line::from(line.to_string()));
        }

        lines
    }

    fn render_html_email_static<'a>(email: &'a EmailContent, theme: &'a Theme) -> Vec<Line<'a>> {
        let mut lines = Vec::new();

        // Headers
        lines.push(Line::from(vec![
            Span::styled("From: ", Style::default().fg(theme.colors.palette.accent)),
            Span::raw(email.headers.from.clone()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Subject: ", Style::default().fg(theme.colors.palette.accent)),
            Span::raw(email.headers.subject.clone()),
        ]));
        lines.push(Line::from(""));

        // Try to render HTML content
        if crate::html::is_html_content(&email.body) {
            // TODO: Fix HTML renderer integration 
            lines.push(Line::from("HTML content (renderer integration needed)"));
            for line in email.body.lines() {
                lines.push(Line::from(line.to_string()));
            }
        } else {
            for line in email.body.lines() {
                lines.push(Line::from(line.to_string()));
            }
        }

        lines
    }

    fn render_headers_email_static<'a>(email: &'a EmailContent, theme: &'a Theme) -> Vec<Line<'a>> {
        let mut lines = Vec::new();

        lines.push(Line::from(vec![
            Span::styled("Email Headers", Style::default()
                .fg(theme.colors.palette.accent)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED))
        ]));
        lines.push(Line::from(""));

        // All headers
        lines.push(Line::from(vec![
            Span::styled("From: ", Style::default().fg(theme.colors.palette.accent)),
            Span::raw(email.headers.from.clone()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("To: ", Style::default().fg(theme.colors.palette.accent)),
            Span::raw(email.headers.to.join(", ")),
        ]));
        
        if !email.headers.cc.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("CC: ", Style::default().fg(theme.colors.palette.accent)),
                Span::raw(email.headers.cc.join(", ")),
            ]));
        }
        
        if !email.headers.bcc.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("BCC: ", Style::default().fg(theme.colors.palette.accent)),
                Span::raw(email.headers.bcc.join(", ")),
            ]));
        }

        lines.push(Line::from(vec![
            Span::styled("Subject: ", Style::default().fg(theme.colors.palette.accent)),
            Span::raw(email.headers.subject.clone()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Date: ", Style::default().fg(theme.colors.palette.accent)),
            Span::raw(email.headers.date.clone()),
        ]));
        
        if !email.headers.message_id.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("Message-ID: ", Style::default().fg(theme.colors.palette.accent)),
                Span::raw(email.headers.message_id.clone()),
            ]));
        }

        if let Some(ref reply_to) = email.headers.reply_to {
            lines.push(Line::from(vec![
                Span::styled("Reply-To: ", Style::default().fg(theme.colors.palette.accent)),
                Span::raw(reply_to.clone()),
            ]));
        }

        if let Some(ref in_reply_to) = email.headers.in_reply_to {
            lines.push(Line::from(vec![
                Span::styled("In-Reply-To: ", Style::default().fg(theme.colors.palette.accent)),
                Span::raw(in_reply_to.clone()),
            ]));
        }

        lines
    }

    fn render_sender_box_static<'a>(headers: &'a EmailHeader, theme: &'a Theme) -> Vec<Line<'a>> {
        let mut lines = Vec::new();
        let box_width = 70;

        // Parse sender info
        let (sender_name, sender_email) = Self::parse_sender_info_static(&headers.from);

        // Top border
        lines.push(Line::from(vec![
            Span::styled("┌─", Style::default().fg(theme.colors.palette.border)),
            Span::styled(" From ", Style::default().fg(theme.colors.palette.accent).add_modifier(Modifier::BOLD)),
            Span::styled("─".repeat(box_width - 8), Style::default().fg(theme.colors.palette.border)),
            Span::styled("┐", Style::default().fg(theme.colors.palette.border)),
        ]));

        // Sender name
        if !sender_name.is_empty() {
            let content_width = box_width - 4;
            let truncated_name = if sender_name.len() > content_width {
                format!("{}...", &sender_name[..content_width.saturating_sub(3)])
            } else {
                sender_name
            };
            let padding = content_width.saturating_sub(truncated_name.len());

            lines.push(Line::from(vec![
                Span::styled("│ ", Style::default().fg(theme.colors.palette.border)),
                Span::styled(truncated_name, Style::default()
                    .fg(theme.colors.palette.accent)
                    .add_modifier(Modifier::BOLD)),
                Span::styled(" ".repeat(padding), Style::default()),
                Span::styled(" │", Style::default().fg(theme.colors.palette.border)),
            ]));
        }

        // Sender email
        if !sender_email.is_empty() {
            let email_display = format!("<{}>", sender_email);
            let content_width = box_width - 4;
            let truncated_email = if email_display.len() > content_width {
                format!("{}...", &email_display[..content_width.saturating_sub(3)])
            } else {
                email_display
            };
            let padding = content_width.saturating_sub(truncated_email.len());

            lines.push(Line::from(vec![
                Span::styled("│ ", Style::default().fg(theme.colors.palette.border)),
                Span::styled(truncated_email, Style::default().fg(theme.colors.palette.text_muted)),
                Span::styled(" ".repeat(padding), Style::default()),
                Span::styled(" │", Style::default().fg(theme.colors.palette.border)),
            ]));
        }

        // Date
        if !headers.date.is_empty() {
            let formatted_date = Self::format_date_static(&headers.date);
            let date_display = format!("📅 {}", formatted_date);
            let content_width = box_width - 4;
            let padding = content_width.saturating_sub(date_display.len());

            lines.push(Line::from(vec![
                Span::styled("│ ", Style::default().fg(theme.colors.palette.border)),
                Span::styled("📅 ", Style::default().fg(theme.colors.palette.info)),
                Span::styled(formatted_date, Style::default().fg(theme.colors.content_preview.body)),
                Span::styled(" ".repeat(padding.saturating_sub(2)), Style::default()),
                Span::styled(" │", Style::default().fg(theme.colors.palette.border)),
            ]));
        }

        // Bottom border
        lines.push(Line::from(vec![
            Span::styled("└", Style::default().fg(theme.colors.palette.border)),
            Span::styled("─".repeat(box_width - 2), Style::default().fg(theme.colors.palette.border)),
            Span::styled("┘", Style::default().fg(theme.colors.palette.border)),
        ]));

        lines
    }

    fn parse_sender_info(&self, from: &str) -> (String, String) {
        Self::parse_sender_info_static(from)
    }

    fn parse_sender_info_static(from: &str) -> (String, String) {
        // Simple parsing of "Name <email>" format
        if let Some(start) = from.find('<') {
            if let Some(end) = from.find('>') {
                let name = from[..start].trim().trim_matches('"').to_string();
                let email = from[start + 1..end].to_string();
                return (name, email);
            }
        }
        
        // Fallback: treat entire string as email
        ("".to_string(), from.to_string())
    }

    fn format_date(&self, date_str: &str) -> String {
        Self::format_date_static(date_str)
    }

    fn format_date_static(date_str: &str) -> String {
        // Try to parse and format the date
        if let Ok(parsed_date) = DateTime::parse_from_rfc2822(date_str) {
            let local_date = parsed_date.with_timezone(&Local);
            local_date.format("%Y-%m-%d %H:%M").to_string()
        } else {
            date_str.to_string()
        }
    }
}

impl Default for EmailViewer {
    fn default() -> Self {
        Self::new()
    }
}