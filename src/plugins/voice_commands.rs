//! Voice Commands Plugin for AI Integration
//! 
//! This plugin provides voice control for AI operations using speech recognition.
//! It integrates with the existing AI service to enable hands-free email and calendar management.

use crate::ai::{AIService, EmailTriageConfig};
use crate::email::StoredMessage;
use crate::plugins::{Plugin, PluginContext, PluginResult, PluginError, PluginMetadata, PluginCapability};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Voice command configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceCommandConfig {
    /// Enable voice commands
    pub enabled: bool,
    /// Speech recognition language
    pub language: String,
    /// Voice activation keywords
    pub activation_keywords: Vec<String>,
    /// Microphone device (None for default)
    pub microphone_device: Option<String>,
    /// Speech confidence threshold (0.0 to 1.0)
    pub confidence_threshold: f32,
    /// Enable continuous listening mode
    pub continuous_listening: bool,
    /// Timeout for voice commands in seconds
    pub command_timeout: u32,
    /// Enable voice feedback responses
    pub voice_feedback: bool,
}

impl Default for VoiceCommandConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Disabled by default since it's a plugin
            language: "en-US".to_string(),
            activation_keywords: vec![
                "comunicado".to_string(),
                "email assistant".to_string(),
                "ai assistant".to_string(),
            ],
            microphone_device: None,
            confidence_threshold: 0.7,
            continuous_listening: false,
            command_timeout: 30,
            voice_feedback: true,
        }
    }
}

/// Voice command types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VoiceCommand {
    /// Email-related commands
    Email {
        action: EmailVoiceAction,
        target: Option<String>,
        content: Option<String>,
    },
    /// Calendar-related commands
    Calendar {
        action: CalendarVoiceAction,
        details: Option<String>,
    },
    /// AI assistant commands
    Assistant {
        action: AssistantVoiceAction,
        query: Option<String>,
    },
    /// Navigation commands
    Navigation {
        action: NavigationVoiceAction,
        target: Option<String>,
    },
}

/// Email voice actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmailVoiceAction {
    Compose,
    Reply,
    Forward,
    Delete,
    Archive,
    MarkRead,
    MarkUnread,
    Search,
    Summarize,
    Triage,
}

/// Calendar voice actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CalendarVoiceAction {
    CreateEvent,
    ViewCalendar,
    NextWeek,
    PreviousWeek,
    Today,
    Schedule,
}

/// AI assistant voice actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssistantVoiceAction {
    Help,
    Summarize,
    Analyze,
    Suggest,
    Configure,
}

/// Navigation voice actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NavigationVoiceAction {
    GoToInbox,
    GoToSent,
    GoToDrafts,
    GoToCalendar,
    NextPane,
    PreviousPane,
}

/// Voice command recognition result
#[derive(Debug, Clone)]
pub struct VoiceRecognitionResult {
    pub text: String,
    pub confidence: f32,
    pub command: Option<VoiceCommand>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Voice Commands Plugin
pub struct VoiceCommandsPlugin {
    config: Arc<RwLock<VoiceCommandConfig>>,
    ai_service: Option<Arc<AIService>>,
    is_listening: Arc<RwLock<bool>>,
    command_history: Arc<RwLock<Vec<VoiceRecognitionResult>>>,
}

impl VoiceCommandsPlugin {
    /// Create a new voice commands plugin
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(VoiceCommandConfig::default())),
            ai_service: None,
            is_listening: Arc::new(RwLock::new(false)),
            command_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Set AI service for the plugin
    pub fn set_ai_service(&mut self, ai_service: Arc<AIService>) {
        self.ai_service = Some(ai_service);
    }

    /// Start listening for voice commands
    pub async fn start_listening(&self) -> Result<(), PluginError> {
        let config = self.config.read().await;
        if !config.enabled {
            return Err(PluginError::NotEnabled("Voice commands are disabled".to_string()));
        }

        let mut is_listening = self.is_listening.write().await;
        if *is_listening {
            return Ok(()); // Already listening
        }

        info!("Starting voice command recognition...");
        *is_listening = true;

        // In a real implementation, this would initialize speech recognition
        // For now, we'll simulate the setup
        debug!("Voice recognition initialized with language: {}", config.language);
        debug!("Activation keywords: {:?}", config.activation_keywords);

        Ok(())
    }

