//! Unified Feedback System
//! 
//! Provides a comprehensive, centralized system for managing all user feedback
//! including toasts, notifications, alerts, progress indicators, and status messages.
//! This system consolidates the various feedback mechanisms into a cohesive experience.

use crate::tea::message::ToastLevel;
use crate::theme::Theme;
use crate::ui::toast::{Toast, ToastManager};
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Clear, Gauge, Paragraph, Wrap},
    Frame,
};
use std::collections::{HashMap, VecDeque};
use tokio::time::{Duration, Instant};
use uuid::Uuid;

/// Maximum number of feedback items to keep in history
const MAX_FEEDBACK_HISTORY: usize = 50;

/// Maximum concurrent status indicators
const MAX_STATUS_INDICATORS: usize = 3;

/// Unified feedback level that extends ToastLevel with additional types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedbackLevel {
    /// Informational message
    Info,
    /// Success confirmation
    Success,
    /// Warning that needs attention
    Warning,
    /// Error that requires action
    Error,
    /// Critical system alert
    Critical,
    /// Progress update
    Progress,
    /// System status change
    Status,
    /// Debug information (only shown in debug mode)
    Debug,
}

impl From<ToastLevel> for FeedbackLevel {
    fn from(level: ToastLevel) -> Self {
        match level {
            ToastLevel::Info => FeedbackLevel::Info,
            ToastLevel::Success => FeedbackLevel::Success,
            ToastLevel::Warning => FeedbackLevel::Warning,
            ToastLevel::Error => FeedbackLevel::Error,
        }
    }
}

impl From<FeedbackLevel> for ToastLevel {
    fn from(level: FeedbackLevel) -> Self {
        match level {
            FeedbackLevel::Info | FeedbackLevel::Status | FeedbackLevel::Debug => ToastLevel::Info,
            FeedbackLevel::Success => ToastLevel::Success,
            FeedbackLevel::Warning | FeedbackLevel::Progress => ToastLevel::Warning,
            FeedbackLevel::Error | FeedbackLevel::Critical => ToastLevel::Error,
        }
    }
}

/// Feedback presentation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedbackMode {
    /// Show as toast notification (default)
    Toast,
    /// Show in status bar only
    StatusBar,
    /// Show as modal dialog (for critical messages)
    Modal,
    /// Show as progress indicator
    Progress,
    /// Show as inline message in current context
    Inline,
    /// Silent (logged only, no UI display)
    Silent,
}

/// Feedback context for intelligent routing
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FeedbackContext {
    /// General application feedback
    Application,
    /// Email-related feedback
    Email,
    /// Calendar-related feedback
    Calendar,
    /// Settings and configuration feedback
    Settings,
    /// AI assistant feedback
    AI,
    /// Network and sync feedback
    Network,
    /// User input and validation feedback
    Input,
    /// System and performance feedback
    System,
}

impl FeedbackContext {
    /// Create user action context
    pub fn user_action(mode: crate::ui::UIMode, focused_pane: crate::ui::FocusedPane) -> Self {
        match mode {
            crate::ui::UIMode::Compose => Self::Email,
            crate::ui::UIMode::Calendar => Self::Calendar,
            crate::ui::UIMode::Settings => Self::Settings,
            _ => match focused_pane {
                crate::ui::FocusedPane::MessageList | crate::ui::FocusedPane::ContentPreview => Self::Email,
                _ => Self::Application,
            }
        }
    }

    /// Create system status context
    pub fn system_status(_mode: crate::ui::UIMode) -> Self {
        Self::System
    }

    /// Create system error context
    pub fn system_error(_mode: crate::ui::UIMode) -> Self {
        Self::System
    }
}

/// Feedback item with rich metadata
#[derive(Debug, Clone)]
pub struct FeedbackItem {
    pub id: String,
    pub message: String,
    pub level: FeedbackLevel,
    pub context: FeedbackContext,
    pub mode: FeedbackMode,
    pub created_at: Instant,
    pub expires_at: Option<Instant>,
    pub progress: Option<f64>, // 0.0 to 1.0 for progress indicators
    pub action_hint: Option<String>, // Hint about user action to resolve
    pub is_persistent: bool,
    pub is_dismissible: bool,
    pub metadata: HashMap<String, String>,
}

