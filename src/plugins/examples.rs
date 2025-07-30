//! Example plugin implementations to demonstrate the plugin architecture
//!
//! These plugins serve as reference implementations and showcase how to create
//! plugins for different categories of functionality.

use super::core::{Plugin, PluginConfig, PluginInfo, PluginResult, PluginType};
use super::types::{
    EmailPlugin, EmailPluginContext, EmailProcessResult, EmailCapability,
    UIPlugin, UIPluginContext, UIComponentResult, UIInputResult, UILayoutPreferences, UIPosition, UICapability,
    CalendarPlugin, CalendarPluginContext, CalendarEventResult, CalendarCapability,
    NotificationPlugin, NotificationPluginContext, NotificationMessage, NotificationResult, NotificationCapability, NotificationType,
    SearchPlugin, SearchPluginContext, SearchQuery, SearchResult, SearchResultItem, SearchCapability,
};

use crate::email::StoredMessage;
use crate::calendar::event::Event;

use ratatui::{Frame, layout::Rect, widgets::{Block, Borders, Paragraph}, style::{Color, Style}};
use std::any::Any;
use std::collections::HashMap;

// ============================================================================
// Example Email Plugin
// ============================================================================

/// Example email plugin that demonstrates spam filtering
pub struct ExampleEmailPlugin {
    info: PluginInfo,
    spam_keywords: Vec<String>,
    processed_count: u64,
}

impl ExampleEmailPlugin {
    pub fn new() -> Self {
        let info = PluginInfo::new(
            "Example Email Plugin".to_string(),
            "1.0.0".to_string(),
            "Demonstrates email processing capabilities".to_string(),
            "Comunicado Team".to_string(),
            PluginType::Email,
            "1.0.0".to_string(),
        );

        Self {
            info,
            spam_keywords: vec![
                "spam".to_string(),
                "offer".to_string(),
                "free".to_string(),
                "winner".to_string(),
                "urgent".to_string(),
            ],
            processed_count: 0,
        }
    }

    fn is_spam(&self, message: &StoredMessage) -> bool {
        let subject_lower = message.subject.to_lowercase();
        let body_lower = message.body_text.as_ref()
            .map(|body| body.to_lowercase())
            .unwrap_or_default();

        self.spam_keywords.iter().any(|keyword| {
            subject_lower.contains(keyword) || body_lower.contains(keyword)
        })
    }
}

impl Plugin for ExampleEmailPlugin {
    fn info(&self) -> PluginInfo {
        self.info.clone()
    }

    fn initialize(&mut self, config: &PluginConfig) -> PluginResult<()> {
        // Load custom spam keywords from config if available
        if let Ok(keywords) = config.get_config::<Vec<String>>("spam_keywords") {
            self.spam_keywords = keywords;
        }
        
        println!("Example Email Plugin initialized with {} spam keywords", self.spam_keywords.len());
        Ok(())
    }

    fn start(&mut self) -> PluginResult<()> {
        println!("Example Email Plugin started");
        Ok(())
    }

