use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState},
    Frame,
};
use crate::theme::Theme;
use crate::oauth2::AccountConfig;

#[derive(Debug, Clone)]
pub struct AccountItem {
    pub account_id: String,
    pub display_name: String,
    pub email_address: String,
    pub provider: String,
    pub is_online: bool,
    pub unread_count: usize,
    pub sync_status: AccountSyncStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountSyncStatus {
    Online,
    Syncing,
    Offline,
    Error,
}

impl AccountItem {
    pub fn from_config(config: &AccountConfig) -> Self {
        Self {
            account_id: config.account_id.clone(),
            display_name: config.display_name.clone(),
            email_address: config.email_address.clone(),
            provider: config.provider.clone(),
            is_online: !config.is_token_expired(),
            unread_count: 0,
            sync_status: if config.is_token_expired() {
                AccountSyncStatus::Error
            } else {
                AccountSyncStatus::Online
            },
        }
    }
    
    pub fn get_provider_icon(&self) -> &'static str {
        match self.provider.to_lowercase().as_str() {
            "gmail" | "google" => "📧",
            "outlook" | "microsoft" => "📬",
            "yahoo" => "📮",
            "imap" => "📫",
            _ => "✉",
        }
    }
    
    pub fn get_status_icon(&self) -> &'static str {
        match self.sync_status {
            AccountSyncStatus::Online => "🟢",
            AccountSyncStatus::Syncing => "🟡",
            AccountSyncStatus::Offline => "⚫",
            AccountSyncStatus::Error => "🔴",
        }
    }
}

pub struct AccountSwitcher {
    accounts: Vec<AccountItem>,
    state: ListState,
    current_account_id: Option<String>,
    is_expanded: bool,
    max_display_accounts: usize,
}

