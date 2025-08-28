//! Enhanced email threading display system with improved visual hierarchy
//!
//! This module provides sophisticated visual representation of email threads,
//! including better indentation, connection lines, threading indicators, and
//! improved readability through visual hierarchy.

use crate::theme::Theme;
use crate::ui::message_list::MessageItem;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use std::collections::HashMap;

/// Threading display style options
#[derive(Debug, Clone, PartialEq)]
pub enum ThreadingStyle {
    Minimal,     // Simple indentation
    Lines,       // Tree-like lines connecting messages
    Modern,      // Modern UI with rounded corners and gradients
    Compact,     // Space-efficient representation
}

/// Thread connection types for visual continuity
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionType {
    Root,           // Thread root message
    Continue,       // Message in middle of thread
    LastChild,      // Last message in thread branch
    ChildContinue,  // Child message with more siblings
}

/// Enhanced threading display manager
pub struct ThreadingDisplay {
    style: ThreadingStyle,
    max_depth: usize,
    show_thread_stats: bool,
    use_unicode_symbols: bool,
    color_by_depth: bool,
    depth_colors: Vec<Color>,
}

impl ThreadingDisplay {
    /// Create a new threading display system
    pub fn new() -> Self {
        Self {
            style: ThreadingStyle::Modern,
            max_depth: 10,
            show_thread_stats: true,
            use_unicode_symbols: true,
            color_by_depth: true,
            depth_colors: vec![
                Color::Cyan,
                Color::Yellow,
                Color::Green,
                Color::Magenta,
                Color::Blue,
                Color::Red,
            ],
        }
    }

    /// Set the threading display style
    pub fn with_style(mut self, style: ThreadingStyle) -> Self {
        self.style = style;
        self
    }

    /// Enable/disable thread statistics display
    pub fn with_thread_stats(mut self, show: bool) -> Self {
        self.show_thread_stats = show;
        self
    }

