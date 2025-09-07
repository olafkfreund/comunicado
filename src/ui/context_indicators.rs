/// Enhanced context indicators for better user orientation
///
/// This module provides a comprehensive context awareness system that helps users
/// understand their current location, available actions, and navigation options
/// within the application through multiple visual indicators.
use crate::theme::Theme;
use crate::ui::layout::{LayoutBreakpoint, LayoutMode};
use crate::ui::mode_indicator::ModeIndicator;
use crate::ui::typography::{TypographyLevel, TypographySystem};
use crate::ui::{FocusedPane, UIMode};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use std::collections::HashMap;
use tokio::time::Instant;

/// Context indicator configuration
#[derive(Debug, Clone)]
pub struct ContextIndicatorConfig {
    /// Show breadcrumb navigation
    pub show_breadcrumb: bool,
    /// Show focus indicators
    pub show_focus_indicators: bool,
    /// Show mode transitions
    pub show_mode_transitions: bool,
    /// Show available actions
    pub show_action_hints: bool,
    /// Show layout context
    pub show_layout_context: bool,
    /// Show keyboard shortcuts
    pub show_keyboard_hints: bool,
    /// Maximum breadcrumb depth
    pub breadcrumb_depth: usize,
    /// Animation duration for transitions
    pub animation_duration_ms: u64,
}

impl Default for ContextIndicatorConfig {
    fn default() -> Self {
        Self {
            show_breadcrumb: true,
            show_focus_indicators: true,
            show_mode_transitions: true,
            show_action_hints: true,
            show_layout_context: true,
            show_keyboard_hints: true,
            breadcrumb_depth: 4,
            animation_duration_ms: 300,
        }
    }
}

/// Focus indicator styles
#[derive(Debug, Clone, PartialEq)]
pub enum FocusIndicatorStyle {
    /// Subtle outline
    Outline,
    /// Colored border
    Border,
    /// Background highlight
    Background,
    /// Corner markers
    Corners,
    /// Glow effect
    Glow,
}

/// Breadcrumb item representing navigation path
#[derive(Debug, Clone)]
pub struct BreadcrumbItem {
    pub label: String,
    pub icon: Option<String>,
    pub mode: UIMode,
    pub context: Option<String>,
    pub is_current: bool,
    pub is_clickable: bool,
}

/// Layout context information
#[derive(Debug, Clone)]
pub struct LayoutContext {
    pub current_mode: LayoutMode,
    pub breakpoint: LayoutBreakpoint,
    pub visible_panes: Vec<FocusedPane>,
    pub hidden_panes: Vec<FocusedPane>,
    pub responsive_changes: Vec<String>,
}

/// Enhanced context indicators system
pub struct ContextIndicators {
    /// Configuration
    config: ContextIndicatorConfig,
    /// Mode indicator
    mode_indicator: ModeIndicator,
    /// Current focused pane
    focused_pane: FocusedPane,
    /// Navigation breadcrumb
    breadcrumb: Vec<BreadcrumbItem>,
    /// Layout context
    layout_context: Option<LayoutContext>,
    /// Available actions for current context
    available_actions: HashMap<String, String>,
    /// Focus indicator style
    focus_style: FocusIndicatorStyle,
    /// Animation timings
    animation_start: Option<Instant>,
    /// Context change history for smooth transitions
    context_history: Vec<(UIMode, FocusedPane, Instant)>,
    /// Tips and hints display
    current_tip: Option<String>,
    /// Location indicator (account/folder/message path)
    location_path: Vec<String>,
}

impl ContextIndicators {
    /// Create new context indicators system
    pub fn new() -> Self {
        Self {
            config: ContextIndicatorConfig::default(),
            mode_indicator: ModeIndicator::new(),
            focused_pane: FocusedPane::MessageList,
            breadcrumb: Vec::new(),
            layout_context: None,
            available_actions: HashMap::new(),
            focus_style: FocusIndicatorStyle::Border,
            animation_start: None,
            context_history: Vec::new(),
            current_tip: None,
            location_path: Vec::new(),
        }
    }

    /// Configure the context indicators
    pub fn configure(&mut self, config: ContextIndicatorConfig) {
        self.mode_indicator
            .set_show_breadcrumb(config.show_breadcrumb);
        self.config = config;
    }

