//! AI configuration UI components for settings management

use crate::ai::config::{AIConfig, AIProviderType, PrivacyMode};
use crate::theme::Theme;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs, Wrap},
    Frame,
};

/// AI configuration UI state
#[derive(Debug, Clone)]
pub struct AIConfigUIState {
    /// Whether the AI config UI is visible
    pub visible: bool,
    /// Current configuration tab
    pub current_tab: AIConfigTab,
    /// AI configuration being edited
    pub config: AIConfig,
    /// Whether configuration has been modified
    pub modified: bool,
    /// Current selection in lists
    pub selected_index: usize,
    /// List state for navigation
    pub list_state: ListState,
    /// Input mode for text fields
    pub input_mode: bool,
    /// Current input field being edited
    pub current_input_field: Option<String>,
    /// Temporary input buffer
    pub input_buffer: String,
    /// Error message if any
    pub error_message: Option<String>,
    /// Success message if any
    pub success_message: Option<String>,
    /// Show confirmation dialog
    pub show_confirmation: bool,
    /// Confirmation message
    pub confirmation_message: String,
}

/// AI configuration tabs
#[derive(Debug, Clone, PartialEq)]
pub enum AIConfigTab {
    /// General AI settings
    General,
    /// Provider configuration
    Providers,
    /// Privacy and consent settings
    Privacy,
    /// Feature-specific settings
    Features,
    /// Advanced configuration
    Advanced,
}

impl Default for AIConfigUIState {
    fn default() -> Self {
        Self {
            visible: false,
            current_tab: AIConfigTab::General,
            config: AIConfig::default(),
            modified: false,
            selected_index: 0,
            list_state: ListState::default(),
            input_mode: false,
            current_input_field: None,
            input_buffer: String::new(),
            error_message: None,
            success_message: None,
            show_confirmation: false,
            confirmation_message: String::new(),
        }
    }
}

impl AIConfigUIState {
    /// Create new AI config UI state
    pub fn new() -> Self {
        Self::default()
    }

    /// Show the AI configuration UI
    pub fn show(&mut self, config: AIConfig) {
        self.visible = true;
        self.config = config;
        self.modified = false;
        self.clear_messages();
    }

    /// Hide the AI configuration UI
    pub fn hide(&mut self) {
        self.visible = false;
        self.input_mode = false;
        self.current_input_field = None;
        self.input_buffer.clear();
        self.clear_messages();
        self.show_confirmation = false;
    }

    /// Clear all messages
    pub fn clear_messages(&mut self) {
        self.error_message = None;
        self.success_message = None;
    }

    /// Set error message
    pub fn set_error(&mut self, message: String) {
        self.error_message = Some(message);
        self.success_message = None;
    }

    /// Set success message
    pub fn set_success(&mut self, message: String) {
        self.success_message = Some(message);
        self.error_message = None;
    }

    /// Show confirmation dialog
    pub fn show_confirmation_dialog(&mut self, message: String) {
        self.confirmation_message = message;
        self.show_confirmation = true;
    }

    /// Hide confirmation dialog
    pub fn hide_confirmation_dialog(&mut self) {
        self.show_confirmation = false;
        self.confirmation_message.clear();
    }

    /// Move to next tab
    pub fn next_tab(&mut self) {
        self.current_tab = match self.current_tab {
            AIConfigTab::General => AIConfigTab::Providers,
            AIConfigTab::Providers => AIConfigTab::Privacy,
            AIConfigTab::Privacy => AIConfigTab::Features,
            AIConfigTab::Features => AIConfigTab::Advanced,
            AIConfigTab::Advanced => AIConfigTab::General,
        };
        self.selected_index = 0;
        self.list_state.select(Some(0));
    }

    /// Move to previous tab
    pub fn previous_tab(&mut self) {
        self.current_tab = match self.current_tab {
            AIConfigTab::General => AIConfigTab::Advanced,
            AIConfigTab::Providers => AIConfigTab::General,
            AIConfigTab::Privacy => AIConfigTab::Providers,
            AIConfigTab::Features => AIConfigTab::Privacy,
            AIConfigTab::Advanced => AIConfigTab::Features,
        };
        self.selected_index = 0;
        self.list_state.select(Some(0));
    }

