use crate::email::{EmailDatabase, MaildirImporter, ImportConfig, ImportStats, MaildirImportError};
use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Wrap,
    },
    Frame,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::mpsc;

/// Errors that can occur in the Import Wizard
#[derive(Error, Debug)]
pub enum ImportWizardError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Import error: {0}")]
    Import(#[from] MaildirImportError),
    
    #[error("UI error: {0}")]
    Ui(String),
    
    #[error("User cancelled operation")]
    UserCancelled,
}

pub type ImportWizardResult<T> = Result<T, ImportWizardError>;

/// States in the import wizard flow
#[derive(Debug, Clone, PartialEq)]
pub enum WizardStep {
    /// Directory selection step
    DirectorySelection,
    /// Folder selection step (which folders to import)
    FolderSelection,
    /// Configuration step (import settings)
    Configuration,
    /// Import progress step (showing progress)
    ImportProgress,
    /// Completion step (showing results)
    Completion,
}

/// Import wizard state
#[derive(Debug)]
pub struct ImportWizardState {
    /// Current step in the wizard
    pub step: WizardStep,
    /// Selected directory path
    pub selected_directory: Option<PathBuf>,
    /// Available directories in current path
    pub directories: Vec<DirectoryEntry>,
    /// Directory list state
    pub directory_list_state: ListState,
    /// Current directory being browsed
    pub current_directory: PathBuf,
    /// Discovered Maildir folders
    pub maildir_folders: Vec<MaildirFolderEntry>,
    /// Folder selection states
    pub folder_selection_states: Vec<bool>,
    /// Folder list state
    pub folder_list_state: ListState,
    /// Import configuration
    pub import_config: ImportConfig,
    /// Current import progress
    pub import_progress: Option<ImportProgress>,
    /// Import statistics
    pub import_stats: Option<ImportStats>,
    /// Error message (if any)
    pub error_message: Option<String>,
    /// Whether user wants to exit
    pub should_exit: bool,
    /// Scroll state for directory list
    pub directory_scroll_state: ScrollbarState,
    /// Scroll state for folder list
    pub folder_scroll_state: ScrollbarState,
}

/// Information about a directory entry
#[derive(Debug, Clone)]
pub struct DirectoryEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_maildir: bool,
    pub is_parent: bool,
    pub message_count: Option<usize>,
}

/// Information about a Maildir folder to import
#[derive(Debug, Clone)]
pub struct MaildirFolderEntry {
    pub name: String,
    pub path: PathBuf,
    pub message_count: usize,
    pub new_messages: usize,
    pub cur_messages: usize,
}

/// Import progress information
#[derive(Debug, Clone)]
pub struct ImportProgress {
    pub current_folder: String,
    pub messages_processed: usize,
    pub total_messages: usize,
    pub folders_processed: usize,
    pub total_folders: usize,
    pub start_time: Instant,
    pub estimated_completion: Option<Instant>,
}

impl ImportProgress {
    pub fn progress_percentage(&self) -> u16 {
        if self.total_messages == 0 {
            100
        } else {
            ((self.messages_processed as f64 / self.total_messages as f64) * 100.0) as u16
        }
    }
    
    pub fn elapsed_time(&self) -> Duration {
        self.start_time.elapsed()
    }
    
    pub fn estimated_remaining(&self) -> Option<Duration> {
        if self.messages_processed == 0 {
            return None;
        }
        
        let elapsed = self.elapsed_time();
        let rate = self.messages_processed as f64 / elapsed.as_secs_f64();
        let remaining_messages = self.total_messages - self.messages_processed;
        
        if rate > 0.0 {
            Some(Duration::from_secs_f64(remaining_messages as f64 / rate))
        } else {
            None
        }
    }
}

impl Default for ImportWizardState {
    fn default() -> Self {
        Self {
            step: WizardStep::DirectorySelection,
            selected_directory: None,
            directories: Vec::new(),
            directory_list_state: ListState::default(),
            current_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
            maildir_folders: Vec::new(),
            folder_selection_states: Vec::new(),
            folder_list_state: ListState::default(),
            import_config: ImportConfig::default(),
            import_progress: None,
            import_stats: None,
            error_message: None,
            should_exit: false,
            directory_scroll_state: ScrollbarState::default(),
            folder_scroll_state: ScrollbarState::default(),
        }
    }
}

/// Import wizard TUI component
pub struct ImportWizard {
    /// Database connection
    database: Arc<EmailDatabase>,
    /// Wizard state
    state: ImportWizardState,
    /// Account ID to import into
    account_id: String,
    /// Import progress receiver
    progress_receiver: Option<mpsc::UnboundedReceiver<ImportProgress>>,
    /// Import task handle
    import_handle: Option<tokio::task::JoinHandle<ImportWizardResult<ImportStats>>>,
}

impl ImportWizard {
    /// Create a new import wizard
    pub fn new(database: Arc<EmailDatabase>, account_id: String) -> Self {
        Self {
            database,
            state: ImportWizardState::default(),
            account_id,
            progress_receiver: None,
            import_handle: None,
        }
    }
    
