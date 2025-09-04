//! User interface for calendar sharing management

use crate::calendar::sharing::{
//     CalendarSharingManager, CalendarShare, SharedUser, SharingInvitation, CalendarPermission,
    DesktopIntegrationType,
};
use crate::theme::Theme;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
//     layout::{Alignment, Constraint, Direction, Layout, Rect},
    // style::{Color, Modifier, Style},
    // text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs, Wrap,
    },
    Frame,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

/// Calendar sharing UI tabs
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SharingTab {
    MySharedCalendars,
    SharedWithMe,
    PendingInvitations,
    PublicCalendars,
    DesktopIntegration,
    Settings,
}

impl SharingTab {
    pub fn title(&self) -> &'static str {
        match self {
            SharingTab::MySharedCalendars => "My Shared Calendars",
            SharingTab::SharedWithMe => "Shared with Me",
            SharingTab::PendingInvitations => "Pending Invitations",
            SharingTab::PublicCalendars => "Public Calendars",
            SharingTab::DesktopIntegration => "Desktop Integration",
            SharingTab::Settings => "Sharing Settings",
        }
    }

    pub fn all() -> Vec<SharingTab> {
        vec![
            SharingTab::MySharedCalendars,
            SharingTab::SharedWithMe,
            SharingTab::PendingInvitations,
            SharingTab::PublicCalendars,
            SharingTab::DesktopIntegration,
            SharingTab::Settings,
        ]
    }
}

/// Actions that can be performed in the sharing UI
#[derive(Debug, Clone, PartialEq)]
pub enum SharingAction {
    ShareCalendar(Uuid, Vec<String>, CalendarPermission),
    AcceptInvitation(String),
    DeclineInvitation(String),
    RevokeAccess(Uuid, String),
    UpdatePermissions(Uuid, String, CalendarPermission),
    EnablePublicSharing(Uuid, bool),
    DisablePublicSharing(Uuid),
    EnableDesktopIntegration(DesktopIntegrationType),
    RefreshShares,
    CreateInvitation,
    EditShare(Uuid),
    CopyPublicUrl(Uuid),
    ExportCalendar(Uuid),
}

/// Sharing UI state
pub struct SharingUIState {
    pub visible: bool,
    pub current_tab: SharingTab,
    pub selected_share: Option<Uuid>,
    pub selected_invitation: Option<Uuid>,
    pub shares_list_state: ListState,
    pub invitations_list_state: ListState,
    pub integration_list_state: ListState,
    pub share_editor: ShareEditor,
    pub status_message: Option<String>,
}

