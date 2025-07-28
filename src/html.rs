use scraper::{Html, Selector, ElementRef};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};
use ammonia::clean;

/// HTML to terminal text converter for email content
pub struct HtmlRenderer {
    /// Maximum width for text wrapping
    pub max_width: usize,
    /// Current text style stack
    #[allow(dead_code)]
    style_stack: Vec<Style>,
    /// Base text color
    #[allow(dead_code)]
    base_color: Color,
}

impl HtmlRenderer {
    pub fn new(max_width: usize) -> Self {
        Self {
            max_width,
            style_stack: vec![Style::default()],
            base_color: Color::White,
        }
    }
    
    /// Convert HTML content to terminal-friendly text with styling (w3m/lynx style)
    pub fn render_html(&mut self, html_content: &str) -> Text<'static> {
        tracing::debug!("HTML Renderer: Processing content of length {}", html_content.len());
        
        // Step 1: Clean and sanitize HTML to remove malicious/unnecessary content
        let cleaned_html = self.clean_and_sanitize_html(html_content);
        
        // Step 2: Try our enhanced w3m-style renderer first
        let enhanced_result = self.render_html_enhanced(&cleaned_html);
        if !enhanced_result.lines.is_empty() {
            tracing::debug!("HTML Renderer: Enhanced rendering successful ({} lines)", enhanced_result.lines.len());
            return enhanced_result;
        }
        
        // Step 3: Fallback to html2text
        let plain_text = self.html_to_plain_text(&cleaned_html);
        if !plain_text.trim().is_empty() {
            tracing::debug!("HTML Renderer: html2text fallback successful ({} chars)", plain_text.len());
            let lines: Vec<Line<'static>> = plain_text
                .lines()
                .map(|line| {
                    let line_content = line.trim_end();
                    if line_content.is_empty() {
                        Line::from("")
                    } else {
                        self.style_text_line(line_content)
                    }
                })
                .collect();
            return Text::from(lines);
        }
        
        // Step 4: Last resort - basic HTML parsing
        tracing::warn!("HTML Renderer: Using basic HTML parsing fallback");
        let document = Html::parse_fragment(&cleaned_html);
        let mut lines = Vec::new();
        self.extract_text_from_html(&document, &mut lines);
        
        if lines.is_empty() {
            let stripped = Self::strip_html_tags(&cleaned_html);
            let fallback_lines: Vec<Line<'static>> = stripped
                .lines()
                .map(|line| Line::from(line.trim().to_string()))
                .collect();
            return Text::from(fallback_lines);
        }
        