    /// Handle keyboard input
    pub async fn handle_event(&mut self, event: Event) -> ImportWizardResult<()> {
        match event {
            Event::Key(key) => self.handle_key_event(key).await?,
            _ => {}
        }
        Ok(())
    }
    
    /// Handle key events
    async fn handle_key_event(&mut self, key: KeyEvent) -> ImportWizardResult<()> {
        // Global key handlers
        match key.code {
            KeyCode::Esc => {
                if self.state.step == WizardStep::ImportProgress {
                    // Cancel import if in progress
                    if let Some(handle) = &self.import_handle {
                        handle.abort();
                        self.import_handle = None;
                    }
                }
                self.state.should_exit = true;
                return Ok(());
            }
            KeyCode::Char('q') => {
                self.state.should_exit = true;
                return Ok(());
            }
            _ => {}
        }
        
        // Step-specific key handlers
        match self.state.step {
            WizardStep::DirectorySelection => self.handle_directory_selection_keys(key).await?,
            WizardStep::FolderSelection => self.handle_folder_selection_keys(key).await?,
            WizardStep::Configuration => self.handle_configuration_keys(key).await?,
            WizardStep::ImportProgress => self.handle_import_progress_keys(key).await?,
            WizardStep::Completion => self.handle_completion_keys(key).await?,
        }
        
        Ok(())
    }
    
    /// Handle directory selection keys
    async fn handle_directory_selection_keys(&mut self, key: KeyEvent) -> ImportWizardResult<()> {
        match key.code {
            KeyCode::Up => {
                if let Some(selected) = self.state.directory_list_state.selected() {
                    if selected > 0 {
                        self.state.directory_list_state.select(Some(selected - 1));
                    }
                }
            }
            KeyCode::Down => {
                if let Some(selected) = self.state.directory_list_state.selected() {
                    if selected < self.state.directories.len().saturating_sub(1) {
                        self.state.directory_list_state.select(Some(selected + 1));
                    }
                } else if !self.state.directories.is_empty() {
                    self.state.directory_list_state.select(Some(0));
                }
            }
            KeyCode::Enter => {
                if let Some(selected) = self.state.directory_list_state.selected() {
                    if let Some(entry) = self.state.directories.get(selected) {
                        if entry.is_parent {
                            // Navigate to parent directory
                            if let Some(parent) = self.state.current_directory.parent() {
                                self.state.current_directory = parent.to_path_buf();
                                self.refresh_directory_list().await?;
                            }
                        } else if entry.path.is_dir() {
                            if entry.is_maildir {
                                // Selected a Maildir directory, move to folder selection
                                self.state.selected_directory = Some(entry.path.clone());
                                self.scan_maildir_folders().await?;
                                self.state.step = WizardStep::FolderSelection;
                            } else {
                                // Navigate into directory
                                self.state.current_directory = entry.path.clone();
                                self.refresh_directory_list().await?;
                            }
                        }
                    }
                }
            }
            KeyCode::Char('r') => {
                // Refresh directory list
                self.refresh_directory_list().await?;
            }
            _ => {}
        }
        
        Ok(())
    }
    
    /// Handle folder selection keys
    async fn handle_folder_selection_keys(&mut self, key: KeyEvent) -> ImportWizardResult<()> {
        match key.code {
            KeyCode::Up => {
                if let Some(selected) = self.state.folder_list_state.selected() {
                    if selected > 0 {
                        self.state.folder_list_state.select(Some(selected - 1));
                    }
                }
            }
            KeyCode::Down => {
                if let Some(selected) = self.state.folder_list_state.selected() {
                    if selected < self.state.maildir_folders.len().saturating_sub(1) {
                        self.state.folder_list_state.select(Some(selected + 1));
                    }
                } else if !self.state.maildir_folders.is_empty() {
                    self.state.folder_list_state.select(Some(0));
                }
            }
            KeyCode::Char(' ') => {
                // Toggle folder selection
                if let Some(selected) = self.state.folder_list_state.selected() {
                    if selected < self.state.folder_selection_states.len() {
                        self.state.folder_selection_states[selected] = !self.state.folder_selection_states[selected];
                    }
                }
            }
            KeyCode::Char('a') => {
                // Select all folders
                for i in 0..self.state.folder_selection_states.len() {
                    self.state.folder_selection_states[i] = true;
                }
            }
            KeyCode::Char('n') => {
                // Select none
                for i in 0..self.state.folder_selection_states.len() {
                    self.state.folder_selection_states[i] = false;
                }
            }
            KeyCode::Enter => {
                // Proceed to configuration if any folders selected
                if self.state.folder_selection_states.iter().any(|&selected| selected) {
                    self.state.step = WizardStep::Configuration;
                }
            }
            KeyCode::Backspace => {
                // Go back to directory selection
                self.state.step = WizardStep::DirectorySelection;
            }
            _ => {}
        }
        
        Ok(())
    }
    
