use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState},
    Frame,
};
use crate::theme::Theme;
use crate::email::EmailDatabase;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FolderType {
    Inbox,
    Sent,
    Drafts,
    Trash,
    Archive,
    Spam,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncStatus {
    Synced,
    Syncing,
    Error,
    Offline,
    NeverSynced,
}

#[derive(Debug, Clone)]
pub struct FolderItem {
    pub name: String,
    pub path: String,
    pub folder_type: FolderType,
    pub is_expanded: bool,
    pub has_children: bool,
    pub depth: usize,
    pub unread_count: usize,
    pub total_count: usize,
    pub sync_status: SyncStatus,
    pub is_subscribed: bool,
    pub can_create_children: bool,
    pub children: Vec<String>, // Paths of child folders
}

impl FolderItem {
    pub fn new(name: String, path: String, depth: usize) -> Self {
        let folder_type = Self::detect_folder_type(&name, &path);
        Self {
            name,
            path,
            folder_type,
            is_expanded: false,
            has_children: false,
            depth,
            unread_count: 0,
            total_count: 0,
            sync_status: SyncStatus::NeverSynced,
            is_subscribed: true,
            can_create_children: true,
            children: Vec::new(),
        }
    }
    
    pub fn new_with_type(name: String, path: String, depth: usize, folder_type: FolderType) -> Self {
        Self {
            name,
            path,
            folder_type,
            is_expanded: false,
            has_children: false,
            depth,
            unread_count: 0,
            total_count: 0,
            sync_status: SyncStatus::NeverSynced,
            is_subscribed: true,
            can_create_children: true,
            children: Vec::new(),
        }
    }
    
    fn detect_folder_type(name: &str, path: &str) -> FolderType {
        let name_lower = name.to_lowercase();
        let path_lower = path.to_lowercase();
        
        if name_lower == "inbox" || path_lower.contains("inbox") {
            FolderType::Inbox
        } else if name_lower == "sent" || path_lower.contains("sent") {
            FolderType::Sent
        } else if name_lower == "drafts" || path_lower.contains("draft") {
            FolderType::Drafts
        } else if name_lower == "trash" || path_lower.contains("trash") || name_lower == "deleted" {
            FolderType::Trash
        } else if name_lower == "archive" || path_lower.contains("archive") {
            FolderType::Archive
        } else if name_lower == "spam" || name_lower == "junk" || path_lower.contains("spam") {
            FolderType::Spam
        } else {
            FolderType::Custom(name.to_string())
        }
    }

    pub fn with_children(mut self, children_paths: Vec<String>) -> Self {
        self.has_children = !children_paths.is_empty();
        self.children = children_paths;
        self
    }

    pub fn with_unread_count(mut self, count: usize) -> Self {
        self.unread_count = count;
        self
    }
    
    pub fn with_total_count(mut self, count: usize) -> Self {
        self.total_count = count;
        self
    }
    
    pub fn with_sync_status(mut self, status: SyncStatus) -> Self {
        self.sync_status = status;
        self
    }
    
    pub fn subscribed(mut self, subscribed: bool) -> Self {
        self.is_subscribed = subscribed;
        self
    }
    
    pub fn can_create_children(mut self, can_create: bool) -> Self {
        self.can_create_children = can_create;
        self
    }
    
    /// Get the appropriate icon for this folder type
    pub fn get_type_icon(&self) -> &'static str {
        match self.folder_type {
            FolderType::Inbox => "📥",
            FolderType::Sent => "📤", 
            FolderType::Drafts => "📝",
            FolderType::Trash => "🗑",
            FolderType::Archive => "📦",
            FolderType::Spam => "⚠",
            FolderType::Custom(_) => "📁",
        }
    }
    
    /// Get professional folder type indicator
    pub fn get_type_indicator(&self) -> &'static str {
        match self.folder_type {
            FolderType::Inbox => "▶",       // Inbox - triangle pointing right
            FolderType::Sent => "◀",        // Sent - triangle pointing left  
            FolderType::Drafts => "◆",      // Drafts - diamond
            FolderType::Trash => "×",       // Trash - X symbol
            FolderType::Archive => "▣",     // Archive - square with pattern
            FolderType::Spam => "⚠",        // Spam - warning triangle
            FolderType::Custom(_) => "●",   // Custom - solid circle
        }
    }
    
    /// Get sync status indicator
    pub fn get_sync_indicator(&self) -> &'static str {
        match self.sync_status {
            SyncStatus::Synced => "●",
            SyncStatus::Syncing => "⟳",
            SyncStatus::Error => "⚠",
            SyncStatus::Offline => "○",
            SyncStatus::NeverSynced => "◌",
        }
    }
    
    /// Check if this folder can be deleted
    pub fn is_deletable(&self) -> bool {
        !matches!(self.folder_type, FolderType::Inbox | FolderType::Sent | FolderType::Drafts | FolderType::Trash)
    }
    
    /// Check if this folder can be renamed
    pub fn is_renamable(&self) -> bool {
        matches!(self.folder_type, FolderType::Custom(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FolderOperation {
    Create,
    Delete,
    Rename,
    Move,
    Subscribe,
    Unsubscribe,
}

pub struct FolderTree {
    folders: Vec<FolderItem>,
    state: ListState,
    search_query: String,
    filtered_folders: Vec<usize>, // Indices of folders matching search
    show_unsubscribed: bool,
    selected_for_operation: Option<(usize, FolderOperation)>,
    database: Option<Arc<EmailDatabase>>,
}

impl FolderTree {
    pub fn new() -> Self {
        let mut tree = Self {
            folders: Vec::new(),
            state: ListState::default(),
            search_query: String::new(),
            filtered_folders: Vec::new(),
            show_unsubscribed: false,
            selected_for_operation: None,
            database: None,
        };
        
        // Initialize with sample folder structure
        tree.initialize_sample_folders();
        tree.state.select(Some(0));
        
        tree
    }

    fn initialize_sample_folders(&mut self) {
        self.folders = vec![
            // Standard IMAP folders
            FolderItem::new_with_type("Inbox".to_string(), "INBOX".to_string(), 0, FolderType::Inbox)
                .with_unread_count(5)
                .with_total_count(127)
                .with_sync_status(SyncStatus::Synced),
                
            FolderItem::new_with_type("Sent".to_string(), "INBOX/Sent".to_string(), 0, FolderType::Sent)
                .with_total_count(89)
                .with_sync_status(SyncStatus::Synced),
                
            FolderItem::new_with_type("Drafts".to_string(), "INBOX/Drafts".to_string(), 0, FolderType::Drafts)
                .with_unread_count(2)
                .with_total_count(4)
                .with_sync_status(SyncStatus::Syncing),
                
            FolderItem::new_with_type("Archive".to_string(), "INBOX/Archive".to_string(), 0, FolderType::Archive)
                .with_total_count(1234)
                .with_sync_status(SyncStatus::Synced),
                
            FolderItem::new_with_type("Trash".to_string(), "INBOX/Trash".to_string(), 0, FolderType::Trash)
                .with_total_count(23)
                .with_sync_status(SyncStatus::Synced),
                
            FolderItem::new_with_type("Spam".to_string(), "INBOX/Spam".to_string(), 0, FolderType::Spam)
                .with_total_count(7)
                .with_sync_status(SyncStatus::Error),
            
            // Custom folders with hierarchy
            FolderItem::new("Work".to_string(), "INBOX/Work".to_string(), 0)
                .with_children(vec!["INBOX/Work/Projects".to_string(), "INBOX/Work/Team".to_string()])
                .with_unread_count(3)
                .with_total_count(45)
                .with_sync_status(SyncStatus::Synced),
                
            FolderItem::new("Projects".to_string(), "INBOX/Work/Projects".to_string(), 1)
                .with_unread_count(1)
                .with_total_count(12)
                .with_sync_status(SyncStatus::Synced),
                
            FolderItem::new("Team".to_string(), "INBOX/Work/Team".to_string(), 1)
                .with_total_count(33)
                .with_sync_status(SyncStatus::Synced),
            
            FolderItem::new("Personal".to_string(), "INBOX/Personal".to_string(), 0)
                .with_children(vec!["INBOX/Personal/Family".to_string(), "INBOX/Personal/Finance".to_string()])
                .with_unread_count(2)
                .with_total_count(78)
                .with_sync_status(SyncStatus::Synced),
                
            FolderItem::new("Family".to_string(), "INBOX/Personal/Family".to_string(), 1)
                .with_unread_count(1)
                .with_total_count(34)
                .with_sync_status(SyncStatus::Synced),
                
            FolderItem::new("Finance".to_string(), "INBOX/Personal/Finance".to_string(), 1)
                .with_unread_count(1)
                .with_total_count(44)
                .with_sync_status(SyncStatus::Synced),
            
            FolderItem::new("Important".to_string(), "INBOX/Important".to_string(), 0)
                .with_unread_count(1)
                .with_total_count(8)
                .with_sync_status(SyncStatus::Synced),
                
            // Unsubscribed folder (normally hidden)
            FolderItem::new("Old Archives".to_string(), "INBOX/OldArchives".to_string(), 0)
                .with_total_count(567)
                .with_sync_status(SyncStatus::Offline)
                .subscribed(false),
        ];
        
        // Build filtered folders list (subscribed folders only by default)
        self.rebuild_filtered_list();
    }
    
    fn rebuild_filtered_list(&mut self) {
        self.filtered_folders.clear();
        
        for (index, folder) in self.folders.iter().enumerate() {
            let matches_search = if self.search_query.is_empty() {
                true
            } else {
                folder.name.to_lowercase().contains(&self.search_query.to_lowercase()) ||
                folder.path.to_lowercase().contains(&self.search_query.to_lowercase())
            };
            
            let is_visible = folder.is_subscribed || self.show_unsubscribed;
            
            if matches_search && is_visible {
                self.filtered_folders.push(index);
            }
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, block: Block, is_focused: bool, theme: &Theme) {
        let items: Vec<ListItem> = self.filtered_folders
            .iter()
            .enumerate()
            .map(|(display_i, &folder_i)| {
                let folder = &self.folders[folder_i];
                let indent = "  ".repeat(folder.depth);
                
                // Expansion indicator
                let expand_icon = if folder.has_children {
                    if folder.is_expanded { "▼ " } else { "▶ " }
                } else {
                    "  "
                };
                
                // Type indicator (professional monochrome icons)
                let type_indicator = folder.get_type_indicator();
                
                // Sync status indicator
                let sync_indicator = folder.get_sync_indicator();
                
                // Count display: show unread if > 0, otherwise total
                let count_display = if folder.unread_count > 0 {
                    format!(" ({}/{})", folder.unread_count, folder.total_count)
                } else if folder.total_count > 0 {
                    format!(" ({})", folder.total_count)
                } else {
                    String::new()
                };

                let is_selected = self.state.selected() == Some(display_i);
                
                // Determine folder style based on state and type
                let folder_style = if is_selected && is_focused {
                    theme.styles.get_selected_style("folder_tree", &theme.colors)
                } else if is_selected {
                    theme.styles.get_selected_style("folder_tree", &theme.colors)
                } else if folder.unread_count > 0 {
                    Style::default()
                        .fg(theme.colors.folder_tree.folder_unread)
                        .add_modifier(Modifier::BOLD)
                } else if matches!(folder.sync_status, SyncStatus::Error) {
                    Style::default()
                        .fg(theme.colors.palette.error)
                } else if matches!(folder.sync_status, SyncStatus::Syncing) {
                    Style::default()
                        .fg(theme.colors.palette.warning)
                } else if !folder.is_subscribed {
                    Style::default()
                        .fg(theme.colors.palette.text_muted)
                        .add_modifier(Modifier::DIM)
                } else {
                    Style::default().fg(theme.colors.folder_tree.folder_normal)
                };

                let line = Line::from(vec![
                    Span::styled(indent, Style::default().fg(theme.colors.folder_tree.expand_icon)),
                    Span::styled(expand_icon, Style::default().fg(theme.colors.folder_tree.expand_icon)),
                    Span::styled(type_indicator, Style::default().fg(theme.colors.folder_tree.folder_normal)),
                    Span::raw(" "),
                    Span::styled(folder.name.clone(), folder_style),
                    Span::styled(count_display, Style::default().fg(theme.colors.folder_tree.count_badge)),
                    Span::raw(" "),
                    Span::styled(sync_indicator, Style::default().fg(match folder.sync_status {
                        SyncStatus::Synced => theme.colors.palette.success,
                        SyncStatus::Syncing => theme.colors.palette.warning,
                        SyncStatus::Error => theme.colors.palette.error,
                        SyncStatus::Offline => theme.colors.palette.text_muted,
                        SyncStatus::NeverSynced => theme.colors.palette.text_muted,
                    })),
                ]);

                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        frame.render_stateful_widget(list, area, &mut self.state.clone());
    }

    pub fn handle_up(&mut self) {
        let selected = match self.state.selected() {
            Some(i) => {
                if i > 0 {
                    Some(i - 1)
                } else {
                    Some(self.filtered_folders.len().saturating_sub(1))
                }
            }
            None => Some(0),
        };
        self.state.select(selected);
    }

    pub fn handle_down(&mut self) {
        let selected = match self.state.selected() {
            Some(i) => {
                if i < self.filtered_folders.len().saturating_sub(1) {
                    Some(i + 1)
                } else {
                    Some(0)
                }
            }
            None => Some(0),
        };
        self.state.select(selected);
    }

    pub fn handle_right(&mut self) {
        if let Some(selected) = self.state.selected() {
            if let Some(&folder_i) = self.filtered_folders.get(selected) {
                if let Some(folder) = self.folders.get_mut(folder_i) {
                    if folder.has_children {
                        folder.is_expanded = true;
                        self.rebuild_filtered_list(); // Refresh display
                    }
                }
            }
        }
    }

    pub fn handle_left(&mut self) {
        if let Some(selected) = self.state.selected() {
            if let Some(&folder_i) = self.filtered_folders.get(selected) {
                if let Some(folder) = self.folders.get_mut(folder_i) {
                    if folder.has_children && folder.is_expanded {
                        folder.is_expanded = false;
                        self.rebuild_filtered_list(); // Refresh display
                    }
                }
            }
        }
    }

    pub fn handle_enter(&mut self) {
        if let Some(selected) = self.state.selected() {
            if let Some(&folder_i) = self.filtered_folders.get(selected) {
                if let Some(folder) = self.folders.get_mut(folder_i) {
                    if folder.has_children {
                        folder.is_expanded = !folder.is_expanded;
                        self.rebuild_filtered_list(); // Refresh display
                    }
                    // In the future, this will also trigger loading emails from the selected folder
                }
            }
        }
    }

    pub fn selected_folder(&self) -> Option<&FolderItem> {
        self.state.selected()
            .and_then(|display_i| self.filtered_folders.get(display_i))
            .and_then(|&folder_i| self.folders.get(folder_i))
    }
    
    // New folder management methods
    
    pub fn set_search_query(&mut self, query: String) {
        self.search_query = query;
        self.rebuild_filtered_list();
        // Reset selection to first item if current selection is out of bounds
        if self.state.selected().unwrap_or(0) >= self.filtered_folders.len() {
            self.state.select(if self.filtered_folders.is_empty() { None } else { Some(0) });
        }
    }
    
    pub fn clear_search(&mut self) {
        self.search_query.clear();
        self.rebuild_filtered_list();
    }
    
    pub fn toggle_show_unsubscribed(&mut self) {
        self.show_unsubscribed = !self.show_unsubscribed;
        self.rebuild_filtered_list();
    }
    
    pub fn refresh_folder(&mut self, path: &str) {
        if let Some(folder) = self.folders.iter_mut().find(|f| f.path == path) {
            folder.sync_status = SyncStatus::Syncing;
            // In a real implementation, this would trigger an IMAP sync
        }
        self.rebuild_filtered_list();
    }
    
    pub fn mark_folder_synced(&mut self, path: &str, unread_count: usize, total_count: usize) {
        if let Some(folder) = self.folders.iter_mut().find(|f| f.path == path) {
            folder.sync_status = SyncStatus::Synced;
            folder.unread_count = unread_count;
            folder.total_count = total_count;
        }
        self.rebuild_filtered_list();
    }
    
    pub fn mark_folder_error(&mut self, path: &str) {
        if let Some(folder) = self.folders.iter_mut().find(|f| f.path == path) {
            folder.sync_status = SyncStatus::Error;
        }
        self.rebuild_filtered_list();
    }
    
    pub fn create_folder(&mut self, parent_path: Option<&str>, name: String) -> Result<String, String> {
        let new_path = if let Some(parent) = parent_path {
            format!("{}/{}", parent, name)
        } else {
            name.clone()
        };
        
        // Check if folder already exists
        if self.folders.iter().any(|f| f.path == new_path) {
            return Err("Folder already exists".to_string());
        }
        
        // Determine depth based on parent
        let depth = if let Some(parent) = parent_path {
            self.folders.iter()
                .find(|f| f.path == parent)
                .map(|f| f.depth + 1)
                .unwrap_or(0)
        } else {
            0
        };
        
        let new_folder = FolderItem::new(name, new_path.clone(), depth)
            .with_sync_status(SyncStatus::NeverSynced);
        
        self.folders.push(new_folder);
        
        // Update parent folder to show it has children
        if let Some(parent_path) = parent_path {
            if let Some(parent_folder) = self.folders.iter_mut().find(|f| f.path == parent_path) {
                if !parent_folder.children.contains(&new_path) {
                    parent_folder.children.push(new_path.clone());
                    parent_folder.has_children = true;
                }
            }
        }
        
        self.rebuild_filtered_list();
        Ok(new_path)
    }
    
    pub fn delete_folder(&mut self, path: &str) -> Result<(), String> {
        // Find the folder
        let folder_index = self.folders.iter()
            .position(|f| f.path == path)
            .ok_or("Folder not found")?;
        
        let folder = &self.folders[folder_index];
        
        // Check if it's deletable
        if !folder.is_deletable() {
            return Err("Cannot delete system folder".to_string());
        }
        
        // Check if it has children
        if folder.has_children {
            return Err("Cannot delete folder with subfolders".to_string());
        }
        
        // Remove from parent's children list
        let parent_path = if let Some(last_slash) = path.rfind('/') {
            Some(path[..last_slash].to_string())
        } else {
            None
        };
        
        if let Some(parent_path) = parent_path {
            if let Some(parent_folder) = self.folders.iter_mut().find(|f| f.path == parent_path) {
                parent_folder.children.retain(|child_path| child_path != path);
                parent_folder.has_children = !parent_folder.children.is_empty();
            }
        }
        
        // Remove the folder
        self.folders.remove(folder_index);
        self.rebuild_filtered_list();
        
        Ok(())
    }
    
    pub fn rename_folder(&mut self, old_path: &str, new_name: String) -> Result<String, String> {
        let folder_index = self.folders.iter()
            .position(|f| f.path == old_path)
            .ok_or("Folder not found")?;
        
        let folder = &self.folders[folder_index];
        
        if !folder.is_renamable() {
            return Err("Cannot rename system folder".to_string());
        }
        
        // Calculate new path
        let new_path = if let Some(last_slash) = old_path.rfind('/') {
            format!("{}/{}", &old_path[..last_slash], new_name)
        } else {
            new_name.clone()
        };
        
        // Check if new path conflicts
        if self.folders.iter().any(|f| f.path == new_path) {
            return Err("Folder with that name already exists".to_string());
        }
        
        // Update the folder
        let folder = &mut self.folders[folder_index];
        folder.name = new_name;
        folder.path = new_path.clone();
        
        self.rebuild_filtered_list();
        Ok(new_path)
    }
    
    pub fn get_folder_stats(&self) -> (usize, usize, usize) {
        let total_folders = self.folders.len();
        let subscribed_folders = self.folders.iter().filter(|f| f.is_subscribed).count();
        let unread_folders = self.folders.iter().filter(|f| f.unread_count > 0).count();
        (total_folders, subscribed_folders, unread_folders)
    }
    
    /// Set the database for loading folders
    pub fn set_database(&mut self, database: Arc<EmailDatabase>) {
        self.database = Some(database);
    }
    
    /// Load folders from database for a specific account
    pub async fn load_folders(&mut self, account_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(database) = &self.database {
            let stored_folders = database.get_folders(account_id).await?;
            
            // Convert stored folders to FolderItems
            let mut folder_items = Vec::new();
            
            for stored_folder in stored_folders {
                let folder_type = FolderItem::detect_folder_type(&stored_folder.name, &stored_folder.full_name);
                
                let mut folder_item = FolderItem::new_with_type(
                    stored_folder.name.clone(),
                    stored_folder.full_name.clone(),
                    0, // Depth will be calculated later if needed
                    folder_type,
                );
                
                // Set folder as synced since it exists in database
                folder_item.sync_status = SyncStatus::Synced;
                
                folder_items.push(folder_item);
            }
            
            // Sort folders: INBOX first, then alphabetically
            folder_items.sort_by(|a, b| {
                match (&a.folder_type, &b.folder_type) {
                    (FolderType::Inbox, FolderType::Inbox) => std::cmp::Ordering::Equal,
                    (FolderType::Inbox, _) => std::cmp::Ordering::Less,
                    (_, FolderType::Inbox) => std::cmp::Ordering::Greater,
                    _ => a.name.cmp(&b.name),
                }
            });
            
            self.folders = folder_items;
            self.rebuild_filtered_list();
            
            // Select INBOX by default if it exists
            if let Some(inbox_index) = self.folders.iter().position(|f| matches!(f.folder_type, FolderType::Inbox)) {
                self.state.select(Some(inbox_index));
            } else if !self.folders.is_empty() {
                self.state.select(Some(0));
            }
            
            tracing::info!("Loaded {} folders from database", self.folders.len());
        } else {
            return Err("Database not set".into());
        }
        
        Ok(())
    }
}

impl Default for FolderTree {
    fn default() -> Self {
        Self::new()
    }
}