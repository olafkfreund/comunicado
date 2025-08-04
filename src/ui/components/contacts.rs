//! Contacts Component Module
//!
//! Implements a modular contacts component with search, management, and integration features.

use super::{
    ComponentId, ComponentState, UIComponent, ComponentResult,
    RenderContext, UIEvent, EventResult, ComponentMetrics,
};
use crate::{
    contacts::{
        Contact, ContactsManager, AdvancedContactSearch,
        ContactAutocomplete, SenderRecognitionService,
    },
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, List, ListItem, ListState, Paragraph, Tabs, Wrap,
    },
};
use std::sync::Arc;
use std::time::{Duration, Instant};
use crossterm::event::{KeyCode, KeyEvent};
use chrono;

/// Contacts view modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactsViewMode {
    List,
    Search,
    Details,
    Edit,
    Create,
}

impl ContactsViewMode {
    pub fn name(&self) -> &'static str {
        match self {
            ContactsViewMode::List => "List",
            ContactsViewMode::Search => "Search",
            ContactsViewMode::Details => "Details",
            ContactsViewMode::Edit => "Edit",
            ContactsViewMode::Create => "Create",
        }
    }
}

/// Contact tabs for different sources
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactTab {
    All,
    Local,
    Google,
    Outlook,
    Recent,
}

impl ContactTab {
    pub fn name(&self) -> &'static str {
        match self {
            ContactTab::All => "All",
            ContactTab::Local => "Local",
            ContactTab::Google => "Google",
            ContactTab::Outlook => "Outlook",
            ContactTab::Recent => "Recent",
        }
    }

    pub fn all() -> &'static [ContactTab] {
        &[
            ContactTab::All,
            ContactTab::Local,
            ContactTab::Google,
            ContactTab::Outlook,
            ContactTab::Recent,
        ]
    }
}

/// Contacts panes for focus management
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactsPane {
    Tabs,
    ContactList,
    ContactDetails,
    SearchBox,
}

/// Contact actions
#[derive(Debug, Clone)]
pub enum ContactAction {
    SelectContact(i64), // Contact ID
    CreateContact,
    EditContact(i64),
    DeleteContact(i64),
    ComposeEmail(String), // Email address
    ViewDetails(i64),
    Search(String),
    ClearSearch,
    SwitchTab(ContactTab),
    ExportContacts,
    ImportContacts,
    Sync,
}

/// Contact form field for editing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactField {
    DisplayName,
    FirstName,
    LastName,
    Company,
    JobTitle,
    Email,
    Phone,
    Notes,
}

/// Contacts component that manages contact listing, search, and operations
pub struct ContactsComponent {
    // Component metadata
    id: ComponentId,
    state: ComponentState,
    metrics: ComponentMetrics,
    
    // View state
    current_view: ContactsViewMode,
    current_tab: ContactTab,
    focused_pane: ContactsPane,
    
    // Data and services
    contacts_manager: Option<Arc<ContactsManager>>,
    sender_recognition: Option<Arc<SenderRecognitionService>>,
    #[allow(dead_code)]
    advanced_search: Option<AdvancedContactSearch>,
    #[allow(dead_code)]
    autocomplete: Option<ContactAutocomplete>,
    
    // Contact data
    contacts: Vec<Contact>,
    filtered_contacts: Vec<Contact>,
    selected_contact: Option<Contact>,
    
    // UI state
    contact_list_state: ListState,
    tab_index: usize,
    search_query: String,
    is_searching: bool,
    
    // Contact editing
    editing_contact: Option<Contact>,
    #[allow(dead_code)]
    focused_field: ContactField,
    
    // Statistics
    total_contacts: usize,
    local_contacts: usize,
    google_contacts: usize,
    outlook_contacts: usize,
    
    // Performance tracking
    #[allow(dead_code)]
    last_render_time: Instant,
    render_count: u64,
}