impl FeedbackItem {
    /// Create a new feedback item
    pub fn new<S: Into<String>>(
        message: S, 
        level: FeedbackLevel, 
        context: FeedbackContext
    ) -> Self {
        let duration = Self::default_duration(level);
        let expires_at = if level == FeedbackLevel::Critical {
            None // Critical messages don't auto-expire
        } else {
            Some(Instant::now() + duration)
        };

        Self {
            id: Uuid::new_v4().to_string(),
            message: message.into(),
            level,
            context,
            mode: Self::default_mode(level),
            created_at: Instant::now(),
            expires_at,
            progress: None,
            action_hint: None,
            is_persistent: level == FeedbackLevel::Critical,
            is_dismissible: true,
            metadata: HashMap::new(),
        }
    }

    /// Create a progress feedback item
    pub fn progress<S: Into<String>>(
        message: S,
        progress: f64,
        context: FeedbackContext
    ) -> Self {
        let mut item = Self::new(message, FeedbackLevel::Progress, context);
        item.progress = Some(progress.clamp(0.0, 1.0));
        item.mode = FeedbackMode::Progress;
        item.expires_at = None; // Progress items don't auto-expire
        item.is_dismissible = false;
        item
    }

    /// Create a persistent feedback item that doesn't auto-dismiss
    pub fn persistent<S: Into<String>>(
        message: S,
        level: FeedbackLevel,
        context: FeedbackContext
    ) -> Self {
        let mut item = Self::new(message, level, context);
        item.is_persistent = true;
        item.expires_at = None;
        item
    }

    /// Create a modal feedback item for critical messages
    pub fn modal<S: Into<String>>(
        message: S,
        level: FeedbackLevel,
        context: FeedbackContext
    ) -> Self {
        let mut item = Self::new(message, level, context);
        item.mode = FeedbackMode::Modal;
        item.is_persistent = true;
        item.expires_at = None;
        item
    }

    /// Add action hint for user guidance
    pub fn with_action_hint<S: Into<String>>(mut self, hint: S) -> Self {
        self.action_hint = Some(hint.into());
        self
    }

    /// Add custom metadata
    pub fn with_metadata<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Set custom presentation mode
    pub fn with_mode(mut self, mode: FeedbackMode) -> Self {
        self.mode = mode;
        self
    }

    /// Check if item has expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Instant::now() >= expires_at
        } else {
            false
        }
    }

    /// Get default duration for feedback level
    fn default_duration(level: FeedbackLevel) -> Duration {
        match level {
            FeedbackLevel::Info | FeedbackLevel::Debug => Duration::from_secs(3),
            FeedbackLevel::Success => Duration::from_secs(2),
            FeedbackLevel::Warning => Duration::from_secs(4),
            FeedbackLevel::Error => Duration::from_secs(6),
            FeedbackLevel::Critical => Duration::from_secs(0), // Persistent
            FeedbackLevel::Progress => Duration::from_secs(0), // Controlled externally
            FeedbackLevel::Status => Duration::from_secs(5),
        }
    }

    /// Get default presentation mode for feedback level
    fn default_mode(level: FeedbackLevel) -> FeedbackMode {
        match level {
            FeedbackLevel::Critical => FeedbackMode::Modal,
            FeedbackLevel::Progress => FeedbackMode::Progress,
            FeedbackLevel::Status => FeedbackMode::StatusBar,
            FeedbackLevel::Debug => FeedbackMode::Silent,
            _ => FeedbackMode::Toast,
        }
    }

    /// Get icon for feedback level
    pub fn icon(&self) -> &'static str {
        match self.level {
            FeedbackLevel::Info => "ℹ",
            FeedbackLevel::Success => "✓",
            FeedbackLevel::Warning => "⚠",
            FeedbackLevel::Error => "✗",
            FeedbackLevel::Critical => "🚨",
            FeedbackLevel::Progress => "⏳",
            FeedbackLevel::Status => "●",
            FeedbackLevel::Debug => "🐛",
        }
    }

    /// Get colors for feedback level
    pub fn colors(&self, theme: &Theme) -> (Color, Color, Color) {
        match self.level {
            FeedbackLevel::Info => (
                theme.colors.palette.info,
                theme.colors.palette.background,
                theme.colors.palette.text_primary,
            ),
            FeedbackLevel::Success => (
                theme.colors.palette.success,
                theme.colors.palette.background,
                theme.colors.palette.text_primary,
            ),
            FeedbackLevel::Warning => (
                theme.colors.palette.warning,
                theme.colors.palette.background,
                theme.colors.palette.text_primary,
            ),
            FeedbackLevel::Error => (
                theme.colors.palette.error,
                theme.colors.palette.background,
                theme.colors.palette.text_primary,
            ),
            FeedbackLevel::Critical => (
                Color::Red,
                theme.colors.palette.background,
                Color::White,
            ),
            FeedbackLevel::Progress => (
                theme.colors.palette.accent,
                theme.colors.palette.background,
                theme.colors.palette.text_primary,
            ),
            FeedbackLevel::Status => (
                theme.colors.palette.text_secondary,
                theme.colors.palette.background,
                theme.colors.palette.text_primary,
            ),
            FeedbackLevel::Debug => (
                theme.colors.palette.text_muted,
                theme.colors.palette.background,
                theme.colors.palette.text_secondary,
            ),
        }
    }
}

