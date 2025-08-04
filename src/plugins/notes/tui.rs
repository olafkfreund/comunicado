//! Terminal User Interface for the Notes Plugin
//! 
//! Provides a comprehensive TUI for browsing, creating, editing, and managing notes
//! with vim-style keyboard navigation and modern terminal interface design.

use super::types::{Note, NoteId, WatchedDirectory};
use super::storage::NoteStorage;
use super::manager::NoteResult;
use super::advanced_search::{AdvancedSearchEngine, AdvancedSearchOptions};

use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    style::Color,
    widgets::{Block, Borders, ListState, ScrollbarState},
};
use tui_textarea::{TextArea, Input};

/// TUI application state
#[derive(Debug)]
pub struct NoteTUI {
    /// Note storage backend
    storage: Arc<NoteStorage>,
    /// Advanced search engine
    #[allow(dead_code)]
    search_engine: Arc<AdvancedSearchEngine>,
    /// Current application mode
    mode: TUIMode,
    /// Currently selected note list
    notes: Vec<Note>,
    /// List state for note browsing
    list_state: ListState,
    /// Currently selected/viewed note
    current_note: Option<Note>,
    /// Text editor for note content
    editor: TextArea<'static>,
    /// Search query input
    search_input: TextArea<'static>,
    /// Current search query
    search_query: String,
    /// Search results
    search_results: Vec<Note>,
    /// Watched directories
    directories: Vec<WatchedDirectory>,
    /// Directory list state
    #[allow(dead_code)]
    directory_state: ListState,
    /// Status message
    status_message: Option<String>,
    /// Error message
    error_message: Option<String>,
    /// Popup state
    popup: Option<PopupState>,
    /// Scroll states for various components
    #[allow(dead_code)]
    scroll_states: ScrollStates,
    /// Configuration
    config: TUIConfig,
    /// Statistics cache
    #[allow(dead_code)]
    stats_cache: Arc<RwLock<Option<TUIStats>>>,
}

/// TUI application modes
#[derive(Debug, Clone, PartialEq)]
pub enum TUIMode {
    /// Browse notes in list view
    Browse,
    /// View a specific note
    View,
    /// Edit note content
    Edit,
    /// Search for notes
    Search,
    /// Create new note
    Create,
    /// Settings and configuration
    Settings,
    /// Help screen
    Help,
}

/// Popup dialog states
#[derive(Debug, Clone)]
pub enum PopupState {
    /// Confirm deletion
    ConfirmDelete { note_id: NoteId },
    /// Create new note dialog
    CreateNote { title_input: TextArea<'static> },
    /// Search options dialog
    SearchOptions { options: AdvancedSearchOptions },
    /// Note information dialog
    NoteInfo { note: Note },
    /// Error dialog
    Error { message: String },
    /// Help dialog
    Help,
}

/// Scroll states for UI components
#[derive(Debug, Default)]
pub struct ScrollStates {
    pub note_list: ScrollbarState,
    pub note_content: ScrollbarState,
    pub search_results: ScrollbarState,
}

/// TUI configuration
#[derive(Debug, Clone)]
pub struct TUIConfig {
    /// Show line numbers in editor
    pub show_line_numbers: bool,
    /// Vim-style keybindings
    pub vim_keybindings: bool,
    /// Auto-save interval (seconds)
    pub auto_save_interval: u64,
    /// Preview pane width ratio
    pub preview_ratio: u16,
    /// Theme colors
    pub theme: TUITheme,
    /// Maximum notes to display in list
    pub max_notes_display: usize,
}

/// TUI color theme
#[derive(Debug, Clone)]
pub struct TUITheme {
    pub primary: Color,
    pub secondary: Color,
    pub accent: Color,
    pub background: Color,
    pub text: Color,
    pub border: Color,
    pub highlight: Color,
    pub error: Color,
    pub success: Color,
}

/// TUI statistics
#[derive(Debug, Clone)]
pub struct TUIStats {
    pub total_notes: usize,
    pub notes_today: usize,
    pub search_count: usize,
    pub last_activity: DateTime<Utc>,
    pub storage_size: u64,
}

impl Default for TUIConfig {
    fn default() -> Self {
        Self {
            show_line_numbers: true,
            vim_keybindings: true,
            auto_save_interval: 30,
            preview_ratio: 50,
            theme: TUITheme::default(),
            max_notes_display: 1000,
        }
    }
}

impl Default for TUITheme {
    fn default() -> Self {
        Self {
            primary: Color::Blue,
            secondary: Color::Cyan,
            accent: Color::Yellow,
            background: Color::Black,
            text: Color::White,
            border: Color::Gray,
            highlight: Color::Green,
            error: Color::Red,
            success: Color::Green,
        }
    }
}

impl NoteTUI {
    /// Create a new TUI instance
    pub async fn new(storage: Arc<NoteStorage>, search_engine: Arc<AdvancedSearchEngine>) -> NoteResult<Self> {
        let notes = storage.get_recent_notes(100).await?;
        let directories = storage.get_watched_directories().await?;
        
        let mut list_state = ListState::default();
        if !notes.is_empty() {
            list_state.select(Some(0));
        }
        
        let mut directory_state = ListState::default();
        if !directories.is_empty() {
            directory_state.select(Some(0));
        }
        
        let mut editor = TextArea::default();
        editor.set_block(Block::default().borders(Borders::ALL).title("Editor"));
        
        let mut search_input = TextArea::default();
        search_input.set_block(Block::default().borders(Borders::ALL).title("Search"));
        
        Ok(Self {
            storage,
            search_engine,
            mode: TUIMode::Browse,
            notes,
            list_state,
            current_note: None,
            editor,
            search_input,
            search_query: String::new(),
            search_results: Vec::new(),
            directories,
            directory_state,
            status_message: Some("Welcome to Comunicado Notes!".to_string()),
            error_message: None,
            popup: None,
            scroll_states: ScrollStates::default(),
            config: TUIConfig::default(),
            stats_cache: Arc::new(RwLock::new(None)),
        })
    }
    
