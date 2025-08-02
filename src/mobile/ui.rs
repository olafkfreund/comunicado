use std::sync::Arc;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap,
        Scrollbar, ScrollbarOrientation, ScrollbarState,
    },
    Frame,
};
use chrono::Local;
use tokio::sync::{RwLock, Mutex};

use crate::mobile::{
    MessageStore, MobileSyncStats,
    kde_connect::types::{SmsMessage, SmsConversation}
};

/// Main SMS/MMS UI component
pub struct SmsUi {
    /// Current conversation list
    conversations: Arc<RwLock<Vec<SmsConversation>>>,
    /// Currently selected conversation
    selected_conversation: Arc<RwLock<Option<SmsConversation>>>,
    /// Current view mode
    view_mode: Arc<RwLock<SmsViewMode>>,
    /// List states for navigation
    conversation_list_state: Arc<Mutex<ListState>>,
    message_list_state: Arc<Mutex<ListState>>,
    /// Current composition state
    composition: Arc<RwLock<SmsComposition>>,
    /// Service statistics
    service_stats: Arc<RwLock<Option<MobileSyncStats>>>,
    /// Scroll states for content
    message_scroll_state: Arc<Mutex<ScrollbarState>>,
    conversation_scroll_state: Arc<Mutex<ScrollbarState>>,
}

/// Different view modes for the SMS interface
#[derive(Debug, Clone, PartialEq)]
pub enum SmsViewMode {
    /// Show conversation list
    ConversationList,
    /// Show messages in a conversation
    MessageThread,
    /// Compose new message
    Compose,
    /// Show message details/attachments
    MessageDetail,
    /// Show service status
    ServiceStatus,
}

/// SMS composition state
#[derive(Debug, Clone)]
pub struct SmsComposition {
    /// Recipients (phone numbers)
    pub recipients: Vec<String>,
    /// Current message body
    pub message_body: String,
    /// Cursor position in message body
    pub cursor_position: usize,
    /// Whether we're editing recipients or message
    pub editing_recipients: bool,
    /// Current recipient being edited
    pub recipient_input: String,
    /// Error message if any
    pub error_message: Option<String>,
}

/// Rendering configuration for SMS UI
#[derive(Debug, Clone)]
pub struct SmsRenderConfig {
    /// Maximum preview length for messages
    pub max_preview_length: usize,
    /// Show timestamps in conversation list
    pub show_timestamps: bool,
    /// Show unread counts
    pub show_unread_counts: bool,
    /// Color scheme
    pub colors: SmsColorScheme,
    /// Date format string
    pub date_format: String,
    /// Time format string
    pub time_format: String,
}

/// Color scheme for SMS UI
#[derive(Debug, Clone)]
pub struct SmsColorScheme {
    pub unread_conversation: Color,
    pub read_conversation: Color,
    pub selected_conversation: Color,
    pub outgoing_message: Color,
    pub incoming_message: Color,
    pub timestamp: Color,
    pub attachment_indicator: Color,
    pub error_message: Color,
    pub service_status_good: Color,
    pub service_status_warning: Color,
    pub service_status_error: Color,
}

impl Default for SmsRenderConfig {
    fn default() -> Self {
        Self {
            max_preview_length: 50,
            show_timestamps: true,
            show_unread_counts: true,
            colors: SmsColorScheme::default(),
            date_format: "%Y-%m-%d".to_string(),
            time_format: "%H:%M".to_string(),
        }
    }
}

impl Default for SmsColorScheme {
    fn default() -> Self {
        Self {
            unread_conversation: Color::White,
            read_conversation: Color::Gray,
            selected_conversation: Color::Cyan,
            outgoing_message: Color::Blue,
            incoming_message: Color::Green,
            timestamp: Color::DarkGray,
            attachment_indicator: Color::Yellow,
            error_message: Color::Red,
            service_status_good: Color::Green,
            service_status_warning: Color::Yellow,
            service_status_error: Color::Red,
        }
    }
}

impl Default for SmsComposition {
    fn default() -> Self {
        Self {
            recipients: Vec::new(),
            message_body: String::new(),
            cursor_position: 0,
            editing_recipients: true,
            recipient_input: String::new(),
            error_message: None,
        }
    }
}

