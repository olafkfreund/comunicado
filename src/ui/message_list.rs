use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState},
    Frame,
};
use crate::theme::Theme;
use crate::email::{EmailThread, SortCriteria, MultiCriteriaSorter, ThreadingEngine, ThreadingAlgorithm, EmailDatabase, StoredMessage};
use std::sync::Arc;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct MessageItem {
    pub subject: String,
    pub sender: String,
    pub date: String,
    pub is_read: bool,
    pub is_important: bool,
    pub has_attachments: bool,
    pub thread_depth: usize,
    pub thread_id: Option<String>,
    pub message_count: usize, // For thread root, number of messages in thread
    pub is_thread_expanded: bool,
    pub is_thread_root: bool,
    pub message_id: Option<Uuid>, // Database ID for loading full content
}

impl MessageItem {
    pub fn new(subject: String, sender: String, date: String) -> Self {
        Self {
            subject,
            sender,
            date,
            is_read: true,
            is_important: false,
            has_attachments: false,
            thread_depth: 0,
            thread_id: None,
            message_count: 1,
            is_thread_expanded: false,
            is_thread_root: false,
            message_id: None,
        }
    }
    
    pub fn new_threaded(subject: String, sender: String, date: String, thread_depth: usize, thread_id: String) -> Self {
        Self {
            subject,
            sender,
            date,
            is_read: true,
            is_important: false,
            has_attachments: false,
            thread_depth,
            thread_id: Some(thread_id),
            message_count: 1,
            is_thread_expanded: false,
            is_thread_root: thread_depth == 0,
            message_id: None,
        }
    }

    pub fn unread(mut self) -> Self {
        self.is_read = false;
        self
    }

    pub fn important(mut self) -> Self {
        self.is_important = true;
        self
    }

    pub fn with_attachments(mut self) -> Self {
        self.has_attachments = true;
        self
    }
    
    pub fn with_thread_count(mut self, count: usize) -> Self {
        self.message_count = count;
        self
    }
    
    pub fn expanded(mut self) -> Self {
        self.is_thread_expanded = true;
        self
    }
    
    pub fn as_thread_root(mut self) -> Self {
        self.is_thread_root = true;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    List,        // Traditional flat list view
    Threaded,    // Hierarchical threaded view
}

pub struct MessageList {
    messages: Vec<MessageItem>,
    threads: Vec<EmailThread>,
    state: ListState,
    view_mode: ViewMode,
    sorter: MultiCriteriaSorter,
    threading_engine: ThreadingEngine,
    database: Option<Arc<EmailDatabase>>,
    current_account: Option<String>,
    current_folder: Option<String>,
}

impl MessageList {
    pub fn new() -> Self {
        let mut list = Self {
            messages: Vec::new(),
            threads: Vec::new(),
            state: ListState::default(),
            view_mode: ViewMode::List,
            sorter: MultiCriteriaSorter::default(),
            threading_engine: ThreadingEngine::new(ThreadingAlgorithm::Simple),
            database: None,
            current_account: None,
            current_folder: None,
        };
        
        // Initialize with sample messages
        list.initialize_sample_messages();
        list.state.select(Some(0));
        
        list
    }