/// Configuration for the unified feedback system
#[derive(Debug, Clone)]
pub struct FeedbackConfig {
    /// Enable debug messages
    pub show_debug: bool,
    /// Maximum toast notifications to show simultaneously
    pub max_toasts: usize,
    /// Animation duration for feedback transitions
    pub animation_duration: Duration,
    /// Whether to use compact mode for narrow screens
    pub compact_mode: bool,
    /// Context-specific routing rules
    pub routing_rules: HashMap<FeedbackContext, FeedbackMode>,
}

impl Default for FeedbackConfig {
    fn default() -> Self {
        let mut routing_rules = HashMap::new();
        routing_rules.insert(FeedbackContext::Network, FeedbackMode::StatusBar);
        routing_rules.insert(FeedbackContext::System, FeedbackMode::StatusBar);
        
        Self {
            show_debug: false,
            max_toasts: 5,
            animation_duration: Duration::from_millis(300),
            compact_mode: false,
            routing_rules,
        }
    }
}

/// Unified feedback manager that coordinates all feedback systems
pub struct UnifiedFeedbackManager {
    /// Configuration
    config: FeedbackConfig,
    /// Active feedback items
    active_items: HashMap<String, FeedbackItem>,
    /// Feedback history for debugging and reference
    history: VecDeque<FeedbackItem>,
    /// Toast manager integration
    toast_manager: ToastManager,
    /// Current progress operations
    progress_items: HashMap<String, FeedbackItem>,
    /// Status bar notifications
    status_items: VecDeque<FeedbackItem>,
    /// Modal feedback queue
    modal_queue: VecDeque<FeedbackItem>,
}

impl UnifiedFeedbackManager {
    /// Create a new unified feedback manager
    pub fn new() -> Self {
        Self {
            config: FeedbackConfig::default(),
            active_items: HashMap::new(),
            history: VecDeque::new(),
            toast_manager: ToastManager::new(),
            progress_items: HashMap::new(),
            status_items: VecDeque::new(),
            modal_queue: VecDeque::new(),
        }
    }

    /// Configure the feedback system
    pub fn configure(&mut self, config: FeedbackConfig) {
        self.config = config;
    }

    /// Add feedback to the system
    pub fn add_feedback(&mut self, item: FeedbackItem) {
        // Check if debug messages should be shown
        if item.level == FeedbackLevel::Debug && !self.config.show_debug {
            return;
        }

        // Apply routing rules
        let final_mode = self.config.routing_rules
            .get(&item.context)
            .copied()
            .unwrap_or(item.mode);

        let mut final_item = item;
        final_item.mode = final_mode;

        // Route to appropriate system based on mode
        match final_mode {
            FeedbackMode::Toast => {
                self.add_toast_feedback(&final_item);
            }
            FeedbackMode::StatusBar => {
                self.add_status_feedback(final_item.clone());
            }
            FeedbackMode::Modal => {
                self.add_modal_feedback(final_item.clone());
            }
            FeedbackMode::Progress => {
                self.add_progress_feedback(final_item.clone());
            }
            FeedbackMode::Inline => {
                // Inline feedback is handled by individual components
                // We just track it for consistency
            }
            FeedbackMode::Silent => {
                // Silent feedback is only logged
            }
        }

        // Store in active items and history
        self.active_items.insert(final_item.id.clone(), final_item.clone());
        self.add_to_history(final_item);
    }

    /// Quick convenience methods for common feedback types
    pub fn info<S: Into<String>>(&mut self, message: S, context: FeedbackContext) {
        self.add_feedback(FeedbackItem::new(message, FeedbackLevel::Info, context));
    }

    pub fn success<S: Into<String>>(&mut self, message: S, context: FeedbackContext) {
        self.add_feedback(FeedbackItem::new(message, FeedbackLevel::Success, context));
    }

