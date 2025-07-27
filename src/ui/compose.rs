use crate::contacts::{Contact, ContactsManager};
use crate::spell::{SpellChecker, SpellCheckResult};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Text},
    widgets::{
        Block, Borders, List, ListItem, ListState, Paragraph, Clear, Wrap
    },
    Frame,
};
use crate::theme::Theme;
use std::sync::Arc;

/// Email composition UI with contact autocomplete and spell checking
pub struct ComposeUI {
    contacts_manager: Arc<ContactsManager>,
    spell_checker: SpellChecker,
    
    // Form fields
    to_field: String,
    cc_field: String,
    bcc_field: String,
    subject_field: String,
    body_text: String,
    
    // UI state
    current_field: ComposeField,
    is_autocomplete_visible: bool,
    autocomplete_suggestions: Vec<Contact>,
    autocomplete_selected: usize,
    
    // Spell check state
    is_spell_check_visible: bool,
    spell_check_result: Option<SpellCheckResult>,
    current_spell_error: usize,
    spell_suggestions: Vec<String>,
    spell_suggestion_selected: usize,
    
    // Spell check configuration state
    is_spell_config_visible: bool,
    available_languages: Vec<String>,
    language_selected: usize,
    
    // Cursor positions for each field
    to_cursor: usize,
    cc_cursor: usize,
    bcc_cursor: usize,
    subject_cursor: usize,
    body_cursor: usize,
    
    // Body text handling
    body_lines: Vec<String>,
    body_line_index: usize,
    
    // Form state
    is_modified: bool,
    spell_check_enabled: bool,
    
    // Auto-save state
    current_draft_id: Option<String>,
    last_auto_save: Option<std::time::Instant>,
    auto_save_interval: std::time::Duration,
    has_auto_save_changes: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ComposeField {
    To,
    Cc,
    Bcc,
    Subject,
    Body,
}

impl ComposeUI {
    /// Create a new compose UI
    pub fn new(contacts_manager: Arc<ContactsManager>) -> Self {
        let spell_checker = SpellChecker::new().unwrap_or_default();
        let available_languages = spell_checker.available_languages();
        
        Self {
            contacts_manager,
            spell_checker,
            to_field: String::new(),
            cc_field: String::new(),
            bcc_field: String::new(),
            subject_field: String::new(),
            body_text: String::new(),
            current_field: ComposeField::To,
            is_autocomplete_visible: false,
            autocomplete_suggestions: Vec::new(),
            autocomplete_selected: 0,
            is_spell_check_visible: false,
            spell_check_result: None,
            current_spell_error: 0,
            spell_suggestions: Vec::new(),
            spell_suggestion_selected: 0,
            is_spell_config_visible: false,
            available_languages,
            language_selected: 0,
            to_cursor: 0,
            cc_cursor: 0,
            bcc_cursor: 0,
            subject_cursor: 0,
            body_cursor: 0,
            body_lines: vec![String::new()],
            body_line_index: 0,
            is_modified: false,
            spell_check_enabled: true,
            current_draft_id: None,
            last_auto_save: None,
            auto_save_interval: std::time::Duration::from_secs(30), // Auto-save every 30 seconds
            has_auto_save_changes: false,
        }
    }
    
    /// Create a new compose UI for replying to a message
    pub fn new_reply(contacts_manager: Arc<ContactsManager>, reply_to: &str, subject: &str) -> Self {
        let mut compose = Self::new(contacts_manager);
        compose.to_field = reply_to.to_string();
        compose.subject_field = if subject.starts_with("Re: ") {
            subject.to_string()
        } else {
            format!("Re: {}", subject)
        };
        compose.current_field = ComposeField::Body;
        compose.to_cursor = compose.to_field.len();
        compose.subject_cursor = compose.subject_field.len();
        compose
    }
    
    /// Create a new compose UI for forwarding a message
    pub fn new_forward(contacts_manager: Arc<ContactsManager>, subject: &str, original_body: &str) -> Self {
        let mut compose = Self::new(contacts_manager);
        compose.subject_field = if subject.starts_with("Fwd: ") {
            subject.to_string()
        } else {
            format!("Fwd: {}", subject)
        };
        compose.body_text = format!("\n\n--- Forwarded Message ---\n{}", original_body);
        compose.body_lines = compose.body_text.lines().map(|s| s.to_string()).collect();
        if compose.body_lines.is_empty() {
            compose.body_lines.push(String::new());
        }
        compose.current_field = ComposeField::To;
        compose.subject_cursor = compose.subject_field.len();
        compose
    }
    
    /// Render the compose UI
    pub fn render(&mut self, f: &mut Frame, area: Rect, theme: &Theme) {
        // Main compose window
        let block = Block::default()
            .title("Compose Email")
            .borders(Borders::ALL)
            .border_style(theme.get_component_style("border", true));
        
        let inner = block.inner(area);
        f.render_widget(block, area);
        
        // Layout: header fields + body
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // To field
                Constraint::Length(2), // Cc field  
                Constraint::Length(2), // Bcc field
                Constraint::Length(2), // Subject field
                Constraint::Length(1), // Separator
                Constraint::Min(0),    // Body
            ])
            .split(inner);
        
        // Render header fields
        self.render_header_field(f, chunks[0], "To:", &self.to_field, self.to_cursor, ComposeField::To, theme);
        self.render_header_field(f, chunks[1], "Cc:", &self.cc_field, self.cc_cursor, ComposeField::Cc, theme);
        self.render_header_field(f, chunks[2], "Bcc:", &self.bcc_field, self.bcc_cursor, ComposeField::Bcc, theme);
        self.render_header_field(f, chunks[3], "Subject:", &self.subject_field, self.subject_cursor, ComposeField::Subject, theme);
        
