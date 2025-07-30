//! Advanced search UI for contacts
//!
//! Provides a comprehensive search interface with multiple criteria,
//! filter options, and search result management.

use crate::contacts::{
    advanced_search::{AdvancedSearchCriteria, SearchResult, SortDirection, SortField},
    ContactSource, ContactsManager,
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs, Wrap,
    },
    Frame,
};
use std::sync::Arc;

/// Advanced search UI state
pub struct AdvancedSearchUI {
    manager: Arc<ContactsManager>,
    
    // Search criteria
    criteria: AdvancedSearchCriteria,
    
    // UI state
    current_tab: SearchTab,
    focused_field: SearchField,
    
    // Form input values
    general_query: String,
    name_query: String,
    email_query: String,
    phone_query: String,
    company_query: String,
    notes_query: String,
    
    // Filter states
    source_filters: Vec<bool>, // [Google, Outlook, Local]
    has_email_filter: Option<bool>,
    has_phone_filter: Option<bool>,
    has_company_filter: Option<bool>,
    has_notes_filter: Option<bool>,
    
    // Options
    fuzzy_matching: bool,
    case_sensitive: bool,
    whole_word_only: bool,
    
    // Results
    search_results: Vec<SearchResult>,
    results_list_state: ListState,
    
    // UI flags
    is_searching: bool,
    search_in_progress: bool,
    show_help: bool,
    
    // Pagination
    current_page: usize,
    results_per_page: usize,
    total_results: usize,
}

/// Search tabs for organizing the interface
#[derive(Debug, Clone, PartialEq)]
pub enum SearchTab {
    BasicSearch,
    Filters,
    Options,
    Results,
}

/// Focusable fields in the search interface
#[derive(Debug, Clone, PartialEq)]
pub enum SearchField {
    GeneralQuery,
    NameQuery,
    EmailQuery,
    PhoneQuery,
    CompanyQuery,
    NotesQuery,
    SourceFilters,
    EmailFilter,
    PhoneFilter,
    CompanyFilter,
    NotesFilter,
    FuzzyMatching,
    CaseSensitive,
    WholeWordOnly,
    ResultsList,
    SearchButton,
    ClearButton,
}

/// Search action results
#[derive(Debug, Clone)]
pub enum SearchAction {
    PerformSearch,
    ClearSearch,
    ViewContact(i64),
    ComposeEmail { to: String, name: String },
    EditContact(i64),
    ExportResults,
    SaveSearch,
    ShowHelp,
    Exit,
}

impl AdvancedSearchUI {
    /// Create new advanced search UI
    pub fn new(manager: Arc<ContactsManager>) -> Self {
        Self {
            manager,
            criteria: AdvancedSearchCriteria::new(),
            current_tab: SearchTab::BasicSearch,
            focused_field: SearchField::GeneralQuery,
            general_query: String::new(),
            name_query: String::new(),
            email_query: String::new(),
            phone_query: String::new(),
            company_query: String::new(),
            notes_query: String::new(),
            source_filters: vec![true, true, true], // All sources enabled by default
            has_email_filter: None,
            has_phone_filter: None,
            has_company_filter: None,
            has_notes_filter: None,
            fuzzy_matching: false,
            case_sensitive: false,
            whole_word_only: false,
            search_results: Vec::new(),
            results_list_state: ListState::default(),
            is_searching: false,
            search_in_progress: false,
            show_help: false,
            current_page: 0,
            results_per_page: 20,
            total_results: 0,
        }
    }

