//! Advanced email filters UI for creating and managing complex filter rules
//!
//! This module provides a comprehensive user interface for:
//! - Creating complex filters with boolean logic
//! - Managing filter templates and presets
//! - Testing and debugging filter rules
//! - Viewing filter statistics and performance

use crate::email::{
    AdvancedEmailFilter, AdvancedFilterEngine, AdvancedCondition, ConditionGroup, BooleanLogic,
    ActionRule, AdvancedFilterAction, FilterTemplateLibrary,
};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, List, ListItem, ListState, Paragraph, Tabs, Wrap,
    },
    Frame,
};
use std::sync::Arc;
use uuid::Uuid;

/// Advanced filters UI state
#[allow(dead_code)]
pub struct AdvancedFiltersUI {
    engine: Arc<AdvancedFilterEngine>,
    
    // UI state
    current_tab: FilterTab,
    focused_area: FocusedArea,
    
    // Filter management
    filters: Vec<AdvancedEmailFilter>,
    selected_filter_index: Option<usize>,
    filter_list_state: ListState,
    
    // Filter editor
    editing_filter: Option<AdvancedEmailFilter>,
    condition_editor: ConditionEditor,
    action_editor: ActionEditor,
    
    // Template management
    templates: FilterTemplateLibrary,
    selected_template: Option<String>,
    
    // Testing
    test_mode: bool,
    test_results: Vec<String>,
    
    // Statistics
    show_statistics: bool,
}

/// Main filter management tabs
#[derive(Debug, Clone, PartialEq)]
pub enum FilterTab {
    FilterList,
    FilterEditor,
    Templates,
    Testing,
    Statistics,
}

/// Focus areas within the UI
#[derive(Debug, Clone, PartialEq)]
pub enum FocusedArea {
    TabBar,
    FilterList,
    ConditionEditor,
    ActionEditor,
    TemplateList,
    TestPanel,
    StatisticsPanel,
}

/// Actions that can be performed in the UI
#[derive(Debug, Clone)]
pub enum FilterUIAction {
    CreateNewFilter,
    EditFilter(Uuid),
    DeleteFilter(Uuid),
    ToggleFilter(Uuid),
    ApplyTemplate(String),
    TestFilter(Uuid),
    ExportFilters,
    ImportFilters,
    ViewStatistics(Uuid),
    SaveChanges,
    CancelEditing,
}

/// Condition editor state
#[allow(dead_code)]
pub struct ConditionEditor {
    current_group: ConditionGroup,
    selected_condition_index: Option<usize>,
    editing_condition: Option<AdvancedCondition>,
    
    // Form fields
    field_input: String,
    operator_input: String,
    value_input: String,
    case_sensitive: bool,
    negate: bool,
    
    // Group logic
    group_logic: BooleanLogic,
    nested_groups: Vec<ConditionGroup>,
}

/// Action editor state
#[allow(dead_code)]
pub struct ActionEditor {
    current_actions: Vec<ActionRule>,
    selected_action_index: Option<usize>,
    editing_action: Option<AdvancedFilterAction>,
    
    // Form fields
    action_type_input: String,
    action_value_input: String,
    condition_enabled: bool,
    priority: i32,
}

impl AdvancedFiltersUI {
    /// Create a new advanced filters UI
    pub fn new(engine: Arc<AdvancedFilterEngine>) -> Self {
        let templates = FilterTemplateLibrary::new();
        
        Self {
            engine,
            current_tab: FilterTab::FilterList,
            focused_area: FocusedArea::FilterList,
            filters: Vec::new(),
            selected_filter_index: None,
            filter_list_state: ListState::default(),
            editing_filter: None,
            condition_editor: ConditionEditor::new(),
            action_editor: ActionEditor::new(),
            templates,
            selected_template: None,
            test_mode: false,
            test_results: Vec::new(),
            show_statistics: false,
        }
    }

