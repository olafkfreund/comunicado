use crate::theme::Theme;
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

/// Professional status bar with powerline-style segments
pub struct StatusBar {
    segments: HashMap<String, Box<dyn StatusSegment>>,
    position: StatusBarPosition,
    segment_order: Vec<String>,
    separator_style: SeparatorStyle,
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
}

impl StatusBar {
    pub fn new(position: StatusBarPosition) -> Self {
        Self {
            segments: HashMap::new(),
            position,
            segment_order: Vec::new(),
            separator_style: SeparatorStyle::Powerline,
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

    /// Render the status bar
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
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

        // Calculate available width for segments
        let available_width = area.width.saturating_sub(2); // Account for borders
        let separator_width = self.get_separator_width();
        let total_separator_width =
            separator_width * (visible_segments.len().saturating_sub(1)) as u16;
        let content_width = available_width.saturating_sub(total_separator_width);

        // Create segments with adaptive sizing
        let segments_content =
            self.create_segments_content(&visible_segments, content_width, theme);

        // Create the status bar block
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme.get_component_style("status_bar", false));

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
        }
    }

    fn get_separator(&self, theme: &Theme) -> Span {
        let separator_text = match self.separator_style {
            SeparatorStyle::Powerline => " ⮰ ",
            SeparatorStyle::Simple => " | ",
            SeparatorStyle::Minimal => "  ",
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

    /// Get current status summary for debugging
    pub fn get_status_summary(&self) -> String {
        format!(
            "StatusBar: {} segments, position: {:?}, style: {:?}",
            self.segments.len(),
            self.position,
            self.separator_style
        )
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new(StatusBarPosition::Bottom)
    }
}
