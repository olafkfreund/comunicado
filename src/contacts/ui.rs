use crate::contacts::{AddressBookStats, Contact, ContactSearchCriteria, ContactsManager};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Cell, Gauge, List, ListItem, ListState, Paragraph, Row, Table, Tabs, Wrap,
    },
    Frame,
};
use std::sync::Arc;

/// Actions that can be triggered from the address book
#[derive(Debug, Clone)]
pub enum AddressBookAction {
    ComposeEmail { to: String, name: String },
    LaunchAdvancedSearch,
    ExportContacts,
}

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
    is_searching: bool,

    // Contact editing
    contact_editor: ContactEditor,
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

/// Contact editor state
#[derive(Debug, Clone)]
pub struct ContactEditor {
    // Form fields
    pub display_name: String,
    pub first_name: String,
    pub last_name: String,
    pub company: String,
    pub job_title: String,
    pub notes: String,
    pub photo_url: String,
    
    // Email addresses
    pub emails: Vec<ContactEmailInput>,
    
    // Phone numbers
    pub phones: Vec<ContactPhoneInput>,
    
    // UI state
    pub focused_field: ContactField,
    pub focused_email_index: usize,
    pub focused_phone_index: usize,
    pub is_editing: bool, // true for edit, false for create
    pub original_contact_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct ContactEmailInput {
    pub address: String,
    pub label: String,
    pub is_primary: bool,
    pub is_focused: bool,
}

#[derive(Debug, Clone)]
pub struct ContactPhoneInput {
    pub number: String,
    pub label: String,
    pub is_primary: bool,
    pub is_focused: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContactField {
    DisplayName,
    FirstName,
    LastName,
    Company,
    JobTitle,
    Notes,
    PhotoUrl,
    Emails,
    Phones,
}

impl ContactEditor {
    pub fn new() -> Self {
        Self {
            display_name: String::new(),
            first_name: String::new(),
            last_name: String::new(),
            company: String::new(),
            job_title: String::new(),
            notes: String::new(),
            photo_url: String::new(),
            emails: vec![ContactEmailInput::new()],
            phones: vec![ContactPhoneInput::new()],
            focused_field: ContactField::DisplayName,
            focused_email_index: 0,
            focused_phone_index: 0,
            is_editing: false,
            original_contact_id: None,
        }
    }

    pub fn from_contact(contact: &Contact) -> Self {
        let emails = if contact.emails.is_empty() {
            vec![ContactEmailInput::new()]
        } else {
            contact.emails.iter().map(ContactEmailInput::from_contact_email).collect()
        };

        let phones = if contact.phones.is_empty() {
            vec![ContactPhoneInput::new()]
        } else {
            contact.phones.iter().map(ContactPhoneInput::from_contact_phone).collect()
        };

        Self {
            display_name: contact.display_name.clone(),
            first_name: contact.first_name.clone().unwrap_or_default(),
            last_name: contact.last_name.clone().unwrap_or_default(),
            company: contact.company.clone().unwrap_or_default(),
            job_title: contact.job_title.clone().unwrap_or_default(),
            notes: contact.notes.clone().unwrap_or_default(),
            photo_url: contact.photo_url.clone().unwrap_or_default(),
            emails,
            phones,
            focused_field: ContactField::DisplayName,
            focused_email_index: 0,
            focused_phone_index: 0,
            is_editing: true,
            original_contact_id: contact.id,
        }
    }

    pub fn to_contact(&self) -> Contact {
        let mut contact = Contact::new(
            self.original_contact_id.map(|id| id.to_string()).unwrap_or_else(|| chrono::Utc::now().timestamp().to_string()),
            crate::contacts::ContactSource::Local,
            self.display_name.clone(),
        );

        contact.id = self.original_contact_id;
        contact.first_name = if self.first_name.is_empty() { None } else { Some(self.first_name.clone()) };
        contact.last_name = if self.last_name.is_empty() { None } else { Some(self.last_name.clone()) };
        contact.company = if self.company.is_empty() { None } else { Some(self.company.clone()) };
        contact.job_title = if self.job_title.is_empty() { None } else { Some(self.job_title.clone()) };
        contact.notes = if self.notes.is_empty() { None } else { Some(self.notes.clone()) };
        contact.photo_url = if self.photo_url.is_empty() { None } else { Some(self.photo_url.clone()) };

        contact.emails = self.emails.iter()
            .filter(|e| !e.address.is_empty())
            .map(|e| e.to_contact_email())
            .collect();

        contact.phones = self.phones.iter()
            .filter(|p| !p.number.is_empty())
            .map(|p| p.to_contact_phone())
            .collect();

        contact
    }

