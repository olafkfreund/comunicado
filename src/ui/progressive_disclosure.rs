/// Progressive disclosure system for expandable/collapsible UI sections
/// 
/// Provides a unified way to handle expanding and collapsing sections throughout
/// the application, helping users manage information density and focus on relevant content.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::theme::Theme;
use crate::ui::typography::{TypographySystem, TypographyLevel};
use std::collections::HashMap;

/// Expandable section state
#[derive(Debug, Clone)]
pub struct ExpandableSection {
    /// Unique identifier for this section
    pub id: String,
    /// Section title
    pub title: String,
    /// Whether section is currently expanded
    pub expanded: bool,
    /// Optional subtitle or description
    pub subtitle: Option<String>,
    /// Number of items in this section (for display)
    pub item_count: Option<usize>,
    /// Whether this section can be collapsed
    pub collapsible: bool,
    /// Nesting level (0 = root, 1 = nested, etc.)
    pub level: usize,
    /// Priority for display order
    pub priority: i32,
}

impl ExpandableSection {
    /// Create a new expandable section
    pub fn new(id: String, title: String) -> Self {
        Self {
            id,
            title,
            expanded: true,
            subtitle: None,
            item_count: None,
            collapsible: true,
            level: 0,
            priority: 0,
        }
    }
    
    /// Set subtitle
    pub fn with_subtitle(mut self, subtitle: String) -> Self {
        self.subtitle = Some(subtitle);
        self
    }
    
    /// Set item count
    pub fn with_item_count(mut self, count: usize) -> Self {
        self.item_count = Some(count);
        self
    }
    
    /// Set whether section can be collapsed
    pub fn collapsible(mut self, collapsible: bool) -> Self {
        self.collapsible = collapsible;
        self
    }
    
    /// Set nesting level
    pub fn with_level(mut self, level: usize) -> Self {
        self.level = level;
        self
    }
    
    /// Set priority
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
    
    /// Set initial expanded state
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }
}

/// Content that can be rendered inside a section
#[derive(Debug, Clone)]
pub enum SectionContent {
    /// Simple text content
    Text(Vec<String>),
    /// List of items
    List(Vec<String>),
    /// Key-value pairs
    KeyValue(Vec<(String, String)>),
    /// Nested sections
    Sections(Vec<ExpandableSection>),
    /// Custom content (placeholder for future extensions)
    Custom(String),
}

/// Section with content and metadata
#[derive(Debug, Clone)]
pub struct Section {
    /// Section metadata and state
    pub meta: ExpandableSection,
    /// Section content
    pub content: SectionContent,
    /// Whether section is currently visible
    pub visible: bool,
}

impl Section {
    /// Create a new section with content
    pub fn new(meta: ExpandableSection, content: SectionContent) -> Self {
        Self {
            meta,
            content,
            visible: true,
        }
    }
    
    /// Toggle expanded state
    pub fn toggle(&mut self) {
        if self.meta.collapsible {
            self.meta.expanded = !self.meta.expanded;
        }
    }
    
    /// Set expanded state
    pub fn set_expanded(&mut self, expanded: bool) {
        if self.meta.collapsible {
            self.meta.expanded = expanded;
        }
    }
    
    /// Check if section is expanded
    pub fn is_expanded(&self) -> bool {
        self.meta.expanded
    }
    
    /// Check if section has content
    pub fn has_content(&self) -> bool {
        match &self.content {
            SectionContent::Text(items) => !items.is_empty(),
            SectionContent::List(items) => !items.is_empty(),
            SectionContent::KeyValue(items) => !items.is_empty(),
            SectionContent::Sections(sections) => !sections.is_empty(),
            SectionContent::Custom(_) => true,
        }
    }
}

/// Manager for progressive disclosure sections
pub struct ProgressiveDisclosureManager {
    /// All sections indexed by ID
    sections: HashMap<String, Section>,
    /// Section display order
    section_order: Vec<String>,
    /// Currently focused section ID
    focused_section: Option<String>,
    /// Global collapse/expand state
    global_expanded: bool,
}

