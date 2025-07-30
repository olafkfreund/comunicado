//! Integrated Email-Calendar Layout System
//! 
//! Provides flexible layouts that can show email and calendar content
//! in various configurations based on user needs and context.

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Layout configuration for integrated email-calendar view
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegratedViewMode {
    /// Primary email interface with calendar sidebar
    EmailPrimary,
    /// Primary calendar interface with email notifications
    CalendarPrimary,
    /// Split view with both email and calendar equally visible
    SplitView,
    /// Context-aware view that adapts based on content
    ContextAware,
    /// Full-screen mode for focused work
    FullScreen(ContentType),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentType {
    Email,
    Calendar,
    Event,
    Invitation,
}

/// Layout areas for the integrated interface
#[derive(Debug, Clone)]
pub struct IntegratedLayout {
    /// Left sidebar for navigation (accounts, folders, calendars)
    pub sidebar: Rect,
    /// Main content area (email list or calendar view)
    pub primary_content: Rect,
    /// Secondary content area (calendar sidebar or email preview)
    pub secondary_content: Option<Rect>,
    /// Details panel for reading emails or viewing events
    pub details_panel: Rect,
    /// Status bar area
    pub status_bar: Rect,
    /// Quick action bar (reply, accept, decline, etc.)
    pub action_bar: Option<Rect>,
}

pub struct IntegratedLayoutManager {
    sidebar_width: u16,
    primary_ratio: u16,
    view_mode: IntegratedViewMode,
    show_action_bar: bool,
}

impl IntegratedLayoutManager {
    pub fn new() -> Self {
        Self {
            sidebar_width: 25,
            primary_ratio: 40,
            view_mode: IntegratedViewMode::EmailPrimary,
            show_action_bar: false,
        }
    }

    /// Calculate layout based on current view mode and screen size
    pub fn calculate_layout(&self, area: Rect) -> IntegratedLayout {
        match self.view_mode {
            IntegratedViewMode::EmailPrimary => self.email_primary_layout(area),
            IntegratedViewMode::CalendarPrimary => self.calendar_primary_layout(area),
            IntegratedViewMode::SplitView => self.split_view_layout(area),
            IntegratedViewMode::ContextAware => self.context_aware_layout(area),
            IntegratedViewMode::FullScreen(content_type) => self.fullscreen_layout(area, content_type),
        }
    }

    /// Email-primary layout: [Sidebar | Email List | Details + Calendar Sidebar]
    fn email_primary_layout(&self, area: Rect) -> IntegratedLayout {
        // Main vertical split: content + status bar + action bar
        let action_bar_height = if self.show_action_bar { 3 } else { 0 };
        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),                             // Main content
                Constraint::Length(3),                          // Status bar
                Constraint::Length(action_bar_height),          // Action bar (conditional)
            ])
            .split(area);

        let main_area = vertical_chunks[0];

        // Horizontal split: [Sidebar | Email Area | Details Area]
        let horizontal_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(self.sidebar_width),         // Fixed sidebar
                Constraint::Percentage(self.primary_ratio),     // Email list
                Constraint::Min(35),                            // Details + calendar
            ])
            .split(main_area);

        // Split details area vertically: [Email Details | Calendar Sidebar]
        let details_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(70),                     // Email content
                Constraint::Min(8),                             // Calendar sidebar
            ])
            .split(horizontal_chunks[2]);

        IntegratedLayout {
            sidebar: horizontal_chunks[0],
            primary_content: horizontal_chunks[1],              // Email list
            secondary_content: Some(details_chunks[1]),         // Calendar sidebar
            details_panel: details_chunks[0],                   // Email details
            status_bar: vertical_chunks[1],
            action_bar: if self.show_action_bar { Some(vertical_chunks[2]) } else { None },
        }
    }

    /// Calendar-primary layout: [Sidebar | Calendar | Email Notifications]
    fn calendar_primary_layout(&self, area: Rect) -> IntegratedLayout {
        let action_bar_height = if self.show_action_bar { 3 } else { 0 };
        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),
                Constraint::Length(3),
                Constraint::Length(action_bar_height),
            ])
            .split(area);

        let main_area = vertical_chunks[0];

        let horizontal_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(self.sidebar_width),         // Sidebar
                Constraint::Percentage(75),                     // Calendar view
                Constraint::Min(25),                            // Email notifications
            ])
            .split(main_area);

        IntegratedLayout {
            sidebar: horizontal_chunks[0],
            primary_content: horizontal_chunks[1],              // Calendar
            secondary_content: Some(horizontal_chunks[2]),      // Email notifications
            details_panel: horizontal_chunks[1],                // Same as primary for calendar
            status_bar: vertical_chunks[1],
            action_bar: if self.show_action_bar { Some(vertical_chunks[2]) } else { None },
        }
    }

    /// Split view layout: [Sidebar | Email | Calendar]
    fn split_view_layout(&self, area: Rect) -> IntegratedLayout {
        let action_bar_height = if self.show_action_bar { 3 } else { 0 };
        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),
                Constraint::Length(3),
                Constraint::Length(action_bar_height),
            ])
            .split(area);

        let main_area = vertical_chunks[0];

        let horizontal_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(self.sidebar_width),         // Sidebar
                Constraint::Percentage(50),                     // Email
                Constraint::Percentage(50),                     // Calendar
            ])
            .split(main_area);

        IntegratedLayout {
            sidebar: horizontal_chunks[0],
            primary_content: horizontal_chunks[1],              // Email
            secondary_content: Some(horizontal_chunks[2]),      // Calendar
            details_panel: horizontal_chunks[1],                // Email details within primary
            status_bar: vertical_chunks[1],
            action_bar: if self.show_action_bar { Some(vertical_chunks[2]) } else { None },
        }
    }

    /// Context-aware layout adapts based on what the user is viewing
    fn context_aware_layout(&self, area: Rect) -> IntegratedLayout {
        // Default to email-primary, but this would be enhanced with context detection
        self.email_primary_layout(area)
    }

    /// Full-screen layout for focused content
    fn fullscreen_layout(&self, area: Rect, _content_type: ContentType) -> IntegratedLayout {
        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),                             // Full content
                Constraint::Length(3),                          // Status bar
            ])
            .split(area);

        IntegratedLayout {
            sidebar: Rect::default(),                           // Hidden
            primary_content: vertical_chunks[0],                // Full screen
            secondary_content: None,                            // Hidden
            details_panel: vertical_chunks[0],                  // Same as primary
            status_bar: vertical_chunks[1],
            action_bar: None,
        }
    }

    /// Switch between different view modes
    pub fn set_view_mode(&mut self, mode: IntegratedViewMode) {
        self.view_mode = mode;
    }

    pub fn get_view_mode(&self) -> IntegratedViewMode {
        self.view_mode
    }

    /// Toggle action bar visibility
    pub fn toggle_action_bar(&mut self) {
        self.show_action_bar = !self.show_action_bar;
    }

    /// Adjust sidebar width
    pub fn set_sidebar_width(&mut self, width: u16) {
        self.sidebar_width = width.clamp(20, 40);
    }

    /// Adjust primary content ratio
    pub fn set_primary_ratio(&mut self, ratio: u16) {
        self.primary_ratio = ratio.clamp(30, 70);
    }
}

impl Default for IntegratedLayoutManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Context detection for smart layout switching
pub struct LayoutContext {
    pub has_calendar_invitation: bool,
    pub has_upcoming_events: bool,
    pub is_reading_email: bool,
    pub is_viewing_calendar: bool,
    pub selected_content_type: ContentType,
}

impl LayoutContext {
    pub fn suggest_layout(&self) -> IntegratedViewMode {
        match (self.has_calendar_invitation, self.is_reading_email, self.is_viewing_calendar) {
            (true, true, _) => IntegratedViewMode::ContextAware, // Show both email and calendar
            (_, _, true) => IntegratedViewMode::CalendarPrimary,  // User actively using calendar
            (_, true, _) => IntegratedViewMode::EmailPrimary,     // User reading email
            _ => IntegratedViewMode::EmailPrimary,                // Default
        }
    }
}