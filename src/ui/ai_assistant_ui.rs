//! AI assistant UI components for email management

use crate::email::{AIEmailAssistant, EmailCompositionAssistance, EmailReplyAssistance, EmailSummary};
use crate::theme::Theme;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use std::sync::Arc;

/// AI assistant UI state
#[derive(Debug, Clone)]
pub struct AIAssistantUIState {
    /// Current mode of AI assistance
    pub mode: AIAssistantMode,
    /// Whether AI assistance is enabled
    pub enabled: bool,
    /// Loading state for AI operations
    pub loading: bool,
    /// Current composition assistance data
    pub composition_assistance: Option<EmailCompositionAssistance>,
    /// Current reply assistance data
    pub reply_assistance: Option<EmailReplyAssistance>,
    /// Current email summary
    pub email_summary: Option<EmailSummary>,
    /// Selected suggestion index
    pub selected_suggestion: usize,
    /// Error message if any
    pub error_message: Option<String>,
    /// List state for navigating suggestions
    pub list_state: ListState,
}

/// AI assistant modes
#[derive(Debug, Clone, PartialEq)]
pub enum AIAssistantMode {
    /// Hidden/inactive
    Hidden,
    /// Email composition assistance
    Compose,
    /// Email reply assistance
    Reply,
    /// Email summarization
    Summarize,
    /// Bulk email analysis
    BulkAnalysis,
}

impl Default for AIAssistantUIState {
    fn default() -> Self {
        Self {
            mode: AIAssistantMode::Hidden,
            enabled: false,
            loading: false,
            composition_assistance: None,
            reply_assistance: None,
            email_summary: None,
            selected_suggestion: 0,
            error_message: None,
            list_state: ListState::default(),
        }
    }
}

impl AIAssistantUIState {
    /// Create new AI assistant UI state
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable AI assistance
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable AI assistance
    pub fn disable(&mut self) {
        self.enabled = false;
        self.mode = AIAssistantMode::Hidden;
    }

    /// Set composition assistance mode
    pub fn set_compose_mode(&mut self, assistance: EmailCompositionAssistance) {
        self.mode = AIAssistantMode::Compose;
        self.composition_assistance = Some(assistance);
        self.selected_suggestion = 0;
        self.list_state.select(Some(0));
        self.error_message = None;
    }

    /// Set reply assistance mode
    pub fn set_reply_mode(&mut self, assistance: EmailReplyAssistance) {
        self.mode = AIAssistantMode::Reply;
        self.reply_assistance = Some(assistance);
        self.selected_suggestion = 0;
        self.list_state.select(Some(0));
        self.error_message = None;
    }

    /// Set summarize mode
    pub fn set_summarize_mode(&mut self, summary: EmailSummary) {
        self.mode = AIAssistantMode::Summarize;
        self.email_summary = Some(summary);
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

    /// Hide the AI assistant
    pub fn hide(&mut self) {
        self.mode = AIAssistantMode::Hidden;
        self.composition_assistance = None;
        self.reply_assistance = None;
        self.email_summary = None;
        self.error_message = None;
        self.loading = false;
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
            AIAssistantMode::Compose => {
                self.composition_assistance
                    .as_ref()
                    .map(|a| a.subject_suggestions.len() + a.body_suggestions.len())
                    .unwrap_or(0)
            },
            AIAssistantMode::Reply => {
                self.reply_assistance
                    .as_ref()
                    .map(|a| a.reply_suggestions.len())
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
            AIAssistantMode::Compose => {
                if let Some(assistance) = &self.composition_assistance {
                    let all_suggestions: Vec<_> = assistance.subject_suggestions
                        .iter()
                        .chain(assistance.body_suggestions.iter())
                        .collect();
                    all_suggestions.get(self.selected_suggestion).map(|s| s.to_string())
                } else {
                    None
                }
            },
            AIAssistantMode::Reply => {
                self.reply_assistance
                    .as_ref()
                    .and_then(|a| a.reply_suggestions.get(self.selected_suggestion))
                    .cloned()
            },
            _ => None,
        }
    }
}

/// AI assistant UI component
pub struct AIAssistantUI {
    assistant: Arc<AIEmailAssistant>,
}

impl AIAssistantUI {
    /// Create new AI assistant UI
    pub fn new(assistant: Arc<AIEmailAssistant>) -> Self {
        Self { assistant }
    }

    /// Get reference to the email assistant
    pub fn assistant(&self) -> &Arc<AIEmailAssistant> {
        &self.assistant
    }

    /// Check if AI is available
    pub async fn is_available(&self) -> bool {
        self.assistant.is_available().await
    }