    pub fn clear(&mut self) {
        *self = Self::new();
    }
}

impl ContactEmailInput {
    pub fn new() -> Self {
        Self {
            address: String::new(),
            label: "work".to_string(),
            is_primary: false,
            is_focused: false,
        }
    }

    pub fn from_contact_email(email: &crate::contacts::ContactEmail) -> Self {
        Self {
            address: email.address.clone(),
            label: email.label.clone(),
            is_primary: email.is_primary,
            is_focused: false,
        }
    }

    pub fn to_contact_email(&self) -> crate::contacts::ContactEmail {
        crate::contacts::ContactEmail {
            address: self.address.clone(),
            label: self.label.clone(),
            is_primary: self.is_primary,
        }
    }
}

impl ContactPhoneInput {
    pub fn new() -> Self {
        Self {
            number: String::new(),
            label: "mobile".to_string(),
            is_primary: false,
            is_focused: false,
        }
    }

    pub fn from_contact_phone(phone: &crate::contacts::ContactPhone) -> Self {
        Self {
            number: phone.number.clone(),
            label: phone.label.clone(),
            is_primary: phone.is_primary,
            is_focused: false,
        }
    }

    pub fn to_contact_phone(&self) -> crate::contacts::ContactPhone {
        crate::contacts::ContactPhone {
            number: self.number.clone(),
            label: self.label.clone(),
            is_primary: self.is_primary,
        }
    }
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
            is_searching: false,
            contact_editor: ContactEditor::new(),
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
        let tab_titles = vec!["All Contacts", "Google", "Outlook", "Local", "Statistics"];

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
                    .bg(Color::DarkGray),
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

        let search_paragraph =
            Paragraph::new(search_text)
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
                        contact.display_name.clone(),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(email_text, Style::default().fg(Color::Cyan)),
                    Span::styled(company_text, Style::default().fg(Color::Gray)),
                ]);

