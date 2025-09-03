//! User interface for backup management

use crate::backup::{
    BackupEngine, BackupConfig, BackupMetadata, BackupStatus, BackupType, BackupTarget,
    DataCategory,
};
use crate::theme::Theme;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, Tabs, Wrap,
    },
    Frame,
};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

/// Backup UI tabs
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BackupTab {
    Overview,
    CreateBackup,
    RestoreData,
    Schedule,
    Settings,
}

impl BackupTab {
    pub fn title(&self) -> &'static str {
        match self {
            BackupTab::Overview => "Overview",
            BackupTab::CreateBackup => "Create Backup",
            BackupTab::RestoreData => "Restore Data",
            BackupTab::Schedule => "Schedule",
            BackupTab::Settings => "Settings",
        }
    }

    pub fn all() -> Vec<BackupTab> {
        vec![
            BackupTab::Overview,
            BackupTab::CreateBackup,
            BackupTab::RestoreData,
            BackupTab::Schedule,
            BackupTab::Settings,
        ]
    }
}

/// UI actions that can be performed
#[derive(Debug, Clone, PartialEq)]
pub enum BackupAction {
    CreateBackup(BackupConfig),
    RestoreBackup(Uuid),
    DeleteBackup(Uuid),
    EditConfig(Uuid),
    StartBackup(Uuid),
    CancelBackup(Uuid),
    RefreshStatus,
    ExportConfig(Uuid),
    ImportConfig(String),
}

/// Backup UI state
pub struct BackupUIState {
    pub visible: bool,
    pub current_tab: BackupTab,
    pub selected_backup: Option<Uuid>,
    pub backup_list_state: ListState,
    pub config_editor: BackupConfigEditor,
    pub restore_list_state: ListState,
    pub progress_visible: bool,
}

/// Backup configuration editor state
pub struct BackupConfigEditor {
    pub config: BackupConfig,
    pub selected_field: ConfigField,
    pub category_states: HashSet<DataCategory>,
    pub input_buffer: String,
    pub edit_mode: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConfigField {
    Name,
    Description,
    BackupType,
    Target,
    Categories,
    Compression,
    CompressionLevel,
    Encryption,
    MaxVersions,
    ExcludePatterns,
    IncludePatterns,
}

impl Default for BackupUIState {
    fn default() -> Self {
        Self {
            visible: false,
            current_tab: BackupTab::Overview,
            selected_backup: None,
            backup_list_state: ListState::default(),
            config_editor: BackupConfigEditor::default(),
            restore_list_state: ListState::default(),
            progress_visible: false,
        }
    }
}

impl Default for BackupConfigEditor {
    fn default() -> Self {
        Self {
            config: BackupConfig::default(),
            selected_field: ConfigField::Name,
            category_states: DataCategory::all_categories().into_iter().collect(),
            input_buffer: String::new(),
            edit_mode: false,
        }
    }
}

/// Main backup UI component
pub struct BackupUI {
    state: BackupUIState,
    backup_engine: Arc<Mutex<BackupEngine>>,
    backup_configs: Vec<BackupConfig>,
    backup_metadata: Vec<BackupMetadata>,
}

impl BackupUI {
    pub fn new(backup_engine: Arc<Mutex<BackupEngine>>) -> Self {
        Self {
            state: BackupUIState::default(),
            backup_engine,
            backup_configs: Vec::new(),
            backup_metadata: Vec::new(),
        }
    }

    pub fn show(&mut self) {
        self.state.visible = true;
    }

    pub fn hide(&mut self) {
        self.state.visible = false;
    }

    pub fn is_visible(&self) -> bool {
        self.state.visible
    }

