use chrono::{DateTime, Utc};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::calendar::invitation::{MeetingInvitation, InvitationMethod, RSVPResponse};
use crate::calendar::event::{AttendeeStatus, EventStatus};
use crate::theme::Theme;

/// Meeting invitation viewer actions
#[derive(Debug, Clone, PartialEq)]
pub enum InvitationAction {
    /// Accept the invitation
    Accept,
    /// Decline the invitation
    Decline,
    /// Mark as tentative
    Tentative,
    /// Close the invitation viewer
    Close,
    /// Add to calendar without responding
    AddToCalendar,
    /// View details
    ViewDetails,
}

/// Meeting invitation viewer component
pub struct InvitationViewer {
    /// Current invitation being displayed
    invitation: Option<MeetingInvitation>,
    
    /// User's current RSVP status
    user_status: Option<AttendeeStatus>,
    
    /// Whether user is invited
    user_invited: bool,
    
    /// Selected action (for keyboard navigation)
    selected_action: usize,
    
    /// Available actions
    actions: Vec<(InvitationAction, String)>,
    
    /// Whether details are expanded
    show_details: bool,
    
    /// Processing state
    is_processing: bool,
    
    /// Error message if any
    error_message: Option<String>,
}

impl InvitationViewer {
    /// Create a new invitation viewer
    pub fn new() -> Self {
        Self {
            invitation: None,
            user_status: None,
            user_invited: false,
            selected_action: 0,
            actions: Vec::new(),
            show_details: false,
            is_processing: false,
            error_message: None,
        }
    }
    
    /// Set the invitation to display
    pub fn set_invitation(&mut self, invitation: MeetingInvitation, user_status: Option<AttendeeStatus>, user_invited: bool) {
        self.invitation = Some(invitation);
        self.user_status = user_status;
        self.user_invited = user_invited;
        self.selected_action = 0;
        self.show_details = false;
        self.error_message = None;
        self.update_actions();
    }
    
    /// Clear the current invitation
    pub fn clear(&mut self) {
        self.invitation = None;
        self.user_status = None;
        self.user_invited = false;
        self.selected_action = 0;
        self.actions.clear();
        self.show_details = false;
        self.error_message = None;
    }
    
    /// Update available actions based on invitation state
    fn update_actions(&mut self) {
        self.actions.clear();
        
        if let Some(ref invitation) = self.invitation {
            match invitation.method {
                InvitationMethod::Request => {
                    if self.user_invited {
                        // User is invited, show RSVP options
                        match self.user_status {
                            Some(AttendeeStatus::NeedsAction) | None => {
                                self.actions.push((InvitationAction::Accept, "Accept".to_string()));
                                self.actions.push((InvitationAction::Tentative, "Maybe".to_string()));
                                self.actions.push((InvitationAction::Decline, "Decline".to_string()));
                            }
                            Some(AttendeeStatus::Accepted) => {
                                self.actions.push((InvitationAction::Tentative, "Change to Maybe".to_string()));
                                self.actions.push((InvitationAction::Decline, "Change to Decline".to_string()));
                            }
                            Some(AttendeeStatus::Tentative) => {
                                self.actions.push((InvitationAction::Accept, "Change to Accept".to_string()));
                                self.actions.push((InvitationAction::Decline, "Change to Decline".to_string()));
                            }
                            Some(AttendeeStatus::Declined) => {
                                self.actions.push((InvitationAction::Accept, "Change to Accept".to_string()));
                                self.actions.push((InvitationAction::Tentative, "Change to Maybe".to_string()));
                            }
                            Some(AttendeeStatus::Delegated) => {
                                // Show view-only options for delegated
                                self.actions.push((InvitationAction::ViewDetails, "View Details".to_string()));
                            }
                        }
                    } else {
                        // User not invited, show add to calendar option
                        self.actions.push((InvitationAction::AddToCalendar, "Add to Calendar".to_string()));
                    }
                }
                InvitationMethod::Cancel => {
                    // Meeting cancelled, show informational actions
                    self.actions.push((InvitationAction::ViewDetails, "View Details".to_string()));
                }
                InvitationMethod::Reply => {
                    // Response from someone else, view only
                    self.actions.push((InvitationAction::ViewDetails, "View Details".to_string()));
                }
                _ => {
                    self.actions.push((InvitationAction::ViewDetails, "View Details".to_string()));
                }
            }
            
            // Always show details toggle and close
            if !self.show_details {
                self.actions.push((InvitationAction::ViewDetails, "Show Details".to_string()));
            }
            self.actions.push((InvitationAction::Close, "Close".to_string()));
        }
    }
    
