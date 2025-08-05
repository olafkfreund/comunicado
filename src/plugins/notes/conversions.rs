//! Note conversion utilities
//! 
//! Provides functionality to convert emails, calendar events, and other content
//! into notes for integrated workflow management.

use super::types::{Note, NoteFrontmatter};
use super::storage::NoteStorage;
use super::manager::NoteResult;
use crate::email::EmailMessage;
use crate::calendar::Event;

use std::sync::Arc;
use chrono::Utc;
use std::path::PathBuf;
use uuid::Uuid;

/// Service for converting various content types to notes
#[derive(Debug, Clone)]
pub struct NoteConversionService {
    storage: Arc<NoteStorage>,
}

impl NoteConversionService {
    /// Create a new conversion service
    pub fn new(storage: Arc<NoteStorage>) -> Self {
        Self { storage }
    }

    /// Convert an email message to a note
    pub async fn convert_email_to_note(&self, email: &EmailMessage, directory_id: i64) -> NoteResult<Note> {
        let title = format!("📧 {}", email.subject());
        
        // Extract content from email
        let mut content = String::new();
        content.push_str(&format!("# {}\n\n", title));
        content.push_str(&format!("**From:** {}\n", email.sender()));
        
        // Format recipients
        let recipients = email.recipients();
        if !recipients.is_empty() {
            let recipient_list: Vec<&str> = recipients.iter().map(|s| s.as_str()).collect();
            content.push_str(&format!("**To:** {}\n", recipient_list.join(", ")));
        }
        
        content.push_str(&format!("**Date:** {}\n", 
            email.timestamp().format("%Y-%m-%d %H:%M:%S")
        ));
        
        content.push_str("\n---\n\n");
        
        // Add email content
        content.push_str(email.content());
        
        // Add attachments info if any
        if email.has_attachments() {
            content.push_str("\n\n## Attachments\n\n");
            content.push_str("*Attachments detected (details not available in current implementation)*\n");
        }
        
        // Create frontmatter with metadata
        let mut frontmatter = NoteFrontmatter::new();
        frontmatter.title = Some(title.clone());
        frontmatter.add_tags(vec!["email".to_string(), "inbox".to_string()]);
        frontmatter.date = Some(*email.timestamp());
        frontmatter.set_metadata("original_message_id".to_string(), 
            serde_yaml::Value::String(email.message_id().to_string()));
        frontmatter.set_metadata("email_from".to_string(), 
            serde_yaml::Value::String(email.sender().to_string()));
        
        // Create note
        let note_id = format!("email-{}", Uuid::new_v4());
        let mut note = Note::new(
            note_id,
            title,
            content,
            PathBuf::from(format!("email-{}.md", Uuid::new_v4())),
        );
        note.frontmatter = Some(frontmatter);
        note.tags = vec!["email".to_string(), "inbox".to_string()];
        
        // Store the note
        self.storage.store_note(&note, directory_id).await?;
        
        Ok(note)
    }

    /// Convert a calendar event to a note
    pub async fn convert_event_to_note(&self, event: &Event, directory_id: i64) -> NoteResult<Note> {
        let title = format!("📅 {}", event.title);
        
        let mut content = String::new();
        content.push_str(&format!("# {}\n\n", title));
        
        // Add event details
        content.push_str(&format!("**Start:** {}\n", event.start_time.format("%Y-%m-%d %H:%M:%S")));
        content.push_str(&format!("**End:** {}\n", event.end_time.format("%Y-%m-%d %H:%M:%S")));
        
        if let Some(location) = &event.location {
            if !location.is_empty() {
                content.push_str(&format!("**Location:** {}\n", location));
            }
        }
        
        if let Some(organizer) = &event.organizer {
            let organizer_name = organizer.name.as_ref().unwrap_or(&organizer.email);
            content.push_str(&format!("**Organizer:** {} <{}>\n", organizer_name, organizer.email));
        }
        
        content.push_str("\n---\n\n");
        
        // Add description
        if let Some(description) = &event.description {
            if !description.is_empty() {
                content.push_str("## Description\n\n");
                content.push_str(description);
                content.push_str("\n\n");
            }
        }
        
        // Add attendees if any
        if !event.attendees.is_empty() {
            content.push_str("## Attendees\n\n");
            for attendee in &event.attendees {
                let name = attendee.name.as_ref().unwrap_or(&attendee.email);
                content.push_str(&format!("- {} <{}> ({})\n", name, attendee.email, 
                    match attendee.status {
                        crate::calendar::event::AttendeeStatus::Accepted => "Accepted",
                        crate::calendar::event::AttendeeStatus::Declined => "Declined", 
                        crate::calendar::event::AttendeeStatus::Tentative => "Tentative",
                        crate::calendar::event::AttendeeStatus::NeedsAction => "Pending",
                        crate::calendar::event::AttendeeStatus::Delegated => "Delegated",
                    }));
            }
            content.push_str("\n");
        }
        
        // Add meeting notes section
        content.push_str("## Meeting Notes\n\n*Add your notes here...*\n\n");
        content.push_str("## Action Items\n\n- [ ] \n\n");
        content.push_str("## Follow-up\n\n");
        
        // Create frontmatter with metadata
        let mut frontmatter = NoteFrontmatter::new();
        frontmatter.title = Some(title.clone());
        frontmatter.add_tags(vec!["calendar".to_string(), "meeting".to_string()]);
        frontmatter.date = Some(event.start_time);
        frontmatter.set_metadata("event_id".to_string(), 
            serde_yaml::Value::String(event.uid.clone()));
        frontmatter.set_metadata("calendar_id".to_string(), 
            serde_yaml::Value::String(event.calendar_id.clone()));
        
        // Create note
        let note_id = format!("event-{}", Uuid::new_v4());
        let mut note = Note::new(
            note_id,
            title,
            content,
            PathBuf::from(format!("event-{}.md", Uuid::new_v4())),
        );
        note.frontmatter = Some(frontmatter);
        note.tags = vec!["calendar".to_string(), "meeting".to_string()];
        
        // Store the note
        self.storage.store_note(&note, directory_id).await?;
        
        Ok(note)
    }

