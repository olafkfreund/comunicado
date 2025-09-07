//! UI components for GPG encryption management

use super::manager::EncryptionManager;
use super::manager::KeySummary;
use super::types::{EncryptionConfig, KeyInfo, TrustLevel};
use crate::theme::Theme;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs, Wrap},
    Frame,
};
use std::sync::Arc;
// RwLock import removed - not used in this file

/// Main encryption UI component
pub struct EncryptionUI {
    /// Encryption manager
    manager: Arc<EncryptionManager>,

    /// Current tab
    current_tab: EncryptionTab,

    /// List of keys
    keys: Vec<KeyInfo>,

    /// Selected key index
    selected_key: usize,

    /// List state for key list
    key_list_state: ListState,

    /// Configuration
    config: EncryptionConfig,

    /// Key summary
    key_summary: Option<KeySummary>,

    /// Currently viewing key details
    viewing_key_details: bool,

    /// Key import/export buffer  
    key_buffer: String,

    /// Status message
    status_message: Option<String>,

    /// Error message
    error_message: Option<String>,
}

/// Encryption UI tabs
#[derive(Debug, Clone, Copy, PartialEq)]
enum EncryptionTab {
    Keys,
    Config,
    Import,
    Export,
    Generate,
}

impl EncryptionTab {
    fn name(&self) -> &'static str {
        match self {
            EncryptionTab::Keys => "Keys",
            EncryptionTab::Config => "Config",
            EncryptionTab::Import => "Import",
            EncryptionTab::Export => "Export",
            EncryptionTab::Generate => "Generate",
        }
    }

    fn all() -> &'static [EncryptionTab] {
        &[
            EncryptionTab::Keys,
            EncryptionTab::Config,
            EncryptionTab::Import,
            EncryptionTab::Export,
            EncryptionTab::Generate,
        ]
    }
}

impl EncryptionUI {
    /// Create new encryption UI
    pub fn new(manager: Arc<EncryptionManager>) -> Self {
        let mut key_list_state = ListState::default();
        key_list_state.select(Some(0));

        Self {
            manager,
            current_tab: EncryptionTab::Keys,
            keys: Vec::new(),
            selected_key: 0,
            key_list_state,
            config: EncryptionConfig::default(),
            key_summary: None,
            viewing_key_details: false,
            key_buffer: String::new(),
            status_message: None,
            error_message: None,
        }
    }

