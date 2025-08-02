//! AI popup system with animations and interactive features

use crate::email::{EmailSummary, EmailReplyAssistance};
use crate::calendar::{CalendarInsights, EventModificationSuggestions, MeetingScheduleAnalysis, ParsedEventInfo};
use crate::theme::Theme;
use crate::ui::typography::{TypographySystem, TypographyLevel};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use std::time::{Duration, Instant};

/// AI popup animation states
#[derive(Debug, Clone, PartialEq)]
pub enum PopupAnimationState {
    /// Popup is hidden
    Hidden,
    /// Popup is animating in (sliding down)
    AnimatingIn { start_time: Instant, duration: Duration },
    /// Popup is fully visible
    Visible,
    /// Popup is animating out (sliding up)
    AnimatingOut { start_time: Instant, duration: Duration },
}

/// Reply tone options
#[derive(Debug, Clone, PartialEq)]
pub enum ReplyTone {
    Casual,
    Formal,
    Professional,
    Friendly,
    Concise,
    Detailed,
}

/// Event modification categories
#[derive(Debug, Clone, PartialEq)]
pub enum ModificationCategory {
    Time,
    Location,
    Title,
    Attendees,
    Optimization,
}

impl ModificationCategory {
    pub fn all_categories() -> Vec<ModificationCategory> {
        vec![
            ModificationCategory::Time,
            ModificationCategory::Location,
            ModificationCategory::Title,
            ModificationCategory::Attendees,
            ModificationCategory::Optimization,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ModificationCategory::Time => "Time",
            ModificationCategory::Location => "Location",
            ModificationCategory::Title => "Title",
            ModificationCategory::Attendees => "Attendees",
            ModificationCategory::Optimization => "Optimization",
        }
    }
}

/// Calendar insights categories
#[derive(Debug, Clone, PartialEq)]
pub enum InsightsCategory {
    Patterns,
    TimeManagement,
    Optimization,
    Productivity,
    FocusTime,
}

impl InsightsCategory {
    pub fn all_categories() -> Vec<InsightsCategory> {
        vec![
            InsightsCategory::Patterns,
            InsightsCategory::TimeManagement,
            InsightsCategory::Optimization,
            InsightsCategory::Productivity,
            InsightsCategory::FocusTime,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            InsightsCategory::Patterns => "Meeting Patterns",
            InsightsCategory::TimeManagement => "Time Management",
            InsightsCategory::Optimization => "Schedule Optimization",
            InsightsCategory::Productivity => "Productivity",
            InsightsCategory::FocusTime => "Focus Time",
        }
    }
}

impl ReplyTone {
    pub fn all_tones() -> Vec<ReplyTone> {
        vec![
            ReplyTone::Professional,
            ReplyTone::Formal,
            ReplyTone::Friendly,
            ReplyTone::Casual,
            ReplyTone::Concise,
            ReplyTone::Detailed,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ReplyTone::Casual => "Casual",
            ReplyTone::Formal => "Formal",
            ReplyTone::Professional => "Professional",
            ReplyTone::Friendly => "Friendly",
            ReplyTone::Concise => "Concise",
            ReplyTone::Detailed => "Detailed",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ReplyTone::Casual => "Relaxed, informal tone",
            ReplyTone::Formal => "Traditional business communication",
            ReplyTone::Professional => "Polished, business-appropriate",
            ReplyTone::Friendly => "Warm and approachable",
            ReplyTone::Concise => "Brief and to the point",
            ReplyTone::Detailed => "Comprehensive and thorough",
        }
    }
}

/// AI popup content types
#[derive(Debug, Clone)]
pub enum AIPopupContent {
    /// Email summary with reply options
    EmailSummary {
        summary: EmailSummary,
        reply_assistance: Option<EmailReplyAssistance>,
        selected_tone: ReplyTone,
        generating_reply: bool,
    },
    /// Natural language event creation
    EventCreation {
        parsed_event: ParsedEventInfo,
        confirmed: bool,
    },
    /// Event modification suggestions
    EventModification {
        suggestions: EventModificationSuggestions,
        selected_category: ModificationCategory,
    },
    /// Meeting scheduling analysis
    MeetingScheduling {
        analysis: MeetingScheduleAnalysis,
        selected_time_index: usize,
    },
    /// Calendar insights and analytics
    CalendarInsights {
        insights: CalendarInsights,
        selected_category: InsightsCategory,
    },
    /// Loading state
    Loading {
        message: String,
        progress: f32,
    },
    /// Error state
    Error {
        message: String,
        retry_available: bool,
    },
}

/// AI popup widget with animation support
#[derive(Debug)]
pub struct AIPopup {
    /// Current animation state
    animation_state: PopupAnimationState,
    /// Popup content
    content: Option<AIPopupContent>,
    /// Selected item in lists
    selected_index: usize,
    /// List state for navigating items
    list_state: ListState,
    /// Current tab (Summary/Reply/Actions)
    current_tab: PopupTab,
    /// Whether the popup accepts input
    interactive: bool,
}

