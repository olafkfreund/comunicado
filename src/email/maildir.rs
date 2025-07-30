use crate::email::{EmailDatabase, StoredMessage};
use chrono::{DateTime, Utc};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

/// Maildir-related errors
#[derive(Error, Debug)]
pub enum MaildirError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Invalid maildir path: {0}")]
    InvalidPath(String),

    #[error("Parsing error: {0}")]
    Parse(String),

    #[error("Email format error: {0}")]
    EmailFormat(String),
}

pub type MaildirResult<T> = Result<T, MaildirError>;

/// Maildir format handler for import/export operations
pub struct MaildirHandler {
    base_path: PathBuf,
}

impl MaildirHandler {
    /// Create a new maildir handler
    pub fn new<P: AsRef<Path>>(base_path: P) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
        }
    }

    /// Export messages from database to maildir format
    pub async fn export_account(
        &self,
        database: &EmailDatabase,
        account_id: &str,
    ) -> MaildirResult<usize> {
        let account_path = self.base_path.join(account_id);
        self.ensure_maildir_structure(&account_path)?;

        // Get all folders for this account
        let folders = database
            .get_folders(account_id)
            .await
            .map_err(|e| MaildirError::Database(e.to_string()))?;

        let mut total_exported = 0;

        for folder in folders {
            let messages = database
                .get_messages(account_id, &folder.name, None, None)
                .await
                .map_err(|e| MaildirError::Database(e.to_string()))?;

            let folder_path = account_path.join(self.sanitize_folder_name(&folder.name));
            self.ensure_maildir_structure(&folder_path)?;

            let message_count = messages.len();
            for message in &messages {
                self.export_message(&folder_path, message)?;
                total_exported += 1;
            }

            tracing::info!(
                "Exported {} messages from folder {} to maildir",
                message_count,
                folder.name
            );
        }

        Ok(total_exported)
    }

    /// Export a single folder to maildir format
    pub async fn export_folder(
        &self,
        database: &EmailDatabase,
        account_id: &str,
        folder_name: &str,
    ) -> MaildirResult<usize> {
        let folder_path = self
            .base_path
            .join(account_id)
            .join(self.sanitize_folder_name(folder_name));
        self.ensure_maildir_structure(&folder_path)?;

        let messages = database
            .get_messages(account_id, folder_name, None, None)
            .await
            .map_err(|e| MaildirError::Database(e.to_string()))?;

        for message in &messages {
            self.export_message(&folder_path, message)?;
        }

        tracing::info!(
            "Exported {} messages from folder {} to maildir",
            messages.len(),
            folder_name
        );

        Ok(messages.len())
    }

    /// Import messages from maildir format into database
    pub async fn import_account(
        &self,
        database: &EmailDatabase,
        account_id: &str,
    ) -> MaildirResult<usize> {
        let account_path = self.base_path.join(account_id);

        if !account_path.exists() {
            return Err(MaildirError::InvalidPath(format!(
                "Account path does not exist: {:?}",
                account_path
            )));
        }

        let mut total_imported = 0;

        // Find all maildir folders in the account path
        for entry in fs::read_dir(&account_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() && self.is_maildir_folder(&path) {
                let folder_name = self.path_to_folder_name(&path)?;
                let imported = self
                    .import_folder(database, account_id, &folder_name, &path)
                    .await?;
                total_imported += imported;
            }
        }

        Ok(total_imported)
    }

    /// Import a single folder from maildir format
    pub async fn import_folder(
        &self,
        database: &EmailDatabase,
        account_id: &str,
        folder_name: &str,
        folder_path: &Path,
    ) -> MaildirResult<usize> {
        let mut imported_count = 0;

        // Import from new/ directory (new messages)
        let new_dir = folder_path.join("new");
        if new_dir.exists() {
            imported_count += self
                .import_messages_from_dir(database, account_id, folder_name, &new_dir, false)
                .await?;
        }

        // Import from cur/ directory (current messages)
        let cur_dir = folder_path.join("cur");
        if cur_dir.exists() {
            imported_count += self
                .import_messages_from_dir(database, account_id, folder_name, &cur_dir, true)
                .await?;
        }

        tracing::info!(
            "Imported {} messages into folder {} from maildir",
            imported_count,
            folder_name
        );

        Ok(imported_count)
    }

    /// Import messages from a specific directory (new/ or cur/)
    async fn import_messages_from_dir(
        &self,
        database: &EmailDatabase,
        account_id: &str,
        folder_name: &str,
        dir_path: &Path,
        parse_flags: bool,
    ) -> MaildirResult<usize> {
        let mut imported_count = 0;

        for entry in fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                match self
                    .import_message_file(database, account_id, folder_name, &path, parse_flags)
                    .await
                {
                    Ok(_) => imported_count += 1,
                    Err(e) => {
                        tracing::warn!("Failed to import message from {:?}: {}", path, e);
                        // Continue with other messages
                    }
                }
            }
        }

        Ok(imported_count)
    }

    /// Import a single message file
    async fn import_message_file(
        &self,
        database: &EmailDatabase,
        account_id: &str,
        folder_name: &str,
        file_path: &Path,
        parse_flags: bool,
    ) -> MaildirResult<()> {
        let content = fs::read_to_string(file_path)?;
        let message = self.parse_message_content(&content, account_id, folder_name, parse_flags)?;

        // Store the message in the database
        database
            .store_message(&message)
            .await
            .map_err(|e| MaildirError::Database(e.to_string()))?;

        Ok(())
    }

    /// Export a single message to maildir format
    fn export_message(&self, folder_path: &Path, message: &StoredMessage) -> MaildirResult<()> {
        let timestamp = message.date.timestamp();
        let hostname = "comunicado";
        let unique_id = message.id.to_string().replace('-', "");

        // Create maildir filename: timestamp.unique_id.hostname
        let filename = format!("{}.{}.{}", timestamp, unique_id, hostname);

        // Determine target directory based on message flags
        let target_dir = if message.flags.iter().any(|f| f == "\\Seen") {
            folder_path.join("cur")
        } else {
            folder_path.join("new")
        };

        // Add maildir flags to filename if in cur/
        let final_filename = if target_dir.ends_with("cur") {
            format!(
                "{}:2,{}",
                filename,
                self.format_maildir_flags(&message.flags)
            )
        } else {
            filename
        };

        let file_path = target_dir.join(final_filename);

        // Generate email content
        let email_content = self.format_message_as_email(message)?;

        // Write to file
        fs::write(&file_path, email_content)?;

        Ok(())
    }

    /// Parse message content from maildir file
    fn parse_message_content(
        &self,
        content: &str,
        account_id: &str,
        folder_name: &str,
        _parse_flags: bool,
    ) -> MaildirResult<StoredMessage> {
        // This is a simplified parser - in a real implementation, you'd use a proper email parsing library
        let mut message = StoredMessage {
            id: Uuid::new_v4(),
            account_id: account_id.to_string(),
            folder_name: folder_name.to_string(),
            imap_uid: 0, // Will be assigned when synced to IMAP
            message_id: None,
            thread_id: None,
            in_reply_to: None,
            references: Vec::new(),
            subject: String::new(),
            from_addr: String::new(),
            from_name: None,
            to_addrs: Vec::new(),
            cc_addrs: Vec::new(),
            bcc_addrs: Vec::new(),
            reply_to: None,
            date: Utc::now(),
            body_text: None,
            body_html: None,
            attachments: Vec::new(),
            flags: Vec::new(),
            labels: Vec::new(),
            size: Some(content.len() as u32),
            priority: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_synced: Utc::now(),
            sync_version: 1,
            is_draft: false,
            is_deleted: false,
        };

        // Parse headers and body
        let mut in_headers = true;
        let mut body_lines = Vec::new();
        let mut header_found = false;

        for line in content.lines() {
            if in_headers {
                if line.is_empty() {
                    in_headers = false;
                    continue;
                }

                // Check if this looks like a header line (contains colon)
                if line.contains(": ") {
                    header_found = true;
                    
                    // Parse common headers
                    if let Some(value) = line.strip_prefix("Subject: ") {
                        message.subject = value.to_string();
                    } else if let Some(value) = line.strip_prefix("From: ") {
                        // Simple from parsing - real implementation would be more robust
                        message.from_addr = value.to_string();
                    } else if let Some(value) = line.strip_prefix("To: ") {
                        message.to_addrs = vec![value.to_string()];
                    } else if let Some(value) = line.strip_prefix("Date: ") {
                        // Parse date - using current time as fallback
                        if let Ok(parsed_date) = DateTime::parse_from_rfc2822(value) {
                            message.date = parsed_date.with_timezone(&Utc);
                        }
                    } else if let Some(value) = line.strip_prefix("Message-ID: ") {
                        message.message_id = Some(value.to_string());
                    }
                } else if !header_found {
                    // If we haven't found any headers yet and this doesn't look like a header,
                    // treat everything as body
                    in_headers = false;
                    body_lines.push(line);
                }
            } else {
                body_lines.push(line);
            }
        }

        // Set body text
        if !body_lines.is_empty() {
            message.body_text = Some(body_lines.join("\n"));
        }

        Ok(message)
    }

    /// Format message as RFC822 email
    fn format_message_as_email(&self, message: &StoredMessage) -> MaildirResult<String> {
        let mut email = String::new();

        // Headers
        if let Some(ref message_id) = message.message_id {
            email.push_str(&format!("Message-ID: {}\r\n", message_id));
        }
        email.push_str(&format!(
            "Date: {}\r\n",
            message.date.format("%a, %d %b %Y %H:%M:%S %z")
        ));
        email.push_str(&format!("From: {}\r\n", message.from_addr));
        email.push_str(&format!("Subject: {}\r\n", message.subject));

        if !message.to_addrs.is_empty() {
            email.push_str(&format!("To: {}\r\n", message.to_addrs.join(", ")));
        }

        if !message.cc_addrs.is_empty() {
            email.push_str(&format!("Cc: {}\r\n", message.cc_addrs.join(", ")));
        }

        if let Some(ref reply_to) = message.reply_to {
            email.push_str(&format!("Reply-To: {}\r\n", reply_to));
        }

        if let Some(ref in_reply_to) = message.in_reply_to {
            email.push_str(&format!("In-Reply-To: {}\r\n", in_reply_to));
        }

        if !message.references.is_empty() {
            email.push_str(&format!("References: {}\r\n", message.references.join(" ")));
        }

        // Empty line separating headers from body
        email.push_str("\r\n");

        // Body
        if let Some(ref body_text) = message.body_text {
            email.push_str(body_text);
        } else if let Some(ref body_html) = message.body_html {
            // For HTML-only messages, wrap in simple MIME structure
            email.push_str("Content-Type: text/html; charset=UTF-8\r\n");
            email.push_str("\r\n");
            email.push_str(body_html);
        }

        Ok(email)
    }

    /// Format IMAP flags as maildir flags
    fn format_maildir_flags(&self, flags: &[String]) -> String {
        let mut maildir_flags = String::new();

        for flag in flags {
            match flag.as_str() {
                "\\Draft" => maildir_flags.push('D'),
                "\\Flagged" => maildir_flags.push('F'),
                "\\Answered" => maildir_flags.push('R'),
                "\\Seen" => maildir_flags.push('S'),
                "\\Deleted" => maildir_flags.push('T'),
                _ => {} // Ignore other flags
            }
        }

        // Sort flags alphabetically as per maildir spec
        let mut chars: Vec<char> = maildir_flags.chars().collect();
        chars.sort();
        chars.into_iter().collect()
    }

    /// Ensure maildir directory structure exists
    fn ensure_maildir_structure(&self, path: &Path) -> MaildirResult<()> {
        fs::create_dir_all(path.join("new"))?;
        fs::create_dir_all(path.join("cur"))?;
        fs::create_dir_all(path.join("tmp"))?;
        Ok(())
    }

    /// Check if a directory is a valid maildir folder
    fn is_maildir_folder(&self, path: &Path) -> bool {
        path.join("new").exists() && path.join("cur").exists() && path.join("tmp").exists()
    }

    /// Convert filesystem path to folder name
    fn path_to_folder_name(&self, path: &Path) -> MaildirResult<String> {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.replace("__", "/")) // Convert back from sanitized format
            .ok_or_else(|| MaildirError::InvalidPath(format!("Invalid folder path: {:?}", path)))
    }

    /// Sanitize folder name for filesystem
    fn sanitize_folder_name(&self, folder_name: &str) -> String {
        folder_name
            .replace('/', "__") // Replace slashes with double underscores
            .replace('\\', "__")
            .replace(':', "_")
            .replace('*', "_")
            .replace('?', "_")
            .replace('"', "_")
            .replace('<', "_")
            .replace('>', "_")
            .replace('|', "_")
    }

    /// Get export statistics for an account
    pub async fn get_export_stats(
        &self,
        database: &EmailDatabase,
        account_id: &str,
    ) -> MaildirResult<MaildirStats> {
        let folders = database
            .get_folders(account_id)
            .await
            .map_err(|e| MaildirError::Database(e.to_string()))?;

        let mut stats = MaildirStats {
            total_folders: folders.len(),
            total_messages: 0,
            folders: Vec::new(),
        };

        for folder in folders {
            let messages = database
                .get_messages(account_id, &folder.name, None, None)
                .await
                .map_err(|e| MaildirError::Database(e.to_string()))?;

            stats.total_messages += messages.len();
            stats.folders.push(MaildirFolderStats {
                name: folder.name.clone(),
                message_count: messages.len(),
            });
        }

        Ok(stats)
    }
}

