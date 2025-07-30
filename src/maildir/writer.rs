use std::fs;
use std::io::Write;
/// Maildir writer for exporting messages
use std::path::{Path, PathBuf};
use tracing::{debug, error};

use crate::maildir::convert::MaildirMessage;
use crate::maildir::types::{MaildirError, MaildirFlag, MaildirResult, MaildirSubdir};
use crate::maildir::utils::MaildirUtils;

/// Maildir writer for exporting email messages
pub struct MaildirWriter {
    base_path: PathBuf,
}

impl MaildirWriter {
    /// Create a new Maildir writer
    pub fn new<P: AsRef<Path>>(path: P) -> MaildirResult<Self> {
        let base_path = path.as_ref().to_path_buf();

        // Create the Maildir structure if it doesn't exist
        if !MaildirUtils::is_maildir(&base_path) {
            MaildirUtils::create_maildir(&base_path)?;
        }

        Ok(Self { base_path })
    }

    /// Write multiple messages to the Maildir
    pub async fn write_messages(&self, messages: &[MaildirMessage]) -> MaildirResult<()> {
        debug!("Writing {} messages to Maildir", messages.len());

        for message in messages {
            if let Err(e) = self.write_message(message).await {
                error!("Failed to write message {}: {}", message.id, e);
                // Continue with other messages
            }
        }

        Ok(())
    }

    /// Write messages to a specific folder
    pub async fn write_to_folder(
        &self,
        folder_name: &str,
        messages: &[MaildirMessage],
    ) -> MaildirResult<()> {
        let folder_path = if folder_name == "INBOX" {
            self.base_path.clone()
        } else {
            self.base_path.join(folder_name)
        };

        // Create the folder if it doesn't exist
        if !MaildirUtils::is_maildir(&folder_path) {
            MaildirUtils::create_maildir(&folder_path)?;
        }

        let writer = MaildirWriter::new(folder_path)?;
        writer.write_messages(messages).await
    }

    /// Write a single message to the Maildir
    pub async fn write_message(&self, message: &MaildirMessage) -> MaildirResult<()> {
        // Use the message ID or generate a unique one
        let unique_id = if message.id.is_empty() {
            MaildirUtils::generate_unique_id()
        } else {
            message.id.clone()
        };

        // Convert to Maildir flags
        let flags = message.to_maildir_flags();

        // Determine which subdirectory to use
        let subdir = if message.is_read {
            MaildirSubdir::Cur
        } else {
            MaildirSubdir::New
        };

        // Create the filename
        let filename = MaildirUtils::create_filename(&unique_id, &flags);

        // Write to tmp first, then move to final location (atomic operation)
        self.write_message_atomic(message, &filename, subdir)
            .await?;

        debug!(
            "Successfully wrote message {} to {}/{}",
            message.id,
            subdir.as_str(),
            filename
        );
        Ok(())
    }

    /// Write a message atomically using the tmp directory
    async fn write_message_atomic(
        &self,
        message: &MaildirMessage,
        filename: &str,
        subdir: MaildirSubdir,
    ) -> MaildirResult<()> {
        // First write to tmp directory
        let tmp_path =
            MaildirUtils::get_message_path(&self.base_path, MaildirSubdir::Tmp, filename);
        let final_path = MaildirUtils::get_message_path(&self.base_path, subdir, filename);

        // Generate the email content using the MaildirMessage RFC 5322 method
        let email_content = message.to_rfc5322();

        // Write to temporary file
        {
            let mut tmp_file = fs::File::create(&tmp_path)?;
            tmp_file.write_all(email_content.as_bytes())?;
            tmp_file.sync_all()?; // Ensure data is written to disk
        }

        // Move from tmp to final location (atomic operation on most filesystems)
        fs::rename(&tmp_path, &final_path)?;

        Ok(())
    }

    /// Create a new folder in the Maildir
    pub fn create_folder(&self, folder_name: &str) -> MaildirResult<()> {
        if folder_name == "INBOX" {
            return Err(MaildirError::InvalidStructure(
                "Cannot create INBOX folder (it's the root)".to_string(),
            ));
        }

        let folder_path = self.base_path.join(folder_name);

        if folder_path.exists() {
            return Err(MaildirError::AlreadyExists(format!(
                "Folder already exists: {}",
                folder_name
            )));
        }

        MaildirUtils::create_maildir(&folder_path)?;
        debug!("Created Maildir folder: {}", folder_name);

        Ok(())
    }

    /// Remove a folder from the Maildir
    pub fn remove_folder(&self, folder_name: &str) -> MaildirResult<()> {
        if folder_name == "INBOX" {
            return Err(MaildirError::InvalidStructure(
                "Cannot remove INBOX folder".to_string(),
            ));
        }

        let folder_path = self.base_path.join(folder_name);

        if !folder_path.exists() {
            return Err(MaildirError::FolderNotFound(folder_name.to_string()));
        }

        fs::remove_dir_all(&folder_path)?;
        debug!("Removed Maildir folder: {}", folder_name);

        Ok(())
    }

    /// Update message flags
    pub fn update_message_flags(
        &self,
        _message_id: &str,
        _flags: &[MaildirFlag],
    ) -> MaildirResult<()> {
        // This is a simplified implementation - in practice, you'd need to:
        // 1. Find the message file by searching through subdirectories
        // 2. Parse the current filename
        // 3. Create a new filename with updated flags
        // 4. Rename the file

        // For now, we'll return an error to indicate this needs implementation
        Err(MaildirError::MessageParsing(
            "Message flag updates not yet implemented".to_string(),
        ))
    }