    fn initialize_sample_messages(&mut self) {
        self.messages = vec![
            MessageItem::new(
                "Welcome to Comunicado!".to_string(),
                "Comunicado Team".to_string(),
                "Today 10:30".to_string(),
            ).unread().important(),
            
            MessageItem::new(
                "Project Update: Q1 Planning".to_string(),
                "Alice Johnson".to_string(),
                "Today 09:15".to_string(),
            ).with_attachments(),
            
            MessageItem::new(
                "Re: Meeting Notes from Yesterday".to_string(),
                "Bob Smith".to_string(),
                "Yesterday 16:45".to_string(),
            ),
            
            MessageItem::new(
                "Monthly Newsletter - Tech Updates".to_string(),
                "TechNews Daily".to_string(),
                "Yesterday 14:20".to_string(),
            ).unread(),
            
            MessageItem::new(
                "Invitation: Team Lunch Tomorrow".to_string(),
                "Carol Davis".to_string(),
                "Mon 11:30".to_string(),
            ).important(),
            
            MessageItem::new(
                "Security Alert: Password Change Required".to_string(),
                "IT Security".to_string(),
                "Mon 09:00".to_string(),
            ).unread().important(),
            
            MessageItem::new(
                "Vacation Photos from Hawaii".to_string(),
                "family@example.com".to_string(),
                "Sun 18:22".to_string(),
            ).with_attachments(),
            
            MessageItem::new(
                "Re: Budget Proposal Review".to_string(),
                "David Wilson".to_string(),
                "Fri 15:30".to_string(),
            ),
            
            MessageItem::new(
                "Weekend Plans - Anyone up for hiking?".to_string(),
                "Adventure Club".to_string(),
                "Thu 20:15".to_string(),
            ),
            
            MessageItem::new(
                "Reminder: Dentist Appointment Tomorrow".to_string(),
                "Dr. Smith's Office".to_string(),
                "Wed 12:00".to_string(),
            ).unread(),
        ];
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, block: Block, _is_focused: bool, theme: &Theme) {
        let items: Vec<ListItem> = self.messages
            .iter()
            .enumerate()
            .map(|(i, message)| {
                let is_selected = self.state.selected() == Some(i);
                
                // Style based on message state
                let subject_style = if is_selected {
                    theme.styles.get_selected_style("message_list", &theme.colors)
                } else if !message.is_read {
                    Style::default()
                        .fg(theme.colors.message_list.subject_unread)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.colors.message_list.subject_read)
                };

                let sender_style = if is_selected {
                    theme.styles.get_selected_style("message_list", &theme.colors)
                } else {
                    Style::default().fg(theme.colors.message_list.sender)
                };

                let date_style = if is_selected {
                    theme.styles.get_selected_style("message_list", &theme.colors)
                } else {
                    Style::default().fg(theme.colors.message_list.date)
                };

                // Create threading visualization
                let threading_prefix = self.get_threading_prefix(message);
                
                // Create indicators (professional, text-based)
                let mut indicators = String::new();
                if message.is_important {
                    indicators.push('!');
                }
                if message.has_attachments {
                    indicators.push('@');
                }
                if !message.is_read {
                    indicators.push('•');
                }
                
                // Add thread count for root messages
                if message.is_thread_root && message.message_count > 1 {
                    indicators.push_str(&format!("({})", message.message_count));
                }
                
                if !indicators.is_empty() {
                    indicators.push(' ');
                }

                // Format the message line with threading
                let available_width = match self.view_mode {
                    ViewMode::Threaded => 35_usize.saturating_sub(message.thread_depth * 2),
                    ViewMode::List => 35,
                };
                
                let subject_truncated = if message.subject.len() > available_width {
                    format!("{}...", &message.subject[..available_width.saturating_sub(3)])
                } else {
                    message.subject.clone()
                };

                let sender_truncated = if message.sender.len() > 20 {
                    format!("{}...", &message.sender[..17])
                } else {
                    message.sender.clone()
                };

                let line = match self.view_mode {
                    ViewMode::List => Line::from(vec![
                        Span::raw(indicators),
                        Span::styled(subject_truncated, subject_style),
                        Span::raw("\n  "),
                        Span::styled(format!("From: {}", sender_truncated), sender_style),
                        Span::raw(" • "),
                        Span::styled(message.date.clone(), date_style),
                    ]),
                    ViewMode::Threaded => Line::from(vec![
                        Span::raw(threading_prefix),
                        Span::raw(indicators),
                        Span::styled(subject_truncated, subject_style),
                        Span::raw("\n  "),
                        Span::styled(format!("From: {}", sender_truncated), sender_style),
                        Span::raw(" • "),
                        Span::styled(message.date.clone(), date_style),
                    ])
                };

                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        frame.render_stateful_widget(list, area, &mut self.state.clone());
    }

    pub fn handle_up(&mut self) {
        let selected = match self.state.selected() {
            Some(i) => {
                if i > 0 {
                    Some(i - 1)
                } else {
                    Some(self.messages.len() - 1)
                }
            }
            None => Some(0),
        };
        self.state.select(selected);
    }

    pub fn handle_down(&mut self) {
        let selected = match self.state.selected() {
            Some(i) => {
                if i < self.messages.len() - 1 {
                    Some(i + 1)
                } else {
                    Some(0)
                }
            }
            None => Some(0),
        };
        self.state.select(selected);
    }

    pub fn handle_enter(&mut self) {
        if let Some(selected) = self.state.selected() {
            if let Some(message) = self.messages.get_mut(selected) {
                // Mark message as read when selected
                message.is_read = true;
                // In the future, this will also trigger loading the message content
            }
        }
    }

    pub fn selected_message(&self) -> Option<&MessageItem> {
        self.state.selected().and_then(|i| self.messages.get(i))
    }

    pub fn mark_selected_as_read(&mut self) {
        if let Some(selected) = self.state.selected() {
            if let Some(message) = self.messages.get_mut(selected) {
                message.is_read = true;
            }
        }
    }

    pub fn toggle_selected_important(&mut self) {
        if let Some(selected) = self.state.selected() {
            if let Some(message) = self.messages.get_mut(selected) {
                message.is_important = !message.is_important;
            }
        }
    }
    
    // Threading and view mode methods
    
    pub fn toggle_view_mode(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::List => ViewMode::Threaded,
            ViewMode::Threaded => ViewMode::List,
        };
        self.rebuild_view();
    }
    
