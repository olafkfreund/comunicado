//! Email Component Module
//!
//! Implements a modular email component system that replaces the monolithic email UI structure.

use super::{
    ComponentId, ComponentMetrics, ComponentResult, ComponentState, EventResult, RenderContext,
    UIComponent, UIEvent,
};
use crate::{
    contacts::{ContactsManager, SenderRecognitionService},
    email::{EmailDatabase, StoredMessage},
    ui::{
        compose::{ComposeAction, ComposeUI},
        content_preview::EmailContent,
        email_viewer::{EmailViewer, EmailViewerAction},
        message_list::MessageList,
    },
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Style,
    widgets::{Block, Borders, Paragraph},
};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Email component state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EmailComponentMode {
    MessageList,
    EmailViewer,
    Compose,
}

/// Email component that manages message listing, viewing, and composition
pub struct EmailComponent {
    // Component metadata
    id: ComponentId,
    state: ComponentState,
    metrics: ComponentMetrics,

    // Component configuration
    mode: EmailComponentMode,

    // UI components
    message_list: MessageList,
    email_viewer: EmailViewer,
    compose_ui: Option<ComposeUI>,

    // Data and services
    database: Option<Arc<EmailDatabase>>,
    sender_recognition: Option<Arc<SenderRecognitionService>>,
    contacts_manager: Option<Arc<ContactsManager>>,

    // Current state
    current_account: Option<String>,
    current_folder: Option<String>,
    selected_message: Option<StoredMessage>,
    current_email_content: Option<EmailContent>,

    // Focus management
    focused_section: EmailSection,

    // Performance tracking
    #[allow(dead_code)]
    last_render_time: Instant,
    render_count: u64,
}

/// Sections within the email component that can receive focus
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmailSection {
    FolderTree,
    MessageList,
    MessagePreview,
}

impl EmailComponent {
    /// Create a new email component
    pub fn new() -> Self {
        Self {
            id: ComponentId::new::<Self>(),
            state: ComponentState::Uninitialized,
            metrics: ComponentMetrics::default(),
            mode: EmailComponentMode::MessageList,
            message_list: MessageList::new(),
            email_viewer: EmailViewer::new(),
            compose_ui: None,
            database: None,
            sender_recognition: None,
            contacts_manager: None,
            current_account: None,
            current_folder: None,
            selected_message: None,
            current_email_content: None,
            focused_section: EmailSection::MessageList,
            last_render_time: Instant::now(),
            render_count: 0,
        }
    }

    /// Initialize with database and services
    pub fn with_services(
        mut self,
        database: Option<Arc<EmailDatabase>>,
        sender_recognition: Option<Arc<SenderRecognitionService>>,
        contacts_manager: Option<Arc<ContactsManager>>,
    ) -> Self {
        self.database = database;
        self.sender_recognition = sender_recognition;
        self.contacts_manager = contacts_manager;
        self
    }

    /// Set the current account and folder
    pub fn set_account_folder(&mut self, account: Option<String>, folder: Option<String>) {
        self.current_account = account;
        self.current_folder = folder;
        // TODO: Load messages for the account/folder
    }

    /// Get the current mode
    pub fn mode(&self) -> EmailComponentMode {
        self.mode.clone()
    }

    /// Set the component mode
    pub fn set_mode(&mut self, mode: EmailComponentMode) -> ComponentResult<()> {
        self.mode = mode;

        // Update component state based on mode
        match mode {
            EmailComponentMode::MessageList => {
                self.focused_section = EmailSection::MessageList;
            }
            EmailComponentMode::EmailViewer => {
                self.focused_section = EmailSection::MessagePreview;
            }
            EmailComponentMode::Compose => {
                // Initialize compose UI if we have contacts manager
                if let Some(ref contacts_manager) = self.contacts_manager {
                    self.compose_ui = Some(ComposeUI::new(contacts_manager.clone()));
                }
            }
        }

        Ok(())
    }

    /// Get the selected message
    pub fn selected_message(&self) -> Option<&StoredMessage> {
        self.selected_message.as_ref()
    }