impl ContactsComponent {
    /// Create a new contacts component
    pub fn new() -> Self {
        Self {
            id: ComponentId::new::<Self>(),
            state: ComponentState::Uninitialized,
            metrics: ComponentMetrics::default(),
            current_view: ContactsViewMode::List,
            current_tab: ContactTab::All,
            focused_pane: ContactsPane::ContactList,
            contacts_manager: None,
            sender_recognition: None,
            advanced_search: None,
            autocomplete: None,
            contacts: Vec::new(),
            filtered_contacts: Vec::new(),
            selected_contact: None,
            contact_list_state: ListState::default(),
            tab_index: 0,
            search_query: String::new(),
            is_searching: false,
            editing_contact: None,
            focused_field: ContactField::DisplayName,
            total_contacts: 0,
            local_contacts: 0,
            google_contacts: 0,
            outlook_contacts: 0,
            last_render_time: Instant::now(),
            render_count: 0,
        }
    }
    
    /// Initialize with contacts manager and services
    pub fn with_services(
        mut self,
        contacts_manager: Option<Arc<ContactsManager>>,
        sender_recognition: Option<Arc<SenderRecognitionService>>,
    ) -> Self {
        self.contacts_manager = contacts_manager;
        self.sender_recognition = sender_recognition;
        
        // Initialize search and autocomplete if manager is available
        if let Some(ref _manager) = self.contacts_manager {
            // TODO: Initialize advanced search and autocomplete
            // self.advanced_search = Some(AdvancedContactSearch::new(manager.clone()));
            // self.autocomplete = Some(ContactAutocomplete::new(manager.clone()));
        }
        
        self
    }
    
    /// Get the current view mode
    pub fn current_view(&self) -> ContactsViewMode {
        self.current_view
    }
    
    /// Set the view mode
    pub fn set_view_mode(&mut self, mode: ContactsViewMode) -> ComponentResult<()> {
        self.current_view = mode;
        
        // Update focused pane based on view
        match mode {
            ContactsViewMode::List => {
                self.focused_pane = ContactsPane::ContactList;
            }
            ContactsViewMode::Search => {
                self.focused_pane = ContactsPane::SearchBox;
            }
            ContactsViewMode::Details | ContactsViewMode::Edit | ContactsViewMode::Create => {
                self.focused_pane = ContactsPane::ContactDetails;
            }
        }
        
        Ok(())
    }
    
    /// Get the current tab
    pub fn current_tab(&self) -> ContactTab {
        self.current_tab
    }
    
    /// Set the current tab
    pub fn set_tab(&mut self, tab: ContactTab) -> ComponentResult<()> {
        self.current_tab = tab;
        self.tab_index = ContactTab::all()
            .iter()
            .position(|&t| t == tab)
            .unwrap_or(0);
        
        // Filter contacts based on tab
        self.apply_tab_filter();
        
        Ok(())
    }
    
    /// Set contacts data
    pub fn set_contacts(&mut self, contacts: Vec<Contact>) {
        self.contacts = contacts;
        self.update_statistics();
        self.apply_current_filters();
    }
    
    /// Get selected contact
    pub fn selected_contact(&self) -> Option<&Contact> {
        self.selected_contact.as_ref()
    }
    
    /// Set search query
    pub fn set_search_query(&mut self, query: String) {
        self.search_query = query;
        self.is_searching = !self.search_query.is_empty();
        self.apply_search_filter();
    }
    
