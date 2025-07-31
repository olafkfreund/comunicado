/// Dynamic keyboard shortcut hints system
/// 
/// Provides context-aware keyboard shortcut hints that change based on the current
/// UI state, focused element, and available actions. Shows relevant shortcuts
/// in a non-intrusive way to help users discover and remember key bindings.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap, Clear},
    Frame,
};

use crate::theme::Theme;
use crate::ui::typography::{TypographySystem, TypographyLevel};
use std::collections::HashMap;

/// Keyboard shortcut information
#[derive(Debug, Clone, PartialEq)]
pub struct KeyboardShortcut {
    /// Key combination (e.g., "Ctrl+N", "F5", "Enter")
    pub key: String,
    /// Description of what the shortcut does
    pub description: String,
    /// Category for grouping shortcuts
    pub category: ShortcutCategory,
    /// Priority for ordering (higher = more important)
    pub priority: i32,
    /// Whether shortcut is currently available
    pub available: bool,
}

impl KeyboardShortcut {
    /// Create a new keyboard shortcut
    pub fn new(key: String, description: String, category: ShortcutCategory) -> Self {
        Self {
            key,
            description,
            category,
            priority: 0,
            available: true,
        }
    }

    /// Set priority
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Set availability
    pub fn available(mut self, available: bool) -> Self {
        self.available = available;
        self
    }
}

/// Categories for organizing shortcuts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShortcutCategory {
    /// Navigation between views and sections
    Navigation,
    /// Email-specific actions
    Email,
    /// Calendar-specific actions
    Calendar,
    /// Contact-specific actions
    Contacts,
    /// General application actions
    General,
    /// Search and filtering
    Search,
    /// Composition and editing
    Compose,
    /// View and display options
    View,
    /// System and application control
    System,
}

impl ShortcutCategory {
    /// Get display name for category
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Navigation => "Navigation",
            Self::Email => "Email",
            Self::Calendar => "Calendar", 
            Self::Contacts => "Contacts",
            Self::General => "General",
            Self::Search => "Search",
            Self::Compose => "Compose",
            Self::View => "View",
            Self::System => "System",
        }
    }

    /// Get category icon
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Navigation => "🧭",
            Self::Email => "📧",
            Self::Calendar => "📅",
            Self::Contacts => "👥",
            Self::General => "⚙️",
            Self::Search => "🔍",
            Self::Compose => "✏️",
            Self::View => "👁️",
            Self::System => "💻",
        }
    }
}

/// Context types for determining relevant shortcuts
#[derive(Debug, Clone, PartialEq)]
pub enum ShortcutContext {
    /// Email list view
    EmailList {
        has_selection: bool,
        can_compose: bool,
        folder_name: String,
    },
    /// Email reading view
    EmailReading {
        is_draft: bool,
        has_attachments: bool,
        can_reply: bool,
    },
    /// Calendar views
    Calendar {
        view_mode: CalendarViewMode,
        has_selection: bool,
        can_create: bool,
    },
    /// Contacts view
    Contacts {
        has_selection: bool,
        can_edit: bool,
    },
    /// Search interface
    Search {
        is_active: bool,
        has_results: bool,
        search_type: SearchType,
    },
    /// Compose/edit interface
    Compose {
        is_draft: bool,
        has_content: bool,
        can_send: bool,
    },
    /// Settings view
    Settings {
        section: String,
        can_save: bool,
    },
    /// General application context
    General,
}

/// Calendar view modes for shortcut context
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CalendarViewMode {
    Day,
    Week,
    Month,
    Agenda,
}

/// Search types for shortcut context
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SearchType {
    Email,
    Calendar,
    Contacts,
    Global,
}

/// Display mode for shortcut hints
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShortcutDisplayMode {
    /// Compact single line at bottom
    StatusBar,
    /// Small popup with key shortcuts
    Popup,
    /// Full overlay with all shortcuts
    Overlay,
    /// Inline hints next to UI elements
    Inline,
    /// Hidden
    Hidden,
}

