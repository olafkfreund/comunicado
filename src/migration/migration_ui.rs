//! Migration UI for managing email client migrations
//!
//! This module provides a comprehensive user interface for:
//! - Detecting email clients
//! - Configuring migration settings
//! - Monitoring migration progress
//! - Handling errors and conflicts

use crate::migration::{
    MigrationEngine, MigrationConfig, MigrationPlan, MigrationProgress, MigrationDataType,
    EmailClient, MigrationStrategy, ValidationResults, ConflictInfo, ConflictResolution,
    ThunderbirdMigrator, OutlookMigrator, AppleMailMigrator,
};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, 
        Tabs, Wrap, Table, Row, Cell,
    },
    Frame,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

/// Migration UI state
pub struct MigrationUI {
    engine: Arc<MigrationEngine>,
    
    // UI navigation
    current_tab: MigrationTab,
    focused_area: FocusedArea,
    
    // Client detection
    detected_clients: Vec<DetectedClientInfo>,
    selected_client_index: Option<usize>,
    client_list_state: ListState,
    
    // Migration configuration
    current_config: MigrationConfig,
    config_form: ConfigForm,
    
    // Migration planning
    current_plan: Option<MigrationPlan>,
    validation_results: Option<ValidationResults>,
    conflicts: Vec<ConflictInfo>,
    
    // Progress monitoring
    active_migrations: HashMap<Uuid, MigrationProgress>,
    selected_migration: Option<Uuid>,
    
    // History
    migration_history: Vec<MigrationHistoryEntry>,
    
    // Status and messages
    status_message: String,
    error_message: Option<String>,
    show_help: bool,
}

/// Main migration tabs
#[derive(Debug, Clone, PartialEq)]
pub enum MigrationTab {
    ClientDetection,
    Configuration,
    Planning,
    Progress,
    History,
}

/// Focus areas within the UI
#[derive(Debug, Clone, PartialEq)]
pub enum FocusedArea {
    TabBar,
    ClientList,
    ConfigForm,
    PlanDetails,
    ProgressMonitor,
    HistoryList,
    ConflictResolution,
}

/// Actions that can be performed in the UI
#[derive(Debug, Clone)]
pub enum MigrationAction {
    DetectClients,
    SelectClient(usize),
    ConfigureSettings,
    CreatePlan,
    ValidatePlan,
    ResolveConflict(usize, ConflictResolution),
    StartMigration,
    PauseMigration(Uuid),
    ResumeMigration(Uuid),
    CancelMigration(Uuid),
    ViewDetails(Uuid),
    ExportHistory,
    ImportConfig,
    ShowHelp,
    Back,
    Quit,
}

/// Detected client information
#[derive(Debug, Clone)]
pub struct DetectedClientInfo {
    pub client: EmailClient,
    pub version: Option<String>,
    pub profile_path: PathBuf,
    pub data_size: Option<u64>,
    pub message_count: Option<usize>,
    pub contact_count: Option<usize>,
    pub last_modified: Option<chrono::DateTime<chrono::Utc>>,
    pub accessibility: ClientAccessibility,
}

/// Client accessibility status
#[derive(Debug, Clone, PartialEq)]
pub enum ClientAccessibility {
    FullAccess,
    PartialAccess(Vec<String>), // Missing data types
    NoAccess(String),           // Error message
}

/// Configuration form state
#[derive(Debug, Clone)]
pub struct ConfigForm {
    pub source_path: String,
    pub target_path: String,
    pub selected_data_types: Vec<bool>, // Index matches MigrationDataType order
    pub migration_strategy: usize,      // Index into strategy options
    pub preserve_structure: bool,
    pub create_backup: bool,
    pub validate_data: bool,
    pub overwrite_existing: bool,
    pub batch_size: String,
    pub focused_field: ConfigField,
}

/// Configuration form fields
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigField {
    SourcePath,
    TargetPath,
    DataTypes,
    Strategy,
    PreserveStructure,
    CreateBackup,
    ValidateData,
    OverwriteExisting,
    BatchSize,
}