impl SmsUi {
    /// Create a new SMS UI component
    pub fn new() -> Self {
        let mut conversation_state = ListState::default();
        conversation_state.select(Some(0));

        let mut message_state = ListState::default();
        message_state.select(Some(0));

        Self {
            conversations: Arc::new(RwLock::new(Vec::new())),
            selected_conversation: Arc::new(RwLock::new(None)),
            view_mode: Arc::new(RwLock::new(SmsViewMode::ConversationList)),
            conversation_list_state: Arc::new(Mutex::new(conversation_state)),
            message_list_state: Arc::new(Mutex::new(message_state)),
            composition: Arc::new(RwLock::new(SmsComposition::default())),
            service_stats: Arc::new(RwLock::new(None)),
            message_scroll_state: Arc::new(Mutex::new(ScrollbarState::default())),
            conversation_scroll_state: Arc::new(Mutex::new(ScrollbarState::default())),
        }
    }

    /// Load conversations from message store
    pub async fn load_conversations(&self, message_store: &MessageStore) -> Result<(), crate::mobile::MobileError> {
        let query = crate::mobile::storage::MessageQuery {
            limit: Some(100),
            offset: Some(0),
            ..Default::default()
        };

        let conversations = message_store.get_conversations(&query).await?;
        *self.conversations.write().await = conversations;
        
        // Update scroll state
        let conv_count = self.conversations.read().await.len();
        let _ = self.conversation_scroll_state.lock().await.content_length(conv_count);

        Ok(())
    }

    /// Load messages for a specific conversation
    pub async fn load_conversation_messages(
        &self, 
        message_store: &MessageStore, 
        thread_id: i64
    ) -> Result<(), crate::mobile::MobileError> {
        let query = crate::mobile::storage::MessageQuery {
            limit: Some(50),
            offset: Some(0),
            ..Default::default()
        };

        let messages = message_store.get_messages(thread_id, &query).await?;
        
        // Find and update the conversation with loaded messages
        let mut conversations = self.conversations.write().await;
        if let Some(conversation) = conversations.iter_mut().find(|c| c.thread_id == thread_id) {
            conversation.messages = messages;
            *self.selected_conversation.write().await = Some(conversation.clone());
            
            // Update scroll state
            let _ = self.message_scroll_state.lock().await.content_length(conversation.messages.len());
        }

        Ok(())
    }

    /// Update service statistics
    pub async fn update_service_stats(&self, stats: MobileSyncStats) {
        *self.service_stats.write().await = Some(stats);
    }

    /// Set the current view mode
    pub async fn set_view_mode(&self, mode: SmsViewMode) {
        *self.view_mode.write().await = mode;
    }

    /// Get the current view mode
    pub async fn get_view_mode(&self) -> SmsViewMode {
        self.view_mode.read().await.clone()
    }

    /// Handle navigation input (arrow keys, enter, etc.)
    pub async fn handle_navigation(&self, key: char) -> Result<(), crate::mobile::MobileError> {
        let current_mode = self.get_view_mode().await;
        
        match current_mode {
            SmsViewMode::ConversationList => {
                self.handle_conversation_navigation(key).await
            }
            SmsViewMode::MessageThread => {
                self.handle_message_navigation(key).await
            }
            SmsViewMode::Compose => {
                self.handle_composition_input(key).await
            }
            _ => Ok(()),
        }
    }

