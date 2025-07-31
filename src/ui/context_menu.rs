/// Context menu system for context-aware actions
/// 
/// Provides right-click and key-triggered context menus with actions appropriate
/// for the current UI context (email messages, folders, calendar events, etc.).

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem},
    Frame,
};

use crate::theme::Theme;
use crate::ui::typography::{TypographySystem, TypographyLevel};

/// Context menu action types
#[derive(Debug, Clone, PartialEq)]
pub enum ContextMenuAction {
    // Email actions
    ReplyToMessage,
    ReplyAllToMessage,
    ForwardMessage,
    DeleteMessage,
    MarkAsRead,
    MarkAsUnread,
    MoveToFolder(String),
    CopyMessage,
    ExportMessage,
    ViewMessageSource,
    
    // Folder actions
    CreateFolder,
    RenameFolder,
    DeleteFolder,
    MarkAllAsRead,
    CompactFolder,
    RefreshFolder,
    
    // Calendar actions
    CreateEvent,
    EditEvent,
    DeleteEvent,
    DuplicateEvent,
    ExportEvent,
    ViewEventDetails,
    
    // Account actions
    RefreshAccount,
    AccountSettings,
    AddAccount,
    RemoveAccount,
    
    // General actions
    Copy,
    Cut,
    Paste,
    SelectAll,
    Properties,
    Cancel,
}

/// Context menu item with label, action, and optional metadata
#[derive(Debug, Clone)]
pub struct ContextMenuItem {
    pub label: String,
    pub action: ContextMenuAction,
    pub shortcut: Option<String>,
    pub icon: Option<String>,
    pub enabled: bool,
    pub separator_after: bool,
}

impl ContextMenuItem {
    /// Create a new context menu item
    pub fn new(label: String, action: ContextMenuAction) -> Self {
        Self {
            label,
            action,
            shortcut: None,
            icon: None,
            enabled: true,
            separator_after: false,
        }
    }

    /// Add keyboard shortcut display
    pub fn with_shortcut(mut self, shortcut: String) -> Self {
        self.shortcut = Some(shortcut);
        self
    }

    /// Add icon
    pub fn with_icon(mut self, icon: String) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Set enabled state
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Add separator after this item
    pub fn with_separator(mut self) -> Self {
        self.separator_after = true;
        self
    }
}

/// Context types for determining appropriate menu items
#[derive(Debug, Clone, PartialEq)]
pub enum ContextType {
    /// Context menu for an email message
    EmailMessage { 
        is_read: bool, 
        is_draft: bool, 
        has_attachments: bool,
        folder_name: String,
    },
    /// Context menu for a folder
    EmailFolder { 
        folder_name: String, 
        is_special: bool, 
        unread_count: usize,
    },
    /// Context menu for calendar event
    CalendarEvent { 
        event_id: String, 
        is_recurring: bool, 
        is_editable: bool,
    },
    /// Context menu for account
    Account { 
        account_id: String, 
        is_online: bool,
    },
    /// Context menu for empty space or general context
    General,
}

/// Context menu component
pub struct ContextMenu {
    /// Whether the context menu is visible
    visible: bool,
    /// Position where the menu should appear
    position: (u16, u16),
    /// Menu items to display
    items: Vec<ContextMenuItem>,
    /// Currently selected item index
    selected_index: usize,
    /// Context type that triggered this menu
    context_type: Option<ContextType>,
    /// Maximum width for the menu
    max_width: u16,
    /// Whether menu was triggered by keyboard (vs mouse)
    keyboard_triggered: bool,
}

impl ContextMenu {
    /// Create a new context menu
    pub fn new() -> Self {
        Self {
            visible: false,
            position: (0, 0),
            items: Vec::new(),
            selected_index: 0,
            context_type: None,
            max_width: 30,
            keyboard_triggered: false,
        }
    }

    /// Show context menu at specific position with items for given context
    pub fn show_at_position(&mut self, x: u16, y: u16, context_type: ContextType) {
        self.position = (x, y);
        self.context_type = Some(context_type.clone());
        self.items = self.build_context_items(&context_type);
        self.selected_index = 0;
        self.visible = true;
        self.keyboard_triggered = false;
    }

