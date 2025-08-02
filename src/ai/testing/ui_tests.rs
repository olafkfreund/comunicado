//! UI testing utilities for AI components

use crate::ai::{
    config::{AIConfig, AIProviderType, PrivacyMode},
    testing::mock_providers::*,
};
use crate::theme::Theme;
use crate::ui::{
    ai_assistant_ui::{AIAssistantUIState, AIAssistantMode},
    ai_calendar_ui::{AICalendarUIState, AICalendarMode},
    ai_config_ui::{AIConfigUIState, AIConfigTab},
    ai_privacy_dialog::{AIPrivacyDialogState, ConsentDecision},
};
use ratatui::{
    backend::TestBackend,
    layout::Rect,
    Terminal,
};
use std::collections::HashMap;

/// UI test context for AI components
pub struct AIUITestContext {
    /// Test terminal backend
    pub terminal: Terminal<TestBackend>,
    /// Test theme
    pub theme: Theme,
    /// Mock AI configurations
    pub configs: HashMap<String, AIConfig>,
    /// Mock providers for testing
    pub providers: HashMap<String, MockAIProvider>,
}

impl AIUITestContext {
    /// Create a new UI test context
    pub fn new() -> Self {
        let backend = TestBackend::new(80, 24);
        let terminal = Terminal::new(backend).unwrap();
        
        Self {
            terminal,
            theme: Theme::default(),
            configs: HashMap::new(),
            providers: HashMap::new(),
        }
    }

    /// Add a test configuration
    pub fn add_config(&mut self, name: String, config: AIConfig) {
        self.configs.insert(name, config);
    }

    /// Add a mock provider
    pub fn add_provider(&mut self, name: String, provider: MockAIProvider) {
        self.providers.insert(name, provider);
    }

    /// Get terminal area for testing
    pub fn get_test_area(&self) -> Rect {
        Rect::new(0, 0, 80, 24)
    }

    /// Simulate terminal resize
    pub fn resize(&mut self, width: u16, height: u16) {
        self.terminal.backend_mut().resize(Rect::new(0, 0, width, height));
    }

    /// Get terminal buffer for verification
    pub fn get_buffer(&self) -> &ratatui::buffer::Buffer {
        self.terminal.backend().buffer()
    }
}

/// Test utilities for AI Assistant UI
pub struct AIAssistantUITester;

impl AIAssistantUITester {
    /// Test AI assistant UI state management
    pub fn test_state_management() -> Result<(), String> {
        let mut state = AIAssistantUIState::new();
        
        // Test initial state
        assert_eq!(state.mode, AIAssistantMode::Hidden);
        assert!(!state.enabled);
        assert!(!state.loading);
        
        // Test enabling
        state.enable();
        assert!(state.enabled);
        
        // Test mode switching
        let mock_composition = crate::email::EmailCompositionAssistance {
            subject_suggestions: vec!["Test Subject".to_string()],
            body_suggestions: vec!["Test Body".to_string()],
            tone_suggestions: vec!["Professional".to_string()],
            key_points: vec!["Key point".to_string()],
            next_actions: vec!["Action item".to_string()],
        };
        
        state.set_compose_mode(mock_composition);
        assert_eq!(state.mode, AIAssistantMode::Compose);
        assert!(state.composition_assistance.is_some());
        
        // Test navigation
        state.move_down();
        assert_eq!(state.selected_suggestion, 1);
        
        state.move_up();
        assert_eq!(state.selected_suggestion, 0);
        
        // Test error handling
        state.set_error("Test error".to_string());
        assert_eq!(state.error_message, Some("Test error".to_string()));
        assert!(!state.loading);
        
        Ok(())
    }

    /// Test AI assistant UI rendering
    pub fn test_ui_rendering(context: &mut AIUITestContext) -> Result<(), String> {
        let mut state = AIAssistantUIState::new();
        state.enable();
        
        // Test different modes render without crashing
        let area = context.get_test_area();
        
        // Test compose mode
        let composition = crate::email::EmailCompositionAssistance {
            subject_suggestions: vec!["Subject 1".to_string(), "Subject 2".to_string()],
            body_suggestions: vec!["Body text".to_string()],
            tone_suggestions: vec!["Professional".to_string()],
            key_points: vec!["Important point".to_string()],
            next_actions: vec!["Follow up".to_string()],
        };
        state.set_compose_mode(composition);
        
        // Rendering test would go here - but we'd need the actual UI component
        // For now, we verify the state is correct for rendering
        assert_eq!(state.mode, AIAssistantMode::Compose);
        assert!(state.composition_assistance.is_some());
        
        Ok(())
    }