    /// Handle conversation list navigation
    async fn handle_conversation_navigation(&self, key: char) -> Result<(), crate::mobile::MobileError> {
        let conversations = self.conversations.read().await;
        let mut state = self.conversation_list_state.lock().await;
        
        match key {
            'j' | '↓' => {
                let i = match state.selected() {
                    Some(i) => {
                        if i >= conversations.len().saturating_sub(1) {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                state.select(Some(i));
            }
            'k' | '↑' => {
                let i = match state.selected() {
                    Some(i) => {
                        if i == 0 {
                            conversations.len().saturating_sub(1)
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                state.select(Some(i));
            }
            '\n' | ' ' => {
                // Enter conversation view
                if let Some(selected) = state.selected() {
                    if let Some(conversation) = conversations.get(selected) {
                        *self.selected_conversation.write().await = Some(conversation.clone());
                        self.set_view_mode(SmsViewMode::MessageThread).await;
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Handle message thread navigation
    async fn handle_message_navigation(&self, key: char) -> Result<(), crate::mobile::MobileError> {
        match key {
            'q' | '←' => {
                // Go back to conversation list
                self.set_view_mode(SmsViewMode::ConversationList).await;
            }
            'r' => {
                // Reply to conversation
                if let Some(conversation) = &*self.selected_conversation.read().await {
                    let mut composition = self.composition.write().await;
                    composition.recipients = conversation.participants
                        .iter()
                        .map(|p| p.address.clone())
                        .collect();
                    composition.editing_recipients = false;
                }
                self.set_view_mode(SmsViewMode::Compose).await;
            }
            _ => {}
        }

        Ok(())
    }

    /// Handle composition input
    async fn handle_composition_input(&self, key: char) -> Result<(), crate::mobile::MobileError> {
        let mut composition = self.composition.write().await;
        
        if composition.editing_recipients {
            match key {
                '\n' => {
                    // Add recipient and switch to message editing
                    if !composition.recipient_input.is_empty() {
                        let recipient = composition.recipient_input.clone();
                        composition.recipients.push(recipient);
                        composition.recipient_input.clear();
                        composition.editing_recipients = false;
                    }
                }
                '\x08' => {
                    // Backspace
                    composition.recipient_input.pop();
                }
                c if c.is_ascii() => {
                    composition.recipient_input.push(c);
                }
                _ => {}
            }
        } else {
            match key {
                '\x08' => {
                    // Backspace
                    if composition.cursor_position > 0 {
                        let cursor_pos = composition.cursor_position;
                        composition.message_body.remove(cursor_pos - 1);
                        composition.cursor_position = cursor_pos.saturating_sub(1);
                    }
                }
                '\n' => {
                    let cursor_pos = composition.cursor_position;
                    composition.message_body.insert(cursor_pos, '\n');
                    composition.cursor_position = cursor_pos + 1;
                }
                c if c.is_ascii() => {
                    let cursor_pos = composition.cursor_position;
                    composition.message_body.insert(cursor_pos, c);
                    composition.cursor_position = cursor_pos + 1;
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Render the SMS UI
    pub async fn render(
        &self,
        f: &mut Frame<'_>,
        area: Rect,
        config: &SmsRenderConfig,
    ) -> Result<(), crate::mobile::MobileError> {
        let current_mode = self.get_view_mode().await;
        
        match current_mode {
            SmsViewMode::ConversationList => {
                self.render_conversation_list(f, area, config).await
            }
            SmsViewMode::MessageThread => {
                self.render_message_thread(f, area, config).await
            }
            SmsViewMode::Compose => {
                self.render_composition(f, area, config).await
            }
            SmsViewMode::ServiceStatus => {
                self.render_service_status(f, area, config).await
            }
            SmsViewMode::MessageDetail => {
                self.render_message_detail(f, area, config).await
            }
        }
    }

    /// Render conversation list view
    async fn render_conversation_list(
        &self,
        f: &mut Frame<'_>,
        area: Rect,
        config: &SmsRenderConfig,
    ) -> Result<(), crate::mobile::MobileError> {
        let conversations = self.conversations.read().await;
        let state = self.conversation_list_state.clone();

        let items: Vec<ListItem> = conversations
            .iter()
            .map(|conv| self.format_conversation_item(conv, config))
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" SMS Conversations ")
            )
            .highlight_style(
                Style::default()
                    .fg(config.colors.selected_conversation)
                    .add_modifier(Modifier::BOLD)
            );

        f.render_stateful_widget(list, area, &mut *state.lock().await);

        // Render scrollbar if needed
        if conversations.len() > area.height as usize {
            let scrollbar = Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));

            f.render_stateful_widget(
                scrollbar,
                area.inner(&ratatui::layout::Margin {
                    vertical: 1,
                    horizontal: 0,
                }),
                &mut *self.conversation_scroll_state.lock().await,
            );
        }

        Ok(())
    }

    /// Render message thread view
    async fn render_message_thread(
        &self,
        f: &mut Frame<'_>,
        area: Rect,
        config: &SmsRenderConfig,
    ) -> Result<(), crate::mobile::MobileError> {
        if let Some(conversation) = &*self.selected_conversation.read().await {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
                .split(area);

            // Header with conversation info
            let header_text = format!(
                "Conversation with {} ({} messages)",
                conversation.display_name,
                conversation.messages.len()
            );
            let header = Paragraph::new(header_text)
                .block(Block::default().borders(Borders::ALL).title(" Message Thread "))
                .wrap(Wrap { trim: true });

            f.render_widget(header, chunks[0]);

            // Message list
            let messages: Vec<ListItem> = conversation
                .messages
                .iter()
                .map(|msg| self.format_message_item(msg, config))
                .collect();

            let message_list = List::new(messages)
                .block(Block::default().borders(Borders::ALL));

            f.render_stateful_widget(message_list, chunks[1], &mut *self.message_list_state.lock().await);

            // Render scrollbar for messages
            if conversation.messages.len() > chunks[1].height as usize {
                let scrollbar = Scrollbar::default()
                    .orientation(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("↑"))
                    .end_symbol(Some("↓"));

                f.render_stateful_widget(
                    scrollbar,
                    chunks[1].inner(&ratatui::layout::Margin {
                        vertical: 1,
                        horizontal: 0,
                    }),
                    &mut *self.message_scroll_state.lock().await,
                );
            }
        }

        Ok(())
    }

    /// Render message composition view
    async fn render_composition(
        &self,
        f: &mut Frame<'_>,
        area: Rect,
        config: &SmsRenderConfig,
    ) -> Result<(), crate::mobile::MobileError> {
        let composition = self.composition.read().await;

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Recipients
                Constraint::Min(3),    // Message body
                Constraint::Length(3), // Status/help
            ])
            .split(area);

        // Recipients field
        let recipients_text = if composition.editing_recipients {
            format!("To: {} (editing...)", composition.recipient_input)
        } else {
            format!("To: {}", composition.recipients.join(", "))
        };

        let recipients_widget = Paragraph::new(recipients_text)
            .block(Block::default().borders(Borders::ALL).title(" Recipients "))
            .wrap(Wrap { trim: true });

        f.render_widget(recipients_widget, chunks[0]);

        // Message body
        let message_widget = Paragraph::new(composition.message_body.as_str())
            .block(Block::default().borders(Borders::ALL).title(" Message "))
            .wrap(Wrap { trim: true });

        f.render_widget(message_widget, chunks[1]);

        // Status/help
        let help_text = if composition.editing_recipients {
            "Enter recipient phone number and press Enter. Tab to switch to message."
        } else {
            "Type your message. Ctrl+S to send, Esc to cancel."
        };

        let help_widget = Paragraph::new(help_text)
            .block(Block::default().borders(Borders::ALL).title(" Help "));

        f.render_widget(help_widget, chunks[2]);

        // Show error if any
        if let Some(error) = &composition.error_message {
            let error_popup = Paragraph::new(error.as_str())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Error ")
                        .style(Style::default().fg(config.colors.error_message))
                );

            let popup_area = Self::centered_rect(60, 20, area);
            f.render_widget(Clear, popup_area);
            f.render_widget(error_popup, popup_area);
        }

        Ok(())
    }

    /// Render service status view
    async fn render_service_status(
        &self,
        f: &mut Frame<'_>,
        area: Rect,
        config: &SmsRenderConfig,
    ) -> Result<(), crate::mobile::MobileError> {
        if let Some(stats) = &*self.service_stats.read().await {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(8),  // Connection status
                    Constraint::Length(6),  // Statistics
                    Constraint::Min(0),     // Recent activity
                ])
                .split(area);

            // Connection Status
            let status_color = if stats.is_running {
                if stats.connected_device_name.is_some() {
                    config.colors.service_status_good
                } else {
                    config.colors.service_status_warning
                }
            } else {
                config.colors.service_status_error
            };

            let status_text = vec![
                Line::from(vec![
                    Span::raw("Service Status: "),
                    Span::styled(
                        if stats.is_running { "Running" } else { "Stopped" },
                        Style::default().fg(status_color)
                    ),
                ]),
                Line::from(vec![
                    Span::raw("Connected Device: "),
                    Span::raw(stats.connected_device_name.as_deref().unwrap_or("None")),
                ]),
                Line::from(vec![
                    Span::raw("Sync Interval: "),
                    Span::raw(format!("{} seconds", stats.sync_interval_seconds)),
                ]),
                Line::from(vec![
                    Span::raw("Uptime: "),
                    Span::raw(Self::format_uptime(stats.uptime_seconds)),
                ]),
            ];

            let status_widget = Paragraph::new(status_text)
                .block(Block::default().borders(Borders::ALL).title(" Service Status "));

            f.render_widget(status_widget, chunks[0]);

            // Statistics
            let stats_text = vec![
                Line::from(format!("Total Syncs: {}", stats.total_syncs)),
                Line::from(format!("Total Errors: {}", stats.total_errors)),
                Line::from(format!("Conversations: {}", stats.conversation_count)),
                Line::from(format!("Messages: {}", stats.message_count)),
                Line::from(format!("Unread: {}", stats.unread_count)),
            ];

            let stats_widget = Paragraph::new(stats_text)
                .block(Block::default().borders(Borders::ALL).title(" Statistics "));

            f.render_widget(stats_widget, chunks[1]);

            // Recent Activity
            let mut activity_lines = Vec::new();
            
            if let Some(last_sync) = &stats.last_sync_time {
                activity_lines.push(Line::from(format!(
                    "Last Sync: {}", 
                    last_sync.with_timezone(&Local).format(&config.time_format)
                )));
            }

            if let Some(last_error) = &stats.last_error {
                activity_lines.push(Line::from(vec![
                    Span::raw("Last Error: "),
                    Span::styled(last_error, Style::default().fg(config.colors.error_message)),
                ]));
            }

            if activity_lines.is_empty() {
                activity_lines.push(Line::from("No recent activity"));
            }

            let activity_widget = Paragraph::new(activity_lines)
                .block(Block::default().borders(Borders::ALL).title(" Recent Activity "));

            f.render_widget(activity_widget, chunks[2]);
        }

        Ok(())
    }

    /// Render message detail view (for attachments, etc.)
    async fn render_message_detail(
        &self,
        f: &mut Frame<'_>,
        area: Rect,
        _config: &SmsRenderConfig,
    ) -> Result<(), crate::mobile::MobileError> {
        // Placeholder for message detail view
        let detail_widget = Paragraph::new("Message detail view - Coming soon")
            .block(Block::default().borders(Borders::ALL).title(" Message Details "));

        f.render_widget(detail_widget, area);
        Ok(())
    }

    /// Format a conversation item for the list
    fn format_conversation_item(&self, conversation: &SmsConversation, config: &SmsRenderConfig) -> ListItem {
        let style = if conversation.has_unread() {
            Style::default().fg(config.colors.unread_conversation)
        } else {
            Style::default().fg(config.colors.read_conversation)
        };

        let preview = if let Some(last_message) = conversation.messages.last() {
            last_message.preview_text(config.max_preview_length)
        } else {
            "No messages".to_string()
        };

        let timestamp = if config.show_timestamps {
            format!(" [{}]", conversation.last_activity_formatted())
        } else {
            String::new()
        };

        let unread_indicator = if config.show_unread_counts && conversation.has_unread() {
            format!(" ({})", conversation.unread_count)
        } else {
            String::new()
        };

        let line = Line::from(vec![
            Span::styled(conversation.display_name.clone(), style.add_modifier(Modifier::BOLD)),
            Span::styled(unread_indicator, style),
            Span::styled(timestamp, Style::default().fg(config.colors.timestamp)),
            Span::raw(" - "),
            Span::styled(preview, style),
        ]);

        ListItem::new(line)
    }

    /// Format a message item for the list
    fn format_message_item<'a>(&self, message: &'a SmsMessage, config: &SmsRenderConfig) -> ListItem<'a> {
        let style = if message.is_outgoing() {
            Style::default().fg(config.colors.outgoing_message)
        } else {
            Style::default().fg(config.colors.incoming_message)
        };

        let sender_prefix = if message.is_outgoing() {
            "You: "
        } else {
            if let Some(sender) = message.sender() {
                sender
            } else {
                "Unknown: "
            }
        };

        let attachment_indicator = if message.has_attachments() {
            format!(" 📎{}", message.attachment_count())
        } else {
            String::new()
        };

        let timestamp = message.formatted_date();

        let line = Line::from(vec![
            Span::styled(format!("[{}]", timestamp), Style::default().fg(config.colors.timestamp)),
            Span::raw(" "),
            Span::styled(sender_prefix, style.add_modifier(Modifier::BOLD)),
            Span::styled(message.body.clone(), style),
            Span::styled(attachment_indicator, Style::default().fg(config.colors.attachment_indicator)),
        ]);

        ListItem::new(line)
    }

    /// Create a centered rectangle for popups
    fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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

    /// Format uptime seconds into human readable format
    fn format_uptime(seconds: u64) -> String {
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        let secs = seconds % 60;

        if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, secs)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, secs)
        } else {
            format!("{}s", secs)
        }
    }

    /// Clear any error messages
    pub async fn clear_error(&self) {
        self.composition.write().await.error_message = None;
    }

    /// Set an error message
    pub async fn set_error(&self, error: String) {
        self.composition.write().await.error_message = Some(error);
    }

    /// Get the current composition for sending
    pub async fn get_composition(&self) -> SmsComposition {
        self.composition.read().await.clone()
    }

    /// Clear the current composition
    pub async fn clear_composition(&self) {
        *self.composition.write().await = SmsComposition::default();
    }

    /// Get selected conversation ID
    pub async fn get_selected_conversation_id(&self) -> Option<i64> {
        self.selected_conversation.read().await.as_ref().map(|c| c.id)
    }

    /// Get currently selected conversation thread ID
    pub async fn get_selected_thread_id(&self) -> Option<i64> {
        self.selected_conversation.read().await.as_ref().map(|c| c.thread_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use crate::mobile::kde_connect::types::{ContactInfo, MessageType, Attachment};

    async fn create_test_ui() -> SmsUi {
        SmsUi::new()
    }

    async fn create_test_conversation() -> SmsConversation {
        SmsConversation {
            id: 1,
            thread_id: 1,
            display_name: "Test Contact".to_string(),
            participants: vec![ContactInfo::new("+1234567890".to_string(), Some("Test Contact".to_string()))],
            message_count: 3,
            unread_count: 1,
            last_message_date: Utc::now(),
            is_archived: false,
            messages: vec![
                SmsMessage {
                    id: 1,
                    body: "First message".to_string(),
                    addresses: vec!["+1234567890".to_string()],
                    date: Utc::now().timestamp() * 1000,
                    message_type: MessageType::Sms,
                    read: true,
                    thread_id: 1,
                    sub_id: 1,
                    attachments: vec![],
                },
                SmsMessage {
                    id: 2,
                    body: "Second message with attachment".to_string(),
                    addresses: vec!["+1234567890".to_string()],
                    date: Utc::now().timestamp() * 1000,
                    message_type: MessageType::Mms,
                    read: true,
                    thread_id: 1,
                    sub_id: 1,
                    attachments: vec![Attachment::new(
                        1,
                        "image/jpeg".to_string(),
                        "photo.jpg".to_string(),
                        vec![1, 2, 3, 4, 5],
                    )],
                },
                SmsMessage {
                    id: 3,
                    body: "Latest unread message".to_string(),
                    addresses: vec!["+1234567890".to_string()],
                    date: Utc::now().timestamp() * 1000,
                    message_type: MessageType::Sms,
                    read: false,
                    thread_id: 1,
                    sub_id: 1,
                    attachments: vec![],
                },
            ],
        }
    }

    async fn create_test_message_store() -> MessageStore {
        // Use in-memory database for tests to avoid file system issues
        MessageStore::new(":memory:").await.unwrap()
    }

    #[tokio::test]
    async fn test_ui_creation() {
        let ui = create_test_ui().await;
        
        assert_eq!(ui.get_view_mode().await, SmsViewMode::ConversationList);
        assert!(ui.conversations.read().await.is_empty());
        assert!(ui.selected_conversation.read().await.is_none());
    }

    #[tokio::test]
    async fn test_view_mode_changes() {
        let ui = create_test_ui().await;
        
        // Test initial state
        assert_eq!(ui.get_view_mode().await, SmsViewMode::ConversationList);
        
        // Test mode changes
        ui.set_view_mode(SmsViewMode::MessageThread).await;
        assert_eq!(ui.get_view_mode().await, SmsViewMode::MessageThread);
        
        ui.set_view_mode(SmsViewMode::Compose).await;
        assert_eq!(ui.get_view_mode().await, SmsViewMode::Compose);
        
        ui.set_view_mode(SmsViewMode::ServiceStatus).await;
        assert_eq!(ui.get_view_mode().await, SmsViewMode::ServiceStatus);
    }

    #[tokio::test]
    async fn test_conversation_loading() {
        let ui = create_test_ui().await;
        let message_store = create_test_message_store().await;
        
        // Store a test conversation
        let test_conversation = create_test_conversation().await;
        for message in &test_conversation.messages {
            message_store.store_message(message.clone()).await.unwrap();
        }
        
        // Load conversations into UI
        ui.load_conversations(&message_store).await.unwrap();
        
        let conversations = ui.conversations.read().await;
        assert!(!conversations.is_empty());
    }

    #[tokio::test]
    async fn test_conversation_navigation() {
        let ui = create_test_ui().await;
        
        // Add test conversations
        let mut conversations = ui.conversations.write().await;
        conversations.push(create_test_conversation().await);
        conversations.push({
            let mut conv = create_test_conversation().await;
            conv.id = 2;
            conv.thread_id = 2;
            conv.display_name = "Second Contact".to_string();
            conv
        });
        drop(conversations);
        
        // Test navigation
        ui.handle_navigation('j').await.unwrap(); // Down
        ui.handle_navigation('k').await.unwrap(); // Up
        
        // Test entering conversation
        ui.handle_navigation('\n').await.unwrap(); // Enter
        assert_eq!(ui.get_view_mode().await, SmsViewMode::MessageThread);
    }

    #[tokio::test]
    async fn test_message_thread_navigation() {
        let ui = create_test_ui().await;
        ui.set_view_mode(SmsViewMode::MessageThread).await;
        
        // Set up a selected conversation
        let test_conversation = create_test_conversation().await;
        *ui.selected_conversation.write().await = Some(test_conversation);
        
        // Test navigation back to conversation list
        ui.handle_navigation('q').await.unwrap();
        assert_eq!(ui.get_view_mode().await, SmsViewMode::ConversationList);
        
        // Reset and test reply
        ui.set_view_mode(SmsViewMode::MessageThread).await;
        ui.handle_navigation('r').await.unwrap();
        assert_eq!(ui.get_view_mode().await, SmsViewMode::Compose);
        
        // Check that recipients were populated
        let composition = ui.get_composition().await;
        assert!(!composition.recipients.is_empty());
        assert_eq!(composition.recipients[0], "+1234567890");
    }

    #[tokio::test]
    async fn test_composition_input() {
        let ui = create_test_ui().await;
        ui.set_view_mode(SmsViewMode::Compose).await;
        
        // Test recipient input
        ui.handle_navigation('+').await.unwrap();
        ui.handle_navigation('1').await.unwrap();
        ui.handle_navigation('\n').await.unwrap(); // Add recipient
        
        let composition = ui.get_composition().await;
        assert_eq!(composition.recipients.len(), 1);
        assert!(!composition.editing_recipients);
        
        // Test message input
        ui.handle_navigation('H').await.unwrap();
        ui.handle_navigation('i').await.unwrap();
        ui.handle_navigation('\n').await.unwrap(); // New line
        
        let composition = ui.get_composition().await;
        assert_eq!(composition.message_body, "Hi\n");
        assert_eq!(composition.cursor_position, 3);
        
        // Test backspace
        ui.handle_navigation('\x08').await.unwrap();
        let composition = ui.get_composition().await;
        assert_eq!(composition.message_body, "Hi");
        assert_eq!(composition.cursor_position, 2);
    }

    #[tokio::test]
    async fn test_service_stats_update() {
        let ui = create_test_ui().await;
        
        let test_stats = MobileSyncStats {
            is_running: true,
            total_syncs: 42,
            total_errors: 1,
            last_sync_time: Some(Utc::now()),
            last_error: Some("Test error".to_string()),
            conversation_count: 5,
            message_count: 25,
            unread_count: 3,
            notification_count: 10,
            sync_interval_seconds: 30,
            connected_device_name: Some("Test Phone".to_string()),
            bytes_synced: 1024,
            uptime_seconds: 3665, // 1h 1m 5s
        };
        
        ui.update_service_stats(test_stats.clone()).await;
        
        let stored_stats = ui.service_stats.read().await.clone();
        assert!(stored_stats.is_some());
        
        let stats = stored_stats.unwrap();
        assert_eq!(stats.total_syncs, 42);
        assert_eq!(stats.connected_device_name, Some("Test Phone".to_string()));
        assert_eq!(stats.uptime_seconds, 3665);
    }

    #[tokio::test]
    async fn test_conversation_formatting() {
        let ui = create_test_ui().await;
        let config = SmsRenderConfig::default();
        let conversation = create_test_conversation().await;
        
        let formatted = ui.format_conversation_item(&conversation, &config);
        
        // The formatted item should contain the conversation name and unread indicator
        // This is a basic test - in a real scenario you'd test the rendered output
        assert!(!format!("{:?}", formatted).is_empty());
    }

    #[tokio::test]
    async fn test_message_formatting() {
        let ui = create_test_ui().await;
        let config = SmsRenderConfig::default();
        let conversation = create_test_conversation().await;
        let message = &conversation.messages[0];
        
        let formatted = ui.format_message_item(message, &config);
        
        // The formatted item should contain message content
        assert!(!format!("{:?}", formatted).is_empty());
    }

    #[tokio::test]
    async fn test_error_handling() {
        let ui = create_test_ui().await;
        
        // Test setting and clearing errors
        ui.set_error("Test error message".to_string()).await;
        let composition = ui.get_composition().await;
        assert_eq!(composition.error_message, Some("Test error message".to_string()));
        
        ui.clear_error().await;
        let composition = ui.get_composition().await;
        assert!(composition.error_message.is_none());
    }

    #[tokio::test]
    async fn test_composition_clearing() {
        let ui = create_test_ui().await;
        
        // Set up some composition data
        {
            let mut composition = ui.composition.write().await;
            composition.recipients.push("+1234567890".to_string());
            composition.message_body = "Test message".to_string();
            composition.cursor_position = 5;
            composition.editing_recipients = false;
        }
        
        // Clear composition
        ui.clear_composition().await;
        
        let composition = ui.get_composition().await;
        assert!(composition.recipients.is_empty());
        assert!(composition.message_body.is_empty());
        assert_eq!(composition.cursor_position, 0);
        assert!(composition.editing_recipients);
    }

    #[tokio::test]
    async fn test_conversation_selection() {
        let ui = create_test_ui().await;
        let test_conversation = create_test_conversation().await;
        
        // Set selected conversation
        *ui.selected_conversation.write().await = Some(test_conversation.clone());
        
        assert_eq!(ui.get_selected_conversation_id().await, Some(1));
        assert_eq!(ui.get_selected_thread_id().await, Some(1));
        
        // Clear selection
        *ui.selected_conversation.write().await = None;
        assert!(ui.get_selected_conversation_id().await.is_none());
        assert!(ui.get_selected_thread_id().await.is_none());
    }

    #[tokio::test]
    async fn test_uptime_formatting() {
        assert_eq!(SmsUi::format_uptime(30), "30s");
        assert_eq!(SmsUi::format_uptime(90), "1m 30s");
        assert_eq!(SmsUi::format_uptime(3665), "1h 1m 5s");
        assert_eq!(SmsUi::format_uptime(7200), "2h 0m 0s");
    }

    #[tokio::test]
    async fn test_color_scheme_defaults() {
        let config = SmsRenderConfig::default();
        
        assert_eq!(config.colors.unread_conversation, Color::White);
        assert_eq!(config.colors.read_conversation, Color::Gray);
        assert_eq!(config.colors.selected_conversation, Color::Cyan);
        assert_eq!(config.colors.outgoing_message, Color::Blue);
        assert_eq!(config.colors.incoming_message, Color::Green);
        assert_eq!(config.colors.error_message, Color::Red);
    }

    #[tokio::test]
    async fn test_render_config_defaults() {
        let config = SmsRenderConfig::default();
        
        assert_eq!(config.max_preview_length, 50);
        assert!(config.show_timestamps);
        assert!(config.show_unread_counts);
        assert_eq!(config.date_format, "%Y-%m-%d");
        assert_eq!(config.time_format, "%H:%M");
    }

    #[tokio::test]
    async fn test_message_loading() {
        let ui = create_test_ui().await;
        let message_store = create_test_message_store().await;
        
        // Store test messages
        let test_conversation = create_test_conversation().await;
        for message in &test_conversation.messages {
            message_store.store_message(message.clone()).await.unwrap();
        }
        
        // Load conversations first to populate the conversation list
        ui.load_conversations(&message_store).await.unwrap();
        
        // Load messages for the conversation - this should populate the selected conversation
        ui.load_conversation_messages(&message_store, 1).await.unwrap();
        
        let selected = ui.selected_conversation.read().await;
        assert!(selected.is_some());
        
        let conversation = selected.as_ref().unwrap();
        assert!(!conversation.messages.is_empty());
        assert_eq!(conversation.thread_id, 1);
    }

    #[test]
    fn test_centered_rect() {
        let area = Rect::new(0, 0, 100, 50);
        let centered = SmsUi::centered_rect(50, 20, area);
        
        // Should be centered horizontally and vertically
        assert_eq!(centered.width, 50);
        assert_eq!(centered.height, 10); // 20% of 50
        assert_eq!(centered.x, 25); // Centered horizontally
        assert_eq!(centered.y, 20); // Centered vertically
    }
}