    fn stop(&mut self) -> PluginResult<()> {
        println!("Example Email Plugin stopped (processed {} emails)", self.processed_count);
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl EmailPlugin for ExampleEmailPlugin {
    async fn process_incoming_email(
        &mut self,
        message: &StoredMessage,
        _context: &EmailPluginContext,
    ) -> PluginResult<EmailProcessResult> {
        self.processed_count += 1;

        // Check if message is spam
        if self.is_spam(message) {
            println!("Detected spam email: {}", message.subject);
            
            // Move spam to spam folder
            return Ok(EmailProcessResult::Move("Spam".to_string()));
        }

        // Check for auto-responder keywords
        if message.subject.to_lowercase().contains("out of office") {
            println!("Detected auto-responder message: {}", message.subject);
            
            // Mark as auto-responder
            return Ok(EmailProcessResult::SetFlags(vec!["auto-responder".to_string()]));
        }

        Ok(EmailProcessResult::NoChange)
    }

    async fn process_outgoing_email(
        &mut self,
        message: &StoredMessage,
        _context: &EmailPluginContext,
    ) -> PluginResult<EmailProcessResult> {
        // Add signature if not present
        if let Some(body) = &message.body_text {
            if !body.contains("Sent via Comunicado") {
                let mut modified_message = message.clone();
                modified_message.body_text = Some(format!(
                    "{}\n\n---\nSent via Comunicado Email Client",
                    body
                ));
                return Ok(EmailProcessResult::Modified(modified_message));
            }
        }

        Ok(EmailProcessResult::NoChange)
    }

    async fn filter_emails(
        &self,
        messages: &[StoredMessage],
        _context: &EmailPluginContext,
    ) -> PluginResult<Vec<bool>> {
        let results = messages.iter()
            .map(|message| !self.is_spam(message))
            .collect();
        
        Ok(results)
    }

    fn get_email_capabilities(&self) -> Vec<EmailCapability> {
        vec![
            EmailCapability::SpamFilter,
            EmailCapability::ContentFilter,
            EmailCapability::ContentAnalysis,
        ]
    }
}

// ============================================================================
// Example UI Plugin
// ============================================================================

/// Example UI plugin that adds a simple status widget
pub struct ExampleUIPlugin {
    info: PluginInfo,
    display_text: String,
    update_count: u32,
}

impl ExampleUIPlugin {
    pub fn new() -> Self {
        let info = PluginInfo::new(
            "Example UI Plugin".to_string(),
            "1.0.0".to_string(),
            "Demonstrates UI extension capabilities".to_string(),
            "Comunicado Team".to_string(),
            PluginType::UI,
            "1.0.0".to_string(),
        );

        Self {
            info,
            display_text: "Example Plugin Active".to_string(),
            update_count: 0,
        }
    }
}

impl Plugin for ExampleUIPlugin {
    fn info(&self) -> PluginInfo {
        self.info.clone()
    }

    fn initialize(&mut self, config: &PluginConfig) -> PluginResult<()> {
        if let Ok(text) = config.get_config::<String>("display_text") {
            self.display_text = text;
        }
        
        println!("Example UI Plugin initialized");
        Ok(())
    }

    fn start(&mut self) -> PluginResult<()> {
        println!("Example UI Plugin started");
        Ok(())
    }

    fn stop(&mut self) -> PluginResult<()> {
        println!("Example UI Plugin stopped");
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl UIPlugin for ExampleUIPlugin {
    fn render_component(
        &self,
        frame: &mut Frame,
        area: Rect,
        _context: &UIPluginContext,
    ) -> PluginResult<UIComponentResult> {
        let text = format!("{} (Updates: {})", self.display_text, self.update_count);
        
        let block = Block::default()
            .title("Example Plugin")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Green));

        let paragraph = Paragraph::new(text)
            .block(block)
            .style(Style::default().fg(Color::White));

        frame.render_widget(paragraph, area);

        Ok(UIComponentResult::Rendered)
    }

    async fn handle_input(
        &mut self,
        key_event: crossterm::event::KeyEvent,
        _context: &UIPluginContext,
    ) -> PluginResult<UIInputResult> {
        use crossterm::event::{KeyCode, KeyModifiers};

        match (key_event.code, key_event.modifiers) {
            (KeyCode::F(5), KeyModifiers::NONE) => {
                self.update_count += 1;
                Ok(UIInputResult::Action("refresh_plugin".to_string(), serde_json::json!({})))
            }
            _ => Ok(UIInputResult::NotHandled)
        }
    }

    fn get_layout_preferences(&self) -> UILayoutPreferences {
        UILayoutPreferences {
            preferred_position: UIPosition::Bottom,
            min_size: (20, 3),
            max_size: Some((80, 5)),
            resizable: true,
            movable: false,
        }
    }

    fn get_ui_capabilities(&self) -> Vec<UICapability> {
        vec![
            UICapability::CustomWidgets,
            UICapability::KeyboardShortcuts,
            UICapability::EventHandling,
        ]
    }
}

// ============================================================================
// Example Calendar Plugin
// ============================================================================

/// Example calendar plugin that demonstrates event processing
pub struct ExampleCalendarPlugin {
    info: PluginInfo,
    processed_events: u64,
}

impl ExampleCalendarPlugin {
    pub fn new() -> Self {
        let info = PluginInfo::new(
            "Example Calendar Plugin".to_string(),
            "1.0.0".to_string(),
            "Demonstrates calendar event processing".to_string(),
            "Comunicado Team".to_string(),
            PluginType::Calendar,
            "1.0.0".to_string(),
        );

        Self {
            info,
            processed_events: 0,
        }
    }
}

impl Plugin for ExampleCalendarPlugin {
    fn info(&self) -> PluginInfo {
        self.info.clone()
    }

    fn initialize(&mut self, _config: &PluginConfig) -> PluginResult<()> {
        println!("Example Calendar Plugin initialized");
        Ok(())
    }

    fn start(&mut self) -> PluginResult<()> {
        println!("Example Calendar Plugin started");
        Ok(())
    }

    fn stop(&mut self) -> PluginResult<()> {
        println!("Example Calendar Plugin stopped (processed {} events)", self.processed_events);
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl CalendarPlugin for ExampleCalendarPlugin {
    async fn process_event(
        &mut self,
        event: &Event,
        _context: &CalendarPluginContext,
    ) -> PluginResult<CalendarEventResult> {
        self.processed_events += 1;

        // Check for conflicting events (simplified logic)
        if event.title.to_lowercase().contains("meeting") {
            println!("Processed meeting event: {}", event.title);
        }

        Ok(CalendarEventResult::Success)
    }

    async fn handle_invitation(
        &mut self,
        _invitation_data: &serde_json::Value,
        _context: &CalendarPluginContext,
    ) -> PluginResult<CalendarEventResult> {
        println!("Handling calendar invitation");
        Ok(CalendarEventResult::Success)
    }

    async fn get_calendar_sources(&self) -> PluginResult<Vec<super::types::CalendarSource>> {
        Ok(vec![])
    }

    async fn sync_calendars(
        &mut self,
        _context: &CalendarPluginContext,
    ) -> PluginResult<super::types::CalendarSyncResult> {
        Ok(super::types::CalendarSyncResult {
            events_synced: 0,
            events_updated: 0,
            events_deleted: 0,
            errors: vec![],
        })
    }

    fn get_calendar_capabilities(&self) -> Vec<CalendarCapability> {
        vec![
            CalendarCapability::EventCreation,
            CalendarCapability::EventModification,
        ]
    }
}

// ============================================================================
// Example Notification Plugin
// ============================================================================

/// Example notification plugin for desktop notifications
pub struct ExampleNotificationPlugin {
    info: PluginInfo,
    sent_notifications: u64,
}

impl ExampleNotificationPlugin {
    pub fn new() -> Self {
        let info = PluginInfo::new(
            "Example Notification Plugin".to_string(),
            "1.0.0".to_string(),
            "Demonstrates notification handling".to_string(),
            "Comunicado Team".to_string(),
            PluginType::Notification,
            "1.0.0".to_string(),
        );

        Self {
            info,
            sent_notifications: 0,
        }
    }
}

impl Plugin for ExampleNotificationPlugin {
    fn info(&self) -> PluginInfo {
        self.info.clone()
    }

    fn initialize(&mut self, _config: &PluginConfig) -> PluginResult<()> {
        println!("Example Notification Plugin initialized");
        Ok(())
    }

    fn start(&mut self) -> PluginResult<()> {
        println!("Example Notification Plugin started");
        Ok(())
    }

    fn stop(&mut self) -> PluginResult<()> {
        println!("Example Notification Plugin stopped (sent {} notifications)", self.sent_notifications);
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl NotificationPlugin for ExampleNotificationPlugin {
    async fn send_notification(
        &mut self,
        notification: &NotificationMessage,
        _context: &NotificationPluginContext,
    ) -> PluginResult<NotificationResult> {
        self.sent_notifications += 1;

        // Simulate sending notification
        println!("Sending notification: {} - {}", notification.title, notification.body);

        Ok(NotificationResult::Sent)
    }

    async fn handle_notification_response(
        &mut self,
        response: &super::types::NotificationResponse,
        _context: &NotificationPluginContext,
    ) -> PluginResult<NotificationResult> {
        println!("Handling notification response: {:?}", response.action_id);
        Ok(NotificationResult::Sent)
    }

    fn get_supported_types(&self) -> Vec<NotificationType> {
        vec![
            NotificationType::NewEmail,
            NotificationType::CalendarReminder,
            NotificationType::System,
        ]
    }

    fn get_notification_capabilities(&self) -> Vec<NotificationCapability> {
        vec![
            NotificationCapability::Desktop,
            NotificationCapability::Sound,
        ]
    }
}

// ============================================================================
// Example Search Plugin
// ============================================================================

/// Example search plugin that demonstrates enhanced search capabilities
pub struct ExampleSearchPlugin {
    info: PluginInfo,
    search_count: u64,
}

impl ExampleSearchPlugin {
    pub fn new() -> Self {
        let info = PluginInfo::new(
            "Example Search Plugin".to_string(),
            "1.0.0".to_string(),
            "Demonstrates enhanced search capabilities".to_string(),
            "Comunicado Team".to_string(),
            PluginType::Search,
            "1.0.0".to_string(),
        );

        Self {
            info,
            search_count: 0,
        }
    }
}

impl Plugin for ExampleSearchPlugin {
    fn info(&self) -> PluginInfo {
        self.info.clone()
    }

    fn initialize(&mut self, _config: &PluginConfig) -> PluginResult<()> {
        println!("Example Search Plugin initialized");
        Ok(())
    }

    fn start(&mut self) -> PluginResult<()> {
        println!("Example Search Plugin started");
        Ok(())
    }

    fn stop(&mut self) -> PluginResult<()> {
        println!("Example Search Plugin stopped (performed {} searches)", self.search_count);
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl SearchPlugin for ExampleSearchPlugin {
    async fn search(
        &self,
        query: &SearchQuery,
        _context: &SearchPluginContext,
    ) -> PluginResult<SearchResult> {
        println!("Performing search: {}", query.query);

        // Simulate search results
        let items = vec![
            SearchResultItem {
                id: "1".to_string(),
                item_type: "email".to_string(),
                score: 0.95,
                snippets: vec!["Example search result snippet".to_string()],
                metadata: HashMap::new(),
            }
        ];

        Ok(SearchResult {
            items,
            total_count: 1,
            execution_time: std::time::Duration::from_millis(50),
            metadata: HashMap::new(),
        })
    }

    async fn index_content(
        &mut self,
        content: &super::types::SearchableContent,
        _context: &SearchPluginContext,
    ) -> PluginResult<()> {
        println!("Indexing content: {}", content.id);
        Ok(())
    }

    async fn get_suggestions(
        &self,
        partial_query: &str,
        _context: &SearchPluginContext,
    ) -> PluginResult<Vec<String>> {
        Ok(vec![
            format!("{} example", partial_query),
            format!("{} suggestion", partial_query),
        ])
    }

    fn get_search_capabilities(&self) -> Vec<SearchCapability> {
        vec![
            SearchCapability::FullText,
            SearchCapability::Fuzzy,
        ]
    }
}

// ============================================================================
// Plugin Factory Function
// ============================================================================

/// Factory function to create example plugins by name
pub fn create_example_plugin(name: &str) -> Option<Box<dyn Plugin>> {
    match name {
        "example_email_plugin" => Some(Box::new(ExampleEmailPlugin::new())),
        "example_ui_plugin" => Some(Box::new(ExampleUIPlugin::new())),
        "example_calendar_plugin" => Some(Box::new(ExampleCalendarPlugin::new())),
        "example_notification_plugin" => Some(Box::new(ExampleNotificationPlugin::new())),
        "example_search_plugin" => Some(Box::new(ExampleSearchPlugin::new())),
        _ => None,
    }
}

/// Get list of all available example plugins
pub fn get_available_example_plugins() -> Vec<PluginInfo> {
    vec![
        ExampleEmailPlugin::new().info(),
        ExampleUIPlugin::new().info(),
        ExampleCalendarPlugin::new().info(),
        ExampleNotificationPlugin::new().info(),
        ExampleSearchPlugin::new().info(),
    ]
}