/// Share editor for creating and editing shares
pub struct ShareEditor {
    pub mode: ShareEditorMode,
    pub calendar_id: Option<Uuid>,
    pub calendar_name: String,
    pub email_input: String,
    pub permission: CalendarPermission,
    pub message: String,
    pub selected_field: ShareEditorField,
    pub edit_mode: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShareEditorMode {
    Create,
    Edit(Uuid),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShareEditorField {
    EmailInput,
    Permission,
    Message,
}

impl Default for SharingUIState {
    fn default() -> Self {
        Self {
            visible: false,
            current_tab: SharingTab::MySharedCalendars,
            selected_share: None,
            selected_invitation: None,
            shares_list_state: ListState::default(),
            invitations_list_state: ListState::default(),
            integration_list_state: ListState::default(),
            share_editor: ShareEditor::default(),
            status_message: None,
        }
    }
}

impl Default for ShareEditor {
    fn default() -> Self {
        Self {
            mode: ShareEditorMode::Create,
            calendar_id: None,
            calendar_name: String::new(),
            email_input: String::new(),
            permission: CalendarPermission::Read,
            message: String::new(),
            selected_field: ShareEditorField::EmailInput,
            edit_mode: false,
        }
    }
}

/// Calendar sharing UI component
pub struct CalendarSharingUI {
    state: SharingUIState,
    sharing_manager: Arc<Mutex<CalendarSharingManager>>,
    shares: Vec<CalendarShare>,
    invitations: Vec<SharingInvitation>,
}

impl CalendarSharingUI {
    pub fn new(sharing_manager: Arc<Mutex<CalendarSharingManager>>) -> Self {
        Self {
            state: SharingUIState::default(),
            sharing_manager,
            shares: Vec::new(),
            invitations: Vec::new(),
        }
    }

    pub fn show(&mut self) {
        self.state.visible = true;
    }

    pub fn hide(&mut self) {
        self.state.visible = false;
    }

    pub fn is_visible(&self) -> bool {
        self.state.visible
    }

    pub fn handle_key(&mut self, key: KeyCode, modifiers: KeyModifiers) -> Option<SharingAction> {
        if !self.state.visible {
            return None;
        }

        match key {
            KeyCode::Esc => {
                if self.state.share_editor.edit_mode {
                    self.state.share_editor.edit_mode = false;
                    None
                } else {
                    self.hide();
                    None
                }
            }
            KeyCode::Tab => {
                self.next_tab();
                None
            }
            KeyCode::BackTab => {
                self.previous_tab();
                None
            }
            KeyCode::Enter => {
                self.handle_enter()
            }
            KeyCode::Char(c) if modifiers.contains(KeyModifiers::CONTROL) => {
                match c {
                    'n' => Some(SharingAction::CreateInvitation),
                    'r' => Some(SharingAction::RefreshShares),
                    'c' => self.handle_copy_url(),
                    'e' => self.handle_export_calendar(),
                    _ => None,
                }
            }
            KeyCode::Char(c) => {
                if self.state.share_editor.edit_mode {
                    match self.state.share_editor.selected_field {
                        ShareEditorField::EmailInput => {
                            self.state.share_editor.email_input.push(c);
                        }
                        ShareEditorField::Message => {
                            self.state.share_editor.message.push(c);
                        }
                        _ => {}
                    }
                    None
                } else {
                    match c {
                        's' => self.handle_share_calendar(),
                        'a' => self.handle_accept_invitation(),
                        'd' => self.handle_decline_invitation(),
                        'r' => self.handle_revoke_access(),
                        'p' => self.handle_toggle_public_sharing(),
                        'e' => self.handle_edit_share(),
                        _ => None,
                    }
                }
            }
            KeyCode::Backspace => {
                if self.state.share_editor.edit_mode {
                    match self.state.share_editor.selected_field {
                        ShareEditorField::EmailInput => {
                            self.state.share_editor.email_input.pop();
                        }
                        ShareEditorField::Message => {
                            self.state.share_editor.message.pop();
                        }
                        _ => {}
                    }
                }
                None
            }
            KeyCode::Up => {
                self.move_selection_up();
                None
            }
            KeyCode::Down => {
                self.move_selection_down();
                None
            }
            _ => None,
        }
    }

    fn next_tab(&mut self) {
        let tabs = SharingTab::all();
        let current_index = tabs.iter().position(|&t| t == self.state.current_tab).unwrap_or(0);
        self.state.current_tab = tabs[(current_index + 1) % tabs.len()];
    }

    fn previous_tab(&mut self) {
        let tabs = SharingTab::all();
        let current_index = tabs.iter().position(|&t| t == self.state.current_tab).unwrap_or(0);
        self.state.current_tab = tabs[(current_index + tabs.len() - 1) % tabs.len()];
    }

    fn handle_enter(&mut self) -> Option<SharingAction> {
        match self.state.current_tab {
            SharingTab::PendingInvitations => {
                if let Some(selected) = self.state.invitations_list_state.selected() {
                    if let Some(invitation) = self.invitations.get(selected) {
                        return Some(SharingAction::AcceptInvitation(invitation.invitation_token.clone()));
                    }
                }
            }
            SharingTab::DesktopIntegration => {
                if let Some(selected) = self.state.integration_list_state.selected() {
                    let integrations = vec![
                        DesktopIntegrationType::Evolution,
                        DesktopIntegrationType::Kontact,
                        DesktopIntegrationType::Thunderbird,
                        DesktopIntegrationType::GenericCalDAV,
                        DesktopIntegrationType::DBusCalendar,
                    ];
                    
                    if let Some(integration_type) = integrations.get(selected) {
                        return Some(SharingAction::EnableDesktopIntegration(integration_type.clone()));
                    }
                }
            }
            _ => {}
        }

        None
    }

    fn handle_share_calendar(&mut self) -> Option<SharingAction> {
        if let Some(calendar_id) = self.state.selected_share {
            if !self.state.share_editor.email_input.is_empty() {
                return Some(SharingAction::ShareCalendar(
                    calendar_id,
                    vec![self.state.share_editor.email_input.clone()],
                    self.state.share_editor.permission.clone(),
                ));
            }
        }
        None
    }

    fn handle_accept_invitation(&mut self) -> Option<SharingAction> {
        if let Some(selected) = self.state.invitations_list_state.selected() {
            if let Some(invitation) = self.invitations.get(selected) {
                return Some(SharingAction::AcceptInvitation(invitation.invitation_token.clone()));
            }
        }
        None
    }

    fn handle_decline_invitation(&mut self) -> Option<SharingAction> {
        if let Some(selected) = self.state.invitations_list_state.selected() {
            if let Some(invitation) = self.invitations.get(selected) {
                return Some(SharingAction::DeclineInvitation(invitation.invitation_token.clone()));
            }
        }
        None
    }

    fn handle_revoke_access(&mut self) -> Option<SharingAction> {
        if let Some(calendar_id) = self.state.selected_share {
            // This would need a way to select the specific user to revoke
            // For now, return None - this would be implemented with a user selection dialog
        }
        None
    }

    fn handle_toggle_public_sharing(&mut self) -> Option<SharingAction> {
        if let Some(calendar_id) = self.state.selected_share {
            if let Some(share) = self.shares.iter().find(|s| s.id == calendar_id) {
                if share.is_public {
                    return Some(SharingAction::DisablePublicSharing(calendar_id));
                } else {
                    return Some(SharingAction::EnablePublicSharing(calendar_id, true));
                }
            }
        }
        None
    }

    fn handle_edit_share(&mut self) -> Option<SharingAction> {
        if let Some(calendar_id) = self.state.selected_share {
            return Some(SharingAction::EditShare(calendar_id));
        }
        None
    }

    fn handle_copy_url(&self) -> Option<SharingAction> {
        if let Some(calendar_id) = self.state.selected_share {
            return Some(SharingAction::CopyPublicUrl(calendar_id));
        }
        None
    }

    fn handle_export_calendar(&self) -> Option<SharingAction> {
        if let Some(calendar_id) = self.state.selected_share {
            return Some(SharingAction::ExportCalendar(calendar_id));
        }
        None
    }

    fn move_selection_up(&mut self) {
        match self.state.current_tab {
            SharingTab::MySharedCalendars | SharingTab::SharedWithMe | SharingTab::PublicCalendars => {
                if let Some(selected) = self.state.shares_list_state.selected() {
                    if selected > 0 {
                        self.state.shares_list_state.select(Some(selected - 1));
                    }
                }
            }
            SharingTab::PendingInvitations => {
                if let Some(selected) = self.state.invitations_list_state.selected() {
                    if selected > 0 {
                        self.state.invitations_list_state.select(Some(selected - 1));
                    }
                }
            }
            SharingTab::DesktopIntegration => {
                if let Some(selected) = self.state.integration_list_state.selected() {
                    if selected > 0 {
                        self.state.integration_list_state.select(Some(selected - 1));
                    }
                }
            }
            _ => {}
        }
    }

    fn move_selection_down(&mut self) {
        match self.state.current_tab {
            SharingTab::MySharedCalendars | SharingTab::SharedWithMe | SharingTab::PublicCalendars => {
                let selected = self.state.shares_list_state.selected().unwrap_or(0);
                if selected + 1 < self.shares.len() {
                    self.state.shares_list_state.select(Some(selected + 1));
                }
            }
            SharingTab::PendingInvitations => {
                let selected = self.state.invitations_list_state.selected().unwrap_or(0);
                if selected + 1 < self.invitations.len() {
                    self.state.invitations_list_state.select(Some(selected + 1));
                }
            }
            SharingTab::DesktopIntegration => {
                let selected = self.state.integration_list_state.selected().unwrap_or(0);
                let integration_count = 5; // Number of integration types
                if selected + 1 < integration_count {
                    self.state.integration_list_state.select(Some(selected + 1));
                }
            }
            _ => {}
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.state.visible {
            return;
        }

        // Clear the area
        frame.render_widget(Clear, area);

        // Main container
        let main_block = Block::default()
            .title("Calendar Sharing Manager")
            .borders(Borders::ALL)
            .style(theme.get_component_style("primary", false));

        let inner_area = main_block.inner(area);
        frame.render_widget(main_block, area);

        // Layout: tabs at top, content below
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(inner_area);

        // Render tabs
        self.render_tabs(frame, chunks[0], theme);

        // Render content based on current tab
        match self.state.current_tab {
            SharingTab::MySharedCalendars => self.render_my_shared_calendars(frame, chunks[1], theme),
            SharingTab::SharedWithMe => self.render_shared_with_me(frame, chunks[1], theme),
            SharingTab::PendingInvitations => self.render_pending_invitations(frame, chunks[1], theme),
            SharingTab::PublicCalendars => self.render_public_calendars(frame, chunks[1], theme),
            SharingTab::DesktopIntegration => self.render_desktop_integration(frame, chunks[1], theme),
            SharingTab::Settings => self.render_settings(frame, chunks[1], theme),
        }
    }

    fn render_tabs(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let tab_names: Vec<&str> = SharingTab::all().iter().map(|t| t.title()).collect();
        let selected_index = SharingTab::all()
            .iter()
            .position(|&t| t == self.state.current_tab)
            .unwrap_or(0);

        let tabs = Tabs::new(tab_names)
            .block(Block::default().borders(Borders::BOTTOM))
            .style(theme.get_component_style("secondary", false))
            .highlight_style(theme.get_component_style("primary", true))
            .select(selected_index);

        frame.render_widget(tabs, area);
    }

    fn render_my_shared_calendars(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let items: Vec<ListItem> = self.shares
            .iter()
            .map(|share| {
                let user_count = share.shared_with.len();
                let public_icon = if share.is_public { "🌐" } else { "🔒" };
                
                let line = format!("{} {} - {} users shared",
                    public_icon,
                    share.calendar_name,
                    user_count
                );

                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default()
                .title("My Shared Calendars")
                .borders(Borders::ALL))
            .style(theme.get_component_style("secondary", false))
            .highlight_style(theme.get_component_style("primary", true));

        frame.render_stateful_widget(list, area, &mut self.state.shares_list_state);
    }

    fn render_shared_with_me(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let text = "Calendars Shared with You\n\nCalendars that other users have shared with you will appear here.";

        let paragraph = Paragraph::new(text)
            .block(Block::default()
                .title("Shared with Me")
                .borders(Borders::ALL))
            .wrap(Wrap { trim: true })
            .style(theme.get_component_style("secondary", false));

        frame.render_widget(paragraph, area);
    }

    fn render_pending_invitations(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let items: Vec<ListItem> = self.invitations
            .iter()
            .filter(|inv| inv.accepted.is_none())
            .map(|invitation| {
                let line = format!("📧 {} invited you to '{}'",
                    invitation.from_email,
                    invitation.calendar_name
                );

                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default()
                .title("Pending Invitations")
                .borders(Borders::ALL))
            .style(theme.get_component_style("secondary", false))
            .highlight_style(theme.get_component_style("primary", true));

        frame.render_stateful_widget(list, area, &mut self.state.invitations_list_state);
    }

    fn render_public_calendars(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let text = "Public Calendars\n\nDiscoverable public calendars will appear here.";

        let paragraph = Paragraph::new(text)
            .block(Block::default()
                .title("Public Calendars")
                .borders(Borders::ALL))
            .wrap(Wrap { trim: true })
            .style(theme.get_component_style("secondary", false));

        frame.render_widget(paragraph, area);
    }

    fn render_desktop_integration(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let integration_types = vec![
            ("Evolution", "GNOME Evolution calendar integration"),
            ("Kontact", "KDE Kontact/KOrganizer integration"),
            ("Thunderbird", "Thunderbird/Lightning integration"),
            ("Generic CalDAV", "Generic CalDAV endpoint for any client"),
            ("D-Bus Calendar", "D-Bus integration for desktop notifications"),
        ];

        let items: Vec<ListItem> = integration_types
            .iter()
            .map(|(name, description)| {
                let line = format!("📅 {} - {}", name, description);
                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default()
                .title("Desktop Integration")
                .borders(Borders::ALL))
            .style(theme.get_component_style("secondary", false))
            .highlight_style(theme.get_component_style("primary", true));

        frame.render_stateful_widget(list, area, &mut self.state.integration_list_state);
    }

    fn render_settings(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let text = "Calendar Sharing Settings\n\nConfigure sharing preferences and permissions.";

        let paragraph = Paragraph::new(text)
            .block(Block::default()
                .title("Sharing Settings")
                .borders(Borders::ALL))
            .wrap(Wrap { trim: true })
            .style(theme.get_component_style("secondary", false));

        frame.render_widget(paragraph, area);
    }
}