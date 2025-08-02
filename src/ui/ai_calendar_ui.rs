//! AI calendar assistant UI components

use crate::calendar::{
    AICalendarAssistant, CalendarInsights, EventModificationSuggestions, 
    MeetingScheduleAnalysis, NaturalLanguageEventRequest, ParsedEventInfo,
};
use crate::theme::Theme;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use std::sync::Arc;

/// AI calendar assistant UI state
#[derive(Debug, Clone)]
pub struct AICalendarUIState {
    /// Current mode of AI calendar assistance
    pub mode: AICalendarMode,
    /// Whether AI assistance is enabled
    pub enabled: bool,
    /// Loading state for AI operations
    pub loading: bool,
    /// Current input text for natural language processing
    pub input_text: String,
    /// Parsed event information
    pub parsed_event: Option<ParsedEventInfo>,
    /// Event modification suggestions
    pub modification_suggestions: Option<EventModificationSuggestions>,
    /// Meeting schedule analysis
    pub schedule_analysis: Option<MeetingScheduleAnalysis>,
    /// Calendar insights
    pub calendar_insights: Option<CalendarInsights>,
    /// Selected suggestion index
    pub selected_suggestion: usize,
    /// Error message if any
    pub error_message: Option<String>,
    /// List state for navigating suggestions
    pub list_state: ListState,
    /// Input mode for text entry
    pub input_mode: bool,
}

/// AI calendar assistant modes
#[derive(Debug, Clone, PartialEq)]
pub enum AICalendarMode {
    /// Hidden/inactive
    Hidden,
    /// Natural language event creation
    CreateEvent,
    /// Event modification suggestions
    ModifyEvent,
    /// Meeting scheduling analysis
    ScheduleMeeting,
    /// Calendar insights and analytics
    Insights,
    /// Quick scheduling commands
    QuickSchedule,
}

impl Default for AICalendarUIState {
    fn default() -> Self {
        Self {
            mode: AICalendarMode::Hidden,
            enabled: false,
            loading: false,
            input_text: String::new(),
            parsed_event: None,
            modification_suggestions: None,
            schedule_analysis: None,
            calendar_insights: None,
            selected_suggestion: 0,
            error_message: None,
            list_state: ListState::default(),
            input_mode: false,
        }
    }
}

impl AICalendarUIState {
    /// Create new AI calendar UI state
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable AI calendar assistance
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable AI calendar assistance
    pub fn disable(&mut self) {
        self.enabled = false;
        self.mode = AICalendarMode::Hidden;
    }

    /// Set create event mode
    pub fn set_create_event_mode(&mut self) {
        self.mode = AICalendarMode::CreateEvent;
        self.input_text.clear();
        self.parsed_event = None;
        self.error_message = None;
        self.input_mode = true;
    }

    /// Set modify event mode
    pub fn set_modify_event_mode(&mut self, suggestions: EventModificationSuggestions) {
        self.mode = AICalendarMode::ModifyEvent;
        self.modification_suggestions = Some(suggestions);
        self.selected_suggestion = 0;
        self.list_state.select(Some(0));
        self.error_message = None;
    }

    /// Set schedule meeting mode
    pub fn set_schedule_meeting_mode(&mut self, analysis: MeetingScheduleAnalysis) {
        self.mode = AICalendarMode::ScheduleMeeting;
        self.schedule_analysis = Some(analysis);
        self.error_message = None;
    }

    /// Set insights mode
    pub fn set_insights_mode(&mut self, insights: CalendarInsights) {
        self.mode = AICalendarMode::Insights;
        self.calendar_insights = Some(insights);
        self.error_message = None;
    }