    /// Stop listening for voice commands
    pub async fn stop_listening(&self) -> Result<(), PluginError> {
        let mut is_listening = self.is_listening.write().await;
        if !*is_listening {
            return Ok(()); // Already stopped
        }

        info!("Stopping voice command recognition...");
        *is_listening = false;

        Ok(())
    }

    /// Process recognized speech text into commands
    pub async fn process_speech_text(&self, text: &str, confidence: f32) -> Result<Option<VoiceCommand>, PluginError> {
        let config = self.config.read().await;
        
        if confidence < config.confidence_threshold {
            debug!("Speech confidence {} below threshold {}", confidence, config.confidence_threshold);
            return Ok(None);
        }

        let command = self.parse_voice_command(text).await?;
        
        // Store in history
        let mut history = self.command_history.write().await;
        history.push(VoiceRecognitionResult {
            text: text.to_string(),
            confidence,
            command: command.clone(),
            timestamp: chrono::Utc::now(),
        });

        // Keep only last 100 commands
        if history.len() > 100 {
            history.remove(0);
        }

        Ok(command)
    }

    /// Parse voice command from text
    async fn parse_voice_command(&self, text: &str) -> Result<Option<VoiceCommand>, PluginError> {
        let text_lower = text.to_lowercase();
        
        // Email commands
        if text_lower.contains("email") || text_lower.contains("message") {
            if text_lower.contains("compose") || text_lower.contains("write") || text_lower.contains("new") {
                return Ok(Some(VoiceCommand::Email {
                    action: EmailVoiceAction::Compose,
                    target: None,
                    content: self.extract_content_after_keywords(&text_lower, &["to", "email", "message"]),
                }));
            } else if text_lower.contains("reply") {
                return Ok(Some(VoiceCommand::Email {
                    action: EmailVoiceAction::Reply,
                    target: None,
                    content: self.extract_content_after_keywords(&text_lower, &["reply"]),
                }));
            } else if text_lower.contains("forward") {
                return Ok(Some(VoiceCommand::Email {
                    action: EmailVoiceAction::Forward,
                    target: self.extract_content_after_keywords(&text_lower, &["to", "forward"]),
                    content: None,
                }));
            } else if text_lower.contains("delete") {
                return Ok(Some(VoiceCommand::Email {
                    action: EmailVoiceAction::Delete,
                    target: None,
                    content: None,
                }));
            } else if text_lower.contains("archive") {
                return Ok(Some(VoiceCommand::Email {
                    action: EmailVoiceAction::Archive,
                    target: None,
                    content: None,
                }));
            } else if text_lower.contains("mark read") || text_lower.contains("mark as read") {
                return Ok(Some(VoiceCommand::Email {
                    action: EmailVoiceAction::MarkRead,
                    target: None,
                    content: None,
                }));
            } else if text_lower.contains("mark unread") || text_lower.contains("mark as unread") {
                return Ok(Some(VoiceCommand::Email {
                    action: EmailVoiceAction::MarkUnread,
                    target: None,
                    content: None,
                }));
            } else if text_lower.contains("search") || text_lower.contains("find") {
                return Ok(Some(VoiceCommand::Email {
                    action: EmailVoiceAction::Search,
                    target: None,
                    content: self.extract_content_after_keywords(&text_lower, &["search", "find", "for"]),
                }));
            } else if text_lower.contains("summarize") || text_lower.contains("summary") {
                return Ok(Some(VoiceCommand::Email {
                    action: EmailVoiceAction::Summarize,
                    target: None,
                    content: None,
                }));
            } else if text_lower.contains("triage") || text_lower.contains("prioritize") {
                return Ok(Some(VoiceCommand::Email {
                    action: EmailVoiceAction::Triage,
                    target: None,
                    content: None,
                }));
            }
        }

        // Calendar commands
        if text_lower.contains("calendar") || text_lower.contains("schedule") || text_lower.contains("meeting") || text_lower.contains("appointment") {
            if text_lower.contains("create") || text_lower.contains("new") || text_lower.contains("schedule") {
                return Ok(Some(VoiceCommand::Calendar {
                    action: CalendarVoiceAction::CreateEvent,
                    details: self.extract_content_after_keywords(&text_lower, &["create", "new", "schedule", "meeting", "appointment"]),
                }));
            } else if text_lower.contains("view") || text_lower.contains("show") {
                return Ok(Some(VoiceCommand::Calendar {
                    action: CalendarVoiceAction::ViewCalendar,
                    details: None,
                }));
            } else if text_lower.contains("next week") {
                return Ok(Some(VoiceCommand::Calendar {
                    action: CalendarVoiceAction::NextWeek,
                    details: None,
                }));
            } else if text_lower.contains("previous week") || text_lower.contains("last week") {
                return Ok(Some(VoiceCommand::Calendar {
                    action: CalendarVoiceAction::PreviousWeek,
                    details: None,
                }));
            } else if text_lower.contains("today") {
                return Ok(Some(VoiceCommand::Calendar {
                    action: CalendarVoiceAction::Today,
                    details: None,
                }));
            }
        }

        // AI Assistant commands
        if text_lower.contains("help") || text_lower.contains("assist") || text_lower.contains("ai") {
            if text_lower.contains("help") {
                return Ok(Some(VoiceCommand::Assistant {
                    action: AssistantVoiceAction::Help,
                    query: self.extract_content_after_keywords(&text_lower, &["help", "with"]),
                }));
            } else if text_lower.contains("analyze") {
                return Ok(Some(VoiceCommand::Assistant {
                    action: AssistantVoiceAction::Analyze,
                    query: self.extract_content_after_keywords(&text_lower, &["analyze"]),
                }));
            } else if text_lower.contains("suggest") || text_lower.contains("recommendation") {
                return Ok(Some(VoiceCommand::Assistant {
                    action: AssistantVoiceAction::Suggest,
                    query: self.extract_content_after_keywords(&text_lower, &["suggest", "recommend"]),
                }));
            } else if text_lower.contains("configure") || text_lower.contains("settings") {
                return Ok(Some(VoiceCommand::Assistant {
                    action: AssistantVoiceAction::Configure,
                    query: None,
                }));
            }
        }

        // Navigation commands
        if text_lower.contains("go to") || text_lower.contains("navigate") || text_lower.contains("switch") {
            if text_lower.contains("inbox") {
                return Ok(Some(VoiceCommand::Navigation {
                    action: NavigationVoiceAction::GoToInbox,
                    target: None,
                }));
            } else if text_lower.contains("sent") {
                return Ok(Some(VoiceCommand::Navigation {
                    action: NavigationVoiceAction::GoToSent,
                    target: None,
                }));
            } else if text_lower.contains("drafts") {
                return Ok(Some(VoiceCommand::Navigation {
                    action: NavigationVoiceAction::GoToDrafts,
                    target: None,
                }));
            } else if text_lower.contains("calendar") {
                return Ok(Some(VoiceCommand::Navigation {
                    action: NavigationVoiceAction::GoToCalendar,
                    target: None,
                }));
            }
        }

        if text_lower.contains("next pane") || text_lower.contains("next panel") {
            return Ok(Some(VoiceCommand::Navigation {
                action: NavigationVoiceAction::NextPane,
                target: None,
            }));
        } else if text_lower.contains("previous pane") || text_lower.contains("previous panel") {
            return Ok(Some(VoiceCommand::Navigation {
                action: NavigationVoiceAction::PreviousPane,
                target: None,
            }));
        }

        debug!("No command recognized from text: {}", text);
        Ok(None)
    }

