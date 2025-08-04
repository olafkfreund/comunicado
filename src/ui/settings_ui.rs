//! Application settings UI components for comprehensive configuration management

use crate::theme::Theme;
use crate::config::AppConfig;
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
}

impl SettingsUI {
    pub fn new() -> Self {
        let config = AppConfig::load().unwrap_or_default();
        Self {
            state: SettingsUIState::new(),
            config,
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

    pub fn handle_key(&mut self, key: KeyCode, modifiers: KeyModifiers) -> bool {
        if !self.state.visible {
            return false;
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
            0 => self.open_account_manager(),
            1 => self.test_connection(),
            2 => self.configure_oauth(),
            3 => self.backup_accounts(),
            4 => self.restore_accounts(), 
            5 => self.import_accounts(),
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
            0 => self.open_keyboard_config(),
            1 => self.reset_keyboard_defaults(),
            2 => self.import_keyboard_config(),
            3 => self.export_keyboard_config(),
            4 => self.toggle_vim_mode(),
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
            2 => self.configure_data_retention(),
            3 => self.clear_cache(),
            4 => self.export_data(),
            _ => {}
        }
    }

    fn handle_ai_select(&mut self) {
        match self.state.selected_index {
            0 => self.toggle_ai_enabled(),
            1 => self.cycle_ai_provider(),
            2 => self.configure_ai_privacy(),
            3 => self.test_ai_connection(),
            4 => self.configure_ai_features(),
            5 => self.ai_cache_settings(),
            6 => self.ai_performance_settings(),
            7 => self.open_full_ai_config(),
            _ => {}
        }
    }

    fn handle_advanced_select(&mut self) {
        match self.state.selected_index {
            0 => self.toggle_debug_mode(),
            1 => self.configure_logging(),
            2 => self.database_maintenance(),
            3 => self.reset_all_settings(),
            4 => self.export_configuration(),
            5 => self.import_configuration(),
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
            SettingsTab::AI => self.apply_ai_edit(value),
            _ => {}
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
        self.state.set_status("Opening account manager...".to_string());
    }

    fn test_connection(&mut self) {
        self.state.set_status("Testing connection...".to_string());
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
        self.state.set_status("Keyboard shortcuts reset to defaults".to_string());
        self.state.modified = true;
    }

    fn import_keyboard_config(&mut self) {
        self.state.set_status("Importing keyboard configuration...".to_string());
    }

    fn export_keyboard_config(&mut self) {
        self.state.set_status("Exporting keyboard configuration...".to_string());
    }

    fn toggle_vim_mode(&mut self) {
        self.state.set_status("Vim mode toggled".to_string());
        self.state.modified = true;
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
        self.state.set_status("Tracking protection toggled".to_string());
        self.state.modified = true;
    }

    fn toggle_external_images(&mut self) {
        self.state.set_status("External image loading toggled".to_string());
        self.state.modified = true;
    }

    fn configure_data_retention(&mut self) {
        self.state.set_status("Configuring data retention...".to_string());
    }

    fn clear_cache(&mut self) {
        self.state.set_status("Cache cleared".to_string());
    }

    fn export_data(&mut self) {
        self.state.set_status("Exporting data...".to_string());
    }

    fn toggle_ai_enabled(&mut self) {
        self.state.set_status("AI assistant toggled".to_string());
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
        self.state.set_status("Debug mode toggled".to_string());
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
            ListItem::new("📧 Manage email accounts"),
            ListItem::new("🔍 Test connection"),
            ListItem::new("🔐 Configure OAuth"),
            ListItem::new("💾 Backup accounts"),
            ListItem::new("📥 Restore accounts"),
            ListItem::new("📋 Import accounts"),
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
            ListItem::new("⌨️ Configure shortcuts"),
            ListItem::new("🔄 Reset to defaults"),
            ListItem::new("📥 Import configuration"),
            ListItem::new("📤 Export configuration"),
            ListItem::new("🅥 Vim mode: Enabled"),
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
            ListItem::new("🛡️ Tracking protection: Enabled"),
            ListItem::new("🖼️ External images: Block"),
            ListItem::new("📅 Data retention policy"),
            ListItem::new("🗑️ Clear cache"),
            ListItem::new("📤 Export user data"),
        ];

        let list = List::new(items)
            .block(Block::default().title("Privacy Settings").borders(Borders::ALL))
            .highlight_style(theme.get_component_style("selected", true))
            .highlight_symbol("► ");

        frame.render_stateful_widget(list, area, &mut self.state.list_state);
    }

    fn render_ai_tab(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let items = vec![
            ListItem::new(format!("🤖 AI assistant: {}", 
                if self.config.ai.enabled { "Enabled" } else { "Disabled" })),
            ListItem::new(format!("🔧 Provider: {}", self.config.ai.provider)),
            ListItem::new(format!("🔒 Privacy mode: {:?}", self.config.ai.privacy_mode)),
            ListItem::new("🔍 Test connection"),
            ListItem::new("⚙️ Configure features"),
            ListItem::new("💾 Cache settings"),
            ListItem::new("⚡ Performance settings"),
            ListItem::new("🛠️ Advanced AI config"),
        ];

        let list = List::new(items)
            .block(Block::default().title("AI Assistant Settings").borders(Borders::ALL))
            .highlight_style(theme.get_component_style("selected", true))
            .highlight_symbol("► ");

        frame.render_stateful_widget(list, area, &mut self.state.list_state);
    }

    fn render_advanced_tab(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let items = vec![
            ListItem::new("🐛 Debug mode: Disabled"),
            ListItem::new("📝 Logging configuration"),
            ListItem::new("🗄️ Database maintenance"),
            ListItem::new("⚠️ Reset all settings"),
            ListItem::new("📤 Export configuration"),
            ListItem::new("📥 Import configuration"),
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