                ListItem::new(line)
            })
            .collect();

        let contacts_list = List::new(contact_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Contacts ({})", contacts_to_display.len())),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
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
                Span::styled(
                    "Name: ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(contact.full_name()),
            ]));

            if let Some(company) = &contact.company {
                text.lines.push(Line::from(vec![
                    Span::styled(
                        "Company: ",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(company.clone()),
                ]));
            }

            if let Some(job_title) = &contact.job_title {
                text.lines.push(Line::from(vec![
                    Span::styled(
                        "Title: ",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(job_title.clone()),
                ]));
            }

            text.lines.push(Line::raw(""));

            // Email addresses
            if !contact.emails.is_empty() {
                text.lines.push(Line::from(vec![Span::styled(
                    "Emails:",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )]));

                for email in &contact.emails {
                    let primary_marker = if email.is_primary { " (Primary)" } else { "" };
                    text.lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(
                            format!("{}: ", email.label),
                            Style::default().fg(Color::Gray),
                        ),
                        Span::raw(format!("{}{}", email.address, primary_marker)),
                    ]));
                }
                text.lines.push(Line::raw(""));
            }

            // Phone numbers
            if !contact.phones.is_empty() {
                text.lines.push(Line::from(vec![Span::styled(
                    "Phones:",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )]));

                for phone in &contact.phones {
                    let primary_marker = if phone.is_primary { " (Primary)" } else { "" };
                    text.lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(
                            format!("{}: ", phone.label),
                            Style::default().fg(Color::Gray),
                        ),
                        Span::raw(format!("{}{}", phone.number, primary_marker)),
                    ]));
                }
                text.lines.push(Line::raw(""));
            }

            // Source information
            text.lines.push(Line::from(vec![
                Span::styled(
                    "Source: ",
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(contact.source.provider_name()),
            ]));

            if let Some(account_id) = contact.source.account_id() {
                text.lines.push(Line::from(vec![
                    Span::styled(
                        "Account: ",
                        Style::default()
                            .fg(Color::Magenta)
                            .add_modifier(Modifier::BOLD),
                    ),
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
                text.lines.push(Line::from(vec![Span::styled(
                    "Notes:",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )]));
                text.lines.push(Line::raw(notes.clone()));
            }

            let paragraph = Paragraph::new(text).wrap(Wrap { trim: true });

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

    /// Render contact editor
    fn render_contact_editor(&self, f: &mut Frame, area: Rect) {
        self.render_contact_form(f, area, "Edit Contact");
    }

    /// Render contact creator
    fn render_contact_creator(&self, f: &mut Frame, area: Rect) {
        self.render_contact_form(f, area, "Create New Contact");
    }

    /// Render contact form (shared by editor and creator)
    fn render_contact_form(&self, f: &mut Frame, area: Rect, title: &str) {
        let main_block = Block::default()
            .borders(Borders::ALL)
            .title(title);

        let inner_area = main_block.inner(area);
        f.render_widget(main_block, area);

        // Split into form fields and controls
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(20), // Form fields
                Constraint::Length(3), // Controls
            ])
            .split(inner_area);

        // Split form into left and right columns
        let form_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50), // Left column
                Constraint::Percentage(50), // Right column
            ])
            .split(chunks[0]);

        // Render left column (basic info)
        self.render_contact_form_left(f, form_chunks[0]);

        // Render right column (emails, phones, notes)
        self.render_contact_form_right(f, form_chunks[1]);

        // Render controls
        self.render_contact_form_controls(f, chunks[1]);
    }

    /// Render left column of contact form
    fn render_contact_form_left(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Display name
                Constraint::Length(3), // First name
                Constraint::Length(3), // Last name
                Constraint::Length(3), // Company
                Constraint::Length(3), // Job title
                Constraint::Min(3),    // Photo URL
            ])
            .split(area);

        // Display Name field
        let display_name_style = if self.contact_editor.focused_field == ContactField::DisplayName {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let display_name = Paragraph::new(self.contact_editor.display_name.clone())
            .block(Block::default().borders(Borders::ALL).title("Display Name *"))
            .style(display_name_style);
        f.render_widget(display_name, chunks[0]);

        // First Name field
        let first_name_style = if self.contact_editor.focused_field == ContactField::FirstName {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let first_name = Paragraph::new(self.contact_editor.first_name.clone())
            .block(Block::default().borders(Borders::ALL).title("First Name"))
            .style(first_name_style);
        f.render_widget(first_name, chunks[1]);

        // Last Name field
        let last_name_style = if self.contact_editor.focused_field == ContactField::LastName {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let last_name = Paragraph::new(self.contact_editor.last_name.clone())
            .block(Block::default().borders(Borders::ALL).title("Last Name"))
            .style(last_name_style);
        f.render_widget(last_name, chunks[2]);

        // Company field
        let company_style = if self.contact_editor.focused_field == ContactField::Company {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let company = Paragraph::new(self.contact_editor.company.clone())
            .block(Block::default().borders(Borders::ALL).title("Company"))
            .style(company_style);
        f.render_widget(company, chunks[3]);

        // Job Title field
        let job_title_style = if self.contact_editor.focused_field == ContactField::JobTitle {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let job_title = Paragraph::new(self.contact_editor.job_title.clone())
            .block(Block::default().borders(Borders::ALL).title("Job Title"))
            .style(job_title_style);
        f.render_widget(job_title, chunks[4]);

        // Photo URL field
        let photo_url_style = if self.contact_editor.focused_field == ContactField::PhotoUrl {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let photo_url = Paragraph::new(self.contact_editor.photo_url.clone())
            .block(Block::default().borders(Borders::ALL).title("Photo URL"))
            .style(photo_url_style)
            .wrap(Wrap { trim: true });
        f.render_widget(photo_url, chunks[5]);
    }

    /// Render right column of contact form
    fn render_contact_form_right(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8),  // Emails
                Constraint::Length(6),  // Phones
                Constraint::Min(3),     // Notes
            ])
            .split(area);

        // Render emails section
        self.render_contact_emails_section(f, chunks[0]);

        // Render phones section
        self.render_contact_phones_section(f, chunks[1]);

        // Notes field
        let notes_style = if self.contact_editor.focused_field == ContactField::Notes {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let notes = Paragraph::new(self.contact_editor.notes.clone())
            .block(Block::default().borders(Borders::ALL).title("Notes"))
            .style(notes_style)
            .wrap(Wrap { trim: true });
        f.render_widget(notes, chunks[2]);
    }

    /// Render emails section
    fn render_contact_emails_section(&self, f: &mut Frame, area: Rect) {
        let emails_style = if self.contact_editor.focused_field == ContactField::Emails {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let emails_block = Block::default()
            .borders(Borders::ALL)
            .title("Email Addresses")
            .border_style(emails_style);

        let inner_area = emails_block.inner(area);
        f.render_widget(emails_block, area);

        // Create email list items
        let email_items: Vec<ListItem> = self.contact_editor.emails
            .iter()
            .enumerate()
            .map(|(i, email)| {
                let primary_marker = if email.is_primary { " (Primary)" } else { "" };
                let focused_marker = if self.contact_editor.focused_field == ContactField::Emails && 
                                         self.contact_editor.focused_email_index == i { "▶ " } else { "  " };
                
                let line = Line::from(vec![
                    Span::raw(focused_marker),
                    Span::styled(
                        format!("{}: ", email.label),
                        Style::default().fg(Color::Cyan)
                    ),
                    Span::raw(format!("{}{}", email.address, primary_marker)),
                ]);

                ListItem::new(line)
            })
            .collect();

        let emails_list = List::new(email_items);
        f.render_widget(emails_list, inner_area);
    }

    /// Render phones section
    fn render_contact_phones_section(&self, f: &mut Frame, area: Rect) {
        let phones_style = if self.contact_editor.focused_field == ContactField::Phones {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let phones_block = Block::default()
            .borders(Borders::ALL)
            .title("Phone Numbers")
            .border_style(phones_style);

        let inner_area = phones_block.inner(area);
        f.render_widget(phones_block, area);

        // Create phone list items
        let phone_items: Vec<ListItem> = self.contact_editor.phones
            .iter()
            .enumerate()
            .map(|(i, phone)| {
                let primary_marker = if phone.is_primary { " (Primary)" } else { "" };
                let focused_marker = if self.contact_editor.focused_field == ContactField::Phones && 
                                         self.contact_editor.focused_phone_index == i { "▶ " } else { "  " };
                
                let line = Line::from(vec![
                    Span::raw(focused_marker),
                    Span::styled(
                        format!("{}: ", phone.label),
                        Style::default().fg(Color::Green)
                    ),
                    Span::raw(format!("{}{}", phone.number, primary_marker)),
                ]);

                ListItem::new(line)
            })
            .collect();

        let phones_list = List::new(phone_items);
        f.render_widget(phones_list, inner_area);
    }

    /// Render contact form controls
    fn render_contact_form_controls(&self, f: &mut Frame, area: Rect) {
        let controls_text = "Tab: Next Field | Enter: Add Email/Phone | Ctrl+S: Save | Ctrl+D: Delete Field | Esc: Cancel";

        let controls = Paragraph::new(controls_text)
            .block(Block::default().borders(Borders::ALL).title("Controls"))
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        f.render_widget(controls, area);
    }

    /// Render search mode with advanced search interface
    fn render_search_mode(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Advanced Search");

        let inner_area = block.inner(area);
        f.render_widget(block, area);

        let search_info = vec![
            Line::from("🔍 Advanced Contact Search"),
            Line::from(""),
            Line::from("Features Available:"),
            Line::from("  • Multi-field search (name, email, phone, company)"),
            Line::from("  • Filter by contact source (Google, Outlook, Local)"),
            Line::from("  • Presence filters (has email, phone, etc.)"),
            Line::from("  • Fuzzy matching and relevance scoring"),
            Line::from("  • Sort by various criteria"),
            Line::from("  • Export and save search results"),
            Line::from(""),
            Line::from("Keyboard Shortcuts:"),
            Line::from("  • A: Launch Advanced Search"),
            Line::from("  • /: Quick search mode"),
            Line::from("  • Esc: Return to contact list"),
            Line::from(""),
            Line::from(Span::styled(
                "Press 'A' to open Advanced Search or '/' for quick search",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            )),
        ];

        let info_paragraph = Paragraph::new(search_info)
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });

        f.render_widget(info_paragraph, inner_area);
    }

    /// Render settings (placeholder)
    fn render_settings(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Address Book Settings (Implementation Pending)");

        let text = Paragraph::new(
            "Settings interface will be implemented in the next phase.\n\nPress Esc to go back.",
        )
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
                    Constraint::Length(8), // Overview stats
                    Constraint::Length(4), // Progress bars
                    Constraint::Min(0),    // Additional info
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
                [Constraint::Percentage(50), Constraint::Percentage(50)],
            )
            .header(Row::new(vec!["Metric", "Count"]).style(Style::default().fg(Color::Yellow)))
            .block(Block::default().borders(Borders::ALL).title("Overview"));

            f.render_widget(overview_table, chunks[0]);

            // Progress bars for provider distribution
            if stats.total_contacts > 0 {
                let google_ratio = stats.google_contacts as f64 / stats.total_contacts as f64;
                let outlook_ratio = stats.outlook_contacts as f64 / stats.total_contacts as f64;

                let google_gauge = Gauge::default()
                    .block(
                        Block::default()
                            .title("Google Contacts")
                            .borders(Borders::ALL),
                    )
                    .gauge_style(Style::default().fg(Color::Blue))
                    .ratio(google_ratio);

                let outlook_gauge = Gauge::default()
                    .block(
                        Block::default()
                            .title("Outlook Contacts")
                            .borders(Borders::ALL),
                    )
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
                    Span::styled(
                        "Last Sync: ",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(last_sync.format("%Y-%m-%d %H:%M:%S UTC").to_string()),
                ]));
            } else {
                info_text.lines.push(Line::from(vec![
                    Span::styled(
                        "Last Sync: ",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("Never", Style::default().fg(Color::Red)),
                ]));
            }

            info_text.lines.push(Line::raw(""));
            info_text.lines.push(Line::from(vec![Span::styled(
                "Data Quality:",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]));

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

            info_text.lines.push(Line::from(vec![Span::raw(format!(
                "  • {:.1}% of contacts have email addresses",
                email_percentage
            ))]));

            info_text.lines.push(Line::from(vec![Span::raw(format!(
                "  • {:.1}% of contacts have phone numbers",
                phone_percentage
            ))]));

            let info_paragraph = Paragraph::new(info_text).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Additional Information"),
            );

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
    pub async fn handle_key(
        &mut self,
        key: crossterm::event::KeyCode,
    ) -> (bool, Option<AddressBookAction>) {
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
                    }
                    KeyCode::Char('n') => {
                        self.contact_editor.clear();
                        self.ui_mode = AddressBookMode::CreateContact;
                    }
                    KeyCode::Char('e') => {
                        if let Some(contact) = self.get_selected_contact() {
                            self.contact_editor = ContactEditor::from_contact(&contact);
                            self.ui_mode = AddressBookMode::EditContact;
                        }
                    }
                    KeyCode::Char('d') => self.delete_selected_contact().await,
                    KeyCode::Char('s') => self.sync_contacts().await,
                    KeyCode::Char('a') | KeyCode::Char('A') => {
                        return (true, Some(AddressBookAction::LaunchAdvancedSearch));
                    }
                    KeyCode::Esc => return (false, None), // Exit address book
                    _ => {}
                }
            }
            AddressBookMode::ViewContact => {
                match key {
                    KeyCode::Esc => self.ui_mode = AddressBookMode::Browse,
                    KeyCode::Char('e') => {
                        if let Some(contact) = &self.selected_contact {
                            self.contact_editor = ContactEditor::from_contact(contact);
                            self.ui_mode = AddressBookMode::EditContact;
                        }
                    }
                    KeyCode::Char('d') => {
                        self.delete_selected_contact().await;
                        self.ui_mode = AddressBookMode::Browse;
                    }
                    KeyCode::Enter => {
                        // Compose email to this contact
                        if let Some(contact) = &self.selected_contact {
                            if let Some(primary_email) = contact.primary_email() {
                                tracing::info!(
                                    "Initiating email composition to contact: {} <{}>",
                                    contact.display_name,
                                    primary_email.address
                                );
                                self.ui_mode = AddressBookMode::Browse;
                                return (
                                    true,
                                    Some(AddressBookAction::ComposeEmail {
                                        to: primary_email.address.clone(),
                                        name: contact.display_name.clone(),
                                    }),
                                );
                            }
                            self.ui_mode = AddressBookMode::Browse;
                        }
                    }
                    _ => {}
                }
            }
            AddressBookMode::Search => match key {
                KeyCode::Esc => {
                    self.ui_mode = AddressBookMode::Browse;
                    self.is_searching = false;
                    self.search_query.clear();
                    self.apply_search().await;
                }
                KeyCode::Enter => {
                    self.is_searching = false;
                    self.apply_search().await;
                }
                KeyCode::Char('a') | KeyCode::Char('A') => {
                    return (true, Some(AddressBookAction::LaunchAdvancedSearch));
                }
                KeyCode::Backspace => {
                    self.search_query.pop();
                }
                KeyCode::Char(c) => {
                    self.search_query.push(c);
                }
                _ => {}
            },
            AddressBookMode::EditContact | AddressBookMode::CreateContact => {
                self.handle_contact_editor_key(key).await
            }
            _ => match key {
                KeyCode::Esc => self.ui_mode = AddressBookMode::Browse,
                _ => {}
            },
        }

        (true, None)
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
                Some(i) => {
                    if i == 0 {
                        contacts.len() - 1
                    } else {
                        i - 1
                    }
                }
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

        self.contact_list_state
            .selected()
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
                }
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
            }
            AddressBookTab::OutlookContacts => {
                // TODO: Add Outlook source filter
                ContactSearchCriteria::new()
            }
            AddressBookTab::LocalContacts => {
                // TODO: Add Local source filter
                ContactSearchCriteria::new()
            }
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
                self.contact_list_state.select(if self.contacts.is_empty() {
                    None
                } else {
                    Some(0)
                });
            }
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

    /// Handle keyboard input for contact editor
    async fn handle_contact_editor_key(&mut self, key: crossterm::event::KeyCode) {
        use crossterm::event::KeyCode;

        match key {
            KeyCode::Esc => {
                self.contact_editor.clear();
                self.ui_mode = AddressBookMode::Browse;
            }
            KeyCode::Tab => {
                self.next_contact_field();
            }
            KeyCode::BackTab => {
                self.previous_contact_field();
            }
            KeyCode::Enter => {
                self.handle_contact_field_enter().await;
            }
            KeyCode::Backspace => {
                self.handle_contact_field_backspace();
            }
            KeyCode::Char(c) => {
                match c {
                    's' if self.is_ctrl_pressed() => {
                        self.save_contact().await;
                    }
                    'd' if self.is_ctrl_pressed() => {
                        self.delete_current_field_item();
                    }
                    _ => {
                        self.handle_contact_field_input(c);
                    }
                }
            }
            _ => {}
        }
    }

    /// Check if Ctrl is pressed (placeholder - would need proper modifier key handling)
    fn is_ctrl_pressed(&self) -> bool {
        // TODO: Implement proper modifier key detection
        // For now, we'll handle Ctrl+S differently in the actual implementation
        false
    }

    /// Navigate to next field in contact editor
    fn next_contact_field(&mut self) {
        self.contact_editor.focused_field = match self.contact_editor.focused_field {
            ContactField::DisplayName => ContactField::FirstName,
            ContactField::FirstName => ContactField::LastName,
            ContactField::LastName => ContactField::Company,
            ContactField::Company => ContactField::JobTitle,
            ContactField::JobTitle => ContactField::PhotoUrl,
            ContactField::PhotoUrl => ContactField::Emails,
            ContactField::Emails => ContactField::Phones,
            ContactField::Phones => ContactField::Notes,
            ContactField::Notes => ContactField::DisplayName,
        };
    }

    /// Navigate to previous field in contact editor
    fn previous_contact_field(&mut self) {
        self.contact_editor.focused_field = match self.contact_editor.focused_field {
            ContactField::DisplayName => ContactField::Notes,
            ContactField::FirstName => ContactField::DisplayName,
            ContactField::LastName => ContactField::FirstName,
            ContactField::Company => ContactField::LastName,
            ContactField::JobTitle => ContactField::Company,
            ContactField::PhotoUrl => ContactField::JobTitle,
            ContactField::Emails => ContactField::PhotoUrl,
            ContactField::Phones => ContactField::Emails,
            ContactField::Notes => ContactField::Phones,
        };
    }

    /// Handle Enter key in contact editor
    async fn handle_contact_field_enter(&mut self) {
        match self.contact_editor.focused_field {
            ContactField::Emails => {
                // Add new email
                self.contact_editor.emails.push(ContactEmailInput::new());
                self.contact_editor.focused_email_index = self.contact_editor.emails.len() - 1;
            }
            ContactField::Phones => {
                // Add new phone
                self.contact_editor.phones.push(ContactPhoneInput::new());
                self.contact_editor.focused_phone_index = self.contact_editor.phones.len() - 1;
            }
            _ => {
                // Save contact
                self.save_contact().await;
            }
        }
    }

    /// Handle Backspace in contact editor
    fn handle_contact_field_backspace(&mut self) {
        match self.contact_editor.focused_field {
            ContactField::DisplayName => {
                self.contact_editor.display_name.pop();
            }
            ContactField::FirstName => {
                self.contact_editor.first_name.pop();
            }
            ContactField::LastName => {
                self.contact_editor.last_name.pop();
            }
            ContactField::Company => {
                self.contact_editor.company.pop();
            }
            ContactField::JobTitle => {
                self.contact_editor.job_title.pop();
            }
            ContactField::PhotoUrl => {
                self.contact_editor.photo_url.pop();
            }
            ContactField::Notes => {
                self.contact_editor.notes.pop();
            }
            ContactField::Emails => {
                if let Some(email) = self.contact_editor.emails.get_mut(self.contact_editor.focused_email_index) {
                    email.address.pop();
                }
            }
            ContactField::Phones => {
                if let Some(phone) = self.contact_editor.phones.get_mut(self.contact_editor.focused_phone_index) {
                    phone.number.pop();
                }
            }
        }
    }

    /// Handle character input in contact editor
    fn handle_contact_field_input(&mut self, c: char) {
        match self.contact_editor.focused_field {
            ContactField::DisplayName => {
                self.contact_editor.display_name.push(c);
            }
            ContactField::FirstName => {
                self.contact_editor.first_name.push(c);
            }
            ContactField::LastName => {
                self.contact_editor.last_name.push(c);
            }
            ContactField::Company => {
                self.contact_editor.company.push(c);
            }
            ContactField::JobTitle => {
                self.contact_editor.job_title.push(c);
            }
            ContactField::PhotoUrl => {
                self.contact_editor.photo_url.push(c);
            }
            ContactField::Notes => {
                self.contact_editor.notes.push(c);
            }
            ContactField::Emails => {
                if let Some(email) = self.contact_editor.emails.get_mut(self.contact_editor.focused_email_index) {
                    email.address.push(c);
                }
            }
            ContactField::Phones => {
                if let Some(phone) = self.contact_editor.phones.get_mut(self.contact_editor.focused_phone_index) {
                    phone.number.push(c);
                }
            }
        }
    }

    /// Delete current field item (email or phone)
    fn delete_current_field_item(&mut self) {
        match self.contact_editor.focused_field {
            ContactField::Emails => {
                if self.contact_editor.emails.len() > 1 {
                    self.contact_editor.emails.remove(self.contact_editor.focused_email_index);
                    if self.contact_editor.focused_email_index >= self.contact_editor.emails.len() {
                        self.contact_editor.focused_email_index = self.contact_editor.emails.len().saturating_sub(1);
                    }
                }
            }
            ContactField::Phones => {
                if self.contact_editor.phones.len() > 1 {
                    self.contact_editor.phones.remove(self.contact_editor.focused_phone_index);
                    if self.contact_editor.focused_phone_index >= self.contact_editor.phones.len() {
                        self.contact_editor.focused_phone_index = self.contact_editor.phones.len().saturating_sub(1);
                    }
                }
            }
            _ => {}
        }
    }

    /// Save contact
    async fn save_contact(&mut self) {
        if self.contact_editor.display_name.is_empty() {
            tracing::warn!("Cannot save contact without display name");
            return;
        }

        let contact = self.contact_editor.to_contact();

        let result = if self.contact_editor.is_editing {
            self.manager.update_contact(contact).await
        } else {
            self.manager.create_contact(contact).await
        };

        match result {
            Ok(_) => {
                tracing::info!("Contact saved successfully");
                self.contact_editor.clear();
                self.ui_mode = AddressBookMode::Browse;
                self.refresh_contacts().await;
            }
            Err(e) => {
                tracing::error!("Failed to save contact: {}", e);
                // TODO: Show error message to user
            }
        }
    }
}