/// Migration history entry
#[derive(Debug, Clone)]
pub struct MigrationHistoryEntry {
    pub id: Uuid,
    pub source_client: EmailClient,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub status: crate::migration::MigrationStatus,
    pub items_migrated: usize,
    pub errors_count: usize,
    pub config_name: String,
}

impl MigrationUI {
    /// Create a new migration UI
    pub fn new(engine: Arc<MigrationEngine>) -> Self {
        Self {
            engine,
            current_tab: MigrationTab::ClientDetection,
            focused_area: FocusedArea::ClientList,
            detected_clients: Vec::new(),
            selected_client_index: None,
            client_list_state: ListState::default(),
            current_config: MigrationConfig::default(),
            config_form: ConfigForm::new(),
            current_plan: None,
            validation_results: None,
            conflicts: Vec::new(),
            active_migrations: HashMap::new(),
            selected_migration: None,
            migration_history: Vec::new(),
            status_message: "Ready to migrate".to_string(),
            error_message: None,
            show_help: false,
        }
    }

    /// Handle keyboard input
    pub async fn handle_key(&mut self, key: KeyCode, modifiers: KeyModifiers) -> (bool, Option<MigrationAction>) {
        // Global shortcuts
        match key {
            KeyCode::Tab => {
                self.next_tab();
                return (true, None);
            }
            KeyCode::BackTab => {
                self.previous_tab();
                return (true, None);
            }
            KeyCode::F(1) => return (true, Some(MigrationAction::ShowHelp)),
            KeyCode::F(2) => return (true, Some(MigrationAction::DetectClients)),
            KeyCode::F(5) => return (true, Some(MigrationAction::StartMigration)),
            KeyCode::Char('q') if modifiers.contains(KeyModifiers::CONTROL) => {
                return (false, Some(MigrationAction::Quit));
            }
            KeyCode::Esc => {
                if self.show_help {
                    self.show_help = false;
                    return (true, None);
                } else if self.error_message.is_some() {
                    self.error_message = None;
                    return (true, None);
                } else {
                    return (true, Some(MigrationAction::Back));
                }
            }
            _ => {}
        }

        // Tab-specific shortcuts
        match self.current_tab {
            MigrationTab::ClientDetection => self.handle_client_detection_key(key, modifiers).await,
            MigrationTab::Configuration => self.handle_configuration_key(key, modifiers).await,
            MigrationTab::Planning => self.handle_planning_key(key, modifiers).await,
            MigrationTab::Progress => self.handle_progress_key(key, modifiers).await,
            MigrationTab::History => self.handle_history_key(key, modifiers).await,
        }
    }

