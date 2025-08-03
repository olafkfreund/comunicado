//! TUI Rendering Implementation
//! 
//! Handles all visual rendering for the notes TUI, including layouts,
//! widgets, and visual styling with responsive design.

use super::tui::{NoteTUI, TUIMode, PopupState};
use super::types::Note;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Alignment, Margin},
    style::{Modifier, Style},
    text::Line,
    widgets::{
        Block, Borders, Clear, List, ListItem, Paragraph, Wrap,
        Scrollbar, ScrollbarOrientation, ScrollbarState,
    },
    Frame,
};
use chrono::Local;

impl NoteTUI {
    /// Render the complete TUI interface
    pub fn render(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Main content
                Constraint::Length(3), // Footer
            ])
            .split(f.size());
        
        // Render header
        self.render_header(f, chunks[0]);
        
        // Render main content based on mode
        match self.current_mode() {
            TUIMode::Browse => self.render_browse_mode(f, chunks[1]),
            TUIMode::View => self.render_view_mode(f, chunks[1]),
            TUIMode::Edit => self.render_edit_mode(f, chunks[1]),
            TUIMode::Search => self.render_search_mode(f, chunks[1]),
            TUIMode::Create => self.render_create_mode(f, chunks[1]),
            TUIMode::Settings => self.render_settings_mode(f, chunks[1]),
            TUIMode::Help => self.render_help_mode(f, chunks[1]),
        }
        
        // Render footer
        self.render_footer(f, chunks[2]);
        
        // Render popup if present
        if let Some(ref popup) = self.popup() {
            self.render_popup(f, popup);
        }
    }
    
    /// Render the header section
    fn render_header(&self, f: &mut Frame, area: Rect) {
        let title = match self.current_mode() {
            TUIMode::Browse => "📝 Comunicado Notes - Browse",
            TUIMode::View => "👁 Comunicado Notes - View",
            TUIMode::Edit => "✏️ Comunicado Notes - Edit",
            TUIMode::Search => "🔍 Comunicado Notes - Search",
            TUIMode::Create => "➕ Comunicado Notes - Create",
            TUIMode::Settings => "⚙️ Comunicado Notes - Settings",
            TUIMode::Help => "❓ Comunicado Notes - Help",
        };
        
        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .title_alignment(Alignment::Center)
            .border_style(Style::default().fg(self.config().theme.border));
        
        let inner = block.inner(area);
        f.render_widget(block, area);
        
        // Show mode indicator and stats
        let stats_text = if self.current_notes().is_empty() {
            "No notes".to_string()
        } else {
            format!("{} notes", self.current_notes().len())
        };
        
        let mode_info = Paragraph::new(stats_text)
            .style(Style::default().fg(self.config().theme.text))
            .alignment(Alignment::Center);
        
        f.render_widget(mode_info, inner);
    }
    
    /// Render the footer section
    fn render_footer(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.config().theme.border));
        
        let inner = block.inner(area);
        f.render_widget(block, area);
        
        // Show keybindings based on current mode
        let keybindings = self.get_keybindings_for_mode();
        
        let footer_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(0),      // Keybindings
                Constraint::Length(20),  // Status/Error
            ])
            .split(inner);
        
        // Render keybindings
        let keys_text = keybindings.join(" | ");
        let keys_paragraph = Paragraph::new(keys_text)
            .style(Style::default().fg(self.config().theme.text))
            .wrap(Wrap { trim: true });
        f.render_widget(keys_paragraph, footer_chunks[0]);
        
        // Render status or error message
        if let Some(error) = self.error_message() {
            let error_paragraph = Paragraph::new(error)
                .style(Style::default().fg(self.config().theme.error))
                .alignment(Alignment::Right);
            f.render_widget(error_paragraph, footer_chunks[1]);
        } else if let Some(status) = self.status_message() {
            let status_paragraph = Paragraph::new(status)
                .style(Style::default().fg(self.config().theme.success))
                .alignment(Alignment::Right);
            f.render_widget(status_paragraph, footer_chunks[1]);
        }
    }
    
    /// Render browse mode
    fn render_browse_mode(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(60), // Note list
                Constraint::Percentage(40), // Preview pane
            ])
            .split(area);
        
        // Render note list first
        self.render_note_list(f, chunks[0]);
        
        // Get selection and notes for preview after mutable borrow is done
        let selected = self.list_state().selected();
        let preview_note = selected.and_then(|i| self.current_notes().get(i).cloned());
        
        // Render preview
        if let Some(note) = preview_note {
            self.render_note_preview(f, chunks[1], &note);
        } else {
            self.render_empty_preview(f, chunks[1]);
        }
    }
    
    /// Render note list widget
    fn render_note_list(&mut self, f: &mut Frame, area: Rect) {
        // Create a block for borrowing issues
        let (items, border_style, notes_len, selected_pos) = {
            let selected = self.list_state().selected();
            let theme = &self.config().theme;
            let notes = self.current_notes();
            
            let items: Vec<ListItem> = notes.iter().enumerate().map(|(i, note)| {
                let style = if Some(i) == selected {
                    Style::default()
                        .bg(theme.highlight)
                        .fg(theme.background)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.text)
                };
                
                let modified = note.modified_at.with_timezone(&Local).format("%m/%d %H:%M");
                let tags_str = if note.tags.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", note.tags.join(", "))
                };
                
                let content = format!(
                    "{} {} ({}){}", 
                    if note.is_deleted { "🗑" } else { "📄" },
                    note.title,
                    modified,
                    tags_str
                );
                
                ListItem::new(content).style(style)
            }).collect();
            
            (items, Style::default().fg(theme.border), notes.len(), selected.unwrap_or(0))
        };
        
        let list = List::new(items)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Notes")
                .border_style(border_style))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("► ");
        
        f.render_stateful_widget(list, area, self.list_state());
        
        // Render scrollbar
        if notes_len > area.height as usize {
            let scrollbar = Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));
            
            let mut scrollbar_state = ScrollbarState::default()
                .content_length(notes_len)
                .position(selected_pos);
            
            f.render_stateful_widget(
                scrollbar,
                area.inner(&Margin { vertical: 1, horizontal: 0 }),
                &mut scrollbar_state,
            );
        }
    }
    
    /// Render note preview pane
    fn render_note_preview(&self, f: &mut Frame, area: Rect, note: &Note) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Preview")
            .border_style(Style::default().fg(self.config().theme.border));
        
        let inner = block.inner(area);
        f.render_widget(block, area);
        
        // Split preview into metadata and content
        let preview_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4), // Metadata
                Constraint::Min(0),    // Content
            ])
            .split(inner);
        
        // Render metadata
        let metadata = format!(
            "📅 Created: {}\n📝 Modified: {}\n📊 Words: {} | Size: {} bytes",
            note.created_at.with_timezone(&Local).format("%Y-%m-%d %H:%M"),
            note.modified_at.with_timezone(&Local).format("%Y-%m-%d %H:%M"),
            note.word_count,
            note.file_size
        );
        
        let metadata_paragraph = Paragraph::new(metadata)
            .style(Style::default().fg(self.config().theme.secondary))
            .wrap(Wrap { trim: true });
        f.render_widget(metadata_paragraph, preview_chunks[0]);
        
        // Render content preview (first few lines)
        let preview_content = note.content.lines()
            .take(preview_chunks[1].height as usize - 2)
            .collect::<Vec<_>>()
            .join("\n");
        
        let content_paragraph = Paragraph::new(preview_content)
            .style(Style::default().fg(self.config().theme.text))
            .wrap(Wrap { trim: true });
        f.render_widget(content_paragraph, preview_chunks[1]);
    }
    
    /// Render empty preview pane
    fn render_empty_preview(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Preview")
            .border_style(Style::default().fg(self.config().theme.border));
        
        let inner = block.inner(area);
        f.render_widget(block, area);
        
        let empty_text = "Select a note to preview";
        let empty_paragraph = Paragraph::new(empty_text)
            .style(Style::default().fg(self.config().theme.secondary))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        
        f.render_widget(empty_paragraph, inner);
    }
    
    /// Render view mode
    fn render_view_mode(&self, f: &mut Frame, area: Rect) {
        if let Some(note) = self.current_note() {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Note info
                    Constraint::Min(0),    // Content
                ])
                .split(area);
            
            // Render note info
            self.render_note_info_bar(f, chunks[0], note);
            
            // Render content
            self.render_note_content(f, chunks[1], note);
        } else {
            self.render_no_note_selected(f, area);
        }
    }
    
    /// Render note info bar
    fn render_note_info_bar(&self, f: &mut Frame, area: Rect, note: &Note) {
        let info = format!(
            "📄 {} | 📅 {} | 📝 {} words | 🏷️ {}",
            note.title,
            note.modified_at.with_timezone(&Local).format("%Y-%m-%d %H:%M"),
            note.word_count,
            if note.tags.is_empty() { "No tags".to_string() } else { note.tags.join(", ") }
        );
        
        let paragraph = Paragraph::new(info)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Note Info")
                .border_style(Style::default().fg(self.config().theme.border)))
            .style(Style::default().fg(self.config().theme.text))
            .wrap(Wrap { trim: true });
        
        f.render_widget(paragraph, area);
    }
    
    /// Render note content
    fn render_note_content(&self, f: &mut Frame, area: Rect, note: &Note) {
        let paragraph = Paragraph::new(note.content.as_str())
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Content")
                .border_style(Style::default().fg(self.config().theme.border)))
            .style(Style::default().fg(self.config().theme.text))
            .wrap(Wrap { trim: true });
        
        f.render_widget(paragraph, area);
    }
    
    /// Render edit mode
    fn render_edit_mode(&self, f: &mut Frame, area: Rect) {
        f.render_widget(self.editor().widget(), area);
    }
    
    /// Render search mode
    fn render_search_mode(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Search input
                Constraint::Min(0),    // Results
            ])
            .split(area);
        
        // Render search input
        f.render_widget(self.search_input().widget(), chunks[0]);
        
        // Render search results
        if !self.search_results().is_empty() {
            self.render_search_results(f, chunks[1]);
        } else if !self.search_query().is_empty() {
            self.render_no_search_results(f, chunks[1]);
        } else {
            self.render_search_help(f, chunks[1]);
        }
    }
    
    /// Render search results
    fn render_search_results(&self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self.search_results().iter().map(|note| {
            let modified = note.modified_at.with_timezone(&Local).format("%m/%d %H:%M");
            let content = format!("{} ({})", note.title, modified);
            ListItem::new(Line::from(content))
        }).collect();
        
        let list = List::new(items)
            .block(Block::default()
                .borders(Borders::ALL)
                .title(format!("Search Results ({})", self.search_results().len()))
                .border_style(Style::default().fg(self.config().theme.border)))
            .style(Style::default().fg(self.config().theme.text));
        
        f.render_widget(list, area);
    }
    
    /// Render no search results
    fn render_no_search_results(&self, f: &mut Frame, area: Rect) {
        let text = format!("No results found for '{}'", self.search_query());
        let paragraph = Paragraph::new(text)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Search Results")
                .border_style(Style::default().fg(self.config().theme.border)))
            .style(Style::default().fg(self.config().theme.secondary))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        
        f.render_widget(paragraph, area);
    }
    
    /// Render search help
    fn render_search_help(&self, f: &mut Frame, area: Rect) {
        let help_text = "Enter search terms to find notes.\nSupports full-text search across all note content.\nPress Enter to search, Esc to return to browse mode.";
        let paragraph = Paragraph::new(help_text)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Search Help")
                .border_style(Style::default().fg(self.config().theme.border)))
            .style(Style::default().fg(self.config().theme.secondary))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        
        f.render_widget(paragraph, area);
    }
    
    /// Render create mode
    fn render_create_mode(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Instructions
                Constraint::Min(0),    // Editor
            ])
            .split(area);
        
        // Render instructions
        let instructions = "Creating new note. Press Ctrl+S to save, Esc to cancel.";
        let instructions_paragraph = Paragraph::new(instructions)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Instructions")
                .border_style(Style::default().fg(self.config().theme.border)))
            .style(Style::default().fg(self.config().theme.text))
            .wrap(Wrap { trim: true });
        
        f.render_widget(instructions_paragraph, chunks[0]);
        
        // Render editor
        f.render_widget(self.editor().widget(), chunks[1]);
    }
    
    /// Render settings mode
    fn render_settings_mode(&self, f: &mut Frame, area: Rect) {
        let settings_text = format!(
            "Settings\n\n\
            Show Line Numbers: {}\n\
            Vim Keybindings: {}\n\
            Auto Save Interval: {} seconds\n\
            Preview Ratio: {}%\n\
            Max Notes Display: {}\n\n\
            Press 'b' or Esc to return to browse mode.",
            if self.config().show_line_numbers { "Yes" } else { "No" },
            if self.config().vim_keybindings { "Yes" } else { "No" },
            self.config().auto_save_interval,
            self.config().preview_ratio,
            self.config().max_notes_display
        );
        
        let paragraph = Paragraph::new(settings_text)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Settings")
                .border_style(Style::default().fg(self.config().theme.border)))
            .style(Style::default().fg(self.config().theme.text))
            .wrap(Wrap { trim: true });
        
        f.render_widget(paragraph, area);
    }
    
    /// Render help mode
    fn render_help_mode(&self, f: &mut Frame, area: Rect) {
        let help_text = "Comunicado Notes - Help\n\n\
            BROWSE MODE:\n\
            j/↓        - Next note\n\
            k/↑        - Previous note\n\
            g          - First note\n\
            G          - Last note\n\
            Enter/o    - View note\n\
            e          - Edit note\n\
            n          - Create new note\n\
            d          - Delete note\n\
            /          - Search notes\n\
            r          - Refresh notes\n\
            i          - Show note info\n\n\
            VIEW MODE:\n\
            e          - Edit note\n\
            d          - Delete note\n\
            b/Backspace- Back to browse\n\
            j/k        - Scroll content\n\n\
            EDIT MODE:\n\
            Ctrl+S     - Save note\n\
            Esc        - Save and return to view\n\n\
            GLOBAL:\n\
            F1/?       - Show this help\n\
            Ctrl+Q     - Quit application\n\
            Esc        - Return to browse mode\n\n\
            Press any key to return to browse mode.";
        
        let paragraph = Paragraph::new(help_text)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Help")
                .border_style(Style::default().fg(self.config().theme.border)))
            .style(Style::default().fg(self.config().theme.text))
            .wrap(Wrap { trim: true });
        
        f.render_widget(paragraph, area);
    }
    
    /// Render popup dialogs
    fn render_popup(&self, f: &mut Frame, popup: &PopupState) {
        match popup {
            PopupState::ConfirmDelete { note_id: _ } => {
                self.render_confirm_dialog(f, "Delete Note", "Are you sure you want to delete this note? (y/N)");
            }
            PopupState::CreateNote { title_input } => {
                self.render_input_dialog(f, "Create Note", "Enter note title:", title_input);
            }
            PopupState::NoteInfo { note } => {
                self.render_note_info_dialog(f, note);
            }
            PopupState::Error { message } => {
                self.render_error_dialog(f, message);
            }
            PopupState::Help => {
                // Help is rendered as a full-screen mode, not a popup
            }
            _ => {}
        }
    }
    
    /// Render confirmation dialog
    fn render_confirm_dialog(&self, f: &mut Frame, title: &str, message: &str) {
        let popup_area = self.centered_rect(50, 20, f.size());
        
        f.render_widget(Clear, popup_area);
        
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .style(Style::default().bg(self.config().theme.background).fg(self.config().theme.border));
        
        f.render_widget(block, popup_area);
        
        let inner = popup_area.inner(&Margin { vertical: 1, horizontal: 1 });
        let paragraph = Paragraph::new(message)
            .style(Style::default().fg(self.config().theme.text))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        
        f.render_widget(paragraph, inner);
    }
    
    /// Render input dialog
    fn render_input_dialog(&self, f: &mut Frame, title: &str, prompt: &str, input: &tui_textarea::TextArea) {
        let popup_area = self.centered_rect(60, 30, f.size());
        
        f.render_widget(Clear, popup_area);
        
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .style(Style::default().bg(self.config().theme.background).fg(self.config().theme.border));
        
        f.render_widget(block, popup_area);
        
        let inner = popup_area.inner(&Margin { vertical: 1, horizontal: 1 });
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Prompt
                Constraint::Length(3), // Input
                Constraint::Min(0),    // Instructions
            ])
            .split(inner);
        
        // Render prompt
        let prompt_paragraph = Paragraph::new(prompt)
            .style(Style::default().fg(self.config().theme.text));
        f.render_widget(prompt_paragraph, chunks[0]);
        
        // Render input
        f.render_widget(input.widget(), chunks[1]);
        
        // Render instructions
        let instructions = "Press Enter to confirm, Esc to cancel";
        let instructions_paragraph = Paragraph::new(instructions)
            .style(Style::default().fg(self.config().theme.secondary))
            .alignment(Alignment::Center);
        f.render_widget(instructions_paragraph, chunks[2]);
    }
    
    /// Render note info dialog
    fn render_note_info_dialog(&self, f: &mut Frame, note: &Note) {
        let popup_area = self.centered_rect(80, 60, f.size());
        
        f.render_widget(Clear, popup_area);
        
        let block = Block::default()
            .title("Note Information")
            .borders(Borders::ALL)
            .style(Style::default().bg(self.config().theme.background).fg(self.config().theme.border));
        
        f.render_widget(block, popup_area);
        
        let inner = popup_area.inner(&Margin { vertical: 1, horizontal: 1 });
        
        let info_text = format!(
            "Title: {}\n\
            ID: {}\n\
            Created: {}\n\
            Modified: {}\n\
            Word Count: {}\n\
            File Size: {} bytes\n\
            Path: {}\n\
            Tags: {}\n\
            Hash: {}\n\n\
            Links: {} wiki links found\n\n\
            Press any key to close.",
            note.title,
            note.id,
            note.created_at.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S"),
            note.modified_at.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S"),
            note.word_count,
            note.file_size,
            note.path.display(),
            if note.tags.is_empty() { "None".to_string() } else { note.tags.join(", ") },
            note.content_hash,
            note.links.len()
        );
        
        let paragraph = Paragraph::new(info_text)
            .style(Style::default().fg(self.config().theme.text))
            .wrap(Wrap { trim: true });
        
        f.render_widget(paragraph, inner);
    }
    
    /// Render error dialog
    fn render_error_dialog(&self, f: &mut Frame, message: &str) {
        let popup_area = self.centered_rect(60, 20, f.size());
        
        f.render_widget(Clear, popup_area);
        
        let block = Block::default()
            .title("Error")
            .borders(Borders::ALL)
            .style(Style::default().bg(self.config().theme.background).fg(self.config().theme.error));
        
        f.render_widget(block, popup_area);
        
        let inner = popup_area.inner(&Margin { vertical: 1, horizontal: 1 });
        let paragraph = Paragraph::new(message)
            .style(Style::default().fg(self.config().theme.error))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        
        f.render_widget(paragraph, inner);
    }
    
    /// Render "no note selected" message
    fn render_no_note_selected(&self, f: &mut Frame, area: Rect) {
        let text = "No note selected. Return to browse mode to select a note.";
        let paragraph = Paragraph::new(text)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("No Note Selected")
                .border_style(Style::default().fg(self.config().theme.border)))
            .style(Style::default().fg(self.config().theme.secondary))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        
        f.render_widget(paragraph, area);
    }
    
    /// Calculate centered rectangle
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
    
    /// Get keybindings for current mode
    fn get_keybindings_for_mode(&self) -> Vec<String> {
        match self.current_mode() {
            TUIMode::Browse => vec![
                "j/k:Nav".to_string(),
                "Enter:View".to_string(),
                "e:Edit".to_string(),
                "n:New".to_string(),
                "d:Delete".to_string(),
                "/:Search".to_string(),
                "r:Refresh".to_string(),
                "F1:Help".to_string(),
                "Ctrl+Q:Quit".to_string(),
            ],
            TUIMode::View => vec![
                "e:Edit".to_string(),
                "d:Delete".to_string(),
                "b:Back".to_string(),
                "j/k:Scroll".to_string(),
                "Esc:Browse".to_string(),
            ],
            TUIMode::Edit => vec![
                "Ctrl+S:Save".to_string(),
                "Esc:View".to_string(),
            ],
            TUIMode::Search => vec![
                "Enter:Search".to_string(),
                "Esc:Browse".to_string(),
            ],
            TUIMode::Create => vec![
                "Ctrl+S:Save".to_string(),
                "Esc:Cancel".to_string(),
            ],
            TUIMode::Settings => vec![
                "b:Back".to_string(),
                "Esc:Browse".to_string(),
            ],
            TUIMode::Help => vec![
                "Any key:Back".to_string(),
            ],
        }
    }
}