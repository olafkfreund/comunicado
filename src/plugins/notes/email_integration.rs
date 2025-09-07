//! Email integration for notes plugin
//!
//! Provides functionality to link notes with emails, contacts, and email threads
//! for practical note-taking in an email client context.

use super::manager::NoteResult;
use super::storage::NoteStorage;
use super::types::{Note, NoteId};
use crate::email::message::EmailMessage;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Email-linked note with metadata about the email connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailNote {
    /// The note content and metadata
    pub note: Note,
    /// Email message ID this note is linked to
    pub message_id: String,
    /// Email subject for quick reference
    pub email_subject: String,
    /// Email sender for context
    pub email_sender: String,
    /// Email timestamp
    pub email_timestamp: DateTime<Utc>,
    /// Type of email link
    pub link_type: EmailLinkType,
}

/// Types of email-note relationships
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmailLinkType {
    /// Note about this specific email
    EmailNote,
    /// Note about the sender/contact
    ContactNote,
    /// Note spanning entire email thread
    ThreadNote,
    /// Meeting notes linked to calendar email
    MeetingNote,
    /// Follow-up or action items from email
    FollowUpNote,
}

/// Contact information extracted from emails
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailContact {
    /// Primary email address
    pub email: String,
    /// Display name if available
    pub name: Option<String>,
    /// First seen timestamp
    pub first_seen: DateTime<Utc>,
    /// Last interaction timestamp
    pub last_interaction: DateTime<Utc>,
    /// Number of emails exchanged
    pub email_count: usize,
    /// Notes associated with this contact
    pub note_ids: Vec<NoteId>,
}

/// Email thread information for note linking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailThread {
    /// Thread identifier (usually root message ID)
    pub thread_id: String,
    /// Subject line of the thread
    pub subject: String,
    /// All message IDs in this thread
    pub message_ids: Vec<String>,
    /// Participants in the thread
    pub participants: Vec<String>,
    /// Thread start timestamp
    pub started_at: DateTime<Utc>,
    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,
    /// Notes associated with this thread
    pub note_ids: Vec<NoteId>,
}

/// Email integration service for notes
pub struct EmailIntegrationService {
    /// Storage layer for notes
    storage: Arc<NoteStorage>,
    /// Contact registry
    contacts: Arc<tokio::sync::RwLock<HashMap<String, EmailContact>>>,
    /// Thread registry
    threads: Arc<tokio::sync::RwLock<HashMap<String, EmailThread>>>,
    /// Email-to-note mappings
    email_notes: Arc<tokio::sync::RwLock<HashMap<String, Vec<NoteId>>>>,
}