    /// Update current context
    pub fn update_context(
        &mut self,
        mode: UIMode,
        focused_pane: FocusedPane,
        sub_context: Option<String>,
    ) {
        // Update mode indicator
        self.mode_indicator
            .set_mode(mode.clone(), sub_context.clone());

        // Track focus changes
        if focused_pane != self.focused_pane {
            self.focused_pane = focused_pane;
            self.animation_start = Some(Instant::now());
        }

        // Update context history
        self.context_history
            .push((mode.clone(), focused_pane, Instant::now()));
        if self.context_history.len() > 10 {
            self.context_history.remove(0);
        }

        // Update available actions based on context
        self.update_available_actions(&mode, &focused_pane);

        // Generate contextual tip
        self.update_contextual_tip(&mode, &focused_pane, sub_context.as_deref());
    }

    /// Update layout context information
    pub fn update_layout_context(&mut self, layout_context: LayoutContext) {
        self.layout_context = Some(layout_context);
    }

    /// Set current location path (account > folder > message)
    pub fn set_location_path(&mut self, path: Vec<String>) {
        self.location_path = path;
        self.update_breadcrumb();
    }

    /// Set focus indicator style
    pub fn set_focus_style(&mut self, style: FocusIndicatorStyle) {
        self.focus_style = style;
    }

    /// Update animation progress
    pub fn update_animations(&mut self, delta_time: f32) {
        self.mode_indicator.update_animation(delta_time);
    }