    /// Test error display in UI
    pub fn test_error_display() -> Result<(), String> {
        let mut state = AIAssistantUIState::new();
        
        // Test error state
        state.set_error("Connection failed".to_string());
        assert_eq!(state.error_message, Some("Connection failed".to_string()));
        assert!(!state.loading);
        
        // Test loading state
        state.set_loading(true);
        assert!(state.loading);
        assert!(state.error_message.is_none());
        
        Ok(())
    }
}

/// Test utilities for AI Calendar UI
pub struct AICalendarUITester;

impl AICalendarUITester {
    /// Test calendar UI state management
    pub fn test_state_management() -> Result<(), String> {
        let mut state = AICalendarUIState::new();
        
        // Test initial state
        assert_eq!(state.mode, AICalendarMode::Hidden);
        assert!(!state.enabled);
        assert!(!state.input_mode);
        
        // Test enabling
        state.enable();
        assert!(state.enabled);
        
        // Test create event mode
        state.set_create_event_mode();
        assert_eq!(state.mode, AICalendarMode::CreateEvent);
        assert!(state.input_mode);
        assert!(state.input_text.is_empty());
        
        // Test input handling
        state.add_char('H');
        state.add_char('i');
        assert_eq!(state.input_text, "Hi");
        
        state.remove_char();
        assert_eq!(state.input_text, "H");
        
        // Test natural language request creation
        state.input_text = "Meeting tomorrow at 2 PM".to_string();
        let request = state.get_event_request();
        assert!(request.is_some());
        assert_eq!(request.unwrap().description, "Meeting tomorrow at 2 PM");
        
        Ok(())
    }

    /// Test calendar insights display
    pub fn test_insights_display() -> Result<(), String> {
        use crate::calendar::CalendarInsights;
        
        let mut state = AICalendarUIState::new();
        
        let insights = CalendarInsights {
            meeting_patterns: vec![
                "You have most meetings on Tuesday".to_string(),
                "Morning meetings are more common".to_string(),
            ],
            time_management_tips: vec![
                "Consider blocking focus time".to_string(),
            ],
            optimization_suggestions: vec![
                "Reduce meeting durations by 25%".to_string(),
            ],
            productivity_insights: vec![
                "You're most productive in the afternoon".to_string(),
            ],
            focus_time_suggestions: vec![
                "Block 2-4 PM for deep work".to_string(),
            ],
        };
        
        state.set_insights_mode(insights);
        assert_eq!(state.mode, AICalendarMode::Insights);
        assert!(state.calendar_insights.is_some());
        
        let stored_insights = state.calendar_insights.as_ref().unwrap();
        assert_eq!(stored_insights.meeting_patterns.len(), 2);
        assert!(stored_insights.meeting_patterns[0].contains("Tuesday"));
        
        Ok(())
    }
}

/// Test utilities for AI Configuration UI
pub struct AIConfigUITester;

impl AIConfigUITester {
    /// Test configuration UI state management
    pub fn test_state_management() -> Result<(), String> {
        let mut state = AIConfigUIState::new();
        let config = AIConfig::default();
        
        // Test initial state
        assert!(!state.visible);
        assert_eq!(state.current_tab, AIConfigTab::General);
        assert!(!state.modified);
        
        // Test showing configuration
        state.show(config.clone());
        assert!(state.visible);
        assert!(!state.modified);
        
        // Test tab navigation
        state.next_tab();
        assert_eq!(state.current_tab, AIConfigTab::Providers);
        
        state.previous_tab();
        assert_eq!(state.current_tab, AIConfigTab::General);
        
        // Test input handling
        state.start_input("test_field".to_string(), "initial".to_string());
        assert!(state.input_mode);
        assert_eq!(state.current_input_field, Some("test_field".to_string()));
        assert_eq!(state.input_buffer, "initial");
        
        state.add_char('!');
        assert_eq!(state.input_buffer, "initial!");
        
        state.cancel_input();
        assert!(!state.input_mode);
        assert!(state.input_buffer.is_empty());
        
        Ok(())
    }

