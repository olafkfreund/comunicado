use crate::email::database::StoredDraft;
use crate::theme::Theme;
use chrono::Local;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

/// Draft list UI component for browsing and managing saved drafts
pub struct DraftListUI {
    drafts: Vec<StoredDraft>,
    selected_index: usize,
    list_state: ListState,
    sort_by: DraftSortBy,
    sort_ascending: bool,
    is_visible: bool,
    show_details: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DraftSortBy {
    CreatedAt,
    UpdatedAt,
    Subject,
    Recipient,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DraftAction {
    Continue,
    LoadDraft(String),   // draft_id
    DeleteDraft(String), // draft_id
    Close,
    ToggleSort,
    ToggleDetails,
    RefreshDrafts,
}

impl DraftListUI {
    /// Create a new draft list UI
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            drafts: Vec::new(),
            selected_index: 0,
            list_state,
            sort_by: DraftSortBy::UpdatedAt,
            sort_ascending: false, // Most recent first
            is_visible: false,
            show_details: false,
        }
    }

    /// Show the draft list UI
    pub fn show(&mut self) {
        self.is_visible = true;
    }

    /// Hide the draft list UI
    pub fn hide(&mut self) {
        self.is_visible = false;
    }

    /// Check if the draft list is visible
    pub fn is_visible(&self) -> bool {
        self.is_visible
    }

    /// Update the list of drafts
    pub fn update_drafts(&mut self, mut drafts: Vec<StoredDraft>) {
        // Sort drafts according to current settings
        self.sort_drafts(&mut drafts);

        self.drafts = drafts;

        // Reset selection if needed
        if self.selected_index >= self.drafts.len() {
            self.selected_index = if self.drafts.is_empty() {
                0
            } else {
                self.drafts.len() - 1
            };
        }

        // Update list state
        if !self.drafts.is_empty() {
            self.list_state.select(Some(self.selected_index));
        } else {
            self.list_state.select(None);
        }
    }

    /// Sort drafts according to current criteria
    fn sort_drafts(&self, drafts: &mut Vec<StoredDraft>) {
        drafts.sort_by(|a, b| {
            let order = match self.sort_by {
                DraftSortBy::CreatedAt => a.created_at.cmp(&b.created_at),
                DraftSortBy::UpdatedAt => a.updated_at.cmp(&b.updated_at),
                DraftSortBy::Subject => a.subject.cmp(&b.subject),
                DraftSortBy::Recipient => {
                    let a_to = a.to_addrs.first().cloned().unwrap_or_default();
                    let b_to = b.to_addrs.first().cloned().unwrap_or_default();
                    a_to.cmp(&b_to)
                }
            };

            if self.sort_ascending {
                order
            } else {
                order.reverse()
            }
        });
    }

    /// Handle keyboard input
    pub async fn handle_key(&mut self, key: crossterm::event::KeyCode) -> DraftAction {
        use crossterm::event::KeyCode;

        match key {
            KeyCode::Esc => DraftAction::Close,
            KeyCode::Up => {
                if !self.drafts.is_empty() && self.selected_index > 0 {
                    self.selected_index -= 1;
                    self.list_state.select(Some(self.selected_index));
                }
                DraftAction::Continue
            }
            KeyCode::Down => {
                if !self.drafts.is_empty() && self.selected_index < self.drafts.len() - 1 {
                    self.selected_index += 1;
                    self.list_state.select(Some(self.selected_index));
                }
                DraftAction::Continue
            }
            KeyCode::Enter => {
                // Load selected draft
                if let Some(draft) = self.drafts.get(self.selected_index) {
                    DraftAction::LoadDraft(draft.id.clone())
                } else {
                    DraftAction::Continue
                }
            }
            KeyCode::Delete | KeyCode::Char('d') => {
                // Delete selected draft
                if let Some(draft) = self.drafts.get(self.selected_index) {
                    DraftAction::DeleteDraft(draft.id.clone())
                } else {
                    DraftAction::Continue
                }
            }
            KeyCode::Char('s') => {
                // Toggle sort
                DraftAction::ToggleSort
            }
            KeyCode::Char('i') | KeyCode::Tab => {
                // Toggle details view
                DraftAction::ToggleDetails
            }
            KeyCode::F(5) | KeyCode::Char('r') => {
                // Refresh drafts
                DraftAction::RefreshDrafts
            }
            _ => DraftAction::Continue,
        }
    }

