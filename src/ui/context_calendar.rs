//! Context-Aware Calendar Integration
//! 
//! Automatically shows relevant calendar information based on email content
//! and user context, without cluttering the interface unnecessarily.

use crate::{
    calendar::{Event, Calendar},
    email::StoredMessage,
    theme::Theme,
};
use chrono::{DateTime, Local, NaiveDate, Utc};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};
use std::collections::HashMap;

/// Context analysis for determining what calendar info to show
#[derive(Debug, Clone)]
pub struct EmailCalendarContext {
    pub email_id: Option<String>,
    pub has_calendar_invitation: bool,
    pub has_meeting_references: bool,
    pub has_date_mentions: bool,
    pub mentioned_dates: Vec<NaiveDate>,
    pub invitation_event_id: Option<String>,
    pub suggested_events: Vec<String>,
    pub urgency_level: ContextUrgency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextUrgency {
    None,        // No calendar relevance
    Low,         // Minor date mentions
    Medium,      // Meeting references
    High,        // Calendar invitations
    Critical,    // Conflicts or urgent responses needed
}

/// Calendar information display modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalendarDisplayMode {
    Hidden,              // No calendar info shown
    InvitationDetails,   // Show event details and RSVP options
    DailyAgenda,         // Show today's schedule
    WeeklyOverview,      // Show week view
    ConflictWarning,     // Show scheduling conflicts
    QuickSchedule,       // Show relevant time slots
}

/// Context-aware calendar widget
pub struct ContextAwareCalendar {
    // Current context
    current_context: EmailCalendarContext,
    display_mode: CalendarDisplayMode,
    
    // Calendar data
    events: Vec<Event>,
    calendars: Vec<Calendar>,
    #[allow(dead_code)]
    current_date: NaiveDate,
    
    // UI state
    #[allow(dead_code)]
    is_visible: bool,
    selected_action: usize,
    #[allow(dead_code)]
    rsvp_response: Option<RsvpResponse>,
    
    // Cache for performance
    agenda_cache: HashMap<NaiveDate, Vec<Event>>,
    last_update: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RsvpResponse {
    Accept,
    Decline,
    Maybe,
    NoResponse,
}

#[derive(Debug, Clone)]
pub enum CalendarAction {
    RespondToInvitation(String, RsvpResponse), // event_id, response
    ViewEventDetails(String),
    CreateMeeting,
    CheckSchedule(NaiveDate),
    AddToCalendar(String), // Quick add from email content
    HideCalendarContext,
}

impl ContextAwareCalendar {
    pub fn new() -> Self {
        Self {
            current_context: EmailCalendarContext::default(),
            display_mode: CalendarDisplayMode::Hidden,
            events: Vec::new(),
            calendars: Vec::new(),
            current_date: Local::now().date_naive(),
            is_visible: false,
            selected_action: 0,
            rsvp_response: None,
            agenda_cache: HashMap::new(),
            last_update: Utc::now(),
        }
    }

    /// Analyze email content and determine calendar context from StoredMessage
    pub fn analyze_email_context(&mut self, email: &StoredMessage) -> CalendarDisplayMode {
        let content = email.body_text.as_deref().unwrap_or(&email.subject);
        let subject = &email.subject;
        
        // Reset context
        self.current_context = EmailCalendarContext::default();
        self.current_context.email_id = Some(email.id.to_string());

        self.analyze_content_for_calendar_context(content, subject)
    }

    /// Analyze email content and determine calendar context from MessageItem
    pub fn analyze_message_item_context(&mut self, message: &crate::ui::message_list::MessageItem) -> CalendarDisplayMode {
        let content = &message.subject; // MessageItem doesn't have body content
        let subject = &message.subject;
        
        // Reset context
        self.current_context = EmailCalendarContext::default();
        self.current_context.email_id = message.message_id.map(|id| id.to_string());

        self.analyze_content_for_calendar_context(content, subject)
    }