    /// Convert a KDE Connect message to a note
    pub async fn convert_kde_message_to_note(&self, title: &str, content: &str, directory_id: i64) -> NoteResult<Note> {
        let note_title = format!("📱 {}", title);
        
        let mut note_content = String::new();
        note_content.push_str(&format!("# {}\n\n", note_title));
        note_content.push_str(&format!("**Received:** {}\n", Utc::now().format("%Y-%m-%d %H:%M:%S")));
        note_content.push_str(&format!("**Source:** KDE Connect\n"));
        note_content.push_str("\n---\n\n");
        note_content.push_str(content);
        
        // Create frontmatter with metadata
        let mut frontmatter = NoteFrontmatter::new();
        frontmatter.title = Some(note_title.clone());
        frontmatter.add_tags(vec!["kde-connect".to_string(), "mobile".to_string()]);
        frontmatter.date = Some(Utc::now());
        frontmatter.set_metadata("source".to_string(), 
            serde_yaml::Value::String("kde_connect".to_string()));
        
        // Create note
        let note_id = format!("kde-{}", Uuid::new_v4());
        let mut note = Note::new(
            note_id,
            note_title,
            note_content,
            PathBuf::from(format!("kde-{}.md", Uuid::new_v4())),
        );
        note.frontmatter = Some(frontmatter);
        note.tags = vec!["kde-connect".to_string(), "mobile".to_string()];
        
        // Store the note
        self.storage.store_note(&note, directory_id).await?;
        
        Ok(note)
    }

    /// Create a quick note from external CLI
    pub async fn create_quick_note(&self, title: &str, content: &str, tags: Vec<String>, directory_id: i64) -> NoteResult<Note> {
        let note_title = if title.is_empty() {
            format!("Quick Note - {}", Utc::now().format("%Y-%m-%d %H:%M"))
        } else {
            title.to_string()
        };
        
        let mut note_content = String::new();
        note_content.push_str(&format!("# {}\n\n", note_title));
        
        if !content.is_empty() {
            note_content.push_str(content);
        } else {
            note_content.push_str("*Add your content here...*");
        }
        
        // Create frontmatter with metadata
        let mut frontmatter = NoteFrontmatter::new();
        frontmatter.title = Some(note_title.clone());
        frontmatter.add_tags(tags.clone());
        frontmatter.date = Some(Utc::now());
        frontmatter.set_metadata("source".to_string(), 
            serde_yaml::Value::String("cli".to_string()));
        
        // Create note
        let note_id = format!("quick-{}", Uuid::new_v4());
        let mut note = Note::new(
            note_id,
            note_title,
            note_content,
            PathBuf::from(format!("quick-{}.md", Uuid::new_v4())),
        );
        note.frontmatter = Some(frontmatter);
        note.tags = tags;
        
        // Store the note
        self.storage.store_note(&note, directory_id).await?;
        
        Ok(note)
    }

    /// Create a note from clipboard content
    pub async fn create_note_from_clipboard(&self, directory_id: i64) -> NoteResult<Note> {
        // This would integrate with system clipboard
        // For now, create a placeholder note
        let title = format!("Clipboard Note - {}", Utc::now().format("%Y-%m-%d %H:%M"));
        let content = "# Clipboard Content\n\n*Paste your clipboard content here...*";
        
        self.create_quick_note(&title, content, vec!["clipboard".to_string()], directory_id).await
    }

    /// Get the default directory for storing notes
    pub async fn get_default_directory(&self) -> NoteResult<i64> {
        let dirs = self.storage.get_watched_directories().await?;
        if let Some(dir) = dirs.first() {
            Ok(dir.id)
        } else {
            Err(super::manager::NoteError::Storage(
                "No watched directories configured. Please add a directory first.".to_string()
            ))
        }
    }
}

/// Format file size in human-readable format
fn format_file_size(size: usize) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = size as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

// Tests temporarily disabled - will be updated to match current EmailMessage and Event structures
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(512), "512 B");
        assert_eq!(format_file_size(1024), "1.0 KB");
        assert_eq!(format_file_size(1536), "1.5 KB");
        assert_eq!(format_file_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_file_size(1024 * 1024 * 1024), "1.0 GB");
        
        println!("✓ File size formatting works correctly");
    }
}