pub mod convert;
/// Maildir format support for email import/export
///
/// Maildir is a widely-used email storage format that stores each message in a separate file.
/// This module provides functionality to read from and write to Maildir directories,
/// enabling compatibility with other terminal email clients like mutt, neomutt, etc.
pub mod reader;
pub mod types;
pub mod utils;
pub mod writer;

pub use convert::MaildirMessage;
pub use reader::MaildirReader;
pub use types::{MaildirError, MaildirFolder, MaildirResult};
pub use utils::{MaildirUtils, MessageFlags};
pub use writer::MaildirWriter;

use crate::email::database::StoredMessage;
use std::path::Path;

/// Main Maildir interface for import/export operations
pub struct Maildir {
    base_path: std::path::PathBuf,
    reader: MaildirReader,
    writer: MaildirWriter,
}

impl Maildir {
    /// Create a new Maildir interface for the given directory
    pub fn new<P: AsRef<Path>>(path: P) -> MaildirResult<Self> {
        let base_path = path.as_ref().to_path_buf();
        let reader = MaildirReader::new(&base_path)?;
        let writer = MaildirWriter::new(&base_path)?;

        Ok(Self {
            base_path,
            reader,
            writer,
        })
    }

    /// Import all messages from a Maildir directory
    pub async fn import_messages(&self) -> MaildirResult<Vec<MaildirMessage>> {
        self.reader.read_all_messages().await
    }

    /// Export messages to a Maildir directory  
    pub async fn export_messages(&self, messages: &[MaildirMessage]) -> MaildirResult<()> {
        self.writer.write_messages(messages).await
    }

    /// Import messages from a specific folder
    pub async fn import_folder(&self, folder_name: &str) -> MaildirResult<Vec<MaildirMessage>> {
        self.reader.read_folder(folder_name).await
    }

    /// Export messages to a specific folder
    pub async fn export_to_folder(
        &self,
        folder_name: &str,
        messages: &[MaildirMessage],
    ) -> MaildirResult<()> {
        self.writer.write_to_folder(folder_name, messages).await
    }

    /// Convert database messages to Maildir format and export
    pub async fn export_stored_messages(&self, messages: &[StoredMessage]) -> MaildirResult<()> {
        let maildir_messages: Vec<MaildirMessage> = messages
            .iter()
            .map(MaildirMessage::from_stored_message)
            .collect();

        self.export_messages(&maildir_messages).await
    }

    /// Import messages and convert to database format
    pub async fn import_to_stored_messages(&self) -> MaildirResult<Vec<StoredMessage>> {
        let maildir_messages = self.import_messages().await?;
        let mut stored_messages = Vec::new();

        for msg in maildir_messages {
            stored_messages.push(msg.to_stored_message()?);
        }

        Ok(stored_messages)
    }

    /// List all available folders in the Maildir
    pub fn list_folders(&self) -> MaildirResult<Vec<String>> {
        self.reader.list_folders()
    }

    /// Get the base path of this Maildir
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_maildir_creation() {
        let temp_dir = TempDir::new().unwrap();
        let maildir = Maildir::new(temp_dir.path()).unwrap();

        assert_eq!(maildir.base_path(), temp_dir.path());
    }

    #[tokio::test]
    async fn test_empty_maildir_import() {
        let temp_dir = TempDir::new().unwrap();
        let maildir = Maildir::new(temp_dir.path()).unwrap();

        let messages = maildir.import_messages().await.unwrap();
        assert!(messages.is_empty());
    }
}