    pub fn handle_key(&mut self, key: KeyCode, modifiers: KeyModifiers) -> Option<BackupAction> {
        if !self.state.visible {
            return None;
        }

        match key {
            KeyCode::Esc => {
                if self.state.config_editor.edit_mode {
                    self.state.config_editor.edit_mode = false;
                    None
                } else {
                    self.hide();
                    None
                }
            }
            KeyCode::Tab => {
                self.next_tab();
                None
            }
            KeyCode::BackTab => {
                self.previous_tab();
                None
            }
            KeyCode::Enter => {
                self.handle_enter()
            }
            KeyCode::Char(c) if modifiers.contains(KeyModifiers::CONTROL) => {
                match c {
                    'n' => self.handle_new_backup(),
                    'r' => Some(BackupAction::RefreshStatus),
                    's' => self.handle_save_config(),
                    _ => None,
                }
            }
            KeyCode::Char(c) => {
                if self.state.config_editor.edit_mode {
                    self.state.config_editor.input_buffer.push(c);
                    None
                } else {
                    match c {
                        'n' => self.handle_new_backup(),
                        'd' => self.handle_delete_backup(),
                        'e' => self.handle_edit_backup(),
                        'r' => self.handle_restore_backup(),
                        _ => None,
                    }
                }
            }
            KeyCode::Backspace => {
                if self.state.config_editor.edit_mode {
                    self.state.config_editor.input_buffer.pop();
                }
                None
            }
            KeyCode::Up => {
                self.move_selection_up();
                None
            }
            KeyCode::Down => {
                self.move_selection_down();
                None
            }
            _ => None,
        }
    }

    fn next_tab(&mut self) {
        let tabs = BackupTab::all();
        let current_index = tabs.iter().position(|&t| t == self.state.current_tab).unwrap_or(0);
        self.state.current_tab = tabs[(current_index + 1) % tabs.len()];
    }

    fn previous_tab(&mut self) {
        let tabs = BackupTab::all();
        let current_index = tabs.iter().position(|&t| t == self.state.current_tab).unwrap_or(0);
        self.state.current_tab = tabs[(current_index + tabs.len() - 1) % tabs.len()];
    }

