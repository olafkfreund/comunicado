use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Wrap},
    Frame,
};
use crate::theme::Theme;
use regex::Regex;
use std::collections::HashMap;

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
    Raw,        // Show raw email content
    Formatted,  // Show formatted/processed content
    Headers,    // Show headers only
}

pub struct ContentPreview {
    email_content: Option<EmailContent>,
    raw_content: Vec<String>,
    scroll: usize,
    view_mode: ViewMode,
    show_headers_expanded: bool,
    url_regex: Regex,
    email_regex: Regex,
}

impl ContentPreview {
    pub fn new() -> Self {
        let url_regex = Regex::new(r"https?://[^\s]+").unwrap();
        let email_regex = Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").unwrap();
        
        let mut preview = Self {
            email_content: None,
            raw_content: Vec::new(),
            scroll: 0,
            view_mode: ViewMode::Formatted,
            show_headers_expanded: false,
            url_regex,
            email_regex,
        };
        
        // Initialize with sample content
        preview.initialize_sample_content();
        
        preview
    }

    fn initialize_sample_content(&mut self) {
        self.raw_content = vec![
            "From: Comunicado Team <team@comunicado.dev>".to_string(),
            "To: user@example.com".to_string(),
            "Date: Today 10:30 AM".to_string(),
            "Subject: Welcome to Comunicado!".to_string(),
            "".to_string(),
            "Welcome to Comunicado - the modern TUI email client!".to_string(),
            "".to_string(),
            "We're excited to have you try out our terminal-based email experience.".to_string(),
            "Comunicado brings modern email features directly to your terminal with:".to_string(),
            "".to_string(),
            "Modern TUI Interface".to_string(),
            "   - Clean, intuitive design with ratatui".to_string(),
            "   - Vim-style keyboard navigation".to_string(),
            "   - Responsive three-pane layout".to_string(),
            "".to_string(),
            "Rich Email Support".to_string(),
            "   - HTML email rendering (coming soon)".to_string(),
            "   - Image and animation display".to_string(),
            "   - Attachment handling".to_string(),
            "".to_string(),
            "Secure Authentication".to_string(),
            "   - OAuth2 support for major providers".to_string(),
            "   - Multi-account management".to_string(),
            "   - Local email storage with Maildir".to_string(),
            "".to_string(),
            "Integrated Calendar (upcoming)".to_string(),
            "   - CalDAV synchronization".to_string(),
            "   - Meeting invitation handling".to_string(),
            "   - Desktop environment integration".to_string(),
            "".to_string(),
            "Getting Started".to_string(),
            "".to_string(),
            "Use these keyboard shortcuts to navigate:".to_string(),
            "".to_string(),
            "Navigation:".to_string(),
            "  Tab / Shift+Tab  - Switch between panes".to_string(),
            "  h/j/k/l          - Vim-style movement".to_string(),
            "  ↑/↓              - Move up/down in lists".to_string(),
            "  Enter            - Select/expand items".to_string(),
            "".to_string(),
            "Global:".to_string(),
            "  q                - Quit application".to_string(),
            "  Ctrl+C           - Force quit".to_string(),
            "".to_string(),
            "This is just the beginning! We're actively developing new features".to_string(),
            "including HTML email rendering, OAuth2 authentication, and".to_string(),
            "integrated calendar functionality.".to_string(),
            "".to_string(),
            "Thank you for trying Comunicado!".to_string(),
            "".to_string(),
            "Best regards,".to_string(),
            "The Comunicado Development Team".to_string(),
            "".to_string(),
            "---".to_string(),
            "This is a sample email displayed in the content preview pane.".to_string(),
            "In the full implementation, this area will show:".to_string(),
            "- Rendered HTML emails with proper formatting".to_string(),
            "- Images and animations from supported terminals".to_string(),
            "- Interactive elements like links and attachments".to_string(),
            "- Scrollable content for long messages".to_string(),
        ];
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, block: Block, is_focused: bool, theme: &Theme) {
        let content_height = area.height.saturating_sub(2) as usize; // Account for block borders
        
        let lines = match self.view_mode {
            ViewMode::Raw => self.render_raw_content(content_height, theme),
            ViewMode::Formatted => self.render_formatted_content(content_height, theme),
            ViewMode::Headers => self.render_headers_only(content_height, theme),
        };

        // Create scroll indicator if content is scrollable
        let total_lines = match self.view_mode {
            ViewMode::Raw => self.raw_content.len(),
            ViewMode::Formatted => {
                if let Some(ref email) = self.email_content {
                    email.parsed_content.len() + 10 // Headers + content
                } else {
                    self.raw_content.len()
                }
            }
            ViewMode::Headers => {
                if let Some(ref email) = self.email_content {
                    8 // Typical header count
                } else {
                    0
                }
            }
        };

        let scroll_indicator = if total_lines > content_height {
            let position = (self.scroll as f32 / (total_lines - content_height) as f32 * 100.0) as u16;
            format!(" ({}%)", position)
        } else {
            String::new()
        };

        let view_mode_indicator = match self.view_mode {
            ViewMode::Raw => " [Raw]",
            ViewMode::Formatted => " [Formatted]", 
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
    
    fn render_raw_content(&self, content_height: usize, theme: &Theme) -> Vec<Line> {
        let start_line = self.scroll;
        let end_line = (start_line + content_height).min(self.raw_content.len());
        
        self.raw_content[start_line..end_line]
            .iter()
            .map(|line| {
                Line::from(vec![
                    Span::styled(line.clone(), Style::default().fg(theme.colors.content_preview.body))
                ])
            })
            .collect()
    }
    
    fn render_formatted_content(&self, content_height: usize, theme: &Theme) -> Vec<Line> {
        if let Some(ref email) = self.email_content {
            let mut all_lines = Vec::new();
            
            // Add headers if requested
            if self.show_headers_expanded {
                all_lines.extend(self.render_email_headers(&email.headers, theme));
                all_lines.push(Line::from("")); // Empty line separator
            } else {
                // Show compact headers
                all_lines.push(Line::from(vec![
                    Span::styled("From: ", Style::default().fg(theme.colors.content_preview.header).add_modifier(Modifier::BOLD)),
                    Span::styled(email.headers.from.clone(), Style::default().fg(theme.colors.content_preview.body)),
                ]));
                all_lines.push(Line::from(vec![
                    Span::styled("Subject: ", Style::default().fg(theme.colors.content_preview.header).add_modifier(Modifier::BOLD)),
                    Span::styled(email.headers.subject.clone(), Style::default().fg(theme.colors.content_preview.body)),
                ]));
                all_lines.push(Line::from("")); // Empty line separator
            }
            
            // Add formatted content lines
            for content_line in &email.parsed_content {
                all_lines.push(self.render_content_line(content_line, theme));
            }
            
            // Apply scrolling
            let start_line = self.scroll;
            let end_line = (start_line + content_height).min(all_lines.len());
            
            all_lines[start_line..end_line].to_vec()
        } else {
            // Fallback to raw content with basic styling
            self.render_raw_content_with_styling(content_height, theme)
        }
    }
    
    fn render_headers_only(&self, content_height: usize, theme: &Theme) -> Vec<Line> {
        if let Some(ref email) = self.email_content {
            let header_lines = self.render_email_headers(&email.headers, theme);
            let start_line = self.scroll;
            let end_line = (start_line + content_height).min(header_lines.len());
            
            header_lines[start_line..end_line].to_vec()
        } else {
            vec![Line::from(vec![
                Span::styled("No email content available", 
                    Style::default().fg(theme.colors.palette.text_muted))
            ])]
        }
    }
    
    fn render_email_headers(&self, headers: &EmailHeader, theme: &Theme) -> Vec<Line> {
        let mut lines = Vec::new();
        
        if !headers.from.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("From: ", Style::default().fg(theme.colors.content_preview.header).add_modifier(Modifier::BOLD)),
                Span::styled(headers.from.clone(), Style::default().fg(theme.colors.content_preview.body)),
            ]));
        }
        
        if !headers.to.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("To: ", Style::default().fg(theme.colors.content_preview.header).add_modifier(Modifier::BOLD)),
                Span::styled(headers.to.join(", "), Style::default().fg(theme.colors.content_preview.body)),
            ]));
        }
        
        if !headers.cc.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("CC: ", Style::default().fg(theme.colors.content_preview.header).add_modifier(Modifier::BOLD)),
                Span::styled(headers.cc.join(", "), Style::default().fg(theme.colors.content_preview.body)),
            ]));
        }
        
        if !headers.date.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("Date: ", Style::default().fg(theme.colors.content_preview.header).add_modifier(Modifier::BOLD)),
                Span::styled(headers.date.clone(), Style::default().fg(theme.colors.content_preview.body)),
            ]));
        }
        
        if !headers.subject.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("Subject: ", Style::default().fg(theme.colors.content_preview.header).add_modifier(Modifier::BOLD)),
                Span::styled(headers.subject.clone(), Style::default().fg(theme.colors.content_preview.body)),
            ]));
        }
        
        if let Some(ref reply_to) = headers.reply_to {
            lines.push(Line::from(vec![
                Span::styled("Reply-To: ", Style::default().fg(theme.colors.content_preview.header).add_modifier(Modifier::BOLD)),
                Span::styled(reply_to.clone(), Style::default().fg(theme.colors.content_preview.body)),
            ]));
        }
        
        if !headers.message_id.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("Message-ID: ", Style::default().fg(theme.colors.content_preview.header).add_modifier(Modifier::BOLD)),
                Span::styled(headers.message_id.clone(), Style::default().fg(theme.colors.palette.text_muted)),
            ]));
        }
        
        lines
    }
    
    fn render_content_line(&self, content_line: &ContentLine, theme: &Theme) -> Line {
        match content_line.line_type {
            LineType::Header => Line::from(vec![
                Span::styled(content_line.text.clone(), 
                    Style::default().fg(theme.colors.content_preview.header).add_modifier(Modifier::BOLD))
            ]),
            LineType::Subject => Line::from(vec![
                Span::styled(content_line.text.clone(), 
                    Style::default().fg(theme.colors.palette.accent).add_modifier(Modifier::BOLD))
            ]),
            LineType::Quote => {
                let depth = content_line.metadata.get("depth")
                    .and_then(|d| d.parse::<usize>().ok())
                    .unwrap_or(1);
                let quote_color = if depth > 1 { 
                    theme.colors.palette.text_muted 
                } else { 
                    theme.colors.palette.info 
                };
                Line::from(vec![
                    Span::styled(content_line.text.clone(), Style::default().fg(quote_color))
                ])
            },
            LineType::Code => Line::from(vec![
                Span::styled(content_line.text.clone(), 
                    Style::default().fg(theme.colors.palette.accent).add_modifier(Modifier::DIM))
            ]),
            LineType::Link => Line::from(vec![
                Span::styled(content_line.text.clone(), 
                    Style::default().fg(theme.colors.content_preview.link).add_modifier(Modifier::UNDERLINED))
            ]),
            LineType::Bullet => Line::from(vec![
                Span::styled(content_line.text.clone(), 
                    Style::default().fg(theme.colors.palette.success))
            ]),
            LineType::Separator => Line::from(vec![
                Span::styled(content_line.text.clone(), 
                    Style::default().fg(theme.colors.palette.border))
            ]),
            LineType::Attachment => Line::from(vec![
                Span::styled("📎 ", Style::default().fg(theme.colors.palette.warning)),
                Span::styled(content_line.text.clone(), 
                    Style::default().fg(theme.colors.palette.warning).add_modifier(Modifier::BOLD))
            ]),
            LineType::Signature => Line::from(vec![
                Span::styled(content_line.text.clone(), 
                    Style::default().fg(theme.colors.palette.text_muted).add_modifier(Modifier::DIM))
            ]),
            LineType::Empty => Line::from(""),
            LineType::Body => Line::from(vec![
                Span::styled(content_line.text.clone(), 
                    Style::default().fg(theme.colors.content_preview.body))
            ]),
        }
    }
    
    fn render_raw_content_with_styling(&self, content_height: usize, theme: &Theme) -> Vec<Line> {
        let start_line = self.scroll;
        let end_line = (start_line + content_height).min(self.raw_content.len());
        
        self.raw_content[start_line..end_line]
            .iter()
            .map(|line| {
                // Basic styling for raw content
                if line.starts_with("From:") || line.starts_with("To:") || 
                   line.starts_with("Date:") || line.starts_with("Subject:") {
                    Line::from(vec![
                        Span::styled(line.clone(), Style::default()
                            .fg(theme.colors.content_preview.header)
                            .add_modifier(Modifier::BOLD))
                    ])
                } else if line.starts_with(">") {
                    Line::from(vec![
                        Span::styled(line.clone(), Style::default().fg(theme.colors.palette.info))
                    ])
                } else if line.starts_with("---") {
                    Line::from(vec![
                        Span::styled(line.clone(), Style::default().fg(theme.colors.palette.border))
                    ])
                } else {
                    Line::from(vec![
                        Span::styled(line.clone(), Style::default().fg(theme.colors.content_preview.body))
                    ])
                }
            })
            .collect()
    }

    pub fn handle_up(&mut self) {
        if self.scroll > 0 {
            self.scroll -= 1;
        }
    }

    pub fn handle_down(&mut self) {
        let max_scroll = self.raw_content.len().saturating_sub(1);
        if self.scroll < max_scroll {
            self.scroll += 1;
        }
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll = 0;
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll = self.raw_content.len().saturating_sub(1);
    }

    pub fn set_content(&mut self, content: Vec<String>) {
        self.raw_content = content;
        self.scroll = 0; // Reset scroll when content changes
    }

    pub fn clear_content(&mut self) {
        self.raw_content.clear();
        self.email_content = None;
        self.scroll = 0;
    }
    
    /// Set rich email content with parsed structure
    pub fn set_email_content(&mut self, email_content: EmailContent) {
        self.email_content = Some(email_content);
        self.view_mode = ViewMode::Formatted;
        self.scroll = 0;
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
            attachments: Vec::new(), // TODO: Parse attachments
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
                    metadata.insert("depth".to_string(), 
                        line.chars().take_while(|&c| c == '>').count().to_string());
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
    
    /// Classify a line of content based on its characteristics
    fn classify_line(&self, line: &str) -> LineType {
        let trimmed = line.trim();
        
        if trimmed.is_empty() {
            LineType::Empty
        } else if trimmed.starts_with('>') {
            LineType::Quote
        } else if trimmed.starts_with("```") || trimmed.starts_with("    ") {
            LineType::Code
        } else if self.url_regex.is_match(trimmed) {
            LineType::Link
        } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("• ") {
            LineType::Bullet
        } else if trimmed.starts_with("---") || trimmed.starts_with("___") {
            LineType::Separator
        } else if trimmed.starts_with("--") && (trimmed.contains("signature") || line.len() < 50) {
            LineType::Signature
        } else {
            LineType::Body
        }
    }
    
    /// Toggle between view modes
    pub fn toggle_view_mode(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::Raw => ViewMode::Formatted,
            ViewMode::Formatted => ViewMode::Headers,
            ViewMode::Headers => ViewMode::Raw,
        };
        self.scroll = 0; // Reset scroll when changing view mode
    }
    
    /// Toggle expanded header display
    pub fn toggle_headers(&mut self) {
        self.show_headers_expanded = !self.show_headers_expanded;
    }
}

impl Default for ContentPreview {
    fn default() -> Self {
        Self::new()
    }
}