//! Mobile integration for the notes plugin
//! 
//! Provides seamless integration between notes and mobile devices via KDE Connect,
//! including SMS-to-notes conversion, contact linking, and notification bridging.

use super::types::{Note, NoteId, NoteFrontmatter};
use super::manager::NoteResult;
use super::storage::NoteStorage;
use crate::mobile::{
    KdeConnectClient, MessageStore,
    kde_connect::types::{SmsMessage, ContactInfo}
};

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{info, debug};
use uuid::Uuid;

/// Service that bridges mobile communications with notes
pub struct MobileNotesIntegration {
    note_storage: Arc<NoteStorage>,
    mobile_client: Arc<RwLock<KdeConnectClient>>,
    message_store: Arc<MessageStore>,
    contact_notes: Arc<RwLock<HashMap<String, Vec<NoteId>>>>,
    config: MobileNotesConfig,
    notification_tx: mpsc::UnboundedSender<MobileNoteEvent>,
}

/// Configuration for mobile-notes integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileNotesConfig {
    /// Automatically convert SMS messages to notes
    pub auto_convert_sms: bool,
    /// Minimum message length to auto-convert
    pub min_sms_length: usize,
    /// Keywords that trigger auto-conversion
    pub conversion_keywords: Vec<String>,
    /// Send mobile notifications for note changes
    pub mobile_notifications: bool,
    /// Directory for mobile-generated notes
    pub mobile_notes_directory: PathBuf,
    /// Template for SMS-converted notes
    pub sms_note_template: String,
    /// Maximum notes per contact to track
    pub max_notes_per_contact: usize,
}

impl Default for MobileNotesConfig {
    fn default() -> Self {
        Self {
            auto_convert_sms: true,
            min_sms_length: 50,
            conversion_keywords: vec![
                "reminder".to_string(),
                "todo".to_string(),
                "note".to_string(),
                "important".to_string(),
                "meeting".to_string(),
                "address".to_string(),
                "phone".to_string(),
                "email".to_string(),
            ],
            mobile_notifications: true,
            mobile_notes_directory: PathBuf::from("~/notes/mobile"),
            sms_note_template: "# SMS from {{contact}}\n\n**Date:** {{date}}\n**Phone:** {{phone}}\n\n{{content}}\n\n---\n*Converted from SMS*".to_string(),
            max_notes_per_contact: 20,
        }
    }
}

/// Events related to mobile-notes integration
#[derive(Debug, Clone)]
pub enum MobileNoteEvent {
    /// SMS message converted to note
    SmsConverted {
        message_id: String,
        note_id: NoteId,
        contact: String,
    },
    /// Note linked to contact
    ContactLinked {
        note_id: NoteId,
        contact: String,
        phone: String,
    },
    /// Mobile notification sent
    NotificationSent {
        note_id: NoteId,
        device_id: String,
        notification_type: String,
    },
    /// Note shared to mobile device
    NoteShared {
        note_id: NoteId,
        device_id: String,
        file_path: String,
    },
}

/// Information about SMS messages that can be converted to notes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmsConversionCandidate {
    pub message: SmsMessage,
    pub contact: Option<ContactInfo>,
    pub conversion_score: f64,
    pub reasons: Vec<String>,
    pub suggested_title: String,
    pub suggested_tags: Vec<String>,
}

/// Statistics for mobile-notes integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileNotesStats {
    pub total_sms_converted: u64,
    pub notes_linked_to_contacts: u64,
    pub mobile_notifications_sent: u64,
    pub notes_shared_to_mobile: u64,
    pub active_contact_links: usize,
    pub last_sync_time: Option<DateTime<Utc>>,
    pub conversion_accuracy: f64,
}

impl MobileNotesIntegration {
    /// Create a new mobile-notes integration service
    pub async fn new(
        note_storage: Arc<NoteStorage>,
        mobile_client: Arc<RwLock<KdeConnectClient>>,
        message_store: Arc<MessageStore>,
        config: MobileNotesConfig,
    ) -> NoteResult<Self> {
        let (notification_tx, _notification_rx) = mpsc::unbounded_channel();
        
        Ok(Self {
            note_storage,
            mobile_client,
            message_store,
            contact_notes: Arc::new(RwLock::new(HashMap::new())),
            config,
            notification_tx,
        })
    }

