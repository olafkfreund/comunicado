pub mod account_inspector;
pub mod account_switcher;
pub mod animated_content;
pub mod animation;
pub mod calendar;
pub mod compose;
pub mod content_preview;
pub mod context_calendar;
pub mod context_menu;
pub mod context_shortcuts;
pub mod dynamic_shortcuts;
pub mod progressive_disclosure;
pub mod date_picker;
pub mod draft_list;
pub mod email_viewer;
pub mod enhanced_message_list;
pub mod folder_tree;
pub mod fuzzy_search;
pub mod graphics;
pub mod help;
pub mod integrated_layout;
pub mod invitation_viewer;
pub mod keyboard_shortcuts;
pub mod layout;
pub mod message_list;
pub mod search;
pub mod startup_progress;
pub mod status_bar;
pub mod sync_progress;
pub mod time_picker;
pub mod toast;
pub mod typography;
pub mod unified_sidebar;


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
    context_shortcuts::ContextShortcutsPopup,
    draft_list::DraftListUI,
    dynamic_shortcuts::{DynamicShortcutsManager, DynamicShortcutsRenderer, ShortcutContext, ShortcutDisplayMode, KeyboardShortcut, ShortcutCategory},
    progressive_disclosure::{ProgressiveDisclosureManager, ProgressiveDisclosureRenderer, Section, ExpandableSection, SectionContent},
    folder_tree::FolderTree,
    help::HelpOverlay,
    keyboard_shortcuts::KeyboardShortcutsUI,
    layout::AppLayout,
    message_list::MessageList,
    status_bar::{
        CalendarStatusSegment, EmailStatusSegment, NavigationHintsSegment, StatusBar, SyncStatus,
        SystemInfoSegment,
    },
    sync_progress::SyncProgressOverlay,
    toast::ToastManager,
    typography::{TypographySystem, InformationDensity},
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
pub use search::{SearchAction, SearchEngine, SearchEngineType, SearchMode, SearchResult, SearchUI};

// Re-export fuzzy search types
pub use fuzzy_search::{FuzzySearchEngine, FuzzySearchConfig};

// Re-export context menu types
pub use context_menu::{ContextMenu, ContextMenuAction, ContextMenuItem, ContextType};

// Re-export dynamic shortcuts types
// (Types already imported above in use self::dynamic_shortcuts::...)

// Re-export progressive disclosure types  
// (Types already imported above in use self::progressive_disclosure::...)

// Re-export animation and graphics types
pub use animated_content::{AnimatedContentManager, AnimatedEmailContent, AnimationControlWidget};
pub use animation::{Animation, AnimationDecoder, AnimationFormat, AnimationManager, AnimationSettings};
pub use graphics::{GraphicsProtocol, ImageRenderer, RenderConfig};

// Re-export integrated layout and context-aware calendar types
pub use context_calendar::{CalendarAction as ContextCalendarAction, ContextAwareCalendar, CalendarDisplayMode, EmailCalendarContext};
pub use integrated_layout::{IntegratedLayout, IntegratedLayoutManager, IntegratedViewMode, ContentType};
pub use unified_sidebar::{UnifiedSidebar, SidebarAction, NavigationItem, QuickActionType};

