use chrono::{DateTime, Local};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::email::StoredMessage;
use crate::theme::Theme;
use crate::ui::content_preview::{ContentType, EmailContent, EmailHeader, ViewMode};
use crate::images::{ImageManager, extract_images_from_html};

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
    AddToContacts,
    Close,
}

/// Full-screen email viewer with action buttons
pub struct EmailViewer {
    email_content: Option<EmailContent>,
    pub current_message: Option<StoredMessage>,
    sender_contact: Option<crate::contacts::Contact>,
    view_mode: ViewMode,
    scroll_position: usize,
    show_raw_headers: bool,
    show_actions: bool,
    selected_action: usize,
    actions: Vec<EmailViewerAction>,
    #[allow(dead_code)]
    image_manager: ImageManager,
}

impl EmailViewer {
    pub fn new() -> Self {
        Self {
            email_content: None,
            current_message: None,
            sender_contact: None,
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
                EmailViewerAction::AddToContacts,
                EmailViewerAction::Close,
            ],
            image_manager: ImageManager::new().unwrap_or_default(),
        }
    }

    /// Set email content to display
    pub fn set_email(&mut self, message: StoredMessage, email_content: EmailContent) {
        self.current_message = Some(message);
        self.email_content = Some(email_content);
        self.sender_contact = None; // Reset contact info when setting new email
        self.scroll_position = 0;
        self.show_actions = false;
        self.selected_action = 0;
    }

    /// Set sender contact information
    pub fn set_sender_contact(&mut self, contact: Option<crate::contacts::Contact>) {
        self.sender_contact = contact;
    }

    /// Get sender contact information
    pub fn get_sender_contact(&self) -> Option<&crate::contacts::Contact> {
        self.sender_contact.as_ref()
    }

    /// Check if sender is a known contact
    pub fn is_sender_known_contact(&self) -> bool {
        self.sender_contact.is_some()
    }

    /// Get sender email address from current message
    pub fn get_sender_email(&self) -> Option<String> {
        if let Some(ref content) = self.email_content {
            Some(content.headers.from.clone())
        } else if let Some(ref message) = self.current_message {
            Some(message.from_addr.clone())
        } else {
            None
        }
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
    pub fn get_scroll_info(
        &self,
        total_lines: usize,
        visible_height: usize,
    ) -> (usize, usize, bool, bool) {
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
    pub fn set_viewport_height(&mut self, _height: usize) {
        // Store viewport height for page scrolling calculations
        // This will be called from the render method
    }

    /// Handle key input
    pub fn handle_key(&mut self, key: crossterm::event::KeyCode) -> Option<EmailViewerAction> {
        self.handle_key_with_viewport(key, 20) // Default viewport height
    }

    /// Handle key input with viewport height for proper page scrolling
    pub fn handle_key_with_viewport(
        &mut self,
        key: crossterm::event::KeyCode,
        viewport_height: usize,
    ) -> Option<EmailViewerAction> {
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
            KeyCode::Char('c') => Some(EmailViewerAction::AddToContacts),
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
                Constraint::Length(3), // Header bar
                Constraint::Min(0),    // Email content
                Constraint::Length(3), // Status/instructions bar
            ])
            .split(area);

        // Render header bar
        self.render_header_bar(frame, chunks[0], theme);

        // Render email content area
        if self.show_actions {
            // Split content area for actions panel
            let content_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
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
            .block(
                Block::default()
                    .title("Email Viewer")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.colors.palette.accent)),
            )
            .style(
                Style::default()
                    .fg(theme.colors.palette.accent)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center);

        frame.render_widget(header, area);
    }

    fn render_email_content(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let content_height = area.height.saturating_sub(2) as usize;

        // Extract data needed for rendering before borrowing self
        let email_ref = self.email_content.as_ref();
        let _sender_contact_ref = self.sender_contact.as_ref();
        let view_mode = self.view_mode;

        let lines = if let Some(email) = email_ref {
            match view_mode {
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
            let (current, max, can_up, can_down) =
                self.get_scroll_info(lines.len(), content_height);
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
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.colors.palette.border)),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(content_paragraph, area);
    }

    fn render_actions_panel(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let mut action_lines = Vec::new();

        action_lines.push(Line::from(vec![Span::styled(
            "Actions",
            Style::default()
                .fg(theme.colors.palette.accent)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )]));
        action_lines.push(Line::from(""));

        for (i, action) in self.actions.iter().enumerate() {
            let is_selected = i == self.selected_action;
            let prefix = if is_selected { "► " } else { "  " };

            let style = if is_selected {
                Style::default()
                    .fg(theme.colors.palette.accent)
                    .add_modifier(Modifier::BOLD)
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
                EmailViewerAction::AddToContacts => "Add to Contacts",
                EmailViewerAction::Close => "Close",
            };

            action_lines.push(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(action_text, style),
            ]));
        }

        let actions_paragraph = Paragraph::new(action_lines)
            .block(
                Block::default()
                    .title("Actions")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.colors.palette.accent)),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(actions_paragraph, area);
    }

    fn render_footer_bar(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let instructions = if self.show_actions {
            "↑↓: Select Action | Enter: Execute | Esc: Hide Actions | r: Reply | f: Forward | c: Add Contact | q: Quit"
        } else {
            "j/k/↑↓: Scroll | PgUp/PgDn: Page | Home/End: Top/Bottom | Space: Actions | v: View | c: Add Contact | q: Quit"
        };

        let footer = Paragraph::new(instructions)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(theme.colors.palette.text_muted))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        frame.render_widget(footer, area);
    }

    fn render_formatted_email_static<'a>(
        email: &'a EmailContent,
        theme: &'a Theme,
    ) -> Vec<Line<'a>> {
        let mut lines = Vec::new();

        // Modern sender box
        lines.extend(Self::render_sender_box_static(&email.headers, theme));
        lines.push(Line::from(""));

        // Subject
        if !email.headers.subject.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(
                    "📧 Subject: ",
                    Style::default()
                        .fg(theme.colors.palette.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    email.headers.subject.clone(),
                    Style::default().fg(theme.colors.palette.accent),
                ),
            ]));
            lines.push(Line::from(""));
        }

        // Content separator
        lines.push(Line::from(vec![Span::styled(
            "─".repeat(80),
            Style::default().fg(theme.colors.palette.border),
        )]));
        lines.push(Line::from(""));

        // Email body - render properly based on content type with aggressive header filtering
        let cleaned_body = Self::filter_email_headers_and_metadata(&email.body);

        match email.content_type {
            ContentType::Html => {
                // Use HTML renderer to convert to plain text for terminal display
                let html_renderer = crate::html::HtmlRenderer::new(80);
                let plain_text = html_renderer.html_to_plain_text(&cleaned_body);

                // Further clean the rendered text of any remaining headers
                for line in plain_text.lines() {
                    let trimmed_line = line.trim();
                    if !trimmed_line.is_empty() && Self::is_content_line(trimmed_line) {
                        lines.push(Line::from(trimmed_line.to_string()));
                    } else if trimmed_line.is_empty() {
                        lines.push(Line::from(""));
                    }
                }
            }
            _ => {
                // Plain text - display as is but clean up any residual HTML and headers
                for line in cleaned_body.lines() {
                    let cleaned_line = if crate::html::is_html_content(line) {
                        let html_renderer = crate::html::HtmlRenderer::new(80);
                        html_renderer.html_to_plain_text(line)
                    } else {
                        line.to_string()
                    };

                    let trimmed_line = cleaned_line.trim();
                    if !trimmed_line.is_empty() && Self::is_content_line(trimmed_line) {
                        lines.push(Line::from(trimmed_line.to_string()));
                    } else if trimmed_line.is_empty() {
                        lines.push(Line::from(""));
                    }
                }
            }
        }

        lines
    }

    /// Render HTML email with embedded images and animations support
    #[allow(dead_code)]
    async fn render_html_email_with_media<'a>(&self, email: &'a EmailContent, theme: &'a Theme) -> Vec<Line<'a>> {
        let mut lines = Vec::new();
        
        // Headers
        lines.push(Line::from(vec![
            Span::styled("From: ", Style::default().fg(theme.colors.palette.accent)),
            Span::raw(email.headers.from.clone()),
        ]));
        lines.push(Line::from(vec![
            Span::styled(
                "Subject: ",
                Style::default().fg(theme.colors.palette.accent),
            ),
            Span::raw(email.headers.subject.clone()),
        ]));
        lines.push(Line::from(""));

        // Get cleaned email body
        let cleaned_body = Self::filter_email_headers_and_metadata(&email.body);
        
        // Check if terminal supports images
        if self.image_manager.supports_images() {
            // Extract and process images from HTML
            let image_refs = extract_images_from_html(&cleaned_body);
            
            if !image_refs.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled(
                        "📷 Images detected - loading...",
                        Style::default().fg(theme.colors.palette.text_secondary),
                    ),
                ]));
                lines.push(Line::from(""));
            }
        }

        // Render HTML content with image placeholders
        if email.content_type == ContentType::Html || crate::html::is_html_content(&cleaned_body) {
            let mut html_renderer = crate::html::HtmlRenderer::new(80);
            let rendered_text = html_renderer.render_html(&cleaned_body);
            
            // Convert ratatui Text to Lines for display with image integration
            for line in rendered_text.lines {
                let line_text = line
                    .spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>();

                // Filter out technical content that may have slipped through
                if !line_text.trim().is_empty() 
                    && !Self::is_line_technical_metadata(&line_text) {
                    
                    // Check if this line should be replaced with image content
                    if line_text.contains("[Image:") || line_text.contains("<img") {
                        // TODO: Replace with actual image terminal output
                        // For now, show a placeholder
                        lines.push(Line::from(vec![
                            Span::styled(
                                "🖼️  [Image]",
                                Style::default()
                                    .fg(theme.colors.palette.accent)
                                    .add_modifier(Modifier::BOLD),
                            ),
                        ]));
                    } else {
                        lines.push(line);
                    }
                }
            }
        } else {
            // Plain text content - split into lines
            for line in cleaned_body.lines() {
                if !line.trim().is_empty() && !Self::is_line_technical_metadata(line) {
                    lines.push(Line::from(line.to_string()));
                }
            }
        }

        lines
    }

    /// Check if a line contains technical metadata that should be filtered
    #[allow(dead_code)]
    fn is_line_technical_metadata(line: &str) -> bool {
        let line_lower = line.to_lowercase();
        
        // Common patterns that indicate technical metadata
        line_lower.contains("message-id:")
            || line_lower.contains("x-")
            || line_lower.contains("dkim-signature:")
            || line_lower.contains("authentication-results:")
            || line_lower.contains("received:")
            || line_lower.contains("return-path:")
            || line_lower.contains("content-type:")
            || line_lower.contains("content-transfer-encoding:")
            || line_lower.contains("mime-version:")
            || (line_lower.starts_with("http") && line_lower.contains("://"))
            || line.trim().chars().all(|c| c.is_ascii_alphanumeric() || "+=/-_".contains(c))
    }

    /// Render formatted email with contact information
    #[allow(dead_code)]
    fn render_formatted_email_with_contact<'a>(&'a self, email: &'a EmailContent, _sender_contact: Option<&'a crate::contacts::Contact>, theme: &'a Theme) -> Vec<Line<'a>> {
        let mut lines = Vec::new();

        // Modern sender box with contact info
        lines.extend(self.render_sender_box(&email.headers, theme));
        lines.push(Line::from(""));

        // Subject
        if !email.headers.subject.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("Subject: ", Style::default().fg(theme.colors.palette.accent)),
                Span::styled(
                    &email.headers.subject,
                    Style::default()
                        .fg(theme.colors.palette.text_primary)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        }

        // Date
        if !email.headers.date.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("Date: ", Style::default().fg(theme.colors.palette.accent)),
                Span::styled(&email.headers.date, Style::default().fg(theme.colors.palette.text_secondary)),
            ]));
        }

        // Content separator
        lines.push(Line::from(vec![Span::styled(
            "─".repeat(80),
            Style::default().fg(theme.colors.palette.border),
        )]));
        lines.push(Line::from(""));

        // Email body - render properly based on content type with aggressive header filtering
        let cleaned_body = Self::filter_email_headers_and_metadata(&email.body);

        match email.content_type {
            ContentType::Html => {
                // Use HTML renderer to convert to plain text for terminal display
                let html_renderer = crate::html::HtmlRenderer::new(80);
                let plain_text = html_renderer.html_to_plain_text(&cleaned_body);

                // Further clean the rendered text of any remaining headers
                for line in plain_text.lines() {
                    let trimmed_line = line.trim();
                    if !trimmed_line.is_empty() && Self::is_content_line(trimmed_line) {
                        lines.push(Line::from(trimmed_line.to_string()));
                    } else if trimmed_line.is_empty() {
                        lines.push(Line::from(""));
                    }
                }
            }
            _ => {
                // Plain text - display as is but clean up any residual HTML and headers
                for line in cleaned_body.lines() {
                    let trimmed_line = line.trim();
                    if !trimmed_line.is_empty() && Self::is_content_line(trimmed_line) {
                        lines.push(Line::from(trimmed_line.to_string()));
                    } else if trimmed_line.is_empty() {
                        lines.push(Line::from(""));
                    }
                }
            }
        }

        lines
    }

    /// Render HTML email with contact information (non-static version)
    #[allow(dead_code)]
    fn render_html_email<'a>(&'a self, email: &'a EmailContent, theme: &'a Theme) -> Vec<Line<'a>> {
        let mut lines = Vec::new();

        // Headers with contact info
        lines.extend(self.render_sender_box(&email.headers, theme));
        
        lines.push(Line::from(vec![
            Span::styled("Subject: ", Style::default().fg(theme.colors.palette.accent)),
            Span::styled(&email.headers.subject, Style::default().fg(theme.colors.palette.text_primary)),
        ]));
        
        if !email.headers.to.is_empty() {
            let to_string = email.headers.to.join(", ");
            lines.push(Line::from(vec![
                Span::styled("To: ", Style::default().fg(theme.colors.palette.accent)),
                Span::styled(to_string, Style::default().fg(theme.colors.palette.text_secondary)),
            ]));
        }
        
        lines.push(Line::from(vec![
            Span::styled("Date: ", Style::default().fg(theme.colors.palette.accent)),
            Span::styled(email.headers.date.clone(), Style::default().fg(theme.colors.palette.text_secondary)),
        ]));
        
        lines.push(Line::from(""));

        // Render HTML content with enhanced rendering and header filtering
        let cleaned_body = Self::filter_email_headers_and_metadata(&email.body);
        if email.content_type == ContentType::Html || crate::html::is_html_content(&cleaned_body) {
            let mut html_renderer = crate::html::HtmlRenderer::new(80);
            let rendered_text = html_renderer.render_html(&cleaned_body);
            
            // Process rendered lines and filter out any remaining headers or metadata
            for line in rendered_text.lines {
                // Check if the rendered line contains actual content
                let line_text = line
                    .spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>();

                if Self::is_content_line(&line_text) {
                    lines.push(line);
                }
            }
        } else {
            // Fallback to plain text rendering if not HTML
            for line in cleaned_body.lines() {
                let trimmed_line = line.trim();
                if !trimmed_line.is_empty() && Self::is_content_line(trimmed_line) {
                    lines.push(Line::from(trimmed_line.to_string()));
                } else if trimmed_line.is_empty() {
                    lines.push(Line::from(""));
                }
            }
        }

        lines
    }

    /// Render sender box with contact information (non-static version)
    #[allow(dead_code)]
    fn render_sender_box<'a>(&'a self, headers: &'a EmailHeader, theme: &'a Theme) -> Vec<Line<'a>> {
        let mut lines = Vec::new();
        let box_width = 70;

        // Parse sender info
        let (sender_name, sender_email) = Self::parse_sender_info_static(&headers.from);

        // Top border with contact indicator
        let title = if self.sender_contact.is_some() {
            " From 👤 " // Contact icon to indicate known contact
        } else {
            " From "
        };

        lines.push(Line::from(vec![
            Span::styled("┌─", Style::default().fg(theme.colors.palette.border)),
            Span::styled(
                title,
                Style::default()
                    .fg(if self.sender_contact.is_some() { 
                        theme.colors.palette.success 
                    } else { 
                        theme.colors.palette.accent 
                    })
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "─".repeat(box_width - title.len() - 2),
                Style::default().fg(theme.colors.palette.border),
            ),
            Span::styled("┐", Style::default().fg(theme.colors.palette.border)),
        ]));

        // Contact name or sender name
        let display_name = if let Some(ref contact) = self.sender_contact {
            if !contact.display_name.is_empty() {
                contact.display_name.clone()
            } else {
                sender_name
            }
        } else {
            sender_name
        };

        if !display_name.is_empty() {
            let content_width = box_width - 4;
            let truncated_name = if display_name.len() > content_width {
                format!("{}...", &display_name[..content_width.saturating_sub(3)])
            } else {
                display_name
            };

            lines.push(Line::from(vec![
                Span::styled("│ ", Style::default().fg(theme.colors.palette.border)),
                Span::styled(
                    format!("{:width$}", truncated_name, width = content_width),
                    Style::default()
                        .fg(if self.sender_contact.is_some() {
                            theme.colors.palette.success
                        } else {
                            theme.colors.palette.text_primary
                        })
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" │", Style::default().fg(theme.colors.palette.border)),
            ]));
        }

        // Email address
        let content_width = box_width - 4;
        let truncated_email = if sender_email.len() > content_width {
            format!("{}...", &sender_email[..content_width.saturating_sub(3)])
        } else {
            sender_email
        };

        lines.push(Line::from(vec![
            Span::styled("│ ", Style::default().fg(theme.colors.palette.border)),
            Span::styled(
                format!("{:width$}", truncated_email, width = content_width),
                Style::default().fg(theme.colors.palette.text_secondary),
            ),
            Span::styled(" │", Style::default().fg(theme.colors.palette.border)),
        ]));

        // Contact details if available
        if let Some(ref contact) = self.sender_contact {
            if let Some(company) = &contact.company {
                if !company.is_empty() {
                    let truncated_company = if company.len() > content_width {
                        format!("{}...", &company[..content_width.saturating_sub(3)])
                    } else {
                        company.clone()
                    };

                    lines.push(Line::from(vec![
                        Span::styled("│ ", Style::default().fg(theme.colors.palette.border)),
                        Span::styled(
                            format!("🏢 {:width$}", truncated_company, width = content_width - 2),
                            Style::default().fg(theme.colors.palette.text_secondary),
                        ),
                        Span::styled(" │", Style::default().fg(theme.colors.palette.border)),
                    ]));
                }
            }

            if let Some(title) = &contact.job_title {
                if !title.is_empty() {
                    let truncated_title = if title.len() > content_width {
                        format!("{}...", &title[..content_width.saturating_sub(3)])
                    } else {
                        title.clone()
                    };

                    lines.push(Line::from(vec![
                        Span::styled("│ ", Style::default().fg(theme.colors.palette.border)),
                        Span::styled(
                            format!("💼 {:width$}", truncated_title, width = content_width - 2),
                            Style::default().fg(theme.colors.palette.text_secondary),
                        ),
                        Span::styled(" │", Style::default().fg(theme.colors.palette.border)),
                    ]));
                }
            }
        }

        // Bottom border
        lines.push(Line::from(vec![
            Span::styled("└", Style::default().fg(theme.colors.palette.border)),
            Span::styled(
                "─".repeat(box_width - 2),
                Style::default().fg(theme.colors.palette.border),
            ),
            Span::styled("┘", Style::default().fg(theme.colors.palette.border)),
        ]));

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
            Span::styled(
                "Subject: ",
                Style::default().fg(theme.colors.palette.accent),
            ),
            Span::raw(email.headers.subject.clone()),
        ]));
        lines.push(Line::from(""));

        // Render HTML content with enhanced rendering and header filtering
        let cleaned_body = Self::filter_email_headers_and_metadata(&email.body);

        if email.content_type == ContentType::Html || crate::html::is_html_content(&cleaned_body) {
            let mut html_renderer = crate::html::HtmlRenderer::new(80);
            let rendered_text = html_renderer.render_html(&cleaned_body);

            // Convert ratatui Text to Lines for display with additional filtering
            for line in rendered_text.lines {
                // Check if the rendered line contains actual content
                let line_text = line
                    .spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>();

                if Self::is_content_line(&line_text) {
                    lines.push(line);
                }
            }
        } else {
            // Plain text content with header filtering
            for line in cleaned_body.lines() {
                let trimmed_line = line.trim();
                if !trimmed_line.is_empty() && Self::is_content_line(trimmed_line) {
                    lines.push(Line::from(trimmed_line.to_string()));
                } else if trimmed_line.is_empty() {
                    lines.push(Line::from(""));
                }
            }
        }

        lines
    }

    /// RFC-compliant email content extraction following industry best practices
    /// Based on RFC 5322, RFC 2045-2049 (MIME), and modern email parsing libraries
    pub fn filter_email_headers_and_metadata(content: &str) -> String {
        // Method 1: RFC 5322 standard - find the blank line separator
        if let Some(body_start) = Self::find_rfc_body_start(content) {
            let body = &content[body_start..];
            return Self::clean_email_body(body);
        }
        
        // Method 2: MIME multipart parsing (for complex emails)
        if content.contains("boundary=") {
            if let Some(body) = Self::extract_mime_text_content(content) {
                return Self::clean_email_body(&body);
            }
        }
        
        // Method 3: Content-type aware parsing
        if let Some(body) = Self::extract_content_by_type(content) {
            return Self::clean_email_body(&body);
        }
        
        // Method 4: Heuristic fallback with aggressive header filtering
        Self::heuristic_content_extraction(content)
    }
    
    /// Find email body start according to RFC 5322 - blank line separates headers from body
    fn find_rfc_body_start(content: &str) -> Option<usize> {
        // Look for double newline (RFC standard separator)
        if let Some(pos) = content.find("\r\n\r\n") {
            return Some(pos + 4);
        }
        if let Some(pos) = content.find("\n\n") {
            return Some(pos + 2);
        }
        None
    }
    
    /// Extract text content from MIME multipart messages
    fn extract_mime_text_content(content: &str) -> Option<String> {
        // Find boundary parameter
        let boundary = Self::extract_mime_boundary(content)?;
        
        // Split content by boundary
        let parts: Vec<&str> = content.split(&format!("--{}", boundary)).collect();
        
        for part in &parts {
            // Look for text/plain or text/html parts
            if part.contains("Content-Type: text/plain") || 
               part.contains("content-type: text/plain") {
                // Find the blank line and extract content
                if let Some(content_start) = Self::find_rfc_body_start(part) {
                    return Some(part[content_start..].trim().to_string());
                }
            }
        }
        
        // Fallback to HTML parts if no plain text found
        for part in &parts {
            if part.contains("Content-Type: text/html") || 
               part.contains("content-type: text/html") {
                if let Some(content_start) = Self::find_rfc_body_start(part) {
                    let html_content = &part[content_start..];
                    // Convert HTML to plain text
                    return Some(Self::html_to_plain_text(html_content));
                }
            }
        }
        
        None
    }
    
    /// Extract MIME boundary from Content-Type header
    fn extract_mime_boundary(content: &str) -> Option<String> {
        for line in content.lines() {
            let line_lower = line.to_lowercase();
            if line_lower.starts_with("content-type:") && line_lower.contains("boundary=") {
                // Extract boundary value
                if let Some(boundary_start) = line.find("boundary=") {
                    let boundary_part = &line[boundary_start + 9..];
                    // Handle quoted and unquoted boundaries
                    if boundary_part.starts_with('"') {
                        // Quoted boundary
                        if let Some(end_quote) = boundary_part[1..].find('"') {
                            return Some(boundary_part[1..end_quote + 1].to_string());
                        }
                    } else {
                        // Unquoted boundary (until semicolon or end of line)
                        let end_pos = boundary_part.find(';').unwrap_or(boundary_part.len());
                        return Some(boundary_part[..end_pos].trim().to_string());
                    }
                }
            }
        }
        None
    }
    
    /// Extract content based on Content-Type analysis
    fn extract_content_by_type(content: &str) -> Option<String> {
        let lines: Vec<&str> = content.lines().collect();
        let mut content_type = String::new();
        let in_headers = true;
        let mut body_start_idx = 0;
        
        // Parse headers to find Content-Type
        for (i, line) in lines.iter().enumerate() {
            if in_headers {
                if line.trim().is_empty() {
                    body_start_idx = i + 1;
                    break;
                }
                
                let line_lower = line.to_lowercase();
                if line_lower.starts_with("content-type:") {
                    content_type = line_lower;
                }
            }
        }
        
        if body_start_idx < lines.len() {
            let body_lines = &lines[body_start_idx..];
            let body_content = body_lines.join("\n");
            
            if content_type.contains("text/html") {
                return Some(Self::html_to_plain_text(&body_content));
            } else {
                return Some(body_content);
            }
        }
        
        None
    }
    
    /// Heuristic content extraction as fallback
    fn heuristic_content_extraction(content: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let mut filtered_lines: Vec<&str> = Vec::new();
        let mut headers_ended = false;
        let mut consecutive_non_headers = 0;
        
        for line in lines {
            let trimmed = line.trim();
            
            // Check if this looks like a header (contains colon and matches header patterns)
            let looks_like_header = trimmed.contains(':') && 
                Self::is_likely_email_header(trimmed);
            
            // Check for technical metadata patterns
            let is_technical = Self::is_technical_metadata(trimmed);
            
            if !headers_ended {
                if !looks_like_header && !is_technical && !trimmed.is_empty() {
                    consecutive_non_headers += 1;
                    // If we see 2+ consecutive non-header lines, assume body started
                    if consecutive_non_headers >= 2 {
                        headers_ended = true;
                        // Include previous lines that weren't headers
                        filtered_lines.push(line);
                    }
                } else {
                    consecutive_non_headers = 0;
                }
            } else {
                // We're in the body - include everything except obvious technical stuff
                if !is_technical {
                    filtered_lines.push(line);
                }
            }
        }
        
        filtered_lines.join("\n")
    }
    
    /// Check if a line is likely an email header
    fn is_likely_email_header(line: &str) -> bool {
        let line_lower = line.to_lowercase();
        
        // Standard email headers
        let header_prefixes = [
            "from:", "to:", "cc:", "bcc:", "subject:", "date:", "reply-to:",
            "message-id:", "in-reply-to:", "references:", "mime-version:",
            "content-type:", "content-transfer-encoding:", "content-disposition:",
            "received:", "return-path:", "delivered-to:", "authentication-results:",
            "dkim-signature:", "arc-", "x-", "list-", "sender:", "envelope-to:",
        ];
        
        header_prefixes.iter().any(|prefix| line_lower.starts_with(prefix))
    }
    
    /// Check if a line is technical metadata that should be filtered
    fn is_technical_metadata(line: &str) -> bool {
        let trimmed = line.trim();
        
        // DKIM signature data patterns
        if trimmed.len() > 40 && 
           (trimmed.chars().all(|c| c.is_ascii_alphanumeric() || "=+/".contains(c)) ||
            trimmed.starts_with("bh=") || 
            trimmed.starts_with("b=") ||
            trimmed.contains("d=") && trimmed.contains("s=")) {
            return true;
        }
        
        // Long base64-like strings
        if trimmed.len() > 60 && 
           trimmed.chars().filter(|c| c.is_ascii_alphanumeric()).count() > trimmed.len() * 3 / 4 {
            return true;
        }
        
        // Timestamp patterns
        if regex::Regex::new(r"^\s*[0-9]{2}:[0-9]{2}:[0-9]{2}\s+[+-]\d{4}").unwrap().is_match(trimmed) {
            return true;
        }
        
        false
    }
    
    /// Clean email body content (remove remaining HTML tags, normalize whitespace)
    fn clean_email_body(body: &str) -> String {
        // Remove HTML tags but preserve content
        let cleaned = Self::html_to_plain_text(body);
        
        // Normalize whitespace and remove excessive blank lines
        let lines: Vec<&str> = cleaned.lines().collect();
        let mut result_lines = Vec::new();
        let mut consecutive_empty = 0;
        
        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                consecutive_empty += 1;
                // Only allow up to 2 consecutive empty lines
                if consecutive_empty <= 2 {
                    result_lines.push(line);
                }
            } else {
                consecutive_empty = 0;
                result_lines.push(line);
            }
        }
        
        result_lines.join("\n").trim().to_string()
    }
    
    /// Convert HTML to plain text (enhanced version)
    fn html_to_plain_text(html: &str) -> String {
        // Simple HTML to text conversion
        // In a real implementation, you might use a proper HTML parser
        let mut result = String::new();
        let mut in_tag = false;
        let mut in_script = false;
        let mut in_style = false;
        let mut tag_name = String::new();
        
        let chars: Vec<char> = html.chars().collect();
        let mut i = 0;
        
        while i < chars.len() {
            let ch = chars[i];
            
            match ch {
                '<' => {
                    // Check for script/style tags
                    let remaining: String = chars[i..].iter().collect();
                    if remaining.to_lowercase().starts_with("<script") {
                        in_script = true;
                    } else if remaining.to_lowercase().starts_with("<style") {
                        in_style = true;
                    } else if remaining.to_lowercase().starts_with("</script") {
                        in_script = false;
                    } else if remaining.to_lowercase().starts_with("</style") {
                        in_style = false;
                    }
                    
                    in_tag = true;
                    tag_name.clear();
                }
                '>' => {
                    in_tag = false;
                    // Add space after block-level tags
                    if ["div", "p", "br", "hr", "h1", "h2", "h3", "h4", "h5", "h6"]
                        .contains(&tag_name.to_lowercase().as_str()) {
                        result.push('\n');
                    }
                }
                _ if in_tag => {
                    if ch.is_ascii_alphabetic() {
                        tag_name.push(ch);
                    }
                }
                _ if !in_tag && !in_script && !in_style => {
                    result.push(ch);
                }
                _ => {}
            }
            
            i += 1;
        }
        
        // Clean up whitespace
        result.split_whitespace().collect::<Vec<_>>().join(" ")
    }


    /// Check if a line contains actual email content (not headers or metadata) - enhanced version
    fn is_content_line(line: &str) -> bool {
        let line_lower = line.to_lowercase();
        let line_trimmed = line.trim();

        // Skip empty lines
        if line_trimmed.is_empty() {
            return true; // Allow empty lines for formatting
        }

        // Enhanced header patterns - more comprehensive
        let header_prefixes = [
            "from:",
            "to:",
            "cc:",
            "bcc:",
            "subject:",
            "date:",
            "reply-to:",
            "sender:",
            "message-id:",
            "in-reply-to:",
            "references:",
            "mime-version:",
            "content-type:",
            "content-transfer-encoding:",
            "content-disposition:",
            "delivered-to:",
            "received:",
            "return-path:",
            "envelope-to:",
            "authentication-results:",
            "received-spf:",
            "dkim-signature:",
            "dkim-filter:",
            "x-received:",
            "x-google-smtp-source:",
            "x-gm-message-state:",
            "x-ms-exchange-",
            "x-originating-ip:",
            "x-microsoft-",
            "x-ms-",
            "x-mailer:",
            "x-apple-",
            "x-",
            "arc-",
            "thread-",
            "list-id:",
            "list-unsubscribe:",
            "list-archive:",
            "list-post:",
            "organization:",
            "user-agent:",
            "importance:",
            "priority:",
        ];

        // Check for header patterns
        for prefix in &header_prefixes {
            if line_lower.starts_with(prefix) {
                return false;
            }
        }

        // Skip lines that contain mostly technical patterns
        let technical_patterns = [
            "spf=pass",
            "dkim=pass",
            "dmarc=pass",
            "smtp.mailfrom=",
            "smtp.helo=",
            "boundary=",
            "charset=",
            "encoding=",
            "content=\"text/html",
            "<!doctype",
            "<html",
            "</html>",
            "<head",
            "</head>",
            "<body",
            "</body>",
            "<meta",
            "<style",
            "</style>",
            "<script",
            "</script>",
        ];

        for pattern in &technical_patterns {
            if line_lower.contains(pattern) {
                return false;
            }
        }

        // Skip encoded content patterns
        if line_trimmed.starts_with('=')
            || line_trimmed.starts_with("--")
            || line_trimmed.starts_with("Message-ID:")
            || line_trimmed.starts_with("MIME-Version")
        {
            return false;
        }

        // Skip long strings that look like encoded content or IDs
        if line_trimmed.len() > 60 {
            // Check if it's mostly alphanumeric (encoded content)
            let alphanumeric_count = line_trimmed
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .count();
            if alphanumeric_count > line_trimmed.len() * 3 / 4 {
                return false;
            }

            // Check if it's mostly base64-like characters
            if line_trimmed
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "=+-/".contains(c))
            {
                return false;
            }
        }

        // Skip lines that are just HTML tags
        if line_trimmed.starts_with('<')
            && line_trimmed.ends_with('>')
            && !line_trimmed.contains(' ')
            && line_trimmed.len() < 50
        {
            return false;
        }

        // Skip lines that look like HTML attribute declarations
        if line_trimmed.contains('=')
            && line_trimmed.contains('"')
            && (line_trimmed.contains("style=")
                || line_trimmed.contains("class=")
                || line_trimmed.contains("id=")
                || line_trimmed.contains("href="))
        {
            return false;
        }

        // Skip lines that are mostly punctuation or special characters
        let punct_count = line_trimmed
            .chars()
            .filter(|c| c.is_ascii_punctuation())
            .count();
        if punct_count > line_trimmed.len() / 2 && line_trimmed.len() > 10 {
            return false;
        }

        // Accept the line as content
        true
    }

    fn render_headers_email_static<'a>(email: &'a EmailContent, theme: &'a Theme) -> Vec<Line<'a>> {
        let mut lines = Vec::new();

        lines.push(Line::from(vec![Span::styled(
            "Email Headers",
            Style::default()
                .fg(theme.colors.palette.accent)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )]));
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
            Span::styled(
                "Subject: ",
                Style::default().fg(theme.colors.palette.accent),
            ),
            Span::raw(email.headers.subject.clone()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Date: ", Style::default().fg(theme.colors.palette.accent)),
            Span::raw(email.headers.date.clone()),
        ]));

        if !email.headers.message_id.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(
                    "Message-ID: ",
                    Style::default().fg(theme.colors.palette.accent),
                ),
                Span::raw(email.headers.message_id.clone()),
            ]));
        }

        if let Some(ref reply_to) = email.headers.reply_to {
            lines.push(Line::from(vec![
                Span::styled(
                    "Reply-To: ",
                    Style::default().fg(theme.colors.palette.accent),
                ),
                Span::raw(reply_to.clone()),
            ]));
        }

        if let Some(ref in_reply_to) = email.headers.in_reply_to {
            lines.push(Line::from(vec![
                Span::styled(
                    "In-Reply-To: ",
                    Style::default().fg(theme.colors.palette.accent),
                ),
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
            Span::styled(
                " From ",
                Style::default()
                    .fg(theme.colors.palette.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "─".repeat(box_width - 8),
                Style::default().fg(theme.colors.palette.border),
            ),
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
                Span::styled(
                    truncated_name,
                    Style::default()
                        .fg(theme.colors.palette.accent)
                        .add_modifier(Modifier::BOLD),
                ),
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
                Span::styled(
                    truncated_email,
                    Style::default().fg(theme.colors.palette.text_muted),
                ),
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
                Span::styled(
                    formatted_date,
                    Style::default().fg(theme.colors.content_preview.body),
                ),
                Span::styled(" ".repeat(padding.saturating_sub(2)), Style::default()),
                Span::styled(" │", Style::default().fg(theme.colors.palette.border)),
            ]));
        }

        // Bottom border
        lines.push(Line::from(vec![
            Span::styled("└", Style::default().fg(theme.colors.palette.border)),
            Span::styled(
                "─".repeat(box_width - 2),
                Style::default().fg(theme.colors.palette.border),
            ),
            Span::styled("┘", Style::default().fg(theme.colors.palette.border)),
        ]));

        lines
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
