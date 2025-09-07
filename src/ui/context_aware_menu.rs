/// Context-aware menu system triggered by Ctrl+D
///
/// Provides hierarchical menus that work reliably across all terminal environments,
/// replacing problematic F-key and Ctrl+Alt shortcuts that don't work in tmux/screen.
use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::theme::Theme;
use crate::ui::{FocusedPane, UIMode};

/// Menu item that can contain submenus or execute actions
#[derive(Debug, Clone)]
pub struct MenuItem {
    pub label: String,
    pub description: String,
    pub action: MenuAction,
    pub enabled: bool,
}

/// Actions that can be triggered from menu items
#[derive(Debug, Clone)]
pub enum MenuAction {
    /// Execute a specific action
    Execute(ContextAction),
    /// Open a submenu
    OpenSubmenu(Vec<MenuItem>),
    /// Close the current menu
    Close,
}

/// Context-aware actions that can be executed
#[derive(Debug, Clone)]
pub enum ContextAction {
    // Email actions
    ComposeNew,
    ReplyToMessage,
    ReplyAllToMessage,
    ForwardMessage,
    DeleteMessage,
    ArchiveMessage,
    MarkAsRead,
    MarkAsUnread,
    ShowDrafts,

    // Folder actions
    CreateFolder,
    DeleteFolder,
    RefreshFolder,
    MarkAllAsRead,

    // Calendar actions
    ShowCalendar,
    CreateEvent,
    EditEvent,
    DeleteEvent,
    NextMonth,
    PrevMonth,
    TodayView,
    DayView,
    WeekView,
    MonthView,
    AgendaView,

    // Contacts actions
    ShowContacts,
    CreateContact,
    EditContact,
    DeleteContact,
    SyncContacts,

    // Account actions
    AddAccount,
    RemoveAccount,
    RefreshAccount,
    AccountSettings,

    // View actions
    ShowSearch,
    ShowSettings,
    ShowKeyboardShortcuts,
    ToggleThreadedView,

    // AI actions
    AISummarizeEmail,
    AIComposeAssist,
    AIQuickReply,
    AICalendarAssist,
}

/// Context-aware menu state
pub struct ContextAwareMenu {
    /// Current menu stack (for hierarchical navigation)
    menu_stack: Vec<Vec<MenuItem>>,
    /// Currently selected item index
    selected_index: usize,
    /// Whether the menu is visible
    visible: bool,
    /// Current UI context for menu generation
    current_context: MenuContext,
}

/// Context information for generating appropriate menus
#[derive(Debug, Clone, PartialEq)]
pub struct MenuContext {
    pub ui_mode: UIMode,
    pub focused_pane: FocusedPane,
    pub has_selected_message: bool,
    pub has_selected_folder: bool,
    pub has_selected_event: bool,
    pub has_selected_contact: bool,
    pub is_composing: bool,
}

impl Default for ContextAwareMenu {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextAwareMenu {
    pub fn new() -> Self {
        Self {
            menu_stack: Vec::new(),
            selected_index: 0,
            visible: false,
            current_context: MenuContext {
                ui_mode: UIMode::Normal,
                focused_pane: FocusedPane::MessageList,
                has_selected_message: false,
                has_selected_folder: false,
                has_selected_event: false,
                has_selected_contact: false,
                is_composing: false,
            },
        }
    }

    /// Show the context-aware menu for the current context
    pub fn show(&mut self, context: MenuContext) {
        tracing::debug!("Context menu: showing menu with context: {:?}", context);
        self.current_context = context;
        self.menu_stack.clear();
        self.menu_stack.push(self.generate_main_menu());
        self.selected_index = 0;
        self.visible = true;
        tracing::debug!("Context menu: menu is now visible");
    }

    /// Hide the menu
    pub fn hide(&mut self) {
        tracing::debug!("Context menu: hiding menu");
        self.visible = false;
        self.menu_stack.clear();
        self.selected_index = 0;
        tracing::debug!("Context menu: menu is now hidden");
    }

    /// Check if menu is visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Navigate up in the current menu
    pub fn navigate_up(&mut self) {
        if let Some(current_menu) = self.menu_stack.last() {
            if self.selected_index > 0 {
                self.selected_index -= 1;
            } else {
                self.selected_index = current_menu.len().saturating_sub(1);
            }
        }
    }