    /// Move selection up
    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.list_state.select(Some(self.selected_index));
        }
    }

    /// Move selection down
    pub fn move_down(&mut self) {
        let max_items = self.get_max_items_for_current_tab();
        if self.selected_index < max_items.saturating_sub(1) {
            self.selected_index += 1;
            self.list_state.select(Some(self.selected_index));
        }
    }

    /// Get maximum items for current tab
    fn get_max_items_for_current_tab(&self) -> usize {
        match self.current_tab {
            AIConfigTab::General => 3, // Enable/Disable, Provider, Model
            AIConfigTab::Providers => 5, // Ollama, OpenAI, Anthropic, Google, None
            AIConfigTab::Privacy => 4, // LocalOnly, LocalPreferred, CloudWithConsent, CloudAllowed
            AIConfigTab::Features => 4, // Email suggestions, summarization, calendar, categorization
            AIConfigTab::Advanced => 5, // Creativity, timeout, retries, cache, context length
        }
    }

    /// Start editing a field
    pub fn start_input(&mut self, field: String, current_value: String) {
        self.input_mode = true;
        self.current_input_field = Some(field);
        self.input_buffer = current_value;
    }

    /// Stop editing and save the value
    pub fn finish_input(&mut self) {
        if let Some(field) = &self.current_input_field {
            match field.as_str() {
                "ollama_endpoint" => {
                    self.config.ollama_endpoint = self.input_buffer.clone();
                    self.modified = true;
                },
                "local_model" => {
                    self.config.local_model = if self.input_buffer.is_empty() {
                        None
                    } else {
                        Some(self.input_buffer.clone())
                    };
                    self.modified = true;
                },
                "openai_api_key" => {
                    if !self.input_buffer.is_empty() {
                        self.config.set_api_key("openai".to_string(), self.input_buffer.clone());
                        self.modified = true;
                    }
                },
                "anthropic_api_key" => {
                    if !self.input_buffer.is_empty() {
                        self.config.set_api_key("anthropic".to_string(), self.input_buffer.clone());
                        self.modified = true;
                    }
                },
                "google_api_key" => {
                    if !self.input_buffer.is_empty() {
                        self.config.set_api_key("google".to_string(), self.input_buffer.clone());
                        self.modified = true;
                    }
                },
                "creativity" => {
                    if let Ok(value) = self.input_buffer.parse::<f32>() {
                        if (0.0..=1.0).contains(&value) {
                            self.config.creativity = value;
                            self.modified = true;
                        } else {
                            self.set_error("Creativity must be between 0.0 and 1.0".to_string());
                        }
                    } else {
                        self.set_error("Invalid creativity value".to_string());
                    }
                },
                "max_context_length" => {
                    if let Ok(value) = self.input_buffer.parse::<usize>() {
                        if value > 0 {
                            self.config.max_context_length = value;
                            self.modified = true;
                        } else {
                            self.set_error("Context length must be greater than 0".to_string());
                        }
                    } else {
                        self.set_error("Invalid context length value".to_string());
                    }
                },
                "max_retries" => {
                    if let Ok(value) = self.input_buffer.parse::<u32>() {
                        self.config.max_retries = value;
                        self.modified = true;
                    } else {
                        self.set_error("Invalid retry count value".to_string());
                    }
                },
                _ => {}
            }
        }
        
        self.input_mode = false;
        self.current_input_field = None;
        self.input_buffer.clear();
    }

    /// Cancel input editing
    pub fn cancel_input(&mut self) {
        self.input_mode = false;
        self.current_input_field = None;
        self.input_buffer.clear();
    }

    /// Add character to input buffer
    pub fn add_char(&mut self, c: char) {
        if self.input_mode {
            self.input_buffer.push(c);
        }
    }

    /// Remove character from input buffer
    pub fn remove_char(&mut self) {
        if self.input_mode {
            self.input_buffer.pop();
        }
    }

    /// Toggle a boolean setting
    pub fn toggle_setting(&mut self) {
        match self.current_tab {
            AIConfigTab::General => {
                match self.selected_index {
                    0 => {
                        self.config.enabled = !self.config.enabled;
                        self.modified = true;
                    },
                    _ => {}
                }
            },
            AIConfigTab::Providers => {
                let provider = match self.selected_index {
                    0 => AIProviderType::Ollama,
                    1 => AIProviderType::OpenAI,
                    2 => AIProviderType::Anthropic,
                    3 => AIProviderType::Google,
                    4 => AIProviderType::None,
                    _ => return,
                };
                self.config.provider = provider;
                self.modified = true;
            },
            AIConfigTab::Privacy => {
                let privacy_mode = match self.selected_index {
                    0 => PrivacyMode::LocalOnly,
                    1 => PrivacyMode::LocalPreferred,
                    2 => PrivacyMode::CloudWithConsent,
                    3 => PrivacyMode::CloudAllowed,
                    _ => return,
                };
                self.config.privacy_mode = privacy_mode;
                self.modified = true;
            },
            AIConfigTab::Features => {
                match self.selected_index {
                    0 => {
                        self.config.email_suggestions_enabled = !self.config.email_suggestions_enabled;
                        self.modified = true;
                    },
                    1 => {
                        self.config.email_summarization_enabled = !self.config.email_summarization_enabled;
                        self.modified = true;
                    },
                    2 => {
                        self.config.calendar_assistance_enabled = !self.config.calendar_assistance_enabled;
                        self.modified = true;
                    },
                    3 => {
                        self.config.email_categorization_enabled = !self.config.email_categorization_enabled;
                        self.modified = true;
                    },
                    _ => {}
                }
            },
            AIConfigTab::Advanced => {
                match self.selected_index {
                    3 => {
                        self.config.cache_responses = !self.config.cache_responses;
                        self.modified = true;
                    },
                    _ => {}
                }
            },
        }
    }

    /// Get the current configuration
    pub fn get_config(&self) -> &AIConfig {
        &self.config
    }

    /// Check if configuration has been modified
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Mark configuration as saved
    pub fn mark_saved(&mut self) {
        self.modified = false;
    }
}

