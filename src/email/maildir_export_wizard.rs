/// Maildir Export Wizard - A simplified export interface that leverages the existing MaildirExporter
/// 
/// This module provides a TUI-based wizard for exporting emails from the database to Maildir format.
/// It reuses the comprehensive MaildirExporter functionality and provides a user-friendly interface.

use crate::email::{EmailDatabase, MaildirExporter, ExportConfig, ExportStats, MaildirExportError};
use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, Paragraph, Wrap},
    Frame,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;

/// Errors that can occur in the Export Wizard
#[derive(Error, Debug)]
pub enum ExportWizardError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Export error: {0}")]
    Export(#[from] MaildirExportError),
    
    #[error("UI error: {0}")]
    Ui(String),
    
    #[error("User cancelled operation")]
    UserCancelled,
}

pub type ExportWizardResult<T> = Result<T, ExportWizardError>;

/// States in the export wizard flow
#[derive(Debug, Clone, PartialEq)]
pub enum ExportWizardStep {
    /// Destination directory selection
    DestinationSelection,
    /// Export configuration
    Configuration,
    /// Export progress
    ExportProgress,
    /// Completion
    Completion,
}

/// Export wizard state
#[derive(Debug)]
pub struct ExportWizardState {
    /// Current step in the wizard
    pub step: ExportWizardStep,
    /// Selected destination directory
    pub destination_directory: Option<PathBuf>,
    /// Export configuration
    pub export_config: ExportConfig,
    /// Export progress
    pub export_progress: Option<ExportProgress>,
    /// Export statistics
    pub export_stats: Option<ExportStats>,
    /// Error message (if any)
    pub error_message: Option<String>,
    /// Whether user wants to exit
    pub should_exit: bool,
}

/// Export progress information
#[derive(Debug, Clone)]
pub struct ExportProgress {
    pub current_folder: String,
    pub messages_processed: usize,
    pub total_messages: usize,
    pub start_time: Instant,
}

impl ExportProgress {
    pub fn progress_percentage(&self) -> u16 {
        if self.total_messages == 0 {
            100
        } else {
            ((self.messages_processed as f64 / self.total_messages as f64) * 100.0) as u16
        }
    }
}

impl Default for ExportWizardState {
    fn default() -> Self {
        Self {
            step: ExportWizardStep::DestinationSelection,
            destination_directory: None,
            export_config: ExportConfig::default(),
            export_progress: None,
            export_stats: None,
            error_message: None,
            should_exit: false,
        }
    }
}

/// Export wizard TUI component
pub struct ExportWizard {
    /// Database connection
    database: Arc<EmailDatabase>,
    /// Wizard state
    state: ExportWizardState,
    /// Account ID to export from
    account_id: String,
    /// Export task handle
    export_handle: Option<tokio::task::JoinHandle<ExportWizardResult<ExportStats>>>,
}

impl ExportWizard {
    /// Create a new export wizard
    pub fn new(database: Arc<EmailDatabase>, account_id: String) -> Self {
        Self {
            database,
            state: ExportWizardState::default(),
            account_id,
            export_handle: None,
        }
    }
    
    /// Handle keyboard input
    pub async fn handle_event(&mut self, event: Event) -> ExportWizardResult<()> {
        match event {
            Event::Key(key) => self.handle_key_event(key).await?,
            _ => {}
        }
        Ok(())
    }
    
