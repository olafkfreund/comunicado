//! Visual mode indicators and transitions for better UX
//!
//! This module provides comprehensive visual feedback for application mode changes,
//! including animated transitions, breadcrumbs, and contextual hints to reduce
//! mode confusion and improve user orientation.

use crate::theme::Theme;
use crate::ui::UIMode;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Gauge, Paragraph, Wrap},
    Frame,
};
use std::collections::VecDeque;

/// Visual mode transition effects
#[derive(Debug, Clone, PartialEq)]
pub enum TransitionEffect {
    None,
    Slide,
    Fade,
    Flash,
}

/// Mode indicator display style
#[derive(Debug, Clone, PartialEq)]
pub enum IndicatorStyle {
    Compact,    // Icon + name only
    Detailed,   // Icon + name + context
    Full,       // Icon + name + context + breadcrumb
}

/// Visual mode indicator system
pub struct ModeIndicator {
    current_mode: UIMode,
    previous_mode: Option<UIMode>,
    mode_history: VecDeque<UIMode>,
    sub_context: Option<String>,
    transition_progress: f32, // 0.0 to 1.0 for animations
    style: IndicatorStyle,
    show_breadcrumb: bool,
    flash_countdown: u8, // For attention-grabbing mode changes
    is_vim_mode: bool,
}

impl ModeIndicator {
    /// Create a new mode indicator
    pub fn new() -> Self {
        Self {
            current_mode: UIMode::Normal,
            previous_mode: None,
            mode_history: VecDeque::with_capacity(5),
            sub_context: None,
            transition_progress: 1.0, // Start fully transitioned
            style: IndicatorStyle::Detailed,
            show_breadcrumb: true,
            flash_countdown: 0,
            is_vim_mode: true, // Default to vim mode
        }
    }

    /// Update the current mode with visual transition
    pub fn set_mode(&mut self, new_mode: UIMode, sub_context: Option<String>) {
        if new_mode != self.current_mode {
            // Add to history
            if !self.mode_history.is_empty() && *self.mode_history.back().unwrap() != self.current_mode {
                self.mode_history.push_back(self.current_mode.clone());
            } else if self.mode_history.is_empty() {
                self.mode_history.push_back(self.current_mode.clone());
            }

            // Keep history size manageable
            if self.mode_history.len() > 5 {
                self.mode_history.pop_front();
            }

            self.previous_mode = Some(self.current_mode.clone());
            self.current_mode = new_mode;
            self.sub_context = sub_context;
            self.transition_progress = 0.0;
            
            // Flash for important mode changes
            if self.is_important_mode_change(&self.current_mode) {
                self.flash_countdown = 3;
            }
        } else {
            // Just update sub-context
            self.sub_context = sub_context;
        }
    }

    /// Set display style
    pub fn set_style(&mut self, style: IndicatorStyle) {
        self.style = style;
    }

    /// Toggle breadcrumb display
    pub fn set_show_breadcrumb(&mut self, show: bool) {
        self.show_breadcrumb = show;
    }

    /// Set vim mode status
    pub fn set_vim_mode(&mut self, is_vim: bool) {
        self.is_vim_mode = is_vim;
    }

    /// Update animation progress (call from main loop)
    pub fn update_animation(&mut self, delta_time: f32) {
        if self.transition_progress < 1.0 {
            self.transition_progress = (self.transition_progress + delta_time * 4.0).min(1.0);
        }

        if self.flash_countdown > 0 {
            self.flash_countdown = self.flash_countdown.saturating_sub(1);
        }
    }