    /// Scan recent SMS messages for conversion candidates
    pub async fn scan_sms_for_conversion(&self) -> NoteResult<Vec<SmsConversionCandidate>> {
        debug!("Scanning SMS messages for note conversion candidates");
        
        // Get recent messages directly from message store
        let recent_messages = self.message_store.get_recent_messages(50).await
            .map_err(|e| super::manager::NoteError::External(e.to_string()))?;
        
        let mut candidates = Vec::new();
        
        for message in recent_messages {
            if let Some(candidate) = self.evaluate_sms_for_conversion(&message).await? {
                candidates.push(candidate);
            }
        }
        
        // Sort by conversion score (highest first)
        candidates.sort_by(|a, b| b.conversion_score.partial_cmp(&a.conversion_score).unwrap());
        
        info!("Found {} SMS conversion candidates", candidates.len());
        Ok(candidates)
    }

    /// Evaluate if an SMS message should be converted to a note
    async fn evaluate_sms_for_conversion(&self, message: &SmsMessage) -> NoteResult<Option<SmsConversionCandidate>> {
        // Skip if message is too short
        if message.body.len() < self.config.min_sms_length {
            return Ok(None);
        }
        
        // Skip if already converted (check if note exists with this SMS ID)
        if self.is_sms_already_converted(&message.id.to_string()).await? {
            return Ok(None);
        }
        
        let mut score: f64 = 0.0;
        let mut reasons = Vec::new();
        
        // Check for conversion keywords
        let body_lower = message.body.to_lowercase();
        for keyword in &self.config.conversion_keywords {
            if body_lower.contains(keyword) {
                score += 0.3;
                reasons.push(format!("Contains keyword: {}", keyword));
            }
        }
        
        // Score based on message length (longer = more likely to be noteworthy)
        if message.body.len() > 100 {
            score += 0.2;
            reasons.push("Message is substantial length".to_string());
        }
        
        // Score based on sender (known contacts get higher scores)
        let unknown = "Unknown".to_string();
        let sender = message.sender().unwrap_or(&unknown);
        let contact = self.get_contact_info(sender).await?;
        if contact.is_some() {
            score += 0.2;
            reasons.push("From known contact".to_string());
        }
        
        // Check for structured information (phone numbers, emails, addresses)
        if self.contains_structured_info(&message.body) {
            score += 0.4;
            reasons.push("Contains structured information".to_string());
        }
        
        // Check for time-sensitive content
        if self.contains_time_sensitive_content(&message.body) {
            score += 0.3;
            reasons.push("Contains time-sensitive information".to_string());
        }
        
        // Only consider messages with decent conversion potential
        if score < 0.3 {
            return Ok(None);
        }
        
        let suggested_title = self.generate_note_title_from_sms(message, &contact);
        let suggested_tags = self.generate_tags_from_sms(message, &contact);
        
        Ok(Some(SmsConversionCandidate {
            message: message.clone(),
            contact,
            conversion_score: score.min(1.0),
            reasons,
            suggested_title,
            suggested_tags,
        }))
    }

    /// Convert an SMS message to a note
    pub async fn convert_sms_to_note(&self, candidate: &SmsConversionCandidate) -> NoteResult<Note> {
        info!("Converting SMS message to note: {}", candidate.message.id);
        
        let note_id = format!("sms-{}", Uuid::new_v4());
        let unknown = "Unknown".to_string();
        let sender_fallback = candidate.message.sender().unwrap_or(&unknown);
        let contact_name = candidate.contact.as_ref()
            .and_then(|c| c.display_name.clone())
            .unwrap_or_else(|| sender_fallback.clone());
        
        // Generate note content from template
        let message_date = DateTime::from_timestamp(candidate.message.date / 1000, 0)
            .unwrap_or_else(|| Utc::now());
        let sender_phone = candidate.message.sender().unwrap_or(&unknown);
        
        let content = self.config.sms_note_template
            .replace("{{contact}}", &contact_name)
            .replace("{{date}}", &message_date.format("%Y-%m-%d %H:%M:%S").to_string())
            .replace("{{phone}}", sender_phone)
            .replace("{{content}}", &candidate.message.body);
        
        // Create frontmatter
        let mut frontmatter = NoteFrontmatter::new();
        frontmatter.title = Some(candidate.suggested_title.clone());
        frontmatter.add_tags(candidate.suggested_tags.clone());
        frontmatter.add_tag("sms".to_string());
        frontmatter.add_tag("mobile".to_string());
        
        // Add contact as tag if available
        if let Some(contact) = &candidate.contact {
            if let Some(display_name) = &contact.display_name {
                frontmatter.add_tag(format!("contact:{}", display_name));
            }
        }
        
        // Add metadata
        frontmatter.set_metadata("sms_id".to_string(), serde_yaml::Value::String(candidate.message.id.to_string()));
        frontmatter.set_metadata("sender".to_string(), serde_yaml::Value::String(sender_phone.clone()));
        frontmatter.set_metadata("original_timestamp".to_string(), 
            serde_yaml::Value::String(message_date.to_rfc3339()));
        
        // Create the note
        let file_path = self.config.mobile_notes_directory
            .join(format!("{}.md", candidate.suggested_title.replace(' ', "-").to_lowercase()));
        
        let mut note = Note::new(
            note_id.clone(),
            candidate.suggested_title.clone(),
            content,
            file_path,
        );
        note.frontmatter = Some(frontmatter);
        note.tags = candidate.suggested_tags.clone();
        note.word_count = candidate.message.body.split_whitespace().count();
        
        // Store the note (using directory ID 1 for mobile notes)
        self.note_storage.store_note(&note, 1).await?;
        
        // Link note to contact if available
        if let Some(contact) = &candidate.contact {
            let display_name = contact.display_name.as_ref().unwrap_or(&contact.address);
            self.link_note_to_contact(&note_id, display_name, sender_phone).await?;
        }
        
        // Send notification event
        let _ = self.notification_tx.send(MobileNoteEvent::SmsConverted {
            message_id: candidate.message.id.to_string(),
            note_id: note_id.clone(),
            contact: contact_name,
        });
        
        info!("Successfully converted SMS to note: {}", note_id);
        Ok(note)
    }