/// Dynamic shortcut hints manager
pub struct DynamicShortcutsManager {
    /// All available shortcuts indexed by context
    shortcuts: HashMap<String, Vec<KeyboardShortcut>>,
    /// Current context
    current_context: Option<ShortcutContext>,
    /// Display mode
    display_mode: ShortcutDisplayMode,
    /// Maximum shortcuts to show in compact mode
    max_compact_shortcuts: usize,
    /// Whether to show category headers
    show_categories: bool,
    /// Custom shortcut overrides
    custom_shortcuts: HashMap<String, KeyboardShortcut>,
    /// Hint visibility duration (for auto-hide)
    hint_duration_ms: u64,
    /// Last context change time
    last_context_change: Option<std::time::Instant>,
}

impl DynamicShortcutsManager {
    /// Create new dynamic shortcuts manager
    pub fn new() -> Self {
        let mut manager = Self {
            shortcuts: HashMap::new(),
            current_context: None,
            display_mode: ShortcutDisplayMode::StatusBar,
            max_compact_shortcuts: 4,
            show_categories: true,
            custom_shortcuts: HashMap::new(),
            hint_duration_ms: 3000, // 3 seconds
            last_context_change: None,
        };

        manager.initialize_default_shortcuts();
        manager
    }

    /// Initialize default shortcuts for all contexts
    fn initialize_default_shortcuts(&mut self) {
        // General shortcuts (always available)
        let general_shortcuts = vec![
            KeyboardShortcut::new("Ctrl+Q".to_string(), "Quit application".to_string(), ShortcutCategory::System).with_priority(100),
            KeyboardShortcut::new("Ctrl+H".to_string(), "Show help".to_string(), ShortcutCategory::General).with_priority(90),
            KeyboardShortcut::new("F1".to_string(), "Context help".to_string(), ShortcutCategory::General).with_priority(85),
            KeyboardShortcut::new("Tab".to_string(), "Switch pane".to_string(), ShortcutCategory::Navigation).with_priority(80),
            KeyboardShortcut::new("F5".to_string(), "Refresh".to_string(), ShortcutCategory::General).with_priority(70),
        ];
        self.shortcuts.insert("general".to_string(), general_shortcuts);

        // Email list shortcuts
        let email_list_shortcuts = vec![
            KeyboardShortcut::new("N".to_string(), "New email".to_string(), ShortcutCategory::Email).with_priority(100),
            KeyboardShortcut::new("R".to_string(), "Reply".to_string(), ShortcutCategory::Email).with_priority(95),
            KeyboardShortcut::new("Shift+R".to_string(), "Reply all".to_string(), ShortcutCategory::Email).with_priority(90),
            KeyboardShortcut::new("F".to_string(), "Forward".to_string(), ShortcutCategory::Email).with_priority(85),
            KeyboardShortcut::new("Del".to_string(), "Delete".to_string(), ShortcutCategory::Email).with_priority(80),
            KeyboardShortcut::new("A".to_string(), "Archive".to_string(), ShortcutCategory::Email).with_priority(75),
            KeyboardShortcut::new("U".to_string(), "Mark unread".to_string(), ShortcutCategory::Email).with_priority(70),
            KeyboardShortcut::new("M".to_string(), "Move to folder".to_string(), ShortcutCategory::Email).with_priority(65),
            KeyboardShortcut::new("↑/↓".to_string(), "Navigate messages".to_string(), ShortcutCategory::Navigation).with_priority(60),
            KeyboardShortcut::new("Enter".to_string(), "Open message".to_string(), ShortcutCategory::Navigation).with_priority(55),
        ];
        self.shortcuts.insert("email_list".to_string(), email_list_shortcuts);

        // Email reading shortcuts
        let email_reading_shortcuts = vec![
            KeyboardShortcut::new("R".to_string(), "Reply".to_string(), ShortcutCategory::Email).with_priority(100),
            KeyboardShortcut::new("Shift+R".to_string(), "Reply all".to_string(), ShortcutCategory::Email).with_priority(95),
            KeyboardShortcut::new("F".to_string(), "Forward".to_string(), ShortcutCategory::Email).with_priority(90),
            KeyboardShortcut::new("Del".to_string(), "Delete".to_string(), ShortcutCategory::Email).with_priority(85),
            KeyboardShortcut::new("A".to_string(), "Archive".to_string(), ShortcutCategory::Email).with_priority(80),
            KeyboardShortcut::new("U".to_string(), "Toggle read status".to_string(), ShortcutCategory::Email).with_priority(75),
            KeyboardShortcut::new("Esc".to_string(), "Back to list".to_string(), ShortcutCategory::Navigation).with_priority(70),
            KeyboardShortcut::new("↑/↓".to_string(), "Previous/Next message".to_string(), ShortcutCategory::Navigation).with_priority(65),
            KeyboardShortcut::new("Space".to_string(), "Scroll down".to_string(), ShortcutCategory::View).with_priority(60),
            KeyboardShortcut::new("Shift+Space".to_string(), "Scroll up".to_string(), ShortcutCategory::View).with_priority(55),
        ];
        self.shortcuts.insert("email_reading".to_string(), email_reading_shortcuts);

        // Calendar shortcuts
        let calendar_shortcuts = vec![
            KeyboardShortcut::new("N".to_string(), "New event".to_string(), ShortcutCategory::Calendar).with_priority(100),
            KeyboardShortcut::new("E".to_string(), "Edit event".to_string(), ShortcutCategory::Calendar).with_priority(95),
            KeyboardShortcut::new("Del".to_string(), "Delete event".to_string(), ShortcutCategory::Calendar).with_priority(90),
            KeyboardShortcut::new("D".to_string(), "Day view".to_string(), ShortcutCategory::View).with_priority(85),
            KeyboardShortcut::new("W".to_string(), "Week view".to_string(), ShortcutCategory::View).with_priority(80),
            KeyboardShortcut::new("M".to_string(), "Month view".to_string(), ShortcutCategory::View).with_priority(75),
            KeyboardShortcut::new("G".to_string(), "Agenda view".to_string(), ShortcutCategory::View).with_priority(70),
            KeyboardShortcut::new("T".to_string(), "Go to today".to_string(), ShortcutCategory::Navigation).with_priority(65),
            KeyboardShortcut::new("←/→".to_string(), "Previous/Next period".to_string(), ShortcutCategory::Navigation).with_priority(60),
            KeyboardShortcut::new("↑/↓".to_string(), "Navigate events".to_string(), ShortcutCategory::Navigation).with_priority(55),
        ];
        self.shortcuts.insert("calendar".to_string(), calendar_shortcuts);

        // Contacts shortcuts
        let contacts_shortcuts = vec![
            KeyboardShortcut::new("N".to_string(), "New contact".to_string(), ShortcutCategory::Contacts).with_priority(100),
            KeyboardShortcut::new("E".to_string(), "Edit contact".to_string(), ShortcutCategory::Contacts).with_priority(95),
            KeyboardShortcut::new("Del".to_string(), "Delete contact".to_string(), ShortcutCategory::Contacts).with_priority(90),
            KeyboardShortcut::new("Ctrl+E".to_string(), "Send email".to_string(), ShortcutCategory::Email).with_priority(85),
            KeyboardShortcut::new("Ctrl+C".to_string(), "Copy contact".to_string(), ShortcutCategory::General).with_priority(80),
            KeyboardShortcut::new("↑/↓".to_string(), "Navigate contacts".to_string(), ShortcutCategory::Navigation).with_priority(75),
            KeyboardShortcut::new("Enter".to_string(), "View contact".to_string(), ShortcutCategory::Navigation).with_priority(70),
            KeyboardShortcut::new("/".to_string(), "Search contacts".to_string(), ShortcutCategory::Search).with_priority(65),
        ];
        self.shortcuts.insert("contacts".to_string(), contacts_shortcuts);

        // Search shortcuts
        let search_shortcuts = vec![
            KeyboardShortcut::new("/".to_string(), "Start search".to_string(), ShortcutCategory::Search).with_priority(100),
            KeyboardShortcut::new("Ctrl+F".to_string(), "Find in page".to_string(), ShortcutCategory::Search).with_priority(95),
            KeyboardShortcut::new("F3".to_string(), "Find next".to_string(), ShortcutCategory::Search).with_priority(90),
            KeyboardShortcut::new("Shift+F3".to_string(), "Find previous".to_string(), ShortcutCategory::Search).with_priority(85),
            KeyboardShortcut::new("Esc".to_string(), "Clear search".to_string(), ShortcutCategory::Search).with_priority(80),
            KeyboardShortcut::new("F5".to_string(), "Toggle fuzzy search".to_string(), ShortcutCategory::Search).with_priority(75),
            KeyboardShortcut::new("Enter".to_string(), "Select result".to_string(), ShortcutCategory::Navigation).with_priority(70),
            KeyboardShortcut::new("↑/↓".to_string(), "Navigate results".to_string(), ShortcutCategory::Navigation).with_priority(65),
        ];
        self.shortcuts.insert("search".to_string(), search_shortcuts);

        // Compose shortcuts
        let compose_shortcuts = vec![
            KeyboardShortcut::new("Ctrl+Enter".to_string(), "Send email".to_string(), ShortcutCategory::Compose).with_priority(100),
            KeyboardShortcut::new("Ctrl+S".to_string(), "Save draft".to_string(), ShortcutCategory::Compose).with_priority(95),
            KeyboardShortcut::new("Ctrl+A".to_string(), "Add attachment".to_string(), ShortcutCategory::Compose).with_priority(90),
            KeyboardShortcut::new("Ctrl+K".to_string(), "Add hyperlink".to_string(), ShortcutCategory::Compose).with_priority(85),
            KeyboardShortcut::new("Tab".to_string(), "Next field".to_string(), ShortcutCategory::Navigation).with_priority(80),
            KeyboardShortcut::new("Shift+Tab".to_string(), "Previous field".to_string(), ShortcutCategory::Navigation).with_priority(75),
            KeyboardShortcut::new("Esc".to_string(), "Cancel compose".to_string(), ShortcutCategory::Navigation).with_priority(70),
            KeyboardShortcut::new("Ctrl+Z".to_string(), "Undo".to_string(), ShortcutCategory::Compose).with_priority(65),
            KeyboardShortcut::new("Ctrl+Y".to_string(), "Redo".to_string(), ShortcutCategory::Compose).with_priority(60),
        ];
        self.shortcuts.insert("compose".to_string(), compose_shortcuts);
    }