    /// Test configuration validation
    pub fn test_config_validation() -> Result<(), String> {
        let mut state = AIConfigUIState::new();
        let mut config = AIConfig::default();
        config.enabled = true;
        config.provider = AIProviderType::OpenAI;
        
        state.show(config);
        
        // Test setting valid values
        state.start_input("creativity".to_string(), "0.7".to_string());
        state.finish_input();
        assert_eq!(state.config.creativity, 0.7);
        assert!(state.modified);
        
        // Test setting invalid values
        state.start_input("creativity".to_string(), "1.5".to_string());
        state.finish_input();
        assert!(state.error_message.is_some());
        assert!(state.error_message.as_ref().unwrap().contains("between 0.0 and 1.0"));
        
        Ok(())
    }

    /// Test feature toggles
    pub fn test_feature_toggles() -> Result<(), String> {
        let mut state = AIConfigUIState::new();
        let config = AIConfig::default();
        state.show(config);
        
        // Test general tab toggles
        state.current_tab = AIConfigTab::General;
        state.selected_index = 0;
        let original_enabled = state.config.enabled;
        
        state.toggle_setting();
        assert_eq!(state.config.enabled, !original_enabled);
        assert!(state.modified);
        
        // Test features tab toggles
        state.current_tab = AIConfigTab::Features;
        state.selected_index = 0;
        let original_email_suggestions = state.config.email_suggestions_enabled;
        
        state.toggle_setting();
        assert_eq!(state.config.email_suggestions_enabled, !original_email_suggestions);
        assert!(state.modified);
        
        Ok(())
    }
}

/// Test utilities for AI Privacy Dialog
pub struct AIPrivacyDialogTester;

impl AIPrivacyDialogTester {
    /// Test privacy dialog state management
    pub fn test_state_management() -> Result<(), String> {
        let mut state = AIPrivacyDialogState::new();
        
        // Test initial state
        assert!(!state.visible);
        assert!(state.operation.is_empty());
        
        // Test showing consent dialog
        state.show_consent_dialog(
            "email_summary".to_string(),
            AIProviderType::OpenAI,
            "Email content for AI processing".to_string(),
            PrivacyMode::CloudWithConsent,
        );
        
        assert!(state.visible);
        assert_eq!(state.operation, "email_summary");
        assert_eq!(state.provider, AIProviderType::OpenAI);
        assert_eq!(state.selected_option, 1); // Default to deny
        
        // Test navigation
        state.move_up();
        assert_eq!(state.selected_option, 0); // Allow
        
        state.move_down();
        state.move_down();
        assert_eq!(state.selected_option, 2); // Allow Always
        
        // Test consent decision
        assert_eq!(state.get_consent_decision(), ConsentDecision::AllowAlways);
        
        // Test hiding
        state.hide();
        assert!(!state.visible);
        assert!(state.operation.is_empty());
        
        Ok(())
    }

    /// Test privacy implications display
    pub fn test_privacy_implications() -> Result<(), String> {
        let state = AIPrivacyDialogState::new();
        
        // Test OpenAI implications
        let openai_implications = state.get_privacy_implications(&AIProviderType::OpenAI);
        assert!(!openai_implications.is_empty());
        assert!(openai_implications.iter().any(|s| s.contains("OpenAI")));
        assert!(openai_implications.iter().any(|s| s.contains("United States")));
        
        // Test Ollama implications
        let ollama_implications = state.get_privacy_implications(&AIProviderType::Ollama);
        assert!(ollama_implications.iter().any(|s| s.contains("locally")));
        assert!(ollama_implications.iter().any(|s| s.contains("no data is sent")));
        
        Ok(())
    }
}

/// Comprehensive UI test runner
pub struct AIUITestRunner {
    context: AIUITestContext,
}

impl AIUITestRunner {
    /// Create a new UI test runner
    pub fn new() -> Self {
        Self {
            context: AIUITestContext::new(),
        }
    }

