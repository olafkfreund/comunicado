use crate::theme::Theme;
use crate::ui::typography::{TypographySystem, TypographyLevel, VisualHierarchy}; // InformationDensity as TypographyInformationDensity
use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::collections::HashMap;

/// Trait for status bar segments that can be rendered
pub trait StatusSegment {
    /// Get the content to display in this segment
    fn content(&self) -> String;

    /// Get the minimum width required for this segment
    fn min_width(&self) -> u16;

    /// Get the priority of this segment (higher = more important)
    fn priority(&self) -> u8;

    /// Whether this segment should be visible
    fn is_visible(&self) -> bool {
        true
    }

    /// Get custom styling for this segment (optional)
    fn custom_style(&self, _theme: &Theme) -> Option<Style> {
        None
    }
}

/// Email status segment showing unread/total counts
#[derive(Debug, Clone)]
pub struct EmailStatusSegment {
    pub unread_count: usize,
    pub total_count: usize,
    pub sync_status: SyncStatus,
}

/// Calendar status segment showing upcoming events
#[derive(Debug, Clone)]
pub struct CalendarStatusSegment {
    pub next_event: Option<String>,
    pub events_today: usize,
    pub next_event_time: Option<chrono::DateTime<chrono::Local>>,
    pub urgent_events: usize,
}

/// System information segment
#[derive(Debug, Clone)]
pub struct SystemInfoSegment {
    pub current_time: String,
    pub active_account: String,
}

/// Network/sync status
#[derive(Debug, Clone, PartialEq)]
pub enum SyncStatus {
    Online,
    Syncing,
    SyncingWithProgress(u32, u32), // (processed, total)
    Offline,
    Error,
}

/// Navigation hints segment
#[derive(Debug, Clone)]
pub struct NavigationHintsSegment {
    pub current_pane: String,
    pub available_shortcuts: Vec<(String, String)>, // (key, description)
}

/// Search status segment showing current search query and results
#[derive(Debug, Clone)]
pub struct SearchStatusSegment {
    pub query: String,
    pub results_count: usize,
    pub is_active: bool,
}

/// Mode indicator segment showing current application mode with visual emphasis
#[derive(Debug, Clone)]
pub struct ModeIndicatorSegment {
    pub current_mode: crate::ui::UIMode,
    pub sub_mode: Option<String>, // For additional context like edit/view
    pub mode_stack: Vec<crate::ui::UIMode>, // History of modes for breadcrumb
}

/// Interactive mode hints segment showing mode-specific shortcuts
#[derive(Debug, Clone)]
pub struct ModeHintsSegment {
    pub current_mode: crate::ui::UIMode,
    pub key_hints: Vec<(String, String)>, // (key combination, description)
    pub is_vim_mode: bool,
}

impl StatusSegment for EmailStatusSegment {
    fn content(&self) -> String {
        let sync_indicator = match &self.sync_status {
            SyncStatus::Online => "●".to_string(),
            SyncStatus::Syncing => "⟳".to_string(),
            SyncStatus::SyncingWithProgress(processed, total) => {
                if *total > 0 {
                    let percent = (*processed * 100) / *total;
                    format!("⟳{}%", percent)
                } else {
                    "⟳".to_string()
                }
            }
            SyncStatus::Offline => "○".to_string(),
            SyncStatus::Error => "⚠".to_string(),
        };

        if self.unread_count > 0 {
            format!(
                "Mail: {} unread {} {}",
                self.unread_count, sync_indicator, self.total_count
            )
        } else {
            format!("Mail: {} {}", sync_indicator, self.total_count)
        }
    }

    fn min_width(&self) -> u16 {
        20
    }

    fn priority(&self) -> u8 {
        90 // High priority
    }

    fn custom_style(&self, theme: &Theme) -> Option<Style> {
        if self.unread_count > 0 {
            Some(
                Style::default()
                    .fg(theme.colors.palette.warning)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            None
        }
    }
}

impl StatusSegment for CalendarStatusSegment {
    fn content(&self) -> String {
        match &self.next_event {
            Some(event) => {
                let urgency_indicator = if self.urgent_events > 0 { "🔴" } else { "" };
                let time_info = match &self.next_event_time {
                    Some(time) => {
                        let now = chrono::Local::now();
                        let duration = time.signed_duration_since(now);
                        if duration.num_minutes() < 60 {
                            format!(" in {}m", duration.num_minutes())
                        } else if duration.num_hours() < 24 {
                            format!(" in {}h", duration.num_hours())
                        } else {
                            format!(" {}", time.format("%m/%d"))
                        }
                    }
                    None => String::new(),
                };
                format!("Cal{}: {}{} ({} today)", urgency_indicator, event, time_info, self.events_today)
            }
            None => {
                if self.events_today > 0 {
                    let urgency_indicator = if self.urgent_events > 0 { "🔴" } else { "" };
                    format!("Cal{}: {} events today", urgency_indicator, self.events_today)
                } else {
                    "Cal: No events".to_string()
                }
            }
        }
    }