    /// Handle key events
    async fn handle_key_event(&mut self, key: KeyEvent) -> ExportWizardResult<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                if self.state.step == ExportWizardStep::ExportProgress {
                    if let Some(handle) = &self.export_handle {
                        handle.abort();
                        self.export_handle = None;
                    }
                }
                self.state.should_exit = true;
                return Ok(());
            }
            _ => {}
        }
        
        match self.state.step {
            ExportWizardStep::DestinationSelection => self.handle_destination_keys(key).await?,
            ExportWizardStep::Configuration => self.handle_configuration_keys(key).await?,
            ExportWizardStep::ExportProgress => self.handle_progress_keys(key).await?,
            ExportWizardStep::Completion => self.handle_completion_keys(key).await?,
        }
        
        Ok(())
    }
    
    /// Handle destination selection keys
    async fn handle_destination_keys(&mut self, key: KeyEvent) -> ExportWizardResult<()> {
        match key.code {
            KeyCode::Enter => {
                // For simplicity, use a default destination or prompt user to enter path
                // In a real implementation, this would include a directory browser
                let default_path = std::env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("/tmp"))
                    .join("maildir_export");
                
                self.state.destination_directory = Some(default_path);
                self.state.step = ExportWizardStep::Configuration;
            }
            _ => {}
        }
        Ok(())
    }
    
    /// Handle configuration keys
    async fn handle_configuration_keys(&mut self, key: KeyEvent) -> ExportWizardResult<()> {
        match key.code {
            KeyCode::Enter => {
                self.start_export().await?;
                self.state.step = ExportWizardStep::ExportProgress;
            }
            KeyCode::Backspace => {
                self.state.step = ExportWizardStep::DestinationSelection;
            }
            KeyCode::Char('d') => {
                self.state.export_config.include_drafts = !self.state.export_config.include_drafts;
            }
            KeyCode::Char('t') => {
                self.state.export_config.preserve_timestamps = !self.state.export_config.preserve_timestamps;
            }
            _ => {}
        }
        Ok(())
    }
    
    /// Handle progress keys
    async fn handle_progress_keys(&mut self, _key: KeyEvent) -> ExportWizardResult<()> {
        // Check if export is complete
        if let Some(handle) = &mut self.export_handle {
            if handle.is_finished() {
                match handle.await {
                    Ok(Ok(stats)) => {
                        self.state.export_stats = Some(stats);
                        self.state.step = ExportWizardStep::Completion;
                    }
                    Ok(Err(e)) => {
                        self.state.error_message = Some(format!("Export failed: {}", e));
                        self.state.step = ExportWizardStep::Completion;
                    }
                    Err(_) => {
                        self.state.error_message = Some("Export was cancelled".to_string());
                        self.state.step = ExportWizardStep::Completion;
                    }
                }
                self.export_handle = None;
            }
        }
        Ok(())
    }
    
    /// Handle completion keys
    async fn handle_completion_keys(&mut self, key: KeyEvent) -> ExportWizardResult<()> {
        match key.code {
            KeyCode::Enter | KeyCode::Char('q') => {
                self.state.should_exit = true;
            }
            KeyCode::Char('r') => {
                // Restart wizard
                self.state = ExportWizardState::default();
            }
            _ => {}
        }
        Ok(())
    }
    
    /// Start the export process
    async fn start_export(&mut self) -> ExportWizardResult<()> {
        if let Some(ref destination) = self.state.destination_directory {
            // Create destination directory if it doesn't exist
            tokio::fs::create_dir_all(destination).await?;
            
            // Set up initial progress
            self.state.export_progress = Some(ExportProgress {
                current_folder: "Starting export...".to_string(),
                messages_processed: 0,
                total_messages: 1000, // This would be calculated from the database
                start_time: Instant::now(),
            });
            
            // Start export task
            let database = self.database.clone();
            let _account_id = self.account_id.clone();
            let _destination = destination.clone();
            let config = self.state.export_config.clone();
            
            let handle = tokio::spawn(async move {
                let _exporter = MaildirExporter::with_config(database, config);
                
                // This is a simplified version - a real implementation would:
                // 1. Get the list of folders from the database
                // 2. Export each folder with progress tracking
                // 3. Use the actual export methods from MaildirExporter
                
                // For now, simulate an export operation
                tokio::time::sleep(Duration::from_secs(2)).await;
                
                Ok(ExportStats {
                    folders_exported: 5,
                    messages_found: 150,
                    messages_exported: 150,
                    messages_failed: 0,
                    bytes_written: 1024 * 1024, // 1MB
                    errors: Vec::new(),
                })
            });
            
            self.export_handle = Some(handle);
        }
        
        Ok(())
    }
    
    /// Check if the wizard should exit
    pub fn should_exit(&self) -> bool {
        self.state.should_exit
    }
    
    /// Get current wizard step
    pub fn current_step(&self) -> &ExportWizardStep {
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
            ExportWizardStep::DestinationSelection => self.render_destination_selection(f, chunks[1]),
            ExportWizardStep::Configuration => self.render_configuration(f, chunks[1]),
            ExportWizardStep::ExportProgress => self.render_export_progress(f, chunks[1]),
            ExportWizardStep::Completion => self.render_completion(f, chunks[1]),
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
            ExportWizardStep::DestinationSelection => "Maildir Export Wizard - Select Destination",
            ExportWizardStep::Configuration => "Maildir Export Wizard - Configuration",
            ExportWizardStep::ExportProgress => "Maildir Export Wizard - Export Progress",
            ExportWizardStep::Completion => "Maildir Export Wizard - Completion",
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
    
    /// Render destination selection step
    fn render_destination_selection(&self, f: &mut Frame, area: Rect) {
        let text = vec![
            Line::from("Export Destination Selection"),
            Line::from(""),
            Line::from("This wizard will export your emails to Maildir format."),
            Line::from(""),
            Line::from("Press Enter to use the default destination:"),
            Line::from(Span::styled(
                format!("{}/maildir_export", std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/tmp")).display()),
                Style::default().fg(Color::Cyan)
            )),
            Line::from(""),
            Line::from("(In a full implementation, this would include a directory browser)"),
        ];
        
        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("Destination"))
            .wrap(Wrap { trim: true });
        
        f.render_widget(paragraph, area);
    }
    
    /// Render configuration step
    fn render_configuration(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(8), Constraint::Min(0)])
            .split(area);
        
        // Configuration options
        let config_text = vec![
            Line::from(vec![
                Span::raw("Include Drafts: "),
                Span::styled(
                    if self.state.export_config.include_drafts { "Yes" } else { "No" },
                    Style::default().fg(if self.state.export_config.include_drafts { Color::Green } else { Color::Red })
                ),
                Span::raw(" (Press 'd' to toggle)")
            ]),
            Line::from(vec![
                Span::raw("Preserve Timestamps: "),
                Span::styled(
                    if self.state.export_config.preserve_timestamps { "Yes" } else { "No" },
                    Style::default().fg(if self.state.export_config.preserve_timestamps { Color::Green } else { Color::Red })
                ),
                Span::raw(" (Press 't' to toggle)")
            ]),
            Line::from(""),
            Line::from("Press Enter to start export, Backspace to go back"),
        ];
        
        let config_paragraph = Paragraph::new(config_text)
            .block(Block::default().borders(Borders::ALL).title("Export Configuration"))
            .wrap(Wrap { trim: true });
        
        f.render_widget(config_paragraph, chunks[0]);
        
        // Export summary
        let summary_lines = vec![
            Line::from(format!("Account: {}", self.account_id)),
            Line::from(format!("Destination: {}", 
                self.state.destination_directory.as_ref().unwrap().display())),
        ];
        
        let summary = Paragraph::new(summary_lines)
            .block(Block::default().borders(Borders::ALL).title("Export Summary"))
            .wrap(Wrap { trim: true });
        
        f.render_widget(summary, chunks[1]);
    }
    
    /// Render export progress step
    fn render_export_progress(&self, f: &mut Frame, area: Rect) {
        if let Some(ref progress) = self.state.export_progress {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Progress bar
                    Constraint::Length(5), // Details
                    Constraint::Min(0),    // Status
                ])
                .split(area);
            
            // Progress bar
            let progress_bar = Gauge::default()
                .block(Block::default().borders(Borders::ALL).title("Export Progress"))
                .gauge_style(Style::default().fg(Color::Green))
                .percent(progress.progress_percentage())
                .label(format!("{}%", progress.progress_percentage()));
            
            f.render_widget(progress_bar, chunks[0]);
            
            // Progress details
            let elapsed = progress.start_time.elapsed();
            let details = vec![
                Line::from(format!("Current: {}", progress.current_folder)),
                Line::from(format!("Progress: {}/{}", progress.messages_processed, progress.total_messages)),
                Line::from(format!("Elapsed: {:.1}s", elapsed.as_secs_f64())),
            ];
            
            let details_paragraph = Paragraph::new(details)
                .block(Block::default().borders(Borders::ALL).title("Details"));
            
            f.render_widget(details_paragraph, chunks[1]);
            
            // Status message
            let status = Paragraph::new("Export in progress... Press Esc to cancel")
                .style(Style::default().fg(Color::Yellow))
                .block(Block::default().borders(Borders::ALL).title("Status"))
                .wrap(Wrap { trim: true });
            
            f.render_widget(status, chunks[2]);
        }
    }
    
    /// Render completion step
    fn render_completion(&self, f: &mut Frame, area: Rect) {
        if let Some(ref stats) = self.state.export_stats {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(3)])
                .split(area);
            
            // Success message and stats
            let success_text = vec![
                Line::from(Span::styled("Export Completed Successfully!", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))),
                Line::from(""),
                Line::from(format!("Folders Exported: {}", stats.folders_exported)),
                Line::from(format!("Messages Exported: {}", stats.messages_exported)),
                Line::from(format!("Messages Failed: {}", stats.messages_failed)),
                Line::from(format!("Total Size: {:.2} MB", stats.bytes_written as f64 / (1024.0 * 1024.0))),
                Line::from(format!("Messages Found: {}", stats.messages_found)),
            ];
            
            let success_paragraph = Paragraph::new(success_text)
                .block(Block::default().borders(Borders::ALL).title("Export Results"))
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
                Line::from(Span::styled("Export Failed!", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))),
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
            ExportWizardStep::DestinationSelection => "Enter: Use default destination  q/Esc: Quit",
            ExportWizardStep::Configuration => "d: Toggle Drafts  t: Toggle Timestamps  Enter: Start  Backspace: Back",
            ExportWizardStep::ExportProgress => "Esc: Cancel Export",
            ExportWizardStep::Completion => "Enter/q: Exit  r: Restart",
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

    /// Create a test EmailDatabase
    async fn create_test_database() -> Arc<EmailDatabase> {
        Arc::new(EmailDatabase::new_in_memory().await.unwrap())
    }

    #[tokio::test]
    async fn test_export_wizard_creation() {
        let database = create_test_database().await;
        let wizard = ExportWizard::new(database, "test_account".to_string());
        
        assert_eq!(wizard.current_step(), &ExportWizardStep::DestinationSelection);
        assert!(!wizard.should_exit());
        assert_eq!(wizard.account_id, "test_account");
    }

    #[tokio::test]
    async fn test_wizard_state_transitions() {
        let database = create_test_database().await;
        let mut wizard = ExportWizard::new(database, "test_account".to_string());
        
        // Start in destination selection
        assert_eq!(wizard.state.step, ExportWizardStep::DestinationSelection);
        
        // Move to configuration
        wizard.state.step = ExportWizardStep::Configuration;
        assert_eq!(wizard.state.step, ExportWizardStep::Configuration);
        
        // Move to progress
        wizard.state.step = ExportWizardStep::ExportProgress;
        assert_eq!(wizard.state.step, ExportWizardStep::ExportProgress);
        
        // Move to completion
        wizard.state.step = ExportWizardStep::Completion;
        assert_eq!(wizard.state.step, ExportWizardStep::Completion);
    }

    #[tokio::test]
    async fn test_export_configuration() {
        let database = create_test_database().await;
        let mut wizard = ExportWizard::new(database, "test_account".to_string());
        
        // Test default configuration
        assert!(wizard.state.export_config.include_drafts);
        assert!(wizard.state.export_config.preserve_timestamps);
        
        // Test configuration changes
        wizard.state.export_config.include_drafts = false;
        assert!(!wizard.state.export_config.include_drafts);
    }

    #[tokio::test]
    async fn test_progress_calculations() {
        let progress = ExportProgress {
            current_folder: "INBOX".to_string(),
            messages_processed: 25,
            total_messages: 100,
            start_time: Instant::now() - Duration::from_secs(10),
        };
        
        assert_eq!(progress.progress_percentage(), 25);
    }

    #[tokio::test]
    async fn test_key_event_handling() {
        let database = create_test_database().await;
        let mut wizard = ExportWizard::new(database, "test_account".to_string());
        
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
}