    /// Extract content after specific keywords
    fn extract_content_after_keywords(&self, text: &str, keywords: &[&str]) -> Option<String> {
        for keyword in keywords {
            if let Some(pos) = text.find(keyword) {
                let after_keyword = &text[pos + keyword.len()..].trim();
                if !after_keyword.is_empty() {
                    return Some(after_keyword.to_string());
                }
            }
        }
        None
    }

    /// Execute a voice command
    pub async fn execute_command(&self, command: VoiceCommand) -> Result<String, PluginError> {
        match command {
            VoiceCommand::Email { action, target, content } => {
                self.execute_email_command(action, target, content).await
            }
            VoiceCommand::Calendar { action, details } => {
                self.execute_calendar_command(action, details).await
            }
            VoiceCommand::Assistant { action, query } => {
                self.execute_assistant_command(action, query).await
            }
            VoiceCommand::Navigation { action, target } => {
                self.execute_navigation_command(action, target).await
            }
        }
    }

    /// Execute email voice command
    async fn execute_email_command(
        &self, 
        action: EmailVoiceAction, 
        target: Option<String>, 
        content: Option<String>
    ) -> Result<String, PluginError> {
        match action {
            EmailVoiceAction::Compose => {
                let response = if let Some(content) = content {
                    format!("Composing new email: {}", content)
                } else {
                    "Opening email composition window".to_string()
                };
                Ok(response)
            }
            EmailVoiceAction::Reply => {
                Ok("Replying to current email".to_string())
            }
            EmailVoiceAction::Forward => {
                let response = if let Some(target) = target {
                    format!("Forwarding email to: {}", target)
                } else {
                    "Opening forward dialog".to_string()
                };
                Ok(response)
            }
            EmailVoiceAction::Delete => {
                Ok("Deleting current email".to_string())
            }
            EmailVoiceAction::Archive => {
                Ok("Archiving current email".to_string())
            }
            EmailVoiceAction::MarkRead => {
                Ok("Marking email as read".to_string())
            }
            EmailVoiceAction::MarkUnread => {
                Ok("Marking email as unread".to_string())
            }
            EmailVoiceAction::Search => {
                let response = if let Some(content) = content {
                    format!("Searching for: {}", content)
                } else {
                    "Opening search dialog".to_string()
                };
                Ok(response)
            }
            EmailVoiceAction::Summarize => {
                if let Some(ai_service) = &self.ai_service {
                    // In a real implementation, this would get the current email and summarize it
                    Ok("Generating AI summary of current email".to_string())
                } else {
                    Err(PluginError::ServiceUnavailable("AI service not available".to_string()))
                }
            }
            EmailVoiceAction::Triage => {
                if let Some(ai_service) = &self.ai_service {
                    Ok("Running AI email triage analysis".to_string())
                } else {
                    Err(PluginError::ServiceUnavailable("AI service not available".to_string()))
                }
            }
        }
    }