    /// Handle configuration keys
    async fn handle_configuration_keys(&mut self, key: KeyEvent) -> ImportWizardResult<()> {
        match key.code {
            KeyCode::Enter => {
                // Start import
                self.start_import().await?;
                self.state.step = WizardStep::ImportProgress;
            }
            KeyCode::Backspace => {
                // Go back to folder selection
                self.state.step = WizardStep::FolderSelection;
            }
            KeyCode::Char('d') => {
                // Toggle skip duplicates
                self.state.import_config.skip_duplicates = !self.state.import_config.skip_duplicates;
            }
            KeyCode::Char('v') => {
                // Toggle validation
                self.state.import_config.validate_format = !self.state.import_config.validate_format;
            }
            KeyCode::Char('t') => {
                // Toggle preserve timestamps
                self.state.import_config.preserve_timestamps = !self.state.import_config.preserve_timestamps;
            }
            _ => {}
        }
        
        Ok(())
    }
    
    /// Handle import progress keys
    async fn handle_import_progress_keys(&mut self, _key: KeyEvent) -> ImportWizardResult<()> {
        // Import progress is mostly passive, just check for completion
        if let Some(handle) = &mut self.import_handle {
            if handle.is_finished() {
                match handle.await {
                    Ok(Ok(stats)) => {
                        self.state.import_stats = Some(stats);
                        self.state.step = WizardStep::Completion;
                    }
                    Ok(Err(e)) => {
                        self.state.error_message = Some(format!("Import failed: {}", e));
                        self.state.step = WizardStep::Completion;
                    }
                    Err(_) => {
                        self.state.error_message = Some("Import was cancelled".to_string());
                        self.state.step = WizardStep::Completion;
                    }
                }
                self.import_handle = None;
            }
        }
        
        Ok(())
    }
    
    /// Handle completion keys
    async fn handle_completion_keys(&mut self, key: KeyEvent) -> ImportWizardResult<()> {
        match key.code {
            KeyCode::Enter | KeyCode::Char('q') => {
                self.state.should_exit = true;
            }
            KeyCode::Char('r') => {
                // Restart wizard
                self.state = ImportWizardState::default();
                self.refresh_directory_list().await?;
            }
            _ => {}
        }
        
        Ok(())
    }
    