    /// Handle keyboard input
    pub async fn handle_key(&mut self, key: KeyCode, modifiers: KeyModifiers) -> (bool, Option<FilterUIAction>) {
        match key {
            KeyCode::Tab => {
                self.next_tab();
                (true, None)
            }
            KeyCode::BackTab => {
                self.previous_tab();
                (true, None)
            }
            KeyCode::F(1) => (true, Some(FilterUIAction::CreateNewFilter)),
            KeyCode::F(2) if self.selected_filter_index.is_some() => {
                let filter_id = self.filters[self.selected_filter_index.unwrap()].id;
                (true, Some(FilterUIAction::EditFilter(filter_id)))
            }
            KeyCode::F(3) if self.selected_filter_index.is_some() => {
                let filter_id = self.filters[self.selected_filter_index.unwrap()].id;
                (true, Some(FilterUIAction::DeleteFilter(filter_id)))
            }
            KeyCode::F(5) => (true, Some(FilterUIAction::TestFilter(
                self.filters.get(self.selected_filter_index.unwrap_or(0))
                    .map(|f| f.id)
                    .unwrap_or_else(Uuid::new_v4)
            ))),
            KeyCode::Char('s') if modifiers.contains(KeyModifiers::CONTROL) => {
                (true, Some(FilterUIAction::SaveChanges))
            }
            KeyCode::Esc => {
                if self.editing_filter.is_some() {
                    self.editing_filter = None;
                    (true, Some(FilterUIAction::CancelEditing))
                } else {
                    (false, None)
                }
            }
            _ => {
                match self.current_tab {
                    FilterTab::FilterList => self.handle_filter_list_key(key, modifiers).await,
                    FilterTab::FilterEditor => self.handle_filter_editor_key(key, modifiers).await,
                    FilterTab::Templates => self.handle_templates_key(key, modifiers).await,
                    FilterTab::Testing => self.handle_testing_key(key, modifiers).await,
                    FilterTab::Statistics => self.handle_statistics_key(key, modifiers).await,
                }
            }
        }
    }

