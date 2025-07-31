/// Modern toast notification system for user feedback
/// 
/// Provides non-intrusive, temporary notifications that appear at the top-right
/// of the screen and automatically dismiss after a configurable duration.

use crate::tea::message::ToastLevel;
use crate::theme::Theme;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use std::collections::VecDeque;
use tokio::time::{Duration, Instant};
use uuid::Uuid;

/// Maximum number of toasts to display simultaneously
const MAX_VISIBLE_TOASTS: usize = 5;

/// Toast notification item
#[derive(Debug, Clone)]
pub struct Toast {
    pub id: String,
    pub message: String,
    pub level: ToastLevel,
    pub created_at: Instant,
    pub duration: Duration,
    pub progress: f64, // 0.0 to 1.0 for fade animation
}

impl Toast {
    /// Create a new toast notification
    pub fn new(message: String, level: ToastLevel) -> Self {
        let duration = match level {
            ToastLevel::Info => Duration::from_secs(3),
            ToastLevel::Success => Duration::from_secs(2),
            ToastLevel::Warning => Duration::from_secs(4),
            ToastLevel::Error => Duration::from_secs(5),
        };

        Self {
            id: Uuid::new_v4().to_string(),
            message,
            level,
            created_at: Instant::now(),
            duration,
            progress: 0.0,
        }
    }

    /// Create a toast with custom duration
    pub fn with_duration(message: String, level: ToastLevel, duration: Duration) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            message,
            level,
            created_at: Instant::now(),
            duration,
            progress: 0.0,
        }
    }

    /// Check if toast has expired
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= self.duration
    }

    /// Get remaining time as percentage (1.0 = full time, 0.0 = expired)
    pub fn remaining_percentage(&self) -> f64 {
        let elapsed = self.created_at.elapsed();
        if elapsed >= self.duration {
            0.0
        } else {
            1.0 - (elapsed.as_secs_f64() / self.duration.as_secs_f64())
        }
    }

    /// Update animation progress
    pub fn update_progress(&mut self) {
        self.progress = 1.0 - self.remaining_percentage();
    }

    /// Get toast icon based on level
    pub fn icon(&self) -> &'static str {
        match self.level {
            ToastLevel::Info => "ℹ",
            ToastLevel::Success => "✓",
            ToastLevel::Warning => "⚠",
            ToastLevel::Error => "✗",
        }
    }

    /// Get toast colors based on level and theme
    pub fn colors(&self, theme: &Theme) -> (Color, Color, Color) {
        match self.level {
            ToastLevel::Info => (
                theme.colors.palette.info,
                theme.colors.palette.background,
                theme.colors.palette.surface,
            ),
            ToastLevel::Success => (
                theme.colors.palette.success,
                theme.colors.palette.background,
                theme.colors.palette.surface,
            ),
            ToastLevel::Warning => (
                theme.colors.palette.warning,
                theme.colors.palette.background,
                theme.colors.palette.surface,
            ),
            ToastLevel::Error => (
                theme.colors.palette.error,
                theme.colors.palette.background,
                theme.colors.palette.surface,
            ),
        }
    }
}

/// Toast notification manager
#[derive(Debug)]
pub struct ToastManager {
    toasts: VecDeque<Toast>,
    max_visible: usize,
}

impl Default for ToastManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ToastManager {
    /// Create a new toast manager
    pub fn new() -> Self {
        Self {
            toasts: VecDeque::new(),
            max_visible: MAX_VISIBLE_TOASTS,
        }
    }

    /// Add a new toast notification
    pub fn add_toast(&mut self, toast: Toast) {
        // Remove oldest toast if at capacity
        if self.toasts.len() >= self.max_visible {
            self.toasts.pop_front();
        }
        
        self.toasts.push_back(toast);
    }

    /// Add a simple toast with message and level
    pub fn show(&mut self, message: String, level: ToastLevel) {
        let toast = Toast::new(message, level);
        self.add_toast(toast);
    }

