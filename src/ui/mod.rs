pub mod folder_tree;
pub mod message_list;
pub mod content_preview;
pub mod layout;

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
}

impl UI {
    pub fn new() -> Self {
        Self {
            focused_pane: FocusedPane::FolderTree,
            folder_tree: FolderTree::new(),
            message_list: MessageList::new(),
            content_preview: ContentPreview::new(),
            layout: AppLayout::new(),
            theme_manager: ThemeManager::new(),
        }
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let size = frame.size();
        let chunks = self.layout.calculate_layout(size);

        // Render each pane with focus styling
        self.render_folder_tree(frame, chunks[0]);
        self.render_message_list(frame, chunks[1]);
        self.render_content_preview(frame, chunks[2]);
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

    // Navigation methods
    pub fn next_pane(&mut self) {
        self.focused_pane = match self.focused_pane {
            FocusedPane::FolderTree => FocusedPane::MessageList,
            FocusedPane::MessageList => FocusedPane::ContentPreview,
            FocusedPane::ContentPreview => FocusedPane::FolderTree,
        };
    }

    pub fn previous_pane(&mut self) {
        self.focused_pane = match self.focused_pane {
            FocusedPane::FolderTree => FocusedPane::ContentPreview,
            FocusedPane::MessageList => FocusedPane::FolderTree,
            FocusedPane::ContentPreview => FocusedPane::MessageList,
        };
    }

    pub fn focused_pane(&self) -> FocusedPane {
        self.focused_pane
    }

    // Accessors for pane components
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
}

impl Default for UI {
    fn default() -> Self {
        Self::new()
    }
}