    /// Set current context and update available shortcuts
    pub fn set_context(&mut self, context: ShortcutContext) {
        if self.current_context.as_ref() != Some(&context) {
            self.current_context = Some(context);
            self.last_context_change = Some(std::time::Instant::now());
        }
    }

    /// Get current context
    pub fn current_context(&self) -> Option<&ShortcutContext> {
        self.current_context.as_ref()
    }

    /// Set display mode
    pub fn set_display_mode(&mut self, mode: ShortcutDisplayMode) {
        self.display_mode = mode;
    }

    /// Get display mode
    pub fn display_mode(&self) -> ShortcutDisplayMode {
        self.display_mode
    }

    /// Get relevant shortcuts for current context
    pub fn get_contextual_shortcuts(&self) -> Vec<KeyboardShortcut> {
        let mut shortcuts = Vec::new();

        // Always include general shortcuts
        if let Some(general) = self.shortcuts.get("general") {
            shortcuts.extend(general.clone());
        }

        // Add context-specific shortcuts
        if let Some(context) = &self.current_context {
            let context_key = self.get_context_key(context);
            if let Some(context_shortcuts) = self.shortcuts.get(&context_key) {
                shortcuts.extend(context_shortcuts.clone());
            }

            // Filter shortcuts based on context state
            shortcuts = self.filter_shortcuts_by_context(shortcuts, context);
        }

        // Add custom shortcuts
        for custom in self.custom_shortcuts.values() {
            shortcuts.push(custom.clone());
        }

        // Sort by priority (highest first)
        shortcuts.sort_by(|a, b| b.priority.cmp(&a.priority));

        // Filter out unavailable shortcuts
        shortcuts.into_iter().filter(|s| s.available).collect()
    }

