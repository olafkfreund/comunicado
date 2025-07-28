use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::email::{StoredMessage, EmailDatabase};
use crate::theme::Theme;

/// Search query modes
#[derive(Debug, Clone, PartialEq)]
pub enum SearchMode {
    /// Simple full-text search across all content
    FullText,
    /// Search only in subject lines
    Subject,
    /// Search only in sender addresses/names
    From,
    /// Search only in email body content
    Body,
    /// Advanced search with multiple criteria
    Advanced,
}

/// Search result with highlighting information
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// The matching message
    pub message: StoredMessage,
    /// Search relevance score (from FTS5)
    pub rank: f64,
    /// Highlighted snippets of matching content
    pub snippets: Vec<SearchSnippet>,
    /// Which fields matched the search
    pub matched_fields: Vec<String>,
}

/// Text snippet with highlighting
#[derive(Debug, Clone)]
pub struct SearchSnippet {
    /// Field name that matched (subject, body_text, from_addr, etc.)
    pub field: String,
    /// Text content with highlighted search terms
    pub content: String,
    /// Positions of highlighted terms
    pub highlights: Vec<(usize, usize)>,
}

/// Search UI component actions
#[derive(Debug, Clone, PartialEq)]
pub enum SearchAction {
    /// Start new search
    StartSearch,
    /// Update search query
    UpdateQuery(String),
    /// Change search mode
    ChangeMode(SearchMode),
    /// Navigate search results
    NextResult,
    PreviousResult,
    /// Select current search result
    SelectResult,
    /// Clear search and return to normal view
    ClearSearch,
    /// Toggle search mode selector
    ToggleModeSelector,
}

/// Search UI component
pub struct SearchUI {
    /// Current search query
    query: String,
    
    /// Current search mode
    mode: SearchMode,
    
    /// Search results
    results: Vec<SearchResult>,
    
    /// Currently selected result index
    selected_index: usize,
    
    /// Whether search is active
    is_active: bool,
    
    /// Whether mode selector is visible
    show_mode_selector: bool,
    
    /// Selected mode in selector
    selected_mode_index: usize,
    
    /// Available search modes
    available_modes: Vec<(SearchMode, String, String)>, // mode, name, description
    
    /// Search is in progress
    is_searching: bool,
    
    /// Last search query (to avoid duplicate searches)
    last_query: String,
    
    /// Error message if search failed
    error_message: Option<String>,
    
    /// Search statistics
    total_results: usize,
    search_time_ms: u64,
}

impl SearchUI {
    /// Create a new search UI
    pub fn new() -> Self {
        let available_modes = vec![
            (SearchMode::FullText, "Full Text".to_string(), "Search across all email content".to_string()),
            (SearchMode::Subject, "Subject".to_string(), "Search only in subject lines".to_string()),
            (SearchMode::From, "From".to_string(), "Search only in sender information".to_string()),
            (SearchMode::Body, "Body".to_string(), "Search only in email body content".to_string()),
            (SearchMode::Advanced, "Advanced".to_string(), "Advanced search with multiple criteria".to_string()),
        ];
        
        Self {
            query: String::new(),
            mode: SearchMode::FullText,
            results: Vec::new(),
            selected_index: 0,
            is_active: false,
            show_mode_selector: false,
            selected_mode_index: 0,
            available_modes,
            is_searching: false,
            last_query: String::new(),
            error_message: None,
            total_results: 0,
            search_time_ms: 0,
        }
    }
    
    /// Start search mode
    pub fn start_search(&mut self) {
        self.is_active = true;
        self.query.clear();
        self.results.clear();
        self.selected_index = 0;
        self.error_message = None;
        self.last_query.clear();
    }
    
    /// End search mode
    pub fn end_search(&mut self) {
        self.is_active = false;
        self.query.clear();
        self.results.clear();
        self.selected_index = 0;
        self.show_mode_selector = false;
        self.error_message = None;
        self.last_query.clear();
    }
    
    /// Update search query
    pub fn set_query(&mut self, query: String) {
        self.query = query;
    }
    
    /// Get current search query
    pub fn query(&self) -> &str {
        &self.query
    }
    
    /// Get current search mode
    pub fn mode(&self) -> &SearchMode {
        &self.mode
    }
    
