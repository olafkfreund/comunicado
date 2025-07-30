pub mod account_switcher;
pub mod animated_content;
pub mod animation;
pub mod calendar;
pub mod compose;
pub mod content_preview;
pub mod context_calendar;
pub mod date_picker;
pub mod draft_list;
pub mod email_viewer;
pub mod enhanced_message_list;
pub mod folder_tree;
pub mod graphics;
pub mod integrated_layout;
pub mod invitation_viewer;
pub mod keyboard_shortcuts;
pub mod layout;
pub mod message_list;
pub mod modern_dashboard;
pub mod modern_dashboard_widgets;
pub mod modern_dashboard_calendar;
pub mod modern_dashboard_contacts;
pub mod modern_dashboard_data;
pub mod search;
pub mod start_page;
pub mod startup_progress;
pub mod status_bar;
pub mod sync_progress;
pub mod time_picker;
pub mod unified_sidebar;

#[cfg(test)]
mod compose_tests;

use crate::email::{
    sync_engine::SyncProgress, EmailDatabase, EmailNotification, EmailNotificationManager,
    UIEmailUpdater,
};
use crate::keyboard::KeyboardManager;
use crate::theme::{Theme, ThemeManager};
use chrono::Duration as ChronoDuration;
use ratatui::{
    layout::Rect,
    widgets::{Block, Borders},
    Frame,
};
use std::sync::Arc;
use tokio::time::{Duration, Instant};

use self::{
    account_switcher::AccountSwitcher,
    compose::ComposeUI,
    content_preview::ContentPreview,
    draft_list::DraftListUI,
    folder_tree::FolderTree,
    keyboard_shortcuts::KeyboardShortcutsUI,
    layout::AppLayout,
    message_list::MessageList,
    start_page::StartPage,
    modern_dashboard::ModernDashboard,
    status_bar::{
        CalendarStatusSegment, EmailStatusSegment, NavigationHintsSegment, StatusBar, SyncStatus,
        SystemInfoSegment,
    },
    sync_progress::SyncProgressOverlay,
};

// Re-export compose and draft types for external use
pub use compose::{ComposeAction, EmailComposeData};
pub use draft_list::{DraftAction, DraftListUI as DraftList};

// Re-export account switcher types for external use
pub use account_switcher::{AccountItem, AccountSyncStatus};

// Re-export calendar types for external use
pub use crate::calendar::{CalendarAction, CalendarUI, CalendarViewMode};

// Re-export date/time picker types
pub use date_picker::DatePicker;
pub use time_picker::{TimeField, TimePicker};

// Re-export email viewer types
pub use email_viewer::{EmailViewer, EmailViewerAction};

// Re-export invitation viewer types
pub use invitation_viewer::{InvitationAction, InvitationViewer};

// Re-export search types
pub use search::{SearchAction, SearchEngine, SearchMode, SearchResult, SearchUI};

// Re-export animation and graphics types
pub use animated_content::{AnimatedContentManager, AnimatedEmailContent, AnimationControlWidget};
pub use animation::{Animation, AnimationDecoder, AnimationFormat, AnimationManager, AnimationSettings};
pub use graphics::{GraphicsProtocol, ImageRenderer, RenderConfig};

