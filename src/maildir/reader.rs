use std::fs;
/// Maildir reader for importing messages
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

use crate::maildir::convert::{parse_raw_email, MaildirMessage};
use crate::maildir::types::{MaildirError, MaildirFolder, MaildirResult, MaildirSubdir};
use crate::maildir::utils::MaildirUtils;

/// Maildir reader for importing email messages
pub struct MaildirReader {
    base_path: PathBuf,
}

impl MaildirReader {
    /// Create a new Maildir reader
    pub fn new<P: AsRef<Path>>(path: P) -> MaildirResult<Self> {
        let base_path = path.as_ref().to_path_buf();

        if !base_path.exists() {
            return Err(MaildirError::InvalidStructure(format!(
                "Maildir path does not exist: {}",
                base_path.display()
            )));
        }

        Ok(Self { base_path })
    }

    /// Read all messages from the Maildir
    pub async fn read_all_messages(&self) -> MaildirResult<Vec<MaildirMessage>> {
        let mut all_messages = Vec::new();

        // Read messages from all subdirectories
        for subdir in [MaildirSubdir::New, MaildirSubdir::Cur] {
            let messages = self.read_messages_from_subdir(subdir).await?;
            all_messages.extend(messages);
        }

        debug!("Read {} total messages from Maildir", all_messages.len());
        Ok(all_messages)
    }

    /// Read messages from a specific folder
    pub async fn read_folder(&self, folder_name: &str) -> MaildirResult<Vec<MaildirMessage>> {
        let folder_path = if folder_name == "INBOX" {
            self.base_path.clone()
        } else {
            self.base_path.join(folder_name)
        };

        if !MaildirUtils::is_maildir(&folder_path) {
            return Err(MaildirError::FolderNotFound(folder_name.to_string()));
        }

        let reader = MaildirReader::new(folder_path)?;
        reader.read_all_messages().await
    }

    /// Read messages from a specific subdirectory (new/cur)
    async fn read_messages_from_subdir(
        &self,
        subdir: MaildirSubdir,
    ) -> MaildirResult<Vec<MaildirMessage>> {
        let message_files = MaildirUtils::list_messages_in_subdir(&self.base_path, subdir)?;
        let mut messages = Vec::new();

        debug!(
            "Reading {} messages from {} subdirectory",
            message_files.len(),
            subdir.as_str()
        );

        for file_path in message_files {
            match self.read_message_file(&file_path, subdir).await {
                Ok(message) => {
                    messages.push(message);
                }
                Err(e) => {
                    warn!("Failed to read message file {}: {}", file_path.display(), e);
                    // Continue reading other messages
                }
            }
        }

        Ok(messages)
    }

    /// Read a single message file
    async fn read_message_file(
        &self,
        file_path: &Path,
        subdir: MaildirSubdir,
    ) -> MaildirResult<MaildirMessage> {
        // Read the raw message content
        let raw_content = fs::read_to_string(file_path).map_err(|e| MaildirError::Io(e))?;

        // Parse the filename to get flags and unique ID
        let filename = file_path
            .file_name()
            .ok_or_else(|| MaildirError::InvalidFilename(file_path.display().to_string()))?
            .to_string_lossy();

        let parsed_filename = MaildirUtils::parse_filename(&filename)?;

        // Use the conversion layer to parse the email
        let mut maildir_message = parse_raw_email(&raw_content, "maildir", "INBOX")?;

        // Set flags based on Maildir subdirectory and filename flags
        let (is_read, is_flagged) = MaildirMessage::from_maildir_flags(&parsed_filename.flags);
        maildir_message.is_read = is_read || (subdir == MaildirSubdir::Cur);
        maildir_message.is_flagged = is_flagged;

        // Use the unique ID from the filename
        maildir_message.id = parsed_filename.unique_id;

        Ok(maildir_message)
    }

    /// List available folders in the Maildir
    pub fn list_folders(&self) -> MaildirResult<Vec<String>> {
        MaildirUtils::discover_folders(&self.base_path)
    }

    /// Get folder information
    pub fn get_folder_info(&self, folder_name: &str) -> MaildirResult<MaildirFolder> {
        let folder_path = if folder_name == "INBOX" {
            self.base_path.clone()
        } else {
            self.base_path.join(folder_name)
        };

        if !MaildirUtils::is_maildir(&folder_path) {
            return Err(MaildirError::FolderNotFound(folder_name.to_string()));
        }

        // Count messages in each subdirectory
        let new_messages = MaildirUtils::list_messages_in_subdir(&folder_path, MaildirSubdir::New)?;
        let cur_messages = MaildirUtils::list_messages_in_subdir(&folder_path, MaildirSubdir::Cur)?;

        let new_count = new_messages.len();
        let cur_count = cur_messages.len();
        let total_count = new_count + cur_count;

        let mut folder =
            MaildirFolder::new(folder_name.to_string(), folder_path, folder_name == "INBOX");

        folder.new_count = new_count;
        folder.cur_count = cur_count;
        folder.total_count = total_count;

        Ok(folder)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_read_empty_maildir() {
        let temp_dir = TempDir::new().unwrap();
        let maildir_path = temp_dir.path().join("test_maildir");

        MaildirUtils::create_maildir(&maildir_path).unwrap();

        let reader = MaildirReader::new(&maildir_path).unwrap();
        let messages = reader.read_all_messages().await.unwrap();

        assert!(messages.is_empty());
    }

    #[tokio::test]
    async fn test_read_folder_info() {
        let temp_dir = TempDir::new().unwrap();
        let maildir_path = temp_dir.path().join("test_maildir");
        MaildirUtils::create_maildir(&maildir_path).unwrap();

        let reader = MaildirReader::new(&maildir_path).unwrap();
        let folder_info = reader.get_folder_info("INBOX").unwrap();

        assert_eq!(folder_info.name, "INBOX");
        assert_eq!(folder_info.total_count, 0);
        assert_eq!(folder_info.new_count, 0);
        assert_eq!(folder_info.cur_count, 0);
    }

    #[tokio::test]
    async fn test_list_folders() {
        let temp_dir = TempDir::new().unwrap();
        let maildir_path = temp_dir.path().join("test_maildir");
        MaildirUtils::create_maildir(&maildir_path).unwrap();

        let reader = MaildirReader::new(&maildir_path).unwrap();
        let folders = reader.list_folders().unwrap();

        // Should at least have INBOX
        assert!(!folders.is_empty());
        assert!(folders.contains(&"INBOX".to_string()));
    }
}