    fn handle_enter(&mut self) -> Option<BackupAction> {
        match self.state.current_tab {
            BackupTab::CreateBackup => {
                if self.state.config_editor.edit_mode {
                    self.apply_config_edit();
                    None
                } else {
                    Some(BackupAction::CreateBackup(self.state.config_editor.config.clone()))
                }
            }
            BackupTab::RestoreData => {
                if let Some(backup_id) = self.state.selected_backup {
                    Some(BackupAction::RestoreBackup(backup_id))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn handle_new_backup(&mut self) -> Option<BackupAction> {
        self.state.current_tab = BackupTab::CreateBackup;
        self.state.config_editor = BackupConfigEditor::default();
        None
    }

    fn handle_delete_backup(&mut self) -> Option<BackupAction> {
        if let Some(backup_id) = self.state.selected_backup {
            Some(BackupAction::DeleteBackup(backup_id))
        } else {
            None
        }
    }

    fn handle_edit_backup(&mut self) -> Option<BackupAction> {
        if let Some(backup_id) = self.state.selected_backup {
            Some(BackupAction::EditConfig(backup_id))
        } else {
            None
        }
    }

    fn handle_restore_backup(&mut self) -> Option<BackupAction> {
        if let Some(backup_id) = self.state.selected_backup {
            Some(BackupAction::RestoreBackup(backup_id))
        } else {
            None
        }
    }

    fn handle_save_config(&mut self) -> Option<BackupAction> {
        // Save current configuration
        None
    }

    fn apply_config_edit(&mut self) {
        match self.state.config_editor.selected_field {
            ConfigField::Name => {
                self.state.config_editor.config.name = self.state.config_editor.input_buffer.clone();
            }
            ConfigField::Description => {
                self.state.config_editor.config.description = self.state.config_editor.input_buffer.clone();
            }
            ConfigField::CompressionLevel => {
                if let Ok(level) = self.state.config_editor.input_buffer.parse::<u8>() {
                    if level <= 9 {
                        self.state.config_editor.config.compression_level = level;
                    }
                }
            }
            ConfigField::MaxVersions => {
                if let Ok(versions) = self.state.config_editor.input_buffer.parse::<u32>() {
                    self.state.config_editor.config.max_versions = versions;
                }
            }
            _ => {}
        }

        self.state.config_editor.edit_mode = false;
        self.state.config_editor.input_buffer.clear();
    }

    fn move_selection_up(&mut self) {
        match self.state.current_tab {
            BackupTab::Overview => {
                if let Some(selected) = self.state.backup_list_state.selected() {
                    if selected > 0 {
                        self.state.backup_list_state.select(Some(selected - 1));
                    }
                }
            }
            BackupTab::CreateBackup => {
                // Move between configuration fields
                self.previous_config_field();
            }
            _ => {}
        }
    }

    fn move_selection_down(&mut self) {
        match self.state.current_tab {
            BackupTab::Overview => {
                let selected = self.state.backup_list_state.selected().unwrap_or(0);
                if selected + 1 < self.backup_configs.len() {
                    self.state.backup_list_state.select(Some(selected + 1));
                }
            }
            BackupTab::CreateBackup => {
                // Move between configuration fields
                self.next_config_field();
            }
            _ => {}
        }
    }

    fn next_config_field(&mut self) {
        self.state.config_editor.selected_field = match self.state.config_editor.selected_field {
            ConfigField::Name => ConfigField::Description,
            ConfigField::Description => ConfigField::BackupType,
            ConfigField::BackupType => ConfigField::Target,
            ConfigField::Target => ConfigField::Categories,
            ConfigField::Categories => ConfigField::Compression,
            ConfigField::Compression => ConfigField::CompressionLevel,
            ConfigField::CompressionLevel => ConfigField::Encryption,
            ConfigField::Encryption => ConfigField::MaxVersions,
            ConfigField::MaxVersions => ConfigField::ExcludePatterns,
            ConfigField::ExcludePatterns => ConfigField::IncludePatterns,
            ConfigField::IncludePatterns => ConfigField::Name,
        };
    }

    fn previous_config_field(&mut self) {
        self.state.config_editor.selected_field = match self.state.config_editor.selected_field {
            ConfigField::Name => ConfigField::IncludePatterns,
            ConfigField::Description => ConfigField::Name,
            ConfigField::BackupType => ConfigField::Description,
            ConfigField::Target => ConfigField::BackupType,
            ConfigField::Categories => ConfigField::Target,
            ConfigField::Compression => ConfigField::Categories,
            ConfigField::CompressionLevel => ConfigField::Compression,
            ConfigField::Encryption => ConfigField::CompressionLevel,
            ConfigField::MaxVersions => ConfigField::Encryption,
            ConfigField::ExcludePatterns => ConfigField::MaxVersions,
            ConfigField::IncludePatterns => ConfigField::ExcludePatterns,
        };
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.state.visible {
            return;
        }

        // Clear the area
        frame.render_widget(Clear, area);

        // Main container
        let main_block = Block::default()
            .title("Backup & Sync Manager")
            .borders(Borders::ALL)
            .style(theme.get_component_style("primary", false));

        let inner_area = main_block.inner(area);
        frame.render_widget(main_block, area);

        // Layout: tabs at top, content below
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(inner_area);

        // Render tabs
        self.render_tabs(frame, chunks[0], theme);

        // Render content based on current tab
        match self.state.current_tab {
            BackupTab::Overview => self.render_overview(frame, chunks[1], theme),
            BackupTab::CreateBackup => self.render_create_backup(frame, chunks[1], theme),
            BackupTab::RestoreData => self.render_restore_data(frame, chunks[1], theme),
            BackupTab::Schedule => self.render_schedule(frame, chunks[1], theme),
            BackupTab::Settings => self.render_settings(frame, chunks[1], theme),
        }
    }

    fn render_tabs(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let tab_names: Vec<&str> = BackupTab::all().iter().map(|t| t.title()).collect();
        let selected_index = BackupTab::all()
            .iter()
            .position(|&t| t == self.state.current_tab)
            .unwrap_or(0);

        let tabs = Tabs::new(tab_names)
            .block(Block::default().borders(Borders::BOTTOM))
            .style(theme.get_component_style("secondary", false))
            .highlight_style(theme.get_component_style("primary", true))
            .select(selected_index);

        frame.render_widget(tabs, area);
    }

    fn render_overview(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Backup list
        self.render_backup_list(frame, chunks[0], theme);

        // Backup details
        self.render_backup_details(frame, chunks[1], theme);
    }

    fn render_backup_list(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let items: Vec<ListItem> = self.backup_metadata
            .iter()
            .map(|backup| {
                let status_icon = match &backup.status {
                    BackupStatus::Completed { .. } => "✅",
                    BackupStatus::Running { .. } => "⏳",
                    BackupStatus::Failed { .. } => "❌",
                    BackupStatus::Cancelled => "⚠️",
                    BackupStatus::Preparing => "🔄",
                };

                let line = format!("{} {} (v{})", 
                    status_icon,
                    backup.started_at.format("%Y-%m-%d %H:%M"),
                    backup.version
                );

                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default()
                .title("Recent Backups")
                .borders(Borders::ALL))
            .style(theme.get_component_style("secondary", false))
            .highlight_style(theme.get_component_style("primary", true));

        frame.render_stateful_widget(list, area, &mut self.state.backup_list_state);
    }

    fn render_backup_details(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let text = if let Some(selected_index) = self.state.backup_list_state.selected() {
            if let Some(backup) = self.backup_metadata.get(selected_index) {
                let status_text = match &backup.status {
                    BackupStatus::Completed { duration_ms, bytes_backed_up } => {
                        format!("Completed in {}ms, {} bytes backed up", duration_ms, bytes_backed_up)
                    }
                    BackupStatus::Running { progress, current_item } => {
                        format!("Running: {:.1}% - {}", progress, current_item)
                    }
                    BackupStatus::Failed { error } => {
                        format!("Failed: {}", error)
                    }
                    BackupStatus::Cancelled => "Cancelled by user".to_string(),
                    BackupStatus::Preparing => "Preparing backup...".to_string(),
                };

                format!(
                    "Backup ID: {}\nType: {:?}\nStarted: {}\nFiles: {}\nTotal Size: {} bytes\nStatus: {}",
                    backup.id,
                    backup.backup_type,
                    backup.started_at.format("%Y-%m-%d %H:%M:%S"),
                    backup.total_files,
                    backup.total_bytes,
                    status_text
                )
            } else {
                "No backup selected".to_string()
            }
        } else {
            "Select a backup to view details".to_string()
        };

        let paragraph = Paragraph::new(text)
            .block(Block::default()
                .title("Backup Details")
                .borders(Borders::ALL))
            .wrap(Wrap { trim: true })
            .style(theme.get_component_style("secondary", false));

        frame.render_widget(paragraph, area);
    }

    fn render_create_backup(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let text = "Create Backup Configuration\n\nPress 'n' to create a new backup\nPress Ctrl+S to save configuration";

        let paragraph = Paragraph::new(text)
            .block(Block::default()
                .title("Create Backup")
                .borders(Borders::ALL))
            .wrap(Wrap { trim: true })
            .style(theme.get_component_style("secondary", false));

        frame.render_widget(paragraph, area);
    }

    fn render_restore_data(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let text = "Restore Data from Backup\n\nSelect a backup and press Enter to restore";

        let paragraph = Paragraph::new(text)
            .block(Block::default()
                .title("Restore Data")
                .borders(Borders::ALL))
            .wrap(Wrap { trim: true })
            .style(theme.get_component_style("secondary", false));

        frame.render_widget(paragraph, area);
    }

    fn render_schedule(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let text = "Backup Scheduling\n\nConfigure automatic backups";

        let paragraph = Paragraph::new(text)
            .block(Block::default()
                .title("Schedule")
                .borders(Borders::ALL))
            .wrap(Wrap { trim: true })
            .style(theme.get_component_style("secondary", false));

        frame.render_widget(paragraph, area);
    }

    fn render_settings(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let text = "Backup Settings\n\nConfigure backup preferences";

        let paragraph = Paragraph::new(text)
            .block(Block::default()
                .title("Settings")
                .borders(Borders::ALL))
            .wrap(Wrap { trim: true })
            .style(theme.get_component_style("secondary", false));

        frame.render_widget(paragraph, area);
    }
}