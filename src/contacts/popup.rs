//! Contact popup modal for quick access from anywhere in the app

use crate::contacts::{Contact, ContactSearchCriteria, ContactsManager};
use crate::theme::Theme;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap,
    },
    Frame,
};
use std::sync::Arc;

/// Actions that can be triggered from the contacts popup
#[derive(Debug, Clone)]
pub enum ContactPopupAction {
    /// Select contact for email composition
    SelectForEmail { to: String, name: String },
    /// Close the popup
    Close,
    /// Open full address book
    OpenFullAddressBook,
    /// View contact details
    ViewContact(Contact),
}

/// Contact popup modes
#[derive(Debug, Clone, PartialEq)]
pub enum ContactPopupMode {
    /// Quick search and select
    QuickSelect,
    /// Browse recent contacts
    Recent,
    /// Browse all contacts
    All,
    /// Browse favorites
    Favorites,
    /// Search mode
    Search,
}

/// Contact popup widget for quick access
pub struct ContactPopup {
    manager: Arc<ContactsManager>,
    
    // UI State
    mode: ContactPopupMode,
    search_query: String,
    is_searching: bool,
    list_state: ListState,
    
    // Data
    contacts: Vec<Contact>,
    filtered_contacts: Vec<Contact>,
    
    // Display settings
    show_details: bool,
    max_results: usize,
}

impl ContactPopup {
    /// Create a new contact popup
    pub fn new(manager: Arc<ContactsManager>) -> Self {
        Self {
            manager,
            mode: ContactPopupMode::QuickSelect,
            search_query: String::new(),
            is_searching: false,
            list_state: ListState::default(),
            contacts: Vec::new(),
            filtered_contacts: Vec::new(),
            show_details: false,
            max_results: 1000, // Increased to support viewing all contacts
        }
    }

    /// Initialize popup with recent contacts
    pub async fn initialize(&mut self) {
        if let Err(e) = self.load_recent_contacts().await {
            tracing::error!("Failed to load recent contacts for popup: {}", e);
        }
    }

    /// Load recent contacts
    async fn load_recent_contacts(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let criteria = ContactSearchCriteria::new().with_limit(self.max_results);
        self.contacts = self.manager.search_contacts(&criteria).await?;
        self.filtered_contacts.clear();
        
        if !self.contacts.is_empty() {
            self.list_state.select(Some(0));
        }
        
        Ok(())
    }

    /// Set popup mode
    pub fn set_mode(&mut self, mode: ContactPopupMode) {
        self.mode = mode;
        self.list_state.select(if self.get_display_contacts().is_empty() { None } else { Some(0) });
    }

    /// Get contacts to display based on current mode
    fn get_display_contacts(&self) -> &[Contact] {
        match self.mode {
            ContactPopupMode::Search if !self.filtered_contacts.is_empty() => &self.filtered_contacts,
            _ => &self.contacts,
        }
    }

    /// Load all contacts for the "All" mode
    async fn load_all_contacts(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let criteria = ContactSearchCriteria::new(); // No limit - get all contacts
        self.contacts = self.manager.search_contacts(&criteria).await?;
        self.filtered_contacts.clear();
        
        if !self.contacts.is_empty() {
            self.list_state.select(Some(0));
        }
        
        tracing::info!("📱 Loaded {} contacts for All mode", self.contacts.len());
        Ok(())
    }