    /// Set search mode
    pub fn set_mode(&mut self, mode: SearchMode) {
        // Update selected mode index first
        self.selected_mode_index = self.available_modes.iter()
            .position(|(m, _, _)| m == &mode)
            .unwrap_or(0);
        self.mode = mode;
    }
    
    /// Check if search is active
    pub fn is_active(&self) -> bool {
        self.is_active
    }
    
    /// Get search results
    pub fn results(&self) -> &[SearchResult] {
        &self.results
    }
    
    /// Get currently selected result
    pub fn selected_result(&self) -> Option<&SearchResult> {
        self.results.get(self.selected_index)
    }
    
    /// Get selected result index
    pub fn selected_index(&self) -> usize {
        self.selected_index
    }
    
    /// Navigate to next result
    pub fn next_result(&mut self) {
        if !self.results.is_empty() && self.selected_index < self.results.len() - 1 {
            self.selected_index += 1;
        }
    }
    
    /// Navigate to previous result
    pub fn previous_result(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }
    
    /// Navigate mode selector
    pub fn next_mode(&mut self) {
        if self.selected_mode_index < self.available_modes.len() - 1 {
            self.selected_mode_index += 1;
        }
    }
    
    pub fn previous_mode(&mut self) {
        if self.selected_mode_index > 0 {
            self.selected_mode_index -= 1;
        }
    }
    
    /// Select current mode
    pub fn select_mode(&mut self) {
        if let Some((mode, _, _)) = self.available_modes.get(self.selected_mode_index) {
            self.mode = mode.clone();
        }
        self.show_mode_selector = false;
    }
    
    /// Toggle mode selector visibility
    pub fn toggle_mode_selector(&mut self) {
        self.show_mode_selector = !self.show_mode_selector;
    }
    
    /// Check if should perform search
    pub fn should_search(&self) -> bool {
        !self.query.is_empty() && 
        self.query != self.last_query && 
        !self.is_searching &&
        self.query.len() >= 2 // Minimum query length
    }
    
    /// Set search in progress
    pub fn set_searching(&mut self, searching: bool) {
        self.is_searching = searching;
        if searching {
            self.error_message = None;
        }
    }
    
    /// Update search results
    pub fn set_results(&mut self, results: Vec<SearchResult>, search_time_ms: u64) {
        self.results = results;
        self.total_results = self.results.len();
        self.selected_index = 0;
        self.search_time_ms = search_time_ms;
        self.last_query = self.query.clone();
        self.is_searching = false;
        self.error_message = None;
    }
    
    /// Set search error
    pub fn set_error(&mut self, error: String) {
        self.error_message = Some(error);
        self.is_searching = false;
    }
    
    /// Handle key input
    pub fn handle_key(&mut self, key: crossterm::event::KeyCode) -> Option<SearchAction> {
        if !self.is_active {
            return None;
        }
        
        if self.show_mode_selector {
            return self.handle_mode_selector_key(key);
        }
        
        match key {
            crossterm::event::KeyCode::Backspace => {
                self.query.pop();
                Some(SearchAction::UpdateQuery(self.query.clone()))
            }
            crossterm::event::KeyCode::Enter => {
                Some(SearchAction::SelectResult)
            }
            crossterm::event::KeyCode::Up => {
                self.previous_result();
                None
            }
            crossterm::event::KeyCode::Down => {
                self.next_result();
                None
            }
            crossterm::event::KeyCode::Tab => {
                Some(SearchAction::ToggleModeSelector)
            }
            crossterm::event::KeyCode::Esc => {
                Some(SearchAction::ClearSearch)
            }
            crossterm::event::KeyCode::F(1) => {
                self.set_mode(SearchMode::FullText);
                Some(SearchAction::ChangeMode(SearchMode::FullText))
            }
            crossterm::event::KeyCode::F(2) => {
                self.set_mode(SearchMode::Subject);
                Some(SearchAction::ChangeMode(SearchMode::Subject))
            }
            crossterm::event::KeyCode::F(3) => {
                self.set_mode(SearchMode::From);
                Some(SearchAction::ChangeMode(SearchMode::From))
            }
            crossterm::event::KeyCode::F(4) => {
                self.set_mode(SearchMode::Body);
                Some(SearchAction::ChangeMode(SearchMode::Body))
            }
            crossterm::event::KeyCode::Char('k') => {
                self.previous_result();
                None
            }
            crossterm::event::KeyCode::Char('j') => {
                self.next_result();
                None
            }
            crossterm::event::KeyCode::Char(c) => {
                self.query.push(c);
                Some(SearchAction::UpdateQuery(self.query.clone()))
            }
            _ => None,
        }
    }
    