    /// Internal method to analyze content for calendar context
    fn analyze_content_for_calendar_context(&mut self, content: &str, subject: &str) -> CalendarDisplayMode {
        // Check for calendar invitations
        if self.is_calendar_invitation(content, subject) {
            self.current_context.has_calendar_invitation = true;
            self.current_context.urgency_level = ContextUrgency::High;
            self.display_mode = CalendarDisplayMode::InvitationDetails;
            return CalendarDisplayMode::InvitationDetails;
        }

        // Check for meeting references
        if self.has_meeting_keywords(content, subject) {
            self.current_context.has_meeting_references = true;
            self.current_context.urgency_level = ContextUrgency::Medium;
            self.display_mode = CalendarDisplayMode::DailyAgenda;
            return CalendarDisplayMode::DailyAgenda;
        }

        // Check for date mentions
        let mentioned_dates = self.extract_date_mentions(content, subject);
        if !mentioned_dates.is_empty() {
            self.current_context.has_date_mentions = true;
            self.current_context.mentioned_dates = mentioned_dates;
            self.current_context.urgency_level = ContextUrgency::Low;
            self.display_mode = CalendarDisplayMode::QuickSchedule;
            return CalendarDisplayMode::QuickSchedule;
        }

        // No calendar context found
        self.display_mode = CalendarDisplayMode::Hidden;
        CalendarDisplayMode::Hidden
    }

    /// Render the context-aware calendar widget
    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if self.display_mode == CalendarDisplayMode::Hidden {
            return;
        }

        match self.display_mode {
            CalendarDisplayMode::InvitationDetails => {
                self.render_invitation_details(frame, area, theme);
            },
            CalendarDisplayMode::DailyAgenda => {
                self.render_daily_agenda(frame, area, theme);
            },
            CalendarDisplayMode::QuickSchedule => {
                self.render_quick_schedule(frame, area, theme);
            },
            CalendarDisplayMode::ConflictWarning => {
                self.render_conflict_warning(frame, area, theme);
            },
            _ => {}
        }
    }