    /// Set the selected message for viewing
    pub fn set_selected_message(&mut self, message: Option<StoredMessage>) {
        self.selected_message = message;

        // Load email content if message is selected
        if let Some(ref msg) = self.selected_message {
            // TODO: Load email content from database
            // For now, create a placeholder content
            let content = EmailContent {
                headers: crate::ui::content_preview::EmailHeader {
                    from: msg.from_addr.clone(),
                    to: msg.to_addrs.clone(),
                    cc: msg.cc_addrs.clone(),
                    bcc: msg.bcc_addrs.clone(),
                    subject: msg.subject.clone(),
                    date: msg.date.to_string(),
                    message_id: msg.message_id.clone().unwrap_or_default(),
                    reply_to: msg.reply_to.clone(),
                    in_reply_to: msg.in_reply_to.clone(),
                },
                body: msg.body_text.clone().unwrap_or_default(),
                content_type: crate::ui::content_preview::ContentType::PlainText,
                attachments: Vec::new(),
                parsed_urls: Vec::new(),
                parsed_content: Vec::new(),
            };

            self.current_email_content = Some(content.clone());
            self.email_viewer.set_email(msg.clone(), content);
        } else {
            self.current_email_content = None;
        }
    }

    /// Handle email viewer actions
    fn handle_email_viewer_action(
        &mut self,
        action: EmailViewerAction,
    ) -> ComponentResult<EventResult> {
        match action {
            EmailViewerAction::Close => {
                self.set_mode(EmailComponentMode::MessageList)?;
                Ok(EventResult::Handled)
            }
            EmailViewerAction::Reply => {
                // TODO: Open compose mode for reply
                Ok(EventResult::Handled)
            }
            EmailViewerAction::ReplyAll => {
                // TODO: Open compose mode for reply all
                Ok(EventResult::Handled)
            }
            EmailViewerAction::Forward => {
                // TODO: Open compose mode for forward
                Ok(EventResult::Handled)
            }
            EmailViewerAction::Delete => {
                // TODO: Delete message
                Ok(EventResult::Handled)
            }
            EmailViewerAction::Archive => {
                // TODO: Archive message
                Ok(EventResult::Handled)
            }
            EmailViewerAction::MarkAsRead => {
                // TODO: Mark as read
                Ok(EventResult::Handled)
            }
            EmailViewerAction::MarkAsUnread => {
                // TODO: Mark as unread
                Ok(EventResult::Handled)
            }
            EmailViewerAction::AddToContacts => {
                // TODO: Add sender to contacts
                Ok(EventResult::Handled)
            }
            EmailViewerAction::Decrypt => {
                // TODO: Handle decrypt action
                Ok(EventResult::Handled)
            }
            EmailViewerAction::Edit => {
                // TODO: Edit draft message
                Ok(EventResult::Handled)
            }
        }
    }

    /// Start compose mode for new email
    pub fn start_compose(&mut self) -> ComponentResult<()> {
        self.set_mode(EmailComponentMode::Compose)?;
        Ok(())
    }

    /// Start compose mode for reply
    pub fn start_reply(&mut self, message: &StoredMessage) -> ComponentResult<()> {
        if let Some(ref contacts_manager) = self.contacts_manager {
            let reply_to = &message.from_addr;
            let subject = if message.subject.starts_with("Re: ") {
                message.subject.clone()
            } else {
                format!("Re: {}", message.subject)
            };

            self.compose_ui = Some(ComposeUI::new_reply(
                contacts_manager.clone(),
                reply_to,
                &subject,
            ));
            self.set_mode(EmailComponentMode::Compose)?;
        }
        Ok(())
    }

    /// Start compose mode for forward
    pub fn start_forward(&mut self, message: &StoredMessage) -> ComponentResult<()> {
        if let Some(ref contacts_manager) = self.contacts_manager {
            let subject = if message.subject.starts_with("Fwd: ") {
                message.subject.clone()
            } else {
                format!("Fwd: {}", message.subject)
            };

            let original_body = message.body_text.as_deref().unwrap_or("");

            self.compose_ui = Some(ComposeUI::new_forward(
                contacts_manager.clone(),
                &subject,
                original_body,
            ));
            self.set_mode(EmailComponentMode::Compose)?;
        }
        Ok(())
    }

    /// Handle compose key events
    pub async fn handle_compose_key(
        &mut self,
        key: KeyEvent,
    ) -> ComponentResult<Option<ComposeAction>> {
        if let Some(ref mut compose_ui) = self.compose_ui {
            let action = compose_ui.handle_key(key).await;
            Ok(Some(action))
        } else {
            Ok(None)
        }
    }

    /// Get compose data for sending
    pub fn get_compose_data(&self) -> Option<crate::ui::EmailComposeData> {
        self.compose_ui.as_ref().map(|ui| ui.get_email_data())
    }