    /// Handle keyboard input
    pub fn handle_key(&mut self, key: char) -> Option<InvitationAction> {
        match key {
            'j' | 'J' => {
                if self.selected_action < self.actions.len().saturating_sub(1) {
                    self.selected_action += 1;
                }
                None
            }
            'k' | 'K' => {
                if self.selected_action > 0 {
                    self.selected_action -= 1;
                }
                None
            }
            '\n' | ' ' => {
                // Execute selected action
                if let Some((action, _)) = self.actions.get(self.selected_action) {
                    Some(action.clone())
                } else {
                    None
                }
            }
            'a' | 'A' => Some(InvitationAction::Accept),
            'd' | 'D' => Some(InvitationAction::Decline),
            't' | 'T' => Some(InvitationAction::Tentative),
            'v' | 'V' => {
                self.show_details = !self.show_details;
                self.update_actions();
                None
            }
            'q' | 'Q' | '\x1b' => Some(InvitationAction::Close), // ESC
            _ => None,
        }
    }
    
    /// Set error message
    pub fn set_error(&mut self, error: String) {
        self.error_message = Some(error);
        self.is_processing = false;
    }
    
    /// Set processing state
    pub fn set_processing(&mut self, processing: bool) {
        self.is_processing = processing;
        if processing {
            self.error_message = None;
        }
    }
    
    /// Check if invitation viewer has content to display
    pub fn has_invitation(&self) -> bool {
        self.invitation.is_some()
    }
    
    /// Render the invitation viewer
    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if let Some(ref invitation) = self.invitation {
            // Create a centered popup
            let popup_area = self.centered_rect(85, 80, area);
            
            // Clear the background
            frame.render_widget(Clear, popup_area);
            
            // Main block
            let block = Block::default()
                .title(" Meeting Invitation ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.colors.palette.accent));
            
            let inner_area = block.inner(popup_area);
            frame.render_widget(block, popup_area);
            
            // Split into sections
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Status header
                    Constraint::Min(8),    // Content
                    Constraint::Length(3), // Actions
                    Constraint::Length(if self.error_message.is_some() { 2 } else { 0 }), // Error
                ])
                .split(inner_area);
            
            // Render status header
            self.render_status_header(frame, chunks[0], invitation, theme);
            
            // Render content
            self.render_content(frame, chunks[1], invitation, theme);
            
            // Render actions
            self.render_actions(frame, chunks[2], theme);
            