    /// Get top shortcuts for compact display
    pub fn get_top_shortcuts(&self, limit: usize) -> Vec<KeyboardShortcut> {
        let shortcuts = self.get_contextual_shortcuts();
        shortcuts.into_iter().take(limit).collect()
    }

    /// Get shortcuts grouped by category
    pub fn get_shortcuts_by_category(&self) -> HashMap<ShortcutCategory, Vec<KeyboardShortcut>> {
        let shortcuts = self.get_contextual_shortcuts();
        let mut grouped: HashMap<ShortcutCategory, Vec<KeyboardShortcut>> = HashMap::new();

        for shortcut in shortcuts {
            grouped.entry(shortcut.category).or_default().push(shortcut);
        }

        // Sort within each category by priority
        for shortcuts in grouped.values_mut() {
            shortcuts.sort_by(|a, b| b.priority.cmp(&a.priority));
        }

        grouped
    }

    /// Add custom shortcut
    pub fn add_custom_shortcut(&mut self, id: String, shortcut: KeyboardShortcut) {
        self.custom_shortcuts.insert(id, shortcut);
    }

    /// Remove custom shortcut
    pub fn remove_custom_shortcut(&mut self, id: &str) {
        self.custom_shortcuts.remove(id);
    }

    /// Check if hints should be shown (based on auto-hide timer)
    pub fn should_show_hints(&self) -> bool {
        match self.display_mode {
            ShortcutDisplayMode::Hidden => false,
            _ => {
                if let Some(last_change) = self.last_context_change {
                    let elapsed = last_change.elapsed();
                    elapsed.as_millis() < self.hint_duration_ms as u128
                } else {
                    true
                }
            }
        }
    }