/// Statistics for maildir export/import operations
#[derive(Debug, Clone)]
pub struct MaildirStats {
    pub total_folders: usize,
    pub total_messages: usize,
    pub folders: Vec<MaildirFolderStats>,
}

/// Statistics for a single folder
#[derive(Debug, Clone)]
pub struct MaildirFolderStats {
    pub name: String,
    pub message_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_maildir_handler_creation() {
        let temp_dir = TempDir::new().unwrap();
        let handler = MaildirHandler::new(temp_dir.path());
        assert_eq!(handler.base_path, temp_dir.path());
    }

    #[test]
    fn test_ensure_maildir_structure() {
        let temp_dir = TempDir::new().unwrap();
        let handler = MaildirHandler::new(temp_dir.path());
        let maildir_path = temp_dir.path().join("test");

        handler.ensure_maildir_structure(&maildir_path).unwrap();

        assert!(maildir_path.join("new").exists());
        assert!(maildir_path.join("cur").exists());
        assert!(maildir_path.join("tmp").exists());
    }

    #[test]
    fn test_sanitize_folder_name() {
        let handler = MaildirHandler::new("/tmp");

        assert_eq!(handler.sanitize_folder_name("INBOX"), "INBOX");
        assert_eq!(
            handler.sanitize_folder_name("INBOX/Subfolder"),
            "INBOX__Subfolder"
        );
        assert_eq!(
            handler.sanitize_folder_name("Folder:With:Colons"),
            "Folder_With_Colons"
        );
    }