    /// Refresh the directory listing
    async fn refresh_directory_list(&mut self) -> ImportWizardResult<()> {
        let mut directories = Vec::new();
        
        // Add parent directory entry if not at root
        if self.state.current_directory.parent().is_some() {
            directories.push(DirectoryEntry {
                name: "..".to_string(),
                path: self.state.current_directory.parent().unwrap().to_path_buf(),
                is_maildir: false,
                is_parent: true,
                message_count: None,
            });
        }
        
        // Read current directory
        let mut entries = tokio::fs::read_dir(&self.state.current_directory).await?;
        let mut dir_entries = Vec::new();
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("?")
                    .to_string();
                
                // Check if it's a Maildir directory
                let is_maildir = self.is_maildir_directory(&path).await;
                let message_count = if is_maildir {
                    self.count_maildir_messages(&path).await.ok()
                } else {
                    None
                };
                
                dir_entries.push(DirectoryEntry {
                    name,
                    path,
                    is_maildir,
                    is_parent: false,
                    message_count,
                });
            }
        }
        
        // Sort directories: Maildir first, then alphabetically
        dir_entries.sort_by(|a, b| {
            match (a.is_maildir, b.is_maildir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });
        
        directories.extend(dir_entries);
        self.state.directories = directories;
        self.state.directory_list_state.select(None);
        
        // Update scroll state
        self.state.directory_scroll_state = ScrollbarState::default()
            .content_length(self.state.directories.len());
        
        Ok(())
    }
    
    /// Check if a directory is a Maildir directory
    async fn is_maildir_directory(&self, path: &Path) -> bool {
        let new_dir = path.join("new");
        let cur_dir = path.join("cur");
        let tmp_dir = path.join("tmp");
        
        new_dir.exists() && cur_dir.exists() && tmp_dir.exists()
    }
    
    /// Count messages in a Maildir directory
    async fn count_maildir_messages(&self, path: &Path) -> ImportWizardResult<usize> {
        let new_dir = path.join("new");
        let cur_dir = path.join("cur");
        
        let mut count = 0;
        
        if new_dir.exists() {
            let mut entries = tokio::fs::read_dir(&new_dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                if entry.file_type().await?.is_file() {
                    count += 1;
                }
            }
        }
        
        if cur_dir.exists() {
            let mut entries = tokio::fs::read_dir(&cur_dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                if entry.file_type().await?.is_file() {
                    count += 1;
                }
            }
        }
        
        Ok(count)
    }
    
    /// Scan for Maildir folders in the selected directory
    async fn scan_maildir_folders(&mut self) -> ImportWizardResult<()> {
        let mut folders = Vec::new();
        
        if let Some(ref base_path) = self.state.selected_directory {
            // Use walkdir to find all Maildir folders recursively
            use walkdir::WalkDir;
            
            for entry in WalkDir::new(base_path).min_depth(1).max_depth(10) {
                let entry = entry.map_err(|e| ImportWizardError::Io(e.into()))?;
                let path = entry.path();
                
                if self.is_maildir_directory(path).await {
                    let name = path.strip_prefix(base_path)
                        .unwrap_or(path)
                        .to_string_lossy()
                        .to_string();
                    
                    let total_count = self.count_maildir_messages(path).await?;
                    let new_count = self.count_messages_in_dir(&path.join("new")).await?;
                    let cur_count = total_count - new_count;
                    
                    folders.push(MaildirFolderEntry {
                        name,
                        path: path.to_path_buf(),
                        message_count: total_count,
                        new_messages: new_count,
                        cur_messages: cur_count,
                    });
                }
            }
        }
        
        // Sort folders by name
        folders.sort_by(|a, b| a.name.cmp(&b.name));
        
        self.state.maildir_folders = folders;
        self.state.folder_selection_states = vec![true; self.state.maildir_folders.len()]; // Select all by default
        self.state.folder_list_state.select(None);
        
        // Update scroll state
        self.state.folder_scroll_state = ScrollbarState::default()
            .content_length(self.state.maildir_folders.len());
        
        Ok(())
    }
    
    /// Count messages in a specific directory
    async fn count_messages_in_dir(&self, dir_path: &Path) -> ImportWizardResult<usize> {
        if !dir_path.exists() {
            return Ok(0);
        }
        
        let mut count = 0;
        let mut entries = tokio::fs::read_dir(dir_path).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_file() {
                count += 1;
            }
        }
        
        Ok(count)
    }
    
    /// Start the import process
    async fn start_import(&mut self) -> ImportWizardResult<()> {
        if let Some(ref directory) = self.state.selected_directory {
            let selected_folders: Vec<_> = self.state.maildir_folders
                .iter()
                .zip(self.state.folder_selection_states.iter())
                .filter(|(_, &selected)| selected)
                .map(|(folder, _)| folder.clone())
                .collect();
            
            if selected_folders.is_empty() {
                return Err(ImportWizardError::Ui("No folders selected for import".to_string()));
            }
            
            // Create progress tracking
            let (progress_sender, progress_receiver) = mpsc::unbounded_channel();
            self.progress_receiver = Some(progress_receiver);
            
            // Calculate total messages
            let total_messages: usize = selected_folders.iter().map(|f| f.message_count).sum();
            
            // Set up initial progress
            self.state.import_progress = Some(ImportProgress {
                current_folder: String::new(),
                messages_processed: 0,
                total_messages,
                folders_processed: 0,
                total_folders: selected_folders.len(),
                start_time: Instant::now(),
                estimated_completion: None,
            });
            
            // Start import task
            let database = self.database.clone();
            let account_id = self.account_id.clone();
            let directory = directory.clone();
            let config = self.state.import_config.clone();
            
            let handle = tokio::spawn(async move {
                let mut importer = MaildirImporter::with_config(database, config);
                
                // Set progress callback
                importer.set_progress_callback(Box::new(move |current, total, folder| {
                    let progress = ImportProgress {
                        current_folder: folder.to_string(),
                        messages_processed: current,
                        total_messages: total,
                        folders_processed: 0, // This would need more sophisticated tracking
                        total_folders: selected_folders.len(),
                        start_time: Instant::now(), // This should be preserved from start
                        estimated_completion: None,
                    };
                    
                    let _ = progress_sender.send(progress);
                }));
                
                importer.import_from_directory(&directory, &account_id).await
                    .map_err(ImportWizardError::Import)
            });
            
            self.import_handle = Some(handle);
        }
        
        Ok(())
    }
    
    /// Update progress from receiver
    pub async fn update_progress(&mut self) {
        if let Some(ref mut receiver) = self.progress_receiver {
            while let Ok(progress) = receiver.try_recv() {
                self.state.import_progress = Some(progress);
            }
        }
    }
    
    /// Check if the wizard should exit
    pub fn should_exit(&self) -> bool {
        self.state.should_exit
    }
    
    /// Get current wizard step
    pub fn current_step(&self) -> &WizardStep {
        &self.state.step
    }
    
    /// Render the wizard
    pub fn render(&mut self, f: &mut Frame) {
        let size = f.size();
        
        // Create main layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(0),    // Content
                Constraint::Length(3), // Help
            ])
            .split(size);
        
        // Render title
        self.render_title(f, chunks[0]);
        
        // Render content based on current step
        match self.state.step {
            WizardStep::DirectorySelection => self.render_directory_selection(f, chunks[1]),
            WizardStep::FolderSelection => self.render_folder_selection(f, chunks[1]),
            WizardStep::Configuration => self.render_configuration(f, chunks[1]),
            WizardStep::ImportProgress => self.render_import_progress(f, chunks[1]),
            WizardStep::Completion => self.render_completion(f, chunks[1]),
        }
        
        // Render help
        self.render_help(f, chunks[2]);
        
        // Render error popup if there's an error
        if let Some(ref error) = self.state.error_message {
            self.render_error_popup(f, size, error);
        }
    }
    
    /// Render the title bar
    fn render_title(&self, f: &mut Frame, area: Rect) {
        let title = match self.state.step {
            WizardStep::DirectorySelection => "Maildir Import Wizard - Select Directory",
            WizardStep::FolderSelection => "Maildir Import Wizard - Select Folders",
            WizardStep::Configuration => "Maildir Import Wizard - Configuration",
            WizardStep::ImportProgress => "Maildir Import Wizard - Import Progress",
            WizardStep::Completion => "Maildir Import Wizard - Completion",
        };
        
        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .title_alignment(Alignment::Center)
            .style(Style::default().fg(Color::Yellow));
        
        let paragraph = Paragraph::new("")
            .block(block);
        
        f.render_widget(paragraph, area);
    }
    
    /// Render directory selection step
    fn render_directory_selection(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100)])
            .split(area);
        
        // Current directory info
        let info_text = format!("Current Directory: {}", self.state.current_directory.display());
        let info = Paragraph::new(info_text)
            .style(Style::default().fg(Color::Cyan))
            .wrap(Wrap { trim: true });
        
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(chunks[0]);
        
        f.render_widget(info, main_chunks[0]);
        
        // Directory list
        let items: Vec<ListItem> = self.state.directories
            .iter()
            .map(|entry| {
                let style = if entry.is_maildir {
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                } else if entry.is_parent {
                    Style::default().fg(Color::Blue)
                } else {
                    Style::default()
                };
                
                let text = if let Some(count) = entry.message_count {
                    format!("{} ({} messages)", entry.name, count)
                } else if entry.is_maildir {
                    format!("{} [Maildir]", entry.name)
                } else {
                    entry.name.clone()
                };
                
                ListItem::new(text).style(style)
            })
            .collect();
        
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Directories"))
            .highlight_style(Style::default().bg(Color::Yellow).fg(Color::Black))
            .highlight_symbol(">> ");
        
        f.render_stateful_widget(list, main_chunks[1], &mut self.state.directory_list_state);
        
        // Render scrollbar
        if self.state.directories.len() > main_chunks[1].height as usize {
            let scrollbar = Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));
            
            let scrollbar_area = main_chunks[1].inner(&Margin {
                vertical: 1,
                horizontal: 0,
            });
            
            f.render_stateful_widget(scrollbar, scrollbar_area, &mut self.state.directory_scroll_state);
        }
    }
    
    /// Render folder selection step
    fn render_folder_selection(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(3)])
            .split(area);
        
        // Summary info
        let selected_count = self.state.folder_selection_states.iter().filter(|&&x| x).count();
        let total_messages: usize = self.state.maildir_folders
            .iter()
            .zip(self.state.folder_selection_states.iter())
            .filter(|(_, &selected)| selected)
            .map(|(folder, _)| folder.message_count)
            .sum();
        
        let info_text = format!(
            "Selected: {}/{} folders ({} messages total)",
            selected_count,
            self.state.maildir_folders.len(),
            total_messages
        );
        
        let info = Paragraph::new(info_text)
            .style(Style::default().fg(Color::Cyan))
            .block(Block::default().borders(Borders::ALL).title("Selection Summary"));
        
        f.render_widget(info, chunks[0]);
        
        // Folder list
        let items: Vec<ListItem> = self.state.maildir_folders
            .iter()
            .zip(self.state.folder_selection_states.iter())
            .map(|(folder, &selected)| {
                let checkbox = if selected { "[x]" } else { "[ ]" };
                let text = format!(
                    "{} {} ({} messages: {} new, {} cur)",
                    checkbox,
                    folder.name,
                    folder.message_count,
                    folder.new_messages,
                    folder.cur_messages
                );
                
                let style = if selected {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default()
                };
                
                ListItem::new(text).style(style)
            })
            .collect();
        
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Folders (Space to toggle)"))
            .highlight_style(Style::default().bg(Color::Yellow).fg(Color::Black))
            .highlight_symbol(">> ");
        
        f.render_stateful_widget(list, chunks[1], &mut self.state.folder_list_state);
        
        // Instructions
        let instructions = Paragraph::new("Use Up/Down to navigate, Space to toggle, 'a' to select all, 'n' to select none, Enter to continue")
            .style(Style::default().fg(Color::Gray))
            .block(Block::default().borders(Borders::ALL).title("Instructions"))
            .wrap(Wrap { trim: true });
        
        f.render_widget(instructions, chunks[2]);
    }
    
    /// Render configuration step
    fn render_configuration(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(10), // Configuration options
                Constraint::Min(0),     // Summary
            ])
            .split(area);
        
        // Configuration options
        let config_text = vec![
            Line::from(vec![
                Span::raw("Skip Duplicates: "),
                Span::styled(
                    if self.state.import_config.skip_duplicates { "Yes" } else { "No" },
                    Style::default().fg(if self.state.import_config.skip_duplicates { Color::Green } else { Color::Red })
                ),
                Span::raw(" (Press 'd' to toggle)")
            ]),
            Line::from(vec![
                Span::raw("Validate Format: "),
                Span::styled(
                    if self.state.import_config.validate_format { "Yes" } else { "No" },
                    Style::default().fg(if self.state.import_config.validate_format { Color::Green } else { Color::Red })
                ),
                Span::raw(" (Press 'v' to toggle)")
            ]),
            Line::from(vec![
                Span::raw("Preserve Timestamps: "),
                Span::styled(
                    if self.state.import_config.preserve_timestamps { "Yes" } else { "No" },
                    Style::default().fg(if self.state.import_config.preserve_timestamps { Color::Green } else { Color::Red })
                ),
                Span::raw(" (Press 't' to toggle)")
            ]),
            Line::from(""),
            Line::from("Press Enter to start import, Backspace to go back"),
        ];
        
        let config_paragraph = Paragraph::new(config_text)
            .block(Block::default().borders(Borders::ALL).title("Import Configuration"))
            .wrap(Wrap { trim: true });
        
        f.render_widget(config_paragraph, chunks[0]);
        
        // Import summary
        let selected_folders: Vec<_> = self.state.maildir_folders
            .iter()
            .zip(self.state.folder_selection_states.iter())
            .filter(|(_, &selected)| selected)
            .collect();
        
        let total_messages: usize = selected_folders.iter().map(|(folder, _)| folder.message_count).sum();
        
        let summary_lines = vec![
            Line::from(format!("Directory: {}", self.state.selected_directory.as_ref().unwrap().display())),
            Line::from(format!("Folders: {}", selected_folders.len())),
            Line::from(format!("Total Messages: {}", total_messages)),
        ];
        
        let summary = Paragraph::new(summary_lines)
            .block(Block::default().borders(Borders::ALL).title("Import Summary"))
            .wrap(Wrap { trim: true });
        
        f.render_widget(summary, chunks[1]);
    }
    
    /// Render import progress step
    fn render_import_progress(&mut self, f: &mut Frame, area: Rect) {
        if let Some(ref progress) = self.state.import_progress {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Progress bar
                    Constraint::Length(7), // Details
                    Constraint::Min(0),    // Status
                ])
                .split(area);
            
            // Progress bar
            let progress_bar = Gauge::default()
                .block(Block::default().borders(Borders::ALL).title("Import Progress"))
                .gauge_style(Style::default().fg(Color::Green))
                .percent(progress.progress_percentage())
                .label(format!("{}/{}%", progress.progress_percentage(), 100));
            
            f.render_widget(progress_bar, chunks[0]);
            
            // Progress details
            let elapsed = progress.elapsed_time();
            let remaining_text = if let Some(remaining) = progress.estimated_remaining() {
                format!("{:.1}s remaining", remaining.as_secs_f64())
            } else {
                "Calculating...".to_string()
            };
            
            let details = vec![
                Line::from(format!("Current Folder: {}", progress.current_folder)),
                Line::from(format!("Messages: {}/{}", progress.messages_processed, progress.total_messages)),
                Line::from(format!("Folders: {}/{}", progress.folders_processed, progress.total_folders)),
                Line::from(format!("Elapsed: {:.1}s", elapsed.as_secs_f64())),
                Line::from(remaining_text),
            ];
            
            let details_paragraph = Paragraph::new(details)
                .block(Block::default().borders(Borders::ALL).title("Details"));
            
            f.render_widget(details_paragraph, chunks[1]);
            
            // Status message
            let status = Paragraph::new("Import in progress... Press Esc to cancel")
                .style(Style::default().fg(Color::Yellow))
                .block(Block::default().borders(Borders::ALL).title("Status"))
                .wrap(Wrap { trim: true });
            
            f.render_widget(status, chunks[2]);
        }
    }
    
    /// Render completion step
    fn render_completion(&mut self, f: &mut Frame, area: Rect) {
        if let Some(ref stats) = self.state.import_stats {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(3)])
                .split(area);
            
            // Success message and stats
            let success_text = vec![
                Line::from(Span::styled("Import Completed Successfully!", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))),
                Line::from(""),
                Line::from(format!("Directories Scanned: {}", stats.directories_scanned)),
                Line::from(format!("Maildir Folders Found: {}", stats.maildir_folders_found)),
                Line::from(format!("Messages Found: {}", stats.messages_found)),
                Line::from(format!("Messages Imported: {}", stats.messages_imported)),
                Line::from(format!("Messages Failed: {}", stats.messages_failed)),
                Line::from(format!("Duplicates Skipped: {}", stats.duplicates_skipped)),
                Line::from(format!("Success Rate: {:.1}%", stats.success_rate())),
            ];
            
            let success_paragraph = Paragraph::new(success_text)
                .block(Block::default().borders(Borders::ALL).title("Import Results"))
                .wrap(Wrap { trim: true });
            
            f.render_widget(success_paragraph, chunks[0]);
            
            // Instructions
            let instructions = Paragraph::new("Press Enter or 'q' to exit, 'r' to restart wizard")
                .style(Style::default().fg(Color::Gray))
                .block(Block::default().borders(Borders::ALL).title("Next Steps"));
            
            f.render_widget(instructions, chunks[1]);
        } else if let Some(ref error) = self.state.error_message {
            // Error message
            let error_text = vec![
                Line::from(Span::styled("Import Failed!", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))),
                Line::from(""),
                Line::from(error.clone()),
            ];
            
            let error_paragraph = Paragraph::new(error_text)
                .block(Block::default().borders(Borders::ALL).title("Error"))
                .wrap(Wrap { trim: true });
            
            f.render_widget(error_paragraph, area);
        }
    }
    
    /// Render help text
    fn render_help(&self, f: &mut Frame, area: Rect) {
        let help_text = match self.state.step {
            WizardStep::DirectorySelection => "↑/↓: Navigate  Enter: Select/Enter  r: Refresh  q/Esc: Quit",
            WizardStep::FolderSelection => "↑/↓: Navigate  Space: Toggle  a: All  n: None  Enter: Continue  Backspace: Back",
            WizardStep::Configuration => "d: Toggle Duplicates  v: Toggle Validation  t: Toggle Timestamps  Enter: Start  Backspace: Back",
            WizardStep::ImportProgress => "Esc: Cancel Import",
            WizardStep::Completion => "Enter/q: Exit  r: Restart",
        };
        
        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray))
            .block(Block::default().borders(Borders::ALL).title("Help"))
            .alignment(Alignment::Center);
        
        f.render_widget(help, area);
    }
    
    /// Render error popup
    fn render_error_popup(&self, f: &mut Frame, area: Rect, error: &str) {
        let popup_area = centered_rect(60, 20, area);
        
        f.render_widget(Clear, popup_area);
        
        let error_text = vec![
            Line::from(Span::styled("Error", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from(error),
            Line::from(""),
            Line::from("Press any key to continue..."),
        ];
        
        let error_popup = Paragraph::new(error_text)
            .block(Block::default().borders(Borders::ALL).style(Style::default().fg(Color::Red)))
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Center);
        
        f.render_widget(error_popup, popup_area);
    }
}