// Re-export integrated layout and context-aware calendar types
pub use context_calendar::{CalendarAction as ContextCalendarAction, ContextAwareCalendar, CalendarDisplayMode, EmailCalendarContext};
pub use integrated_layout::{IntegratedLayout, IntegratedLayoutManager, IntegratedViewMode, ContentType};
pub use unified_sidebar::{UnifiedSidebar, SidebarAction, NavigationItem, QuickActionType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPane {
    AccountSwitcher,
    FolderTree,
    MessageList,
    ContentPreview,
    Compose,
    StartPage,
    DraftList,
    Calendar,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UIMode {
    StartPage,
    Normal,
    Compose,
    DraftList,
    Calendar,
    ContextAware, // New context-aware email-calendar integration
    EventCreate,
    EventEdit,
    EventView,
    EmailViewer,
    InvitationViewer,
    Search,
    KeyboardShortcuts,
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
    draft_list: DraftListUI,
    start_page: StartPage,
    modern_dashboard: ModernDashboard,
    calendar_ui: CalendarUI,
    event_form_ui: Option<crate::calendar::EventFormUI>,
    email_viewer: EmailViewer,
    invitation_viewer: InvitationViewer,
    search_ui: SearchUI,
    search_engine: Option<SearchEngine>,
    keyboard_shortcuts_ui: KeyboardShortcutsUI,
    // Context-aware integration components
    context_calendar: ContextAwareCalendar,
    integrated_layout: IntegratedLayoutManager,
    unified_sidebar: UnifiedSidebar,
    // Notification system
    notification_message: Option<String>,
    notification_expires_at: Option<tokio::time::Instant>,
}

impl UI {
    pub fn new() -> Self {
        let mut ui = Self {
            focused_pane: FocusedPane::StartPage,
            account_switcher: AccountSwitcher::new(),
            folder_tree: FolderTree::new(),
            message_list: MessageList::new(),
            content_preview: ContentPreview::new(),
            layout: AppLayout::new(),
            theme_manager: ThemeManager::new(),
            status_bar: StatusBar::default(),
            email_updater: None,
            sync_progress_overlay: SyncProgressOverlay::new(),
            mode: UIMode::StartPage,
            compose_ui: None,
            draft_list: DraftListUI::new(),
            start_page: StartPage::new(),
            modern_dashboard: {
                let mut dashboard = ModernDashboard::new();
                dashboard.initialize_with_sample_data();
                dashboard
            },
            calendar_ui: CalendarUI::new(),
            event_form_ui: None,
            email_viewer: EmailViewer::new(),
            invitation_viewer: InvitationViewer::new(),
            search_ui: SearchUI::new(),
            search_engine: None,
            keyboard_shortcuts_ui: KeyboardShortcutsUI::new(),
            // Initialize context-aware integration components
            context_calendar: ContextAwareCalendar::new(),
            integrated_layout: IntegratedLayoutManager::new(),
            unified_sidebar: UnifiedSidebar::new(),
            // Initialize notification system
            notification_message: None,
            notification_expires_at: None,
        };

        // Initialize status bar with default segments
        ui.initialize_status_bar();
        ui
    }

    fn initialize_status_bar(&mut self) {
        // Add email status segment (will be updated with real data when messages are loaded)
        let email_segment = EmailStatusSegment {
            unread_count: 0,
            total_count: 0,
            sync_status: SyncStatus::Offline,
        };
        self.status_bar
            .add_segment("email".to_string(), email_segment);

        // Add calendar segment
        let calendar_segment = CalendarStatusSegment {
            next_event: Some("Team Meeting".to_string()),
            events_today: 3,
        };
        self.status_bar
            .add_segment("calendar".to_string(), calendar_segment);

        // Add system info segment
        let current_time = chrono::Local::now().format("%H:%M").to_string();
        let active_account = if let Some(account) = self.account_switcher.get_current_account() {
            account.email_address.clone()
        } else {
            "No account selected".to_string()
        };

        let system_segment = SystemInfoSegment {
            current_time,
            active_account,
        };
        self.status_bar
            .add_segment("system".to_string(), system_segment);

        // Add navigation hints
        let nav_segment = NavigationHintsSegment {
            current_pane: "Folders".to_string(),
            available_shortcuts: vec![
                ("Tab".to_string(), "Switch".to_string()),
                ("q".to_string(), "Quit".to_string()),
                ("h/j/k/l".to_string(), "Navigate".to_string()),
            ],
        };
        self.status_bar
            .add_segment("navigation".to_string(), nav_segment);
    }

    pub fn render(&mut self, frame: &mut Frame) {
        self.render_with_keyboard_manager(frame, &KeyboardManager::default())
    }

    pub fn render_with_keyboard_manager(
        &mut self,
        frame: &mut Frame,
        keyboard_manager: &KeyboardManager,
    ) {
        let size = frame.size();

        match self.mode {
            UIMode::StartPage => {
                // Render modern dashboard in full screen
                let theme = self.theme_manager.current_theme();
                self.modern_dashboard.render(frame, size, theme);
            }
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
            UIMode::DraftList => {
                // Render draft list UI in full screen
                let theme = self.theme_manager.current_theme();
                self.draft_list.render(frame, size, theme);
            }
            UIMode::Calendar => {
                // Render calendar UI in full screen
                let theme = self.theme_manager.current_theme();
                self.calendar_ui.render(frame, size, theme);
            }
            UIMode::ContextAware => {
                // Render context-aware email-calendar integration
                self.render_context_aware_layout(frame, size);
            }
            UIMode::EventCreate | UIMode::EventEdit | UIMode::EventView => {
                // Render event form UI in full screen
                if let Some(ref mut event_form) = self.event_form_ui {
                    let theme = self.theme_manager.current_theme();
                    event_form.render(frame, size, theme);
                }
            }
            UIMode::EmailViewer => {
                // Render email viewer in full screen
                let theme = self.theme_manager.current_theme();
                self.email_viewer.render(frame, size, theme);
            }
            UIMode::InvitationViewer => {
                // Render invitation viewer in full screen
                let theme = self.theme_manager.current_theme();
                self.invitation_viewer.render(frame, size, theme);
            }
            UIMode::Search => {
                // Render search UI over the normal interface
                let chunks = self.layout.calculate_layout(size);

                // Render normal interface in background
                self.render_account_switcher(frame, chunks[0]);
                self.render_folder_tree(frame, chunks[1]);
                self.render_message_list(frame, chunks[2]);
                self.render_content_preview(frame, chunks[3]);

                // Render the status bar
                if chunks.len() > 4 {
                    self.render_status_bar(frame, chunks[4]);
                }

                // Render search UI on top
                let theme = self.theme_manager.current_theme();
                self.search_ui.render(frame, size, theme);
            }
            UIMode::KeyboardShortcuts => {
                // Render keyboard shortcuts over the normal interface
                let chunks = self.layout.calculate_layout(size);

                // Render normal interface in background
                self.render_account_switcher(frame, chunks[0]);
                self.render_folder_tree(frame, chunks[1]);
                self.render_message_list(frame, chunks[2]);
                self.render_content_preview(frame, chunks[3]);

                // Render the status bar
                if chunks.len() > 4 {
                    self.render_status_bar(frame, chunks[4]);
                }

                // Render keyboard shortcuts UI on top
                let theme = self.theme_manager.current_theme();
                self.keyboard_shortcuts_ui
                    .render(frame, size, theme, keyboard_manager);
            }
        }
    }

    /// Render context-aware email-calendar integrated layout
    fn render_context_aware_layout(&mut self, frame: &mut Frame, area: Rect) {
        let layout = self.integrated_layout.calculate_layout(area);

        // Analyze current email for calendar context first
        let mut calendar_context_mode = CalendarDisplayMode::Hidden;
        if let Some(selected_message) = self.message_list.get_selected_message_for_preview() {
            calendar_context_mode = self.context_calendar.analyze_message_item_context(&selected_message);
        }

        // Render components that need mutable access first
        self.render_context_aware_details(frame, layout.details_panel);

        // Now get theme and render components that need immutable access
        let theme = self.theme_manager.current_theme();

        // Render unified sidebar
        self.unified_sidebar.render(frame, layout.sidebar, theme);

        // Render primary content (email list)
        self.render_context_aware_email_list(frame, layout.primary_content);

        // Render context-aware calendar sidebar (only if relevant)
        if let Some(secondary_area) = layout.secondary_content {
            if calendar_context_mode != CalendarDisplayMode::Hidden {
                self.context_calendar.render(frame, secondary_area, theme);
            }
        }

        // Render status bar
        self.render_status_bar(frame, layout.status_bar);

        // Render action bar if visible
        if let Some(action_area) = layout.action_bar {
            self.render_context_aware_actions(frame, action_area, calendar_context_mode);
        }
    }

    /// Render email list with context-aware highlighting
    fn render_context_aware_email_list(&self, frame: &mut Frame, area: Rect) {
        let is_focused = matches!(self.focused_pane, FocusedPane::MessageList);
        let theme = self.theme_manager.current_theme();

        let border_style = theme.get_component_style("border", is_focused);
        let block = Block::default()
            .title("Messages")
            .borders(Borders::ALL)
            .border_style(border_style);

        self.message_list.render(frame, area, block, is_focused, theme);
    }

    /// Render email content details with context awareness
    fn render_context_aware_details(&mut self, frame: &mut Frame, area: Rect) {
        let is_focused = matches!(self.focused_pane, FocusedPane::ContentPreview);
        let theme = self.theme_manager.current_theme();

        let border_style = theme.get_component_style("border", is_focused);
        let block = Block::default()
            .title("Content")
            .borders(Borders::ALL)
            .border_style(border_style);

        self.content_preview.render(frame, area, block, is_focused, theme);
    }

    /// Render context-aware action bar for calendar/email actions
    fn render_context_aware_actions(&self, frame: &mut Frame, area: Rect, context_mode: CalendarDisplayMode) {
        use ratatui::{
            layout::Alignment,
            style::{Modifier, Style},
            text::{Line, Span},
            widgets::{Block, Borders, Paragraph},
        };

        let theme = self.theme_manager.current_theme();
        
        let actions = match context_mode {
            CalendarDisplayMode::InvitationDetails => vec![
                Span::styled(" [←→] Navigate ", Style::default().fg(theme.colors.palette.text_muted)),
                Span::styled(" [Enter] RSVP ", Style::default().fg(theme.colors.palette.accent).add_modifier(Modifier::BOLD)),
                Span::styled(" [Esc] Hide ", Style::default().fg(theme.colors.palette.text_muted)),
            ],
            CalendarDisplayMode::DailyAgenda => vec![
                Span::styled(" [c] Create Event ", Style::default().fg(theme.colors.palette.accent).add_modifier(Modifier::BOLD)),
                Span::styled(" [r] Reply ", Style::default().fg(theme.colors.palette.text_muted)),
                Span::styled(" [f] Forward ", Style::default().fg(theme.colors.palette.text_muted)),
            ],
            CalendarDisplayMode::QuickSchedule => vec![
                Span::styled(" [c] Create Meeting ", Style::default().fg(theme.colors.palette.accent).add_modifier(Modifier::BOLD)),
                Span::styled(" [s] Schedule ", Style::default().fg(theme.colors.palette.text_muted)),
                Span::styled(" [r] Reply ", Style::default().fg(theme.colors.palette.text_muted)),
            ],
            _ => vec![
                Span::styled(" [r] Reply ", Style::default().fg(theme.colors.palette.text_muted)),
                Span::styled(" [f] Forward ", Style::default().fg(theme.colors.palette.text_muted)),
                Span::styled(" [c] Compose ", Style::default().fg(theme.colors.palette.text_muted)),
            ],
        };

        let action_line = Line::from(actions);
        let action_paragraph = Paragraph::new(action_line)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.colors.palette.border))
            )
            .alignment(Alignment::Center);

        frame.render_widget(action_paragraph, area);
    }

    fn render_account_switcher(&mut self, frame: &mut Frame, area: Rect) {
        let is_focused = matches!(self.focused_pane, FocusedPane::AccountSwitcher);
        let theme = self.theme_manager.current_theme();

        let border_style = theme.get_component_style("border", is_focused);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style);

        self.account_switcher
            .render(frame, area, block, is_focused, theme);
    }

    fn render_folder_tree(&self, frame: &mut Frame, area: Rect) {
        let is_focused = matches!(self.focused_pane, FocusedPane::FolderTree);
        let theme = self.theme_manager.current_theme();

        let border_style = theme.get_component_style("border", is_focused);
        let block = Block::default()
            .title("Folders")
            .borders(Borders::ALL)
            .border_style(border_style);

        self.folder_tree
            .render(frame, area, block, is_focused, theme);
    }

    fn render_message_list(&self, frame: &mut Frame, area: Rect) {
        let is_focused = matches!(self.focused_pane, FocusedPane::MessageList);
        let theme = self.theme_manager.current_theme();

        let border_style = theme.get_component_style("border", is_focused);
        let block = Block::default()
            .title("Messages")
            .borders(Borders::ALL)
            .border_style(border_style);

        self.message_list
            .render(frame, area, block, is_focused, theme);
    }

    fn render_content_preview(&mut self, frame: &mut Frame, area: Rect) {
        let is_focused = matches!(self.focused_pane, FocusedPane::ContentPreview);
        let theme = self.theme_manager.current_theme();

        let border_style = theme.get_component_style("border", is_focused);
        let block = Block::default()
            .title("Content")
            .borders(Borders::ALL)
            .border_style(border_style);

        self.content_preview
            .render(frame, area, block, is_focused, theme);
    }

    fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        let theme = self.theme_manager.current_theme();

        // Check if we have a notification to display
        if let Some(notification) = self.get_notification() {
            // Render notification on top of status bar with special styling
            use ratatui::{
                layout::Alignment,
                style::{Color, Modifier, Style},
                text::{Line, Span},
                widgets::{Block, Borders, Paragraph},
            };

            let notification_text = vec![Line::from(vec![Span::styled(
                notification.clone(),
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )])];

            let notification_widget = Paragraph::new(notification_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Yellow)),
                )
                .alignment(Alignment::Center);

            frame.render_widget(notification_widget, area);
        } else {
            // Normal status bar rendering
            self.status_bar.render(frame, area, theme);
        }
    }

    // Navigation methods
    pub fn next_pane(&mut self) {
        if matches!(self.mode, UIMode::Compose | UIMode::Calendar) {
            return; // No pane switching in compose or calendar mode
        }

        self.focused_pane = match self.focused_pane {
            FocusedPane::AccountSwitcher => FocusedPane::FolderTree,
            FocusedPane::FolderTree => FocusedPane::MessageList,
            FocusedPane::MessageList => FocusedPane::ContentPreview,
            FocusedPane::ContentPreview => FocusedPane::AccountSwitcher,
            FocusedPane::Compose => FocusedPane::Compose, // Stay in compose
            FocusedPane::StartPage => FocusedPane::StartPage, // Stay in start page
            FocusedPane::DraftList => FocusedPane::DraftList, // Stay in draft list
            FocusedPane::Calendar => FocusedPane::Calendar, // Stay in calendar
        };
        self.update_navigation_hints();
    }

    pub fn previous_pane(&mut self) {
        if matches!(self.mode, UIMode::Compose | UIMode::Calendar) {
            return; // No pane switching in compose or calendar mode
        }

        self.focused_pane = match self.focused_pane {
            FocusedPane::AccountSwitcher => FocusedPane::ContentPreview,
            FocusedPane::FolderTree => FocusedPane::AccountSwitcher,
            FocusedPane::MessageList => FocusedPane::FolderTree,
            FocusedPane::ContentPreview => FocusedPane::MessageList,
            FocusedPane::Compose => FocusedPane::Compose, // Stay in compose
            FocusedPane::StartPage => FocusedPane::StartPage, // Stay in start page
            FocusedPane::DraftList => FocusedPane::DraftList, // Stay in draft list
            FocusedPane::Calendar => FocusedPane::Calendar, // Stay in calendar
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

    pub fn content_preview(&self) -> &ContentPreview {
        &self.content_preview
    }

    pub fn keyboard_shortcuts_ui_mut(&mut self) -> &mut KeyboardShortcutsUI {
        &mut self.keyboard_shortcuts_ui
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
                FocusedPane::StartPage => "Start Page", // Shouldn't happen in normal mode
                FocusedPane::DraftList => "Draft List", // Shouldn't happen in normal mode
                FocusedPane::Calendar => "Calendar", // Shouldn't happen in normal mode
            },
            UIMode::Compose => "Compose Email",
            UIMode::StartPage => "Dashboard",
            UIMode::DraftList => "Draft Manager",
            UIMode::Calendar => "Calendar",
            UIMode::ContextAware => "Context-Aware View",
            UIMode::EventCreate => "Create Event",
            UIMode::EventEdit => "Edit Event",
            UIMode::EventView => "View Event",
            UIMode::EmailViewer => "Email Viewer",
            UIMode::InvitationViewer => "Meeting Invitation",
            UIMode::Search => "Search",
            UIMode::KeyboardShortcuts => "Keyboard Shortcuts",
        };

        let nav_segment = NavigationHintsSegment {
            current_pane: current_pane_name.to_string(),
            available_shortcuts: self.get_current_shortcuts(),
        };

        self.status_bar
            .add_segment("navigation".to_string(), nav_segment);
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
                    ("Ctrl+R".to_string(), "Refresh".to_string()),
                    ("F5".to_string(), "Sync".to_string()),
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
                    ("a".to_string(), "Select Attachment".to_string()),
                    ("Ctrl+j/k".to_string(), "Navigate Attachments".to_string()),
                    ("s".to_string(), "Save Attachment".to_string()),
                    ("Home/End".to_string(), "Jump".to_string()),
                    ("r".to_string(), "Reply".to_string()),
                    ("f".to_string(), "Forward".to_string()),
                ],
                FocusedPane::StartPage => vec![],
                _ => vec![],
            },
            UIMode::Compose => vec![
                ("Tab".to_string(), "Next Field".to_string()),
                ("F1".to_string(), "Send".to_string()),
                ("F2".to_string(), "Save Draft".to_string()),
                ("@".to_string(), "Contact Lookup".to_string()),
                ("Esc".to_string(), "Cancel".to_string()),
            ],
            UIMode::StartPage => vec![
                ("h/l".to_string(), "Navigate".to_string()),
                ("Enter/e".to_string(), "Email".to_string()),
                ("c".to_string(), "Compose".to_string()),
                ("/".to_string(), "Search".to_string()),
                ("q".to_string(), "Quit".to_string()),
            ],
            UIMode::DraftList => vec![
                ("↑↓".to_string(), "Navigate".to_string()),
                ("Enter".to_string(), "Load Draft".to_string()),
                ("d".to_string(), "Delete".to_string()),
                ("s".to_string(), "Sort".to_string()),
                ("Tab".to_string(), "Details".to_string()),
                ("F5".to_string(), "Refresh".to_string()),
                ("Esc".to_string(), "Close".to_string()),
            ],
            UIMode::Calendar => vec![
                ("1-4".to_string(), "Views".to_string()),
                ("h/l".to_string(), "Navigate".to_string()),
                ("Space".to_string(), "Today".to_string()),
                ("c".to_string(), "Create Event".to_string()),
                ("Enter".to_string(), "Event Details".to_string()),
                ("e".to_string(), "Edit Event".to_string()),
                ("r".to_string(), "Refresh".to_string()),
                ("Esc".to_string(), "Close".to_string()),
            ],
            UIMode::EventCreate => vec![
                ("Tab".to_string(), "Next Field".to_string()),
                ("Shift+Tab".to_string(), "Prev Field".to_string()),
                ("Enter".to_string(), "Edit Field".to_string()),
                ("F1".to_string(), "Save Event".to_string()),
                ("Esc".to_string(), "Cancel".to_string()),
            ],
            UIMode::EventEdit => vec![
                ("Tab".to_string(), "Next Field".to_string()),
                ("Shift+Tab".to_string(), "Prev Field".to_string()),
                ("Enter".to_string(), "Edit Field".to_string()),
                ("F1".to_string(), "Save Changes".to_string()),
                ("F3".to_string(), "Delete Event".to_string()),
                ("Esc".to_string(), "Cancel".to_string()),
            ],
            UIMode::EventView => vec![
                ("e".to_string(), "Edit Event".to_string()),
                ("d".to_string(), "Delete Event".to_string()),
                ("r".to_string(), "RSVP".to_string()),
                ("Esc".to_string(), "Close".to_string()),
            ],
            UIMode::EmailViewer => vec![
                ("j/k".to_string(), "Scroll".to_string()),
                ("r".to_string(), "Reply".to_string()),
                ("R".to_string(), "Reply All".to_string()),
                ("f".to_string(), "Forward".to_string()),
                ("Space".to_string(), "Actions".to_string()),
                ("v".to_string(), "View Mode".to_string()),
                ("q/Esc".to_string(), "Close".to_string()),
            ],
            UIMode::InvitationViewer => vec![
                ("j/k".to_string(), "Navigate".to_string()),
                ("Enter".to_string(), "Select Action".to_string()),
                ("a".to_string(), "Accept".to_string()),
                ("d".to_string(), "Decline".to_string()),
                ("t".to_string(), "Tentative".to_string()),
                ("v".to_string(), "Details".to_string()),
                ("q/Esc".to_string(), "Close".to_string()),
            ],
            UIMode::Search => vec![
                ("Type".to_string(), "Search Query".to_string()),
                ("↑↓/j/k".to_string(), "Navigate Results".to_string()),
                ("Enter".to_string(), "Open Result".to_string()),
                ("Tab".to_string(), "Search Mode".to_string()),
                ("F1-F4".to_string(), "Quick Mode".to_string()),
                ("Esc".to_string(), "Close Search".to_string()),
            ],
            UIMode::ContextAware => vec![
                ("Tab".to_string(), "Switch Pane".to_string()),
                ("j/k".to_string(), "Navigate".to_string()),
                ("Enter".to_string(), "Select/RSVP".to_string()),
                ("c".to_string(), "Create Event".to_string()),
                ("r".to_string(), "Reply".to_string()),
                ("f".to_string(), "Forward".to_string()),
                ("Esc".to_string(), "Hide Calendar".to_string()),
            ],
            UIMode::KeyboardShortcuts => vec![
                ("↑↓/j/k".to_string(), "Scroll".to_string()),
                ("?".to_string(), "Close".to_string()),
                ("Esc".to_string(), "Close".to_string()),
            ],
        }
    }

    pub fn update_email_status(&mut self, unread: usize, total: usize, sync_status: SyncStatus) {
        let email_segment = EmailStatusSegment {
            unread_count: unread,
            total_count: total,
            sync_status,
        };
        self.status_bar
            .add_segment("email".to_string(), email_segment);
    }

    pub fn update_system_time(&mut self, time: String) {
        // Get the current system segment and update only the time
        let active_account = if let Some(account) = self.account_switcher.get_current_account() {
            account.email_address.clone()
        } else {
            "No account selected".to_string()
        };

        let system_segment = SystemInfoSegment {
            current_time: time,
            active_account,
        };
        self.status_bar
            .add_segment("system".to_string(), system_segment);
    }

    /// Update status bar with current account information
    pub fn update_status_bar_account_info(&mut self) {
        let current_time = chrono::Local::now().format("%H:%M").to_string();
        let active_account = if let Some(account) = self.account_switcher.get_current_account() {
            account.email_address.clone()
        } else {
            "No account selected".to_string()
        };

        let system_segment = SystemInfoSegment {
            current_time,
            active_account,
        };
        self.status_bar
            .add_segment("system".to_string(), system_segment);
    }

    /// Set the database for email operations
    pub fn set_database(&mut self, database: Arc<EmailDatabase>) {
        self.message_list.set_database(database.clone());
        self.content_preview.set_database(database.clone());
        self.folder_tree.set_database(database);
    }

    /// Set the notification manager for real-time updates
    pub fn set_notification_manager(
        &mut self,
        notification_manager: Arc<EmailNotificationManager>,
    ) {
        self.email_updater = Some(UIEmailUpdater::new(&notification_manager));
    }

    /// Load folders for a specific account
    pub async fn load_folders(
        &mut self,
        account_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.folder_tree.load_folders(account_id).await?;
        Ok(())
    }

    /// Load messages for a specific account and folder
    pub async fn load_messages(
        &mut self,
        account_id: String,
        folder_name: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.message_list
            .load_messages(account_id.clone(), folder_name.clone())
            .await?;

        // Subscribe to notifications for this folder
        if let Some(ref mut updater) = self.email_updater {
            updater.subscribe_to_folder(account_id, folder_name);
        }

        // Update email status after loading
        let message_count = self.message_list.messages().len();
        let unread_count = self
            .message_list
            .messages()
            .iter()
            .filter(|msg| !msg.is_read)
            .count();

        self.update_email_status(unread_count, message_count, SyncStatus::Online);
        Ok(())
    }

    /// Set available accounts in the account switcher
    pub fn set_accounts(&mut self, accounts: Vec<AccountItem>) {
        self.account_switcher.set_accounts(accounts);

        // Update status bar with current account after setting accounts
        self.update_status_bar_account_info();
    }

    /// Add a new account to the account switcher
    pub fn add_account(&mut self, account: AccountItem) {
        self.account_switcher.update_account(account);
    }

    /// Remove an account from the account switcher
    pub fn remove_account(&mut self, account_id: &str) {
        self.account_switcher.remove_account(account_id);
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
    pub async fn switch_to_account(
        &mut self,
        account_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.account_switcher.set_current_account(account_id) {
            // Load folders for the new account
            self.load_folders(account_id).await?;

            // Load messages for the new account's INBOX
            self.load_messages(account_id.to_string(), "INBOX".to_string())
                .await?;

            // Update the system status to show the new account
            let current_time = chrono::Local::now().format("%H:%M").to_string();
            if let Some(account) = self.account_switcher.get_current_account() {
                let system_segment = SystemInfoSegment {
                    current_time,
                    active_account: account.email_address.clone(),
                };
                self.status_bar
                    .add_segment("system".to_string(), system_segment);
            }
        }
        Ok(())
    }

    /// Update account status and unread count
    pub fn update_account_status(
        &mut self,
        account_id: &str,
        status: AccountSyncStatus,
        unread_count: Option<usize>,
    ) {
        self.account_switcher
            .update_account_status(account_id, status, unread_count);
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
            EmailNotification::NewMessage {
                account_id,
                folder_name,
                message,
            } => {
                // Check if this notification is for the currently displayed folder
                if let (Some(current_account), Some(current_folder)) =
                    self.message_list.get_current_context()
                {
                    if current_account == &account_id && current_folder == &folder_name {
                        // Refresh the message list to show the new message
                        let _ = self.message_list.refresh_messages().await;

                        // Update status bar with new counts
                        let message_count = self.message_list.messages().len();
                        let unread_count = self
                            .message_list
                            .messages()
                            .iter()
                            .filter(|msg| !msg.is_read)
                            .count();
                        self.update_email_status(unread_count, message_count, SyncStatus::Online);
                    }
                }

                tracing::info!(
                    "New message received: {} from {}",
                    message.subject,
                    message.from_addr
                );
            }

            EmailNotification::MessageUpdated {
                account_id,
                folder_name,
                message,
                ..
            } => {
                // Check if this notification is for the currently displayed folder
                if let (Some(current_account), Some(current_folder)) =
                    self.message_list.get_current_context()
                {
                    if current_account == &account_id && current_folder == &folder_name {
                        // Refresh the message list to show updated message
                        let _ = self.message_list.refresh_messages().await;

                        // Update status bar
                        let message_count = self.message_list.messages().len();
                        let unread_count = self
                            .message_list
                            .messages()
                            .iter()
                            .filter(|msg| !msg.is_read)
                            .count();
                        self.update_email_status(unread_count, message_count, SyncStatus::Online);
                    }
                }

                tracing::info!("Message updated: {}", message.subject);
            }

            EmailNotification::MessageDeleted {
                account_id,
                folder_name,
                ..
            } => {
                // Check if this notification is for the currently displayed folder
                if let (Some(current_account), Some(current_folder)) =
                    self.message_list.get_current_context()
                {
                    if current_account == &account_id && current_folder == &folder_name {
                        // Refresh the message list to remove deleted message
                        let _ = self.message_list.refresh_messages().await;

                        // Update status bar
                        let message_count = self.message_list.messages().len();
                        let unread_count = self
                            .message_list
                            .messages()
                            .iter()
                            .filter(|msg| !msg.is_read)
                            .count();
                        self.update_email_status(unread_count, message_count, SyncStatus::Online);
                    }
                }

                tracing::info!("Message deleted from {}/{}", account_id, folder_name);
            }

            EmailNotification::SyncStarted {
                account_id,
                folder_name,
            } => {
                // Update status bar to show sync in progress
                if let (Some(current_account), Some(current_folder)) =
                    self.message_list.get_current_context()
                {
                    if current_account == &account_id && current_folder == &folder_name {
                        let message_count = self.message_list.messages().len();
                        let unread_count = self
                            .message_list
                            .messages()
                            .iter()
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

            EmailNotification::SyncCompleted {
                account_id,
                folder_name,
                new_count,
                updated_count,
            } => {
                // Update status bar to show sync completed
                if let (Some(current_account), Some(current_folder)) =
                    self.message_list.get_current_context()
                {
                    if current_account == &account_id && current_folder == &folder_name {
                        // Refresh messages after sync
                        let _ = self.message_list.refresh_messages().await;

                        let message_count = self.message_list.messages().len();
                        let unread_count = self
                            .message_list
                            .messages()
                            .iter()
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

                tracing::info!(
                    "Sync completed for {}/{}: {} new, {} updated",
                    account_id,
                    folder_name,
                    new_count,
                    updated_count
                );
            }

            EmailNotification::SyncFailed {
                account_id,
                folder_name,
                error,
            } => {
                // Update status bar to show sync error
                if let (Some(current_account), Some(current_folder)) =
                    self.message_list.get_current_context()
                {
                    if current_account == &account_id && current_folder == &folder_name {
                        let message_count = self.message_list.messages().len();
                        let unread_count = self
                            .message_list
                            .messages()
                            .iter()
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
        if let (Some(current_account), Some(current_folder)) =
            self.message_list.get_current_context()
        {
            if current_account == &progress.account_id && current_folder == &progress.folder_name {
                let message_count = self.message_list.messages().len();
                let unread_count = self
                    .message_list
                    .messages()
                    .iter()
                    .filter(|msg| !msg.is_read)
                    .count();

                let sync_status = match progress.phase {
                    crate::email::sync_engine::SyncPhase::Complete => SyncStatus::Online,
                    crate::email::sync_engine::SyncPhase::Error(_) => SyncStatus::Error,
                    _ => {
                        if progress.total_messages > 0 {
                            SyncStatus::SyncingWithProgress(
                                progress.messages_processed,
                                progress.total_messages,
                            )
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
    pub fn start_reply(
        &mut self,
        contacts_manager: Arc<crate::contacts::ContactsManager>,
        reply_to: &str,
        subject: &str,
    ) {
        self.compose_ui = Some(ComposeUI::new_reply(contacts_manager, reply_to, subject));
        self.mode = UIMode::Compose;
        self.focused_pane = FocusedPane::Compose;
    }

    /// Enter compose mode for forwarding a message
    pub fn start_forward(
        &mut self,
        contacts_manager: Arc<crate::contacts::ContactsManager>,
        subject: &str,
        body: &str,
    ) {
        self.compose_ui = Some(ComposeUI::new_forward(contacts_manager, subject, body));
        self.mode = UIMode::Compose;
        self.focused_pane = FocusedPane::Compose;
    }

    /// Enter compose mode for replying to a specific message
    pub fn start_reply_from_message(
        &mut self,
        message: crate::email::StoredMessage,
        contacts_manager: Arc<crate::contacts::ContactsManager>,
    ) {
        // Extract sender information for reply
        let reply_to = message.reply_to.unwrap_or(message.from_addr.clone());

        // Add "Re: " prefix to subject if not already present
        let subject = if message.subject.starts_with("Re: ") {
            message.subject
        } else {
            format!("Re: {}", message.subject)
        };

        self.start_reply(contacts_manager, &reply_to, &subject);
    }

    /// Enter compose mode for replying to all recipients of a specific message
    pub fn start_reply_all_from_message(
        &mut self,
        message: crate::email::StoredMessage,
        contacts_manager: Arc<crate::contacts::ContactsManager>,
    ) {
        // Extract sender information for reply
        let reply_to = message.reply_to.unwrap_or(message.from_addr.clone());

        // Add "Re: " prefix to subject if not already present
        let subject = if message.subject.starts_with("Re: ") {
            message.subject
        } else {
            format!("Re: {}", message.subject)
        };

        // For reply-all, we would need to include all original recipients
        // For now, just reply to sender (this needs to be enhanced)
        self.start_reply(contacts_manager, &reply_to, &subject);
    }

    /// Enter compose mode for forwarding a specific message
    pub fn start_forward_from_message(
        &mut self,
        message: crate::email::StoredMessage,
        contacts_manager: Arc<crate::contacts::ContactsManager>,
    ) {
        // Store original subject for use in the forwarded message body
        let original_subject = message.subject.clone();

        // Add "Fwd: " prefix to subject if not already present
        let subject = if original_subject.starts_with("Fwd: ") {
            original_subject.clone()
        } else {
            format!("Fwd: {}", original_subject)
        };

        // Create forwarded message body with original content
        let body = match message.body_text {
            Some(ref text) => format!(
                "\n\n---------- Forwarded message ----------\n\
                From: {}\n\
                Date: {}\n\
                Subject: {}\n\
                To: {}\n\n{}",
                message.from_addr,
                message.date.format("%Y-%m-%d %H:%M:%S"),
                original_subject,
                message.to_addrs.join(", "),
                text
            ),
            None => format!(
                "\n\n---------- Forwarded message ----------\n\
                From: {}\n\
                Date: {}\n\
                Subject: {}\n\
                To: {}\n\n(No message content)",
                message.from_addr,
                message.date.format("%Y-%m-%d %H:%M:%S"),
                original_subject,
                message.to_addrs.join(", ")
            ),
        };

        self.start_forward(contacts_manager, &subject, &body);
    }

    /// Enter compose mode for editing a specific message (draft)
    pub fn start_edit_from_message(
        &mut self,
        message: crate::email::StoredMessage,
        contacts_manager: Arc<crate::contacts::ContactsManager>,
    ) {
        // For now, treat edit as a simple compose with the message body
        // TODO: Implement proper draft editing with pre-filled recipients and subject
        let body = message.body_text.unwrap_or_else(|| String::new());

        // Use the forward constructor as a base and customize it for editing
        self.compose_ui = Some(ComposeUI::new_forward(
            contacts_manager,
            &message.subject,
            &body,
        ));
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
    pub async fn handle_compose_key(
        &mut self,
        key: crossterm::event::KeyCode,
    ) -> Option<ComposeAction> {
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
        self.compose_ui
            .as_ref()
            .map(|ui| ui.is_modified())
            .unwrap_or(false)
    }

    /// Clear the compose modified flag
    pub fn clear_compose_modified(&mut self) {
        if let Some(ref mut compose_ui) = self.compose_ui {
            compose_ui.clear_modified();
        }
    }

    /// Get the current draft ID for auto-save operations
    pub fn get_compose_draft_id(&self) -> Option<&String> {
        self.compose_ui
            .as_ref()
            .and_then(|ui| ui.current_draft_id())
    }

    /// Set the current draft ID
    pub fn set_compose_draft_id(&mut self, draft_id: Option<String>) {
        if let Some(ref mut compose_ui) = self.compose_ui {
            compose_ui.set_current_draft_id(draft_id);
        }
    }

    /// Mark that auto-save has been performed
    pub fn mark_compose_auto_saved(&mut self) {
        if let Some(ref mut compose_ui) = self.compose_ui {
            compose_ui.mark_auto_saved();
        }
    }

    /// Check if auto-save should be triggered
    pub fn check_compose_auto_save(&self) -> Option<ComposeAction> {
        self.compose_ui.as_ref().and_then(|ui| ui.check_auto_save())
    }

    /// Show draft list UI
    pub fn show_draft_list(&mut self) {
        self.draft_list.show();
        self.mode = UIMode::DraftList;
        self.focused_pane = FocusedPane::DraftList;
    }

    /// Hide draft list UI and return to normal mode
    pub fn hide_draft_list(&mut self) {
        self.draft_list.hide();
        self.mode = UIMode::Normal;
        self.focused_pane = FocusedPane::MessageList;
    }

    /// Update the draft list with new drafts
    pub fn update_draft_list(&mut self, drafts: Vec<crate::email::database::StoredDraft>) {
        self.draft_list.update_drafts(drafts);
    }

    /// Handle key input for draft list
    pub async fn handle_draft_list_key(
        &mut self,
        key: crossterm::event::KeyCode,
    ) -> Option<DraftAction> {
        if self.mode == UIMode::DraftList {
            Some(self.draft_list.handle_key(key).await)
        } else {
            None
        }
    }

    /// Remove a draft from the list
    pub fn remove_draft_from_list(&mut self, draft_id: &str) {
        self.draft_list.remove_draft(draft_id);
    }

    /// Check if currently in draft list mode
    pub fn is_draft_list_visible(&self) -> bool {
        matches!(self.mode, UIMode::DraftList)
    }

    /// Load a draft into compose mode
    pub fn load_draft_for_editing(
        &mut self,
        compose_data: EmailComposeData,
        draft_id: String,
        contacts_manager: Arc<crate::contacts::ContactsManager>,
    ) {
        // Create new compose UI and load the draft
        let mut compose_ui = ComposeUI::new(contacts_manager);
        compose_ui.load_from_draft(compose_data, draft_id);

        self.compose_ui = Some(compose_ui);
        self.mode = UIMode::Compose;
        self.focused_pane = FocusedPane::Compose;
    }

    /// Get current UI mode
    pub fn mode(&self) -> &UIMode {
        &self.mode
    }

    /// Check if currently in compose mode
    pub fn is_composing(&self) -> bool {
        matches!(self.mode, UIMode::Compose)
    }

    /// Check if currently on start page
    pub fn is_on_start_page(&self) -> bool {
        matches!(self.mode, UIMode::StartPage)
    }

    /// Switch to start page mode
    pub fn show_start_page(&mut self) {
        self.mode = UIMode::StartPage;
        self.focused_pane = FocusedPane::StartPage;
    }

    /// Show keyboard shortcuts popup
    pub fn show_keyboard_shortcuts(&mut self) {
        self.mode = UIMode::KeyboardShortcuts;
    }

    /// Switch to normal email mode
    pub fn show_email_interface(&mut self) {
        self.mode = UIMode::Normal;
        self.focused_pane = FocusedPane::AccountSwitcher;
    }

    /// Get mutable reference to start page for data updates
    pub fn start_page_mut(&mut self) -> &mut StartPage {
        &mut self.start_page
    }

    /// Get reference to start page
    pub fn start_page(&self) -> &StartPage {
        &self.start_page
    }

    /// Get mutable reference to modern dashboard for data updates
    pub fn modern_dashboard_mut(&mut self) -> &mut ModernDashboard {
        &mut self.modern_dashboard
    }

    /// Get reference to modern dashboard
    pub fn modern_dashboard(&self) -> &ModernDashboard {
        &self.modern_dashboard
    }

    /// Handle start page navigation
    pub fn handle_start_page_navigation(&mut self, direction: StartPageNavigation) {
        match direction {
            StartPageNavigation::Next => self.start_page.next_widget(),
            StartPageNavigation::Previous => self.start_page.previous_widget(),
        }
    }

    /// Get selected quick action from start page
    pub fn get_start_page_quick_action(
        &self,
        action_id: &str,
    ) -> Option<&crate::ui::start_page::QuickAction> {
        self.start_page.get_quick_action(action_id)
    }

    // Calendar mode methods

    /// Show calendar interface
    pub fn show_calendar(&mut self) {
        self.mode = UIMode::Calendar;
        self.focused_pane = FocusedPane::Calendar;
        self.calendar_ui.set_focus(true);
        self.update_navigation_hints();
    }

    /// Hide calendar interface and return to normal mode
    pub fn hide_calendar(&mut self) {
        self.mode = UIMode::Normal;
        self.focused_pane = FocusedPane::FolderTree;
        self.calendar_ui.set_focus(false);
        self.update_navigation_hints();
    }

    /// Check if currently in calendar mode
    pub fn is_calendar_visible(&self) -> bool {
        matches!(self.mode, UIMode::Calendar)
    }

    /// Handle calendar key input
    pub async fn handle_calendar_key(
        &mut self,
        key: crossterm::event::KeyCode,
    ) -> Option<CalendarAction> {
        if self.mode == UIMode::Calendar {
            self.calendar_ui.handle_key(key).await
        } else {
            None
        }
    }

    /// Start creating a new event
    pub fn start_event_create(
        &mut self,
        calendars: Vec<crate::calendar::Calendar>,
        default_calendar_id: Option<String>,
    ) {
        self.event_form_ui = Some(crate::calendar::EventFormUI::new_create(
            calendars,
            default_calendar_id,
        ));
        self.mode = UIMode::EventCreate;
        self.focused_pane = FocusedPane::Calendar;
        self.update_navigation_hints();
    }

    /// Start editing an existing event
    pub fn start_event_edit(
        &mut self,
        event: crate::calendar::Event,
        calendars: Vec<crate::calendar::Calendar>,
    ) {
        self.event_form_ui = Some(crate::calendar::EventFormUI::new_edit(event, calendars));
        self.mode = UIMode::EventEdit;
        self.focused_pane = FocusedPane::Calendar;
        self.update_navigation_hints();
    }

    /// Start viewing an existing event (read-only)
    pub fn start_event_view(
        &mut self,
        event: crate::calendar::Event,
        calendars: Vec<crate::calendar::Calendar>,
    ) {
        self.event_form_ui = Some(crate::calendar::EventFormUI::new_view(event, calendars));
        self.mode = UIMode::EventView;
        self.focused_pane = FocusedPane::Calendar;
        self.update_navigation_hints();
    }

    /// Exit event form and return to calendar view
    pub fn exit_event_form(&mut self) {
        self.event_form_ui = None;
        self.mode = UIMode::Calendar;
        self.focused_pane = FocusedPane::Calendar;
        self.update_navigation_hints();
    }

    /// Check if currently in event form mode
    pub fn is_event_form_visible(&self) -> bool {
        matches!(
            self.mode,
            UIMode::EventCreate | UIMode::EventEdit | UIMode::EventView
        )
    }

    /// Handle event form key input
    pub async fn handle_event_form_key(
        &mut self,
        key: crossterm::event::KeyCode,
    ) -> Option<crate::calendar::EventFormAction> {
        let is_visible = self.is_event_form_visible();
        if is_visible {
            if let Some(ref mut event_form) = self.event_form_ui {
                return event_form.handle_key(key).await;
            }
        }
        None
    }

    /// Get current event form for external access
    pub fn event_form_ui(&self) -> Option<&crate::calendar::EventFormUI> {
        self.event_form_ui.as_ref()
    }

    /// Get mutable event form for external access
    pub fn event_form_ui_mut(&mut self) -> Option<&mut crate::calendar::EventFormUI> {
        self.event_form_ui.as_mut()
    }

    /// Set calendar events to display
    pub fn set_calendar_events(&mut self, events: Vec<crate::calendar::Event>) {
        self.calendar_ui.set_events(events);
    }

    /// Set available calendars
    pub fn set_calendars(&mut self, calendars: Vec<crate::calendar::Calendar>) {
        self.calendar_ui.set_calendars(calendars);
    }

    /// Get calendar UI for direct access
    pub fn calendar_ui(&self) -> &CalendarUI {
        &self.calendar_ui
    }

    /// Get mutable calendar UI for direct access
    pub fn calendar_ui_mut(&mut self) -> &mut CalendarUI {
        &mut self.calendar_ui
    }

    /// Get current calendar view mode
    pub fn calendar_view_mode(&self) -> CalendarViewMode {
        self.calendar_ui.current_view()
    }

    /// Set calendar view mode
    pub fn set_calendar_view_mode(&mut self, mode: CalendarViewMode) {
        self.calendar_ui.set_view_mode(mode);
    }

    /// Navigate calendar to today
    pub fn calendar_go_to_today(&mut self) {
        self.calendar_ui.navigate_to_today();
    }

    /// Set calendar enabled state
    pub fn set_calendar_enabled(&mut self, calendar_id: String, enabled: bool) {
        self.calendar_ui.set_calendar_enabled(calendar_id, enabled);
    }

    /// Show calendar list overlay
    pub fn show_calendar_list(&mut self) {
        self.calendar_ui.show_calendar_list();
    }

    /// Start email viewer mode with current message
    pub fn start_email_viewer(
        &mut self,
        message: crate::email::StoredMessage,
        email_content: crate::ui::content_preview::EmailContent,
    ) {
        self.email_viewer.set_email(message, email_content);
        self.mode = UIMode::EmailViewer;
    }

    /// Exit email viewer mode
    pub fn exit_email_viewer(&mut self) {
        self.mode = UIMode::Normal;
    }

    /// Check if email viewer is active
    pub fn is_email_viewer_active(&self) -> bool {
        matches!(self.mode, UIMode::EmailViewer)
    }

    /// Handle email viewer key input
    pub fn handle_email_viewer_key(
        &mut self,
        key: crossterm::event::KeyCode,
    ) -> Option<crate::ui::email_viewer::EmailViewerAction> {
        self.email_viewer.handle_key(key)
    }

    /// Get mutable reference to email viewer
    pub fn email_viewer_mut(&mut self) -> &mut crate::ui::email_viewer::EmailViewer {
        &mut self.email_viewer
    }

    /// Start invitation viewer mode
    pub fn start_invitation_viewer(
        &mut self,
        invitation: crate::calendar::MeetingInvitation,
        user_status: Option<crate::calendar::event::AttendeeStatus>,
        user_invited: bool,
    ) {
        self.invitation_viewer
            .set_invitation(invitation, user_status, user_invited);
        self.mode = UIMode::InvitationViewer;
    }

    /// Exit invitation viewer mode
    pub fn exit_invitation_viewer(&mut self) {
        self.invitation_viewer.clear();
        self.mode = UIMode::Normal;
    }

    /// Check if invitation viewer is active
    pub fn is_invitation_viewer_active(&self) -> bool {
        matches!(self.mode, UIMode::InvitationViewer)
    }

    /// Handle invitation viewer key input
    pub fn handle_invitation_viewer_key(&mut self, key: char) -> Option<InvitationAction> {
        self.invitation_viewer.handle_key(key)
    }

    /// Get mutable reference to invitation viewer
    pub fn invitation_viewer_mut(&mut self) -> &mut InvitationViewer {
        &mut self.invitation_viewer
    }

    /// Start search mode
    pub fn start_search(&mut self) {
        self.search_ui.start_search();
        self.mode = UIMode::Search;
        self.update_navigation_hints();
    }

    /// End search mode
    pub fn end_search(&mut self) {
        self.search_ui.end_search();
        self.mode = UIMode::Normal;
        self.update_navigation_hints();
    }

    /// Check if search is active
    pub fn is_search_active(&self) -> bool {
        matches!(self.mode, UIMode::Search)
    }

    /// Handle search key input
    pub fn handle_search_key(&mut self, key: crossterm::event::KeyCode) -> Option<SearchAction> {
        self.search_ui.handle_key(key)
    }

    /// Get search UI for direct access
    pub fn search_ui(&self) -> &SearchUI {
        &self.search_ui
    }

    /// Get mutable search UI for direct access
    pub fn search_ui_mut(&mut self) -> &mut SearchUI {
        &mut self.search_ui
    }

    /// Set search engine
    pub fn set_search_engine(&mut self, search_engine: SearchEngine) {
        self.search_engine = Some(search_engine);
    }

    /// Perform search with current query
    pub async fn perform_search(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref search_engine) = self.search_engine {
            if let Some(account_id) = self.get_current_account_id().cloned() {
                let query = self.search_ui.query().to_string();
                let mode = self.search_ui.mode().clone();

                if !query.is_empty() && query.len() >= 2 {
                    self.search_ui.set_searching(true);

                    match search_engine
                        .search(&account_id, &query, &mode, Some(50))
                        .await
                    {
                        Ok(results) => {
                            self.search_ui.set_results(results, 0); // TODO: Get actual search time
                        }
                        Err(e) => {
                            self.search_ui.set_error(format!("Search failed: {}", e));
                        }
                    }
                }
            } else {
                self.search_ui.set_error("No account selected".to_string());
            }
        } else {
            self.search_ui
                .set_error("Search engine not initialized".to_string());
        }

        Ok(())
    }

    /// Open selected search result
    pub async fn open_search_result(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(result) = self.search_ui.selected_result() {
            // Clone the necessary data to avoid borrowing conflicts
            let message_id = result.message.id;
            let account_id = result.message.account_id.clone();
            let folder_name = result.message.folder_name.clone();

            // Load the message in the content preview using the database ID
            self.content_preview.load_message_by_id(message_id).await?;

            // Switch to the folder containing the message
            if let Some(current_account) = self.get_current_account_id() {
                if current_account != &account_id {
                    self.switch_to_account(&account_id).await?;
                }

                // Load messages for the folder
                self.load_messages(account_id, folder_name).await?;

                // Find and select the message in the list using the database ID
                if let Some(index) = self
                    .message_list
                    .messages()
                    .iter()
                    .position(|msg| msg.message_id == Some(message_id))
                {
                    self.message_list.set_selected_index(index);
                }
            }

            // Close search and return to normal view
            self.end_search();
        }

        Ok(())
    }

    /// Show a notification message on the bottom powerline
    pub fn show_notification(&mut self, message: String, duration: Duration) {
        tracing::debug!("Showing notification: {}", message);
        self.notification_message = Some(message);
        self.notification_expires_at = Some(Instant::now() + duration);
    }

    /// Clear expired notifications
    pub fn update_notifications(&mut self) {
        if let Some(expires_at) = self.notification_expires_at {
            if Instant::now() >= expires_at {
                self.notification_message = None;
                self.notification_expires_at = None;
            }
        }
    }

    /// Get current notification message for rendering
    pub fn get_notification(&self) -> Option<&String> {
        self.notification_message.as_ref()
    }

    // Context-aware integration methods

    /// Switch to context-aware mode
    pub fn show_context_aware_interface(&mut self) {
        self.mode = UIMode::ContextAware;
        self.focused_pane = FocusedPane::MessageList;
        self.integrated_layout.set_view_mode(IntegratedViewMode::ContextAware);
        self.update_navigation_hints();
    }

    /// Check if currently in context-aware mode
    pub fn is_context_aware_active(&self) -> bool {
        matches!(self.mode, UIMode::ContextAware)
    }

    /// Handle context-aware calendar key input
    pub fn handle_context_calendar_key(&mut self, key: crossterm::event::KeyCode) -> Option<ContextCalendarAction> {
        if self.is_context_aware_active() {
            self.context_calendar.handle_key(key)
        } else {
            None
        }
    }

    /// Handle unified sidebar navigation
    pub fn handle_sidebar_key(&mut self, key: crossterm::event::KeyCode) -> Option<SidebarAction> {
        if self.is_context_aware_active() {
            self.unified_sidebar.handle_key(key)
        } else {
            None
        }
    }

    /// Update unified sidebar with current data
    pub fn update_sidebar_data(
        &mut self,
        accounts: &[(String, String, AccountSyncStatus, u32)],
        folders: &[(String, String, u32, bool, usize)],
        calendars: &[crate::calendar::Calendar],
    ) {
        self.unified_sidebar.update_data(accounts, folders, calendars);
    }

    /// Set context-aware calendar events and calendars
    pub fn update_context_calendar(&mut self, events: Vec<crate::calendar::Event>, calendars: Vec<crate::calendar::Calendar>) {
        self.context_calendar.update_calendar_data(events, calendars);
    }

    /// Toggle integrated layout view mode
    pub fn cycle_integrated_view_mode(&mut self) {
        let current_mode = self.integrated_layout.get_view_mode();
        let next_mode = match current_mode {
            IntegratedViewMode::EmailPrimary => IntegratedViewMode::CalendarPrimary,
            IntegratedViewMode::CalendarPrimary => IntegratedViewMode::SplitView,
            IntegratedViewMode::SplitView => IntegratedViewMode::ContextAware,
            IntegratedViewMode::ContextAware => IntegratedViewMode::EmailPrimary,
            IntegratedViewMode::FullScreen(_) => IntegratedViewMode::EmailPrimary,
        };
        self.integrated_layout.set_view_mode(next_mode);
    }

    /// Get current integrated view mode
    pub fn get_integrated_view_mode(&self) -> IntegratedViewMode {
        self.integrated_layout.get_view_mode()
    }

    /// Check calendar urgency level for priority handling
    pub fn get_calendar_urgency(&self) -> crate::ui::context_calendar::ContextUrgency {
        self.context_calendar.get_urgency_level()
    }

    /// Get mutable reference to context calendar for external updates
    pub fn context_calendar_mut(&mut self) -> &mut ContextAwareCalendar {
        &mut self.context_calendar
    }

    /// Get reference to unified sidebar
    pub fn unified_sidebar(&self) -> &UnifiedSidebar {
        &self.unified_sidebar
    }

    /// Get mutable reference to unified sidebar
    pub fn unified_sidebar_mut(&mut self) -> &mut UnifiedSidebar {
        &mut self.unified_sidebar
    }
}

/// Navigation directions for start page
#[derive(Debug, Clone, Copy)]
pub enum StartPageNavigation {
    Next,
    Previous,
}

impl Default for UI {
    fn default() -> Self {
        Self::new()
    }
}