        // Separator line
        let separator = Paragraph::new("─".repeat(chunks[4].width as usize))
            .style(Style::default().fg(Color::Gray));
        f.render_widget(separator, chunks[4]);
        
        // Body area
        self.render_body(f, chunks[5], theme);
        
        // Render autocomplete suggestions if visible
        if self.is_autocomplete_visible && !self.autocomplete_suggestions.is_empty() {
            self.render_autocomplete(f, area, theme);
        }
        
        // Render spell check suggestions if visible
        if self.is_spell_check_visible && self.spell_check_enabled {
            self.render_spell_check_popup(f, area, theme);
        }
        
        // Render spell check configuration if visible
        if self.is_spell_config_visible {
            self.render_spell_config_popup(f, area, theme);
        }
        
        // Status line at bottom
        self.render_status_line(f, area, theme);
    }
    
    /// Render a header field (To, Cc, Bcc, Subject)
    fn render_header_field(
        &self, 
        f: &mut Frame, 
        area: Rect, 
        label: &str, 
        value: &str, 
        cursor_pos: usize,
        field_type: ComposeField,
        theme: &Theme
    ) {
        let is_focused = self.current_field == field_type;
        
        // Split area into label and input
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(8), Constraint::Min(0)])
            .split(area);
        
        // Label
        let label_style = if is_focused {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };
        
        let label_paragraph = Paragraph::new(label)
            .style(label_style)
            .alignment(Alignment::Right);
        f.render_widget(label_paragraph, chunks[0]);
        
        // Input field
        let input_style = if is_focused {
            theme.get_component_style("input_focused", true)
        } else {
            theme.get_component_style("input", false)
        };
        
        // Show cursor for focused field
        let display_value = if is_focused {
            let mut chars: Vec<char> = value.chars().collect();
            if cursor_pos <= chars.len() {
                chars.insert(cursor_pos, '|');
            }
            chars.into_iter().collect()
        } else {
            value.to_string()
        };
        
        let input_paragraph = Paragraph::new(display_value)
            .style(input_style)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(input_paragraph, chunks[1]);
    }
    
    /// Render the email body
    fn render_body(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let is_focused = self.current_field == ComposeField::Body;
        
        let style = if is_focused {
            theme.get_component_style("input_focused", true)
        } else {
            theme.get_component_style("input", false)
        };
        
        let block = Block::default()
            .title("Message Body")
            .borders(Borders::ALL)
            .border_style(style);
        
        let inner = block.inner(area);
        f.render_widget(block, area);
        
        // Create text with cursor and spell check highlighting if focused
        let mut text = Text::default();
        
        for (line_idx, line) in self.body_lines.iter().enumerate() {
            if is_focused && line_idx == self.body_line_index {
                // Show cursor on current line with spell check highlighting
                let line_with_cursor = if self.spell_check_enabled && self.current_field == ComposeField::Body {
                    self.create_highlighted_line_with_cursor(line, self.body_cursor)
                } else {
                    let mut chars: Vec<char> = line.chars().collect();
                    let cursor_pos = self.body_cursor.min(chars.len());
                    chars.insert(cursor_pos, '|');
                    Line::from(chars.into_iter().collect::<String>())
                };
                text.lines.push(line_with_cursor);
            } else if self.spell_check_enabled && self.current_field == ComposeField::Body {
                // Highlight misspelled words without cursor
                text.lines.push(self.create_highlighted_line(line, line_idx));
            } else {
                text.lines.push(Line::from(line.clone()));
            }
        }
        
        let paragraph = Paragraph::new(text)
            .style(style)
            .wrap(Wrap { trim: true });
        
        f.render_widget(paragraph, inner);
    }
    
    /// Render autocomplete suggestions
    fn render_autocomplete(&mut self, f: &mut Frame, compose_area: Rect, theme: &Theme) {
        // Calculate position for autocomplete popup
        let popup_width = 50;
        let popup_height = (self.autocomplete_suggestions.len() + 2).min(8) as u16;
        
        let popup_area = Rect {
            x: compose_area.x + 10,
            y: compose_area.y + 3, // Position below the To field
            width: popup_width,
            height: popup_height,
        };
        
        // Clear the background
        f.render_widget(Clear, popup_area);
        
        // Create suggestion list
        let suggestions: Vec<ListItem> = self.autocomplete_suggestions
            .iter()
            .map(|contact| {
                let email = contact.primary_email()
                    .map(|e| e.address.clone())
                    .unwrap_or_else(|| "No email".to_string());
                
                let display_text = if contact.display_name.is_empty() {
                    email
                } else {
                    format!("{} <{}>", contact.display_name, email)
                };
                
                ListItem::new(display_text)
            })
            .collect();
        
        let mut list_state = ListState::default();
        list_state.select(Some(self.autocomplete_selected));
        
        let suggestions_list = List::new(suggestions)
            .block(Block::default()
                .title("Contact Suggestions")
                .borders(Borders::ALL)
                .border_style(theme.get_component_style("popup", true)))
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            )
            .highlight_symbol("▶ ");
        
        f.render_stateful_widget(suggestions_list, popup_area, &mut list_state);
    }
    
    /// Render status line with shortcuts
    fn render_status_line(&self, f: &mut Frame, compose_area: Rect, theme: &Theme) {
        let status_area = Rect {
            x: compose_area.x,
            y: compose_area.y + compose_area.height - 1,
            width: compose_area.width,
            height: 1,
        };
        
        let status_text = if self.is_autocomplete_visible {
            "↑↓ Navigate suggestions | Tab Complete | Esc Cancel | Enter Select"
        } else if self.is_spell_config_visible {
            "↑↓ Select language | Enter Apply | Esc/F10 Close"
        } else if self.spell_check_enabled && self.is_spell_check_visible {
            "F7 Toggle | F8/F9 Next/Prev error | F10 Config | ↑↓ Navigate suggestions | Tab Apply | Esc Cancel"
        } else {
            "Tab Next field | F1 Send | F2 Save | F7 Spell check | F8/F9 Errors | F10 Config | Esc Cancel | @ Contact"
        };
        
        let modified_indicator = if self.is_modified { " [Modified]" } else { "" };
        
        let status = Paragraph::new(format!("{}{}", status_text, modified_indicator))
            .style(theme.get_component_style("status", false))
            .alignment(Alignment::Center);
        
        f.render_widget(status, status_area);
    }
    
    /// Render spell check popup with suggestions
    fn render_spell_check_popup(&self, f: &mut Frame, compose_area: Rect, theme: &Theme) {
        if let Some(ref result) = self.spell_check_result {
            if let Some(error) = result.misspelled_words.get(self.current_spell_error) {
                // Calculate position for spell check popup
                let popup_width = 60;
                let popup_height = (self.spell_suggestions.len() + 5).min(12) as u16;
                
                let popup_area = Rect {
                    x: compose_area.x + compose_area.width.saturating_sub(popup_width + 2),
                    y: compose_area.y + 5,
                    width: popup_width,
                    height: popup_height,
                };
                
                // Clear the background
                f.render_widget(Clear, popup_area);
                
                // Create spell check content
                let mut lines = vec![
                    Line::from(format!("Misspelled: \"{}\"", error.word)),
                    Line::from(""),
                    Line::from("Suggestions:"),
                ];
                
                for (i, suggestion) in self.spell_suggestions.iter().enumerate() {
                    let marker = if i == self.spell_suggestion_selected { "▶ " } else { "  " };
                    let style = if i == self.spell_suggestion_selected {
                        Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    lines.push(Line::from(vec![
                        ratatui::text::Span::styled(marker, style),
                        ratatui::text::Span::styled(suggestion, style),
                    ]));
                }
                
                // Add "Add to dictionary" option
                lines.push(Line::from(""));
                let add_marker = if self.spell_suggestion_selected >= self.spell_suggestions.len() { "▶ " } else { "  " };
                let add_style = if self.spell_suggestion_selected >= self.spell_suggestions.len() {
                    Style::default().bg(Color::Green).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Green)
                };
                lines.push(Line::from(vec![
                    ratatui::text::Span::styled(add_marker, add_style),
                    ratatui::text::Span::styled("Add to dictionary", add_style),
                ]));
                
                let content = Text::from(lines);
                
                let popup = Paragraph::new(content)
                    .block(Block::default()
                        .title(format!("Spell Check ({}/{})", self.current_spell_error + 1, result.misspelled_words.len()))
                        .borders(Borders::ALL)
                        .border_style(theme.get_component_style("popup", true)))
                    .wrap(Wrap { trim: true });
                
                f.render_widget(popup, popup_area);
            }
        }
    }
    
    /// Render spell check configuration popup
    fn render_spell_config_popup(&self, f: &mut Frame, compose_area: Rect, theme: &Theme) {
        // Calculate position for configuration popup
        let popup_width = 50;
        let popup_height = (self.available_languages.len() + 6).min(15) as u16;
        
        let popup_area = Rect {
            x: compose_area.x + (compose_area.width.saturating_sub(popup_width)) / 2,
            y: compose_area.y + (compose_area.height.saturating_sub(popup_height)) / 2,
            width: popup_width,
            height: popup_height,
        };
        
        // Clear the background
        f.render_widget(Clear, popup_area);
        
        // Create configuration content
        let mut lines = vec![
            Line::from("Spell Check Configuration"),
            Line::from(""),
            Line::from(format!("Current Language: {}", self.spell_checker.current_language())),
            Line::from(""),
            Line::from("Available Languages:"),
            Line::from(""),
        ];
        
        for (i, language) in self.available_languages.iter().enumerate() {
            let marker = if i == self.language_selected { "▶ " } else { "  " };
            let style = if i == self.language_selected {
                Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            
            // Format language display name
            let display_name = match language.as_str() {
                "en_US" => "English (US)",
                "es_ES" => "Spanish (ES)",  
                "fr_FR" => "French (FR)",
                "de_DE" => "German (DE)",
                _ => language,
            };
            
            lines.push(Line::from(vec![
                ratatui::text::Span::styled(marker, style),
                ratatui::text::Span::styled(display_name, style),
            ]));
        }
        
        lines.push(Line::from(""));
        lines.push(Line::from("Use ↑/↓ to select, Enter to apply, Esc/F10 to close"));
        
        let content = Text::from(lines);
        
        let popup = Paragraph::new(content)
            .block(Block::default()
                .title("Spell Check Settings")
                .borders(Borders::ALL)
                .border_style(theme.get_component_style("popup", true)))
            .wrap(Wrap { trim: true });
        
        f.render_widget(popup, popup_area);
    }
    
    /// Create highlighted line with misspelled words marked
    fn create_highlighted_line(&self, line: &str, _line_idx: usize) -> Line<'static> {
        if let Some(ref result) = self.spell_check_result {
            let mut spans = Vec::new();
            let mut last_pos = 0;
            
            // Find misspelled words in this line
            let line_start = self.calculate_line_offset(_line_idx);
            let line_end = line_start + line.len();
            
            for error in &result.misspelled_words {
                if error.position >= line_start && error.position < line_end {
                    let error_start = error.position - line_start;
                    let error_end = error_start + error.length;
                    
                    // Add text before error
                    if error_start > last_pos {
                        spans.push(ratatui::text::Span::raw(line[last_pos..error_start].to_string()));
                    }
                    
                    // Add highlighted error
                    let error_style = if self.current_spell_error < result.misspelled_words.len() 
                        && result.misspelled_words[self.current_spell_error].position == error.position {
                        Style::default().bg(Color::Red).fg(Color::White).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().bg(Color::Yellow).fg(Color::Black)
                    };
                    
                    spans.push(ratatui::text::Span::styled(
                        line[error_start..error_end.min(line.len())].to_string(),
                        error_style
                    ));
                    
                    last_pos = error_end.min(line.len());
                }
            }
            
            // Add remaining text
            if last_pos < line.len() {
                spans.push(ratatui::text::Span::raw(line[last_pos..].to_string()));
            }
            
            Line::from(spans)
        } else {
            Line::from(line.to_string())
        }
    }
    
    /// Create highlighted line with cursor
    fn create_highlighted_line_with_cursor(&self, line: &str, cursor_pos: usize) -> Line<'static> {
        let highlighted_line = self.create_highlighted_line(line, self.body_line_index);
        
        // Insert cursor character
        let cursor_inserted = if cursor_pos <= line.len() {
            let mut line_chars: Vec<char> = line.chars().collect();
            line_chars.insert(cursor_pos, '|');
            line_chars.into_iter().collect::<String>()
        } else {
            format!("{}|", line)
        };
        
        // For simplicity, recreate the line with cursor - in a more sophisticated version
        // we would preserve the highlighting and insert cursor properly
        let spans = highlighted_line.spans;
        if spans.is_empty() {
            Line::from(cursor_inserted)
        } else {
            // Use highlighted version but note this is simplified
            Line::from(spans)
        }
    }
    
    /// Calculate character offset of a line in the full body text
    fn calculate_line_offset(&self, line_idx: usize) -> usize {
        self.body_lines.iter()
            .take(line_idx)
            .map(|line| line.len() + 1) // +1 for newline
            .sum()
    }
    
    /// Handle keyboard input
    pub async fn handle_key(&mut self, key: crossterm::event::KeyCode) -> ComposeAction {
        use crossterm::event::KeyCode;
        
        if self.is_autocomplete_visible {
            return self.handle_autocomplete_key(key).await;
        }
        
        if self.is_spell_config_visible {
            return self.handle_spell_config_key(key).await;
        }
        
        match key {
            KeyCode::Esc => ComposeAction::Cancel,
            KeyCode::F(1) => ComposeAction::Send,
            KeyCode::F(2) => ComposeAction::SaveDraft,
            KeyCode::F(7) => {
                // Toggle spell checking
                self.toggle_spell_check().await;
                ComposeAction::Continue
            }
            KeyCode::F(8) => {
                // Next spell error
                if self.spell_check_enabled {
                    self.next_spell_error();
                }
                ComposeAction::Continue
            }
            KeyCode::F(9) => {
                // Previous spell error  
                if self.spell_check_enabled {
                    self.previous_spell_error();
                }
                ComposeAction::Continue
            }
            KeyCode::F(10) => {
                // Open spell check configuration
                self.toggle_spell_config();
                ComposeAction::Continue
            }
            KeyCode::Tab => {
                if self.is_spell_check_visible && self.spell_check_enabled {
                    // Apply selected spell suggestion
                    self.apply_spell_suggestion();
                    self.next_spell_error();
                } else {
                    self.next_field();
                }
                ComposeAction::Continue
            }
            KeyCode::BackTab => {
                self.previous_field();
                ComposeAction::Continue
            }
            KeyCode::Char('@') => {
                // Trigger contact lookup
                self.trigger_contact_autocomplete().await;
                ComposeAction::Continue
            }
            KeyCode::Char(c) => {
                self.insert_char(c);
                
                // Check if we should trigger autocomplete for email fields
                if matches!(self.current_field, ComposeField::To | ComposeField::Cc | ComposeField::Bcc) {
                    self.update_autocomplete().await;
                }
                
                ComposeAction::Continue
            }
            KeyCode::Backspace => {
                self.delete_char();
                
                // Update autocomplete if in email field
                if matches!(self.current_field, ComposeField::To | ComposeField::Cc | ComposeField::Bcc) {
                    self.update_autocomplete().await;
                }
                
                ComposeAction::Continue
            }
            KeyCode::Enter => {
                if self.current_field == ComposeField::Body {
                    self.insert_newline();
                } else {
                    self.next_field();
                }
                ComposeAction::Continue
            }
            KeyCode::Left => {
                self.move_cursor_left();
                ComposeAction::Continue
            }
            KeyCode::Right => {
                self.move_cursor_right();
                ComposeAction::Continue
            }
            KeyCode::Up => {
                if self.is_spell_check_visible && self.spell_check_enabled {
                    // Navigate spell check suggestions
                    if self.spell_suggestion_selected > 0 {
                        self.spell_suggestion_selected -= 1;
                    }
                } else if self.current_field == ComposeField::Body {
                    self.move_cursor_up();
                }
                ComposeAction::Continue
            }
            KeyCode::Down => {
                if self.is_spell_check_visible && self.spell_check_enabled {
                    // Navigate spell check suggestions (including "Add to dictionary" option)
                    let max_index = self.spell_suggestions.len(); // +1 for "Add to dictionary"
                    if self.spell_suggestion_selected < max_index {
                        self.spell_suggestion_selected += 1;
                    }
                } else if self.current_field == ComposeField::Body {
                    self.move_cursor_down();
                }
                ComposeAction::Continue
            }
            _ => ComposeAction::Continue,
        }
    }
    
    /// Handle autocomplete-specific keyboard input
    async fn handle_autocomplete_key(&mut self, key: crossterm::event::KeyCode) -> ComposeAction {
        use crossterm::event::KeyCode;
        
        match key {
            KeyCode::Esc => {
                self.hide_autocomplete();
                ComposeAction::Continue
            }
            KeyCode::Up => {
                if self.autocomplete_selected > 0 {
                    self.autocomplete_selected -= 1;
                }
                ComposeAction::Continue
            }
            KeyCode::Down => {
                if self.autocomplete_selected < self.autocomplete_suggestions.len().saturating_sub(1) {
                    self.autocomplete_selected += 1;
                }
                ComposeAction::Continue
            }
            KeyCode::Enter | KeyCode::Tab => {
                self.select_autocomplete_suggestion();
                ComposeAction::Continue
            }
            KeyCode::Char(c) => {
                // Continue typing and update autocomplete
                self.hide_autocomplete();
                self.insert_char(c);
                self.update_autocomplete().await;
                ComposeAction::Continue
            }
            KeyCode::Backspace => {
                self.hide_autocomplete();
                self.delete_char();
                self.update_autocomplete().await;
                ComposeAction::Continue
            }
            _ => ComposeAction::Continue,
        }
    }
    
    /// Trigger contact autocomplete manually
    async fn trigger_contact_autocomplete(&mut self) {
        if matches!(self.current_field, ComposeField::To | ComposeField::Cc | ComposeField::Bcc) {
            // Get current input to use as search query
            let query = self.get_current_field_value();
            let search_query = query.split(',').last().unwrap_or("").trim();
            
            if !search_query.is_empty() {
                match self.contacts_manager.search_by_email(search_query, Some(10)).await {
                    Ok(contacts) => {
                        self.autocomplete_suggestions = contacts;
                        self.autocomplete_selected = 0;
                        self.is_autocomplete_visible = !self.autocomplete_suggestions.is_empty();
                    }
                    Err(e) => {
                        tracing::error!("Failed to search contacts: {}", e);
                    }
                }
            }
        }
    }
    
    /// Update autocomplete suggestions based on current input
    async fn update_autocomplete(&mut self) {
        let current_value = self.get_current_field_value();
        let last_entry = current_value.split(',').last().unwrap_or("").trim();
        
        if last_entry.len() >= 2 {
            match self.contacts_manager.search_by_email(last_entry, Some(5)).await {
                Ok(contacts) => {
                    self.autocomplete_suggestions = contacts;
                    self.autocomplete_selected = 0;
                    self.is_autocomplete_visible = !self.autocomplete_suggestions.is_empty();
                }
                Err(_) => {
                    self.hide_autocomplete();
                }
            }
        } else {
            self.hide_autocomplete();
        }
    }
    
    /// Select the currently highlighted autocomplete suggestion
    fn select_autocomplete_suggestion(&mut self) {
        if let Some(contact) = self.autocomplete_suggestions.get(self.autocomplete_selected) {
            if let Some(email) = contact.primary_email() {
                let current_value = self.get_current_field_value();
                let parts: Vec<&str> = current_value.split(',').collect();
                
                let new_value = if parts.len() > 1 {
                    // Replace the last entry
                    let mut new_parts = parts;
                    new_parts.pop(); // Remove last incomplete entry
                    let formatted_contact = format!("{} <{}>", contact.display_name, email.address);
                    new_parts.push(&formatted_contact);
                    new_parts.join(", ")
                } else {
                    // Replace entire field
                    format!("{} <{}>", contact.display_name, email.address)
                };
                
                self.set_current_field_value(new_value);
            }
        }
        
        self.hide_autocomplete();
    }
    
    /// Hide autocomplete suggestions
    fn hide_autocomplete(&mut self) {
        self.is_autocomplete_visible = false;
        self.autocomplete_suggestions.clear();
        self.autocomplete_selected = 0;
    }
    
    /// Navigation methods
    fn next_field(&mut self) {
        self.current_field = match self.current_field {
            ComposeField::To => ComposeField::Cc,
            ComposeField::Cc => ComposeField::Bcc,
            ComposeField::Bcc => ComposeField::Subject,
            ComposeField::Subject => ComposeField::Body,
            ComposeField::Body => ComposeField::To,
        };
    }
    
    fn previous_field(&mut self) {
        self.current_field = match self.current_field {
            ComposeField::To => ComposeField::Body,
            ComposeField::Cc => ComposeField::To,
            ComposeField::Bcc => ComposeField::Cc,
            ComposeField::Subject => ComposeField::Bcc,
            ComposeField::Body => ComposeField::Subject,
        };
    }
    
    /// Text editing methods
    fn insert_char(&mut self, c: char) {
        self.mark_content_modified();
        
        match self.current_field {
            ComposeField::To => {
                self.to_field.insert(self.to_cursor, c);
                self.to_cursor += 1;
            }
            ComposeField::Cc => {
                self.cc_field.insert(self.cc_cursor, c);
                self.cc_cursor += 1;
            }
            ComposeField::Bcc => {
                self.bcc_field.insert(self.bcc_cursor, c);
                self.bcc_cursor += 1;
            }
            ComposeField::Subject => {
                self.subject_field.insert(self.subject_cursor, c);
                self.subject_cursor += 1;
            }
            ComposeField::Body => {
                if let Some(line) = self.body_lines.get_mut(self.body_line_index) {
                    line.insert(self.body_cursor, c);
                    self.body_cursor += 1;
                }
            }
        }
    }
    
    fn delete_char(&mut self) {
        if self.get_cursor_position() == 0 {
            return;
        }
        
        self.mark_content_modified();
        
        match self.current_field {
            ComposeField::To => {
                if self.to_cursor > 0 {
                    self.to_field.remove(self.to_cursor - 1);
                    self.to_cursor -= 1;
                }
            }
            ComposeField::Cc => {
                if self.cc_cursor > 0 {
                    self.cc_field.remove(self.cc_cursor - 1);
                    self.cc_cursor -= 1;
                }
            }
            ComposeField::Bcc => {
                if self.bcc_cursor > 0 {
                    self.bcc_field.remove(self.bcc_cursor - 1);
                    self.bcc_cursor -= 1;
                }
            }
            ComposeField::Subject => {
                if self.subject_cursor > 0 {
                    self.subject_field.remove(self.subject_cursor - 1);
                    self.subject_cursor -= 1;
                }
            }
            ComposeField::Body => {
                if self.body_cursor > 0 {
                    if let Some(line) = self.body_lines.get_mut(self.body_line_index) {
                        line.remove(self.body_cursor - 1);
                        self.body_cursor -= 1;
                    }
                } else if self.body_line_index > 0 {
                    // Join with previous line
                    let current_line = self.body_lines.remove(self.body_line_index);
                    self.body_line_index -= 1;
                    if let Some(prev_line) = self.body_lines.get_mut(self.body_line_index) {
                        self.body_cursor = prev_line.len();
                        prev_line.push_str(&current_line);
                    }
                }
            }
        }
    }
    
    fn insert_newline(&mut self) {
        if self.current_field == ComposeField::Body {
            self.mark_content_modified();
            
            // Split current line at cursor
            let current_line = self.body_lines.get(self.body_line_index).cloned().unwrap_or_default();
            let (before, after) = current_line.split_at(self.body_cursor);
            
            // Update current line and insert new line
            self.body_lines[self.body_line_index] = before.to_string();
            self.body_lines.insert(self.body_line_index + 1, after.to_string());
            
            // Move cursor to start of new line
            self.body_line_index += 1;
            self.body_cursor = 0;
        }
    }
    
    /// Cursor movement methods
    fn move_cursor_left(&mut self) {
        match self.current_field {
            ComposeField::To => {
                if self.to_cursor > 0 {
                    self.to_cursor -= 1;
                }
            }
            ComposeField::Cc => {
                if self.cc_cursor > 0 {
                    self.cc_cursor -= 1;
                }
            }
            ComposeField::Bcc => {
                if self.bcc_cursor > 0 {
                    self.bcc_cursor -= 1;
                }
            }
            ComposeField::Subject => {
                if self.subject_cursor > 0 {
                    self.subject_cursor -= 1;
                }
            }
            ComposeField::Body => {
                if self.body_cursor > 0 {
                    self.body_cursor -= 1;
                } else if self.body_line_index > 0 {
                    self.body_line_index -= 1;
                    self.body_cursor = self.body_lines.get(self.body_line_index)
                        .map(|line| line.len())
                        .unwrap_or(0);
                }
            }
        }
    }
    
    fn move_cursor_right(&mut self) {
        match self.current_field {
            ComposeField::To => {
                if self.to_cursor < self.to_field.len() {
                    self.to_cursor += 1;
                }
            }
            ComposeField::Cc => {
                if self.cc_cursor < self.cc_field.len() {
                    self.cc_cursor += 1;
                }
            }
            ComposeField::Bcc => {
                if self.bcc_cursor < self.bcc_field.len() {
                    self.bcc_cursor += 1;
                }
            }
            ComposeField::Subject => {
                if self.subject_cursor < self.subject_field.len() {
                    self.subject_cursor += 1;
                }
            }
            ComposeField::Body => {
                let current_line = self.body_lines.get(self.body_line_index);
                if let Some(line) = current_line {
                    if self.body_cursor < line.len() {
                        self.body_cursor += 1;
                    } else if self.body_line_index < self.body_lines.len() - 1 {
                        self.body_line_index += 1;
                        self.body_cursor = 0;
                    }
                }
            }
        }
    }
    
    fn move_cursor_up(&mut self) {
        if self.current_field == ComposeField::Body && self.body_line_index > 0 {
            self.body_line_index -= 1;
            let line_len = self.body_lines.get(self.body_line_index)
                .map(|line| line.len())
                .unwrap_or(0);
            self.body_cursor = self.body_cursor.min(line_len);
        }
    }
    
    fn move_cursor_down(&mut self) {
        if self.current_field == ComposeField::Body && self.body_line_index < self.body_lines.len() - 1 {
            self.body_line_index += 1;
            let line_len = self.body_lines.get(self.body_line_index)
                .map(|line| line.len())
                .unwrap_or(0);
            self.body_cursor = self.body_cursor.min(line_len);
        }
    }
    
    /// Helper methods
    fn get_current_field_value(&self) -> String {
        match self.current_field {
            ComposeField::To => self.to_field.clone(),
            ComposeField::Cc => self.cc_field.clone(),
            ComposeField::Bcc => self.bcc_field.clone(),
            ComposeField::Subject => self.subject_field.clone(),
            ComposeField::Body => self.body_lines.join("\n"),
        }
    }
    
    fn set_current_field_value(&mut self, value: String) {
        self.mark_content_modified();
        
        match self.current_field {
            ComposeField::To => {
                self.to_field = value;
                self.to_cursor = self.to_field.len();
            }
            ComposeField::Cc => {
                self.cc_field = value;
                self.cc_cursor = self.cc_field.len();
            }
            ComposeField::Bcc => {
                self.bcc_field = value;
                self.bcc_cursor = self.bcc_field.len();
            }
            ComposeField::Subject => {
                self.subject_field = value;
                self.subject_cursor = self.subject_field.len();
            }
            ComposeField::Body => {
                self.body_lines = value.lines().map(|s| s.to_string()).collect();
                if self.body_lines.is_empty() {
                    self.body_lines.push(String::new());
                }
                self.body_line_index = self.body_lines.len() - 1;
                self.body_cursor = self.body_lines.last()
                    .map(|line| line.len())
                    .unwrap_or(0);
            }
        }
    }
    
    fn get_cursor_position(&self) -> usize {
        match self.current_field {
            ComposeField::To => self.to_cursor,
            ComposeField::Cc => self.cc_cursor,
            ComposeField::Bcc => self.bcc_cursor,
            ComposeField::Subject => self.subject_cursor,
            ComposeField::Body => self.body_cursor,
        }
    }
    
    /// Get email data for sending
    pub fn get_email_data(&self) -> EmailComposeData {
        EmailComposeData {
            to: self.to_field.clone(),
            cc: self.cc_field.clone(),
            bcc: self.bcc_field.clone(),
            subject: self.subject_field.clone(),
            body: self.body_lines.join("\n"),
        }
    }
    
    /// Check if the compose form has been modified
    pub fn is_modified(&self) -> bool {
        self.is_modified
    }
    
    /// Clear the modified flag (e.g., after saving)
    pub fn clear_modified(&mut self) {
        self.is_modified = false;
        self.has_auto_save_changes = false;
    }
    
    /// Check if auto-save is needed based on time interval and changes
    pub fn should_auto_save(&self) -> bool {
        if !self.has_auto_save_changes {
            return false;
        }
        
        match self.last_auto_save {
            Some(last_save) => last_save.elapsed() >= self.auto_save_interval,
            None => true, // First auto-save
        }
    }
    
    /// Get the current draft ID for auto-save operations
    pub fn current_draft_id(&self) -> Option<&String> {
        self.current_draft_id.as_ref()
    }
    
    /// Set the current draft ID (when loading an existing draft)
    pub fn set_current_draft_id(&mut self, draft_id: Option<String>) {
        self.current_draft_id = draft_id;
    }
    
    /// Mark that auto-save has been performed
    pub fn mark_auto_saved(&mut self) {
        self.last_auto_save = Some(std::time::Instant::now());
        self.has_auto_save_changes = false;
    }
    
    /// Load compose data from a draft (preserving draft ID)
    pub fn load_from_draft(&mut self, compose_data: crate::ui::EmailComposeData, draft_id: String) {
        self.to_field = compose_data.to;
        self.cc_field = compose_data.cc;
        self.bcc_field = compose_data.bcc;
        self.subject_field = compose_data.subject;
        self.body_text = compose_data.body.clone();
        self.body_lines = if compose_data.body.is_empty() {
            vec![String::new()]
        } else {
            compose_data.body.lines().map(|s| s.to_string()).collect()
        };
        self.current_draft_id = Some(draft_id);
        self.is_modified = false;
        self.has_auto_save_changes = false;
        self.last_auto_save = Some(std::time::Instant::now());
    }
    
    /// Get auto-save interval in seconds
    pub fn auto_save_interval_secs(&self) -> u64 {
        self.auto_save_interval.as_secs()
    }
    
    /// Set auto-save interval
    pub fn set_auto_save_interval(&mut self, seconds: u64) {
        self.auto_save_interval = std::time::Duration::from_secs(seconds);
    }
    
    /// Mark content as modified (triggers both manual and auto-save flags)
    fn mark_content_modified(&mut self) {
        self.is_modified = true;
        self.has_auto_save_changes = true;
    }
    
    /// Check if auto-save should be triggered and return action if needed
    pub fn check_auto_save(&self) -> Option<ComposeAction> {
        if self.should_auto_save() {
            Some(ComposeAction::AutoSave)
        } else {
            None
        }
    }
    
    /// Toggle spell checking on/off
    async fn toggle_spell_check(&mut self) {
        self.spell_check_enabled = !self.spell_check_enabled;
        
        if self.spell_check_enabled {
            // Perform spell check on current content
            self.run_spell_check().await;
        } else {
            // Clear spell check results
            self.spell_check_result = None;
            self.is_spell_check_visible = false;
        }
    }
    
    /// Run spell check on current field content
    async fn run_spell_check(&mut self) {
        let text_to_check = match self.current_field {
            ComposeField::Subject => &self.subject_field,
            ComposeField::Body => &self.body_lines.join("\n"),
            _ => return, // Don't spell check email addresses
        };
        
        if !text_to_check.trim().is_empty() {
            let result = self.spell_checker.check_text(text_to_check);
            self.spell_check_result = Some(result);
            self.current_spell_error = 0;
            self.is_spell_check_visible = self.spell_check_result.as_ref()
                .map(|r| !r.misspelled_words.is_empty())
                .unwrap_or(false);
        }
    }
    
    /// Move to next spell check error
    pub fn next_spell_error(&mut self) {
        if let Some(ref result) = self.spell_check_result {
            if !result.misspelled_words.is_empty() {
                self.current_spell_error = (self.current_spell_error + 1) % result.misspelled_words.len();
                self.update_spell_suggestions();
            }
        }
    }
    
    /// Move to previous spell check error
    pub fn previous_spell_error(&mut self) {
        if let Some(ref result) = self.spell_check_result {
            if !result.misspelled_words.is_empty() {
                if self.current_spell_error == 0 {
                    self.current_spell_error = result.misspelled_words.len() - 1;
                } else {
                    self.current_spell_error -= 1;
                }
                self.update_spell_suggestions();
            }
        }
    }
    
    /// Update suggestions for current spell error
    fn update_spell_suggestions(&mut self) {
        if let Some(ref result) = self.spell_check_result {
            if let Some(error) = result.misspelled_words.get(self.current_spell_error) {
                self.spell_suggestions = error.suggestions.clone();
                self.spell_suggestion_selected = 0;
            }
        }
    }
    
    /// Apply selected spell suggestion or add to dictionary
    pub fn apply_spell_suggestion(&mut self) {
        if let Some(ref result) = self.spell_check_result {
            if let Some(error) = result.misspelled_words.get(self.current_spell_error) {
                if self.spell_suggestion_selected >= self.spell_suggestions.len() {
                    // "Add to dictionary" option selected
                    self.add_word_to_dictionary(error.word.clone());
                    return;
                }
                
                if self.spell_suggestions.is_empty() || self.spell_suggestion_selected >= self.spell_suggestions.len() {
                    return;
                }
                
                let replacement = &self.spell_suggestions[self.spell_suggestion_selected];
                
                match self.current_field {
                    ComposeField::Subject => {
                        let start = error.position;
                        let end = start + error.length;
                        if end <= self.subject_field.len() {
                            self.subject_field.replace_range(start..end, replacement);
                            self.subject_cursor = start + replacement.len();
                        }
                    }
                    ComposeField::Body => {
                        // Replace in body text - more complex due to line structure
                        let full_text = self.body_lines.join("\n");
                        let start = error.position;
                        let end = start + error.length;
                        if end <= full_text.len() {
                            let mut new_text = full_text;
                            new_text.replace_range(start..end, replacement);
                            self.body_lines = new_text.lines().map(|s| s.to_string()).collect();
                            if self.body_lines.is_empty() {
                                self.body_lines.push(String::new());
                            }
                        }
                    }
                    _ => {}
                }
                
                self.mark_content_modified();
            }
        }
    }
    
    /// Add word to custom dictionary
    fn add_word_to_dictionary(&mut self, word: String) {
        // Add to spell checker's custom words
        self.spell_checker.add_custom_word(word.clone());
        
        // Re-run spell check to update results
        if self.spell_check_enabled {
            // This would need to be async, but for now we'll mark it as handled
            // by removing it from current results
            if let Some(ref mut result) = self.spell_check_result {
                result.misspelled_words.retain(|w| w.word != word);
                result.error_count = result.misspelled_words.len();
                
                // Update visibility
                self.is_spell_check_visible = !result.misspelled_words.is_empty();
                
                // Reset current error if we're at the end
                if self.current_spell_error >= result.misspelled_words.len() {
                    self.current_spell_error = result.misspelled_words.len().saturating_sub(1);
                }
            }
        }
    }
    
    /// Toggle spell check configuration popup
    fn toggle_spell_config(&mut self) {
        self.is_spell_config_visible = !self.is_spell_config_visible;
        if self.is_spell_config_visible {
            // Find current language index
            let current_lang = self.spell_checker.current_language();
            if let Some(index) = self.available_languages.iter().position(|lang| lang == current_lang) {
                self.language_selected = index;
            }
        }
    }
    
    /// Handle spell configuration popup key input
    async fn handle_spell_config_key(&mut self, key: crossterm::event::KeyCode) -> ComposeAction {
        use crossterm::event::KeyCode;
        
        match key {
            KeyCode::Esc | KeyCode::F(10) => {
                self.is_spell_config_visible = false;
                ComposeAction::Continue
            }
            KeyCode::Up => {
                if self.language_selected > 0 {
                    self.language_selected -= 1;
                }
                ComposeAction::Continue
            }
            KeyCode::Down => {
                if self.language_selected + 1 < self.available_languages.len() {
                    self.language_selected += 1;
                }
                ComposeAction::Continue
            }
            KeyCode::Enter => {
                // Apply selected language
                if self.language_selected < self.available_languages.len() {
                    let selected_lang = self.available_languages[self.language_selected].clone();
                    if let Err(e) = self.spell_checker.set_language(&selected_lang).await {
                        tracing::error!("Failed to set language to {}: {}", selected_lang, e);
                    } else {
                        tracing::info!("Spell check language changed to: {}", selected_lang);
                        // Re-run spell check if enabled
                        if self.spell_check_enabled {
                            self.run_spell_check().await;
                        }
                    }
                }
                self.is_spell_config_visible = false;
                ComposeAction::Continue
            }
            _ => ComposeAction::Continue,
        }
    }
}

