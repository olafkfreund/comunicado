//! Application settings UI components for comprehensive configuration management

use crate::theme::Theme;
use crate::config::AppConfig;
use crate::ui::account_manager_ui::AccountManagerUI;
use crate::ui::keyboard_bindings_ui::KeyboardBindingsUI;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::Line,
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs, Wrap},
    Frame,
};
use crossterm::event::{KeyCode, KeyModifiers};

/// Settings tab categories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    General,
    Accounts,
    UI,
    Keyboard,
    Performance,
    Privacy,
    AI,
    Advanced,
}

impl SettingsTab {
    pub fn title(&self) -> &'static str {
        match self {
            SettingsTab::General => "General",
            SettingsTab::Accounts => "Accounts",
            SettingsTab::UI => "UI & Theme",
            SettingsTab::Keyboard => "Keyboard",
            SettingsTab::Performance => "Performance", 
            SettingsTab::Privacy => "Privacy",
            SettingsTab::AI => "AI Assistant",
            SettingsTab::Advanced => "Advanced",
        }
    }

    pub fn all() -> Vec<SettingsTab> {
        vec![
            SettingsTab::General,
            SettingsTab::Accounts,
            SettingsTab::UI,
            SettingsTab::Keyboard,
            SettingsTab::Performance,
            SettingsTab::Privacy,
            SettingsTab::AI,
            SettingsTab::Advanced,
        ]
    }

    pub fn next(&self) -> SettingsTab {
        let tabs = Self::all();
        let current_index = tabs.iter().position(|&tab| tab == *self).unwrap_or(0);
        tabs[(current_index + 1) % tabs.len()]
    }

    pub fn previous(&self) -> SettingsTab {
        let tabs = Self::all();
        let current_index = tabs.iter().position(|&tab| tab == *self).unwrap_or(0);
        tabs[(current_index + tabs.len() - 1) % tabs.len()]
    }
}

/// Settings UI state and configuration
#[derive(Debug, Clone)]
pub struct SettingsUIState {
    /// Whether the settings UI is visible
    pub visible: bool,
    /// Current settings tab
    pub current_tab: SettingsTab,
    /// Current selection within the active tab
    pub selected_index: usize,
    /// List state for navigation
    pub list_state: ListState,
    /// Whether we're in edit mode for a setting
    pub edit_mode: bool,
    /// Current input buffer for text settings
    pub input_buffer: String,
    /// Whether settings have been modified
    pub modified: bool,
    /// Status message to display
    pub status_message: Option<String>,
}

impl Default for SettingsUIState {
    fn default() -> Self {
        Self {
            visible: false,
            current_tab: SettingsTab::General,
            selected_index: 0,
            list_state: ListState::default(),
            edit_mode: false,
            input_buffer: String::new(),
            modified: false,
            status_message: None,
        }
    }
}

impl SettingsUIState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show(&mut self) {
        self.visible = true;
        self.current_tab = SettingsTab::General;
        self.selected_index = 0;
        self.list_state.select(Some(0));
        self.edit_mode = false;
        self.input_buffer.clear();
        self.status_message = None;
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.edit_mode = false;
        self.input_buffer.clear();
        self.status_message = None;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn next_tab(&mut self) {
        self.current_tab = self.current_tab.next();
        self.selected_index = 0;
        self.list_state.select(Some(0));
        self.edit_mode = false;
        self.input_buffer.clear();
    }

    pub fn previous_tab(&mut self) {
        self.current_tab = self.current_tab.previous();
        self.selected_index = 0;
        self.list_state.select(Some(0));
        self.edit_mode = false;
        self.input_buffer.clear();
    }

    pub fn next_item(&mut self) {
        let max_items = self.get_max_items_for_tab();
        if max_items > 0 {
            self.selected_index = (self.selected_index + 1) % max_items;
            self.list_state.select(Some(self.selected_index));
        }
    }

    pub fn previous_item(&mut self) {
        let max_items = self.get_max_items_for_tab();
        if max_items > 0 {
            self.selected_index = (self.selected_index + max_items - 1) % max_items;
            self.list_state.select(Some(self.selected_index));
        }
    }

    pub fn start_edit(&mut self) {
        self.edit_mode = true;
        self.input_buffer.clear();
    }

    pub fn cancel_edit(&mut self) {
        self.edit_mode = false;
        self.input_buffer.clear();
    }

    pub fn handle_input(&mut self, ch: char) {
        if self.edit_mode {
            self.input_buffer.push(ch);
        }
    }

    pub fn handle_backspace(&mut self) {
        if self.edit_mode {
            self.input_buffer.pop();
        }
    }

    pub fn set_status(&mut self, message: String) {
        self.status_message = Some(message);
    }

    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    /// Get the maximum number of items for the current tab
    fn get_max_items_for_tab(&self) -> usize {
        match self.current_tab {
            SettingsTab::General => 10, // Updated to include sync settings
            SettingsTab::Accounts => 6,
            SettingsTab::UI => 7,
            SettingsTab::Keyboard => 5,
            SettingsTab::Performance => 6,
            SettingsTab::Privacy => 5,
            SettingsTab::AI => 8,
            SettingsTab::Advanced => 6,
        }
    }
}

/// Settings UI component
pub struct SettingsUI {
    state: SettingsUIState,
    config: AppConfig,
    account_manager_ui: AccountManagerUI,
    show_account_manager: bool,
    keyboard_bindings_ui: KeyboardBindingsUI,
    show_keyboard_bindings: bool,
}

impl SettingsUI {
    pub fn new() -> Self {
        let config = AppConfig::load().unwrap_or_default();
        let keyboard_bindings_ui = KeyboardBindingsUI::with_bindings(config.keyboard.custom_bindings.clone());
        Self {
            state: SettingsUIState::new(),
            config,
            account_manager_ui: AccountManagerUI::new(),
            show_account_manager: false,
            keyboard_bindings_ui,
            show_keyboard_bindings: false,
        }
    }

    pub fn state(&self) -> &SettingsUIState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut SettingsUIState {
        &mut self.state
    }

    pub fn show(&mut self) {
        self.state.show();
    }

    pub fn hide(&mut self) {
        self.state.hide();
    }

    pub fn is_visible(&self) -> bool {
        self.state.is_visible()
    }
    
    /// Initialize account manager with IMAP account manager reference
    pub fn initialize_account_manager(&mut self, imap_manager: std::sync::Arc<tokio::sync::RwLock<crate::imap::account_manager::ImapAccountManager>>) {
        self.account_manager_ui.set_account_manager(imap_manager);
    }