    /// Render the UI
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Tab bar
                Constraint::Min(0),    // Content area
                Constraint::Length(3), // Status bar
            ])
            .split(area);

        // Render tab bar
        self.render_tab_bar(frame, chunks[0]);

        // Render content based on current tab
        match self.current_tab {
            FilterTab::FilterList => self.render_filter_list(frame, chunks[1]),
            FilterTab::FilterEditor => self.render_filter_editor(frame, chunks[1]),
            FilterTab::Templates => self.render_templates(frame, chunks[1]),
            FilterTab::Testing => self.render_testing(frame, chunks[1]),
            FilterTab::Statistics => self.render_statistics(frame, chunks[1]),
        }

        // Render status bar
        self.render_status_bar(frame, chunks[2]);
    }

    /// Render tab bar
    fn render_tab_bar(&self, frame: &mut Frame, area: Rect) {
        let tab_titles = vec![
            "Filter List",
            "Editor", 
            "Templates",
            "Testing",
            "Statistics",
        ];

        let selected_tab = match self.current_tab {
            FilterTab::FilterList => 0,
            FilterTab::FilterEditor => 1,
            FilterTab::Templates => 2,
            FilterTab::Testing => 3,
            FilterTab::Statistics => 4,
        };

        let tabs = Tabs::new(tab_titles)
            .block(Block::default().borders(Borders::ALL).title("Advanced Email Filters"))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .select(selected_tab);

        frame.render_widget(tabs, area);
    }

    /// Render filter list tab
    fn render_filter_list(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(60), // Filter list
                Constraint::Percentage(40), // Filter details
            ])
            .split(area);

        // Filter list
        let filter_items: Vec<ListItem> = self.filters
            .iter()
            .enumerate()
            .map(|(i, filter)| {
                let enabled_indicator = if filter.enabled { "✓" } else { "✗" };
                let priority_indicator = format!("[{}]", filter.priority);
                let stats = format!("({} matches)", filter.statistics.matches_count);
                
                let content = format!("{} {} {} - {}", 
                    enabled_indicator, 
                    priority_indicator,
                    filter.name,
                    stats
                );
                
                let style = if Some(i) == self.selected_filter_index {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else if filter.enabled {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Gray)
                };

                ListItem::new(content).style(style)
            })
            .collect();

        let filter_list = List::new(filter_items)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Email Filters")
                .border_style(if self.focused_area == FocusedArea::FilterList {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                }))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol("→ ");

        frame.render_stateful_widget(filter_list, chunks[0], &mut self.filter_list_state);

        // Filter details
        if let Some(index) = self.selected_filter_index {
            if let Some(filter) = self.filters.get(index) {
                self.render_filter_details(frame, chunks[1], filter);
            }
        } else {
            let help_text = vec![
                Line::from("Keyboard Shortcuts:"),
                Line::from(""),
                Line::from("F1  - Create new filter"),
                Line::from("F2  - Edit selected filter"),
                Line::from("F3  - Delete selected filter"),
                Line::from("F5  - Test selected filter"),
                Line::from("↑/↓ - Navigate filters"),
                Line::from("Tab - Switch tabs"),
                Line::from("Ctrl+S - Save changes"),
            ];

            let help = Paragraph::new(help_text)
                .block(Block::default().borders(Borders::ALL).title("Help"))
                .wrap(Wrap { trim: true });

            frame.render_widget(help, chunks[1]);
        }
    }

    /// Render filter details panel
    fn render_filter_details(&self, frame: &mut Frame, area: Rect, filter: &AdvancedEmailFilter) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6),  // Basic info
                Constraint::Min(5),     // Conditions
                Constraint::Min(3),     // Actions
                Constraint::Length(4),  // Statistics
            ])
            .split(area);

        // Basic filter info
        let info_lines = vec![
            Line::from(vec![
                Span::styled("Name: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&filter.name),
            ]),
            Line::from(vec![
                Span::styled("Priority: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(filter.priority.to_string()),
            ]),
            Line::from(vec![
                Span::styled("Enabled: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(
                    if filter.enabled { "Yes" } else { "No" },
                    Style::default().fg(if filter.enabled { Color::Green } else { Color::Red })
                ),
            ]),
            Line::from(vec![
                Span::styled("Tags: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(filter.tags.join(", ")),
            ]),
        ];

        let info_paragraph = Paragraph::new(info_lines)
            .block(Block::default().borders(Borders::ALL).title("Filter Info"));
        frame.render_widget(info_paragraph, chunks[0]);

        // Conditions preview
        let conditions_text = self.format_condition_group(&filter.condition_group, 0);
        let conditions_paragraph = Paragraph::new(conditions_text)
            .block(Block::default().borders(Borders::ALL).title("Conditions"))
            .wrap(Wrap { trim: true });
        frame.render_widget(conditions_paragraph, chunks[1]);

        // Actions preview
        let actions_text: Vec<Line> = filter.action_rules
            .iter()
            .flat_map(|rule| {
                rule.actions.iter().map(|action| {
                    Line::from(format!("• {:?}", action))
                })
            })
            .collect();

        let actions_paragraph = Paragraph::new(actions_text)
            .block(Block::default().borders(Borders::ALL).title("Actions"))
            .wrap(Wrap { trim: true });
        frame.render_widget(actions_paragraph, chunks[2]);

        // Statistics
        let stats_lines = vec![
            Line::from(format!("Matches: {}", filter.statistics.matches_count)),
            Line::from(format!("Actions: {}", filter.statistics.actions_executed)),
            Line::from(format!("Avg Time: {:.2}ms", filter.statistics.average_execution_time_ms)),
        ];

        let stats_paragraph = Paragraph::new(stats_lines)
            .block(Block::default().borders(Borders::ALL).title("Statistics"));
        frame.render_widget(stats_paragraph, chunks[3]);
    }

    /// Format condition group for display
    fn format_condition_group(&self, group: &ConditionGroup, indent: usize) -> Vec<Line> {
        let mut lines = Vec::new();
        let indent_str = "  ".repeat(indent);

        lines.push(Line::from(format!("{}Logic: {:?}", indent_str, group.logic)));

        for condition in &group.conditions {
            lines.push(Line::from(format!(
                "{}• {:?} {:?} {:?}",
                indent_str,
                condition.field,
                condition.operator,
                condition.value
            )));
        }

        for nested_group in &group.nested_groups {
            lines.push(Line::from(format!("{}Group:", indent_str)));
            lines.extend(self.format_condition_group(nested_group, indent + 1));
        }

        lines
    }

    /// Render filter editor tab
    fn render_filter_editor(&mut self, frame: &mut Frame, area: Rect) {
        if self.editing_filter.is_none() {
            let help_text = vec![
                Line::from("No filter selected for editing."),
                Line::from(""),
                Line::from("Press F1 to create a new filter,"),
                Line::from("or select a filter from the Filter List tab and press F2."),
            ];

            let help = Paragraph::new(help_text)
                .block(Block::default().borders(Borders::ALL).title("Filter Editor"))
                .alignment(Alignment::Center);

            frame.render_widget(help, area);
            return;
        }

        // Split into condition and action editors
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(60), // Condition editor
                Constraint::Percentage(40), // Action editor
            ])
            .split(area);

        self.render_condition_editor(frame, chunks[0]);
        self.render_action_editor(frame, chunks[1]);
    }

    /// Render condition editor
    fn render_condition_editor(&self, frame: &mut Frame, area: Rect) {
        let placeholder_text = vec![
            Line::from("Condition Editor"),
            Line::from(""),
            Line::from("This would contain:"),
            Line::from("• Field selection dropdown"),
            Line::from("• Operator selection"),
            Line::from("• Value input field"),
            Line::from("• Boolean logic controls"),
            Line::from("• Nested group management"),
        ];

        let editor = Paragraph::new(placeholder_text)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Condition Editor")
                .border_style(if self.focused_area == FocusedArea::ConditionEditor {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                }));

        frame.render_widget(editor, area);
    }

    /// Render action editor
    fn render_action_editor(&self, frame: &mut Frame, area: Rect) {
        let placeholder_text = vec![
            Line::from("Action Editor"),
            Line::from(""),
            Line::from("This would contain:"),
            Line::from("• Action type selection"),
            Line::from("• Action parameters"),
            Line::from("• Conditional action rules"),
            Line::from("• Priority settings"),
        ];

        let editor = Paragraph::new(placeholder_text)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Action Editor")
                .border_style(if self.focused_area == FocusedArea::ActionEditor {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                }));

        frame.render_widget(editor, area);
    }

    /// Render templates tab
    fn render_templates(&self, frame: &mut Frame, area: Rect) {
        let template_names: Vec<&String> = self.templates.get_templates().keys().collect();
        let template_items: Vec<ListItem> = template_names
            .iter()
            .map(|name| {
                ListItem::new(name.as_str())
            })
            .collect();

        let template_list = List::new(template_items)
            .block(Block::default().borders(Borders::ALL).title("Filter Templates"))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol("→ ");

        frame.render_widget(template_list, area);
    }

    /// Render testing tab
    fn render_testing(&self, frame: &mut Frame, area: Rect) {
        let test_lines = if self.test_results.is_empty() {
            vec![
                Line::from("Filter Testing"),
                Line::from(""),
                Line::from("Select a filter and press F5 to test it against sample emails."),
                Line::from("Results will appear here."),
            ]
        } else {
            self.test_results.iter().map(|result| Line::from(result.as_str())).collect()
        };

        let test_panel = Paragraph::new(test_lines)
            .block(Block::default().borders(Borders::ALL).title("Filter Testing"))
            .wrap(Wrap { trim: true });

        frame.render_widget(test_panel, area);
    }

    /// Render statistics tab
    fn render_statistics(&self, frame: &mut Frame, area: Rect) {
        let stats_lines = vec![
            Line::from(format!("Total Filters: {}", self.filters.len())),
            Line::from(format!("Enabled Filters: {}", self.filters.iter().filter(|f| f.enabled).count())),
            Line::from(""),
            Line::from("Top Performing Filters:"),
        ];

        let stats_panel = Paragraph::new(stats_lines)
            .block(Block::default().borders(Borders::ALL).title("Filter Statistics"));

        frame.render_widget(stats_panel, area);
    }

    /// Render status bar
    fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        let status_text = match self.current_tab {
            FilterTab::FilterList => "F1: New | F2: Edit | F3: Delete | F5: Test",
            FilterTab::FilterEditor => "Ctrl+S: Save | Esc: Cancel",
            FilterTab::Templates => "Enter: Apply Template",
            FilterTab::Testing => "F5: Run Test",
            FilterTab::Statistics => "View filter performance metrics",
        };

        let status = Paragraph::new(status_text)
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Center);

        frame.render_widget(status, area);
    }

    /// Navigation methods
    fn next_tab(&mut self) {
        self.current_tab = match self.current_tab {
            FilterTab::FilterList => FilterTab::FilterEditor,
            FilterTab::FilterEditor => FilterTab::Templates,
            FilterTab::Templates => FilterTab::Testing,
            FilterTab::Testing => FilterTab::Statistics,
            FilterTab::Statistics => FilterTab::FilterList,
        };
        self.update_focus_for_tab();
    }

    fn previous_tab(&mut self) {
        self.current_tab = match self.current_tab {
            FilterTab::FilterList => FilterTab::Statistics,
            FilterTab::FilterEditor => FilterTab::FilterList,
            FilterTab::Templates => FilterTab::FilterEditor,
            FilterTab::Testing => FilterTab::Templates,
            FilterTab::Statistics => FilterTab::Testing,
        };
        self.update_focus_for_tab();
    }

    fn update_focus_for_tab(&mut self) {
        self.focused_area = match self.current_tab {
            FilterTab::FilterList => FocusedArea::FilterList,
            FilterTab::FilterEditor => FocusedArea::ConditionEditor,
            FilterTab::Templates => FocusedArea::TemplateList,
            FilterTab::Testing => FocusedArea::TestPanel,
            FilterTab::Statistics => FocusedArea::StatisticsPanel,
        };
    }

    /// Key handling for different tabs
    async fn handle_filter_list_key(&mut self, key: KeyCode, _modifiers: KeyModifiers) -> (bool, Option<FilterUIAction>) {
        match key {
            KeyCode::Up => {
                if let Some(index) = self.selected_filter_index {
                    if index > 0 {
                        self.selected_filter_index = Some(index - 1);
                        self.filter_list_state.select(Some(index - 1));
                    }
                } else if !self.filters.is_empty() {
                    self.selected_filter_index = Some(0);
                    self.filter_list_state.select(Some(0));
                }
                (true, None)
            }
            KeyCode::Down => {
                if let Some(index) = self.selected_filter_index {
                    if index < self.filters.len() - 1 {
                        self.selected_filter_index = Some(index + 1);
                        self.filter_list_state.select(Some(index + 1));
                    }
                } else if !self.filters.is_empty() {
                    self.selected_filter_index = Some(0);
                    self.filter_list_state.select(Some(0));
                }
                (true, None)
            }
            KeyCode::Char(' ') => {
                if let Some(index) = self.selected_filter_index {
                    let filter_id = self.filters[index].id;
                    (true, Some(FilterUIAction::ToggleFilter(filter_id)))
                } else {
                    (true, None)
                }
            }
            _ => (true, None),
        }
    }

    async fn handle_filter_editor_key(&mut self, _key: KeyCode, _modifiers: KeyModifiers) -> (bool, Option<FilterUIAction>) {
        // TODO: Implement filter editor key handling
        (true, None)
    }

    async fn handle_templates_key(&mut self, _key: KeyCode, _modifiers: KeyModifiers) -> (bool, Option<FilterUIAction>) {
        // TODO: Implement template key handling
        (true, None)
    }

    async fn handle_testing_key(&mut self, _key: KeyCode, _modifiers: KeyModifiers) -> (bool, Option<FilterUIAction>) {
        // TODO: Implement testing key handling
        (true, None)
    }

    async fn handle_statistics_key(&mut self, _key: KeyCode, _modifiers: KeyModifiers) -> (bool, Option<FilterUIAction>) {
        // TODO: Implement statistics key handling
        (true, None)
    }
}