    /// Add a toast with custom duration
    pub fn show_with_duration(&mut self, message: String, level: ToastLevel, duration: Duration) {
        let toast = Toast::with_duration(message, level, duration);
        self.add_toast(toast);
    }

    /// Remove a specific toast by ID
    pub fn remove_toast(&mut self, toast_id: &str) {
        self.toasts.retain(|toast| toast.id != toast_id);
    }

    /// Update all toasts and remove expired ones
    pub fn update(&mut self) {
        // Update progress for all toasts
        for toast in &mut self.toasts {
            toast.update_progress();
        }

        // Remove expired toasts
        self.toasts.retain(|toast| !toast.is_expired());
    }

    /// Get current toasts
    pub fn toasts(&self) -> &VecDeque<Toast> {
        &self.toasts
    }

    /// Check if there are any active toasts
    pub fn has_toasts(&self) -> bool {
        !self.toasts.is_empty()
    }

    /// Clear all toasts
    pub fn clear(&mut self) {
        self.toasts.clear();
    }

    /// Set maximum number of visible toasts
    pub fn set_max_visible(&mut self, max: usize) {
        self.max_visible = max;
        // Trim excess toasts if necessary
        while self.toasts.len() > self.max_visible {
            self.toasts.pop_front();
        }
    }
}

/// Toast renderer for displaying notifications
pub struct ToastRenderer;

impl ToastRenderer {
    /// Render toast notifications in the top-right corner
    pub fn render(frame: &mut Frame, area: Rect, toasts: &VecDeque<Toast>, theme: &Theme) {
        if toasts.is_empty() {
            return;
        }

        // Calculate toast area (top-right corner)
        let toast_width = area.width.min(50); // Max 50 chars wide
        let toast_area = Rect {
            x: area.width.saturating_sub(toast_width).saturating_sub(2),
            y: 1, // Leave space for status bar if at top
            width: toast_width,
            height: area.height.saturating_sub(2),
        };

        // Render each toast from newest to oldest (bottom to top)
        let mut current_y = toast_area.y;
        let toast_height = 4; // Height per toast (including borders)

        for (index, toast) in toasts.iter().rev().enumerate() {
            if current_y + toast_height > toast_area.y + toast_area.height {
                break; // No more space
            }

            let individual_toast_area = Rect {
                x: toast_area.x,
                y: current_y,
                width: toast_area.width,
                height: toast_height,
            };

            Self::render_individual_toast(frame, individual_toast_area, toast, theme, index);
            current_y += toast_height + 1; // Add spacing between toasts
        }
    }

    /// Render an individual toast notification
    fn render_individual_toast(
        frame: &mut Frame,
        area: Rect,
        toast: &Toast,
        theme: &Theme,
        _index: usize,
    ) {
        // Clear the area first for proper overlay
        frame.render_widget(Clear, area);

        let (accent_color, text_color, bg_color) = toast.colors(theme);

        // Create the main toast block
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(accent_color))
            .style(Style::default().bg(bg_color));

