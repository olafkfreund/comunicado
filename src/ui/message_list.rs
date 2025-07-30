use crate::email::{
    EmailDatabase, EmailMessage, EmailThread, MessageId, MultiCriteriaSorter, SortCriteria,
    StoredMessage, ThreadingAlgorithm, ThreadingEngine,
};
use crate::theme::Theme;
use chrono::{DateTime, Utc};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState},
    Frame,
};
use std::sync::Arc;
use std::collections::HashMap;
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

    pub fn new_threaded(
        subject: String,
        sender: String,
        date: String,
        thread_depth: usize,
        thread_id: String,
    ) -> Self {
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

    /// Mark this message as unread
    pub fn unread(mut self) -> Self {
        self.is_read = false;
        self
    }

    /// Mark this message as important
    pub fn important(mut self) -> Self {
        self.is_important = true;
        self
    }

    /// Indicate this message has attachments
    pub fn with_attachments(mut self) -> Self {
        self.has_attachments = true;
        self
    }

    /// Set the number of messages in this thread
    pub fn with_thread_count(mut self, count: usize) -> Self {
        self.message_count = count;
        self
    }

    /// Mark this thread as expanded
    pub fn expanded(mut self) -> Self {
        self.is_thread_expanded = true;
        self
    }

    /// Mark this message as a thread root
    pub fn as_thread_root(mut self) -> Self {
        self.is_thread_root = true;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    List,     // Traditional flat list view
    Threaded, // Hierarchical threaded view
}

pub struct MessageList {
    messages: Vec<MessageItem>,
    filtered_messages: Vec<MessageItem>,
    #[allow(dead_code)]
    threads: Vec<EmailThread>,
    state: ListState,
    view_mode: ViewMode,
    sorter: MultiCriteriaSorter,
    #[allow(dead_code)]
    threading_engine: ThreadingEngine,
    database: Option<Arc<EmailDatabase>>,
    current_account: Option<String>,
    current_folder: Option<String>,
    // Search functionality
    search_query: String,
    search_active: bool,
    search_results_count: usize,
    // Threading cache to avoid blocking database calls
    threading_cache: HashMap<String, Vec<StoredMessage>>,
    threading_cache_key: Option<String>,
}

impl MessageList {
    pub fn new() -> Self {
        let mut list = Self {
            messages: Vec::new(),
            filtered_messages: Vec::new(),
            threads: Vec::new(),
            state: ListState::default(),
            view_mode: ViewMode::List,
            sorter: MultiCriteriaSorter::default(),
            threading_engine: ThreadingEngine::new(ThreadingAlgorithm::Simple),
            database: None,
            current_account: None,
            current_folder: None,
            search_query: String::new(),
            search_active: false,
            search_results_count: 0,
            threading_cache: HashMap::new(),
            threading_cache_key: None,
        };

        // Don't initialize with sample messages initially - they will be loaded from database
        // or initialized later if no database is available
        list.state.select(None);

        list
    }

    fn initialize_sample_messages(&mut self) {
        self.messages = vec![
            MessageItem::new(
                "Welcome to Comunicado!".to_string(),
                "Comunicado Team".to_string(),
                "Today 10:30".to_string(),
            )
            .unread()
            .important(),
            MessageItem::new(
                "Project Update: Q1 Planning".to_string(),
                "Alice Johnson".to_string(),
                "Today 09:15".to_string(),
            )
            .with_attachments(),
            MessageItem::new(
                "Re: Meeting Notes from Yesterday".to_string(),
                "Bob Smith".to_string(),
                "Yesterday 16:45".to_string(),
            ),
            MessageItem::new(
                "Monthly Newsletter - Tech Updates".to_string(),
                "TechNews Daily".to_string(),
                "Yesterday 14:20".to_string(),
            )
            .unread(),
            MessageItem::new(
                "Invitation: Team Lunch Tomorrow".to_string(),
                "Carol Davis".to_string(),
                "Mon 11:30".to_string(),
            )
            .important(),
            MessageItem::new(
                "Security Alert: Password Change Required".to_string(),
                "IT Security".to_string(),
                "Mon 09:00".to_string(),
            )
            .unread()
            .important(),
            MessageItem::new(
                "Vacation Photos from Hawaii".to_string(),
                "family@example.com".to_string(),
                "Sun 18:22".to_string(),
            )
            .with_attachments(),
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
            )
            .unread(),
        ];
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        block: Block,
        _is_focused: bool,
        theme: &Theme,
    ) {
        tracing::debug!("MessageList::render called with {} messages, current_account: {:?}, current_folder: {:?}", 
                       self.messages.len(), self.current_account, self.current_folder);

        // Use filtered messages if search is active, otherwise use all messages
        let messages_to_display = if self.search_active {
            &self.filtered_messages
        } else {
            &self.messages
        };

        let items: Vec<ListItem> = messages_to_display
            .iter()
            .enumerate()
            .map(|(i, message)| {
                let is_selected = self.state.selected() == Some(i);

                // Style based on message state
                let subject_style = if is_selected {
                    theme
                        .styles
                        .get_selected_style("message_list", &theme.colors)
                } else if !message.is_read {
                    Style::default()
                        .fg(theme.colors.message_list.subject_unread)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.colors.message_list.subject_read)
                };

                let sender_style = if is_selected {
                    theme
                        .styles
                        .get_selected_style("message_list", &theme.colors)
                } else {
                    Style::default().fg(theme.colors.message_list.sender)
                };

                let date_style = if is_selected {
                    theme
                        .styles
                        .get_selected_style("message_list", &theme.colors)
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
                    format!(
                        "{}...",
                        &message.subject[..available_width.saturating_sub(3)]
                    )
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
                    ]),
                };

                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        frame.render_stateful_widget(list, area, &mut self.state.clone());
    }

    /// Handle up arrow key press - move selection up with wraparound
    pub fn handle_up(&mut self) {
        let message_count = if self.search_active {
            self.filtered_messages.len()
        } else {
            self.messages.len()
        };

        if message_count == 0 {
            return;
        }

        let selected = match self.state.selected() {
            Some(i) => {
                if i > 0 {
                    Some(i - 1)
                } else {
                    Some(message_count - 1)
                }
            }
            None => Some(0),
        };
        self.state.select(selected);
    }

    /// Handle down arrow key press - move selection down with wraparound
    pub fn handle_down(&mut self) {
        let message_count = if self.search_active {
            self.filtered_messages.len()
        } else {
            self.messages.len()
        };

        if message_count == 0 {
            return;
        }

        let selected = match self.state.selected() {
            Some(i) => {
                if i < message_count - 1 {
                    Some(i + 1)
                } else {
                    Some(0)
                }
            }
            None => Some(0),
        };
        self.state.select(selected);
    }

    /// Handle enter key press - mark selected message as read
    pub fn handle_enter(&mut self) {
        if let Some(selected) = self.state.selected() {
            let messages_to_modify = if self.search_active {
                &mut self.filtered_messages
            } else {
                &mut self.messages
            };

            if let Some(message) = messages_to_modify.get_mut(selected) {
                // Mark message as read when selected
                message.is_read = true;
                // In the future, this will also trigger loading the message content
            }
        }
    }

    /// Get the currently selected message
    pub fn selected_message(&self) -> Option<&MessageItem> {
        let messages_to_check = if self.search_active {
            &self.filtered_messages
        } else {
            &self.messages
        };

        self.state.selected().and_then(|i| messages_to_check.get(i))
    }

    /// Mark the currently selected message as read
    pub fn mark_selected_as_read(&mut self) {
        if let Some(selected) = self.state.selected() {
            if let Some(message) = self.messages.get_mut(selected) {
                message.is_read = true;
            }
        }
    }

    /// Toggle the important status of the currently selected message
    pub fn toggle_selected_important(&mut self) {
        if let Some(selected) = self.state.selected() {
            if let Some(message) = self.messages.get_mut(selected) {
                message.is_important = !message.is_important;
            }
        }
    }

    // Threading and view mode methods

    /// Toggle between list and threaded view modes
    pub fn toggle_view_mode(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::List => ViewMode::Threaded,
            ViewMode::Threaded => ViewMode::List,
        };
        self.rebuild_view();
    }

    /// Set the view mode to list or threaded
    pub fn set_view_mode(&mut self, mode: ViewMode) {
        if self.view_mode != mode {
            self.view_mode = mode;
            self.rebuild_view();
        }
    }

    /// Get the current view mode
    pub fn current_view_mode(&self) -> ViewMode {
        self.view_mode
    }

    /// Get view mode display string for UI status indicators
    pub fn get_view_mode_display(&self) -> &'static str {
        match self.view_mode {
            ViewMode::List => "[Flat]",
            ViewMode::Threaded => "[Threaded]",
        }
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
        tracing::info!("rebuild_view called, current view_mode: {:?}, current_account: {:?}, current_folder: {:?}, messages count: {}", 
                      self.view_mode, self.current_account, self.current_folder, self.messages.len());
        match self.view_mode {
            ViewMode::List => self.build_flat_view(),
            ViewMode::Threaded => self.build_threaded_view(),
        }
        tracing::info!(
            "rebuild_view completed, messages count after rebuild: {}",
            self.messages.len()
        );
    }

    fn build_flat_view(&mut self) {
        // Only initialize sample messages if we don't have real messages loaded
        if self.current_account.is_none() && self.current_folder.is_none() {
            tracing::info!("No real messages loaded, using sample messages");
            self.initialize_sample_messages();
        } else {
            tracing::info!(
                "Using real messages for flat view, {} messages available",
                self.messages.len()
            );
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
            tracing::info!(
                "Using real messages for threaded view, {} messages available",
                self.messages.len()
            );
            // Apply threading algorithm to real messages
            self.apply_threading_to_real_messages();
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
                "thread1".to_string(),
            )
            .with_attachments()
            .with_thread_count(3)
            .expanded()
            .as_thread_root(),
            MessageItem::new_threaded(
                "Re: Project Update: Q1 Planning".to_string(),
                "Bob Smith".to_string(),
                "Today 10:30".to_string(),
                1,
                "thread1".to_string(),
            ),
            MessageItem::new_threaded(
                "Re: Project Update: Q1 Planning".to_string(),
                "Carol Davis".to_string(),
                "Today 11:45".to_string(),
                1,
                "thread1".to_string(),
            )
            .unread(),
            // Thread 2: Meeting Notes (collapsed)
            MessageItem::new_threaded(
                "Meeting Notes from Yesterday".to_string(),
                "David Wilson".to_string(),
                "Yesterday 16:45".to_string(),
                0,
                "thread2".to_string(),
            )
            .with_thread_count(2)
            .as_thread_root(),
            // Thread 3: Security Alert (expanded)
            MessageItem::new_threaded(
                "Security Alert: Password Change Required".to_string(),
                "IT Security".to_string(),
                "Mon 09:00".to_string(),
                0,
                "thread3".to_string(),
            )
            .unread()
            .important()
            .with_thread_count(4)
            .expanded()
            .as_thread_root(),
            MessageItem::new_threaded(
                "Re: Security Alert: Action Required".to_string(),
                "System Admin".to_string(),
                "Mon 09:15".to_string(),
                1,
                "thread3".to_string(),
            )
            .important(),
            MessageItem::new_threaded(
                "Re: Security Alert: Completed".to_string(),
                "Alice Johnson".to_string(),
                "Mon 09:30".to_string(),
                1,
                "thread3".to_string(),
            ),
            MessageItem::new_threaded(
                "Re: Security Alert: All Clear".to_string(),
                "IT Security".to_string(),
                "Mon 10:00".to_string(),
                1,
                "thread3".to_string(),
            ),
            // Standalone messages
            MessageItem::new(
                "Welcome to Comunicado!".to_string(),
                "Comunicado Team".to_string(),
                "Today 10:30".to_string(),
            )
            .unread()
            .important(),
            MessageItem::new(
                "Monthly Newsletter - Tech Updates".to_string(),
                "TechNews Daily".to_string(),
                "Yesterday 14:20".to_string(),
            )
            .unread(),
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

    /// Initialize with sample messages if no database is available (for demo purposes)
    pub fn ensure_sample_messages_if_no_database(&mut self) {
        if self.database.is_none() && self.messages.is_empty() {
            tracing::info!(
                "No database available and no messages loaded, initializing sample messages"
            );
            self.initialize_sample_messages();
            self.state.select(Some(0));
        }
    }

    /// Load messages from database for a specific account and folder
    pub async fn load_messages(
        &mut self,
        account_id: String,
        folder_name: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!(
            "MessageList::load_messages called with account_id: {}, folder_name: {}",
            account_id,
            folder_name
        );

        // Check if we're switching folders - if so, clear threading cache
        let folder_changed = self.current_account.as_ref() != Some(&account_id) 
            || self.current_folder.as_ref() != Some(&folder_name);
        
        if folder_changed {
            self.clear_threading_cache();
        }
        
        if let Some(ref database) = self.database {
            self.current_account = Some(account_id.clone());
            self.current_folder = Some(folder_name.clone());

            // Load messages from database
            tracing::info!("Loading messages from database...");
            let stored_messages = database
                .get_messages(&account_id, &folder_name, Some(100), None)
                .await?;
            tracing::info!("Loaded {} messages from database", stored_messages.len());
            
            // If no messages in database for this folder, it means it hasn't been synced yet
            // This happens for non-inbox folders that user is accessing for the first time
            if stored_messages.is_empty() {
                tracing::info!("No cached messages for folder '{}'. User can press 'R' to fetch from IMAP.", folder_name);
                // Note: The force refresh functionality (Ctrl+R) will fetch messages from IMAP
                // This provides the user control over when to sync folders
            }

            // Convert stored messages to MessageItems
            self.messages = stored_messages
                .into_iter()
                .map(|msg| MessageItem::from_stored_message(&msg))
                .collect();

            tracing::info!("Converted to {} MessageItems", self.messages.len());

            // Sort messages by date (newest first)
            self.messages.sort_by(|a, b| b.date.cmp(&a.date));

            // Reset selection
            if !self.messages.is_empty() {
                self.state.select(Some(0));
                tracing::info!(
                    "Selected first message, total messages: {}",
                    self.messages.len()
                );
            } else {
                self.state.select(None);
                tracing::warn!("No messages loaded, selection cleared");
            }
        } else {
            tracing::error!("Database not available in MessageList");
            return Err("Database not available".into());
        }

        // Schedule threading cache preloading in the background (non-blocking)
        if folder_changed {
            // Don't await this - let it run in background to avoid blocking the UI
            let account_id_clone = account_id.clone();
            let folder_name_clone = folder_name.clone();
            if let Some(database_clone) = self.database.clone() {
                tokio::spawn(async move {
                    tracing::info!("Background: Preloading threading cache for {}:{}", account_id_clone, folder_name_clone);
                    // This runs in background without blocking folder loading
                    match database_clone.get_messages(&account_id_clone, &folder_name_clone, Some(1000), None).await {
                        Ok(messages) => {
                            tracing::info!("Background: Cached {} messages for threading", messages.len());
                            // TODO: In a full implementation, we'd send the cached data back to the UI
                        }
                        Err(e) => {
                            tracing::warn!("Background: Failed to preload threading cache: {}", e);
                        }
                    }
                });
            }
        }

        Ok(())
    }

    /// Refresh current folder's messages
    pub async fn refresh_messages(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let (Some(account), Some(folder)) =
            (self.current_account.clone(), self.current_folder.clone())
        {
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

    // Search functionality methods

    /// Start search mode
    pub fn start_search(&mut self) {
        self.search_active = true;
        self.search_query.clear();
        self.filtered_messages.clear();
        self.search_results_count = 0;
        self.state.select(None);
    }

    /// End search mode and return to normal view
    pub fn end_search(&mut self) {
        self.search_active = false;
        self.search_query.clear();
        self.filtered_messages.clear();
        self.search_results_count = 0;
        // Reset selection to first message if available
        if !self.messages.is_empty() {
            self.state.select(Some(0));
        } else {
            self.state.select(None);
        }
    }

    /// Update search query and filter messages
    pub fn update_search(&mut self, query: String) {
        self.search_query = query.to_lowercase();
        self.filter_messages();
    }

    /// Get current search query
    pub fn search_query(&self) -> &str {
        &self.search_query
    }

    /// Check if search is active
    pub fn is_search_active(&self) -> bool {
        self.search_active
    }

    /// Get search results count
    pub fn search_results_count(&self) -> usize {
        self.search_results_count
    }

    /// Filter messages based on current search query
    fn filter_messages(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_messages = self.messages.clone();
        } else {
            self.filtered_messages = self
                .messages
                .iter()
                .filter(|message| self.message_matches_search(message))
                .cloned()
                .collect();
        }

        self.search_results_count = self.filtered_messages.len();

        // Reset selection to first result if available
        if !self.filtered_messages.is_empty() {
            self.state.select(Some(0));
        } else {
            self.state.select(None);
        }
    }

    /// Check if a message matches the current search query
    fn message_matches_search(&self, message: &MessageItem) -> bool {
        if self.search_query.is_empty() {
            return true;
        }

        // Search in subject, sender, and date
        let search_in = format!(
            "{} {} {}",
            message.subject.to_lowercase(),
            message.sender.to_lowercase(),
            message.date.to_lowercase()
        );

        // Support both simple substring search and space-separated terms
        let query_terms: Vec<&str> = self.search_query.split_whitespace().collect();

        if query_terms.is_empty() {
            return true;
        }

        // All terms must match (AND logic)
        query_terms.iter().all(|term| search_in.contains(term))
    }

    /// Get status text for search mode
    pub fn get_search_status(&self) -> String {
        if self.search_active {
            if self.search_query.is_empty() {
                "Search: (type to search)".to_string()
            } else {
                format!(
                    "Search: {} ({} results)",
                    self.search_query, self.search_results_count
                )
            }
        } else {
            String::new()
        }
    }

    /// Set the selected message index
    pub fn set_selected_index(&mut self, index: usize) {
        let message_count = if self.search_active {
            self.filtered_messages.len()
        } else {
            self.messages.len()
        };

        if index < message_count {
            self.state.select(Some(index));
        } else if message_count > 0 {
            // If index is out of bounds, select the last message
            self.state.select(Some(message_count - 1));
        } else {
            self.state.select(None);
        }
    }

    /// Clear the threading cache (call when switching folders)
    pub fn clear_threading_cache(&mut self) {
        self.threading_cache.clear();
        self.threading_cache_key = None;
        tracing::info!("Threading cache cleared");
    }

    /// Preload threading data into cache (call this asynchronously when folder changes)
    pub async fn preload_threading_cache(&mut self) {
        if let Some(ref database) = self.database {
            if let (Some(ref account_id), Some(ref folder_name)) =
                (self.current_account.as_ref(), self.current_folder.as_ref())
            {
                let cache_key = format!("{}:{}", account_id, folder_name);
                
                // Only load if not already cached
                if self.threading_cache_key.as_ref() != Some(&cache_key) {
                    tracing::info!("Preloading threading cache for {}", cache_key);
                    
                    match database.get_messages(account_id, folder_name, Some(1000), None).await {
                        Ok(stored_messages) => {
                            tracing::info!("Cached {} messages for threading", stored_messages.len());
                            self.threading_cache.insert(cache_key.clone(), stored_messages);
                            self.threading_cache_key = Some(cache_key);
                        }
                        Err(e) => {
                            tracing::error!("Failed to preload threading cache: {}", e);
                        }
                    }
                }
            }
        }
    }

    /// Apply threading algorithm to real messages using cached data (non-blocking)
    fn apply_threading_to_real_messages(&mut self) {
        tracing::info!(
            "Applying threading algorithm to {} real messages",
            self.messages.len()
        );

        // Step 1: Use cached stored messages for threading (non-blocking)
        if let (Some(ref account_id), Some(ref folder_name)) =
            (self.current_account.as_ref(), self.current_folder.as_ref())
        {
            let cache_key = format!("{}:{}", account_id, folder_name);
            
            // Try to get stored messages from cache first
            if let Some(stored_messages) = self.threading_cache.get(&cache_key) {
                tracing::info!(
                    "Using cached {} stored messages for threading",
                    stored_messages.len()
                );

                // Convert to EmailMessage objects
                let email_messages: Vec<EmailMessage> = stored_messages
                    .iter()
                    .filter_map(|stored| Self::stored_message_to_email_message(stored))
                    .collect();

                tracing::info!(
                    "Converted {} stored messages to EmailMessage objects",
                    email_messages.len()
                );

                // Apply threading algorithm using the ThreadingEngine
                let threads = self.threading_engine.thread_messages(email_messages);
                tracing::info!("Threading algorithm produced {} threads", threads.len());

                // Convert threads back to MessageItems for display
                self.messages = Self::threads_to_message_items(threads, &stored_messages);
                tracing::info!(
                    "Converted threads to {} MessageItems for display",
                    self.messages.len()
                );

                // Sort threads by latest message date (newest first)
                self.messages.sort_by(|a, b| b.date.cmp(&a.date));
            } else {
                tracing::warn!("Threading cache not available for {}. Threading disabled until cache is populated.", cache_key);
                // Fall back to flat view without threading
                self.messages.sort_by(|a, b| b.date.cmp(&a.date));
            }
        }
    }

    /// Convert StoredMessage to EmailMessage for threading
    fn stored_message_to_email_message(stored: &StoredMessage) -> Option<EmailMessage> {
        // Create MessageId from stored message_id
        let message_id = stored
            .message_id
            .as_ref()
            .and_then(|id| MessageId::parse(id).ok())
            .unwrap_or_else(|| MessageId::new(format!("local-{}", stored.id)));

        // Determine sender name/address
        let sender = if let Some(ref name) = stored.from_name {
            format!("{} <{}>", name, stored.from_addr)
        } else {
            stored.from_addr.clone()
        };

        // Get recipients
        let mut recipients = stored.to_addrs.clone();
        recipients.extend(stored.cc_addrs.clone());

        // Get content (prefer text over HTML for threading purposes)
        let content = stored
            .body_text
            .as_ref()
            .or(stored.body_html.as_ref())
            .map(|s| s.clone())
            .unwrap_or_default();

        // Create EmailMessage
        let mut email_message = EmailMessage::new(
            message_id,
            stored.subject.clone(),
            sender,
            recipients,
            content,
            stored.date,
        );

        // Set threading information
        if let Some(ref in_reply_to) = stored.in_reply_to {
            if let Ok(reply_to_id) = MessageId::parse(in_reply_to) {
                email_message.set_in_reply_to(reply_to_id);
            }
        }

        if !stored.references.is_empty() {
            let references_str = stored.references.join(" ");
            email_message.set_references(references_str);
        }

        // Set message state
        email_message.set_read(stored.flags.contains(&"\\Seen".to_string()));
        email_message.set_important(stored.flags.contains(&"\\Flagged".to_string()));
        email_message.set_attachments(!stored.attachments.is_empty());

        Some(email_message)
    }

    /// Convert threads back to MessageItems for display
    fn threads_to_message_items(
        threads: Vec<EmailThread>,
        stored_messages: &[StoredMessage],
    ) -> Vec<MessageItem> {
        let mut message_items = Vec::new();

        // Create a lookup map for stored messages by message ID
        let stored_lookup: std::collections::HashMap<String, &StoredMessage> = stored_messages
            .iter()
            .filter_map(|stored| stored.message_id.as_ref().map(|id| (id.clone(), stored)))
            .collect();

        for thread in threads {
            Self::add_thread_to_message_items(&thread, &mut message_items, &stored_lookup, 0, true);
        }

        message_items
    }

    /// Recursively add thread messages to MessageItems list
    fn add_thread_to_message_items(
        thread: &EmailThread,
        items: &mut Vec<MessageItem>,
        stored_lookup: &std::collections::HashMap<String, &StoredMessage>,
        depth: usize,
        is_root: bool,
    ) {
        let root_message = thread.root_message();
        let message_id_str = root_message.message_id().as_str();

        // Find corresponding stored message for additional data
        if let Some(stored) = stored_lookup.get(message_id_str) {
            let date_str = MessageItem::format_message_date(stored.date);
            let sender = if let Some(ref name) = stored.from_name {
                name.clone()
            } else {
                stored.from_addr.clone()
            };

            let thread_id = stored
                .thread_id
                .clone()
                .unwrap_or_else(|| format!("thread-{}", stored.id));

            let mut message_item = if depth == 0 {
                // Root message
                MessageItem::new_threaded(
                    stored.subject.clone(),
                    sender,
                    date_str,
                    depth,
                    thread_id.clone(),
                )
                .with_thread_count(thread.message_count())
                .as_thread_root()
            } else {
                // Child message
                MessageItem::new_threaded(
                    stored.subject.clone(),
                    sender,
                    date_str,
                    depth,
                    thread_id.clone(),
                )
            };

            // Set message state
            if !stored.flags.contains(&"\\Seen".to_string()) {
                message_item = message_item.unread();
            }
            if stored.flags.contains(&"\\Flagged".to_string()) {
                message_item = message_item.important();
            }
            if !stored.attachments.is_empty() {
                message_item = message_item.with_attachments();
            }

            // For root messages with children, expand by default
            if is_root && thread.has_children() {
                message_item = message_item.expanded();
            }

            // Set database ID for message loading
            message_item.message_id = Some(stored.id);

            items.push(message_item);

            // Add children if thread is expanded (or if we're showing all for now)
            if is_root && thread.has_children() {
                for child_thread in thread.children() {
                    Self::add_thread_to_message_items(
                        child_thread,
                        items,
                        stored_lookup,
                        depth + 1,
                        false,
                    );
                }
            }
        }
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