impl ProgressiveDisclosureManager {
    /// Create a new progressive disclosure manager
    pub fn new() -> Self {
        Self {
            sections: HashMap::new(),
            section_order: Vec::new(),
            focused_section: None,
            global_expanded: true,
        }
    }
    
    /// Add a section
    pub fn add_section(&mut self, section: Section) {
        let id = section.meta.id.clone();
        self.sections.insert(id.clone(), section);
        if !self.section_order.contains(&id) {
            self.section_order.push(id);
        }
        self.sort_sections();
    }
    
    /// Remove a section
    pub fn remove_section(&mut self, id: &str) {
        self.sections.remove(id);
        self.section_order.retain(|section_id| section_id != id);
        if self.focused_section.as_ref() == Some(&id.to_string()) {
            self.focused_section = None;
        }
    }
    
    /// Get a section by ID
    pub fn get_section(&self, id: &str) -> Option<&Section> {
        self.sections.get(id)
    }
    
    /// Get a mutable section by ID
    pub fn get_section_mut(&mut self, id: &str) -> Option<&mut Section> {
        self.sections.get_mut(id)
    }
    
    /// Toggle section expanded state
    pub fn toggle_section(&mut self, id: &str) -> bool {
        if let Some(section) = self.sections.get_mut(id) {
            section.toggle();
            true
        } else {
            false
        }
    }
    
    /// Expand all sections
    pub fn expand_all(&mut self) {
        for section in self.sections.values_mut() {
            section.set_expanded(true);
        }
        self.global_expanded = true;
    }
    
    /// Collapse all sections
    pub fn collapse_all(&mut self) {
        for section in self.sections.values_mut() {
            section.set_expanded(false);
        }
        self.global_expanded = false;
    }
    
    /// Toggle global expanded state
    pub fn toggle_global(&mut self) {
        if self.global_expanded {
            self.collapse_all();
        } else {
            self.expand_all();
        }
    }
    
    /// Set focused section
    pub fn set_focused_section(&mut self, id: Option<String>) {
        self.focused_section = id;
    }
    
    /// Get focused section ID
    pub fn get_focused_section(&self) -> Option<&String> {
        self.focused_section.as_ref()
    }
    
    /// Get sections in display order
    pub fn get_sections_ordered(&self) -> Vec<&Section> {
        self.section_order
            .iter()
            .filter_map(|id| self.sections.get(id))
            .filter(|section| section.visible)
            .collect()
    }
    
    /// Navigate to next section
    pub fn next_section(&mut self) -> Option<String> {
        let visible_sections: Vec<String> = self.section_order
            .iter()
            .filter(|id| self.sections.get(*id).map(|s| s.visible).unwrap_or(false))
            .cloned()
            .collect();
            
        if visible_sections.is_empty() {
            return None;
        }
        
        let current_index = if let Some(focused) = &self.focused_section {
            visible_sections.iter().position(|id| id == focused).unwrap_or(0)
        } else {
            0
        };
        
        let next_index = (current_index + 1) % visible_sections.len();
        let next_id = visible_sections[next_index].clone();
        self.focused_section = Some(next_id.clone());
        Some(next_id)
    }
    
    /// Navigate to previous section
    pub fn previous_section(&mut self) -> Option<String> {
        let visible_sections: Vec<String> = self.section_order
            .iter()
            .filter(|id| self.sections.get(*id).map(|s| s.visible).unwrap_or(false))
            .cloned()
            .collect();
            
        if visible_sections.is_empty() {
            return None;
        }
        
        let current_index = if let Some(focused) = &self.focused_section {
            visible_sections.iter().position(|id| id == focused).unwrap_or(0)
        } else {
            0
        };
        
        let prev_index = if current_index == 0 {
            visible_sections.len() - 1
        } else {
            current_index - 1
        };
        
        let prev_id = visible_sections[prev_index].clone();
        self.focused_section = Some(prev_id.clone());
        Some(prev_id)
    }
    
