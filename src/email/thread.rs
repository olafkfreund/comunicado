use crate::email::message::{EmailMessage, MessageId};
use chrono::{DateTime, Utc};

/// Statistics about an email thread
#[derive(Debug, Clone)]
pub struct ThreadStatistics {
    pub total_messages: usize,
    pub max_depth: usize,
    pub unique_senders: usize,
    pub date_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    pub unread_count: usize,
}

/// Represents a threaded email conversation
#[derive(Debug, Clone)]
pub struct EmailThread {
    root_message: EmailMessage,
    children: Vec<EmailThread>,
    is_expanded: bool,
    depth: usize,
}

impl EmailThread {
    /// Create a new thread with a root message
    pub fn new(root_message: EmailMessage) -> Self {
        Self {
            root_message,
            children: Vec::new(),
            is_expanded: false,
            depth: 0,
        }
    }

    /// Add a reply to this thread
    pub fn add_reply(&mut self, reply: EmailMessage) {
        // Try to find the best parent for this reply
        if let Some(parent_thread) = self.find_best_parent(&reply) {
            let mut child_thread = EmailThread::new(reply);
            child_thread.depth = parent_thread.depth + 1;
            parent_thread.children.push(child_thread);
        } else {
            // Add as direct child of root
            let mut child_thread = EmailThread::new(reply);
            child_thread.depth = self.depth + 1;
            self.children.push(child_thread);
        }
    }

    /// Find the best parent thread for a reply message
    fn find_best_parent(&mut self, reply: &EmailMessage) -> Option<&mut EmailThread> {
        // Check if this is a direct reply to the root message
        if let Some(in_reply_to) = reply.in_reply_to() {
            if in_reply_to == self.root_message.message_id() {
                return Some(self);
            }
        }

        // Check children - we need to handle the borrow checker carefully
        let reply_to_id = reply.in_reply_to().cloned();

        for child in &mut self.children {
            // First check if this child is the direct parent
            if let Some(ref reply_to) = reply_to_id {
                if reply_to == child.root_message.message_id() {
                    return Some(child);
                }
            }

            // Then check recursively
            if let Some(parent) = child.find_best_parent(reply) {
                return Some(parent);
            }
        }

        None
    }

    /// Get the total number of messages in this thread
    pub fn message_count(&self) -> usize {
        1 + self
            .children
            .iter()
            .map(|child| child.message_count())
            .sum::<usize>()
    }

    /// Get the maximum depth of this thread
    pub fn depth(&self) -> usize {
        if self.children.is_empty() {
            self.depth
        } else {
            self.children
                .iter()
                .map(|child| child.depth())
                .max()
                .unwrap_or(self.depth)
        }
    }

    /// Check if this thread has child messages
    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    /// Get the subject of the thread (from root message)
    pub fn subject(&self) -> &str {
        self.root_message.subject()
    }

    /// Get the normalized subject for grouping
    pub fn normalized_subject(&self) -> String {
        Self::normalize_subject(self.root_message.subject())
    }

    /// Normalize subject for threading (remove Re:, Fwd:, etc.)
    pub fn normalize_subject(subject: &str) -> String {
        let mut normalized = subject.trim().to_string();

        // Remove common prefixes (case insensitive)
        loop {
            let lower = normalized.to_lowercase();
            let mut changed = false;

            // Remove Re: prefix
            if lower.starts_with("re:") {
                normalized = normalized[3..].trim().to_string();
                changed = true;
            }
            // Remove Fwd: prefix
            else if lower.starts_with("fwd:") || lower.starts_with("fw:") {
                let prefix_len = if lower.starts_with("fwd:") { 4 } else { 3 };
                normalized = normalized[prefix_len..].trim().to_string();
                changed = true;
            }
            // Remove [prefix] patterns
            else if normalized.starts_with('[') {
                if let Some(end_bracket) = normalized.find(']') {
                    normalized = normalized[end_bracket + 1..].trim().to_string();
                    changed = true;
                }
            }

            if !changed {
                break;
            }
        }

        normalized
    }

    /// Check if thread is expanded
    pub fn is_expanded(&self) -> bool {
        self.is_expanded
    }

    /// Set thread expansion state
    pub fn set_expanded(&mut self, expanded: bool) {
        self.is_expanded = expanded;
    }

    /// Toggle thread expansion state
    pub fn toggle_expanded(&mut self) {
        self.is_expanded = !self.is_expanded;
    }

