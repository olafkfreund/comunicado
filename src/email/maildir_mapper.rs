use crate::email::StoredMessage;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

/// Errors that can occur during maildir metadata mapping
#[derive(Error, Debug)]
pub enum MaildirMapperError {
    #[error("Invalid flag format: {0}")]
    InvalidFlag(String),
    
    #[error("Invalid filename format: {0}")]
    InvalidFilename(String),
    
    #[error("Unsupported maildir version: {0}")]
    UnsupportedVersion(String),
    
    #[error("Metadata conversion error: {0}")]
    ConversionError(String),
}

pub type MaildirMapperResult<T> = Result<T, MaildirMapperError>;

/// Standard IMAP flags and their Maildir equivalents
#[derive(Debug, Clone)]
pub struct FlagMapping {
    /// IMAP flag name
    pub imap_flag: String,
    /// Maildir flag character
    pub maildir_char: char,
    /// Description of the flag
    pub description: String,
}

/// Bidirectional metadata converter between Comunicado and Maildir formats
pub struct MaildirMapper {
    /// Standard flag mappings
    flag_mappings: Vec<FlagMapping>,
    /// Custom flag mappings (for non-standard flags)
    custom_mappings: HashMap<String, char>,
    /// Hostname for generating unique filenames
    hostname: String,
}

impl Default for MaildirMapper {
    fn default() -> Self {
        Self::new()
    }
}

impl MaildirMapper {
    /// Create a new MaildirMapper with standard flag mappings
    pub fn new() -> Self {
        let hostname = hostname::get()
            .unwrap_or_else(|_| "comunicado".into())
            .to_string_lossy()
            .to_string();

        Self {
            flag_mappings: Self::default_flag_mappings(),
            custom_mappings: HashMap::new(),
            hostname,
        }
    }

    /// Create a MaildirMapper with custom hostname
    pub fn with_hostname<S: Into<String>>(hostname: S) -> Self {
        Self {
            flag_mappings: Self::default_flag_mappings(),
            custom_mappings: HashMap::new(),
            hostname: hostname.into(),
        }
    }

    /// Get default IMAP to Maildir flag mappings
    fn default_flag_mappings() -> Vec<FlagMapping> {
        vec![
            FlagMapping {
                imap_flag: "\\Draft".to_string(),
                maildir_char: 'D',
                description: "Message is a draft".to_string(),
            },
            FlagMapping {
                imap_flag: "\\Flagged".to_string(),
                maildir_char: 'F',
                description: "Message is flagged for urgent/special attention".to_string(),
            },
            FlagMapping {
                imap_flag: "\\Answered".to_string(),
                maildir_char: 'R',
                description: "Message has been replied to".to_string(),
            },
            FlagMapping {
                imap_flag: "\\Seen".to_string(),
                maildir_char: 'S',
                description: "Message has been read".to_string(),
            },
            FlagMapping {
                imap_flag: "\\Deleted".to_string(),
                maildir_char: 'T',
                description: "Message is marked for deletion".to_string(),
            },
        ]
    }

    /// Add a custom flag mapping
    pub fn add_custom_mapping<S: Into<String>>(
        &mut self,
        imap_flag: S,
        maildir_char: char,
    ) -> MaildirMapperResult<()> {
        let imap_flag = imap_flag.into();
        
        // Validate that the character isn't already used
        if self.flag_mappings.iter().any(|m| m.maildir_char == maildir_char)
            || self.custom_mappings.values().any(|&c| c == maildir_char)
        {
            return Err(MaildirMapperError::InvalidFlag(format!(
                "Maildir character '{}' is already in use",
                maildir_char
            )));
        }

        self.custom_mappings.insert(imap_flag, maildir_char);
        Ok(())
    }

    /// Convert IMAP flags to Maildir flag string
    pub fn imap_flags_to_maildir(&self, imap_flags: &[String]) -> String {
        let mut maildir_flags = Vec::new();

        for flag in imap_flags {
            // Check standard mappings
            if let Some(mapping) = self.flag_mappings.iter().find(|m| &m.imap_flag == flag) {
                maildir_flags.push(mapping.maildir_char);
            }
            // Check custom mappings
            else if let Some(&maildir_char) = self.custom_mappings.get(flag) {
                maildir_flags.push(maildir_char);
            }
            // Skip unknown flags (they'll be preserved in database)
        }

        // Sort flags alphabetically as per Maildir specification
        maildir_flags.sort();
        maildir_flags.into_iter().collect()
    }

    /// Convert Maildir flag string to IMAP flags
    pub fn maildir_flags_to_imap(&self, maildir_flags: &str) -> Vec<String> {
        let mut imap_flags = Vec::new();

        for flag_char in maildir_flags.chars() {
            // Check standard mappings
            if let Some(mapping) = self.flag_mappings.iter().find(|m| m.maildir_char == flag_char) {
                imap_flags.push(mapping.imap_flag.clone());
            }
            // Check custom mappings
            else if let Some(imap_flag) = self
                .custom_mappings
                .iter()
                .find(|(_, &c)| c == flag_char)
                .map(|(flag, _)| flag)
            {
                imap_flags.push(imap_flag.clone());
            }
            // Skip unknown flags
        }

        imap_flags
    }

