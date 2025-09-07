use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Responsive breakpoint definitions for different terminal sizes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutBreakpoint {
    /// Extra small: 40-59 columns (minimal mobile-like experience)
    ExtraSmall,
    /// Small: 60-79 columns (compact interface)
    Small,
    /// Medium: 80-119 columns (standard interface)
    Medium,
    /// Large: 120-159 columns (comfortable interface)
    Large,
    /// Extra large: 160+ columns (spacious interface)
    ExtraLarge,
}

/// Layout configuration that adapts to different screen sizes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    /// Full three-column layout: sidebar | messages | content
    ThreeColumn,
    /// Two-column layout: messages | content (sidebar hidden)
    TwoColumn,
    /// Single column: focus mode (only one pane visible)
    SingleColumn,
    /// Stacked layout: vertical arrangement for narrow screens
    Stacked,
}

/// Configuration for responsive layout behavior
#[derive(Debug, Clone)]
pub struct ResponsiveConfig {
    /// Breakpoints for different layout modes
    pub breakpoints: ResponsiveBreakpoints,
    /// Minimum widths for different components
    pub min_widths: ComponentMinWidths,
    /// Adaptive panel behavior settings
    pub adaptive_behavior: AdaptiveBehavior,
}

#[derive(Debug, Clone)]
pub struct ResponsiveBreakpoints {
    pub extra_small_max: u16,
    pub small_max: u16,
    pub medium_max: u16,
    pub large_max: u16,
}

#[derive(Debug, Clone)]
pub struct ComponentMinWidths {
    pub sidebar_min: u16,
    pub message_list_min: u16,
    pub content_panel_min: u16,
    pub status_bar_min: u16,
}

#[derive(Debug, Clone)]
pub struct AdaptiveBehavior {
    /// Hide sidebar on narrow screens
    pub auto_hide_sidebar: bool,
    /// Stack panels vertically on very narrow screens
    pub auto_stack_panels: bool,
    /// Adjust font/spacing based on screen density
    pub scale_content: bool,
    /// Show simplified UI elements on small screens
    pub simplify_interface: bool,
}

pub struct AppLayout {
    folder_width: u16,
    message_width_ratio: u16,
    responsive_config: ResponsiveConfig,
    current_breakpoint: LayoutBreakpoint,
    current_layout_mode: LayoutMode,
}

impl AppLayout {
    pub fn new() -> Self {
        Self {
            folder_width: 25,
            message_width_ratio: 40,
            responsive_config: ResponsiveConfig::default(),
            current_breakpoint: LayoutBreakpoint::Medium,
            current_layout_mode: LayoutMode::ThreeColumn,
        }
    }

    /// Calculate responsive layout based on terminal size
    pub fn calculate_layout(&mut self, area: Rect) -> Vec<Rect> {
        // Update responsive state based on current area
        self.update_responsive_state(area);

        match self.current_layout_mode {
            LayoutMode::ThreeColumn => self.three_column_layout(area),
            LayoutMode::TwoColumn => self.two_column_layout(area),
            LayoutMode::SingleColumn => self.single_column_layout(area),
            LayoutMode::Stacked => self.stacked_layout(area),
        }
    }

    /// Update responsive state based on current terminal size
    fn update_responsive_state(&mut self, area: Rect) {
        let width = area.width;

        // Determine current breakpoint
        self.current_breakpoint = match width {
            w if w <= self.responsive_config.breakpoints.extra_small_max => {
                LayoutBreakpoint::ExtraSmall
            }
            w if w <= self.responsive_config.breakpoints.small_max => LayoutBreakpoint::Small,
            w if w <= self.responsive_config.breakpoints.medium_max => LayoutBreakpoint::Medium,
            w if w <= self.responsive_config.breakpoints.large_max => LayoutBreakpoint::Large,
            _ => LayoutBreakpoint::ExtraLarge,
        };

        // Determine appropriate layout mode
        self.current_layout_mode = match self.current_breakpoint {
            LayoutBreakpoint::ExtraSmall => {
                if self.responsive_config.adaptive_behavior.auto_stack_panels {
                    LayoutMode::Stacked
                } else {
                    LayoutMode::SingleColumn
                }
            }
            LayoutBreakpoint::Small => {
                if self.responsive_config.adaptive_behavior.auto_hide_sidebar {
                    LayoutMode::TwoColumn
                } else {
                    LayoutMode::ThreeColumn
                }
            }
            LayoutBreakpoint::Medium => LayoutMode::ThreeColumn,
            LayoutBreakpoint::Large | LayoutBreakpoint::ExtraLarge => LayoutMode::ThreeColumn,
        };

        // Adjust component sizes based on breakpoint
        self.adjust_component_sizes();
    }