    /// Render the advanced search interface
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        // Main layout: header, tabs, content, footer
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Length(3), // Tabs
                Constraint::Min(10),   // Content
                Constraint::Length(2), // Footer
            ])
            .split(area);

        // Render header
        self.render_header(frame, chunks[0]);

        // Render tabs
        self.render_tabs(frame, chunks[1]);

        // Render content based on current tab
        match self.current_tab {
            SearchTab::BasicSearch => self.render_basic_search(frame, chunks[2]),
            SearchTab::Filters => self.render_filters(frame, chunks[2]),
            SearchTab::Options => self.render_options(frame, chunks[2]),
            SearchTab::Results => self.render_results(frame, chunks[2]),
        }

        // Render footer
        self.render_footer(frame, chunks[3]);

        // Render help overlay if active
        if self.show_help {
            self.render_help_overlay(frame, area);
        }
    }

    /// Render header with title and search status
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let title = if self.search_in_progress {
            "🔍 Advanced Contact Search - Searching..."
        } else if !self.search_results.is_empty() {
            "🔍 Advanced Contact Search - Results Found"
        } else {
            "🔍 Advanced Contact Search"
        };

        let header = Paragraph::new(title)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center);

        frame.render_widget(header, area);
    }

    /// Render tab bar
    fn render_tabs(&self, frame: &mut Frame, area: Rect) {
        let tab_titles = vec!["Basic Search", "Filters", "Options", "Results"];
        
        let selected_index = match self.current_tab {
            SearchTab::BasicSearch => 0,
            SearchTab::Filters => 1,
            SearchTab::Options => 2,
            SearchTab::Results => 3,
        };

        let tabs = Tabs::new(tab_titles)
            .block(Block::default().borders(Borders::ALL))
            .select(selected_index)
            .style(Style::default().fg(Color::White))
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
                    .bg(Color::DarkGray),
            );

        frame.render_widget(tabs, area);
    }

    /// Render basic search tab
    fn render_basic_search(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // General query
                Constraint::Length(3), // Name query
                Constraint::Length(3), // Email query
                Constraint::Length(3), // Phone query
                Constraint::Length(3), // Company query
                Constraint::Min(3),    // Notes query
            ])
            .split(area);

        // General query field
        let general_style = if self.focused_field == SearchField::GeneralQuery {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let general_query = Paragraph::new(self.general_query.clone())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("General Search (searches all fields)")
                    .border_style(general_style),
            )
            .style(general_style);
        frame.render_widget(general_query, chunks[0]);

        // Name query field
        let name_style = if self.focused_field == SearchField::NameQuery {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let name_query = Paragraph::new(self.name_query.clone())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Name Search (first, last, display name)")
                    .border_style(name_style),
            )
            .style(name_style);
        frame.render_widget(name_query, chunks[1]);

        // Email query field
        let email_style = if self.focused_field == SearchField::EmailQuery {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let email_query = Paragraph::new(self.email_query.clone())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Email Search")
                    .border_style(email_style),
            )
            .style(email_style);
        frame.render_widget(email_query, chunks[2]);

        // Phone query field
        let phone_style = if self.focused_field == SearchField::PhoneQuery {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let phone_query = Paragraph::new(self.phone_query.clone())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Phone Search")
                    .border_style(phone_style),
            )
            .style(phone_style);
        frame.render_widget(phone_query, chunks[3]);

        // Company query field
        let company_style = if self.focused_field == SearchField::CompanyQuery {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let company_query = Paragraph::new(self.company_query.clone())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Company Search")
                    .border_style(company_style),
            )
            .style(company_style);
        frame.render_widget(company_query, chunks[4]);

        // Notes query field
        let notes_style = if self.focused_field == SearchField::NotesQuery {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let notes_query = Paragraph::new(self.notes_query.clone())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Notes Search")
                    .border_style(notes_style),
            )
            .style(notes_style)
            .wrap(Wrap { trim: true });
        frame.render_widget(notes_query, chunks[5]);
    }

    /// Render filters tab
    fn render_filters(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50), // Source filters
                Constraint::Percentage(50), // Presence filters
            ])
            .split(area);

        // Source filters
        self.render_source_filters(frame, chunks[0]);

        // Presence filters
        self.render_presence_filters(frame, chunks[1]);
    }

    /// Render source filters section
    fn render_source_filters(&self, frame: &mut Frame, area: Rect) {
        let source_style = if self.focused_field == SearchField::SourceFilters {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title("Contact Sources")
            .border_style(source_style);

        let inner_area = block.inner(area);
        frame.render_widget(block, area);

        let source_names = ["Google Contacts", "Outlook Contacts", "Local Contacts"];
        let mut source_lines = Vec::new();

        for (_i, (name, &enabled)) in source_names.iter().zip(&self.source_filters).enumerate() {
            let checkbox = if enabled { "☑" } else { "☐" };
            let color = if enabled { Color::Green } else { Color::Gray };
            
            let line = Line::from(vec![
                Span::styled(format!("{} ", checkbox), Style::default().fg(color)),
                Span::styled(*name, Style::default().fg(Color::White)),
            ]);
            source_lines.push(line);
        }

        let source_list = Paragraph::new(source_lines);
        frame.render_widget(source_list, inner_area);
    }

    /// Render presence filters section
    fn render_presence_filters(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Required Fields");

        let inner_area = block.inner(area);
        frame.render_widget(block, area);

        let mut filter_lines = Vec::new();

        // Email filter
        let email_text = match self.has_email_filter {
            Some(true) => "☑ Has Email",
            Some(false) => "☐ No Email",
            None => "○ Any Email",
        };
        let email_color = if self.focused_field == SearchField::EmailFilter {
            Color::Yellow
        } else {
            Color::White
        };
        filter_lines.push(Line::from(Span::styled(email_text, Style::default().fg(email_color))));

        // Phone filter
        let phone_text = match self.has_phone_filter {
            Some(true) => "☑ Has Phone",
            Some(false) => "☐ No Phone",
            None => "○ Any Phone",
        };
        let phone_color = if self.focused_field == SearchField::PhoneFilter {
            Color::Yellow
        } else {
            Color::White
        };
        filter_lines.push(Line::from(Span::styled(phone_text, Style::default().fg(phone_color))));

        // Company filter
        let company_text = match self.has_company_filter {
            Some(true) => "☑ Has Company",
            Some(false) => "☐ No Company",
            None => "○ Any Company",
        };
        let company_color = if self.focused_field == SearchField::CompanyFilter {
            Color::Yellow
        } else {
            Color::White
        };
        filter_lines.push(Line::from(Span::styled(company_text, Style::default().fg(company_color))));

        // Notes filter
        let notes_text = match self.has_notes_filter {
            Some(true) => "☑ Has Notes",
            Some(false) => "☐ No Notes",
            None => "○ Any Notes",
        };
        let notes_color = if self.focused_field == SearchField::NotesFilter {
            Color::Yellow
        } else {
            Color::White
        };
        filter_lines.push(Line::from(Span::styled(notes_text, Style::default().fg(notes_color))));

        let filters_list = Paragraph::new(filter_lines);
        frame.render_widget(filters_list, inner_area);
    }

    /// Render options tab
    fn render_options(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8), // Search options
                Constraint::Min(5),    // Action buttons
            ])
            .split(area);

        // Search options
        let options_block = Block::default()
            .borders(Borders::ALL)
            .title("Search Options");

        let inner_area = options_block.inner(chunks[0]);
        frame.render_widget(options_block, chunks[0]);

        let mut option_lines = Vec::new();

        // Fuzzy matching
        let fuzzy_text = if self.fuzzy_matching { "☑" } else { "☐" };
        let fuzzy_color = if self.focused_field == SearchField::FuzzyMatching {
            Color::Yellow
        } else {
            Color::White
        };
        option_lines.push(Line::from(vec![
            Span::styled(format!("{} ", fuzzy_text), Style::default().fg(fuzzy_color)),
            Span::styled("Fuzzy Matching", Style::default().fg(fuzzy_color)),
        ]));

        // Case sensitive
        let case_text = if self.case_sensitive { "☑" } else { "☐" };
        let case_color = if self.focused_field == SearchField::CaseSensitive {
            Color::Yellow
        } else {
            Color::White
        };
        option_lines.push(Line::from(vec![
            Span::styled(format!("{} ", case_text), Style::default().fg(case_color)),
            Span::styled("Case Sensitive", Style::default().fg(case_color)),
        ]));

        // Whole word only
        let word_text = if self.whole_word_only { "☑" } else { "☐" };
        let word_color = if self.focused_field == SearchField::WholeWordOnly {
            Color::Yellow
        } else {
            Color::White
        };
        option_lines.push(Line::from(vec![
            Span::styled(format!("{} ", word_text), Style::default().fg(word_color)),
            Span::styled("Whole Words Only", Style::default().fg(word_color)),
        ]));

        let options_list = Paragraph::new(option_lines);
        frame.render_widget(options_list, inner_area);

        // Action buttons
        self.render_action_buttons(frame, chunks[1]);
    }

    /// Render action buttons
    fn render_action_buttons(&self, frame: &mut Frame, area: Rect) {
        let button_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25), // Search button
                Constraint::Percentage(25), // Clear button
                Constraint::Percentage(25), // Help button
                Constraint::Percentage(25), // Exit button
            ])
            .split(area);

        // Search button
        let search_style = if self.focused_field == SearchField::SearchButton {
            Style::default().fg(Color::Black).bg(Color::Green)
        } else {
            Style::default().fg(Color::Green)
        };

        let search_text = if self.search_in_progress {
            "Searching..."
        } else {
            "Search"
        };

        let search_button = Paragraph::new(search_text)
            .block(Block::default().borders(Borders::ALL))
            .style(search_style)
            .alignment(Alignment::Center);
        frame.render_widget(search_button, button_chunks[0]);

        // Clear button
        let clear_style = if self.focused_field == SearchField::ClearButton {
            Style::default().fg(Color::Black).bg(Color::Red)
        } else {
            Style::default().fg(Color::Red)
        };

        let clear_button = Paragraph::new("Clear")
            .block(Block::default().borders(Borders::ALL))
            .style(clear_style)
            .alignment(Alignment::Center);
        frame.render_widget(clear_button, button_chunks[1]);

        // Help button
        let help_button = Paragraph::new("Help")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Center);
        frame.render_widget(help_button, button_chunks[2]);

        // Exit button
        let exit_button = Paragraph::new("Exit")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        frame.render_widget(exit_button, button_chunks[3]);
    }

    /// Render results tab
    fn render_results(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Results summary
                Constraint::Min(10),   // Results list
                Constraint::Length(3), // Pagination
            ])
            .split(area);

        // Results summary
        let summary_text = if self.search_results.is_empty() {
            "No results found. Try adjusting your search criteria.".to_string()
        } else {
            format!(
                "Found {} contacts (showing {}-{} of {})",
                self.total_results,
                self.current_page * self.results_per_page + 1,
                std::cmp::min((self.current_page + 1) * self.results_per_page, self.total_results),
                self.total_results
            )
        };

        let summary = Paragraph::new(summary_text)
            .block(Block::default().borders(Borders::ALL).title("Search Results"))
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Center);
        frame.render_widget(summary, chunks[0]);

        // Results list
        if !self.search_results.is_empty() {
            self.render_results_list(frame, chunks[1]);
        } else {
            let empty_message = Paragraph::new("No search results to display.\n\nTry:\n• Broadening your search terms\n• Using fewer filters\n• Enabling fuzzy matching")
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::Gray))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });
            frame.render_widget(empty_message, chunks[1]);
        }

        // Pagination controls
        if self.total_results > self.results_per_page {
            self.render_pagination(frame, chunks[2]);
        }
    }

    /// Render results list
    fn render_results_list(&mut self, frame: &mut Frame, area: Rect) {
        let list_items: Vec<ListItem> = self.search_results
            .iter()
            .enumerate()
            .map(|(i, result)| {
                let contact = &result.contact;
                let relevance_bar = "█".repeat((result.relevance_score / 2.0) as usize);
                
                let email_text = contact
                    .primary_email()
                    .map(|e| format!(" <{}>", e.address))
                    .unwrap_or_default();

                let company_text = contact
                    .company
                    .as_ref()
                    .map(|c| format!(" - {}", c))
                    .unwrap_or_default();

                let line = Line::from(vec![
                    Span::styled(
                        format!("{:2}. ", i + 1),
                        Style::default().fg(Color::Gray),
                    ),
                    Span::styled(
                        contact.display_name.clone(),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(email_text, Style::default().fg(Color::Cyan)),
                    Span::styled(company_text, Style::default().fg(Color::Yellow)),
                    Span::styled(
                        format!(" [{}] {:.1}", relevance_bar, result.relevance_score),
                        Style::default().fg(Color::Green),
                    ),
                ]);

                ListItem::new(line)
            })
            .collect();

        let results_style = if self.focused_field == SearchField::ResultsList {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let results_list = List::new(list_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Contacts")
                    .border_style(results_style),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(results_list, area, &mut self.results_list_state);
    }

    /// Render pagination controls
    fn render_pagination(&self, frame: &mut Frame, area: Rect) {
        let total_pages = (self.total_results + self.results_per_page - 1) / self.results_per_page;
        let current_page = self.current_page + 1; // Display 1-based page numbers

        let pagination_text = format!(
            "Page {} of {} | ← → Navigate | Enter View | Ctrl+E Export",
            current_page,
            total_pages
        );

        let pagination = Paragraph::new(pagination_text)
            .block(Block::default().borders(Borders::ALL).title("Navigation"))
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);

        frame.render_widget(pagination, area);
    }

    /// Render footer with shortcuts
    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let shortcuts = match self.current_tab {
            SearchTab::BasicSearch => "Tab: Next Field | Ctrl+F: Search | Ctrl+C: Clear | F1: Help | Esc: Exit",
            SearchTab::Filters => "↑↓: Navigate | Space: Toggle | Tab: Switch Tabs | F1: Help",
            SearchTab::Options => "↑↓: Navigate | Space: Toggle | Ctrl+F: Search | F1: Help",
            SearchTab::Results => "↑↓: Navigate | Enter: View | E: Edit | C: Compose | F1: Help",
        };

        let footer = Paragraph::new(shortcuts)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);

        frame.render_widget(footer, area);
    }

    /// Render help overlay
    fn render_help_overlay(&self, frame: &mut Frame, area: Rect) {
        // Calculate overlay area (80% of screen)
        let popup_area = self.centered_rect(80, 80, area);

        // Clear background
        frame.render_widget(Clear, popup_area);

        let help_text = vec![
            Line::from("Advanced Contact Search Help"),
            Line::from(""),
            Line::from("Search Tabs:"),
            Line::from("  • Basic Search: Enter search terms for different fields"),
            Line::from("  • Filters: Filter by contact source and field presence"),
            Line::from("  • Options: Configure search behavior and sorting"),
            Line::from("  • Results: View and interact with search results"),
            Line::from(""),
            Line::from("Search Features:"),
            Line::from("  • Fuzzy Matching: Find contacts with similar spellings"),
            Line::from("  • Case Sensitive: Exact case matching"),
            Line::from("  • Whole Words: Match complete words only"),
            Line::from("  • Relevance Scoring: Results ranked by match quality"),
            Line::from(""),
            Line::from("Keyboard Shortcuts:"),
            Line::from("  • Tab/Shift+Tab: Navigate between tabs"),
            Line::from("  • ↑↓←→: Navigate within tabs"),
            Line::from("  • Space: Toggle options/filters"),
            Line::from("  • Enter: Activate/view selected item"),
            Line::from("  • Ctrl+F: Perform search"),
            Line::from("  • Ctrl+C: Clear all search criteria"),
            Line::from("  • F1: Show/hide this help"),
            Line::from("  • Esc: Exit advanced search"),
            Line::from(""),
            Line::from("Press F1 or Esc to close this help"),
        ];

        let help_paragraph = Paragraph::new(help_text)
            .block(
                Block::default()
                    .title("Help")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true });

        frame.render_widget(help_paragraph, popup_area);
    }

    /// Calculate centered rectangle for overlays
    fn centered_rect(&self, percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }

    /// Handle keyboard input
    pub async fn handle_key(&mut self, key: crossterm::event::KeyCode) -> (bool, Option<SearchAction>) {
        use crossterm::event::KeyCode;

        // Global shortcuts
        match key {
            KeyCode::Esc => return (false, Some(SearchAction::Exit)),
            KeyCode::F(1) => {
                self.show_help = !self.show_help;
                return (true, None);
            }
            KeyCode::Tab => {
                self.next_tab();
                return (true, None);
            }
            KeyCode::BackTab => {
                self.previous_tab();
                return (true, None);
            }
            _ => {}
        }

        // Handle tab-specific input
        match self.current_tab {
            SearchTab::BasicSearch => self.handle_basic_search_key(key).await,
            SearchTab::Filters => self.handle_filters_key(key).await,
            SearchTab::Options => self.handle_options_key(key).await,
            SearchTab::Results => self.handle_results_key(key).await,
        }
    }

    /// Handle basic search tab input
    async fn handle_basic_search_key(&mut self, key: crossterm::event::KeyCode) -> (bool, Option<SearchAction>) {
        use crossterm::event::KeyCode;

        match key {
            KeyCode::Up => self.previous_basic_field(),
            KeyCode::Down => self.next_basic_field(),
            KeyCode::Char(c) => {
                match c {
                    'f' if self.is_ctrl() => return (true, Some(SearchAction::PerformSearch)),
                    'c' if self.is_ctrl() => return (true, Some(SearchAction::ClearSearch)),
                    _ => self.handle_text_input(c),
                }
            }
            KeyCode::Backspace => self.handle_backspace(),
            KeyCode::Enter => return (true, Some(SearchAction::PerformSearch)),
            _ => {}
        }

        (true, None)
    }

    /// Handle filters tab input
    async fn handle_filters_key(&mut self, key: crossterm::event::KeyCode) -> (bool, Option<SearchAction>) {
        use crossterm::event::KeyCode;

        match key {
            KeyCode::Up => self.previous_filter_field(),
            KeyCode::Down => self.next_filter_field(),
            KeyCode::Left | KeyCode::Right => self.navigate_filter_options(),
            KeyCode::Char(' ') => self.toggle_current_filter(),
            _ => {}
        }

        (true, None)
    }

    /// Handle options tab input
    async fn handle_options_key(&mut self, key: crossterm::event::KeyCode) -> (bool, Option<SearchAction>) {
        use crossterm::event::KeyCode;

        match key {
            KeyCode::Up => self.previous_option_field(),
            KeyCode::Down => self.next_option_field(),
            KeyCode::Char(' ') => self.toggle_current_option(),
            KeyCode::Enter => return self.activate_current_option().await,
            KeyCode::Char(c) => {
                match c {
                    'f' if self.is_ctrl() => return (true, Some(SearchAction::PerformSearch)),
                    'c' if self.is_ctrl() => return (true, Some(SearchAction::ClearSearch)),
                    _ => {}
                }
            }
            _ => {}
        }

        (true, None)
    }

    /// Handle results tab input
    async fn handle_results_key(&mut self, key: crossterm::event::KeyCode) -> (bool, Option<SearchAction>) {
        use crossterm::event::KeyCode;

        match key {
            KeyCode::Up => self.previous_result(),
            KeyCode::Down => self.next_result(),
            KeyCode::Left => self.previous_page(),
            KeyCode::Right => self.next_page(),
            KeyCode::Enter => {
                if let Some(contact) = self.get_selected_contact() {
                    if let Some(contact_id) = contact.id {
                        return (true, Some(SearchAction::ViewContact(contact_id)));
                    }
                }
            }
            KeyCode::Char(c) => {
                match c {
                    'e' => {
                        if let Some(contact) = self.get_selected_contact() {
                            if let Some(contact_id) = contact.id {
                                return (true, Some(SearchAction::EditContact(contact_id)));
                            }
                        }
                    }
                    'c' => {
                        if let Some(contact) = self.get_selected_contact() {
                            if let Some(primary_email) = contact.primary_email() {
                                return (true, Some(SearchAction::ComposeEmail {
                                    to: primary_email.address.clone(),
                                    name: contact.display_name.clone(),
                                }));
                            }
                        }
                    }
                    'E' if self.is_ctrl() => return (true, Some(SearchAction::ExportResults)),
                    'S' if self.is_ctrl() => return (true, Some(SearchAction::SaveSearch)),
                    _ => {}
                }
            }
            _ => {}
        }

        (true, None)
    }

    // Navigation helper methods

    fn next_tab(&mut self) {
        self.current_tab = match self.current_tab {
            SearchTab::BasicSearch => SearchTab::Filters,
            SearchTab::Filters => SearchTab::Options,
            SearchTab::Options => SearchTab::Results,
            SearchTab::Results => SearchTab::BasicSearch,
        };
        self.update_focused_field_for_tab();
    }

    fn previous_tab(&mut self) {
        self.current_tab = match self.current_tab {
            SearchTab::BasicSearch => SearchTab::Results,
            SearchTab::Filters => SearchTab::BasicSearch,
            SearchTab::Options => SearchTab::Filters,
            SearchTab::Results => SearchTab::Options,
        };
        self.update_focused_field_for_tab();
    }

    fn update_focused_field_for_tab(&mut self) {
        self.focused_field = match self.current_tab {
            SearchTab::BasicSearch => SearchField::GeneralQuery,
            SearchTab::Filters => SearchField::SourceFilters,
            SearchTab::Options => SearchField::FuzzyMatching,
            SearchTab::Results => SearchField::ResultsList,
        };
    }

    fn next_basic_field(&mut self) {
        self.focused_field = match self.focused_field {
            SearchField::GeneralQuery => SearchField::NameQuery,
            SearchField::NameQuery => SearchField::EmailQuery,
            SearchField::EmailQuery => SearchField::PhoneQuery,
            SearchField::PhoneQuery => SearchField::CompanyQuery,
            SearchField::CompanyQuery => SearchField::NotesQuery,
            SearchField::NotesQuery => SearchField::GeneralQuery,
            _ => SearchField::GeneralQuery,
        };
    }

    fn previous_basic_field(&mut self) {
        self.focused_field = match self.focused_field {
            SearchField::GeneralQuery => SearchField::NotesQuery,
            SearchField::NameQuery => SearchField::GeneralQuery,
            SearchField::EmailQuery => SearchField::NameQuery,
            SearchField::PhoneQuery => SearchField::EmailQuery,
            SearchField::CompanyQuery => SearchField::PhoneQuery,
            SearchField::NotesQuery => SearchField::CompanyQuery,
            _ => SearchField::GeneralQuery,
        };
    }

    // Implement other navigation and input handling methods...
    // (The implementation would continue with similar patterns for other methods)

    /// Check if Ctrl is pressed (placeholder implementation)
    fn is_ctrl(&self) -> bool {
        // In a real implementation, this would check modifier keys
        false
    }

    /// Handle text input for current field
    fn handle_text_input(&mut self, c: char) {
        match self.focused_field {
            SearchField::GeneralQuery => self.general_query.push(c),
            SearchField::NameQuery => self.name_query.push(c),
            SearchField::EmailQuery => self.email_query.push(c),
            SearchField::PhoneQuery => self.phone_query.push(c),
            SearchField::CompanyQuery => self.company_query.push(c),
            SearchField::NotesQuery => self.notes_query.push(c),
            _ => {}
        }
    }

    /// Handle backspace for current field
    fn handle_backspace(&mut self) {
        match self.focused_field {
            SearchField::GeneralQuery => { self.general_query.pop(); }
            SearchField::NameQuery => { self.name_query.pop(); }
            SearchField::EmailQuery => { self.email_query.pop(); }
            SearchField::PhoneQuery => { self.phone_query.pop(); }
            SearchField::CompanyQuery => { self.company_query.pop(); }
            SearchField::NotesQuery => { self.notes_query.pop(); }
            _ => {}
        }
    }

    /// Get currently selected contact from results
    fn get_selected_contact(&self) -> Option<&crate::contacts::Contact> {
        if let Some(selected) = self.results_list_state.selected() {
            self.search_results.get(selected).map(|r| &r.contact)
        } else {
            None
        }
    }

    // Placeholder implementations for remaining methods
    fn previous_filter_field(&mut self) { /* Implementation */ }
    fn next_filter_field(&mut self) { /* Implementation */ }
    fn navigate_filter_options(&mut self) { /* Implementation */ }
    fn toggle_current_filter(&mut self) { /* Implementation */ }
    
    fn previous_option_field(&mut self) { /* Implementation */ }
    fn next_option_field(&mut self) { /* Implementation */ }
    fn toggle_current_option(&mut self) { /* Implementation */ }
    async fn activate_current_option(&mut self) -> (bool, Option<SearchAction>) { (true, None) }
    
    fn previous_result(&mut self) { /* Implementation */ }
    fn next_result(&mut self) { /* Implementation */ }
    fn previous_page(&mut self) { /* Implementation */ }
    fn next_page(&mut self) { /* Implementation */ }

    /// Build search criteria from UI inputs
    pub fn build_search_criteria(&self) -> AdvancedSearchCriteria {
        let mut criteria = AdvancedSearchCriteria::new();

        // Set queries
        if !self.general_query.is_empty() {
            criteria = criteria.with_query(self.general_query.clone());
        }
        if !self.name_query.is_empty() {
            criteria = criteria.with_name_query(self.name_query.clone());
        }
        if !self.email_query.is_empty() {
            criteria = criteria.with_email_query(self.email_query.clone());
        }
        if !self.phone_query.is_empty() {
            criteria = criteria.with_phone_query(self.phone_query.clone());
        }
        if !self.company_query.is_empty() {
            criteria = criteria.with_company_query(self.company_query.clone());
        }

        // Set filters
        let mut sources = Vec::new();
        if self.source_filters[0] {
            sources.push(ContactSource::Google { account_id: "default".to_string() });
        }
        if self.source_filters[1] {
            sources.push(ContactSource::Outlook { account_id: "default".to_string() });
        }
        if self.source_filters[2] {
            sources.push(ContactSource::Local);
        }
        if !sources.is_empty() {
            criteria = criteria.with_sources(sources);
        }

        // Set presence filters
        if let Some(has_email) = self.has_email_filter {
            criteria = criteria.with_email_filter(has_email);
        }
        if let Some(has_phone) = self.has_phone_filter {
            criteria = criteria.with_phone_filter(has_phone);
        }

        // Set options
        criteria = criteria.with_fuzzy_matching(self.fuzzy_matching);
        criteria.case_sensitive = self.case_sensitive;
        criteria.whole_word_only = self.whole_word_only;

        // Set sorting (default to relevance)
        criteria = criteria.with_sort(SortField::Relevance, SortDirection::Descending);

        criteria
    }

    /// Perform search with current criteria
    pub async fn perform_search(&mut self) -> Result<(), String> {
        self.search_in_progress = true;
        let criteria = self.build_search_criteria();

        // Use the basic search for now (in a real implementation, would use AdvancedContactSearch)
        let basic_criteria = criteria.to_basic_criteria();
        
        match self.manager.search_contacts(&basic_criteria).await {
            Ok(contacts) => {
                // Convert contacts to search results with mock relevance scores
                self.search_results = contacts.into_iter().map(|contact| {
                    SearchResult {
                        relevance_score: 5.0, // Mock score
                        matching_fields: vec!["display_name".to_string()],
                        snippets: std::collections::HashMap::new(),
                        contact,
                    }
                }).collect();
                
                self.total_results = self.search_results.len();
                self.current_page = 0;
                self.current_tab = SearchTab::Results;
                self.focused_field = SearchField::ResultsList;
                self.results_list_state.select(if self.search_results.is_empty() { None } else { Some(0) });
            }
            Err(e) => {
                tracing::error!("Search failed: {}", e);
                return Err(format!("Search failed: {}", e));
            }
        }

        self.search_in_progress = false;
        Ok(())
    }

    /// Clear all search criteria
    pub fn clear_search(&mut self) {
        self.general_query.clear();
        self.name_query.clear();
        self.email_query.clear();
        self.phone_query.clear();
        self.company_query.clear();
        self.notes_query.clear();
        
        self.source_filters = vec![true, true, true];
        self.has_email_filter = None;
        self.has_phone_filter = None;
        self.has_company_filter = None;
        self.has_notes_filter = None;
        
        self.fuzzy_matching = false;
        self.case_sensitive = false;
        self.whole_word_only = false;
        
        self.search_results.clear();
        self.total_results = 0;
        self.current_page = 0;
        self.results_list_state.select(None);
        
        self.current_tab = SearchTab::BasicSearch;
        self.focused_field = SearchField::GeneralQuery;
    }
}