    /// Render AI assistant UI
    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &mut AIAssistantUIState,
        theme: &Theme,
    ) {
        if !state.enabled || state.mode == AIAssistantMode::Hidden {
            return;
        }

        // Clear the area
        frame.render_widget(Clear, area);

        let block = Block::default()
            .title(" AI Assistant ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.ai_assistant_border()));

        match &state.mode {
            AIAssistantMode::Compose => {
                self.render_compose_assistance(frame, area, state, theme, block);
            },
            AIAssistantMode::Reply => {
                self.render_reply_assistance(frame, area, state, theme, block);
            },
            AIAssistantMode::Summarize => {
                self.render_email_summary(frame, area, state, theme, block);
            },
            AIAssistantMode::BulkAnalysis => {
                self.render_bulk_analysis(frame, area, state, theme, block);
            },
            AIAssistantMode::Hidden => {
                // Already handled above
            },
        }
    }

    /// Render composition assistance
    fn render_compose_assistance(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &AIAssistantUIState,
        theme: &Theme,
        block: Block,
    ) {
        if state.loading {
            self.render_loading(frame, area, theme, block, "Generating email suggestions...");
            return;
        }

        if let Some(error) = &state.error_message {
            self.render_error(frame, area, theme, block, error);
            return;
        }

        let assistance = match &state.composition_assistance {
            Some(assistance) => assistance,
            None => {
                self.render_error(frame, area, theme, block, "No composition assistance available");
                return;
            }
        };

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(5),    // Suggestions
                Constraint::Length(2), // Instructions
            ])
            .split(inner);

        // Title
        let title = Paragraph::new("Email Composition Assistance")
            .style(Style::default().fg(theme.ai_assistant_title()))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        frame.render_widget(title, chunks[0]);

        // Suggestions
        let mut items = Vec::new();
        
