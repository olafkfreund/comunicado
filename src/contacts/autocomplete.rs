use crate::contacts::{ContactsManager, ContactSearchCriteria};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
    Frame,
};
use std::sync::Arc;

/// Contact autocomplete suggestions
#[derive(Debug, Clone)]
pub struct ContactSuggestion {
    pub contact_id: Option<i64>,
    pub display_name: String,
    pub email: String,
    pub source: String,
}

/// Contact autocomplete widget for email composition
pub struct ContactAutocomplete {
    manager: Arc<ContactsManager>,
    suggestions: Vec<ContactSuggestion>,
    list_state: ListState,
    query: String,
    is_visible: bool,
    is_focused: bool,
}

impl ContactAutocomplete {
    pub fn new(manager: Arc<ContactsManager>) -> Self {
        Self {
            manager,
            suggestions: Vec::new(),
            list_state: ListState::default(),
            query: String::new(),
            is_visible: false,
            is_focused: false,
        }
    }

    /// Update autocomplete suggestions based on query
    pub async fn update_suggestions(&mut self, query: &str) {
        self.query = query.to_string();
        
        if query.len() < 2 {
            self.suggestions.clear();
            self.is_visible = false;
            return;
        }

        // Search contacts by email and name
        let criteria = ContactSearchCriteria::new()
            .with_query(query.to_string())
            .with_limit(10);

        match self.manager.search_contacts(&criteria).await {
            Ok(contacts) => {
                self.suggestions = contacts
                    .into_iter()
                    .flat_map(|contact| {
                        contact.emails.into_iter().filter_map(move |email| {
                            if email.address.to_lowercase().contains(&query.to_lowercase()) ||
                               contact.display_name.to_lowercase().contains(&query.to_lowercase()) {
                                Some(ContactSuggestion {
                                    contact_id: contact.id,
                                    display_name: contact.display_name.clone(),
                                    email: email.address,
                                    source: contact.source.provider_name().to_string(),
                                })
                            } else {
                                None
                            }
                        })
                    })
                    .collect();

                self.is_visible = !self.suggestions.is_empty();
                if self.is_visible {
                    self.list_state.select(Some(0));
                }
            }
            Err(e) => {
                tracing::error!("Failed to search contacts: {}", e);
                self.suggestions.clear();
                self.is_visible = false;
            }
        }
    }

    /// Get selected suggestion
    pub fn get_selected_suggestion(&self) -> Option<&ContactSuggestion> {
        if let Some(selected) = self.list_state.selected() {
            self.suggestions.get(selected)
        } else {
            None
        }
    }

    /// Select next suggestion
    pub fn select_next(&mut self) {
        if self.suggestions.is_empty() {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => (i + 1) % self.suggestions.len(),
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    /// Select previous suggestion
    pub fn select_previous(&mut self) {
        if self.suggestions.is_empty() {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.suggestions.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    /// Hide autocomplete
    pub fn hide(&mut self) {
        self.is_visible = false;
        self.is_focused = false;
        self.suggestions.clear();
        self.query.clear();
    }

    /// Show autocomplete
    pub fn show(&mut self) {
        if !self.suggestions.is_empty() {
            self.is_visible = true;
            self.list_state.select(Some(0));
        }
    }

    /// Set focus state
    pub fn set_focus(&mut self, focused: bool) {
        self.is_focused = focused;
    }

    /// Check if autocomplete is visible
    pub fn is_visible(&self) -> bool {
        self.is_visible
    }

    /// Check if autocomplete is focused
    pub fn is_focused(&self) -> bool {
        self.is_focused
    }

    /// Render autocomplete popup
    pub fn render(&mut self, frame: &mut Frame, area: Rect, anchor_x: u16, anchor_y: u16) {
        if !self.is_visible || self.suggestions.is_empty() {
            return;
        }

        // Calculate popup position and size
        let popup_height = (self.suggestions.len() + 2).min(8) as u16; // +2 for borders, max 8 rows
        let popup_width = 60.min(area.width);
        
        let popup_x = anchor_x.min(area.width.saturating_sub(popup_width));
        let popup_y = if anchor_y + popup_height <= area.height {
            anchor_y + 1 // Show below anchor
        } else {
            anchor_y.saturating_sub(popup_height) // Show above anchor
        };

        let popup_area = Rect {
            x: popup_x,
            y: popup_y,
            width: popup_width,
            height: popup_height,
        };

        // Clear background
        frame.render_widget(Clear, popup_area);

        // Create list items
        let items: Vec<ListItem> = self.suggestions
            .iter()
            .map(|suggestion| {
                let line = Line::from(vec![
                    Span::styled(
                        suggestion.display_name.clone(),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" <", Style::default().fg(Color::Gray)),
                    Span::styled(
                        suggestion.email.clone(),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::styled(">", Style::default().fg(Color::Gray)),
                    Span::styled(
                        format!(" ({})", suggestion.source),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]);
                ListItem::new(line)
            })
            .collect();

        // Create and render list
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Contact Suggestions")
                    .border_style(if self.is_focused {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::White)
                    }),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(list, popup_area, &mut self.list_state);
    }

    /// Format selected suggestion for insertion
    pub fn format_suggestion(&self, suggestion: &ContactSuggestion) -> String {
        if suggestion.display_name.is_empty() {
            suggestion.email.clone()
        } else {
            format!("{} <{}>", suggestion.display_name, suggestion.email)
        }
    }

    /// Get frequent contacts for quick access
    pub async fn get_frequent_contacts(&self, limit: usize) -> Vec<ContactSuggestion> {
        match self.manager.get_frequent_contacts(limit).await {
            Ok(contacts) => contacts
                .into_iter()
                .filter_map(|contact| {
                    let display_name = contact.display_name.clone();
                    let contact_id = contact.id;
                    let source = contact.source.provider_name().to_string();
                    
                    contact.primary_email().map(|email| ContactSuggestion {
                        contact_id,
                        display_name,
                        email: email.address.clone(),
                        source,
                    })
                })
                .collect(),
            Err(e) => {
                tracing::error!("Failed to get frequent contacts: {}", e);
                Vec::new()
            }
        }
    }

    /// Search contacts by email domain for organization-wide suggestions
    pub async fn search_by_domain(&mut self, domain: &str) -> Vec<ContactSuggestion> {
        if domain.is_empty() {
            return Vec::new();
        }

        let criteria = ContactSearchCriteria::new()
            .with_email(format!("@{}", domain))
            .with_limit(20);

        match self.manager.search_contacts(&criteria).await {
            Ok(contacts) => contacts
                .into_iter()
                .flat_map(|contact| {
                    let display_name = contact.display_name.clone();
                    let contact_id = contact.id;
                    let source = contact.source.provider_name().to_string();
                    
                    contact.emails.into_iter().map(move |email| ContactSuggestion {
                        contact_id,
                        display_name: display_name.clone(),
                        email: email.address,
                        source: source.clone(),
                    })
                })
                .collect(),
            Err(e) => {
                tracing::error!("Failed to search contacts by domain: {}", e);
                Vec::new()
            }
        }
    }
}