        // Split into icon and content areas
        let inner_area = block.inner(area);
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(3), // Icon area
                Constraint::Min(0),    // Content area
            ])
            .split(inner_area);

        // Render the block background
        frame.render_widget(block, area);

        // Render icon
        if let Some(icon_area) = chunks.get(0) {
            let icon_paragraph = Paragraph::new(toast.icon())
                .style(Style::default().fg(accent_color).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Center);
            frame.render_widget(icon_paragraph, *icon_area);
        }

        // Render message content
        if let Some(content_area) = chunks.get(1) {
            let message_lines: Vec<Line> = toast
                .message
                .lines()
                .map(|line| Line::from(Span::styled(line, Style::default().fg(text_color))))
                .collect();

            let content_paragraph = Paragraph::new(message_lines)
                .wrap(Wrap { trim: true })
                .alignment(Alignment::Left);

            frame.render_widget(content_paragraph, *content_area);
        }

        // Render progress bar at the bottom
        Self::render_progress_bar(frame, area, toast, theme);
    }

    /// Render progress bar showing remaining time
    fn render_progress_bar(frame: &mut Frame, area: Rect, toast: &Toast, theme: &Theme) {
        let progress_area = Rect {
            x: area.x + 1,
            y: area.y + area.height - 1,
            width: area.width.saturating_sub(2),
            height: 1,
        };

        let remaining = toast.remaining_percentage();
        let filled_width = ((progress_area.width as f64) * remaining) as u16;

        let (accent_color, _, _) = toast.colors(theme);

        // Create progress bar content
        let progress_content = if filled_width > 0 {
            "█".repeat(filled_width as usize)
        } else {
            String::new()
        };

        let progress_paragraph = Paragraph::new(progress_content)
            .style(Style::default().fg(accent_color));

        frame.render_widget(progress_paragraph, progress_area);
    }

    /// Calculate the total height needed for all toasts
    pub fn calculate_required_height(toast_count: usize) -> u16 {
        if toast_count == 0 {
            0
        } else {
            (toast_count as u16 * 4) + (toast_count.saturating_sub(1) as u16) // 4 per toast + spacing
        }
    }
}

/// Convenience functions for common toast types
impl ToastManager {
    /// Show an info toast
    pub fn info<S: Into<String>>(&mut self, message: S) {
        self.show(message.into(), ToastLevel::Info);
    }

    /// Show a success toast
    pub fn success<S: Into<String>>(&mut self, message: S) {
        self.show(message.into(), ToastLevel::Success);
    }

    /// Show a warning toast
    pub fn warning<S: Into<String>>(&mut self, message: S) {
        self.show(message.into(), ToastLevel::Warning);
    }

    /// Show an error toast
    pub fn error<S: Into<String>>(&mut self, message: S) {
        self.show(message.into(), ToastLevel::Error);
    }

    /// Show a persistent toast (longer duration)
    pub fn persistent<S: Into<String>>(&mut self, message: S, level: ToastLevel) {
        self.show_with_duration(message.into(), level, Duration::from_secs(10));
    }

    /// Show a quick toast (shorter duration)
    pub fn quick<S: Into<String>>(&mut self, message: S, level: ToastLevel) {
        self.show_with_duration(message.into(), level, Duration::from_secs(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toast_creation() {
        let toast = Toast::new("Test message".to_string(), ToastLevel::Info);
        assert_eq!(toast.message, "Test message");
        assert!(matches!(toast.level, ToastLevel::Info));
        assert!(!toast.is_expired());
    }

    #[test]
    fn test_toast_manager() {
        let mut manager = ToastManager::new();
        assert!(!manager.has_toasts());

        manager.info("Test info");
        assert!(manager.has_toasts());
        assert_eq!(manager.toasts().len(), 1);

        manager.success("Test success");
        manager.warning("Test warning");
        manager.error("Test error");
        assert_eq!(manager.toasts().len(), 4);

        manager.clear();
        assert!(!manager.has_toasts());
    }

    #[test]
    fn test_max_visible_toasts() {
        let mut manager = ToastManager::new();
        manager.set_max_visible(2);

        manager.info("Toast 1");
        manager.info("Toast 2");
        manager.info("Toast 3"); // Should remove Toast 1

        assert_eq!(manager.toasts().len(), 2);
        assert_eq!(manager.toasts()[0].message, "Toast 2");
        assert_eq!(manager.toasts()[1].message, "Toast 3");
    }

    #[test]
    fn test_toast_progress() {
        let mut toast = Toast::with_duration(
            "Test".to_string(),
            ToastLevel::Info,
            Duration::from_secs(2),
        );

        assert_eq!(toast.remaining_percentage(), 1.0);
        
        // Simulate some time passing
        std::thread::sleep(Duration::from_millis(100));
        toast.update_progress();
        assert!(toast.remaining_percentage() < 1.0);
        assert!(toast.remaining_percentage() > 0.8);
    }
}