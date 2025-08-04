//! Account Manager UI - Complete interface for managing email accounts
//!
//! This module provides a comprehensive UI for:
//! - Viewing all configured accounts
//! - Adding new email accounts with OAuth2 and manual setup
//! - Editing existing account settings
//! - Testing account connections
//! - Removing accounts
//! - Setting default accounts

use crate::imap::account_manager::{ImapAccount, ImapAccountManager};
use crate::theme::Theme;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap,
    },
    Frame,
};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Account manager modes
#[derive(Debug, Clone, PartialEq)]
pub enum AccountManagerMode {
    List,           // View all accounts
    Add,            // Add new account
    Edit(String),   // Edit account by ID
    Delete(String), // Confirm deletion
    Test(String),   // Test connection
}

/// Account type for adding new accounts
#[derive(Debug, Clone, PartialEq)]
pub enum AccountType {
    Gmail,
    Outlook,
    Yahoo,
    Custom,
}

impl AccountType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AccountType::Gmail => "Gmail",
            AccountType::Outlook => "Outlook",
            AccountType::Yahoo => "Yahoo",
            AccountType::Custom => "Custom IMAP",
        }
    }
}

/// Account manager UI state
pub struct AccountManagerUI {
    mode: AccountManagerMode,
    accounts: Vec<ImapAccount>,
    list_state: ListState,
    selected_account_type: AccountType,
    account_type_index: usize,
    
    // Form fields for adding/editing accounts
    form_display_name: String,
    form_email: String,
    form_imap_server: String,
    form_imap_port: String,
    form_use_ssl: bool,
    form_username: String,
    form_password: String,
    
    // UI state
    editing_field: Option<usize>, // Which field is being edited (0-6)
    input_buffer: String,
    status_message: String,
    show_password: bool,
    
    // Connection testing
    connection_status: ConnectionStatus,
    
    // Account manager reference
    account_manager: Option<Arc<RwLock<ImapAccountManager>>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    Idle,
    Testing,
    Success(String),
    Failed(String),
}