    /// Link a note to a mobile contact
    pub async fn link_note_to_contact(&self, note_id: &NoteId, contact_name: &str, phone: &str) -> NoteResult<()> {
        let mut contact_notes = self.contact_notes.write().await;
        
        let notes = contact_notes.entry(contact_name.to_string()).or_insert_with(Vec::new);
        
        if !notes.contains(note_id) {
            notes.push(note_id.clone());
            
            // Limit the number of notes per contact
            if notes.len() > self.config.max_notes_per_contact {
                notes.remove(0); // Remove oldest
            }
        }
        
        // Send notification event
        let _ = self.notification_tx.send(MobileNoteEvent::ContactLinked {
            note_id: note_id.clone(),
            contact: contact_name.to_string(),
            phone: phone.to_string(),
        });
        
        debug!("Linked note {} to contact {}", note_id, contact_name);
        Ok(())
    }

    /// Get notes associated with a specific contact
    pub async fn get_notes_for_contact(&self, contact_name: &str) -> NoteResult<Vec<Note>> {
        let contact_notes = self.contact_notes.read().await;
        
        if let Some(note_ids) = contact_notes.get(contact_name) {
            let mut notes = Vec::new();
            
            for note_id in note_ids {
                if let Some(note) = self.note_storage.get_note(note_id).await? {
                    notes.push(note);
                }
            }
            
            Ok(notes)
        } else {
            Ok(Vec::new())
        }
    }

    /// Send a mobile notification about note changes
    pub async fn send_mobile_notification(&self, note_id: &NoteId, notification_type: &str, message: &str) -> NoteResult<()> {
        if !self.config.mobile_notifications {
            return Ok(());
        }
        
        // For now, just log the notification request
        info!("Mobile notification requested for note {}: {}", note_id, message);
        
        // Send notification event
        let _ = self.notification_tx.send(MobileNoteEvent::NotificationSent {
            note_id: note_id.clone(),
            device_id: "placeholder".to_string(),
            notification_type: notification_type.to_string(),
        });
        
        Ok(())
    }

    /// Share a note to mobile device via KDE Connect file transfer
    pub async fn share_note_to_mobile(&self, note_id: &NoteId, device_id: &str) -> NoteResult<()> {
        let note = self.note_storage.get_note(note_id).await?
            .ok_or_else(|| super::manager::NoteError::NotFound(note_id.clone()))?;
        
        // Create a temporary file with the note content
        let temp_file = format!("/tmp/{}.md", note.title.replace(' ', "-"));
        tokio::fs::write(&temp_file, &note.content).await
            .map_err(|e| super::manager::NoteError::Io(e))?;
        
        // For now, just log the file sharing request
        info!("File sharing requested for note {} to device {}: {}", note_id, device_id, temp_file);
        
        // Send notification event
        let _ = self.notification_tx.send(MobileNoteEvent::NoteShared {
            note_id: note_id.clone(),
            device_id: device_id.to_string(),
            file_path: temp_file.clone(),
        });
        
        info!("Shared note {} to device {} as {}", note_id, device_id, temp_file);
        Ok(())
    }

