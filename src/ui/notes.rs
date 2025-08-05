//! Notes UI integration for the main TUI application
//! 
//! This module provides the bridge between the main TUI application and the
//! Notes plugin TUI implementation, handling state synchronization and 
//! message passing between the main TEA architecture and the Notes system.

use crate::plugins::notes::{NoteStorage, NoteConversionService};
use crate::plugins::notes::types::Note;
use crate::plugins::notes::tui::NoteTUI;
use crate::plugins::notes::advanced_search::AdvancedSearchEngine; 
use crate::tea::model::NotesState;

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use std::sync::Arc;
use std::path::Path;
use tokio::sync::mpsc;

/// Notes UI renderer for the main TUI application
pub struct NotesUI {
    /// Notes TUI instance
    note_tui: Option<NoteTUI>,
    /// Notes storage reference
    storage: Option<Arc<NoteStorage>>,
    /// Conversion service for email/event to note
    conversion_service: Option<NoteConversionService>,
    /// Message sender for TEA communication
    message_sender: Option<mpsc::UnboundedSender<crate::tea::Message>>,
}

impl NotesUI {
    /// Create a new notes UI instance
    pub fn new() -> Self {
        Self {
            note_tui: None,
            storage: None,
            conversion_service: None,
            message_sender: None,
        }
    }
    
    /// Initialize the notes UI with a notes directory path
    pub async fn initialize(&mut self, notes_directory: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Initialize storage
        let storage = Arc::new(NoteStorage::new(Path::new(notes_directory)).await?);
        
        // Initialize advanced search engine
        let search_engine = Arc::new(AdvancedSearchEngine::new(storage.clone()));
        
        // Initialize conversion service
        let conversion_service = NoteConversionService::new(storage.clone());
        
        // Initialize Notes TUI
        let note_tui = NoteTUI::new(storage.clone(), search_engine).await?;
        
        self.storage = Some(storage);
        self.conversion_service = Some(conversion_service);
        self.note_tui = Some(note_tui);
        
        Ok(())
    }
    
    /// Set the message sender for TEA communication
    pub fn set_message_sender(&mut self, sender: mpsc::UnboundedSender<crate::tea::Message>) {
        self.message_sender = Some(sender);
    }
    
    /// Render the notes view
    pub fn render(&mut self, f: &mut Frame, area: Rect, state: &NotesState) {
        // Sync state from main application to Notes TUI first
        self.sync_state_to_tui(state);
        
        if let Some(ref mut note_tui) = self.note_tui {
            // Use the Notes TUI render implementation
            note_tui.render(f);
        } else {
            // Render fallback if Notes TUI is not initialized
            self.render_not_initialized(f, area);
        }
    }
    
    /// Render not initialized message
    fn render_not_initialized(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Notes - Not Initialized");
        
        let inner = block.inner(area);
        f.render_widget(block, area);
        
        let paragraph = Paragraph::new(
            "Notes plugin is not initialized.\n\n\
            Please check your configuration and ensure the notes directory is accessible."
        )
        .wrap(Wrap { trim: true })
        .style(Style::default().add_modifier(Modifier::ITALIC));
        
        f.render_widget(paragraph, inner);
    }
    
    /// Sync state from main application to Notes TUI
    fn sync_state_to_tui(&mut self, state: &NotesState) {
        // For now, this is a placeholder for state synchronization
        // The Notes TUI maintains its own state internally
        // Future implementation could sync selected notes, search queries, etc.
        
        // TODO: Implement state synchronization when Notes TUI provides
        // methods for external state updates
        let _ = state; // Suppress unused parameter warning
    }
    
    /// Handle keyboard input for notes view
    pub async fn handle_input(&mut self, key_event: crossterm::event::KeyEvent) -> bool {
        if let Some(ref mut note_tui) = self.note_tui {
            // Use Notes TUI input handling
            match note_tui.handle_key(key_event).await {
                Ok(handled) => {
                    // TODO: Check if TUI generated any actions that need to be communicated
                    // to the main application (like switching views, creating notes, etc.)
                    
                    handled
                }
                Err(e) => {
                    // Log error and return false to indicate not handled
                    tracing::error!("Notes TUI input error: {}", e);
                    false
                }
            }
        } else {
            false
        }
    }
    