    pub fn warning<S: Into<String>>(&mut self, message: S, context: FeedbackContext) {
        self.add_feedback(FeedbackItem::new(message, FeedbackLevel::Warning, context));
    }

    pub fn error<S: Into<String>>(&mut self, message: S, context: FeedbackContext) {
        self.add_feedback(FeedbackItem::new(message, FeedbackLevel::Error, context));
    }

    pub fn critical<S: Into<String>>(&mut self, message: S, context: FeedbackContext) {
        self.add_feedback(FeedbackItem::modal(message, FeedbackLevel::Critical, context));
    }

    pub fn progress<S: Into<String>>(&mut self, message: S, progress: f64, context: FeedbackContext) {
        self.add_feedback(FeedbackItem::progress(message, progress, context));
    }

    pub fn status<S: Into<String>>(&mut self, message: S, context: FeedbackContext) {
        self.add_feedback(FeedbackItem::new(message, FeedbackLevel::Status, context).with_mode(FeedbackMode::StatusBar));
    }

    /// Update progress for an existing progress item
    pub fn update_progress(&mut self, id: &str, progress: f64, message: Option<String>) {
        if let Some(item) = self.progress_items.get_mut(id) {
            item.progress = Some(progress.clamp(0.0, 1.0));
            if let Some(msg) = message {
                item.message = msg;
            }
        }
    }

    /// Complete a progress operation
    pub fn complete_progress(&mut self, id: &str, success_message: Option<String>) {
        if self.progress_items.remove(id).is_some() {
            if let Some(message) = success_message {
                self.success(message, FeedbackContext::System);
            }
        }
    }

    /// Dismiss a specific feedback item
    pub fn dismiss(&mut self, id: &str) {
        if let Some(item) = self.active_items.remove(id) {
            match item.mode {
                FeedbackMode::Toast => {
                    // Toast manager handles its own dismissal
                }
                FeedbackMode::StatusBar => {
                    self.status_items.retain(|item| item.id != id);
                }
                FeedbackMode::Modal => {
                    self.modal_queue.retain(|item| item.id != id);
                }
                FeedbackMode::Progress => {
                    self.progress_items.remove(id);
                }
                _ => {}
            }
        }
    }

    /// Clear all feedback of a specific type
    pub fn clear_context(&mut self, context: FeedbackContext) {
        let ids_to_remove: Vec<String> = self.active_items
            .iter()
            .filter(|(_, item)| item.context == context)
            .map(|(id, _)| id.clone())
            .collect();

        for id in ids_to_remove {
            self.dismiss(&id);
        }
    }

    /// Update the feedback system (call each frame)
    pub fn update(&mut self) {
        // Update toast manager
        self.toast_manager.update();

        // Clean up expired items
        let mut expired_ids = Vec::new();
        for (id, item) in &self.active_items {
            if item.is_expired() {
                expired_ids.push(id.clone());
            }
        }

        for id in expired_ids {
            self.dismiss(&id);
        }

        // Clean up expired status items
        self.status_items.retain(|item| !item.is_expired());
    }

    /// Render all feedback components
    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Render toasts
        if self.toast_manager.has_toasts() {
            crate::ui::toast::ToastRenderer::render(frame, area, self.toast_manager.toasts(), theme);
        }

        // Render progress indicators
        self.render_progress_indicators(frame, area, theme);