    /// Execute calendar voice command
    async fn execute_calendar_command(
        &self, 
        action: CalendarVoiceAction, 
        details: Option<String>
    ) -> Result<String, PluginError> {
        match action {
            CalendarVoiceAction::CreateEvent => {
                let response = if let Some(details) = details {
                    format!("Creating calendar event: {}", details)
                } else {
                    "Opening event creation dialog".to_string()
                };
                Ok(response)
            }
            CalendarVoiceAction::ViewCalendar => {
                Ok("Switching to calendar view".to_string())
            }
            CalendarVoiceAction::NextWeek => {
                Ok("Navigating to next week".to_string())
            }
            CalendarVoiceAction::PreviousWeek => {
                Ok("Navigating to previous week".to_string())
            }
            CalendarVoiceAction::Today => {
                Ok("Navigating to today".to_string())
            }
            CalendarVoiceAction::Schedule => {
                Ok("Opening scheduling assistant".to_string())
            }
        }
    }

    /// Execute AI assistant voice command
    async fn execute_assistant_command(
        &self, 
        action: AssistantVoiceAction, 
        query: Option<String>
    ) -> Result<String, PluginError> {
        match action {
            AssistantVoiceAction::Help => {
                let response = if let Some(query) = query {
                    format!("Getting help for: {}", query)
                } else {
                    "Available voice commands: compose email, reply, schedule meeting, view calendar, search, summarize, triage emails".to_string()
                };
                Ok(response)
            }
            AssistantVoiceAction::Summarize => {
                if let Some(ai_service) = &self.ai_service {
                    Ok("Generating AI summary".to_string())
                } else {
                    Err(PluginError::ServiceUnavailable("AI service not available".to_string()))
                }
            }
            AssistantVoiceAction::Analyze => {
                if let Some(ai_service) = &self.ai_service {
                    let response = if let Some(query) = query {
                        format!("Analyzing: {}", query)
                    } else {
                        "Running AI analysis".to_string()
                    };
                    Ok(response)
                } else {
                    Err(PluginError::ServiceUnavailable("AI service not available".to_string()))
                }
            }
            AssistantVoiceAction::Suggest => {
                if let Some(ai_service) = &self.ai_service {
                    let response = if let Some(query) = query {
                        format!("Getting suggestions for: {}", query)
                    } else {
                        "Generating AI suggestions".to_string()
                    };
                    Ok(response)
                } else {
                    Err(PluginError::ServiceUnavailable("AI service not available".to_string()))
                }
            }
            AssistantVoiceAction::Configure => {
                Ok("Opening AI configuration settings".to_string())
            }
        }
    }