    /// Show context menu at cursor position (keyboard triggered)
    pub fn show_at_cursor(&mut self, context_type: ContextType) {
        // Position will be calculated based on current cursor position
        self.position = (0, 0); // Will be adjusted during render
        self.context_type = Some(context_type.clone());
        self.items = self.build_context_items(&context_type);
        self.selected_index = 0;
        self.visible = true;
        self.keyboard_triggered = true;
    }

    /// Hide context menu
    pub fn hide(&mut self) {
        self.visible = false;
        self.items.clear();
        self.context_type = None;
    }

    /// Check if context menu is visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Navigate to next menu item
    pub fn next_item(&mut self) {
        if !self.items.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.items.len();
            // Skip disabled items
            while !self.items[self.selected_index].enabled {
                self.selected_index = (self.selected_index + 1) % self.items.len();
            }
        }
    }

    /// Navigate to previous menu item
    pub fn previous_item(&mut self) {
        if !self.items.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.items.len() - 1
            } else {
                self.selected_index - 1
            };
            // Skip disabled items
            while !self.items[self.selected_index].enabled {
                self.selected_index = if self.selected_index == 0 {
                    self.items.len() - 1
                } else {
                    self.selected_index - 1
                };
            }
        }
    }

    /// Get currently selected action
    pub fn selected_action(&self) -> Option<&ContextMenuAction> {
        self.items.get(self.selected_index).map(|item| &item.action)
    }

    /// Handle key input for navigation
    pub fn handle_key(&mut self, key: crossterm::event::KeyCode) -> Option<ContextMenuAction> {
        if !self.visible {
            return None;
        }

        match key {
            crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k') => {
                self.previous_item();
                None
            }
            crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j') => {
                self.next_item();
                None
            }
            crossterm::event::KeyCode::Enter => {
                let action = self.selected_action().cloned();
                self.hide();
                action
            }
            crossterm::event::KeyCode::Esc => {
                self.hide();
                None
            }
            _ => None,
        }
    }

    /// Handle mouse click at specific position
    pub fn handle_mouse_click(&mut self, x: u16, y: u16) -> Option<ContextMenuAction> {
        if !self.visible {
            return None;
        }

        // Calculate menu bounds and check if click is within menu
        let menu_rect = self.calculate_menu_rect(Rect::new(0, 0, 100, 100)); // Dummy area for calculation
        
        if x >= menu_rect.x && x < menu_rect.x + menu_rect.width &&
           y >= menu_rect.y && y < menu_rect.y + menu_rect.height {
            // Click is within menu - calculate which item was clicked
            let item_y = y.saturating_sub(menu_rect.y + 1); // +1 for border
            if (item_y as usize) < self.items.len() {
                self.selected_index = item_y as usize;
                let action = self.selected_action().cloned();
                self.hide();
                return action;
            }
        } else {
            // Click outside menu - hide it
            self.hide();
        }

        None
    }

    /// Build context menu items based on context type
    fn build_context_items(&self, context_type: &ContextType) -> Vec<ContextMenuItem> {
        match context_type {
            ContextType::EmailMessage { is_read, is_draft, has_attachments, folder_name } => {
                self.build_email_message_items(*is_read, *is_draft, *has_attachments, folder_name)
            }
            ContextType::EmailFolder { folder_name, is_special, unread_count } => {
                self.build_email_folder_items(folder_name, *is_special, *unread_count)
            }
            ContextType::CalendarEvent { event_id: _, is_recurring, is_editable } => {
                self.build_calendar_event_items(*is_recurring, *is_editable)
            }
            ContextType::Account { account_id: _, is_online } => {
                self.build_account_items(*is_online)
            }
            ContextType::General => {
                self.build_general_items()
            }
        }
    }

    /// Build context menu items for email messages
    fn build_email_message_items(&self, is_read: bool, is_draft: bool, has_attachments: bool, folder_name: &str) -> Vec<ContextMenuItem> {
        let mut items = Vec::new();

        if is_draft {
            items.push(ContextMenuItem::new("Edit Draft".to_string(), ContextMenuAction::ReplyToMessage)
                .with_icon("📝".to_string())
                .with_shortcut("E".to_string()));
        } else {
            items.push(ContextMenuItem::new("Reply".to_string(), ContextMenuAction::ReplyToMessage)
                .with_icon("↩️".to_string())
                .with_shortcut("R".to_string()));
            
            items.push(ContextMenuItem::new("Reply All".to_string(), ContextMenuAction::ReplyAllToMessage)
                .with_icon("↩️".to_string())
                .with_shortcut("Shift+R".to_string()));
        }

        items.push(ContextMenuItem::new("Forward".to_string(), ContextMenuAction::ForwardMessage)
            .with_icon("➡️".to_string())
            .with_shortcut("F".to_string())
            .with_separator());

        // Read/Unread toggle
        if is_read {
            items.push(ContextMenuItem::new("Mark as Unread".to_string(), ContextMenuAction::MarkAsUnread)
                .with_icon("📧".to_string())
                .with_shortcut("U".to_string()));
        } else {
            items.push(ContextMenuItem::new("Mark as Read".to_string(), ContextMenuAction::MarkAsRead)
                .with_icon("📖".to_string())
                .with_shortcut("R".to_string()));
        }

        items.push(ContextMenuItem::new("Move to Folder...".to_string(), ContextMenuAction::MoveToFolder(folder_name.to_string()))
            .with_icon("📁".to_string())
            .with_shortcut("M".to_string()));

        items.push(ContextMenuItem::new("Copy".to_string(), ContextMenuAction::Copy)
            .with_icon("📋".to_string())
            .with_shortcut("Ctrl+C".to_string())
            .with_separator());

        if has_attachments {
            items.push(ContextMenuItem::new("View Attachments".to_string(), ContextMenuAction::Properties)
                .with_icon("📎".to_string()));
        }

        items.push(ContextMenuItem::new("Export Message".to_string(), ContextMenuAction::ExportMessage)
            .with_icon("💾".to_string()));

        items.push(ContextMenuItem::new("View Source".to_string(), ContextMenuAction::ViewMessageSource)
            .with_icon("🔍".to_string()));

        items.push(ContextMenuItem::new("Delete".to_string(), ContextMenuAction::DeleteMessage)
            .with_icon("🗑️".to_string())
            .with_shortcut("Del".to_string())
            .with_separator());

        items.push(ContextMenuItem::new("Properties".to_string(), ContextMenuAction::Properties)
            .with_icon("ℹ️".to_string()));

        items
    }

    /// Build context menu items for email folders
    fn build_email_folder_items(&self, folder_name: &str, is_special: bool, unread_count: usize) -> Vec<ContextMenuItem> {
        let mut items = Vec::new();

        items.push(ContextMenuItem::new("Refresh Folder".to_string(), ContextMenuAction::RefreshFolder)
            .with_icon("🔄".to_string())
            .with_shortcut("F5".to_string()));

        if unread_count > 0 {
            items.push(ContextMenuItem::new("Mark All as Read".to_string(), ContextMenuAction::MarkAllAsRead)
                .with_icon("📖".to_string())
                .with_shortcut("Ctrl+Shift+R".to_string()));
        }

        items.push(ContextMenuItem::new("Compact Folder".to_string(), ContextMenuAction::CompactFolder)
            .with_icon("📦".to_string())
            .with_separator());

        if !is_special {
            items.push(ContextMenuItem::new("Create Subfolder".to_string(), ContextMenuAction::CreateFolder)
                .with_icon("📁".to_string())
                .with_shortcut("Ctrl+Shift+N".to_string()));

            items.push(ContextMenuItem::new(format!("Rename '{}'", folder_name), ContextMenuAction::RenameFolder)
                .with_icon("✏️".to_string())
                .with_shortcut("F2".to_string()));

            items.push(ContextMenuItem::new(format!("Delete '{}'", folder_name), ContextMenuAction::DeleteFolder)
                .with_icon("🗑️".to_string())
                .with_shortcut("Del".to_string())
                .with_separator());
        }

        items.push(ContextMenuItem::new("Properties".to_string(), ContextMenuAction::Properties)
            .with_icon("ℹ️".to_string()));

        items
    }

    /// Build context menu items for calendar events
    fn build_calendar_event_items(&self, is_recurring: bool, is_editable: bool) -> Vec<ContextMenuItem> {
        let mut items = Vec::new();

        items.push(ContextMenuItem::new("View Details".to_string(), ContextMenuAction::ViewEventDetails)
            .with_icon("👁️".to_string())
            .with_shortcut("Enter".to_string()));

        if is_editable {
            items.push(ContextMenuItem::new("Edit Event".to_string(), ContextMenuAction::EditEvent)
                .with_icon("✏️".to_string())
                .with_shortcut("E".to_string()));

            items.push(ContextMenuItem::new("Duplicate Event".to_string(), ContextMenuAction::DuplicateEvent)
                .with_icon("📄".to_string())
                .with_shortcut("Ctrl+D".to_string()));
        }

        items.push(ContextMenuItem::new("Export Event".to_string(), ContextMenuAction::ExportEvent)
            .with_icon("💾".to_string())
            .with_separator());

        if is_editable {
            let delete_text = if is_recurring {
                "Delete Event Series"
            } else {
                "Delete Event"
            };
            
            items.push(ContextMenuItem::new(delete_text.to_string(), ContextMenuAction::DeleteEvent)
                .with_icon("🗑️".to_string())
                .with_shortcut("Del".to_string()));
        }

        items
    }

    /// Build context menu items for accounts
    fn build_account_items(&self, is_online: bool) -> Vec<ContextMenuItem> {
        let mut items = Vec::new();

        items.push(ContextMenuItem::new("Refresh Account".to_string(), ContextMenuAction::RefreshAccount)
            .with_icon("🔄".to_string())
            .with_shortcut("F5".to_string()));

        items.push(ContextMenuItem::new("Account Settings".to_string(), ContextMenuAction::AccountSettings)
            .with_icon("⚙️".to_string())
            .with_shortcut("Ctrl+,".to_string())
            .with_separator());

        items.push(ContextMenuItem::new("Add Account".to_string(), ContextMenuAction::AddAccount)
            .with_icon("➕".to_string())
            .with_shortcut("Ctrl+N".to_string()));

        items.push(ContextMenuItem::new("Remove Account".to_string(), ContextMenuAction::RemoveAccount)
            .with_icon("➖".to_string())
            .enabled(is_online)); // Only allow removal if account is online/accessible

        items
    }

    /// Build general context menu items
    fn build_general_items(&self) -> Vec<ContextMenuItem> {
        vec![
            ContextMenuItem::new("Refresh".to_string(), ContextMenuAction::RefreshAccount)
                .with_icon("🔄".to_string())
                .with_shortcut("F5".to_string()),
            
            ContextMenuItem::new("Select All".to_string(), ContextMenuAction::SelectAll)
                .with_icon("☑️".to_string())
                .with_shortcut("Ctrl+A".to_string())
                .with_separator(),
            
            ContextMenuItem::new("Properties".to_string(), ContextMenuAction::Properties)
                .with_icon("ℹ️".to_string()),
        ]
    }

    /// Calculate menu rectangle based on position and content
    fn calculate_menu_rect(&self, area: Rect) -> Rect {
        let menu_height = self.items.len() as u16 + 2; // +2 for borders
        let menu_width = self.calculate_menu_width().min(self.max_width);

        let mut x = self.position.0;
        let mut y = self.position.1;

        // Adjust position to keep menu within bounds
        if x + menu_width > area.width {
            x = area.width.saturating_sub(menu_width);
        }
        if y + menu_height > area.height {
            y = area.height.saturating_sub(menu_height);
        }

        Rect::new(x, y, menu_width, menu_height)
    }

    /// Calculate required width for menu based on content
    fn calculate_menu_width(&self) -> u16 {
        let mut max_width = 10; // Minimum width

        for item in &self.items {
            let mut item_width = item.label.len() as u16;
            
            if let Some(ref icon) = item.icon {
                item_width += icon.chars().count() as u16 + 1; // +1 for space
            }
            
            if let Some(ref shortcut) = item.shortcut {
                item_width += shortcut.len() as u16 + 3; // +3 for spacing
            }

            max_width = max_width.max(item_width + 4); // +4 for padding
        }

        max_width
    }

    /// Render the context menu
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme, typography: &TypographySystem) {
        if !self.visible || self.items.is_empty() {
            return;
        }

        let menu_rect = self.calculate_menu_rect(area);

        // Clear the background
        frame.render_widget(Clear, menu_rect);

        // Create menu block
        let menu_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.colors.palette.accent))
            .style(Style::default().bg(theme.colors.palette.overlay));

        let inner_area = menu_block.inner(menu_rect);
        frame.render_widget(menu_block, menu_rect);

        // Create menu items
        let mut list_items = Vec::new();
        
        for (i, item) in self.items.iter().enumerate() {
            let is_selected = i == self.selected_index;
            
            let mut spans = Vec::new();

            // Add icon if present
            if let Some(ref icon) = item.icon {
                spans.push(Span::styled(
                    format!("{} ", icon),
                    if item.enabled {
                        Style::default().fg(theme.colors.palette.accent)
                    } else {
                        Style::default().fg(theme.colors.palette.text_muted)
                    }
                ));
            }

            // Add label
            let label_style = if is_selected && item.enabled {
                Style::default()
                    .fg(theme.colors.palette.background)
                    .bg(theme.colors.palette.accent)
                    .add_modifier(Modifier::BOLD)
            } else if item.enabled {
                typography.get_typography_style(TypographyLevel::Body, theme)
            } else {
                Style::default().fg(theme.colors.palette.text_muted)
            };

            spans.push(Span::styled(item.label.clone(), label_style));

            // Add shortcut if present
            if let Some(ref shortcut) = item.shortcut {
                spans.push(Span::styled(
                    format!(" {}", shortcut),
                    if is_selected && item.enabled {
                        Style::default()
                            .fg(theme.colors.palette.background)
                            .bg(theme.colors.palette.accent)
                    } else {
                        Style::default().fg(theme.colors.palette.text_muted)
                    }
                ));
            }

            let line = Line::from(spans);
            list_items.push(ListItem::new(vec![line]));

            // Add separator if requested
            if item.separator_after {
                let separator_line = Line::from(vec![
                    Span::styled(
                        "─".repeat(inner_area.width as usize),
                        Style::default().fg(theme.colors.palette.text_muted)
                    )
                ]);
                list_items.push(ListItem::new(vec![separator_line]));
            }
        }

        let menu_list = List::new(list_items);
        frame.render_widget(menu_list, inner_area);
    }
}