    /// Navigate down in the current menu
    pub fn navigate_down(&mut self) {
        if let Some(current_menu) = self.menu_stack.last() {
            if self.selected_index < current_menu.len().saturating_sub(1) {
                self.selected_index += 1;
            } else {
                self.selected_index = 0;
            }
        }
    }

    /// Navigate back to parent menu
    pub fn navigate_back(&mut self) -> bool {
        if self.menu_stack.len() > 1 {
            self.menu_stack.pop();
            self.selected_index = 0;
            true
        } else {
            false
        }
    }

    /// Select the current menu item
    pub fn select_current(&mut self) -> Option<ContextAction> {
        let selected_index = self.selected_index;

        if let Some(current_menu) = self.menu_stack.last().cloned() {
            if let Some(item) = current_menu.get(selected_index) {
                if !item.enabled {
                    return None;
                }

                match &item.action {
                    MenuAction::Execute(action) => {
                        let action_clone = action.clone();
                        self.hide();
                        Some(action_clone)
                    }
                    MenuAction::OpenSubmenu(submenu) => {
                        let submenu_clone = submenu.clone();
                        self.menu_stack.push(submenu_clone);
                        self.selected_index = 0;
                        None
                    }
                    MenuAction::Close => {
                        self.hide();
                        None
                    }
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Generate the main context-aware menu
    fn generate_main_menu(&self) -> Vec<MenuItem> {
        let mut menu = Vec::new();

        // Context-aware main categories
        match self.current_context.ui_mode {
            UIMode::Normal => match self.current_context.focused_pane {
                FocusedPane::MessageList => {
                    menu.extend(self.generate_email_menu());
                    menu.extend(self.generate_view_menu());
                }
                FocusedPane::FolderTree => {
                    menu.extend(self.generate_folder_menu());
                    menu.extend(self.generate_email_menu());
                }
                FocusedPane::AccountSwitcher => {
                    menu.extend(self.generate_account_menu());
                    menu.extend(self.generate_email_menu());
                }
                _ => {
                    menu.extend(self.generate_email_menu());
                }
            },
            UIMode::Calendar => {
                menu.extend(self.generate_calendar_menu());
                menu.extend(self.generate_view_menu());
            }
            UIMode::Contacts => {
                menu.extend(self.generate_contacts_menu());
                menu.extend(self.generate_view_menu());
            }
            UIMode::EmailViewer => {
                menu.extend(self.generate_email_viewer_menu());
            }
            _ => {
                menu.extend(self.generate_general_menu());
            }
        }

        // Always include navigation options
        menu.push(MenuItem {
            label: "Navigation".to_string(),
            description: "Switch between views and modes".to_string(),
            action: MenuAction::OpenSubmenu(self.generate_navigation_menu()),
            enabled: true,
        });

        // Always include settings
        menu.push(MenuItem {
            label: "Settings & Help".to_string(),
            description: "Configuration and help options".to_string(),
            action: MenuAction::OpenSubmenu(self.generate_settings_menu()),
            enabled: true,
        });

        menu
    }

    /// Generate email-specific menu items
    fn generate_email_menu(&self) -> Vec<MenuItem> {
        let mut menu = Vec::new();

        menu.push(MenuItem {
            label: "Compose New Email".to_string(),
            description: "Create and send a new email message".to_string(),
            action: MenuAction::Execute(ContextAction::ComposeNew),
            enabled: true,
        });

        if self.current_context.has_selected_message {
            menu.push(MenuItem {
                label: "Message Actions".to_string(),
                description: "Actions for the selected message".to_string(),
                action: MenuAction::OpenSubmenu(self.generate_message_actions_menu()),
                enabled: true,
            });
        }

        menu.push(MenuItem {
            label: "Show Drafts".to_string(),
            description: "View and manage draft messages".to_string(),
            action: MenuAction::Execute(ContextAction::ShowDrafts),
            enabled: true,
        });

        menu
    }

    /// Generate message actions submenu
    fn generate_message_actions_menu(&self) -> Vec<MenuItem> {
        vec![
            MenuItem {
                label: "Reply".to_string(),
                description: "Reply to sender".to_string(),
                action: MenuAction::Execute(ContextAction::ReplyToMessage),
                enabled: true,
            },
            MenuItem {
                label: "Reply All".to_string(),
                description: "Reply to all recipients".to_string(),
                action: MenuAction::Execute(ContextAction::ReplyAllToMessage),
                enabled: true,
            },
            MenuItem {
                label: "Forward".to_string(),
                description: "Forward message to others".to_string(),
                action: MenuAction::Execute(ContextAction::ForwardMessage),
                enabled: true,
            },
            MenuItem {
                label: "Delete".to_string(),
                description: "Delete this message".to_string(),
                action: MenuAction::Execute(ContextAction::DeleteMessage),
                enabled: true,
            },
            MenuItem {
                label: "Archive".to_string(),
                description: "Archive this message".to_string(),
                action: MenuAction::Execute(ContextAction::ArchiveMessage),
                enabled: true,
            },
            MenuItem {
                label: "Mark as Read".to_string(),
                description: "Mark message as read".to_string(),
                action: MenuAction::Execute(ContextAction::MarkAsRead),
                enabled: true,
            },
            MenuItem {
                label: "Mark as Unread".to_string(),
                description: "Mark message as unread".to_string(),
                action: MenuAction::Execute(ContextAction::MarkAsUnread),
                enabled: true,
            },
        ]
    }

    /// Generate folder-specific menu items
    fn generate_folder_menu(&self) -> Vec<MenuItem> {
        let mut menu = Vec::new();

        menu.push(MenuItem {
            label: "Folder Actions".to_string(),
            description: "Manage folders and organization".to_string(),
            action: MenuAction::OpenSubmenu(vec![
                MenuItem {
                    label: "Create Folder".to_string(),
                    description: "Create a new folder".to_string(),
                    action: MenuAction::Execute(ContextAction::CreateFolder),
                    enabled: true,
                },
                MenuItem {
                    label: "Delete Folder".to_string(),
                    description: "Delete selected folder".to_string(),
                    action: MenuAction::Execute(ContextAction::DeleteFolder),
                    enabled: self.current_context.has_selected_folder,
                },
                MenuItem {
                    label: "Refresh Folder".to_string(),
                    description: "Refresh folder contents".to_string(),
                    action: MenuAction::Execute(ContextAction::RefreshFolder),
                    enabled: true,
                },
                MenuItem {
                    label: "Mark All as Read".to_string(),
                    description: "Mark all messages in folder as read".to_string(),
                    action: MenuAction::Execute(ContextAction::MarkAllAsRead),
                    enabled: true,
                },
            ]),
            enabled: true,
        });

        menu
    }

    /// Generate calendar-specific menu items
    fn generate_calendar_menu(&self) -> Vec<MenuItem> {
        vec![
            MenuItem {
                label: "Create Event".to_string(),
                description: "Create a new calendar event".to_string(),
                action: MenuAction::Execute(ContextAction::CreateEvent),
                enabled: true,
            },
            MenuItem {
                label: "Calendar Views".to_string(),
                description: "Switch calendar view mode".to_string(),
                action: MenuAction::OpenSubmenu(vec![
                    MenuItem {
                        label: "Day View".to_string(),
                        description: "Show daily calendar view".to_string(),
                        action: MenuAction::Execute(ContextAction::DayView),
                        enabled: true,
                    },
                    MenuItem {
                        label: "Week View".to_string(),
                        description: "Show weekly calendar view".to_string(),
                        action: MenuAction::Execute(ContextAction::WeekView),
                        enabled: true,
                    },
                    MenuItem {
                        label: "Month View".to_string(),
                        description: "Show monthly calendar view".to_string(),
                        action: MenuAction::Execute(ContextAction::MonthView),
                        enabled: true,
                    },
                    MenuItem {
                        label: "Agenda View".to_string(),
                        description: "Show agenda list view".to_string(),
                        action: MenuAction::Execute(ContextAction::AgendaView),
                        enabled: true,
                    },
                ]),
                enabled: true,
            },
            MenuItem {
                label: "Calendar Navigation".to_string(),
                description: "Navigate calendar dates".to_string(),
                action: MenuAction::OpenSubmenu(vec![
                    MenuItem {
                        label: "Today".to_string(),
                        description: "Go to today's date".to_string(),
                        action: MenuAction::Execute(ContextAction::TodayView),
                        enabled: true,
                    },
                    MenuItem {
                        label: "Previous Month".to_string(),
                        description: "Go to previous month".to_string(),
                        action: MenuAction::Execute(ContextAction::PrevMonth),
                        enabled: true,
                    },
                    MenuItem {
                        label: "Next Month".to_string(),
                        description: "Go to next month".to_string(),
                        action: MenuAction::Execute(ContextAction::NextMonth),
                        enabled: true,
                    },
                ]),
                enabled: true,
            },
        ]
    }

    /// Generate contacts-specific menu items
    fn generate_contacts_menu(&self) -> Vec<MenuItem> {
        vec![
            MenuItem {
                label: "Create Contact".to_string(),
                description: "Add a new contact".to_string(),
                action: MenuAction::Execute(ContextAction::CreateContact),
                enabled: true,
            },
            MenuItem {
                label: "Sync Contacts".to_string(),
                description: "Synchronize contacts from all sources".to_string(),
                action: MenuAction::Execute(ContextAction::SyncContacts),
                enabled: true,
            },
        ]
    }

    /// Generate email viewer specific menu items
    fn generate_email_viewer_menu(&self) -> Vec<MenuItem> {
        vec![
            MenuItem {
                label: "Reply".to_string(),
                description: "Reply to this message".to_string(),
                action: MenuAction::Execute(ContextAction::ReplyToMessage),
                enabled: true,
            },
            MenuItem {
                label: "Reply All".to_string(),
                description: "Reply to all recipients".to_string(),
                action: MenuAction::Execute(ContextAction::ReplyAllToMessage),
                enabled: true,
            },
            MenuItem {
                label: "Forward".to_string(),
                description: "Forward this message".to_string(),
                action: MenuAction::Execute(ContextAction::ForwardMessage),
                enabled: true,
            },
            MenuItem {
                label: "AI Actions".to_string(),
                description: "AI-powered email assistance".to_string(),
                action: MenuAction::OpenSubmenu(self.generate_ai_menu()),
                enabled: true,
            },
        ]
    }

    /// Generate AI assistance menu items
    fn generate_ai_menu(&self) -> Vec<MenuItem> {
        vec![
            MenuItem {
                label: "Summarize Email".to_string(),
                description: "Generate AI summary of this email".to_string(),
                action: MenuAction::Execute(ContextAction::AISummarizeEmail),
                enabled: true,
            },
            MenuItem {
                label: "Quick Reply".to_string(),
                description: "Generate AI-assisted reply".to_string(),
                action: MenuAction::Execute(ContextAction::AIQuickReply),
                enabled: true,
            },
            MenuItem {
                label: "Compose Assist".to_string(),
                description: "AI assistance for composing".to_string(),
                action: MenuAction::Execute(ContextAction::AIComposeAssist),
                enabled: true,
            },
            MenuItem {
                label: "Calendar Assist".to_string(),
                description: "AI calendar scheduling assistance".to_string(),
                action: MenuAction::Execute(ContextAction::AICalendarAssist),
                enabled: true,
            },
        ]
    }

    /// Generate account management menu items
    fn generate_account_menu(&self) -> Vec<MenuItem> {
        vec![
            MenuItem {
                label: "Add Account".to_string(),
                description: "Add a new email account".to_string(),
                action: MenuAction::Execute(ContextAction::AddAccount),
                enabled: true,
            },
            MenuItem {
                label: "Remove Account".to_string(),
                description: "Remove current account".to_string(),
                action: MenuAction::Execute(ContextAction::RemoveAccount),
                enabled: true,
            },
            MenuItem {
                label: "Refresh Account".to_string(),
                description: "Refresh account connection".to_string(),
                action: MenuAction::Execute(ContextAction::RefreshAccount),
                enabled: true,
            },
            MenuItem {
                label: "Account Settings".to_string(),
                description: "Configure account settings".to_string(),
                action: MenuAction::Execute(ContextAction::AccountSettings),
                enabled: true,
            },
        ]
    }

    /// Generate view management menu items
    fn generate_view_menu(&self) -> Vec<MenuItem> {
        vec![
            MenuItem {
                label: "Search".to_string(),
                description: "Search emails and calendar".to_string(),
                action: MenuAction::Execute(ContextAction::ShowSearch),
                enabled: true,
            },
            MenuItem {
                label: "Toggle Threaded View".to_string(),
                description: "Toggle message threading".to_string(),
                action: MenuAction::Execute(ContextAction::ToggleThreadedView),
                enabled: true,
            },
        ]
    }

    /// Generate navigation menu items
    fn generate_navigation_menu(&self) -> Vec<MenuItem> {
        vec![
            MenuItem {
                label: "Email".to_string(),
                description: "Switch to email interface".to_string(),
                action: MenuAction::Execute(ContextAction::ComposeNew), // Placeholder - will be handled differently
                enabled: true,
            },
            MenuItem {
                label: "Calendar".to_string(),
                description: "Switch to calendar view".to_string(),
                action: MenuAction::Execute(ContextAction::ShowCalendar),
                enabled: true,
            },
            MenuItem {
                label: "Contacts".to_string(),
                description: "Switch to contacts view".to_string(),
                action: MenuAction::Execute(ContextAction::ShowContacts),
                enabled: true,
            },
        ]
    }

    /// Generate settings and help menu items
    fn generate_settings_menu(&self) -> Vec<MenuItem> {
        vec![
            MenuItem {
                label: "Settings".to_string(),
                description: "Application settings and preferences".to_string(),
                action: MenuAction::Execute(ContextAction::ShowSettings),
                enabled: true,
            },
            MenuItem {
                label: "Keyboard Shortcuts".to_string(),
                description: "View all keyboard shortcuts".to_string(),
                action: MenuAction::Execute(ContextAction::ShowKeyboardShortcuts),
                enabled: true,
            },
        ]
    }

    /// Generate general menu items (fallback)
    fn generate_general_menu(&self) -> Vec<MenuItem> {
        vec![
            MenuItem {
                label: "Email".to_string(),
                description: "Email management and composition".to_string(),
                action: MenuAction::OpenSubmenu(self.generate_email_menu()),
                enabled: true,
            },
            MenuItem {
                label: "Calendar".to_string(),
                description: "Calendar and event management".to_string(),
                action: MenuAction::Execute(ContextAction::ShowCalendar),
                enabled: true,
            },
            MenuItem {
                label: "Contacts".to_string(),
                description: "Contact management".to_string(),
                action: MenuAction::Execute(ContextAction::ShowContacts),
                enabled: true,
            },
        ]
    }

    /// Render the context menu
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.visible {
            return;
        }

        let current_menu = if let Some(menu) = self.menu_stack.last() {
            menu
        } else {
            return;
        };

        // Calculate menu size based on content
        let menu_width = 50.min(area.width.saturating_sub(4));
        let menu_height = (current_menu.len() as u16 + 4).min(area.height.saturating_sub(2));

        // Center the menu on screen
        let menu_x = (area.width.saturating_sub(menu_width)) / 2;
        let menu_y = (area.height.saturating_sub(menu_height)) / 2;

        let menu_area = Rect {
            x: menu_x,
            y: menu_y,
            width: menu_width,
            height: menu_height,
        };

        // Clear the background
        frame.render_widget(Clear, menu_area);

        // Create menu items
        let items: Vec<ListItem> = current_menu
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let style = if i == self.selected_index {
                    Style::default()
                        .bg(theme.colors.palette.accent)
                        .fg(theme.colors.palette.background)
                        .add_modifier(Modifier::BOLD)
                } else if !item.enabled {
                    Style::default().fg(theme.colors.palette.text_muted)
                } else {
                    Style::default().fg(theme.colors.palette.text_primary)
                };

                let label = if item.enabled {
                    format!(" {} ", item.label)
                } else {
                    format!(" {} (disabled) ", item.label)
                };

                ListItem::new(Line::from(Span::styled(label, style)))
            })
            .collect();

        // Create the list widget
        let title = if self.menu_stack.len() > 1 {
            "Actions (Esc: Back, Enter: Select)"
        } else {
            "Actions (Esc: Close, Enter: Select)"
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(theme.colors.palette.border),
            )
            .style(Style::default().bg(theme.colors.palette.background));

        frame.render_widget(list, menu_area);

        // Show description for selected item
        if let Some(item) = current_menu.get(self.selected_index) {
            let desc_area = Rect {
                x: menu_area.x,
                y: menu_area.y + menu_area.height,
                width: menu_area.width,
                height: 3,
            };

            if desc_area.y + desc_area.height <= area.height {
                frame.render_widget(Clear, desc_area);

                let description = Paragraph::new(item.description.as_str())
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(theme.colors.palette.border),
                    )
                    .style(
                        Style::default()
                            .bg(theme.colors.palette.background)
                            .fg(theme.colors.palette.text_secondary),
                    )
                    .alignment(Alignment::Center)
                    .wrap(ratatui::widgets::Wrap { trim: true });

                frame.render_widget(description, desc_area);
            }
        }
    }
}