    /// Execute navigation voice command
    async fn execute_navigation_command(
        &self, 
        action: NavigationVoiceAction, 
        _target: Option<String>
    ) -> Result<String, PluginError> {
        match action {
            NavigationVoiceAction::GoToInbox => {
                Ok("Navigating to inbox".to_string())
            }
            NavigationVoiceAction::GoToSent => {
                Ok("Navigating to sent items".to_string())
            }
            NavigationVoiceAction::GoToDrafts => {
                Ok("Navigating to drafts".to_string())
            }
            NavigationVoiceAction::GoToCalendar => {
                Ok("Navigating to calendar".to_string())
            }
            NavigationVoiceAction::NextPane => {
                Ok("Moving to next pane".to_string())
            }
            NavigationVoiceAction::PreviousPane => {
                Ok("Moving to previous pane".to_string())
            }
        }
    }

    /// Get command history
    pub async fn get_command_history(&self) -> Vec<VoiceRecognitionResult> {
        self.command_history.read().await.clone()
    }

    /// Update plugin configuration
    pub async fn update_config(&self, new_config: VoiceCommandConfig) -> Result<(), PluginError> {
        let mut config = self.config.write().await;
        *config = new_config;
        info!("Voice commands configuration updated");
        Ok(())
    }

    /// Get current configuration
    pub async fn get_config(&self) -> VoiceCommandConfig {
        self.config.read().await.clone()
    }
}

#[async_trait]
impl Plugin for VoiceCommandsPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "Voice Commands".to_string(),
            version: "1.0.0".to_string(),
            description: "Voice control integration for AI-powered email and calendar management".to_string(),
            author: "Comunicado Team".to_string(),
            capabilities: vec![
                PluginCapability::EmailIntegration,
                PluginCapability::CalendarIntegration,
                PluginCapability::AIIntegration,
                PluginCapability::UserInterface,
            ],
            dependencies: vec!["ai".to_string()],
            optional_dependencies: vec!["speech_recognition".to_string()],
            configuration_schema: Some(serde_json::to_value(VoiceCommandConfig::default()).unwrap()),
        }
    }

    async fn initialize(&mut self, context: PluginContext) -> PluginResult<()> {
        info!("Initializing Voice Commands plugin");
        
        // Check if speech recognition is available
        // In a real implementation, this would check for speech recognition libraries
        debug!("Checking speech recognition capabilities...");
        
        // Load configuration if available
        if let Some(config_value) = context.config {
            if let Ok(config) = serde_json::from_value::<VoiceCommandConfig>(config_value) {
                self.update_config(config).await.map_err(|e| PluginError::InitializationFailed(e.to_string()))?;
            }
        }

        info!("Voice Commands plugin initialized successfully");
        Ok(())
    }

    async fn activate(&mut self) -> PluginResult<()> {
        info!("Activating Voice Commands plugin");
        
        let config = self.config.read().await;
        if config.enabled {
            self.start_listening().await.map_err(|e| PluginError::ActivationFailed(e.to_string()))?;
        }
        
        Ok(())
    }

    async fn deactivate(&mut self) -> PluginResult<()> {
        info!("Deactivating Voice Commands plugin");
        
        self.stop_listening().await.map_err(|e| PluginError::DeactivationFailed(e.to_string()))?;
        
        Ok(())
    }

    async fn execute(&mut self, action: String, parameters: HashMap<String, serde_json::Value>) -> PluginResult<serde_json::Value> {
        match action.as_str() {
            "start_listening" => {
                self.start_listening().await.map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
                Ok(serde_json::json!({"status": "listening_started"}))
            }
            "stop_listening" => {
                self.stop_listening().await.map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
                Ok(serde_json::json!({"status": "listening_stopped"}))
            }
            "process_speech" => {
                let text = parameters.get("text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| PluginError::InvalidParameters("Missing 'text' parameter".to_string()))?;
                
                let confidence = parameters.get("confidence")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(1.0) as f32;

                let command = self.process_speech_text(text, confidence).await
                    .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

                if let Some(cmd) = command {
                    let result = self.execute_command(cmd.clone()).await
                        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
                    
                    Ok(serde_json::json!({
                        "command": cmd,
                        "result": result
                    }))
                } else {
                    Ok(serde_json::json!({
                        "command": null,
                        "result": "No command recognized"
                    }))
                }
            }
            "get_history" => {
                let history = self.get_command_history().await;
                Ok(serde_json::to_value(history).unwrap())
            }
            "update_config" => {
                let config_value = parameters.get("config")
                    .ok_or_else(|| PluginError::InvalidParameters("Missing 'config' parameter".to_string()))?;
                
                let config: VoiceCommandConfig = serde_json::from_value(config_value.clone())
                    .map_err(|e| PluginError::InvalidParameters(format!("Invalid config: {}", e)))?;
                
                self.update_config(config).await
                    .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
                
                Ok(serde_json::json!({"status": "config_updated"}))
            }
            "get_config" => {
                let config = self.get_config().await;
                Ok(serde_json::to_value(config).unwrap())
            }
            _ => Err(PluginError::InvalidAction(format!("Unknown action: {}", action)))
        }
    }

    async fn get_status(&self) -> PluginResult<HashMap<String, serde_json::Value>> {
        let mut status = HashMap::new();
        let config = self.config.read().await;
        let is_listening = *self.is_listening.read().await;
        let history_count = self.command_history.read().await.len();

        status.insert("enabled".to_string(), serde_json::json!(config.enabled));
        status.insert("listening".to_string(), serde_json::json!(is_listening));
        status.insert("language".to_string(), serde_json::json!(config.language));
        status.insert("confidence_threshold".to_string(), serde_json::json!(config.confidence_threshold));
        status.insert("command_history_count".to_string(), serde_json::json!(history_count));
        status.insert("continuous_listening".to_string(), serde_json::json!(config.continuous_listening));
        
        Ok(status)
    }
}