    /// Render the contact popup
    pub fn render(&mut self, f: &mut Frame, area: Rect, theme: &Theme) {
        // Calculate popup size (80% of screen, centered)
        let popup_area = self.calculate_popup_area(area);
        
        // Clear the background
        f.render_widget(Clear, popup_area);
        
        // Main popup block
        let popup_block = Block::default()
            .title("📞 Quick Contacts")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.colors.palette.accent))
            .style(Style::default().bg(theme.colors.palette.background));

        let inner_area = popup_block.inner(popup_area);
        f.render_widget(popup_block, popup_area);

        // Split into header, content, and footer
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header with search
                Constraint::Min(5),    // Contact list
                Constraint::Length(2), // Footer with help
            ])
            .split(inner_area);

        // Render header with search bar
        self.render_search_header(f, chunks[0], theme);
        
        // Render contact list
        self.render_contact_list(f, chunks[1], theme);
        
        // Render footer with help
        self.render_footer(f, chunks[2], theme);
    }

    /// Calculate popup area (centered, 80% of screen)
    fn calculate_popup_area(&self, area: Rect) -> Rect {
        let popup_width = (area.width as f32 * 0.8) as u16;
        let popup_height = (area.height as f32 * 0.8) as u16;
        
        let x = (area.width.saturating_sub(popup_width)) / 2;
        let y = (area.height.saturating_sub(popup_height)) / 2;
        
        Rect {
            x: area.x + x,
            y: area.y + y,
            width: popup_width,
            height: popup_height,
        }
    }

    /// Render search header
    fn render_search_header(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let search_block = Block::default()
            .borders(Borders::ALL)
            .title(match self.mode {
                ContactPopupMode::QuickSelect => "Search Contacts",
                ContactPopupMode::Recent => "Recent Contacts",
                ContactPopupMode::All => "All Contacts",
                ContactPopupMode::Favorites => "Favorite Contacts", 
                ContactPopupMode::Search => "Search Results",
            })
            .border_style(if self.is_searching {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(theme.colors.palette.border)
            });

        let search_text = if self.is_searching {
            format!("🔍 {}_", self.search_query)
        } else if self.search_query.is_empty() {
            "Type to search contacts...".to_string()
        } else {
            format!("🔍 {} (Enter to search)", self.search_query)
        };

        let search_paragraph = Paragraph::new(search_text)
            .block(search_block)
            .style(if self.is_searching {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(theme.colors.palette.text_muted)
            });

        f.render_widget(search_paragraph, area);
    }

    /// Render contact list
    fn render_contact_list(&mut self, f: &mut Frame, area: Rect, theme: &Theme) {
        let contacts = self.get_display_contacts();
        
        if contacts.is_empty() {
            self.render_empty_state(f, area, theme);
            return;
        }

        // Create contact items without borrowing self
        let contact_items: Vec<ListItem> = contacts
            .iter()
            .enumerate()
            .map(|(_i, contact)| {
                // Create contact item without borrowing self
                let display_name = if contact.display_name.is_empty() {
                    contact.primary_email()
                        .map(|e| e.address.clone())
                        .unwrap_or_else(|| "Unknown Contact".to_string())
                } else {
                    contact.display_name.clone()
                };

                let email = contact.primary_email()
                    .map(|e| e.address.clone())
                    .unwrap_or_else(|| "No email".to_string());

                // Build enhanced contact display with more details
                let mut info_parts = Vec::new();
                
                // Add company and job title if available
                if let Some(company) = &contact.company {
                    if let Some(job_title) = &contact.job_title {
                        info_parts.push(format!("{} at {}", job_title, company));
                    } else {
                        info_parts.push(format!("🏢 {}", company));
                    }
                } else if let Some(job_title) = &contact.job_title {
                    info_parts.push(format!("💼 {}", job_title));
                }
                
                // Add phone if available
                if let Some(phone) = contact.primary_phone() {
                    info_parts.push(format!("📞 {}", phone.number));
                }
                
                // Add profile picture indicator or initial
                let avatar = if contact.photo_url.is_some() {
                    "📷".to_string() // Camera icon for contacts with photos
                } else {
                    // Generate initial from display name
                    contact.display_name.chars().next()
                        .map(|c| format!("({})", c.to_uppercase()))
                        .unwrap_or_else(|| "(??)".to_string())
                };
                
                // Add source indicator
                let source_icon = match contact.source {
                    crate::contacts::ContactSource::Google { .. } => "🌐G",
                    crate::contacts::ContactSource::Outlook { .. } => "📧O", 
                    crate::contacts::ContactSource::Local => "💾L",
                };
                
                // Format: Avatar Name <email> | Additional Info | Source
                let mut text = format!("{} {} <{}>", avatar, display_name, email);
                if !info_parts.is_empty() {
                    text.push_str(&format!(" │ {}", info_parts.join(" │ ")));
                }
                text.push_str(&format!(" │ {}", source_icon));
                
                ListItem::new(text)
                    .style(Style::default().fg(theme.colors.palette.text_primary))
            })
            .collect();

        let contacts_list = List::new(contact_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Contacts ({})", contacts.len()))
                    .border_style(Style::default().fg(theme.colors.palette.border))
            )
            .highlight_style(
                Style::default()
                    .bg(theme.colors.palette.accent)
                    .fg(theme.colors.palette.background)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        f.render_stateful_widget(contacts_list, area, &mut self.list_state);
    }

    /// Create contact list item
    #[allow(dead_code)]
    fn create_contact_item(&self, contact: &Contact, _index: usize, theme: &Theme) -> ListItem {
        let mut lines = vec![];
        
        // Main line with name and primary email
        let email_text = contact
            .primary_email()
            .map(|e| format!(" <{}>", e.address))
            .unwrap_or_default();

        let source_icon = match contact.source {
            crate::contacts::ContactSource::Google { .. } => "🌐",
            crate::contacts::ContactSource::Outlook { .. } => "📧", 
            crate::contacts::ContactSource::Local => "💾",
        };

        lines.push(Line::from(vec![
            Span::styled(
                format!("{} {}", source_icon, contact.display_name),
                Style::default()
                    .fg(theme.colors.palette.text_primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                email_text,
                Style::default().fg(theme.colors.palette.accent),
            ),
        ]));

        // Optional details line
        if self.show_details {
            let mut details = vec![];
            
            if let Some(company) = &contact.company {
                details.push(format!("🏢 {}", company));
            }
            
            if let Some(job_title) = &contact.job_title {
                details.push(format!("💼 {}", job_title));
            }
            
            if !details.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {}", details.join(" • ")),
                        Style::default().fg(theme.colors.palette.text_muted),
                    )
                ]));
            }
        }

        ListItem::new(lines)
    }

    /// Render empty state
    fn render_empty_state(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let message = match self.mode {
            ContactPopupMode::Search => "No contacts match your search",
            ContactPopupMode::Recent => "No recent contacts found",
            ContactPopupMode::All => "No contacts available",
            ContactPopupMode::Favorites => "No favorite contacts found",
            ContactPopupMode::QuickSelect => "No contacts available",
        };

        let empty_paragraph = Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    message,
                    Style::default()
                        .fg(theme.colors.palette.text_muted)
                        .add_modifier(Modifier::ITALIC),
                )
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Press 's' to sync contacts or 'f' to view all contacts",
                    Style::default().fg(theme.colors.palette.text_muted),
                )
            ]),
        ])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.colors.palette.border))
        );

        f.render_widget(empty_paragraph, area);
    }

    /// Render footer with help text
    fn render_footer(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let help_text = match self.mode {
            ContactPopupMode::QuickSelect | ContactPopupMode::Search => {
                "↑↓ Navigate | Enter View Details | Tab Toggle Details | r Recent | f All Contacts | s Sync | Esc Close"
            }
            ContactPopupMode::Recent => {
                "↑↓ Navigate | Enter View Details | / Search | f All Contacts | s Sync | Esc Close"
            }
            ContactPopupMode::All => {
                "↑↓ Navigate | Enter View Details | / Search | r Recent | s Sync | Esc Close"
            }
            ContactPopupMode::Favorites => {
                "↑↓ Navigate | Enter View Details | / Search | r Recent | f All Contacts | s Sync | Esc Close"
            }
        };

        let help_paragraph = Paragraph::new(help_text)
            .style(Style::default().fg(theme.colors.palette.text_muted))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        f.render_widget(help_paragraph, area);
    }

    /// Handle keyboard input
    pub async fn handle_key(&mut self, key: crossterm::event::KeyCode) -> Option<ContactPopupAction> {
        use crossterm::event::KeyCode;

        match key {
            KeyCode::Esc => Some(ContactPopupAction::Close),
            
            KeyCode::Enter => {
                if let Some(contact) = self.get_selected_contact() {
                    // Always show contact details when Enter is pressed
                    Some(ContactPopupAction::ViewContact(contact.clone()))
                } else {
                    None
                }
            }
            
            KeyCode::Up => {
                self.select_previous_contact();
                None
            }
            
            KeyCode::Down => {
                self.select_next_contact();
                None
            }
            
            KeyCode::Tab => {
                self.show_details = !self.show_details;
                None
            }
            
            KeyCode::Char('/') => {
                self.set_mode(ContactPopupMode::Search);
                self.is_searching = true;
                None
            }
            
            KeyCode::Char('r') => {
                self.set_mode(ContactPopupMode::Recent);
                None
            }
            
            KeyCode::Char('f') if !self.is_searching => {
                // Change 'f' to show ALL contacts instead of favorites
                self.set_mode(ContactPopupMode::All);
                // Load all contacts
                if let Err(e) = self.load_all_contacts().await {
                    tracing::error!("Failed to load all contacts: {}", e);
                }
                None
            }
            
            KeyCode::Char('F') if !self.is_searching => {
                Some(ContactPopupAction::OpenFullAddressBook)
            }
            
            KeyCode::Char('s') if !self.is_searching => {
                // 's' should trigger sync, not show limited contacts
                tracing::info!("🔄 Starting contact sync...");
                self.sync_contacts().await;
                None
            }
            
            KeyCode::Backspace if self.is_searching => {
                self.search_query.pop();
                self.perform_search().await;
                None
            }
            
            KeyCode::Char(c) if self.is_searching => {
                self.search_query.push(c);
                self.perform_search().await;
                None
            }
            
            _ => None,
        }
    }

    /// Get currently selected contact
    pub fn get_selected_contact(&self) -> Option<&Contact> {
        let contacts = self.get_display_contacts();
        self.list_state
            .selected()
            .and_then(|i| contacts.get(i))
    }

    /// Select next contact
    fn select_next_contact(&mut self) {
        let contacts = self.get_display_contacts();
        if !contacts.is_empty() {
            let i = match self.list_state.selected() {
                Some(i) => (i + 1) % contacts.len(),
                None => 0,
            };
            self.list_state.select(Some(i));
        }
    }

    /// Select previous contact
    fn select_previous_contact(&mut self) {
        let contacts = self.get_display_contacts();
        if !contacts.is_empty() {
            let i = match self.list_state.selected() {
                Some(i) => {
                    if i == 0 {
                        contacts.len() - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.list_state.select(Some(i));
        }
    }

    /// Perform search
    async fn perform_search(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_contacts.clear();
            self.list_state.select(if self.contacts.is_empty() { None } else { Some(0) });
            return;
        }

        let criteria = ContactSearchCriteria::new()
            .with_query(self.search_query.clone())
            .with_limit(self.max_results);

        match self.manager.search_contacts(&criteria).await {
            Ok(contacts) => {
                self.filtered_contacts = contacts;
                self.list_state.select(if self.filtered_contacts.is_empty() { None } else { Some(0) });
            }
            Err(e) => {
                tracing::error!("Contact search failed: {}", e);
            }
        }
    }

    /// Sync contacts
    async fn sync_contacts(&mut self) {
        if let Err(e) = self.manager.sync_all_contacts().await {
            tracing::error!("Failed to sync contacts: {}", e);
        } else {
            // Reload contacts after sync
            if let Err(e) = self.load_recent_contacts().await {
                tracing::error!("Failed to reload contacts after sync: {}", e);
            }
        }
    }

    /// Reset popup state
    pub fn reset(&mut self) {
        self.mode = ContactPopupMode::QuickSelect;
        self.search_query.clear();
        self.is_searching = false;
        self.show_details = false;
        self.filtered_contacts.clear();
        self.list_state.select(if self.contacts.is_empty() { None } else { Some(0) });
    }

    /// Check if popup is in search mode
    pub fn is_searching(&self) -> bool {
        self.is_searching
    }

    /// Get current search query
    pub fn search_query(&self) -> &str {
        &self.search_query
    }

    /// Show contact details for a specific contact
    pub fn show_contact_details(&mut self, contact: Contact) {
        // Clear search state and show the specific contact
        self.contacts = vec![contact];
        self.filtered_contacts.clear();
        self.search_query.clear();
        self.is_searching = false;
        self.show_details = true;
        self.mode = ContactPopupMode::QuickSelect;
        self.list_state.select(Some(0));
    }

    /// Start editing a specific contact
    pub fn start_edit_contact(&mut self, contact: Contact) {
        // For now, show contact details (full edit functionality would need more UI work)
        self.show_contact_details(contact);
        // TODO: Implement full contact editing interface
    }
}