    /// Set loading state
    pub fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
        if loading {
            self.error_message = None;
        }
    }

    /// Set error message
    pub fn set_error(&mut self, message: String) {
        self.error_message = Some(message);
        self.loading = false;
    }

    /// Hide the AI calendar assistant
    pub fn hide(&mut self) {
        self.mode = AICalendarMode::Hidden;
        self.input_text.clear();
        self.parsed_event = None;
        self.modification_suggestions = None;
        self.schedule_analysis = None;
        self.calendar_insights = None;
        self.error_message = None;
        self.loading = false;
        self.input_mode = false;
    }

    /// Add character to input text
    pub fn add_char(&mut self, c: char) {
        if self.input_mode {
            self.input_text.push(c);
        }
    }

    /// Remove character from input text
    pub fn remove_char(&mut self) {
        if self.input_mode {
            self.input_text.pop();
        }
    }

    /// Toggle input mode
    pub fn toggle_input_mode(&mut self) {
        self.input_mode = !self.input_mode;
    }

    /// Move selection up
    pub fn move_up(&mut self) {
        if self.selected_suggestion > 0 {
            self.selected_suggestion -= 1;
            self.list_state.select(Some(self.selected_suggestion));
        }
    }

    /// Move selection down
    pub fn move_down(&mut self) {
        let max_items = match &self.mode {
            AICalendarMode::ModifyEvent => {
                self.modification_suggestions
                    .as_ref()
                    .map(|s| s.time_suggestions.len() + s.location_suggestions.len() + 
                           s.title_suggestions.len() + s.attendee_suggestions.len() + 
                           s.optimization_tips.len())
                    .unwrap_or(0)
            },
            AICalendarMode::ScheduleMeeting => {
                self.schedule_analysis
                    .as_ref()
                    .map(|a| a.optimal_times.len())
                    .unwrap_or(0)
            },
            _ => 0,
        };

        if self.selected_suggestion < max_items.saturating_sub(1) {
            self.selected_suggestion += 1;
            self.list_state.select(Some(self.selected_suggestion));
        }
    }

    /// Get currently selected suggestion text
    pub fn get_selected_suggestion(&self) -> Option<String> {
        match &self.mode {
            AICalendarMode::ModifyEvent => {
                if let Some(suggestions) = &self.modification_suggestions {
                    let all_suggestions: Vec<_> = suggestions.time_suggestions
                        .iter()
                        .chain(suggestions.location_suggestions.iter())
                        .chain(suggestions.title_suggestions.iter())
                        .chain(suggestions.attendee_suggestions.iter())
                        .chain(suggestions.optimization_tips.iter())
                        .collect();
                    all_suggestions.get(self.selected_suggestion).map(|s| s.to_string())
                } else {
                    None
                }
            },
            AICalendarMode::ScheduleMeeting => {
                self.schedule_analysis
                    .as_ref()
                    .and_then(|a| a.optimal_times.get(self.selected_suggestion))
                    .map(|time| time.format("%Y-%m-%d %H:%M UTC").to_string())
            },
            _ => None,
        }
    }

    /// Get natural language event request from current input
    pub fn get_event_request(&self) -> Option<NaturalLanguageEventRequest> {
        if self.input_text.trim().is_empty() {
            return None;
        }

        Some(NaturalLanguageEventRequest {
            description: self.input_text.clone(),
            context: None,
            calendar_id: None,
        })
    }
}

/// AI calendar assistant UI component
pub struct AICalendarUI {
    #[allow(dead_code)]
    assistant: Arc<AICalendarAssistant>,
}

impl AICalendarUI {
    /// Create new AI calendar UI
    pub fn new(assistant: Arc<AICalendarAssistant>) -> Self {
        Self { assistant }
    }