    fn min_width(&self) -> u16 {
        25
    }

    fn priority(&self) -> u8 {
        70
    }
}

impl StatusSegment for SystemInfoSegment {
    fn content(&self) -> String {
        format!("{} | {}", self.active_account, self.current_time)
    }

    fn min_width(&self) -> u16 {
        30
    }

    fn priority(&self) -> u8 {
        50
    }
}

impl StatusSegment for NavigationHintsSegment {
    fn content(&self) -> String {
        let shortcuts: Vec<String> = self
            .available_shortcuts
            .iter()
            .take(3) // Show max 3 shortcuts to avoid crowding
            .map(|(key, desc)| format!("{}: {}", key, desc))
            .collect();

        format!("{} | {}", self.current_pane, shortcuts.join(" | "))
    }

    fn min_width(&self) -> u16 {
        40
    }

    fn priority(&self) -> u8 {
        30
    }

    fn custom_style(&self, theme: &Theme) -> Option<Style> {
        Some(Style::default().fg(theme.colors.palette.text_muted))
    }
}

impl StatusSegment for SearchStatusSegment {
    fn content(&self) -> String {
        if self.is_active {
            if self.query.is_empty() {
                "Search: (type to search)".to_string()
            } else {
                format!("Search: {} ({} results)", self.query, self.results_count)
            }
        } else {
            String::new()
        }
    }

    fn min_width(&self) -> u16 {
        25
    }

    fn priority(&self) -> u8 {
        95 // Very high priority when active
    }

    fn is_visible(&self) -> bool {
        self.is_active
    }

    fn custom_style(&self, theme: &Theme) -> Option<Style> {
        if self.is_active {
            Some(
                Style::default()
                    .fg(theme.colors.palette.text_primary)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            None
        }
    }
}

impl StatusSegment for ModeIndicatorSegment {
    fn content(&self) -> String {
        let mode_icon = self.get_mode_icon();
        let mode_name = self.get_mode_display_name();
        
        let mut content = format!("{} {}", mode_icon, mode_name);
        
        if let Some(sub_mode) = &self.sub_mode {
            content.push_str(&format!(" ({})", sub_mode));
        }
        
        // Add breadcrumb if there's a mode stack
        if self.mode_stack.len() > 1 {
            let breadcrumb: Vec<String> = self.mode_stack
                .iter()
                .rev()
                .take(2) // Show last 2 modes in breadcrumb
                .skip(1) // Skip current mode (already shown)
                .map(|m| self.get_mode_name(m))
                .collect();
            
            if !breadcrumb.is_empty() {
                content.push_str(&format!(" ← {}", breadcrumb.join(" ← ")));
            }
        }
        
        content
    }

    fn min_width(&self) -> u16 {
        15
    }

    fn priority(&self) -> u8 {
        100 // Highest priority - always visible
    }

    fn custom_style(&self, theme: &Theme) -> Option<Style> {
        let base_color = match self.current_mode {
            crate::ui::UIMode::Normal => theme.colors.palette.text_primary,
            crate::ui::UIMode::Compose => theme.colors.palette.warning,
            crate::ui::UIMode::Search => theme.colors.palette.info,
            crate::ui::UIMode::Settings => theme.colors.palette.accent,
            crate::ui::UIMode::Calendar => theme.colors.palette.success,
            crate::ui::UIMode::EmailViewer => theme.colors.palette.info,
            crate::ui::UIMode::EventCreate | crate::ui::UIMode::EventEdit => theme.colors.palette.warning,
            _ => theme.colors.palette.text_secondary,
        };
        
        Some(
            Style::default()
                .fg(base_color)
                .add_modifier(Modifier::BOLD)
        )
    }
}

impl ModeIndicatorSegment {
    fn get_mode_icon(&self) -> &'static str {
        match self.current_mode {
            crate::ui::UIMode::Normal => "📧",
            crate::ui::UIMode::Compose => "✏️",
            crate::ui::UIMode::DraftList => "📝",
            crate::ui::UIMode::Calendar => "📅",
            crate::ui::UIMode::EventCreate => "➕",
            crate::ui::UIMode::EventEdit => "📝",
            crate::ui::UIMode::EventView => "👁️",
            crate::ui::UIMode::EmailViewer => "📖",
            crate::ui::UIMode::InvitationViewer => "📨",
            crate::ui::UIMode::Search => "🔍",
            crate::ui::UIMode::KeyboardShortcuts => "⌨️",
            crate::ui::UIMode::Settings => "⚙️",
            crate::ui::UIMode::ContactsPopup => "👤",
            crate::ui::UIMode::Contacts => "📞",
            crate::ui::UIMode::ContextAware => "🔗",
        }
    }
    
