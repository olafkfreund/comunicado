use crate::contacts::{Contact, ContactSearchCriteria, ContactsManager, AddressBookStats};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, List, ListItem, ListState, Paragraph, Table, Row, Cell,
        Gauge, Tabs, Wrap
    },
    Frame,
};
use std::sync::Arc;

/// Address book UI state and components
pub struct AddressBookUI {
    manager: Arc<ContactsManager>,
    
    // UI State
    selected_tab: AddressBookTab,
    contact_list_state: ListState,
    search_query: String,
    
    // Data
    contacts: Vec<Contact>,
    filtered_contacts: Vec<Contact>,
    selected_contact: Option<Contact>,
    stats: Option<AddressBookStats>,
    
    // UI Mode
    ui_mode: AddressBookMode,
    
    // Search and filters
    current_search: ContactSearchCriteria,
    is_searching: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AddressBookTab {
    AllContacts,
    GoogleContacts,
    OutlookContacts,
    LocalContacts,
    Statistics,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AddressBookMode {
    Browse,
    Search,
    ViewContact,
    EditContact,
    CreateContact,
    Settings,
}

impl AddressBookUI {
    /// Create a new address book UI
    pub fn new(manager: Arc<ContactsManager>) -> Self {
        Self {
            manager,
            selected_tab: AddressBookTab::AllContacts,
            contact_list_state: ListState::default(),
            search_query: String::new(),
            contacts: Vec::new(),
            filtered_contacts: Vec::new(),
            selected_contact: None,
            stats: None,
            ui_mode: AddressBookMode::Browse,
            current_search: ContactSearchCriteria::new(),
            is_searching: false,
        }
    }
    
    /// Render the address book UI
    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        // Main layout: tabs at top, content below
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);
        
        // Render tabs
        self.render_tabs(f, chunks[0]);
        
        // Render content based on selected tab and mode
        match self.ui_mode {
            AddressBookMode::ViewContact => self.render_contact_detail(f, chunks[1]),
            AddressBookMode::EditContact => self.render_contact_editor(f, chunks[1]),
            AddressBookMode::CreateContact => self.render_contact_creator(f, chunks[1]),
            AddressBookMode::Search => self.render_search_mode(f, chunks[1]),
            AddressBookMode::Settings => self.render_settings(f, chunks[1]),
            _ => self.render_main_content(f, chunks[1]),
        }
    }
    
    /// Render the tab bar
    fn render_tabs(&self, f: &mut Frame, area: Rect) {
        let tab_titles = vec![
            "All Contacts",
            "Google",
            "Outlook", 
            "Local",
            "Statistics"
        ];
        
        let selected_index = match self.selected_tab {
            AddressBookTab::AllContacts => 0,
            AddressBookTab::GoogleContacts => 1,
            AddressBookTab::OutlookContacts => 2,
            AddressBookTab::LocalContacts => 3,
            AddressBookTab::Statistics => 4,
        };
        
        let tabs = Tabs::new(tab_titles)
            .block(Block::default().borders(Borders::ALL).title("Address Book"))
            .select(selected_index)
            .style(Style::default().fg(Color::Cyan))
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .bg(Color::DarkGray)
            );
        