    /// Render the UI
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        if self.show_help {
            self.render_help_overlay(frame, area);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Tab bar
                Constraint::Min(0),    // Content area
                Constraint::Length(3), // Status bar
            ])
            .split(area);

        // Render tab bar
        self.render_tab_bar(frame, chunks[0]);

        // Render content based on current tab
        match self.current_tab {
            MigrationTab::ClientDetection => self.render_client_detection(frame, chunks[1]),
            MigrationTab::Configuration => self.render_configuration(frame, chunks[1]),
            MigrationTab::Planning => self.render_planning(frame, chunks[1]),
            MigrationTab::Progress => self.render_progress(frame, chunks[1]),
            MigrationTab::History => self.render_history(frame, chunks[1]),
        }

        // Render status bar
        self.render_status_bar(frame, chunks[2]);

        // Render error overlay if needed
        if self.error_message.is_some() {
            self.render_error_overlay(frame, area);
        }
    }

    /// Render tab bar
    fn render_tab_bar(&self, frame: &mut Frame, area: Rect) {
        let tab_titles = vec![
            "Client Detection",
            "Configuration",
            "Planning",
            "Progress",
            "History",
        ];

        let selected_tab = match self.current_tab {
            MigrationTab::ClientDetection => 0,
            MigrationTab::Configuration => 1,
            MigrationTab::Planning => 2,
            MigrationTab::Progress => 3,
            MigrationTab::History => 4,
        };

        let tabs = Tabs::new(tab_titles)
            .block(Block::default().borders(Borders::ALL).title("Email Client Migration"))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .select(selected_tab);

        frame.render_widget(tabs, area);
    }

    /// Render client detection tab
    fn render_client_detection(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50), // Client list
                Constraint::Percentage(50), // Client details
            ])
            .split(area);

        // Client list
        let client_items: Vec<ListItem> = self.detected_clients
            .iter()
            .enumerate()
            .map(|(i, client_info)| {
                let accessibility_icon = match client_info.accessibility {
                    ClientAccessibility::FullAccess => "✓",
                    ClientAccessibility::PartialAccess(_) => "⚠",
                    ClientAccessibility::NoAccess(_) => "✗",
                };

                let content = format!("{} {:?} - {}", 
                    accessibility_icon,
                    client_info.client,
                    client_info.version.as_deref().unwrap_or("Unknown version")
                );
                
                let style = if Some(i) == self.selected_client_index {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    match client_info.accessibility {
                        ClientAccessibility::FullAccess => Style::default().fg(Color::Green),
                        ClientAccessibility::PartialAccess(_) => Style::default().fg(Color::Yellow),
                        ClientAccessibility::NoAccess(_) => Style::default().fg(Color::Red),
                    }
                };

                ListItem::new(content).style(style)
            })
            .collect();

        let client_list = List::new(client_items)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Detected Email Clients")
                .border_style(if self.focused_area == FocusedArea::ClientList {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                }))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol("→ ");

        frame.render_stateful_widget(client_list, chunks[0], &mut self.client_list_state);

        // Client details
        if let Some(index) = self.selected_client_index {
            if let Some(client_info) = self.detected_clients.get(index) {
                self.render_client_details(frame, chunks[1], client_info);
            }
        } else {
            let help_text = vec![
                Line::from("Keyboard Shortcuts:"),
                Line::from(""),
                Line::from("F2  - Detect email clients"),
                Line::from("↑/↓ - Navigate clients"),
                Line::from("Enter - Select client"),
                Line::from("Tab - Next tab"),
                Line::from("F1  - Show help"),
            ];

            let help = Paragraph::new(help_text)
                .block(Block::default().borders(Borders::ALL).title("Help"))
                .wrap(Wrap { trim: true });

            frame.render_widget(help, chunks[1]);
        }
    }

    /// Render client details panel
    fn render_client_details(&self, frame: &mut Frame, area: Rect, client_info: &DetectedClientInfo) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8),  // Basic info
                Constraint::Min(0),     // Details
            ])
            .split(area);

        // Basic client info
        let info_lines = vec![
            Line::from(vec![
                Span::styled("Client: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{:?}", client_info.client)),
            ]),
            Line::from(vec![
                Span::styled("Version: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(client_info.version.as_deref().unwrap_or("Unknown")),
            ]),
            Line::from(vec![
                Span::styled("Profile Path: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(client_info.profile_path.to_string_lossy()),
            ]),
            Line::from(vec![
                Span::styled("Messages: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(client_info.message_count.map_or("Unknown".to_string(), |c| c.to_string())),
            ]),
            Line::from(vec![
                Span::styled("Contacts: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(client_info.contact_count.map_or("Unknown".to_string(), |c| c.to_string())),
            ]),
            Line::from(vec![
                Span::styled("Data Size: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(client_info.data_size.map_or("Unknown".to_string(), |s| {
                    format!("{:.2} MB", s as f64 / (1024.0 * 1024.0))
                })),
            ]),
        ];

        let info_paragraph = Paragraph::new(info_lines)
            .block(Block::default().borders(Borders::ALL).title("Client Information"));
        frame.render_widget(info_paragraph, chunks[0]);

        // Accessibility details
        let accessibility_text = match &client_info.accessibility {
            ClientAccessibility::FullAccess => {
                vec![
                    Line::from(Span::styled("✓ Full Access", Style::default().fg(Color::Green))),
                    Line::from("All data types can be migrated."),
                ]
            }
            ClientAccessibility::PartialAccess(missing) => {
                let mut lines = vec![
                    Line::from(Span::styled("⚠ Partial Access", Style::default().fg(Color::Yellow))),
                    Line::from("Some data types cannot be accessed:"),
                ];
                for item in missing {
                    lines.push(Line::from(format!("  • {}", item)));
                }
                lines
            }
            ClientAccessibility::NoAccess(reason) => {
                vec![
                    Line::from(Span::styled("✗ No Access", Style::default().fg(Color::Red))),
                    Line::from(format!("Error: {}", reason)),
                ]
            }
        };

        let accessibility_paragraph = Paragraph::new(accessibility_text)
            .block(Block::default().borders(Borders::ALL).title("Accessibility Status"))
            .wrap(Wrap { trim: true });
        frame.render_widget(accessibility_paragraph, chunks[1]);
    }

    /// Render configuration tab
    fn render_configuration(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(10), // Source and target
                Constraint::Length(8),  // Data types
                Constraint::Length(6),  // Options
                Constraint::Min(0),     // Strategy and advanced
            ])
            .split(area);

        self.render_paths_config(frame, chunks[0]);
        self.render_data_types_config(frame, chunks[1]);
        self.render_options_config(frame, chunks[2]);
        self.render_strategy_config(frame, chunks[3]);
    }

    /// Render paths configuration
    fn render_paths_config(&self, frame: &mut Frame, area: Rect) {
        let paths_text = vec![
            Line::from(vec![
                Span::styled("Source Path: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&self.config_form.source_path),
            ]),
            Line::from(vec![
                Span::styled("Target Path: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&self.config_form.target_path),
            ]),
        ];

        let paths_paragraph = Paragraph::new(paths_text)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Migration Paths")
                .border_style(if matches!(self.focused_area, FocusedArea::ConfigForm) {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                }));

        frame.render_widget(paths_paragraph, area);
    }

    /// Render data types configuration
    fn render_data_types_config(&self, frame: &mut Frame, area: Rect) {
        let data_types = [
            "Emails", "Contacts", "Calendar", "Filters", 
            "Settings", "Accounts", "Signatures", "Templates"
        ];

        let mut type_lines = vec![Line::from("Select data types to migrate:")];
        for (i, &data_type) in data_types.iter().enumerate() {
            let selected = self.config_form.selected_data_types.get(i).unwrap_or(&false);
            let checkbox = if *selected { "☑" } else { "☐" };
            type_lines.push(Line::from(format!("  {} {}", checkbox, data_type)));
        }

        let types_paragraph = Paragraph::new(type_lines)
            .block(Block::default().borders(Borders::ALL).title("Data Types"));

        frame.render_widget(types_paragraph, area);
    }

    /// Render options configuration
    fn render_options_config(&self, frame: &mut Frame, area: Rect) {
        let options_text = vec![
            Line::from(format!("☐ Preserve folder structure: {}", 
                if self.config_form.preserve_structure { "Yes" } else { "No" })),
            Line::from(format!("☐ Create backup: {}", 
                if self.config_form.create_backup { "Yes" } else { "No" })),
            Line::from(format!("☐ Validate data: {}", 
                if self.config_form.validate_data { "Yes" } else { "No" })),
            Line::from(format!("☐ Overwrite existing: {}", 
                if self.config_form.overwrite_existing { "Yes" } else { "No" })),
        ];

        let options_paragraph = Paragraph::new(options_text)
            .block(Block::default().borders(Borders::ALL).title("Migration Options"));

        frame.render_widget(options_paragraph, area);
    }

    /// Render strategy configuration
    fn render_strategy_config(&self, frame: &mut Frame, area: Rect) {
        let strategies = ["Complete", "Selective", "Incremental", "Merge", "Replace"];
        let current_strategy = strategies.get(self.config_form.migration_strategy)
            .unwrap_or(&"Complete");

        let strategy_text = vec![
            Line::from(vec![
                Span::styled("Migration Strategy: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(*current_strategy),
            ]),
            Line::from(vec![
                Span::styled("Batch Size: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&self.config_form.batch_size),
            ]),
        ];

        let strategy_paragraph = Paragraph::new(strategy_text)
            .block(Block::default().borders(Borders::ALL).title("Advanced Settings"));

        frame.render_widget(strategy_paragraph, area);
    }

    /// Render planning tab
    fn render_planning(&self, frame: &mut Frame, area: Rect) {
        if let Some(plan) = &self.current_plan {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(6),  // Plan summary
                    Constraint::Length(8),  // Tasks
                    Constraint::Min(0),     // Validation/Conflicts
                ])
                .split(area);

            self.render_plan_summary(frame, chunks[0], plan);
            self.render_plan_tasks(frame, chunks[1], plan);
            self.render_plan_validation(frame, chunks[2]);
        } else {
            let no_plan_text = vec![
                Line::from("No migration plan created yet."),
                Line::from(""),
                Line::from("Complete the configuration and press F3 to create a plan."),
            ];

            let no_plan = Paragraph::new(no_plan_text)
                .block(Block::default().borders(Borders::ALL).title("Migration Plan"))
                .alignment(Alignment::Center);

            frame.render_widget(no_plan, area);
        }
    }

    /// Render plan summary
    fn render_plan_summary(&self, frame: &mut Frame, area: Rect, plan: &MigrationPlan) {
        let summary_text = vec![
            Line::from(format!("Source: {:?}", plan.source.client)),
            Line::from(format!("Estimated Items: {}", plan.total_estimated_items)),
            Line::from(format!("Estimated Size: {:.2} MB", 
                plan.total_estimated_size as f64 / (1024.0 * 1024.0))),
            Line::from(format!("Tasks: {}", plan.tasks.len())),
            Line::from(format!("Conflicts: {}", plan.conflicts.len())),
        ];

        let summary = Paragraph::new(summary_text)
            .block(Block::default().borders(Borders::ALL).title("Plan Summary"));

        frame.render_widget(summary, area);
    }

    /// Render plan tasks
    fn render_plan_tasks(&self, frame: &mut Frame, area: Rect, plan: &MigrationPlan) {
        let task_items: Vec<ListItem> = plan.tasks
            .iter()
            .map(|task| {
                let status_icon = match task.status {
                    crate::migration::MigrationStatus::Pending => "⏳",
                    crate::migration::MigrationStatus::Running => "🔄",
                    crate::migration::MigrationStatus::Completed => "✅",
                    crate::migration::MigrationStatus::Failed => "❌",
                    crate::migration::MigrationStatus::Cancelled => "🚫",
                    crate::migration::MigrationStatus::Paused => "⏸️",
                };

                ListItem::new(format!("{} {} ({:?})", 
                    status_icon, task.name, task.data_type))
            })
            .collect();

        let task_list = List::new(task_items)
            .block(Block::default().borders(Borders::ALL).title("Migration Tasks"));

        frame.render_widget(task_list, area);
    }

    /// Render plan validation
    fn render_plan_validation(&self, frame: &mut Frame, area: Rect) {
        if let Some(validation) = &self.validation_results {
            let validation_text = if validation.is_valid {
                vec![
                    Line::from(Span::styled("✓ Plan is valid", Style::default().fg(Color::Green))),
                    Line::from(format!("Checks performed: {}", validation.checks_performed.len())),
                    Line::from(format!("Issues found: {}", validation.issues_found.len())),
                    Line::from(format!("Recommendations: {}", validation.recommendations.len())),
                ]
            } else {
                vec![
                    Line::from(Span::styled("✗ Plan has critical issues", Style::default().fg(Color::Red))),
                    Line::from("Please resolve issues before proceeding."),
                ]
            };

            let validation_paragraph = Paragraph::new(validation_text)
                .block(Block::default().borders(Borders::ALL).title("Validation Results"));

            frame.render_widget(validation_paragraph, area);
        } else {
            let not_validated_text = vec![
                Line::from("Plan not validated yet."),
                Line::from("Press F4 to validate the migration plan."),
            ];

            let not_validated = Paragraph::new(not_validated_text)
                .block(Block::default().borders(Borders::ALL).title("Validation"))
                .alignment(Alignment::Center);

            frame.render_widget(not_validated, area);
        }
    }

    /// Render progress tab
    fn render_progress(&self, frame: &mut Frame, area: Rect) {
        if self.active_migrations.is_empty() {
            let no_migrations_text = vec![
                Line::from("No active migrations."),
                Line::from(""),
                Line::from("Start a migration from the Planning tab."),
            ];

            let no_migrations = Paragraph::new(no_migrations_text)
                .block(Block::default().borders(Borders::ALL).title("Migration Progress"))
                .alignment(Alignment::Center);

            frame.render_widget(no_migrations, area);
        } else {
            // TODO: Render active migration progress
            let progress_text = vec![
                Line::from(format!("Active migrations: {}", self.active_migrations.len())),
            ];

            let progress = Paragraph::new(progress_text)
                .block(Block::default().borders(Borders::ALL).title("Migration Progress"));

            frame.render_widget(progress, area);
        }
    }

    /// Render history tab
    fn render_history(&mut self, frame: &mut Frame, area: Rect) {
        if self.migration_history.is_empty() {
            let no_history_text = vec![
                Line::from("No migration history."),
                Line::from(""),
                Line::from("Completed migrations will appear here."),
            ];

            let no_history = Paragraph::new(no_history_text)
                .block(Block::default().borders(Borders::ALL).title("Migration History"))
                .alignment(Alignment::Center);

            frame.render_widget(no_history, area);
        } else {
            // TODO: Render migration history list
        }
    }

    /// Render status bar
    fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        let status_text = match self.current_tab {
            MigrationTab::ClientDetection => "F2: Detect Clients | Enter: Select | ↑/↓: Navigate",
            MigrationTab::Configuration => "Tab: Navigate fields | Enter: Edit | Space: Toggle",
            MigrationTab::Planning => "F3: Create Plan | F4: Validate | F5: Start Migration",
            MigrationTab::Progress => "F6: Pause | F7: Resume | F8: Cancel",
            MigrationTab::History => "Enter: View Details | Del: Remove from history",
        };

        let status = Paragraph::new(status_text)
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Center);

        frame.render_widget(status, area);
    }

    /// Render help overlay
    fn render_help_overlay(&self, frame: &mut Frame, area: Rect) {
        let popup_area = centered_rect(80, 60, area);

        // Clear the area
        frame.render_widget(Clear, popup_area);

        let help_text = vec![
            Line::from(Span::styled("Email Client Migration Help", 
                Style::default().add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from("Global Shortcuts:"),
            Line::from("  F1  - Show this help"),
            Line::from("  F2  - Detect email clients"),
            Line::from("  Tab - Next tab"),
            Line::from("  Esc - Close help/Go back"),
            Line::from("  Ctrl+Q - Quit"),
            Line::from(""),
            Line::from("Navigation:"),
            Line::from("  ↑/↓ - Navigate lists"),
            Line::from("  Enter - Select/Confirm"),
            Line::from("  Space - Toggle options"),
            Line::from(""),
            Line::from("Migration Process:"),
            Line::from("  1. Detect email clients"),
            Line::from("  2. Configure migration settings"),
            Line::from("  3. Create and validate plan"),
            Line::from("  4. Start migration"),
            Line::from("  5. Monitor progress"),
            Line::from(""),
            Line::from("Press Esc to close this help."),
        ];

        let help = Paragraph::new(help_text)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Help")
                .title_alignment(Alignment::Center))
            .wrap(Wrap { trim: true });

        frame.render_widget(help, popup_area);
    }

    /// Render error overlay
    fn render_error_overlay(&self, frame: &mut Frame, area: Rect) {
        if let Some(error_msg) = &self.error_message {
            let popup_area = centered_rect(60, 20, area);

            frame.render_widget(Clear, popup_area);

            let error_text = vec![
                Line::from(Span::styled("Error", 
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))),
                Line::from(""),
                Line::from(error_msg.as_str()),
                Line::from(""),
                Line::from("Press Esc to dismiss."),
            ];

            let error = Paragraph::new(error_text)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Red)))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });

            frame.render_widget(error, popup_area);
        }
    }

    /// Navigation methods
    fn next_tab(&mut self) {
        self.current_tab = match self.current_tab {
            MigrationTab::ClientDetection => MigrationTab::Configuration,
            MigrationTab::Configuration => MigrationTab::Planning,
            MigrationTab::Planning => MigrationTab::Progress,
            MigrationTab::Progress => MigrationTab::History,
            MigrationTab::History => MigrationTab::ClientDetection,
        };
        self.update_focus_for_tab();
    }

    fn previous_tab(&mut self) {
        self.current_tab = match self.current_tab {
            MigrationTab::ClientDetection => MigrationTab::History,
            MigrationTab::Configuration => MigrationTab::ClientDetection,
            MigrationTab::Planning => MigrationTab::Configuration,
            MigrationTab::Progress => MigrationTab::Planning,
            MigrationTab::History => MigrationTab::Progress,
        };
        self.update_focus_for_tab();
    }

    fn update_focus_for_tab(&mut self) {
        self.focused_area = match self.current_tab {
            MigrationTab::ClientDetection => FocusedArea::ClientList,
            MigrationTab::Configuration => FocusedArea::ConfigForm,
            MigrationTab::Planning => FocusedArea::PlanDetails,
            MigrationTab::Progress => FocusedArea::ProgressMonitor,
            MigrationTab::History => FocusedArea::HistoryList,
        };
    }

    /// Key handling for different tabs
    async fn handle_client_detection_key(&mut self, key: KeyCode, _modifiers: KeyModifiers) -> (bool, Option<MigrationAction>) {
        match key {
            KeyCode::Up => {
                if let Some(index) = self.selected_client_index {
                    if index > 0 {
                        self.selected_client_index = Some(index - 1);
                        self.client_list_state.select(Some(index - 1));
                    }
                } else if !self.detected_clients.is_empty() {
                    self.selected_client_index = Some(0);
                    self.client_list_state.select(Some(0));
                }
                (true, None)
            }
            KeyCode::Down => {
                if let Some(index) = self.selected_client_index {
                    if index < self.detected_clients.len() - 1 {
                        self.selected_client_index = Some(index + 1);
                        self.client_list_state.select(Some(index + 1));
                    }
                } else if !self.detected_clients.is_empty() {
                    self.selected_client_index = Some(0);
                    self.client_list_state.select(Some(0));
                }
                (true, None)
            }
            KeyCode::Enter => {
                if let Some(index) = self.selected_client_index {
                    (true, Some(MigrationAction::SelectClient(index)))
                } else {
                    (true, None)
                }
            }
            _ => (true, None),
        }
    }

    async fn handle_configuration_key(&mut self, _key: KeyCode, _modifiers: KeyModifiers) -> (bool, Option<MigrationAction>) {
        // TODO: Implement configuration key handling
        (true, None)
    }

    async fn handle_planning_key(&mut self, _key: KeyCode, _modifiers: KeyModifiers) -> (bool, Option<MigrationAction>) {
        // TODO: Implement planning key handling
        (true, None)
    }

    async fn handle_progress_key(&mut self, _key: KeyCode, _modifiers: KeyModifiers) -> (bool, Option<MigrationAction>) {
        // TODO: Implement progress key handling
        (true, None)
    }

    async fn handle_history_key(&mut self, _key: KeyCode, _modifiers: KeyModifiers) -> (bool, Option<MigrationAction>) {
        // TODO: Implement history key handling
        (true, None)
    }
}