    /// Sort sections by priority and level
    pub fn sort_sections(&mut self) {
        self.section_order.sort_by(|a, b| {
            let section_a = self.sections.get(a);
            let section_b = self.sections.get(b);
            
            match (section_a, section_b) {
                (Some(a), Some(b)) => {
                    // Sort by level first, then by priority
                    a.meta.level.cmp(&b.meta.level)
                        .then_with(|| a.meta.priority.cmp(&b.meta.priority))
                        .then_with(|| a.meta.title.cmp(&b.meta.title))
                }
                _ => std::cmp::Ordering::Equal,
            }
        });
    }
    
    /// Get total item count across all sections
    pub fn total_item_count(&self) -> usize {
        self.sections
            .values()
            .filter_map(|section| section.meta.item_count)
            .sum()
    }
    
    /// Get expanded section count
    pub fn expanded_section_count(&self) -> usize {
        self.sections
            .values()
            .filter(|section| section.is_expanded())
            .count()
    }
    
    /// Check if globally expanded
    pub fn is_globally_expanded(&self) -> bool {
        self.global_expanded
    }
}

impl Default for ProgressiveDisclosureManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Progressive disclosure renderer
pub struct ProgressiveDisclosureRenderer;

impl ProgressiveDisclosureRenderer {
    /// Render all sections in a layout
    pub fn render_sections(
        frame: &mut Frame,
        area: Rect,
        manager: &ProgressiveDisclosureManager,
        theme: &Theme,
        typography: &TypographySystem,
    ) {
        let sections = manager.get_sections_ordered();
        if sections.is_empty() {
            return;
        }
        
        // Calculate layout constraints based on sections
        let constraints: Vec<Constraint> = sections
            .iter()
            .map(|section| {
                if section.is_expanded() {
                    Constraint::Min(Self::calculate_section_height(section, area.width))
                } else {
                    Constraint::Length(1) // Just header
                }
            })
            .collect();
        
        let section_areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);
        