    /// Handle mode selector key input
    fn handle_mode_selector_key(&mut self, key: crossterm::event::KeyCode) -> Option<SearchAction> {
        match key {
            crossterm::event::KeyCode::Up => {
                self.previous_mode();
                None
            }
            crossterm::event::KeyCode::Down => {
                self.next_mode();
                None
            }
            crossterm::event::KeyCode::Char('k') => {
                self.previous_mode();
                None
            }
            crossterm::event::KeyCode::Char('j') => {
                self.next_mode();
                None
            }
            crossterm::event::KeyCode::Enter => {
                self.select_mode();
                Some(SearchAction::ChangeMode(self.mode.clone()))
            }
            crossterm::event::KeyCode::Esc | crossterm::event::KeyCode::Tab => {
                self.show_mode_selector = false;
                None
            }
            _ => None,
        }
    }
    
    /// Render the search UI
    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.is_active {
            return;
        }
        
        // Create search popup
        let popup_area = self.centered_rect(90, 85, area);
        
        // Clear background
        frame.render_widget(Clear, popup_area);
        
        // Main search block
        let block = Block::default()
            .title(" Search Email ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.colors.palette.accent));
        
        let inner_area = block.inner(popup_area);
        frame.render_widget(block, popup_area);
        
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Search input and mode
                Constraint::Length(2), // Status line
                Constraint::Min(10),   // Results
                Constraint::Length(2), // Help text
            ])
            .split(inner_area);
        
        // Render search input
        self.render_search_input(frame, chunks[0], theme);
        
        // Render status
        self.render_status(frame, chunks[1], theme);
        
        // Render results or mode selector
        if self.show_mode_selector {
            self.render_mode_selector(frame, chunks[2], theme);
        } else {
            self.render_results(frame, chunks[2], theme);
        }
        
        // Render help
        self.render_help(frame, chunks[3], theme);
    }
    
    /// Render search input field
    fn render_search_input(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(20),    // Query input
                Constraint::Length(15), // Mode indicator
            ])
            .split(area);
        
        // Search query input
        let query_text = if self.is_searching {
            format!("🔍 Searching: {}", self.query)
        } else {
            format!("🔍 {}", self.query)
        };
        
        let input_paragraph = Paragraph::new(query_text)
            .style(Style::default().fg(theme.colors.palette.text_primary))
            .block(Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.colors.palette.accent)));
        
        frame.render_widget(input_paragraph, chunks[0]);
        
        // Mode indicator
        let mode_text = match self.mode {
            SearchMode::FullText => "📄 Full Text",
            SearchMode::Subject => "📧 Subject",
            SearchMode::From => "👤 From",
            SearchMode::Body => "📝 Body",
            SearchMode::Advanced => "⚙️ Advanced",
        };
        
        let mode_paragraph = Paragraph::new(mode_text)
            .style(Style::default().fg(theme.colors.palette.text_muted))
            .alignment(Alignment::Center)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.colors.palette.text_muted)));
        
        frame.render_widget(mode_paragraph, chunks[1]);
    }
    
    /// Render search status line
    fn render_status(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let status_text = if let Some(ref error) = self.error_message {
            format!("❌ Error: {}", error)
        } else if self.is_searching {
            "⏳ Searching...".to_string()
        } else if self.results.is_empty() && !self.query.is_empty() {
            "No results found".to_string()
        } else if !self.results.is_empty() {
            format!("Found {} results in {}ms", self.total_results, self.search_time_ms)
        } else {
            "Type to search (minimum 2 characters)".to_string()
        };
        
        let status_color = if self.error_message.is_some() {
            Color::Red
        } else if self.is_searching {
            Color::Yellow
        } else {
            theme.colors.palette.text_muted
        };
        
        let status_paragraph = Paragraph::new(status_text)
            .style(Style::default().fg(status_color))
            .alignment(Alignment::Center);
        
        frame.render_widget(status_paragraph, area);
    }
    
    /// Render search results
    fn render_results(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if self.results.is_empty() {
            let empty_paragraph = Paragraph::new("No search results to display")
                .style(Style::default().fg(theme.colors.palette.text_muted))
                .alignment(Alignment::Center);
            frame.render_widget(empty_paragraph, area);
            return;
        }
        
        let mut items = Vec::new();
        
        for (i, result) in self.results.iter().enumerate() {
            let is_selected = i == self.selected_index;
            
            // Format message info
            let from_display = if let Some(ref name) = result.message.from_name {
                format!("{} <{}>", name, result.message.from_addr)
            } else {
                result.message.from_addr.clone()
            };
            
            let date_str = result.message.date.format("%m/%d %H:%M").to_string();
            
            // Create main line with subject and metadata
            let main_line = Line::from(vec![
                Span::styled(
                    format!("{:50}", truncate_text(&result.message.subject, 50)),
                    if is_selected {
                        Style::default().fg(theme.colors.palette.background).bg(theme.colors.palette.accent).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.colors.palette.text_primary).add_modifier(Modifier::BOLD)
                    }
                ),
                Span::styled(
                    format!(" {:30}", truncate_text(&from_display, 30)),
                    if is_selected {
                        Style::default().fg(theme.colors.palette.background).bg(theme.colors.palette.accent)
                    } else {
                        Style::default().fg(theme.colors.palette.text_primary)
                    }
                ),
                Span::styled(
                    format!(" {}", date_str),
                    if is_selected {
                        Style::default().fg(theme.colors.palette.background).bg(theme.colors.palette.accent)
                    } else {
                        Style::default().fg(theme.colors.palette.text_muted)
                    }
                ),
            ]);
            
            items.push(ListItem::new(vec![main_line]));
            
            // Add snippet preview if available
            if !result.snippets.is_empty() && is_selected {
                for snippet in &result.snippets {
                    let snippet_line = Line::from(vec![
                        Span::styled("  ", Style::default()),
                        Span::styled(&snippet.field, Style::default()
                            .fg(theme.colors.palette.accent)
                            .add_modifier(Modifier::ITALIC)),
                        Span::styled(": ", Style::default()),
                        Span::styled(
                            truncate_text(&snippet.content, 100),
                            Style::default().fg(theme.colors.palette.text_muted)
                        ),
                    ]);
                    items.push(ListItem::new(vec![snippet_line]));
                }
            }
        }
        
        let results_list = List::new(items)
            .block(Block::default()
                .title(format!(" Results ({}/{}) ", self.selected_index + 1, self.results.len()))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.colors.palette.text_muted)));
        
        frame.render_widget(results_list, area);
    }
    
    /// Render mode selector
    fn render_mode_selector(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let mut items = Vec::new();
        
        for (i, (mode, name, description)) in self.available_modes.iter().enumerate() {
            let is_selected = i == self.selected_mode_index;
            let is_current = mode == &self.mode;
            
            let indicator = if is_current { "●" } else { "○" };
            let prefix = if is_selected { "► " } else { "  " };
            
            let style = if is_selected {
                Style::default()
                    .fg(theme.colors.palette.background)
                    .bg(theme.colors.palette.accent)
                    .add_modifier(Modifier::BOLD)
            } else if is_current {
                Style::default()
                    .fg(theme.colors.palette.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.colors.palette.text_primary)
            };
            
            let line = Line::from(vec![
                Span::styled(format!("{}{} {}", prefix, indicator, name), style),
                Span::styled(format!(" - {}", description), 
                    if is_selected {
                        Style::default().fg(theme.colors.palette.background).bg(theme.colors.palette.accent)
                    } else {
                        Style::default().fg(theme.colors.palette.text_muted)
                    }
                ),
            ]);
            
            items.push(ListItem::new(vec![line]));
        }
        
        let mode_list = List::new(items)
            .block(Block::default()
                .title(" Search Mode ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.colors.palette.accent)));
        
        frame.render_widget(mode_list, area);
    }
    
    /// Render help text
    fn render_help(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let help_text = if self.show_mode_selector {
            "↑↓: Navigate • Enter: Select • Esc/Tab: Close"
        } else {
            "Type: Search • ↑↓: Navigate • Enter: Open • Tab: Mode • F1-F4: Quick Mode • Esc: Close"
        };
        
        let help_paragraph = Paragraph::new(help_text)
            .style(Style::default().fg(theme.colors.palette.text_muted))
            .alignment(Alignment::Center);
        
        frame.render_widget(help_paragraph, area);
    }
    
    /// Helper to create centered rectangle
    fn centered_rect(&self, percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }
}

impl Default for SearchUI {
    fn default() -> Self {
        Self::new()
    }
}

/// Truncate text to specified length with ellipsis
fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        format!("{}…", &text[..max_len.saturating_sub(1)])
    }
}

