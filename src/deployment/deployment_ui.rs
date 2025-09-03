//! Comprehensive deployment management user interface
//!
//! This module provides a complete TUI interface for managing all deployment aspects:
//! - Package building and distribution
//! - Container image management  
//! - Distribution-specific packages (AUR, Nix, Homebrew)
//! - CI/CD pipeline configuration
//! - Version management and releases
//! - Health monitoring and deployment status
//! - Auto-update configuration

use crate::deployment::{
    DeploymentOrchestrator, DeploymentTarget, DeploymentStrategy, Platform, Architecture,
    OperatingSystem, Status, DeploymentStatus, DeploymentArtifact,
    packaging::{PackageManager, PackageConfig, PackageType},
    containers::{ContainerManager, ContainerConfig, ContainerRuntime},
    distributions::{DistributionManager, DistributionConfig, DistributionType},
};
use crate::theme::Theme;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs, Wrap,
        Gauge, Table, Row, Cell,
    },
    Frame,
};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// Deployment UI tabs
#[derive(Debug, Clone, PartialEq)]
pub enum DeploymentTab {
    Overview,
    Packaging,
    Containers,
    Distributions,
    Releases,
    Monitoring,
    Configuration,
}

impl DeploymentTab {
    pub fn title(&self) -> &str {
        match self {
            DeploymentTab::Overview => "Overview",
            DeploymentTab::Packaging => "Packaging", 
            DeploymentTab::Containers => "Containers",
            DeploymentTab::Distributions => "Distributions",
            DeploymentTab::Releases => "Releases",
            DeploymentTab::Monitoring => "Monitoring",
            DeploymentTab::Configuration => "Configuration",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            Self::Overview,
            Self::Packaging,
            Self::Containers,
            Self::Distributions,
            Self::Releases,
            Self::Monitoring,
            Self::Configuration,
        ]
    }
}

/// Deployment action events
#[derive(Debug, Clone, PartialEq)]
pub enum DeploymentAction {
    SwitchTab(DeploymentTab),
    BuildPackage(PackageConfig),
    BuildContainer(ContainerConfig),
    BuildDistribution(DistributionConfig),
    DeployToTarget(DeploymentTarget, DeploymentStrategy),
    CreateRelease(String),
    ConfigureMonitoring,
    UpdateSettings,
    RefreshStatus,
    ViewDeploymentDetails(Uuid),
}

/// Deployment UI state
#[derive(Debug)]
pub struct DeploymentUIState {
    pub active_tab: DeploymentTab,
    pub selected_deployment: Option<Uuid>,
    pub selected_package: Option<Uuid>,
    pub selected_container: Option<Uuid>,
    pub show_details: bool,
    pub deployments_list_state: ListState,
    pub packages_list_state: ListState,
    pub containers_list_state: ListState,
    pub status_message: Option<String>,
    pub loading: bool,
}

impl Default for DeploymentUIState {
    fn default() -> Self {
        Self {
            active_tab: DeploymentTab::Overview,
            selected_deployment: None,
            selected_package: None,
            selected_container: None,
            show_details: false,
            deployments_list_state: ListState::default(),
            packages_list_state: ListState::default(),
            containers_list_state: ListState::default(),
            status_message: None,
            loading: false,
        }
    }
}

/// Main deployment UI component
pub struct DeploymentUI {
    state: DeploymentUIState,
    orchestrator: Arc<Mutex<DeploymentOrchestrator>>,
    package_manager: Arc<Mutex<PackageManager>>,
    container_manager: Arc<Mutex<ContainerManager>>,
    distribution_manager: Arc<Mutex<DistributionManager>>,
    // Sample data for demonstration
    sample_deployments: Vec<DeploymentStatus>,
    sample_artifacts: Vec<DeploymentArtifact>,
}