    /// Render the mode indicator
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        match self.style {
            IndicatorStyle::Compact => self.render_compact(frame, area, theme),
            IndicatorStyle::Detailed => self.render_detailed(frame, area, theme),
            IndicatorStyle::Full => self.render_full(frame, area, theme),
        }
    }

    /// Render compact mode indicator (header bar)
    pub fn render_compact(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let mode_info = self.get_mode_info();
        let content = format!("{} {}", mode_info.icon, mode_info.name);

        let style = if self.flash_countdown > 0 {
            Style::default()
                .fg(theme.colors.palette.warning)
                .add_modifier(Modifier::BOLD | Modifier::RAPID_BLINK)
        } else {
            Style::default()
                .fg(mode_info.color)
                .add_modifier(Modifier::BOLD)
        };

        let paragraph = Paragraph::new(content)
            .style(style)
            .alignment(Alignment::Center);

        frame.render_widget(paragraph, area);
    }

    /// Render detailed mode indicator with context
    pub fn render_detailed(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let mode_info = self.get_mode_info();
        let mut spans = vec![
            Span::styled(mode_info.icon, Style::default().fg(mode_info.color)),
            Span::styled(" ", Style::default()),
            Span::styled(
                mode_info.name,
                Style::default()
                    .fg(mode_info.color)
                    .add_modifier(Modifier::BOLD),
            ),
        ];

        // Add sub-context if available
        if let Some(ref context) = self.sub_context {
            spans.extend(vec![
                Span::styled(" (", Style::default().fg(theme.colors.palette.text_secondary)),
                Span::styled(
                    context.clone(),
                    Style::default()
                        .fg(theme.colors.palette.text_secondary)
                        .add_modifier(Modifier::ITALIC),
                ),
                Span::styled(")", Style::default().fg(theme.colors.palette.text_secondary)),
            ]);
        }

        // Add vim mode indicator
        if self.is_vim_mode {
            spans.extend(vec![
                Span::styled(" [", Style::default().fg(theme.colors.palette.text_muted)),
                Span::styled(
                    "VIM",
                    Style::default()
                        .fg(theme.colors.palette.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("]", Style::default().fg(theme.colors.palette.text_muted)),
            ]);
        }

        let paragraph = Paragraph::new(Line::from(spans))
            .alignment(Alignment::Center);

        frame.render_widget(paragraph, area);
    }

    /// Render full mode indicator with breadcrumb and transition
    pub fn render_full(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if area.height < 3 {
            self.render_detailed(frame, area, theme);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Main mode indicator
                Constraint::Length(1), // Breadcrumb or transition indicator
                Constraint::Min(1),    // Additional context
            ])
            .split(area);

        // Main mode line
        self.render_detailed(frame, chunks[0], theme);

        // Breadcrumb or transition indicator
        if self.transition_progress < 1.0 && self.previous_mode.is_some() {
            self.render_transition_indicator(frame, chunks[1], theme);
        } else if self.show_breadcrumb && !self.mode_history.is_empty() {
            self.render_breadcrumb(frame, chunks[1], theme);
        }

        // Additional context area
        if chunks[2].height > 0 {
            self.render_context_hints(frame, chunks[2], theme);
        }
    }

    /// Render transition animation
    fn render_transition_indicator(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if let Some(ref prev_mode) = self.previous_mode {
            let prev_info = self.get_mode_info_for_mode(prev_mode);
            let curr_info = self.get_mode_info();
            
            let transition_text = format!(
                "{} {} → {} {}",
                prev_info.icon, prev_info.name,
                curr_info.icon, curr_info.name
            );

            let progress_bar = Gauge::default()
                .percent((self.transition_progress * 100.0) as u16)
                .label(transition_text)
                .gauge_style(Style::default().fg(theme.colors.palette.accent))
                .use_unicode(true);

            frame.render_widget(progress_bar, area);
        }
    }

    /// Render mode history breadcrumb
    fn render_breadcrumb(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if self.mode_history.is_empty() {
            return;
        }

        let mut spans = Vec::new();
        let recent_modes: Vec<_> = self.mode_history.iter().rev().take(3).collect();
        
        for (i, mode) in recent_modes.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(" ← ", Style::default().fg(theme.colors.palette.text_muted)));
            }
            
            let mode_info = self.get_mode_info_for_mode(mode);
            spans.push(Span::styled(
                format!("{} {}", mode_info.icon, mode_info.name),
                Style::default().fg(theme.colors.palette.text_muted),
            ));
        }

        let paragraph = Paragraph::new(Line::from(spans))
            .alignment(Alignment::Center)
            .style(Style::default().add_modifier(Modifier::DIM));

        frame.render_widget(paragraph, area);
    }

    /// Render contextual hints and shortcuts
    fn render_context_hints(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let hints = self.get_context_hints();
        if hints.is_empty() {
            return;
        }

        let hints_text: Vec<String> = hints
            .iter()
            .take(3)
            .map(|(key, desc)| format!("{}: {}", key, desc))
            .collect();

        let content = hints_text.join(" | ");
        
        let paragraph = Paragraph::new(content)
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.colors.palette.text_muted))
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    /// Check if a mode change is important enough to flash
    fn is_important_mode_change(&self, mode: &UIMode) -> bool {
        matches!(
            mode,
            UIMode::Compose | UIMode::Settings | UIMode::EventCreate | UIMode::EventEdit
        )
    }

    /// Get mode information for current mode
    fn get_mode_info(&self) -> ModeInfo {
        self.get_mode_info_for_mode(&self.current_mode)
    }
    
    /// Get the description of the current mode
    pub fn get_mode_description(&self) -> &'static str {
        self.get_mode_info().description
    }

    /// Get mode information for any mode
    fn get_mode_info_for_mode(&self, mode: &UIMode) -> ModeInfo {
        match mode {
            UIMode::Normal => ModeInfo {
                name: "Mail",
                icon: "📧",
                color: Color::Cyan,
                description: "Email management",
            },
            UIMode::Compose => ModeInfo {
                name: "Compose",
                icon: "✏️",
                color: Color::Yellow,
                description: "Writing email",
            },
            UIMode::DraftList => ModeInfo {
                name: "Drafts",
                icon: "📝",
                color: Color::LightYellow,
                description: "Draft messages",
            },
            UIMode::Calendar => ModeInfo {
                name: "Calendar",
                icon: "📅",
                color: Color::Green,
                description: "Schedule and events",
            },
            UIMode::EventCreate => ModeInfo {
                name: "New Event",
                icon: "➕",
                color: Color::LightGreen,
                description: "Creating event",
            },
            UIMode::EventEdit => ModeInfo {
                name: "Edit Event",
                icon: "📝",
                color: Color::Yellow,
                description: "Editing event",
            },
            UIMode::EventView => ModeInfo {
                name: "Event",
                icon: "👁️",
                color: Color::Blue,
                description: "Viewing event",
            },
            UIMode::EmailViewer => ModeInfo {
                name: "Reading",
                icon: "📖",
                color: Color::Blue,
                description: "Reading email",
            },
            UIMode::InvitationViewer => ModeInfo {
                name: "Invitation",
                icon: "📨",
                color: Color::Magenta,
                description: "Calendar invitation",
            },
            UIMode::Search => ModeInfo {
                name: "Search",
                icon: "🔍",
                color: Color::Cyan,
                description: "Finding content",
            },
            UIMode::KeyboardShortcuts => ModeInfo {
                name: "Shortcuts",
                icon: "⌨️",
                color: Color::White,
                description: "Keyboard reference",
            },
            UIMode::Settings => ModeInfo {
                name: "Settings",
                icon: "⚙️",
                color: Color::Gray,
                description: "Configuration",
            },
            UIMode::ContactsPopup => ModeInfo {
                name: "Contacts",
                icon: "👤",
                color: Color::LightBlue,
                description: "Contact picker",
            },
            UIMode::Contacts => ModeInfo {
                name: "Address Book",
                icon: "📞",
                color: Color::Blue,
                description: "Contact management",
            },
            UIMode::ContextAware => ModeInfo {
                name: "Context",
                icon: "🔗",
                color: Color::Magenta,
                description: "Integrated view",
            },
        }
    }

    /// Get context-sensitive keyboard hints
    fn get_context_hints(&self) -> Vec<(String, String)> {
        match self.current_mode {
            UIMode::Normal => vec![
                ("j/k".to_string(), "navigate".to_string()),
                ("Enter".to_string(), "open".to_string()),
                ("c".to_string(), "compose".to_string()),
            ],
            UIMode::Compose => vec![
                ("Ctrl+S".to_string(), "send".to_string()),
                ("Ctrl+D".to_string(), "draft".to_string()),
                ("Esc".to_string(), "cancel".to_string()),
            ],
            UIMode::Calendar => vec![
                ("n".to_string(), "new event".to_string()),
                ("d/w/m".to_string(), "day/week/month".to_string()),
                ("t".to_string(), "today".to_string()),
            ],
            UIMode::Search => vec![
                ("/".to_string(), "search".to_string()),
                ("n/N".to_string(), "next/prev".to_string()),
                ("Esc".to_string(), "clear".to_string()),
            ],
            UIMode::Settings => vec![
                ("Tab".to_string(), "sections".to_string()),
                ("Enter".to_string(), "edit".to_string()),
                ("r".to_string(), "reset".to_string()),
            ],
            _ => vec![
                ("?".to_string(), "help".to_string()),
                ("Esc".to_string(), "back".to_string()),
                ("q".to_string(), "quit".to_string()),
            ],
        }
    }

    /// Get current mode for external access
    pub fn current_mode(&self) -> &UIMode {
        &self.current_mode
    }

    /// Get mode history
    pub fn mode_history(&self) -> &VecDeque<UIMode> {
        &self.mode_history
    }

    /// Clear mode history
    pub fn clear_history(&mut self) {
        self.mode_history.clear();
    }
}