        // Subject suggestions
        if !assistance.subject_suggestions.is_empty() {
            items.push(ListItem::new(Line::from(vec![
                Span::styled("Subject Suggestions:", Style::default().fg(theme.ai_assistant_section()).add_modifier(Modifier::BOLD))
            ])));
            
            for (i, suggestion) in assistance.subject_suggestions.iter().enumerate() {
                let style = if i == state.selected_suggestion {
                    Style::default().fg(theme.ai_assistant_selected()).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.ai_assistant_text())
                };
                items.push(ListItem::new(format!("  • {}", suggestion)).style(style));
            }
        }

        // Body suggestions
        if !assistance.body_suggestions.is_empty() {
            items.push(ListItem::new(Line::from(vec![
                Span::styled("Body Suggestions:", Style::default().fg(theme.ai_assistant_section()).add_modifier(Modifier::BOLD))
            ])));
            
            for (i, suggestion) in assistance.body_suggestions.iter().enumerate() {
                let idx = assistance.subject_suggestions.len() + i;
                let style = if idx == state.selected_suggestion {
                    Style::default().fg(theme.ai_assistant_selected()).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.ai_assistant_text())
                };
                
                // Truncate long suggestions for display
                let display_text = if suggestion.len() > 100 {
                    format!("  • {}...", &suggestion[..97])
                } else {
                    format!("  • {}", suggestion)
                };
                items.push(ListItem::new(display_text).style(style));
            }
        }

        let list = List::new(items)
            .style(Style::default().fg(theme.ai_assistant_text()));
        
        frame.render_stateful_widget(list, chunks[1], &mut state.list_state.clone());

        // Instructions
        let instructions = Paragraph::new("↑/↓: Navigate • Enter: Use suggestion • Esc: Close")
            .style(Style::default().fg(theme.ai_assistant_help()))
            .alignment(Alignment::Center);
        frame.render_widget(instructions, chunks[2]);
    }

    /// Render reply assistance
    fn render_reply_assistance(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &AIAssistantUIState,
        theme: &Theme,
        block: Block,
    ) {
        if state.loading {
            self.render_loading(frame, area, theme, block, "Generating reply suggestions...");
            return;
        }

        if let Some(error) = &state.error_message {
            self.render_error(frame, area, theme, block, error);
            return;
        }

        let assistance = match &state.reply_assistance {
            Some(assistance) => assistance,
            None => {
                self.render_error(frame, area, theme, block, "No reply assistance available");
                return;
            }
        };

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Length(3), // Context
                Constraint::Min(5),    // Suggestions
                Constraint::Length(2), // Instructions
            ])
            .split(inner);

        // Title
        let title = Paragraph::new("Email Reply Assistance")
            .style(Style::default().fg(theme.ai_assistant_title()))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        frame.render_widget(title, chunks[0]);

        // Context
        let context_text = format!(
            "Original tone: {} | Suggested tone: {} | Context: {}",
            assistance.original_tone,
            assistance.suggested_tone,
            if assistance.context_summary.len() > 50 {
                format!("{}...", &assistance.context_summary[..47])
            } else {
                assistance.context_summary.clone()
            }
        );
        let context = Paragraph::new(context_text)
            .style(Style::default().fg(theme.ai_assistant_context()))
            .wrap(Wrap { trim: true });
        frame.render_widget(context, chunks[1]);

        // Reply suggestions
        let items: Vec<ListItem> = assistance.reply_suggestions
            .iter()
            .enumerate()
            .map(|(i, suggestion)| {
                let style = if i == state.selected_suggestion {
                    Style::default().fg(theme.ai_assistant_selected()).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.ai_assistant_text())
                };
                
                let display_text = if suggestion.len() > 150 {
                    format!("{}...", &suggestion[..147])
                } else {
                    suggestion.clone()
                };
                
                ListItem::new(display_text).style(style)
            })
            .collect();

        let list = List::new(items)
            .style(Style::default().fg(theme.ai_assistant_text()))
            .highlight_style(Style::default().fg(theme.ai_assistant_selected()).add_modifier(Modifier::BOLD));
        
        frame.render_stateful_widget(list, chunks[2], &mut state.list_state.clone());

        // Instructions
        let instructions = Paragraph::new("↑/↓: Navigate • Enter: Use reply • Esc: Close")
            .style(Style::default().fg(theme.ai_assistant_help()))
            .alignment(Alignment::Center);
        frame.render_widget(instructions, chunks[3]);
    }

    /// Render email summary
    fn render_email_summary(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &AIAssistantUIState,
        theme: &Theme,
        block: Block,
    ) {
        if state.loading {
            self.render_loading(frame, area, theme, block, "Analyzing email...");
            return;
        }

        if let Some(error) = &state.error_message {
            self.render_error(frame, area, theme, block, error);
            return;
        }

        let summary = match &state.email_summary {
            Some(summary) => summary,
            None => {
                self.render_error(frame, area, theme, block, "No email summary available");
                return;
            }
        };

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(3),    // Summary
                Constraint::Min(3),    // Key points
                Constraint::Length(3), // Category and confidence
                Constraint::Length(2), // Instructions
            ])
            .split(inner);

        // Title
        let title = Paragraph::new("Email Summary")
            .style(Style::default().fg(theme.ai_assistant_title()))
            .alignment(Alignment::Center);
        frame.render_widget(title, chunks[0]);

        // Summary
        let summary_widget = Paragraph::new(summary.summary.clone())
            .style(Style::default().fg(theme.ai_assistant_text()))
            .wrap(Wrap { trim: true })
            .block(Block::default().title("Summary").borders(Borders::ALL));
        frame.render_widget(summary_widget, chunks[1]);

        // Key points
        let key_points_text = summary.key_points.join("\n• ");
        let key_points_widget = Paragraph::new(format!("• {}", key_points_text))
            .style(Style::default().fg(theme.ai_assistant_text()))
            .wrap(Wrap { trim: true })
            .block(Block::default().title("Key Points").borders(Borders::ALL));
        frame.render_widget(key_points_widget, chunks[2]);

        // Category and confidence
        let category_text = format!(
            "Category: {} | Confidence: {:.1}%",
            summary.category,
            summary.confidence * 100.0
        );
        let category_widget = Paragraph::new(category_text)
            .style(Style::default().fg(theme.ai_assistant_context()))
            .alignment(Alignment::Center);
        frame.render_widget(category_widget, chunks[3]);

        // Instructions
        let instructions = Paragraph::new("Esc: Close")
            .style(Style::default().fg(theme.ai_assistant_help()))
            .alignment(Alignment::Center);
        frame.render_widget(instructions, chunks[4]);
    }

    /// Render bulk analysis placeholder
    fn render_bulk_analysis(
        &self,
        frame: &mut Frame,
        area: Rect,
        _state: &AIAssistantUIState,
        theme: &Theme,
        block: Block,
    ) {
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let placeholder = Paragraph::new("Bulk email analysis feature coming soon...")
            .style(Style::default().fg(theme.ai_assistant_text()))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        frame.render_widget(placeholder, inner);
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
    use crate::email::EmailCompositionAssistance;

    #[test]
    fn test_ai_assistant_ui_state() {
        let mut state = AIAssistantUIState::new();
        assert_eq!(state.mode, AIAssistantMode::Hidden);
        assert!(!state.enabled);

        state.enable();
        assert!(state.enabled);

        let assistance = EmailCompositionAssistance {
            subject_suggestions: vec!["Test Subject".to_string()],
            body_suggestions: vec!["Test Body".to_string()],
            tone_suggestions: vec!["Professional".to_string()],
            key_points: vec!["Key point".to_string()],
            next_actions: vec!["Action item".to_string()],
        };

        state.set_compose_mode(assistance);
        assert_eq!(state.mode, AIAssistantMode::Compose);
        assert!(state.composition_assistance.is_some());

        state.move_down();
        assert_eq!(state.selected_suggestion, 1);

        state.move_up();
        assert_eq!(state.selected_suggestion, 0);
    }

    #[test]
    fn test_selected_suggestion_retrieval() {
        let mut state = AIAssistantUIState::new();
        let assistance = EmailCompositionAssistance {
            subject_suggestions: vec!["Subject 1".to_string(), "Subject 2".to_string()],
            body_suggestions: vec!["Body 1".to_string()],
            tone_suggestions: vec![],
            key_points: vec![],
            next_actions: vec![],
        };

        state.set_compose_mode(assistance);
        
        assert_eq!(state.get_selected_suggestion(), Some("Subject 1".to_string()));
        
        state.move_down();
        assert_eq!(state.get_selected_suggestion(), Some("Subject 2".to_string()));
        
        state.move_down();
        assert_eq!(state.get_selected_suggestion(), Some("Body 1".to_string()));
    }
}