    /// Generate a Maildir filename from message metadata
    pub fn generate_maildir_filename(
        &self,
        message: &StoredMessage,
        in_cur_directory: bool,
    ) -> MaildirMapperResult<String> {
        let timestamp = message.date.timestamp();
        let unique_id = message.id.to_string().replace('-', "");

        // Basic filename: timestamp.unique_id.hostname
        let base_filename = format!("{}.{}.{}", timestamp, unique_id, self.hostname);

        if in_cur_directory {
            // Add maildir flags for messages in cur/ directory
            let flags = self.imap_flags_to_maildir(&message.flags);
            Ok(format!("{}:2,{}", base_filename, flags))
        } else {
            // Messages in new/ directory don't have flags
            Ok(base_filename)
        }
    }

    /// Parse a Maildir filename to extract metadata
    pub fn parse_maildir_filename(&self, filename: &str) -> MaildirMapperResult<MaildirFilenameInfo> {
        // Split on the first colon to separate filename from flags
        let (base_part, flags_part) = if let Some(colon_pos) = filename.find(':') {
            let (base, rest) = filename.split_at(colon_pos);
            (base, Some(rest))
        } else {
            (filename, None)
        };

        // Parse base filename: timestamp.unique_id.hostname
        let parts: Vec<&str> = base_part.split('.').collect();
        if parts.len() < 3 {
            return Err(MaildirMapperError::InvalidFilename(format!(
                "Invalid filename format: {}",
                filename
            )));
        }

        let timestamp = parts[0]
            .parse::<i64>()
            .map_err(|_| MaildirMapperError::InvalidFilename(format!(
                "Invalid timestamp in filename: {}",
                filename
            )))?;

        let unique_id = parts[1];
        let hostname = parts[2..].join(".");

        // Parse flags if present
        let flags = if let Some(flags_part) = flags_part {
            if flags_part.starts_with(":2,") {
                let flag_chars = &flags_part[3..];
                self.maildir_flags_to_imap(flag_chars)
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        let datetime = DateTime::from_timestamp(timestamp, 0)
            .ok_or_else(|| MaildirMapperError::InvalidFilename(format!(
                "Invalid timestamp: {}",
                timestamp
            )))?
            .with_timezone(&Utc);

        Ok(MaildirFilenameInfo {
            timestamp: datetime,
            unique_id: unique_id.to_string(),
            hostname,
            flags,
            is_in_cur: flags_part.is_some(),
        })
    }

    /// Convert a StoredMessage to Maildir format metadata
    pub fn message_to_maildir_metadata(&self, message: &StoredMessage) -> MaildirMessageMetadata {
        MaildirMessageMetadata {
            unique_id: message.id,
            timestamp: message.date,
            flags: message.flags.clone(),
            size: message.size.unwrap_or(0) as usize,
            hostname: self.hostname.clone(),
        }
    }

    /// Update a StoredMessage with parsed Maildir metadata
    pub fn update_message_from_maildir(
        &self,
        message: &mut StoredMessage,
        filename_info: &MaildirFilenameInfo,
    ) {
        // Update flags from filename
        message.flags = filename_info.flags.clone();
        
        // If the message doesn't have a date, use the one from filename
        if message.date == Utc::now() || message.date.timestamp() == 0 {
            message.date = filename_info.timestamp;
        }
    }

    /// Get supported flag mappings for display/configuration
    pub fn get_flag_mappings(&self) -> &[FlagMapping] {
        &self.flag_mappings
    }

    /// Get custom flag mappings
    pub fn get_custom_mappings(&self) -> &HashMap<String, char> {
        &self.custom_mappings
    }
}

/// Information parsed from a Maildir filename
#[derive(Debug, Clone)]
pub struct MaildirFilenameInfo {
    /// Timestamp from filename
    pub timestamp: DateTime<Utc>,
    /// Unique identifier
    pub unique_id: String,
    /// Hostname
    pub hostname: String,
    /// IMAP flags parsed from Maildir flags
    pub flags: Vec<String>,
    /// Whether this file is in cur/ directory (has flags)
    pub is_in_cur: bool,
}

/// Maildir-specific message metadata
#[derive(Debug, Clone)]
pub struct MaildirMessageMetadata {
    /// Message unique ID
    pub unique_id: Uuid,
    /// Message timestamp
    pub timestamp: DateTime<Utc>,
    /// Message flags
    pub flags: Vec<String>,
    /// Message size in bytes
    pub size: usize,
    /// Hostname where message was processed
    pub hostname: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_mapper_creation() {
        let mapper = MaildirMapper::new();
        assert!(!mapper.hostname.is_empty());
        assert_eq!(mapper.flag_mappings.len(), 5);
    }

    #[test]
    fn test_mapper_with_hostname() {
        let mapper = MaildirMapper::with_hostname("test.example.com");
        assert_eq!(mapper.hostname, "test.example.com");
    }

    #[test]
    fn test_imap_to_maildir_flags() {
        let mapper = MaildirMapper::new();
        
        let flags = vec![
            "\\Seen".to_string(),
            "\\Flagged".to_string(),
            "\\Draft".to_string(),
        ];
        
        let result = mapper.imap_flags_to_maildir(&flags);
        assert_eq!(result, "DFS"); // Sorted: Draft, Flagged, Seen
    }

    #[test]
    fn test_maildir_to_imap_flags() {
        let mapper = MaildirMapper::new();
        
        let flags = "DFS";
        let result = mapper.maildir_flags_to_imap(flags);
        
        assert_eq!(result.len(), 3);
        assert!(result.contains(&"\\Draft".to_string()));
        assert!(result.contains(&"\\Flagged".to_string()));
        assert!(result.contains(&"\\Seen".to_string()));
    }

    #[test]
    fn test_custom_flag_mapping() {
        let mut mapper = MaildirMapper::new();
        
        mapper.add_custom_mapping("\\CustomFlag", 'C').unwrap();
        
        let flags = vec!["\\CustomFlag".to_string()];
        let result = mapper.imap_flags_to_maildir(&flags);
        assert_eq!(result, "C");
        
        let imap_flags = mapper.maildir_flags_to_imap("C");
        assert_eq!(imap_flags, vec!["\\CustomFlag".to_string()]);
    }

    #[test]
    fn test_duplicate_custom_flag_error() {
        let mut mapper = MaildirMapper::new();
        
        // Try to add a mapping with already used character 'D'
        let result = mapper.add_custom_mapping("\\CustomFlag", 'D');
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_maildir_filename_new() {
        let mapper = MaildirMapper::with_hostname("test.example.com");
        
        let message = create_test_message();
        let filename = mapper.generate_maildir_filename(&message, false).unwrap();
        
        assert!(filename.contains("test.example.com"));
        assert!(!filename.contains(":2,")); // No flags in new/ directory
    }

    #[test]
    fn test_generate_maildir_filename_cur() {
        let mapper = MaildirMapper::with_hostname("test.example.com");
        
        let mut message = create_test_message();
        message.flags = vec!["\\Seen".to_string(), "\\Flagged".to_string()];
        
        let filename = mapper.generate_maildir_filename(&message, true).unwrap();
        
        assert!(filename.contains("test.example.com"));
        assert!(filename.contains(":2,FS")); // Flags in cur/ directory
    }

    #[test]
    fn test_parse_maildir_filename_basic() {
        let mapper = MaildirMapper::new();
        
        let filename = "1609459200.abc123.test.example.com";
        let info = mapper.parse_maildir_filename(filename).unwrap();
        
        assert_eq!(info.unique_id, "abc123");
        assert_eq!(info.hostname, "test.example.com");
        assert!(info.flags.is_empty());
        assert!(!info.is_in_cur);
    }

    #[test]
    fn test_parse_maildir_filename_with_flags() {
        let mapper = MaildirMapper::new();
        
        let filename = "1609459200.abc123.test.example.com:2,FS";
        let info = mapper.parse_maildir_filename(filename).unwrap();
        
        assert_eq!(info.unique_id, "abc123");
        assert_eq!(info.hostname, "test.example.com");
        assert_eq!(info.flags.len(), 2);
        assert!(info.flags.contains(&"\\Seen".to_string()));
        assert!(info.flags.contains(&"\\Flagged".to_string()));
        assert!(info.is_in_cur);
    }

    #[test]
    fn test_parse_invalid_filename() {
        let mapper = MaildirMapper::new();
        
        let filename = "invalid";
        let result = mapper.parse_maildir_filename(filename);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_message_from_maildir() {
        let mapper = MaildirMapper::new();
        let mut message = create_test_message();
        
        let filename_info = MaildirFilenameInfo {
            timestamp: Utc.with_ymd_and_hms(2021, 1, 1, 12, 0, 0).unwrap(),
            unique_id: "test123".to_string(),
            hostname: "test.com".to_string(),
            flags: vec!["\\Seen".to_string()],
            is_in_cur: true,
        };
        
        mapper.update_message_from_maildir(&mut message, &filename_info);
        
        assert_eq!(message.flags, vec!["\\Seen".to_string()]);
    }

    fn create_test_message() -> StoredMessage {
        StoredMessage {
            id: Uuid::new_v4(),
            account_id: "test_account".to_string(),
            folder_name: "INBOX".to_string(),
            imap_uid: 123,
            message_id: Some("<test@example.com>".to_string()),
            thread_id: None,
            in_reply_to: None,
            references: Vec::new(),
            subject: "Test Subject".to_string(),
            from_addr: "sender@example.com".to_string(),
            from_name: None,
            to_addrs: vec!["recipient@example.com".to_string()],
            cc_addrs: Vec::new(),
            bcc_addrs: Vec::new(),
            reply_to: None,
            date: Utc.with_ymd_and_hms(2021, 1, 1, 12, 0, 0).unwrap(),
            body_text: Some("Test body".to_string()),
            body_html: None,
            attachments: Vec::new(),
            flags: Vec::new(),
            labels: Vec::new(),
            size: Some(100),
            priority: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_synced: Utc::now(),
            sync_version: 1,
            is_draft: false,
            is_deleted: false,
        }
    }
}