    /// Render all context indicators
    pub fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        theme: &Theme,
        typography: &TypographySystem,
    ) {
        // Main mode indicator (top priority)
        if area.height >= 1 {
            let mode_area = Rect {
                x: area.x,
                y: area.y,
                width: area.width,
                height: 1,
            };
            self.mode_indicator.render(frame, mode_area, theme);
        }

        // Extended context information for larger areas
        if area.height >= 3 {
            let extended_area = Rect {
                x: area.x,
                y: area.y + 1,
                width: area.width,
                height: area.height - 1,
            };
            self.render_extended_context(frame, extended_area, theme, typography);
        }
    }

    /// Render focus indicators for a specific pane
    pub fn render_focus_indicator(
        &self,
        frame: &mut Frame,
        area: Rect,
        pane: FocusedPane,
        theme: &Theme,
    ) {
        if !self.config.show_focus_indicators || pane != self.focused_pane {
            return;
        }

        match self.focus_style {
            FocusIndicatorStyle::Outline => self.render_outline_focus(frame, area, theme),
            FocusIndicatorStyle::Border => self.render_border_focus(frame, area, theme),
            FocusIndicatorStyle::Background => self.render_background_focus(frame, area, theme),
            FocusIndicatorStyle::Corners => self.render_corner_focus(frame, area, theme),
            FocusIndicatorStyle::Glow => self.render_glow_focus(frame, area, theme),
        }
    }

    /// Render breadcrumb navigation
    pub fn render_breadcrumb(
        &self,
        frame: &mut Frame,
        area: Rect,
        theme: &Theme,
        typography: &TypographySystem,
    ) {
        if !self.config.show_breadcrumb || self.breadcrumb.is_empty() {
            return;
        }

        let mut spans = Vec::new();
        let max_items = (area.width as usize / 15).max(3); // Estimate space per item
        let items_to_show = self
            .breadcrumb
            .iter()
            .rev()
            .take(max_items)
            .collect::<Vec<_>>();

        for (i, item) in items_to_show.iter().enumerate() {
            if i > 0 {
                spans.push(typography.create_span(
                    " > ".to_string(),
                    TypographyLevel::Caption,
                    theme,
                ));
            }

            // Add icon if available
            if let Some(ref icon) = item.icon {
                spans.push(Span::styled(
                    format!("{} ", icon),
                    Style::default().fg(theme.colors.palette.accent),
                ));
            }

            // Add label with appropriate styling
            let style = if item.is_current {
                Style::default()
                    .fg(theme.colors.palette.text_primary)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.colors.palette.text_secondary)
            };

            spans.push(Span::styled(item.label.clone(), style));
        }

        if spans.len() > max_items * 3 {
            // Truncate if too long and add ellipsis
            spans.truncate(max_items * 3 - 1);
            spans.push(Span::styled(
                "...",
                Style::default().fg(theme.colors.palette.text_muted),
            ));
        }

        let paragraph = Paragraph::new(Line::from(spans))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    /// Render available actions hints
    pub fn render_action_hints(
        &self,
        frame: &mut Frame,
        area: Rect,
        theme: &Theme,
        typography: &TypographySystem,
    ) {
        if !self.config.show_action_hints || self.available_actions.is_empty() {
            return;
        }

        let actions: Vec<String> = self
            .available_actions
            .iter()
            .take(5) // Limit to 5 most important actions
            .map(|(key, desc)| format!("{}: {}", key, desc))
            .collect();

        let content = actions.join(" | ");
        let span = typography.create_span(content, TypographyLevel::Caption, theme);

        let paragraph = Paragraph::new(Line::from(vec![span]))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    /// Render layout context information
    pub fn render_layout_context(
        &self,
        frame: &mut Frame,
        area: Rect,
        theme: &Theme,
        typography: &TypographySystem,
    ) {
        if !self.config.show_layout_context {
            return;
        }

        let context_info = if let Some(ref ctx) = self.layout_context {
            format!(
                "{:?} ({:?}) | {} visible",
                ctx.current_mode,
                ctx.breakpoint,
                ctx.visible_panes.len()
            )
        } else {
            "Layout: Unknown".to_string()
        };

        let span = typography.create_span(context_info, TypographyLevel::Metadata, theme);
        let paragraph = Paragraph::new(Line::from(vec![span])).alignment(Alignment::Right);

        frame.render_widget(paragraph, area);
    }

    /// Get current contextual tip
    pub fn get_current_tip(&self) -> Option<&String> {
        self.current_tip.as_ref()
    }

    /// Get current focus indicator position for external use
    pub fn get_focus_area_highlight(&self, base_area: Rect) -> Option<Rect> {
        if !self.config.show_focus_indicators {
            return None;
        }

        // Expand area slightly for focus indication
        Some(Rect {
            x: base_area.x.saturating_sub(1),
            y: base_area.y.saturating_sub(1),
            width: base_area.width.saturating_add(2).min(base_area.width + 2),
            height: base_area.height.saturating_add(2).min(base_area.height + 2),
        })
    }

    // Private helper methods

    /// Render extended context information
    fn render_extended_context(
        &self,
        frame: &mut Frame,
        area: Rect,
        theme: &Theme,
        typography: &TypographySystem,
    ) {
        if area.height < 2 {
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Breadcrumb
                Constraint::Length(1), // Action hints or layout context
            ])
            .split(area);

        // Render breadcrumb if we have space
        if chunks.len() >= 1 {
            self.render_breadcrumb(frame, chunks[0], theme, typography);
        }

        // Render action hints or layout context in bottom line
        if chunks.len() >= 2 && area.width > 40 {
            if self.config.show_action_hints && !self.available_actions.is_empty() {
                self.render_action_hints(frame, chunks[1], theme, typography);
            } else if self.config.show_layout_context {
                self.render_layout_context(frame, chunks[1], theme, typography);
            }
        }
    }

    /// Render outline focus indicator
    fn render_outline_focus(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let focus_color = theme.colors.palette.accent;
        let block = Block::default().borders(Borders::ALL).border_style(
            Style::default()
                .fg(focus_color)
                .add_modifier(Modifier::BOLD),
        );

        frame.render_widget(Clear, area);
        frame.render_widget(block, area);
    }

    /// Render border focus indicator
    fn render_border_focus(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let focus_color = theme.colors.palette.accent;

        // Top border
        let top_border = "━".repeat(area.width as usize);
        let top_para = Paragraph::new(top_border).style(Style::default().fg(focus_color));
        frame.render_widget(
            top_para,
            Rect {
                x: area.x,
                y: area.y,
                width: area.width,
                height: 1,
            },
        );

        // Bottom border if area is tall enough
        if area.height > 1 {
            let bottom_para = Paragraph::new("━".repeat(area.width as usize))
                .style(Style::default().fg(focus_color));
            frame.render_widget(
                bottom_para,
                Rect {
                    x: area.x,
                    y: area.y + area.height - 1,
                    width: area.width,
                    height: 1,
                },
            );
        }
    }

    /// Render background focus indicator
    fn render_background_focus(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let focus_bg = theme.colors.palette.surface;
        let highlight = Block::default().style(Style::default().bg(focus_bg));

        frame.render_widget(highlight, area);
    }

    /// Render corner focus indicator
    fn render_corner_focus(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let focus_color = theme.colors.palette.accent;

        // Top-left corner
        let tl_para = Paragraph::new("┏").style(
            Style::default()
                .fg(focus_color)
                .add_modifier(Modifier::BOLD),
        );
        frame.render_widget(
            tl_para,
            Rect {
                x: area.x,
                y: area.y,
                width: 1,
                height: 1,
            },
        );

        // Top-right corner
        if area.width > 1 {
            let tr_para = Paragraph::new("┓").style(
                Style::default()
                    .fg(focus_color)
                    .add_modifier(Modifier::BOLD),
            );
            frame.render_widget(
                tr_para,
                Rect {
                    x: area.x + area.width - 1,
                    y: area.y,
                    width: 1,
                    height: 1,
                },
            );
        }

        // Bottom corners if tall enough
        if area.height > 1 {
            let bl_para = Paragraph::new("┗").style(
                Style::default()
                    .fg(focus_color)
                    .add_modifier(Modifier::BOLD),
            );
            frame.render_widget(
                bl_para,
                Rect {
                    x: area.x,
                    y: area.y + area.height - 1,
                    width: 1,
                    height: 1,
                },
            );

            if area.width > 1 {
                let br_para = Paragraph::new("┛").style(
                    Style::default()
                        .fg(focus_color)
                        .add_modifier(Modifier::BOLD),
                );
                frame.render_widget(
                    br_para,
                    Rect {
                        x: area.x + area.width - 1,
                        y: area.y + area.height - 1,
                        width: 1,
                        height: 1,
                    },
                );
            }
        }
    }

    /// Render glow focus indicator
    fn render_glow_focus(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Simulate glow with multiple colored borders
        let colors = [
            theme.colors.palette.accent,
            theme.colors.palette.info,
            theme.colors.palette.success,
        ];

        for (i, &color) in colors.iter().enumerate() {
            if area.width > i as u16 * 2 && area.height > i as u16 * 2 {
                let glow_area = Rect {
                    x: area.x + i as u16,
                    y: area.y + i as u16,
                    width: area.width.saturating_sub(i as u16 * 2),
                    height: area.height.saturating_sub(i as u16 * 2),
                };

                let glow_block = Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(color).add_modifier(Modifier::DIM));

                frame.render_widget(glow_block, glow_area);
            }
        }
    }

    /// Update available actions based on current context
    fn update_available_actions(&mut self, mode: &UIMode, focused_pane: &FocusedPane) {
        self.available_actions.clear();

        match (mode, focused_pane) {
            (UIMode::Normal, FocusedPane::MessageList) => {
                self.available_actions
                    .insert("Enter".to_string(), "Open email".to_string());
                self.available_actions
                    .insert("c".to_string(), "Compose".to_string());
                self.available_actions
                    .insert("r".to_string(), "Reply".to_string());
                self.available_actions
                    .insert("d".to_string(), "Delete".to_string());
                self.available_actions
                    .insert("/".to_string(), "Search".to_string());
            }
            (UIMode::Normal, FocusedPane::FolderTree) => {
                self.available_actions
                    .insert("Enter".to_string(), "Open folder".to_string());
                self.available_actions
                    .insert("n".to_string(), "New folder".to_string());
                self.available_actions
                    .insert("Tab".to_string(), "Switch pane".to_string());
            }
            (UIMode::Compose, _) => {
                self.available_actions
                    .insert("Ctrl+S".to_string(), "Send email".to_string());
                self.available_actions
                    .insert("Ctrl+D".to_string(), "Save draft".to_string());
                self.available_actions
                    .insert("Esc".to_string(), "Cancel".to_string());
                self.available_actions
                    .insert("Tab".to_string(), "Next field".to_string());
            }
            (UIMode::Calendar, _) => {
                self.available_actions
                    .insert("n".to_string(), "New event".to_string());
                self.available_actions
                    .insert("d".to_string(), "Day view".to_string());
                self.available_actions
                    .insert("w".to_string(), "Week view".to_string());
                self.available_actions
                    .insert("m".to_string(), "Month view".to_string());
                self.available_actions
                    .insert("t".to_string(), "Go to today".to_string());
            }
            (UIMode::Search, _) => {
                self.available_actions
                    .insert("Enter".to_string(), "Open result".to_string());
                self.available_actions
                    .insert("n".to_string(), "Next result".to_string());
                self.available_actions
                    .insert("N".to_string(), "Previous result".to_string());
                self.available_actions
                    .insert("Esc".to_string(), "Clear search".to_string());
            }
            _ => {
                self.available_actions
                    .insert("?".to_string(), "Help".to_string());
                self.available_actions
                    .insert("Esc".to_string(), "Back".to_string());
                self.available_actions
                    .insert("q".to_string(), "Quit mode".to_string());
            }
        }
    }

    /// Update contextual tip based on current state
    fn update_contextual_tip(
        &mut self,
        mode: &UIMode,
        focused_pane: &FocusedPane,
        sub_context: Option<&str>,
    ) {
        self.current_tip = match (mode, focused_pane) {
            (UIMode::Normal, FocusedPane::MessageList) if sub_context.is_some() => {
                Some(format!("Viewing messages in {}", sub_context.unwrap()))
            }
            (UIMode::Normal, FocusedPane::MessageList) => {
                Some("Use j/k to navigate messages, Enter to open".to_string())
            }
            (UIMode::Normal, FocusedPane::FolderTree) => {
                Some("Navigate folders with j/k, Enter to select".to_string())
            }
            (UIMode::Compose, _) => Some("Fill in email details, Ctrl+S to send".to_string()),
            (UIMode::Calendar, _) => {
                Some("Navigate calendar events, 'n' for new event".to_string())
            }
            (UIMode::Search, _) => {
                Some("Search results: Enter to open, n/N for navigation".to_string())
            }
            (UIMode::Settings, _) => {
                Some("Adjust application settings, Tab to navigate sections".to_string())
            }
            _ => None,
        };
    }

    /// Update breadcrumb based on current location
    fn update_breadcrumb(&mut self) {
        self.breadcrumb.clear();

        // Add location path items (account > folder > message)
        for (i, path_item) in self.location_path.iter().enumerate() {
            let is_current = i == self.location_path.len() - 1;
            let icon = match i {
                0 => Some("👤".to_string()), // Account
                1 => Some("📁".to_string()), // Folder
                _ => Some("📧".to_string()), // Message or sub-folder
            };

            self.breadcrumb.push(BreadcrumbItem {
                label: path_item.clone(),
                icon,
                mode: UIMode::Normal, // Default mode for path items
                context: None,
                is_current,
                is_clickable: !is_current,
            });
        }

        // Add mode context if different from Normal
        let current_mode = self.mode_indicator.current_mode();
        if *current_mode != UIMode::Normal {
            let mode_info = match current_mode {
                UIMode::Compose => ("Compose", Some("✏️".to_string())),
                UIMode::Calendar => ("Calendar", Some("📅".to_string())),
                UIMode::Search => ("Search", Some("🔍".to_string())),
                UIMode::Settings => ("Settings", Some("⚙️".to_string())),
                _ => ("View", Some("👁️".to_string())),
            };

            self.breadcrumb.push(BreadcrumbItem {
                label: mode_info.0.to_string(),
                icon: mode_info.1,
                mode: current_mode.clone(),
                context: None,
                is_current: true,
                is_clickable: false,
            });
        }

        // Limit breadcrumb depth
        if self.breadcrumb.len() > self.config.breadcrumb_depth {
            let start = self.breadcrumb.len() - self.config.breadcrumb_depth;
            self.breadcrumb = self.breadcrumb[start..].to_vec();
        }
    }
}