    /// Render calendar invitation details and RSVP options
    fn render_invitation_details(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .title("📅 Calendar Invitation")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.colors.palette.accent));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Split into sections: event details + RSVP buttons
        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),     // Event details
                Constraint::Length(3),  // RSVP buttons
            ])
            .split(inner);

        // Event details (mock data for now)
        let event_details = vec![
            Line::from(vec![
                Span::styled("📍 ", Style::default().fg(Color::Blue)),
                Span::styled("Team Meeting", Style::default().add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("🕐 ", Style::default().fg(Color::Green)),
                Span::raw("Thu, Jul 30 at 2:00 PM - 3:00 PM"),
            ]),
            Line::from(vec![
                Span::styled("📍 ", Style::default().fg(Color::Yellow)),
                Span::raw("Conference Room B"),
            ]),
            Line::from(""),
            Line::from("Project planning and Q3 roadmap discussion."),
            Line::from("Please bring your current sprint reports."),
        ];

        let details_paragraph = Paragraph::new(event_details)
            .wrap(Wrap { trim: true });
        frame.render_widget(details_paragraph, sections[0]);

        // RSVP buttons
        let rsvp_line = Line::from(vec![
            Span::styled(
                if self.selected_action == 0 { " [✅ Accept] " } else { " ✅ Accept " },
                if self.selected_action == 0 {
                    Style::default().bg(theme.colors.palette.success).fg(Color::Black).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.colors.palette.success)
                }
            ),
            Span::styled(
                if self.selected_action == 1 { " [❌ Decline] " } else { " ❌ Decline " },
                if self.selected_action == 1 {
                    Style::default().bg(Color::Red).fg(Color::White).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Red)
                }
            ),
            Span::styled(
                if self.selected_action == 2 { " [❓ Maybe] " } else { " ❓ Maybe " },
                if self.selected_action == 2 {
                    Style::default().bg(Color::Yellow).fg(Color::Black).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Yellow)
                }
            ),
        ]);

        let rsvp_paragraph = Paragraph::new(rsvp_line)
            .alignment(Alignment::Center);
        frame.render_widget(rsvp_paragraph, sections[1]);
    }

    /// Render daily agenda view
    fn render_daily_agenda(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .title("📊 Today's Schedule")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.colors.palette.accent));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Show today's events (mock data)
        let agenda_items = vec![
            ListItem::new(Line::from(vec![
                Span::styled("🕘 ", Style::default().fg(Color::Green)),
                Span::styled("9:00 AM", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" - Daily Standup"),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("🕐 ", Style::default().fg(Color::Blue)),
                Span::styled("2:00 PM", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" - Team Meeting (NEW)"),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("🕓 ", Style::default().fg(Color::Yellow)),
                Span::styled("4:00 PM", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" - Code Review"),
            ])),
            ListItem::new(Line::from("")),
            ListItem::new(Line::from(vec![
                Span::styled("💡 ", Style::default().fg(theme.colors.palette.accent)),
                Span::styled("Free time: 10:00 AM - 1:00 PM", Style::default().fg(theme.colors.palette.text_muted)),
            ])),
        ];

        let agenda_list = List::new(agenda_items);
        frame.render_widget(agenda_list, inner);
    }

    /// Render quick schedule view for date mentions
    fn render_quick_schedule(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .title("🗓️ Schedule Context")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.colors.palette.accent));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let schedule_info = vec![
            Line::from(vec![
                Span::styled("📅 ", Style::default().fg(Color::Blue)),
                Span::styled("Referenced: Thu, Jul 30", Style::default().add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from("Available time slots:"),
            Line::from(vec![
                Span::styled("✅ ", Style::default().fg(Color::Green)),
                Span::raw("10:00 AM - 12:00 PM"),
            ]),
            Line::from(vec![
                Span::styled("✅ ", Style::default().fg(Color::Green)),
                Span::raw("3:30 PM - 5:00 PM"),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("⚠️  ", Style::default().fg(Color::Red)),
                Span::raw("Conflict: 2:00 PM - 3:00 PM"),
            ]),
        ];

        let schedule_paragraph = Paragraph::new(schedule_info);
        frame.render_widget(schedule_paragraph, inner);
    }

    /// Render conflict warning
    fn render_conflict_warning(&mut self, frame: &mut Frame, area: Rect, _theme: &Theme) {
        let block = Block::default()
            .title("⚠️ Scheduling Conflict")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let warning_text = vec![
            Line::from(vec![
                Span::styled("❌ ", Style::default().fg(Color::Red)),
                Span::styled("Time conflict detected!", Style::default().add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from("The proposed meeting time conflicts with:"),
            Line::from(vec![
                Span::styled("🕐 ", Style::default().fg(Color::Yellow)),
                Span::raw("2:00 PM - Team Meeting (existing)"),
            ]),
            Line::from(""),
            Line::from("Suggested alternatives:"),
            Line::from(vec![
                Span::styled("✅ ", Style::default().fg(Color::Green)),
                Span::raw("1:00 PM - 2:00 PM"),
            ]),
            Line::from(vec![
                Span::styled("✅ ", Style::default().fg(Color::Green)),
                Span::raw("3:00 PM - 4:00 PM"),
            ]),
        ];

        let warning_paragraph = Paragraph::new(warning_text);
        frame.render_widget(warning_paragraph, inner);
    }

    /// Handle keyboard input for calendar actions
    pub fn handle_key(&mut self, key: crossterm::event::KeyCode) -> Option<CalendarAction> {
        match self.display_mode {
            CalendarDisplayMode::InvitationDetails => {
                match key {
                    crossterm::event::KeyCode::Left => {
                        if self.selected_action > 0 {
                            self.selected_action -= 1;
                        }
                        None
                    },
                    crossterm::event::KeyCode::Right => {
                        if self.selected_action < 2 {
                            self.selected_action += 1;
                        }
                        None
                    },
                    crossterm::event::KeyCode::Enter => {
                        let response = match self.selected_action {
                            0 => RsvpResponse::Accept,
                            1 => RsvpResponse::Decline,
                            2 => RsvpResponse::Maybe,
                            _ => RsvpResponse::NoResponse,
                        };
                        
                        if let Some(event_id) = &self.current_context.invitation_event_id {
                            Some(CalendarAction::RespondToInvitation(event_id.clone(), response))
                        } else {
                            None
                        }
                    },
                    crossterm::event::KeyCode::Esc => {
                        Some(CalendarAction::HideCalendarContext)
                    },
                    _ => None,
                }
            },
            _ => {
                match key {
                    crossterm::event::KeyCode::Esc => {
                        Some(CalendarAction::HideCalendarContext)
                    },
                    _ => None,
                }
            }
        }
    }

    /// Update calendar data
    pub fn update_calendar_data(&mut self, events: Vec<Event>, calendars: Vec<Calendar>) {
        self.events = events;
        self.calendars = calendars;
        self.last_update = Utc::now();
        
        // Rebuild agenda cache
        self.rebuild_agenda_cache();
    }

    /// Check if the current display mode should be visible
    pub fn should_be_visible(&self) -> bool {
        self.display_mode != CalendarDisplayMode::Hidden
    }

    /// Get the current urgency level
    pub fn get_urgency_level(&self) -> ContextUrgency {
        self.current_context.urgency_level
    }

    // Private helper methods

    /// Check if email contains calendar invitation
    fn is_calendar_invitation(&self, content: &str, subject: &str) -> bool {
        let invitation_keywords = [
            "calendar invitation", "meeting invitation", "event invitation",
            "rsvp", "please respond", "accept", "decline", "meeting request",
            "calendar event", "when2meet", "doodle poll", "scheduling",
            "BEGIN:VCALENDAR", "VEVENT", "ics attachment"
        ];

        let text = format!("{} {}", content.to_lowercase(), subject.to_lowercase());
        invitation_keywords.iter().any(|keyword| text.contains(keyword))
    }

    /// Check for meeting-related keywords
    fn has_meeting_keywords(&self, content: &str, subject: &str) -> bool {
        let meeting_keywords = [
            "meeting", "call", "conference", "sync", "standup", "review",
            "presentation", "demo", "discussion", "catch up", "check-in",
            "interview", "1:1", "one-on-one", "team meeting", "all hands"
        ];

        let text = format!("{} {}", content.to_lowercase(), subject.to_lowercase());
        meeting_keywords.iter().any(|keyword| text.contains(keyword))
    }

    /// Extract date mentions from email content
    fn extract_date_mentions(&self, content: &str, subject: &str) -> Vec<NaiveDate> {
        let mut dates = Vec::new();
        let text = format!("{} {}", content, subject);
        
        // Simple date pattern matching (this could be enhanced with better parsing)
        let date_patterns = [
            "today", "tomorrow", "next week", "monday", "tuesday", "wednesday",
            "thursday", "friday", "saturday", "sunday", "this week"
        ];

        if date_patterns.iter().any(|pattern| text.to_lowercase().contains(pattern)) {
            // For now, just add today's date as an example
            dates.push(Local::now().date_naive());
        }

        dates
    }

    /// Rebuild the agenda cache for quick lookups
    fn rebuild_agenda_cache(&mut self) {
        self.agenda_cache.clear();
        
        for event in &self.events {
            let start_date = event.start_time.date_naive();
            self.agenda_cache.entry(start_date)
                .or_insert_with(Vec::new)
                .push(event.clone());
        }
    }
}

impl Default for EmailCalendarContext {
    fn default() -> Self {
        Self {
            email_id: None,
            has_calendar_invitation: false,
            has_meeting_references: false,
            has_date_mentions: false,
            mentioned_dates: Vec::new(),
            invitation_event_id: None,
            suggested_events: Vec::new(),
            urgency_level: ContextUrgency::None,
        }
    }
}

impl Default for ContextAwareCalendar {
    fn default() -> Self {
        Self::new()
    }
}