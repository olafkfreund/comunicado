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
                ("q".to_string(), "Quit".to_string()),
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
}

impl Default for UI {
    fn default() -> Self {
        Self::new()
    }
}