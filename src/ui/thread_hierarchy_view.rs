//! Enhanced email thread hierarchy visualization
//!
//! This module provides a specialized UI component for displaying email threads
//! with improved visual hierarchy, better readability, and enhanced user experience.
//! It includes features like conversation flow, participant tracking, and smart
//! thread summarization.

use crate::theme::Theme;
use crate::ui::threading_display::{ThreadingDisplay, ThreadingStyle, ThreadContext, ConnectionType};
use crate::ui::message_list::MessageItem;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use std::collections::HashMap;

/// Thread hierarchy visualization modes
#[derive(Debug, Clone, PartialEq)]
pub enum HierarchyMode {
    Conversation,   // Chat-like conversation view
    TreeStructure,  // Traditional tree structure
    Timeline,       // Chronological timeline view
    Compact,        // Space-efficient compact view
}

/// Thread participant information
#[derive(Debug, Clone)]
pub struct ThreadParticipant {
    pub email: String,
    pub display_name: String,
    pub message_count: usize,
    pub is_current_user: bool,
    pub last_message_date: String,
}

/// Enhanced thread hierarchy viewer
pub struct ThreadHierarchyView {
    threading_display: ThreadingDisplay,
    hierarchy_mode: HierarchyMode,
    show_participants_panel: bool,
    show_thread_summary: bool,
    focus_on_unread: bool,
    max_preview_lines: usize,
    participants: HashMap<String, ThreadParticipant>,
    thread_statistics: ThreadStatistics,
}

/// Thread statistics for enhanced display
#[derive(Debug, Clone, Default)]
pub struct ThreadStatistics {
    pub total_messages: usize,
    pub unread_count: usize,
    pub participants_count: usize,
    pub date_range: (String, String), // (oldest, newest)
    pub has_attachments: bool,
    pub has_important: bool,
}

impl ThreadHierarchyView {
    /// Create a new thread hierarchy view
    pub fn new() -> Self {
        Self {
            threading_display: ThreadingDisplay::new().with_style(ThreadingStyle::Modern),
            hierarchy_mode: HierarchyMode::TreeStructure,
            show_participants_panel: true,
            show_thread_summary: true,
            focus_on_unread: true,
            max_preview_lines: 3,
            participants: HashMap::new(),
            thread_statistics: ThreadStatistics::default(),
        }
    }

    /// Set the hierarchy display mode
    pub fn with_mode(mut self, mode: HierarchyMode) -> Self {
        self.hierarchy_mode = mode;
        self
    }

    /// Enable/disable participants panel
    pub fn with_participants_panel(mut self, show: bool) -> Self {
        self.show_participants_panel = show;
        self
    }

    /// Set threading display style
    pub fn with_threading_style(mut self, style: ThreadingStyle) -> Self {
        self.threading_display = self.threading_display.with_style(style);
        self
    }

    /// Update thread data for enhanced display
    pub fn update_thread_data(&mut self, messages: &[MessageItem]) {
        self.analyze_thread_participants(messages);
        self.calculate_thread_statistics(messages);
    }