    /// Export messages with progress callback
    pub async fn export_with_progress<F>(
        &self,
        messages: &[MaildirMessage],
        mut progress_callback: F,
    ) -> MaildirResult<()>
    where
        F: FnMut(usize, usize),
    {
        let total = messages.len();

        for (index, message) in messages.iter().enumerate() {
            if let Err(e) = self.write_message(message).await {
                error!("Failed to export message {}: {}", message.id, e);
                // Continue with other messages
            }

            progress_callback(index + 1, total);
        }

        Ok(())
    }

    /// Get statistics about the Maildir
    pub fn get_statistics(&self) -> MaildirResult<MaildirStatistics> {
        let new_messages =
            MaildirUtils::list_messages_in_subdir(&self.base_path, MaildirSubdir::New)?;
        let cur_messages =
            MaildirUtils::list_messages_in_subdir(&self.base_path, MaildirSubdir::Cur)?;

        Ok(MaildirStatistics {
            new_count: new_messages.len(),
            cur_count: cur_messages.len(),
            total_count: new_messages.len() + cur_messages.len(),
        })
    }
}

/// Statistics about a Maildir
#[derive(Debug, Clone)]
pub struct MaildirStatistics {
    pub new_count: usize,
    pub cur_count: usize,
    pub total_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::content_preview::ContentType;
    use chrono::Utc;
    use tempfile::TempDir;

    fn create_test_message() -> MaildirMessage {
        MaildirMessage {
            id: "test123".to_string(),
            message_id: Some("test123@example.com".to_string()),
            account_id: "test_account".to_string(),
            folder_name: "INBOX".to_string(),
            from_addr: "sender@example.com".to_string(),
            from_name: None,
            to_addrs: vec!["recipient@example.com".to_string()],
            cc_addrs: vec![],
            bcc_addrs: vec![],
            subject: "Test Message".to_string(),
            body_text: Some("This is a test message body.".to_string()),
            body_html: None,
            content_type: ContentType::PlainText,
            date: Utc::now(),
            is_read: false,
            is_flagged: false,
            has_attachments: false,
            in_reply_to: None,
            references: vec![],
        }
    }

    #[tokio::test]
    async fn test_write_single_message() {
        let temp_dir = TempDir::new().unwrap();
        let maildir_path = temp_dir.path().join("test_maildir");

        let writer = MaildirWriter::new(&maildir_path).unwrap();
        let message = create_test_message();

        writer.write_message(&message).await.unwrap();

        // Verify the message was written
        let new_messages =
            MaildirUtils::list_messages_in_subdir(&maildir_path, MaildirSubdir::New).unwrap();
        assert_eq!(new_messages.len(), 1);

        // Verify the content
        let content = fs::read_to_string(&new_messages[0]).unwrap();
        assert!(content.contains("From: sender@example.com"));
        assert!(content.contains("To: recipient@example.com"));
        assert!(content.contains("Subject: Test Message"));
        assert!(content.contains("This is a test message body."));
    }

    #[tokio::test]
    async fn test_write_multiple_messages() {
        let temp_dir = TempDir::new().unwrap();
        let maildir_path = temp_dir.path().join("test_maildir");

        let writer = MaildirWriter::new(&maildir_path).unwrap();

        let mut messages = Vec::new();
        for i in 0..3 {
            let mut message = create_test_message();
            message.id = format!("test{}", i);
            message.subject = format!("Test Message {}", i);
            messages.push(message);
        }

        writer.write_messages(&messages).await.unwrap();

        // Verify all messages were written
        let new_messages =
            MaildirUtils::list_messages_in_subdir(&maildir_path, MaildirSubdir::New).unwrap();
        assert_eq!(new_messages.len(), 3);
    }

    #[test]
    fn test_generate_email_content() {
        let message = create_test_message();
        let content = message.to_rfc5322();

        assert!(content.contains("Message-ID: test123@example.com"));
        assert!(content.contains("From: sender@example.com"));
        assert!(content.contains("To: recipient@example.com"));
        assert!(content.contains("Subject: Test Message"));
        assert!(content.contains("Content-Type: text/plain; charset=utf-8"));
        assert!(content.contains("This is a test message body."));
    }

    #[test]
    fn test_create_and_remove_folder() {
        let temp_dir = TempDir::new().unwrap();
        let maildir_path = temp_dir.path().join("test_maildir");

        let writer = MaildirWriter::new(&maildir_path).unwrap();

        // Create a folder
        writer.create_folder("TestFolder").unwrap();

        let folder_path = maildir_path.join("TestFolder");
        assert!(MaildirUtils::is_maildir(&folder_path));

        // Remove the folder
        writer.remove_folder("TestFolder").unwrap();

        assert!(!folder_path.exists());
    }

    #[test]
    fn test_get_statistics() {
        let temp_dir = TempDir::new().unwrap();
        let maildir_path = temp_dir.path().join("test_maildir");

        let writer = MaildirWriter::new(&maildir_path).unwrap();

        let stats = writer.get_statistics().unwrap();
        assert_eq!(stats.total_count, 0);
        assert_eq!(stats.new_count, 0);
        assert_eq!(stats.cur_count, 0);
    }
}