    /// Handle contact actions
    fn handle_contact_action(&mut self, action: ContactAction) -> ComponentResult<EventResult> {
        match action {
            ContactAction::SelectContact(contact_id) => {
                self.selected_contact = self.contacts.iter()
                    .find(|c| c.id.unwrap_or(0) == contact_id)
                    .cloned();
                Ok(EventResult::Handled)
            }
            ContactAction::CreateContact => {
                self.set_view_mode(ContactsViewMode::Create)?;
                // Create a new local contact 
                self.editing_contact = Some(Contact::new(
                    format!("local-{}", chrono::Utc::now().timestamp()),
                    crate::contacts::ContactSource::Local,
                    "New Contact".to_string()
                ));
                Ok(EventResult::Handled)
            }
            ContactAction::EditContact(contact_id) => {
                if let Some(contact) = self.contacts.iter().find(|c| c.id.unwrap_or(0) == contact_id) {
                    self.editing_contact = Some(contact.clone());
                    self.set_view_mode(ContactsViewMode::Edit)?;
                }
                Ok(EventResult::Handled)
            }
            ContactAction::ViewDetails(contact_id) => {
                self.selected_contact = self.contacts.iter()
                    .find(|c| c.id.unwrap_or(0) == contact_id)
                    .cloned();
                self.set_view_mode(ContactsViewMode::Details)?;
                Ok(EventResult::Handled)
            }
            ContactAction::Search(query) => {
                self.set_search_query(query);
                self.set_view_mode(ContactsViewMode::Search)?;
                Ok(EventResult::Handled)
            }
            ContactAction::ClearSearch => {
                self.set_search_query(String::new());
                self.set_view_mode(ContactsViewMode::List)?;
                Ok(EventResult::Handled)
            }
            ContactAction::SwitchTab(tab) => {
                self.set_tab(tab)?;
                Ok(EventResult::Handled)
            }
            ContactAction::ComposeEmail(_email) => {
                // TODO: Trigger email composition with this address
                Ok(EventResult::RequestModeChange("compose".to_string()))
            }
            ContactAction::DeleteContact(_contact_id) => {
                // TODO: Implement contact deletion
                Ok(EventResult::Handled)
            }
            ContactAction::ExportContacts => {
                // TODO: Implement contact export
                Ok(EventResult::Handled)
            }
            ContactAction::ImportContacts => {
                // TODO: Implement contact import
                Ok(EventResult::Handled)
            }
            ContactAction::Sync => {
                // TODO: Trigger contact sync
                Ok(EventResult::Handled)
            }
        }
    }
    
    /// Apply tab filter to contacts
    fn apply_tab_filter(&mut self) {
        match self.current_tab {
            ContactTab::All => {
                self.filtered_contacts = self.contacts.clone();
            }
            ContactTab::Local => {
                self.filtered_contacts = self.contacts.iter()
                    .filter(|c| matches!(c.source, crate::contacts::ContactSource::Local))
                    .cloned()
                    .collect();
            }
            ContactTab::Google => {
                self.filtered_contacts = self.contacts.iter()
                    .filter(|c| matches!(c.source, crate::contacts::ContactSource::Google { .. }))
                    .cloned()
                    .collect();
            }
            ContactTab::Outlook => {
                self.filtered_contacts = self.contacts.iter()
                    .filter(|c| matches!(c.source, crate::contacts::ContactSource::Outlook { .. }))
                    .cloned()
                    .collect();
            }
            ContactTab::Recent => {
                // TODO: Filter by recently accessed contacts
                self.filtered_contacts = self.contacts.clone();
            }
        }
    }
    
    /// Apply search filter to contacts
    fn apply_search_filter(&mut self) {
        if self.search_query.is_empty() {
            self.apply_tab_filter();
            return;
        }
        
        let query = self.search_query.to_lowercase();
        self.filtered_contacts = self.contacts.iter()
            .filter(|contact| {
                contact.display_name.to_lowercase().contains(&query) ||
                contact.first_name.as_deref().unwrap_or("").to_lowercase().contains(&query) ||
                contact.last_name.as_deref().unwrap_or("").to_lowercase().contains(&query) ||
                contact.company.as_deref().unwrap_or("").to_lowercase().contains(&query) ||
                contact.emails.iter().any(|email| email.address.to_lowercase().contains(&query))
            })
            .cloned()
            .collect();
    }
    
    /// Apply all current filters
    fn apply_current_filters(&mut self) {
        if self.is_searching {
            self.apply_search_filter();
        } else {
            self.apply_tab_filter();
        }
    }
    
    /// Update contact statistics
    fn update_statistics(&mut self) {
        self.total_contacts = self.contacts.len();
        self.local_contacts = self.contacts.iter()
            .filter(|c| matches!(c.source, crate::contacts::ContactSource::Local))
            .count();
        self.google_contacts = self.contacts.iter()
            .filter(|c| matches!(c.source, crate::contacts::ContactSource::Google { .. }))
            .count();
        self.outlook_contacts = self.contacts.iter()
            .filter(|c| matches!(c.source, crate::contacts::ContactSource::Outlook { .. }))
            .count();
    }
    
