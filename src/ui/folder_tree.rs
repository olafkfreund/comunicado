use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState},
    Frame,
};
use crate::theme::Theme;

#[derive(Debug, Clone)]
pub struct FolderItem {
    pub name: String,
    pub path: String,
    pub is_expanded: bool,
    pub has_children: bool,
    pub depth: usize,
    pub unread_count: usize,
}

impl FolderItem {
    pub fn new(name: String, path: String, depth: usize) -> Self {
        Self {
            name,
            path,
            is_expanded: false,
            has_children: false,
            depth,
            unread_count: 0,
        }
    }

    pub fn with_children(mut self) -> Self {
        self.has_children = true;
        self
    }

    pub fn with_unread_count(mut self, count: usize) -> Self {
        self.unread_count = count;
        self
    }
}

pub struct FolderTree {
    folders: Vec<FolderItem>,
    state: ListState,
}

impl FolderTree {
    pub fn new() -> Self {
        let mut tree = Self {
            folders: Vec::new(),
            state: ListState::default(),
        };
        
        // Initialize with sample folder structure
        tree.initialize_sample_folders();
        tree.state.select(Some(0));
        
        tree
    }

    fn initialize_sample_folders(&mut self) {
        self.folders = vec![
            FolderItem::new("Inbox".to_string(), "INBOX".to_string(), 0)
                .with_unread_count(5),
            FolderItem::new("Sent".to_string(), "INBOX/Sent".to_string(), 0),
            FolderItem::new("Drafts".to_string(), "INBOX/Drafts".to_string(), 0)
                .with_unread_count(2),
            FolderItem::new("Trash".to_string(), "INBOX/Trash".to_string(), 0),
            FolderItem::new("Work".to_string(), "INBOX/Work".to_string(), 0)
                .with_children()
                .with_unread_count(3),
            FolderItem::new("Projects".to_string(), "INBOX/Work/Projects".to_string(), 1)
                .with_unread_count(1),
            FolderItem::new("Team".to_string(), "INBOX/Work/Team".to_string(), 1),
            FolderItem::new("Personal".to_string(), "INBOX/Personal".to_string(), 0)
                .with_children(),
            FolderItem::new("Family".to_string(), "INBOX/Personal/Family".to_string(), 1),
            FolderItem::new("Important".to_string(), "INBOX/Important".to_string(), 0)
                .with_unread_count(1),
        ];
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, block: Block, is_focused: bool, theme: &Theme) {
        let items: Vec<ListItem> = self.folders
            .iter()
            .enumerate()
            .map(|(i, folder)| {
                let indent = "  ".repeat(folder.depth);
                let expand_icon = if folder.has_children {
                    if folder.is_expanded { "▼ " } else { "▶ " }
                } else {
                    "  "
                };
                
                let unread_indicator = if folder.unread_count > 0 {
                    format!(" ({})", folder.unread_count)
                } else {
                    String::new()
                };

                let is_selected = self.state.selected() == Some(i);
                let style = if is_selected && is_focused {
                    theme.styles.get_selected_style("folder_tree", &theme.colors)
                } else if is_selected {
                    theme.styles.get_selected_style("folder_tree", &theme.colors)
                } else if folder.unread_count > 0 {
                    Style::default()
                        .fg(theme.colors.folder_tree.folder_unread)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.colors.folder_tree.folder_normal)
                };

                let line = Line::from(vec![
                    Span::styled(indent, Style::default().fg(theme.colors.folder_tree.expand_icon)),
                    Span::styled(expand_icon, Style::default().fg(theme.colors.folder_tree.expand_icon)),
                    Span::styled(folder.name.clone(), style),
                    Span::styled(unread_indicator, Style::default().fg(theme.colors.folder_tree.count_badge)),
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
                    Some(self.folders.len() - 1)
                }
            }
            None => Some(0),
        };
        self.state.select(selected);
    }

    pub fn handle_down(&mut self) {
        let selected = match self.state.selected() {
            Some(i) => {
                if i < self.folders.len() - 1 {
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
            if let Some(folder) = self.folders.get_mut(selected) {
                if folder.has_children {
                    folder.is_expanded = true;
                }
            }
        }
    }

    pub fn handle_left(&mut self) {
        if let Some(selected) = self.state.selected() {
            if let Some(folder) = self.folders.get_mut(selected) {
                if folder.has_children && folder.is_expanded {
                    folder.is_expanded = false;
                }
            }
        }
    }

    pub fn handle_enter(&mut self) {
        if let Some(selected) = self.state.selected() {
            if let Some(folder) = self.folders.get_mut(selected) {
                if folder.has_children {
                    folder.is_expanded = !folder.is_expanded;
                }
                // In the future, this will also trigger loading emails from the selected folder
            }
        }
    }

    pub fn selected_folder(&self) -> Option<&FolderItem> {
        self.state.selected().and_then(|i| self.folders.get(i))
    }
}

impl Default for FolderTree {
    fn default() -> Self {
        Self::new()
    }
}