impl EmailIntegrationService {
    /// Create a new email integration service
    pub fn new(storage: Arc<NoteStorage>) -> Self {
        Self {
            storage,
            contacts: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            threads: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            email_notes: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Create a note linked to a specific email
    pub async fn create_email_note(
        &self,
        email_message: &EmailMessage,
        note_content: String,
        link_type: EmailLinkType,
    ) -> NoteResult<EmailNote> {
        // Generate note ID and title
        let note_id = format!("email_note_{}", uuid::Uuid::new_v4().simple());
        let note_title = match link_type {
            EmailLinkType::EmailNote => format!("Note: {}", email_message.subject()),
            EmailLinkType::ContactNote => format!("Contact: {}", email_message.sender()),
            EmailLinkType::ThreadNote => format!("Thread: {}", email_message.subject()),
            EmailLinkType::MeetingNote => format!("Meeting: {}", email_message.subject()),
            EmailLinkType::FollowUpNote => format!("Follow-up: {}", email_message.subject()),
        };

        // Create the note
        let note_path = self.generate_note_path(&note_id, link_type)?;
        let mut note = Note::new(note_id.clone(), note_title, note_content, note_path);

        // Add email-related tags
        note.tags.push("email-linked".to_string());
        note.tags.push(format!("sender:{}", email_message.sender()));

        match link_type {
            EmailLinkType::ContactNote => note.tags.push("contact".to_string()),
            EmailLinkType::ThreadNote => note.tags.push("thread".to_string()),
            EmailLinkType::MeetingNote => note.tags.push("meeting".to_string()),
            EmailLinkType::FollowUpNote => note.tags.push("follow-up".to_string()),
            _ => {}
        }

        // Store the note (using directory_id 1 for now)
        self.storage.store_note(&note, 1).await?;

        // Create email note wrapper
        let email_note = EmailNote {
            note,
            message_id: email_message.message_id().as_str().to_string(),
            email_subject: email_message.subject().to_string(),
            email_sender: email_message.sender().to_string(),
            email_timestamp: *email_message.timestamp(),
            link_type,
        };

        // Update mappings
        self.update_email_note_mapping(&email_note).await?;
        self.update_contact_registry(email_message, &note_id)
            .await?;

        Ok(email_note)
    }

    /// Get all notes linked to a specific email
    pub async fn get_email_notes(&self, message_id: &str) -> NoteResult<Vec<EmailNote>> {
        let email_notes_map = self.email_notes.read().await;
        let note_ids = email_notes_map.get(message_id).cloned().unwrap_or_default();
        drop(email_notes_map);

        let mut email_notes = Vec::new();
        for note_id in note_ids {
            if let Some(note) = self.storage.get_note(&note_id).await? {
                // Reconstruct EmailNote from stored note
                if let Some(email_note) = self.reconstruct_email_note(note, message_id).await? {
                    email_notes.push(email_note);
                }
            }
        }

        Ok(email_notes)
    }

    /// Get all notes for a specific contact
    pub async fn get_contact_notes(&self, email_address: &str) -> NoteResult<Vec<Note>> {
        let contacts = self.contacts.read().await;
        let contact = contacts.get(email_address);
        let note_ids = contact.map(|c| c.note_ids.clone()).unwrap_or_default();
        drop(contacts);

        let mut notes = Vec::new();
        for note_id in note_ids {
            if let Some(note) = self.storage.get_note(&note_id).await? {
                notes.push(note);
            }
        }

        Ok(notes)
    }

    /// Get notes for an email thread
    pub async fn get_thread_notes(&self, thread_id: &str) -> NoteResult<Vec<Note>> {
        let threads = self.threads.read().await;
        let thread = threads.get(thread_id);
        let note_ids = thread.map(|t| t.note_ids.clone()).unwrap_or_default();
        drop(threads);

        let mut notes = Vec::new();
        for note_id in note_ids {
            if let Some(note) = self.storage.get_note(&note_id).await? {
                notes.push(note);
            }
        }

        Ok(notes)
    }

    /// Search for notes related to email content or contacts
    pub async fn search_email_related_notes(&self, query: &str) -> NoteResult<Vec<Note>> {
        // Use the storage search functionality
        let search_results = self.storage.search_notes(query, 50).await?;

        // Filter for email-related notes
        let mut email_related = Vec::new();
        for result in search_results {
            if result.note.tags.contains(&"email-linked".to_string()) {
                email_related.push(result.note);
            }
        }

        Ok(email_related)
    }

    /// Create a quick note from email context (simplified interface)
    pub async fn quick_note_from_email(
        &self,
        email_message: &EmailMessage,
        note_content: String,
    ) -> NoteResult<EmailNote> {
        self.create_email_note(email_message, note_content, EmailLinkType::EmailNote)
            .await
    }

    /// Create a contact note
    pub async fn create_contact_note(
        &self,
        email_message: &EmailMessage,
        note_content: String,
    ) -> NoteResult<EmailNote> {
        self.create_email_note(email_message, note_content, EmailLinkType::ContactNote)
            .await
    }

    /// Get contact information
    pub async fn get_contact(&self, email_address: &str) -> Option<EmailContact> {
        let contacts = self.contacts.read().await;
        contacts.get(email_address).cloned()
    }

    /// Get all contacts with notes
    pub async fn get_contacts_with_notes(&self) -> Vec<EmailContact> {
        let contacts = self.contacts.read().await;
        contacts
            .values()
            .filter(|contact| !contact.note_ids.is_empty())
            .cloned()
            .collect()
    }

    /// Register an email thread for note linking
    pub async fn register_email_thread(
        &self,
        thread_id: String,
        subject: String,
        message_ids: Vec<String>,
        participants: Vec<String>,
    ) -> NoteResult<()> {
        let now = Utc::now();
        let thread = EmailThread {
            thread_id: thread_id.clone(),
            subject,
            message_ids,
            participants,
            started_at: now,
            last_activity: now,
            note_ids: Vec::new(),
        };

        let mut threads = self.threads.write().await;
        threads.insert(thread_id, thread);
        Ok(())
    }

    /// Delete a note and clean up email mappings
    pub async fn delete_email_note(&self, note_id: &str) -> NoteResult<()> {
        // Delete from storage
        self.storage.delete_note(&note_id.to_string()).await?;

        // Clean up mappings
        self.cleanup_note_mappings(note_id).await?;

        Ok(())
    }

    /// Get statistics about email-linked notes
    pub async fn get_email_notes_stats(&self) -> EmailNotesStats {
        let email_notes = self.email_notes.read().await;
        let contacts = self.contacts.read().await;
        let threads = self.threads.read().await;

        EmailNotesStats {
            total_email_notes: email_notes.values().map(|v| v.len()).sum(),
            linked_emails: email_notes.len(),
            contacts_with_notes: contacts.values().filter(|c| !c.note_ids.is_empty()).count(),
            threads_with_notes: threads.values().filter(|t| !t.note_ids.is_empty()).count(),
        }
    }

    // ==================== Private Helper Methods ====================

    /// Generate appropriate file path for email-linked note
    fn generate_note_path(
        &self,
        note_id: &str,
        link_type: EmailLinkType,
    ) -> NoteResult<std::path::PathBuf> {
        let folder = match link_type {
            EmailLinkType::EmailNote => "email-notes",
            EmailLinkType::ContactNote => "contacts",
            EmailLinkType::ThreadNote => "threads",
            EmailLinkType::MeetingNote => "meetings",
            EmailLinkType::FollowUpNote => "follow-ups",
        };

        Ok(std::path::PathBuf::from(format!(
            "notes/{}/{}.md",
            folder, note_id
        )))
    }

    /// Update email-to-note mapping
    async fn update_email_note_mapping(&self, email_note: &EmailNote) -> NoteResult<()> {
        let mut email_notes = self.email_notes.write().await;
        email_notes
            .entry(email_note.message_id.clone())
            .or_insert_with(Vec::new)
            .push(email_note.note.id.clone());
        Ok(())
    }

    /// Update contact registry with new interaction
    async fn update_contact_registry(
        &self,
        email_message: &EmailMessage,
        note_id: &str,
    ) -> NoteResult<()> {
        let mut contacts = self.contacts.write().await;
        let now = Utc::now();

        let contact = contacts
            .entry(email_message.sender().to_string())
            .or_insert_with(|| EmailContact {
                email: email_message.sender().to_string(),
                name: None, // TODO: Extract from display name
                first_seen: now,
                last_interaction: now,
                email_count: 0,
                note_ids: Vec::new(),
            });

        contact.last_interaction = now;
        contact.email_count += 1;
        if !contact.note_ids.contains(&note_id.to_string()) {
            contact.note_ids.push(note_id.to_string());
        }

        Ok(())
    }

    /// Reconstruct EmailNote from stored note
    async fn reconstruct_email_note(
        &self,
        note: Note,
        message_id: &str,
    ) -> NoteResult<Option<EmailNote>> {
        // This is a simplified reconstruction - in a real implementation,
        // we'd store email metadata in the note frontmatter or separate table

        // Extract email info from tags
        let mut email_sender = "unknown@example.com".to_string();
        for tag in &note.tags {
            if tag.starts_with("sender:") {
                email_sender = tag[7..].to_string();
                break;
            }
        }

        // Determine link type from tags
        let link_type = if note.tags.contains(&"contact".to_string()) {
            EmailLinkType::ContactNote
        } else if note.tags.contains(&"thread".to_string()) {
            EmailLinkType::ThreadNote
        } else if note.tags.contains(&"meeting".to_string()) {
            EmailLinkType::MeetingNote
        } else if note.tags.contains(&"follow-up".to_string()) {
            EmailLinkType::FollowUpNote
        } else {
            EmailLinkType::EmailNote
        };

        Ok(Some(EmailNote {
            note,
            message_id: message_id.to_string(),
            email_subject: "Unknown Subject".to_string(), // Would need to store this
            email_sender,
            email_timestamp: Utc::now(), // Would need to store this
            link_type,
        }))
    }

    /// Clean up note mappings when deleting a note
    async fn cleanup_note_mappings(&self, note_id: &str) -> NoteResult<()> {
        // Clean up email_notes mapping
        let mut email_notes = self.email_notes.write().await;
        for note_ids in email_notes.values_mut() {
            note_ids.retain(|id| id != note_id);
        }
        email_notes.retain(|_, note_ids| !note_ids.is_empty());
        drop(email_notes);

        // Clean up contact mappings
        let mut contacts = self.contacts.write().await;
        for contact in contacts.values_mut() {
            contact.note_ids.retain(|id| id != note_id);
        }
        drop(contacts);

        // Clean up thread mappings
        let mut threads = self.threads.write().await;
        for thread in threads.values_mut() {
            thread.note_ids.retain(|id| id != note_id);
        }

        Ok(())
    }
}

/// Statistics about email-linked notes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailNotesStats {
    pub total_email_notes: usize,
    pub linked_emails: usize,
    pub contacts_with_notes: usize,
    pub threads_with_notes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::email::message::{EmailMessage, MessageId};
    use crate::plugins::notes::storage::NoteStorage;
    use chrono::Utc;
    use std::sync::Arc;

    async fn create_test_storage() -> Arc<NoteStorage> {
        let storage = Arc::new(NoteStorage::new_in_memory().await.unwrap());

        // Add a test directory for storing notes
        use crate::plugins::notes::types::WatchedDirectory;
        let directory = WatchedDirectory::new(
            std::path::PathBuf::from("/test"),
            "Test Directory".to_string(),
        );
        storage.add_watched_directory(directory).await.unwrap();

        storage
    }

    fn create_test_email() -> EmailMessage {
        EmailMessage::new(
            MessageId::new("test@example.com".to_string()),
            "Test Subject".to_string(),
            "sender@example.com".to_string(),
            vec!["recipient@example.com".to_string()],
            "Test email content".to_string(),
            Utc::now(),
        )
    }

    #[tokio::test]
    async fn test_create_email_note() {
        let storage = create_test_storage().await;
        let service = EmailIntegrationService::new(storage);
        let email = create_test_email();

        let email_note = service
            .create_email_note(
                &email,
                "This is a note about the email".to_string(),
                EmailLinkType::EmailNote,
            )
            .await
            .unwrap();

        assert_eq!(email_note.link_type, EmailLinkType::EmailNote);
        assert_eq!(email_note.email_sender, "sender@example.com");
        assert_eq!(email_note.email_subject, "Test Subject");
        assert!(email_note.note.tags.contains(&"email-linked".to_string()));
    }

    #[tokio::test]
    async fn test_create_contact_note() {
        let storage = create_test_storage().await;
        let service = EmailIntegrationService::new(storage);
        let email = create_test_email();

        let email_note = service
            .create_contact_note(&email, "This person is a key client".to_string())
            .await
            .unwrap();

        assert_eq!(email_note.link_type, EmailLinkType::ContactNote);
        assert!(email_note.note.tags.contains(&"contact".to_string()));
        assert!(email_note.note.title.contains("Contact:"));
    }

    #[tokio::test]
    async fn test_get_email_notes() {
        let storage = create_test_storage().await;
        let service = EmailIntegrationService::new(storage);
        let email = create_test_email();
        let message_id = email.message_id().as_str();

        // Create multiple notes for the same email
        service
            .create_email_note(&email, "Note 1".to_string(), EmailLinkType::EmailNote)
            .await
            .unwrap();
        service
            .create_email_note(&email, "Note 2".to_string(), EmailLinkType::FollowUpNote)
            .await
            .unwrap();

        let notes = service.get_email_notes(message_id).await.unwrap();
        assert_eq!(notes.len(), 2);
    }

    #[tokio::test]
    async fn test_contact_registry() {
        let storage = create_test_storage().await;
        let service = EmailIntegrationService::new(storage);
        let email = create_test_email();

        service
            .create_contact_note(&email, "Important client".to_string())
            .await
            .unwrap();

        let contact = service.get_contact("sender@example.com").await.unwrap();
        assert_eq!(contact.email, "sender@example.com");
        assert_eq!(contact.email_count, 1);
        assert_eq!(contact.note_ids.len(), 1);
    }

    #[tokio::test]
    async fn test_quick_note_from_email() {
        let storage = create_test_storage().await;
        let service = EmailIntegrationService::new(storage);
        let email = create_test_email();

        let email_note = service
            .quick_note_from_email(&email, "Quick note about this email".to_string())
            .await
            .unwrap();

        assert_eq!(email_note.link_type, EmailLinkType::EmailNote);
    }

    #[tokio::test]
    async fn test_search_email_related_notes() {
        let storage = create_test_storage().await;
        let service = EmailIntegrationService::new(storage);
        let email = create_test_email();

        service
            .create_email_note(
                &email,
                "Important client meeting notes".to_string(),
                EmailLinkType::EmailNote,
            )
            .await
            .unwrap();

        let results = service.search_email_related_notes("client").await.unwrap();
        assert!(!results.is_empty());

        // All results should be email-linked
        for note in results {
            assert!(note.tags.contains(&"email-linked".to_string()));
        }
    }

    #[tokio::test]
    async fn test_email_thread_registration() {
        let storage = create_test_storage().await;
        let service = EmailIntegrationService::new(storage);

        service
            .register_email_thread(
                "thread-123".to_string(),
                "Project Discussion".to_string(),
                vec!["msg1".to_string(), "msg2".to_string()],
                vec![
                    "alice@example.com".to_string(),
                    "bob@example.com".to_string(),
                ],
            )
            .await
            .unwrap();

        let thread_notes = service.get_thread_notes("thread-123").await.unwrap();
        assert_eq!(thread_notes.len(), 0); // No notes yet
    }

    #[tokio::test]
    async fn test_delete_email_note() {
        let storage = create_test_storage().await;
        let service = EmailIntegrationService::new(storage);
        let email = create_test_email();

        let email_note = service
            .create_email_note(&email, "Test note".to_string(), EmailLinkType::EmailNote)
            .await
            .unwrap();
        let note_id = email_note.note.id.clone();

        service.delete_email_note(&note_id).await.unwrap();

        // Note should be gone from storage
        let retrieved = service.storage.get_note(&note_id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_email_notes_stats() {
        let storage = create_test_storage().await;
        let service = EmailIntegrationService::new(storage);
        let email = create_test_email();

        service
            .create_email_note(&email, "Note 1".to_string(), EmailLinkType::EmailNote)
            .await
            .unwrap();
        service
            .create_contact_note(&email, "Contact note".to_string())
            .await
            .unwrap();

        let stats = service.get_email_notes_stats().await;
        assert_eq!(stats.total_email_notes, 2);
        assert_eq!(stats.linked_emails, 1);
        assert_eq!(stats.contacts_with_notes, 1);
    }

    #[tokio::test]
    async fn test_contacts_with_notes() {
        let storage = create_test_storage().await;
        let service = EmailIntegrationService::new(storage);
        let email = create_test_email();

        service
            .create_contact_note(&email, "Important contact".to_string())
            .await
            .unwrap();

        let contacts = service.get_contacts_with_notes().await;
        assert_eq!(contacts.len(), 1);
        assert_eq!(contacts[0].email, "sender@example.com");
    }
}
