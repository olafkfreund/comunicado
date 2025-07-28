use scraper::Html;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};

/// HTML to terminal text converter for email content
pub struct HtmlRenderer {
    /// Maximum width for text wrapping
    pub max_width: usize,
    /// Current text style stack
    style_stack: Vec<Style>,
    /// Base text color
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
    
    /// Convert HTML content to terminal-friendly text with styling
    pub fn render_html(&mut self, html_content: &str) -> Text<'static> {
        // Debug logging to help identify issues
        tracing::debug!("HTML Renderer: Processing content of length {}", html_content.len());
        tracing::debug!("HTML Renderer: Content preview: {:?}", &html_content[..html_content.len().min(200)]);
        
        // Parse HTML
        let document = Html::parse_fragment(html_content);
        
        // Convert to text with styling
        let mut lines = Vec::new();
        self.process_element(&document.root_element(), &mut lines);
        
        // If no lines were generated, fall back to html2text
        if lines.is_empty() {
            tracing::warn!("HTML Renderer: No lines generated, falling back to html2text");
            let plain_text = self.html_to_plain_text(html_content);
            let plain_lines: Vec<Line<'static>> = plain_text
                .lines()
                .map(|line| Line::from(line.to_string()))
                .collect();
            return Text::from(plain_lines);
        }
        
        tracing::debug!("HTML Renderer: Generated {} lines", lines.len());
        