        // Render modal feedback
        if let Some(modal_item) = self.modal_queue.front() {
            self.render_modal_feedback(frame, area, modal_item, theme);
        }
    }

    /// Get current status bar feedback items
    pub fn get_status_items(&self) -> &VecDeque<FeedbackItem> {
        &self.status_items
    }

    /// Check if there are any critical notifications requiring attention
    pub fn has_critical_notifications(&self) -> bool {
        self.modal_queue.iter().any(|item| item.level == FeedbackLevel::Critical)
    }

    /// Get summary of current feedback state for debugging
    pub fn get_feedback_summary(&self) -> String {
        format!(
            "Feedback: {} active, {} toasts, {} progress, {} status, {} modal",
            self.active_items.len(),
            self.toast_manager.active_count(),
            self.progress_items.len(),
            self.status_items.len(),
            self.modal_queue.len()
        )
    }

    /// Show contextual feedback with intelligent routing
    pub fn show_contextual<S: Into<String>>(&mut self, message: S, level: FeedbackLevel, context: FeedbackContext) {
        self.add_feedback(FeedbackItem::new(message, level, context));
    }

    /// Show progress feedback
    pub fn show_progress<S: Into<String>>(&mut self, message: S, progress: f32, context: FeedbackContext) {
        self.progress(message, progress as f64, context);
    }

    /// Clear all active feedback items
    pub fn clear_all(&mut self) {
        self.active_items.clear();
        self.toast_manager.clear();
        self.progress_items.clear();
        self.status_items.clear();
        self.modal_queue.clear();
    }

    /// Check if there are any active feedback items
    pub fn has_active_items(&self) -> bool {
        !self.active_items.is_empty()
    }

    // Private helper methods

    fn add_toast_feedback(&mut self, item: &FeedbackItem) {
        let toast = Toast::new(item.message.clone(), item.level.into());
        self.toast_manager.add_toast(toast);
    }

    fn add_status_feedback(&mut self, item: FeedbackItem) {
        self.status_items.push_back(item);
        // Keep only the most recent status items
        while self.status_items.len() > MAX_STATUS_INDICATORS {
            self.status_items.pop_front();
        }
    }

    fn add_modal_feedback(&mut self, item: FeedbackItem) {
        self.modal_queue.push_back(item);
    }

    fn add_progress_feedback(&mut self, item: FeedbackItem) {
        self.progress_items.insert(item.id.clone(), item);
    }

    fn add_to_history(&mut self, item: FeedbackItem) {
        self.history.push_back(item);
        while self.history.len() > MAX_FEEDBACK_HISTORY {
            self.history.pop_front();
        }
    }

    fn render_progress_indicators(
        &self, 
        frame: &mut Frame, 
        area: Rect, 
        theme: &Theme
    ) {
        if self.progress_items.is_empty() {
            return;
        }

        // Create area for progress indicators in the bottom section
        let progress_height = (self.progress_items.len() as u16 * 3).min(area.height / 3);
        let progress_area = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(progress_height),
            width: area.width,
            height: progress_height,
        };

        let mut y_offset = 0;
        for item in self.progress_items.values() {
            if y_offset + 3 > progress_area.height {
                break;
            }

            let item_area = Rect {
                x: progress_area.x,
                y: progress_area.y + y_offset,
                width: progress_area.width,
                height: 3,
            };

            self.render_progress_item(frame, item_area, item, theme);
            y_offset += 3;
        }
    }

    fn render_progress_item(&self, frame: &mut Frame, area: Rect, item: &FeedbackItem, theme: &Theme) {
        let progress = item.progress.unwrap_or(0.0);
        let (border_color, bg_color, _text_color) = item.colors(theme);

        let gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("{} {}", item.icon(), item.message))
                    .border_style(Style::default().fg(border_color))
            )
            .gauge_style(Style::default().fg(border_color).bg(bg_color))
            .percent((progress * 100.0) as u16);

        frame.render_widget(Clear, area);
        frame.render_widget(gauge, area);
    }

    fn render_modal_feedback(
        &self,
        frame: &mut Frame,
        area: Rect,
        item: &FeedbackItem,
        theme: &Theme,
    ) {
        // Create centered modal area
        let modal_width = (area.width * 2 / 3).max(50);
        let modal_height = 10.min(area.height / 2);
        let modal_x = (area.width.saturating_sub(modal_width)) / 2;
        let modal_y = (area.height.saturating_sub(modal_height)) / 2;

        let modal_area = Rect {
            x: modal_x,
            y: modal_y,
            width: modal_width,
            height: modal_height,
        };

        let (border_color, bg_color, text_color) = item.colors(theme);

        // Clear the background
        frame.render_widget(Clear, modal_area);

        // Create modal content
        let title = format!("{} {}", item.icon(), 
            match item.level {
                FeedbackLevel::Critical => "Critical Alert",
                FeedbackLevel::Error => "Error",
                FeedbackLevel::Warning => "Warning",
                _ => "Notification",
            }
        );

        let mut content = vec![
            Line::from(item.message.clone())
        ];

        if let Some(action_hint) = &item.action_hint {
            content.push(Line::from(""));
            content.push(Line::from(format!("Action: {}", action_hint)));
        }

        if item.is_dismissible {
            content.push(Line::from(""));
            content.push(Line::from("Press Esc to dismiss".to_string()));
        }

        let paragraph = Paragraph::new(content)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color))
            )
            .style(Style::default().fg(text_color).bg(bg_color))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, modal_area);
    }
}

impl Default for UnifiedFeedbackManager {
    fn default() -> Self {
        Self::new()
    }
}