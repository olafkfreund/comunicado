use chrono::{DateTime, Utc};
use std::cmp::Ordering;

/// Unique identifier for email messages
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MessageId {
    id: String,
}

impl MessageId {
    /// Create a new message ID
    pub fn new(id: String) -> Self {
        Self { id }
    }

    /// Parse message ID from RFC format (with or without angle brackets)
    pub fn parse(id: &str) -> Result<Self, String> {
        let cleaned_id = if id.starts_with('<') && id.ends_with('>') {
            id[1..id.len() - 1].to_string()
        } else {
            id.to_string()
        };

        if cleaned_id.is_empty() {
            Err("Message ID cannot be empty".to_string())
        } else {
            Ok(Self::new(cleaned_id))
        }
    }

    /// Get the message ID as a string reference
    pub fn as_str(&self) -> &str {
        &self.id
    }
}

impl std::fmt::Display for MessageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}

/// Email message representation for threading and display
#[derive(Debug, Clone)]
pub struct EmailMessage {
    message_id: MessageId,
    subject: String,
    sender: String,
    recipients: Vec<String>,
    content: String,
    timestamp: DateTime<Utc>,
    in_reply_to: Option<MessageId>,
    references: Option<String>,
    is_read: bool,
    is_important: bool,
    has_attachments: bool,
}

impl EmailMessage {
    /// Create a new email message with the provided details
    pub fn new(
        message_id: MessageId,
        subject: String,
        sender: String,
        recipients: Vec<String>,
        content: String,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self {
            message_id,
            subject,
            sender,
            recipients,
            content,
            timestamp,
            in_reply_to: None,
            references: None,
            is_read: false,
            is_important: false,
            has_attachments: false,
        }
    }

    // Getters
    /// Get the message ID
    pub fn message_id(&self) -> &MessageId {
        &self.message_id
    }

    /// Get the message subject
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// Get the message sender
    pub fn sender(&self) -> &str {
        &self.sender
    }

    /// Get the message recipients
    pub fn recipients(&self) -> &[String] {
        &self.recipients
    }

    /// Get the message content
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get the message timestamp
    pub fn timestamp(&self) -> &DateTime<Utc> {
        &self.timestamp
    }

    /// Get the message this is in reply to
    pub fn in_reply_to(&self) -> Option<&MessageId> {
        self.in_reply_to.as_ref()
    }

    /// Get the message references header
    pub fn references(&self) -> Option<&String> {
        self.references.as_ref()
    }

    /// Check if the message has been read
    pub fn is_read(&self) -> bool {
        self.is_read
    }

    /// Check if the message is marked as important
    pub fn is_important(&self) -> bool {
        self.is_important
    }

    /// Check if the message has attachments
    pub fn has_attachments(&self) -> bool {
        self.has_attachments
    }

    // Setters
    /// Set the message this is in reply to
    pub fn set_in_reply_to(&mut self, reply_to: MessageId) {
        self.in_reply_to = Some(reply_to);
    }

    /// Set the message references header
    pub fn set_references(&mut self, references: String) {
        self.references = Some(references);
    }

    /// Set the read status of the message
    pub fn set_read(&mut self, is_read: bool) {
        self.is_read = is_read;
    }

    /// Set the importance flag of the message
    pub fn set_important(&mut self, is_important: bool) {
        self.is_important = is_important;
    }

    /// Set whether the message has attachments
    pub fn set_attachments(&mut self, has_attachments: bool) {
        self.has_attachments = has_attachments;
    }

    /// Update the message sender
    pub fn set_sender(&mut self, sender: String) {
        self.sender = sender;
    }

    /// Check if this message is a reply to another message
    pub fn is_reply(&self) -> bool {
        self.in_reply_to.is_some()
            || self.subject.to_lowercase().starts_with("re:")
            || self.subject.to_lowercase().starts_with("fwd:")
    }

    /// Get normalized subject for threading (removes Re:, Fwd:, etc.)
    pub fn normalized_subject(&self) -> String {
        crate::email::thread::EmailThread::normalize_subject(&self.subject)
    }

    /// Extract message IDs from References header
    pub fn get_reference_ids(&self) -> Vec<MessageId> {
        if let Some(refs) = &self.references {
            refs.split_whitespace()
                .filter_map(|id| MessageId::parse(id).ok())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get all related message IDs (in-reply-to + references)
    pub fn get_all_related_ids(&self) -> Vec<MessageId> {
        let mut ids = self.get_reference_ids();
        if let Some(reply_to) = &self.in_reply_to {
            ids.push(reply_to.clone());
        }
        ids
    }

    /// Check if this message could be part of the same thread as another
    pub fn could_be_threaded_with(&self, other: &EmailMessage) -> bool {
        // Same normalized subject
        if self.normalized_subject() == other.normalized_subject() {
            return true;
        }

        // Direct reply relationship
        if let Some(reply_to) = &self.in_reply_to {
            if reply_to == &other.message_id {
                return true;
            }
        }

        if let Some(other_reply_to) = &other.in_reply_to {
            if other_reply_to == &self.message_id {
                return true;
            }
        }

        // References relationship
        let self_refs = self.get_reference_ids();
        let other_refs = other.get_reference_ids();

        if self_refs.contains(&other.message_id) || other_refs.contains(&self.message_id) {
            return true;
        }

        // Shared references
        for self_ref in &self_refs {
            if other_refs.contains(self_ref) {
                return true;
            }
        }

        false
    }
}

impl PartialEq for EmailMessage {
    fn eq(&self, other: &Self) -> bool {
        self.message_id == other.message_id
    }
}

impl Eq for EmailMessage {}

impl PartialOrd for EmailMessage {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EmailMessage {
    fn cmp(&self, other: &Self) -> Ordering {
        // Default ordering: by timestamp (newest first)
        other.timestamp.cmp(&self.timestamp)
    }
}