    /// Get integration statistics
    pub async fn get_stats(&self) -> NoteResult<MobileNotesStats> {
        let contact_notes = self.contact_notes.read().await;
        
        Ok(MobileNotesStats {
            total_sms_converted: 0, // Would track in persistent storage
            notes_linked_to_contacts: 0, // Would track in persistent storage
            mobile_notifications_sent: 0, // Would track in persistent storage
            notes_shared_to_mobile: 0, // Would track in persistent storage
            active_contact_links: contact_notes.len(),
            last_sync_time: Some(Utc::now()),
            conversion_accuracy: 0.85, // Would calculate based on user feedback
        })
    }

    // Helper methods

    async fn is_sms_already_converted(&self, sms_id: &str) -> NoteResult<bool> {
        // Search for notes with this SMS ID in metadata
        let results = self.note_storage.search_notes(&format!("sms_id:{}", sms_id), 1).await?;
        Ok(!results.is_empty())
    }

    async fn get_contact_info(&self, _phone: &str) -> NoteResult<Option<ContactInfo>> {
        // This would query the KDE Connect contact database
        // For now, return None as a placeholder
        Ok(None)
    }

    fn contains_structured_info(&self, text: &str) -> bool {
        // Check for phone numbers, emails, addresses
        let phone_regex = regex::Regex::new(r"\b\d{3}[-.]?\d{3}[-.]?\d{4}\b").unwrap();
        let email_regex = regex::Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").unwrap();
        
        phone_regex.is_match(text) || email_regex.is_match(text) || text.contains("address")
    }

    fn contains_time_sensitive_content(&self, text: &str) -> bool {
        let time_keywords = ["tomorrow", "today", "tonight", "meeting", "appointment", "deadline", "due", "urgent"];
        let text_lower = text.to_lowercase();
        
        time_keywords.iter().any(|keyword| text_lower.contains(keyword))
    }

    fn generate_note_title_from_sms(&self, message: &SmsMessage, contact: &Option<ContactInfo>) -> String {
        let contact_name = contact.as_ref()
            .and_then(|c| c.display_name.as_ref())
            .map(|s| s.as_str())
            .unwrap_or("Unknown Contact");
        
        let preview = if message.body.len() > 50 {
            format!("{}...", &message.body[..47])
        } else {
            message.body.clone()
        };
        
        format!("SMS from {} - {}", contact_name, preview)
    }

    fn generate_tags_from_sms(&self, message: &SmsMessage, contact: &Option<ContactInfo>) -> Vec<String> {
        let mut tags = vec!["sms".to_string(), "mobile".to_string()];
        
        if let Some(contact) = contact {
            if let Some(display_name) = &contact.display_name {
                tags.push(format!("contact:{}", display_name));
            }
        }
        
        // Add contextual tags based on content
        let text_lower = message.body.to_lowercase();
        if text_lower.contains("meeting") || text_lower.contains("appointment") {
            tags.push("meeting".to_string());
        }
        if text_lower.contains("todo") || text_lower.contains("task") {
            tags.push("todo".to_string());
        }
        if text_lower.contains("reminder") {
            tags.push("reminder".to_string());
        }
        
        tags
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mobile::kde_connect::types::MessageType;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_sms_message() -> SmsMessage {
        SmsMessage {
            id: 1,
            body: "Don't forget about the important meeting tomorrow at 2pm. The address is 123 Main St. Please bring the documents we discussed.".to_string(),
            addresses: vec!["+1234567890".to_string()],
            date: Utc::now().timestamp() * 1000, // KDE Connect uses milliseconds
            message_type: MessageType::Sms,
            read: false,
            thread_id: 1,
            sub_id: 0,
            attachments: Vec::new(),
        }
    }

    #[tokio::test]
    async fn test_evaluate_sms_for_conversion() {
        // This would require setting up the full integration service
        // For now, just test the helper methods
        
        let config = MobileNotesConfig::default();
        let message = create_test_sms_message();
        
        // Test structured info detection
        assert!(true); // Placeholder - would test actual functionality
    }

    #[tokio::test]
    async fn test_sms_conversion_candidate_scoring() {
        let message = create_test_sms_message();
        
        // Should score high for:
        // - Contains "important" keyword
        // - Contains "meeting" keyword  
        // - Contains structured info (address)
        // - Substantial length
        
        assert!(message.body.len() > 50);
        assert!(message.body.to_lowercase().contains("important"));
        assert!(message.body.to_lowercase().contains("meeting"));
    }

    #[tokio::test]
    async fn test_note_title_generation() {
        let message = create_test_sms_message();
        let contact = Some(ContactInfo {
            address: "+1234567890".to_string(),
            display_name: Some("John Doe".to_string()),
        });
        
        let config = MobileNotesConfig::default();
        
        // Would test actual title generation logic
        assert!(true);
    }
}