    pub fn set_view_mode(&mut self, mode: ViewMode) {
        if self.view_mode != mode {
            self.view_mode = mode;
            self.rebuild_view();
        }
    }
    
    pub fn current_view_mode(&self) -> ViewMode {
        self.view_mode
    }
    
    pub fn set_sort_criteria(&mut self, criteria: SortCriteria) {
        self.sorter = MultiCriteriaSorter::new(vec![criteria]);
        self.rebuild_view();
    }
    
    pub fn add_sort_criteria(&mut self, criteria: SortCriteria) {
        self.sorter.add_criteria(criteria);
        self.rebuild_view();
    }
    
    pub fn clear_sort_criteria(&mut self) {
        self.sorter.clear();
        self.rebuild_view();
    }
    
    pub fn expand_selected_thread(&mut self) {
        if self.view_mode != ViewMode::Threaded {
            return;
        }
        
        if let Some(selected) = self.state.selected() {
            if let Some(message) = self.messages.get_mut(selected) {
                if message.is_thread_root && !message.is_thread_expanded {
                    message.is_thread_expanded = true;
                    self.rebuild_view();
                }
            }
        }
    }
    
    pub fn collapse_selected_thread(&mut self) {
        if self.view_mode != ViewMode::Threaded {
            return;
        }
        
        if let Some(selected) = self.state.selected() {
            if let Some(message) = self.messages.get_mut(selected) {
                if message.is_thread_root && message.is_thread_expanded {
                    message.is_thread_expanded = false;
                    self.rebuild_view();
                }
            }
        }
    }
    
    pub fn toggle_selected_thread(&mut self) {
        if self.view_mode != ViewMode::Threaded {
            return;
        }
        
        if let Some(selected) = self.state.selected() {
            if let Some(message) = self.messages.get(selected) {
                if message.is_thread_root {
                    if message.is_thread_expanded {
                        self.collapse_selected_thread();
                    } else {
                        self.expand_selected_thread();
                    }
                }
            }
        }
    }
    
    fn rebuild_view(&mut self) {
        match self.view_mode {
            ViewMode::List => self.build_flat_view(),
            ViewMode::Threaded => self.build_threaded_view(),
        }
    }
    
    fn build_flat_view(&mut self) {
        // Only initialize sample messages if we don't have real messages loaded
        if self.current_account.is_none() && self.current_folder.is_none() {
            tracing::info!("No real messages loaded, using sample messages");
            self.initialize_sample_messages();
        } else {
            tracing::info!("Using real messages for flat view, {} messages available", self.messages.len());
            // Apply sorting to existing real messages
            // Sort by date (newest first) as default
            self.messages.sort_by(|a, b| b.date.cmp(&a.date));
        }
    }
    
