use crate::oauth2::{AccountConfig, SecureStorage};
use crate::theme::Theme;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table, Wrap},
    Frame,
};
use std::sync::Arc;

/// Account inspector for viewing and managing OAuth2 accounts
pub struct AccountInspector {
    accounts: Vec<AccountConfig>,
    selected_index: usize,
    list_state: ListState,
    storage: Arc<SecureStorage>,
    theme: Theme,
}

impl AccountInspector {
    /// Create a new account inspector
    pub fn new(storage: Arc<SecureStorage>) -> Result<Self, Box<dyn std::error::Error>> {
        let mut inspector = Self {
            accounts: Vec::new(),
            selected_index: 0,
            list_state: ListState::default(),
            storage,
            theme: Theme::default(),
        };
        
        inspector.refresh_accounts()?;
        Ok(inspector)
    }

    /// Refresh the list of accounts from storage
    pub fn refresh_accounts(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.accounts = self.storage.load_all_accounts()?;
        
        // Update selection if needed
        if !self.accounts.is_empty() {
            self.selected_index = self.selected_index.min(self.accounts.len() - 1);
            self.list_state.select(Some(self.selected_index));
        } else {
            self.selected_index = 0;
            self.list_state.select(None);
        }
        
        Ok(())
    }

    /// Move selection up
    pub fn select_previous(&mut self) {
        if !self.accounts.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.accounts.len() - 1
            } else {
                self.selected_index - 1
            };
            self.list_state.select(Some(self.selected_index));
        }
    }

    /// Move selection down
    pub fn select_next(&mut self) {
        if !self.accounts.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.accounts.len();
            self.list_state.select(Some(self.selected_index));
        }
    }

    /// Get the currently selected account
    pub fn get_selected_account(&self) -> Option<&AccountConfig> {
        self.accounts.get(self.selected_index)
    }

    /// Delete the currently selected account
    pub fn delete_selected_account(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        if let Some(account) = self.get_selected_account() {
            let account_id = account.account_id.clone();
            self.storage.delete_account(&account_id)?;
            self.refresh_accounts()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Render the account inspector interface
    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        // Split the area into two parts: list and details
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)].as_ref())
            .split(area);

        self.render_account_list(f, chunks[0]);
        self.render_account_details(f, chunks[1]);
    }

    /// Render the account list
    fn render_account_list(&mut self, f: &mut Frame, area: Rect) {
        if self.accounts.is_empty() {
            let no_accounts = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    "No accounts configured",
                    Style::default().fg(Color::Yellow),
                )),
                Line::from(""),
                Line::from("To add a new account, use the CLI:"),
                Line::from(""),
                Line::from(Span::styled(
                    "comunicado setup-gmail",
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    "comunicado setup-outlook",
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from("Press 'q' to return to email view"),
            ])
            .block(
                Block::default()
                    .title("Account Inspector")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(self.theme.colors.palette.border)),
            )
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

            f.render_widget(no_accounts, area);
            return;
        }

        let items: Vec<ListItem> = self
            .accounts
            .iter()
            .enumerate()
            .map(|(i, account)| {
                let status_symbol = if account.is_token_expired() {
                    "⚠️ "
                } else {
                    "✅ "
                };

                let content = vec![Line::from(vec![
                    Span::raw(status_symbol),
                    Span::styled(
                        &account.display_name,
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" ("),
                    Span::styled(&account.email_address, Style::default().fg(Color::Cyan)),
                    Span::raw(")"),
                ])];

                ListItem::new(content).style(if i == self.selected_index {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                })
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Accounts")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(self.theme.colors.palette.border)),
            )
            .highlight_style(Style::default().bg(Color::DarkGray));

        f.render_stateful_widget(list, area, &mut self.list_state);
    }

    /// Render account details
    fn render_account_details(&self, f: &mut Frame, area: Rect) {
        if let Some(account) = self.get_selected_account() {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(12), Constraint::Min(6)].as_ref())
                .split(area);

            self.render_account_info(f, chunks[0], account);
            self.render_help_text(f, chunks[1]);
        } else {
            let empty = Paragraph::new("Select an account to view details")
                .block(
                    Block::default()
                        .title("Account Details")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(self.theme.colors.palette.border)),
                )
                .alignment(Alignment::Center);

            f.render_widget(empty, area);
        }
    }

    /// Render detailed account information
    fn render_account_info(&self, f: &mut Frame, area: Rect, account: &AccountConfig) {
        let status = if account.is_token_expired() {
            ("⚠️ Expired", Color::Red)
        } else {
            ("✅ Active", Color::Green)
        };

        let token_info = if account.access_token.is_empty() {
            "No access token"
        } else {
            "Token available"
        };

        let refresh_info = if account.refresh_token.is_some() {
            "Can refresh"
        } else {
            "Manual re-auth needed"
        };

        let expiry_info = if let Some(expires_at) = account.token_expires_at {
            format!("Expires: {}", expires_at.format("%Y-%m-%d %H:%M:%S UTC"))
        } else {
            "No expiration set".to_string()
        };

        let rows = vec![
            Row::new(vec![
                Cell::from("Display Name"),
                Cell::from(account.display_name.clone()),
            ]),
            Row::new(vec![
                Cell::from("Email Address"),
                Cell::from(account.email_address.clone()),
            ]),
            Row::new(vec![
                Cell::from("Provider"),
                Cell::from(account.provider.clone()),
            ]),
            Row::new(vec![
                Cell::from("Status"),
                Cell::from(Span::styled(status.0, Style::default().fg(status.1))),
            ]),
            Row::new(vec![
                Cell::from("IMAP Server"),
                Cell::from(format!("{}:{}", account.imap_server, account.imap_port)),
            ]),
            Row::new(vec![
                Cell::from("SMTP Server"),
                Cell::from(format!("{}:{}", account.smtp_server, account.smtp_port)),
            ]),
            Row::new(vec![Cell::from("Token Info"), Cell::from(token_info)]),
            Row::new(vec![Cell::from("Refresh"), Cell::from(refresh_info)]),
            Row::new(vec![Cell::from("Expiry"), Cell::from(expiry_info)]),
        ];

        let table = Table::new(rows, [Constraint::Length(15), Constraint::Min(30)])
            .block(
                Block::default()
                    .title("Account Details")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(self.theme.colors.palette.border)),
            )
            .column_spacing(2);

        f.render_widget(table, area);
    }

    /// Render help text
    fn render_help_text(&self, f: &mut Frame, area: Rect) {
        let help_lines = vec![
            Line::from("Keyboard shortcuts:"),
            Line::from(""),
            Line::from(vec![
                Span::styled("↑/↓", Style::default().fg(Color::Yellow)),
                Span::raw(" - Navigate accounts"),
            ]),
            Line::from(vec![
                Span::styled("d", Style::default().fg(Color::Yellow)),
                Span::raw(" - Delete selected account"),
            ]),
            Line::from(vec![
                Span::styled("r", Style::default().fg(Color::Yellow)),
                Span::raw(" - Refresh account list"),
            ]),
            Line::from(vec![
                Span::styled("q", Style::default().fg(Color::Yellow)),
                Span::raw(" - Return to email view"),
            ]),
            Line::from(""),
            Line::from("To add new accounts:"),
            Line::from(vec![
                Span::styled("comunicado setup-gmail", Style::default().fg(Color::Green)),
            ]),
            Line::from(vec![
                Span::styled("comunicado setup-outlook", Style::default().fg(Color::Green)),
            ]),
        ];

        let help = Paragraph::new(help_lines)
            .block(
                Block::default()
                    .title("Help")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(self.theme.colors.palette.border)),
            )
            .wrap(Wrap { trim: true });

        f.render_widget(help, area);
    }

    /// Get the number of accounts
    pub fn account_count(&self) -> usize {
        self.accounts.len()
    }

    /// Check if any accounts exist
    pub fn has_accounts(&self) -> bool {
        !self.accounts.is_empty()
    }
}