impl AccountSwitcher {
    pub fn new() -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        
        Self {
            accounts: Vec::new(),
            state,
            current_account_id: None,
            is_expanded: false,
            max_display_accounts: 5,
        }
    }
    
    /// Set the list of available accounts
    pub fn set_accounts(&mut self, accounts: Vec<AccountItem>) {
        self.accounts = accounts;
        
        // If no account is selected and we have accounts, select the first one
        if self.current_account_id.is_none() && !self.accounts.is_empty() {
            self.current_account_id = Some(self.accounts[0].account_id.clone());
            self.state.select(Some(0));
        }
        
        // Ensure the selected index is valid
        if let Some(selected) = self.state.selected() {
            if selected >= self.accounts.len() {
                if self.accounts.is_empty() {
                    self.state.select(None);
                } else {
                    self.state.select(Some(0));
                    self.current_account_id = Some(self.accounts[0].account_id.clone());
                }
            }
        }
    }
    
    /// Add or update an account
    pub fn update_account(&mut self, account: AccountItem) {
        if let Some(pos) = self.accounts.iter().position(|a| a.account_id == account.account_id) {
            self.accounts[pos] = account;
        } else {
            self.accounts.push(account);
        }
    }
    
    /// Remove an account
    pub fn remove_account(&mut self, account_id: &str) {
        if let Some(pos) = self.accounts.iter().position(|a| a.account_id == account_id) {
            self.accounts.remove(pos);
            
            // Update selection if needed
            if self.current_account_id.as_ref() == Some(&account_id.to_string()) {
                if self.accounts.is_empty() {
                    self.current_account_id = None;
                    self.state.select(None);
                } else {
                    let new_index = pos.min(self.accounts.len() - 1);
                    self.current_account_id = Some(self.accounts[new_index].account_id.clone());
                    self.state.select(Some(new_index));
                }
            }
        }
    }
    
    /// Get the currently selected account
    pub fn get_current_account(&self) -> Option<&AccountItem> {
        self.current_account_id.as_ref()
            .and_then(|id| self.accounts.iter().find(|a| &a.account_id == id))
    }
    
    /// Get the currently selected account ID
    pub fn get_current_account_id(&self) -> Option<&String> {
        self.current_account_id.as_ref()
    }
    
    /// Set the current account by ID
    pub fn set_current_account(&mut self, account_id: &str) -> bool {
        if let Some(pos) = self.accounts.iter().position(|a| a.account_id == account_id) {
            self.current_account_id = Some(account_id.to_string());
            self.state.select(Some(pos));
            true
        } else {
            false
        }
    }
    
    /// Toggle the expanded state
    pub fn toggle_expanded(&mut self) {
        self.is_expanded = !self.is_expanded;
    }
    
    /// Check if expanded
    pub fn is_expanded(&self) -> bool {
        self.is_expanded
    }
    
    /// Navigate to the next account
    pub fn next_account(&mut self) -> bool {
        if self.accounts.is_empty() {
            return false;
        }
        
        let current = self.state.selected().unwrap_or(0);
        let next = (current + 1) % self.accounts.len();
        
        self.state.select(Some(next));
        self.current_account_id = Some(self.accounts[next].account_id.clone());
        true
    }
    
    /// Navigate to the previous account
    pub fn previous_account(&mut self) -> bool {
        if self.accounts.is_empty() {
            return false;
        }
        
        let current = self.state.selected().unwrap_or(0);
        let previous = if current == 0 {
            self.accounts.len() - 1
        } else {
            current - 1
        };
        
        self.state.select(Some(previous));
        self.current_account_id = Some(self.accounts[previous].account_id.clone());
        true
    }
    
    /// Select the currently highlighted account
    pub fn select_current(&mut self) -> Option<String> {
        if let Some(selected) = self.state.selected() {
            if selected < self.accounts.len() {
                let account_id = self.accounts[selected].account_id.clone();
                self.current_account_id = Some(account_id.clone());
                return Some(account_id);
            }
        }
        None
    }
    
    /// Get all accounts
    pub fn accounts(&self) -> &[AccountItem] {
        &self.accounts
    }
    
    /// Update account sync status
    pub fn update_account_status(&mut self, account_id: &str, status: AccountSyncStatus, unread_count: Option<usize>) {
        if let Some(account) = self.accounts.iter_mut().find(|a| a.account_id == account_id) {
            account.sync_status = status;
            account.is_online = !matches!(status, AccountSyncStatus::Offline | AccountSyncStatus::Error);
            
            if let Some(unread) = unread_count {
                account.unread_count = unread;
            }
        }
    }
    
    /// Check if multiple accounts are available
    pub fn has_multiple_accounts(&self) -> bool {
        self.accounts.len() > 1
    }
    
    /// Render the account switcher
    pub fn render(&mut self, frame: &mut Frame, area: Rect, block: Block, is_focused: bool, theme: &Theme) {
        if self.accounts.is_empty() {
            // Show "No accounts" message
            let items = vec![
                ListItem::new(Line::from(vec![
                    Span::styled("No accounts configured", Style::default().fg(theme.colors.palette.text_muted))
                ]))
            ];
            
            let list = List::new(items)
                .block(block.title("Accounts"))
                .highlight_style(Style::default().add_modifier(Modifier::BOLD));
                
            frame.render_widget(list, area);
            return;
        }
        
        // Create items outside of the main render logic to avoid borrow checker issues
        let selected_index = self.state.selected().unwrap_or(0);
        let (items, title, should_use_state) = if self.is_expanded {
            // Show all accounts when expanded
            let mut items = Vec::new();
            for (index, account) in self.accounts.iter().enumerate() {
                let item = AccountSwitcher::create_account_list_item_static(account, index == selected_index, theme);
                items.push(item);
            }
            (items, format!("Accounts ({}/{})", self.accounts.len(), self.accounts.len()), true)
        } else {
            // Show only current account when collapsed
            if let Some(current_account) = self.get_current_account() {
                let item = AccountSwitcher::create_current_account_item_static(current_account, &self.accounts, theme);
                (vec![item], "Account".to_string(), false)
            } else {
                let item = ListItem::new(Line::from(vec![
                    Span::styled("No account selected", Style::default().fg(theme.colors.palette.text_muted))
                ]));
                (vec![item], "Account".to_string(), false)
            }
        };
        
        let highlight_style = if is_focused {
            Style::default()
                .bg(theme.colors.palette.accent)
                .fg(theme.colors.palette.background)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .bg(theme.colors.palette.border)
                .add_modifier(Modifier::BOLD)
        };
        
        let list = List::new(items)
            .block(block.title(title))
            .highlight_style(highlight_style);
            
        if should_use_state {
            frame.render_stateful_widget(list, area, &mut self.state);
        } else {
            frame.render_widget(list, area);
        }
    }
    
    /// Create a list item for an account in expanded view (static version)
    fn create_account_list_item_static(account: &AccountItem, is_selected: bool, theme: &Theme) -> ListItem<'static> {
        let provider_icon = account.get_provider_icon();
        let status_icon = account.get_status_icon();
        
        let name_style = if is_selected {
            Style::default().fg(theme.colors.palette.accent).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.colors.palette.text_primary)
        };
        
        let email_style = Style::default().fg(theme.colors.palette.text_muted);
        let unread_style = Style::default().fg(theme.colors.palette.warning).add_modifier(Modifier::BOLD);
        
        let mut spans = vec![
            Span::raw(format!("{} ", provider_icon)),
            Span::styled(account.display_name.clone(), name_style),
            Span::raw(" ".to_string()),
            Span::styled(format!("<{}>", account.email_address), email_style),
        ];
        
        // Add unread count if > 0
        if account.unread_count > 0 {
            spans.push(Span::raw(" ".to_string()));
            spans.push(Span::styled(format!("({})", account.unread_count), unread_style));
        }
        
        // Add status indicator
        spans.push(Span::raw(" ".to_string()));
        spans.push(Span::raw(status_icon.to_string()));
        
        ListItem::new(Line::from(spans))
    }
    
    
    /// Create a list item for the current account in collapsed view (static version)
    fn create_current_account_item_static(account: &AccountItem, accounts: &[AccountItem], theme: &Theme) -> ListItem<'static> {
        let provider_icon = account.get_provider_icon();
        let status_icon = account.get_status_icon();
        
        let mut spans = vec![
            Span::raw(format!("{} ", provider_icon)),
            Span::styled(account.display_name.clone(), 
                Style::default().fg(theme.colors.palette.text_primary).add_modifier(Modifier::BOLD)),
        ];
        
        // Add unread count if > 0
        if account.unread_count > 0 {
            spans.push(Span::raw(" ".to_string()));
            spans.push(Span::styled(format!("({})", account.unread_count), 
                Style::default().fg(theme.colors.palette.warning).add_modifier(Modifier::BOLD)));
        }
        
        // Show expansion hint and status
        spans.push(Span::raw(" ".to_string()));
        if accounts.len() > 1 {
            spans.push(Span::styled("▼".to_string(), Style::default().fg(theme.colors.palette.text_muted)));
            spans.push(Span::raw(" ".to_string()));
        }
        spans.push(Span::raw(status_icon.to_string()));
        
        ListItem::new(Line::from(spans))
    }
    
}

impl Default for AccountSwitcher {
    fn default() -> Self {
        Self::new()
    }
}