/// Popup tabs
#[derive(Debug, Clone, PartialEq)]
pub enum PopupTab {
    Summary,
    Reply,
    Actions,
}

impl PopupTab {
    pub fn all_tabs() -> Vec<PopupTab> {
        vec![PopupTab::Summary, PopupTab::Reply, PopupTab::Actions]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            PopupTab::Summary => "Summary",
            PopupTab::Reply => "Reply",
            PopupTab::Actions => "Actions",
        }
    }
}

impl Default for AIPopup {
    fn default() -> Self {
        Self::new()
    }
}

impl AIPopup {
    /// Create new AI popup
    pub fn new() -> Self {
        Self {
            animation_state: PopupAnimationState::Hidden,
            content: None,
            selected_index: 0,
            list_state: ListState::default(),
            current_tab: PopupTab::Summary,
            interactive: true,
        }
    }

    /// Show email summary popup with animation
    pub fn show_email_summary(&mut self, summary: EmailSummary) {
        self.content = Some(AIPopupContent::EmailSummary {
            summary,
            reply_assistance: None,
            selected_tone: ReplyTone::Professional,
            generating_reply: false,
        });
        self.current_tab = PopupTab::Summary;
        self.selected_index = 0;
        self.list_state.select(Some(0));
        self.start_animation_in();
    }

    /// Show loading state
    pub fn show_loading(&mut self, message: String) {
        self.content = Some(AIPopupContent::Loading {
            message,
            progress: 0.0,
        });
        self.start_animation_in();
    }

    /// Update loading progress
    pub fn update_loading_progress(&mut self, progress: f32, message: Option<String>) {
        if let Some(AIPopupContent::Loading { progress: ref mut p, message: ref mut m }) = &mut self.content {
            *p = progress.clamp(0.0, 1.0);
            if let Some(new_message) = message {
                *m = new_message;
            }
        }
    }

    /// Show error state
    pub fn show_error(&mut self, message: String, retry_available: bool) {
        self.content = Some(AIPopupContent::Error {
            message,
            retry_available,
        });
        self.start_animation_in();
    }

    /// Show event creation popup with parsed event data
    pub fn show_event_creation(&mut self, parsed_event: ParsedEventInfo) {
        self.content = Some(AIPopupContent::EventCreation {
            parsed_event,
            confirmed: false,
        });
        self.current_tab = PopupTab::Summary;
        self.selected_index = 0;
        self.list_state.select(Some(0));
        self.start_animation_in();
    }

    /// Show event modification suggestions
    pub fn show_event_modification(&mut self, suggestions: EventModificationSuggestions) {
        self.content = Some(AIPopupContent::EventModification {
            suggestions,
            selected_category: ModificationCategory::Time,
        });
        self.current_tab = PopupTab::Summary;
        self.selected_index = 0;
        self.list_state.select(Some(0));
        self.start_animation_in();
    }

    /// Show meeting scheduling analysis
    pub fn show_meeting_scheduling(&mut self, analysis: MeetingScheduleAnalysis) {
        self.content = Some(AIPopupContent::MeetingScheduling {
            analysis,
            selected_time_index: 0,
        });
        self.current_tab = PopupTab::Summary;
        self.selected_index = 0;
        self.list_state.select(Some(0));
        self.start_animation_in();
    }

    /// Show calendar insights
    pub fn show_calendar_insights(&mut self, insights: CalendarInsights) {
        self.content = Some(AIPopupContent::CalendarInsights {
            insights,
            selected_category: InsightsCategory::Patterns,
        });
        self.current_tab = PopupTab::Summary;
        self.selected_index = 0;
        self.list_state.select(Some(0));
        self.start_animation_in();
    }

    /// Hide popup with animation
    pub fn hide(&mut self) {
        if matches!(self.animation_state, PopupAnimationState::Hidden) {
            return;
        }
        self.start_animation_out();
    }

    /// Check if popup is visible (including animating)
    pub fn is_visible(&self) -> bool {
        !matches!(self.animation_state, PopupAnimationState::Hidden)
    }

    /// Check if popup is fully visible and interactive
    pub fn is_interactive(&self) -> bool {
        matches!(self.animation_state, PopupAnimationState::Visible) && self.interactive
    }

    /// Start animation in
    fn start_animation_in(&mut self) {
        self.animation_state = PopupAnimationState::AnimatingIn {
            start_time: Instant::now(),
            duration: Duration::from_millis(300),
        };
    }

    /// Start animation out
    fn start_animation_out(&mut self) {
        self.animation_state = PopupAnimationState::AnimatingOut {
            start_time: Instant::now(),
            duration: Duration::from_millis(200),
        };
    }