    #[test]
    fn test_format_maildir_flags() {
        let handler = MaildirHandler::new("/tmp");

        let flags = vec!["\\Seen".to_string(), "\\Flagged".to_string()];
        let result = handler.format_maildir_flags(&flags);
        assert_eq!(result, "FS"); // Sorted: Flagged, Seen

        let flags = vec!["\\Draft".to_string(), "\\Answered".to_string()];
        let result = handler.format_maildir_flags(&flags);
        assert_eq!(result, "DR"); // Sorted: Draft, Replied
    }

    #[test]
    fn test_is_maildir_folder_valid() {
        let temp_dir = TempDir::new().unwrap();
        let handler = MaildirHandler::new(temp_dir.path());
        let maildir_path = temp_dir.path().join("valid_maildir");

        // Create proper maildir structure
        handler.ensure_maildir_structure(&maildir_path).unwrap();

        assert!(handler.is_maildir_folder(&maildir_path));
    }

    #[test]
    fn test_is_maildir_folder_invalid() {
        let temp_dir = TempDir::new().unwrap();
        let handler = MaildirHandler::new(temp_dir.path());
        let invalid_path = temp_dir.path().join("invalid_maildir");

        // Create directory without proper maildir structure
        fs::create_dir_all(&invalid_path).unwrap();
        fs::create_dir_all(invalid_path.join("new")).unwrap();
        // Missing cur/ and tmp/ directories

        assert!(!handler.is_maildir_folder(&invalid_path));
    }