        // Render each section
        for (i, section) in sections.iter().enumerate() {
            if let Some(section_area) = section_areas.get(i) {
                Self::render_section(
                    frame,
                    *section_area,
                    section,
                    manager.get_focused_section() == Some(&section.meta.id),
                    theme,
                    typography,
                );
            }
        }
    }
    
    /// Render a single section
    pub fn render_section(
        frame: &mut Frame,
        area: Rect,
        section: &Section,
        is_focused: bool,
        theme: &Theme,
        typography: &TypographySystem,
    ) {
        // Create section header
        let header = Self::create_section_header(section, is_focused, theme, typography);
        
        if section.is_expanded() && area.height > 1 {
            // Split area for header and content
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Min(0)])
                .split(area);
            
            // Render header
            frame.render_widget(header, layout[0]);
            
            // Render content if there's space
            if layout.len() > 1 && layout[1].height > 0 {
                Self::render_section_content(
                    frame,
                    layout[1],
                    &section.content,
                    section.meta.level,
                    theme,
                    typography,
                );
            }
        } else {
            // Just render header
            frame.render_widget(header, area);
        }
    }
    
    /// Create section header
    fn create_section_header(
        section: &Section,
        is_focused: bool,
        theme: &Theme,
        typography: &TypographySystem,
    ) -> Paragraph<'static> {
        let mut spans = Vec::new();
        
        // Add indentation for nested sections
        if section.meta.level > 0 {
            spans.push(Span::raw("  ".repeat(section.meta.level)));
        }
        
        // Add expand/collapse indicator
        if section.meta.collapsible {
            let indicator = if section.is_expanded() { "▼" } else { "▶" };
            spans.push(Span::styled(
                format!("{} ", indicator),
                if is_focused {
                    Style::default()
                        .fg(theme.colors.palette.accent)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.colors.palette.text_muted)
                }
            ));
        } else {
            spans.push(Span::raw("  "));
        }
        
        // Add title
        let title_style = if is_focused {
            Style::default()
                .fg(theme.colors.palette.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            typography.get_typography_style(
                match section.meta.level {
                    0 => TypographyLevel::Heading3,
                    1 => TypographyLevel::Body,
                    _ => TypographyLevel::Caption,
                },
                theme
            )
        };
        
        spans.push(Span::styled(section.meta.title.clone(), title_style));
        
        // Add item count if available
        if let Some(count) = section.meta.item_count {
            spans.push(Span::styled(
                format!(" ({})", count),
                Style::default().fg(theme.colors.palette.text_muted)
            ));
        }
        
        // Add subtitle if available
        if let Some(ref subtitle) = section.meta.subtitle {
            spans.push(Span::raw(" - "));
            spans.push(Span::styled(
                subtitle.clone(),
                Style::default().fg(theme.colors.palette.text_muted)
            ));
        }
        
        Paragraph::new(Line::from(spans))
    }
    
    /// Render section content
    fn render_section_content(
        frame: &mut Frame,
        area: Rect,
        content: &SectionContent,
        level: usize,
        theme: &Theme,
        typography: &TypographySystem,
    ) {
        match content {
            SectionContent::Text(lines) => {
                let text: Vec<Line> = lines
                    .iter()
                    .map(|line| {
                        Line::from(vec![
                            Span::raw("  ".repeat(level + 1)),
                            Span::styled(
                                line.clone(),
                                typography.get_typography_style(TypographyLevel::Body, theme)
                            ),
                        ])
                    })
                    .collect();
                
                let paragraph = Paragraph::new(text).wrap(Wrap { trim: true });
                frame.render_widget(paragraph, area);
            }
            
            SectionContent::List(items) => {
                let list_items: Vec<ListItem> = items
                    .iter()
                    .map(|item| {
                        ListItem::new(Line::from(vec![
                            Span::raw("  ".repeat(level + 1)),
                            Span::raw("• "),
                            Span::styled(
                                item.clone(),
                                typography.get_typography_style(TypographyLevel::Body, theme)
                            ),
                        ]))
                    })
                    .collect();
                
                let list = List::new(list_items);
                frame.render_widget(list, area);
            }
            
            SectionContent::KeyValue(pairs) => {
                let items: Vec<ListItem> = pairs
                    .iter()
                    .map(|(key, value)| {
                        ListItem::new(Line::from(vec![
                            Span::raw("  ".repeat(level + 1)),
                            Span::styled(
                                format!("{}: ", key),
                                typography.get_typography_style(TypographyLevel::Label, theme)
                            ),
                            Span::styled(
                                value.clone(),
                                typography.get_typography_style(TypographyLevel::Body, theme)
                            ),
                        ]))
                    })
                    .collect();
                
                let list = List::new(items);
                frame.render_widget(list, area);
            }
            
            SectionContent::Sections(nested_sections) => {
                // Create nested manager for recursive rendering
                let mut nested_manager = ProgressiveDisclosureManager::new();
                for nested_section in nested_sections {
                    let section = Section::new(
                        nested_section.clone(),
                        SectionContent::Text(vec!["Nested section content".to_string()])
                    );
                    nested_manager.add_section(section);
                }
                
                Self::render_sections(frame, area, &nested_manager, theme, typography);
            }
            
            SectionContent::Custom(text) => {
                let paragraph = Paragraph::new(Line::from(vec![
                    Span::raw("  ".repeat(level + 1)),
                    Span::styled(
                        text.clone(),
                        typography.get_typography_style(TypographyLevel::Body, theme)
                    ),
                ]));
                frame.render_widget(paragraph, area);
            }
        }
    }
    
    /// Calculate required height for a section
    fn calculate_section_height(section: &Section, width: u16) -> u16 {
        let mut height = 1; // Header
        
        if section.is_expanded() {
            height += match &section.content {
                SectionContent::Text(lines) => {
                    lines.iter()
                        .map(|line| Self::calculate_text_height(line, width, section.meta.level + 1))
                        .sum()
                }
                SectionContent::List(items) => items.len() as u16,
                SectionContent::KeyValue(pairs) => pairs.len() as u16,
                SectionContent::Sections(sections) => sections.len() as u16,
                SectionContent::Custom(_) => 1,
            };
        }
        
        height.max(1)
    }
    
    /// Calculate height needed for text with wrapping
    fn calculate_text_height(text: &str, width: u16, indent_level: usize) -> u16 {
        let available_width = width.saturating_sub((indent_level * 2) as u16);
        if available_width == 0 {
            return 1;
        }
        
        let line_count = (text.len() as u16 + available_width - 1) / available_width;
        line_count.max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_expandable_section_creation() {
        let section = ExpandableSection::new("test".to_string(), "Test Section".to_string())
            .with_subtitle("Test subtitle".to_string())
            .with_item_count(5)
            .with_level(1)
            .expanded(false);
        
        assert_eq!(section.id, "test");
        assert_eq!(section.title, "Test Section");
        assert_eq!(section.subtitle, Some("Test subtitle".to_string()));
        assert_eq!(section.item_count, Some(5));
        assert_eq!(section.level, 1);
        assert!(!section.expanded);
    }
    
    #[test]
    fn test_section_toggle() {
        let meta = ExpandableSection::new("test".to_string(), "Test".to_string());
        let content = SectionContent::Text(vec!["Test content".to_string()]);
        let mut section = Section::new(meta, content);
        
        assert!(section.is_expanded());
        section.toggle();
        assert!(!section.is_expanded());
        section.toggle();
        assert!(section.is_expanded());
    }
    
    #[test]
    fn test_progressive_disclosure_manager() {
        let mut manager = ProgressiveDisclosureManager::new();
        
        let meta = ExpandableSection::new("test".to_string(), "Test Section".to_string());
        let content = SectionContent::Text(vec!["Test content".to_string()]);
        let section = Section::new(meta, content);
        
        manager.add_section(section);
        assert!(manager.get_section("test").is_some());
        
        assert!(manager.toggle_section("test"));
        assert!(!manager.get_section("test").unwrap().is_expanded());
        
        manager.expand_all();
        assert!(manager.get_section("test").unwrap().is_expanded());
        
        manager.collapse_all();
        assert!(!manager.get_section("test").unwrap().is_expanded());
    }
    
    #[test]
    fn test_section_navigation() {
        let mut manager = ProgressiveDisclosureManager::new();
        
        // Add multiple sections
        for i in 0..3 {
            let meta = ExpandableSection::new(
                format!("test_{}", i),
                format!("Test Section {}", i)
            );
            let content = SectionContent::Text(vec![format!("Content {}", i)]);
            let section = Section::new(meta, content);
            manager.add_section(section);
        }
        
        // Test navigation
        let first = manager.next_section();
        assert_eq!(first, Some("test_0".to_string()));
        
        let second = manager.next_section();
        assert_eq!(second, Some("test_1".to_string()));
        
        let prev = manager.previous_section();
        assert_eq!(prev, Some("test_0".to_string()));
    }
    
    #[test]
    fn test_section_content_types() {
        // Test different content types
        let text_content = SectionContent::Text(vec!["Line 1".to_string(), "Line 2".to_string()]);
        let list_content = SectionContent::List(vec!["Item 1".to_string(), "Item 2".to_string()]);
        let kv_content = SectionContent::KeyValue(vec![
            ("Key 1".to_string(), "Value 1".to_string()),
            ("Key 2".to_string(), "Value 2".to_string()),
        ]);
        
        let meta = ExpandableSection::new("test".to_string(), "Test".to_string());
        
        let text_section = Section::new(meta.clone(), text_content);
        assert!(text_section.has_content());
        
        let list_section = Section::new(meta.clone(), list_content);
        assert!(list_section.has_content());
        
        let kv_section = Section::new(meta, kv_content);
        assert!(kv_section.has_content());
    }
}