/// Actions that can be returned from the compose UI
#[derive(Debug, Clone, PartialEq)]
pub enum ComposeAction {
    Continue,
    Send,
    SaveDraft,
    AutoSave,
    Cancel,
    StartCompose,
}

/// Email composition data
#[derive(Debug, Clone)]
pub struct EmailComposeData {
    pub to: String,
    pub cc: String,
    pub bcc: String,
    pub subject: String,
    pub body: String,
}

impl EmailComposeData {
    /// Parse email addresses from a field (handles "Name <email>" format)
    pub fn parse_addresses(field: &str) -> Vec<String> {
        field.split(',')
            .map(|addr| addr.trim())
            .filter(|addr| !addr.is_empty())
            .map(|addr| {
                // Extract email from "Name <email>" format
                if let Some(start) = addr.find('<') {
                    if let Some(end) = addr.find('>') {
                        addr[start + 1..end].to_string()
                    } else {
                        addr.to_string()
                    }
                } else {
                    addr.to_string()
                }
            })
            .collect()
    }
    
    /// Get all recipient addresses
    pub fn get_all_recipients(&self) -> Vec<String> {
        let mut recipients = Vec::new();
        recipients.extend(Self::parse_addresses(&self.to));
        recipients.extend(Self::parse_addresses(&self.cc));
        recipients.extend(Self::parse_addresses(&self.bcc));
        recipients
    }
    
    /// Validate that required fields are present
    pub fn validate(&self) -> Result<(), String> {
        if self.to.trim().is_empty() {
            return Err("To field is required".to_string());
        }
        
        if self.subject.trim().is_empty() {
            return Err("Subject is required".to_string());
        }
        
        // Validate email addresses
        let all_recipients = self.get_all_recipients();
        for addr in all_recipients {
            if !addr.contains('@') {
                return Err(format!("Invalid email address: {}", addr));
            }
        }
        
        Ok(())
    }
}