/// Search functionality for database queries
pub struct SearchEngine {
    database: std::sync::Arc<EmailDatabase>,
}

impl SearchEngine {
    /// Create a new search engine
    pub fn new(database: std::sync::Arc<EmailDatabase>) -> Self {
        Self { database }
    }
    
    /// Perform full-text search
    pub async fn search(
        &self,
        account_id: &str,
        query: &str,
        mode: &SearchMode,
        limit: Option<u32>,
    ) -> Result<Vec<SearchResult>, Box<dyn std::error::Error>> {
        let start_time = std::time::Instant::now();
        
        // Build FTS5 query based on mode
        let fts_query = self.build_fts_query(query, mode)?;
        
        // Execute search
        let messages = self.database.search_messages(account_id, &fts_query, limit).await?;
        
        // Convert to search results with highlighting
        let mut results = Vec::new();
        for message in messages {
            let result = self.create_search_result(message, query, mode).await?;
            results.push(result);
        }
        
        let search_time = start_time.elapsed().as_millis() as u64;
        
        // Sort by relevance (FTS5 already provides ranking, but we can enhance it)
        results.sort_by(|a, b| b.rank.partial_cmp(&a.rank).unwrap_or(std::cmp::Ordering::Equal));
        
        Ok(results)
    }
    
