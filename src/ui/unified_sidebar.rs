//! Unified Sidebar for Email and Calendar Navigation
//! 
//! Combines email folder tree and calendar list into a single,
//! cohesive navigation experience.

use crate::{
    theme::Theme,
    ui::{AccountSyncStatus, FolderTree},
    calendar::Calendar,
};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};
use std::collections::HashMap;

/// Navigation sections in the unified sidebar
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SidebarSection {
    Accounts,
    EmailFolders,
    Calendars,
    QuickActions,
}

/// Navigation item types
#[derive(Debug, Clone)]
pub enum NavigationItem {
    Account {
        id: String,
        name: String,
        status: AccountSyncStatus,
        unread_count: u32,
    },
    EmailFolder {
        path: String,
        name: String,
        unread_count: u32,
        has_children: bool,
        depth: usize,
    },
    Calendar {
        id: String,
        name: String,
        color: Color,
        enabled: bool,
        event_count: u32,
    },
    Separator {
        label: String,
    },
    QuickAction {
        label: String,
        icon: String,
        action: QuickActionType,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickActionType {
    ComposeEmail,
    CreateEvent,
    RefreshAll,
    SyncCalendars,
    ImportCalendar,
    Settings,
}

/// Unified sidebar state and rendering
pub struct UnifiedSidebar {
    /// All navigation items in display order
    items: Vec<NavigationItem>,
    /// Current selection state
    list_state: ListState,
    /// Which section is currently expanded
    expanded_sections: HashMap<SidebarSection, bool>,
    /// Current focus within sidebar
    focused_section: SidebarSection,
    /// Folder tree integration
    #[allow(dead_code)]
    folder_tree: FolderTree,
    /// Whether calendar section is visible
    show_calendars: bool,
}

impl UnifiedSidebar {
    pub fn new() -> Self {
        let mut expanded_sections = HashMap::new();
        expanded_sections.insert(SidebarSection::Accounts, true);
        expanded_sections.insert(SidebarSection::EmailFolders, true);
        expanded_sections.insert(SidebarSection::Calendars, true);
        expanded_sections.insert(SidebarSection::QuickActions, false);

        Self {
            items: Vec::new(),
            list_state: ListState::default(),
            expanded_sections,
            focused_section: SidebarSection::EmailFolders,
            folder_tree: FolderTree::new(),
            show_calendars: true,
        }
    }

    /// Update sidebar with current data
    pub fn update_data(
        &mut self, 
        accounts: &[(String, String, AccountSyncStatus, u32)], // (id, name, status, unread)
        folders: &[(String, String, u32, bool, usize)], // (path, name, unread, has_children, depth)
        calendars: &[Calendar],
    ) {
        self.items.clear();

        // Add accounts section
        if self.is_section_expanded(SidebarSection::Accounts) {
            self.items.push(NavigationItem::Separator { 
                label: "📧 ACCOUNTS".to_string() 
            });
            
            for (id, name, status, unread_count) in accounts {
                self.items.push(NavigationItem::Account {
                    id: id.clone(),
                    name: name.clone(),
                    status: *status,
                    unread_count: *unread_count,
                });
            }
        }

        // Add email folders section
        if self.is_section_expanded(SidebarSection::EmailFolders) {
            self.items.push(NavigationItem::Separator { 
                label: "📁 FOLDERS".to_string() 
            });
            
            for (path, name, unread_count, has_children, depth) in folders {
                self.items.push(NavigationItem::EmailFolder {
                    path: path.clone(),
                    name: name.clone(),
                    unread_count: *unread_count,
                    has_children: *has_children,
                    depth: *depth,
                });
            }
        }

        // Add calendars section
        if self.show_calendars && self.is_section_expanded(SidebarSection::Calendars) {
            self.items.push(NavigationItem::Separator { 
                label: "📅 CALENDARS".to_string() 
            });
            
            for calendar in calendars {
                self.items.push(NavigationItem::Calendar {
                    id: calendar.id.clone(),
                    name: calendar.name.clone(),
                    color: self.parse_calendar_color(&calendar.color.as_deref().unwrap_or("blue")),
                    enabled: !calendar.read_only, // Use read_only as inverse of enabled
                    event_count: 0, // TODO: Calculate event count from calendar data
                });
            }
        }

        // Add quick actions section
        if self.is_section_expanded(SidebarSection::QuickActions) {
            self.items.push(NavigationItem::Separator { 
                label: "⚡ QUICK ACTIONS".to_string() 
            });
            
            self.items.extend([
                NavigationItem::QuickAction {
                    label: "Compose Email".to_string(),
                    icon: "✉️".to_string(),
                    action: QuickActionType::ComposeEmail,
                },
                NavigationItem::QuickAction {
                    label: "Create Event".to_string(),
                    icon: "📅".to_string(),
                    action: QuickActionType::CreateEvent,
                },
                NavigationItem::QuickAction {
                    label: "Refresh All".to_string(),
                    icon: "🔄".to_string(),
                    action: QuickActionType::RefreshAll,
                },
                NavigationItem::QuickAction {
                    label: "Sync Calendars".to_string(),
                    icon: "⚡".to_string(),
                    action: QuickActionType::SyncCalendars,
                },
            ]);
        }
    }

    /// Render the unified sidebar
    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .title(" Navigation ")
            .borders(Borders::ALL)
            .border_style(theme.get_component_style("border", false));

        let inner_area = block.inner(area);
        frame.render_widget(block, area);

        // Convert items to ratatui ListItems
        let list_items: Vec<ListItem> = self.items.iter().map(|item| {
            match item {
                NavigationItem::Account { name, status, unread_count, .. } => {
                    let status_icon = match status {
                        AccountSyncStatus::Online => "🟢",
                        AccountSyncStatus::Syncing => "🟡",
                        AccountSyncStatus::Error => "🔴",
                        AccountSyncStatus::Offline => "⚫",
                    };
                    
                    let unread_text = if *unread_count > 0 {
                        format!(" ({})", unread_count)
                    } else {
                        String::new()
                    };

                    ListItem::new(Line::from(vec![
                        Span::raw(status_icon),
                        Span::raw(" "),
                        Span::styled(name, Style::default().add_modifier(Modifier::BOLD)),
                        Span::styled(unread_text, Style::default().fg(theme.colors.palette.accent)),
                    ]))
                },
                
                NavigationItem::EmailFolder { name, unread_count, depth, .. } => {
                    let indent = "  ".repeat(*depth);
                    let folder_icon = if *unread_count > 0 { "📬" } else { "📁" };
                    
                    let unread_text = if *unread_count > 0 {
                        format!(" ({})", unread_count)
                    } else {
                        String::new()
                    };

                    ListItem::new(Line::from(vec![
                        Span::raw(indent),
                        Span::raw(folder_icon),
                        Span::raw(" "),
                        Span::styled(name, Style::default()),
                        Span::styled(unread_text, Style::default().fg(theme.colors.palette.accent)),
                    ]))
                },
                
                NavigationItem::Calendar { name, color, enabled, event_count, .. } => {
                    let calendar_icon = if *enabled { "📅" } else { "📋" };
                    let event_text = if *event_count > 0 {
                        format!(" ({})", event_count)
                    } else {
                        String::new()
                    };

                    let style = if *enabled {
                        Style::default().fg(*color)
                    } else {
                        Style::default().fg(theme.colors.palette.text_muted)
                    };

                    ListItem::new(Line::from(vec![
                        Span::raw("  "),
                        Span::raw(calendar_icon),
                        Span::raw(" "),
                        Span::styled(name, style),
                        Span::styled(event_text, Style::default().fg(theme.colors.palette.accent)),
                    ]))
                },
                
                NavigationItem::Separator { label } => {
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            label, 
                            Style::default()
                                .fg(theme.colors.palette.text_muted)
                                .add_modifier(Modifier::BOLD)
                        ),
                    ]))
                },
                
                NavigationItem::QuickAction { icon, label, .. } => {
                    ListItem::new(Line::from(vec![
                        Span::raw("  "),
                        Span::raw(icon),
                        Span::raw(" "),
                        Span::styled(label, Style::default().fg(theme.colors.palette.text_secondary)),
                    ]))
                },
            }
        }).collect();

        let list = List::new(list_items)
            .highlight_style(
                Style::default()
                    .bg(theme.colors.palette.accent)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            );

        frame.render_stateful_widget(list, inner_area, &mut self.list_state);
    }

    /// Handle navigation input
    pub fn handle_key(&mut self, key: crossterm::event::KeyCode) -> Option<SidebarAction> {
        match key {
            crossterm::event::KeyCode::Up => {
                self.previous_item();
                None
            },
            crossterm::event::KeyCode::Down => {
                self.next_item();
                None
            },
            crossterm::event::KeyCode::Enter => {
                self.activate_current_item()
            },
            crossterm::event::KeyCode::Char(' ') => {
                self.toggle_current_item()
            },
            crossterm::event::KeyCode::Tab => {
                self.next_section();
                None
            },
            _ => None,
        }
    }

    /// Move to previous item
    fn previous_item(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if selected > 0 {
                self.list_state.select(Some(selected - 1));
            } else {
                self.list_state.select(Some(self.items.len().saturating_sub(1)));
            }
        } else {
            self.list_state.select(Some(0));
        }
    }

    /// Move to next item
    fn next_item(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if selected < self.items.len().saturating_sub(1) {
                self.list_state.select(Some(selected + 1));
            } else {
                self.list_state.select(Some(0));
            }
        } else {
            self.list_state.select(Some(0));
        }
    }

    /// Activate the currently selected item
    fn activate_current_item(&self) -> Option<SidebarAction> {
        if let Some(selected) = self.list_state.selected() {
            if let Some(item) = self.items.get(selected) {
                match item {
                    NavigationItem::Account { id, .. } => {
                        Some(SidebarAction::SwitchAccount(id.clone()))
                    },
                    NavigationItem::EmailFolder { path, .. } => {
                        Some(SidebarAction::SelectFolder(path.clone()))
                    },
                    NavigationItem::Calendar { id, .. } => {
                        Some(SidebarAction::SelectCalendar(id.clone()))
                    },
                    NavigationItem::QuickAction { action, .. } => {
                        Some(SidebarAction::QuickAction(*action))
                    },
                    NavigationItem::Separator { .. } => None,
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Toggle the current item (for calendars and expandable sections)
    fn toggle_current_item(&mut self) -> Option<SidebarAction> {
        if let Some(selected) = self.list_state.selected() {
            if let Some(item) = self.items.get(selected) {
                match item {
                    NavigationItem::Calendar { id, enabled, .. } => {
                        Some(SidebarAction::ToggleCalendar(id.clone(), !enabled))
                    },
                    _ => None,
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Move focus to next section
    fn next_section(&mut self) {
        self.focused_section = match self.focused_section {
            SidebarSection::Accounts => SidebarSection::EmailFolders,
            SidebarSection::EmailFolders => SidebarSection::Calendars,
            SidebarSection::Calendars => SidebarSection::QuickActions,
            SidebarSection::QuickActions => SidebarSection::Accounts,
        };
    }

    /// Check if a section is expanded
    fn is_section_expanded(&self, section: SidebarSection) -> bool {
        self.expanded_sections.get(&section).copied().unwrap_or(false)
    }

    /// Toggle section expansion
    pub fn toggle_section(&mut self, section: SidebarSection) {
        let current = self.is_section_expanded(section);
        self.expanded_sections.insert(section, !current);
    }

    /// Toggle calendar visibility
    pub fn toggle_calendar_visibility(&mut self) {
        self.show_calendars = !self.show_calendars;
    }

    /// Parse calendar color from string
    fn parse_calendar_color(&self, color_str: &str) -> Color {
        match color_str.to_lowercase().as_str() {
            "red" => Color::Red,
            "green" => Color::Green,
            "blue" => Color::Blue,
            "yellow" => Color::Yellow,
            "magenta" => Color::Magenta,
            "cyan" => Color::Cyan,
            _ => Color::White,
        }
    }

    /// Get currently selected item
    pub fn get_selected_item(&self) -> Option<&NavigationItem> {
        if let Some(selected) = self.list_state.selected() {
            self.items.get(selected)
        } else {
            None
        }
    }
}

/// Actions that can be triggered from the sidebar
#[derive(Debug, Clone)]
pub enum SidebarAction {
    SwitchAccount(String),
    SelectFolder(String),
    SelectCalendar(String),
    ToggleCalendar(String, bool),
    QuickAction(QuickActionType),
}

impl Default for UnifiedSidebar {
    fn default() -> Self {
        Self::new()
    }
}