    /// Check if compose form is modified
    pub fn is_compose_modified(&self) -> bool {
        self.compose_ui
            .as_ref()
            .map(|ui| ui.is_modified())
            .unwrap_or(false)
    }

    /// Clear compose modifications (after saving)
    pub fn clear_compose_modified(&mut self) {
        if let Some(ref mut compose_ui) = self.compose_ui {
            compose_ui.clear_modified();
        }
    }

    /// Exit compose mode
    pub fn exit_compose(&mut self) -> ComponentResult<()> {
        self.compose_ui = None;
        self.set_mode(EmailComponentMode::MessageList)?;
        Ok(())
    }

    /// Render the message list view
    fn render_message_list(&mut self, context: &mut RenderContext<'_>) -> ComponentResult<()> {
        // Create layout for folder tree and message list
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25), // Folder tree
                Constraint::Percentage(75), // Message list
            ])
            .split(context.area);

        // Render folder tree placeholder
        let folder_block = Block::default()
            .title("Folders")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(
                if self.focused_section == EmailSection::FolderTree {
                    context.theme.colors.palette.accent
                } else {
                    context.theme.colors.palette.border
                },
            ));

        let folder_content =
            Paragraph::new("📁 Inbox\n📁 Sent\n📁 Drafts\n📁 Trash").block(folder_block);

        context.frame.render_widget(folder_content, chunks[0]);

        // Render message list
        let list_block = Block::default()
            .title("Messages")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(
                if self.focused_section == EmailSection::MessageList {
                    context.theme.colors.palette.accent
                } else {
                    context.theme.colors.palette.border
                },
            ));

        self.message_list.render(
            context.frame,
            chunks[1],
            list_block,
            self.focused_section == EmailSection::MessageList && context.is_focused,
            context.theme,
        );

        Ok(())
    }

    /// Render the email viewer
    fn render_email_viewer(&mut self, context: &mut RenderContext<'_>) -> ComponentResult<()> {
        self.email_viewer
            .render(context.frame, context.area, context.theme);
        Ok(())
    }

    /// Render the compose view
    fn render_compose(&mut self, context: &mut RenderContext<'_>) -> ComponentResult<()> {
        if let Some(ref mut compose_ui) = self.compose_ui {
            compose_ui.render(context.frame, context.area, context.theme);
        } else {
            // Show fallback message if no compose UI is available
            let placeholder =
                Paragraph::new("Compose view requires contacts manager to be initialized").block(
                    Block::default()
                        .title("Compose Email")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(context.theme.colors.palette.border)),
                );

            context.frame.render_widget(placeholder, context.area);
        }
        Ok(())
    }

    /// Handle key events for message list mode
    fn handle_message_list_key(&mut self, key: KeyEvent) -> ComponentResult<EventResult> {
        match key.code {
            KeyCode::Enter => {
                // Open selected message in viewer
                if let Some(_selected_message) = self.selected_message.clone() {
                    self.set_mode(EmailComponentMode::EmailViewer)?;
                    Ok(EventResult::Handled)
                } else {
                    Ok(EventResult::Ignored)
                }
            }
            KeyCode::Char('c') => {
                // Open compose mode
                self.set_mode(EmailComponentMode::Compose)?;
                Ok(EventResult::Handled)
            }
            KeyCode::Tab => {
                // Cycle focus between sections
                self.focused_section = match self.focused_section {
                    EmailSection::FolderTree => EmailSection::MessageList,
                    EmailSection::MessageList => EmailSection::MessagePreview,
                    EmailSection::MessagePreview => EmailSection::FolderTree,
                };
                Ok(EventResult::Handled)
            }
            KeyCode::Up | KeyCode::Down | KeyCode::PageUp | KeyCode::PageDown => {
                // Pass navigation to message list
                // TODO: Handle message list navigation
                Ok(EventResult::Handled)
            }
            _ => Ok(EventResult::Ignored),
        }
    }

    /// Handle key events for email viewer mode
    fn handle_email_viewer_key(&mut self, key: KeyEvent) -> ComponentResult<EventResult> {
        if let Some(action) = self.email_viewer.handle_key(key.code) {
            self.handle_email_viewer_action(action)
        } else {
            Ok(EventResult::Ignored)
        }
    }

    /// Handle key events for compose mode (internal)
    fn handle_compose_key_internal(&mut self, _key: KeyEvent) -> ComponentResult<EventResult> {
        // This would handle compose key events synchronously if needed
        // For now, delegate to the async version via the main event loop
        Ok(EventResult::Ignored)
    }
}