impl DeploymentUI {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            state: DeploymentUIState::default(),
            orchestrator: Arc::new(Mutex::new(DeploymentOrchestrator::new()?)),
            package_manager: Arc::new(Mutex::new(PackageManager::new()?)),
            container_manager: Arc::new(Mutex::new(ContainerManager::new()?)),
            distribution_manager: Arc::new(Mutex::new(DistributionManager::new()?)),
            sample_deployments: Self::create_sample_deployments(),
            sample_artifacts: Self::create_sample_artifacts(),
        })
    }

    /// Render the deployment management interface
    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Main layout: tabs at top, content below
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        // Render tabs
        self.render_tabs(frame, main_chunks[0], theme);

        // Render active tab content
        match self.state.active_tab {
            DeploymentTab::Overview => self.render_overview_tab(frame, main_chunks[1], theme),
            DeploymentTab::Packaging => self.render_packaging_tab(frame, main_chunks[1], theme),
            DeploymentTab::Containers => self.render_containers_tab(frame, main_chunks[1], theme),
            DeploymentTab::Distributions => self.render_distributions_tab(frame, main_chunks[1], theme),
            DeploymentTab::Releases => self.render_releases_tab(frame, main_chunks[1], theme),
            DeploymentTab::Monitoring => self.render_monitoring_tab(frame, main_chunks[1], theme),
            DeploymentTab::Configuration => self.render_configuration_tab(frame, main_chunks[1], theme),
        }

        // Render status message if present
        if let Some(ref message) = self.state.status_message {
            self.render_status_message(frame, area, message, theme);
        }
    }

    /// Handle input events
    pub fn handle_input(&mut self, key: KeyCode, _modifiers: KeyModifiers) -> Option<DeploymentAction> {
        match key {
            KeyCode::Tab => {
                let current_index = DeploymentTab::all()
                    .iter()
                    .position(|t| t == &self.state.active_tab)
                    .unwrap_or(0);
                let next_index = (current_index + 1) % DeploymentTab::all().len();
                let next_tab = DeploymentTab::all()[next_index].clone();
                self.state.active_tab = next_tab.clone();
                Some(DeploymentAction::SwitchTab(next_tab))
            }
            KeyCode::BackTab => {
                let current_index = DeploymentTab::all()
                    .iter()
                    .position(|t| t == &self.state.active_tab)
                    .unwrap_or(0);
                let prev_index = if current_index == 0 {
                    DeploymentTab::all().len() - 1
                } else {
                    current_index - 1
                };
                let prev_tab = DeploymentTab::all()[prev_index].clone();
                self.state.active_tab = prev_tab.clone();
                Some(DeploymentAction::SwitchTab(prev_tab))
            }
            KeyCode::Enter => {
                match self.state.active_tab {
                    DeploymentTab::Overview => {
                        if let Some(selected) = self.state.selected_deployment {
                            Some(DeploymentAction::ViewDeploymentDetails(selected))
                        } else {
                            None
                        }
                    }
                    DeploymentTab::Packaging => {
                        // Create sample package config
                        let config = PackageConfig::for_comunicado(
                            Platform::linux_x86_64(),
                            PackageType::Debian,
                        );
                        Some(DeploymentAction::BuildPackage(config))
                    }
                    DeploymentTab::Containers => {
                        // Create sample container config
                        let config = ContainerConfig::for_comunicado(
                            Platform::linux_x86_64(),
                            ContainerRuntime::Docker,
                        );
                        Some(DeploymentAction::BuildContainer(config))
                    }
                    _ => None,
                }
            }
            KeyCode::Up => {
                match self.state.active_tab {
                    DeploymentTab::Overview => {
                        self.move_selection_up(&mut self.state.deployments_list_state, self.sample_deployments.len());
                    }
                    DeploymentTab::Packaging => {
                        self.move_selection_up(&mut self.state.packages_list_state, 5); // Sample count
                    }
                    DeploymentTab::Containers => {
                        self.move_selection_up(&mut self.state.containers_list_state, 3); // Sample count
                    }
                    _ => {}
                }
                None
            }
            KeyCode::Down => {
                match self.state.active_tab {
                    DeploymentTab::Overview => {
                        self.move_selection_down(&mut self.state.deployments_list_state, self.sample_deployments.len());
                    }
                    DeploymentTab::Packaging => {
                        self.move_selection_down(&mut self.state.packages_list_state, 5);
                    }
                    DeploymentTab::Containers => {
                        self.move_selection_down(&mut self.state.containers_list_state, 3);
                    }
                    _ => {}
                }
                None
            }
            KeyCode::F(5) => {
                Some(DeploymentAction::RefreshStatus)
            }
            KeyCode::Char('1')..=KeyCode::Char('7') => {
                if let KeyCode::Char(c) = key {
                    let index = c.to_digit(10).unwrap() as usize - 1;
                    if index < DeploymentTab::all().len() {
                        let tab = DeploymentTab::all()[index].clone();
                        self.state.active_tab = tab.clone();
                        Some(DeploymentAction::SwitchTab(tab))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Update deployment status
    pub fn update_status(&mut self, message: Option<String>) {
        self.state.status_message = message;
    }

    /// Set loading state
    pub fn set_loading(&mut self, loading: bool) {
        self.state.loading = loading;
    }

    /// Private rendering methods
    fn render_tabs(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let tab_titles: Vec<Line> = DeploymentTab::all()
            .iter()
            .enumerate()
            .map(|(i, tab)| {
                let title = format!("{}. {}", i + 1, tab.title());
                Line::from(Span::styled(
                    title,
                    if tab == &self.state.active_tab {
                        Style::default().fg(theme.highlight_color).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.text_color)
                    },
                ))
            })
            .collect();

        let tabs = Tabs::new(tab_titles)
            .block(Block::default().borders(Borders::ALL).title("Deployment Management"))
            .style(Style::default().fg(theme.text_color))
            .highlight_style(Style::default().fg(theme.highlight_color).add_modifier(Modifier::BOLD))
            .select(DeploymentTab::all().iter().position(|t| t == &self.state.active_tab).unwrap_or(0));

        frame.render_widget(tabs, area);
    }

    fn render_overview_tab(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);

        // Left: Active Deployments
        self.render_deployments_list(frame, chunks[0], theme);

        // Right: Deployment Details
        if let Some(selected_id) = self.state.selected_deployment {
            if let Some(deployment) = self.sample_deployments.iter().find(|d| d.id == selected_id) {
                self.render_deployment_details(frame, chunks[1], deployment, theme);
            }
        } else {
            self.render_deployment_summary(frame, chunks[1], theme);
        }
    }

    fn render_deployments_list(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let items: Vec<ListItem> = self.sample_deployments
            .iter()
            .map(|deployment| {
                let status_icon = match deployment.status {
                    Status::Pending => "⏳",
                    Status::InProgress { .. } => "🔄",
                    Status::Completed => "✅",
                    Status::Failed { .. } => "❌",
                    Status::RolledBack => "🔄",
                    Status::Cancelled => "⚠️",
                };

                let content = format!(
                    "{} {} v{} → {:?}",
                    status_icon,
                    "comunicado", // deployment.name would be here
                    deployment.version,
                    deployment.target
                );

                ListItem::new(Line::from(Span::styled(
                    content,
                    Style::default().fg(theme.text_color),
                )))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Active Deployments")
                    .style(Style::default().fg(theme.border_color)),
            )
            .style(Style::default().fg(theme.text_color))
            .highlight_style(Style::default().fg(theme.highlight_color).add_modifier(Modifier::BOLD));

        frame.render_stateful_widget(list, area, &mut self.state.deployments_list_state);
    }

    fn render_deployment_details(&self, frame: &mut Frame, area: Rect, deployment: &DeploymentStatus, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6), // Basic info
                Constraint::Length(4), // Progress
                Constraint::Min(0),    // Logs
            ])
            .split(area);

        // Basic info
        let info_text = vec![
            Line::from(vec![
                Span::styled("Target: ", Style::default().fg(theme.text_color).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{:?}", deployment.target), Style::default().fg(theme.text_color)),
            ]),
            Line::from(vec![
                Span::styled("Version: ", Style::default().fg(theme.text_color).add_modifier(Modifier::BOLD)),
                Span::styled(&deployment.version, Style::default().fg(theme.text_color)),
            ]),
            Line::from(vec![
                Span::styled("Strategy: ", Style::default().fg(theme.text_color).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{:?}", deployment.strategy), Style::default().fg(theme.text_color)),
            ]),
            Line::from(vec![
                Span::styled("Started: ", Style::default().fg(theme.text_color).add_modifier(Modifier::BOLD)),
                Span::styled(deployment.started_at.format("%Y-%m-%d %H:%M:%S").to_string(), Style::default().fg(theme.text_color)),
            ]),
        ];

        let info_paragraph = Paragraph::new(info_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Deployment Details")
                    .style(Style::default().fg(theme.border_color)),
            );

        frame.render_widget(info_paragraph, chunks[0]);

        // Progress bar
        let (progress, label) = match &deployment.status {
            Status::Pending => (0.0, "Pending...".to_string()),
            Status::InProgress { stage, progress } => (*progress, format!("{}: {}%", stage, (progress * 100.0) as u32)),
            Status::Completed => (1.0, "Completed".to_string()),
            Status::Failed { error } => (0.0, format!("Failed: {}", error)),
            Status::RolledBack => (0.0, "Rolled Back".to_string()),
            Status::Cancelled => (0.0, "Cancelled".to_string()),
        };

        let progress_color = match deployment.status {
            Status::Completed => Color::Green,
            Status::Failed { .. } => Color::Red,
            Status::RolledBack => Color::Yellow,
            Status::Cancelled => Color::Yellow,
            _ => theme.highlight_color,
        };

        let gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Progress")
                    .style(Style::default().fg(theme.border_color)),
            )
            .gauge_style(Style::default().fg(progress_color))
            .percent((progress * 100.0) as u16)
            .label(label);

        frame.render_widget(gauge, chunks[1]);

        // Logs
        let log_items: Vec<ListItem> = deployment.logs
            .iter()
            .map(|log| ListItem::new(Line::from(Span::styled(log, Style::default().fg(theme.text_color)))))
            .collect();

        let logs_list = List::new(log_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Deployment Logs")
                    .style(Style::default().fg(theme.border_color)),
            );

        frame.render_widget(logs_list, chunks[2]);
    }

    fn render_deployment_summary(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let summary_text = vec![
            Line::from(Span::styled("Deployment Summary", Style::default().fg(theme.text_color).add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from(vec![
                Span::styled("Total Deployments: ", Style::default().fg(theme.text_color)),
                Span::styled(self.sample_deployments.len().to_string(), Style::default().fg(theme.highlight_color)),
            ]),
            Line::from(vec![
                Span::styled("Active: ", Style::default().fg(theme.text_color)),
                Span::styled(
                    self.sample_deployments.iter().filter(|d| matches!(d.status, Status::InProgress { .. })).count().to_string(),
                    Style::default().fg(Color::Yellow),
                ),
            ]),
            Line::from(vec![
                Span::styled("Completed: ", Style::default().fg(theme.text_color)),
                Span::styled(
                    self.sample_deployments.iter().filter(|d| matches!(d.status, Status::Completed)).count().to_string(),
                    Style::default().fg(Color::Green),
                ),
            ]),
            Line::from(vec![
                Span::styled("Failed: ", Style::default().fg(theme.text_color)),
                Span::styled(
                    self.sample_deployments.iter().filter(|d| matches!(d.status, Status::Failed { .. })).count().to_string(),
                    Style::default().fg(Color::Red),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled("Select a deployment to view details", Style::default().fg(theme.text_color).add_modifier(Modifier::ITALIC))),
        ];

        let summary_paragraph = Paragraph::new(summary_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Overview")
                    .style(Style::default().fg(theme.border_color)),
            );

        frame.render_widget(summary_paragraph, area);
    }

    fn render_packaging_tab(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Left: Available package types
        let package_types = vec![
            "Debian Package (.deb)",
            "RPM Package (.rpm)", 
            "AppImage (Portable)",
            "Flatpak (Sandboxed)",
            "Archive (tar.gz)",
        ];

        let package_items: Vec<ListItem> = package_types
            .iter()
            .map(|pkg_type| ListItem::new(Line::from(Span::styled(*pkg_type, Style::default().fg(theme.text_color)))))
            .collect();

        let package_list = List::new(package_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Package Types")
                    .style(Style::default().fg(theme.border_color)),
            )
            .highlight_style(Style::default().fg(theme.highlight_color).add_modifier(Modifier::BOLD));

        frame.render_widget(package_list, chunks[0]);

        // Right: Build configuration and status
        let config_text = vec![
            Line::from(Span::styled("Build Configuration", Style::default().fg(theme.text_color).add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from(vec![
                Span::styled("Project: ", Style::default().fg(theme.text_color)),
                Span::styled("comunicado", Style::default().fg(theme.highlight_color)),
            ]),
            Line::from(vec![
                Span::styled("Version: ", Style::default().fg(theme.text_color)),
                Span::styled(env!("CARGO_PKG_VERSION"), Style::default().fg(theme.highlight_color)),
            ]),
            Line::from(vec![
                Span::styled("Architecture: ", Style::default().fg(theme.text_color)),
                Span::styled("x86_64", Style::default().fg(theme.text_color)),
            ]),
            Line::from(""),
            Line::from(Span::styled("Press Enter to build selected package type", Style::default().fg(theme.text_color).add_modifier(Modifier::ITALIC))),
            Line::from(Span::styled("Use Up/Down to select package type", Style::default().fg(theme.text_color).add_modifier(Modifier::ITALIC))),
        ];

        let config_paragraph = Paragraph::new(config_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Configuration")
                    .style(Style::default().fg(theme.border_color)),
            );

        frame.render_widget(config_paragraph, chunks[1]);
    }

    fn render_containers_tab(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(8), Constraint::Min(0)])
            .split(area);

        // Top: Container configuration
        let config_text = vec![
            Line::from(Span::styled("Container Configuration", Style::default().fg(theme.text_color).add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from(vec![
                Span::styled("Runtime: ", Style::default().fg(theme.text_color)),
                Span::styled("Docker", Style::default().fg(theme.highlight_color)),
            ]),
            Line::from(vec![
                Span::styled("Base Image: ", Style::default().fg(theme.text_color)),
                Span::styled("debian:bookworm-slim", Style::default().fg(theme.text_color)),
            ]),
            Line::from(vec![
                Span::styled("Multi-stage: ", Style::default().fg(theme.text_color)),
                Span::styled("Yes", Style::default().fg(Color::Green)),
            ]),
            Line::from(""),
            Line::from(Span::styled("Press Enter to build container image", Style::default().fg(theme.text_color).add_modifier(Modifier::ITALIC))),
        ];

        let config_paragraph = Paragraph::new(config_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Docker Container")
                    .style(Style::default().fg(theme.border_color)),
            );

        frame.render_widget(config_paragraph, chunks[0]);

        // Bottom: Available images
        let image_items = vec![
            ListItem::new(Line::from("comunicado:latest")),
            ListItem::new(Line::from("comunicado:0.1.0")),
            ListItem::new(Line::from("comunicado:dev")),
        ];

        let images_list = List::new(image_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Available Images")
                    .style(Style::default().fg(theme.border_color)),
            )
            .style(Style::default().fg(theme.text_color));

        frame.render_widget(images_list, chunks[1]);
    }

    fn render_distributions_tab(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Left: Distribution types
        let dist_items = vec![
            ListItem::new(Line::from("🏛️  AUR (Arch Linux)")),
            ListItem::new(Line::from("❄️  Nix (NixOS)")),
            ListItem::new(Line::from("🍺  Homebrew (macOS/Linux)")),
            ListItem::new(Line::from("📦  Snap (Ubuntu)")),
            ListItem::new(Line::from("🍫  Chocolatey (Windows)")),
        ];

        let dist_list = List::new(dist_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Distribution Packages")
                    .style(Style::default().fg(theme.border_color)),
            )
            .style(Style::default().fg(theme.text_color));

        frame.render_widget(dist_list, chunks[0]);

        // Right: Status and configuration
        let status_text = vec![
            Line::from(Span::styled("Distribution Status", Style::default().fg(theme.text_color).add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from(vec![
                Span::styled("AUR: ", Style::default().fg(theme.text_color)),
                Span::styled("✅ Published", Style::default().fg(Color::Green)),
            ]),
            Line::from(vec![
                Span::styled("Nix: ", Style::default().fg(theme.text_color)),
                Span::styled("🔄 Pending Review", Style::default().fg(Color::Yellow)),
            ]),
            Line::from(vec![
                Span::styled("Homebrew: ", Style::default().fg(theme.text_color)),
                Span::styled("❌ Not Submitted", Style::default().fg(Color::Red)),
            ]),
            Line::from(vec![
                Span::styled("Snap: ", Style::default().fg(theme.text_color)),
                Span::styled("✅ Published", Style::default().fg(Color::Green)),
            ]),
            Line::from(""),
            Line::from(Span::styled("Select distribution to manage", Style::default().fg(theme.text_color).add_modifier(Modifier::ITALIC))),
        ];

        let status_paragraph = Paragraph::new(status_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Publication Status")
                    .style(Style::default().fg(theme.border_color)),
            );

        frame.render_widget(status_paragraph, chunks[1]);
    }

    fn render_releases_tab(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let release_text = vec![
            Line::from(Span::styled("Release Management", Style::default().fg(theme.text_color).add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from(vec![
                Span::styled("Current Version: ", Style::default().fg(theme.text_color)),
                Span::styled(env!("CARGO_PKG_VERSION"), Style::default().fg(theme.highlight_color)),
            ]),
            Line::from(vec![
                Span::styled("Next Release: ", Style::default().fg(theme.text_color)),
                Span::styled("v0.2.0", Style::default().fg(theme.text_color)),
            ]),
            Line::from(vec![
                Span::styled("Release Type: ", Style::default().fg(theme.text_color)),
                Span::styled("Minor", Style::default().fg(Color::Yellow)),
            ]),
            Line::from(""),
            Line::from(Span::styled("🚀 Features:", Style::default().fg(theme.text_color).add_modifier(Modifier::BOLD))),
            Line::from("   • Production deployment system"),
            Line::from("   • Multi-platform packaging"),
            Line::from("   • Container support"),
            Line::from("   • Distribution integration"),
            Line::from(""),
            Line::from(Span::styled("Press Enter to create release", Style::default().fg(theme.text_color).add_modifier(Modifier::ITALIC))),
        ];

        let release_paragraph = Paragraph::new(release_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Version 0.2.0 Release")
                    .style(Style::default().fg(theme.border_color)),
            );

        frame.render_widget(release_paragraph, area);
    }

    fn render_monitoring_tab(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        // Top: Health checks
        let health_items = vec![
            ListItem::new(Line::from(vec![
                Span::styled("✅ ", Style::default().fg(Color::Green)),
                Span::styled("Application Health", Style::default().fg(theme.text_color)),
                Span::styled(" (200ms)", Style::default().fg(theme.text_color).add_modifier(Modifier::DIM)),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("✅ ", Style::default().fg(Color::Green)),
                Span::styled("Database Connection", Style::default().fg(theme.text_color)),
                Span::styled(" (15ms)", Style::default().fg(theme.text_color).add_modifier(Modifier::DIM)),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("⚠️ ", Style::default().fg(Color::Yellow)),
                Span::styled("Memory Usage", Style::default().fg(theme.text_color)),
                Span::styled(" (85%)", Style::default().fg(Color::Yellow)),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("❌ ", Style::default().fg(Color::Red)),
                Span::styled("External API", Style::default().fg(theme.text_color)),
                Span::styled(" (timeout)", Style::default().fg(Color::Red)),
            ])),
        ];

        let health_list = List::new(health_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Health Checks")
                    .style(Style::default().fg(theme.border_color)),
            );

        frame.render_widget(health_list, chunks[0]);

        // Bottom: Metrics
        let metrics_text = vec![
            Line::from(Span::styled("System Metrics", Style::default().fg(theme.text_color).add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from(vec![
                Span::styled("CPU Usage: ", Style::default().fg(theme.text_color)),
                Span::styled("45%", Style::default().fg(Color::Green)),
            ]),
            Line::from(vec![
                Span::styled("Memory: ", Style::default().fg(theme.text_color)),
                Span::styled("512MB / 1GB", Style::default().fg(Color::Yellow)),
            ]),
            Line::from(vec![
                Span::styled("Disk: ", Style::default().fg(theme.text_color)),
                Span::styled("2.1GB / 10GB", Style::default().fg(Color::Green)),
            ]),
            Line::from(vec![
                Span::styled("Network: ", Style::default().fg(theme.text_color)),
                Span::styled("125 MB/s", Style::default().fg(theme.text_color)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Active Connections: ", Style::default().fg(theme.text_color)),
                Span::styled("42", Style::default().fg(theme.highlight_color)),
            ]),
            Line::from(vec![
                Span::styled("Requests/min: ", Style::default().fg(theme.text_color)),
                Span::styled("1,247", Style::default().fg(theme.highlight_color)),
            ]),
        ];

        let metrics_paragraph = Paragraph::new(metrics_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Performance Metrics")
                    .style(Style::default().fg(theme.border_color)),
            );

        frame.render_widget(metrics_paragraph, chunks[1]);
    }

    fn render_configuration_tab(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let config_text = vec![
            Line::from(Span::styled("Deployment Configuration", Style::default().fg(theme.text_color).add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from(Span::styled("🎯 Targets:", Style::default().fg(theme.text_color).add_modifier(Modifier::BOLD))),
            Line::from("   • Production: https://comunicado.app"),
            Line::from("   • Staging: https://staging.comunicado.app"),
            Line::from("   • Development: http://localhost:3000"),
            Line::from(""),
            Line::from(Span::styled("🔧 CI/CD:", Style::default().fg(theme.text_color).add_modifier(Modifier::BOLD))),
            Line::from("   • GitHub Actions: ✅ Enabled"),
            Line::from("   • Auto-deploy to staging: ✅ Enabled"),
            Line::from("   • Manual production deploy: ✅ Enabled"),
            Line::from(""),
            Line::from(Span::styled("📦 Auto-update:", Style::default().fg(theme.text_color).add_modifier(Modifier::BOLD))),
            Line::from("   • Update channel: Stable"),
            Line::from("   • Check interval: 24 hours"),
            Line::from("   • Auto-install: ❌ Disabled"),
            Line::from(""),
            Line::from(Span::styled("⚙️ Build Settings:", Style::default().fg(theme.text_color).add_modifier(Modifier::BOLD))),
            Line::from("   • Parallel builds: 4"),
            Line::from("   • Cleanup after build: ✅ Enabled"),
            Line::from("   • Cache builds: ✅ Enabled"),
        ];

        let config_paragraph = Paragraph::new(config_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("System Configuration")
                    .style(Style::default().fg(theme.border_color)),
            );

        frame.render_widget(config_paragraph, area);
    }

    fn render_status_message(&self, frame: &mut Frame, area: Rect, message: &str, theme: &Theme) {
        let popup_area = Self::centered_rect(60, 20, area);

        // Clear the area
        frame.render_widget(Clear, popup_area);

        let message_paragraph = Paragraph::new(message)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Status")
                    .style(Style::default().fg(theme.border_color)),
            )
            .style(Style::default().fg(theme.text_color))
            .wrap(Wrap { trim: true });

        frame.render_widget(message_paragraph, popup_area);
    }

    /// Helper methods
    fn move_selection_up(&self, list_state: &mut ListState, item_count: usize) {
        if item_count == 0 {
            return;
        }
        let selected = list_state.selected().unwrap_or(0);
        let new_selected = if selected == 0 { item_count - 1 } else { selected - 1 };
        list_state.select(Some(new_selected));
    }

    fn move_selection_down(&self, list_state: &mut ListState, item_count: usize) {
        if item_count == 0 {
            return;
        }
        let selected = list_state.selected().unwrap_or(0);
        let new_selected = (selected + 1) % item_count;
        list_state.select(Some(new_selected));
    }

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

    /// Create sample data for demonstration
    fn create_sample_deployments() -> Vec<DeploymentStatus> {
        use chrono::Utc;
        
        vec![
            DeploymentStatus {
                id: Uuid::new_v4(),
                target: DeploymentTarget::Production,
                version: "0.1.0".to_string(),
                strategy: DeploymentStrategy::RollingUpdate,
                status: Status::Completed,
                started_at: Utc::now() - chrono::Duration::hours(2),
                completed_at: Some(Utc::now() - chrono::Duration::hours(1)),
                artifacts: vec![Uuid::new_v4()],
                logs: vec![
                    "Starting deployment...".to_string(),
                    "Building artifacts...".to_string(),
                    "Deploying to production...".to_string(),
                    "Deployment completed successfully".to_string(),
                ],
                health_checks: Vec::new(),
            },
            DeploymentStatus {
                id: Uuid::new_v4(),
                target: DeploymentTarget::Staging,
                version: "0.2.0-beta".to_string(),
                strategy: DeploymentStrategy::BlueGreen,
                status: Status::InProgress {
                    stage: "Health Checks".to_string(),
                    progress: 0.75,
                },
                started_at: Utc::now() - chrono::Duration::minutes(30),
                completed_at: None,
                artifacts: vec![Uuid::new_v4()],
                logs: vec![
                    "Starting blue-green deployment...".to_string(),
                    "Deploying to green environment...".to_string(),
                    "Running health checks...".to_string(),
                ],
                health_checks: Vec::new(),
            },
        ]
    }

    fn create_sample_artifacts() -> Vec<DeploymentArtifact> {
        use chrono::Utc;
        use std::collections::HashMap;
        
        vec![
            DeploymentArtifact {
                id: Uuid::new_v4(),
                name: "comunicado-0.1.0.deb".to_string(),
                version: "0.1.0".to_string(),
                platform: Platform::linux_x86_64(),
                artifact_type: ArtifactType::Deb,
                file_path: std::path::PathBuf::from("target/packages/comunicado-0.1.0.deb"),
                checksum: "sha256:abc123...".to_string(),
                size_bytes: 15_728_640, // ~15MB
                created_at: Utc::now() - chrono::Duration::hours(1),
                metadata: HashMap::new(),
            },
        ]
    }
}