    /// Set maximum threading depth
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }
    
    /// Enable/disable unicode symbols for threading display
    pub fn with_unicode_symbols(mut self, use_unicode: bool) -> Self {
        self.use_unicode_symbols = use_unicode;
        self
    }
    
    /// Check if unicode symbols are enabled
    pub fn unicode_symbols_enabled(&self) -> bool {
        self.use_unicode_symbols
    }

    /// Enable/disable color coding by depth
    pub fn with_color_by_depth(mut self, enabled: bool) -> Self {
        self.color_by_depth = enabled;
        self
    }

    /// Generate threading prefix for a message
    pub fn get_threading_prefix(
        &self,
        message: &MessageItem,
        connection_type: ConnectionType,
        thread_context: &ThreadContext,
    ) -> ThreadingPrefix {
        match self.style {
            ThreadingStyle::Minimal => self.get_minimal_prefix(message),
            ThreadingStyle::Lines => self.get_lines_prefix(message, connection_type, thread_context),
            ThreadingStyle::Modern => self.get_modern_prefix(message, connection_type, thread_context),
            ThreadingStyle::Compact => self.get_compact_prefix(message, connection_type),
        }
    }

    /// Generate minimal threading prefix (simple indentation)
    fn get_minimal_prefix(&self, message: &MessageItem) -> ThreadingPrefix {
        let mut spans = Vec::new();
        
        // Simple indentation
        if message.thread_depth > 0 {
            let indent_width = message.thread_depth * 2;
            spans.push(Span::raw(" ".repeat(indent_width)));
            
            if message.is_thread_root {
                spans.push(Span::raw("▸ "));
            } else {
                spans.push(Span::raw("• "));
            }
        } else if message.is_thread_root && message.message_count > 1 {
            if message.is_thread_expanded {
                spans.push(Span::raw("▾ "));
            } else {
                spans.push(Span::raw("▸ "));
            }
        }

        let total_width = spans.iter().map(|s| s.content.len()).sum();
        ThreadingPrefix {
            spans,
            total_width,
        }
    }

    /// Generate lines-based threading prefix (tree structure)
    fn get_lines_prefix(
        &self,
        message: &MessageItem,
        connection_type: ConnectionType,
        thread_context: &ThreadContext,
    ) -> ThreadingPrefix {
        let mut spans = Vec::new();
        
        if message.thread_depth == 0 {
            // Root message
            if message.message_count > 1 {
                let symbol = if message.is_thread_expanded { "─┐" } else { "─► " };
                spans.push(Span::styled(symbol, Style::default().fg(Color::Cyan)));
                
                if self.show_thread_stats {
                    spans.push(Span::styled(
                        format!(" ({})", message.message_count),
                        Style::default().fg(Color::Gray).add_modifier(Modifier::DIM),
                    ));
                }
            }
        } else {
            // Build the connection line
            for depth in 0..message.thread_depth {
                let color = if self.color_by_depth {
                    self.depth_colors[depth % self.depth_colors.len()]
                } else {
                    Color::Gray
                };
                
                if depth < message.thread_depth - 1 {
                    // Intermediate levels - show connection or space
                    if thread_context.has_continuation_at_depth(depth) {
                        spans.push(Span::styled("│ ", Style::default().fg(color)));
                    } else {
                        spans.push(Span::raw("  "));
                    }
                } else {
                    // Final level - show the connection type
                    let symbol = match connection_type {
                        ConnectionType::Continue => "├─",
                        ConnectionType::LastChild => "└─",
                        ConnectionType::ChildContinue => "├─",
                        ConnectionType::Root => "──",
                    };
                    spans.push(Span::styled(format!("{} ", symbol), Style::default().fg(color)));
                }
            }
        }

        let total_width = spans.iter().map(|s| s.content.chars().count()).sum();
        ThreadingPrefix {
            spans,
            total_width,
        }
    }

    /// Generate modern threading prefix (rounded corners, better aesthetics)
    fn get_modern_prefix(
        &self,
        message: &MessageItem,
        connection_type: ConnectionType,
        thread_context: &ThreadContext,
    ) -> ThreadingPrefix {
        let mut spans = Vec::new();
        
        if message.thread_depth == 0 {
            // Root message with modern styling
            if message.message_count > 1 {
                let (symbol, color) = if message.is_thread_expanded {
                    ("◣ ", Color::Green)
                } else {
                    ("◥ ", Color::Blue)
                };
                spans.push(Span::styled(symbol, Style::default().fg(color).add_modifier(Modifier::BOLD)));
                
                if self.show_thread_stats {
                    spans.push(Span::styled(
                        format!("{}msgs ", message.message_count),
                        Style::default().fg(Color::Gray).add_modifier(Modifier::ITALIC),
                    ));
                }
            }
        } else {
            // Modern connection visualization with rounded elements
            for depth in 0..message.thread_depth {
                let color = if self.color_by_depth {
                    self.depth_colors[depth % self.depth_colors.len()]
                } else {
                    Color::Rgb(100 + depth as u8 * 30, 100 + depth as u8 * 20, 150)
                };
                
                if depth < message.thread_depth - 1 {
                    if thread_context.has_continuation_at_depth(depth) {
                        spans.push(Span::styled("┃ ", Style::default().fg(color)));
                    } else {
                        spans.push(Span::raw("  "));
                    }
                } else {
                    let symbol = match connection_type {
                        ConnectionType::Continue => "┠─",
                        ConnectionType::LastChild => "┖─",
                        ConnectionType::ChildContinue => "┠─",
                        ConnectionType::Root => "━━",
                    };
                    spans.push(Span::styled(format!("{} ", symbol), Style::default().fg(color)));
                }
            }
        }

        let total_width = spans.iter().map(|s| s.content.chars().count()).sum();
        ThreadingPrefix {
            spans,
            total_width,
        }
    }

    /// Generate compact threading prefix (space-efficient)
    fn get_compact_prefix(&self, message: &MessageItem, _connection_type: ConnectionType) -> ThreadingPrefix {
        let mut spans = Vec::new();
        
        if message.thread_depth > 0 {
            // Use minimal space with just depth indicator
            let depth_char = match message.thread_depth {
                1 => "▫",
                2 => "▪",
                3 => "◆",
                4 => "◇",
                5 => "●",
                _ => "○",
            };
            
            let color = if self.color_by_depth {
                self.depth_colors[(message.thread_depth - 1) % self.depth_colors.len()]
            } else {
                Color::Gray
            };
            
            spans.push(Span::styled(format!("{} ", depth_char), Style::default().fg(color)));
        } else if message.is_thread_root && message.message_count > 1 {
            let symbol = if message.is_thread_expanded { "▽" } else { "△" };
            spans.push(Span::styled(format!("{} ", symbol), Style::default().fg(Color::Cyan)));
        }

        let total_width = spans.iter().map(|s| s.content.chars().count()).sum();
        ThreadingPrefix {
            spans,
            total_width,
        }
    }

    /// Create enhanced thread header for root messages
    pub fn create_thread_header(
        &self,
        message: &MessageItem,
        theme: &Theme,
        show_stats: bool,
    ) -> Line {
        let mut spans = Vec::new();
        
        if message.is_thread_root && message.message_count > 1 {
            // Thread indicator
            let thread_symbol = if message.is_thread_expanded { "📂" } else { "📁" };
            spans.push(Span::styled(
                format!("{} ", thread_symbol),
                Style::default().fg(theme.colors.palette.accent),
            ));
            
            // Thread title
            spans.push(Span::styled(
                "Thread: ",
                Style::default().fg(theme.colors.palette.text_muted),
            ));
            
            // Subject
            spans.push(Span::styled(
                message.subject.clone(),
                Style::default()
                    .fg(theme.colors.palette.text_primary)
                    .add_modifier(Modifier::BOLD),
            ));
            
            if show_stats {
                // Statistics
                spans.push(Span::styled(
                    format!(" ({} messages)", message.message_count),
                    Style::default()
                        .fg(theme.colors.palette.text_secondary)
                        .add_modifier(Modifier::ITALIC),
                ));
                
                // Unread count if any
                // This would need to be calculated by the caller
                spans.push(Span::styled(
                    " • Active conversation",
                    Style::default().fg(theme.colors.palette.success),
                ));
            }
        }
        
        Line::from(spans)
    }

    /// Get thread summary for collapsed threads
    pub fn get_thread_summary(
        &self,
        root_message: &MessageItem,
        recent_senders: &[String],
        theme: &Theme,
    ) -> Line {
        let mut spans = Vec::new();
        
        // Summary prefix
        spans.push(Span::styled(
            "↳ ",
            Style::default().fg(theme.colors.palette.text_muted),
        ));
        
        // Recent participants
        let participants = if recent_senders.len() > 3 {
            format!("{}, {} and {} others", 
                recent_senders[0], 
                recent_senders[1], 
                recent_senders.len() - 2)
        } else {
            recent_senders.join(", ")
        };
        
        spans.push(Span::styled(
            participants,
            Style::default().fg(theme.colors.palette.text_secondary),
        ));
        
        // Message count
        spans.push(Span::styled(
            format!(" • {} messages", root_message.message_count),
            Style::default()
                .fg(theme.colors.palette.text_muted)
                .add_modifier(Modifier::ITALIC),
        ));
        
        Line::from(spans)
    }
}

