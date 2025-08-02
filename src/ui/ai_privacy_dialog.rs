//! AI privacy consent dialog components

use crate::ai::config::{AIProviderType, PrivacyMode};
use crate::theme::Theme;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

/// Privacy consent dialog state
#[derive(Debug, Clone)]
pub struct AIPrivacyDialogState {
    /// Whether the dialog is visible
    pub visible: bool,
    /// Current operation requesting consent
    pub operation: String,
    /// Provider that would be used
    pub provider: AIProviderType,
    /// Data that would be sent
    pub data_description: String,
    /// Privacy implications
    pub privacy_implications: Vec<String>,
    /// User's current privacy mode
    pub privacy_mode: PrivacyMode,
    /// Selected option (0 = Allow, 1 = Deny, 2 = Allow Always, 3 = Deny Always)
    pub selected_option: usize,
    /// List state for navigation
    pub list_state: ListState,
    /// Whether to remember the decision
    pub remember_decision: bool,
}

impl Default for AIPrivacyDialogState {
    fn default() -> Self {
        Self {
            visible: false,
            operation: String::new(),
            provider: AIProviderType::None,
            data_description: String::new(),
            privacy_implications: Vec::new(),
            privacy_mode: PrivacyMode::LocalPreferred,
            selected_option: 1, // Default to "Deny"
            list_state: ListState::default(),
            remember_decision: false,
        }
    }
}

impl AIPrivacyDialogState {
    /// Create new privacy dialog state
    pub fn new() -> Self {
        Self::default()
    }

    /// Show privacy consent dialog
    pub fn show_consent_dialog(
        &mut self,
        operation: String,
        provider: AIProviderType,
        data_description: String,
        privacy_mode: PrivacyMode,
    ) {
        self.visible = true;
        self.operation = operation;
        self.privacy_implications = self.get_privacy_implications(&provider);
        self.provider = provider;
        self.data_description = data_description;
        self.privacy_mode = privacy_mode;
        self.selected_option = 1; // Default to deny
        self.remember_decision = false;
        
        self.list_state.select(Some(self.selected_option));
    }

    /// Hide the privacy dialog
    pub fn hide(&mut self) {
        self.visible = false;
        self.operation.clear();
        self.provider = AIProviderType::None;
        self.data_description.clear();
        self.privacy_implications.clear();
        self.selected_option = 1;
        self.remember_decision = false;
    }

    /// Move selection up
    pub fn move_up(&mut self) {
        if self.selected_option > 0 {
            self.selected_option -= 1;
            self.list_state.select(Some(self.selected_option));
        }
    }

    /// Move selection down
    pub fn move_down(&mut self) {
        let max_options = 4; // Allow, Deny, Allow Always, Deny Always
        if self.selected_option < max_options - 1 {
            self.selected_option += 1;
            self.list_state.select(Some(self.selected_option));
        }
    }

    /// Toggle remember decision
    pub fn toggle_remember(&mut self) {
        self.remember_decision = !self.remember_decision;
    }

    /// Get the user's consent decision
    pub fn get_consent_decision(&self) -> ConsentDecision {
        match self.selected_option {
            0 => ConsentDecision::Allow,
            1 => ConsentDecision::Deny,
            2 => ConsentDecision::AllowAlways,
            3 => ConsentDecision::DenyAlways,
            _ => ConsentDecision::Deny,
        }
    }

    /// Get privacy implications for a provider
    fn get_privacy_implications(&self, provider: &AIProviderType) -> Vec<String> {
        match provider {
            AIProviderType::OpenAI => vec![
                "Data will be sent to OpenAI's servers in the United States".to_string(),
                "OpenAI may store and process your data according to their privacy policy".to_string(),
                "Data transmission occurs over encrypted connections".to_string(),
                "OpenAI has committed to not using API data for training".to_string(),
                "Your data may be subject to US data protection laws".to_string(),
            ],
            AIProviderType::Anthropic => vec![
                "Data will be sent to Anthropic's servers in the United States".to_string(),
                "Anthropic may process your data according to their privacy policy".to_string(),
                "All communications are encrypted in transit".to_string(),
                "Anthropic does not train models on customer data".to_string(),
                "Data processing is subject to US privacy regulations".to_string(),
            ],
            AIProviderType::Google => vec![
                "Data will be sent to Google's AI services infrastructure".to_string(),
                "Google may process data according to their AI services terms".to_string(),
                "Data is encrypted during transmission and processing".to_string(),
                "Google's data retention policies apply".to_string(),
                "Processing may occur in multiple geographic regions".to_string(),
            ],
            AIProviderType::Ollama => vec![
                "Data will be processed locally on your machine".to_string(),
                "No data is sent to external servers".to_string(),
                "All processing remains under your control".to_string(),
                "No external privacy policies apply".to_string(),
            ],
            AIProviderType::None => vec![
                "No AI processing will occur".to_string(),
            ],
        }
    }
}