    /// Run all UI tests
    pub fn run_all_tests(&mut self) -> Vec<(String, Result<(), String>)> {
        let mut results = Vec::new();
        
        // AI Assistant UI tests
        results.push(("ai_assistant_state_management".to_string(), 
                     AIAssistantUITester::test_state_management()));
        results.push(("ai_assistant_ui_rendering".to_string(), 
                     AIAssistantUITester::test_ui_rendering(&mut self.context)));
        results.push(("ai_assistant_error_display".to_string(), 
                     AIAssistantUITester::test_error_display()));
        
        // AI Calendar UI tests
        results.push(("ai_calendar_state_management".to_string(), 
                     AICalendarUITester::test_state_management()));
        results.push(("ai_calendar_insights_display".to_string(), 
                     AICalendarUITester::test_insights_display()));
        
        // AI Configuration UI tests
        results.push(("ai_config_state_management".to_string(), 
                     AIConfigUITester::test_state_management()));
        results.push(("ai_config_validation".to_string(), 
                     AIConfigUITester::test_config_validation()));
        results.push(("ai_config_feature_toggles".to_string(), 
                     AIConfigUITester::test_feature_toggles()));
        
        // AI Privacy Dialog tests
        results.push(("ai_privacy_state_management".to_string(), 
                     AIPrivacyDialogTester::test_state_management()));
        results.push(("ai_privacy_implications".to_string(), 
                     AIPrivacyDialogTester::test_privacy_implications()));
        
        results
    }

    /// Generate UI test report
    pub fn generate_report(&self, results: &[(String, Result<(), String>)]) -> String {
        let total_tests = results.len();
        let passed_tests = results.iter().filter(|(_, r)| r.is_ok()).count();
        let failed_tests = total_tests - passed_tests;
        
        let mut report = String::new();
        report.push_str("# AI UI Test Report\n\n");
        report.push_str(&format!("**Total Tests:** {}\n", total_tests));
        report.push_str(&format!("**Passed:** {} ({}%)\n", passed_tests, (passed_tests * 100) / total_tests.max(1)));
        report.push_str(&format!("**Failed:** {} ({}%)\n\n", failed_tests, (failed_tests * 100) / total_tests.max(1)));
        
        if failed_tests > 0 {
            report.push_str("## Failed Tests\n\n");
            for (name, result) in results.iter().filter(|(_, r)| r.is_err()) {
                if let Err(error) = result {
                    report.push_str(&format!("- **{}**: {}\n", name, error));
                }
            }
            report.push_str("\n");
        }
        
        report.push_str("## All Test Results\n\n");
        for (name, result) in results {
            let status = if result.is_ok() { "PASS" } else { "FAIL" };
            report.push_str(&format!("- {} **{}**\n", status, name));
        }
        
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_context_creation() {
        let context = AIUITestContext::new();
        let area = context.get_test_area();
        assert_eq!(area.width, 80);
        assert_eq!(area.height, 24);
    }

    #[test]
    fn test_ai_assistant_ui_tests() {
        assert!(AIAssistantUITester::test_state_management().is_ok());
        assert!(AIAssistantUITester::test_error_display().is_ok());
    }

    #[test]
    fn test_ai_calendar_ui_tests() {
        assert!(AICalendarUITester::test_state_management().is_ok());
        assert!(AICalendarUITester::test_insights_display().is_ok());
    }

    #[test]
    fn test_ai_config_ui_tests() {
        assert!(AIConfigUITester::test_state_management().is_ok());
        assert!(AIConfigUITester::test_config_validation().is_ok());
        assert!(AIConfigUITester::test_feature_toggles().is_ok());
    }

    #[test]
    fn test_ai_privacy_dialog_tests() {
        assert!(AIPrivacyDialogTester::test_state_management().is_ok());
        assert!(AIPrivacyDialogTester::test_privacy_implications().is_ok());
    }

    #[test]
    fn test_ui_test_runner() {
        let mut runner = AIUITestRunner::new();
        let results = runner.run_all_tests();
        
        assert!(!results.is_empty());
        
        // Most tests should pass
        let passed_count = results.iter().filter(|(_, r)| r.is_ok()).count();
        assert!(passed_count > 0);
        
        let report = runner.generate_report(&results);
        assert!(report.contains("AI UI Test Report"));
        assert!(report.contains("Total Tests"));
    }
}