        Text::from(lines)
    }
    
    /// Clean and sanitize HTML content (like w3m preprocessing)
    pub fn clean_and_sanitize_html(&self, html: &str) -> String {
        // Use ammonia to clean HTML and remove dangerous/unnecessary elements
        let clean_html = clean(html);
        
        // Additional cleaning for email-specific issues
        let mut cleaned = clean_html
            // Remove script and style content completely
            .replace(r#"<script[^>]*>.*?</script>"#, "")
            .replace(r#"<style[^>]*>.*?</style>"#, "")
            // Normalize whitespace
            .trim()
            .to_string();
        
        // If the content doesn't look like proper HTML, wrap it
        if !cleaned.starts_with('<') && !cleaned.contains("<html") && !cleaned.contains("<body") {
            cleaned = format!("<div>{}</div>", cleaned);
        }
        
        cleaned
    }
    
    /// Enhanced HTML renderer (w3m/lynx style)
    fn render_html_enhanced(&mut self, html: &str) -> Text<'static> {
        let document = Html::parse_fragment(html);
        let mut lines = Vec::new();
        let mut current_line = Vec::new();
        let mut list_depth = 0;
        let mut in_pre = false;
        
        // Walk through the HTML tree and render like a terminal browser
        self.render_element_enhanced(&document.root_element(), &mut lines, &mut current_line, &mut list_depth, &mut in_pre);
        
        // Add any remaining content
        if !current_line.is_empty() {
            lines.push(Line::from(current_line));
        }
        
        Text::from(lines)
    }
    
    /// Render HTML elements in a w3m/lynx style
    fn render_element_enhanced(
        &self,
        element: &ElementRef,
        lines: &mut Vec<Line<'static>>,
        current_line: &mut Vec<Span<'static>>,
        list_depth: &mut usize,
        in_pre: &mut bool,
    ) {
        use scraper::Node;
        
        for node in element.children() {
            match node.value() {
                Node::Element(elem) => {
                    if let Some(element_ref) = ElementRef::wrap(node) {
                        let tag_name = elem.name();
                        
                        match tag_name {
                            // Skip these entirely
                            "script" | "style" | "meta" | "link" | "head" => continue,
                            
                            // Block elements - create new lines
                            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                                self.flush_current_line(lines, current_line);
                                let text = element_ref.text().collect::<String>();
                                if !text.trim().is_empty() {
                                    let level = tag_name.chars().last().unwrap().to_digit(10).unwrap_or(1) as usize;
                                    let style = match level {
                                        1 => Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                                        2 => Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                                        _ => Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                                    };
                                    lines.push(Line::from(vec![Span::styled(text.trim().to_string(), style)]));
                                    lines.push(Line::from("")); // Add spacing after headers
                                }
                            }
                            
                            "p" | "div" => {
                                self.flush_current_line(lines, current_line);
                                self.render_element_enhanced(&element_ref, lines, current_line, list_depth, in_pre);
                                self.flush_current_line(lines, current_line);
                                lines.push(Line::from("")); // Add spacing
                            }
                            
                            "br" => {
                                self.flush_current_line(lines, current_line);
                            }
                            
                            "ul" | "ol" => {
                                self.flush_current_line(lines, current_line);
                                *list_depth += 1;
                                self.render_element_enhanced(&element_ref, lines, current_line, list_depth, in_pre);
                                *list_depth -= 1;
                                self.flush_current_line(lines, current_line);
                            }
                            
                            "li" => {
                                self.flush_current_line(lines, current_line);
                                let indent = "  ".repeat(*list_depth);
                                current_line.push(Span::raw(format!("{}• ", indent)));
                                self.render_element_enhanced(&element_ref, lines, current_line, list_depth, in_pre);
                            }
                            
                            "a" => {
                                let text = element_ref.text().collect::<String>();
                                if !text.trim().is_empty() {
                                    let href = element_ref.value().attr("href").unwrap_or("");
                                    let display_text = if href.is_empty() {
                                        text.trim().to_string()
                                    } else {
                                        format!("{} ({})", text.trim(), href)
                                    };
                                    current_line.push(Span::styled(
                                        display_text,
                                        Style::default().fg(Color::Blue).add_modifier(Modifier::UNDERLINED)
                                    ));
                                }
                            }
                            
                            "strong" | "b" => {
                                let text = element_ref.text().collect::<String>();
                                if !text.trim().is_empty() {
                                    current_line.push(Span::styled(
                                        text.trim().to_string(),
                                        Style::default().add_modifier(Modifier::BOLD)
                                    ));
                                }
                            }
                            
                            "em" | "i" => {
                                let text = element_ref.text().collect::<String>();
                                if !text.trim().is_empty() {
                                    current_line.push(Span::styled(
                                        text.trim().to_string(),
                                        Style::default().add_modifier(Modifier::ITALIC)
                                    ));
                                }
                            }
                            
                            "pre" | "code" => {
                                let old_pre = *in_pre;
                                *in_pre = true;
                                let text = element_ref.text().collect::<String>();
                                if !text.trim().is_empty() {
                                    self.flush_current_line(lines, current_line);
                                    for line in text.lines() {
                                        lines.push(Line::from(vec![Span::styled(
                                            line.to_string(),
                                            Style::default().fg(Color::Green)
                                        )]));
                                    }
                                }
                                *in_pre = old_pre;
                            }
                            
                            "table" => {
                                self.flush_current_line(lines, current_line);
                                lines.push(Line::from("┌─ Table ─┐"));
                                self.render_element_enhanced(&element_ref, lines, current_line, list_depth, in_pre);
                                lines.push(Line::from("└─────────┘"));
                            }
                            
                            "tr" => {
                                self.flush_current_line(lines, current_line);
                                current_line.push(Span::raw("│ "));
                                self.render_element_enhanced(&element_ref, lines, current_line, list_depth, in_pre);
                                current_line.push(Span::raw(" │"));
                                self.flush_current_line(lines, current_line);
                            }
                            
                            "td" | "th" => {
                                let text = element_ref.text().collect::<String>();
                                if !text.trim().is_empty() {
                                    let style = if tag_name == "th" {
                                        Style::default().add_modifier(Modifier::BOLD)
                                    } else {
                                        Style::default()
                                    };
                                    current_line.push(Span::styled(format!("{} ", text.trim()), style));
                                }
                            }
                            
                            // Recursive rendering for other elements
                            _ => {
                                self.render_element_enhanced(&element_ref, lines, current_line, list_depth, in_pre);
                            }
                        }
                    }
                }
                Node::Text(text) => {
                    let text_content = if *in_pre {
                        text.to_string()
                    } else {
                        text.trim().to_string()
                    };
                    
                    if !text_content.is_empty() {
                        current_line.push(Span::raw(text_content));
                    }
                }
                _ => {}
            }
        }
    }
    
    /// Helper to flush current line to lines vector
    fn flush_current_line(&self, lines: &mut Vec<Line<'static>>, current_line: &mut Vec<Span<'static>>) {
        if !current_line.is_empty() {
            lines.push(Line::from(current_line.drain(..).collect::<Vec<_>>()));
        }
    }
    
    /// Convert HTML to plain text using html2text (most reliable method)
    pub fn html_to_plain_text(&self, html_content: &str) -> String {
        // Use html2text for conversion with proper width
        let result = html2text::from_read(html_content.as_bytes(), self.max_width);
        
        // Clean up the result - remove excessive whitespace but preserve formatting
        let cleaned = result
            .lines()
            .map(|line| line.trim_end()) // Remove trailing whitespace but keep leading
            .collect::<Vec<_>>()
            .join("\n");
        
        // Remove excessive blank lines (more than 2 consecutive)
        let mut final_result = String::new();
        let mut blank_line_count = 0;
        
        for line in cleaned.lines() {
            if line.trim().is_empty() {
                blank_line_count += 1;
                if blank_line_count <= 2 {
                    final_result.push('\n');
                }
            } else {
                blank_line_count = 0;
                final_result.push_str(line);
                final_result.push('\n');
            }
        }
        
        final_result
    }
    
    /// Style a text line based on its content patterns
    fn style_text_line(&self, line: &str) -> Line<'static> {
        let trimmed = line.trim();
        
        // Detect different types of content and apply appropriate styling
        if trimmed.is_empty() {
            return Line::from("");
        }
        
        // Headers (lines that look like headings)
        if trimmed.len() < 100 && (
            trimmed.ends_with(':') ||
            trimmed.chars().all(|c| c.is_uppercase() || c.is_whitespace() || c.is_ascii_punctuation()) ||
            (trimmed.starts_with('*') && trimmed.ends_with('*') && trimmed.len() < 50)
        ) {
            return Line::styled(
                line.to_string(),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            );
        }
        
        // Quote lines (lines starting with >)
        if trimmed.starts_with('>') {
            return Line::styled(
                line.to_string(),
                Style::default().fg(Color::Gray).add_modifier(Modifier::ITALIC)
            );
        }
        
        // Bullet points
        if trimmed.starts_with("• ") || trimmed.starts_with("* ") || 
           trimmed.starts_with("- ") || trimmed.matches(". ").next().map_or(false, |_| {
               trimmed.split(". ").next().unwrap_or("").chars().all(|c| c.is_ascii_digit())
           }) {
            return Line::styled(
                line.to_string(),
                Style::default().fg(Color::Yellow)
            );
        }
        
        // URLs and email addresses
        if trimmed.contains("http://") || trimmed.contains("https://") || 
           trimmed.contains("www.") || (trimmed.contains('@') && trimmed.contains('.')) {
            return Line::styled(
                line.to_string(),
                Style::default().fg(Color::Blue).add_modifier(Modifier::UNDERLINED)
            );
        }
        
        // Default styling
        Line::styled(
            line.to_string(),
            Style::default().fg(Color::White)
        )
    }
    
    /// Extract text content from HTML document
    fn extract_text_from_html(&self, document: &Html, lines: &mut Vec<Line<'static>>) {
        // Try to find body content first
        if let Ok(body_selector) = Selector::parse("body") {
            if let Some(body) = document.select(&body_selector).next() {
                self.extract_element_text(&body, lines);
                return;
            }
        }
        
        // If no body found, extract from root
        self.extract_element_text(&document.root_element(), lines);
    }
    
    /// Extract text from a specific element
    fn extract_element_text(&self, element: &scraper::ElementRef, lines: &mut Vec<Line<'static>>) {
        use scraper::Node;
        
        for node in element.children() {
            match node.value() {
                Node::Element(elem) => {
                    let tag_name = elem.name();
                    if let Some(element_ref) = scraper::ElementRef::wrap(node) {
                        match tag_name {
                            // Skip these elements entirely
                            "script" | "style" | "meta" | "link" | "head" => continue,
                            
                            // Block elements that should create new lines
                            "p" | "div" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "br" => {
                                let text_content = element_ref.text().collect::<Vec<_>>().join(" ");
                                if !text_content.trim().is_empty() {
                                    lines.push(Line::from(text_content.trim().to_string()));
                                }
                                if matches!(tag_name, "p" | "div" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6") {
                                    lines.push(Line::from(""));
                                }
                            }
                            
                            // List items
                            "li" => {
                                let text_content = element_ref.text().collect::<Vec<_>>().join(" ");
                                if !text_content.trim().is_empty() {
                                    lines.push(Line::from(format!("• {}", text_content.trim())));
                                }
                            }
                            
                            // Table cells
                            "td" | "th" => {
                                let text_content = element_ref.text().collect::<Vec<_>>().join(" ");
                                if !text_content.trim().is_empty() {
                                    lines.push(Line::from(format!("{} ", text_content.trim())));
                                }
                            }
                            
                            // Inline elements - just extract text
                            _ => {
                                self.extract_element_text(&element_ref, lines);
                            }
                        }
                    }
                }
                Node::Text(text) => {
                    let text_content = text.trim();
                    if !text_content.is_empty() {
                        lines.push(Line::from(text_content.to_string()));
                    }
                }
                _ => {}
            }
        }
    }
    
    /// Simple HTML tag removal as last resort
    fn strip_html_tags(html: &str) -> String {
        let mut result = String::new();
        let mut in_tag = false;
        
        for ch in html.chars() {
            match ch {
                '<' => in_tag = true,
                '>' => in_tag = false,
                _ if !in_tag => result.push(ch),
                _ => {}
            }
        }
        
        // Clean up whitespace
        result
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }
    
}