        // Create Text with proper line breaks
        Text::from(lines)
    }
    
    /// Convert HTML to plain text (fallback for simple terminals)
    pub fn html_to_plain_text(&self, html_content: &str) -> String {
        // Use html2text for conversion
        html2text::from_read(html_content.as_bytes(), self.max_width)
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
    
    /// Process HTML elements recursively
    fn process_element(&mut self, element: &scraper::ElementRef, lines: &mut Vec<Line<'static>>) {
        use scraper::Node;
        
        for node in element.children() {
            match node.value() {
                Node::Element(elem) => {
                    let tag_name = elem.name();
                    let element_ref = scraper::ElementRef::wrap(node).unwrap();
                    
                    // Apply inline styles if present
                    self.apply_inline_styles(&element_ref);
                    
                    // Handle different HTML tags
                    match tag_name {
                        "p" => {
                            self.process_paragraph(&element_ref, lines);
                            lines.push(Line::from(""));  // Add blank line after paragraph
                        }
                        "br" => {
                            lines.push(Line::from(""));
                        }
                        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                            self.process_heading(&element_ref, lines, tag_name);
                        }
                        "strong" | "b" => {
                            self.push_style(Style::default().add_modifier(Modifier::BOLD));
                            self.process_element(&element_ref, lines);
                            self.pop_style();
                        }
                        "em" | "i" => {
                            self.push_style(Style::default().add_modifier(Modifier::ITALIC));
                            self.process_element(&element_ref, lines);
                            self.pop_style();
                        }
                        "u" => {
                            self.push_style(Style::default().add_modifier(Modifier::UNDERLINED));
                            self.process_element(&element_ref, lines);
                            self.pop_style();
                        }
                        "a" => {
                            self.process_link(&element_ref, lines);
                        }
                        "blockquote" => {
                            self.process_blockquote(&element_ref, lines);
                        }
                        "ul" | "ol" => {
                            self.process_list(&element_ref, lines, tag_name);
                        }
                        "pre" | "code" => {
                            self.process_code(&element_ref, lines);
                        }
                        "img" => {
                            self.process_image(&element_ref, lines);
                        }
                        "table" => {
                            self.process_table(&element_ref, lines);
                        }
                        "div" | "span" | "body" | "html" => {
                            // Container elements - just process children
                            self.process_element(&element_ref, lines);
                        }
                        _ => {
                            // Unknown elements - process children
                            self.process_element(&element_ref, lines);
                        }
                    }
                    
                    // Pop inline styles if they were applied
                    if element_ref.value().attr("style").is_some() {
                        self.pop_style();
                    }
                }
                Node::Text(text) => {
                    let text_content = text.trim();
                    if !text_content.is_empty() {
                        self.add_text_to_lines(text_content, lines);
                    }
                }
                _ => {}
            }
        }
    }
    
    /// Process paragraph elements
    fn process_paragraph(&mut self, element: &scraper::ElementRef, lines: &mut Vec<Line<'static>>) {
        let mut paragraph_lines = Vec::new();
        self.process_element(element, &mut paragraph_lines);
        
        // Join paragraph content into single line if short, or keep multiple lines
        if paragraph_lines.len() == 1 {
            lines.extend(paragraph_lines);
        } else {
            lines.extend(paragraph_lines);
        }
    }
    
    /// Process heading elements
    fn process_heading(&mut self, element: &scraper::ElementRef, lines: &mut Vec<Line<'static>>, tag: &str) {
        let level = tag.chars().last().unwrap().to_digit(10).unwrap_or(1) as usize;
        let style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        
        self.push_style(style);
        
        // Add visual hierarchy
        let prefix = match level {
            1 => "# ",
            2 => "## ",
            3 => "### ",
            _ => "#### ",
        };
        
        let mut heading_lines = Vec::new();
        self.process_element(element, &mut heading_lines);
        
        if let Some(first_line) = heading_lines.first() {
            let mut spans = vec![Span::styled(prefix, style)];
            spans.extend(first_line.spans.clone());
            lines.push(Line::from(spans));
        }
        
        lines.push(Line::from(""));  // Add blank line after heading
        self.pop_style();
    }
    
    /// Process links
    fn process_link(&mut self, element: &scraper::ElementRef, lines: &mut Vec<Line<'static>>) {
        let href = element.value().attr("href").unwrap_or("#");
        let link_style = Style::default()
            .fg(Color::Blue)
            .add_modifier(Modifier::UNDERLINED);
        
        self.push_style(link_style);
        self.process_element(element, lines);
        self.pop_style();
        
        // Add href in parentheses if it's different from text content
        let text_content = element.text().collect::<String>();
        if href != "#" && href != text_content.trim() {
            self.add_text_to_lines(&format!(" ({})", href), lines);
        }
    }
    
    /// Process blockquotes
    fn process_blockquote(&mut self, element: &scraper::ElementRef, lines: &mut Vec<Line<'static>>) {
        let quote_style = Style::default().fg(Color::Gray);
        
        lines.push(Line::from(""));
        let mut quote_lines = Vec::new();
        self.process_element(element, &mut quote_lines);
        
        // Add quote prefix to each line
        for line in quote_lines {
            let mut spans = vec![Span::styled("> ", quote_style)];
            spans.extend(line.spans);
            lines.push(Line::from(spans));
        }
        lines.push(Line::from(""));
    }
    
    /// Process lists
    fn process_list(&mut self, element: &scraper::ElementRef, lines: &mut Vec<Line<'static>>, list_type: &str) {
        lines.push(Line::from(""));
        
        let mut item_count = 0;
        for child in element.children() {
            if let Some(li) = scraper::ElementRef::wrap(child) {
                if li.value().name() == "li" {
                    item_count += 1;
                    let prefix = if list_type == "ul" {
                        "• ".to_string()
                    } else {
                        format!("{}. ", item_count)
                    };
                    
                    let mut item_lines = Vec::new();
                    self.process_element(&li, &mut item_lines);
                    
                    if let Some(first_line) = item_lines.first() {
                        let mut spans = vec![Span::raw(prefix)];
                        spans.extend(first_line.spans.clone());
                        lines.push(Line::from(spans));
                        
                        // Add remaining lines with proper indentation
                        for line in item_lines.iter().skip(1) {
                            let mut spans = vec![Span::raw("  ")];
                            spans.extend(line.spans.clone());
                            lines.push(Line::from(spans));
                        }
                    }
                }
            }
        }
        lines.push(Line::from(""));
    }
    
    /// Process code blocks
    fn process_code(&mut self, element: &scraper::ElementRef, lines: &mut Vec<Line<'static>>) {
        let code_style = Style::default()
            .fg(Color::Green)
            .bg(Color::DarkGray);
        
        let code_text = element.text().collect::<String>();
        
        if element.value().name() == "pre" {
            // Multi-line code block
            lines.push(Line::from(""));
            lines.push(Line::styled("```", code_style));
            
            for line in code_text.lines() {
                lines.push(Line::styled(line.to_string(), code_style));
            }
            
            lines.push(Line::styled("```", code_style));
            lines.push(Line::from(""));
        } else {
            // Inline code
            lines.push(Line::styled(format!("`{}`", code_text), code_style));
        }
    }
    
    /// Process images (placeholder - actual rendering happens in content preview)
    fn process_image(&mut self, element: &scraper::ElementRef, lines: &mut Vec<Line<'static>>) {
        let src = element.value().attr("src").unwrap_or("");
        let alt = element.value().attr("alt").unwrap_or("Image");
        
        let image_style = Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::ITALIC);
        
        // Create a marker that the content preview can recognize and replace with actual image
        lines.push(Line::styled(
            format!("IMG_PLACEHOLDER:{}:{}", src, alt),
            image_style
        ));
        lines.push(Line::from(""));
    }
    
    /// Process tables (simplified)
    fn process_table(&mut self, element: &scraper::ElementRef, lines: &mut Vec<Line<'static>>) {
        lines.push(Line::from(""));
        lines.push(Line::styled("[ Table Content ]", Style::default().fg(Color::Yellow)));
        
        // TODO: Implement proper table rendering
        let text_content = element.text().collect::<String>();
        self.add_text_to_lines(&text_content, lines);
        
        lines.push(Line::from(""));
    }
    
    /// Add text content to lines with current style
    fn add_text_to_lines(&self, text: &str, lines: &mut Vec<Line<'static>>) {
        let current_style = self.current_style();
        
        // Word wrap if necessary
        if text.len() > self.max_width {
            for wrapped_line in self.wrap_text(text) {
                lines.push(Line::styled(wrapped_line, current_style));
            }
        } else {
            lines.push(Line::styled(text.to_string(), current_style));
        }
    }
    
    /// Simple text wrapping
    fn wrap_text(&self, text: &str) -> Vec<String> {
        let mut lines = Vec::new();
        let mut current_line = String::new();
        
        for word in text.split_whitespace() {
            if current_line.len() + word.len() + 1 > self.max_width {
                if !current_line.is_empty() {
                    lines.push(current_line.clone());
                    current_line.clear();
                }
            }
            
            if !current_line.is_empty() {
                current_line.push(' ');
            }
            current_line.push_str(word);
        }
        
        if !current_line.is_empty() {
            lines.push(current_line);
        }
        
        lines
    }
    
    /// Push a new style onto the stack
    fn push_style(&mut self, style: Style) {
        self.style_stack.push(style);
    }
    
    /// Pop the last style from the stack
    fn pop_style(&mut self) {
        if self.style_stack.len() > 1 {
            self.style_stack.pop();
        }
    }
    
    /// Get the current style (combination of all stacked styles)
    fn current_style(&self) -> Style {
        self.style_stack.last().copied().unwrap_or_default()
    }
    
    /// Parse CSS color values (basic implementation)
    fn parse_css_color(&self, color_str: &str) -> Option<Color> {
        let color = color_str.trim().to_lowercase();
        
        // Named colors
        match color.as_str() {
            "red" => Some(Color::Red),
            "green" => Some(Color::Green),
            "blue" => Some(Color::Blue),
            "yellow" => Some(Color::Yellow),
            "cyan" => Some(Color::Cyan),
            "magenta" => Some(Color::Magenta),
            "white" => Some(Color::White),
            "black" => Some(Color::Black),
            "gray" | "grey" => Some(Color::Gray),
            "darkgray" | "darkgrey" => Some(Color::DarkGray),
            "lightred" => Some(Color::LightRed),
            "lightgreen" => Some(Color::LightGreen),
            "lightblue" => Some(Color::LightBlue),
            "lightyellow" => Some(Color::LightYellow),
            "lightcyan" => Some(Color::LightCyan),
            "lightmagenta" => Some(Color::LightMagenta),
            _ => {
                // Try to parse hex colors
                if color.starts_with('#') && color.len() == 7 {
                    if let (Ok(r), Ok(g), Ok(b)) = (
                        u8::from_str_radix(&color[1..3], 16),
                        u8::from_str_radix(&color[3..5], 16),
                        u8::from_str_radix(&color[5..7], 16),
                    ) {
                        return Some(Color::Rgb(r, g, b));
                    }
                }
                
                // Try to parse rgb() colors
                if color.starts_with("rgb(") && color.ends_with(')') {
                    let rgb_part = &color[4..color.len()-1];
                    let parts: Vec<&str> = rgb_part.split(',').collect();
                    if parts.len() == 3 {
                        if let (Ok(r), Ok(g), Ok(b)) = (
                            parts[0].trim().parse::<u8>(),
                            parts[1].trim().parse::<u8>(),
                            parts[2].trim().parse::<u8>(),
                        ) {
                            return Some(Color::Rgb(r, g, b));
                        }
                    }
                }
                
                None
            }
        }
    }
    
    /// Apply inline styles from style attribute
    fn apply_inline_styles(&mut self, element: &scraper::ElementRef) {
        if let Some(style_attr) = element.value().attr("style") {
            let mut style = self.current_style();
            
            // Parse CSS properties
            for declaration in style_attr.split(';') {
                let parts: Vec<&str> = declaration.splitn(2, ':').collect();
                if parts.len() == 2 {
                    let property = parts[0].trim().to_lowercase();
                    let value = parts[1].trim();
                    
                    match property.as_str() {
                        "color" => {
                            if let Some(color) = self.parse_css_color(value) {
                                style = style.fg(color);
                            }
                        }
                        "background-color" => {
                            if let Some(bg_color) = self.parse_css_color(value) {
                                style = style.bg(bg_color);
                            }
                        }
                        "font-weight" => {
                            if value == "bold" || value == "bolder" || value.parse::<i32>().unwrap_or(400) >= 600 {
                                style = style.add_modifier(Modifier::BOLD);
                            }
                        }
                        "font-style" => {
                            if value == "italic" || value == "oblique" {
                                style = style.add_modifier(Modifier::ITALIC);
                            }
                        }
                        "text-decoration" => {
                            if value.contains("underline") {
                                style = style.add_modifier(Modifier::UNDERLINED);
                            }
                        }
                        _ => {} // Ignore unknown properties
                    }
                }
            }
            
            self.push_style(style);
        }
    }
}

/// Check if content appears to be HTML
pub fn is_html_content(content: &str) -> bool {
    let content_lower = content.to_lowercase();
    let trimmed = content.trim();
    
    // Check for obvious HTML patterns
    if content_lower.contains("<html") || 
       content_lower.contains("<!doctype") ||
       content_lower.contains("<body") ||
       content_lower.contains("<div") ||
       content_lower.contains("<p>") ||
       content_lower.contains("<br") ||
       content_lower.contains("<span") ||
       content_lower.contains("<strong") ||
       content_lower.contains("<em") ||
       content_lower.contains("<a ") {
        return true;
    }
    
    // Check if content has multiple HTML-like tags
    let tag_count = content.matches('<').count();
    if tag_count > 1 && content.contains('>') {
        return true;
    }
    
    // Check if content starts with HTML
    trimmed.starts_with('<') && trimmed.contains('>')
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
}