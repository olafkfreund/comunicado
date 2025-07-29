use crate::email::{AttachmentInfo, AttachmentType};
use crate::images::ImageManager;
use crate::theme::Theme;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Wrap},
    Frame,
};
use std::io::Write;
use std::path::Path;

/// Viewer modes for different attachment types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewerMode {
    /// Display image attachments using terminal graphics protocols
    Image,
    /// Display text content with syntax highlighting if possible
    Text,
    /// Show a preview of the file with metadata
    Preview,
    /// Show file information only (for unsupported types)
    Info,
}

/// Result of attempting to view an attachment
#[derive(Debug)]
pub enum ViewResult {
    /// Successfully loaded content for display
    Content(Vec<Line<'static>>),
    /// Content is available but requires external viewer
    ExternalViewer(String), // Path to temp file
    /// Content cannot be viewed in terminal
    NotSupported(String), // Reason
    /// Error occurred while loading
    Error(String),
}

/// Attachment viewer for displaying various file formats in the terminal
pub struct AttachmentViewer {
    image_manager: ImageManager,
    temp_dir: std::path::PathBuf,
    current_content: Option<Vec<Line<'static>>>,
    current_attachment: Option<AttachmentInfo>,
    viewer_mode: ViewerMode,
    scroll: usize,
    max_text_size: usize, // Maximum size for text preview (bytes)
}

impl AttachmentViewer {
    /// Create a new attachment viewer
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = std::env::temp_dir().join("comunicado-attachments");
        std::fs::create_dir_all(&temp_dir)?;

        Ok(Self {
            image_manager: ImageManager::new().unwrap_or_default(),
            temp_dir,
            current_content: None,
            current_attachment: None,
            viewer_mode: ViewerMode::Preview,
            scroll: 0,
            max_text_size: 1024 * 1024, // 1MB max for text preview
        })
    }

    /// View an attachment and return the display content
    pub async fn view_attachment(
        &mut self,
        attachment: &AttachmentInfo,
        data: &[u8],
    ) -> ViewResult {
        self.current_attachment = Some(attachment.clone());

        // Determine the best viewer mode for this attachment
        self.viewer_mode = self.determine_viewer_mode(&attachment.attachment_type);

        match self.viewer_mode {
            ViewerMode::Image => self.view_image_attachment(attachment, data).await,
            ViewerMode::Text => self.view_text_attachment(attachment, data),
            ViewerMode::Preview => self.view_preview_attachment(attachment, data),
            ViewerMode::Info => self.view_info_attachment(attachment),
        }
    }

    /// Determine the best viewer mode for an attachment type
    fn determine_viewer_mode(&self, attachment_type: &AttachmentType) -> ViewerMode {
        if attachment_type.is_image() && self.image_manager.supports_images() {
            ViewerMode::Image
        } else if attachment_type.is_previewable() {
            ViewerMode::Text
        } else if attachment_type.is_document() || attachment_type.is_archive() {
            ViewerMode::Preview
        } else {
            ViewerMode::Info
        }
    }