/// Check if content appears to be HTML
pub fn is_html_content(content: &str) -> bool {
    let content_lower = content.to_lowercase();
    let trimmed = content.trim();
    
    // More comprehensive HTML detection
    if content_lower.contains("<!doctype html") || 
       content_lower.contains("<html") ||
       content_lower.contains("</html>") {
        return true;
    }
    
    // Check for common HTML tags
    let html_tags = [
        "<body", "</body>", "<div", "</div>", "<p>", "</p>",
        "<br", "<span", "</span>", "<strong", "</strong>",
        "<em", "</em>", "<a ", "</a>", "<img", "<table",
        "</table>", "<tr", "</tr>", "<td", "</td>", "<th", "</th>",
        "<ul", "</ul>", "<ol", "</ol>", "<li", "</li>",
        "<h1", "<h2", "<h3", "<h4", "<h5", "<h6"
    ];
    
    let mut tag_count = 0;
    for tag in &html_tags {
        if content_lower.contains(tag) {
            tag_count += 1;
            if tag_count >= 2 {
                return true;
            }
        }
    }
    
    // Check if content has HTML structure (opening and closing tags)
    if content.contains('<') && content.contains('>') {
        let tag_pairs = content.matches('<').count();
        let close_tags = content.matches('>').count();
        if tag_pairs > 1 && close_tags > 1 {
            return true;
        }
    }
    
    // Check if content starts with HTML
    if trimmed.starts_with('<') && trimmed.contains('>') && trimmed.len() > 10 {
        return true;
    }
    
    false
}