    /// Render the contacts header with tabs
    fn render_header(&self, context: &mut RenderContext<'_>, area: Rect) -> ComponentResult<()> {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(40),     // Tabs
                Constraint::Length(30),  // Search box
                Constraint::Length(20),  // Actions
            ])
            .split(area);

        // Render tabs
        let tab_titles: Vec<Line> = ContactTab::all()
            .iter()
            .enumerate()
            .map(|(_i, tab)| {
                let count = match tab {
                    ContactTab::All => self.total_contacts,
                    ContactTab::Local => self.local_contacts,
                    ContactTab::Google => self.google_contacts,
                    ContactTab::Outlook => self.outlook_contacts,
                    ContactTab::Recent => 0, // TODO: Calculate recent contacts
                };
                Line::from(format!("{} ({})", tab.name(), count))
            })
            .collect();

        let tabs = Tabs::new(tab_titles)
            .block(Block::default().borders(Borders::ALL).title("Sources"))
            .highlight_style(
                Style::default()
                    .fg(context.theme.colors.palette.accent)
                    .add_modifier(Modifier::BOLD)
            )
            .select(self.tab_index);

        context.frame.render_widget(tabs, chunks[0]);

        // Render search box
        let search_text = if self.search_query.is_empty() {
            "Type to search...".to_string()
        } else {
            self.search_query.clone()
        };

        let search_box = Paragraph::new(search_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Search")
                    .border_style(Style::default().fg(
                        if self.focused_pane == ContactsPane::SearchBox && context.is_focused {
                            context.theme.colors.palette.accent
                        } else {
                            context.theme.colors.palette.border
                        }
                    ))
            )
            .style(Style::default().fg(
                if self.search_query.is_empty() {
                    context.theme.colors.palette.text_muted
                } else {
                    context.theme.colors.palette.text_primary
                }
            ));

        context.frame.render_widget(search_box, chunks[1]);

        // Render actions
        let actions_text = "N: New\nE: Edit\nD: Delete\nS: Sync";
        let actions = Paragraph::new(actions_text)
            .block(Block::default().borders(Borders::ALL).title("Actions"))
            .style(Style::default().fg(context.theme.colors.palette.text_muted));

        context.frame.render_widget(actions, chunks[2]);
        
        Ok(())
    }
    
    /// Render contact list
    fn render_contact_list(&mut self, context: &mut RenderContext<'_>, area: Rect) -> ComponentResult<()> {
        let list_items: Vec<ListItem> = self.filtered_contacts.iter()
            .map(|contact| {
                let primary_email = contact.emails.first()
                    .map(|e| e.address.as_str())
                    .unwrap_or("No email");
                
                let company_info = contact.company.as_deref().unwrap_or("");
                let company_display = if company_info.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", company_info)
                };
                
                let content = format!(
                    "{}{}\n  📧 {}",
                    contact.display_name,
                    company_display,
                    primary_email
                );
                
                ListItem::new(content)
                    .style(Style::default().fg(context.theme.colors.palette.text_primary))
            })
            .collect();

        let list = List::new(list_items)
            .block(
                Block::default()
                    .title(format!("Contacts ({})", self.filtered_contacts.len()))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(
                        if self.focused_pane == ContactsPane::ContactList && context.is_focused {
                            context.theme.colors.palette.accent
                        } else {
                            context.theme.colors.palette.border
                        }
                    ))
            )
            .highlight_style(
                Style::default()
                    .fg(context.theme.colors.palette.accent)
                    .add_modifier(Modifier::BOLD)
            );

        context.frame.render_stateful_widget(list, area, &mut self.contact_list_state);
        Ok(())
    }
    
    /// Render contact details
    fn render_contact_details(&self, context: &mut RenderContext<'_>, area: Rect) -> ComponentResult<()> {
        if let Some(ref contact) = self.selected_contact {
            let mut lines = Vec::new();
            
            // Basic info
            lines.push(Line::from(vec![
                Span::styled("Name: ", Style::default().fg(context.theme.colors.palette.accent)),
                Span::styled(&contact.display_name, Style::default().fg(context.theme.colors.palette.text_primary)),
            ]));
            
            if let Some(ref company) = contact.company {
                if !company.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled("Company: ", Style::default().fg(context.theme.colors.palette.accent)),
                        Span::styled(company, Style::default().fg(context.theme.colors.palette.text_primary)),
                    ]));
                }
            }
            
            if let Some(ref job_title) = contact.job_title {
                if !job_title.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled("Title: ", Style::default().fg(context.theme.colors.palette.accent)),
                        Span::styled(job_title, Style::default().fg(context.theme.colors.palette.text_primary)),
                    ]));
                }
            }
            
            lines.push(Line::from(""));
            
            // Email addresses
            if !contact.emails.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("📧 Email Addresses:", Style::default().fg(context.theme.colors.palette.accent)),
                ]));
                
                for email in &contact.emails {
                    let label = if email.label.is_empty() { "Personal" } else { &email.label };
                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(&email.address, Style::default().fg(context.theme.colors.palette.text_primary)),
                        Span::styled(format!(" ({})", label), Style::default().fg(context.theme.colors.palette.text_muted)),
                    ]));
                }
                lines.push(Line::from(""));
            }
            
            // Phone numbers
            if !contact.phones.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("📞 Phone Numbers:", Style::default().fg(context.theme.colors.palette.accent)),
                ]));
                
                for phone in &contact.phones {
                    let label = if phone.label.is_empty() { "Personal" } else { &phone.label };
                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(&phone.number, Style::default().fg(context.theme.colors.palette.text_primary)),
                        Span::styled(format!(" ({})", label), Style::default().fg(context.theme.colors.palette.text_muted)),
                    ]));
                }
                lines.push(Line::from(""));
            }
            
            // Notes
            if let Some(ref notes) = contact.notes {
                if !notes.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled("📝 Notes:", Style::default().fg(context.theme.colors.palette.accent)),
                    ]));
                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(notes, Style::default().fg(context.theme.colors.palette.text_primary)),
                    ]));
                }
            }
            
            let details = Paragraph::new(lines)
                .block(
                    Block::default()
                        .title("Contact Details")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(
                            if self.focused_pane == ContactsPane::ContactDetails && context.is_focused {
                                context.theme.colors.palette.accent
                            } else {
                                context.theme.colors.palette.border
                            }
                        ))
                )
                .wrap(Wrap { trim: true });
                
            context.frame.render_widget(details, area);
        } else {
            let placeholder = Paragraph::new("Select a contact to view details")
                .block(
                    Block::default()
                        .title("Contact Details")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(context.theme.colors.palette.border))
                )
                .alignment(Alignment::Center)
                .style(Style::default().fg(context.theme.colors.palette.text_muted));
                
            context.frame.render_widget(placeholder, area);
        }
        
        Ok(())
    }
    
    /// Handle key events
    fn handle_key_event(&mut self, key: KeyEvent) -> ComponentResult<EventResult> {
        match key.code {
            KeyCode::Char('1') => self.handle_contact_action(ContactAction::SwitchTab(ContactTab::All)),
            KeyCode::Char('2') => self.handle_contact_action(ContactAction::SwitchTab(ContactTab::Local)),
            KeyCode::Char('3') => self.handle_contact_action(ContactAction::SwitchTab(ContactTab::Google)),
            KeyCode::Char('4') => self.handle_contact_action(ContactAction::SwitchTab(ContactTab::Outlook)),
            KeyCode::Char('5') => self.handle_contact_action(ContactAction::SwitchTab(ContactTab::Recent)),
            KeyCode::Char('n') | KeyCode::Char('N') => self.handle_contact_action(ContactAction::CreateContact),
            KeyCode::Char('s') | KeyCode::Char('S') => self.handle_contact_action(ContactAction::Sync),
            KeyCode::Char('/') => {
                self.set_view_mode(ContactsViewMode::Search)?;
                Ok(EventResult::Handled)
            }
            KeyCode::Esc => {
                if self.current_view == ContactsViewMode::Search {
                    self.handle_contact_action(ContactAction::ClearSearch)
                } else if self.current_view != ContactsViewMode::List {
                    self.set_view_mode(ContactsViewMode::List)?;
                    Ok(EventResult::Handled)
                } else {
                    Ok(EventResult::Ignored)
                }
            }
            KeyCode::Enter => {
                if let Some(selected_idx) = self.contact_list_state.selected() {
                    if let Some(contact) = self.filtered_contacts.get(selected_idx) {
                        if let Some(contact_id) = contact.id {
                            return self.handle_contact_action(ContactAction::ViewDetails(contact_id));
                        }
                    }
                }
                Ok(EventResult::Ignored)
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.focused_pane == ContactsPane::ContactList {
                    let selected = self.contact_list_state.selected().unwrap_or(0);
                    let new_selected = if selected > 0 { selected - 1 } else { self.filtered_contacts.len().saturating_sub(1) };
                    self.contact_list_state.select(Some(new_selected));
                }
                Ok(EventResult::Handled)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.focused_pane == ContactsPane::ContactList {
                    let selected = self.contact_list_state.selected().unwrap_or(0);
                    let new_selected = if selected < self.filtered_contacts.len().saturating_sub(1) { selected + 1 } else { 0 };
                    self.contact_list_state.select(Some(new_selected));
                }
                Ok(EventResult::Handled)
            }
            KeyCode::Tab => {
                // Cycle through panes
                self.focused_pane = match self.focused_pane {
                    ContactsPane::Tabs => ContactsPane::SearchBox,
                    ContactsPane::SearchBox => ContactsPane::ContactList,
                    ContactsPane::ContactList => ContactsPane::ContactDetails,
                    ContactsPane::ContactDetails => ContactsPane::Tabs,
                };
                Ok(EventResult::Handled)
            }
            _ => Ok(EventResult::Ignored),
        }
    }
}