    /// Handle keyboard input
    pub async fn handle_key(&mut self, key: KeyEvent) -> NoteResult<bool> {
        // Handle global keybindings first
        if self.handle_global_keys(key).await? {
            return Ok(false);
        }
        
        // Handle popup-specific keys
        if self.popup.is_some() {
            return self.handle_popup_keys(key).await;
        }
        
        // Handle mode-specific keys
        match self.mode {
            TUIMode::Browse => self.handle_browse_keys(key).await,
            TUIMode::View => self.handle_view_keys(key).await,
            TUIMode::Edit => self.handle_edit_keys(key).await,
            TUIMode::Search => self.handle_search_keys(key).await,
            TUIMode::Create => self.handle_create_keys(key).await,
            TUIMode::Settings => self.handle_settings_keys(key).await,
            TUIMode::Help => self.handle_help_keys(key).await,
        }
    }
    
    /// Handle global keybindings
    async fn handle_global_keys(&mut self, key: KeyEvent) -> NoteResult<bool> {
        match key.code {
            // Quit application
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(true);
            }
            // Help
            KeyCode::F(1) | KeyCode::Char('?') => {
                self.mode = TUIMode::Help;
                return Ok(false);
            }
            // Escape - return to browse mode
            KeyCode::Esc => {
                self.popup = None;
                self.mode = TUIMode::Browse;
                self.clear_messages();
                return Ok(false);
            }
            _ => {}
        }
        Ok(false)
    }
    
    /// Handle browse mode keys
    async fn handle_browse_keys(&mut self, key: KeyEvent) -> NoteResult<bool> {
        match key.code {
            // Navigation
            KeyCode::Char('j') | KeyCode::Down => {
                self.next_note();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.previous_note();
            }
            KeyCode::Char('g') => {
                self.first_note();
            }
            KeyCode::Char('G') => {
                self.last_note();
            }
            // Actions
            KeyCode::Enter | KeyCode::Char('o') => {
                self.view_selected_note().await?;
            }
            KeyCode::Char('e') => {
                self.edit_selected_note().await?;
            }
            KeyCode::Char('n') => {
                self.mode = TUIMode::Create;
                self.setup_create_mode().await?;
            }
            KeyCode::Char('d') => {
                self.delete_selected_note().await?;
            }
            KeyCode::Char('/') => {
                self.mode = TUIMode::Search;
                self.search_input.delete_line_by_head();
            }
            KeyCode::Char('r') => {
                self.refresh_notes().await?;
            }
            KeyCode::Char('i') => {
                self.show_note_info().await?;
            }
            _ => {}
        }
        Ok(false)
    }
    
    /// Handle view mode keys
    async fn handle_view_keys(&mut self, key: KeyEvent) -> NoteResult<bool> {
        match key.code {
            KeyCode::Char('e') => {
                self.edit_current_note().await?;
            }
            KeyCode::Char('d') => {
                self.delete_current_note().await?;
            }
            KeyCode::Char('b') | KeyCode::Backspace => {
                self.mode = TUIMode::Browse;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.scroll_content_down();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.scroll_content_up();
            }
            _ => {}
        }
        Ok(false)
    }
    