    /// Initialize the UI (load keys and config)
    pub async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.refresh_keys().await?;
        self.refresh_config().await?;
        self.refresh_summary().await?;
        Ok(())
    }

    /// Refresh key list
    async fn refresh_keys(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        match self.manager.list_keys(false).await {
            Ok(keys) => {
                self.keys = keys;
                if self.selected_key >= self.keys.len() && !self.keys.is_empty() {
                    self.selected_key = self.keys.len() - 1;
                }
                self.key_list_state.select(Some(self.selected_key));
                self.status_message = Some(format!("Loaded {} keys", self.keys.len()));
                self.error_message = None;
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to load keys: {}", e));
            }
        }
        Ok(())
    }

    /// Refresh configuration
    async fn refresh_config(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.config = self.manager.get_config().await;
        Ok(())
    }

    /// Refresh key summary
    async fn refresh_summary(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        match self.manager.get_key_summary().await {
            Ok(summary) => {
                self.key_summary = Some(summary);
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to get key summary: {}", e));
            }
        }
        Ok(())
    }

    /// Handle key input
    pub async fn handle_key(&mut self, key: KeyEvent) -> Result<bool, Box<dyn std::error::Error>> {
        match key.code {
            KeyCode::Tab => {
                self.next_tab();
                Ok(false)
            }
            KeyCode::BackTab => {
                self.previous_tab();
                Ok(false)
            }
            KeyCode::Esc => {
                if self.viewing_key_details {
                    self.viewing_key_details = false;
                    Ok(false)
                } else {
                    Ok(true) // Exit encryption UI
                }
            }
            _ => match self.current_tab {
                EncryptionTab::Keys => self.handle_keys_tab_key(key).await,
                EncryptionTab::Config => self.handle_config_tab_key(key).await,
                EncryptionTab::Import => self.handle_import_tab_key(key).await,
                EncryptionTab::Export => self.handle_export_tab_key(key).await,
                EncryptionTab::Generate => self.handle_generate_tab_key(key).await,
            },
        }
    }

    /// Handle keys tab input
    async fn handle_keys_tab_key(
        &mut self,
        key: KeyEvent,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        match key.code {
            KeyCode::Up => {
                if self.selected_key > 0 {
                    self.selected_key -= 1;
                    self.key_list_state.select(Some(self.selected_key));
                }
                Ok(false)
            }
            KeyCode::Down => {
                if self.selected_key < self.keys.len().saturating_sub(1) {
                    self.selected_key += 1;
                    self.key_list_state.select(Some(self.selected_key));
                }
                Ok(false)
            }
            KeyCode::Enter => {
                self.viewing_key_details = !self.viewing_key_details;
                Ok(false)
            }
            KeyCode::Char('r') => {
                self.refresh_keys().await?;
                Ok(false)
            }
            _ => Ok(false),
        }
    }

    /// Handle config tab input
    async fn handle_config_tab_key(
        &mut self,
        _key: KeyEvent,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        // TODO: Implement config editing
        Ok(false)
    }

    /// Handle import tab input
    async fn handle_import_tab_key(
        &mut self,
        key: KeyEvent,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        match key.code {
            KeyCode::Char(c) => {
                self.key_buffer.push(c);
                Ok(false)
            }
            KeyCode::Backspace => {
                self.key_buffer.pop();
                Ok(false)
            }
            KeyCode::Enter => {
                if !self.key_buffer.trim().is_empty() {
                    match self.manager.import_key(&self.key_buffer).await {
                        Ok(imported) => {
                            self.status_message =
                                Some(format!("Imported {} key(s)", imported.len()));
                            self.key_buffer.clear();
                            self.refresh_keys().await?;
                        }
                        Err(e) => {
                            self.error_message = Some(format!("Import failed: {}", e));
                        }
                    }
                }
                Ok(false)
            }
            _ => Ok(false),
        }
    }

    /// Handle export tab input
    async fn handle_export_tab_key(
        &mut self,
        key: KeyEvent,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        match key.code {
            KeyCode::Enter => {
                if let Some(selected_key) = self.keys.get(self.selected_key) {
                    match self.manager.export_key(&selected_key.key_id, false).await {
                        Ok(exported) => {
                            self.key_buffer = exported;
                            self.status_message = Some("Key exported to buffer".to_string());
                        }
                        Err(e) => {
                            self.error_message = Some(format!("Export failed: {}", e));
                        }
                    }
                }
                Ok(false)
            }
            _ => Ok(false),
        }
    }

    /// Handle generate tab input
    async fn handle_generate_tab_key(
        &mut self,
        _key: KeyEvent,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        // TODO: Implement key generation interface
        Ok(false)
    }

    /// Switch to next tab
    fn next_tab(&mut self) {
        let tabs = EncryptionTab::all();
        let current_index = tabs
            .iter()
            .position(|&t| t == self.current_tab)
            .unwrap_or(0);
        let next_index = (current_index + 1) % tabs.len();
        self.current_tab = tabs[next_index];
    }

    /// Switch to previous tab
    fn previous_tab(&mut self) {
        let tabs = EncryptionTab::all();
        let current_index = tabs
            .iter()
            .position(|&t| t == self.current_tab)
            .unwrap_or(0);
        let prev_index = if current_index == 0 {
            tabs.len() - 1
        } else {
            current_index - 1
        };
        self.current_tab = tabs[prev_index];
    }

    /// Render the encryption UI
    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Main layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Tabs
                Constraint::Min(0),    // Content
                Constraint::Length(3), // Status
            ])
            .split(area);

        // Render tabs
        self.render_tabs(frame, chunks[0], theme);

        // Render content based on current tab
        match self.current_tab {
            EncryptionTab::Keys => self.render_keys_tab(frame, chunks[1], theme),
            EncryptionTab::Config => self.render_config_tab(frame, chunks[1], theme),
            EncryptionTab::Import => self.render_import_tab(frame, chunks[1], theme),
            EncryptionTab::Export => self.render_export_tab(frame, chunks[1], theme),
            EncryptionTab::Generate => self.render_generate_tab(frame, chunks[1], theme),
        }

        // Render status
        self.render_status(frame, chunks[2], theme);

        // Render key details popup if viewing
        if self.viewing_key_details {
            self.render_key_details_popup(frame, area, theme);
        }
    }

    /// Render tab bar
    fn render_tabs(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let tab_names: Vec<&str> = EncryptionTab::all().iter().map(|t| t.name()).collect();
        let current_index = EncryptionTab::all()
            .iter()
            .position(|&t| t == self.current_tab)
            .unwrap_or(0);

        let tabs = Tabs::new(tab_names)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("GPG Encryption"),
            )
            .style(Style::default().fg(theme.colors.palette.text_primary))
            .highlight_style(
                Style::default()
                    .fg(theme.colors.palette.accent)
                    .add_modifier(Modifier::BOLD),
            )
            .select(current_index);

        frame.render_widget(tabs, area);
    }

    /// Render keys tab
    fn render_keys_tab(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(area);

        // Key list
        let key_items: Vec<ListItem> = self
            .keys
            .iter()
            .enumerate()
            .map(|(i, key)| {
                let style = if i == self.selected_key {
                    Style::default()
                        .bg(theme.colors.palette.accent)
                        .fg(theme.colors.palette.background)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.colors.palette.text_primary)
                };

                let trust_indicator = match key.trust_level {
                    TrustLevel::Ultimate => "🔐",
                    TrustLevel::Full => "✅",
                    TrustLevel::Marginal => "⚠️",
                    TrustLevel::Never => "❌",
                    TrustLevel::Unknown => "❓",
                };

                let capabilities = format!(
                    "{}{}{}{}",
                    if key.capabilities.can_encrypt {
                        "E"
                    } else {
                        "-"
                    },
                    if key.capabilities.can_sign { "S" } else { "-" },
                    if key.capabilities.can_certify {
                        "C"
                    } else {
                        "-"
                    },
                    if key.capabilities.can_authenticate {
                        "A"
                    } else {
                        "-"
                    },
                );

                let line = format!(
                    "{} {} {} [{}] {}",
                    trust_indicator,
                    key.key_id.chars().take(8).collect::<String>(),
                    capabilities,
                    key.primary_identity().unwrap_or("No ID"),
                    if key.is_expired { "(EXPIRED)" } else { "" }
                );

                ListItem::new(line).style(style)
            })
            .collect();

        let key_list = List::new(key_items)
            .block(Block::default().borders(Borders::ALL).title("GPG Keys"))
            .highlight_style(
                Style::default()
                    .bg(theme.colors.palette.accent)
                    .add_modifier(Modifier::BOLD),
            );

        frame.render_stateful_widget(key_list, chunks[0], &mut self.key_list_state);

        // Key summary
        if let Some(ref summary) = self.key_summary {
            let summary_text = vec![
                Line::from(format!("Total Keys: {}", summary.total_keys)),
                Line::from(format!("Secret Keys: {}", summary.secret_keys)),
                Line::from(format!("Can Encrypt: {}", summary.encryption_keys)),
                Line::from(format!("Can Sign: {}", summary.signing_keys)),
                Line::from(""),
                Line::from(format!("Expired: {}", summary.expired_keys)),
                Line::from(format!("Revoked: {}", summary.revoked_keys)),
            ];

            let summary_paragraph = Paragraph::new(summary_text)
                .block(Block::default().borders(Borders::ALL).title("Summary"))
                .wrap(Wrap { trim: true });

            frame.render_widget(summary_paragraph, chunks[1]);
        }
    }

    /// Render config tab
    fn render_config_tab(&self, frame: &mut Frame, area: Rect, _theme: &Theme) {
        let config_items = vec![
            format!("Always Sign: {}", self.config.always_sign),
            format!("Auto Encrypt: {}", self.config.auto_encrypt),
            format!(
                "Default Signing Key: {}",
                self.config.default_signing_key.as_deref().unwrap_or("None")
            ),
            format!(
                "Key Server: {}",
                self.config.key_server.as_deref().unwrap_or("None")
            ),
            format!("Operation Timeout: {}s", self.config.operation_timeout),
        ];

        let config_text: Vec<Line> = config_items
            .into_iter()
            .map(|item| Line::from(item))
            .collect();

        let config_paragraph = Paragraph::new(config_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Configuration"),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(config_paragraph, area);
    }

    /// Render import tab
    fn render_import_tab(&self, frame: &mut Frame, area: Rect, _theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        // Instructions
        let instructions = Paragraph::new("Paste GPG key data below and press Enter to import:")
            .block(Block::default().borders(Borders::ALL).title("Import Key"))
            .wrap(Wrap { trim: true });

        frame.render_widget(instructions, chunks[0]);

        // Text area
        let import_text = Paragraph::new(self.key_buffer.as_str())
            .block(Block::default().borders(Borders::ALL).title("Key Data"))
            .wrap(Wrap { trim: true });

        frame.render_widget(import_text, chunks[1]);
    }

    /// Render export tab
    fn render_export_tab(&self, frame: &mut Frame, area: Rect, _theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        // Instructions
        let instructions =
            Paragraph::new("Select a key from the Keys tab and press Enter to export:")
                .block(Block::default().borders(Borders::ALL).title("Export Key"))
                .wrap(Wrap { trim: true });

        frame.render_widget(instructions, chunks[0]);

        // Export buffer
        let export_text = Paragraph::new(self.key_buffer.as_str())
            .block(Block::default().borders(Borders::ALL).title("Exported Key"))
            .wrap(Wrap { trim: true });

        frame.render_widget(export_text, chunks[1]);
    }

    /// Render generate tab
    fn render_generate_tab(&self, frame: &mut Frame, area: Rect, _theme: &Theme) {
        let placeholder = Paragraph::new("Key generation interface - Coming soon!")
            .block(Block::default().borders(Borders::ALL).title("Generate Key"))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        frame.render_widget(placeholder, area);
    }

    /// Render status bar
    fn render_status(&self, frame: &mut Frame, area: Rect, _theme: &Theme) {
        let status_text = if let Some(ref error) = self.error_message {
            Text::from(vec![Line::from(vec![
                Span::styled("Error: ", Style::default().fg(Color::Red)),
                Span::raw(error),
            ])])
        } else if let Some(ref status) = self.status_message {
            Text::from(vec![Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::Green)),
                Span::raw(status),
            ])])
        } else {
            Text::from("Ready")
        };

        let status_paragraph = Paragraph::new(status_text)
            .block(Block::default().borders(Borders::ALL).title("Status"))
            .wrap(Wrap { trim: true });

        frame.render_widget(status_paragraph, area);
    }

    /// Render key details popup
    fn render_key_details_popup(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if let Some(key) = self.keys.get(self.selected_key) {
            // Calculate popup area (80% of screen)
            let popup_area = Rect {
                x: area.width / 10,
                y: area.height / 10,
                width: area.width * 8 / 10,
                height: area.height * 8 / 10,
            };

            // Clear background
            frame.render_widget(Clear, popup_area);

            // Key details
            let details = vec![
                Line::from(format!("Key ID: {}", key.key_id)),
                Line::from(format!("Fingerprint: {}", key.fingerprint)),
                Line::from(""),
                Line::from("User IDs:"),
            ];

            let mut all_lines = details;
            for uid in &key.user_ids {
                all_lines.push(Line::from(format!("  {}", uid)));
            }

            all_lines.push(Line::from(""));
            all_lines.push(Line::from(format!("Trust Level: {}", key.trust_level)));
            all_lines.push(Line::from(format!(
                "Can Encrypt: {}",
                key.capabilities.can_encrypt
            )));
            all_lines.push(Line::from(format!(
                "Can Sign: {}",
                key.capabilities.can_sign
            )));
            all_lines.push(Line::from(format!(
                "Can Certify: {}",
                key.capabilities.can_certify
            )));
            all_lines.push(Line::from(format!(
                "Can Authenticate: {}",
                key.capabilities.can_authenticate
            )));

            if let Some(creation) = key.creation_date {
                all_lines.push(Line::from(format!(
                    "Created: {}",
                    creation.format("%Y-%m-%d")
                )));
            }

            if let Some(expiration) = key.expiration_date {
                all_lines.push(Line::from(format!(
                    "Expires: {}",
                    expiration.format("%Y-%m-%d")
                )));
            }

            all_lines.push(Line::from(""));
            all_lines.push(Line::from("Press Esc to close"));

            let details_text = Text::from(all_lines);

            let popup = Paragraph::new(details_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Key Details")
                        .border_style(Style::default().fg(theme.colors.palette.accent)),
                )
                .wrap(Wrap { trim: true });

            frame.render_widget(popup, popup_area);
        }
    }
}