    /// Render AI calendar assistant UI
    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &mut AICalendarUIState,
        theme: &Theme,
    ) {
        if !state.enabled || state.mode == AICalendarMode::Hidden {
            return;
        }

        // Clear the area
        frame.render_widget(Clear, area);

        let block = Block::default()
            .title(" AI Calendar Assistant ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.ai_assistant_border()));

        match &state.mode {
            AICalendarMode::CreateEvent => {
                self.render_create_event(frame, area, state, theme, block);
            },
            AICalendarMode::ModifyEvent => {
                self.render_modify_event(frame, area, state, theme, block);
            },
            AICalendarMode::ScheduleMeeting => {
                self.render_schedule_meeting(frame, area, state, theme, block);
            },
            AICalendarMode::Insights => {
                self.render_calendar_insights(frame, area, state, theme, block);
            },
            AICalendarMode::QuickSchedule => {
                self.render_quick_schedule(frame, area, state, theme, block);
            },
            AICalendarMode::Hidden => {
                // Already handled above
            },
        }
    }

    /// Render event creation interface
    fn render_create_event(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &AICalendarUIState,
        theme: &Theme,
        block: Block,
    ) {
        if state.loading {
            self.render_loading(frame, area, theme, block, "Creating event from natural language...");
            return;
        }

        if let Some(error) = &state.error_message {
            self.render_error(frame, area, theme, block, error);
            return;
        }

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Length(3), // Input area
                Constraint::Min(5),    // Event preview or results
                Constraint::Length(2), // Instructions
            ])
            .split(inner);

        // Title
        let title = Paragraph::new("Create Event with Natural Language")
            .style(Style::default().fg(theme.ai_assistant_title()))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        frame.render_widget(title, chunks[0]);

        // Input area
        let input_style = if state.input_mode {
            Style::default().fg(theme.ai_assistant_selected()).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.ai_assistant_text())
        };

        let input_widget = Paragraph::new(state.input_text.as_str())
            .style(input_style)
            .block(Block::default().title("Describe your event").borders(Borders::ALL))
            .wrap(Wrap { trim: true });
        frame.render_widget(input_widget, chunks[1]);

        // Event preview or placeholder
        if let Some(parsed) = &state.parsed_event {
            self.render_parsed_event_preview(frame, chunks[2], parsed, theme);
        } else {
            let placeholder = Paragraph::new("Type your event description above. Examples:\n• \"Team meeting tomorrow at 2 PM for 1 hour\"\n• \"Lunch with Sarah next Friday at noon\"\n• \"Doctor appointment on Monday at 3:30 PM\"")
                .style(Style::default().fg(theme.ai_assistant_context()))
                .wrap(Wrap { trim: true });
            frame.render_widget(placeholder, chunks[2]);
        }

        // Instructions
        let instructions = if state.input_mode {
            "Type to enter text • Enter: Process • Esc: Cancel input"
        } else {
            "i: Edit text • Enter: Create event • Esc: Close"
        };

        let instructions_widget = Paragraph::new(instructions)
            .style(Style::default().fg(theme.ai_assistant_help()))
            .alignment(Alignment::Center);
        frame.render_widget(instructions_widget, chunks[3]);
    }

    /// Render event modification suggestions
    fn render_modify_event(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &AICalendarUIState,
        theme: &Theme,
        block: Block,
    ) {
        if state.loading {
            self.render_loading(frame, area, theme, block, "Generating event suggestions...");
            return;
        }

        if let Some(error) = &state.error_message {
            self.render_error(frame, area, theme, block, error);
            return;
        }

        let suggestions = match &state.modification_suggestions {
            Some(suggestions) => suggestions,
            None => {
                self.render_error(frame, area, theme, block, "No modification suggestions available");
                return;
            }
        };

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(8),    // Suggestions
                Constraint::Length(2), // Instructions
            ])
            .split(inner);

        // Title
        let title = Paragraph::new("Event Modification Suggestions")
            .style(Style::default().fg(theme.ai_assistant_title()))
            .alignment(Alignment::Center);
        frame.render_widget(title, chunks[0]);

        // Suggestions list
        let mut items = Vec::new();
        let mut current_index = 0;

        // Add time suggestions
        if !suggestions.time_suggestions.is_empty() {
            items.push(ListItem::new(Line::from(vec![
                Span::styled("Time Suggestions:", Style::default().fg(theme.ai_assistant_section()).add_modifier(Modifier::BOLD))
            ])));
            
            for suggestion in &suggestions.time_suggestions {
                let style = if current_index == state.selected_suggestion {
                    Style::default().fg(theme.ai_assistant_selected()).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.ai_assistant_text())
                };
                items.push(ListItem::new(format!("  • {}", suggestion)).style(style));
                current_index += 1;
            }
        }

        // Add location suggestions
        if !suggestions.location_suggestions.is_empty() {
            items.push(ListItem::new(Line::from(vec![
                Span::styled("Location Suggestions:", Style::default().fg(theme.ai_assistant_section()).add_modifier(Modifier::BOLD))
            ])));
            
            for suggestion in &suggestions.location_suggestions {
                let style = if current_index == state.selected_suggestion {
                    Style::default().fg(theme.ai_assistant_selected()).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.ai_assistant_text())
                };
                items.push(ListItem::new(format!("  • {}", suggestion)).style(style));
                current_index += 1;
            }
        }

        // Add optimization tips
        if !suggestions.optimization_tips.is_empty() {
            items.push(ListItem::new(Line::from(vec![
                Span::styled("Optimization Tips:", Style::default().fg(theme.ai_assistant_section()).add_modifier(Modifier::BOLD))
            ])));
            
            for tip in &suggestions.optimization_tips {
                let style = if current_index == state.selected_suggestion {
                    Style::default().fg(theme.ai_assistant_selected()).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.ai_assistant_text())
                };
                items.push(ListItem::new(format!("  • {}", tip)).style(style));
                current_index += 1;
            }
        }

        let list = List::new(items)
            .style(Style::default().fg(theme.ai_assistant_text()));
        
        frame.render_stateful_widget(list, chunks[1], &mut state.list_state.clone());

        // Instructions
        let instructions = Paragraph::new("↑/↓: Navigate • Enter: Apply suggestion • Esc: Close")
            .style(Style::default().fg(theme.ai_assistant_help()))
            .alignment(Alignment::Center);
        frame.render_widget(instructions, chunks[2]);
    }

    /// Render meeting scheduling analysis
    fn render_schedule_meeting(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &AICalendarUIState,
        theme: &Theme,
        block: Block,
    ) {
        if state.loading {
            self.render_loading(frame, area, theme, block, "Analyzing optimal meeting times...");
            return;
        }

        if let Some(error) = &state.error_message {
            self.render_error(frame, area, theme, block, error);
            return;
        }

        let analysis = match &state.schedule_analysis {
            Some(analysis) => analysis,
            None => {
                self.render_error(frame, area, theme, block, "No schedule analysis available");
                return;
            }
        };

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(5),    // Optimal times
                Constraint::Length(4), // Recommendations
                Constraint::Length(2), // Instructions
            ])
            .split(inner);

        // Title
        let title = Paragraph::new("Meeting Schedule Analysis")
            .style(Style::default().fg(theme.ai_assistant_title()))
            .alignment(Alignment::Center);
        frame.render_widget(title, chunks[0]);

        // Optimal times
        let time_items: Vec<ListItem> = analysis.optimal_times
            .iter()
            .enumerate()
            .map(|(i, time)| {
                let style = if i == state.selected_suggestion {
                    Style::default().fg(theme.ai_assistant_selected()).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.ai_assistant_text())
                };
                
                ListItem::new(format!("• {}", time.format("%A, %B %d at %H:%M UTC")))
                    .style(style)
            })
            .collect();

        let times_list = List::new(time_items)
            .style(Style::default().fg(theme.ai_assistant_text()))
            .block(Block::default().title("Optimal Meeting Times").borders(Borders::ALL));
        
        frame.render_stateful_widget(times_list, chunks[1], &mut state.list_state.clone());

        // Recommendations
        let mut recommendations = Vec::new();
        recommendations.push(format!("Duration: {} minutes", analysis.suggested_duration));
        
        if let Some(location) = &analysis.location_recommendation {
            recommendations.push(format!("Location: {}", location));
        }
        
        if let Some(prep_time) = analysis.preparation_time {
            recommendations.push(format!("Preparation time: {} minutes", prep_time));
        }

        let recommendations_text = recommendations.join(" • ");
        let recommendations_widget = Paragraph::new(recommendations_text)
            .style(Style::default().fg(theme.ai_assistant_context()))
            .block(Block::default().title("Recommendations").borders(Borders::ALL))
            .wrap(Wrap { trim: true });
        frame.render_widget(recommendations_widget, chunks[2]);

        // Instructions
        let instructions = Paragraph::new("↑/↓: Select time • Enter: Schedule meeting • Esc: Close")
            .style(Style::default().fg(theme.ai_assistant_help()))
            .alignment(Alignment::Center);
        frame.render_widget(instructions, chunks[3]);
    }

    /// Render calendar insights
    fn render_calendar_insights(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &AICalendarUIState,
        theme: &Theme,
        block: Block,
    ) {
        if state.loading {
            self.render_loading(frame, area, theme, block, "Analyzing calendar patterns...");
            return;
        }

        if let Some(error) = &state.error_message {
            self.render_error(frame, area, theme, block, error);
            return;
        }

        let insights = match &state.calendar_insights {
            Some(insights) => insights,
            None => {
                self.render_error(frame, area, theme, block, "No calendar insights available");
                return;
            }
        };

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(3),    // Meeting patterns
                Constraint::Min(3),    // Productivity insights
                Constraint::Min(3),    // Focus time suggestions
                Constraint::Length(2), // Instructions
            ])
            .split(inner);

        // Title
        let title = Paragraph::new("Calendar Insights & Analytics")
            .style(Style::default().fg(theme.ai_assistant_title()))
            .alignment(Alignment::Center);
        frame.render_widget(title, chunks[0]);

        // Meeting patterns
        let patterns_text = if insights.meeting_patterns.is_empty() {
            "No meeting patterns identified".to_string()
        } else {
            insights.meeting_patterns.join("\n• ")
        };
        let patterns_widget = Paragraph::new(format!("• {}", patterns_text))
            .style(Style::default().fg(theme.ai_assistant_text()))
            .block(Block::default().title("Meeting Patterns").borders(Borders::ALL))
            .wrap(Wrap { trim: true });
        frame.render_widget(patterns_widget, chunks[1]);

        // Productivity insights
        let productivity_text = if insights.productivity_insights.is_empty() {
            "No productivity insights available".to_string()
        } else {
            insights.productivity_insights.join("\n• ")
        };
        let productivity_widget = Paragraph::new(format!("• {}", productivity_text))
            .style(Style::default().fg(theme.ai_assistant_text()))
            .block(Block::default().title("Productivity Insights").borders(Borders::ALL))
            .wrap(Wrap { trim: true });
        frame.render_widget(productivity_widget, chunks[2]);

        // Focus time suggestions
        let focus_text = if insights.focus_time_suggestions.is_empty() {
            "No focus time recommendations".to_string()
        } else {
            insights.focus_time_suggestions.join("\n• ")
        };
        let focus_widget = Paragraph::new(format!("• {}", focus_text))
            .style(Style::default().fg(theme.ai_assistant_text()))
            .block(Block::default().title("Focus Time Recommendations").borders(Borders::ALL))
            .wrap(Wrap { trim: true });
        frame.render_widget(focus_widget, chunks[3]);

        // Instructions
        let instructions = Paragraph::new("Esc: Close")
            .style(Style::default().fg(theme.ai_assistant_help()))
            .alignment(Alignment::Center);
        frame.render_widget(instructions, chunks[4]);
    }

    /// Render quick schedule interface
    fn render_quick_schedule(
        &self,
        frame: &mut Frame,
        area: Rect,
        _state: &AICalendarUIState,
        theme: &Theme,
        block: Block,
    ) {
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let placeholder = Paragraph::new("Quick scheduling commands:\n\n• \"Schedule team standup daily at 9 AM\"\n• \"Block focus time every afternoon 2-4 PM\"\n• \"Monthly review meeting first Friday\"\n• \"Coffee with John next week\"")
            .style(Style::default().fg(theme.ai_assistant_text()))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        frame.render_widget(placeholder, inner);
    }

    /// Render parsed event preview
    fn render_parsed_event_preview(
        &self,
        frame: &mut Frame,
        area: Rect,
        parsed: &ParsedEventInfo,
        theme: &Theme,
    ) {
        let preview_text = format!(
            "Event Preview:\n\nTitle: {}\nStart: {}\nEnd: {}\nLocation: {}\nAttendees: {}\nConfidence: {:.1}%",
            parsed.title,
            parsed.start_time.format("%Y-%m-%d %H:%M UTC"),
            parsed.end_time.format("%Y-%m-%d %H:%M UTC"),
            parsed.location.as_deref().unwrap_or("Not specified"),
            if parsed.attendees.is_empty() { "None".to_string() } else { parsed.attendees.join(", ") },
            parsed.confidence * 100.0
        );

        let preview_widget = Paragraph::new(preview_text)
            .style(Style::default().fg(theme.ai_assistant_text()))
            .block(Block::default().title("Parsed Event").borders(Borders::ALL))
            .wrap(Wrap { trim: true });
        frame.render_widget(preview_widget, area);
    }

    /// Render loading state
    fn render_loading(
        &self,
        frame: &mut Frame,
        area: Rect,
        theme: &Theme,
        block: Block,
        message: &str,
    ) {
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let loading_text = format!("🤖 {}", message);
        let loading = Paragraph::new(loading_text)
            .style(Style::default().fg(theme.ai_assistant_loading()))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        frame.render_widget(loading, inner);
    }

    /// Render error state
    fn render_error(
        &self,
        frame: &mut Frame,
        area: Rect,
        theme: &Theme,
        block: Block,
        error: &str,
    ) {
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let error_text = format!("❌ Error: {}", error);
        let error_widget = Paragraph::new(error_text)
            .style(Style::default().fg(theme.ai_assistant_error()))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        frame.render_widget(error_widget, inner);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_calendar_ui_state() {
        let mut state = AICalendarUIState::new();
        assert_eq!(state.mode, AICalendarMode::Hidden);
        assert!(!state.enabled);

        state.enable();
        assert!(state.enabled);

        state.set_create_event_mode();
        assert_eq!(state.mode, AICalendarMode::CreateEvent);
        assert!(state.input_mode);

        state.add_char('H');
        state.add_char('i');
        assert_eq!(state.input_text, "Hi");

        state.remove_char();
        assert_eq!(state.input_text, "H");
    }

    #[test]
    fn test_natural_language_request_creation() {
        let mut state = AICalendarUIState::new();
        state.input_text = "Team meeting tomorrow at 2 PM".to_string();
        
        let request = state.get_event_request();
        assert!(request.is_some());
        assert_eq!(request.unwrap().description, "Team meeting tomorrow at 2 PM");
    }
}