impl ConfigForm {
    fn new() -> Self {
        Self {
            source_path: String::new(),
            target_path: String::new(),
            selected_data_types: vec![true; 8], // Default to all selected
            migration_strategy: 0, // Complete
            preserve_structure: true,
            create_backup: true,
            validate_data: true,
            overwrite_existing: false,
            batch_size: "100".to_string(),
            focused_field: ConfigField::SourcePath,
        }
    }
}

/// Create a centered rectangle
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
    use std::sync::Arc;

    #[test]
    fn test_config_form_creation() {
        let form = ConfigForm::new();
        assert!(form.preserve_structure);
        assert!(form.create_backup);
        assert_eq!(form.batch_size, "100");
    }

    #[test]
    fn test_tab_navigation() {
        let engine = Arc::new(MigrationEngine::new(
            Arc::new(crate::email::EmailDatabase::new("test").await.unwrap()),
            Arc::new(crate::contacts::ContactsDatabase::new("test").await.unwrap()),
        ).await.unwrap());
        
        let mut ui = MigrationUI::new(engine);
        
        assert_eq!(ui.current_tab, MigrationTab::ClientDetection);
        
        ui.next_tab();
        assert_eq!(ui.current_tab, MigrationTab::Configuration);
        
        ui.previous_tab();
        assert_eq!(ui.current_tab, MigrationTab::ClientDetection);
    }
}