    /// Get conversion service for email/event to note conversion
    pub fn conversion_service(&self) -> Option<&NoteConversionService> {
        self.conversion_service.as_ref()
    }
    
    /// Convert email to note
    pub async fn convert_email_to_note(&self, email: &crate::email::EmailMessage) -> Result<Note, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(ref conversion_service) = self.conversion_service {
            // Get default directory
            let directory_id = conversion_service.get_default_directory().await?;
            
            // Convert email to note
            let note = conversion_service.convert_email_to_note(email, directory_id).await?;
            
            Ok(note)
        } else {
            Err("Conversion service not initialized".into())
        }
    }
    
    /// Convert calendar event to note
    pub async fn convert_event_to_note(&self, event: &crate::calendar::Event) -> Result<Note, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(ref conversion_service) = self.conversion_service {
            // Get default directory
            let directory_id = conversion_service.get_default_directory().await?;
            
            // Convert event to note
            let note = conversion_service.convert_event_to_note(event, directory_id).await?;
            
            Ok(note)
        } else {
            Err("Conversion service not initialized".into())
        }
    }
    
    /// Convert KDE Connect message to note
    pub async fn convert_kde_message_to_note(&self, title: &str, content: &str) -> Result<Note, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(ref conversion_service) = self.conversion_service {
            // Get default directory
            let directory_id = conversion_service.get_default_directory().await?;
            
            // Convert KDE Connect message to note
            let note = conversion_service.convert_kde_message_to_note(title, content, directory_id).await?;
            
            Ok(note)
        } else {
            Err("Conversion service not initialized".into())
        }
    }
    
    /// Check if notes UI is initialized
    pub fn is_initialized(&self) -> bool {
        self.note_tui.is_some() && self.storage.is_some() && self.conversion_service.is_some()
    }
    
    /// Get current notes count
    pub fn notes_count(&self) -> usize {
        // TODO: Get count from Notes TUI when interface is available
        // For now, return 0 as placeholder
        0
    }
    
    /// Load notes from storage
    pub async fn load_notes(&mut self) -> Result<Vec<Note>, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(ref storage) = self.storage {
            // Use get_recent_notes to load notes (similar to get_all_notes)
            let notes = storage.get_recent_notes(1000).await?;
            
            // TODO: Update TUI with loaded notes when interface is available
            
            Ok(notes)
        } else {
            Err("Storage not initialized".into())
        }
    }
    
    /// Search notes
    pub async fn search_notes(&mut self, query: &str) -> Result<Vec<crate::plugins::notes::types::NoteSearchResult>, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(ref storage) = self.storage {
            let results = storage.search_notes(query, 100).await?;
            
            // TODO: Update TUI with search results when interface is available
            
            Ok(results)
        } else {
            Err("Storage not initialized".into())
        }
    }
}

impl Default for NotesUI {
    fn default() -> Self {
        Self::new()
    }
}

/// Notes tab information for the main UI
pub struct NotesTab {
    /// Number of notes
    pub count: usize,
    /// Current mode indicator
    pub mode: String,
    /// Has unread/new notes
    pub has_new: bool,
}

impl NotesTab {
    /// Create notes tab info from state
    pub fn from_state(state: &NotesState) -> Self {
        Self {
            count: state.notes.len(),
            mode: format!("{:?}", state.tui_mode),
            has_new: false, // TODO: Implement new notes tracking
        }
    }
    
    /// Get tab title with count
    pub fn title(&self) -> String {
        if self.count > 0 {
            format!("📝 Notes ({})", self.count)
        } else {
            "📝 Notes".to_string()
        }
    }
    
    /// Get tab style based on state
    pub fn style(&self) -> Style {
        if self.has_new {
            Style::default().add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        }
    }
}