    /// Handle edit mode keys
    async fn handle_edit_keys(&mut self, key: KeyEvent) -> NoteResult<bool> {
        match key.code {
            KeyCode::Esc => {
                self.save_current_note().await?;
                self.mode = TUIMode::View;
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.save_current_note().await?;
                self.set_status("Note saved successfully");
            }
            _ => {
                // Pass key to text editor
                self.editor.input(Input::from(key));
            }
        }
        Ok(false)
    }
    
    /// Handle search mode keys
    async fn handle_search_keys(&mut self, key: KeyEvent) -> NoteResult<bool> {
        match key.code {
            KeyCode::Enter => {
                self.perform_search().await?;
            }
            KeyCode::Esc => {
                self.mode = TUIMode::Browse;
                self.search_input.delete_line_by_head();
            }
            _ => {
                self.search_input.input(Input::from(key));
            }
        }
        Ok(false)
    }
    
    /// Handle create mode keys
    async fn handle_create_keys(&mut self, key: KeyEvent) -> NoteResult<bool> {
        match key.code {
            KeyCode::Esc => {
                self.mode = TUIMode::Browse;
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.save_new_note().await?;
            }
            _ => {
                self.editor.input(Input::from(key));
            }
        }
        Ok(false)
    }
    
    /// Handle settings mode keys
    async fn handle_settings_keys(&mut self, key: KeyEvent) -> NoteResult<bool> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('b') => {
                self.mode = TUIMode::Browse;
            }
            _ => {}
        }
        Ok(false)
    }
    
    /// Handle help mode keys
    async fn handle_help_keys(&mut self, key: KeyEvent) -> NoteResult<bool> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('b') => {
                self.mode = TUIMode::Browse;
            }
            _ => {}
        }
        Ok(false)
    }
    
    /// Handle popup-specific keys
    async fn handle_popup_keys(&mut self, key: KeyEvent) -> NoteResult<bool> {
        if let Some(popup) = self.popup.clone() {
            match popup {
            PopupState::ConfirmDelete { note_id } => {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        let note_id = note_id.clone();
                        self.storage.delete_note(&note_id).await?;
                        self.popup = None;
                        self.refresh_notes().await?;
                        self.set_status("Note deleted successfully");
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        self.popup = None;
                    }
                    _ => {}
                }
            }
            PopupState::CreateNote { mut title_input } => {
                match key.code {
                    KeyCode::Enter => {
                        let title = title_input.lines()[0].clone();
                        self.create_note_with_title(title).await?;
                        self.popup = None;
                    }
                    KeyCode::Esc => {
                        self.popup = None;
                    }
                    _ => {
                        title_input.input(Input::from(key));
                        self.popup = Some(PopupState::CreateNote { title_input });
                    }
                }
            }
            PopupState::Error { .. } | PopupState::Help | PopupState::NoteInfo { .. } => {
                match key.code {
                    KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => {
                        self.popup = None;
                    }
                    _ => {}
                }
            }
            _ => {
                if matches!(key.code, KeyCode::Esc) {
                    self.popup = None;
                }
            }
            }
        }
        Ok(false)
    }
    
    /// Navigation helpers
    fn next_note(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.notes.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }
    
    fn previous_note(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.notes.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }
    
    fn first_note(&mut self) {
        self.list_state.select(Some(0));
    }
    
    fn last_note(&mut self) {
        if !self.notes.is_empty() {
            self.list_state.select(Some(self.notes.len() - 1));
        }
    }
    
    /// Action helpers
    async fn view_selected_note(&mut self) -> NoteResult<()> {
        if let Some(i) = self.list_state.selected() {
            if let Some(note) = self.notes.get(i) {
                self.current_note = Some(note.clone());
                self.editor.delete_line_by_head();
                for line in note.content.lines() {
                    self.editor.insert_str(line);
                    self.editor.insert_newline();
                }
                self.mode = TUIMode::View;
            }
        }
        Ok(())
    }
    
    async fn edit_selected_note(&mut self) -> NoteResult<()> {
        self.view_selected_note().await?;
        self.mode = TUIMode::Edit;
        Ok(())
    }
    
    async fn edit_current_note(&mut self) -> NoteResult<()> {
        self.mode = TUIMode::Edit;
        Ok(())
    }
    
    async fn delete_selected_note(&mut self) -> NoteResult<()> {
        if let Some(i) = self.list_state.selected() {
            if let Some(note) = self.notes.get(i) {
                self.popup = Some(PopupState::ConfirmDelete {
                    note_id: note.id.clone(),
                });
            }
        }
        Ok(())
    }
    
    async fn delete_current_note(&mut self) -> NoteResult<()> {
        if let Some(ref note) = self.current_note {
            self.popup = Some(PopupState::ConfirmDelete {
                note_id: note.id.clone(),
            });
        }
        Ok(())
    }
    
    async fn setup_create_mode(&mut self) -> NoteResult<()> {
        self.editor.delete_line_by_head();
        self.editor.insert_str("# New Note\n\n");
        Ok(())
    }
    
    async fn save_current_note(&mut self) -> NoteResult<()> {
        if let Some(ref mut note) = self.current_note {
            let content = self.editor.lines().join("\n");
            note.content = content;
            note.modified_at = Utc::now();
            
            // Get first directory for saving (should be configurable)
            if let Some(dir) = self.directories.first() {
                self.storage.store_note(note, dir.id).await?;
                self.set_status("Note saved successfully");
            } else {
                self.set_error("No watched directories configured");
            }
        }
        Ok(())
    }
    
    async fn save_new_note(&mut self) -> NoteResult<()> {
        let content = self.editor.lines().join("\n");
        let lines: Vec<&str> = content.lines().collect();
        
        // Extract title from first line or use default
        let title = if let Some(first_line) = lines.first() {
            first_line.trim_start_matches('#').trim().to_string()
        } else {
            "Untitled Note".to_string()
        };
        
        if title.is_empty() {
            self.set_error("Note title cannot be empty");
            return Ok(());
        }
        
        // Create note with unique ID
        let note_id = format!("note-{}", uuid::Uuid::new_v4());
        let note = Note::new(
            note_id,
            title,
            content,
            std::path::PathBuf::from(format!("{}.md", uuid::Uuid::new_v4())),
        );
        
        // Save to first available directory
        if let Some(dir) = self.directories.first() {
            self.storage.store_note(&note, dir.id).await?;
            self.refresh_notes().await?;
            self.mode = TUIMode::Browse;
            self.set_status("Note created successfully");
        } else {
            self.set_error("No watched directories configured");
        }
        
        Ok(())
    }
    
    async fn create_note_with_title(&mut self, title: String) -> NoteResult<()> {
        if title.trim().is_empty() {
            self.set_error("Note title cannot be empty");
            return Ok(());
        }
        
        let note_id = format!("note-{}", uuid::Uuid::new_v4());
        let content = format!("# {}\n\n", title);
        let note = Note::new(
            note_id,
            title,
            content,
            std::path::PathBuf::from(format!("{}.md", uuid::Uuid::new_v4())),
        );
        
        if let Some(dir) = self.directories.first() {
            self.storage.store_note(&note, dir.id).await?;
            self.refresh_notes().await?;
            self.set_status("Note created successfully");
        } else {
            self.set_error("No watched directories configured");
        }
        
        Ok(())
    }
    
    async fn perform_search(&mut self) -> NoteResult<()> {
        let query = self.search_input.lines()[0].clone();
        if query.trim().is_empty() {
            return Ok(());
        }
        
        self.search_query = query.clone();
        self.search_results = self.storage.search_notes(&query, 50).await?
            .into_iter()
            .map(|result| result.note)
            .collect();
        
        self.notes = self.search_results.clone();
        self.list_state.select(if self.notes.is_empty() { None } else { Some(0) });
        self.mode = TUIMode::Browse;
        
        self.set_status(&format!("Found {} notes for '{}'", self.notes.len(), query));
        Ok(())
    }
    
    async fn refresh_notes(&mut self) -> NoteResult<()> {
        self.notes = self.storage.get_recent_notes(self.config.max_notes_display).await?;
        self.list_state.select(if self.notes.is_empty() { None } else { Some(0) });
        self.set_status("Notes refreshed");
        Ok(())
    }
    
    async fn show_note_info(&mut self) -> NoteResult<()> {
        if let Some(i) = self.list_state.selected() {
            if let Some(note) = self.notes.get(i) {
                self.popup = Some(PopupState::NoteInfo { note: note.clone() });
            }
        }
        Ok(())
    }
    
    /// Scroll content helpers
    fn scroll_content_down(&mut self) {
        // Implement content scrolling logic
    }
    
    fn scroll_content_up(&mut self) {
        // Implement content scrolling logic
    }
    
    /// Message helpers
    fn set_status(&mut self, message: &str) {
        self.status_message = Some(message.to_string());
        self.error_message = None;
    }
    
    fn set_error(&mut self, message: &str) {
        self.error_message = Some(message.to_string());
        self.status_message = None;
    }
    
    fn clear_messages(&mut self) {
        self.status_message = None;
        self.error_message = None;
    }
    
    /// Get current mode for display
    pub fn current_mode(&self) -> &TUIMode {
        &self.mode
    }
    
    /// Get current notes for display
    pub fn current_notes(&self) -> &[Note] {
        &self.notes
    }
    
    /// Get list state for note browsing
    pub fn list_state(&mut self) -> &mut ListState {
        &mut self.list_state
    }
    
    /// Get current note for display
    pub fn current_note(&self) -> Option<&Note> {
        self.current_note.as_ref()
    }
    
    /// Get editor widget
    pub fn editor(&self) -> &TextArea {
        &self.editor
    }
    
    /// Get search input widget  
    pub fn search_input(&self) -> &TextArea {
        &self.search_input
    }
    
    /// Get current popup state
    pub fn popup(&self) -> Option<&PopupState> {
        self.popup.as_ref()
    }
    
    /// Get status message
    pub fn status_message(&self) -> Option<&str> {
        self.status_message.as_deref()
    }
    
    /// Get error message
    pub fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }
    
    /// Get configuration
    pub fn config(&self) -> &TUIConfig {
        &self.config
    }
    
    /// Update configuration
    pub fn update_config(&mut self, config: TUIConfig) {
        self.config = config;
    }
    
    /// Get search results
    pub fn search_results(&self) -> &[Note] {
        &self.search_results
    }
    
    /// Get current search query
    pub fn search_query(&self) -> &str {
        &self.search_query
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    async fn create_test_tui() -> NoteTUI {
        let storage = Arc::new(NoteStorage::new_in_memory().await.unwrap());
        let search_engine = Arc::new(AdvancedSearchEngine::new(storage.clone()));
        
        NoteTUI::new(storage, search_engine).await.unwrap()
    }
    
    #[tokio::test]
    async fn test_tui_creation() {
        let tui = create_test_tui().await;
        assert_eq!(tui.mode, TUIMode::Browse);
        assert!(tui.notes.is_empty());
        println!("✓ TUI creation works correctly");
    }
    
    #[tokio::test]
    async fn test_mode_switching() {
        let mut tui = create_test_tui().await;
        
        // Test help mode
        tui.handle_key(KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE)).await.unwrap();
        assert_eq!(tui.mode, TUIMode::Help);
        
        // Test escape to browse
        tui.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)).await.unwrap();
        assert_eq!(tui.mode, TUIMode::Browse);
        
        // Test search mode
        tui.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE)).await.unwrap();
        assert_eq!(tui.mode, TUIMode::Search);
        
        println!("✓ Mode switching works correctly");
    }
    
    #[tokio::test]
    async fn test_navigation() {
        let mut tui = create_test_tui().await;
        
        // Add some test notes first  
        let dir = super::super::types::WatchedDirectory::new(
            std::path::PathBuf::from("/test"),
            "Test".to_string(),
        );
        let stored_dir = tui.storage.add_watched_directory(dir).await.unwrap();
        
        // Create test notes
        for i in 0..5 {
            let note = Note::new(
                format!("test-{}", i),
                format!("Test Note {}", i),
                format!("Content {}", i),
                std::path::PathBuf::from(format!("/test/test-{}.md", i)),
            );
            tui.storage.store_note(&note, stored_dir.id).await.unwrap();
        }
        
        // Refresh notes
        tui.refresh_notes().await.unwrap();
        assert_eq!(tui.notes.len(), 5);
        
        // Test navigation
        assert_eq!(tui.list_state.selected(), Some(0));
        
        tui.next_note();
        assert_eq!(tui.list_state.selected(), Some(1));
        
        tui.previous_note();
        assert_eq!(tui.list_state.selected(), Some(0));
        
        tui.last_note();
        assert_eq!(tui.list_state.selected(), Some(4));
        
        tui.first_note();
        assert_eq!(tui.list_state.selected(), Some(0));
        
        println!("✓ Navigation works correctly");
    }
    
    #[tokio::test]
    async fn test_config_defaults() {
        let config = TUIConfig::default();
        assert!(config.show_line_numbers);
        assert!(config.vim_keybindings);
        assert_eq!(config.auto_save_interval, 30);
        assert_eq!(config.preview_ratio, 50);
        assert_eq!(config.max_notes_display, 1000);
        
        let theme = TUITheme::default();
        assert_eq!(theme.primary, Color::Blue);
        assert_eq!(theme.error, Color::Red);
        assert_eq!(theme.success, Color::Green);
        
        println!("✓ Configuration defaults work correctly");
    }
}