    /// Toggle sort order
    pub fn toggle_sort(&mut self) {
        // Cycle through sort options
        self.sort_by = match self.sort_by {
            DraftSortBy::UpdatedAt => DraftSortBy::CreatedAt,
            DraftSortBy::CreatedAt => DraftSortBy::Subject,
            DraftSortBy::Subject => DraftSortBy::Recipient,
            DraftSortBy::Recipient => DraftSortBy::UpdatedAt,
        };

        // Re-sort drafts with new criteria
        let mut drafts = self.drafts.clone();
        self.sort_drafts(&mut drafts);
        self.drafts = drafts;
    }

    /// Toggle details view
    pub fn toggle_details(&mut self) {
        self.show_details = !self.show_details;
    }

    /// Get currently selected draft
    pub fn get_selected_draft(&self) -> Option<&StoredDraft> {
        self.drafts.get(self.selected_index)
    }

    /// Remove a draft from the list
    pub fn remove_draft(&mut self, draft_id: &str) {
        if let Some(pos) = self.drafts.iter().position(|d| d.id == draft_id) {
            self.drafts.remove(pos);

            // Adjust selection
            if self.selected_index >= self.drafts.len() && !self.drafts.is_empty() {
                self.selected_index = self.drafts.len() - 1;
            }

            // Update list state
            if !self.drafts.is_empty() {
                self.list_state.select(Some(self.selected_index));
            } else {
                self.list_state.select(None);
            }
        }
    }