// Re-export help system types
pub use help::{HelpContent, HelpSection, KeyBinding, KeyBindingCategory};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPane {
    AccountSwitcher,
    FolderTree,
    MessageList,
    ContentPreview,
    Compose,
    DraftList,
    Calendar,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UIMode {
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
    ContactsPopup, // Quick contacts popup overlay
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
    calendar_ui: CalendarUI,
    event_form_ui: Option<crate::calendar::EventFormUI>,
    email_viewer: EmailViewer,
    invitation_viewer: InvitationViewer,
    search_ui: SearchUI,
    search_engine: Option<SearchEngine>,
    fuzzy_search_engine: Option<FuzzySearchEngine>,
    keyboard_shortcuts_ui: KeyboardShortcutsUI,
    context_shortcuts_popup: ContextShortcutsPopup,
    // Context-aware integration components
    context_calendar: ContextAwareCalendar,
    integrated_layout: IntegratedLayoutManager,
    unified_sidebar: UnifiedSidebar,
    // Modern toast notification system
    toast_manager: ToastManager,
    // Typography and visual hierarchy system
    typography: TypographySystem,
    // Contextual help overlay system
    help_overlay: HelpOverlay,
    // Context menu system
    context_menu: ContextMenu,
    // Progressive disclosure system
    progressive_disclosure_manager: ProgressiveDisclosureManager,
    // Dynamic shortcuts system
    dynamic_shortcuts_manager: DynamicShortcutsManager,
    // Legacy notification system (to be phased out)
    notification_message: Option<String>,
    notification_expires_at: Option<tokio::time::Instant>,
    // Contacts popup
    contacts_popup: Option<crate::contacts::ContactPopup>,
}

impl UI {
    /// Create a new UI instance with default components and configuration
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
            draft_list: DraftListUI::new(),
            calendar_ui: CalendarUI::new(),
            event_form_ui: None,
            email_viewer: EmailViewer::new(),
            invitation_viewer: InvitationViewer::new(),
            search_ui: SearchUI::new(),
            search_engine: None,
            fuzzy_search_engine: None,
            keyboard_shortcuts_ui: KeyboardShortcutsUI::new(),
            context_shortcuts_popup: ContextShortcutsPopup::new(),
            // Initialize context-aware integration components
            context_calendar: ContextAwareCalendar::new(),
            integrated_layout: IntegratedLayoutManager::new(),
            unified_sidebar: UnifiedSidebar::new(),
            // Initialize modern toast notification system
            toast_manager: ToastManager::new(),
            // Initialize typography and visual hierarchy system
            typography: TypographySystem::new(),
            // Initialize contextual help overlay system
            help_overlay: HelpOverlay::new(),
            // Initialize context menu system
            context_menu: ContextMenu::new(),
            // Initialize progressive disclosure system
            progressive_disclosure_manager: ProgressiveDisclosureManager::new(),
            // Initialize dynamic shortcuts system
            dynamic_shortcuts_manager: DynamicShortcutsManager::new(),
            // Initialize legacy notification system (to be phased out)
            notification_message: None,
            notification_expires_at: None,
            // Initialize contacts popup
            contacts_popup: None,
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
            next_event: None,
            events_today: 0,
            next_event_time: None,
            urgent_events: 0,
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
            UIMode::ContactsPopup => {
                // Render contacts popup over the normal interface
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

                // Render contacts popup on top
                if let Some(ref mut contacts_popup) = self.contacts_popup {
                    let theme = self.theme_manager.current_theme();
                    contacts_popup.render(frame, size, theme);
                }
            }
        }

        // Render toast notifications on top of everything
        let theme = self.theme_manager.current_theme();
        if self.toast_manager.has_toasts() {
            crate::ui::toast::ToastRenderer::render(frame, size, self.toast_manager.toasts(), theme);
        }

        // Render context shortcuts popup on top of everything if visible
        self.context_shortcuts_popup.render(frame, size, theme, &self.mode);

        // Render help overlay on top of everything if visible
        if self.help_overlay.is_visible() {
            self.help_overlay.render(frame, size, theme, &self.typography);
        }

        // Render context menu on top of everything if visible
        if self.context_menu.is_visible() {
            self.context_menu.render(frame, size, theme, &self.typography);
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
            // Enhanced status bar rendering with typography
            self.status_bar.render_with_typography(frame, area, theme, &self.typography);
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
            FocusedPane::DraftList => FocusedPane::DraftList, // Stay in draft list
            FocusedPane::Calendar => FocusedPane::Calendar, // Stay in calendar
        };
        self.update_navigation_hints();
    }

    /// Get the currently focused pane
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
                FocusedPane::DraftList => "Draft List", // Shouldn't happen in normal mode
                FocusedPane::Calendar => "Calendar", // Shouldn't happen in normal mode
            },
            UIMode::Compose => "Compose Email",
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
            UIMode::ContactsPopup => "Contacts",
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
                    ("g".to_string(), "Calendar".to_string()),
                    ("Ctrl+R".to_string(), "Refresh".to_string()),
                    ("F5".to_string(), "Sync".to_string()),
                ],
                FocusedPane::FolderTree => vec![
                    ("Tab".to_string(), "Switch".to_string()),
                    ("j/k".to_string(), "Navigate".to_string()),
                    ("l".to_string(), "Expand".to_string()),
                    ("h".to_string(), "Collapse".to_string()),
                    ("c".to_string(), "Compose".to_string()),
                    ("g".to_string(), "Calendar".to_string()),
                ],
                FocusedPane::MessageList => vec![
                    ("Tab".to_string(), "Switch".to_string()),
                    ("j/k".to_string(), "Navigate".to_string()),
                    ("Enter".to_string(), "Open".to_string()),
                    ("c".to_string(), "Compose".to_string()),
                    ("r".to_string(), "Reply".to_string()),
                    ("f".to_string(), "Forward".to_string()),
                    ("g".to_string(), "Calendar".to_string()),
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
                    ("g".to_string(), "Calendar".to_string()),
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
                ("F3".to_string(), "Calendar".to_string()),
                ("e".to_string(), "New Event".to_string()),
                ("T".to_string(), "New Todo".to_string()),
                ("t".to_string(), "View Todos".to_string()),
                ("1-4".to_string(), "Day/Week/Month/Agenda".to_string()),
                ("←→".to_string(), "Prev/Next Month".to_string()),
                (".".to_string(), "Today".to_string()),
                ("Enter".to_string(), "Event Details".to_string()),
                ("Ctrl+e".to_string(), "Edit Event".to_string()),
                ("Del".to_string(), "Delete Event".to_string()),
                ("Space".to_string(), "Toggle Todo".to_string()),
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
                ("Ctrl+S".to_string(), "Save Changes".to_string()),
                ("d".to_string(), "Delete Event".to_string()),
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
                ("c".to_string(), "Add Contact".to_string()),
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
            UIMode::ContactsPopup => vec![
                ("↑↓/j/k".to_string(), "Navigate".to_string()),
                ("Enter".to_string(), "Select Contact".to_string()),
                ("/".to_string(), "Search".to_string()),
                ("Tab".to_string(), "Change Mode".to_string()),
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

    /// Update calendar status with real-time calendar information
    pub fn update_calendar_status(
        &mut self,
        next_event: Option<String>,
        events_today: usize,
        next_event_time: Option<chrono::DateTime<chrono::Local>>,
        urgent_events: usize,
    ) {
        let calendar_segment = CalendarStatusSegment {
            next_event,
            events_today,
            next_event_time,
            urgent_events,
        };
        self.status_bar
            .add_segment("calendar".to_string(), calendar_segment);
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

    /// Refresh all status bar segments with current data
    pub fn refresh_status_bar(&mut self) {
        // Update system time
        self.update_system_time(chrono::Local::now().format("%H:%M").to_string());
        
        // Update navigation hints for current mode
        self.update_navigation_hints();
        
        // Update email status if we have message data
        let message_count = self.message_list.messages().len();
        let unread_count = self
            .message_list
            .messages()
            .iter()
            .filter(|msg| !msg.is_read)
            .count();
        
        // Determine sync status based on current state
        let sync_status = if self.is_sync_progress_visible() {
            SyncStatus::Syncing
        } else {
            SyncStatus::Online
        };
        
        self.update_email_status(unread_count, message_count, sync_status);
        
        // Update calendar status with current calendar data
        self.refresh_calendar_status();
    }

    /// Update calendar status with data from calendar UI
    fn refresh_calendar_status(&mut self) {
        // Get current date for today's events calculation
        let today = chrono::Local::now().date_naive();
        let now = chrono::Utc::now();
        
        // Get calendar events from calendar UI
        let events = self.calendar_ui.get_events();
        
        // Calculate events today
        let events_today = events
            .iter()
            .filter(|event| {
                let local_start = event.start_time.with_timezone(&chrono::Local);
                local_start.date_naive() == today
            })
            .count();
        
        // Find next upcoming event
        let mut upcoming_events: Vec<_> = events
            .iter()
            .filter(|event| event.start_time > now)
            .collect();
        upcoming_events.sort_by_key(|event| event.start_time);
        
        let (next_event, next_event_time) = if let Some(next) = upcoming_events.first() {
            let local_time = next.start_time.with_timezone(&chrono::Local);
            (Some(next.title.clone()), Some(local_time))
        } else {
            (None, None)
        };
        
        // Calculate urgent events (events starting within 1 hour)
        let urgent_threshold = now + chrono::Duration::hours(1);
        let urgent_events = upcoming_events
            .iter()
            .filter(|event| event.start_time <= urgent_threshold)
            .count();
        
        self.update_calendar_status(next_event, events_today, next_event_time, urgent_events);
    }

    /// Set the database for email operations
    pub fn set_database(&mut self, database: Arc<EmailDatabase>) {
        self.message_list.set_database(database.clone());
        self.content_preview.set_database(database.clone());
        self.folder_tree.set_database(database.clone());
        
        // Initialize search engines with database
        self.search_engine = Some(SearchEngine::new(database.clone()));
        self.fuzzy_search_engine = Some(FuzzySearchEngine::new(database));
    }

    /// Set the contacts manager and initialize sender recognition
    pub fn set_contacts_manager(&mut self, contacts_manager: Arc<crate::contacts::ContactsManager>) {
        use crate::contacts::SenderRecognitionService;
        
        // Create sender recognition service
        let sender_recognition = Arc::new(SenderRecognitionService::new(contacts_manager));
        
        // Set it up in the message list
        self.message_list.set_sender_recognition(sender_recognition);
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
            let key_event = crossterm::event::KeyEvent::new(key, crossterm::event::KeyModifiers::empty());
            Some(compose_ui.handle_key(key_event).await)
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


    /// Show keyboard shortcuts popup
    pub fn show_keyboard_shortcuts(&mut self) {
        self.mode = UIMode::KeyboardShortcuts;
    }

    /// Toggle context shortcuts popup visibility
    pub fn toggle_context_shortcuts(&mut self) {
        self.context_shortcuts_popup.toggle();
    }

    /// Show context shortcuts popup
    pub fn show_context_shortcuts(&mut self) {
        self.context_shortcuts_popup.show();
    }

    /// Hide context shortcuts popup
    pub fn hide_context_shortcuts(&mut self) {
        self.context_shortcuts_popup.hide();
    }

    /// Check if context shortcuts popup is visible
    pub fn is_context_shortcuts_visible(&self) -> bool {
        self.context_shortcuts_popup.is_visible()
    }

    /// Handle context shortcuts popup navigation
    pub fn context_shortcuts_scroll_up(&mut self) {
        self.context_shortcuts_popup.scroll_up();
    }

    /// Handle context shortcuts popup navigation
    pub fn context_shortcuts_scroll_down(&mut self) {
        self.context_shortcuts_popup.scroll_down();
    }

    /// Set initial UI mode based on CLI arguments
    pub fn set_initial_mode(&mut self, mode: crate::cli::StartupMode) {
        use crate::cli::StartupMode;
        match mode {
            StartupMode::Default | StartupMode::Email => {
                self.mode = UIMode::Normal;
                self.focused_pane = FocusedPane::AccountSwitcher;
            }
            StartupMode::Calendar => {
                self.show_calendar();
            }
            StartupMode::Contacts => {
                // TODO: Implement contacts mode when available
                // For now, fall back to email mode
                self.mode = UIMode::Normal;
                self.focused_pane = FocusedPane::AccountSwitcher;
            }
        }
    }

    /// Switch to normal email mode
    pub fn show_email_interface(&mut self) {
        self.mode = UIMode::Normal;
        self.focused_pane = FocusedPane::AccountSwitcher;
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
                let key_event = crossterm::event::KeyEvent::new(key, crossterm::event::KeyModifiers::empty());
                return event_form.handle_key(key_event).await;
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

    /// Get reference to email viewer
    pub fn email_viewer(&self) -> &crate::ui::email_viewer::EmailViewer {
        &self.email_viewer
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

    /// Perform fuzzy search with current query and live search-as-you-type
    pub async fn perform_fuzzy_search(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Get necessary data before borrowing fuzzy_search_engine
        let account_id = self.get_current_account_id().cloned();
        let query = self.search_ui.query().to_string();
        let mode = self.search_ui.mode().clone();
        let should_search = self.search_ui.should_search();

        if let Some(ref mut fuzzy_search_engine) = self.fuzzy_search_engine {
            if let Some(account_id) = account_id {
                if should_search {
                    self.search_ui.set_searching(true);

                    let search_start = std::time::Instant::now();
                    match fuzzy_search_engine
                        .live_search(&account_id, &query, &mode)
                        .await
                    {
                        Ok(results) => {
                            let search_time = search_start.elapsed().as_millis() as u64;
                            self.search_ui.set_results(results, search_time);
                        }
                        Err(e) => {
                            self.search_ui.set_error(format!("Fuzzy search failed: {}", e));
                        }
                    }
                }
            } else {
                self.search_ui.set_error("No account selected".to_string());
            }
        } else {
            self.search_ui
                .set_error("Fuzzy search engine not initialized".to_string());
        }

        Ok(())
    }

    /// Get fuzzy search engine for configuration
    pub fn fuzzy_search_engine(&self) -> Option<&FuzzySearchEngine> {
        self.fuzzy_search_engine.as_ref()
    }

    /// Get mutable fuzzy search engine for configuration updates
    pub fn fuzzy_search_engine_mut(&mut self) -> Option<&mut FuzzySearchEngine> {
        self.fuzzy_search_engine.as_mut()
    }

    /// Update fuzzy search configuration
    pub fn update_fuzzy_search_config(&mut self, config: FuzzySearchConfig) {
        if let Some(ref mut fuzzy_search_engine) = self.fuzzy_search_engine {
            fuzzy_search_engine.update_config(config);
        }
    }

    /// Clear fuzzy search cache
    pub fn clear_fuzzy_search_cache(&mut self) {
        if let Some(ref mut fuzzy_search_engine) = self.fuzzy_search_engine {
            fuzzy_search_engine.clear_cache();
        }
    }

    /// Show context menu at specific position
    pub fn show_context_menu_at(&mut self, x: u16, y: u16, context_type: ContextType) {
        self.context_menu.show_at_position(x, y, context_type);
    }

    /// Show context menu at cursor position (keyboard triggered)
    pub fn show_context_menu(&mut self, context_type: ContextType) {
        self.context_menu.show_at_cursor(context_type);
    }

    /// Hide context menu
    pub fn hide_context_menu(&mut self) {
        self.context_menu.hide();
    }

    /// Check if context menu is visible
    pub fn is_context_menu_visible(&self) -> bool {
        self.context_menu.is_visible()
    }

    /// Handle context menu key input
    pub fn handle_context_menu_key(&mut self, key: crossterm::event::KeyCode) -> Option<ContextMenuAction> {
        self.context_menu.handle_key(key)
    }

    /// Handle context menu mouse click
    pub fn handle_context_menu_click(&mut self, x: u16, y: u16) -> Option<ContextMenuAction> {
        self.context_menu.handle_mouse_click(x, y)
    }

    /// Get context menu for direct access
    pub fn context_menu(&self) -> &ContextMenu {
        &self.context_menu
    }

    /// Get mutable context menu for direct access
    pub fn context_menu_mut(&mut self) -> &mut ContextMenu {
        &mut self.context_menu
    }

    /// Get current context type based on UI state
    pub fn get_current_context_type(&self) -> ContextType {
        match self.focused_pane {
            FocusedPane::MessageList => {
                // Check if there's a selected message
                if let Some(selected_message) = self.message_list.selected_message() {
                    ContextType::EmailMessage {
                        is_read: selected_message.is_read,
                        is_draft: false, // TODO: Determine if message is draft
                        has_attachments: selected_message.has_attachments,
                        folder_name: "INBOX".to_string(), // TODO: Get actual folder name
                    }
                } else {
                    ContextType::General
                }
            }
            FocusedPane::FolderTree => {
                // Check if there's a selected folder
                if let Some(selected_folder) = self.folder_tree.selected_folder() {
                    let is_special = matches!(selected_folder.name.as_str(), "INBOX" | "Sent" | "Drafts" | "Trash" | "Spam");
                    let unread_count = selected_folder.unread_count;
                    
                    ContextType::EmailFolder {
                        folder_name: selected_folder.name.clone(),
                        is_special,
                        unread_count,
                    }
                } else {
                    ContextType::General
                }
            }
            FocusedPane::AccountSwitcher => {
                // Check if there's a current account
                if let Some(account) = self.account_switcher.get_current_account() {
                    ContextType::Account {
                        account_id: account.account_id.clone(),
                        is_online: matches!(account.sync_status, AccountSyncStatus::Online | AccountSyncStatus::Syncing),
                    }
                } else {
                    ContextType::General
                }
            }
            _ => ContextType::General,
        }
    }

    /// Get context type for calendar events
    pub fn get_calendar_context_type(&self, event_id: Option<String>) -> ContextType {
        if let Some(event_id) = event_id {
            ContextType::CalendarEvent {
                event_id,
                is_recurring: false, // TODO: Determine if event is recurring
                is_editable: true,   // TODO: Determine if event is editable
            }
        } else {
            ContextType::General
        }
    }

    /// Show context menu for current selection with keyboard trigger
    pub fn show_context_menu_for_current(&mut self) {
        let context_type = self.get_current_context_type();
        self.show_context_menu(context_type);
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

    /// Show contacts popup
    pub fn show_contacts_popup(&mut self, contacts_manager: Arc<crate::contacts::ContactsManager>) {
        self.contacts_popup = Some(crate::contacts::ContactPopup::new(contacts_manager));
        self.mode = UIMode::ContactsPopup;
        self.update_navigation_hints();
    }

    /// Hide contacts popup and return to normal mode
    pub fn hide_contacts_popup(&mut self) {
        self.contacts_popup = None;
        self.mode = UIMode::Normal;
        self.update_navigation_hints();
    }

    /// Check if contacts popup is visible
    pub fn is_contacts_popup_visible(&self) -> bool {
        matches!(self.mode, UIMode::ContactsPopup)
    }

    /// Handle contacts popup key input
    pub async fn handle_contacts_popup_key(
        &mut self,
        key: crossterm::event::KeyCode,
    ) -> Option<crate::contacts::ContactPopupAction> {
        if let Some(ref mut contacts_popup) = self.contacts_popup {
            contacts_popup.handle_key(key).await
        } else {
            None
        }
    }

    /// Get selected contact from popup
    pub fn get_selected_contact_from_popup(&self) -> Option<&crate::contacts::Contact> {
        self.contacts_popup.as_ref()?.get_selected_contact()
    }

    /// Show contacts popup with a specific contact pre-selected
    pub fn show_contacts_popup_with_contact(
        &mut self,
        contacts_manager: Arc<crate::contacts::ContactsManager>,
        contact: crate::contacts::Contact,
    ) {
        let mut contacts_popup = crate::contacts::ContactPopup::new(contacts_manager);
        contacts_popup.show_contact_details(contact);
        self.contacts_popup = Some(contacts_popup);
        self.mode = UIMode::ContactsPopup;
        self.update_navigation_hints();
    }

    /// Show contact edit dialog
    pub fn show_contact_edit_dialog(
        &mut self,
        contacts_manager: Arc<crate::contacts::ContactsManager>,
        contact: crate::contacts::Contact,
    ) {
        let mut contacts_popup = crate::contacts::ContactPopup::new(contacts_manager);
        contacts_popup.start_edit_contact(contact);
        self.contacts_popup = Some(contacts_popup);
        self.mode = UIMode::ContactsPopup;
        self.update_navigation_hints();
    }

    /// Show contact context menu for quick actions
    pub fn show_contact_context_menu(
        &mut self,
        email: String,
        contact: Option<crate::contacts::Contact>,
    ) {
        // For now, show a notification with available actions
        match contact {
            Some(contact) => {
                let message = format!(
                    "Contact: {} <{}> - Press 'i' to view, Ctrl+i to edit, 'x' to remove", 
                    contact.display_name, 
                    email
                );
                self.show_notification(message, Duration::from_secs(5));
            }
            None => {
                let message = format!(
                    "Email: {} - Press 'a' to add as contact", 
                    email
                );
                self.show_notification(message, Duration::from_secs(3));
            }
        }
    }

    // ===== MODERN TOAST NOTIFICATION SYSTEM =====

    /// Show an info toast notification
    pub fn show_toast_info<S: Into<String>>(&mut self, message: S) {
        self.toast_manager.info(message);
    }

    /// Show a success toast notification
    pub fn show_toast_success<S: Into<String>>(&mut self, message: S) {
        self.toast_manager.success(message);
    }

    /// Show a warning toast notification
    pub fn show_toast_warning<S: Into<String>>(&mut self, message: S) {
        self.toast_manager.warning(message);
    }

    /// Show an error toast notification
    pub fn show_toast_error<S: Into<String>>(&mut self, message: S) {
        self.toast_manager.error(message);
    }

    /// Show a custom toast notification with specific level and duration
    pub fn show_custom_toast<S: Into<String>>(
        &mut self, 
        message: S, 
        level: crate::tea::message::ToastLevel, 
        duration: Duration
    ) {
        self.toast_manager.show_with_duration(message.into(), level, duration);
    }

    /// Show a persistent toast (longer duration)
    pub fn show_persistent_toast<S: Into<String>>(
        &mut self, 
        message: S, 
        level: crate::tea::message::ToastLevel
    ) {
        self.toast_manager.persistent(message, level);
    }

    /// Show a quick toast (shorter duration)
    pub fn show_quick_toast<S: Into<String>>(
        &mut self, 
        message: S, 
        level: crate::tea::message::ToastLevel
    ) {
        self.toast_manager.quick(message, level);
    }

    /// Update toast system (should be called every frame)
    pub fn update_toasts(&mut self) {
        self.toast_manager.update();
    }

    /// Remove a specific toast by ID
    pub fn remove_toast(&mut self, toast_id: &str) {
        self.toast_manager.remove_toast(toast_id);
    }

    /// Clear all active toasts
    pub fn clear_toasts(&mut self) {
        self.toast_manager.clear();
    }

    /// Check if there are any active toasts
    pub fn has_active_toasts(&self) -> bool {
        self.toast_manager.has_toasts()
    }

    /// Get access to the toast manager for advanced operations
    pub fn toast_manager(&mut self) -> &mut ToastManager {
        &mut self.toast_manager
    }

    // ===== TYPOGRAPHY AND VISUAL HIERARCHY SYSTEM =====

    /// Get access to the typography system
    pub fn typography(&self) -> &TypographySystem {
        &self.typography
    }

    /// Get mutable access to the typography system
    pub fn typography_mut(&mut self) -> &mut TypographySystem {
        &mut self.typography
    }

    /// Set information density for the interface
    pub fn set_information_density(&mut self, density: InformationDensity) {
        self.typography = self.typography.clone().with_density(density);
    }

    /// Get current information density
    pub fn information_density(&self) -> InformationDensity {
        self.typography.density()
    }

    /// Cycle to next information density mode
    pub fn cycle_information_density(&mut self) {
        let next_density = match self.typography.density() {
            InformationDensity::Compact => InformationDensity::Comfortable,
            InformationDensity::Comfortable => InformationDensity::Relaxed,
            InformationDensity::Relaxed => InformationDensity::Compact,
        };
        self.set_information_density(next_density);
        
        // Show toast to inform user of the change
        let density_name = match next_density {
            InformationDensity::Compact => "Compact",
            InformationDensity::Comfortable => "Comfortable",
            InformationDensity::Relaxed => "Relaxed",
        };
        self.show_toast_info(format!("Information density: {}", density_name));
    }

    /// Show contextual help overlay for current view mode
    pub fn show_help(&mut self, view_mode: crate::tea::message::ViewMode) {
        self.help_overlay.show(view_mode);
        self.show_toast_info("Help overlay displayed. Press Ctrl+H or ? to close".to_string());
    }

    /// Hide contextual help overlay
    pub fn hide_help(&mut self) {
        self.help_overlay.hide();
    }

    /// Toggle contextual help overlay for current view mode
    pub fn toggle_help(&mut self, view_mode: crate::tea::message::ViewMode) {
        self.help_overlay.toggle(view_mode);
        
        if self.help_overlay.is_visible() {
            self.show_toast_info("Help overlay displayed. Press Ctrl+H, ? or Esc to close".to_string());
        }
    }

    /// Check if help overlay is currently visible
    pub fn is_help_visible(&self) -> bool {
        self.help_overlay.is_visible()
    }

    /// Get mutable reference to progressive disclosure manager
    pub fn progressive_disclosure_manager_mut(&mut self) -> &mut ProgressiveDisclosureManager {
        &mut self.progressive_disclosure_manager
    }

    /// Get reference to progressive disclosure manager  
    pub fn progressive_disclosure_manager(&self) -> &ProgressiveDisclosureManager {
        &self.progressive_disclosure_manager
    }

    /// Add a section to progressive disclosure
    pub fn add_disclosure_section(&mut self, section: Section) {
        self.progressive_disclosure_manager.add_section(section);
    }

    /// Toggle a section's expanded state
    pub fn toggle_disclosure_section(&mut self, section_id: &str) -> bool {
        let result = self.progressive_disclosure_manager.toggle_section(section_id);
        if result {
            let section = self.progressive_disclosure_manager.get_section(section_id);
            if let Some(section) = section {
                let state = if section.is_expanded() { "expanded" } else { "collapsed" };
                self.show_toast_info(format!("Section '{}' {}", section.meta.title, state));
            }
        }
        result
    }

    /// Expand all disclosure sections
    pub fn expand_all_sections(&mut self) {
        self.progressive_disclosure_manager.expand_all();
        self.show_toast_info("All sections expanded".to_string());
    }

    /// Collapse all disclosure sections
    pub fn collapse_all_sections(&mut self) {
        self.progressive_disclosure_manager.collapse_all();
        self.show_toast_info("All sections collapsed".to_string());
    }

    /// Toggle global disclosure state
    pub fn toggle_global_disclosure(&mut self) {
        let was_expanded = self.progressive_disclosure_manager.expanded_section_count() > 0;
        self.progressive_disclosure_manager.toggle_global();
        let message = if was_expanded {
            "All sections collapsed"
        } else {
            "All sections expanded"
        };
        self.show_toast_info(message.to_string());
    }

    /// Navigate to next disclosure section
    pub fn next_disclosure_section(&mut self) -> Option<String> {
        let result = self.progressive_disclosure_manager.next_section();
        if let Some(ref section_id) = result {
            if let Some(section) = self.progressive_disclosure_manager.get_section(section_id) {
                self.show_toast_info(format!("Focused: {}", section.meta.title));
            }
        }
        result 
    }

    /// Navigate to previous disclosure section
    pub fn prev_disclosure_section(&mut self) -> Option<String> {
        let result = self.progressive_disclosure_manager.previous_section();
        if let Some(ref section_id) = result {
            if let Some(section) = self.progressive_disclosure_manager.get_section(section_id) {
                self.show_toast_info(format!("Focused: {}", section.meta.title));
            }
        }
        result
    }

    /// Setup default progressive disclosure sections for email view
    pub fn setup_email_disclosure_sections(&mut self) {
        // Email metadata section
        let email_meta = ExpandableSection::new(
            "email_metadata".to_string(),
            "Email Metadata".to_string()
        )
        .with_subtitle("Headers and technical details".to_string())
        .expanded(false)
        .with_priority(1);

        let meta_content = SectionContent::KeyValue(vec![
            ("From".to_string(), "sender@example.com".to_string()),
            ("To".to_string(), "recipient@example.com".to_string()),
            ("Subject".to_string(), "Email subject line".to_string()),
            ("Date".to_string(), "2025-01-31 12:00:00".to_string()),
        ]);

        self.add_disclosure_section(Section::new(email_meta, meta_content));

        // Attachments section
        let attachments = ExpandableSection::new(
            "attachments".to_string(),
            "Attachments".to_string()
        )
        .with_item_count(3)
        .expanded(true)
        .with_priority(2);

        let attachment_content = SectionContent::List(vec![
            "document.pdf (1.2 MB)".to_string(),
            "image.jpg (850 KB)".to_string(),
            "spreadsheet.xlsx (2.1 MB)".to_string(),
        ]);

        self.add_disclosure_section(Section::new(attachments, attachment_content));

        // Email actions section
        let actions = ExpandableSection::new(
            "email_actions".to_string(),
            "Quick Actions".to_string()
        )
        .expanded(false)
        .with_priority(3);

        let action_content = SectionContent::List(vec![
            "Reply (R)".to_string(),
            "Forward (F)".to_string(),
            "Archive (A)".to_string(),
            "Delete (Del)".to_string(),
            "Mark as unread (U)".to_string(),
        ]);

        self.add_disclosure_section(Section::new(actions, action_content));
    }

    /// Setup default progressive disclosure sections for calendar view
    pub fn setup_calendar_disclosure_sections(&mut self) {
        // Event details section
        let event_details = ExpandableSection::new(
            "event_details".to_string(),
            "Event Details".to_string()
        )
        .expanded(true)
        .with_priority(1);

        let details_content = SectionContent::KeyValue(vec![
            ("Title".to_string(), "Team Meeting".to_string()),
            ("Start".to_string(), "2025-01-31 14:00".to_string()),
            ("End".to_string(), "2025-01-31 15:00".to_string()),
            ("Location".to_string(), "Conference Room A".to_string()),
            ("Attendees".to_string(), "5 people".to_string()),
        ]);

        self.add_disclosure_section(Section::new(event_details, details_content));

        // Calendar views section
        let views = ExpandableSection::new(
            "calendar_views".to_string(),
            "View Options".to_string()
        )
        .expanded(false)
        .with_priority(2);

        let views_content = SectionContent::List(vec![
            "Day view (D)".to_string(),
            "Week view (W)".to_string(),
            "Month view (M)".to_string(),
            "Agenda view (A)".to_string(),
        ]);

        self.add_disclosure_section(Section::new(views, views_content));

        // Calendar actions section  
        let cal_actions = ExpandableSection::new(
            "calendar_actions".to_string(),
            "Calendar Actions".to_string()
        )
        .expanded(false)
        .with_priority(3);

        let cal_action_content = SectionContent::List(vec![
            "Create event (N)".to_string(),
            "Edit event (E)".to_string(),
            "Delete event (Del)".to_string(),
            "Export calendar (Ctrl+E)".to_string(),
            "Sync calendars (F5)".to_string(),
        ]);

        self.add_disclosure_section(Section::new(cal_actions, cal_action_content));
    }

    /// Render progressive disclosure sections in a given area
    pub fn render_progressive_disclosure(&self, frame: &mut Frame, area: Rect) {
        let theme = self.theme_manager.current_theme();
        ProgressiveDisclosureRenderer::render_sections(
            frame,
            area, 
            &self.progressive_disclosure_manager,
            theme,
            &self.typography
        );
    }

    /// Get mutable reference to dynamic shortcuts manager
    pub fn dynamic_shortcuts_manager_mut(&mut self) -> &mut DynamicShortcutsManager {
        &mut self.dynamic_shortcuts_manager
    }

    /// Get reference to dynamic shortcuts manager
    pub fn dynamic_shortcuts_manager(&self) -> &DynamicShortcutsManager {
        &self.dynamic_shortcuts_manager
    }

    /// Set shortcut context and update available shortcuts
    pub fn set_shortcut_context(&mut self, context: ShortcutContext) {
        self.dynamic_shortcuts_manager.set_context(context);
    }

    /// Set shortcut display mode
    pub fn set_shortcut_display_mode(&mut self, mode: ShortcutDisplayMode) {
        self.dynamic_shortcuts_manager.set_display_mode(mode);
        
        let mode_name = match mode {
            ShortcutDisplayMode::StatusBar => "Status bar",
            ShortcutDisplayMode::Popup => "Popup",
            ShortcutDisplayMode::Overlay => "Full overlay",
            ShortcutDisplayMode::Inline => "Inline hints",
            ShortcutDisplayMode::Hidden => "Hidden",
        };
        self.show_toast_info(format!("Shortcut hints: {}", mode_name));
    }

    /// Toggle shortcut display mode
    pub fn toggle_shortcut_display_mode(&mut self) {
        let current_mode = self.dynamic_shortcuts_manager.display_mode();
        let next_mode = match current_mode {
            ShortcutDisplayMode::StatusBar => ShortcutDisplayMode::Popup,
            ShortcutDisplayMode::Popup => ShortcutDisplayMode::Overlay,
            ShortcutDisplayMode::Overlay => ShortcutDisplayMode::Inline,
            ShortcutDisplayMode::Inline => ShortcutDisplayMode::Hidden,
            ShortcutDisplayMode::Hidden => ShortcutDisplayMode::StatusBar,
        };
        self.set_shortcut_display_mode(next_mode);
    }

    /// Force show shortcut hints (reset auto-hide timer)
    pub fn show_shortcut_hints(&mut self) {
        self.dynamic_shortcuts_manager.show_hints();
        self.show_toast_info("Shortcut hints displayed".to_string());
    }

    /// Add custom shortcut
    pub fn add_custom_shortcut(&mut self, id: String, shortcut: KeyboardShortcut) {
        self.dynamic_shortcuts_manager.add_custom_shortcut(id, shortcut);
    }

    /// Setup contextual shortcuts for email list view
    pub fn setup_email_list_shortcuts(&mut self, has_selection: bool, can_compose: bool, folder_name: String) {
        let context = ShortcutContext::EmailList {
            has_selection,
            can_compose,
            folder_name,
        };
        self.set_shortcut_context(context);
    }

    /// Setup contextual shortcuts for email reading view
    pub fn setup_email_reading_shortcuts(&mut self, is_draft: bool, has_attachments: bool, can_reply: bool) {
        let context = ShortcutContext::EmailReading {
            is_draft,
            has_attachments,
            can_reply,
        };
        self.set_shortcut_context(context);
    }

    /// Setup contextual shortcuts for calendar view
    pub fn setup_calendar_shortcuts(&mut self, view_mode: crate::calendar::CalendarViewMode, has_selection: bool, can_create: bool) {
        let shortcut_view_mode = match view_mode {
            crate::calendar::CalendarViewMode::Day => crate::ui::dynamic_shortcuts::CalendarViewMode::Day,
            crate::calendar::CalendarViewMode::Week => crate::ui::dynamic_shortcuts::CalendarViewMode::Week,
            crate::calendar::CalendarViewMode::Month => crate::ui::dynamic_shortcuts::CalendarViewMode::Month,
            crate::calendar::CalendarViewMode::Agenda => crate::ui::dynamic_shortcuts::CalendarViewMode::Agenda,
        };
        
        let context = ShortcutContext::Calendar {
            view_mode: shortcut_view_mode,
            has_selection,
            can_create,
        };
        self.set_shortcut_context(context);
    }

    /// Setup contextual shortcuts for contacts view
    pub fn setup_contacts_shortcuts(&mut self, has_selection: bool, can_edit: bool) {
        let context = ShortcutContext::Contacts {
            has_selection,
            can_edit,
        };
        self.set_shortcut_context(context);
    }

    /// Setup contextual shortcuts for search interface
    pub fn setup_search_shortcuts(&mut self, is_active: bool, has_results: bool, search_type: String) {
        let shortcut_search_type = match search_type.as_str() {
            "email" => crate::ui::dynamic_shortcuts::SearchType::Email,
            "calendar" => crate::ui::dynamic_shortcuts::SearchType::Calendar,
            "contacts" => crate::ui::dynamic_shortcuts::SearchType::Contacts,
            _ => crate::ui::dynamic_shortcuts::SearchType::Global,
        };
        
        let context = ShortcutContext::Search {
            is_active,
            has_results,
            search_type: shortcut_search_type,
        };
        self.set_shortcut_context(context);
    }

    /// Setup contextual shortcuts for compose interface
    pub fn setup_compose_shortcuts(&mut self, is_draft: bool, has_content: bool, can_send: bool) {
        let context = ShortcutContext::Compose {
            is_draft,
            has_content,
            can_send,
        };
        self.set_shortcut_context(context);
    }

    /// Render dynamic shortcut hints in status bar
    pub fn render_shortcut_hints_status_bar(&self, frame: &mut Frame, area: Rect) {
        let theme = self.theme_manager.current_theme();
        DynamicShortcutsRenderer::render_status_bar(
            frame,
            area,
            &self.dynamic_shortcuts_manager,
            theme,
            &self.typography
        );
    }

    /// Render dynamic shortcut hints as popup
    pub fn render_shortcut_hints_popup(&self, frame: &mut Frame, area: Rect) {
        let theme = self.theme_manager.current_theme();
        DynamicShortcutsRenderer::render_popup(
            frame,
            area,
            &self.dynamic_shortcuts_manager,
            theme,
            &self.typography
        );
    }

    /// Render dynamic shortcut hints as full overlay
    pub fn render_shortcut_hints_overlay(&self, frame: &mut Frame, area: Rect) {
        let theme = self.theme_manager.current_theme();
        DynamicShortcutsRenderer::render_overlay(
            frame,
            area,
            &self.dynamic_shortcuts_manager,
            theme,
            &self.typography
        );
    }

    /// Render inline shortcut hint for a specific shortcut
    pub fn render_inline_shortcut_hint(&self, frame: &mut Frame, area: Rect, shortcut: &KeyboardShortcut) {
        let theme = self.theme_manager.current_theme();
        DynamicShortcutsRenderer::render_inline_hint(
            frame,
            area,
            shortcut,
            theme,
            &self.typography
        );
    }

    /// Get current contextual shortcuts
    pub fn get_contextual_shortcuts(&self) -> Vec<KeyboardShortcut> {
        self.dynamic_shortcuts_manager.get_contextual_shortcuts()
    }

    /// Get top shortcuts for compact display
    pub fn get_top_shortcuts(&self, limit: usize) -> Vec<KeyboardShortcut> {
        self.dynamic_shortcuts_manager.get_top_shortcuts(limit)
    }
}


impl Default for UI {
    fn default() -> Self {
        Self::new()
    }
}