    fn get_mode_display_name(&self) -> &'static str {
        match self.current_mode {
            crate::ui::UIMode::Normal => "Mail",
            crate::ui::UIMode::Compose => "Compose",
            crate::ui::UIMode::DraftList => "Drafts",
            crate::ui::UIMode::Calendar => "Calendar",
            crate::ui::UIMode::EventCreate => "New Event",
            crate::ui::UIMode::EventEdit => "Edit Event",
            crate::ui::UIMode::EventView => "Event",
            crate::ui::UIMode::EmailViewer => "Reading",
            crate::ui::UIMode::InvitationViewer => "Invitation",
            crate::ui::UIMode::Search => "Search",
            crate::ui::UIMode::KeyboardShortcuts => "Shortcuts",
            crate::ui::UIMode::Settings => "Settings",
            crate::ui::UIMode::ContactsPopup => "Contacts",
            crate::ui::UIMode::Contacts => "Address Book",
            crate::ui::UIMode::ContextAware => "Context",
        }
    }
    
    fn get_mode_name(&self, mode: &crate::ui::UIMode) -> String {
        match mode {
            crate::ui::UIMode::Normal => "Mail".to_string(),
            crate::ui::UIMode::Compose => "Compose".to_string(),
            crate::ui::UIMode::DraftList => "Drafts".to_string(),
            crate::ui::UIMode::Calendar => "Cal".to_string(),
            crate::ui::UIMode::EventCreate => "New".to_string(),
            crate::ui::UIMode::EventEdit => "Edit".to_string(),
            crate::ui::UIMode::EventView => "Event".to_string(),
            crate::ui::UIMode::EmailViewer => "Read".to_string(),
            crate::ui::UIMode::InvitationViewer => "Invite".to_string(),
            crate::ui::UIMode::Search => "Search".to_string(),
            crate::ui::UIMode::KeyboardShortcuts => "Keys".to_string(),
            crate::ui::UIMode::Settings => "Config".to_string(),
            crate::ui::UIMode::ContactsPopup => "Contact".to_string(),
            crate::ui::UIMode::Contacts => "Contacts".to_string(),
            crate::ui::UIMode::ContextAware => "Context".to_string(),
        }
    }
}

impl StatusSegment for ModeHintsSegment {
    fn content(&self) -> String {
        let mode_prefix = if self.is_vim_mode { "VIM" } else { "STD" };
        
        if self.key_hints.is_empty() {
            format!("[{}]", mode_prefix)
        } else {
            let hints: Vec<String> = self.key_hints
                .iter()
                .take(3) // Limit to 3 hints to avoid crowding
                .map(|(key, desc)| format!("{}: {}", key, desc))
                .collect();
            
            format!("[{}] {}", mode_prefix, hints.join(" | "))
        }
    }

    fn min_width(&self) -> u16 {
        20
    }

    fn priority(&self) -> u8 {
        40 // Medium priority
    }

    fn custom_style(&self, theme: &Theme) -> Option<Style> {
        Some(
            Style::default()
                .fg(theme.colors.palette.text_muted)
                .add_modifier(if self.is_vim_mode { Modifier::ITALIC } else { Modifier::empty() })
        )
    }
}