impl UIComponent for EmailComponent {
    fn component_id(&self) -> ComponentId {
        self.id
    }

    fn component_name(&self) -> &str {
        "EmailComponent"
    }

    fn state(&self) -> ComponentState {
        self.state
    }

    fn initialize(&mut self) -> ComponentResult<()> {
        self.state = ComponentState::Ready;
        Ok(())
    }

    fn render(&mut self, context: &mut RenderContext<'_>) -> ComponentResult<()> {
        let start_time = Instant::now();

        // Render based on current mode
        let result = match self.mode {
            EmailComponentMode::MessageList => self.render_message_list(context),
            EmailComponentMode::EmailViewer => self.render_email_viewer(context),
            EmailComponentMode::Compose => self.render_compose(context),
        };

        // Update render metrics
        let render_time = start_time.elapsed();
        self.metrics.last_render_time = render_time;
        self.metrics.render_calls += 1;
        self.render_count += 1;

        // Update average render time (simple moving average)
        let weight = 0.1;
        self.metrics.avg_render_time = Duration::from_nanos(
            (self.metrics.avg_render_time.as_nanos() as f64 * (1.0 - weight)
                + render_time.as_nanos() as f64 * weight) as u64,
        );

        self.metrics.last_updated = Instant::now();

        result
    }

    fn handle_event(&mut self, event: &UIEvent) -> ComponentResult<EventResult> {
        match event {
            UIEvent::Key(key) => {
                self.metrics.events_processed += 1;

                match self.mode {
                    EmailComponentMode::MessageList => self.handle_message_list_key(*key),
                    EmailComponentMode::EmailViewer => self.handle_email_viewer_key(*key),
                    EmailComponentMode::Compose => self.handle_compose_key_internal(*key),
                }
            }
            UIEvent::FocusGained => Ok(EventResult::Handled),
            UIEvent::FocusLost => Ok(EventResult::Handled),
            _ => Ok(EventResult::Ignored),
        }
    }

    fn metrics(&self) -> &ComponentMetrics {
        &self.metrics
    }

    fn set_state(&mut self, new_state: ComponentState) -> ComponentResult<()> {
        self.state = new_state;
        Ok(())
    }

    fn can_focus(&self) -> bool {
        matches!(self.state, ComponentState::Ready | ComponentState::Focused)
    }

    fn cleanup(&mut self) -> ComponentResult<()> {
        self.state = ComponentState::Destroying;
        Ok(())
    }
}

impl Default for EmailComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for EmailComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmailComponent")
            .field("id", &self.id)
            .field("state", &self.state)
            .field("mode", &self.mode)
            .field("current_account", &self.current_account)
            .field("current_folder", &self.current_folder)
            .field("focused_section", &self.focused_section)
            .field("render_count", &self.render_count)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_component_creation() {
        let component = EmailComponent::new();
        assert_eq!(component.state(), ComponentState::Uninitialized);
        assert_eq!(component.mode(), EmailComponentMode::MessageList);
        assert_eq!(component.component_name(), "EmailComponent");
    }

    #[test]
    fn test_email_component_initialization() {
        let mut component = EmailComponent::new();
        component.initialize().unwrap();
        assert_eq!(component.state(), ComponentState::Ready);
    }

    #[test]
    fn test_mode_switching() {
        let mut component = EmailComponent::new();
        component.initialize().unwrap();

        // Switch to email viewer mode
        component.set_mode(EmailComponentMode::EmailViewer).unwrap();
        assert_eq!(component.mode(), EmailComponentMode::EmailViewer);

        // Switch to compose mode
        component.set_mode(EmailComponentMode::Compose).unwrap();
        assert_eq!(component.mode(), EmailComponentMode::Compose);

        // Switch back to message list
        component.set_mode(EmailComponentMode::MessageList).unwrap();
        assert_eq!(component.mode(), EmailComponentMode::MessageList);
    }

    #[test]
    fn test_focus_management() {
        let component = EmailComponent::new();
        assert!(!component.can_focus()); // Uninitialized components can't focus

        let mut component = EmailComponent::new();
        component.initialize().unwrap();
        assert!(component.can_focus()); // Ready components can focus
    }
}