impl Default for ContextIndicators {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_indicators_creation() {
        let indicators = ContextIndicators::new();
        assert_eq!(indicators.focused_pane, FocusedPane::MessageList);
        assert!(indicators.breadcrumb.is_empty());
    }

    #[test]
    fn test_context_updates() {
        let mut indicators = ContextIndicators::new();

        indicators.update_context(
            UIMode::Compose,
            FocusedPane::MessageList,
            Some("New email".to_string()),
        );

        assert_eq!(indicators.focused_pane, FocusedPane::MessageList);
        assert!(!indicators.available_actions.is_empty());
        assert!(indicators.current_tip.is_some());
    }

    #[test]
    fn test_location_path() {
        let mut indicators = ContextIndicators::new();
        let path = vec![
            "user@example.com".to_string(),
            "INBOX".to_string(),
            "Important email".to_string(),
        ];

        indicators.set_location_path(path);
        assert_eq!(indicators.breadcrumb.len(), 3);
        assert!(indicators.breadcrumb.last().unwrap().is_current);
    }

    #[test]
    fn test_available_actions() {
        let mut indicators = ContextIndicators::new();

        indicators.update_context(UIMode::Normal, FocusedPane::MessageList, None);
        assert!(indicators.available_actions.contains_key("Enter"));
        assert!(indicators.available_actions.contains_key("c"));

        indicators.update_context(UIMode::Compose, FocusedPane::MessageList, None);
        assert!(indicators.available_actions.contains_key("Ctrl+S"));
        assert!(!indicators.available_actions.contains_key("c"));
    }

    #[test]
    fn test_focus_styles() {
        let mut indicators = ContextIndicators::new();

        indicators.set_focus_style(FocusIndicatorStyle::Glow);
        assert_eq!(indicators.focus_style, FocusIndicatorStyle::Glow);

        indicators.set_focus_style(FocusIndicatorStyle::Border);
        assert_eq!(indicators.focus_style, FocusIndicatorStyle::Border);
    }
}