/// Consent decision options
#[derive(Debug, Clone, PartialEq)]
pub enum ConsentDecision {
    /// Allow this one operation
    Allow,
    /// Deny this operation
    Deny,
    /// Allow this type of operation always
    AllowAlways,
    /// Deny this type of operation always
    DenyAlways,
}

/// AI Privacy consent dialog component
pub struct AIPrivacyDialog;

impl AIPrivacyDialog {
    /// Create new privacy dialog
    pub fn new() -> Self {
        Self
    }

    /// Render privacy consent dialog
    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &mut AIPrivacyDialogState,
        theme: &Theme,
    ) {
        if !state.visible {
            return;
        }

        // Create a large centered dialog
        let dialog_area = self.centered_rect(80, 70, area);
        
        // Clear the area
        frame.render_widget(Clear, dialog_area);

        let block = Block::default()
            .title(" AI Privacy Consent Required ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        let inner = block.inner(dialog_area);
        frame.render_widget(block, dialog_area);

        // Layout: operation info, data description, privacy implications, options
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Operation description
                Constraint::Length(4), // Data description
                Constraint::Min(6),    // Privacy implications
                Constraint::Length(6), // Options
                Constraint::Length(2), // Instructions
            ])
            .split(inner);

        // Operation description
        self.render_operation_info(frame, chunks[0], state, theme);

        // Data description
        self.render_data_description(frame, chunks[1], state, theme);

        // Privacy implications
        self.render_privacy_implications(frame, chunks[2], state, theme);

        // Options
        self.render_consent_options(frame, chunks[3], state, theme);

        // Instructions
        self.render_instructions(frame, chunks[4], state, theme);
    }

    /// Render operation information
    fn render_operation_info(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &AIPrivacyDialogState,
        theme: &Theme,
    ) {
        let text = format!(
            "The AI operation \"{}\" requires sending data to {} ({}).\n\nYour current privacy mode is: {}",
            state.operation,
            state.provider,
            match state.provider {
                AIProviderType::OpenAI => "Cloud Service",
                AIProviderType::Anthropic => "Cloud Service", 
                AIProviderType::Google => "Cloud Service",
                AIProviderType::Ollama => "Local Processing",
                AIProviderType::None => "Disabled",
            },
            match state.privacy_mode {
                PrivacyMode::LocalOnly => "Local Only",
                PrivacyMode::LocalPreferred => "Local Preferred",
                PrivacyMode::CloudWithConsent => "Cloud with Consent",
                PrivacyMode::CloudAllowed => "Cloud Allowed",
            }
        );

        let paragraph = Paragraph::new(text)
            .style(Style::default().fg(theme.ai_assistant_text()))
            .block(Block::default().title("Operation Details").borders(Borders::ALL))
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    /// Render data description
    fn render_data_description(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &AIPrivacyDialogState,
        theme: &Theme,
    ) {
        let paragraph = Paragraph::new(state.data_description.clone())
            .style(Style::default().fg(theme.ai_assistant_text()))
            .block(Block::default().title("Data to be Processed").borders(Borders::ALL))
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    /// Render privacy implications
    fn render_privacy_implications(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &AIPrivacyDialogState,
        theme: &Theme,
    ) {
        let text = if state.privacy_implications.is_empty() {
            "No specific privacy implications identified.".to_string()
        } else {
            format!("• {}", state.privacy_implications.join("\n• "))
        };

        let paragraph = Paragraph::new(text)
            .style(Style::default().fg(theme.ai_assistant_context()))
            .block(Block::default().title("Privacy Implications").borders(Borders::ALL))
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    /// Render consent options
    fn render_consent_options(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &AIPrivacyDialogState,
        theme: &Theme,
    ) {
        let options = vec![
            ("Allow Once", "Allow this operation one time"),
            ("Deny", "Deny this operation"),
            ("Allow Always", "Always allow this type of operation"),
            ("Deny Always", "Always deny this type of operation"),
        ];

        let items: Vec<ListItem> = options
            .iter()
            .enumerate()
            .map(|(i, (name, description))| {
                let is_selected = i == state.selected_option;
                
                let spans = vec![
                    Span::styled(
                        format!("● {} ", name),
                        Style::default().fg(if is_selected {
                            theme.ai_assistant_selected()
                        } else {
                            theme.ai_assistant_text()
                        }).add_modifier(if is_selected {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        })
                    ),
                    Span::styled(
                        format!("- {}", description),
                        Style::default().fg(theme.ai_assistant_context())
                    ),
                ];

                ListItem::new(Line::from(spans))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().title("Your Decision").borders(Borders::ALL))
            .style(Style::default().fg(theme.ai_assistant_text()));

        frame.render_stateful_widget(list, area, &mut state.list_state.clone());
    }

    /// Render instructions
    fn render_instructions(
        &self,
        frame: &mut Frame,
        area: Rect,
        _state: &AIPrivacyDialogState,
        theme: &Theme,
    ) {
        let instructions = "↑/↓: Navigate options • Enter: Confirm decision • Esc: Cancel • r: Toggle remember";

        let paragraph = Paragraph::new(instructions)
            .style(Style::default().fg(theme.ai_assistant_help()))
            .alignment(Alignment::Center);

        frame.render_widget(paragraph, area);
    }

    /// Helper function to create a centered rectangle
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

/// Privacy consent manager for handling consent logic
pub struct PrivacyConsentManager {
    /// Stored consent decisions
    consent_cache: std::collections::HashMap<String, ConsentDecision>,
}

impl PrivacyConsentManager {
    /// Create new consent manager
    pub fn new() -> Self {
        Self {
            consent_cache: std::collections::HashMap::new(),
        }
    }

    /// Check if consent is required for an operation
    pub fn is_consent_required(
        &self,
        operation: &str,
        provider: &AIProviderType,
        privacy_mode: &PrivacyMode,
    ) -> bool {
        match privacy_mode {
            PrivacyMode::LocalOnly => {
                // Only allow local processing
                !matches!(provider, AIProviderType::Ollama | AIProviderType::None)
            },
            PrivacyMode::CloudAllowed => {
                // Always allow cloud processing
                false
            },
            PrivacyMode::LocalPreferred | PrivacyMode::CloudWithConsent => {
                // Check if we have a cached decision
                if let Some(decision) = self.consent_cache.get(operation) {
                    match decision {
                        ConsentDecision::AllowAlways => false,
                        ConsentDecision::DenyAlways => false, // Will be handled as denial
                        _ => true, // Need to ask again
                    }
                } else {
                    // Need consent for cloud providers
                    matches!(
                        provider,
                        AIProviderType::OpenAI | AIProviderType::Anthropic | AIProviderType::Google
                    )
                }
            },
        }
    }

    /// Record a consent decision
    pub fn record_consent(&mut self, operation: String, decision: ConsentDecision) {
        match decision {
            ConsentDecision::AllowAlways | ConsentDecision::DenyAlways => {
                self.consent_cache.insert(operation, decision);
            },
            _ => {
                // Don't cache one-time decisions
            }
        }
    }

    /// Check if an operation is allowed based on cached consent
    pub fn is_operation_allowed(&self, operation: &str) -> Option<bool> {
        self.consent_cache.get(operation).map(|decision| {
            matches!(decision, ConsentDecision::AllowAlways)
        })
    }

    /// Clear all consent decisions
    pub fn clear_all_consent(&mut self) {
        self.consent_cache.clear();
    }

    /// Get all stored consent decisions
    pub fn get_all_consent(&self) -> &std::collections::HashMap<String, ConsentDecision> {
        &self.consent_cache
    }
}

impl Default for PrivacyConsentManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_privacy_dialog_state() {
        let mut state = AIPrivacyDialogState::new();
        assert!(!state.visible);

        state.show_consent_dialog(
            "email_summary".to_string(),
            AIProviderType::OpenAI,
            "Email content for summarization".to_string(),
            PrivacyMode::CloudWithConsent,
        );

        assert!(state.visible);
        assert_eq!(state.operation, "email_summary");
        assert_eq!(state.provider, AIProviderType::OpenAI);
        assert_eq!(state.selected_option, 1); // Default to deny

        state.move_up();
        assert_eq!(state.selected_option, 0); // Allow

        state.hide();
        assert!(!state.visible);
    }

    #[test]
    fn test_consent_manager() {
        let mut manager = PrivacyConsentManager::new();

        // Test consent requirement checking
        assert!(manager.is_consent_required(
            "email_summary",
            &AIProviderType::OpenAI,
            &PrivacyMode::CloudWithConsent
        ));

        assert!(!manager.is_consent_required(
            "email_summary",
            &AIProviderType::Ollama,
            &PrivacyMode::CloudWithConsent
        ));

        // Test consent recording
        manager.record_consent(
            "email_summary".to_string(),
            ConsentDecision::AllowAlways
        );

        assert_eq!(
            manager.is_operation_allowed("email_summary"),
            Some(true)
        );

        // Test that one-time decisions aren't cached
        manager.record_consent(
            "email_compose".to_string(),
            ConsentDecision::Allow
        );

        assert_eq!(
            manager.is_operation_allowed("email_compose"),
            None
        );
    }

    #[test]
    fn test_privacy_implications() {
        let state = AIPrivacyDialogState::new();

        let openai_implications = state.get_privacy_implications(&AIProviderType::OpenAI);
        assert!(!openai_implications.is_empty());
        assert!(openai_implications.iter().any(|s| s.contains("OpenAI")));

        let ollama_implications = state.get_privacy_implications(&AIProviderType::Ollama);
        assert!(ollama_implications.iter().any(|s| s.contains("locally")));
    }
}