/// Helper function to create a centered rectangle
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    /// Create a test EmailDatabase
    async fn create_test_database() -> Arc<EmailDatabase> {
        Arc::new(EmailDatabase::new_in_memory().await.unwrap())
    }

    /// Create a mock Maildir structure for testing
    async fn create_mock_maildir(base_path: &Path) -> Result<()> {
        // Create INBOX folder
        let inbox_path = base_path.join("INBOX");
        create_maildir_folder(&inbox_path).await?;
        
        // Add sample messages to INBOX
        create_test_message(&inbox_path.join("new"), "msg1", "Test content 1").await?;
        create_test_message(&inbox_path.join("cur"), "msg2:2,S", "Test content 2").await?;
        
        // Create a subfolder
        let work_path = base_path.join("Work");
        create_maildir_folder(&work_path).await?;
        create_test_message(&work_path.join("new"), "msg3", "Work content").await?;

        Ok(())
    }

    /// Create a Maildir folder structure
    async fn create_maildir_folder(path: &Path) -> Result<()> {
        fs::create_dir_all(path.join("new")).await?;
        fs::create_dir_all(path.join("cur")).await?;
        fs::create_dir_all(path.join("tmp")).await?;
        Ok(())
    }

    /// Create a test message file
    async fn create_test_message(dir: &Path, filename: &str, content: &str) -> Result<()> {
        let file_path = dir.join(filename);
        fs::write(file_path, content).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_import_wizard_creation() {
        let database = create_test_database().await;
        let wizard = ImportWizard::new(database, "test_account".to_string());
        
        assert_eq!(wizard.current_step(), &WizardStep::DirectorySelection);
        assert!(!wizard.should_exit());
        assert_eq!(wizard.account_id, "test_account");
    }

    #[tokio::test]
    async fn test_directory_detection() {
        let temp_dir = TempDir::new().unwrap();
        let database = create_test_database().await;
        let mut wizard = ImportWizard::new(database, "test_account".to_string());
        
        // Create mock structure
        create_mock_maildir(temp_dir.path()).await.unwrap();
        
        // Set current directory and refresh
        wizard.state.current_directory = temp_dir.path().to_path_buf();
        wizard.refresh_directory_list().await.unwrap();
        
        // Should find both Maildir directories
        let maildir_count = wizard.state.directories.iter()
            .filter(|d| d.is_maildir)
            .count();
        
        assert_eq!(maildir_count, 2); // INBOX and Work
    }

    #[tokio::test]
    async fn test_maildir_folder_scanning() {
        let temp_dir = TempDir::new().unwrap();
        let database = create_test_database().await;
        let mut wizard = ImportWizard::new(database, "test_account".to_string());
        
        // Create mock structure
        create_mock_maildir(temp_dir.path()).await.unwrap();
        
        // Set selected directory and scan
        wizard.state.selected_directory = Some(temp_dir.path().to_path_buf());
        wizard.scan_maildir_folders().await.unwrap();
        
        // Should find folders
        assert_eq!(wizard.state.maildir_folders.len(), 2);
        
        // Check message counts
        let inbox_folder = wizard.state.maildir_folders.iter()
            .find(|f| f.name == "INBOX")
            .unwrap();
        assert_eq!(inbox_folder.message_count, 2);
        
        let work_folder = wizard.state.maildir_folders.iter()
            .find(|f| f.name == "Work")
            .unwrap();
        assert_eq!(work_folder.message_count, 1);
    }

    #[tokio::test]
    async fn test_wizard_state_transitions() {
        let database = create_test_database().await;
        let mut wizard = ImportWizard::new(database, "test_account".to_string());
        
        // Start in directory selection
        assert_eq!(wizard.state.step, WizardStep::DirectorySelection);
        
        // Mock selecting a directory
        wizard.state.selected_directory = Some(PathBuf::from("/test"));
        wizard.state.maildir_folders = vec![
            MaildirFolderEntry {
                name: "INBOX".to_string(),
                path: PathBuf::from("/test/INBOX"),
                message_count: 10,
                new_messages: 5,
                cur_messages: 5,
            }
        ];
        wizard.state.folder_selection_states = vec![true];
        
        // Move to folder selection
        wizard.state.step = WizardStep::FolderSelection;
        assert_eq!(wizard.state.step, WizardStep::FolderSelection);
        
        // Move to configuration
        wizard.state.step = WizardStep::Configuration;
        assert_eq!(wizard.state.step, WizardStep::Configuration);
    }

    #[tokio::test]
    async fn test_import_configuration() {
        let database = create_test_database().await;
        let mut wizard = ImportWizard::new(database, "test_account".to_string());
        
        // Test default configuration
        assert!(wizard.state.import_config.skip_duplicates);
        assert!(wizard.state.import_config.validate_format);
        assert!(wizard.state.import_config.preserve_timestamps);
        
        // Test configuration changes
        wizard.state.import_config.skip_duplicates = false;
        assert!(!wizard.state.import_config.skip_duplicates);
    }

    #[tokio::test]
    async fn test_progress_calculations() {
        let progress = ImportProgress {
            current_folder: "INBOX".to_string(),
            messages_processed: 25,
            total_messages: 100,
            folders_processed: 1,
            total_folders: 4,
            start_time: Instant::now() - Duration::from_secs(10),
            estimated_completion: None,
        };
        
        assert_eq!(progress.progress_percentage(), 25);
        assert!(progress.elapsed_time().as_secs() >= 10);
        
        let remaining = progress.estimated_remaining();
        assert!(remaining.is_some());
        assert!(remaining.unwrap().as_secs() > 0);
    }

    #[tokio::test]
    async fn test_key_event_handling() {
        let database = create_test_database().await;
        let mut wizard = ImportWizard::new(database, "test_account".to_string());
        
        // Test quit key
        let quit_event = Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        wizard.handle_event(quit_event).await.unwrap();
        assert!(wizard.should_exit());
        
        // Reset and test escape
        wizard.state.should_exit = false;
        let esc_event = Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        wizard.handle_event(esc_event).await.unwrap();
        assert!(wizard.should_exit());
    }

    #[tokio::test]
    async fn test_directory_navigation() {
        let temp_dir = TempDir::new().unwrap();
        let database = create_test_database().await;
        let mut wizard = ImportWizard::new(database, "test_account".to_string());
        
        // Create subdirectory
        let sub_dir = temp_dir.path().join("subdir");
        fs::create_dir_all(&sub_dir).await.unwrap();
        
        // Set current directory and refresh
        wizard.state.current_directory = temp_dir.path().to_path_buf();
        wizard.refresh_directory_list().await.unwrap();
        
        // Should have parent (..) and subdirectory
        assert!(wizard.state.directories.len() >= 1);
        
        // Test navigation up
        if wizard.state.current_directory.parent().is_some() {
            let parent_entry = wizard.state.directories.iter().find(|d| d.is_parent);
            assert!(parent_entry.is_some());
        }
    }

    #[tokio::test]
    async fn test_folder_selection_states() {
        let database = create_test_database().await;
        let mut wizard = ImportWizard::new(database, "test_account".to_string());
        
        // Mock some folders
        wizard.state.maildir_folders = vec![
            MaildirFolderEntry {
                name: "INBOX".to_string(),
                path: PathBuf::from("/test/INBOX"),
                message_count: 10,
                new_messages: 5,
                cur_messages: 5,
            },
            MaildirFolderEntry {
                name: "Sent".to_string(),
                path: PathBuf::from("/test/Sent"),
                message_count: 20,
                new_messages: 0,
                cur_messages: 20,
            },
        ];
        
        wizard.state.folder_selection_states = vec![true, false];
        
        // Test selection state
        assert!(wizard.state.folder_selection_states[0]);
        assert!(!wizard.state.folder_selection_states[1]);
        
        // Toggle selection
        wizard.state.folder_selection_states[1] = true;
        assert!(wizard.state.folder_selection_states[1]);
    }
}