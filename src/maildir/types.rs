use crate::email::database::StoredMessage;
use chrono::{DateTime, Utc};
/// Type definitions for Maildir operations
use std::path::PathBuf;
use thiserror::Error;

/// Result type for Maildir operations
pub type MaildirResult<T> = Result<T, MaildirError>;

/// Errors that can occur during Maildir operations
#[derive(Error, Debug)]
pub enum MaildirError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid Maildir structure: {0}")]
    InvalidStructure(String),

    #[error("Message parsing error: {0}")]
    MessageParsing(String),

    #[error("Invalid message filename: {0}")]
    InvalidFilename(String),

    #[error("Folder not found: {0}")]
    FolderNotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Maildir already exists at path: {0}")]
    AlreadyExists(String),
}

/// Represents a message in Maildir format
#[derive(Debug, Clone)]
pub struct MaildirMessage {
    /// The message content
    pub message: StoredMessage,

    /// The original filename in the Maildir
    pub filename: String,

    /// The full path to the message file
    pub file_path: PathBuf,

    /// Message flags (seen, replied, etc.)
    pub flags: Vec<char>,

    /// The subdirectory (new, cur, tmp)
    pub subdirectory: MaildirSubdir,

    /// Message delivery timestamp
    pub delivery_time: DateTime<Utc>,

    /// Unique identifier for the message
    pub unique_id: String,
}

/// Maildir subdirectories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaildirSubdir {
    /// New messages (not yet seen by mail client)
    New,
    /// Current messages (seen by mail client)
    Cur,
    /// Temporary messages (being delivered)
    Tmp,
}

impl MaildirSubdir {
    pub fn as_str(&self) -> &'static str {
        match self {
            MaildirSubdir::New => "new",
            MaildirSubdir::Cur => "cur",
            MaildirSubdir::Tmp => "tmp",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "new" => Some(MaildirSubdir::New),
            "cur" => Some(MaildirSubdir::Cur),
            "tmp" => Some(MaildirSubdir::Tmp),
            _ => None,
        }
    }
}

/// Represents a Maildir folder
#[derive(Debug, Clone)]
pub struct MaildirFolder {
    /// Folder name
    pub name: String,

    /// Full path to the folder
    pub path: PathBuf,

    /// Whether this is the root folder (INBOX)
    pub is_root: bool,

    /// Number of messages in new/
    pub new_count: usize,

    /// Number of messages in cur/
    pub cur_count: usize,

    /// Total number of messages
    pub total_count: usize,
}

impl MaildirFolder {
    pub fn new(name: String, path: PathBuf, is_root: bool) -> Self {
        Self {
            name,
            path,
            is_root,
            new_count: 0,
            cur_count: 0,
            total_count: 0,
        }
    }
}

/// Maildir message flags as defined in the specification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaildirFlag {
    /// Message has been read
    Seen,
    /// Message has been replied to
    Replied,
    /// Message is marked for deletion
    Trashed,
    /// Message is a draft
    Draft,
    /// Message is flagged/important
    Flagged,
    /// Message has been forwarded (non-standard but common)
    Passed,
}

impl MaildirFlag {
    pub fn as_char(&self) -> char {
        match self {
            MaildirFlag::Seen => 'S',
            MaildirFlag::Replied => 'R',
            MaildirFlag::Trashed => 'T',
            MaildirFlag::Draft => 'D',
            MaildirFlag::Flagged => 'F',
            MaildirFlag::Passed => 'P',
        }
    }

    pub fn from_char(c: char) -> Option<Self> {
        match c {
            'S' => Some(MaildirFlag::Seen),
            'R' => Some(MaildirFlag::Replied),
            'T' => Some(MaildirFlag::Trashed),
            'D' => Some(MaildirFlag::Draft),
            'F' => Some(MaildirFlag::Flagged),
            'P' => Some(MaildirFlag::Passed),
            _ => None,
        }
    }

    pub fn all_flags() -> Vec<MaildirFlag> {
        vec![
            MaildirFlag::Seen,
            MaildirFlag::Replied,
            MaildirFlag::Trashed,
            MaildirFlag::Draft,
            MaildirFlag::Flagged,
            MaildirFlag::Passed,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_maildir_subdir() {
        assert_eq!(MaildirSubdir::New.as_str(), "new");
        assert_eq!(MaildirSubdir::Cur.as_str(), "cur");
        assert_eq!(MaildirSubdir::Tmp.as_str(), "tmp");

        assert_eq!(MaildirSubdir::from_str("new"), Some(MaildirSubdir::New));
        assert_eq!(MaildirSubdir::from_str("cur"), Some(MaildirSubdir::Cur));
        assert_eq!(MaildirSubdir::from_str("tmp"), Some(MaildirSubdir::Tmp));
        assert_eq!(MaildirSubdir::from_str("invalid"), None);
    }

    #[test]
    fn test_maildir_flags() {
        assert_eq!(MaildirFlag::Seen.as_char(), 'S');
        assert_eq!(MaildirFlag::Replied.as_char(), 'R');

        assert_eq!(MaildirFlag::from_char('S'), Some(MaildirFlag::Seen));
        assert_eq!(MaildirFlag::from_char('R'), Some(MaildirFlag::Replied));
        assert_eq!(MaildirFlag::from_char('X'), None);
    }
}