            // Render error if present
            if let Some(ref error) = self.error_message {
                let error_paragraph = Paragraph::new(error.as_str())
                    .style(Style::default().fg(Color::Red))
                    .wrap(Wrap { trim: true });
                frame.render_widget(error_paragraph, chunks[3]);
            }
        }
    }
    
    /// Render status header
    fn render_status_header(&self, frame: &mut Frame, area: Rect, invitation: &MeetingInvitation, theme: &Theme) {
        let mut lines = Vec::new();
        
        // Method and status
        let method_text = match invitation.method {
            InvitationMethod::Request => "📧 Meeting Request",
            InvitationMethod::Reply => "↩️ Meeting Response", 
            InvitationMethod::Cancel => "❌ Meeting Cancelled",
            InvitationMethod::Refresh => "🔄 Meeting Update",
            InvitationMethod::Counter => "🔄 Counter Proposal",
            InvitationMethod::DeclineCounter => "❌ Counter Declined",
        };
        
        lines.push(Line::from(vec![
            Span::styled(method_text, Style::default()
                .fg(theme.colors.palette.accent)
                .add_modifier(Modifier::BOLD)),
        ]));
        
        // User's status if invited
        if self.user_invited {
            let status_text = match self.user_status {
                Some(AttendeeStatus::Accepted) => "✅ You have accepted",
                Some(AttendeeStatus::Declined) => "❌ You have declined", 
                Some(AttendeeStatus::Tentative) => "❓ You responded maybe",
                Some(AttendeeStatus::NeedsAction) | None => "⏳ Awaiting your response",
                Some(AttendeeStatus::Delegated) => "👥 You have delegated",
            };
            
            lines.push(Line::from(vec![
                Span::styled(status_text, Style::default().fg(theme.colors.palette.text_primary)),
            ]));
        }
        
        let paragraph = Paragraph::new(lines)
            .alignment(Alignment::Center);
            
        frame.render_widget(paragraph, area);
    }
    
    /// Render invitation content
    fn render_content(&self, frame: &mut Frame, area: Rect, invitation: &MeetingInvitation, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6), // Basic info
                Constraint::Min(3),    // Details or attendees
            ])
            .split(area);
        
        // Basic information
        self.render_basic_info(frame, chunks[0], invitation, theme);
        
        // Details or attendee list
        if self.show_details {
            self.render_details(frame, chunks[1], invitation, theme);
        } else {
            self.render_attendees(frame, chunks[1], invitation, theme);
        }
    }
    
    /// Render basic meeting information
    fn render_basic_info(&self, frame: &mut Frame, area: Rect, invitation: &MeetingInvitation, theme: &Theme) {
        let mut lines = Vec::new();
        
        // Title
        lines.push(Line::from(vec![
            Span::styled("📅 ", Style::default().fg(theme.colors.palette.accent)),
            Span::styled(&invitation.title, Style::default()
                .fg(theme.colors.palette.text_primary)
                .add_modifier(Modifier::BOLD)),
        ]));
        
        // Time
        let time_format = if invitation.all_day {
            format!("🕐 {} (All day)", invitation.start_time.format("%A, %B %d, %Y"))
        } else {
            format!("🕐 {} - {}", 
                invitation.start_time.format("%A, %B %d, %Y at %I:%M %p"),
                invitation.end_time.format("%I:%M %p"))
        };
        
        lines.push(Line::from(vec![
            Span::styled(time_format, Style::default().fg(theme.colors.palette.text_primary)),
        ]));
        
        // Location
        if let Some(ref location) = invitation.location {
            lines.push(Line::from(vec![
                Span::styled("📍 ", Style::default().fg(theme.colors.palette.accent)),
                Span::styled(location, Style::default().fg(theme.colors.palette.text_primary)),
            ]));
        }
        
        // Organizer
        if let Some(ref organizer) = invitation.organizer {
            let organizer_text = organizer.name.as_ref()
                .map(|name| format!("{} <{}>", name, organizer.email))
                .unwrap_or_else(|| organizer.email.clone());
                
            lines.push(Line::from(vec![
                Span::styled("👤 ", Style::default().fg(theme.colors.palette.accent)),
                Span::styled(organizer_text, Style::default().fg(theme.colors.palette.text_primary)),
            ]));
        }
        
        // Processing indicator
        if self.is_processing {
            lines.push(Line::from(vec![
                Span::styled("⏳ Processing...", Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::ITALIC)),
            ]));
        }
        
        let paragraph = Paragraph::new(lines)
            .wrap(Wrap { trim: true });
            
        frame.render_widget(paragraph, area);
    }
    
    /// Render attendees list
    fn render_attendees(&self, frame: &mut Frame, area: Rect, invitation: &MeetingInvitation, theme: &Theme) {
        if invitation.attendees.is_empty() {
            return;
        }
        
        let mut items = Vec::new();
        
        // Header
        items.push(ListItem::new(Line::from(vec![
            Span::styled("👥 Attendees:", Style::default()
                .fg(theme.colors.palette.accent)
                .add_modifier(Modifier::BOLD)),
        ])));
        
        // Attendee list
        for attendee in &invitation.attendees {
            let status_icon = match attendee.status {
                AttendeeStatus::Accepted => "✅",
                AttendeeStatus::Declined => "❌", 
                AttendeeStatus::Tentative => "❓",
                AttendeeStatus::NeedsAction => "⏳",
                AttendeeStatus::Delegated => "👥",
            };
            
            let name_text = attendee.name.as_ref()
                .map(|name| format!("{} <{}>", name, attendee.email))
                .unwrap_or_else(|| attendee.email.clone());
            
            items.push(ListItem::new(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(status_icon, Style::default().fg(theme.colors.palette.accent)),
                Span::styled(" ", Style::default()),
                Span::styled(name_text, Style::default().fg(theme.colors.palette.text_primary)),
            ])));
        }
        
        let list = List::new(items)
            .style(Style::default().fg(theme.colors.palette.text_primary));
            
        frame.render_widget(list, area);
    }
    
    /// Render detailed information
    fn render_details(&self, frame: &mut Frame, area: Rect, invitation: &MeetingInvitation, theme: &Theme) {
        let mut lines = Vec::new();
        
        // Description
        if let Some(ref description) = invitation.description {
            lines.push(Line::from(vec![
                Span::styled("📝 Description:", Style::default()
                    .fg(theme.colors.palette.accent)
                    .add_modifier(Modifier::BOLD)),
            ]));
            
            // Split description into lines and add with indentation
            for desc_line in description.lines() {
                lines.push(Line::from(vec![
                    Span::styled("  ", Style::default()),
                    Span::styled(desc_line, Style::default().fg(theme.colors.palette.text_primary)),
                ]));
            }
            
            lines.push(Line::from(""));
        }
        
        // Additional details
        lines.push(Line::from(vec![
            Span::styled("📋 Details:", Style::default()
                .fg(theme.colors.palette.accent)
                .add_modifier(Modifier::BOLD)),
        ]));
        
        lines.push(Line::from(vec![
            Span::styled("  UID: ", Style::default().fg(theme.colors.palette.text_muted)),
            Span::styled(&invitation.uid, Style::default().fg(theme.colors.palette.text_primary)),
        ]));
        
        lines.push(Line::from(vec![
            Span::styled("  Status: ", Style::default().fg(theme.colors.palette.text_muted)),
            Span::styled(format!("{:?}", invitation.status), Style::default().fg(theme.colors.palette.text_primary)),
        ]));
        
        lines.push(Line::from(vec![
            Span::styled("  Sequence: ", Style::default().fg(theme.colors.palette.text_muted)),
            Span::styled(invitation.sequence.to_string(), Style::default().fg(theme.colors.palette.text_primary)),
        ]));
        
        let paragraph = Paragraph::new(lines)
            .wrap(Wrap { trim: true });
            
        frame.render_widget(paragraph, area);
    }
    
    /// Render action buttons
    fn render_actions(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if self.actions.is_empty() {
            return;
        }
        
        let mut items = Vec::new();
        
        for (i, (_, label)) in self.actions.iter().enumerate() {
            let style = if i == self.selected_action {
                Style::default()
                    .fg(theme.colors.palette.background)
                    .bg(theme.colors.palette.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.colors.palette.text_primary)
            };
            
            let prefix = if i == self.selected_action { "► " } else { "  " };
            
            items.push(ListItem::new(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(label, style),
            ])));
        }
        
        let list = List::new(items)
            .block(Block::default()
                .title(" Actions (↑↓ to navigate, Enter to select) ")
                .borders(Borders::TOP))
            .style(Style::default().fg(theme.colors.palette.text_primary));
            
        frame.render_widget(list, area);
    }
    
    /// Helper to create centered rectangle
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
}

impl Default for InvitationViewer {
    fn default() -> Self {
        Self::new()
    }
}