    #[test]
    fn test_is_maildir_folder_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let handler = MaildirHandler::new(temp_dir.path());
        let nonexistent_path = temp_dir.path().join("does_not_exist");

        assert!(!handler.is_maildir_folder(&nonexistent_path));
    }

    #[test]
    fn test_path_to_folder_name() {
        let handler = MaildirHandler::new("/tmp");
        
        let path = std::path::Path::new("/some/path/INBOX__Subfolder");
        let result = handler.path_to_folder_name(path).unwrap();
        assert_eq!(result, "INBOX/Subfolder");

        let path = std::path::Path::new("/some/path/INBOX");
        let result = handler.path_to_folder_name(path).unwrap();
        assert_eq!(result, "INBOX");
    }

    #[test]
    fn test_path_to_folder_name_invalid() {
        let handler = MaildirHandler::new("/tmp");
        
        // Path with no filename
        let path = std::path::Path::new("/");
        let result = handler.path_to_folder_name(path);
        assert!(result.is_err());
    }

    #[test]
    fn test_sanitize_folder_name_special_characters() {
        let handler = MaildirHandler::new("/tmp");

        // Test various special characters that need sanitization
        assert_eq!(handler.sanitize_folder_name("Folder/With\\Slash"), "Folder__With__Slash");
        assert_eq!(handler.sanitize_folder_name("Folder:With*Special?Chars"), "Folder_With_Special_Chars");
        assert_eq!(handler.sanitize_folder_name("Folder\"With<More>Chars|"), "Folder_With_More_Chars_");
    }

    #[test]
    fn test_format_maildir_flags_comprehensive() {
        let handler = MaildirHandler::new("/tmp");

        // Test all supported flags
        let flags = vec![
            "\\Draft".to_string(),
            "\\Flagged".to_string(), 
            "\\Answered".to_string(),
            "\\Seen".to_string(),
            "\\Deleted".to_string(),
        ];
        let result = handler.format_maildir_flags(&flags);
        assert_eq!(result, "DFRST"); // Sorted alphabetically

        // Test empty flags
        let flags = vec![];
        let result = handler.format_maildir_flags(&flags);
        assert_eq!(result, "");

        // Test unsupported flags (should be ignored)
        let flags = vec!["\\CustomFlag".to_string(), "\\Seen".to_string()];
        let result = handler.format_maildir_flags(&flags);
        assert_eq!(result, "S");
    }

    #[test]
    fn test_maildir_validation_with_subdirectories() {
        let temp_dir = TempDir::new().unwrap();
        let handler = MaildirHandler::new(temp_dir.path());
        
        // Create nested maildir structure
        let inbox_path = temp_dir.path().join("INBOX");
        let subfolder_path = temp_dir.path().join("INBOX__Subfolder");
        
        handler.ensure_maildir_structure(&inbox_path).unwrap();
        handler.ensure_maildir_structure(&subfolder_path).unwrap();

        assert!(handler.is_maildir_folder(&inbox_path));
        assert!(handler.is_maildir_folder(&subfolder_path));
    }

    #[test]
    fn test_message_parsing_basic_headers() {
        let handler = MaildirHandler::new("/tmp");
        let content = r#"Subject: Test Subject
From: test@example.com
To: recipient@example.com
Date: Wed, 01 Jan 2020 12:00:00 +0000
Message-ID: <test@example.com>

This is the body of the email."#;

        let message = handler.parse_message_content(content, "test_account", "INBOX", false).unwrap();
        
        assert_eq!(message.subject, "Test Subject");
        assert_eq!(message.from_addr, "test@example.com");
        assert_eq!(message.to_addrs, vec!["recipient@example.com"]);
        assert_eq!(message.message_id, Some("<test@example.com>".to_string()));
        assert_eq!(message.body_text, Some("This is the body of the email.".to_string()));
    }

    #[test]
    fn test_message_parsing_no_headers() {
        let handler = MaildirHandler::new("/tmp");
        let content = "Just a body with no headers";

        let message = handler.parse_message_content(content, "test_account", "INBOX", false).unwrap();
        
        assert_eq!(message.subject, "");
        assert_eq!(message.from_addr, "");
        assert!(message.to_addrs.is_empty());
        assert_eq!(message.message_id, None);
        assert_eq!(message.body_text, Some("Just a body with no headers".to_string()));
    }

    #[test]
    fn test_format_message_as_email() {
        let handler = MaildirHandler::new("/tmp");
        let message = StoredMessage {
            id: Uuid::new_v4(),
            account_id: "test_account".to_string(),
            folder_name: "INBOX".to_string(),
            imap_uid: 123,
            message_id: Some("<test@example.com>".to_string()),
            thread_id: None,
            in_reply_to: None,
            references: vec!["<ref1@example.com>".to_string(), "<ref2@example.com>".to_string()],
            subject: "Test Subject".to_string(),
            from_addr: "sender@example.com".to_string(),
            from_name: None,
            to_addrs: vec!["recipient@example.com".to_string()],
            cc_addrs: vec!["cc@example.com".to_string()],
            bcc_addrs: vec![],
            reply_to: Some("reply@example.com".to_string()),
            date: DateTime::parse_from_rfc3339("2020-01-01T12:00:00Z").unwrap().with_timezone(&Utc),
            body_text: Some("This is the email body".to_string()),
            body_html: None,
            attachments: vec![],
            flags: vec![],
            labels: vec![],
            size: Some(100),
            priority: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_synced: Utc::now(),
            sync_version: 1,
            is_draft: false,
            is_deleted: false,
        };

        let email_content = handler.format_message_as_email(&message).unwrap();
        
        assert!(email_content.contains("Message-ID: <test@example.com>"));
        assert!(email_content.contains("Subject: Test Subject"));
        assert!(email_content.contains("From: sender@example.com"));
        assert!(email_content.contains("To: recipient@example.com"));
        assert!(email_content.contains("Cc: cc@example.com"));
        assert!(email_content.contains("Reply-To: reply@example.com"));
        assert!(email_content.contains("References: <ref1@example.com> <ref2@example.com>"));
        assert!(email_content.contains("This is the email body"));
    }
}