/// Quick HTML to plain text conversion for previews
pub fn html_to_text_preview(html: &str, max_length: Option<usize>) -> String {
    let text = html2text::from_read(html.as_bytes(), 80);
    
    // Clean up whitespace and trim to max length
    let cleaned = text
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    
    if let Some(max_len) = max_length {
        if cleaned.len() > max_len {
            format!("{}...", &cleaned[..max_len])
        } else {
            cleaned
        }
    } else {
        cleaned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_detection() {
        assert!(is_html_content("<html><body>Test</body></html>"));
        assert!(is_html_content("<p>This is a paragraph</p>"));
        assert!(is_html_content("Hello <br> World"));
        assert!(is_html_content("<div><span>Content</span></div>"));
        assert!(!is_html_content("Plain text email"));
        assert!(!is_html_content("Email with > and < but no tags"));
    }

    #[test]
    fn test_html_to_text_preview() {
        let html = "<p>Hello <strong>world</strong>!</p>";
        let text = html_to_text_preview(html, Some(20));
        assert!(text.contains("Hello world"));
        assert!(text.len() <= 23); // accounting for "..."
    }

    #[test]
    fn test_simple_html_rendering() {
        let mut renderer = HtmlRenderer::new(80);
        let html = "<p>Hello <strong>world</strong>!</p>";
        let result = renderer.render_html(html);
        
        // Should have at least one line of content
        assert!(!result.lines.is_empty());
    }

    #[test]
    fn test_html_to_plain_text() {
        let mut renderer = HtmlRenderer::new(80);
        let html = "<p>Hello <strong>world</strong>!</p><p>Second paragraph.</p>";
        let result = renderer.html_to_plain_text(html);
        
        assert!(result.contains("Hello world"));
        assert!(result.contains("Second paragraph"));
    }
}