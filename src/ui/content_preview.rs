use crate::animation::AnimationManager;
use crate::clipboard::ClipboardManager;
use crate::email::{AttachmentInfo, AttachmentViewer, EmailDatabase, StoredMessage};
use crate::images::{extract_images_from_html, ImageManager};
use crate::theme::Theme;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Wrap},
    Frame,
};
use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct EmailHeader {
    pub from: String,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub subject: String,
    pub date: String,
    pub message_id: String,
    pub reply_to: Option<String>,
    pub in_reply_to: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Attachment {
    pub filename: String,
    pub content_type: String,
    pub size: usize,
    pub is_inline: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentType {
    PlainText,
    Html,
    Markdown,
    Code(String), // Language identifier
}

#[derive(Debug, Clone)]
pub struct EmailContent {
    pub headers: EmailHeader,
    pub body: String,
    pub content_type: ContentType,
    pub attachments: Vec<Attachment>,
    pub parsed_urls: Vec<String>,
    pub parsed_content: Vec<ContentLine>,
}

#[derive(Debug, Clone)]
pub struct ContentLine {
    pub text: String,
    pub line_type: LineType,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineType {
    Header,
    Subject,
    Body,
    Quote,
    Code,
    Link,
    Bullet,
    Separator,
    Attachment,
    Signature,
    Empty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Raw,       // Show raw email content
    Formatted, // Show formatted/processed content
    Html,      // Show HTML content rendered for terminal
    Headers,   // Show headers only
}

pub struct ContentPreview {
    email_content: Option<EmailContent>,
    raw_content: Vec<String>,
    scroll: usize,
    view_mode: ViewMode,
    show_headers_expanded: bool,
    url_regex: Regex,
    email_regex: Regex,
    database: Option<Arc<EmailDatabase>>,
    current_message_id: Option<Uuid>,
    loading: bool,
    html_renderer: crate::html::HtmlRenderer,
    image_manager: ImageManager,
    animation_manager: Option<Arc<AnimationManager>>,
    processed_images: HashMap<String, String>, // URL -> rendered content
    active_animations: HashMap<String, String>, // Animation ID -> display content
    selected_attachment: Option<usize>,        // Index of selected attachment
    attachment_viewer: AttachmentViewer,
    is_viewing_attachment: bool,
    clipboard_manager: ClipboardManager,
    imap_manager: Option<Arc<crate::imap::ImapAccountManager>>,
}

impl ContentPreview {
    pub fn new() -> Self {
        let url_regex = Regex::new(r"https?://[^\s]+").unwrap();
        let email_regex =
            Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").unwrap();

        let mut preview = Self {
            email_content: None,
            raw_content: Vec::new(),
            scroll: 0,
            view_mode: ViewMode::Formatted,
            show_headers_expanded: false,
            url_regex,
            email_regex,
            database: None,
            current_message_id: None,
            loading: false,
            html_renderer: crate::html::HtmlRenderer::new(80),
            image_manager: ImageManager::new().unwrap_or_default(),
            animation_manager: None, // Will be initialized later
            processed_images: HashMap::new(),
            active_animations: HashMap::new(),
            selected_attachment: None,
            attachment_viewer: AttachmentViewer::default(),
            is_viewing_attachment: false,
            clipboard_manager: ClipboardManager::new(),
            imap_manager: None,
        };

        // Initialize with sample content
        preview.initialize_sample_content();

        // Initialize animation manager
        preview.initialize_animation_manager();

        preview
    }

    fn initialize_sample_content(&mut self) {
        // Create a sample EmailContent with proper headers and body
        let headers = EmailHeader {
            from: "Comunicado Team <team@comunicado.dev>".to_string(),
            to: vec!["user@example.com".to_string()],
            cc: vec![],
            bcc: vec![],
            subject: "Welcome to Comunicado! 🎉".to_string(),
            date: "Today 10:30 AM".to_string(),
            message_id: "<welcome@comunicado.dev>".to_string(),
            reply_to: None,
            in_reply_to: None,
        };

        let body = "Welcome to Comunicado - the modern TUI email client!

We're excited to have you try out our terminal-based email experience.
Comunicado brings modern email features directly to your terminal with:

• Modern TUI Interface
  - Clean, intuitive design with ratatui
  - Vim-style keyboard navigation
  - Responsive three-pane layout

• Rich Email Support  
  - HTML email rendering ✨
  - Image and animation display
  - Attachment handling

• Secure Authentication
  - OAuth2 support for major providers
  - Multi-account management
  - Local email storage with Maildir

• Integrated Calendar
  - CalDAV synchronization
  - Meeting invitation handling
  - Desktop environment integration

Getting Started

Use these keyboard shortcuts to navigate:

Navigation:
  Tab / Shift+Tab  - Switch between panes
  h/j/k/l          - Vim-style movement
  ↑/↓              - Move up/down in lists
  Enter            - Select/expand items

Content Controls:
  H                - Toggle headers view
  m                - Switch view modes
  v                - View attachments

This is just the beginning! We're actively developing new features
including advanced HTML email rendering, OAuth2 authentication, and
integrated calendar functionality.

Thank you for trying Comunicado!

Best regards,
The Comunicado Development Team

---
This is a sample email showcasing the modern email display format."
            .to_string();

        // Parse the body content
        let parsed_content = self.parse_content_lines(&body);
        let parsed_urls = self.extract_urls(&body);

        let email_content = EmailContent {
            headers,
            body,
            content_type: ContentType::PlainText,
            attachments: vec![
                Attachment {
                    filename: "welcome-guide.pdf".to_string(),
                    content_type: "application/pdf".to_string(),
                    size: 2_048_000, // 2MB
                    is_inline: false,
                },
                Attachment {
                    filename: "logo.png".to_string(),
                    content_type: "image/png".to_string(),
                    size: 45000, // 45KB
                    is_inline: true,
                },
            ],
            parsed_urls,
            parsed_content,
        };

        self.email_content = Some(email_content);
        self.view_mode = ViewMode::Formatted;
    }

    pub fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        block: Block,
        is_focused: bool,
        theme: &Theme,
    ) {
        // If we're viewing an attachment, delegate to the attachment viewer
        if self.is_viewing_attachment {
            self.attachment_viewer.render(frame, area, block, theme);
            return;
        }

        // Update image dimensions based on current area
        self.update_image_dimensions(area);

        let content_height = area.height.saturating_sub(2) as usize; // Account for block borders

        // Extract values needed to avoid borrowing conflicts
        let view_mode = self.view_mode;
        let current_scroll = self.scroll;
        let raw_content_len = self.raw_content.len();
        let email_lines_estimate = if let Some(ref email) = self.email_content {
            match view_mode {
                ViewMode::Formatted => email.parsed_content.len() + 10, // Headers + content
                ViewMode::Html => email.body.lines().count() + 10, // Estimate HTML rendered line count
                ViewMode::Headers => 8,                            // Typical header count
                ViewMode::Raw => raw_content_len,
            }
        } else {
            raw_content_len
        };

        let lines = match view_mode {
            ViewMode::Raw => self.render_raw_content(content_height, theme),
            ViewMode::Formatted => self.render_formatted_content(content_height, theme),
            ViewMode::Html => self.render_html_content(content_height, theme),
            ViewMode::Headers => self.render_headers_only(content_height, theme),
        };

        // Create scroll indicator if content is scrollable
        let total_lines = match view_mode {
            ViewMode::Raw => raw_content_len,
            _ => email_lines_estimate,
        };

        let scroll_indicator = if total_lines > content_height {
            let position =
                (current_scroll as f32 / (total_lines - content_height) as f32 * 100.0) as u16;
            format!(" ({}%)", position)
        } else {
            String::new()
        };

        let view_mode_indicator = match view_mode {
            ViewMode::Raw => " [Raw]",
            ViewMode::Formatted => " [Formatted]",
            ViewMode::Html => " [HTML]",
            ViewMode::Headers => " [Headers]",
        };

        let title = if is_focused {
            format!("Content{}{}", view_mode_indicator, scroll_indicator)
        } else {
            format!("Content{}{}", view_mode_indicator, scroll_indicator)
        };

        let paragraph = Paragraph::new(lines)
            .block(block.title(title))
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }

    fn render_raw_content(&mut self, content_height: usize, theme: &Theme) -> Vec<Line> {
        let start_line = self.scroll;
        let end_line = (start_line + content_height).min(self.raw_content.len());

        self.raw_content[start_line..end_line]
            .iter()
            .map(|line| {
                Line::from(vec![Span::styled(
                    line.clone(),
                    Style::default().fg(theme.colors.content_preview.body),
                )])
            })
            .collect()
    }

    fn render_formatted_content(&mut self, content_height: usize, theme: &Theme) -> Vec<Line> {
        if let Some(email) = self.email_content.clone() {
            // Extract all data we need before any mutable borrows
            let show_headers_expanded = self.show_headers_expanded;
            let _terminal_width = self.html_renderer.max_width as u16;
            let scroll = self.scroll;
            let selected_attachment = self.selected_attachment;

            let mut all_lines = Vec::new();

            // Do all mutable operations first to avoid borrowing conflicts
            let html_lines = if email.content_type == ContentType::Html {
                tracing::debug!(
                    "Content Preview: Processing HTML content of length {}",
                    email.body.len()
                );

                // Always try HTML rendering first - the new renderer handles detection internally
                let email_body = email.body.clone();
                let rendered_text = self.html_renderer.render_html(&email_body);

                tracing::debug!(
                    "Content Preview: HTML renderer generated {} lines",
                    rendered_text.lines.len()
                );

                // Return rendered HTML lines directly
                Some(rendered_text.lines)
            } else {
                tracing::debug!("Content Preview: Processing plain text content");
                None
            };

            // Now do immutable operations
            // Add header information based on user preference
            if show_headers_expanded {
                // Show full headers when expanded (for advanced users)
                all_lines.extend(self.render_email_headers(&email.headers, theme));
                all_lines.push(Line::from("")); // Empty line separator
            } else {
                // Minimal header showing only From and Subject as requested
                all_lines.extend(self.render_minimal_headers(&email.headers, theme));
                all_lines.push(Line::from("")); // Empty line separator
            }

            // Add body content with better visual separation
            all_lines.push(Line::from("")); // Extra spacing before body
            all_lines.push(Line::from(vec![Span::styled(
                "─".repeat(60),
                Style::default()
                    .fg(theme.colors.palette.border)
                    .add_modifier(Modifier::DIM),
            )]));
            all_lines.push(Line::from("")); // Spacing after separator

            // Add formatted content lines with HTML detection
            if let Some(processed_lines) = html_lines {
                all_lines.extend(processed_lines);
            } else {
                // Use parsed content lines for plain text with header filtering
                let filtered_content = self.filter_raw_headers_from_content(&email.parsed_content);
                for content_line in &filtered_content {
                    all_lines.push(self.render_content_line(content_line, theme));
                }
            }

            // Add attachments section if there are any
            let attachments = email.attachments.clone(); // Clone to avoid borrowing issues
            if !attachments.is_empty() {
                all_lines.push(Line::from("")); // Empty line separator
                all_lines.push(Line::from(vec![Span::styled(
                    "📎 Attachments:",
                    Style::default()
                        .fg(theme.colors.content_preview.header)
                        .add_modifier(Modifier::BOLD),
                )]));

                for (index, attachment) in attachments.iter().enumerate() {
                    // Enhanced attachment info with type detection
                    let attachment_type =
                        crate::email::AttachmentType::from_content_type(&attachment.content_type);
                    let attachment_icon = attachment_type.icon();
                    let type_description = attachment_type.description();

                    let size_display = if attachment.size > 0 {
                        if attachment.size < 1024 {
                            format!(" ({}B)", attachment.size)
                        } else if attachment.size < 1024 * 1024 {
                            format!(" ({:.1}KB)", attachment.size as f64 / 1024.0)
                        } else {
                            format!(" ({:.1}MB)", attachment.size as f64 / (1024.0 * 1024.0))
                        }
                    } else {
                        String::new()
                    };

                    let is_selected = selected_attachment == Some(index);
                    let selection_prefix = if is_selected { "► " } else { "  " };

                    let number_style = if is_selected {
                        Style::default()
                            .fg(theme.colors.palette.accent)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.colors.content_preview.quote)
                    };

                    let filename_style = if is_selected {
                        Style::default()
                            .fg(theme.colors.palette.accent)
                            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
                    } else {
                        Style::default()
                            .fg(theme.colors.content_preview.body)
                            .add_modifier(Modifier::UNDERLINED)
                    };

                    // Color code by safety and type
                    let type_color = if attachment_type.is_image() {
                        theme.colors.palette.success
                    } else if attachment_type.is_document() {
                        theme.colors.palette.accent
                    } else if attachment_type.is_archive() {
                        theme.colors.palette.warning
                    } else {
                        theme.colors.content_preview.quote
                    };

                    let attachment_line = Line::from(vec![
                        Span::styled(format!("{}{}. ", selection_prefix, index + 1), number_style),
                        Span::styled(
                            format!("{} ", attachment_icon),
                            Style::default().fg(type_color),
                        ),
                        Span::styled(attachment.filename.clone(), filename_style),
                        Span::styled(
                            size_display,
                            Style::default().fg(theme.colors.content_preview.quote),
                        ),
                        Span::styled(
                            format!(" [{}]", type_description),
                            Style::default().fg(type_color),
                        ),
                    ]);

                    all_lines.push(attachment_line);
                }

                // Add instruction lines
                all_lines.push(Line::from(vec![Span::styled(
                    "  Press 'v' to view, 'O' to open with system app, or 's' to save",
                    Style::default()
                        .fg(theme.colors.content_preview.quote)
                        .add_modifier(Modifier::ITALIC),
                )]));
            }

            // Apply scrolling
            let start_line = scroll;
            let end_line = (start_line + content_height).min(all_lines.len());

            all_lines[start_line..end_line].to_vec()
        } else {
            // Fallback to raw content with basic styling
            self.render_raw_content_with_styling(content_height, theme)
        }
    }

    fn render_html_content(&mut self, content_height: usize, _theme: &Theme) -> Vec<Line> {
        if let Some(ref email) = self.email_content {
            let mut all_lines = Vec::new();

            // Add compact headers first
            all_lines.push(Line::from(vec![
                Span::styled("From: ", Style::default().fg(Color::Yellow)),
                Span::raw(format!("{}", email.headers.from)),
            ]));
            all_lines.push(Line::from(vec![
                Span::styled("Subject: ", Style::default().fg(Color::Yellow)),
                Span::raw(format!("{}", email.headers.subject)),
            ]));
            all_lines.push(Line::from("")); // Separator

            // Always render HTML content - the renderer handles everything internally
            if !email.body.is_empty() {
                tracing::debug!(
                    "HTML Content Rendering: Content length = {}",
                    email.body.len()
                );

                // Use the HTML renderer to convert HTML to readable text
                let mut rendered_text = self.html_renderer.render_html(&email.body);

                // Replace any animation placeholders with current frame data
                for line in &mut rendered_text.lines {
                    for span in &mut line.spans {
                        // Only modify span content if we have active animations
                        if !self.active_animations.is_empty() {
                            span.content = self.render_animations_in_content(&span.content).into();
                        }
                    }
                }

                // Add rendered HTML lines directly
                all_lines.extend(rendered_text.lines);

                tracing::debug!(
                    "HTML Content Rendering: Generated {} lines after processing",
                    all_lines.len() - 3
                ); // Subtract headers
            } else {
                all_lines.push(Line::styled(
                    "(No HTML content available)",
                    Style::default().fg(Color::Gray),
                ));
            }

            // Apply scrolling
            let start_line = self.scroll;
            let end_line = (start_line + content_height).min(all_lines.len());

            if end_line > start_line {
                all_lines[start_line..end_line].to_vec()
            } else {
                vec![Line::from("(End of content)")]
            }
        } else {
            vec![Line::styled(
                "No HTML content loaded",
                Style::default().fg(Color::Gray),
            )]
        }
    }

    fn render_headers_only(&mut self, content_height: usize, theme: &Theme) -> Vec<Line> {
        if let Some(ref email) = self.email_content {
            let header_lines = self.render_email_headers(&email.headers, theme);
            let start_line = self.scroll;
            let end_line = (start_line + content_height).min(header_lines.len());

            header_lines[start_line..end_line].to_vec()
        } else {
            vec![Line::from(vec![Span::styled(
                "No email content available",
                Style::default().fg(theme.colors.palette.text_muted),
            )])]
        }
    }

    /// Render minimal headers showing only From and Subject as requested by user
    fn render_minimal_headers(&self, headers: &EmailHeader, theme: &Theme) -> Vec<Line> {
        let mut lines = Vec::new();

        // Show From field (simplified, clean format)
        if !headers.from.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(
                    "From: ",
                    Style::default()
                        .fg(theme.colors.content_preview.header)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    headers.from.clone(),
                    Style::default().fg(theme.colors.content_preview.body),
                ),
            ]));
        }

        // Show To field if available
        if !headers.to.is_empty() {
            let to_display = if headers.to.len() > 1 {
                format!("{} and {} others", headers.to[0], headers.to.len() - 1)
            } else {
                headers.to[0].clone()
            };
            lines.push(Line::from(vec![
                Span::styled(
                    "To: ",
                    Style::default()
                        .fg(theme.colors.content_preview.header)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    to_display,
                    Style::default().fg(theme.colors.content_preview.body),
                ),
            ]));
        }

        // Show Date field
        if !headers.date.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(
                    "Date: ",
                    Style::default()
                        .fg(theme.colors.content_preview.header)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    headers.date.clone(),
                    Style::default().fg(theme.colors.content_preview.body),
                ),
            ]));
        }

        // Show Subject field (simplified, clean format)
        if !headers.subject.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(
                    "Subject: ",
                    Style::default()
                        .fg(theme.colors.content_preview.header)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    headers.subject.clone(),
                    Style::default()
                        .fg(theme.colors.palette.accent)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        }

        lines
    }

    fn render_email_headers(&self, headers: &EmailHeader, theme: &Theme) -> Vec<Line> {
        let mut lines = Vec::new();

        if !headers.from.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(
                    "From: ",
                    Style::default()
                        .fg(theme.colors.content_preview.header)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    headers.from.clone(),
                    Style::default().fg(theme.colors.content_preview.body),
                ),
            ]));
        }

        if !headers.to.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(
                    "To: ",
                    Style::default()
                        .fg(theme.colors.content_preview.header)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    headers.to.join(", "),
                    Style::default().fg(theme.colors.content_preview.body),
                ),
            ]));
        }

        if !headers.cc.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(
                    "CC: ",
                    Style::default()
                        .fg(theme.colors.content_preview.header)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    headers.cc.join(", "),
                    Style::default().fg(theme.colors.content_preview.body),
                ),
            ]));
        }

        if !headers.date.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(
                    "Date: ",
                    Style::default()
                        .fg(theme.colors.content_preview.header)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    headers.date.clone(),
                    Style::default().fg(theme.colors.content_preview.body),
                ),
            ]));
        }

        if !headers.subject.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(
                    "Subject: ",
                    Style::default()
                        .fg(theme.colors.content_preview.header)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    headers.subject.clone(),
                    Style::default().fg(theme.colors.content_preview.body),
                ),
            ]));
        }

        if let Some(ref reply_to) = headers.reply_to {
            lines.push(Line::from(vec![
                Span::styled(
                    "Reply-To: ",
                    Style::default()
                        .fg(theme.colors.content_preview.header)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    reply_to.clone(),
                    Style::default().fg(theme.colors.content_preview.body),
                ),
            ]));
        }

        if !headers.message_id.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(
                    "Message-ID: ",
                    Style::default()
                        .fg(theme.colors.content_preview.header)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    headers.message_id.clone(),
                    Style::default().fg(theme.colors.palette.text_muted),
                ),
            ]));
        }

        lines
    }

    fn render_content_line(&self, content_line: &ContentLine, theme: &Theme) -> Line {
        match content_line.line_type {
            LineType::Header => self.render_header_line(content_line, theme),
            LineType::Subject => Line::from(vec![Span::styled(
                content_line.text.clone(),
                Style::default()
                    .fg(theme.colors.palette.accent)
                    .add_modifier(Modifier::BOLD),
            )]),
            LineType::Quote => self.render_quote_line(content_line, theme),
            LineType::Code => self.render_code_line(content_line, theme),
            LineType::Link => self.render_link_line(content_line, theme),
            LineType::Bullet => self.render_bullet_line(content_line, theme),
            LineType::Separator => self.render_separator_line(content_line, theme),
            LineType::Attachment => Line::from(vec![
                Span::styled("📎 ", Style::default().fg(theme.colors.palette.warning)),
                Span::styled(
                    content_line.text.clone(),
                    Style::default()
                        .fg(theme.colors.palette.warning)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            LineType::Signature => self.render_signature_line(content_line, theme),
            LineType::Empty => Line::from(""),
            LineType::Body => self.render_body_line(content_line, theme),
        }
    }

    /// Enhanced header line rendering
    fn render_header_line(&self, content_line: &ContentLine, theme: &Theme) -> Line<'static> {
        let text = &content_line.text;

        // Split header into label and value
        if let Some(colon_pos) = text.find(':') {
            let label = text[..colon_pos + 1].to_string();
            let value = text[colon_pos + 1..].trim().to_string();

            Line::from(vec![
                Span::styled(
                    label,
                    Style::default()
                        .fg(theme.colors.content_preview.header)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" ".to_string(), Style::default()),
                Span::styled(
                    value,
                    Style::default().fg(theme.colors.content_preview.body),
                ),
            ])
        } else {
            Line::from(vec![Span::styled(
                content_line.text.clone(),
                Style::default()
                    .fg(theme.colors.content_preview.header)
                    .add_modifier(Modifier::BOLD),
            )])
        }
    }

    /// Enhanced quote line rendering with depth indicators
    fn render_quote_line(&self, content_line: &ContentLine, theme: &Theme) -> Line<'static> {
        let depth = content_line
            .metadata
            .get("depth")
            .and_then(|d| d.parse::<usize>().ok())
            .unwrap_or(1);

        let (quote_color, quote_char) = match depth {
            1 => (theme.colors.palette.info, "▌"),
            2 => (theme.colors.palette.warning, "▐"),
            _ => (theme.colors.palette.text_muted, "│"),
        };

        // Remove the original '>' characters and clean up the text
        let clean_text = content_line
            .text
            .trim_start_matches('>')
            .trim_start()
            .to_string();

        Line::from(vec![
            Span::styled(
                quote_char.to_string(),
                Style::default()
                    .fg(quote_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ".to_string(), Style::default()),
            Span::styled(
                clean_text,
                Style::default()
                    .fg(quote_color)
                    .add_modifier(Modifier::ITALIC),
            ),
        ])
    }

    /// Enhanced code line rendering
    fn render_code_line(&self, content_line: &ContentLine, theme: &Theme) -> Line<'static> {
        let text = &content_line.text;

        // Handle code blocks vs inline code
        if text.trim().starts_with("```") {
            Line::from(vec![Span::styled(
                "┌─ Code ─",
                Style::default()
                    .fg(theme.colors.palette.accent)
                    .add_modifier(Modifier::BOLD),
            )])
        } else {
            Line::from(vec![
                Span::styled("│ ", Style::default().fg(theme.colors.palette.border)),
                Span::styled(
                    text.trim().to_string(),
                    Style::default()
                        .fg(theme.colors.palette.accent)
                        .add_modifier(Modifier::DIM),
                ),
            ])
        }
    }

    /// Enhanced link line rendering with better detection
    fn render_link_line(&self, content_line: &ContentLine, theme: &Theme) -> Line<'static> {
        let text = &content_line.text;
        let mut spans = Vec::new();

        // Try to find and highlight URLs within the text
        let mut last_end = 0;

        for url_match in self.url_regex.find_iter(text) {
            // Add text before the URL
            if url_match.start() > last_end {
                spans.push(Span::styled(
                    text[last_end..url_match.start()].to_string(),
                    Style::default().fg(theme.colors.content_preview.body),
                ));
            }

            // Add the URL with special styling
            spans.push(Span::styled(
                "🔗 ".to_string(),
                Style::default().fg(theme.colors.content_preview.link),
            ));
            spans.push(Span::styled(
                url_match.as_str().to_string(),
                Style::default()
                    .fg(theme.colors.content_preview.link)
                    .add_modifier(Modifier::UNDERLINED | Modifier::BOLD),
            ));

            last_end = url_match.end();
        }

        // Add remaining text
        if last_end < text.len() {
            spans.push(Span::styled(
                text[last_end..].to_string(),
                Style::default().fg(theme.colors.content_preview.body),
            ));
        }

        // If no URLs found, treat as regular link
        if spans.is_empty() {
            spans.push(Span::styled(
                text.clone(),
                Style::default()
                    .fg(theme.colors.content_preview.link)
                    .add_modifier(Modifier::UNDERLINED),
            ));
        }

        Line::from(spans)
    }

    /// Enhanced bullet point rendering
    fn render_bullet_line(&self, content_line: &ContentLine, theme: &Theme) -> Line<'static> {
        let text = &content_line.text;
        let trimmed = text.trim_start();
        let indent = text.len() - trimmed.len();

        // Different bullet styles based on the original character
        let (bullet_char, bullet_style) = if trimmed.starts_with("- ") {
            (
                "▸",
                Style::default()
                    .fg(theme.colors.palette.accent)
                    .add_modifier(Modifier::BOLD),
            )
        } else if trimmed.starts_with("* ") {
            (
                "•",
                Style::default()
                    .fg(theme.colors.palette.success)
                    .add_modifier(Modifier::BOLD),
            )
        } else if trimmed.starts_with("• ") {
            (
                "◦",
                Style::default()
                    .fg(theme.colors.palette.info)
                    .add_modifier(Modifier::BOLD),
            )
        } else if let Some(num_end) = trimmed.find(". ") {
            // Numbered list
            let number = &trimmed[..num_end];
            return Line::from(vec![
                Span::styled(" ".repeat(indent), Style::default()),
                Span::styled(
                    format!("{}.", number),
                    Style::default()
                        .fg(theme.colors.palette.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" ".to_string(), Style::default()),
                Span::styled(
                    trimmed[num_end + 2..].to_string(),
                    Style::default().fg(theme.colors.content_preview.body),
                ),
            ]);
        } else {
            ("▪", Style::default().fg(theme.colors.palette.accent))
        };

        // Extract content after bullet
        let content = if let Some(space_pos) = trimmed.find(' ') {
            &trimmed[space_pos + 1..]
        } else {
            trimmed
        };

        Line::from(vec![
            Span::styled(" ".repeat(indent), Style::default()),
            Span::styled(format!("{} ", bullet_char), bullet_style),
            Span::styled(
                content.to_string(),
                Style::default().fg(theme.colors.content_preview.body),
            ),
        ])
    }

    /// Enhanced separator line rendering
    fn render_separator_line(&self, content_line: &ContentLine, theme: &Theme) -> Line<'static> {
        let text = &content_line.text.trim();

        // Create a stylized separator based on the original characters
        let separator_char = if text.starts_with("---") {
            "─"
        } else if text.starts_with("===") {
            "═"
        } else if text.starts_with("***") {
            "*"
        } else if text.starts_with("...") {
            "·"
        } else {
            "─"
        };

        let separator_width = 40.min(text.len().max(20));
        let separator_line = separator_char.repeat(separator_width);

        Line::from(vec![Span::styled(
            separator_line,
            Style::default()
                .fg(theme.colors.palette.border)
                .add_modifier(Modifier::DIM),
        )])
    }

    /// Enhanced signature line rendering
    fn render_signature_line(&self, content_line: &ContentLine, theme: &Theme) -> Line<'static> {
        let text = &content_line.text;

        // Special handling for classic "--" signature separator
        if text.trim() == "--" || text.trim().starts_with("-- ") {
            Line::from(vec![
                Span::styled("─── ", Style::default().fg(theme.colors.palette.border)),
                Span::styled(
                    "Signature",
                    Style::default()
                        .fg(theme.colors.palette.text_muted)
                        .add_modifier(Modifier::ITALIC),
                ),
                Span::styled(" ───", Style::default().fg(theme.colors.palette.border)),
            ])
        } else {
            Line::from(vec![
                Span::styled("✍ ", Style::default().fg(theme.colors.palette.text_muted)),
                Span::styled(
                    text.clone(),
                    Style::default()
                        .fg(theme.colors.palette.text_muted)
                        .add_modifier(Modifier::ITALIC | Modifier::DIM),
                ),
            ])
        }
    }

    /// Enhanced body line rendering with smart text flow
    fn render_body_line(&self, content_line: &ContentLine, theme: &Theme) -> Line<'static> {
        let text = &content_line.text;

        // Add subtle paragraph indicators for better readability
        if text.trim().is_empty() {
            Line::from("")
        } else if text.len() > 80 {
            // Long lines get a subtle indicator that they continue
            Line::from(vec![
                Span::styled(
                    text.clone(),
                    Style::default().fg(theme.colors.content_preview.body),
                ),
                Span::styled(
                    " ↩",
                    Style::default()
                        .fg(theme.colors.palette.border)
                        .add_modifier(Modifier::DIM),
                ),
            ])
        } else {
            Line::from(vec![Span::styled(
                text.clone(),
                Style::default().fg(theme.colors.content_preview.body),
            )])
        }
    }

    fn render_raw_content_with_styling(
        &mut self,
        content_height: usize,
        theme: &Theme,
    ) -> Vec<Line> {
        let start_line = self.scroll;
        let end_line = (start_line + content_height).min(self.raw_content.len());

        self.raw_content[start_line..end_line]
            .iter()
            .map(|line| {
                // Basic styling for raw content
                if line.starts_with("From:")
                    || line.starts_with("To:")
                    || line.starts_with("Date:")
                    || line.starts_with("Subject:")
                {
                    Line::from(vec![Span::styled(
                        line.clone(),
                        Style::default()
                            .fg(theme.colors.content_preview.header)
                            .add_modifier(Modifier::BOLD),
                    )])
                } else if line.starts_with(">") {
                    Line::from(vec![Span::styled(
                        line.clone(),
                        Style::default().fg(theme.colors.palette.info),
                    )])
                } else if line.starts_with("---") {
                    Line::from(vec![Span::styled(
                        line.clone(),
                        Style::default().fg(theme.colors.palette.border),
                    )])
                } else {
                    Line::from(vec![Span::styled(
                        line.clone(),
                        Style::default().fg(theme.colors.content_preview.body),
                    )])
                }
            })
            .collect()
    }

    /// Handle up key press for scrolling
    pub fn handle_up(&mut self) {
        if self.is_viewing_attachment {
            self.attachment_viewer.scroll_up();
        } else {
            self.scroll_up(1);
        }
    }

    /// Handle down key press for scrolling  
    pub fn handle_down(&mut self) {
        if self.is_viewing_attachment {
            self.attachment_viewer.scroll_down();
        } else {
            self.scroll_down(1);
        }
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll = 0;
    }

    /// Scroll content up by specified lines
    pub fn scroll_up(&mut self, lines: usize) {
        self.scroll = self.scroll.saturating_sub(lines);
    }

    /// Scroll content down by specified lines  
    pub fn scroll_down(&mut self, lines: usize) {
        let max_scroll = self.get_max_scroll(20); // Approximate content height
        self.scroll = (self.scroll.saturating_add(lines)).min(max_scroll);
    }

    /// Scroll content down by specified lines with proper viewport height
    pub fn scroll_down_with_height(&mut self, lines: usize, visible_height: usize) {
        let max_scroll = self.get_max_scroll(visible_height);
        self.scroll = (self.scroll.saturating_add(lines)).min(max_scroll);
    }

    /// Scroll by a full page up
    pub fn page_up(&mut self, visible_height: usize) {
        let page_size = visible_height.saturating_sub(2); // Leave some overlap
        self.scroll_up(page_size);
    }

    /// Scroll by a full page down  
    pub fn page_down(&mut self, visible_height: usize) {
        let page_size = visible_height.saturating_sub(2); // Leave some overlap
        self.scroll_down_with_height(page_size, visible_height);
    }

    /// Scroll to the bottom
    pub fn scroll_to_bottom(&mut self, visible_height: usize) {
        let max_scroll = self.get_max_scroll(visible_height);
        self.scroll = max_scroll;
    }

    /// Get scroll information for indicators
    pub fn get_scroll_info(&self, visible_height: usize) -> (usize, usize, bool, bool) {
        let max_scroll = self.get_max_scroll(visible_height);
        let can_scroll_up = self.scroll > 0;
        let can_scroll_down = self.scroll < max_scroll;
        (self.scroll, max_scroll, can_scroll_up, can_scroll_down)
    }

    /// Get current scroll position
    pub fn get_scroll_position(&self) -> usize {
        self.scroll
    }

    /// Get maximum scroll position based on content
    pub fn get_max_scroll(&self, visible_height: usize) -> usize {
        let total_lines = match self.view_mode {
            ViewMode::Raw => self.raw_content.len(),
            ViewMode::Formatted => {
                if let Some(ref email) = self.email_content {
                    email.parsed_content.len() + 15 // Headers + content + spacing
                } else {
                    self.raw_content.len()
                }
            }
            ViewMode::Html => {
                if let Some(ref email) = self.email_content {
                    email.parsed_content.len() + 15
                } else {
                    self.raw_content.len()
                }
            }
            ViewMode::Headers => {
                if let Some(ref _email) = self.email_content {
                    20 // Estimated header lines
                } else {
                    0
                }
            }
        };

        total_lines.saturating_sub(visible_height)
    }

    pub fn set_content(&mut self, content: Vec<String>) {
        // Check if content contains HTML and clean it
        let cleaned_content: Vec<String> = content
            .iter()
            .map(|line| {
                if crate::html::is_html_content(line) {
                    // Convert HTML to plain text for raw content display
                    let plain_text = self.html_renderer.html_to_plain_text(line);
                    // Split into lines if the conversion created multiple lines
                    plain_text
                        .lines()
                        .map(|l| l.to_string())
                        .collect::<Vec<String>>()
                        .join("\n")
                } else {
                    line.clone()
                }
            })
            .collect();

        self.raw_content = cleaned_content;
        self.scroll = 0; // Reset scroll when content changes
    }

    pub fn clear_content(&mut self) {
        self.raw_content.clear();
        self.email_content = None;
        self.scroll = 0;
    }

    /// Set rich email content with parsed structure
    pub fn set_email_content(&mut self, email_content: EmailContent) {
        // Always use Formatted mode - it handles HTML content properly
        self.view_mode = ViewMode::Formatted;

        // Clear any existing animations before setting new content
        if !self.active_animations.is_empty() {
            let animation_manager = self.animation_manager.clone();
            let animation_ids: Vec<String> = self.active_animations.keys().cloned().collect();

            tokio::spawn(async move {
                if let Some(manager) = animation_manager {
                    for id in animation_ids {
                        let _ = manager.stop_animation(&id).await;
                    }
                }
            });

            self.active_animations.clear();
        }

        // Store the email content
        let html_body = email_content.body.clone();
        self.email_content = Some(email_content);
        self.scroll = 0;

        // Process animations for HTML content in the background
        if self.view_mode == ViewMode::Html || self.view_mode == ViewMode::Formatted {
            self.schedule_animation_processing(html_body);
        }
    }

    pub fn get_email_content(&self) -> Option<&EmailContent> {
        self.email_content.as_ref()
    }

    /// Parse raw email content into structured format
    pub fn parse_email_content(&mut self, raw_email: &str) -> EmailContent {
        let lines: Vec<&str> = raw_email.lines().collect();
        let mut headers = EmailHeader {
            from: String::new(),
            to: Vec::new(),
            cc: Vec::new(),
            bcc: Vec::new(),
            subject: String::new(),
            date: String::new(),
            message_id: String::new(),
            reply_to: None,
            in_reply_to: None,
        };

        let mut body_start = 0;
        let mut in_headers = true;

        // Parse headers
        for (i, line) in lines.iter().enumerate() {
            if in_headers {
                if line.is_empty() {
                    body_start = i + 1;
                    in_headers = false;
                    continue;
                }

                if let Some(colon_pos) = line.find(':') {
                    let header_name = line[..colon_pos].to_lowercase();
                    let header_value = line[colon_pos + 1..].trim().to_string();

                    match header_name.as_str() {
                        "from" => headers.from = header_value,
                        "to" => headers.to = vec![header_value],
                        "cc" => headers.cc = vec![header_value],
                        "bcc" => headers.bcc = vec![header_value],
                        "subject" => headers.subject = header_value,
                        "date" => headers.date = header_value,
                        "message-id" => headers.message_id = header_value,
                        "reply-to" => headers.reply_to = Some(header_value),
                        "in-reply-to" => headers.in_reply_to = Some(header_value),
                        _ => {}
                    }
                }
            }
        }

        // Extract body
        let body = if body_start < lines.len() {
            lines[body_start..].join("\n")
        } else {
            String::new()
        };

        // Parse URLs
        let parsed_urls = self.extract_urls(&body);

        // Parse content into structured lines
        let parsed_content = self.parse_content_lines(&body);

        EmailContent {
            headers,
            body,
            content_type: ContentType::PlainText, // For now, assume plain text
            attachments: Vec::new(),              // TODO: Parse attachments
            parsed_urls,
            parsed_content,
        }
    }

    /// Extract URLs from text content
    fn extract_urls(&self, text: &str) -> Vec<String> {
        self.url_regex
            .find_iter(text)
            .map(|m| m.as_str().to_string())
            .collect()
    }

    /// Parse content into structured lines with type classification
    fn parse_content_lines(&self, body: &str) -> Vec<ContentLine> {
        let mut parsed_lines = Vec::new();
        let lines: Vec<&str> = body.lines().collect();

        for line in lines {
            let line_type = self.classify_line(line);
            let mut metadata = HashMap::new();

            // Add metadata based on line type
            match line_type {
                LineType::Link => {
                    if let Some(url) = self.url_regex.find(line) {
                        metadata.insert("url".to_string(), url.as_str().to_string());
                    }
                }
                LineType::Code => {
                    metadata.insert("language".to_string(), "text".to_string());
                }
                LineType::Quote => {
                    metadata.insert(
                        "depth".to_string(),
                        line.chars().take_while(|&c| c == '>').count().to_string(),
                    );
                }
                _ => {}
            }

            parsed_lines.push(ContentLine {
                text: line.to_string(),
                line_type,
                metadata,
            });
        }

        parsed_lines
    }

    /// Classify a line of content based on its characteristics with enhanced detection
    fn classify_line(&self, line: &str) -> LineType {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            LineType::Empty
        } else if trimmed.starts_with('>') {
            LineType::Quote
        } else if trimmed.starts_with("```")
            || trimmed.starts_with("    ")
            || trimmed.starts_with("\t")
        {
            LineType::Code
        } else if self.is_enhanced_link_line(trimmed) {
            LineType::Link
        } else if self.is_bullet_point(trimmed) {
            LineType::Bullet
        } else if self.is_separator_line(trimmed) {
            LineType::Separator
        } else if self.is_signature_line(line) {
            LineType::Signature
        } else if self.is_header_line(trimmed) {
            LineType::Header
        } else {
            LineType::Body
        }
    }

    /// Enhanced link detection
    fn is_enhanced_link_line(&self, text: &str) -> bool {
        // Check for URLs
        if self.url_regex.is_match(text) {
            return true;
        }

        // Check for email addresses
        if self.email_regex.is_match(text) {
            return true;
        }

        // Check for common link patterns
        let lower = text.to_lowercase();
        lower.starts_with("http")
            || lower.starts_with("www.")
            || lower.starts_with("ftp://")
            || (text.contains("://") && text.len() < 200) // Reasonable URL length
    }

    /// Enhanced bullet point detection
    fn is_bullet_point(&self, text: &str) -> bool {
        let patterns = ["- ", "* ", "• ", "◦ ", "‣ ", "▸ ", "▪ ", "▫ "];
        for pattern in &patterns {
            if text.starts_with(pattern) {
                return true;
            }
        }

        // Check for numbered lists
        if text.len() > 3 {
            let chars: Vec<char> = text.chars().take(5).collect();
            if chars.len() >= 3 {
                // Pattern: "1. ", "12. ", "(1) ", etc.
                if chars[0].is_ascii_digit() {
                    if chars[1] == '.' && chars[2] == ' ' {
                        return true;
                    }
                    if chars.len() >= 4
                        && chars[1].is_ascii_digit()
                        && chars[2] == '.'
                        && chars[3] == ' '
                    {
                        return true;
                    }
                }
                // Pattern: "(1) ", "(a) "
                if chars[0] == '(' && chars.len() >= 4 && chars[2] == ')' && chars[3] == ' ' {
                    return true;
                }
            }
        }

        false
    }

    /// Enhanced separator line detection
    fn is_separator_line(&self, text: &str) -> bool {
        if text.len() < 3 {
            return false;
        }

        // Common separator patterns
        let separators = ["---", "___", "===", "***", "...", "~~~"];
        for sep in &separators {
            if text.starts_with(sep)
                && text
                    .chars()
                    .all(|c| c == sep.chars().next().unwrap() || c.is_whitespace())
            {
                return true;
            }
        }

        // Lines with mostly repeating characters
        if text.len() > 5 {
            let first_char = text.chars().next().unwrap();
            let non_space_chars: Vec<char> = text.chars().filter(|c| !c.is_whitespace()).collect();
            if non_space_chars.len() > 3 && non_space_chars.iter().all(|&c| c == first_char) {
                return true;
            }
        }

        false
    }

    /// Enhanced signature detection
    fn is_signature_line(&self, line: &str) -> bool {
        let trimmed = line.trim();

        // Classic email signature indicators
        if trimmed == "--" || trimmed.starts_with("-- ") {
            return true;
        }

        // Common signature patterns
        let signature_patterns = [
            "best regards",
            "kind regards",
            "sincerely",
            "yours truly",
            "best",
            "cheers",
            "thanks",
            "thank you",
            "sent from",
            "get outlook",
            "confidential",
            "disclaimer",
            "unsubscribe",
            "privacy policy",
        ];

        let lower = trimmed.to_lowercase();
        for pattern in &signature_patterns {
            if lower.contains(pattern) {
                return true;
            }
        }

        // Lines that look like contact info
        if trimmed.len() < 100
            && (trimmed.contains('@')
                || trimmed.contains("phone:")
                || trimmed.contains("tel:")
                || trimmed.contains("mobile:")
                || (trimmed.matches(char::is_numeric).count() > 6 && trimmed.contains('-')))
        {
            return true;
        }

        false
    }

    /// Enhanced header detection for plain text emails
    fn is_header_line(&self, text: &str) -> bool {
        if text.len() > 100 {
            return false; // Headers are usually short
        }

        // Common header patterns in plain text
        let header_patterns = [
            "subject:",
            "from:",
            "to:",
            "cc:",
            "bcc:",
            "date:",
            "reply-to:",
            "message-id:",
            "in-reply-to:",
            "references:",
            "sender:",
        ];

        let lower = text.to_lowercase();
        for pattern in &header_patterns {
            if lower.starts_with(pattern) {
                return true;
            }
        }

        // Lines that end with colon and look like headers
        if text.ends_with(':') && text.len() < 50 && !text.contains(' ') {
            return true;
        }

        false
    }

    /// Toggle between view modes
    pub fn toggle_view_mode(&mut self) {
        let old_mode = self.view_mode;
        self.view_mode = match self.view_mode {
            ViewMode::Raw => ViewMode::Formatted,
            ViewMode::Formatted => ViewMode::Html,
            ViewMode::Html => ViewMode::Headers,
            ViewMode::Headers => ViewMode::Raw,
        };
        self.scroll = 0; // Reset scroll when changing view mode

        // If switching to HTML or Formatted mode, restart animation processing
        if (self.view_mode == ViewMode::Html || self.view_mode == ViewMode::Formatted)
            && (old_mode != ViewMode::Html && old_mode != ViewMode::Formatted)
        {
            if let Some(ref email) = self.email_content {
                self.schedule_animation_processing(email.body.clone());
            }
        }

        // If switching away from HTML/Formatted mode, clear animations
        if (old_mode == ViewMode::Html || old_mode == ViewMode::Formatted)
            && (self.view_mode != ViewMode::Html && self.view_mode != ViewMode::Formatted)
        {
            if !self.active_animations.is_empty() {
                let animation_manager = self.animation_manager.clone();
                let animation_ids: Vec<String> = self.active_animations.keys().cloned().collect();

                tokio::spawn(async move {
                    if let Some(manager) = animation_manager {
                        for id in animation_ids {
                            let _ = manager.stop_animation(&id).await;
                        }
                    }
                });

                self.active_animations.clear();
                tracing::info!("Cleared animations when switching view mode");
            }
        }
    }

    /// Toggle expanded header display
    pub fn toggle_headers(&mut self) {
        self.show_headers_expanded = !self.show_headers_expanded;
    }

    /// Set the database for loading email content
    pub fn set_database(&mut self, database: Arc<EmailDatabase>) {
        self.database = Some(database);
    }

    pub fn set_imap_manager(&mut self, imap_manager: Arc<crate::imap::ImapAccountManager>) {
        self.imap_manager = Some(imap_manager);
    }

    /// Load email content from database by message ID
    pub async fn load_message_by_id(
        &mut self,
        message_id: Uuid,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref database) = self.database {
            self.loading = true;
            self.current_message_id = Some(message_id);

            // Find the message in the database by ID
            if let Some(message) = self.find_message_by_id(database, message_id).await? {
                self.load_stored_message(&message).await?;
            } else {
                self.show_error_message(&format!("Message with ID {} not found", message_id));
            }

            self.loading = false;
        }

        Ok(())
    }

    /// Load email content from a StoredMessage
    pub async fn load_stored_message(
        &mut self,
        message: &StoredMessage,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.current_message_id = Some(message.id);

        // Convert StoredMessage to EmailContent
        let email_content = self.convert_stored_message_to_email_content(message);

        // Check if we have HTML content and load images
        if email_content.content_type == ContentType::Html && !email_content.body.is_empty() {
            if crate::html::is_html_content(&email_content.body) {
                self.load_images_from_html(&email_content.body).await;
            }
        }

        // Set the content
        self.set_email_content(email_content);

        Ok(())
    }

    /// Clear the current message and show empty state
    pub fn clear_message(&mut self) {
        self.email_content = None;
        self.current_message_id = None;
        self.loading = false;
        self.scroll = 0;

        // Show empty state message
        self.raw_content = vec![
            "".to_string(),
            "No message selected".to_string(),
            "".to_string(),
            "Select a message from the list to view its content here.".to_string(),
            "".to_string(),
            "Navigation:".to_string(),
            "  ↑/↓ or j/k  - Scroll content".to_string(),
            "  v           - Toggle view mode".to_string(),
            "  H           - Toggle headers".to_string(),
            "  Home/End    - Jump to top/bottom".to_string(),
        ];
    }

    /// Show an error message in the content area
    fn show_error_message(&mut self, error: &str) {
        self.raw_content = vec![
            "".to_string(),
            "Error loading message".to_string(),
            "".to_string(),
            error.to_string(),
            "".to_string(),
            "Try selecting another message or refreshing the folder.".to_string(),
        ];
        self.email_content = None;
    }

    /// Find a message by ID in the database (helper method)
    async fn find_message_by_id(
        &self,
        database: &EmailDatabase,
        message_id: Uuid,
    ) -> Result<Option<StoredMessage>, Box<dyn std::error::Error>> {
        // Since we don't have a direct "get by ID" method, we'll need to query
        // This is a simplified approach - in practice, we'd want to add an index lookup
        let query = format!("SELECT id, account_id, folder_name, imap_uid, message_id, thread_id, in_reply_to, message_references,
                           subject, from_addr, from_name, to_addrs, cc_addrs, bcc_addrs, reply_to, date,
                           body_text, body_html, attachments,
                           flags, labels, size, priority,
                           created_at, updated_at, last_synced, sync_version, is_draft, is_deleted
                    FROM messages WHERE id = ? AND is_deleted = FALSE");

        let row = sqlx::query(&query)
            .bind(message_id.to_string())
            .fetch_optional(&database.pool)
            .await?;

        match row {
            Some(row) => {
                let message = database.row_to_stored_message(row)?;
                Ok(Some(message))
            }
            None => Ok(None),
        }
    }

    /// Convert a StoredMessage to EmailContent for display
    fn convert_stored_message_to_email_content(&self, message: &StoredMessage) -> EmailContent {
        let headers = EmailHeader {
            from: format!(
                "{} <{}>",
                message.from_name.as_deref().unwrap_or(""),
                message.from_addr
            ),
            to: message.to_addrs.clone(),
            cc: message.cc_addrs.clone(),
            bcc: message.bcc_addrs.clone(),
            subject: message.subject.clone(),
            date: message.date.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            message_id: message.message_id.clone().unwrap_or_default(),
            reply_to: message.reply_to.clone(),
            in_reply_to: message.in_reply_to.clone(),
        };

        // Prefer HTML body if available and has content, otherwise use text body
        // NOTE: Content should already be cleaned by the database layer, so we use it directly
        let (body, content_type) = if let Some(ref html_body) = message.body_html {
            if !html_body.trim().is_empty() {
                // Always treat HTML as HTML regardless of detection - the renderer will handle it
                tracing::debug!(
                    "Content Preview: Using pre-cleaned HTML body (length: {})",
                    html_body.len()
                );
                (html_body.clone(), ContentType::Html)
            } else if let Some(ref text_body) = message.body_text {
                // Use pre-cleaned text body and check if it contains HTML content
                if crate::html::is_html_content(text_body) {
                    tracing::debug!("Content Preview: HTML body empty, but text body contains HTML (length: {})", text_body.len());
                    (text_body.clone(), ContentType::Html)
                } else {
                    tracing::debug!(
                        "Content Preview: HTML body empty, using plain text body (length: {})",
                        text_body.len()
                    );
                    (text_body.clone(), ContentType::PlainText)
                }
            } else {
                tracing::debug!("Content Preview: Only empty HTML body available");
                ("No content available".to_string(), ContentType::PlainText)
            }
        } else if let Some(ref text_body) = message.body_text {
            // Use pre-cleaned text body and check if it contains HTML content
            if crate::html::is_html_content(text_body) {
                tracing::debug!(
                    "Content Preview: No HTML body, but text body contains HTML (length: {})",
                    text_body.len()
                );
                (text_body.clone(), ContentType::Html)
            } else {
                tracing::debug!(
                    "Content Preview: Using plain text body (length: {})",
                    text_body.len()
                );
                (text_body.clone(), ContentType::PlainText)
            }
        } else {
            tracing::debug!("Content Preview: No content available");
            ("No content available".to_string(), ContentType::PlainText)
        };

        // Parse URLs from the body
        let parsed_urls = self.extract_urls(&body);

        // Parse content into structured lines
        let parsed_content = self.parse_content_lines(&body);

        // Convert attachments with enhanced type information
        let attachments = message
            .attachments
            .iter()
            .map(|att| {
                let attachment_info = crate::email::AttachmentInfo::from_stored(att.clone());
                Attachment {
                    filename: attachment_info.display_name.clone(),
                    content_type: att.content_type.clone(),
                    size: att.size as usize,
                    is_inline: att.is_inline,
                }
            })
            .collect();

        EmailContent {
            headers,
            body,
            content_type,
            attachments,
            parsed_urls,
            parsed_content,
        }
    }

    /// Get the current message ID being displayed
    pub fn current_message_id(&self) -> Option<Uuid> {
        self.current_message_id
    }

    /// Check if content is currently loading
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// Check if there is content to display
    pub fn has_content(&self) -> bool {
        self.email_content.is_some() || !self.raw_content.is_empty()
    }

    /// Asynchronously load images from HTML content
    pub async fn load_images_from_html(&mut self, html_content: &str) {
        if !self.image_manager.supports_images() {
            // No point in loading images if terminal doesn't support them
            return;
        }

        let image_refs = extract_images_from_html(html_content);

        for img_ref in image_refs {
            // Skip if already processed
            if self.processed_images.contains_key(&img_ref.src) {
                continue;
            }

            let rendered_content = if img_ref.is_data_url() {
                // Handle base64 embedded images
                if let Some((data, mime_type)) = img_ref.parse_data_url() {
                    match self
                        .image_manager
                        .load_image_from_base64(&data, mime_type.as_deref())
                        .await
                    {
                        Ok(content) => content,
                        Err(e) => {
                            tracing::warn!("Failed to load embedded image: {}", e);
                            self.image_manager.generate_placeholder(
                                img_ref.alt.as_deref(),
                                img_ref.width,
                                img_ref.height,
                            )
                        }
                    }
                } else {
                    self.image_manager.generate_placeholder(
                        img_ref.alt.as_deref(),
                        img_ref.width,
                        img_ref.height,
                    )
                }
            } else if img_ref.is_http_url() {
                // Handle remote images
                match self.image_manager.load_image_from_url(&img_ref.src).await {
                    Ok(content) => content,
                    Err(e) => {
                        tracing::warn!("Failed to load remote image {}: {}", img_ref.src, e);
                        self.image_manager.generate_placeholder(
                            img_ref.alt.as_deref(),
                            img_ref.width,
                            img_ref.height,
                        )
                    }
                }
            } else {
                // Relative URLs or other unsupported formats
                self.image_manager.generate_placeholder(
                    img_ref.alt.as_deref(),
                    img_ref.width,
                    img_ref.height,
                )
            };

            // Cache the rendered content
            self.processed_images
                .insert(img_ref.src.clone(), rendered_content);
        }
    }

    /// Clear image cache
    pub async fn clear_image_cache(&mut self) {
        self.processed_images.clear();
        self.image_manager.clear_cache().await;
    }

    /// Get image display capability information
    pub fn get_image_capabilities(&self) -> (bool, String) {
        let supports = self.image_manager.supports_images();
        let protocol = match self.image_manager.protocol() {
            crate::images::TerminalProtocol::Kitty => "Kitty Graphics",
            crate::images::TerminalProtocol::Sixel => "Sixel Graphics",
            crate::images::TerminalProtocol::None => "None (ASCII placeholders only)",
        };
        (supports, protocol.to_string())
    }

    /// Update image manager dimensions based on current area
    pub fn update_image_dimensions(&mut self, area: Rect) {
        // Convert terminal area to approximate character dimensions
        let char_width = area.width.saturating_sub(4); // Account for borders and padding
        let char_height = area.height.saturating_sub(6); // Account for headers and borders

        self.image_manager
            .set_max_dimensions(char_width as u32, char_height as u32);

        // Also update HTML renderer max width for proper text wrapping
        self.html_renderer.max_width = char_width as usize;
    }

    /// Navigate to the next attachment
    pub fn next_attachment(&mut self) {
        if let Some(ref email) = self.email_content {
            if !email.attachments.is_empty() {
                self.selected_attachment = Some(match self.selected_attachment {
                    Some(current) => {
                        if current + 1 < email.attachments.len() {
                            current + 1
                        } else {
                            0 // Wrap to first attachment
                        }
                    }
                    None => 0, // Select first attachment
                });
            }
        }
    }

    /// Navigate to the previous attachment
    pub fn previous_attachment(&mut self) {
        if let Some(ref email) = self.email_content {
            if !email.attachments.is_empty() {
                self.selected_attachment = Some(match self.selected_attachment {
                    Some(current) => {
                        if current > 0 {
                            current - 1
                        } else {
                            email.attachments.len() - 1 // Wrap to last attachment
                        }
                    }
                    None => email.attachments.len() - 1, // Select last attachment
                });
            }
        }
    }

    /// Select the first attachment
    pub fn select_first_attachment(&mut self) {
        if let Some(ref email) = self.email_content {
            if !email.attachments.is_empty() {
                self.selected_attachment = Some(0);
            }
        }
    }

    /// Clear attachment selection
    pub fn clear_attachment_selection(&mut self) {
        self.selected_attachment = None;
    }

    /// Get information about the currently selected attachment
    pub fn get_selected_attachment(&self) -> Option<&Attachment> {
        if let (Some(ref email), Some(index)) = (&self.email_content, self.selected_attachment) {
            email.attachments.get(index)
        } else {
            None
        }
    }

    /// Check if any attachments are available
    pub fn has_attachments(&self) -> bool {
        if let Some(ref email) = self.email_content {
            !email.attachments.is_empty()
        } else {
            false
        }
    }

    /// Get the count of attachments
    pub fn attachment_count(&self) -> usize {
        if let Some(ref email) = self.email_content {
            email.attachments.len()
        } else {
            0
        }
    }

    /// Save the currently selected attachment
    pub async fn save_selected_attachment(
        &self,
        save_path: Option<std::path::PathBuf>,
    ) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        if let Some(attachment) = self.get_selected_attachment() {
            self.save_attachment(attachment, save_path).await
        } else {
            Err("No attachment selected".into())
        }
    }

    /// Save a specific attachment to the specified path or default downloads location
    pub async fn save_attachment(
        &self,
        attachment: &Attachment,
        save_path: Option<std::path::PathBuf>,
    ) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        let target_path = if let Some(path) = save_path {
            // Use provided path
            if path.is_dir() {
                path.join(&attachment.filename)
            } else {
                path
            }
        } else {
            // Use default downloads directory
            let downloads_dir = self.get_downloads_directory()?;
            std::fs::create_dir_all(&downloads_dir)?;
            downloads_dir.join(&attachment.filename)
        };

        // Check if we need to download the attachment data
        let attachment_data = self.get_attachment_data(attachment).await?;

        // Write the data to file
        std::fs::write(&target_path, attachment_data)?;

        Ok(target_path)
    }

    /// Get the default downloads directory
    fn get_downloads_directory(&self) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        // Try to find standard downloads directory
        if let Some(home_dir) = dirs::home_dir() {
            let downloads = home_dir.join("Downloads");
            if downloads.exists() {
                return Ok(downloads);
            }

            // Fallback to creating a comunicado downloads folder in home
            let comunicado_downloads = home_dir.join("comunicado-downloads");
            return Ok(comunicado_downloads);
        }

        // Ultimate fallback to current directory
        Ok(std::path::PathBuf::from("./downloads"))
    }

    /// Get attachment data (either from memory or by downloading from IMAP server)
    async fn get_attachment_data(
        &self,
        attachment: &Attachment,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // First, try to find the attachment in the current message's stored attachments
        if let (Some(ref database), Some(message_id)) = (&self.database, self.current_message_id) {
            if let Some(stored_message) = self.find_message_by_id(database, message_id).await? {
                // Find the matching stored attachment
                for stored_attachment in &stored_message.attachments {
                    if stored_attachment.filename == attachment.filename {
                        if let Some(ref data) = stored_attachment.data {
                            // Attachment data is already stored
                            return Ok(data.clone());
                        } else if let Some(ref file_path) = stored_attachment.file_path {
                            // Attachment is stored as a file
                            return Ok(std::fs::read(file_path)?);
                        }

                        // If we get here, we need to download the attachment from IMAP
                        return self
                            .download_attachment_from_imap(&stored_message, stored_attachment)
                            .await;
                    }
                }
            }
        }

        Err("Attachment data not found and cannot download from IMAP".into())
    }

    /// Download attachment data from IMAP server
    async fn download_attachment_from_imap(
        &self,
        message: &StoredMessage,
        attachment: &crate::email::StoredAttachment,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        if let Some(ref imap_manager) = self.imap_manager {
            tracing::info!(
                "Downloading attachment '{}' from IMAP for message UID {}",
                attachment.filename,
                message.imap_uid
            );

            // For now, we'll use the attachment ID as a simple part identifier
            // In a full implementation, we'd parse BODYSTRUCTURE to get the correct part
            let attachment_part = if attachment.id.contains('.') {
                attachment.id.clone()
            } else {
                // Assume it's part 2 for the first attachment (part 1 is usually the body)
                "2".to_string()
            };

            match imap_manager
                .fetch_attachment_data(
                    &message.account_id,
                    &message.folder_name,
                    message.imap_uid,
                    &attachment_part,
                )
                .await
            {
                Ok(attachment_data) => {
                    tracing::info!(
                        "Successfully downloaded attachment '{}': {} bytes",
                        attachment.filename,
                        attachment_data.len()
                    );
                    Ok(attachment_data)
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to download attachment '{}' from IMAP: {}",
                        attachment.filename,
                        e
                    );
                    Err(format!("Failed to download attachment from IMAP: {}", e).into())
                }
            }
        } else {
            tracing::error!("IMAP manager not available for attachment download");
            Err("IMAP manager not available for attachment download".into())
        }
    }

    /// Open the attachment viewer for the selected attachment
    pub async fn view_selected_attachment(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(attachment) = self.get_selected_attachment() {
            // Get attachment data
            let attachment_data = self.get_attachment_data(attachment).await?;

            // Convert to AttachmentInfo
            let stored_attachment =
                self.find_stored_attachment_by_filename(&attachment.filename)?;
            let attachment_info = AttachmentInfo::from_stored(stored_attachment);

            // View in attachment viewer
            let _result = self
                .attachment_viewer
                .view_attachment(&attachment_info, &attachment_data)
                .await;
            self.is_viewing_attachment = true;

            Ok(())
        } else {
            Err("No attachment selected".into())
        }
    }

    /// Close the attachment viewer and return to content view
    pub fn close_attachment_viewer(&mut self) {
        self.is_viewing_attachment = false;
        self.attachment_viewer.clear();
    }

    /// Open the selected attachment with the system default application (xdg-open)
    pub async fn open_attachment_with_system(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(attachment) = self.get_selected_attachment() {
            // Get attachment data
            let attachment_data = self.get_attachment_data(attachment).await?;

            // Create a temporary file for the attachment
            let temp_dir = std::env::temp_dir();
            let temp_path = temp_dir.join(&attachment.filename);

            // Write attachment data to temporary file
            std::fs::write(&temp_path, &attachment_data)?;

            // Use xdg-open to open the file with system default application
            let output = std::process::Command::new("xdg-open")
                .arg(&temp_path)
                .output();

            match output {
                Ok(result) => {
                    if result.status.success() {
                        tracing::info!(
                            "Successfully opened attachment '{}' with system application",
                            attachment.filename
                        );
                        Ok(())
                    } else {
                        let error_msg = String::from_utf8_lossy(&result.stderr);
                        Err(format!("xdg-open failed: {}", error_msg).into())
                    }
                }
                Err(e) => Err(format!("Failed to execute xdg-open: {}", e).into()),
            }
        } else {
            Err("No attachment selected".into())
        }
    }

    /// Check if we're currently viewing an attachment
    pub fn is_viewing_attachment(&self) -> bool {
        self.is_viewing_attachment
    }

    /// Handle key input for attachment viewer
    pub async fn handle_attachment_viewer_key(
        &mut self,
        key: char,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        match key {
            'q' => {
                self.close_attachment_viewer();
                Ok(true)
            }
            't' => {
                self.attachment_viewer.switch_to_text_mode().await?;
                Ok(true)
            }
            's' => {
                if let Some(attachment_info) = self.attachment_viewer.current_attachment() {
                    let path = self
                        .save_attachment_from_viewer(attachment_info, None)
                        .await?;
                    tracing::info!("Attachment saved to: {:?}", path);
                }
                Ok(true)
            }
            _ => Ok(false), // Key not handled
        }
    }

    /// Save attachment from the viewer
    async fn save_attachment_from_viewer(
        &self,
        attachment_info: &AttachmentInfo,
        save_path: Option<std::path::PathBuf>,
    ) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        // Convert AttachmentInfo back to the format expected by save_attachment
        let attachment = Attachment {
            filename: attachment_info.display_name.clone(),
            content_type: attachment_info.stored.content_type.clone(),
            size: attachment_info.stored.size as usize,
            is_inline: attachment_info.stored.is_inline,
        };

        self.save_attachment(&attachment, save_path).await
    }

    /// Find stored attachment by filename
    fn find_stored_attachment_by_filename(
        &self,
        filename: &str,
    ) -> Result<crate::email::StoredAttachment, Box<dyn std::error::Error>> {
        if let (Some(_database), Some(_message_id)) = (&self.database, self.current_message_id) {
            // This is a simplified approach - in practice we'd need async access to the database
            // For now, we'll create a basic StoredAttachment from the display data
            let stored = crate::email::StoredAttachment {
                id: uuid::Uuid::new_v4().to_string(),
                filename: filename.to_string(),
                content_type: "application/octet-stream".to_string(), // Default
                size: 0,
                content_id: None,
                is_inline: false,
                data: None,
                file_path: None,
            };

            Ok(stored)
        } else {
            Err("No database connection or message ID available".into())
        }
    }

    /// Copy email content to clipboard
    pub fn copy_email_content(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref email) = self.email_content {
            let content_to_copy = match self.view_mode {
                ViewMode::Html => {
                    // For HTML mode, copy plain text version
                    self.html_renderer.html_to_plain_text(&email.body)
                }
                ViewMode::Formatted | ViewMode::Raw => email.body.clone(),
                ViewMode::Headers => {
                    // Copy headers + body
                    format!(
                        "From: {}\nTo: {}\nSubject: {}\nDate: {}\n\n{}",
                        email.headers.from,
                        email.headers.to.join(", "),
                        email.headers.subject,
                        email.headers.date,
                        email.body
                    )
                }
            };

            self.clipboard_manager.copy(&content_to_copy)?;
            tracing::info!(
                "Copied email content to clipboard ({} characters)",
                content_to_copy.len()
            );
        }
        Ok(())
    }

    /// Copy selected attachment name/path to clipboard  
    pub fn copy_attachment_info(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(attachment_index) = self.selected_attachment {
            if let Some(ref email) = self.email_content {
                if let Some(attachment) = email.attachments.get(attachment_index) {
                    let info = format!("{} ({})", attachment.filename, attachment.content_type);
                    self.clipboard_manager.copy(&info)?;
                    tracing::info!("Copied attachment info to clipboard: {}", info);
                }
            }
        }
        Ok(())
    }

    /// Check if clipboard is available
    pub fn is_clipboard_available(&self) -> bool {
        self.clipboard_manager.is_available()
    }

    /// Initialize animation manager
    pub fn initialize_animation_manager(&mut self) {
        if self.animation_manager.is_none() {
            let image_manager = Arc::new(self.image_manager.clone());
            self.animation_manager = Some(Arc::new(AnimationManager::new(image_manager)));
        }
    }

    /// Load and start playing a GIF animation from URL
    pub async fn load_and_play_gif_animation(
        &mut self,
        url: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.initialize_animation_manager();

        if let Some(ref animation_manager) = self.animation_manager {
            // Load the GIF animation
            let animation_id = animation_manager.load_gif_from_url(url).await?;

            // Start playing the animation
            let mut receiver = animation_manager
                .play_animation(&animation_id, true)
                .await?;

            // Spawn a task to handle animation frames
            let active_animations =
                Arc::new(tokio::sync::RwLock::new(self.active_animations.clone()));
            let animation_id_clone = animation_id.clone();

            tokio::spawn(async move {
                while let Some(result) = receiver.recv().await {
                    match result {
                        crate::animation::AnimationResult::Frame {
                            id: _,
                            frame_data,
                            frame_index: _,
                        } => {
                            let mut animations = active_animations.write().await;
                            animations.insert(animation_id_clone.clone(), frame_data);
                        }
                        crate::animation::AnimationResult::Finished { id: _ } => {
                            tracing::info!("Animation finished: {}", animation_id_clone);
                            break;
                        }
                        crate::animation::AnimationResult::Error { id: _, error } => {
                            tracing::error!("Animation error: {}", error);
                            break;
                        }
                    }
                }
            });

            tracing::info!("Started GIF animation: {}", animation_id);
        }

        Ok(())
    }

    /// Load and start playing a GIF animation from base64 data
    pub async fn load_and_play_gif_base64(
        &mut self,
        data: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.initialize_animation_manager();

        if let Some(ref animation_manager) = self.animation_manager {
            // Load the GIF animation from base64
            let animation_id = animation_manager.load_gif_from_base64(data).await?;

            // Start playing the animation
            let mut receiver = animation_manager
                .play_animation(&animation_id, true)
                .await?;

            // Spawn a task to handle animation frames
            let active_animations =
                Arc::new(tokio::sync::RwLock::new(self.active_animations.clone()));
            let animation_id_clone = animation_id.clone();

            tokio::spawn(async move {
                while let Some(result) = receiver.recv().await {
                    match result {
                        crate::animation::AnimationResult::Frame {
                            id: _,
                            frame_data,
                            frame_index: _,
                        } => {
                            let mut animations = active_animations.write().await;
                            animations.insert(animation_id_clone.clone(), frame_data);
                        }
                        crate::animation::AnimationResult::Finished { id: _ } => {
                            tracing::info!("Animation finished: {}", animation_id_clone);
                            break;
                        }
                        crate::animation::AnimationResult::Error { id: _, error } => {
                            tracing::error!("Animation error: {}", error);
                            break;
                        }
                    }
                }
            });

            tracing::info!("Started GIF animation from base64 data: {}", animation_id);
        }

        Ok(())
    }

    /// Stop a playing animation
    pub async fn stop_animation(
        &mut self,
        animation_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref animation_manager) = self.animation_manager {
            animation_manager.stop_animation(animation_id).await?;
            self.active_animations.remove(animation_id);
            tracing::info!("Stopped animation: {}", animation_id);
        }
        Ok(())
    }

    /// Get the current frame data for an active animation
    pub fn get_animation_frame(&self, animation_id: &str) -> Option<&String> {
        self.active_animations.get(animation_id)
    }

    /// Clear all active animations
    pub async fn clear_all_animations(&mut self) {
        if let Some(ref animation_manager) = self.animation_manager {
            for animation_id in self.active_animations.keys() {
                let _ = animation_manager.stop_animation(animation_id).await;
            }
        }
        self.active_animations.clear();
        tracing::info!("Cleared all active animations");
    }

    /// Process HTML content and automatically load GIF animations
    pub async fn process_html_animations(
        &mut self,
        html_content: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use crate::images::extract_images_from_html;

        let images = extract_images_from_html(html_content);

        for image_ref in images {
            // Check if it's a GIF image
            if image_ref.src.to_lowercase().contains(".gif") {
                if image_ref.is_http_url() {
                    // Load GIF from URL
                    if let Err(e) = self.load_and_play_gif_animation(&image_ref.src).await {
                        tracing::warn!(
                            "Failed to load GIF animation from {}: {}",
                            image_ref.src,
                            e
                        );
                    }
                } else if image_ref.is_data_url() {
                    // Load GIF from data URL
                    if let Some((data, mime_type)) = image_ref.parse_data_url() {
                        if mime_type.as_deref() == Some("image/gif") {
                            if let Err(e) = self.load_and_play_gif_base64(&data).await {
                                tracing::warn!("Failed to load GIF animation from data URL: {}", e);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Check if animations are supported in the current terminal
    pub fn animations_supported(&self) -> bool {
        if let Some(ref animation_manager) = self.animation_manager {
            animation_manager.supports_animations()
        } else {
            false
        }
    }

    /// Schedule animation processing for HTML content in the background
    fn schedule_animation_processing(&mut self, html_body: String) {
        // Only process if we have an animation manager and animations are supported
        if let Some(animation_manager) = self.animation_manager.clone() {
            if animation_manager.supports_animations() {
                // Create a channel to communicate animation updates back to the main thread
                let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();

                // Spawn the animation processing task
                tokio::spawn(async move {
                    // Extract images from HTML and check for GIFs
                    use crate::images::extract_images_from_html;
                    let images = extract_images_from_html(&html_body);

                    for image_ref in images {
                        // Check if it's a GIF image
                        if image_ref.src.to_lowercase().contains(".gif") {
                            let animation_id = if image_ref.is_http_url() {
                                // Load GIF from URL
                                match animation_manager.load_gif_from_url(&image_ref.src).await {
                                    Ok(id) => Some(id),
                                    Err(e) => {
                                        tracing::warn!(
                                            "Failed to load GIF animation from {}: {}",
                                            image_ref.src,
                                            e
                                        );
                                        None
                                    }
                                }
                            } else if image_ref.is_data_url() {
                                // Extract base64 data and load GIF
                                if let Some((data, _mime_type)) = image_ref.parse_data_url() {
                                    match animation_manager.load_gif_from_base64(&data).await {
                                        Ok(id) => Some(id),
                                        Err(e) => {
                                            tracing::warn!(
                                                "Failed to load GIF animation from data URL: {}",
                                                e
                                            );
                                            None
                                        }
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            };

                            // Start playing the animation if loaded successfully
                            if let Some(id) = animation_id {
                                match animation_manager.play_animation(&id, true).await {
                                    Ok(mut animation_receiver) => {
                                        // Forward animation frames to the main channel
                                        let id_clone = id.clone();
                                        let sender_clone = sender.clone();
                                        tokio::spawn(async move {
                                            while let Some(result) = animation_receiver.recv().await
                                            {
                                                if sender_clone
                                                    .send((id_clone.clone(), result))
                                                    .is_err()
                                                {
                                                    break; // Channel closed
                                                }
                                            }
                                        });
                                        tracing::info!("Started GIF animation: {}", id);
                                    }
                                    Err(e) => {
                                        tracing::warn!(
                                            "Failed to start GIF animation {}: {}",
                                            id,
                                            e
                                        );
                                    }
                                }
                            }
                        }
                    }
                });

                // Store the receiver for processing animation updates
                // Note: In a real implementation, we'd need a better way to handle this
                // For now, we'll rely on the animation processing methods being called
                tracing::info!("Scheduled animation processing for HTML content");
            }
        }
    }

    /// Replace GIF placeholders in rendered content with animation frames
    pub fn render_animations_in_content(&self, content: &str) -> String {
        if self.active_animations.is_empty() {
            return content.to_string();
        }

        let mut rendered_content = content.to_string();

        // Replace GIF placeholders with current animation frames
        for (animation_id, frame_data) in &self.active_animations {
            // Look for placeholder patterns that might contain this animation
            let placeholder_patterns = [
                format!("[GIF: {}]", animation_id),
                format!("[Loading GIF: {}]", animation_id),
                format!("[Animation: {}]", animation_id),
            ];

            for pattern in &placeholder_patterns {
                if rendered_content.contains(pattern) {
                    rendered_content = rendered_content.replace(pattern, frame_data);
                }
            }
        }

        rendered_content
    }

    /// Filter out raw email headers from content lines
    fn filter_raw_headers(&self, content_lines: &[String]) -> Vec<String> {
        let mut filtered_lines = Vec::new();
        let mut skip_headers = true;
        let mut found_content_start = false;
        
        // Common email headers to filter out
        let header_patterns = [
            "Message-ID:", "Date:", "From:", "To:", "Subject:", "Mime-Version:",
            "Content-Type:", "Content-Transfer-Encoding:", "X-", "Received:",
            "Return-Path:", "Delivered-To:", "Authentication-Results:",
            "DKIM-Signature:", "Reply-To:", "Cc:", "Bcc:", "In-Reply-To:",
            "References:", "Thread-Topic:", "Thread-Index:", "Precedence:",
            "List-", "Organization:", "User-Agent:", "X-Mailer:", "Content-",
            "MIME-", "Boundary=", "charset=", "name=", "filename=",
        ];
        
        for line in content_lines {
            let trimmed = line.trim();
            
            // Skip empty lines at the start
            if skip_headers && trimmed.is_empty() {
                continue;
            }
            
            // Check if this line looks like a header
            let is_header = header_patterns.iter().any(|pattern| {
                trimmed.starts_with(pattern) || 
                trimmed.to_lowercase().starts_with(&pattern.to_lowercase())
            });
            
            // Also check for header continuation lines (starting with whitespace)
            let is_header_continuation = skip_headers && 
                (trimmed.starts_with(' ') || trimmed.starts_with('\t')) &&
                !found_content_start;
            
            if skip_headers && (is_header || is_header_continuation) {
                continue;
            }
            
            // If we find a line that doesn't look like a header, start including content
            if skip_headers && !is_header && !is_header_continuation && !trimmed.is_empty() {
                skip_headers = false;
                found_content_start = true;
            }
            
            // Include this line if we're past the headers
            if !skip_headers {
                filtered_lines.push(line.clone());
            }
        }
        
        filtered_lines
    }

    /// Filter out raw email headers from ContentLine objects
    fn filter_raw_headers_from_content(&self, content_lines: &[ContentLine]) -> Vec<ContentLine> {
        let mut filtered_lines = Vec::new();
        let mut skip_headers = true;
        let mut found_content_start = false;
        
        // Common email headers to filter out
        let header_patterns = [
            "Message-ID:", "Date:", "From:", "To:", "Subject:", "Return-Path:",
            "Received:", "Authentication-Results:", "ARC-", "Delivered-To:",
            "DKIM-Signature:", "Reply-To:", "Cc:", "Bcc:", "In-Reply-To:",
            "References:", "Thread-Topic:", "Thread-Index:", "Precedence:",
            "List-", "Organization:", "User-Agent:", "X-Mailer:", "Content-",
            "MIME-", "Boundary=", "charset=", "name=", "filename=",
        ];
        
        for content_line in content_lines {
            let trimmed = content_line.text.trim();
            
            // Skip empty lines at the start
            if skip_headers && trimmed.is_empty() {
                continue;
            }
            
            // Check if this line looks like a header
            let is_header = header_patterns.iter().any(|pattern| {
                trimmed.starts_with(pattern) || 
                trimmed.to_lowercase().starts_with(&pattern.to_lowercase())
            });
            
            // Also check for header continuation lines (starting with whitespace)
            let is_header_continuation = skip_headers && 
                (trimmed.starts_with(' ') || trimmed.starts_with('\t')) &&
                !found_content_start;
            
            if skip_headers && (is_header || is_header_continuation) {
                continue;
            }
            
            // If we find a line that doesn't look like a header, start including content
            if skip_headers && !is_header && !is_header_continuation && !trimmed.is_empty() {
                skip_headers = false;
                found_content_start = true;
            }
            
            // Include this line if we're past the headers
            if !skip_headers {
                filtered_lines.push(content_line.clone());
            }
        }
        
        filtered_lines
    }
}

impl Default for ContentPreview {
    fn default() -> Self {
        Self::new()
    }
}