/// AI configuration UI component
pub struct AIConfigUI;

impl AIConfigUI {
    /// Create new AI configuration UI
    pub fn new() -> Self {
        Self
    }

    /// Render AI configuration UI
    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &mut AIConfigUIState,
        theme: &Theme,
    ) {
        if !state.visible {
            return;
        }

        // Clear the area
        frame.render_widget(Clear, area);

        // Main container
        let block = Block::default()
            .title(" AI Configuration ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.ai_assistant_border()));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Handle confirmation dialog
        if state.show_confirmation {
            self.render_confirmation_dialog(frame, area, state, theme);
            return;
        }

        // Layout: tabs, content, status bar
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Tabs
                Constraint::Min(10),   // Content
                Constraint::Length(3), // Status bar
            ])
            .split(inner);

        // Render tabs
        self.render_tabs(frame, chunks[0], state, theme);

        // Render current tab content
        match state.current_tab {
            AIConfigTab::General => self.render_general_tab(frame, chunks[1], state, theme),
            AIConfigTab::Providers => self.render_providers_tab(frame, chunks[1], state, theme),
            AIConfigTab::Privacy => self.render_privacy_tab(frame, chunks[1], state, theme),
            AIConfigTab::Features => self.render_features_tab(frame, chunks[1], state, theme),
            AIConfigTab::Advanced => self.render_advanced_tab(frame, chunks[1], state, theme),
        }

        // Render status bar
        self.render_status_bar(frame, chunks[2], state, theme);
    }

    /// Render tab navigation
    fn render_tabs(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &AIConfigUIState,
        theme: &Theme,
    ) {
        let tab_titles = vec!["General", "Providers", "Privacy", "Features", "Advanced"];
        let selected_tab = match state.current_tab {
            AIConfigTab::General => 0,
            AIConfigTab::Providers => 1,
            AIConfigTab::Privacy => 2,  
            AIConfigTab::Features => 3,
            AIConfigTab::Advanced => 4,
        };

        let tabs = Tabs::new(tab_titles)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(theme.ai_assistant_text()))
            .highlight_style(Style::default().fg(theme.ai_assistant_selected()).add_modifier(Modifier::BOLD))
            .select(selected_tab);

        frame.render_widget(tabs, area);
    }

    /// Render general settings tab
    fn render_general_tab(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &AIConfigUIState,
        theme: &Theme,
    ) {
        let items = vec![
            self.create_bool_item("AI Enabled", state.config.enabled, 0 == state.selected_index, theme),
            self.create_text_item(
                "Current Provider", 
                &state.config.provider.to_string(), 
                1 == state.selected_index, 
                theme
            ),
            self.create_text_item(
                "Local Model", 
                state.config.local_model.as_deref().unwrap_or("Not set"), 
                2 == state.selected_index, 
                theme
            ),
        ];

        let list = List::new(items)
            .block(Block::default().title("General Settings").borders(Borders::ALL))
            .style(Style::default().fg(theme.ai_assistant_text()));

        frame.render_stateful_widget(list, area, &mut state.list_state.clone());
    }

    /// Render providers configuration tab
    fn render_providers_tab(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &AIConfigUIState,
        theme: &Theme,
    ) {
        let providers = vec![
            ("Ollama (Local)", AIProviderType::Ollama),
            ("OpenAI", AIProviderType::OpenAI),
            ("Anthropic", AIProviderType::Anthropic),
            ("Google", AIProviderType::Google),
            ("Disabled", AIProviderType::None),
        ];

        let items: Vec<ListItem> = providers
            .iter()
            .enumerate()
            .map(|(i, (name, provider_type))| {
                let is_selected = i == state.selected_index;
                let is_current = *provider_type == state.config.provider;
                
                let mut spans = vec![
                    Span::styled(
                        if is_current { "● " } else { "○ " },
                        Style::default().fg(if is_current { 
                            theme.ai_assistant_selected() 
                        } else { 
                            theme.ai_assistant_text() 
                        })
                    ),
                    Span::styled(
                        *name,
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
                ];

                // Add status information
                match provider_type {
                    AIProviderType::Ollama => {
                        spans.push(Span::styled(
                            format!(" ({})", state.config.ollama_endpoint),
                            Style::default().fg(theme.ai_assistant_context())
                        ));
                    },
                    AIProviderType::OpenAI => {
                        let has_key = state.config.get_api_key("openai").is_some();
                        spans.push(Span::styled(
                            if has_key { " ✓" } else { " ✗" },
                            Style::default().fg(if has_key { 
                                Color::Green 
                            } else { 
                                Color::Red 
                            })
                        ));
                    },
                    AIProviderType::Anthropic => {
                        let has_key = state.config.get_api_key("anthropic").is_some();
                        spans.push(Span::styled(
                            if has_key { " ✓" } else { " ✗" },
                            Style::default().fg(if has_key { 
                                Color::Green 
                            } else { 
                                Color::Red 
                            })
                        ));
                    },
                    AIProviderType::Google => {
                        let has_key = state.config.get_api_key("google").is_some();
                        spans.push(Span::styled(
                            if has_key { " ✓" } else { " ✗" },
                            Style::default().fg(if has_key { 
                                Color::Green 
                            } else { 
                                Color::Red 
                            })
                        ));
                    },
                    AIProviderType::None => {},
                }

                ListItem::new(Line::from(spans))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().title("AI Providers").borders(Borders::ALL))
            .style(Style::default().fg(theme.ai_assistant_text()));

        frame.render_stateful_widget(list, area, &mut state.list_state.clone());
    }

    /// Render privacy settings tab
    fn render_privacy_tab(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &AIConfigUIState,
        theme: &Theme,
    ) {
        let privacy_modes = vec![
            ("Local Only", PrivacyMode::LocalOnly, "Only use local AI processing"),
            ("Local Preferred", PrivacyMode::LocalPreferred, "Prefer local, fallback to cloud"),
            ("Cloud with Consent", PrivacyMode::CloudWithConsent, "Allow cloud with explicit consent"),
            ("Cloud Allowed", PrivacyMode::CloudAllowed, "Allow cloud processing freely"),
        ];

        let items: Vec<ListItem> = privacy_modes
            .iter()
            .enumerate()
            .map(|(i, (name, mode, description))| {
                let is_selected = i == state.selected_index;
                let is_current = *mode == state.config.privacy_mode;
                
                let spans = vec![
                    Span::styled(
                        if is_current { "● " } else { "○ " },
                        Style::default().fg(if is_current { 
                            theme.ai_assistant_selected() 
                        } else { 
                            theme.ai_assistant_text() 
                        })
                    ),
                    Span::styled(
                        *name,
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
                        format!(" - {}", description),
                        Style::default().fg(theme.ai_assistant_context())
                    ),
                ];

                ListItem::new(Line::from(spans))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().title("Privacy & Data Handling").borders(Borders::ALL))
            .style(Style::default().fg(theme.ai_assistant_text()));

        frame.render_stateful_widget(list, area, &mut state.list_state.clone());
    }

    /// Render features settings tab
    fn render_features_tab(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &AIConfigUIState,
        theme: &Theme,
    ) {
        let features = vec![
            ("Email Suggestions", state.config.email_suggestions_enabled),
            ("Email Summarization", state.config.email_summarization_enabled),
            ("Calendar Assistance", state.config.calendar_assistance_enabled),
            ("Email Categorization", state.config.email_categorization_enabled),
        ];

        let items: Vec<ListItem> = features
            .iter()
            .enumerate()
            .map(|(i, (name, enabled))| {
                self.create_bool_item(name, *enabled, i == state.selected_index, theme)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().title("AI Features").borders(Borders::ALL))
            .style(Style::default().fg(theme.ai_assistant_text()));

        frame.render_stateful_widget(list, area, &mut state.list_state.clone());
    }

    /// Render advanced settings tab
    fn render_advanced_tab(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &AIConfigUIState,
        theme: &Theme,
    ) {
        let items = vec![
            self.create_text_item(
                "Creativity",
                &format!("{:.1}", state.config.creativity),
                0 == state.selected_index,
                theme
            ),
            self.create_text_item(
                "Request Timeout",
                &format!("{}s", state.config.request_timeout.as_secs()),
                1 == state.selected_index,
                theme
            ),
            self.create_text_item(
                "Max Retries",
                &state.config.max_retries.to_string(),
                2 == state.selected_index,
                theme
            ),
            self.create_bool_item(
                "Cache Responses",
                state.config.cache_responses,
                3 == state.selected_index,
                theme
            ),
            self.create_text_item(
                "Max Context Length",
                &state.config.max_context_length.to_string(),
                4 == state.selected_index,
                theme
            ),
        ];

        let list = List::new(items)
            .block(Block::default().title("Advanced Settings").borders(Borders::ALL))
            .style(Style::default().fg(theme.ai_assistant_text()));

        frame.render_stateful_widget(list, area, &mut state.list_state.clone());
    }

    /// Render status bar
    fn render_status_bar(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &AIConfigUIState,
        theme: &Theme,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Left side: messages
        if let Some(error) = &state.error_message {
            let error_widget = Paragraph::new(format!("❌ {}", error))
                .style(Style::default().fg(theme.ai_assistant_error()))
                .alignment(Alignment::Left);
            frame.render_widget(error_widget, chunks[0]);
        } else if let Some(success) = &state.success_message {
            let success_widget = Paragraph::new(format!("✅ {}", success))
                .style(Style::default().fg(Color::Green))
                .alignment(Alignment::Left);
            frame.render_widget(success_widget, chunks[0]);
        } else if state.input_mode {
            let input_text = if let Some(field) = &state.current_input_field {
                format!("Editing {}: {}", field, state.input_buffer)
            } else {
                "Input mode".to_string()
            };
            let input_widget = Paragraph::new(input_text)
                .style(Style::default().fg(theme.ai_assistant_selected()))
                .alignment(Alignment::Left);
            frame.render_widget(input_widget, chunks[0]);
        }

        // Right side: help text
        let help_text = if state.show_confirmation {
            "y: Confirm • n: Cancel"
        } else if state.input_mode {
            "Enter: Save • Esc: Cancel • Type to edit"
        } else {
            "Tab: Switch tabs • ↑/↓: Navigate • Enter: Edit/Toggle • s: Save • q: Quit"
        };

        let help_widget = Paragraph::new(help_text)
            .style(Style::default().fg(theme.ai_assistant_help()))
            .alignment(Alignment::Right);
        frame.render_widget(help_widget, chunks[1]);

        // Show modified indicator
        if state.modified {
            let modified_text = " [Modified]";
            let modified_widget = Paragraph::new(modified_text)
                .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Right);
            
            // Render in the bottom right corner
            let modified_area = Rect {
                x: chunks[1].x + chunks[1].width.saturating_sub(modified_text.len() as u16),
                y: chunks[1].y,
                width: modified_text.len() as u16,
                height: 1,
            };
            frame.render_widget(modified_widget, modified_area);
        }
    }

    /// Render confirmation dialog
    fn render_confirmation_dialog(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &AIConfigUIState,
        theme: &Theme,
    ) {
        // Center the dialog
        let dialog_area = self.centered_rect(50, 20, area);
        
        // Clear the area
        frame.render_widget(Clear, dialog_area);

        let block = Block::default()
            .title(" Confirmation ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        let inner = block.inner(dialog_area);
        frame.render_widget(block, dialog_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(1)])
            .split(inner);

        // Message
        let message_widget = Paragraph::new(state.confirmation_message.clone())
            .style(Style::default().fg(theme.ai_assistant_text()))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        frame.render_widget(message_widget, chunks[0]);

        // Instructions
        let instructions = Paragraph::new("y: Yes • n: No")
            .style(Style::default().fg(theme.ai_assistant_help()))
            .alignment(Alignment::Center);
        frame.render_widget(instructions, chunks[1]);
    }

    /// Create a boolean setting list item
    fn create_bool_item(
        &self,
        name: &str,
        value: bool,
        selected: bool,
        theme: &Theme,
    ) -> ListItem {
        let spans = vec![
            Span::styled(
                format!("{}: ", name),
                Style::default().fg(if selected {
                    theme.ai_assistant_selected()
                } else {
                    theme.ai_assistant_text()
                }).add_modifier(if selected {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                })
            ),
            Span::styled(
                if value { "Enabled" } else { "Disabled" },
                Style::default().fg(if value {
                    Color::Green
                } else {
                    Color::Red
                })
            ),
        ];

        ListItem::new(Line::from(spans))
    }

    /// Create a text setting list item
    fn create_text_item(
        &self,
        name: &str,
        value: &str,
        selected: bool,
        theme: &Theme,
    ) -> ListItem<'static> {
        let spans = vec![
            Span::styled(
                format!("{}: ", name),
                Style::default().fg(if selected {
                    theme.ai_assistant_selected()
                } else {
                    theme.ai_assistant_text()
                }).add_modifier(if selected {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                })
            ),
            Span::styled(
                value.to_string(),
                Style::default().fg(theme.ai_assistant_context())
            ),
        ];

        ListItem::new(Line::from(spans))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_config_ui_state() {
        let mut state = AIConfigUIState::new();
        assert!(!state.visible);
        assert_eq!(state.current_tab, AIConfigTab::General);
        assert!(!state.modified);

        let config = AIConfig::default();
        state.show(config);
        assert!(state.visible);
        assert!(!state.modified);

        state.hide();
        assert!(!state.visible);
    }

    #[test]
    fn test_tab_navigation() {
        let mut state = AIConfigUIState::new();
        assert_eq!(state.current_tab, AIConfigTab::General);

        state.next_tab();
        assert_eq!(state.current_tab, AIConfigTab::Providers);

        state.next_tab();
        assert_eq!(state.current_tab, AIConfigTab::Privacy);

        state.previous_tab();
        assert_eq!(state.current_tab, AIConfigTab::Providers);

        state.previous_tab();
        assert_eq!(state.current_tab, AIConfigTab::General);
    }

    #[test]
    fn test_input_handling() {
        let mut state = AIConfigUIState::new();
        assert!(!state.input_mode);

        state.start_input("test_field".to_string(), "initial_value".to_string());
        assert!(state.input_mode);
        assert_eq!(state.current_input_field, Some("test_field".to_string()));
        assert_eq!(state.input_buffer, "initial_value");

        state.add_char('x');
        assert_eq!(state.input_buffer, "initial_valuex");

        state.remove_char();
        assert_eq!(state.input_buffer, "initial_value");

        state.cancel_input();
        assert!(!state.input_mode);
        assert_eq!(state.current_input_field, None);
        assert!(state.input_buffer.is_empty());
    }

    #[test]
    fn test_setting_toggles() {
        let mut state = AIConfigUIState::new();
        let original_enabled = state.config.enabled;

        state.current_tab = AIConfigTab::General;
        state.selected_index = 0;
        state.toggle_setting();
        
        assert_eq!(state.config.enabled, !original_enabled);
        assert!(state.modified);
    }
}