pub mod folder_tree;
pub mod message_list;
pub mod content_preview;
pub mod layout;
pub mod status_bar;

use ratatui::{
    layout::Rect,
    widgets::{Block, Borders},
    Frame,
};
use crate::theme::{Theme, ThemeManager};
use crate::email::{EmailDatabase, EmailNotificationManager, UIEmailUpdater, EmailNotification};
use std::sync::Arc;

use self::{
    folder_tree::FolderTree,
    message_list::MessageList,
    content_preview::ContentPreview,
    layout::AppLayout,
    status_bar::{StatusBar, EmailStatusSegment, CalendarStatusSegment, SystemInfoSegment, NavigationHintsSegment, SyncStatus},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPane {
    FolderTree,
    MessageList,
    ContentPreview,
}

pub struct UI {
    focused_pane: FocusedPane,
    folder_tree: FolderTree,
    message_list: MessageList,
    content_preview: ContentPreview,
    layout: AppLayout,
    theme_manager: ThemeManager,
    status_bar: StatusBar,
    email_updater: Option<UIEmailUpdater>,
}

impl UI {
    pub fn new() -> Self {
        let mut ui = Self {
            focused_pane: FocusedPane::FolderTree,
            folder_tree: FolderTree::new(),
            message_list: MessageList::new(),
            content_preview: ContentPreview::new(),
            layout: AppLayout::new(),
            theme_manager: ThemeManager::new(),
            status_bar: StatusBar::default(),
            email_updater: None,
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
        let chunks = self.layout.calculate_layout(size);

        // Render each pane with focus styling
        self.render_folder_tree(frame, chunks[0]);
        self.render_message_list(frame, chunks[1]);
        self.render_content_preview(frame, chunks[2]);
        
        // Render the status bar
        if chunks.len() > 3 {
            self.render_status_bar(frame, chunks[3]);
        }
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

    fn render_content_preview(&self, frame: &mut Frame, area: Rect) {
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
        self.focused_pane = match self.focused_pane {
            FocusedPane::FolderTree => FocusedPane::MessageList,
            FocusedPane::MessageList => FocusedPane::ContentPreview,
            FocusedPane::ContentPreview => FocusedPane::FolderTree,
        };
        self.update_navigation_hints();
    }

    pub fn previous_pane(&mut self) {
        self.focused_pane = match self.focused_pane {
            FocusedPane::FolderTree => FocusedPane::ContentPreview,
            FocusedPane::MessageList => FocusedPane::FolderTree,
            FocusedPane::ContentPreview => FocusedPane::MessageList,
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
        let current_pane_name = match self.focused_pane {
            FocusedPane::FolderTree => "Folders",
            FocusedPane::MessageList => "Messages", 
            FocusedPane::ContentPreview => "Content",
        };
        
        let nav_segment = NavigationHintsSegment {
            current_pane: current_pane_name.to_string(),
            available_shortcuts: self.get_current_shortcuts(),
        };
        
        self.status_bar.add_segment("navigation".to_string(), nav_segment);
    }
    
    fn get_current_shortcuts(&self) -> Vec<(String, String)> {
        match self.focused_pane {
            FocusedPane::FolderTree => vec![
                ("Tab".to_string(), "Switch".to_string()),
                ("j/k".to_string(), "Navigate".to_string()),
                ("l".to_string(), "Expand".to_string()),
                ("h".to_string(), "Collapse".to_string()),
            ],
            FocusedPane::MessageList => vec![
                ("Tab".to_string(), "Switch".to_string()),
                ("j/k".to_string(), "Navigate".to_string()),
                ("Enter".to_string(), "Open".to_string()),
            ],
            FocusedPane::ContentPreview => vec![
                ("Tab".to_string(), "Switch".to_string()),
                ("j/k".to_string(), "Scroll".to_string()),
                ("v".to_string(), "View Mode".to_string()),
                ("H".to_string(), "Headers".to_string()),
                ("Home/End".to_string(), "Jump".to_string()),
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
                
                tracing::error!("Sync failed for {}/{}: {}", account_id, folder_name, error);
            }
        }
    }
}

impl Default for UI {
    fn default() -> Self {
        Self::new()
    }
}