        f.render_widget(tabs, area);
    }
    
    /// Render main content area
    fn render_main_content(&mut self, f: &mut Frame, area: Rect) {
        match self.selected_tab {
            AddressBookTab::Statistics => self.render_statistics(f, area),
            _ => self.render_contact_list(f, area),
        }
    }
    
    /// Render the contact list
    fn render_contact_list(&mut self, f: &mut Frame, area: Rect) {
        // Split into search bar and list
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);
        
        // Search bar
        let search_block = Block::default()
            .borders(Borders::ALL)
            .title("Search (/ to focus, Esc to clear)");
        
        let search_text = if self.is_searching {
            format!("🔍 {}_", self.search_query)
        } else if self.search_query.is_empty() {
            "Type / to search contacts...".to_string()
        } else {
            format!("🔍 {} (Press Enter to search)", self.search_query)
        };
        
        let search_paragraph = Paragraph::new(search_text)
            .block(search_block)
            .style(if self.is_searching {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Gray)
            });
        
        f.render_widget(search_paragraph, chunks[0]);
        
        // Contact list
        let contacts_to_display = if self.filtered_contacts.is_empty() {
            &self.contacts
        } else {
            &self.filtered_contacts
        };
        
        let contact_items: Vec<ListItem> = contacts_to_display
            .iter()
            .map(|contact| {
                let email_text = contact.primary_email()
                    .map(|e| format!(" <{}>", e.address))
                    .unwrap_or_default();
                
                let company_text = contact.company
                    .as_ref()
                    .map(|c| format!(" - {}", c))
                    .unwrap_or_default();
                
                let line = Line::from(vec![
                    Span::styled(
                        contact.display_name.clone(),
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                    ),
                    Span::styled(
                        email_text,
                        Style::default().fg(Color::Cyan)
                    ),
                    Span::styled(
                        company_text,
                        Style::default().fg(Color::Gray)
                    ),
                ]);
                
                ListItem::new(line)
            })
            .collect();
        
        let contacts_list = List::new(contact_items)
            .block(Block::default()
                .borders(Borders::ALL)
                .title(format!("Contacts ({})", contacts_to_display.len())))
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            )
            .highlight_symbol("▶ ");
        
        f.render_stateful_widget(contacts_list, chunks[1], &mut self.contact_list_state);
        
        // Status bar at bottom
        let status_text = if contacts_to_display.is_empty() {
            if self.search_query.is_empty() {
                "No contacts found. Press 'n' to create a new contact, 's' to sync.".to_string()
            } else {
                "No contacts match your search. Try a different query.".to_string()
            }
        } else {
            format!(
                "↑↓ Navigate | Enter View | n New | e Edit | d Delete | s Sync | / Search | Tab Switch tabs"
            )
        };
        
        let status_area = Rect {
            x: area.x,
            y: area.y + area.height - 1,
            width: area.width,
            height: 1,
        };
        
        let status_paragraph = Paragraph::new(status_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        
        f.render_widget(status_paragraph, status_area);
    }
    
    /// Render contact detail view
    fn render_contact_detail(&self, f: &mut Frame, area: Rect) {
        if let Some(contact) = &self.selected_contact {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(format!("Contact Details - {}", contact.display_name));
            
            let inner = block.inner(area);
            f.render_widget(block, area);
            
            // Create detail text
            let mut text = Text::default();
            
            // Name information
            text.lines.push(Line::from(vec![
                Span::styled("Name: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(contact.full_name()),
            ]));
            
            if let Some(company) = &contact.company {
                text.lines.push(Line::from(vec![
                    Span::styled("Company: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    Span::raw(company.clone()),
                ]));
            }
            
            if let Some(job_title) = &contact.job_title {
                text.lines.push(Line::from(vec![
                    Span::styled("Title: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    Span::raw(job_title.clone()),
                ]));
            }
            
            text.lines.push(Line::raw(""));
            
            // Email addresses
            if !contact.emails.is_empty() {
                text.lines.push(Line::from(vec![
                    Span::styled("Emails:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                ]));
                
                for email in &contact.emails {
                    let primary_marker = if email.is_primary { " (Primary)" } else { "" };
                    text.lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(format!("{}: ", email.label), Style::default().fg(Color::Gray)),
                        Span::raw(format!("{}{}", email.address, primary_marker)),
                    ]));
                }
                text.lines.push(Line::raw(""));
            }
            
            // Phone numbers
            if !contact.phones.is_empty() {
                text.lines.push(Line::from(vec![
                    Span::styled("Phones:", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                ]));
                
                for phone in &contact.phones {
                    let primary_marker = if phone.is_primary { " (Primary)" } else { "" };
                    text.lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(format!("{}: ", phone.label), Style::default().fg(Color::Gray)),
                        Span::raw(format!("{}{}", phone.number, primary_marker)),
                    ]));
                }
                text.lines.push(Line::raw(""));
            }
            
            // Source information
            text.lines.push(Line::from(vec![
                Span::styled("Source: ", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
                Span::raw(contact.source.provider_name()),
            ]));
            
            if let Some(account_id) = contact.source.account_id() {
                text.lines.push(Line::from(vec![
                    Span::styled("Account: ", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
                    Span::raw(account_id),
                ]));
            }
            
            // Timestamps
            text.lines.push(Line::raw(""));
            text.lines.push(Line::from(vec![
                Span::styled("Created: ", Style::default().fg(Color::Gray)),
                Span::raw(contact.created_at.format("%Y-%m-%d %H:%M").to_string()),
            ]));
            
            text.lines.push(Line::from(vec![
                Span::styled("Updated: ", Style::default().fg(Color::Gray)),
                Span::raw(contact.updated_at.format("%Y-%m-%d %H:%M").to_string()),
            ]));
            
            if let Some(synced_at) = contact.synced_at {
                text.lines.push(Line::from(vec![
                    Span::styled("Last Sync: ", Style::default().fg(Color::Gray)),
                    Span::raw(synced_at.format("%Y-%m-%d %H:%M").to_string()),
                ]));
            }
            
            // Notes
            if let Some(notes) = &contact.notes {
                text.lines.push(Line::raw(""));
                text.lines.push(Line::from(vec![
                    Span::styled("Notes:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                ]));
                text.lines.push(Line::raw(notes.clone()));
            }
            
            let paragraph = Paragraph::new(text)
                .wrap(Wrap { trim: true });
            
            f.render_widget(paragraph, inner);
            
            // Controls at bottom
            let controls_area = Rect {
                x: area.x,
                y: area.y + area.height - 1,
                width: area.width,
                height: 1,
            };
            
            let controls = Paragraph::new("e Edit | d Delete | Esc Back | Enter Compose Email")
                .style(Style::default().fg(Color::Gray))
                .alignment(Alignment::Center);
            
            f.render_widget(controls, controls_area);
        }
    }
    
    /// Render contact editor (placeholder)
    fn render_contact_editor(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Edit Contact (Implementation Pending)");
        
        let text = Paragraph::new("Contact editor will be implemented in the next phase.\n\nPress Esc to go back.")
            .block(block)
            .alignment(Alignment::Center);
        
        f.render_widget(text, area);
    }
    
    /// Render contact creator (placeholder)
    fn render_contact_creator(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Create New Contact (Implementation Pending)");
        
        let text = Paragraph::new("Contact creator will be implemented in the next phase.\n\nPress Esc to go back.")
            .block(block)
            .alignment(Alignment::Center);
        
        f.render_widget(text, area);
    }
    
    /// Render search mode (placeholder)
    fn render_search_mode(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Advanced Search (Implementation Pending)");
        
        let text = Paragraph::new("Advanced search interface will be implemented in the next phase.\n\nPress Esc to go back.")
            .block(block)
            .alignment(Alignment::Center);
        
        f.render_widget(text, area);
    }
    
    /// Render settings (placeholder)
    fn render_settings(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Address Book Settings (Implementation Pending)");
        
        let text = Paragraph::new("Settings interface will be implemented in the next phase.\n\nPress Esc to go back.")
            .block(block)
            .alignment(Alignment::Center);
        
        f.render_widget(text, area);
    }
    
    /// Render statistics tab
    fn render_statistics(&self, f: &mut Frame, area: Rect) {
        if let Some(stats) = &self.stats {
            let block = Block::default()
                .borders(Borders::ALL)
                .title("Address Book Statistics");
            
            let inner = block.inner(area);
            f.render_widget(block, area);
            
            // Create statistics layout
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(8),  // Overview stats
                    Constraint::Length(4),  // Progress bars
                    Constraint::Min(0),     // Additional info
                ])
                .split(inner);
            
            // Overview statistics
            let overview_data = vec![
                Row::new(vec![
                    Cell::from("Total Contacts"),
                    Cell::from(stats.total_contacts.to_string()),
                ]),
                Row::new(vec![
                    Cell::from("Google Contacts"),
                    Cell::from(stats.google_contacts.to_string()),
                ]),
                Row::new(vec![
                    Cell::from("Outlook Contacts"),
                    Cell::from(stats.outlook_contacts.to_string()),
                ]),
                Row::new(vec![
                    Cell::from("Local Contacts"),
                    Cell::from(stats.local_contacts.to_string()),
                ]),
                Row::new(vec![
                    Cell::from("With Email"),
                    Cell::from(stats.contacts_with_email.to_string()),
                ]),
                Row::new(vec![
                    Cell::from("With Phone"),
                    Cell::from(stats.contacts_with_phone.to_string()),
                ]),
                Row::new(vec![
                    Cell::from("Groups"),
                    Cell::from(stats.groups_count.to_string()),
                ]),
            ];
            
            let overview_table = Table::new(
                overview_data,
                [Constraint::Percentage(50), Constraint::Percentage(50)]
            )
                .header(Row::new(vec!["Metric", "Count"]).style(Style::default().fg(Color::Yellow)))
                .block(Block::default().borders(Borders::ALL).title("Overview"));
                
            f.render_widget(overview_table, chunks[0]);
            
            // Progress bars for provider distribution
            if stats.total_contacts > 0 {
                let google_ratio = stats.google_contacts as f64 / stats.total_contacts as f64;
                let outlook_ratio = stats.outlook_contacts as f64 / stats.total_contacts as f64;
                
                let google_gauge = Gauge::default()
                    .block(Block::default().title("Google Contacts").borders(Borders::ALL))
                    .gauge_style(Style::default().fg(Color::Blue))
                    .ratio(google_ratio);
                
                let outlook_gauge = Gauge::default()
                    .block(Block::default().title("Outlook Contacts").borders(Borders::ALL)) 
                    .gauge_style(Style::default().fg(Color::Green))
                    .ratio(outlook_ratio);
                
                let gauge_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(chunks[1]);
                
                f.render_widget(google_gauge, gauge_chunks[0]);
                f.render_widget(outlook_gauge, gauge_chunks[1]);
            }
            
            // Additional info
            let mut info_text = Text::default();
            
            if let Some(last_sync) = stats.last_sync {
                info_text.lines.push(Line::from(vec![
                    Span::styled("Last Sync: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::raw(last_sync.format("%Y-%m-%d %H:%M:%S UTC").to_string()),
                ]));
            } else {
                info_text.lines.push(Line::from(vec![
                    Span::styled("Last Sync: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::styled("Never", Style::default().fg(Color::Red)),
                ]));
            }
            
            info_text.lines.push(Line::raw(""));
            info_text.lines.push(Line::from(vec![
                Span::styled("Data Quality:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ]));
            
            let email_percentage = if stats.total_contacts > 0 {
                (stats.contacts_with_email as f64 / stats.total_contacts as f64) * 100.0
            } else {
                0.0
            };
            
            let phone_percentage = if stats.total_contacts > 0 {
                (stats.contacts_with_phone as f64 / stats.total_contacts as f64) * 100.0
            } else {
                0.0
            };
            
            info_text.lines.push(Line::from(vec![
                Span::raw(format!("  • {:.1}% of contacts have email addresses", email_percentage)),
            ]));
            
            info_text.lines.push(Line::from(vec![
                Span::raw(format!("  • {:.1}% of contacts have phone numbers", phone_percentage)),
            ]));
            
            let info_paragraph = Paragraph::new(info_text)
                .block(Block::default().borders(Borders::ALL).title("Additional Information"));
            
            f.render_widget(info_paragraph, chunks[2]);
        } else {
            // No stats available
            let text = Paragraph::new("Loading statistics...")
                .block(Block::default().borders(Borders::ALL).title("Statistics"))
                .alignment(Alignment::Center);
            
            f.render_widget(text, area);
        }
    }
    
    /// Handle keyboard input
    pub async fn handle_key(&mut self, key: crossterm::event::KeyCode) -> bool {
        use crossterm::event::KeyCode;
        
        match self.ui_mode {
            AddressBookMode::Browse => {
                match key {
                    KeyCode::Tab => self.next_tab(),
                    KeyCode::BackTab => self.previous_tab(),
                    KeyCode::Up => self.select_previous_contact(),
                    KeyCode::Down => self.select_next_contact(),
                    KeyCode::Enter => self.view_selected_contact(),
                    KeyCode::Char('/') => {
                        self.ui_mode = AddressBookMode::Search;
                        self.is_searching = true;
                    },
                    KeyCode::Char('n') => self.ui_mode = AddressBookMode::CreateContact,
                    KeyCode::Char('e') => {
                        if self.get_selected_contact().is_some() {
                            self.ui_mode = AddressBookMode::EditContact;
                        }
                    },
                    KeyCode::Char('d') => self.delete_selected_contact().await,
                    KeyCode::Char('s') => self.sync_contacts().await,
                    KeyCode::Esc => return false, // Exit address book
                    _ => {},
                }
            },
            AddressBookMode::ViewContact => {
                match key {
                    KeyCode::Esc => self.ui_mode = AddressBookMode::Browse,
                    KeyCode::Char('e') => self.ui_mode = AddressBookMode::EditContact,
                    KeyCode::Char('d') => {
                        self.delete_selected_contact().await;
                        self.ui_mode = AddressBookMode::Browse;
                    },
                    KeyCode::Enter => {
                        // TODO: Compose email to this contact
                        self.ui_mode = AddressBookMode::Browse;
                    },
                    _ => {},
                }
            },
            AddressBookMode::Search => {
                match key {
                    KeyCode::Esc => {
                        self.ui_mode = AddressBookMode::Browse;
                        self.is_searching = false;
                        self.search_query.clear();
                        self.apply_search().await;
                    },
                    KeyCode::Enter => {
                        self.is_searching = false;
                        self.apply_search().await;
                    },
                    KeyCode::Backspace => {
                        self.search_query.pop();
                    },
                    KeyCode::Char(c) => {
                        self.search_query.push(c);
                    },
                    _ => {},
                }
            },
            _ => {
                match key {
                    KeyCode::Esc => self.ui_mode = AddressBookMode::Browse,
                    _ => {},
                }
            }
        }
        
        true
    }
    
    // Navigation methods
    fn next_tab(&mut self) {
        self.selected_tab = match self.selected_tab {
            AddressBookTab::AllContacts => AddressBookTab::GoogleContacts,
            AddressBookTab::GoogleContacts => AddressBookTab::OutlookContacts,
            AddressBookTab::OutlookContacts => AddressBookTab::LocalContacts,
            AddressBookTab::LocalContacts => AddressBookTab::Statistics,
            AddressBookTab::Statistics => AddressBookTab::AllContacts,
        };
        
        // Reset list selection when changing tabs
        self.contact_list_state.select(Some(0));
    }
    
    fn previous_tab(&mut self) {
        self.selected_tab = match self.selected_tab {
            AddressBookTab::AllContacts => AddressBookTab::Statistics,
            AddressBookTab::GoogleContacts => AddressBookTab::AllContacts,
            AddressBookTab::OutlookContacts => AddressBookTab::GoogleContacts,
            AddressBookTab::LocalContacts => AddressBookTab::OutlookContacts,
            AddressBookTab::Statistics => AddressBookTab::LocalContacts,
        };
        
        // Reset list selection when changing tabs
        self.contact_list_state.select(Some(0));
    }
    
    fn select_next_contact(&mut self) {
        let contacts = if self.filtered_contacts.is_empty() {
            &self.contacts
        } else {
            &self.filtered_contacts
        };
        
        if !contacts.is_empty() {
            let i = match self.contact_list_state.selected() {
                Some(i) => (i + 1) % contacts.len(),
                None => 0,
            };
            self.contact_list_state.select(Some(i));
        }
    }
    
    fn select_previous_contact(&mut self) {
        let contacts = if self.filtered_contacts.is_empty() {
            &self.contacts
        } else {
            &self.filtered_contacts
        };
        
        if !contacts.is_empty() {
            let i = match self.contact_list_state.selected() {
                Some(i) => if i == 0 { contacts.len() - 1 } else { i - 1 },
                None => 0,
            };
            self.contact_list_state.select(Some(i));
        }
    }
    
    fn view_selected_contact(&mut self) {
        if let Some(contact) = self.get_selected_contact() {
            self.selected_contact = Some(contact);
            self.ui_mode = AddressBookMode::ViewContact;
        }
    }
    
    fn get_selected_contact(&self) -> Option<Contact> {
        let contacts = if self.filtered_contacts.is_empty() {
            &self.contacts
        } else {
            &self.filtered_contacts
        };
        
        self.contact_list_state.selected()
            .and_then(|i| contacts.get(i))
            .cloned()
    }
    
    async fn delete_selected_contact(&mut self) {
        if let Some(contact) = self.get_selected_contact() {
            if let Some(id) = contact.id {
                if let Err(e) = self.manager.delete_contact(id).await {
                    tracing::error!("Failed to delete contact: {}", e);
                } else {
                    // Refresh the contact list
                    self.refresh_contacts().await;
                }
            }
        }
    }
    
    async fn sync_contacts(&mut self) {
        if let Err(e) = self.manager.sync_all_contacts().await {
            tracing::error!("Failed to sync contacts: {}", e);
        } else {
            self.refresh_contacts().await;
        }
    }
    
    async fn apply_search(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_contacts.clear();
        } else {
            let criteria = ContactSearchCriteria::new()
                .with_query(self.search_query.clone())
                .with_limit(100);
            
            match self.manager.search_contacts(&criteria).await {
                Ok(contacts) => {
                    self.filtered_contacts = contacts;
                    self.contact_list_state.select(Some(0));
                },
                Err(e) => {
                    tracing::error!("Search failed: {}", e);
                }
            }
        }
    }
    
    /// Refresh contacts based on current tab
    pub async fn refresh_contacts(&mut self) {
        let criteria = match self.selected_tab {
            AddressBookTab::AllContacts => ContactSearchCriteria::new(),
            AddressBookTab::GoogleContacts => {
                // TODO: Add Google source filter
                ContactSearchCriteria::new()
            },
            AddressBookTab::OutlookContacts => {
                // TODO: Add Outlook source filter  
                ContactSearchCriteria::new()
            },
            AddressBookTab::LocalContacts => {
                // TODO: Add Local source filter
                ContactSearchCriteria::new()
            },
            AddressBookTab::Statistics => {
                // Load stats instead
                match self.manager.get_stats().await {
                    Ok(stats) => self.stats = Some(stats),
                    Err(e) => tracing::error!("Failed to load stats: {}", e),
                }
                return;
            }
        };
        
        match self.manager.search_contacts(&criteria).await {
            Ok(contacts) => {
                self.contacts = contacts;
                self.contact_list_state.select(if self.contacts.is_empty() { None } else { Some(0) });
            },
            Err(e) => {
                tracing::error!("Failed to load contacts: {}", e);
            }
        }
    }
    
    /// Get current UI mode
    pub fn mode(&self) -> AddressBookMode {
        self.ui_mode.clone()
    }
    
    /// Set UI mode
    pub fn set_mode(&mut self, mode: AddressBookMode) {
        self.ui_mode = mode;
    }
}