    /// Build FTS5 query string based on search mode
    fn build_fts_query(&self, query: &str, mode: &SearchMode) -> Result<String, Box<dyn std::error::Error>> {
        let sanitized_query = self.sanitize_fts_query(query)?;
        
        let fts_query = match mode {
            SearchMode::FullText => sanitized_query,
            SearchMode::Subject => format!("subject:{}", sanitized_query),
            SearchMode::From => format!("(from_addr:{} OR from_name:{})", sanitized_query, sanitized_query),
            SearchMode::Body => format!("body_text:{}", sanitized_query),
            SearchMode::Advanced => {
                // For advanced mode, parse query for field:value pairs
                self.parse_advanced_query(&sanitized_query)?
            }
        };
        
        Ok(fts_query)
    }
    
    /// Sanitize query for FTS5
    fn sanitize_fts_query(&self, query: &str) -> Result<String, Box<dyn std::error::Error>> {
        // Remove FTS5 special characters and quote phrases
        let mut sanitized = query.replace("\"", "");
        
        // If query contains spaces, treat as phrase search
        if sanitized.contains(' ') {
            sanitized = format!("\"{}\"", sanitized);
        }
        
        Ok(sanitized)
    }
    
    /// Parse advanced search query (field:value syntax)
    fn parse_advanced_query(&self, query: &str) -> Result<String, Box<dyn std::error::Error>> {
        let mut conditions = Vec::new();
        let parts: Vec<&str> = query.split_whitespace().collect();
        
        for part in parts {
            if let Some(colon_pos) = part.find(':') {
                let field = &part[..colon_pos];
                let value = &part[colon_pos + 1..];
                
                let fts_field = match field.to_lowercase().as_str() {
                    "subject" | "s" => "subject",
                    "from" | "f" => "from_addr",
                    "body" | "b" => "body_text",
                    _ => continue, // Skip unknown fields
                };
                
                conditions.push(format!("{}:{}", fts_field, value));
            } else {
                // Default to full-text if no field specified
                conditions.push(part.to_string());
            }
        }
        
        if conditions.is_empty() {
            return Ok(query.to_string());
        }
        
        Ok(conditions.join(" AND "))
    }
    