impl UIComponent for ContactsComponent {
    fn component_id(&self) -> ComponentId {
        self.id
    }
    
    fn component_name(&self) -> &str {
        "ContactsComponent"
    }
    
    fn state(&self) -> ComponentState {
        self.state
    }
    
    fn initialize(&mut self) -> ComponentResult<()> {
        self.state = ComponentState::Ready;
        // Select first contact if available
        if !self.filtered_contacts.is_empty() {
            self.contact_list_state.select(Some(0));
        }
        Ok(())
    }
    
    fn render(&mut self, context: &mut RenderContext<'_>) -> ComponentResult<()> {
        let start_time = Instant::now();
        
        // Create main layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(5),    // Main content
            ])
            .split(context.area);

        // Render header
        self.render_header(context, chunks[0])?;

        // Render main content based on current view
        match self.current_view {
            ContactsViewMode::List | ContactsViewMode::Search => {
                // Split between contact list and details
                let main_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(60),
                        Constraint::Percentage(40),
                    ])
                    .split(chunks[1]);

                self.render_contact_list(context, main_chunks[0])?;
                self.render_contact_details(context, main_chunks[1])?;
            }
            ContactsViewMode::Details => {
                self.render_contact_details(context, chunks[1])?;
            }
            ContactsViewMode::Edit | ContactsViewMode::Create => {
                // TODO: Implement contact editor view
                let placeholder = Paragraph::new("Contact editor - Not implemented yet")
                    .block(
                        Block::default()
                            .title("Edit Contact")
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(context.theme.colors.palette.border))
                    )
                    .alignment(Alignment::Center);
                    
                context.frame.render_widget(placeholder, chunks[1]);
            }
        }
        
        // Update render metrics
        let render_time = start_time.elapsed();
        self.metrics.last_render_time = render_time;
        self.metrics.render_calls += 1;
        self.render_count += 1;
        
        // Update average render time
        let weight = 0.1;
        self.metrics.avg_render_time = Duration::from_nanos(
            (self.metrics.avg_render_time.as_nanos() as f64 * (1.0 - weight) +
             render_time.as_nanos() as f64 * weight) as u64
        );
        
        self.metrics.last_updated = Instant::now();
        
        Ok(())
    }
    
    fn handle_event(&mut self, event: &UIEvent) -> ComponentResult<EventResult> {
        match event {
            UIEvent::Key(key) => {
                self.metrics.events_processed += 1;
                self.handle_key_event(*key)
            }
            UIEvent::FocusGained => {
                Ok(EventResult::Handled)
            }
            UIEvent::FocusLost => {
                Ok(EventResult::Handled)
            }
            _ => Ok(EventResult::Ignored),
        }
    }
    
    fn metrics(&self) -> &ComponentMetrics {
        &self.metrics
    }
    
    fn set_state(&mut self, new_state: ComponentState) -> ComponentResult<()> {
        self.state = new_state;
        Ok(())
    }
    
    fn can_focus(&self) -> bool {
        matches!(self.state, ComponentState::Ready | ComponentState::Focused)
    }
    
    fn cleanup(&mut self) -> ComponentResult<()> {
        self.state = ComponentState::Destroying;
        Ok(())
    }
}