    /// Adjust component sizes based on current breakpoint
    fn adjust_component_sizes(&mut self) {
        match self.current_breakpoint {
            LayoutBreakpoint::ExtraSmall => {
                self.folder_width = self.responsive_config.min_widths.sidebar_min;
                self.message_width_ratio = 60; // Give more space to messages on small screens
            }
            LayoutBreakpoint::Small => {
                self.folder_width = 18;
                self.message_width_ratio = 45;
            }
            LayoutBreakpoint::Medium => {
                self.folder_width = 25;
                self.message_width_ratio = 40;
            }
            LayoutBreakpoint::Large => {
                self.folder_width = 30;
                self.message_width_ratio = 35;
            }
            LayoutBreakpoint::ExtraLarge => {
                self.folder_width = 35;
                self.message_width_ratio = 30;
            }
        }
    }

    /// Three-column layout: [Sidebar | Messages | Content]
    fn three_column_layout(&self, area: Rect) -> Vec<Rect> {
        // First, split vertically to reserve space for status bar
        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),    // Main content area
                Constraint::Length(3), // Status bar (fixed height)
            ])
            .split(area);

        let main_area = vertical_chunks[0];

        // Split the main area horizontally: [Left Panel | Messages | Content]
        let horizontal_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(self.folder_width), // Adaptive sidebar width
                Constraint::Percentage(self.message_width_ratio), // Adaptive message width
                Constraint::Min(self.responsive_config.min_widths.content_panel_min), // Remaining space
            ])
            .split(main_area);

        // Split the left panel vertically: [Account Switcher | Folders]
        let left_panel_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(if self.current_breakpoint == LayoutBreakpoint::ExtraSmall {
                    3
                } else {
                    4
                }), // Compact account switcher
                Constraint::Min(5), // Remaining space for folders
            ])
            .split(horizontal_chunks[0]);

        // Return all chunks: [account_switcher, folder, message, content, status_bar]
        vec![
            left_panel_chunks[0], // Account switcher
            left_panel_chunks[1], // Folder tree
            horizontal_chunks[1], // Message list
            horizontal_chunks[2], // Content preview
            vertical_chunks[1],   // Status bar
        ]
    }

    /// Two-column layout: [Messages | Content] (sidebar hidden)
    fn two_column_layout(&self, area: Rect) -> Vec<Rect> {
        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),
                Constraint::Length(3), // Status bar
            ])
            .split(area);

        let main_area = vertical_chunks[0];
        let horizontal_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50), // Messages take 50%
                Constraint::Percentage(50), // Content takes 50%
            ])
            .split(main_area);

        // Return layout with hidden sidebar (empty rects for account switcher and folder tree)
        vec![
            Rect::new(0, 0, 0, 0), // Hidden account switcher
            Rect::new(0, 0, 0, 0), // Hidden folder tree
            horizontal_chunks[0],  // Message list
            horizontal_chunks[1],  // Content preview
            vertical_chunks[1],    // Status bar
        ]
    }

    /// Single column layout: Focus on one pane at a time
    fn single_column_layout(&self, area: Rect) -> Vec<Rect> {
        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),
                Constraint::Length(3), // Status bar
            ])
            .split(area);

        let main_area = vertical_chunks[0];

        // In single column, we focus on the message list primarily
        // Other panes are hidden or minimized
        vec![
            Rect::new(0, 0, 0, 0), // Hidden account switcher
            Rect::new(0, 0, 0, 0), // Hidden folder tree
            main_area,             // Full-width message list
            Rect::new(0, 0, 0, 0), // Hidden content preview
            vertical_chunks[1],    // Status bar
        ]
    }

    /// Stacked layout: Vertical arrangement for very narrow screens
    fn stacked_layout(&self, area: Rect) -> Vec<Rect> {
        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Compact account switcher
                Constraint::Length(8), // Compact folder tree
                Constraint::Min(10),   // Message list (main focus)
                Constraint::Length(3), // Status bar
            ])
            .split(area);

        // Return vertically stacked layout
        vec![
            vertical_chunks[0],    // Account switcher (top)
            vertical_chunks[1],    // Folder tree
            vertical_chunks[2],    // Message list (main area)
            Rect::new(0, 0, 0, 0), // Hidden content preview
            vertical_chunks[3],    // Status bar (bottom)
        ]
    }

    pub fn set_folder_width(&mut self, width: u16) {
        self.folder_width = width;
    }

    pub fn set_message_width_ratio(&mut self, ratio: u16) {
        self.message_width_ratio = ratio;
    }

    /// Get current responsive breakpoint
    pub fn current_breakpoint(&self) -> LayoutBreakpoint {
        self.current_breakpoint
    }

    /// Get current layout mode
    pub fn current_layout_mode(&self) -> LayoutMode {
        self.current_layout_mode
    }

    /// Check if sidebar should be visible in current layout
    pub fn is_sidebar_visible(&self) -> bool {
        matches!(self.current_layout_mode, LayoutMode::ThreeColumn)
    }

    /// Check if content preview should be visible
    pub fn is_content_preview_visible(&self) -> bool {
        !matches!(
            self.current_layout_mode,
            LayoutMode::SingleColumn | LayoutMode::Stacked
        )
    }

    /// Get adaptive spacing based on current breakpoint
    pub fn get_adaptive_spacing(&self) -> u16 {
        match self.current_breakpoint {
            LayoutBreakpoint::ExtraSmall => 0,
            LayoutBreakpoint::Small => 1,
            LayoutBreakpoint::Medium => 1,
            LayoutBreakpoint::Large => 2,
            LayoutBreakpoint::ExtraLarge => 2,
        }
    }

    /// Configure responsive behavior
    pub fn configure_responsive_behavior(&mut self, behavior: AdaptiveBehavior) {
        self.responsive_config.adaptive_behavior = behavior;
    }

    /// Update responsive configuration
    pub fn update_responsive_config(&mut self, config: ResponsiveConfig) {
        self.responsive_config = config;
    }

    /// Get layout summary for debugging
    pub fn get_layout_summary(&self) -> String {
        format!(
            "Layout: {:?} | Breakpoint: {:?} | Sidebar: {}px | Message: {}%",
            self.current_layout_mode,
            self.current_breakpoint,
            self.folder_width,
            self.message_width_ratio
        )
    }
}

impl Default for AppLayout {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ResponsiveConfig {
    fn default() -> Self {
        Self {
            breakpoints: ResponsiveBreakpoints::default(),
            min_widths: ComponentMinWidths::default(),
            adaptive_behavior: AdaptiveBehavior::default(),
        }
    }
}

impl Default for ResponsiveBreakpoints {
    fn default() -> Self {
        Self {
            extra_small_max: 59,
            small_max: 79,
            medium_max: 119,
            large_max: 159,
        }
    }
}

impl Default for ComponentMinWidths {
    fn default() -> Self {
        Self {
            sidebar_min: 15,
            message_list_min: 25,
            content_panel_min: 30,
            status_bar_min: 40,
        }
    }
}

impl Default for AdaptiveBehavior {
    fn default() -> Self {
        Self {
            auto_hide_sidebar: true,
            auto_stack_panels: true,
            scale_content: true,
            simplify_interface: true,
        }
    }
}