    /// Force show hints (reset auto-hide timer)
    pub fn show_hints(&mut self) {
        self.last_context_change = Some(std::time::Instant::now());
    }

    /// Get context key for shortcut lookup
    fn get_context_key(&self, context: &ShortcutContext) -> String {
        match context {
            ShortcutContext::EmailList { .. } => "email_list".to_string(),
            ShortcutContext::EmailReading { .. } => "email_reading".to_string(),
            ShortcutContext::Calendar { .. } => "calendar".to_string(),
            ShortcutContext::Contacts { .. } => "contacts".to_string(),
            ShortcutContext::Search { .. } => "search".to_string(),
            ShortcutContext::Compose { .. } => "compose".to_string(),
            ShortcutContext::Settings { .. } => "general".to_string(),
            ShortcutContext::General => "general".to_string(),
        }
    }

    /// Filter shortcuts based on context state
    fn filter_shortcuts_by_context(&self, shortcuts: Vec<KeyboardShortcut>, context: &ShortcutContext) -> Vec<KeyboardShortcut> {
        shortcuts.into_iter().map(|mut shortcut| {
            // Update availability based on context
            shortcut.available = match context {
                ShortcutContext::EmailList { has_selection, can_compose, .. } => {
                    match shortcut.key.as_str() {
                        "R" | "Shift+R" | "F" | "Del" | "A" | "U" | "M" => *has_selection,
                        "N" => *can_compose,
                        _ => true,
                    }
                }
                ShortcutContext::EmailReading { is_draft, has_attachments, can_reply, .. } => {
                    match shortcut.key.as_str() {
                        "R" | "Shift+R" => *can_reply,
                        "F" => !*is_draft,
                        "Ctrl+A" => *has_attachments,
                        _ => true,
                    }
                }
                ShortcutContext::Calendar { has_selection, can_create, .. } => {
                    match shortcut.key.as_str() {
                        "E" | "Del" => *has_selection,
                        "N" => *can_create,
                        _ => true,
                    }
                }
                ShortcutContext::Search { is_active, has_results, .. } => {
                    match shortcut.key.as_str() {
                        "F3" | "Shift+F3" | "Enter" | "↑/↓" => *has_results,
                        "Esc" => *is_active,
                        "/" | "Ctrl+F" => !*is_active,
                        _ => true,
                    }
                }
                ShortcutContext::Compose { has_content, can_send, .. } => {
                    match shortcut.key.as_str() {
                        "Ctrl+Enter" => *can_send,
                        "Ctrl+S" | "Ctrl+Z" | "Ctrl+Y" => *has_content,
                        _ => true,
                    }
                }
                _ => true,
            };
            shortcut
        }).collect()
    }
}