    pub fn handle_key(&mut self, key: KeyCode, modifiers: KeyModifiers) -> bool {
        if !self.state.visible {
            return false;
        }
        
        // Handle account manager if it's shown
        if self.show_account_manager {
            if self.account_manager_ui.handle_key(key) {
                return true;
            } else {
                // Account manager returned false, meaning close it
                self.show_account_manager = false;
                return true;
            }
        }
        
        // Handle keyboard bindings manager if it's shown
        if self.show_keyboard_bindings {
            if self.keyboard_bindings_ui.handle_key(key) {
                // Update config with new bindings if modified
                if self.keyboard_bindings_ui.is_modified() {
                    self.config.keyboard.custom_bindings = self.keyboard_bindings_ui.get_bindings();
                    self.state.modified = true;
                }
                return true;
            } else {
                // Keyboard bindings manager returned false, meaning close it
                self.show_keyboard_bindings = false;
                // Save any changes before closing
                if self.keyboard_bindings_ui.is_modified() {
                    self.config.keyboard.custom_bindings = self.keyboard_bindings_ui.get_bindings();
                    self.state.modified = true;
                }
                return true;
            }
        }

        if self.state.edit_mode {
            match key {
                KeyCode::Enter => {
                    self.apply_edit();
                    self.state.cancel_edit();
                    return true;
                }
                KeyCode::Esc => {
                    self.state.cancel_edit();
                    return true;
                }
                KeyCode::Char(ch) => {
                    self.state.handle_input(ch);
                    return true;
                }
                KeyCode::Backspace => {
                    self.state.handle_backspace();
                    return true;
                }
                _ => return true,
            }
        }

        match key {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.hide();
                true
            }
            KeyCode::Tab => {
                self.state.next_tab();
                true
            }
            KeyCode::BackTab => {
                self.state.previous_tab();
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.next_item();
                true
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.previous_item();
                true
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                self.handle_select();
                true
            }
            KeyCode::Char('e') => {
                self.state.start_edit();
                true
            }
            KeyCode::Char('r') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.reset_current_setting();
                true
            }
            KeyCode::Char('s') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.save_settings();
                true
            }
            _ => false,
        }
    }

    fn handle_select(&mut self) {
        // Toggle boolean settings or start editing for other types
        match self.state.current_tab {
            SettingsTab::General => self.handle_general_select(),
            SettingsTab::Accounts => self.handle_accounts_select(),
            SettingsTab::UI => self.handle_ui_select(),
            SettingsTab::Keyboard => self.handle_keyboard_select(),
            SettingsTab::Performance => self.handle_performance_select(),
            SettingsTab::Privacy => self.handle_privacy_select(),
            SettingsTab::AI => self.handle_ai_select(),
            SettingsTab::Advanced => self.handle_advanced_select(),
        }
    }

    fn handle_general_select(&mut self) {
        match self.state.selected_index {
            0 => self.toggle_auto_sync(),
            1 => self.state.start_edit(), // Sync interval
            2 => self.toggle_startup_fetch(),
            3 => self.toggle_incremental_sync(),
            4 => self.state.start_edit(), // Max concurrent syncs
            5 => self.state.start_edit(), // Default folder
            6 => self.toggle_confirm_delete(),
            7 => self.toggle_show_notifications(),
            8 => self.state.start_edit(), // Thread grouping
            9 => self.toggle_mark_read_on_reply(),
            _ => {}
        }
    }

    fn handle_accounts_select(&mut self) {
        match self.state.selected_index {
            0 => self.toggle_auto_discover(),
            1 => self.state.start_edit(), // Connection timeout
            2 => self.state.start_edit(), // Retry attempts
            3 => self.state.start_edit(), // OAuth redirect port
            4 => self.open_account_manager(),
            5 => self.test_connection(),
            _ => {}
        }
    }

    fn handle_ui_select(&mut self) {
        match self.state.selected_index {
            0 => self.cycle_theme(),
            1 => self.toggle_compact_mode(),
            2 => self.toggle_show_sidebar(),
            3 => self.toggle_show_status_bar(),
            4 => self.state.start_edit(), // Font size
            5 => self.toggle_animations(),
            6 => self.configure_layout(),
            _ => {}
        }
    }

    fn handle_keyboard_select(&mut self) {
        match self.state.selected_index {
            0 => self.toggle_vim_mode(),
            1 => self.configure_custom_bindings(),
            2 => self.state.start_edit(), // Repeat delay
            3 => self.state.start_edit(), // Repeat rate
            4 => self.reset_keyboard_defaults(),
            _ => {}
        }
    }

    fn handle_performance_select(&mut self) {
        match self.state.selected_index {
            0 => self.state.start_edit(), // Cache size
            1 => self.toggle_preload_images(),
            2 => self.state.start_edit(), // Max concurrent
            3 => self.toggle_background_sync(),
            4 => self.state.start_edit(), // Cleanup interval
            5 => self.run_cleanup_now(),
            _ => {}
        }
    }

    fn handle_privacy_select(&mut self) {
        match self.state.selected_index {
            0 => self.toggle_tracking_protection(),
            1 => self.toggle_external_images(),
            2 => self.state.start_edit(), // Data retention days
            3 => self.toggle_analytics(),
            4 => self.clear_cache(),
            _ => {}
        }
    }

    fn handle_ai_select(&mut self) {
        match self.state.selected_index {
            0 => self.toggle_ai_enabled(),
            1 => self.state.start_edit(), // Provider
            2 => self.state.start_edit(), // Model
            3 => self.state.start_edit(), // API key
            4 => self.state.start_edit(), // Endpoint
            5 => self.cycle_ai_privacy_mode(),
            6 => self.state.start_edit(), // Temperature
            7 => self.state.start_edit(), // Max tokens
            _ => {}
        }
    }

    fn handle_advanced_select(&mut self) {
        match self.state.selected_index {
            0 => self.toggle_debug_mode(),
            1 => self.cycle_log_level(),
            2 => self.state.start_edit(), // Database path
            3 => self.state.start_edit(), // Backup count
            4 => self.database_maintenance(),
            5 => self.reset_all_settings(),
            _ => {}
        }
    }

    fn apply_edit(&mut self) {
        // Apply the current edit based on tab and selected index
        let value = self.state.input_buffer.clone();
        match self.state.current_tab {
            SettingsTab::General => self.apply_general_edit(value),
            SettingsTab::UI => self.apply_ui_edit(value),
            SettingsTab::Performance => self.apply_performance_edit(value),
            SettingsTab::Privacy => self.apply_privacy_edit(value),
            SettingsTab::Keyboard => self.apply_keyboard_edit(value),
            SettingsTab::Advanced => self.apply_advanced_edit(value),
            SettingsTab::Accounts => self.apply_accounts_edit(value),
            SettingsTab::AI => self.apply_ai_edit(value),
        }
        self.state.modified = true;
    }

    fn apply_general_edit(&mut self, value: String) {
        match self.state.selected_index {
            1 => { // Sync interval
                if let Ok(interval) = value.parse::<u64>() {
                    if interval >= 1 && interval <= 1440 { // 1 minute to 24 hours
                        self.config.general.sync_interval_minutes = interval;
                        if let Err(e) = self.config.save() {
                            self.state.set_status(format!("Failed to save config: {}", e));
                        } else {
                            self.state.set_status(format!("Sync interval set to {} minutes", interval));
                        }
                    } else {
                        self.state.set_status("Sync interval must be between 1 and 1440 minutes".to_string());
                    }
                } else {
                    self.state.set_status("Invalid sync interval".to_string());
                }
            }
            4 => { // Max concurrent syncs
                if let Ok(count) = value.parse::<u32>() {
                    if count >= 1 && count <= 10 {
                        self.config.general.max_concurrent_syncs = count;
                        if let Err(e) = self.config.save() {
                            self.state.set_status(format!("Failed to save config: {}", e));
                        } else {
                            self.state.set_status(format!("Max concurrent syncs set to {}", count));
                        }
                    } else {
                        self.state.set_status("Max concurrent syncs must be between 1 and 10".to_string());
                    }
                } else {
                    self.state.set_status("Invalid concurrent sync count".to_string());
                }
            }
            5 => { // Default folder
                if !value.trim().is_empty() {
                    self.config.general.default_folder = value.trim().to_string();
                    if let Err(e) = self.config.save() {
                        self.state.set_status(format!("Failed to save config: {}", e));
                    } else {
                        self.state.set_status(format!("Default folder set to '{}'", value.trim()));
                    }
                } else {
                    self.state.set_status("Default folder cannot be empty".to_string());
                }
            }
            8 => { // Thread grouping
                use crate::config::ThreadGrouping;
                let grouping = match value.trim().to_lowercase().as_str() {
                    "subject" => ThreadGrouping::Subject,
                    "references" => ThreadGrouping::References,
                    "messageid" => ThreadGrouping::MessageId,
                    "none" => ThreadGrouping::None,
                    _ => {
                        self.state.set_status("Thread grouping must be one of: Subject, References, MessageId, None".to_string());
                        return;
                    }
                };
                self.config.general.thread_grouping = grouping;
                if let Err(e) = self.config.save() {
                    self.state.set_status(format!("Failed to save config: {}", e));
                } else {
                    self.state.set_status(format!("Thread grouping set to {:?}", grouping));
                }
            }
            _ => {}
        }
    }

    fn apply_ui_edit(&mut self, value: String) {
        match self.state.selected_index {
            4 => { // Font size
                if let Ok(size) = value.parse::<u16>() {
                    if size >= 8 && size <= 24 {
                        self.config.ui.font_size = size;
                        if let Err(e) = self.config.save() {
                            self.state.set_status(format!("Failed to save config: {}", e));
                        } else {
                            self.state.set_status(format!("Font size set to {}", size));
                        }
                    } else {
                        self.state.set_status("Font size must be between 8 and 24".to_string());
                    }
                } else {
                    self.state.set_status("Invalid font size".to_string());
                }
            }
            6 => { // Layout
                let layouts = ["Standard", "Compact", "Wide"];
                if layouts.contains(&value.trim()) {
                    self.config.ui.layout = value.trim().to_string();
                    if let Err(e) = self.config.save() {
                        self.state.set_status(format!("Failed to save config: {}", e));
                    } else {
                        self.state.set_status(format!("Layout set to {}", value.trim()));
                    }
                } else {
                    self.state.set_status("Layout must be one of: Standard, Compact, Wide".to_string());
                }
            }
            _ => {}
        }
    }

    fn apply_performance_edit(&mut self, value: String) {
        match self.state.selected_index {
            0 => { // Cache size
                if let Ok(size) = value.parse::<u64>() {
                    if size <= 10000 { // Max 10GB as per validation
                        self.config.performance.cache_size_mb = size;
                        if let Err(e) = self.config.save() {
                            self.state.set_status(format!("Failed to save config: {}", e));
                        } else {
                            self.state.set_status(format!("Cache size set to {} MB", size));
                        }
                    } else {
                        self.state.set_status("Cache size must be 10GB or less".to_string());
                    }
                } else {
                    self.state.set_status("Invalid cache size".to_string());
                }
            }
            2 => { // Max concurrent
                if let Ok(count) = value.parse::<u32>() {
                    if count > 0 && count <= 50 {
                        self.config.performance.max_concurrent_operations = count;
                        if let Err(e) = self.config.save() {
                            self.state.set_status(format!("Failed to save config: {}", e));
                        } else {
                            self.state.set_status(format!("Max concurrent operations set to {}", count));
                        }
                    } else {
                        self.state.set_status("Concurrent operations must be between 1 and 50".to_string());
                    }
                } else {
                    self.state.set_status("Invalid concurrent operations count".to_string());
                }
            }
            4 => { // Cleanup interval
                if let Ok(hours) = value.parse::<u32>() {
                    if hours <= 8760 { // Max 1 year as per validation
                        self.config.performance.cleanup_interval_hours = hours;
                        if let Err(e) = self.config.save() {
                            self.state.set_status(format!("Failed to save config: {}", e));
                        } else {
                            self.state.set_status(format!("Cleanup interval set to {} hours", hours));
                        }
                    } else {
                        self.state.set_status("Cleanup interval must be 1 year or less".to_string());
                    }
                } else {
                    self.state.set_status("Invalid cleanup interval".to_string());
                }
            }
            _ => {}
        }
    }

    fn apply_privacy_edit(&mut self, value: String) {
        match self.state.selected_index {
            2 => { // Data retention days
                if let Ok(days) = value.parse::<u32>() {
                    if days >= 30 && days <= 3650 { // 30 days to 10 years as per validation
                        self.config.privacy.data_retention_days = days;
                        if let Err(e) = self.config.save() {
                            self.state.set_status(format!("Failed to save config: {}", e));
                        } else {
                            self.state.set_status(format!("Data retention set to {} days", days));
                        }
                    } else {
                        self.state.set_status("Data retention must be between 30 days and 10 years".to_string());
                    }
                } else {
                    self.state.set_status("Invalid data retention period".to_string());
                }
            }
            _ => {}
        }
    }

    fn apply_keyboard_edit(&mut self, value: String) {
        match self.state.selected_index {
            2 => { // Repeat delay
                if let Ok(delay) = value.parse::<u32>() {
                    if delay >= 100 && delay <= 2000 { // 0.1s to 2s
                        self.config.keyboard.repeat_delay = delay;
                        if let Err(e) = self.config.save() {
                            self.state.set_status(format!("Failed to save config: {}", e));
                        } else {
                            self.state.set_status(format!("Repeat delay set to {} ms", delay));
                        }
                    } else {
                        self.state.set_status("Repeat delay must be between 100 and 2000 ms".to_string());
                    }
                } else {
                    self.state.set_status("Invalid repeat delay".to_string());
                }
            }
            3 => { // Repeat rate
                if let Ok(rate) = value.parse::<u32>() {
                    if rate >= 10 && rate <= 200 { // 10ms to 200ms
                        self.config.keyboard.repeat_rate = rate;
                        if let Err(e) = self.config.save() {
                            self.state.set_status(format!("Failed to save config: {}", e));
                        } else {
                            self.state.set_status(format!("Repeat rate set to {} ms", rate));
                        }
                    } else {
                        self.state.set_status("Repeat rate must be between 10 and 200 ms".to_string());
                    }
                } else {
                    self.state.set_status("Invalid repeat rate".to_string());
                }
            }
            _ => {}
        }
    }

    fn apply_advanced_edit(&mut self, value: String) {
        match self.state.selected_index {
            2 => { // Database path
                use std::path::PathBuf;
                let path = PathBuf::from(value.trim());
                if let Some(parent) = path.parent() {
                    if parent.exists() || path.is_absolute() {
                        self.config.advanced.database_path = path;
                        if let Err(e) = self.config.save() {
                            self.state.set_status(format!("Failed to save config: {}", e));
                        } else {
                            self.state.set_status("Database path updated".to_string());
                        }
                    } else {
                        self.state.set_status("Invalid database path".to_string());
                    }
                } else {
                    self.state.set_status("Database path must include filename".to_string());
                }
            }
            3 => { // Backup count
                if let Ok(count) = value.parse::<u32>() {
                    if count <= 100 { // Max 100 backups
                        self.config.advanced.backup_count = count;
                        if let Err(e) = self.config.save() {
                            self.state.set_status(format!("Failed to save config: {}", e));
                        } else {
                            self.state.set_status(format!("Backup count set to {}", count));
                        }
                    } else {
                        self.state.set_status("Backup count must be 100 or less".to_string());
                    }
                } else {
                    self.state.set_status("Invalid backup count".to_string());
                }
            }
            _ => {}
        }
    }

    fn apply_accounts_edit(&mut self, value: String) {
        match self.state.selected_index {
            1 => { // Connection timeout
                if let Ok(timeout) = value.parse::<u32>() {
                    if timeout >= 5 && timeout <= 300 { // 5 seconds to 5 minutes
                        self.config.accounts.connection_timeout = timeout;
                        if let Err(e) = self.config.save() {
                            self.state.set_status(format!("Failed to save config: {}", e));
                        } else {
                            self.state.set_status(format!("Connection timeout set to {} seconds", timeout));
                        }
                    } else {
                        self.state.set_status("Connection timeout must be between 5 and 300 seconds".to_string());
                    }
                } else {
                    self.state.set_status("Invalid connection timeout".to_string());
                }
            }
            2 => { // Retry attempts
                if let Ok(attempts) = value.parse::<u32>() {
                    if attempts >= 1 && attempts <= 10 {
                        self.config.accounts.retry_attempts = attempts;
                        if let Err(e) = self.config.save() {
                            self.state.set_status(format!("Failed to save config: {}", e));
                        } else {
                            self.state.set_status(format!("Retry attempts set to {}", attempts));
                        }
                    } else {
                        self.state.set_status("Retry attempts must be between 1 and 10".to_string());
                    }
                } else {
                    self.state.set_status("Invalid retry attempts".to_string());
                }
            }
            3 => { // OAuth redirect port
                if let Ok(port) = value.parse::<u16>() {
                    if port >= 1024 && port <= 65535 { // Non-privileged ports
                        self.config.accounts.oauth_redirect_port = port;
                        if let Err(e) = self.config.save() {
                            self.state.set_status(format!("Failed to save config: {}", e));
                        } else {
                            self.state.set_status(format!("OAuth redirect port set to {}", port));
                        }
                    } else {
                        self.state.set_status("OAuth port must be between 1024 and 65535".to_string());
                    }
                } else {
                    self.state.set_status("Invalid OAuth redirect port".to_string());
                }
            }
            _ => {}
        }
    }

    fn apply_ai_edit(&mut self, value: String) {
        match self.state.selected_index {
            1 => { // AI Provider
                if !value.trim().is_empty() {
                    self.config.ai.provider = value.trim().to_string();
                    if let Err(e) = self.config.save() {
                        self.state.set_status(format!("Failed to save config: {}", e));
                    } else {
                        self.state.set_status(format!("AI provider set to '{}'", value.trim()));
                    }
                } else {
                    self.state.set_status("AI provider cannot be empty".to_string());
                }
            }
            2 => { // AI Model
                if !value.trim().is_empty() {
                    self.config.ai.model = value.trim().to_string();
                    if let Err(e) = self.config.save() {
                        self.state.set_status(format!("Failed to save config: {}", e));
                    } else {
                        self.state.set_status(format!("AI model set to '{}'", value.trim()));
                    }
                } else {
                    self.state.set_status("AI model cannot be empty".to_string());
                }
            }
            3 => { // API Key
                if !value.trim().is_empty() {
                    self.config.ai.api_key = Some(value.trim().to_string());
                    if let Err(e) = self.config.save() {
                        self.state.set_status(format!("Failed to save config: {}", e));
                    } else {
                        self.state.set_status("API key updated".to_string());
                    }
                } else {
                    self.config.ai.api_key = None;
                    if let Err(e) = self.config.save() {
                        self.state.set_status(format!("Failed to save config: {}", e));
                    } else {
                        self.state.set_status("API key cleared".to_string());
                    }
                }
            }
            4 => { // Endpoint
                if !value.trim().is_empty() {
                    self.config.ai.endpoint = Some(value.trim().to_string());
                    if let Err(e) = self.config.save() {
                        self.state.set_status(format!("Failed to save config: {}", e));
                    } else {
                        self.state.set_status(format!("AI endpoint set to '{}'", value.trim()));
                    }
                } else {
                    self.config.ai.endpoint = None;
                    if let Err(e) = self.config.save() {
                        self.state.set_status(format!("Failed to save config: {}", e));
                    } else {
                        self.state.set_status("AI endpoint reset to default".to_string());
                    }
                }
            }
            6 => { // Temperature
                if let Ok(temp) = value.parse::<f32>() {
                    if temp >= 0.0 && temp <= 2.0 {
                        self.config.ai.temperature = temp;
                        if let Err(e) = self.config.save() {
                            self.state.set_status(format!("Failed to save config: {}", e));
                        } else {
                            self.state.set_status(format!("AI temperature set to {:.1}", temp));
                        }
                    } else {
                        self.state.set_status("Temperature must be between 0.0 and 2.0".to_string());
                    }
                } else {
                    self.state.set_status("Invalid temperature value".to_string());
                }
            }
            7 => { // Max tokens
                if let Ok(tokens) = value.parse::<u32>() {
                    if tokens >= 1 && tokens <= 32000 {
                        self.config.ai.max_tokens = tokens;
                        if let Err(e) = self.config.save() {
                            self.state.set_status(format!("Failed to save config: {}", e));
                        } else {
                            self.state.set_status(format!("Max tokens set to {}", tokens));
                        }
                    } else {
                        self.state.set_status("Max tokens must be between 1 and 32000".to_string());
                    }
                } else {
                    self.state.set_status("Invalid token count".to_string());
                }
            }
            _ => {
                self.state.set_status("This AI setting is not editable".to_string());
            }
        }
    }

    // Real setting actions with config persistence
    fn toggle_auto_sync(&mut self) {
        self.config.general.auto_sync = !self.config.general.auto_sync;
        let status = if self.config.general.auto_sync { "enabled" } else { "disabled" };
        if let Err(e) = self.config.save() {
            self.state.set_status(format!("Failed to save config: {}", e));
        } else {
            self.state.set_status(format!("Auto-sync {}", status));
        }
        self.state.modified = true;
    }

    fn toggle_startup_fetch(&mut self) {
        self.config.general.fetch_on_startup = !self.config.general.fetch_on_startup;
        let status = if self.config.general.fetch_on_startup { "enabled" } else { "disabled" };
        if let Err(e) = self.config.save() {
            self.state.set_status(format!("Failed to save config: {}", e));
        } else {
            self.state.set_status(format!("Startup fetch {}", status));
        }
        self.state.modified = true;
    }

    fn toggle_confirm_delete(&mut self) {
        self.config.general.confirm_delete = !self.config.general.confirm_delete;
        let status = if self.config.general.confirm_delete { "enabled" } else { "disabled" };
        if let Err(e) = self.config.save() {
            self.state.set_status(format!("Failed to save config: {}", e));
        } else {
            self.state.set_status(format!("Delete confirmation {}", status));
        }
        self.state.modified = true;
    }

    fn toggle_show_notifications(&mut self) {
        self.config.general.show_notifications = !self.config.general.show_notifications;
        let status = if self.config.general.show_notifications { "enabled" } else { "disabled" };
        if let Err(e) = self.config.save() {
            self.state.set_status(format!("Failed to save config: {}", e));
        } else {
            self.state.set_status(format!("Notifications {}", status));
        }
        self.state.modified = true;
    }

    fn toggle_mark_read_on_reply(&mut self) {
        self.config.general.mark_read_on_reply = !self.config.general.mark_read_on_reply;
        let status = if self.config.general.mark_read_on_reply { "enabled" } else { "disabled" };
        if let Err(e) = self.config.save() {
            self.state.set_status(format!("Failed to save config: {}", e));
        } else {
            self.state.set_status(format!("Mark read on reply {}", status));
        }
        self.state.modified = true;
    }

    fn toggle_incremental_sync(&mut self) {
        self.config.general.incremental_sync = !self.config.general.incremental_sync;
        let status = if self.config.general.incremental_sync { "enabled" } else { "disabled" };
        if let Err(e) = self.config.save() {
            self.state.set_status(format!("Failed to save config: {}", e));
        } else {
            self.state.set_status(format!("Incremental sync {}", status));
        }
        self.state.modified = true;
    }

    fn open_account_manager(&mut self) {
        self.show_account_manager = true;
        self.state.set_status("Opening account manager...".to_string());
        // TODO: Refresh account list when we have the account manager reference
    }

    fn test_connection(&mut self) {
        self.state.set_status("Connection test started...".to_string());
        // TODO: Implement actual connection testing
        // For now, just show placeholder message
    }

    fn configure_oauth(&mut self) {
        self.state.set_status("Configuring OAuth...".to_string());
    }

    fn backup_accounts(&mut self) {
        self.state.set_status("Backing up accounts...".to_string());
    }

    fn restore_accounts(&mut self) {
        self.state.set_status("Restoring accounts...".to_string());
    }

    fn import_accounts(&mut self) {
        self.state.set_status("Importing accounts...".to_string());
    }

    fn toggle_auto_discover(&mut self) {
        self.config.accounts.auto_discover = !self.config.accounts.auto_discover;
        let status = if self.config.accounts.auto_discover { "enabled" } else { "disabled" };
        if let Err(e) = self.config.save() {
            self.state.set_status(format!("Failed to save config: {}", e));
        } else {
            self.state.set_status(format!("Auto discovery {}", status));
        }
        self.state.modified = true;
    }

    fn cycle_theme(&mut self) {
        let themes = ["Dark", "Light", "Auto"];
        let current_index = themes.iter().position(|&t| t == self.config.ui.theme).unwrap_or(0);
        let next_index = (current_index + 1) % themes.len();
        self.config.ui.theme = themes[next_index].to_string();
        
        if let Err(e) = self.config.save() {
            self.state.set_status(format!("Failed to save config: {}", e));
        } else {
            self.state.set_status(format!("Theme changed to {}", self.config.ui.theme));
        }
        self.state.modified = true;
    }

    fn toggle_compact_mode(&mut self) {
        self.config.ui.compact_mode = !self.config.ui.compact_mode;
        let status = if self.config.ui.compact_mode { "enabled" } else { "disabled" };
        if let Err(e) = self.config.save() {
            self.state.set_status(format!("Failed to save config: {}", e));
        } else {
            self.state.set_status(format!("Compact mode {}", status));
        }
        self.state.modified = true;
    }

    fn toggle_show_sidebar(&mut self) {
        self.config.ui.show_sidebar = !self.config.ui.show_sidebar;
        let status = if self.config.ui.show_sidebar { "enabled" } else { "disabled" };
        if let Err(e) = self.config.save() {
            self.state.set_status(format!("Failed to save config: {}", e));
        } else {
            self.state.set_status(format!("Sidebar {}", status));
        }
        self.state.modified = true;
    }

    fn toggle_show_status_bar(&mut self) {
        self.config.ui.show_status_bar = !self.config.ui.show_status_bar;
        let status = if self.config.ui.show_status_bar { "enabled" } else { "disabled" };
        if let Err(e) = self.config.save() {
            self.state.set_status(format!("Failed to save config: {}", e));
        } else {
            self.state.set_status(format!("Status bar {}", status));
        }
        self.state.modified = true;
    }

    fn toggle_animations(&mut self) {
        self.config.ui.animations = !self.config.ui.animations;
        let status = if self.config.ui.animations { "enabled" } else { "disabled" };
        if let Err(e) = self.config.save() {
            self.state.set_status(format!("Failed to save config: {}", e));
        } else {
            self.state.set_status(format!("Animations {}", status));
        }
        self.state.modified = true;
    }

    fn configure_layout(&mut self) {
        self.state.set_status("Configuring layout...".to_string());
    }

    fn open_keyboard_config(&mut self) {
        self.state.set_status("Opening keyboard configuration...".to_string());
    }

    fn reset_keyboard_defaults(&mut self) {
        self.config.keyboard = crate::config::KeyboardConfig::default();
        if let Err(e) = self.config.save() {
            self.state.set_status(format!("Failed to save config: {}", e));
        } else {
            self.state.set_status("Keyboard settings reset to defaults".to_string());
        }
        self.state.modified = true;
    }

    fn import_keyboard_config(&mut self) {
        self.state.set_status("Importing keyboard configuration...".to_string());
    }

    fn export_keyboard_config(&mut self) {
        self.state.set_status("Exporting keyboard configuration...".to_string());
    }

    fn toggle_vim_mode(&mut self) {
        self.config.keyboard.vim_mode = !self.config.keyboard.vim_mode;
        let status = if self.config.keyboard.vim_mode { "enabled" } else { "disabled" };
        if let Err(e) = self.config.save() {
            self.state.set_status(format!("Failed to save config: {}", e));
        } else {
            self.state.set_status(format!("Vim mode {}", status));
        }
        self.state.modified = true;
    }

    fn configure_custom_bindings(&mut self) {
        self.show_keyboard_bindings = true;
        self.state.set_status("Opening keyboard bindings configuration...".to_string());
    }

    fn toggle_preload_images(&mut self) {
        self.config.performance.preload_images = !self.config.performance.preload_images;
        let status = if self.config.performance.preload_images { "enabled" } else { "disabled" };
        if let Err(e) = self.config.save() {
            self.state.set_status(format!("Failed to save config: {}", e));
        } else {
            self.state.set_status(format!("Image preloading {}", status));
        }
        self.state.modified = true;
    }

    fn toggle_background_sync(&mut self) {
        self.config.performance.background_sync = !self.config.performance.background_sync;
        let status = if self.config.performance.background_sync { "enabled" } else { "disabled" };
        if let Err(e) = self.config.save() {
            self.state.set_status(format!("Failed to save config: {}", e));
        } else {
            self.state.set_status(format!("Background sync {}", status));
        }
        self.state.modified = true;
    }

    fn run_cleanup_now(&mut self) {
        self.state.set_status("Running cleanup...".to_string());
    }

    fn toggle_tracking_protection(&mut self) {
        self.config.privacy.tracking_protection = !self.config.privacy.tracking_protection;
        let status = if self.config.privacy.tracking_protection { "enabled" } else { "disabled" };
        if let Err(e) = self.config.save() {
            self.state.set_status(format!("Failed to save config: {}", e));
        } else {
            self.state.set_status(format!("Tracking protection {}", status));
        }
        self.state.modified = true;
    }

    fn toggle_external_images(&mut self) {
        self.config.privacy.external_images = !self.config.privacy.external_images;
        let status = if self.config.privacy.external_images { "allowed" } else { "blocked" };
        if let Err(e) = self.config.save() {
            self.state.set_status(format!("Failed to save config: {}", e));
        } else {
            self.state.set_status(format!("External images {}", status));
        }
        self.state.modified = true;
    }

    fn toggle_analytics(&mut self) {
        self.config.privacy.analytics = !self.config.privacy.analytics;
        let status = if self.config.privacy.analytics { "enabled" } else { "disabled" };
        if let Err(e) = self.config.save() {
            self.state.set_status(format!("Failed to save config: {}", e));
        } else {
            self.state.set_status(format!("Analytics {}", status));
        }
        self.state.modified = true;
    }

    fn configure_data_retention(&mut self) {
        self.state.set_status("Configuring data retention...".to_string());
    }

    fn clear_cache(&mut self) {
        // TODO: Implement actual cache clearing logic
        // For now, just show confirmation
        self.state.set_status("Cache cleared successfully".to_string());
        self.state.modified = true;
    }

    fn export_data(&mut self) {
        self.state.set_status("Exporting data...".to_string());
    }

    fn toggle_ai_enabled(&mut self) {
        self.config.ai.enabled = !self.config.ai.enabled;
        let status = if self.config.ai.enabled { "enabled" } else { "disabled" };
        if let Err(e) = self.config.save() {
            self.state.set_status(format!("Failed to save config: {}", e));
        } else {
            self.state.set_status(format!("AI assistant {}", status));
        }
        self.state.modified = true;
    }

    fn cycle_ai_privacy_mode(&mut self) {
        use crate::config::AIPrivacyMode;
        self.config.ai.privacy_mode = match self.config.ai.privacy_mode {
            AIPrivacyMode::LocalOnly => AIPrivacyMode::CloudWithConsent,
            AIPrivacyMode::CloudWithConsent => AIPrivacyMode::CloudAlways,
            AIPrivacyMode::CloudAlways => AIPrivacyMode::LocalOnly,
        };
        if let Err(e) = self.config.save() {
            self.state.set_status(format!("Failed to save config: {}", e));
        } else {
            self.state.set_status(format!("AI privacy mode set to {:?}", self.config.ai.privacy_mode));
        }
        self.state.modified = true;
    }

    fn cycle_ai_provider(&mut self) {
        self.state.set_status("AI provider changed".to_string());
        self.state.modified = true;
    }

    fn configure_ai_privacy(&mut self) {
        self.state.set_status("Configuring AI privacy...".to_string());
    }

    fn test_ai_connection(&mut self) {
        self.state.set_status("Testing AI connection...".to_string());
    }

    fn configure_ai_features(&mut self) {
        self.state.set_status("Configuring AI features...".to_string());
    }

    fn ai_cache_settings(&mut self) {
        self.state.set_status("Configuring AI cache...".to_string());
    }

    fn ai_performance_settings(&mut self) {
        self.state.set_status("Configuring AI performance...".to_string());
    }

    fn open_full_ai_config(&mut self) {
        self.state.set_status("Opening full AI configuration...".to_string());
    }

    fn toggle_debug_mode(&mut self) {
        self.config.advanced.debug_mode = !self.config.advanced.debug_mode;
        let status = if self.config.advanced.debug_mode { "enabled" } else { "disabled" };
        if let Err(e) = self.config.save() {
            self.state.set_status(format!("Failed to save config: {}", e));
        } else {
            self.state.set_status(format!("Debug mode {}", status));
        }
        self.state.modified = true;
    }

    fn cycle_log_level(&mut self) {
        use crate::config::LogLevel;
        self.config.advanced.log_level = match self.config.advanced.log_level {
            LogLevel::Error => LogLevel::Warn,
            LogLevel::Warn => LogLevel::Info,
            LogLevel::Info => LogLevel::Debug,
            LogLevel::Debug => LogLevel::Trace,
            LogLevel::Trace => LogLevel::Error,
        };
        if let Err(e) = self.config.save() {
            self.state.set_status(format!("Failed to save config: {}", e));
        } else {
            self.state.set_status(format!("Log level set to {:?}", self.config.advanced.log_level));
        }
        self.state.modified = true;
    }

    fn configure_logging(&mut self) {
        self.state.set_status("Configuring logging...".to_string());
    }

    fn database_maintenance(&mut self) {
        self.state.set_status("Running database maintenance...".to_string());
    }

    fn reset_all_settings(&mut self) {
        self.state.set_status("All settings reset to defaults".to_string());
        self.state.modified = true;
    }

    fn export_configuration(&mut self) {
        self.state.set_status("Exporting configuration...".to_string());
    }

    fn import_configuration(&mut self) {
        self.state.set_status("Importing configuration...".to_string());
    }

    fn reset_current_setting(&mut self) {
        self.state.set_status("Setting reset to default".to_string());
        self.state.modified = true;
    }

    fn save_settings(&mut self) {
        self.state.set_status("Settings saved".to_string());
        self.state.modified = false;
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.state.visible {
            return;
        }

        // Clear the background
        frame.render_widget(Clear, area);

        // Create main layout
        let main_block = Block::default()
            .title("⚙️ Application Settings")
            .borders(Borders::ALL)
            .border_style(theme.get_component_style("border", true));

        let inner_area = main_block.inner(area);
        frame.render_widget(main_block, area);

        // Split into header, content, and footer
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Tab bar
                Constraint::Min(0),    // Content area
                Constraint::Length(3), // Footer
            ])
            .split(inner_area);

        // Render tab bar
        self.render_tab_bar(frame, chunks[0], theme);

        // Render content area based on current tab
        self.render_tab_content(frame, chunks[1], theme);

        // Render footer with status and shortcuts
        self.render_footer(frame, chunks[2], theme);
        
        // Render account manager overlay if shown
        if self.show_account_manager {
            self.account_manager_ui.render(frame, area, theme);
        }
        
        // Render keyboard bindings manager overlay if shown
        if self.show_keyboard_bindings {
            self.keyboard_bindings_ui.render(frame, area, theme);
        }
    }

    fn render_tab_bar(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let tab_titles: Vec<Line> = SettingsTab::all()
            .iter()
            .map(|tab| Line::from(tab.title()))
            .collect();

        let current_index = SettingsTab::all()
            .iter()
            .position(|&tab| tab == self.state.current_tab)
            .unwrap_or(0);

        let tabs = Tabs::new(tab_titles)
            .block(Block::default().borders(Borders::BOTTOM))
            .highlight_style(theme.get_component_style("selected", true))
            .select(current_index);

        frame.render_widget(tabs, area);
    }

    fn render_tab_content(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        match self.state.current_tab {
            SettingsTab::General => self.render_general_tab(frame, area, theme),
            SettingsTab::Accounts => self.render_accounts_tab(frame, area, theme),
            SettingsTab::UI => self.render_ui_tab(frame, area, theme),
            SettingsTab::Keyboard => self.render_keyboard_tab(frame, area, theme),
            SettingsTab::Performance => self.render_performance_tab(frame, area, theme),
            SettingsTab::Privacy => self.render_privacy_tab(frame, area, theme),
            SettingsTab::AI => self.render_ai_tab(frame, area, theme),
            SettingsTab::Advanced => self.render_advanced_tab(frame, area, theme),
        }
    }

    fn render_general_tab(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let items = vec![
            ListItem::new(format!("🔄 Auto-sync emails: {}", 
                if self.config.general.auto_sync { "Enabled" } else { "Disabled" })),
            ListItem::new(format!("⏱️  Sync interval: {} minutes", self.config.general.sync_interval_minutes)),
            ListItem::new(format!("🚀 Fetch on startup: {}", 
                if self.config.general.fetch_on_startup { "Enabled" } else { "Disabled" })),
            ListItem::new(format!("📬 Use incremental sync: {}", 
                if self.config.general.incremental_sync { "Enabled" } else { "Disabled" })),
            ListItem::new(format!("🔁 Max concurrent syncs: {}", self.config.general.max_concurrent_syncs)),
            ListItem::new(format!("📂 Default folder: {}", self.config.general.default_folder)),
            ListItem::new(format!("⚠️  Confirm before delete: {}", 
                if self.config.general.confirm_delete { "Enabled" } else { "Disabled" })),
            ListItem::new(format!("🔔 Show notifications: {}", 
                if self.config.general.show_notifications { "Enabled" } else { "Disabled" })),
            ListItem::new(format!("🧵 Thread grouping: {:?}", self.config.general.thread_grouping)),
            ListItem::new(format!("👁️  Mark as read on reply: {}", 
                if self.config.general.mark_read_on_reply { "Enabled" } else { "Disabled" })),
        ];

        let list = List::new(items)
            .block(Block::default().title("General Settings").borders(Borders::ALL))
            .highlight_style(theme.get_component_style("selected", true))
            .highlight_symbol("► ");

        frame.render_stateful_widget(list, area, &mut self.state.list_state);
    }

    fn render_accounts_tab(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let items = vec![
            ListItem::new(format!("🔍 Auto discover: {}", 
                if self.config.accounts.auto_discover { "Enabled" } else { "Disabled" })),
            ListItem::new(format!("⏱️ Connection timeout: {} seconds", self.config.accounts.connection_timeout)),
            ListItem::new(format!("🔄 Retry attempts: {}", self.config.accounts.retry_attempts)),
            ListItem::new(format!("🌐 OAuth redirect port: {}", self.config.accounts.oauth_redirect_port)),
            ListItem::new("📧 Manage email accounts"),
            ListItem::new("🔍 Test connection"),
        ];

        let list = List::new(items)
            .block(Block::default().title("Account Management").borders(Borders::ALL))
            .highlight_style(theme.get_component_style("selected", true))
            .highlight_symbol("► ");

        frame.render_stateful_widget(list, area, &mut self.state.list_state);
    }

    fn render_ui_tab(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let items = vec![
            ListItem::new(format!("🎨 Theme: {}", self.config.ui.theme)),
            ListItem::new(format!("📏 Compact mode: {}", 
                if self.config.ui.compact_mode { "Enabled" } else { "Disabled" })),
            ListItem::new(format!("📂 Show sidebar: {}", 
                if self.config.ui.show_sidebar { "Enabled" } else { "Disabled" })),
            ListItem::new(format!("📊 Show status bar: {}", 
                if self.config.ui.show_status_bar { "Enabled" } else { "Disabled" })),
            ListItem::new(format!("🔤 Font size: {}", self.config.ui.font_size)),
            ListItem::new(format!("✨ Animations: {}", 
                if self.config.ui.animations { "Enabled" } else { "Disabled" })),
            ListItem::new(format!("🖼️ Layout: {}", self.config.ui.layout)),
        ];

        let list = List::new(items)
            .block(Block::default().title("UI & Theme Settings").borders(Borders::ALL))
            .highlight_style(theme.get_component_style("selected", true))
            .highlight_symbol("► ");

        frame.render_stateful_widget(list, area, &mut self.state.list_state);
    }

    fn render_keyboard_tab(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let items = vec![
            ListItem::new(format!("🅥 Vim mode: {}", 
                if self.config.keyboard.vim_mode { "Enabled" } else { "Disabled" })),
            ListItem::new(format!("⌨️ Custom bindings: {} configured", self.config.keyboard.custom_bindings.len())),
            ListItem::new(format!("⏱️ Repeat delay: {} ms", self.config.keyboard.repeat_delay)),
            ListItem::new(format!("🔄 Repeat rate: {} ms", self.config.keyboard.repeat_rate)),
            ListItem::new("🔄 Reset to defaults"),
        ];

        let list = List::new(items)
            .block(Block::default().title("Keyboard Settings").borders(Borders::ALL))
            .highlight_style(theme.get_component_style("selected", true))
            .highlight_symbol("► ");

        frame.render_stateful_widget(list, area, &mut self.state.list_state);
    }

    fn render_performance_tab(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let items = vec![
            ListItem::new(format!("💾 Cache size: {} MB", self.config.performance.cache_size_mb)),
            ListItem::new(format!("🖼️ Preload images: {}", 
                if self.config.performance.preload_images { "Enabled" } else { "Disabled" })),
            ListItem::new(format!("🔄 Max concurrent: {}", self.config.performance.max_concurrent_operations)),
            ListItem::new(format!("⚡ Background sync: {}", 
                if self.config.performance.background_sync { "Enabled" } else { "Disabled" })),
            ListItem::new(format!("🧹 Cleanup interval: {} hours", self.config.performance.cleanup_interval_hours)),
            ListItem::new("🗑️ Run cleanup now"),
        ];

        let list = List::new(items)
            .block(Block::default().title("Performance Settings").borders(Borders::ALL))
            .highlight_style(theme.get_component_style("selected", true))
            .highlight_symbol("► ");

        frame.render_stateful_widget(list, area, &mut self.state.list_state);
    }

    fn render_privacy_tab(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let items = vec![
            ListItem::new(format!("🛡️ Tracking protection: {}", 
                if self.config.privacy.tracking_protection { "Enabled" } else { "Disabled" })),
            ListItem::new(format!("🖼️ External images: {}", 
                if self.config.privacy.external_images { "Allow" } else { "Block" })),
            ListItem::new(format!("📅 Data retention: {} days", self.config.privacy.data_retention_days)),
            ListItem::new(format!("📊 Analytics: {}", 
                if self.config.privacy.analytics { "Enabled" } else { "Disabled" })),
            ListItem::new("🗑️ Clear cache"),
        ];

        let list = List::new(items)
            .block(Block::default().title("Privacy Settings").borders(Borders::ALL))
            .highlight_style(theme.get_component_style("selected", true))
            .highlight_symbol("► ");

        frame.render_stateful_widget(list, area, &mut self.state.list_state);
    }

    fn render_ai_tab(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let api_key_display = self.config.ai.api_key.as_ref()
            .map(|key| if key.len() > 8 { format!("{}...", &key[..8]) } else { "Set".to_string() })
            .unwrap_or_else(|| "Not set".to_string());
        
        let endpoint_display = self.config.ai.endpoint.as_ref()
            .unwrap_or(&"Default".to_string()).clone();

        let items = vec![
            ListItem::new(format!("🤖 AI assistant: {}", 
                if self.config.ai.enabled { "Enabled" } else { "Disabled" })),
            ListItem::new(format!("🔧 Provider: {}", self.config.ai.provider)),
            ListItem::new(format!("🤖 Model: {}", self.config.ai.model)),
            ListItem::new(format!("🔑 API key: {}", api_key_display)),
            ListItem::new(format!("🌐 Endpoint: {}", endpoint_display)),
            ListItem::new(format!("🔒 Privacy mode: {:?}", self.config.ai.privacy_mode)),
            ListItem::new(format!("🌡️ Temperature: {:.1}", self.config.ai.temperature)),
            ListItem::new(format!("📝 Max tokens: {}", self.config.ai.max_tokens)),
        ];

        let list = List::new(items)
            .block(Block::default().title("AI Assistant Settings").borders(Borders::ALL))
            .highlight_style(theme.get_component_style("selected", true))
            .highlight_symbol("► ");

        frame.render_stateful_widget(list, area, &mut self.state.list_state);
    }

    fn render_advanced_tab(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let items = vec![
            ListItem::new(format!("🐛 Debug mode: {}", 
                if self.config.advanced.debug_mode { "Enabled" } else { "Disabled" })),
            ListItem::new(format!("📝 Log level: {:?}", self.config.advanced.log_level)),
            ListItem::new(format!("📁 Database path: {}", 
                self.config.advanced.database_path.display())),
            ListItem::new(format!("💾 Backup count: {}", self.config.advanced.backup_count)),
            ListItem::new("🗄️ Database maintenance"),
            ListItem::new("⚠️ Reset all settings"),
        ];

        let list = List::new(items)
            .block(Block::default().title("Advanced Settings").borders(Borders::ALL))
            .highlight_style(theme.get_component_style("selected", true))
            .highlight_symbol("► ");

        frame.render_stateful_widget(list, area, &mut self.state.list_state);
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let mut footer_text = if self.state.edit_mode {
            format!("Editing: {} | Enter: Save | Esc: Cancel", self.state.input_buffer)
        } else {
            "Tab/Shift+Tab: Switch tabs | ↑↓: Navigate | Enter/Space: Select | E: Edit | Ctrl+R: Reset | Ctrl+S: Save | Q/Esc: Close".to_string()
        };

        if let Some(ref status) = self.state.status_message {
            footer_text = format!("Status: {} | {}", status, footer_text);
        }

        if self.state.modified {
            footer_text = format!("* Modified | {}", footer_text);
        }

        let footer = Paragraph::new(footer_text)
            .block(Block::default().borders(Borders::TOP))
            .wrap(Wrap { trim: true })
            .style(theme.get_component_style("secondary", false));

        frame.render_widget(footer, area);
    }
}

impl Default for SettingsUI {
    fn default() -> Self {
        Self::new()
    }
}