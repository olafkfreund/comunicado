use crate::contacts::{Contact, ContactsManager};
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

/// Email composition UI with contact autocomplete
pub struct ComposeUI {
    contacts_manager: Arc<ContactsManager>,
    
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
        Self {
            contacts_manager,
            to_field: String::new(),
            cc_field: String::new(),
            bcc_field: String::new(),
            subject_field: String::new(),
            body_text: String::new(),
            current_field: ComposeField::To,
            is_autocomplete_visible: false,
            autocomplete_suggestions: Vec::new(),
            autocomplete_selected: 0,
            to_cursor: 0,
            cc_cursor: 0,
            bcc_cursor: 0,
            subject_cursor: 0,
            body_cursor: 0,
            body_lines: vec![String::new()],
            body_line_index: 0,
            is_modified: false,
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
        
        // Create text with cursor if focused
        let mut text = Text::default();
        
        for (line_idx, line) in self.body_lines.iter().enumerate() {
            if is_focused && line_idx == self.body_line_index {
                // Show cursor on current line
                let mut chars: Vec<char> = line.chars().collect();
                let cursor_pos = self.body_cursor.min(chars.len());
                chars.insert(cursor_pos, '|');
                let line_with_cursor: String = chars.into_iter().collect();
                text.lines.push(Line::from(line_with_cursor));
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
        } else {
            "Tab Next field | F1 Send | F2 Save draft | Esc Cancel | @ Contact lookup"
        };
        
        let modified_indicator = if self.is_modified { " [Modified]" } else { "" };
        
        let status = Paragraph::new(format!("{}{}", status_text, modified_indicator))
            .style(theme.get_component_style("status", false))
            .alignment(Alignment::Center);
        
        f.render_widget(status, status_area);
    }
    
    /// Handle keyboard input
    pub async fn handle_key(&mut self, key: crossterm::event::KeyCode) -> ComposeAction {
        use crossterm::event::KeyCode;
        
        if self.is_autocomplete_visible {
            return self.handle_autocomplete_key(key).await;
        }
        
        match key {
            KeyCode::Esc => ComposeAction::Cancel,
            KeyCode::F(1) => ComposeAction::Send,
            KeyCode::F(2) => ComposeAction::SaveDraft,
            KeyCode::Tab => {
                self.next_field();
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
                if self.current_field == ComposeField::Body {
                    self.move_cursor_up();
                }
                ComposeAction::Continue
            }
            KeyCode::Down => {
                if self.current_field == ComposeField::Body {
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
                        eprintln!("Failed to search contacts: {}", e);
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
        self.is_modified = true;
        
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
        
        self.is_modified = true;
        
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
            self.is_modified = true;
            
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
        self.is_modified = true;
        
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
    }
}

/// Actions that can be returned from the compose UI
#[derive(Debug, Clone, PartialEq)]
pub enum ComposeAction {
    Continue,
    Send,
    SaveDraft,
    Cancel,
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