    /// Update animation state
    pub fn update_animation(&mut self) {
        match &self.animation_state {
            PopupAnimationState::AnimatingIn { start_time, duration } => {
                if start_time.elapsed() >= *duration {
                    self.animation_state = PopupAnimationState::Visible;
                }
            }
            PopupAnimationState::AnimatingOut { start_time, duration } => {
                if start_time.elapsed() >= *duration {
                    self.animation_state = PopupAnimationState::Hidden;
                    self.content = None;
                }
            }
            _ => {}
        }
    }

    /// Get animation progress (0.0 to 1.0)
    fn get_animation_progress(&self) -> f32 {
        match &self.animation_state {
            PopupAnimationState::AnimatingIn { start_time, duration } => {
                let elapsed = start_time.elapsed().as_millis() as f32;
                let total = duration.as_millis() as f32;
                (elapsed / total).clamp(0.0, 1.0)
            }
            PopupAnimationState::AnimatingOut { start_time, duration } => {
                let elapsed = start_time.elapsed().as_millis() as f32;
                let total = duration.as_millis() as f32;
                1.0 - (elapsed / total).clamp(0.0, 1.0)
            }
            PopupAnimationState::Visible => 1.0,
            PopupAnimationState::Hidden => 0.0,
        }
    }

    /// Switch to next tab
    pub fn next_tab(&mut self) {
        if !self.is_interactive() {
            return;
        }
        
        let tabs = PopupTab::all_tabs();
        let current_index = tabs.iter().position(|t| t == &self.current_tab).unwrap_or(0);
        let next_index = (current_index + 1) % tabs.len();
        self.current_tab = tabs[next_index].clone();
        self.selected_index = 0;
        self.list_state.select(Some(0));
    }

    /// Switch to previous tab
    pub fn previous_tab(&mut self) {
        if !self.is_interactive() {
            return;
        }

        let tabs = PopupTab::all_tabs();
        let current_index = tabs.iter().position(|t| t == &self.current_tab).unwrap_or(0);
        let prev_index = if current_index == 0 { tabs.len() - 1 } else { current_index - 1 };
        self.current_tab = tabs[prev_index].clone();
        self.selected_index = 0;
        self.list_state.select(Some(0));
    }