impl Default for ContactsComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for ContactsComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContactsComponent")
            .field("id", &self.id)
            .field("state", &self.state)
            .field("view_mode", &self.current_view)
            .field("current_tab", &self.current_tab)
            .field("focused_pane", &self.focused_pane)
            .field("search_query", &self.search_query)
            .field("contacts_count", &self.contacts.len())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_contacts_component_creation() {
        let component = ContactsComponent::new();
        assert_eq!(component.state(), ComponentState::Uninitialized);
        assert_eq!(component.current_view(), ContactsViewMode::List);
        assert_eq!(component.current_tab(), ContactTab::All);
        assert_eq!(component.component_name(), "ContactsComponent");
    }
    
    #[test]
    fn test_contacts_component_initialization() {
        let mut component = ContactsComponent::new();
        component.initialize().unwrap();
        assert_eq!(component.state(), ComponentState::Ready);
    }
    
    #[test]
    fn test_view_mode_switching() {
        let mut component = ContactsComponent::new();
        component.initialize().unwrap();
        
        // Switch to search view
        component.set_view_mode(ContactsViewMode::Search).unwrap();
        assert_eq!(component.current_view(), ContactsViewMode::Search);
        
        // Switch to details view
        component.set_view_mode(ContactsViewMode::Details).unwrap();
        assert_eq!(component.current_view(), ContactsViewMode::Details);
    }
    
    #[test]
    fn test_tab_switching() {
        let mut component = ContactsComponent::new();
        component.initialize().unwrap();
        
        // Switch to local tab
        component.set_tab(ContactTab::Local).unwrap();
        assert_eq!(component.current_tab(), ContactTab::Local);
        
        // Switch to Google tab
        component.set_tab(ContactTab::Google).unwrap();
        assert_eq!(component.current_tab(), ContactTab::Google);
    }
    
    #[test]
    fn test_search_functionality() {
        let mut component = ContactsComponent::new();
        component.initialize().unwrap();
        
        // Add some test contacts
        let mut contact1 = Contact::new(
            "john-doe-1".to_string(),
            crate::contacts::ContactSource::Local,
            "John Doe".to_string()
        );
        contact1.id = Some(1);
        contact1.first_name = Some("John".to_string());
        contact1.last_name = Some("Doe".to_string());
        contact1.company = Some("Acme Corp".to_string());

        let mut contact2 = Contact::new(
            "jane-smith-2".to_string(),
            crate::contacts::ContactSource::Local,
            "Jane Smith".to_string()
        );
        contact2.id = Some(2);
        contact2.first_name = Some("Jane".to_string());
        contact2.last_name = Some("Smith".to_string());
        contact2.company = Some("Tech Inc".to_string());

        let contacts = vec![contact1, contact2];
        
        component.set_contacts(contacts);
        
        // Test search
        component.set_search_query("John".to_string());
        assert_eq!(component.filtered_contacts.len(), 1);
        assert_eq!(component.filtered_contacts[0].display_name, "John Doe");
        
        // Clear search
        component.set_search_query(String::new());
        assert_eq!(component.filtered_contacts.len(), 2);
    }
}