use crate::maildir::types::{MaildirError, MaildirFlag, MaildirResult, MaildirSubdir};
use std::fs;
/// Utility functions for Maildir operations
use std::path::{Path, PathBuf};

/// Utilities for working with Maildir format
pub struct MaildirUtils;

impl MaildirUtils {
    /// Check if a directory is a valid Maildir
    pub fn is_maildir<P: AsRef<Path>>(path: P) -> bool {
        let path = path.as_ref();

        // Check if the required subdirectories exist
        let new_dir = path.join("new");
        let cur_dir = path.join("cur");
        let tmp_dir = path.join("tmp");

        new_dir.is_dir() && cur_dir.is_dir() && tmp_dir.is_dir()
    }

    /// Create a Maildir directory structure
    pub fn create_maildir<P: AsRef<Path>>(path: P) -> MaildirResult<()> {
        let path = path.as_ref();

        if path.exists() && Self::is_maildir(path) {
            return Err(MaildirError::AlreadyExists(path.display().to_string()));
        }

        // Create the base directory
        fs::create_dir_all(path)?;

        // Create the required subdirectories
        fs::create_dir_all(path.join("new"))?;
        fs::create_dir_all(path.join("cur"))?;
        fs::create_dir_all(path.join("tmp"))?;

        Ok(())
    }

    /// Parse a Maildir filename to extract components
    pub fn parse_filename(filename: &str) -> MaildirResult<MaildirFilename> {
        // Maildir filename format: <unique_id>:<flags>
        // or just: <unique_id>

        let parts: Vec<&str> = filename.splitn(2, ':').collect();
        let unique_id = parts[0].to_string();

        let flags = if parts.len() > 1 {
            Self::parse_flags(parts[1])?
        } else {
            Vec::new()
        };

        Ok(MaildirFilename {
            unique_id,
            flags,
            original: filename.to_string(),
        })
    }

    /// Parse Maildir flags from a flag string
    pub fn parse_flags(flag_string: &str) -> MaildirResult<Vec<MaildirFlag>> {
        let mut flags = Vec::new();

        // Flag string format: "2,<flags>" or just "<flags>"
        let flag_chars = if flag_string.starts_with("2,") {
            &flag_string[2..]
        } else {
            flag_string
        };

        for c in flag_chars.chars() {
            if let Some(flag) = MaildirFlag::from_char(c) {
                flags.push(flag);
            }
            // Ignore unknown flags (they might be custom extensions)
        }

        Ok(flags)
    }

    /// Generate a unique ID for a new message
    pub fn generate_unique_id() -> String {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let pid = std::process::id();
        let hostname = hostname::get()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        format!("{}.{}.{}", now, pid, hostname)
    }

    /// Create a Maildir filename from components
    pub fn create_filename(unique_id: &str, flags: &[MaildirFlag]) -> String {
        if flags.is_empty() {
            unique_id.to_string()
        } else {
            let mut flag_chars: Vec<char> = flags.iter().map(|f| f.as_char()).collect();
            flag_chars.sort(); // Flags should be sorted

            let flag_string: String = flag_chars.into_iter().collect();
            format!("{}:2,{}", unique_id, flag_string)
        }
    }

    /// Get the full path for a message file
    pub fn get_message_path<P: AsRef<Path>>(
        maildir_path: P,
        subdir: MaildirSubdir,
        filename: &str,
    ) -> PathBuf {
        maildir_path.as_ref().join(subdir.as_str()).join(filename)
    }

    /// List all message files in a Maildir subdirectory
    pub fn list_messages_in_subdir<P: AsRef<Path>>(
        maildir_path: P,
        subdir: MaildirSubdir,
    ) -> MaildirResult<Vec<PathBuf>> {
        let subdir_path = maildir_path.as_ref().join(subdir.as_str());

        if !subdir_path.exists() {
            return Ok(Vec::new());
        }

        let mut messages = Vec::new();

        for entry in fs::read_dir(subdir_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                messages.push(path);
            }
        }