/// Threading prefix result
#[derive(Debug, Clone)]
pub struct ThreadingPrefix {
    pub spans: Vec<Span<'static>>,
    pub total_width: usize,
}

/// Thread context for determining connection types
#[derive(Debug, Clone)]
pub struct ThreadContext {
    /// Tracks which depths have continuation lines
    continuations: HashMap<usize, bool>,
}

impl ThreadContext {
    pub fn new() -> Self {
        Self {
            continuations: HashMap::new(),
        }
    }

    pub fn set_continuation_at_depth(&mut self, depth: usize, has_continuation: bool) {
        self.continuations.insert(depth, has_continuation);
    }

    pub fn has_continuation_at_depth(&self, depth: usize) -> bool {
        self.continuations.get(&depth).unwrap_or(&false).clone()
    }
}

impl Default for ThreadingDisplay {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ThreadContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threading_display_creation() {
        let display = ThreadingDisplay::new();
        assert_eq!(display.style, ThreadingStyle::Modern);
        assert!(display.show_thread_stats);
    }

    #[test]
    fn test_minimal_prefix() {
        let display = ThreadingDisplay::new().with_style(ThreadingStyle::Minimal);
        let mut message = MessageItem::new("Test".to_string(), "sender".to_string(), "date".to_string());
        message.thread_depth = 1;
        
        let prefix = display.get_minimal_prefix(&message);
        assert!(!prefix.spans.is_empty());
        assert!(prefix.total_width > 0);
    }

    #[test]
    fn test_thread_context() {
        let mut context = ThreadContext::new();
        context.set_continuation_at_depth(0, true);
        assert!(context.has_continuation_at_depth(0));
        assert!(!context.has_continuation_at_depth(1));
    }
}