    /// Render the draft list UI
    pub fn render(&mut self, f: &mut Frame, area: Rect, theme: &Theme) {
        if !self.is_visible {
            return;
        }

        // Clear the background
        f.render_widget(Clear, area);

        // Main container
        let block = Block::default()
            .title("Draft Manager")
            .borders(Borders::ALL)
            .border_style(theme.get_component_style("popup", true));

        let inner = block.inner(area);
        f.render_widget(block, area);

        if self.drafts.is_empty() {
            self.render_empty_state(f, inner, theme);
            return;
        }

        // Layout: list + optional details
        let chunks = if self.show_details {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(50), Constraint::Length(40)])
                .split(inner)
        } else {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(100)])
                .split(inner)
        };

        // Render draft list
        self.render_draft_list(f, chunks[0], theme);

        // Render details if enabled
        if self.show_details && chunks.len() > 1 {
            self.render_draft_details(f, chunks[1], theme);
        }

        // Render status line
        self.render_status_line(f, area, theme);
    }

    /// Render empty state when no drafts are available
    fn render_empty_state(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let text = Text::from(vec![
            Line::from(""),
            Line::from("No drafts found"),
            Line::from(""),
            Line::from("Press 'c' to compose a new email"),
            Line::from("Press Esc to return to main view"),
        ]);

        let paragraph = Paragraph::new(text)
            .style(theme.get_component_style("text", false))
            .alignment(Alignment::Center)
            .wrap(ratatui::widgets::Wrap { trim: true });

        f.render_widget(paragraph, area);
    }

    /// Render the list of drafts
    fn render_draft_list(&mut self, f: &mut Frame, area: Rect, theme: &Theme) {
        // Extract data to avoid borrowing issues
        let selected_index = self.selected_index;
        let drafts_len = self.drafts.len();
        let sort_by = self.sort_by.clone();
        let sort_ascending = self.sort_ascending;

        // Pre-create all list items without self borrow conflicts
        let mut items = Vec::new();
        for (i, draft) in self.drafts.iter().enumerate() {
            let is_selected = i == selected_index;
            items.push(Self::create_draft_list_item_static(
                draft,
                is_selected,
                theme,
            ));
        }

        let list = List::new(items)
            .block(
                Block::default()
                    .title(format!(
                        "Drafts ({}) - Sort: {:?} {}",
                        drafts_len,
                        sort_by,
                        if sort_ascending { "↑" } else { "↓" }
                    ))
                    .borders(Borders::ALL)
                    .border_style(theme.get_component_style("border", true)),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        f.render_stateful_widget(list, area, &mut self.list_state);
    }

    /// Create a list item for a draft (static version to avoid borrowing issues)
    fn create_draft_list_item_static<'a>(
        draft: &'a StoredDraft,
        is_selected: bool,
        theme: &'a Theme,
    ) -> ListItem<'a> {
        // Get recipients
        let to_display = draft
            .to_addrs
            .first()
            .map(|r| {
                if r.len() > 25 {
                    format!("{}...", &r[..22])
                } else {
                    r.clone()
                }
            })
            .unwrap_or_else(|| "No recipient".to_string());

        // Format dates
        let updated_time = draft
            .updated_at
            .with_timezone(&Local)
            .format("%m/%d %H:%M")
            .to_string();

        // Truncate subject
        let subject_display = if draft.subject.is_empty() {
            "(No subject)".to_string()
        } else if draft.subject.len() > 30 {
            format!("{}...", &draft.subject[..27])
        } else {
            draft.subject.clone()
        };

        // Create line with draft info
        let auto_saved_indicator = if draft.auto_saved { " [Auto]" } else { "" };
        let line_text = format!(
            "{} │ {} │ {}{}",
            updated_time, to_display, subject_display, auto_saved_indicator
        );

        let style = if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else if draft.auto_saved {
            Style::default().fg(Color::Gray)
        } else {
            theme.get_component_style("text", false)
        };

        ListItem::new(line_text).style(style)
    }

    /// Render draft details in side panel
    fn render_draft_details(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        if let Some(draft) = self.get_selected_draft() {
            let block = Block::default()
                .title("Draft Details")
                .borders(Borders::ALL)
                .border_style(theme.get_component_style("border", false));

            let inner = block.inner(area);
            f.render_widget(block, area);

            // Get recipients (already parsed Vec<String>)
            let to_addrs = &draft.to_addrs;
            let cc_addrs = &draft.cc_addrs;
            let bcc_addrs = &draft.bcc_addrs;

            // Format dates
            let created_time = draft
                .created_at
                .with_timezone(&Local)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string();
            let updated_time = draft
                .updated_at
                .with_timezone(&Local)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string();

            let mut lines = vec![
                Line::from(format!("ID: {}", &draft.id[..8])),
                Line::from(""),
                Line::from(format!("To: {}", to_addrs.join(", "))),
            ];

            if !cc_addrs.is_empty() {
                lines.push(Line::from(format!("Cc: {}", cc_addrs.join(", "))));
            }
            if !bcc_addrs.is_empty() {
                lines.push(Line::from(format!("Bcc: {}", bcc_addrs.join(", "))));
            }

            lines.extend(vec![
                Line::from(""),
                Line::from(format!(
                    "Subject: {}",
                    if draft.subject.is_empty() {
                        "(No subject)"
                    } else {
                        &draft.subject
                    }
                )),
                Line::from(""),
                Line::from(format!("Created: {}", created_time)),
                Line::from(format!("Updated: {}", updated_time)),
                Line::from(format!(
                    "Auto-saved: {}",
                    if draft.auto_saved { "Yes" } else { "No" }
                )),
                Line::from(""),
                Line::from("Body Preview:"),
                Line::from("─".repeat(inner.width as usize)),
            ]);

            // Add body preview (first few lines)
            let body_lines: Vec<&str> = draft.body_text.lines().take(8).collect();
            for line in body_lines {
                let truncated = if line.len() > inner.width as usize - 2 {
                    format!("{}...", &line[..inner.width as usize - 5])
                } else {
                    line.to_string()
                };
                lines.push(Line::from(truncated));
            }

            if draft.body_text.lines().count() > 8 {
                lines.push(Line::from("..."));
            }

            let text = Text::from(lines);
            let paragraph = Paragraph::new(text)
                .style(theme.get_component_style("text", false))
                .wrap(ratatui::widgets::Wrap { trim: true });

            f.render_widget(paragraph, inner);
        }
    }

    /// Render status line with shortcuts
    fn render_status_line(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let status_area = Rect {
            x: area.x,
            y: area.y + area.height - 1,
            width: area.width,
            height: 1,
        };

        let status_text = if self.drafts.is_empty() {
            "Esc Close | c Compose"
        } else {
            "Enter Load | d Delete | s Sort | Tab Details | F5 Refresh | Esc Close"
        };

        let status = Paragraph::new(status_text)
            .style(theme.get_component_style("status", false))
            .alignment(Alignment::Center);

        f.render_widget(status, status_area);
    }
}

impl Default for DraftListUI {
    fn default() -> Self {
        Self::new()
    }
}
