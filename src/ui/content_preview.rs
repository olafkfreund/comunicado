use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Wrap},
    Frame,
};

pub struct ContentPreview {
    content: Vec<String>,
    scroll: usize,
}

impl ContentPreview {
    pub fn new() -> Self {
        let mut preview = Self {
            content: Vec::new(),
            scroll: 0,
        };
        
        // Initialize with sample content
        preview.initialize_sample_content();
        
        preview
    }

    fn initialize_sample_content(&mut self) {
        self.content = vec![
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
            "✨ Modern TUI Interface".to_string(),
            "   - Clean, intuitive design with ratatui".to_string(),
            "   - Vim-style keyboard navigation".to_string(),
            "   - Responsive three-pane layout".to_string(),
            "".to_string(),
            "📧 Rich Email Support".to_string(),
            "   - HTML email rendering (coming soon)".to_string(),
            "   - Image and animation display".to_string(),
            "   - Attachment handling".to_string(),
            "".to_string(),
            "🔒 Secure Authentication".to_string(),
            "   - OAuth2 support for major providers".to_string(),
            "   - Multi-account management".to_string(),
            "   - Local email storage with Maildir".to_string(),
            "".to_string(),
            "📅 Integrated Calendar (upcoming)".to_string(),
            "   - CalDAV synchronization".to_string(),
            "   - Meeting invitation handling".to_string(),
            "   - Desktop environment integration".to_string(),
            "".to_string(),
            "🚀 Getting Started".to_string(),
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

    pub fn render(&self, frame: &mut Frame, area: Rect, block: Block, is_focused: bool) {
        let content_height = area.height.saturating_sub(2) as usize; // Account for block borders
        
        // Calculate visible content range
        let start_line = self.scroll;
        let end_line = (start_line + content_height).min(self.content.len());
        
        // Create lines for display
        let lines: Vec<Line> = self.content[start_line..end_line]
            .iter()
            .map(|line| {
                // Parse different types of content for styling
                if line.starts_with("From:") || line.starts_with("To:") || 
                   line.starts_with("Date:") || line.starts_with("Subject:") {
                    // Header styling
                    Line::from(vec![
                        Span::styled(line.clone(), Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD))
                    ])
                } else if line.starts_with("✨") || line.starts_with("📧") || 
                          line.starts_with("🔒") || line.starts_with("📅") || 
                          line.starts_with("🚀") {
                    // Section headers with emojis
                    Line::from(vec![
                        Span::styled(line.clone(), Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD))
                    ])
                } else if line.starts_with("  ") && (line.contains("-") || line.contains("•")) {
                    // Bullet points
                    Line::from(vec![
                        Span::styled(line.clone(), Style::default().fg(Color::Green))
                    ])
                } else if line.starts_with("Navigation:") || line.starts_with("Global:") {
                    // Keyboard shortcut section headers
                    Line::from(vec![
                        Span::styled(line.clone(), Style::default()
                            .fg(Color::Magenta)
                            .add_modifier(Modifier::BOLD))
                    ])
                } else if line.contains("/") && (line.contains("Tab") || line.contains("Ctrl")) {
                    // Keyboard shortcuts
                    Line::from(vec![
                        Span::styled(line.clone(), Style::default().fg(Color::Blue))
                    ])
                } else if line.starts_with("---") {
                    // Separators
                    Line::from(vec![
                        Span::styled(line.clone(), Style::default().fg(Color::DarkGray))
                    ])
                } else if line.is_empty() {
                    // Empty lines
                    Line::from("")
                } else {
                    // Regular content
                    Line::from(vec![
                        Span::styled(line.clone(), Style::default().fg(Color::White))
                    ])
                }
            })
            .collect();

        // Create scroll indicator if content is scrollable
        let scroll_indicator = if self.content.len() > content_height {
            let position = (self.scroll as f32 / (self.content.len() - content_height) as f32 * 100.0) as u16;
            format!(" ({}%)", position)
        } else {
            String::new()
        };

        let title = if is_focused {
            format!("Content{}", scroll_indicator)
        } else {
            format!("Content{}", scroll_indicator)
        };

        let paragraph = Paragraph::new(lines)
            .block(block.title(title))
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }

    pub fn handle_up(&mut self) {
        if self.scroll > 0 {
            self.scroll -= 1;
        }
    }

    pub fn handle_down(&mut self) {
        let max_scroll = self.content.len().saturating_sub(1);
        if self.scroll < max_scroll {
            self.scroll += 1;
        }
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll = 0;
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll = self.content.len().saturating_sub(1);
    }

    pub fn set_content(&mut self, content: Vec<String>) {
        self.content = content;
        self.scroll = 0; // Reset scroll when content changes
    }

    pub fn clear_content(&mut self) {
        self.content.clear();
        self.scroll = 0;
    }
}

impl Default for ContentPreview {
    fn default() -> Self {
        Self::new()
    }
}