/// Professional status bar with powerline-style segments
pub struct StatusBar {
    segments: HashMap<String, Box<dyn StatusSegment>>,
    position: StatusBarPosition,
    segment_order: Vec<String>,
    separator_style: SeparatorStyle,
    /// Context-aware segment priority adjustment
    context_priorities: HashMap<String, u8>,
    /// Responsive layout thresholds
    layout_thresholds: ResponsiveThresholds,
    /// Information density preference
    information_density: InformationDensity,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StatusBarPosition {
    Top,
    Bottom,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SeparatorStyle {
    Powerline, // ⮰ ⮱ ⮲ ⮳
    Simple,    // |
    Minimal,   // space
    Adaptive,  // Changes based on context and space
}

/// Responsive layout thresholds for different terminal sizes
#[derive(Debug, Clone)]
pub struct ResponsiveThresholds {
    /// Minimum width for full status bar (all segments visible)
    pub full_width: u16,
    /// Minimum width for compact status bar (essential segments only)
    pub compact_width: u16,
    /// Minimum width for minimal status bar (critical info only)
    pub minimal_width: u16,
}

/// Information density levels for status bar content
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InformationDensity {
    Minimal,   // Only critical information
    Compact,   // Important information with abbreviated labels
    Standard,  // Normal information display
    Detailed,  // Full information with descriptions
}

/// Status bar context for intelligent priority adjustment
#[derive(Debug, Clone, PartialEq)]
pub enum StatusBarContext {
    EmailFocus,     // User is working with emails
    CalendarFocus,  // User is working with calendar
    ComposeFocus,   // User is composing
    SearchActive,   // User is searching
    SettingsActive, // User is in settings
    SystemAlert,    // System requires attention
}

impl StatusBar {
    pub fn new(position: StatusBarPosition) -> Self {
        Self {
            segments: HashMap::new(),
            position,
            segment_order: Vec::new(),
            separator_style: SeparatorStyle::Adaptive,
            context_priorities: HashMap::new(),
            layout_thresholds: ResponsiveThresholds {
                full_width: 120,
                compact_width: 80,
                minimal_width: 40,
            },
            information_density: InformationDensity::Standard,
        }
    }

    /// Add a status segment
    pub fn add_segment<T: StatusSegment + 'static>(&mut self, name: String, segment: T) {
        self.segments.insert(name.clone(), Box::new(segment));
        if !self.segment_order.contains(&name) {
            // Insert in priority order
            let priority = self.segments[&name].priority();
            let insert_pos = self
                .segment_order
                .iter()
                .position(|existing_name| self.segments[existing_name].priority() < priority)
                .unwrap_or(self.segment_order.len());
            self.segment_order.insert(insert_pos, name);
        }
    }

    /// Remove a status segment
    pub fn remove_segment(&mut self, name: &str) {
        self.segments.remove(name);
        self.segment_order.retain(|n| n != name);
    }

    /// Update segment order
    pub fn set_segment_order(&mut self, order: Vec<String>) {
        // Only include segments that actually exist
        self.segment_order = order
            .into_iter()
            .filter(|name| self.segments.contains_key(name))
            .collect();
    }

    /// Set separator style
    pub fn set_separator_style(&mut self, style: SeparatorStyle) {
        self.separator_style = style;
    }

    /// Set information density level
    pub fn set_information_density(&mut self, density: InformationDensity) {
        self.information_density = density;
    }

    /// Update context priorities for intelligent segment management
    pub fn update_context(&mut self, context: StatusBarContext) {
        // Clear previous context priorities
        self.context_priorities.clear();
        
        match context {
            StatusBarContext::EmailFocus => {
                self.context_priorities.insert("email".to_string(), 100);
                self.context_priorities.insert("mode".to_string(), 95);
                self.context_priorities.insert("navigation".to_string(), 85);
                self.context_priorities.insert("calendar".to_string(), 70);
                self.context_priorities.insert("system".to_string(), 60);
            }
            StatusBarContext::CalendarFocus => {
                self.context_priorities.insert("calendar".to_string(), 100);
                self.context_priorities.insert("mode".to_string(), 95);
                self.context_priorities.insert("email".to_string(), 80);
                self.context_priorities.insert("system".to_string(), 70);
                self.context_priorities.insert("navigation".to_string(), 60);
            }
            StatusBarContext::ComposeFocus => {
                self.context_priorities.insert("mode".to_string(), 100);
                self.context_priorities.insert("mode_hints".to_string(), 95);
                self.context_priorities.insert("system".to_string(), 80);
                self.context_priorities.insert("email".to_string(), 60);
            }
            StatusBarContext::SearchActive => {
                self.context_priorities.insert("search".to_string(), 100);
                self.context_priorities.insert("mode".to_string(), 95);
                self.context_priorities.insert("email".to_string(), 85);
                self.context_priorities.insert("navigation".to_string(), 70);
            }
            StatusBarContext::SettingsActive => {
                self.context_priorities.insert("mode".to_string(), 100);
                self.context_priorities.insert("navigation".to_string(), 90);
                self.context_priorities.insert("system".to_string(), 80);
            }
            StatusBarContext::SystemAlert => {
                self.context_priorities.insert("system".to_string(), 100);
                self.context_priorities.insert("mode".to_string(), 95);
                // Suppress less critical segments during alerts
                self.context_priorities.insert("navigation".to_string(), 40);
            }
        }
    }

    /// Render the status bar with intelligent layout adaptation
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if area.height == 0 {
            return;
        }

        // Determine layout mode based on available width
        let layout_mode = self.determine_layout_mode(area.width);
        
        // Get priority-sorted visible segments for the current layout
        let visible_segments = self.get_prioritized_segments(layout_mode);

        if visible_segments.is_empty() {
            return;
        }

        // Calculate available width for segments
        let available_width = area.width.saturating_sub(2); // Account for borders
        let separator_style = self.get_adaptive_separator_style(layout_mode);
        let separator_width = self.get_separator_width_for_style(&separator_style);
        let total_separator_width =
            separator_width * (visible_segments.len().saturating_sub(1)) as u16;
        let content_width = available_width.saturating_sub(total_separator_width);

        // Create segments with intelligent sizing
        let segments_content = self.create_intelligent_segments_content(
            &visible_segments, 
            content_width, 
            theme, 
            layout_mode
        );

        // Create the status bar block with context-appropriate styling
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.get_context_border_style(theme));