impl Default for VoiceCommandsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

/// Create and register the voice commands plugin
pub fn create_voice_commands_plugin() -> Box<dyn Plugin> {
    Box::new(VoiceCommandsPlugin::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_voice_command_config_default() {
        let config = VoiceCommandConfig::default();
        
        assert!(!config.enabled); // Should be disabled by default for plugin
        assert_eq!(config.language, "en-US");
        assert!(!config.activation_keywords.is_empty());
        assert_eq!(config.confidence_threshold, 0.7);
    }

    #[tokio::test]
    async fn test_voice_command_parsing() {
        let plugin = VoiceCommandsPlugin::new();
        
        // Test email commands
        let command = plugin.parse_voice_command("Compose email to john").await.unwrap();
        assert!(matches!(command, Some(VoiceCommand::Email { action: EmailVoiceAction::Compose, .. })));
        
        let command = plugin.parse_voice_command("Reply to this email").await.unwrap();
        assert!(matches!(command, Some(VoiceCommand::Email { action: EmailVoiceAction::Reply, .. })));
        
        // Test calendar commands
        let command = plugin.parse_voice_command("Create meeting tomorrow at 3 PM").await.unwrap();
        assert!(matches!(command, Some(VoiceCommand::Calendar { action: CalendarVoiceAction::CreateEvent, .. })));
        
        // Test navigation commands
        let command = plugin.parse_voice_command("Go to inbox").await.unwrap();
        assert!(matches!(command, Some(VoiceCommand::Navigation { action: NavigationVoiceAction::GoToInbox, .. })));
    }

    #[tokio::test]
    async fn test_plugin_metadata() {
        let plugin = VoiceCommandsPlugin::new();
        let metadata = plugin.metadata();
        
        assert_eq!(metadata.name, "Voice Commands");
        assert_eq!(metadata.version, "1.0.0");
        assert!(metadata.capabilities.contains(&PluginCapability::AIIntegration));
        assert!(metadata.dependencies.contains(&"ai".to_string()));
    }

    #[tokio::test]
    async fn test_command_execution() {
        let plugin = VoiceCommandsPlugin::new();
        
        let command = VoiceCommand::Email {
            action: EmailVoiceAction::Compose,
            target: None,
            content: Some("Hello world".to_string()),
        };
        
        let result = plugin.execute_command(command).await.unwrap();
        assert!(result.contains("Composing"));
        assert!(result.contains("Hello world"));
    }

    #[tokio::test]
    async fn test_voice_recognition_result() {
        let result = VoiceRecognitionResult {
            text: "Compose email".to_string(),
            confidence: 0.85,
            command: Some(VoiceCommand::Email {
                action: EmailVoiceAction::Compose,
                target: None,
                content: None,
            }),
            timestamp: chrono::Utc::now(),
        };
        
        assert_eq!(result.text, "Compose email");
        assert_eq!(result.confidence, 0.85);
        assert!(result.command.is_some());
    }
}