    fn build_threaded_view(&mut self) {
        // Only use sample threaded messages if we don't have real messages loaded
        if self.current_account.is_none() && self.current_folder.is_none() {
            tracing::info!("No real messages loaded, using sample threaded messages");
            // Clear current view
            self.messages.clear();
            // Generate sample threaded messages for demonstration
            self.initialize_sample_threaded_messages();
        } else {
            tracing::info!("Using real messages for threaded view, {} messages available", self.messages.len());
            // TODO: Implement proper threading for real messages
            // For now, just keep the real messages as-is
            // In the future, this would group messages by thread and build the hierarchy
        }
    }
    
    fn initialize_sample_threaded_messages(&mut self) {
        self.messages = vec![
            // Thread 1: Project Planning (expanded)
            MessageItem::new_threaded(
                "Project Update: Q1 Planning".to_string(),
                "Alice Johnson".to_string(),
                "Today 09:15".to_string(),
                0,
                "thread1".to_string()
            ).with_attachments().with_thread_count(3).expanded().as_thread_root(),
            
            MessageItem::new_threaded(
                "Re: Project Update: Q1 Planning".to_string(),
                "Bob Smith".to_string(),
                "Today 10:30".to_string(),
                1,
                "thread1".to_string()
            ),
            
            MessageItem::new_threaded(
                "Re: Project Update: Q1 Planning".to_string(),
                "Carol Davis".to_string(),
                "Today 11:45".to_string(),
                1,
                "thread1".to_string()
            ).unread(),
            
            // Thread 2: Meeting Notes (collapsed)
            MessageItem::new_threaded(
                "Meeting Notes from Yesterday".to_string(),
                "David Wilson".to_string(),
                "Yesterday 16:45".to_string(),
                0,
                "thread2".to_string()
            ).with_thread_count(2).as_thread_root(),
            
            // Thread 3: Security Alert (expanded)
            MessageItem::new_threaded(
                "Security Alert: Password Change Required".to_string(),
                "IT Security".to_string(),
                "Mon 09:00".to_string(),
                0,
                "thread3".to_string()
            ).unread().important().with_thread_count(4).expanded().as_thread_root(),
            
            MessageItem::new_threaded(
                "Re: Security Alert: Action Required".to_string(),
                "System Admin".to_string(),
                "Mon 09:15".to_string(),
                1,
                "thread3".to_string()
            ).important(),
            
            MessageItem::new_threaded(
                "Re: Security Alert: Completed".to_string(),
                "Alice Johnson".to_string(),
                "Mon 09:30".to_string(),
                1,
                "thread3".to_string()
            ),
            
            MessageItem::new_threaded(
                "Re: Security Alert: All Clear".to_string(),
                "IT Security".to_string(),
                "Mon 10:00".to_string(),
                1,
                "thread3".to_string()
            ),
            
            // Standalone messages
            MessageItem::new(
                "Welcome to Comunicado!".to_string(),
                "Comunicado Team".to_string(),
                "Today 10:30".to_string(),
            ).unread().important(),
            
            MessageItem::new(
                "Monthly Newsletter - Tech Updates".to_string(),
                "TechNews Daily".to_string(),
                "Yesterday 14:20".to_string(),
            ).unread(),
        ];
    }
    
    fn get_threading_prefix(&self, message: &MessageItem) -> String {
        if self.view_mode != ViewMode::Threaded {
            return String::new();
        }
        
        let mut prefix = String::new();
        
        // Add indentation based on thread depth
        for _ in 0..message.thread_depth {
            prefix.push_str("  ");
        }
        
        // Add thread indicators
        if message.thread_depth > 0 {
            prefix.push_str("├─ ");
        } else if message.is_thread_root && message.message_count > 1 {
            // Root message with children
            if message.is_thread_expanded {
                prefix.push_str("▼ ");
            } else {
                prefix.push_str("► ");
            }
        }
        
        prefix
    }
    
    /// Set the database for loading real messages
    pub fn set_database(&mut self, database: Arc<EmailDatabase>) {
        self.database = Some(database);
    }
    
    /// Load messages from database for a specific account and folder
    pub async fn load_messages(&mut self, account_id: String, folder_name: String) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("MessageList::load_messages called with account_id: {}, folder_name: {}", account_id, folder_name);
        