    /// View an image attachment using terminal graphics
    async fn view_image_attachment(
        &mut self,
        attachment: &AttachmentInfo,
        data: &[u8],
    ) -> ViewResult {
        match self
            .image_manager
            .load_image_from_bytes(data, Some(&attachment.stored.content_type))
            .await
        {
            Ok(rendered_content) => {
                let lines: Vec<Line<'static>> = rendered_content
                    .lines()
                    .map(|line| Line::raw(line.to_string()))
                    .collect();

                self.current_content = Some(lines.clone());
                ViewResult::Content(lines)
            }
            Err(e) => {
                let error_lines = vec![
                    Line::styled(
                        "🖼️ Image Viewer",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Line::raw(""),
                    Line::styled(
                        format!("Failed to load image: {}", e),
                        Style::default().fg(Color::Red),
                    ),
                    Line::raw(""),
                    Line::styled(
                        "File Information:",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Line::raw(format!("  Filename: {}", attachment.display_name)),
                    Line::raw(format!(
                        "  Type: {}",
                        attachment.attachment_type.description()
                    )),
                    Line::raw(format!("  Size: {}", attachment.format_size())),
                    Line::raw(format!("  MIME Type: {}", attachment.stored.content_type)),
                ];

                self.current_content = Some(error_lines.clone());
                ViewResult::Content(error_lines)
            }
        }
    }

    /// View a text attachment with basic syntax highlighting
    fn view_text_attachment(&mut self, attachment: &AttachmentInfo, data: &[u8]) -> ViewResult {
        // Check size limit
        if data.len() > self.max_text_size {
            return ViewResult::NotSupported(format!(
                "Text file too large ({} bytes). Maximum size for preview is {} bytes.",
                data.len(),
                self.max_text_size
            ));
        }

        // Try to decode as UTF-8
        let text_content = match std::str::from_utf8(data) {
            Ok(text) => text.to_string(),
            Err(_) => {
                // Try with lossy conversion
                String::from_utf8_lossy(data).to_string()
            }
        };

        let mut lines = vec![
            Line::styled(
                "📄 Text Viewer",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Line::raw(format!(
                "File: {} ({})",
                attachment.display_name,
                attachment.format_size()
            )),
            Line::raw(""),
        ];

        // Detect language for syntax highlighting
        let language = self.detect_language(&attachment.display_name, &text_content);
        if !language.is_empty() {
            lines.push(Line::styled(
                format!("Language: {}", language),
                Style::default().fg(Color::Green),
            ));
            lines.push(Line::raw(""));
        }

        // Add content lines with basic syntax coloring
        for (line_num, content_line) in text_content.lines().enumerate().take(1000) {
            // Limit lines for performance
            let line = if language == "json" || language == "xml" {
                self.highlight_structured_text(content_line, &language)
            } else {
                Line::raw(content_line.to_string())
            };

            // Add line numbers for code files
            if self.should_show_line_numbers(&language) {
                let line_num_span = Span::styled(
                    format!("{:4} │ ", line_num + 1),
                    Style::default().fg(Color::Gray),
                );
                let mut spans = vec![line_num_span];
                spans.extend(line.spans);
                lines.push(Line::from(spans));
            } else {
                lines.push(line);
            }
        }

        if text_content.lines().count() > 1000 {
            lines.push(Line::raw(""));
            lines.push(Line::styled(
                "... (content truncated)",
                Style::default().fg(Color::Yellow),
            ));
        }

        self.current_content = Some(lines.clone());
        ViewResult::Content(lines)
    }

    /// View a preview of a document or other file type
    fn view_preview_attachment(&mut self, attachment: &AttachmentInfo, data: &[u8]) -> ViewResult {
        let lines = vec![
            Line::styled(
                "📋 File Preview",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Line::raw(""),
            Line::styled(
                "File Information",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Line::raw(format!("  📄 Name: {}", attachment.display_name)),
            Line::raw(format!(
                "  📊 Type: {}",
                attachment.attachment_type.description()
            )),
            Line::raw(format!("  📏 Size: {}", attachment.format_size())),
            Line::raw(format!("  🏷️  MIME: {}", attachment.stored.content_type)),
            Line::raw(format!(
                "  🔒 Safe: {}",
                if attachment.is_safe {
                    "Yes"
                } else {
                    "⚠️  No"
                }
            )),
            Line::raw(""),
            Line::styled(
                "Content Summary",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ];

        let mut content_lines = lines;

        // Add type-specific preview information
        match attachment.attachment_type {
            AttachmentType::Pdf => {
                content_lines.extend(self.preview_pdf_info(data));
            }
            AttachmentType::Word | AttachmentType::Excel | AttachmentType::PowerPoint => {
                content_lines.extend(self.preview_office_info(&attachment.attachment_type, data));
            }
            AttachmentType::Zip
            | AttachmentType::Rar
            | AttachmentType::SevenZip
            | AttachmentType::Tar => {
                content_lines.extend(self.preview_archive_info(&attachment.attachment_type, data));
            }
            _ => {
                content_lines.push(Line::raw("  No preview available for this file type."));
                content_lines.push(Line::raw("  Use 's' to save the file to view externally."));
            }
        }

        content_lines.push(Line::raw(""));
        content_lines.push(Line::styled(
            "Actions",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ));
        content_lines.push(Line::raw("  s - Save attachment to disk"));
        if attachment.attachment_type.is_previewable() {
            content_lines.push(Line::raw("  t - Switch to text view"));
        }
        content_lines.push(Line::raw("  q - Close viewer"));

        self.current_content = Some(content_lines.clone());
        ViewResult::Content(content_lines)
    }

    /// Show information about an attachment that cannot be viewed
    fn view_info_attachment(&mut self, attachment: &AttachmentInfo) -> ViewResult {
        let lines = vec![
            Line::styled(
                "ℹ️  File Information",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Line::raw(""),
            Line::raw(format!("  📄 Filename: {}", attachment.display_name)),
            Line::raw(format!(
                "  📊 Type: {}",
                attachment.attachment_type.description()
            )),
            Line::raw(format!("  📏 Size: {}", attachment.format_size())),
            Line::raw(format!(
                "  🏷️  MIME Type: {}",
                attachment.stored.content_type
            )),
            Line::raw(format!(
                "  🔒 Safety: {}",
                if attachment.is_safe {
                    "Safe"
                } else {
                    "⚠️  Potentially unsafe"
                }
            )),
            Line::raw(""),
            Line::styled(
                "This file type cannot be viewed in the terminal.",
                Style::default().fg(Color::Yellow),
            ),
            Line::raw("Use 's' to save the file and open it with an external application."),
            Line::raw(""),
            Line::styled(
                "Actions",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Line::raw("  s - Save attachment to disk"),
            Line::raw("  q - Close viewer"),
        ];

        self.current_content = Some(lines.clone());
        ViewResult::Content(lines)
    }

    /// Detect programming language from filename and content
    fn detect_language(&self, filename: &str, _content: &str) -> String {
        let extension = Path::new(filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase())
            .unwrap_or_default();

        match extension.as_str() {
            "rs" => "rust".to_string(),
            "py" => "python".to_string(),
            "js" | "ts" => "javascript".to_string(),
            "json" => "json".to_string(),
            "xml" | "html" | "htm" => "xml".to_string(),
            "css" => "css".to_string(),
            "md" => "markdown".to_string(),
            "toml" => "toml".to_string(),
            "yaml" | "yml" => "yaml".to_string(),
            "sh" | "bash" => "bash".to_string(),
            "c" | "h" => "c".to_string(),
            "cpp" | "cc" | "cxx" => "cpp".to_string(),
            "go" => "go".to_string(),
            "java" => "java".to_string(),
            _ => String::new(),
        }
    }

    /// Check if line numbers should be shown for this language
    fn should_show_line_numbers(&self, language: &str) -> bool {
        matches!(
            language,
            "rust" | "python" | "javascript" | "c" | "cpp" | "go" | "java" | "json" | "xml"
        )
    }

    /// Apply basic syntax highlighting to structured text
    fn highlight_structured_text(&self, line: &str, language: &str) -> Line<'static> {
        let trimmed = line.trim();

        match language {
            "json" => {
                if trimmed.starts_with('"') && trimmed.contains(':') {
                    // JSON key
                    Line::styled(line.to_string(), Style::default().fg(Color::Cyan))
                } else if trimmed.starts_with('"') {
                    // JSON string value
                    Line::styled(line.to_string(), Style::default().fg(Color::Green))
                } else if trimmed.parse::<f64>().is_ok() {
                    // JSON number
                    Line::styled(line.to_string(), Style::default().fg(Color::Magenta))
                } else if trimmed == "true" || trimmed == "false" || trimmed == "null" {
                    // JSON literals
                    Line::styled(line.to_string(), Style::default().fg(Color::Yellow))
                } else {
                    Line::raw(line.to_string())
                }
            }
            "xml" => {
                if trimmed.starts_with('<') && trimmed.ends_with('>') {
                    // XML tag
                    Line::styled(line.to_string(), Style::default().fg(Color::Blue))
                } else if trimmed.starts_with("<!--") {
                    // XML comment
                    Line::styled(line.to_string(), Style::default().fg(Color::Gray))
                } else {
                    Line::raw(line.to_string())
                }
            }
            _ => Line::raw(line.to_string()),
        }
    }

    /// Generate preview information for PDF files
    fn preview_pdf_info(&self, data: &[u8]) -> Vec<Line<'static>> {
        let mut lines = vec![];

        // Basic PDF header check
        if data.len() >= 4 && &data[0..4] == b"%PDF" {
            lines.push(Line::raw("  📄 Valid PDF file"));

            // Try to extract PDF version
            if let Ok(header) = std::str::from_utf8(&data[0..20.min(data.len())]) {
                if let Some(version_start) = header.find("%PDF-") {
                    let version = &header[version_start..].chars().take(8).collect::<String>();
                    lines.push(Line::raw(format!("  📋 Version: {}", version)));
                }
            }

            lines.push(Line::raw("  💡 Use a PDF viewer to open this file"));
        } else {
            lines.push(Line::styled(
                "  ⚠️  Invalid PDF format",
                Style::default().fg(Color::Red),
            ));
        }

        lines
    }

    /// Generate preview information for Office documents
    fn preview_office_info(
        &self,
        attachment_type: &AttachmentType,
        data: &[u8],
    ) -> Vec<Line<'static>> {
        let mut lines = vec![];

        // Check for Office format signatures
        let is_office_format = if data.len() >= 8 {
            // Check for ZIP signature (modern Office formats are ZIP-based)
            &data[0..4] == b"PK\x03\x04" || 
            // Check for older Office format signature
            &data[0..8] == b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1"
        } else {
            false
        };

        if is_office_format {
            let doc_type = match attachment_type {
                AttachmentType::Word => "Word Document",
                AttachmentType::Excel => "Excel Spreadsheet",
                AttachmentType::PowerPoint => "PowerPoint Presentation",
                _ => "Office Document",
            };

            lines.push(Line::raw(format!("  📄 Valid {} format", doc_type)));
            lines.push(Line::raw(
                "  💡 Use Microsoft Office or LibreOffice to open",
            ));
        } else {
            lines.push(Line::styled(
                "  ⚠️  Unrecognized Office format",
                Style::default().fg(Color::Yellow),
            ));
        }

        lines
    }

    /// Generate preview information for archive files
    fn preview_archive_info(
        &self,
        attachment_type: &AttachmentType,
        data: &[u8],
    ) -> Vec<Line<'static>> {
        let mut lines = vec![];

        let archive_type = match attachment_type {
            AttachmentType::Zip => "ZIP",
            AttachmentType::Rar => "RAR",
            AttachmentType::SevenZip => "7-Zip",
            AttachmentType::Tar => "TAR",
            _ => "Archive",
        };

        // Basic format validation
        let is_valid = match attachment_type {
            AttachmentType::Zip => data.len() >= 4 && &data[0..4] == b"PK\x03\x04",
            AttachmentType::Rar => data.len() >= 7 && &data[0..7] == b"Rar!\x1A\x07\x00",
            AttachmentType::SevenZip => data.len() >= 6 && &data[0..6] == b"7z\xBC\xAF\x27\x1C",
            AttachmentType::Tar => data.len() >= 512, // TAR files have 512-byte headers
            _ => false,
        };

        if is_valid {
            lines.push(Line::raw(format!("  📦 Valid {} archive", archive_type)));
            lines.push(Line::raw(
                "  💡 Extract with archive utility to view contents",
            ));
            lines.push(Line::raw("  ⚠️  Always scan archives before extracting"));
        } else {
            lines.push(Line::styled(
                format!("  ⚠️  Invalid {} format", archive_type),
                Style::default().fg(Color::Red),
            ));
        }

        lines
    }

    /// Render the attachment viewer
    pub fn render(&mut self, frame: &mut Frame, area: Rect, block: Block, theme: &Theme) {
        let content_height = area.height.saturating_sub(2) as usize; // Account for block borders

        let lines = if let Some(ref content) = self.current_content {
            let start_line = self.scroll;
            let end_line = (start_line + content_height).min(content.len());

            if end_line > start_line {
                content[start_line..end_line].to_vec()
            } else {
                vec![Line::from("(End of content)")]
            }
        } else {
            vec![Line::styled(
                "No attachment loaded",
                Style::default().fg(theme.colors.palette.text_muted),
            )]
        };

        // Create scroll indicator
        let scroll_indicator = if let Some(ref content) = self.current_content {
            if content.len() > content_height {
                let position =
                    (self.scroll as f32 / (content.len() - content_height) as f32 * 100.0) as u16;
                format!(" ({}%)", position)
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let mode_indicator = match self.viewer_mode {
            ViewerMode::Image => " [Image]",
            ViewerMode::Text => " [Text]",
            ViewerMode::Preview => " [Preview]",
            ViewerMode::Info => " [Info]",
        };

        let title = format!("Attachment Viewer{}{}", mode_indicator, scroll_indicator);

        let paragraph = Paragraph::new(lines)
            .block(block.title(title))
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }

    /// Handle scroll up
    pub fn scroll_up(&mut self) {
        if self.scroll > 0 {
            self.scroll -= 1;
        }
    }

    /// Handle scroll down
    pub fn scroll_down(&mut self) {
        if let Some(ref content) = self.current_content {
            let max_scroll = content.len().saturating_sub(1);
            if self.scroll < max_scroll {
                self.scroll += 1;
            }
        }
    }

    /// Jump to top of content
    pub fn scroll_to_top(&mut self) {
        self.scroll = 0;
    }

    /// Jump to bottom of content
    pub fn scroll_to_bottom(&mut self) {
        if let Some(ref content) = self.current_content {
            self.scroll = content.len().saturating_sub(1);
        }
    }

    /// Switch to text view mode (if applicable)
    pub async fn switch_to_text_mode(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref attachment) = self.current_attachment.clone() {
            if attachment.attachment_type.is_previewable() {
                self.viewer_mode = ViewerMode::Text;
                // Would need attachment data to re-render - this is a simplified implementation
                // In practice, we'd store the data or have a way to retrieve it
            }
        }
        Ok(())
    }

    /// Get current attachment being viewed
    pub fn current_attachment(&self) -> Option<&AttachmentInfo> {
        self.current_attachment.as_ref()
    }

    /// Check if content is available for viewing
    pub fn has_content(&self) -> bool {
        self.current_content.is_some()
    }

    /// Clear current content
    pub fn clear(&mut self) {
        self.current_content = None;
        self.current_attachment = None;
        self.scroll = 0;
    }

    /// Create a temporary file for the attachment (for external viewing)
    pub async fn create_temp_file(
        &self,
        attachment: &AttachmentInfo,
        data: &[u8],
    ) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        let temp_path = self.temp_dir.join(&attachment.display_name);
        let mut file = std::fs::File::create(&temp_path)?;
        file.write_all(data)?;
        file.sync_all()?;
        Ok(temp_path)
    }

    /// Clean up temporary files
    pub async fn cleanup_temp_files(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.temp_dir.exists() {
            std::fs::remove_dir_all(&self.temp_dir)?;
            std::fs::create_dir_all(&self.temp_dir)?;
        }
        Ok(())
    }
}

impl Default for AttachmentViewer {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            // Fallback implementation if temp directory creation fails
            Self {
                image_manager: ImageManager::new().unwrap_or_default(),
                temp_dir: std::path::PathBuf::from("/tmp/comunicado-attachments"),
                current_content: None,
                current_attachment: None,
                viewer_mode: ViewerMode::Info,
                scroll: 0,
                max_text_size: 1024 * 1024,
            }
        })
    }
}