impl ConditionEditor {
    fn new() -> Self {
        Self {
            current_group: ConditionGroup {
                logic: BooleanLogic::And,
                conditions: Vec::new(),
                nested_groups: Vec::new(),
            },
            selected_condition_index: None,
            editing_condition: None,
            field_input: String::new(),
            operator_input: String::new(),
            value_input: String::new(),
            case_sensitive: false,
            negate: false,
            group_logic: BooleanLogic::And,
            nested_groups: Vec::new(),
        }
    }
}

impl ActionEditor {
    fn new() -> Self {
        Self {
            current_actions: Vec::new(),
            selected_action_index: None,
            editing_action: None,
            action_type_input: String::new(),
            action_value_input: String::new(),
            condition_enabled: false,
            priority: 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_advanced_filters_ui_creation() {
        let engine = Arc::new(AdvancedFilterEngine::new());
        let ui = AdvancedFiltersUI::new(engine);
        
        assert_eq!(ui.current_tab, FilterTab::FilterList);
        assert_eq!(ui.focused_area, FocusedArea::FilterList);
        assert!(ui.filters.is_empty());
    }

    #[tokio::test]
    async fn test_tab_navigation() {
        let engine = Arc::new(AdvancedFilterEngine::new());
        let mut ui = AdvancedFiltersUI::new(engine);
        
        ui.next_tab();
        assert_eq!(ui.current_tab, FilterTab::FilterEditor);
        
        ui.previous_tab();
        assert_eq!(ui.current_tab, FilterTab::FilterList);
    }
}