    /// Render the complete thread hierarchy view
    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme, messages: &[MessageItem], selected_index: usize) {
        let layout = if self.show_participants_panel && area.width > 80 {
            // Three-column layout: hierarchy | messages | participants
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(30), // Thread hierarchy
                    Constraint::Percentage(50), // Message content
                    Constraint::Percentage(20), // Participants panel
                ])
                .split(area)
        } else {
            // Two-column layout: hierarchy | messages
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(40), // Thread hierarchy
                    Constraint::Percentage(60), // Message content
                ])
                .split(area)
        };

        // Render thread summary at top if enabled
        if self.show_thread_summary && layout[0].height > 8 {
            let summary_area = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Summary
                    Constraint::Min(0),    // Main content
                ])
                .split(layout[0]);

            self.render_thread_summary(frame, summary_area[0], theme);
            self.render_hierarchy_view(frame, summary_area[1], theme, messages, selected_index);
        } else {
            self.render_hierarchy_view(frame, layout[0], theme, messages, selected_index);
        }

        // Render message preview
        self.render_message_preview(frame, layout[1], theme, messages, selected_index);

        // Render participants panel if enabled
        if self.show_participants_panel && layout.len() > 2 {
            self.render_participants_panel(frame, layout[2], theme);
        }
    }

    /// Render thread summary header
    fn render_thread_summary(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let summary_text = self.create_thread_summary_text(theme);
        
        let paragraph = Paragraph::new(summary_text)
            .block(Block::default()
                .title("Thread Overview")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.colors.palette.border)))
            .alignment(Alignment::Left)
            .wrap(ratatui::widgets::Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    /// Create thread summary text with statistics
    fn create_thread_summary_text(&self, theme: &Theme) -> Line {
        let stats = &self.thread_statistics;
        let mut spans = Vec::new();

        // Message count with icon
        spans.push(Span::styled("📧 ", Style::default().fg(theme.colors.palette.info)));
        spans.push(Span::styled(
            format!("{} msgs", stats.total_messages),
            Style::default().fg(theme.colors.palette.text_primary),
        ));

        // Unread count if any
        if stats.unread_count > 0 {
            spans.push(Span::raw(" • "));
            spans.push(Span::styled("● ", Style::default().fg(theme.colors.palette.warning)));
            spans.push(Span::styled(
                format!("{} unread", stats.unread_count),
                Style::default().fg(theme.colors.palette.warning).add_modifier(Modifier::BOLD),
            ));
        }

        // Participants
        spans.push(Span::raw(" • "));
        spans.push(Span::styled("👥 ", Style::default().fg(theme.colors.palette.accent)));
        spans.push(Span::styled(
            format!("{} participants", stats.participants_count),
            Style::default().fg(theme.colors.palette.text_secondary),
        ));

        // Important/attachments indicators
        if stats.has_important {
            spans.push(Span::raw(" • "));
            spans.push(Span::styled("🔴 Important", Style::default().fg(theme.colors.palette.error)));
        }

        if stats.has_attachments {
            spans.push(Span::raw(" • "));
            spans.push(Span::styled("📎 Files", Style::default().fg(theme.colors.palette.success)));
        }

        Line::from(spans)
    }

    /// Render the main hierarchy view based on current mode
    fn render_hierarchy_view(&self, frame: &mut Frame, area: Rect, theme: &Theme, messages: &[MessageItem], selected_index: usize) {
        match self.hierarchy_mode {
            HierarchyMode::Conversation => self.render_conversation_view(frame, area, theme, messages, selected_index),
            HierarchyMode::TreeStructure => self.render_tree_structure_view(frame, area, theme, messages, selected_index),
            HierarchyMode::Timeline => self.render_timeline_view(frame, area, theme, messages, selected_index),
            HierarchyMode::Compact => self.render_compact_view(frame, area, theme, messages, selected_index),
        }
    }

    /// Render conversation-style view (chat-like)
    fn render_conversation_view(&self, frame: &mut Frame, area: Rect, theme: &Theme, messages: &[MessageItem], selected_index: usize) {
        let mut list_items = Vec::new();
        let _context = ThreadContext::new();

        for (index, message) in messages.iter().enumerate() {
            let is_selected = index == selected_index;
            let is_current_user = self.is_current_user(&message.sender);
            
            // Create conversation bubble style
            let bubble_style = if is_current_user {
                Style::default()
                    .bg(theme.colors.palette.accent.into())
                    .fg(Color::White)
            } else {
                Style::default()
                    .bg(theme.colors.palette.surface.into())
                    .fg(theme.colors.palette.text_primary)
            };

            let alignment = if is_current_user { "→" } else { "←" };
            
            let mut spans = Vec::new();
            
            // Add alignment indicator
            spans.push(Span::styled(
                format!("{} ", alignment),
                Style::default().fg(if is_current_user { 
                    theme.colors.palette.success 
                } else { 
                    theme.colors.palette.info 
                }),
            ));

            // Sender name (abbreviated)
            let sender_name = message.sender.split('@').next().unwrap_or(&message.sender);
            spans.push(Span::styled(
                format!("{}: ", sender_name),
                bubble_style.add_modifier(Modifier::BOLD),
            ));

            // Message subject/preview
            spans.push(Span::styled(
                self.truncate_text(&message.subject, 40),
                bubble_style,
            ));

            // Time indicator
            spans.push(Span::styled(
                format!(" ({})", self.format_relative_time(&message.date)),
                Style::default().fg(theme.colors.palette.text_muted).add_modifier(Modifier::ITALIC),
            ));

            let item_style = if is_selected {
                Style::default().bg(theme.colors.palette.selection.into())
            } else {
                Style::default()
            };

            list_items.push(ListItem::new(Line::from(spans)).style(item_style));
        }

        let list = List::new(list_items)
            .block(Block::default()
                .title("💬 Conversation")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.colors.palette.border)))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol("• ");

        frame.render_widget(list, area);
    }

    /// Render tree structure view (traditional)
    fn render_tree_structure_view(&self, frame: &mut Frame, area: Rect, theme: &Theme, messages: &[MessageItem], selected_index: usize) {
        let mut list_items = Vec::new();
        let mut context = ThreadContext::new();

        // Build the threading context
        for (i, message) in messages.iter().enumerate() {
            if i < messages.len() - 1 {
                let next_message = &messages[i + 1];
                context.set_continuation_at_depth(
                    message.thread_depth,
                    next_message.thread_depth > message.thread_depth
                );
            }
        }

        for (index, message) in messages.iter().enumerate() {
            let is_selected = index == selected_index;
            let connection_type = self.determine_connection_type(message, messages, index);
            
            let prefix = self.threading_display.get_threading_prefix(message, connection_type, &context);
            
            let mut spans = prefix.spans;
            
            // Add status indicators
            self.add_status_indicators(&mut spans, message, theme);
            
            // Add subject
            spans.push(Span::styled(
                self.truncate_text(&message.subject, 50),
                if message.is_read {
                    Style::default().fg(theme.colors.palette.text_primary)
                } else {
                    Style::default().fg(theme.colors.palette.warning).add_modifier(Modifier::BOLD)
                }
            ));

            // Add sender info
            spans.push(Span::styled(
                format!(" - {}", self.format_sender_short(&message.sender)),
                Style::default().fg(theme.colors.palette.text_secondary),
            ));

            let item_style = if is_selected {
                Style::default().bg(theme.colors.palette.selection.into())
            } else {
                Style::default()
            };

            list_items.push(ListItem::new(Line::from(spans)).style(item_style));
        }

        let list = List::new(list_items)
            .block(Block::default()
                .title("🌳 Thread Structure")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.colors.palette.border)))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol("► ");

        frame.render_widget(list, area);
    }

    /// Render timeline view (chronological)
    fn render_timeline_view(&self, frame: &mut Frame, area: Rect, theme: &Theme, messages: &[MessageItem], selected_index: usize) {
        let mut list_items = Vec::new();

        for (index, message) in messages.iter().enumerate() {
            let is_selected = index == selected_index;
            let mut spans = Vec::new();

            // Timeline indicator
            let timeline_symbol = if index == 0 { "●" } else if index == messages.len() - 1 { "○" } else { "●" };
            spans.push(Span::styled(
                format!("{} ", timeline_symbol),
                Style::default().fg(theme.colors.palette.accent),
            ));

            // Time/date
            spans.push(Span::styled(
                format!("{} ", self.format_timeline_date(&message.date)),
                Style::default().fg(theme.colors.palette.text_muted).add_modifier(Modifier::ITALIC),
            ));

            // Sender
            spans.push(Span::styled(
                format!("{}: ", self.format_sender_short(&message.sender)),
                Style::default().fg(theme.colors.palette.text_secondary).add_modifier(Modifier::BOLD),
            ));

            // Subject
            spans.push(Span::styled(
                self.truncate_text(&message.subject, 45),
                if message.is_read {
                    Style::default().fg(theme.colors.palette.text_primary)
                } else {
                    Style::default().fg(theme.colors.palette.warning).add_modifier(Modifier::BOLD)
                }
            ));

            let item_style = if is_selected {
                Style::default().bg(theme.colors.palette.selection.into())
            } else {
                Style::default()
            };

            list_items.push(ListItem::new(Line::from(spans)).style(item_style));
        }

        let list = List::new(list_items)
            .block(Block::default()
                .title("⏰ Timeline")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.colors.palette.border)))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol("▶ ");

        frame.render_widget(list, area);
    }

    /// Render compact view (space-efficient)
    fn render_compact_view(&self, frame: &mut Frame, area: Rect, theme: &Theme, messages: &[MessageItem], selected_index: usize) {
        let mut list_items = Vec::new();

        for (index, message) in messages.iter().enumerate() {
            let is_selected = index == selected_index;
            let mut spans = Vec::new();

            // Compact depth indicator
            if message.thread_depth > 0 {
                spans.push(Span::styled(
                    "▸".repeat(message.thread_depth.min(3)),
                    Style::default().fg(theme.colors.palette.accent),
                ));
                spans.push(Span::raw(" "));
            }

            // Status dot
            let status_color = if !message.is_read {
                theme.colors.palette.warning
            } else if message.is_important {
                theme.colors.palette.error
            } else {
                theme.colors.palette.text_muted
            };
            
            spans.push(Span::styled("● ", Style::default().fg(status_color)));

            // Abbreviated content
            spans.push(Span::styled(
                format!("{}: {}", 
                    self.format_sender_short(&message.sender),
                    self.truncate_text(&message.subject, 35)
                ),
                Style::default().fg(theme.colors.palette.text_primary),
            ));

            let item_style = if is_selected {
                Style::default().bg(theme.colors.palette.selection.into())
            } else {
                Style::default()
            };

            list_items.push(ListItem::new(Line::from(spans)).style(item_style));
        }

        let list = List::new(list_items)
            .block(Block::default()
                .title("📋 Compact")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.colors.palette.border)))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol("→ ");

        frame.render_widget(list, area);
    }

    /// Render message preview panel
    fn render_message_preview(&self, frame: &mut Frame, area: Rect, theme: &Theme, messages: &[MessageItem], selected_index: usize) {
        if selected_index < messages.len() {
            let message = &messages[selected_index];
            let mut lines = Vec::new();

            // Message header
            lines.push(Line::from(vec![
                Span::styled("From: ", Style::default().fg(theme.colors.palette.text_muted)),
                Span::styled(message.sender.clone(), Style::default().fg(theme.colors.palette.text_primary).add_modifier(Modifier::BOLD)),
            ]));

            lines.push(Line::from(vec![
                Span::styled("Date: ", Style::default().fg(theme.colors.palette.text_muted)),
                Span::styled(message.date.clone(), Style::default().fg(theme.colors.palette.text_secondary)),
            ]));

            lines.push(Line::from(vec![
                Span::styled("Subject: ", Style::default().fg(theme.colors.palette.text_muted)),
                Span::styled(message.subject.clone(), Style::default().fg(theme.colors.palette.text_primary).add_modifier(Modifier::BOLD)),
            ]));

            lines.push(Line::from(""));

            // Message preview (this would typically come from email content)
            lines.push(Line::from(vec![
                Span::styled("[Message preview would appear here...]", 
                    Style::default().fg(theme.colors.palette.text_secondary).add_modifier(Modifier::ITALIC)),
            ]));

            let paragraph = Paragraph::new(lines)
                .block(Block::default()
                    .title("📖 Message Preview")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.colors.palette.border)))
                .wrap(ratatui::widgets::Wrap { trim: true });

            frame.render_widget(paragraph, area);
        }
    }

    /// Render participants panel
    fn render_participants_panel(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let mut list_items = Vec::new();

        for (_, participant) in self.participants.iter() {
            let mut spans = Vec::new();

            // User indicator
            let user_symbol = if participant.is_current_user { "👤" } else { "👥" };
            spans.push(Span::styled(
                format!("{} ", user_symbol),
                Style::default().fg(theme.colors.palette.accent),
            ));

            // Name
            spans.push(Span::styled(
                participant.display_name.clone(),
                Style::default().fg(theme.colors.palette.text_primary).add_modifier(Modifier::BOLD),
            ));

            // Message count
            spans.push(Span::styled(
                format!(" ({})", participant.message_count),
                Style::default().fg(theme.colors.palette.text_muted),
            ));

            list_items.push(ListItem::new(Line::from(spans)));
        }

        let list = List::new(list_items)
            .block(Block::default()
                .title("👥 Participants")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.colors.palette.border)));

        frame.render_widget(list, area);
    }

    // Helper methods

    fn analyze_thread_participants(&mut self, messages: &[MessageItem]) {
        self.participants.clear();

        for message in messages {
            let email = message.sender.clone();
            let display_name = self.format_sender_short(&email).to_string();
            let is_current_user = self.is_current_user(&email);

            self.participants.entry(email.clone()).and_modify(|p| {
                p.message_count += 1;
                p.last_message_date = message.date.clone();
            }).or_insert(ThreadParticipant {
                email: email.clone(),
                display_name,
                message_count: 1,
                is_current_user,
                last_message_date: message.date.clone(),
            });
        }
    }

    fn calculate_thread_statistics(&mut self, messages: &[MessageItem]) {
        self.thread_statistics = ThreadStatistics {
            total_messages: messages.len(),
            unread_count: messages.iter().filter(|m| !m.is_read).count(),
            participants_count: self.participants.len(),
            date_range: self.get_date_range(messages),
            has_attachments: messages.iter().any(|m| m.has_attachments),
            has_important: messages.iter().any(|m| m.is_important),
        };
    }

    fn get_date_range(&self, messages: &[MessageItem]) -> (String, String) {
        if messages.is_empty() {
            return ("".to_string(), "".to_string());
        }
        
        // This is a simplified implementation - would need proper date parsing
        (messages.last().unwrap().date.clone(), messages.first().unwrap().date.clone())
    }

    fn determine_connection_type(&self, message: &MessageItem, messages: &[MessageItem], index: usize) -> ConnectionType {
        if message.thread_depth == 0 {
            ConnectionType::Root
        } else if index == messages.len() - 1 {
            ConnectionType::LastChild
        } else {
            let next_message = &messages[index + 1];
            if next_message.thread_depth <= message.thread_depth {
                ConnectionType::LastChild
            } else {
                ConnectionType::ChildContinue
            }
        }
    }

    fn add_status_indicators(&self, spans: &mut Vec<Span>, message: &MessageItem, theme: &Theme) {
        if !message.is_read {
            spans.push(Span::styled("● ", Style::default().fg(theme.colors.palette.warning)));
        }
        if message.is_important {
            spans.push(Span::styled("🔴 ", Style::default()));
        }
        if message.has_attachments {
            spans.push(Span::styled("📎 ", Style::default().fg(theme.colors.palette.success)));
        }
    }

    fn is_current_user(&self, email: &str) -> bool {
        // This would typically check against current user's email addresses
        // For now, simplified implementation
        email.contains("me@") || email.contains("current@")
    }

    fn format_sender_short<'a>(&self, sender: &'a str) -> &'a str {
        sender.split('@').next().unwrap_or(sender)
    }

    fn format_relative_time(&self, _date: &str) -> String {
        // Simplified - would need proper date parsing and relative formatting
        "2h ago".to_string()
    }

    fn format_timeline_date(&self, date: &str) -> String {
        // Simplified - would format as "14:23" or "Mar 15" etc.
        date.split(' ').next().unwrap_or(date).to_string()
    }

    fn truncate_text(&self, text: &str, max_len: usize) -> String {
        if text.len() > max_len {
            format!("{}…", &text[..max_len.saturating_sub(1)])
        } else {
            text.to_string()
        }
    }

    /// Toggle hierarchy mode
    pub fn toggle_mode(&mut self) {
        self.hierarchy_mode = match self.hierarchy_mode {
            HierarchyMode::TreeStructure => HierarchyMode::Conversation,
            HierarchyMode::Conversation => HierarchyMode::Timeline,
            HierarchyMode::Timeline => HierarchyMode::Compact,
            HierarchyMode::Compact => HierarchyMode::TreeStructure,
        };
    }

    /// Get current mode name for UI display
    pub fn current_mode_name(&self) -> &'static str {
        match self.hierarchy_mode {
            HierarchyMode::Conversation => "Conversation",
            HierarchyMode::TreeStructure => "Tree",
            HierarchyMode::Timeline => "Timeline",
            HierarchyMode::Compact => "Compact",
        }
    }
}

impl Default for ThreadHierarchyView {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thread_hierarchy_view_creation() {
        let view = ThreadHierarchyView::new();
        assert_eq!(view.hierarchy_mode, HierarchyMode::TreeStructure);
        assert!(view.show_participants_panel);
    }

    #[test]
    fn test_mode_toggling() {
        let mut view = ThreadHierarchyView::new();
        assert_eq!(view.hierarchy_mode, HierarchyMode::TreeStructure);
        
        view.toggle_mode();
        assert_eq!(view.hierarchy_mode, HierarchyMode::Conversation);
        
        view.toggle_mode();
        assert_eq!(view.hierarchy_mode, HierarchyMode::Timeline);
    }

    #[test]
    fn test_text_truncation() {
        let view = ThreadHierarchyView::new();
        let long_text = "This is a very long text that should be truncated";
        let truncated = view.truncate_text(long_text, 10);
        assert!(truncated.len() <= 10);
        assert!(truncated.ends_with('…'));
    }
}