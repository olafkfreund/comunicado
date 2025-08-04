//! AI assistant UI components for email management

use crate::email::{AIEmailAssistant, EmailCompositionAssistance, EmailReplyAssistance, EmailSummary, BulkEmailAnalysis, BulkAnalysisStats};
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
    /// Current bulk analysis results
    pub bulk_analysis: Option<BulkEmailAnalysis>,
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
            bulk_analysis: None,
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

    /// Render bulk analysis interface
    fn render_bulk_analysis(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &AIAssistantUIState,
        theme: &Theme,
        block: Block,
    ) {
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if let Some(bulk_analysis) = &state.bulk_analysis {
            self.render_bulk_analysis_results(frame, inner, bulk_analysis, theme);
        } else {
            self.render_bulk_analysis_start_screen(frame, inner, theme);
        }
    }

    /// Render bulk analysis results
    fn render_bulk_analysis_results(
        &self,
        frame: &mut Frame,
        area: Rect,
        analysis: &BulkEmailAnalysis,
        theme: &Theme,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8),  // Stats overview
                Constraint::Length(6),  // Category distribution
                Constraint::Min(0),     // Insights
            ])
            .split(area);

        // Stats overview
        self.render_analysis_stats(frame, chunks[0], &analysis.stats, theme);
        
        // Category distribution
        self.render_category_distribution(frame, chunks[1], &analysis.category_distribution, theme);
        
        // Overall insights
        self.render_analysis_insights(frame, chunks[2], &analysis.insights, theme);
    }

    /// Render analysis statistics
    fn render_analysis_stats(
        &self,
        frame: &mut Frame,
        area: Rect,
        stats: &BulkAnalysisStats,
        theme: &Theme,
    ) {
        let success_rate = if stats.total_processed > 0 {
            (stats.successful as f32 / stats.total_processed as f32) * 100.0
        } else {
            0.0
        };

        let stats_text = format!(
            "📊 Analysis Statistics\n\n\
            Total Emails Processed: {}\n\
            Successfully Analyzed: {} ({:.1}%)\n\
            Failed Analyses: {}\n\
            Processing Time: {:.2}s",
            stats.total_processed,
            stats.successful,
            success_rate,
            stats.failed,
            stats.processing_time_ms as f32 / 1000.0
        );

        let stats_widget = Paragraph::new(stats_text)
            .style(Style::default().fg(theme.ai_assistant_text()))
            .block(
                Block::default()
                    .title("Statistics")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.ai_assistant_border()))
            )
            .wrap(Wrap { trim: true });
        
        frame.render_widget(stats_widget, area);
    }

    /// Render category distribution
    fn render_category_distribution(
        &self,
        frame: &mut Frame,
        area: Rect,
        categories: &std::collections::HashMap<crate::ai::EmailCategory, usize>,
        theme: &Theme,
    ) {
        let mut category_items = Vec::new();
        for (category, count) in categories {
            let category_name = match category {
                crate::ai::EmailCategory::Work => "💼 Work",
                crate::ai::EmailCategory::Personal => "👤 Personal", 
                crate::ai::EmailCategory::Promotional => "📢 Promotional",
                crate::ai::EmailCategory::Social => "👥 Social",
                crate::ai::EmailCategory::Financial => "💰 Financial",
                crate::ai::EmailCategory::Travel => "✈️ Travel",
                crate::ai::EmailCategory::Shopping => "🛒 Shopping",
                crate::ai::EmailCategory::Newsletter => "📰 Newsletter",
                crate::ai::EmailCategory::System => "⚙️ System",
                crate::ai::EmailCategory::Spam => "🚫 Spam",
                crate::ai::EmailCategory::Uncategorized => "📁 Uncategorized",
            };
            category_items.push(ListItem::new(format!("  {} - {} emails", category_name, count)));
        }

        let category_list = List::new(category_items)
            .style(Style::default().fg(theme.ai_assistant_text()))
            .block(
                Block::default()
                    .title("Email Categories")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.ai_assistant_border()))
            );

        frame.render_widget(category_list, area);
    }

    /// Render analysis insights
    fn render_analysis_insights(
        &self,
        frame: &mut Frame,
        area: Rect,
        insights: &[String],
        theme: &Theme,
    ) {
        let insights_text = if insights.is_empty() {
            "No specific insights available for this analysis.".to_string()
        } else {
            format!("🔍 Key Insights:\n\n• {}", insights.join("\n• "))
        };

        let insights_widget = Paragraph::new(insights_text)
            .style(Style::default().fg(theme.ai_assistant_text()))
            .block(
                Block::default()
                    .title("Analysis Insights")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.ai_assistant_border()))
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(insights_widget, area);
    }

    /// Render bulk analysis start screen
    fn render_bulk_analysis_start_screen(
        &self,
        frame: &mut Frame,
        area: Rect,
        theme: &Theme,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6),  // Title and description
                Constraint::Min(0),     // Instructions
            ])
            .split(area);

        // Title and description
        let title_text = "🤖 AI Bulk Email Analysis\n\n\
            Analyze multiple emails at once to:\n\
            • Categorize emails automatically\n\
            • Extract key insights and patterns\n\
            • Generate summary reports";

        let title_widget = Paragraph::new(title_text)
            .style(Style::default().fg(theme.ai_assistant_text()))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        frame.render_widget(title_widget, chunks[0]);

        // Instructions
        let instructions_text = "📋 How to Use:\n\n\
            1. Select emails in your inbox that you want to analyze\n\
            2. Press 'A' to start bulk analysis\n\
            3. Choose analysis parameters (categories, insights, etc.)\n\
            4. Review results and insights\n\n\
            💡 Features:\n\
            • Smart email categorization\n\
            • Sentiment analysis\n\
            • Priority detection\n\
            • Action item extraction\n\
            • Duplicate detection\n\n\
            Press 'A' when you have emails selected to begin analysis.";

        let instructions_widget = Paragraph::new(instructions_text)
            .style(Style::default().fg(theme.ai_assistant_context()))
            .block(
                Block::default()
                    .title("Getting Started")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.ai_assistant_border()))
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(instructions_widget, chunks[1]);
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