    /// Get thread statistics
    pub fn get_statistics(&self) -> ThreadStatistics {
        let mut all_messages = Vec::new();
        self.collect_all_messages(&mut all_messages);

        let total_messages = all_messages.len();
        let max_depth = self.depth();

        let mut unique_senders = std::collections::HashSet::new();
        let mut earliest_date: Option<DateTime<Utc>> = None;
        let mut latest_date: Option<DateTime<Utc>> = None;
        let mut unread_count = 0;

        for message in &all_messages {
            unique_senders.insert(message.sender());

            let timestamp = *message.timestamp();
            earliest_date = Some(match earliest_date {
                Some(date) => date.min(timestamp),
                None => timestamp,
            });
            latest_date = Some(match latest_date {
                Some(date) => date.max(timestamp),
                None => timestamp,
            });

            if !message.is_read() {
                unread_count += 1;
            }
        }

        let date_range = match (earliest_date, latest_date) {
            (Some(start), Some(end)) => Some((start, end)),
            _ => None,
        };

        ThreadStatistics {
            total_messages,
            max_depth,
            unique_senders: unique_senders.len(),
            date_range,
            unread_count,
        }
    }

    /// Collect all messages in this thread (depth-first)
    fn collect_all_messages<'a>(&'a self, messages: &mut Vec<&'a EmailMessage>) {
        messages.push(&self.root_message);
        for child in &self.children {
            child.collect_all_messages(messages);
        }
    }

    /// Get all messages in this thread (owned)
    pub fn get_all_messages(&self) -> Vec<EmailMessage> {
        let mut messages = Vec::new();
        self.collect_all_messages_owned(&mut messages);
        messages
    }

    fn collect_all_messages_owned(&self, messages: &mut Vec<EmailMessage>) {
        messages.push(self.root_message.clone());
        for child in &self.children {
            child.collect_all_messages_owned(messages);
        }
    }

    /// Get root message reference
    pub fn root_message(&self) -> &EmailMessage {
        &self.root_message
    }

    /// Get children threads
    pub fn children(&self) -> &[EmailThread] {
        &self.children
    }

    /// Get mutable children threads
    pub fn children_mut(&mut self) -> &mut Vec<EmailThread> {
        &mut self.children
    }

    /// Find a message by ID in this thread
    pub fn find_message(&self, message_id: &MessageId) -> Option<&EmailMessage> {
        if self.root_message.message_id() == message_id {
            return Some(&self.root_message);
        }

        for child in &self.children {
            if let Some(message) = child.find_message(message_id) {
                return Some(message);
            }
        }

        None
    }

    /// Get the latest message timestamp in this thread
    pub fn latest_timestamp(&self) -> DateTime<Utc> {
        let mut latest = *self.root_message.timestamp();

        for child in &self.children {
            let child_latest = child.latest_timestamp();
            if child_latest > latest {
                latest = child_latest;
            }
        }

        latest
    }

    /// Get the earliest message timestamp in this thread
    pub fn earliest_timestamp(&self) -> DateTime<Utc> {
        let mut earliest = *self.root_message.timestamp();

        for child in &self.children {
            let child_earliest = child.earliest_timestamp();
            if child_earliest < earliest {
                earliest = child_earliest;
            }
        }

        earliest
    }

    /// Check if thread contains unread messages
    pub fn has_unread(&self) -> bool {
        if !self.root_message.is_read() {
            return true;
        }

        self.children.iter().any(|child| child.has_unread())
    }

    /// Mark all messages in thread as read
    pub fn mark_all_read(&mut self) {
        // Note: This would need to update the actual message storage
        // For now, it's a placeholder for the interface
    }

    /// Get visual representation info for UI rendering
    pub fn get_render_info(&self) -> ThreadRenderInfo {
        ThreadRenderInfo {
            subject: self.normalized_subject(),
            sender: self.root_message.sender().to_string(),
            timestamp: self.latest_timestamp(),
            message_count: self.message_count(),
            has_unread: self.has_unread(),
            is_expanded: self.is_expanded,
            depth: self.depth,
        }
    }
}

/// Information needed for rendering a thread in the UI
#[derive(Debug, Clone)]
pub struct ThreadRenderInfo {
    pub subject: String,
    pub sender: String,
    pub timestamp: DateTime<Utc>,
    pub message_count: usize,
    pub has_unread: bool,
    pub is_expanded: bool,
    pub depth: usize,
}

impl PartialEq for EmailThread {
    fn eq(&self, other: &Self) -> bool {
        self.root_message == other.root_message
    }
}

impl Eq for EmailThread {}