impl Default for DynamicShortcutsManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Dynamic shortcut hints renderer
pub struct DynamicShortcutsRenderer;

impl DynamicShortcutsRenderer {
    /// Render shortcuts in status bar format
    pub fn render_status_bar(
        frame: &mut Frame,
        area: Rect,
        manager: &DynamicShortcutsManager,
        theme: &Theme,
        typography: &TypographySystem,
    ) {
        if !manager.should_show_hints() {
            return;
        }

        let shortcuts = manager.get_top_shortcuts(manager.max_compact_shortcuts);
        if shortcuts.is_empty() {
            return;
        }

        let mut spans = Vec::new();

        for (i, shortcut) in shortcuts.iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw(" | "));
            }

            // Key in accent color
            spans.push(Span::styled(
                shortcut.key.clone(),
                Style::default()
                    .fg(theme.colors.palette.accent)
                    .add_modifier(Modifier::BOLD)
            ));

            spans.push(Span::raw(": "));

            // Description in normal color
            spans.push(Span::styled(
                shortcut.description.clone(),
                typography.get_typography_style(TypographyLevel::Caption, theme)
            ));
        }

        let paragraph = Paragraph::new(Line::from(spans))
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Center);

        frame.render_widget(paragraph, area);
    }

    /// Render shortcuts as popup
    pub fn render_popup(
        frame: &mut Frame,
        area: Rect,
        manager: &DynamicShortcutsManager,
        theme: &Theme,
        typography: &TypographySystem,
    ) {
        if !manager.should_show_hints() {
            return;
        }

        let shortcuts = manager.get_contextual_shortcuts();
        if shortcuts.is_empty() {
            return;
        }

        // Calculate popup size
        let popup_height = (shortcuts.len() + 2).min(10) as u16; // +2 for borders
        let popup_width = 50.min(area.width);

        // Center the popup
        let popup_area = Rect::new(
            (area.width.saturating_sub(popup_width)) / 2,
            (area.height.saturating_sub(popup_height)) / 2,
            popup_width,
            popup_height,
        );

        // Clear background
        frame.render_widget(Clear, popup_area);

        // Create popup block
        let block = Block::default()
            .title("Keyboard Shortcuts")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.colors.palette.accent))
            .style(Style::default().bg(theme.colors.palette.overlay));

        let inner_area = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        // Create shortcut items
        let items: Vec<ListItem> = shortcuts
            .into_iter()
            .take((inner_area.height as usize).saturating_sub(1))
            .map(|shortcut| {
                let spans = vec![
                    Span::styled(
                        format!("{:12}", shortcut.key),
                        Style::default()
                            .fg(theme.colors.palette.accent)
                            .add_modifier(Modifier::BOLD)
                    ),
                    Span::raw(" "),
                    Span::styled(
                        shortcut.description,
                        if shortcut.available {
                            typography.get_typography_style(TypographyLevel::Body, theme)
                        } else {
                            Style::default().fg(theme.colors.palette.text_muted)
                        }
                    ),
                ];
                ListItem::new(Line::from(spans))
            })
            .collect();

        let list = List::new(items);
        frame.render_widget(list, inner_area);
    }

    /// Render shortcuts as full overlay
    pub fn render_overlay(
        frame: &mut Frame,
        area: Rect,
        manager: &DynamicShortcutsManager,
        theme: &Theme,
        typography: &TypographySystem,
    ) {
        let shortcuts_by_category = manager.get_shortcuts_by_category();
        if shortcuts_by_category.is_empty() {
            return;
        }

        // Clear entire background
        frame.render_widget(Clear, area);

        // Create overlay block
        let block = Block::default()
            .title("All Keyboard Shortcuts")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.colors.palette.accent))
            .style(Style::default().bg(theme.colors.palette.overlay));

        let inner_area = block.inner(area);
        frame.render_widget(block, area);

        // Calculate layout for categories
        let category_count = shortcuts_by_category.len();
        let cols = if category_count <= 2 { 1 } else if category_count <= 4 { 2 } else { 3 };
        let rows = (category_count + cols - 1) / cols;

        let col_constraints: Vec<Constraint> = (0..cols)
            .map(|_| Constraint::Percentage(100 / cols as u16))
            .collect();

        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(col_constraints)
            .split(inner_area);

        // Render each category
        let categories: Vec<ShortcutCategory> = shortcuts_by_category.keys().copied().collect();
        for (col_idx, column_area) in columns.iter().enumerate() {
            let start_idx = col_idx * rows;
            let end_idx = (start_idx + rows).min(categories.len());

            if start_idx >= categories.len() {
                continue;
            }

            let row_constraints: Vec<Constraint> = (start_idx..end_idx)
                .map(|_| Constraint::Min(5))
                .collect();

            let rows_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints(row_constraints)
                .split(*column_area);

            for (row_idx, &category) in categories[start_idx..end_idx].iter().enumerate() {
                if let (Some(row_area), Some(shortcuts)) = (rows_layout.get(row_idx), shortcuts_by_category.get(&category)) {
                    Self::render_category_section(*row_area, category, shortcuts, theme, typography, frame);
                }
            }
        }
    }

    /// Render a category section
    fn render_category_section(
        area: Rect,
        category: ShortcutCategory,
        shortcuts: &[KeyboardShortcut],
        theme: &Theme,
        typography: &TypographySystem,
        frame: &mut Frame,
    ) {
        if area.height < 3 {
            return;
        }

        // Category header
        let header = Paragraph::new(Line::from(vec![
            Span::raw(category.icon()),
            Span::raw(" "),
            Span::styled(
                category.display_name(),
                typography.get_typography_style(TypographyLevel::Heading3, theme)
            ),
        ]));

        let header_area = Rect::new(area.x, area.y, area.width, 1);
        frame.render_widget(header, header_area);

        // Shortcuts list
        if area.height > 1 {
            let list_area = Rect::new(area.x, area.y + 1, area.width, area.height - 1);
            let items: Vec<ListItem> = shortcuts
                .iter()
                .take(list_area.height as usize)
                .map(|shortcut| {
                    let spans = vec![
                        Span::raw("  "),
                        Span::styled(
                            format!("{:10}", shortcut.key),
                            Style::default()
                                .fg(theme.colors.palette.accent)
                                .add_modifier(Modifier::BOLD)
                        ),
                        Span::raw(" "),
                        Span::styled(
                            shortcut.description.clone(),
                            if shortcut.available {
                                typography.get_typography_style(TypographyLevel::Caption, theme)
                            } else {
                                Style::default().fg(theme.colors.palette.text_muted)
                            }
                        ),
                    ];
                    ListItem::new(Line::from(spans))
                })
                .collect();

            let list = List::new(items);
            frame.render_widget(list, list_area);
        }
    }

    /// Render inline hints next to UI elements
    pub fn render_inline_hint(
        frame: &mut Frame,
        area: Rect,
        shortcut: &KeyboardShortcut,
        theme: &Theme,
        typography: &TypographySystem,
    ) {
        if !shortcut.available {
            return;
        }

        let spans = vec![
            Span::raw("("),
            Span::styled(
                shortcut.key.clone(),
                Style::default()
                    .fg(theme.colors.palette.accent)
                    .add_modifier(Modifier::BOLD)
            ),
            Span::raw(")"),
        ];

        let hint = Paragraph::new(Line::from(spans))
            .style(typography.get_typography_style(TypographyLevel::Caption, theme));

        frame.render_widget(hint, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shortcut_creation() {
        let shortcut = KeyboardShortcut::new(
            "Ctrl+N".to_string(),
            "New email".to_string(),
            ShortcutCategory::Email
        ).with_priority(100).available(true);

        assert_eq!(shortcut.key, "Ctrl+N");
        assert_eq!(shortcut.description, "New email");
        assert_eq!(shortcut.category, ShortcutCategory::Email);
        assert_eq!(shortcut.priority, 100);
        assert!(shortcut.available);
    }

    #[test]
    fn test_context_shortcuts() {
        let mut manager = DynamicShortcutsManager::new();
        
        let context = ShortcutContext::EmailList {
            has_selection: true,
            can_compose: true,
            folder_name: "INBOX".to_string(),
        };
        
        manager.set_context(context);
        let shortcuts = manager.get_contextual_shortcuts();
        
        // Should have general + email list shortcuts
        assert!(!shortcuts.is_empty());
        
        // Should have reply shortcut available
        let has_reply = shortcuts.iter().any(|s| s.key == "R" && s.available);
        assert!(has_reply);
    }

    #[test]
    fn test_shortcut_filtering() {
        let mut manager = DynamicShortcutsManager::new();
        
        // Context with no selection should disable selection-dependent shortcuts
        let context = ShortcutContext::EmailList {
            has_selection: false,
            can_compose: true,
            folder_name: "INBOX".to_string(),
        };
        
        manager.set_context(context);
        let shortcuts = manager.get_contextual_shortcuts();
        
        // Reply shortcut should be unavailable
        let reply_shortcut = shortcuts.iter().find(|s| s.key == "R");
        if let Some(shortcut) = reply_shortcut {
            assert!(!shortcut.available);
        }
        
        // New email shortcut should be available
        let new_shortcut = shortcuts.iter().find(|s| s.key == "N");
        if let Some(shortcut) = new_shortcut {
            assert!(shortcut.available);
        }
    }

    #[test]
    fn test_shortcuts_by_category() {
        let manager = DynamicShortcutsManager::new();
        let context = ShortcutContext::EmailList {
            has_selection: true,
            can_compose: true,
            folder_name: "INBOX".to_string(),
        };
        
        let mut test_manager = manager;
        test_manager.set_context(context);
        let grouped = test_manager.get_shortcuts_by_category();
        
        // Should have multiple categories
        assert!(!grouped.is_empty());
        
        // Should have email category
        assert!(grouped.contains_key(&ShortcutCategory::Email));
        
        // Should have navigation category
        assert!(grouped.contains_key(&ShortcutCategory::Navigation));
    }

    #[test]
    fn test_custom_shortcuts() {
        let mut manager = DynamicShortcutsManager::new();
        
        let custom_shortcut = KeyboardShortcut::new(
            "Ctrl+T".to_string(),
            "Custom action".to_string(),
            ShortcutCategory::General
        );
        
        manager.add_custom_shortcut("custom_test".to_string(), custom_shortcut);
        
        let shortcuts = manager.get_contextual_shortcuts();
        let has_custom = shortcuts.iter().any(|s| s.key == "Ctrl+T");
        assert!(has_custom);
        
        manager.remove_custom_shortcut("custom_test");
        let shortcuts = manager.get_contextual_shortcuts();
        let has_custom = shortcuts.iter().any(|s| s.key == "Ctrl+T");
        assert!(!has_custom);
    }

    #[test]
    fn test_auto_hide_timer() {
        let mut manager = DynamicShortcutsManager::new();
        manager.hint_duration_ms = 100; // 100ms for test
        
        // Initially should show
        assert!(manager.should_show_hints());
        
        // Set context to start timer
        manager.set_context(ShortcutContext::General);
        assert!(manager.should_show_hints());
        
        // Wait for auto-hide (would require actual delay in real test)
        // For unit test, we can manually set the timestamp
        manager.last_context_change = Some(std::time::Instant::now() - std::time::Duration::from_millis(200));
        assert!(!manager.should_show_hints());
        
        // Force show should reset timer
        manager.show_hints();
        assert!(manager.should_show_hints());
    }
}