    /// Move selection up
    pub fn move_up(&mut self) {
        if !self.is_interactive() {
            return;
        }

        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.list_state.select(Some(self.selected_index));
        }
    }

    /// Move selection down
    pub fn move_down(&mut self) {
        if !self.is_interactive() {
            return;
        }

        let max_items = self.get_max_items();
        if self.selected_index < max_items.saturating_sub(1) {
            self.selected_index += 1;
            self.list_state.select(Some(self.selected_index));
        }
    }

    /// Get maximum items for current content
    fn get_max_items(&self) -> usize {
        match (&self.content, &self.current_tab) {
            (Some(AIPopupContent::EmailSummary { .. }), PopupTab::Reply) => {
                ReplyTone::all_tones().len()
            }
            (Some(AIPopupContent::EmailSummary { summary, .. }), PopupTab::Actions) => {
                summary.action_items.len()
            }
            (Some(AIPopupContent::EventModification { selected_category, suggestions }), _) => {
                match selected_category {
                    ModificationCategory::Time => suggestions.time_suggestions.len(),
                    ModificationCategory::Location => suggestions.location_suggestions.len(),
                    ModificationCategory::Title => suggestions.title_suggestions.len(),
                    ModificationCategory::Attendees => suggestions.attendee_suggestions.len(),
                    ModificationCategory::Optimization => suggestions.optimization_tips.len(),
                }
            }
            (Some(AIPopupContent::MeetingScheduling { analysis, .. }), _) => {
                analysis.optimal_times.len()
            }
            (Some(AIPopupContent::CalendarInsights { insights, selected_category }), _) => {
                match selected_category {
                    InsightsCategory::Patterns => insights.meeting_patterns.len(),
                    InsightsCategory::TimeManagement => insights.time_management_tips.len(),
                    InsightsCategory::Optimization => insights.optimization_suggestions.len(),
                    InsightsCategory::Productivity => insights.productivity_insights.len(),
                    InsightsCategory::FocusTime => insights.focus_time_suggestions.len(),
                }
            }
            _ => 1,
        }
    }

    /// Select current item
    pub fn select_current(&mut self) -> Option<PopupAction> {
        if !self.is_interactive() {
            return None;
        }

        match (&mut self.content, &self.current_tab) {
            (Some(AIPopupContent::EmailSummary { selected_tone, generating_reply, .. }), PopupTab::Reply) => {
                if !*generating_reply {
                    let tones = ReplyTone::all_tones();
                    if let Some(tone) = tones.get(self.selected_index) {
                        *selected_tone = tone.clone();
                        *generating_reply = true;
                        return Some(PopupAction::GenerateReply(tone.clone()));
                    }
                }
            }
            (Some(AIPopupContent::EventCreation { confirmed, .. }), _) => {
                if !*confirmed {
                    *confirmed = true;
                    return Some(PopupAction::ConfirmEventCreation);
                }
            }
            (Some(AIPopupContent::EventModification { selected_category, suggestions }), _) => {
                let suggestions_vec = match selected_category {
                    ModificationCategory::Time => &suggestions.time_suggestions,
                    ModificationCategory::Location => &suggestions.location_suggestions,
                    ModificationCategory::Title => &suggestions.title_suggestions,
                    ModificationCategory::Attendees => &suggestions.attendee_suggestions,
                    ModificationCategory::Optimization => &suggestions.optimization_tips,
                };
                
                if let Some(suggestion) = suggestions_vec.get(self.selected_index) {
                    return Some(PopupAction::ApplyEventModification(selected_category.clone(), suggestion.clone()));
                }
            }
            (Some(AIPopupContent::MeetingScheduling { selected_time_index, .. }), _) => {
                *selected_time_index = self.selected_index;
                return Some(PopupAction::SelectMeetingTime(self.selected_index));
            }
            (Some(AIPopupContent::CalendarInsights { .. }), _) => {
                return Some(PopupAction::CreateEventFromSuggestion);
            }
            (Some(AIPopupContent::Error { retry_available, .. }), _) => {
                if *retry_available {
                    return Some(PopupAction::Retry);
                } else {
                    return Some(PopupAction::Close);
                }
            }
            (Some(AIPopupContent::Loading { .. }), _) => {
                // For loading state, Enter should close the popup
                return Some(PopupAction::Close);
            }
            _ => {}
        }

        None
    }

    /// Set reply assistance
    pub fn set_reply_assistance(&mut self, reply_assistance: EmailReplyAssistance) {
        if let Some(AIPopupContent::EmailSummary { reply_assistance: ref mut ra, generating_reply, .. }) = &mut self.content {
            *ra = Some(reply_assistance);
            *generating_reply = false;
        }
    }

    /// Calculate popup rect with animation offset
    fn calculate_popup_rect(&self, area: Rect) -> Rect {
        let popup_width = (area.width as f32 * 0.8).min(100.0) as u16;
        let popup_height = (area.height as f32 * 0.7).min(30.0) as u16;
        
        let x = (area.width.saturating_sub(popup_width)) / 2;
        let base_y = (area.height.saturating_sub(popup_height)) / 2;
        
        // Apply animation offset
        let progress = self.get_animation_progress();
        let animation_offset = ((1.0 - progress) * 10.0) as u16;
        let y = base_y.saturating_sub(animation_offset);
        
        Rect::new(x, y, popup_width, popup_height)
    }

    /// Render the popup
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme, typography: &TypographySystem) {
        if matches!(self.animation_state, PopupAnimationState::Hidden) {
            return;
        }

        let popup_rect = self.calculate_popup_rect(area);
        
        // Calculate opacity based on animation
        let progress = self.get_animation_progress();
        let opacity = (progress * 255.0) as u8;
        
        // Clear background with semi-transparent overlay
        frame.render_widget(Clear, area);
        
        // Render popup content
        self.render_popup_content(frame, popup_rect, theme, typography, opacity);
    }

    /// Render popup content
    fn render_popup_content(&self, frame: &mut Frame, area: Rect, theme: &Theme, typography: &TypographySystem, _opacity: u8) {
        // Main popup block
        let popup_block = Block::default()
            .borders(Borders::ALL)
            .title(" AI Assistant ")
            .title_style(typography.get_typography_style(TypographyLevel::Heading2, theme))
            .border_style(theme.get_component_style("border", true))
            .style(Style::default().bg(theme.colors.palette.overlay));

        let inner_area = popup_block.inner(area);
        frame.render_widget(popup_block, area);

        match &self.content {
            Some(AIPopupContent::EmailSummary { summary, reply_assistance, selected_tone, generating_reply }) => {
                self.render_email_summary(frame, inner_area, theme, typography, summary, reply_assistance, selected_tone, *generating_reply);
            }
            Some(AIPopupContent::EventCreation { parsed_event, confirmed }) => {
                self.render_event_creation(frame, inner_area, theme, typography, parsed_event, *confirmed);
            }
            Some(AIPopupContent::EventModification { suggestions, selected_category }) => {
                self.render_event_modification(frame, inner_area, theme, typography, suggestions, selected_category);
            }
            Some(AIPopupContent::MeetingScheduling { analysis, selected_time_index }) => {
                self.render_meeting_scheduling(frame, inner_area, theme, typography, analysis, *selected_time_index);
            }
            Some(AIPopupContent::CalendarInsights { insights, selected_category }) => {
                self.render_calendar_insights(frame, inner_area, theme, typography, insights, selected_category);
            }
            Some(AIPopupContent::Loading { message, progress }) => {
                self.render_loading(frame, inner_area, theme, typography, message, *progress);
            }
            Some(AIPopupContent::Error { message, retry_available }) => {
                self.render_error(frame, inner_area, theme, typography, message, *retry_available);
            }
            _ => {}
        }
    }

    /// Render email summary content
    fn render_email_summary(
        &self,
        frame: &mut Frame,
        area: Rect,
        theme: &Theme,
        typography: &TypographySystem,
        summary: &EmailSummary,
        reply_assistance: &Option<EmailReplyAssistance>,
        selected_tone: &ReplyTone,
        generating_reply: bool,
    ) {
        // Split area for tabs and content
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Tabs
                Constraint::Min(0),    // Content
                Constraint::Length(3), // Footer
            ])
            .split(area);

        // Render tabs
        self.render_tabs(frame, chunks[0], theme, typography);

        // Render content based on current tab
        match self.current_tab {
            PopupTab::Summary => self.render_summary_tab(frame, chunks[1], theme, typography, summary),
            PopupTab::Reply => self.render_reply_tab(frame, chunks[1], theme, typography, reply_assistance, selected_tone, generating_reply),
            PopupTab::Actions => self.render_actions_tab(frame, chunks[1], theme, typography, summary),
        }

        // Render footer
        self.render_footer(frame, chunks[2], theme, typography);
    }

    /// Render tabs
    fn render_tabs(&self, frame: &mut Frame, area: Rect, theme: &Theme, _typography: &TypographySystem) {
        let tabs = PopupTab::all_tabs();
        let tab_width = area.width / tabs.len() as u16;
        
        for (i, tab) in tabs.iter().enumerate() {
            let x = area.x + (i as u16 * tab_width);
            let tab_area = Rect::new(x, area.y, tab_width, area.height);
            
            let is_selected = tab == &self.current_tab;
            let style = if is_selected {
                Style::default()
                    .bg(theme.colors.palette.selection)
                    .fg(theme.colors.palette.background)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.colors.palette.text_muted)
            };
            
            let tab_text = Paragraph::new(tab.display_name())
                .style(style)
                .alignment(Alignment::Center);
            
            frame.render_widget(tab_text, tab_area);
        }
    }

    /// Render summary tab
    fn render_summary_tab(&self, frame: &mut Frame, area: Rect, theme: &Theme, typography: &TypographySystem, summary: &EmailSummary) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Category
                Constraint::Min(3),    // Summary
                Constraint::Length(5), // Key points
            ])
            .split(area);

        // Category
        let category_text = Line::from(vec![
            typography.create_span("Category: ".to_string(), TypographyLevel::Caption, theme),
            typography.create_emphasis(&summary.category.to_string(), theme),
        ]);
        let category = Paragraph::new(category_text);
        frame.render_widget(category, chunks[0]);

        // Summary
        let summary_text = typography.create_text(&summary.summary, TypographyLevel::Body, theme);
        let summary_para = Paragraph::new(summary_text)
            .wrap(Wrap { trim: true })
            .block(Block::default().title("Summary").borders(Borders::ALL));
        frame.render_widget(summary_para, chunks[1]);

        // Key points
        let key_points: Vec<ListItem> = summary.key_points
            .iter()
            .map(|point| ListItem::new(format!("• {}", point)))
            .collect();
        
        let key_points_list = List::new(key_points)
            .block(Block::default().title("Key Points").borders(Borders::ALL))
            .style(typography.get_typography_style(TypographyLevel::Body, theme));
        
        frame.render_widget(key_points_list, chunks[2]);
    }

    /// Render reply tab
    fn render_reply_tab(
        &self,
        frame: &mut Frame,
        area: Rect,
        theme: &Theme,
        typography: &TypographySystem,
        reply_assistance: &Option<EmailReplyAssistance>,
        selected_tone: &ReplyTone,
        generating_reply: bool,
    ) {
        if generating_reply {
            let loading_text = typography.create_text(
                &format!("Generating {} reply...", selected_tone.display_name().to_lowercase()),
                TypographyLevel::Body,
                theme,
            );
            let loading_para = Paragraph::new(loading_text)
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });
            frame.render_widget(loading_para, area);
            return;
        }

        if let Some(reply) = reply_assistance {
            // Show generated replies
            let replies: Vec<ListItem> = reply.reply_suggestions
                .iter()
                .enumerate()
                .map(|(i, suggestion)| {
                    let style = if i == self.selected_index {
                        Style::default().bg(theme.colors.palette.selection).fg(theme.colors.palette.background)
                    } else {
                        Style::default()
                    };
                    ListItem::new(suggestion.clone()).style(style)
                })
                .collect();

            let replies_list = List::new(replies)
                .block(Block::default().title("Reply Suggestions").borders(Borders::ALL))
                .highlight_style(Style::default().add_modifier(Modifier::BOLD));

            frame.render_widget(replies_list, area);
        } else {
            // Show tone selection
            let tones: Vec<ListItem> = ReplyTone::all_tones()
                .iter()
                .enumerate()
                .map(|(i, tone)| {
                    let style = if i == self.selected_index {
                        Style::default().bg(theme.colors.palette.selection).fg(theme.colors.palette.background)
                    } else {
                        Style::default()
                    };
                    ListItem::new(format!("{} - {}", tone.display_name(), tone.description())).style(style)
                })
                .collect();

            let tones_list = List::new(tones)
                .block(Block::default().title("Select Reply Tone").borders(Borders::ALL))
                .highlight_style(Style::default().add_modifier(Modifier::BOLD));

            frame.render_widget(tones_list, area);
        }
    }

    /// Render actions tab
    fn render_actions_tab(&self, frame: &mut Frame, area: Rect, theme: &Theme, typography: &TypographySystem, summary: &EmailSummary) {
        if summary.action_items.is_empty() {
            let no_actions = typography.create_text("No action items found in this email.", TypographyLevel::Body, theme);
            let no_actions_para = Paragraph::new(no_actions)
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });
            frame.render_widget(no_actions_para, area);
            return;
        }

        let actions: Vec<ListItem> = summary.action_items
            .iter()
            .enumerate()
            .map(|(i, action)| {
                let style = if i == self.selected_index {
                    Style::default().bg(theme.colors.palette.selection).fg(theme.colors.palette.background)
                } else {
                    Style::default()
                };
                ListItem::new(format!("• {}", action)).style(style)
            })
            .collect();

        let actions_list = List::new(actions)
            .block(Block::default().title("Action Items").borders(Borders::ALL))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        frame.render_widget(actions_list, area);
    }

    /// Render loading state
    fn render_loading(&self, frame: &mut Frame, area: Rect, theme: &Theme, typography: &TypographySystem, message: &str, progress: f32) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Message
                Constraint::Length(3), // Progress bar
                Constraint::Min(0),    // Spacer
            ])
            .split(area);

        // Message
        let message_text = typography.create_text(message, TypographyLevel::Body, theme);
        let message_para = Paragraph::new(message_text)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        frame.render_widget(message_para, chunks[0]);

        // Progress bar
        let progress_width = (chunks[1].width as f32 * progress) as u16;
        let progress_area = Rect::new(chunks[1].x, chunks[1].y + 1, progress_width, 1);
        
        let progress_block = Block::default()
            .style(Style::default().bg(theme.colors.palette.selection));
        frame.render_widget(progress_block, progress_area);
    }

    /// Render error state
    fn render_error(&self, frame: &mut Frame, area: Rect, theme: &Theme, typography: &TypographySystem, message: &str, retry_available: bool) {
        let error_text = typography.create_text(message, TypographyLevel::Body, theme);
        let mut error_para = Paragraph::new(error_text)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(Color::Red));

        if retry_available {
            let retry_text = Line::from(vec![
                typography.create_span("\n\nPress ".to_string(), TypographyLevel::Caption, theme),
                typography.create_emphasis("Enter", theme),
                typography.create_span(" to retry".to_string(), TypographyLevel::Caption, theme),
            ]);
            error_para = error_para.block(Block::default().title(retry_text));
        }

        frame.render_widget(error_para, area);
    }

    /// Render footer
    fn render_footer(&self, frame: &mut Frame, area: Rect, theme: &Theme, typography: &TypographySystem) {
        let footer_text = Line::from(vec![
            typography.create_emphasis("Tab", theme),
            typography.create_span("/".to_string(), TypographyLevel::Caption, theme),
            typography.create_emphasis("Shift+Tab", theme),
            typography.create_span(": Switch tabs  ".to_string(), TypographyLevel::Caption, theme),
            typography.create_emphasis("↑/↓", theme),
            typography.create_span(": Navigate  ".to_string(), TypographyLevel::Caption, theme),
            typography.create_emphasis("Enter", theme),
            typography.create_span(": Select  ".to_string(), TypographyLevel::Caption, theme),
            typography.create_emphasis("Esc", theme),
            typography.create_span(": Close".to_string(), TypographyLevel::Caption, theme),
        ]);

        let footer_para = Paragraph::new(footer_text)
            .alignment(Alignment::Center)
            .style(typography.get_typography_style(TypographyLevel::Caption, theme));

        frame.render_widget(footer_para, area);
    }

    /// Render event creation content
    fn render_event_creation(
        &self, 
        frame: &mut Frame, 
        area: Rect, 
        theme: &Theme, 
        typography: &TypographySystem,
        parsed_event: &ParsedEventInfo,
        confirmed: bool,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(3)])
            .split(area);

        // Title
        let title = format!("🗓️ Create Event (Confidence: {:.0}%)", parsed_event.confidence * 100.0);
        let title_para = Paragraph::new(title)
            .alignment(Alignment::Center)
            .style(typography.get_typography_style(TypographyLevel::Heading3, theme));
        frame.render_widget(title_para, chunks[0]);

        // Event details
        let mut details = vec![
            Line::from(vec![
                Span::styled("Title: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&parsed_event.title),
            ]),
            Line::from(vec![
                Span::styled("Start: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(parsed_event.start_time.format("%Y-%m-%d %H:%M").to_string()),
            ]),
            Line::from(vec![
                Span::styled("End: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(parsed_event.end_time.format("%Y-%m-%d %H:%M").to_string()),
            ]),
        ];

        if let Some(location) = &parsed_event.location {
            details.push(Line::from(vec![
                Span::styled("Location: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(location.clone()),
            ]));
        }

        if !parsed_event.attendees.is_empty() {
            details.push(Line::from(vec![
                Span::styled("Attendees: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(parsed_event.attendees.join(", ")),
            ]));
        }

        if let Some(description) = &parsed_event.description {
            details.push(Line::from(""));
            details.push(Line::from(vec![
                Span::styled("Description: ", Style::default().add_modifier(Modifier::BOLD)),
            ]));
            details.push(Line::from(Span::raw(description.clone())));
        }

        let details_para = Paragraph::new(details)
            .wrap(Wrap { trim: true })
            .style(typography.get_typography_style(TypographyLevel::Body, theme));
        frame.render_widget(details_para, chunks[1]);

        // Footer
        let footer_text = if confirmed {
            "✅ Event confirmed! Press Escape to close"
        } else {
            "Press Enter to confirm event creation, Tab to modify, Escape to cancel"
        };
        
        let footer_para = Paragraph::new(footer_text)
            .alignment(Alignment::Center)
            .style(typography.get_typography_style(TypographyLevel::Caption, theme));
        frame.render_widget(footer_para, chunks[2]);
    }

    /// Render event modification content
    fn render_event_modification(
        &self,
        frame: &mut Frame,
        area: Rect,
        theme: &Theme,
        typography: &TypographySystem,
        suggestions: &EventModificationSuggestions,
        selected_category: &ModificationCategory,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Length(3), Constraint::Min(0), Constraint::Length(3)])
            .split(area);

        // Title
        let title = "🛠️ Event Modification Suggestions";
        let title_para = Paragraph::new(title)
            .alignment(Alignment::Center)
            .style(typography.get_typography_style(TypographyLevel::Heading3, theme));
        frame.render_widget(title_para, chunks[0]);

        // Category tabs
        let categories = ModificationCategory::all_categories();
        let category_names: Vec<String> = categories.iter()
            .map(|c| if c == selected_category {
                format!("[{}]", c.display_name())
            } else {
                c.display_name().to_string()
            })
            .collect();
        
        let tabs_para = Paragraph::new(category_names.join("  "))
            .alignment(Alignment::Center)
            .style(typography.get_typography_style(TypographyLevel::Body, theme));
        frame.render_widget(tabs_para, chunks[1]);

        // Suggestions list
        let current_suggestions = match selected_category {
            ModificationCategory::Time => &suggestions.time_suggestions,
            ModificationCategory::Location => &suggestions.location_suggestions,
            ModificationCategory::Title => &suggestions.title_suggestions,
            ModificationCategory::Attendees => &suggestions.attendee_suggestions,
            ModificationCategory::Optimization => &suggestions.optimization_tips,
        };

        let items: Vec<ListItem> = current_suggestions.iter()
            .enumerate()
            .map(|(i, suggestion)| {
                let style = if i == self.selected_index {
                    Style::default().bg(theme.colors.palette.selection).fg(theme.colors.palette.background)
                } else {
                    Style::default()
                };
                ListItem::new(suggestion.clone()).style(style)
            })
            .collect();

        let list = List::new(items)
            .style(typography.get_typography_style(TypographyLevel::Body, theme));
        frame.render_widget(list, chunks[2]);

        // Footer
        let footer_text = "Use ↑↓ to navigate, Enter to apply, Tab to switch categories, Escape to close";
        let footer_para = Paragraph::new(footer_text)
            .alignment(Alignment::Center)
            .style(typography.get_typography_style(TypographyLevel::Caption, theme));
        frame.render_widget(footer_para, chunks[3]);
    }

    /// Render meeting scheduling content
    fn render_meeting_scheduling(
        &self,
        frame: &mut Frame,
        area: Rect,
        theme: &Theme,
        typography: &TypographySystem,
        analysis: &MeetingScheduleAnalysis,
        _selected_time_index: usize,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(3)])
            .split(area);

        // Title
        let title = format!("📅 Meeting Scheduling Analysis (Duration: {}min)", analysis.suggested_duration);
        let title_para = Paragraph::new(title)
            .alignment(Alignment::Center)
            .style(typography.get_typography_style(TypographyLevel::Heading3, theme));
        frame.render_widget(title_para, chunks[0]);

        // Optimal times list
        let items: Vec<ListItem> = analysis.optimal_times.iter()
            .enumerate()
            .map(|(i, time)| {
                let mut text = time.format("%Y-%m-%d %H:%M").to_string();
                
                // Add conflict information
                let conflicts: Vec<_> = analysis.conflicts.iter()
                    .filter(|c| c.conflict_time == *time)
                    .collect();
                
                if !conflicts.is_empty() {
                    text.push_str(&format!(" (⚠️ {} conflicts)", conflicts.len()));
                }

                let style = if i == self.selected_index {
                    Style::default().bg(theme.colors.palette.selection).fg(theme.colors.palette.background)
                } else if !conflicts.is_empty() {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                };
                
                ListItem::new(text).style(style)
            })
            .collect();

        let list = List::new(items)
            .style(typography.get_typography_style(TypographyLevel::Body, theme));
        frame.render_widget(list, chunks[1]);

        // Footer
        let footer_text = "Use ↑↓ to navigate optimal times, Enter to select, Escape to close";
        let footer_para = Paragraph::new(footer_text)
            .alignment(Alignment::Center)
            .style(typography.get_typography_style(TypographyLevel::Caption, theme));
        frame.render_widget(footer_para, chunks[2]);
    }

    /// Render calendar insights content
    fn render_calendar_insights(
        &self,
        frame: &mut Frame,
        area: Rect,
        theme: &Theme,
        typography: &TypographySystem,
        insights: &CalendarInsights,
        selected_category: &InsightsCategory,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Length(3), Constraint::Min(0), Constraint::Length(3)])
            .split(area);

        // Title
        let title = "📊 Calendar Insights & Analytics";
        let title_para = Paragraph::new(title)
            .alignment(Alignment::Center)
            .style(typography.get_typography_style(TypographyLevel::Heading3, theme));
        frame.render_widget(title_para, chunks[0]);

        // Category tabs
        let categories = InsightsCategory::all_categories();
        let category_names: Vec<String> = categories.iter()
            .map(|c| if c == selected_category {
                format!("[{}]", c.display_name())
            } else {
                c.display_name().to_string()
            })
            .collect();
        
        let tabs_para = Paragraph::new(category_names.join("  "))
            .alignment(Alignment::Center)
            .style(typography.get_typography_style(TypographyLevel::Body, theme));
        frame.render_widget(tabs_para, chunks[1]);

        // Insights list
        let current_insights = match selected_category {
            InsightsCategory::Patterns => &insights.meeting_patterns,
            InsightsCategory::TimeManagement => &insights.time_management_tips,
            InsightsCategory::Optimization => &insights.optimization_suggestions,
            InsightsCategory::Productivity => &insights.productivity_insights,
            InsightsCategory::FocusTime => &insights.focus_time_suggestions,
        };

        let items: Vec<ListItem> = current_insights.iter()
            .enumerate()
            .map(|(i, insight)| {
                let style = if i == self.selected_index {
                    Style::default().bg(theme.colors.palette.selection).fg(theme.colors.palette.background)
                } else {
                    Style::default()
                };
                ListItem::new(format!("• {}", insight)).style(style)
            })
            .collect();

        let list = List::new(items)
            .style(typography.get_typography_style(TypographyLevel::Body, theme));
        frame.render_widget(list, chunks[2]);

        // Footer
        let footer_text = "Use ↑↓ to navigate, Tab to switch categories, Enter to create event, Escape to close";
        let footer_para = Paragraph::new(footer_text)
            .alignment(Alignment::Center)
            .style(typography.get_typography_style(TypographyLevel::Caption, theme));
        frame.render_widget(footer_para, chunks[3]);
    }
}

/// Actions that can be triggered from the popup
#[derive(Debug, Clone)]
pub enum PopupAction {
    /// Generate reply with specific tone
    GenerateReply(ReplyTone),
    /// Close popup
    Close,
    /// Retry operation
    Retry,
    /// Select action item
    SelectAction(String),
    /// Confirm event creation
    ConfirmEventCreation,
    /// Apply event modification
    ApplyEventModification(ModificationCategory, String),
    /// Select meeting time
    SelectMeetingTime(usize),
    /// Create calendar event from suggestion
    CreateEventFromSuggestion,
}