        // Render the paragraph with segments
        let paragraph = Paragraph::new(segments_content)
            .block(block)
            .alignment(Alignment::Left)
            .style(theme.get_component_style("status_bar", false));

        frame.render_widget(paragraph, area);
    }

    fn get_separator_width(&self) -> u16 {
        match self.separator_style {
            SeparatorStyle::Powerline => 3, // " ⮰ "
            SeparatorStyle::Simple => 3,    // " | "
            SeparatorStyle::Minimal => 2,   // "  "
            SeparatorStyle::Adaptive => 2,  // Determined at render time
        }
    }
    
    fn get_separator_width_for_style(&self, style: &SeparatorStyle) -> u16 {
        match style {
            SeparatorStyle::Powerline => 3,
            SeparatorStyle::Simple => 3,
            SeparatorStyle::Minimal => 2,
            SeparatorStyle::Adaptive => 2,
        }
    }

    fn get_separator(&self, theme: &Theme) -> Span {
        let separator_text = match self.separator_style {
            SeparatorStyle::Powerline => " ⮰ ",
            SeparatorStyle::Simple => " | ",
            SeparatorStyle::Minimal => "  ",
            SeparatorStyle::Adaptive => " · ", // Adaptive uses middle dot
        };

        Span::styled(
            separator_text,
            Style::default().fg(theme.colors.status_bar.section_separator),
        )
    }
    
    fn get_separator_for_style(&self, style: &SeparatorStyle, theme: &Theme) -> Span {
        let separator_text = match style {
            SeparatorStyle::Powerline => " ⮰ ",
            SeparatorStyle::Simple => " | ",
            SeparatorStyle::Minimal => "  ",
            SeparatorStyle::Adaptive => " · ",
        };

        Span::styled(
            separator_text,
            Style::default().fg(theme.colors.status_bar.section_separator),
        )
    }

    fn create_segments_content(
        &self,
        visible_segments: &[(&String, &Box<dyn StatusSegment>)],
        available_width: u16,
        theme: &Theme,
    ) -> Line {
        let mut spans = Vec::new();
        let mut remaining_width = available_width;

        for (i, (_name, segment)) in visible_segments.iter().enumerate() {
            // Add separator between segments
            if i > 0 {
                spans.push(self.get_separator(theme));
                remaining_width = remaining_width.saturating_sub(self.get_separator_width());
            }

            // Get segment content
            let content = segment.content();
            let segment_width = (content.len() as u16).min(remaining_width);

            // Truncate content if necessary
            let display_content = if content.len() as u16 > segment_width {
                if segment_width > 3 {
                    format!("{}...", &content[..((segment_width - 3) as usize)])
                } else {
                    "...".to_string()
                }
            } else {
                content
            };

            // Apply custom styling or default
            let style = segment
                .custom_style(theme)
                .unwrap_or_else(|| theme.get_component_style("status_bar", false));

            spans.push(Span::styled(display_content, style));
            remaining_width = remaining_width.saturating_sub(segment_width);

            if remaining_width == 0 {
                break;
            }
        }

        Line::from(spans)
    }

    /// Enhanced render method using typography system for better visual hierarchy
    pub fn render_with_typography(
        &self, 
        frame: &mut Frame, 
        area: Rect, 
        theme: &Theme, 
        typography: &TypographySystem
    ) {
        if area.height == 0 {
            return;
        }

        // Filter visible segments and sort by order
        let visible_segments: Vec<_> = self
            .segment_order
            .iter()
            .filter_map(|name| {
                self.segments.get(name).and_then(|segment| {
                    if segment.is_visible() {
                        Some((name, segment))
                    } else {
                        None
                    }
                })
            })
            .collect();

        if visible_segments.is_empty() {
            return;
        }

        // Create enhanced segments with typography
        let segments_content = self.create_enhanced_segments_content(
            &visible_segments, 
            area.width.saturating_sub(2), 
            theme, 
            typography
        );

        // Create the status bar block with better spacing
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme.get_component_style("status_bar", false));

        // Use typography-aware paragraph rendering
        let paragraph = Paragraph::new(segments_content)
            .block(block)
            .alignment(Alignment::Left)
            .style(typography.get_typography_style(TypographyLevel::Caption, theme));

        frame.render_widget(paragraph, area);
    }

    /// Create enhanced segments content with typography and visual hierarchy
    fn create_enhanced_segments_content(
        &self,
        visible_segments: &[(&String, &Box<dyn StatusSegment>)],
        available_width: u16,
        theme: &Theme,
        typography: &TypographySystem,
    ) -> Line {
        let mut spans = Vec::new();
        let spacing = typography.spacing();
        let separator_width = spacing.sm;
        let total_separator_width = separator_width * (visible_segments.len().saturating_sub(1) as u16);
        let content_width = available_width.saturating_sub(total_separator_width);
        let mut remaining_width = content_width;

        for (i, (name, segment)) in visible_segments.iter().enumerate() {
            if remaining_width == 0 {
                break;
            }

            // Add separator between segments with proper spacing
            if i > 0 {
                spans.push(typography.create_span(
                    " ".repeat(spacing.xs as usize),
                    TypographyLevel::Metadata,
                    theme,
                ));

                // Add visual separator based on style
                let separator = match self.separator_style {
                    SeparatorStyle::Powerline => "⮰",
                    SeparatorStyle::Simple => "|",
                    SeparatorStyle::Minimal => "·",
                    SeparatorStyle::Adaptive => "·",
                };

                spans.push(typography.create_span(
                    separator.to_string(),
                    TypographyLevel::Metadata,
                    theme,
                ));

                spans.push(typography.create_span(
                    " ".repeat(spacing.xs as usize),
                    TypographyLevel::Metadata,
                    theme,
                ));
            }

            // Determine typography level based on segment type and priority
            let typography_level = match name.as_str() {
                "email" => TypographyLevel::Body,
                "calendar" => TypographyLevel::Body,
                "system" => TypographyLevel::Caption,
                "search" => TypographyLevel::Body,
                "navigation" => TypographyLevel::Metadata,
                _ => TypographyLevel::Caption,
            };

            // Get segment content and apply enhanced styling
            let content = segment.content();
            let segment_width = content.len() as u16;

            if segment_width <= remaining_width {
                // Check for special content formatting
                if name.as_str() == "email" && content.contains("unread") {
                    // Highlight unread count
                    let parts: Vec<&str> = content.split_whitespace().collect();
                    for (j, part) in parts.iter().enumerate() {
                        if j > 0 {
                            spans.push(typography.create_span(" ".to_string(), typography_level, theme));
                        }
                        
                        if part.chars().all(|c| c.is_ascii_digit()) {
                            // This is likely the unread count - emphasize it
                            spans.push(typography.create_emphasis(part, theme));
                        } else {
                            spans.push(typography.create_span(part.to_string(), typography_level, theme));
                        }
                    }
                } else if name.as_str() == "calendar" && content.contains("event") {
                    // Add status indicator for upcoming events
                    spans.push(VisualHierarchy::status_indicator("📅", theme.colors.palette.info));
                    spans.push(typography.create_span(
                        format!(" {}", content),
                        typography_level,
                        theme,
                    ));
                } else {
                    // Regular content with appropriate typography
                    spans.push(typography.create_span(content, typography_level, theme));
                }

                remaining_width = remaining_width.saturating_sub(segment_width);
            } else if remaining_width > 3 {
                // Truncate with ellipsis
                let truncated = format!("{}…", &content[..(remaining_width.saturating_sub(1) as usize).min(content.len())]);
                spans.push(typography.create_span(truncated, typography_level, theme));
                remaining_width = 0;
            }
        }

        Line::from(spans)
    }

    /// Determine appropriate layout mode based on available width
    fn determine_layout_mode(&self, width: u16) -> InformationDensity {
        if width >= self.layout_thresholds.full_width {
            InformationDensity::Standard
        } else if width >= self.layout_thresholds.compact_width {
            InformationDensity::Compact
        } else if width >= self.layout_thresholds.minimal_width {
            InformationDensity::Minimal
        } else {
            InformationDensity::Minimal // Force minimal even for very small widths
        }
    }

    /// Get priority-sorted segments based on layout mode and context
    fn get_prioritized_segments(&self, layout_mode: InformationDensity) -> Vec<(&String, &Box<dyn StatusSegment>)> {
        let mut segments: Vec<_> = self
            .segment_order
            .iter()
            .filter_map(|name| {
                self.segments.get(name).and_then(|segment| {
                    if segment.is_visible() {
                        Some((name, segment))
                    } else {
                        None
                    }
                })
            })
            .collect();

        // Sort by combined priority (base priority + context priority)
        segments.sort_by(|a, b| {
            let priority_a = self.get_effective_priority(a.0, a.1);
            let priority_b = self.get_effective_priority(b.0, b.1);
            priority_b.cmp(&priority_a) // Higher priority first
        });

        // Filter based on layout mode
        match layout_mode {
            InformationDensity::Minimal => {
                // Only show critical segments (priority >= 95)
                segments.into_iter().filter(|(name, segment)| {
                    self.get_effective_priority(name, segment) >= 95
                }).take(2).collect()
            }
            InformationDensity::Compact => {
                // Show important segments (priority >= 70)
                segments.into_iter().filter(|(name, segment)| {
                    self.get_effective_priority(name, segment) >= 70
                }).take(4).collect()
            }
            InformationDensity::Standard => {
                // Show all segments with priority >= 50
                segments.into_iter().filter(|(name, segment)| {
                    self.get_effective_priority(name, segment) >= 50
                }).collect()
            }
            InformationDensity::Detailed => segments, // Show all segments
        }
    }

    /// Calculate effective priority combining base and context priorities
    fn get_effective_priority(&self, name: &str, segment: &Box<dyn StatusSegment>) -> u8 {
        let base_priority = segment.priority();
        let context_boost = self.context_priorities.get(name).unwrap_or(&0);
        base_priority.saturating_add(*context_boost / 4) // Context provides up to 25% boost
    }

    /// Get adaptive separator style based on layout mode
    fn get_adaptive_separator_style(&self, layout_mode: InformationDensity) -> SeparatorStyle {
        match &self.separator_style {
            SeparatorStyle::Adaptive => {
                match layout_mode {
                    InformationDensity::Minimal => SeparatorStyle::Minimal,
                    InformationDensity::Compact => SeparatorStyle::Simple,
                    InformationDensity::Standard => SeparatorStyle::Powerline,
                    InformationDensity::Detailed => SeparatorStyle::Powerline,
                }
            }
            other => other.clone(), // Use fixed style if not adaptive
        }
    }

    /// Get context-appropriate border style
    fn get_context_border_style(&self, theme: &Theme) -> Style {
        // Check if any segment has high priority (indicating important status)
        let has_critical_info = self.segments.iter().any(|(name, segment)| {
            self.get_effective_priority(name, segment) >= 98
        });

        if has_critical_info {
            Style::default().fg(theme.colors.palette.warning)
        } else {
            theme.get_component_style("status_bar", false)
        }
    }

    /// Create intelligent segments content with context-aware formatting
    fn create_intelligent_segments_content(
        &self,
        visible_segments: &[(&String, &Box<dyn StatusSegment>)],
        available_width: u16,
        theme: &Theme,
        layout_mode: InformationDensity,
    ) -> Line {
        let mut spans = Vec::new();
        let separator_style = self.get_adaptive_separator_style(layout_mode);
        let separator_width = self.get_separator_width_for_style(&separator_style);
        let total_separator_width = separator_width * (visible_segments.len().saturating_sub(1)) as u16;
        let content_width = available_width.saturating_sub(total_separator_width);
        let mut remaining_width = content_width;

        for (i, (name, segment)) in visible_segments.iter().enumerate() {
            if remaining_width == 0 {
                break;
            }

            // Add separator between segments
            if i > 0 {
                spans.push(self.get_separator_for_style(&separator_style, theme));
            }

            // Get and format segment content based on layout mode
            let content = self.format_segment_content(name, segment, layout_mode);
            let segment_width = (content.len() as u16).min(remaining_width);

            // Apply intelligent truncation
            let display_content = if content.len() as u16 > segment_width {
                self.intelligent_truncate(&content, segment_width, name)
            } else {
                content
            };

            // Apply segment styling with emphasis for high-priority items
            let style = self.get_segment_style(name, segment, theme, layout_mode);
            spans.push(Span::styled(display_content, style));

            remaining_width = remaining_width.saturating_sub(segment_width);
        }

        Line::from(spans)
    }

    /// Format segment content based on information density
    fn format_segment_content(&self, name: &str, segment: &Box<dyn StatusSegment>, layout_mode: InformationDensity) -> String {
        let raw_content = segment.content();
        
        match layout_mode {
            InformationDensity::Minimal => {
                // Ultra-compact: just essential info
                match name {
                    "email" => self.compact_email_content(&raw_content),
                    "calendar" => self.compact_calendar_content(&raw_content),
                    "mode" => self.compact_mode_content(&raw_content),
                    "system" => self.compact_system_content(&raw_content),
                    _ => raw_content.chars().take(8).collect(),
                }
            }
            InformationDensity::Compact => {
                // Abbreviated labels but key info preserved
                match name {
                    "email" => raw_content.replace("unread", "u").replace("Mail:", "M:"),
                    "calendar" => raw_content.replace("events", "ev").replace("Calendar:", "C:"),
                    _ => raw_content,
                }
            }
            InformationDensity::Standard | InformationDensity::Detailed => raw_content,
        }
    }

    /// Compact email content for minimal display
    fn compact_email_content(&self, content: &str) -> String {
        if content.contains("unread") {
            // Extract just the unread count
            content.split_whitespace()
                .nth(1)
                .map(|count| format!("✉{}", count))
                .unwrap_or_else(|| "✉".to_string())
        } else {
            "✉".to_string()
        }
    }

    /// Compact calendar content for minimal display  
    fn compact_calendar_content(&self, content: &str) -> String {
        if content.contains("events today") {
            content.split_whitespace()
                .nth(1)
                .map(|count| format!("📅{}", count))
                .unwrap_or_else(|| "📅".to_string())
        } else {
            "📅".to_string()
        }
    }

    /// Compact mode content for minimal display
    fn compact_mode_content(&self, content: &str) -> String {
        // Extract just the icon if present
        content.split_whitespace()
            .next()
            .filter(|s| s.chars().any(|c| c as u32 > 127)) // Contains emoji/unicode
            .unwrap_or("•")
            .to_string()
    }

    /// Compact system content for minimal display
    fn compact_system_content(&self, content: &str) -> String {
        if content.contains('|') {
            content.split('|').last().unwrap_or(content).trim().to_string()
        } else {
            content.to_string()
        }
    }

    /// Intelligent truncation preserving important parts
    fn intelligent_truncate(&self, content: &str, max_width: u16, segment_name: &str) -> String {
        let max_len = max_width as usize;
        if content.len() <= max_len {
            return content.to_string();
        }

        if max_len <= 3 {
            return "…".to_string();
        }

        match segment_name {
            "email" => {
                // Preserve numbers (unread counts)
                if let Some(num_pos) = content.find(char::is_numeric) {
                    let num_end = content[num_pos..].find(char::is_whitespace).map(|i| i + num_pos).unwrap_or(content.len());
                    format!("{}…{}", &content[..2], &content[num_pos..num_end])
                } else {
                    format!("{}…", &content[..max_len - 1])
                }
            }
            "calendar" => {
                // Preserve event indicators
                if content.contains("🔴") {
                    format!("🔴{}…", &content[content.find("🔴").unwrap() + 4..].chars().take(max_len - 3).collect::<String>())
                } else {
                    format!("{}…", &content[..max_len - 1])
                }
            }
            _ => {
                format!("{}…", &content[..max_len - 1])
            }
        }
    }

    /// Get segment style with context-aware emphasis
    fn get_segment_style(&self, name: &str, segment: &Box<dyn StatusSegment>, theme: &Theme, _layout_mode: InformationDensity) -> Style {
        let base_style = segment.custom_style(theme).unwrap_or_else(|| {
            theme.get_component_style("status_bar", false)
        });

        let priority = self.get_effective_priority(name, segment);
        
        // Add emphasis for high-priority items
        if priority >= 95 {
            base_style.add_modifier(Modifier::BOLD)
        } else if priority >= 85 {
            base_style.add_modifier(Modifier::UNDERLINED)
        } else {
            base_style
        }
    }

    /// Get current status summary for debugging
    pub fn get_status_summary(&self) -> String {
        format!(
            "StatusBar: {} segments, position: {:?}, style: {:?}, density: {:?}",
            self.segments.len(),
            self.position,
            self.separator_style,
            self.information_density
        )
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new(StatusBarPosition::Bottom)
    }
}

impl Default for ResponsiveThresholds {
    fn default() -> Self {
        Self {
            full_width: 120,
            compact_width: 80,
            minimal_width: 40,
        }
    }
}