impl Default for ContextMenu {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_menu_creation() {
        let menu = ContextMenu::new();
        assert!(!menu.is_visible());
        assert_eq!(menu.selected_index, 0);
    }

    #[test]
    fn test_email_message_context() {
        let mut menu = ContextMenu::new();
        let context = ContextType::EmailMessage {
            is_read: false,
            is_draft: false,
            has_attachments: true,
            folder_name: "INBOX".to_string(),
        };

        menu.show_at_position(10, 10, context);
        assert!(menu.is_visible());
        assert!(!menu.items.is_empty());
        
        // Should have reply actions for non-draft messages
        let has_reply = menu.items.iter().any(|item| 
            matches!(item.action, ContextMenuAction::ReplyToMessage)
        );
        assert!(has_reply);
    }

    #[test]
    fn test_menu_navigation() {
        let mut menu = ContextMenu::new();
        let context = ContextType::General;
        
        menu.show_at_position(0, 0, context);
        assert_eq!(menu.selected_index, 0);
        
        menu.next_item();
        assert!(menu.selected_index > 0 || menu.items.len() <= 1);
        
        menu.previous_item();
        // Should wrap around or go back
    }

    #[test]
    fn test_context_menu_actions() {
        let mut menu = ContextMenu::new();
        let context = ContextType::EmailFolder {
            folder_name: "Test".to_string(),
            is_special: false,
            unread_count: 5,
        };

        menu.show_at_position(0, 0, context);
        
        // Should have mark all as read when there are unread messages
        let has_mark_all_read = menu.items.iter().any(|item|
            matches!(item.action, ContextMenuAction::MarkAllAsRead)
        );
        assert!(has_mark_all_read);
    }
}