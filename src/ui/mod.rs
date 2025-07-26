pub mod folder_tree;
pub mod message_list;
pub mod content_preview;
pub mod layout;
pub mod status_bar;
pub mod sync_progress;
pub mod compose;
pub mod account_switcher;

use ratatui::{
    layout::Rect,
    widgets::{Block, Borders},
    Frame,
};
use crate::theme::{Theme, ThemeManager};
use crate::email::{EmailDatabase, EmailNotificationManager, UIEmailUpdater, EmailNotification, sync_engine::SyncProgress};
use chrono::Duration as ChronoDuration;
use std::sync::Arc;

use self::{
    folder_tree::FolderTree,
    message_list::MessageList,
    content_preview::ContentPreview,
    layout::AppLayout,
    status_bar::{StatusBar, EmailStatusSegment, CalendarStatusSegment, SystemInfoSegment, NavigationHintsSegment, SyncStatus},
    sync_progress::SyncProgressOverlay,
    compose::ComposeUI,
    account_switcher::AccountSwitcher,
};

// Re-export compose types for external use
pub use compose::{ComposeAction, EmailComposeData};

// Re-export account switcher types for external use
pub use account_switcher::{AccountItem, AccountSyncStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPane {
    AccountSwitcher,
    FolderTree,
    MessageList,
    ContentPreview,
    Compose,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UIMode {
    Normal,
    Compose,
}

pub struct UI {
    focused_pane: FocusedPane,
    account_switcher: AccountSwitcher,
    folder_tree: FolderTree,
    message_list: MessageList,
    content_preview: ContentPreview,
    layout: AppLayout,
    theme_manager: ThemeManager,
    status_bar: StatusBar,
    email_updater: Option<UIEmailUpdater>,
    sync_progress_overlay: SyncProgressOverlay,
    mode: UIMode,
    compose_ui: Option<ComposeUI>,
}

impl UI {
    pub fn new() -> Self {
        let mut ui = Self {
            focused_pane: FocusedPane::AccountSwitcher,
            account_switcher: AccountSwitcher::new(),
            folder_tree: FolderTree::new(),
            message_list: MessageList::new(),
            content_preview: ContentPreview::new(),
            layout: AppLayout::new(),
            theme_manager: ThemeManager::new(),
            status_bar: StatusBar::default(),
            email_updater: None,
            sync_progress_overlay: SyncProgressOverlay::new(),
            mode: UIMode::Normal,
            compose_ui: None,
        };
        
        // Initialize status bar with default segments
        ui.initialize_status_bar();
        ui
    }
    
    fn initialize_status_bar(&mut self) {
        // Add email status segment
        let email_segment = EmailStatusSegment {
            unread_count: 5, // Sample data
            total_count: 127,
            sync_status: SyncStatus::Online,
        };
        self.status_bar.add_segment("email".to_string(), email_segment);
        
        // Add calendar segment
        let calendar_segment = CalendarStatusSegment {
            next_event: Some("Team Meeting".to_string()),
            events_today: 3,
        };
        self.status_bar.add_segment("calendar".to_string(), calendar_segment);
        
        // Add system info segment
        let system_segment = SystemInfoSegment {
            current_time: "14:30".to_string(),
            active_account: "work@example.com".to_string(),
        };
        self.status_bar.add_segment("system".to_string(), system_segment);
        
        // Add navigation hints
        let nav_segment = NavigationHintsSegment {
            current_pane: "Folders".to_string(),
            available_shortcuts: vec![
                ("Tab".to_string(), "Switch".to_string()),
                ("q".to_string(), "Quit".to_string()),
                ("h/j/k/l".to_string(), "Navigate".to_string()),
            ],
        };
        self.status_bar.add_segment("navigation".to_string(), nav_segment);
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let size = frame.size();
        
        match self.mode {
            UIMode::Normal => {
                let chunks = self.layout.calculate_layout(size);

                // Render each pane with focus styling
                self.render_account_switcher(frame, chunks[0]);
                self.render_folder_tree(frame, chunks[1]);
                self.render_message_list(frame, chunks[2]);
                self.render_content_preview(frame, chunks[3]);
                
                // Render the status bar
                if chunks.len() > 4 {
                    self.render_status_bar(frame, chunks[4]);
                }
                
                // Render sync progress overlay (on top of everything)
                if self.sync_progress_overlay.is_visible() {
                    let theme = self.theme_manager.current_theme();
                    self.sync_progress_overlay.render(frame, size, theme);
                }
            }
            UIMode::Compose => {
                // Render compose UI in full screen
                if let Some(ref mut compose_ui) = self.compose_ui {
                    let theme = self.theme_manager.current_theme();
                    compose_ui.render(frame, size, theme);
                }
            }
        }
    }

    fn render_account_switcher(&mut self, frame: &mut Frame, area: Rect) {
        let is_focused = matches!(self.focused_pane, FocusedPane::AccountSwitcher);
        let theme = self.theme_manager.current_theme();
        
        let border_style = theme.get_component_style("border", is_focused);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style);

        self.account_switcher.render(frame, area, block, is_focused, theme);
    }

    fn render_folder_tree(&self, frame: &mut Frame, area: Rect) {
        let is_focused = matches!(self.focused_pane, FocusedPane::FolderTree);
        let theme = self.theme_manager.current_theme();
        
        let border_style = theme.get_component_style("border", is_focused);
        let block = Block::default()
            .title("Folders")
            .borders(Borders::ALL)
            .border_style(border_style);

        self.folder_tree.render(frame, area, block, is_focused, theme);
    }

    fn render_message_list(&self, frame: &mut Frame, area: Rect) {
        let is_focused = matches!(self.focused_pane, FocusedPane::MessageList);
        let theme = self.theme_manager.current_theme();
        
        let border_style = theme.get_component_style("border", is_focused);
        let block = Block::default()
            .title("Messages")
            .borders(Borders::ALL)
            .border_style(border_style);

        self.message_list.render(frame, area, block, is_focused, theme);
    }

    fn render_content_preview(&mut self, frame: &mut Frame, area: Rect) {
        let is_focused = matches!(self.focused_pane, FocusedPane::ContentPreview);
        let theme = self.theme_manager.current_theme();
        
        let border_style = theme.get_component_style("border", is_focused);
        let block = Block::default()
            .title("Content")
            .borders(Borders::ALL)
            .border_style(border_style);

        self.content_preview.render(frame, area, block, is_focused, theme);
    }

    fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        let theme = self.theme_manager.current_theme();
        self.status_bar.render(frame, area, theme);
    }

    // Navigation methods
    pub fn next_pane(&mut self) {
        if matches!(self.mode, UIMode::Compose) {
            return; // No pane switching in compose mode
        }
        
        self.focused_pane = match self.focused_pane {
            FocusedPane::AccountSwitcher => FocusedPane::FolderTree,
            FocusedPane::FolderTree => FocusedPane::MessageList,
            FocusedPane::MessageList => FocusedPane::ContentPreview,
            FocusedPane::ContentPreview => FocusedPane::AccountSwitcher,
            FocusedPane::Compose => FocusedPane::Compose, // Stay in compose
        };
        self.update_navigation_hints();
    }

    pub fn previous_pane(&mut self) {
        if matches!(self.mode, UIMode::Compose) {
            return; // No pane switching in compose mode
        }
        
        self.focused_pane = match self.focused_pane {
            FocusedPane::AccountSwitcher => FocusedPane::ContentPreview,
            FocusedPane::FolderTree => FocusedPane::AccountSwitcher,
            FocusedPane::MessageList => FocusedPane::FolderTree,
            FocusedPane::ContentPreview => FocusedPane::MessageList,
            FocusedPane::Compose => FocusedPane::Compose, // Stay in compose
        };
        self.update_navigation_hints();
    }

    pub fn focused_pane(&self) -> FocusedPane {
        self.focused_pane
    }

    // Accessors for pane components
    pub fn folder_tree(&self) -> &FolderTree {
        &self.folder_tree
    }
    
    pub fn folder_tree_mut(&mut self) -> &mut FolderTree {
        &mut self.folder_tree
    }

    pub fn message_list_mut(&mut self) -> &mut MessageList {
        &mut self.message_list
    }

    pub fn content_preview_mut(&mut self) -> &mut ContentPreview {
        &mut self.content_preview
    }
    
    pub fn account_switcher(&self) -> &AccountSwitcher {
        &self.account_switcher
    }
    
    pub fn account_switcher_mut(&mut self) -> &mut AccountSwitcher {
        &mut self.account_switcher
    }

    // Theme management methods
    pub fn theme_manager(&self) -> &ThemeManager {
        &self.theme_manager
    }

    pub fn theme_manager_mut(&mut self) -> &mut ThemeManager {
        &mut self.theme_manager
    }

    pub fn set_theme(&mut self, theme_name: &str) -> Result<(), String> {
        self.theme_manager.set_theme(theme_name)
    }

    pub fn current_theme(&self) -> &Theme {
        self.theme_manager.current_theme()
    }

    // Status bar management methods
    pub fn update_navigation_hints(&mut self) {
        let current_pane_name = match self.mode {
            UIMode::Normal => match self.focused_pane {
                FocusedPane::AccountSwitcher => "Accounts",
                FocusedPane::FolderTree => "Folders",
                FocusedPane::MessageList => "Messages", 
                FocusedPane::ContentPreview => "Content",
                FocusedPane::Compose => "Compose", // Shouldn't happen in normal mode
            },
            UIMode::Compose => "Compose Email",
        };
        
        let nav_segment = NavigationHintsSegment {
            current_pane: current_pane_name.to_string(),
            available_shortcuts: self.get_current_shortcuts(),
        };
        
        self.status_bar.add_segment("navigation".to_string(), nav_segment);
    }
    
    fn get_current_shortcuts(&self) -> Vec<(String, String)> {
        match self.mode {
            UIMode::Normal => match self.focused_pane {
                FocusedPane::AccountSwitcher => vec![
                    ("Tab".to_string(), "Switch".to_string()),
                    ("j/k".to_string(), "Navigate".to_string()),
                    ("Enter".to_string(), "Select".to_string()),
                    ("Space".to_string(), "Expand".to_string()),
                    ("c".to_string(), "Compose".to_string()),
                ],
                FocusedPane::FolderTree => vec![
                    ("Tab".to_string(), "Switch".to_string()),
                    ("j/k".to_string(), "Navigate".to_string()),
                    ("l".to_string(), "Expand".to_string()),
                    ("h".to_string(), "Collapse".to_string()),
                    ("c".to_string(), "Compose".to_string()),
                ],
                FocusedPane::MessageList => vec![
                    ("Tab".to_string(), "Switch".to_string()),
                    ("j/k".to_string(), "Navigate".to_string()),
                    ("Enter".to_string(), "Open".to_string()),
                    ("c".to_string(), "Compose".to_string()),
                    ("r".to_string(), "Reply".to_string()),
                    ("f".to_string(), "Forward".to_string()),
                ],
                FocusedPane::ContentPreview => vec![
                    ("Tab".to_string(), "Switch".to_string()),
                    ("j/k".to_string(), "Scroll".to_string()),
                    ("v".to_string(), "View Mode".to_string()),
                    ("H".to_string(), "Headers".to_string()),
                    ("Home/End".to_string(), "Jump".to_string()),
                    ("r".to_string(), "Reply".to_string()),
                    ("f".to_string(), "Forward".to_string()),
                ],
                _ => vec![],
            },
            UIMode::Compose => vec![
                ("Tab".to_string(), "Next Field".to_string()),
                ("F1".to_string(), "Send".to_string()),
                ("F2".to_string(), "Save Draft".to_string()),
                ("@".to_string(), "Contact Lookup".to_string()),
                ("Esc".to_string(), "Cancel".to_string()),
            ],
        }
    }
    
    pub fn update_email_status(&mut self, unread: usize, total: usize, sync_status: SyncStatus) {
        let email_segment = EmailStatusSegment {
            unread_count: unread,
            total_count: total,
            sync_status,
        };
        self.status_bar.add_segment("email".to_string(), email_segment);
    }
    
    pub fn update_system_time(&mut self, time: String) {
        // Get the current system segment and update only the time
        let system_segment = SystemInfoSegment {
            current_time: time,
            active_account: "work@example.com".to_string(), // TODO: Get from actual account
        };
        self.status_bar.add_segment("system".to_string(), system_segment);
    }
    
    /// Set the database for email operations
    pub fn set_database(&mut self, database: Arc<EmailDatabase>) {
        self.message_list.set_database(database.clone());
        self.content_preview.set_database(database);
    }
    
    /// Set the notification manager for real-time updates
    pub fn set_notification_manager(&mut self, notification_manager: Arc<EmailNotificationManager>) {
        self.email_updater = Some(UIEmailUpdater::new(&notification_manager));
    }
    
    /// Load messages for a specific account and folder
    pub async fn load_messages(&mut self, account_id: String, folder_name: String) -> Result<(), Box<dyn std::error::Error>> {
        self.message_list.load_messages(account_id.clone(), folder_name.clone()).await?;
        
        // Subscribe to notifications for this folder
        if let Some(ref mut updater) = self.email_updater {
            updater.subscribe_to_folder(account_id, folder_name);
        }
        
        // Update email status after loading
        let message_count = self.message_list.messages().len();
        let unread_count = self.message_list.messages().iter()
            .filter(|msg| !msg.is_read)
            .count();
            
        self.update_email_status(unread_count, message_count, SyncStatus::Online);
        Ok(())
    }
    
    /// Set available accounts in the account switcher
    pub fn set_accounts(&mut self, accounts: Vec<AccountItem>) {
        self.account_switcher.set_accounts(accounts);
    }
    
    /// Add a new account to the account switcher
    pub fn add_account(&mut self, account: AccountItem) {
        self.account_switcher.update_account(account);
    }
    
    /// Get the currently selected account
    pub fn get_current_account(&self) -> Option<&AccountItem> {
        self.account_switcher.get_current_account()
    }
    
    /// Get the current account ID
    pub fn get_current_account_id(&self) -> Option<&String> {
        self.account_switcher.get_current_account_id()
    }
    
    /// Switch to a specific account
    pub async fn switch_to_account(&mut self, account_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        if self.account_switcher.set_current_account(account_id) {
            // Load messages for the new account's INBOX
            self.load_messages(account_id.to_string(), "INBOX".to_string()).await?;
            
            // Update the system status to show the new account
            let current_time = chrono::Local::now().format("%H:%M").to_string();
            if let Some(account) = self.account_switcher.get_current_account() {
                let system_segment = SystemInfoSegment {
                    current_time,
                    active_account: account.email_address.clone(),
                };
                self.status_bar.add_segment("system".to_string(), system_segment);
            }
        }
        Ok(())
    }
    
    /// Update account status and unread count
    pub fn update_account_status(&mut self, account_id: &str, status: AccountSyncStatus, unread_count: Option<usize>) {
        self.account_switcher.update_account_status(account_id, status, unread_count);
    }
    
    /// Refresh current folder's messages
    pub async fn refresh_messages(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.message_list.refresh_messages().await
    }
    
    /// Get reference to message list for direct access
    pub fn message_list(&self) -> &MessageList {
        &self.message_list
    }
    
    /// Handle message selection and load content in preview
    pub async fn handle_message_selection(&mut self) {
        if let Some(selected_message) = self.message_list.get_selected_message_for_preview() {
            if let Some(message_id) = selected_message.message_id {
                // Load the selected message content
                if let Err(e) = self.content_preview.load_message_by_id(message_id).await {
                    tracing::error!("Failed to load message content: {}", e);
                    // Show error in preview if loading fails
                    self.content_preview.clear_message();
                }
            } else {
                // No message ID available (probably sample data)
                self.content_preview.clear_message();
            }
        } else {
            // No message selected
            self.content_preview.clear_message();
        }
    }
    
    /// Process pending email notifications (non-blocking)
    pub async fn process_notifications(&mut self) {
        let mut notifications = Vec::new();
        
        // Collect all pending notifications first
        if let Some(ref mut updater) = self.email_updater {
            while let Some(notification) = updater.try_recv_notification() {
                notifications.push(notification);
            }
        }
        
        // Process all collected notifications
        for notification in notifications {
            self.handle_notification(notification).await;
        }
    }
    
    /// Handle a specific email notification
    async fn handle_notification(&mut self, notification: EmailNotification) {
        match notification {
            EmailNotification::NewMessage { account_id, folder_name, message } => {
                // Check if this notification is for the currently displayed folder
                if let (Some(current_account), Some(current_folder)) = self.message_list.get_current_context() {
                    if current_account == &account_id && current_folder == &folder_name {
                        // Refresh the message list to show the new message
                        let _ = self.message_list.refresh_messages().await;
                        
                        // Update status bar with new counts
                        let message_count = self.message_list.messages().len();
                        let unread_count = self.message_list.messages().iter()
                            .filter(|msg| !msg.is_read)
                            .count();
                        self.update_email_status(unread_count, message_count, SyncStatus::Online);
                    }
                }
                
                tracing::info!("New message received: {} from {}", message.subject, message.from_addr);
            }
            
            EmailNotification::MessageUpdated { account_id, folder_name, message, .. } => {
                // Check if this notification is for the currently displayed folder
                if let (Some(current_account), Some(current_folder)) = self.message_list.get_current_context() {
                    if current_account == &account_id && current_folder == &folder_name {
                        // Refresh the message list to show updated message
                        let _ = self.message_list.refresh_messages().await;
                        
                        // Update status bar
                        let message_count = self.message_list.messages().len();
                        let unread_count = self.message_list.messages().iter()
                            .filter(|msg| !msg.is_read)
                            .count();
                        self.update_email_status(unread_count, message_count, SyncStatus::Online);
                    }
                }
                
                tracing::info!("Message updated: {}", message.subject);
            }
            
            EmailNotification::MessageDeleted { account_id, folder_name, .. } => {
                // Check if this notification is for the currently displayed folder
                if let (Some(current_account), Some(current_folder)) = self.message_list.get_current_context() {
                    if current_account == &account_id && current_folder == &folder_name {
                        // Refresh the message list to remove deleted message
                        let _ = self.message_list.refresh_messages().await;
                        
                        // Update status bar
                        let message_count = self.message_list.messages().len();
                        let unread_count = self.message_list.messages().iter()
                            .filter(|msg| !msg.is_read)
                            .count();
                        self.update_email_status(unread_count, message_count, SyncStatus::Online);
                    }
                }
                
                tracing::info!("Message deleted from {}/{}", account_id, folder_name);
            }
            
            EmailNotification::SyncStarted { account_id, folder_name } => {
                // Update status bar to show sync in progress
                if let (Some(current_account), Some(current_folder)) = self.message_list.get_current_context() {
                    if current_account == &account_id && current_folder == &folder_name {
                        let message_count = self.message_list.messages().len();
                        let unread_count = self.message_list.messages().iter()
                            .filter(|msg| !msg.is_read)
                            .count();
                        self.update_email_status(unread_count, message_count, SyncStatus::Syncing);
                    }
                }
                
                // Create initial sync progress entry
                let initial_progress = SyncProgress {
                    account_id: account_id.clone(),
                    folder_name: folder_name.clone(),
                    phase: crate::email::sync_engine::SyncPhase::Initializing,
                    messages_processed: 0,
                    total_messages: 0,
                    bytes_downloaded: 0,
                    started_at: chrono::Utc::now(),
                    estimated_completion: None,
                };
                self.update_sync_progress(initial_progress);
                
                tracing::info!("Sync started for {}/{}", account_id, folder_name);
            }
            
            EmailNotification::SyncCompleted { account_id, folder_name, new_count, updated_count } => {
                // Update status bar to show sync completed
                if let (Some(current_account), Some(current_folder)) = self.message_list.get_current_context() {
                    if current_account == &account_id && current_folder == &folder_name {
                        // Refresh messages after sync
                        let _ = self.message_list.refresh_messages().await;
                        
                        let message_count = self.message_list.messages().len();
                        let unread_count = self.message_list.messages().iter()
                            .filter(|msg| !msg.is_read)
                            .count();
                        self.update_email_status(unread_count, message_count, SyncStatus::Online);
                    }
                }
                
                // Update sync progress to completed
                let completed_progress = SyncProgress {
                    account_id: account_id.clone(),
                    folder_name: folder_name.clone(),
                    phase: crate::email::sync_engine::SyncPhase::Complete,
                    messages_processed: new_count + updated_count,
                    total_messages: new_count + updated_count,
                    bytes_downloaded: 0, // TODO: Get actual bytes from sync engine
                    started_at: chrono::Utc::now() - chrono::Duration::seconds(1), // Approximate
                    estimated_completion: Some(chrono::Utc::now()),
                };
                self.update_sync_progress(completed_progress);
                
                tracing::info!("Sync completed for {}/{}: {} new, {} updated", 
                             account_id, folder_name, new_count, updated_count);
            }
            
            EmailNotification::SyncFailed { account_id, folder_name, error } => {
                // Update status bar to show sync error
                if let (Some(current_account), Some(current_folder)) = self.message_list.get_current_context() {
                    if current_account == &account_id && current_folder == &folder_name {
                        let message_count = self.message_list.messages().len();
                        let unread_count = self.message_list.messages().iter()
                            .filter(|msg| !msg.is_read)
                            .count();
                        self.update_email_status(unread_count, message_count, SyncStatus::Error);
                    }
                }
                
                // Update sync progress to error state
                let error_progress = SyncProgress {
                    account_id: account_id.clone(),
                    folder_name: folder_name.clone(),
                    phase: crate::email::sync_engine::SyncPhase::Error(error.clone()),
                    messages_processed: 0,
                    total_messages: 0,
                    bytes_downloaded: 0,
                    started_at: chrono::Utc::now() - chrono::Duration::seconds(1), // Approximate
                    estimated_completion: None,
                };
                self.update_sync_progress(error_progress);
                
                tracing::error!("Sync failed for {}/{}: {}", account_id, folder_name, error);
            }
        }
    }
    
    /// Update sync progress indicators
    pub fn update_sync_progress(&mut self, progress: SyncProgress) {
        self.sync_progress_overlay.update_progress(progress.clone());
        
        // Also update status bar with progress if this is for the current folder
        if let (Some(current_account), Some(current_folder)) = self.message_list.get_current_context() {
            if current_account == &progress.account_id && current_folder == &progress.folder_name {
                let message_count = self.message_list.messages().len();
                let unread_count = self.message_list.messages().iter()
                    .filter(|msg| !msg.is_read)
                    .count();
                    
                let sync_status = match progress.phase {
                    crate::email::sync_engine::SyncPhase::Complete => SyncStatus::Online,
                    crate::email::sync_engine::SyncPhase::Error(_) => SyncStatus::Error,
                    _ => {
                        if progress.total_messages > 0 {
                            SyncStatus::SyncingWithProgress(progress.messages_processed, progress.total_messages)
                        } else {
                            SyncStatus::Syncing
                        }
                    }
                };
                
                self.update_email_status(unread_count, message_count, sync_status);
            }
        }
    }
    
    /// Toggle sync progress overlay visibility
    pub fn toggle_sync_progress_overlay(&mut self) {
        self.sync_progress_overlay.toggle_visibility();
    }
    
    /// Clean up completed sync progress entries
    pub fn cleanup_sync_progress(&mut self) {
        // Remove completed syncs after 3 seconds
        let threshold = ChronoDuration::seconds(3);
        self.sync_progress_overlay.cleanup_completed(threshold);
    }
    
    /// Navigate sync progress overlay (for keyboard interaction)
    pub fn sync_progress_next(&mut self) {
        self.sync_progress_overlay.next_sync();
    }
    
    pub fn sync_progress_previous(&mut self) {
        self.sync_progress_overlay.previous_sync();
    }
    
    /// Check if sync progress overlay is currently visible
    pub fn is_sync_progress_visible(&self) -> bool {
        self.sync_progress_overlay.is_visible()
    }
    
    // Compose mode methods
    
    /// Enter compose mode with a new email
    pub fn start_compose(&mut self, contacts_manager: Arc<crate::contacts::ContactsManager>) {
        self.compose_ui = Some(ComposeUI::new(contacts_manager));
        self.mode = UIMode::Compose;
        self.focused_pane = FocusedPane::Compose;
    }
    
    /// Enter compose mode for replying to a message
    pub fn start_reply(&mut self, contacts_manager: Arc<crate::contacts::ContactsManager>, reply_to: &str, subject: &str) {
        self.compose_ui = Some(ComposeUI::new_reply(contacts_manager, reply_to, subject));
        self.mode = UIMode::Compose;
        self.focused_pane = FocusedPane::Compose;
    }
    
    /// Enter compose mode for forwarding a message
    pub fn start_forward(&mut self, contacts_manager: Arc<crate::contacts::ContactsManager>, subject: &str, body: &str) {
        self.compose_ui = Some(ComposeUI::new_forward(contacts_manager, subject, body));
        self.mode = UIMode::Compose;
        self.focused_pane = FocusedPane::Compose;
    }
    
    /// Exit compose mode and return to normal view
    pub fn exit_compose(&mut self) {
        self.compose_ui = None;
        self.mode = UIMode::Normal;
        self.focused_pane = FocusedPane::FolderTree;
    }
    
    /// Handle key input for compose mode
    pub async fn handle_compose_key(&mut self, key: crossterm::event::KeyCode) -> Option<ComposeAction> {
        if let Some(ref mut compose_ui) = self.compose_ui {
            Some(compose_ui.handle_key(key).await)
        } else {
            None
        }
    }
    
    /// Get the current email composition data
    pub fn get_compose_data(&self) -> Option<EmailComposeData> {
        self.compose_ui.as_ref().map(|ui| ui.get_email_data())
    }
    
    /// Check if compose form has been modified
    pub fn is_compose_modified(&self) -> bool {
        self.compose_ui.as_ref().map(|ui| ui.is_modified()).unwrap_or(false)
    }
    
    /// Clear the compose modified flag
    pub fn clear_compose_modified(&mut self) {
        if let Some(ref mut compose_ui) = self.compose_ui {
            compose_ui.clear_modified();
        }
    }
    
    /// Get current UI mode
    pub fn mode(&self) -> &UIMode {
        &self.mode
    }
    
    /// Check if currently in compose mode
    pub fn is_composing(&self) -> bool {
        matches!(self.mode, UIMode::Compose)
    }
}

impl Default for UI {
    fn default() -> Self {
        Self::new()
    }
}