    /// Create search result with snippets and highlighting
    async fn create_search_result(
        &self,
        message: StoredMessage,
        query: &str,
        mode: &SearchMode,
    ) -> Result<SearchResult, Box<dyn std::error::Error>> {
        let mut snippets = Vec::new();
        let mut matched_fields = Vec::new();
        
        // Extract query terms for highlighting
        let query_terms = self.extract_query_terms(query);
        
        // Check each field for matches and create snippets
        if self.field_matches(&message.subject, &query_terms) {
            matched_fields.push("subject".to_string());
            if let Some(snippet) = self.create_snippet("subject", &message.subject, &query_terms) {
                snippets.push(snippet);
            }
        }
        
        if let Some(ref body) = message.body_text {
            if self.field_matches(body, &query_terms) {
                matched_fields.push("body_text".to_string());
                if let Some(snippet) = self.create_snippet("body", body, &query_terms) {
                    snippets.push(snippet);
                }
            }
        }
        
        if self.field_matches(&message.from_addr, &query_terms) ||
           message.from_name.as_ref().map(|n| self.field_matches(n, &query_terms)).unwrap_or(false) {
            matched_fields.push("from".to_string());
            let from_text = message.from_name.as_ref()
                .map(|name| format!("{} <{}>", name, message.from_addr))
                .unwrap_or_else(|| message.from_addr.clone());
            if let Some(snippet) = self.create_snippet("from", &from_text, &query_terms) {
                snippets.push(snippet);
            }
        }
        
        // Calculate relevance score (simplified)
        let rank = self.calculate_relevance_score(&message, &matched_fields, &query_terms);
        
        Ok(SearchResult {
            message,
            rank,
            snippets,
            matched_fields,
        })
    }
    
    /// Extract search terms from query
    fn extract_query_terms(&self, query: &str) -> Vec<String> {
        query.to_lowercase()
            .split_whitespace()
            .filter(|term| !term.is_empty())
            .map(|term| term.trim_matches('"').to_string())
            .collect()
    }
    
    /// Check if field matches query terms
    fn field_matches(&self, field: &str, query_terms: &[String]) -> bool {
        let field_lower = field.to_lowercase();
        query_terms.iter().any(|term| field_lower.contains(term))
    }
    
    /// Create text snippet with highlighting
    fn create_snippet(&self, field: &str, content: &str, query_terms: &[String]) -> Option<SearchSnippet> {
        let content_lower = content.to_lowercase();
        let mut highlights = Vec::new();
        
        // Find all occurrences of query terms
        for term in query_terms {
            let mut start = 0;
            while let Some(pos) = content_lower[start..].find(term) {
                let actual_pos = start + pos;
                highlights.push((actual_pos, actual_pos + term.len()));
                start = actual_pos + term.len();
            }
        }
        
        if highlights.is_empty() {
            return None;
        }
        
        // Sort highlights by position
        highlights.sort_by_key(|&(start, _)| start);
        
        // Create snippet around first match (context window)
        let first_match = highlights[0].0;
        let snippet_start = first_match.saturating_sub(50);
        let snippet_end = std::cmp::min(content.len(), first_match + 150);
        let snippet_content = content[snippet_start..snippet_end].to_string();
        
        // Adjust highlight positions relative to snippet
        let adjusted_highlights: Vec<(usize, usize)> = highlights.iter()
            .filter_map(|&(start, end)| {
                if start >= snippet_start && start < snippet_end {
                    Some((start - snippet_start, std::cmp::min(end, snippet_end) - snippet_start))
                } else {
                    None
                }
            })
            .collect();
        
        Some(SearchSnippet {
            field: field.to_string(),
            content: snippet_content,
            highlights: adjusted_highlights,
        })
    }
    
    /// Calculate relevance score for search result
    fn calculate_relevance_score(&self, message: &StoredMessage, matched_fields: &[String], query_terms: &[String]) -> f64 {
        let mut score = 0.0;
        
        // Base score from number of matched fields
        score += matched_fields.len() as f64 * 10.0;
        
        // Higher score for subject matches
        if matched_fields.contains(&"subject".to_string()) {
            score += 20.0;
        }
        
        // Medium score for from matches
        if matched_fields.contains(&"from".to_string()) {
            score += 15.0;
        }
        
        // Lower score for body matches (but still valuable)
        if matched_fields.contains(&"body_text".to_string()) {
            score += 5.0;
        }
        
        // Boost score for recent messages
        let days_old = (chrono::Utc::now() - message.date).num_days();
        if days_old < 7 {
            score += 10.0;
        } else if days_old < 30 {
            score += 5.0;
        }
        
        // Boost score for multiple query term matches
        score += query_terms.len() as f64 * 2.0;
        
        score
    }
}