/// Mode information structure
#[derive(Debug, Clone)]
struct ModeInfo {
    name: &'static str,
    icon: &'static str,
    color: Color,
    description: &'static str,
}

impl Default for ModeIndicator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mode_indicator_creation() {
        let indicator = ModeIndicator::new();
        assert_eq!(*indicator.current_mode(), UIMode::Normal);
        assert!(indicator.mode_history().is_empty());
    }

    #[test]
    fn test_mode_transitions() {
        let mut indicator = ModeIndicator::new();
        
        indicator.set_mode(UIMode::Compose, Some("new email".to_string()));
        assert_eq!(*indicator.current_mode(), UIMode::Compose);
        assert_eq!(indicator.mode_history().len(), 1);
        
        indicator.set_mode(UIMode::Settings, None);
        assert_eq!(*indicator.current_mode(), UIMode::Settings);
        assert_eq!(indicator.mode_history().len(), 2);
    }

    #[test]
    fn test_history_limit() {
        let mut indicator = ModeIndicator::new();
        
        // Add more than 5 modes to test limit
        for i in 0..10 {
            let mode = if i % 2 == 0 { UIMode::Compose } else { UIMode::Calendar };
            indicator.set_mode(mode, None);
        }
        
        assert!(indicator.mode_history().len() <= 5);
    }

    #[test]
    fn test_important_mode_changes() {
        let indicator = ModeIndicator::new();
        
        assert!(indicator.is_important_mode_change(&UIMode::Compose));
        assert!(indicator.is_important_mode_change(&UIMode::Settings));
        assert!(!indicator.is_important_mode_change(&UIMode::Normal));
        assert!(!indicator.is_important_mode_change(&UIMode::EmailViewer));
    }
}