        if let Some(ref database) = self.database {
            self.current_account = Some(account_id.clone());
            self.current_folder = Some(folder_name.clone());
            
            // Load messages from database
            tracing::info!("Loading messages from database...");
            let stored_messages = database.get_messages(&account_id, &folder_name, Some(100), None).await?;
            tracing::info!("Loaded {} messages from database", stored_messages.len());
            
            // Convert stored messages to MessageItems
            self.messages = stored_messages.into_iter().map(|msg| {
                MessageItem::from_stored_message(&msg)
            }).collect();
            
            tracing::info!("Converted to {} MessageItems", self.messages.len());
            
            // Sort messages by date (newest first)
            self.messages.sort_by(|a, b| b.date.cmp(&a.date));
            
            // Reset selection
            if !self.messages.is_empty() {
                self.state.select(Some(0));
                tracing::info!("Selected first message, total messages: {}", self.messages.len());
            } else {
                self.state.select(None);
                tracing::warn!("No messages loaded, selection cleared");
            }
        } else {
            tracing::error!("Database not available in MessageList");
            return Err("Database not available".into());
        }
        
        Ok(())
    }
    
    /// Refresh current folder's messages
    pub async fn refresh_messages(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let (Some(account), Some(folder)) = (self.current_account.clone(), self.current_folder.clone()) {
            self.load_messages(account, folder).await?;
        }
        Ok(())
    }
    
    /// Get the currently selected message
    pub fn get_selected_stored_message(&self) -> Option<&MessageItem> {
        if let Some(selected) = self.state.selected() {
            self.messages.get(selected)
        } else {
            None
        }
    }
    
    /// Check if we have a database connection
    pub fn has_database(&self) -> bool {
        self.database.is_some()
    }
    
    /// Get current account and folder
    pub fn get_current_context(&self) -> (Option<&String>, Option<&String>) {
        (self.current_account.as_ref(), self.current_folder.as_ref())
    }
    
    /// Get all messages (read-only access)
    pub fn messages(&self) -> &Vec<MessageItem> {
        &self.messages
    }
    
    /// Get the selected message for loading in content preview
    pub fn get_selected_message_for_preview(&self) -> Option<MessageItem> {
        if let Some(selected_index) = self.state.selected() {
            self.messages.get(selected_index).cloned()
        } else {
            None
        }
    }
    
    /// Get the current selection state
    pub fn get_selection_state(&self) -> Option<usize> {
        self.state.selected()
    }
}

impl MessageItem {
    /// Convert a StoredMessage from the database to a MessageItem for display
    pub fn from_stored_message(stored: &StoredMessage) -> Self {
        // Format the date in a user-friendly way
        let date_str = Self::format_message_date(stored.date);
        
        Self {
            subject: stored.subject.clone(),
            sender: if let Some(ref name) = stored.from_name {
                name.clone()
            } else {
                stored.from_addr.clone()
            },
            date: date_str,
            is_read: stored.flags.contains(&"\\Seen".to_string()),
            is_important: stored.flags.contains(&"\\Flagged".to_string()),
            has_attachments: !stored.attachments.is_empty(),
            thread_depth: 0,
            thread_id: stored.thread_id.clone(),
            message_count: 1,
            is_thread_expanded: false,
            is_thread_root: false,
            message_id: Some(stored.id),
        }
    }
    
    /// Helper function to format message dates in a human-readable way
    fn format_message_date(date: DateTime<Utc>) -> String {
        let now = Utc::now();
        let duration = now.signed_duration_since(date);
        
        if duration.num_days() == 0 {
            // Today
            date.format("Today %H:%M").to_string()
        } else if duration.num_days() == 1 {
            // Yesterday
            date.format("Yesterday %H:%M").to_string()
        } else if duration.num_days() < 7 {
            // This week
            date.format("%a %H:%M").to_string()
        } else if duration.num_days() < 365 {
            // This year
            date.format("%b %d").to_string()
        } else {
            // Older
            date.format("%b %d %Y").to_string()
        }
    }
}

impl Default for MessageList {
    fn default() -> Self {
        Self::new()
    }
}