        messages.sort();
        Ok(messages)
    }

    /// Discover Maildir folders in a directory
    pub fn discover_folders<P: AsRef<Path>>(base_path: P) -> MaildirResult<Vec<String>> {
        let base_path = base_path.as_ref();
        let mut folders = Vec::new();

        // Check if the base directory is itself a Maildir
        if Self::is_maildir(base_path) {
            folders.push("INBOX".to_string());
        }

        // Look for subdirectories that are Maildirs
        if base_path.is_dir() {
            for entry in fs::read_dir(base_path)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    if let Some(folder_name) = path.file_name() {
                        let folder_name = folder_name.to_string_lossy().to_string();

                        // Skip hidden directories and standard Maildir subdirs
                        if !folder_name.starts_with('.')
                            && folder_name != "new"
                            && folder_name != "cur"
                            && folder_name != "tmp"
                            && Self::is_maildir(&path)
                        {
                            folders.push(folder_name);
                        }
                    }
                }
            }
        }

        folders.sort();
        Ok(folders)
    }

    /// Convert StoredMessage flags to Maildir flags
    pub fn stored_message_to_maildir_flags(
        message: &crate::email::database::StoredMessage,
    ) -> Vec<MaildirFlag> {
        let mut flags = Vec::new();

        // Check the flags vector for IMAP flags
        for flag in &message.flags {
            match flag.as_str() {
                "\\Seen" => flags.push(MaildirFlag::Seen),
                "\\Flagged" => flags.push(MaildirFlag::Flagged),
                "\\Answered" => flags.push(MaildirFlag::Replied),
                "\\Draft" => flags.push(MaildirFlag::Draft),
                "\\Deleted" => flags.push(MaildirFlag::Trashed),
                _ => {} // Ignore unknown flags
            }
        }

        flags
    }
}

/// Parsed components of a Maildir filename
#[derive(Debug, Clone)]
pub struct MaildirFilename {
    pub unique_id: String,
    pub flags: Vec<MaildirFlag>,
    pub original: String,
}

/// Message flags wrapper for convenience
#[derive(Debug, Clone)]
pub struct MessageFlags {
    flags: Vec<MaildirFlag>,
}

impl MessageFlags {
    pub fn new() -> Self {
        Self { flags: Vec::new() }
    }

    pub fn from_vec(flags: Vec<MaildirFlag>) -> Self {
        Self { flags }
    }

    pub fn add_flag(&mut self, flag: MaildirFlag) {
        if !self.flags.contains(&flag) {
            self.flags.push(flag);
            self.flags.sort_by_key(|f| f.as_char());
        }
    }

    pub fn remove_flag(&mut self, flag: MaildirFlag) {
        self.flags.retain(|&f| f != flag);
    }

    pub fn has_flag(&self, flag: MaildirFlag) -> bool {
        self.flags.contains(&flag)
    }

    pub fn to_vec(&self) -> Vec<MaildirFlag> {
        self.flags.clone()
    }

    pub fn is_seen(&self) -> bool {
        self.has_flag(MaildirFlag::Seen)
    }

    pub fn is_replied(&self) -> bool {
        self.has_flag(MaildirFlag::Replied)
    }

    pub fn is_flagged(&self) -> bool {
        self.has_flag(MaildirFlag::Flagged)
    }

    pub fn is_draft(&self) -> bool {
        self.has_flag(MaildirFlag::Draft)
    }
}

impl Default for MessageFlags {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_generate_unique_id() {
        let id1 = MaildirUtils::generate_unique_id();
        let id2 = MaildirUtils::generate_unique_id();

        assert_ne!(id1, id2);
        assert!(id1.len() > 0);
        assert!(id2.len() > 0);
    }

    #[test]
    fn test_create_filename() {
        let filename = MaildirUtils::create_filename("123456", &[]);
        assert_eq!(filename, "123456");

        let filename =
            MaildirUtils::create_filename("123456", &[MaildirFlag::Seen, MaildirFlag::Replied]);
        assert_eq!(filename, "123456:2,RS");
    }

    #[test]
    fn test_parse_filename() {
        let parsed = MaildirUtils::parse_filename("123456").unwrap();
        assert_eq!(parsed.unique_id, "123456");
        assert!(parsed.flags.is_empty());

        let parsed = MaildirUtils::parse_filename("123456:2,RS").unwrap();
        assert_eq!(parsed.unique_id, "123456");
        assert_eq!(parsed.flags.len(), 2);
        assert!(parsed.flags.contains(&MaildirFlag::Seen));
        assert!(parsed.flags.contains(&MaildirFlag::Replied));
    }

    #[test]
    fn test_create_maildir() {
        let temp_dir = TempDir::new().unwrap();
        let maildir_path = temp_dir.path().join("test_maildir");

        assert!(!MaildirUtils::is_maildir(&maildir_path));

        MaildirUtils::create_maildir(&maildir_path).unwrap();

        assert!(MaildirUtils::is_maildir(&maildir_path));
        assert!(maildir_path.join("new").is_dir());
        assert!(maildir_path.join("cur").is_dir());
        assert!(maildir_path.join("tmp").is_dir());
    }

    #[test]
    fn test_message_flags() {
        let mut flags = MessageFlags::new();
        assert!(!flags.is_seen());

        flags.add_flag(MaildirFlag::Seen);
        assert!(flags.is_seen());

        flags.remove_flag(MaildirFlag::Seen);
        assert!(!flags.is_seen());
    }
}