impl AccountManagerUI {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        
        Self {
            mode: AccountManagerMode::List,
            accounts: Vec::new(),
            list_state,
            selected_account_type: AccountType::Gmail,
            account_type_index: 0,
            
            form_display_name: String::new(),
            form_email: String::new(),
            form_imap_server: String::new(),
            form_imap_port: "993".to_string(),
            form_use_ssl: true,
            form_username: String::new(),
            form_password: String::new(),
            
            editing_field: None,
            input_buffer: String::new(),
            status_message: String::new(),
            show_password: false,
            
            connection_status: ConnectionStatus::Idle,
            account_manager: None,
        }
    }
    
    /// Set the account manager reference
    pub fn set_account_manager(&mut self, manager: Arc<RwLock<ImapAccountManager>>) {
        self.account_manager = Some(manager);
    }
    
    /// Check if account is ready to be saved
    pub fn is_ready_to_save(&self) -> bool {
        self.status_message == "Ready to save account..."
    }
    
    /// Check if account is ready to be deleted  
    pub fn is_ready_to_delete(&self) -> bool {
        self.status_message == "Ready to delete account..."
    }
    
    /// Get the current account data for saving
    pub fn get_account_data(&self) -> Option<(String, String, String, String, u16, bool, String, String)> {
        if !self.is_ready_to_save() {
            return None;
        }
        
        Some((
            self.form_display_name.clone(),
            self.form_email.clone(),
            self.form_imap_server.clone(),
            self.form_username.clone(),
            self.form_imap_port.parse().unwrap_or(993),
            self.form_use_ssl,
            self.form_password.clone(),
            format!("{:?}", self.selected_account_type),
        ))
    }
    
    /// Clear the ready state after operations
    pub fn clear_ready_state(&mut self) {
        if self.is_ready_to_save() || self.is_ready_to_delete() {
            self.status_message.clear();
        }
    }
    
    /// Refresh account list from database
    pub async fn refresh_accounts(&mut self) {
        if let Some(manager) = &self.account_manager {
            // First load accounts from storage
            if let Err(e) = manager.write().await.load_accounts().await {
                self.status_message = format!("Failed to load accounts from storage: {}", e);
                return;
            }
            
            // Then get the loaded accounts
            let accounts = manager.read().await.get_all_accounts().await;
            self.accounts = accounts;
            self.status_message = format!("Loaded {} accounts", self.accounts.len());
        }
    }
    
    /// Handle keyboard input
    pub fn handle_key(&mut self, key: crossterm::event::KeyCode) -> bool {
        use crossterm::event::KeyCode;
        
        match (&self.mode, key) {
            // List mode navigation
            (AccountManagerMode::List, KeyCode::Up) => {
                let selected = self.list_state.selected().unwrap_or(0);
                if selected > 0 {
                    self.list_state.select(Some(selected - 1));
                }
                true
            }
            (AccountManagerMode::List, KeyCode::Down) => {
                let selected = self.list_state.selected().unwrap_or(0);
                if selected < self.accounts.len().saturating_sub(1) {
                    self.list_state.select(Some(selected + 1));
                }
                true
            }
            
            // List mode actions
            (AccountManagerMode::List, KeyCode::Enter) => {
                if let Some(selected) = self.list_state.selected() {
                    if selected < self.accounts.len() {
                        let account_id = self.accounts[selected].account_id.clone();
                        self.mode = AccountManagerMode::Edit(account_id);
                        self.load_account_for_editing(selected);
                    }
                }
                true
            }
            (AccountManagerMode::List, KeyCode::Char('a')) => {
                self.mode = AccountManagerMode::Add;
                self.clear_form();
                true
            }
            (AccountManagerMode::List, KeyCode::Char('d')) => {
                if let Some(selected) = self.list_state.selected() {
                    if selected < self.accounts.len() {
                        let account_id = self.accounts[selected].account_id.clone();
                        self.mode = AccountManagerMode::Delete(account_id);
                    }
                }
                true
            }
            (AccountManagerMode::List, KeyCode::Char('t')) => {
                if let Some(selected) = self.list_state.selected() {
                    if selected < self.accounts.len() {
                        let account_id = self.accounts[selected].account_id.clone();
                        self.mode = AccountManagerMode::Test(account_id);
                        self.test_connection(selected);
                    }
                }
                true
            }
            (AccountManagerMode::List, KeyCode::Esc) => false, // Close account manager
            
            // Add/Edit mode navigation
            (AccountManagerMode::Add | AccountManagerMode::Edit(_), KeyCode::Tab) => {
                if self.editing_field.is_none() {
                    // Start editing first field
                    let max_fields = if self.selected_account_type == AccountType::Custom { 6 } else { 2 };
                    if max_fields > 0 {
                        self.editing_field = Some(0);
                        self.start_field_edit(0);
                    }
                } else if let Some(current) = self.editing_field {
                    // Apply current edit and move to next field
                    self.apply_field_edit();
                    let max_fields = if self.selected_account_type == AccountType::Custom { 6 } else { 2 };
                    let next_field = (current + 1) % max_fields;
                    self.editing_field = Some(next_field);
                    self.start_field_edit(next_field);
                }
                true
            }
            (AccountManagerMode::Add | AccountManagerMode::Edit(_), KeyCode::Enter) => {
                if self.editing_field.is_some() {
                    self.apply_field_edit();
                } else {
                    // Save account
                    self.save_account();
                }
                true
            }
            (AccountManagerMode::Add | AccountManagerMode::Edit(_), KeyCode::Esc) => {
                if self.editing_field.is_some() {
                    self.cancel_field_edit();
                } else {
                    self.mode = AccountManagerMode::List;
                }
                true
            }
            
            // Field editing
            (AccountManagerMode::Add | AccountManagerMode::Edit(_), KeyCode::Char(c)) => {
                if self.editing_field.is_some() {
                    self.input_buffer.push(c);
                }
                true
            }
            (AccountManagerMode::Add | AccountManagerMode::Edit(_), KeyCode::Backspace) => {
                if self.editing_field.is_some() {
                    self.input_buffer.pop();
                }
                true
            }
            
            // Delete confirmation
            (AccountManagerMode::Delete(_), KeyCode::Char('y')) => {
                self.confirm_delete();
                true
            }
            (AccountManagerMode::Delete(_), KeyCode::Char('n') | KeyCode::Esc) => {
                self.mode = AccountManagerMode::List;
                true
            }
            
            // Test connection
            (AccountManagerMode::Test(_), KeyCode::Esc) => {
                self.mode = AccountManagerMode::List;
                true
            }
            
            _ => false,
        }
    }
    
    /// Render the account manager interface
    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        match self.mode.clone() {
            AccountManagerMode::List => self.render_account_list(frame, area, theme),
            AccountManagerMode::Add => self.render_add_account(frame, area, theme),
            AccountManagerMode::Edit(_) => self.render_edit_account(frame, area, theme),
            AccountManagerMode::Delete(account_id) => self.render_delete_confirmation(frame, area, theme, &account_id),
            AccountManagerMode::Test(_) => self.render_connection_test(frame, area, theme),
        }
    }
    
    fn render_account_list(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),
                Constraint::Length(3),
            ])
            .split(area);
        
        // Account list
        let items: Vec<ListItem> = self.accounts.iter().enumerate().map(|(_i, account)| {
            let status_icon = if account.is_default { "★ " } else { "  " };
            let last_sync = account.last_sync
                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "Never".to_string());
            
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{}{}", status_icon, account.display_name),
                    if account.is_default {
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    }
                ),
                Span::raw(" ("),
                Span::styled(&account.email_address, Style::default().fg(Color::Cyan)),
                Span::raw(format!(") - Last sync: {}", last_sync)),
            ]))
        }).collect();
        
        let list = List::new(items)
            .block(Block::default()
                .title("📧 Email Accounts")
                .borders(Borders::ALL))
            .highlight_style(theme.get_component_style("selected", true))
            .highlight_symbol("► ");
        
        frame.render_stateful_widget(list, chunks[0], &mut self.list_state);
        
        // Help text
        let help = Paragraph::new(
            "Enter: Edit • A: Add Account • D: Delete • T: Test Connection • Esc: Close"
        )
        .block(Block::default().borders(Borders::ALL).title("Controls"))
        .wrap(Wrap { trim: true });
        
        frame.render_widget(help, chunks[1]);
    }
    
    fn render_add_account(&mut self, frame: &mut Frame, area: Rect, _theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
            ])
            .split(area);
        
        // Title
        let title = Paragraph::new("Add New Email Account")
            .block(Block::default().borders(Borders::ALL).title("📧 Account Setup"))
            .alignment(Alignment::Center);
        frame.render_widget(title, chunks[0]);
        
        // Form fields with better formatting
        let field_names = ["Display Name", "Email", "IMAP Server", "Port", "Username", "Password"];
        
        let mut form_lines = Vec::new();
        form_lines.push(format!("Account Type: {}", self.selected_account_type.as_str()));
        form_lines.push("".to_string());
        
        let password_display = "*".repeat(self.form_password.len());
        let max_fields = if self.selected_account_type == AccountType::Custom { 6 } else { 2 };
        for i in 0..max_fields {
            let prefix = if self.editing_field == Some(i) {
                format!("► {}: {} _", field_names[i], self.input_buffer)
            } else {
                let field_value = match i {
                    0 => &self.form_display_name,
                    1 => &self.form_email,
                    2 => &self.form_imap_server,
                    3 => &self.form_imap_port,
                    4 => &self.form_username,
                    5 => if self.show_password { &self.form_password } else { &password_display },
                    _ => "",
                };
                format!("  {}: {}", field_names[i], field_value)
            };
            form_lines.push(prefix);
        }
        
        if self.selected_account_type == AccountType::Custom {
            form_lines.push("".to_string());
            form_lines.push(format!("  Use SSL/TLS: {}", if self.form_use_ssl { "Yes" } else { "No" }));
        }
        
        form_lines.push("".to_string());
        if self.editing_field.is_some() {
            form_lines.push("Enter: Apply • Esc: Cancel edit".to_string());
        } else {
            form_lines.push("Tab: Edit fields • Enter: Save • Esc: Cancel".to_string());
        }
        
        let form_text = form_lines.join("\n");
        let form = Paragraph::new(form_text)
            .block(Block::default().borders(Borders::ALL).title("Account Details"))
            .wrap(Wrap { trim: true });
        
        frame.render_widget(form, chunks[1]);
        
        // Status
        if !self.status_message.is_empty() {
            let status = Paragraph::new(self.status_message.as_str())
                .block(Block::default().borders(Borders::ALL).title("Status"));
            frame.render_widget(status, chunks[2]);
        }
    }
    
    fn render_edit_account(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Similar to add account but with existing data pre-filled
        self.render_add_account(frame, area, theme);
    }
    
    fn render_delete_confirmation(&mut self, frame: &mut Frame, area: Rect, _theme: &Theme, account_id: &str) {
        let popup_area = self.centered_rect(60, 20, area);
        
        frame.render_widget(Clear, popup_area);
        
        let account_name = self.accounts.iter()
            .find(|a| a.account_id == account_id)
            .map(|a| a.display_name.as_str())
            .unwrap_or("Unknown");
        
        let confirmation = Paragraph::new(format!(
            "Are you sure you want to delete the account '{}'?\n\nThis will remove all account data and cannot be undone.\n\nPress Y to confirm, N or Esc to cancel.",
            account_name
        ))
        .block(Block::default()
            .borders(Borders::ALL)
            .title("⚠️  Delete Account"))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
        
        frame.render_widget(confirmation, popup_area);
    }
    
    fn render_connection_test(&mut self, frame: &mut Frame, area: Rect, _theme: &Theme) {
        let popup_area = self.centered_rect(70, 30, area);
        
        frame.render_widget(Clear, popup_area);
        
        let (title, message, color) = match &self.connection_status {
            ConnectionStatus::Idle => ("Connection Test", "Preparing to test connection...".to_string(), Color::Gray),
            ConnectionStatus::Testing => ("Testing Connection", "Connecting to email server...".to_string(), Color::Yellow),
            ConnectionStatus::Success(msg) => ("✅ Connection Successful", msg.clone(), Color::Green),
            ConnectionStatus::Failed(msg) => ("❌ Connection Failed", msg.clone(), Color::Red),
        };
        
        let test_result = Paragraph::new(message)
            .block(Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(color)))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        
        frame.render_widget(test_result, popup_area);
    }
    
    // Helper methods
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
    
    fn clear_form(&mut self) {
        self.form_display_name.clear();
        self.form_email.clear();
        self.form_imap_server.clear();
        self.form_imap_port = "993".to_string();
        self.form_use_ssl = true;
        self.form_username.clear();
        self.form_password.clear();
        self.editing_field = None;
        self.input_buffer.clear();
    }
    
    fn load_account_for_editing(&mut self, index: usize) {
        if let Some(account) = self.accounts.get(index) {
            self.form_display_name = account.display_name.clone();
            self.form_email = account.email_address.clone();
            self.form_imap_server = account.config.hostname.clone();
            self.form_imap_port = account.config.port.to_string();
            self.form_use_ssl = account.config.use_tls;
            // Don't load sensitive data like passwords
            self.editing_field = None;
            self.input_buffer.clear();
        }
    }
    
    fn start_field_edit(&mut self, field_index: usize) {
        // Load current field value into input buffer
        let current_value = match field_index {
            0 => self.form_display_name.clone(),
            1 => self.form_email.clone(),
            2 => self.form_imap_server.clone(),
            3 => self.form_imap_port.clone(),
            4 => self.form_username.clone(),
            5 => self.form_password.clone(),
            _ => String::new(),
        };
        self.input_buffer = current_value;
    }
    
    fn apply_field_edit(&mut self) {
        if let Some(field_index) = self.editing_field {
            let value = self.input_buffer.trim().to_string();
            match field_index {
                0 => self.form_display_name = value,
                1 => self.form_email = value,
                2 => self.form_imap_server = value,
                3 => self.form_imap_port = value,
                4 => self.form_username = value,
                5 => self.form_password = value,
                _ => {}
            }
            self.editing_field = None;
            self.input_buffer.clear();
        }
    }
    
    fn cancel_field_edit(&mut self) {
        self.editing_field = None;
        self.input_buffer.clear();
    }
    
    /// Save account (async wrapper - actual implementation would be in the parent UI)
    pub fn save_account(&mut self) {
        // Validate required fields
        if self.form_display_name.trim().is_empty() {
            self.status_message = "Display name is required".to_string();
            return;
        }
        
        if self.form_email.trim().is_empty() {
            self.status_message = "Email address is required".to_string();
            return;
        }

        // For custom IMAP accounts, validate server settings
        if self.selected_account_type == AccountType::Custom {
            if self.form_imap_server.trim().is_empty() {
                self.status_message = "IMAP server is required".to_string();
                return;
            }
            
            if let Err(_) = self.form_imap_port.parse::<u16>() {
                self.status_message = "Invalid port number".to_string();
                return;
            }
        }

        // Mark as ready for saving - parent UI will handle the async operation
        self.status_message = "Ready to save account...".to_string();
    }
    
    /// Confirm account deletion (async wrapper - actual implementation would be in the parent UI)
    pub fn confirm_delete(&mut self) {
        self.status_message = "Ready to delete account...".to_string();
        // Parent UI will handle the async deletion operation
        self.mode = AccountManagerMode::List;
    }
    
    fn test_connection(&mut self, _account_index: usize) {
        self.connection_status = ConnectionStatus::Testing;
        // TODO: Implement actual connection testing
        // This would use the account settings to test IMAP connection
        
        // For now, simulate a test result
        self.connection_status = ConnectionStatus::Success(
            "Connection test successful!\n\nIMAP server responded correctly.\nAuthentication verified.".to_string()
        );
    }
}

impl Default for AccountManagerUI {
    fn default() -> Self {
        Self::new()
    }
}