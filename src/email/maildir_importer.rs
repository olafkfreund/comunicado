use crate::email::{
    EmailDatabase, FolderHierarchyMapper, MaildirErrorHandler, MaildirMapper, 
    MaildirOperationContext, StoredMessage, TimestampUtils,
};
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tokio::fs;
use uuid::Uuid;
use walkdir::WalkDir;

/// Errors that can occur during Maildir import operations
#[derive(Error, Debug)]
pub enum MaildirImportError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("Invalid Maildir structure: {0}")]
    InvalidStructure(String),
    
    #[error("Email parsing error: {0}")]
    EmailParsing(String),
    
    #[error("Folder mapping error: {0}")]
    FolderMapping(String),
    
    #[error("Import cancelled by user")]
    Cancelled,
    
    #[error("Permission denied: {0}")]
    Permission(String),
}

pub type MaildirImportResult<T> = Result<T, MaildirImportError>;

/// Statistics for import operations
#[derive(Debug, Clone, Default)]
pub struct ImportStats {
    /// Total directories scanned
    pub directories_scanned: usize,
    /// Total valid Maildir folders found
    pub maildir_folders_found: usize,
    /// Total messages found
    pub messages_found: usize,
    /// Successfully imported messages
    pub messages_imported: usize,
    /// Failed message imports
    pub messages_failed: usize,
    /// Duplicate messages skipped
    pub duplicates_skipped: usize,
    /// Processing errors encountered
    pub errors: Vec<String>,
}

impl ImportStats {
    /// Calculate success rate as percentage
    pub fn success_rate(&self) -> f64 {
        if self.messages_found == 0 {
            0.0
        } else {
            (self.messages_imported as f64 / self.messages_found as f64) * 100.0
        }
    }
    
    /// Check if import was successful overall
    pub fn is_successful(&self) -> bool {
        self.messages_failed == 0 && !self.errors.is_empty() == false
    }
}

/// Configuration for import operations
#[derive(Debug, Clone)]
pub struct ImportConfig {
    /// Maximum number of messages to import (None = unlimited)
    pub max_messages: Option<usize>,
    /// Skip duplicate messages (based on Message-ID)
    pub skip_duplicates: bool,
    /// Validate email format before importing
    pub validate_format: bool,
    /// Update existing messages if found
    pub update_existing: bool,
    /// Preserve original timestamps
    pub preserve_timestamps: bool,
    /// Show progress during import
    pub show_progress: bool,
}

impl Default for ImportConfig {
    fn default() -> Self {
        Self {
            max_messages: None,
            skip_duplicates: true,
            validate_format: true,
            update_existing: false,
            preserve_timestamps: true,
            show_progress: true,
        }
    }
}

/// Import progress callback function type
pub type ProgressCallback = Box<dyn Fn(usize, usize, &str) + Send + Sync>;

/// Async Maildir importer with progress tracking and error handling
pub struct MaildirImporter {
    /// Database connection
    database: Arc<EmailDatabase>,
    /// Maildir metadata mapper
    mapper: MaildirMapper,
    /// Folder hierarchy mapper
    folder_mapper: FolderHierarchyMapper,
    /// Import configuration
    config: ImportConfig,
    /// Progress callback
    progress_callback: Option<ProgressCallback>,
    /// Cancellation flag
    cancelled: Arc<std::sync::atomic::AtomicBool>,
    /// Error handler for robust error handling
    error_handler: MaildirErrorHandler,
    /// Operation context for detailed error reporting
    #[allow(dead_code)]
    operation_context: MaildirOperationContext,
}

impl MaildirImporter {
    /// Create a new Maildir importer
    pub fn new(database: Arc<EmailDatabase>) -> Self {
        Self {
            database,
            mapper: MaildirMapper::new(),
            folder_mapper: FolderHierarchyMapper::new(),
            config: ImportConfig::default(),
            progress_callback: None,
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            error_handler: MaildirErrorHandler::default(),
            operation_context: MaildirOperationContext::new("Maildir Import".to_string()),
        }
    }

    /// Create importer with custom configuration
    pub fn with_config(database: Arc<EmailDatabase>, config: ImportConfig) -> Self {
        Self {
            database,
            mapper: MaildirMapper::new(),
            folder_mapper: FolderHierarchyMapper::new(),
            config,
            progress_callback: None,
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            error_handler: MaildirErrorHandler::default(),
            operation_context: MaildirOperationContext::new("Maildir Import".to_string()),
        }
    }

    /// Set progress callback for tracking import progress
    pub fn set_progress_callback(&mut self, callback: ProgressCallback) {
        self.progress_callback = Some(callback);
    }

    /// Cancel the current import operation
    pub fn cancel(&self) {
        self.cancelled.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    /// Check if import has been cancelled
    fn is_cancelled(&self) -> bool {
        self.cancelled.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Import all messages from a Maildir directory structure with robust error handling
    pub async fn import_from_directory<P: AsRef<Path>>(
        &self,
        maildir_path: P,
        account_id: &str,
    ) -> MaildirImportResult<ImportStats> {
        let path = maildir_path.as_ref();
        
        // Create operation context for this import
        let mut operation_context = MaildirOperationContext::new("Maildir Import".to_string())
            .with_paths(Some(path.to_path_buf()), None);

        // Validate path with enhanced error handling
        if let Err(io_error) = std::fs::metadata(path) {
            let maildir_error = self.error_handler.classify_error(io_error, path, "validating import path").await;
            let detailed_error = operation_context.create_detailed_error(maildir_error);
            return Err(MaildirImportError::InvalidStructure(detailed_error));
        }

        if !path.is_dir() {
            return Err(MaildirImportError::InvalidStructure(format!(
                "Path is not a directory: {:?}",
                path
            )));
        }

        let mut stats = ImportStats::default();
        let progress_bar = if self.config.show_progress {
            Some(self.create_progress_bar())
        } else {
            None
        };

        // First pass: scan directory structure 
        let maildir_folders = self.scan_maildir_structure(path, &mut stats).await?;
        
        if let Some(ref pb) = progress_bar {
            pb.set_length(stats.messages_found as u64);
            pb.set_message("Importing messages...");
        }

        // Second pass: import messages from each folder with progress tracking
        for (folder_index, folder_info) in maildir_folders.iter().enumerate() {
            if self.is_cancelled() {
                return Err(MaildirImportError::Cancelled);
            }

            // Update progress context
            operation_context.update_progress(
                folder_index, 
                maildir_folders.len(), 
                Some(folder_info.imap_name.clone())
            );

            // Import folder with error recovery
            match self.import_folder_with_retry(&folder_info, account_id, &mut stats, &progress_bar).await {
                Ok(_) => {},
                Err(e) => {
                    // Log error but continue with other folders
                    let error_msg = format!("Failed to import folder '{}': {}", folder_info.imap_name, e);
                    stats.errors.push(error_msg);
                    stats.messages_failed += folder_info.message_count;
                }
            }
        }

        if let Some(pb) = progress_bar {
            pb.finish_with_message("Import completed");
        }

        Ok(stats)
    }

    /// Scan directory structure and identify valid Maildir folders
    async fn scan_maildir_structure<P: AsRef<Path>>(
        &self,
        base_path: P,
        stats: &mut ImportStats,
    ) -> MaildirImportResult<Vec<MaildirFolderInfo>> {
        let mut folders = Vec::new();
        let base = base_path.as_ref();

        for entry in WalkDir::new(base).min_depth(1).max_depth(10) {
            if self.is_cancelled() {
                return Err(MaildirImportError::Cancelled);
            }

            let entry = entry.map_err(|e| MaildirImportError::Io(e.into()))?;
            let path = entry.path();
            
            stats.directories_scanned += 1;

            if self.is_valid_maildir_folder(path).await? {
                let folder_info = self.analyze_maildir_folder(base, path).await?;
                stats.messages_found += folder_info.message_count;
                stats.maildir_folders_found += 1;
                folders.push(folder_info);
            }
        }

        Ok(folders)
    }

    /// Check if a directory is a valid Maildir folder
    async fn is_valid_maildir_folder<P: AsRef<Path>>(&self, path: P) -> MaildirImportResult<bool> {
        let path = path.as_ref();
        
        if !path.is_dir() {
            return Ok(false);
        }

        // Check for required Maildir subdirectories
        let new_dir = path.join("new");
        let cur_dir = path.join("cur");
        let tmp_dir = path.join("tmp");

        Ok(new_dir.exists() && cur_dir.exists() && tmp_dir.exists())
    }

    /// Analyze a Maildir folder and extract metadata
    async fn analyze_maildir_folder<P: AsRef<Path>>(
        &self,
        base_path: P,
        folder_path: P,
    ) -> MaildirImportResult<MaildirFolderInfo> {
        let base = base_path.as_ref();
        let folder = folder_path.as_ref();

        // Extract folder hierarchy information
        let hierarchy = self
            .folder_mapper
            .extract_hierarchy_from_path(base, folder)
            .map_err(|e| MaildirImportError::FolderMapping(e.to_string()))?;

        // Count messages in new/ and cur/ directories
        let new_count = self.count_messages_in_dir(&folder.join("new")).await?;
        let cur_count = self.count_messages_in_dir(&folder.join("cur")).await?;

        Ok(MaildirFolderInfo {
            path: folder.to_path_buf(),
            imap_name: hierarchy.imap_path,
            message_count: new_count + cur_count,
            new_messages: new_count,
            cur_messages: cur_count,
        })
    }

    /// Count messages in a directory
    async fn count_messages_in_dir<P: AsRef<Path>>(&self, dir_path: P) -> MaildirImportResult<usize> {
        let path = dir_path.as_ref();
        
        if !path.exists() {
            return Ok(0);
        }

        let mut count = 0;
        let mut entries = fs::read_dir(path).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_file() {
                count += 1;
            }
        }

        Ok(count)
    }

    /// Import messages from a single Maildir folder
    async fn import_folder(
        &self,
        folder_info: &MaildirFolderInfo,
        account_id: &str,
        stats: &mut ImportStats,
        progress_bar: &Option<ProgressBar>,
    ) -> MaildirImportResult<()> {
        // Ensure folder exists in database
        self.ensure_folder_exists(account_id, &folder_info.imap_name).await?;

        // Import messages from new/ directory
        self.import_messages_from_dir(
            &folder_info.path.join("new"),
            account_id,
            &folder_info.imap_name,
            false, // new messages don't have flags
            stats,
            progress_bar,
        ).await?;

        // Import messages from cur/ directory
        self.import_messages_from_dir(
            &folder_info.path.join("cur"),
            account_id,
            &folder_info.imap_name,
            true, // cur messages may have flags
            stats,
            progress_bar,
        ).await?;

        Ok(())
    }

    /// Import messages from a specific directory (new/ or cur/)
    async fn import_messages_from_dir<P: AsRef<Path>>(
        &self,
        dir_path: P,
        account_id: &str,
        folder_name: &str,
        parse_flags: bool,
        stats: &mut ImportStats,
        progress_bar: &Option<ProgressBar>,
    ) -> MaildirImportResult<()> {
        let path = dir_path.as_ref();
        
        if !path.exists() {
            return Ok(());
        }

        let mut entries = fs::read_dir(path).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            if self.is_cancelled() {
                return Err(MaildirImportError::Cancelled);
            }

            if entry.file_type().await?.is_file() {
                match self.import_message_file(
                    &entry.path(),
                    account_id,
                    folder_name,
                    parse_flags,
                ).await {
                    Ok(imported) => {
                        if imported {
                            stats.messages_imported += 1;
                        } else {
                            stats.duplicates_skipped += 1;
                        }
                    }
                    Err(e) => {
                        stats.messages_failed += 1;
                        stats.errors.push(format!("File {:?}: {}", entry.path(), e));
                    }
                }

                if let Some(ref pb) = progress_bar {
                    pb.inc(1);
                }

                // Call progress callback if set
                if let Some(ref callback) = self.progress_callback {
                    callback(
                        stats.messages_imported + stats.duplicates_skipped + stats.messages_failed,
                        stats.messages_found,
                        &format!("Processing {}", folder_name),
                    );
                }

                // Check message limit
                if let Some(max) = self.config.max_messages {
                    if stats.messages_imported >= max {
                        return Ok(());
                    }
                }
            }
        }

        Ok(())
    }

    /// Import a single message file
    async fn import_message_file<P: AsRef<Path>>(
        &self,
        file_path: P,
        account_id: &str,
        folder_name: &str,
        parse_flags: bool,
    ) -> MaildirImportResult<bool> {
        let path = file_path.as_ref();
        let content = fs::read_to_string(path).await?;

        // Parse the email content
        let mut message = self.parse_email_content(&content, account_id, folder_name)?;

        // Extract flags from filename if in cur/ directory
        if parse_flags {
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                if let Ok(filename_info) = self.mapper.parse_maildir_filename(filename) {
                    self.mapper.update_message_from_maildir(&mut message, &filename_info);
                }
            }
        }

        // Check for duplicates if configured
        if self.config.skip_duplicates {
            if let Some(ref message_id) = message.message_id {
                if self.message_exists(account_id, message_id).await? {
                    return Ok(false); // Duplicate skipped
                }
            }
        }

        // Preserve original timestamps if configured
        if self.config.preserve_timestamps {
            if let Ok(file_time) = TimestampUtils::get_file_modification_time(path) {
                message.date = file_time;
            }
        }

        // Store message in database
        self.database
            .store_message(&message)
            .await
            .map_err(|e| MaildirImportError::Database(e.to_string()))?;

        Ok(true) // Successfully imported
    }

    /// Parse email content into StoredMessage
    fn parse_email_content(
        &self,
        content: &str,
        account_id: &str,
        folder_name: &str,
    ) -> MaildirImportResult<StoredMessage> {
        // Use the existing parsing logic from MaildirHandler
        // This is a simplified version - in production, use a proper email parsing library
        let mut message = StoredMessage {
            id: Uuid::new_v4(),
            account_id: account_id.to_string(),
            folder_name: folder_name.to_string(),
            imap_uid: 0, // Will be assigned during IMAP sync
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
            date: TimestampUtils::now_utc(),
            body_text: None,
            body_html: None,
            attachments: Vec::new(),
            flags: Vec::new(),
            labels: Vec::new(),
            size: Some(content.len() as u32),
            priority: None,
            created_at: TimestampUtils::now_utc(),
            updated_at: TimestampUtils::now_utc(),
            last_synced: TimestampUtils::now_utc(),
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

                // Check if this looks like a header line
                if line.contains(": ") {
                    header_found = true;
                    
                    if let Some(value) = line.strip_prefix("Subject: ") {
                        message.subject = value.to_string();
                    } else if let Some(value) = line.strip_prefix("From: ") {
                        message.from_addr = value.to_string();
                    } else if let Some(value) = line.strip_prefix("To: ") {
                        message.to_addrs = vec![value.to_string()];
                    } else if let Some(value) = line.strip_prefix("Cc: ") {
                        message.cc_addrs = vec![value.to_string()];
                    } else if let Some(value) = line.strip_prefix("Date: ") {
                        if let Ok(parsed_date) = TimestampUtils::parse_email_date(value) {
                            message.date = parsed_date;
                        }
                    } else if let Some(value) = line.strip_prefix("Message-ID: ") {
                        message.message_id = Some(value.to_string());
                    } else if let Some(value) = line.strip_prefix("In-Reply-To: ") {
                        message.in_reply_to = Some(value.to_string());
                    } else if let Some(value) = line.strip_prefix("References: ") {
                        message.references = value.split_whitespace().map(|s| s.to_string()).collect();
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

        // Validate email format if configured
        if self.config.validate_format {
            self.validate_message_format(&message)?;
        }

        Ok(message)
    }

    /// Validate message format
    fn validate_message_format(&self, message: &StoredMessage) -> MaildirImportResult<()> {
        if message.from_addr.is_empty() {
            return Err(MaildirImportError::EmailParsing(
                "Missing From address".to_string(),
            ));
        }

        if message.subject.is_empty() && message.body_text.is_none() {
            return Err(MaildirImportError::EmailParsing(
                "Message has no subject or body".to_string(),
            ));
        }

        Ok(())
    }

    /// Check if a message already exists in the database
    async fn message_exists(&self, _account_id: &str, _message_id: &str) -> MaildirImportResult<bool> {
        // This would need to be implemented in EmailDatabase
        // For now, return false (assume no duplicates)
        Ok(false)
    }

    /// Ensure folder exists in database
    async fn ensure_folder_exists(&self, _account_id: &str, _folder_name: &str) -> MaildirImportResult<()> {
        // This would need to be implemented to create folder if it doesn't exist
        // For now, assume folder exists
        Ok(())
    }

    /// Create progress bar for import operations
    fn create_progress_bar(&self) -> ProgressBar {
        let pb = ProgressBar::new(0);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );
        pb.set_message("Scanning directories...");
        pb
    }

    /// Get import configuration
    pub fn config(&self) -> &ImportConfig {
        &self.config
    }

    /// Update import configuration
    pub fn set_config(&mut self, config: ImportConfig) {
        self.config = config;
    }

    /// Import a folder with retry logic and error recovery
    async fn import_folder_with_retry(
        &self,
        folder_info: &MaildirFolderInfo,
        account_id: &str,
        stats: &mut ImportStats,
        progress_bar: &Option<ProgressBar>,
    ) -> MaildirImportResult<()> {
        // For now, call import_folder directly - full retry would need refactoring
        self.import_folder(folder_info, account_id, stats, progress_bar).await
    }

    /// Create user-friendly error report for import failures
    pub fn create_error_report(&self, error: &MaildirImportError) -> String {
        match error {
            MaildirImportError::Io(io_error) => {
                format!("❌ **I/O Error**\n\n**Details:** {}\n\n**Suggestion:** Check file permissions and disk space.", io_error)
            },
            MaildirImportError::Permission(msg) => {
                format!("❌ **Permission Error**\n\n**Details:** {}\n\n**Suggestion:** Check file permissions and ensure you have read access to the Maildir directory.", msg)
            },
            MaildirImportError::InvalidStructure(msg) => {
                format!("❌ **Invalid Maildir Structure**\n\n**Details:** {}\n\n**Suggestion:** Ensure the directory contains valid Maildir folders with 'new', 'cur', and 'tmp' subdirectories.", msg)
            },
            MaildirImportError::Cancelled => {
                "⚠️ **Import Cancelled**\n\nThe import operation was cancelled by user request.".to_string()
            },
            _ => format!("❌ **Import Error**\n\n**Details:** {}\n\n**Suggestion:** Check the error details and try again.", error),
        }
    }

    /// Save import progress for resume capability
    pub async fn save_progress_checkpoint(&self, stats: &ImportStats, current_folder: &str) -> MaildirImportResult<()> {
        // Implementation would save progress to a resume file
        // For now, this is a placeholder for the resume capability
        println!("Progress checkpoint: {}/{} messages imported, current folder: {}", 
                stats.messages_imported, stats.messages_found, current_folder);
        Ok(())
    }

    /// Resume import from a previous checkpoint
    pub async fn resume_from_checkpoint<P: AsRef<Path>>(
        &self,
        maildir_path: P,
        account_id: &str,
        _checkpoint_file: P,
    ) -> MaildirImportResult<ImportStats> {
        // Implementation would load previous progress and continue from there
        // For now, this is a placeholder - would read checkpoint file and skip already processed folders
        println!("Resume capability not yet fully implemented - starting fresh import");
        self.import_from_directory(maildir_path, account_id).await
    }
}

/// Information about a discovered Maildir folder
#[derive(Debug, Clone)]
struct MaildirFolderInfo {
    /// Filesystem path to the folder
    path: PathBuf,
    /// IMAP-style folder name
    imap_name: String,
    /// Total message count
    message_count: usize,
    /// Messages in new/ directory
    #[allow(dead_code)]
    new_messages: usize,
    /// Messages in cur/ directory
    #[allow(dead_code)]
    cur_messages: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::email::EmailDatabase;
    use tempfile::TempDir;
    use tokio::fs;

    /// Create a test EmailDatabase
    async fn create_test_database() -> Arc<EmailDatabase> {
        Arc::new(EmailDatabase::new_in_memory().await.unwrap())
    }

    /// Create a mock Maildir structure for testing
    async fn create_mock_maildir(base_path: &Path) -> Result<()> {
        // Create INBOX folder
        let inbox_path = base_path.join("INBOX");
        create_maildir_folder(&inbox_path).await?;
        
        // Add sample messages to INBOX
        create_test_message(&inbox_path.join("new"), "1234567890.msg1.hostname", TEST_EMAIL_1).await?;
        create_test_message(&inbox_path.join("cur"), "1234567891.msg2.hostname:2,S", TEST_EMAIL_2).await?;
        
        // Create a subfolder
        let work_path = base_path.join("INBOX__Work");
        create_maildir_folder(&work_path).await?;
        create_test_message(&work_path.join("new"), "1234567892.msg3.hostname", TEST_EMAIL_3).await?;

        Ok(())
    }

    /// Create a Maildir folder structure
    async fn create_maildir_folder(path: &Path) -> Result<()> {
        fs::create_dir_all(path.join("new")).await?;
        fs::create_dir_all(path.join("cur")).await?;
        fs::create_dir_all(path.join("tmp")).await?;
        Ok(())
    }

    /// Create a test message file
    async fn create_test_message(dir: &Path, filename: &str, content: &str) -> Result<()> {
        let file_path = dir.join(filename);
        fs::write(file_path, content).await?;
        Ok(())
    }

    const TEST_EMAIL_1: &str = r#"From: sender1@example.com
To: recipient@example.com
Subject: Test Message 1
Date: Wed, 01 Jan 2020 12:00:00 +0000
Message-ID: <test1@example.com>

This is the body of test message 1."#;

    const TEST_EMAIL_2: &str = r#"From: sender2@example.com
To: recipient@example.com
Subject: Test Message 2
Date: Thu, 02 Jan 2020 12:00:00 +0000
Message-ID: <test2@example.com>

This is the body of test message 2."#;

    const TEST_EMAIL_3: &str = r#"From: sender3@example.com
To: recipient@example.com
Subject: Work Message
Date: Fri, 03 Jan 2020 12:00:00 +0000
Message-ID: <test3@example.com>

This is a work-related message."#;

    #[tokio::test]
    async fn test_importer_creation() {
        let database = create_test_database().await;
        let importer = MaildirImporter::new(database);
        assert!(!importer.is_cancelled());
        assert!(importer.progress_callback.is_none());
    }

    #[tokio::test]
    async fn test_importer_with_config() {
        let database = create_test_database().await;
        let config = ImportConfig {
            max_messages: Some(100),
            skip_duplicates: false,
            validate_format: false,
            ..Default::default()
        };
        let importer = MaildirImporter::with_config(database, config.clone());
        assert_eq!(importer.config.max_messages, Some(100));
        assert!(!importer.config.skip_duplicates);
        assert!(!importer.config.validate_format);
    }

    #[tokio::test]
    async fn test_cancellation() {
        let database = create_test_database().await;
        let importer = MaildirImporter::new(database);
        
        assert!(!importer.is_cancelled());
        importer.cancel();
        assert!(importer.is_cancelled());
    }

    #[tokio::test]
    async fn test_is_valid_maildir_folder() {
        let temp_dir = TempDir::new().unwrap();
        let database = create_test_database().await;
        let importer = MaildirImporter::new(database);
        
        // Create valid Maildir structure
        let valid_path = temp_dir.path().join("valid");
        create_maildir_folder(&valid_path).await.unwrap();
        
        assert!(importer.is_valid_maildir_folder(&valid_path).await.unwrap());
        
        // Test invalid structure (missing directories)
        let invalid_path = temp_dir.path().join("invalid");
        fs::create_dir_all(&invalid_path).await.unwrap();
        
        assert!(!importer.is_valid_maildir_folder(&invalid_path).await.unwrap());
    }

    #[tokio::test]
    async fn test_count_messages_in_dir() {
        let temp_dir = TempDir::new().unwrap();
        let database = create_test_database().await;
        let importer = MaildirImporter::new(database);
        
        let test_dir = temp_dir.path().join("test");
        fs::create_dir_all(&test_dir).await.unwrap();
        
        // Initially empty
        assert_eq!(importer.count_messages_in_dir(&test_dir).await.unwrap(), 0);
        
        // Add some files
        fs::write(test_dir.join("msg1"), "content1").await.unwrap();
        fs::write(test_dir.join("msg2"), "content2").await.unwrap();
        
        assert_eq!(importer.count_messages_in_dir(&test_dir).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_analyze_maildir_folder() {
        let temp_dir = TempDir::new().unwrap();
        let database = create_test_database().await;
        let importer = MaildirImporter::new(database);
        
        let base_path = temp_dir.path();
        let folder_path = base_path.join("test_account").join("INBOX");
        
        create_maildir_folder(&folder_path).await.unwrap();
        create_test_message(&folder_path.join("new"), "msg1", "content1").await.unwrap();
        create_test_message(&folder_path.join("cur"), "msg2", "content2").await.unwrap();
        create_test_message(&folder_path.join("cur"), "msg3", "content3").await.unwrap();
        
        let folder_info = importer
            .analyze_maildir_folder(base_path, &folder_path)
            .await
            .unwrap();
        
        assert_eq!(folder_info.message_count, 3);
        assert_eq!(folder_info.new_messages, 1);
        assert_eq!(folder_info.cur_messages, 2);
        assert_eq!(folder_info.imap_name, "INBOX");
    }

    #[tokio::test]
    async fn test_parse_email_content_with_headers() {
        let database = create_test_database().await;
        let importer = MaildirImporter::new(database);
        
        let message = importer
            .parse_email_content(TEST_EMAIL_1, "test_account", "INBOX")
            .unwrap();
        
        assert_eq!(message.subject, "Test Message 1");
        assert_eq!(message.from_addr, "sender1@example.com");
        assert_eq!(message.to_addrs, vec!["recipient@example.com"]);
        assert_eq!(message.message_id, Some("<test1@example.com>".to_string()));
        assert_eq!(message.body_text, Some("This is the body of test message 1.".to_string()));
    }

    #[tokio::test]
    async fn test_parse_email_content_no_headers() {
        let database = create_test_database().await;
        let mut importer = MaildirImporter::new(database);
        
        // Disable validation for this test
        importer.config.validate_format = false;
        
        let content = "Just a body with no headers";
        let message = importer
            .parse_email_content(content, "test_account", "INBOX")
            .unwrap();
        
        assert_eq!(message.subject, "");
        assert_eq!(message.from_addr, "");
        assert!(message.to_addrs.is_empty());
        assert_eq!(message.message_id, None);
        assert_eq!(message.body_text, Some("Just a body with no headers".to_string()));
    }

    #[tokio::test]
    async fn test_validate_message_format() {
        let database = create_test_database().await;
        let mut importer = MaildirImporter::new(database);
        
        // Configure validation
        importer.config.validate_format = true;
        
        // Valid message
        let valid_message = StoredMessage {
            id: Uuid::new_v4(),
            account_id: "test".to_string(),
            folder_name: "INBOX".to_string(),
            imap_uid: 0,
            message_id: None,
            thread_id: None,
            in_reply_to: None,
            references: Vec::new(),
            subject: "Test".to_string(),
            from_addr: "test@example.com".to_string(),
            from_name: None,
            to_addrs: Vec::new(),
            cc_addrs: Vec::new(),
            bcc_addrs: Vec::new(),
            reply_to: None,
            date: TimestampUtils::now_utc(),
            body_text: Some("Body".to_string()),
            body_html: None,
            attachments: Vec::new(),
            flags: Vec::new(),
            labels: Vec::new(),
            size: Some(100),
            priority: None,
            created_at: TimestampUtils::now_utc(),
            updated_at: TimestampUtils::now_utc(),
            last_synced: TimestampUtils::now_utc(),
            sync_version: 1,
            is_draft: false,
            is_deleted: false,
        };
        
        assert!(importer.validate_message_format(&valid_message).is_ok());
        
        // Invalid message (no from address)
        let mut invalid_message = valid_message.clone();
        invalid_message.from_addr = String::new();
        
        assert!(importer.validate_message_format(&invalid_message).is_err());
    }

    #[tokio::test]
    async fn test_scan_maildir_structure() {
        let temp_dir = TempDir::new().unwrap();
        let database = create_test_database().await;
        let importer = MaildirImporter::new(database);
        
        create_mock_maildir(temp_dir.path()).await.unwrap();
        
        let mut stats = ImportStats::default();
        let folders = importer
            .scan_maildir_structure(temp_dir.path(), &mut stats)
            .await
            .unwrap();
        
        assert_eq!(folders.len(), 2); // INBOX and INBOX__Work
        assert_eq!(stats.maildir_folders_found, 2);
        assert_eq!(stats.messages_found, 3); // Total messages across all folders
        assert!(stats.directories_scanned > 0);
    }

    #[tokio::test]
    async fn test_import_stats_calculations() {
        let mut stats = ImportStats::default();
        stats.messages_found = 100;
        stats.messages_imported = 80;
        stats.messages_failed = 5;
        stats.duplicates_skipped = 15;
        
        assert_eq!(stats.success_rate(), 80.0);
        
        // Test with no messages
        let empty_stats = ImportStats::default();
        assert_eq!(empty_stats.success_rate(), 0.0);
    }

    #[tokio::test]
    async fn test_import_config_defaults() {
        let config = ImportConfig::default();
        assert_eq!(config.max_messages, None);
        assert!(config.skip_duplicates);
        assert!(config.validate_format);
        assert!(!config.update_existing);
        assert!(config.preserve_timestamps);
        assert!(config.show_progress);
    }

    #[tokio::test]
    async fn test_progress_callback() {
        let database = create_test_database().await;
        let mut importer = MaildirImporter::new(database);
        
        let progress_data = Arc::new(std::sync::Mutex::new(Vec::new()));
        let progress_data_clone = progress_data.clone();
        
        importer.set_progress_callback(Box::new(move |current, total, message| {
            progress_data_clone.lock().unwrap().push((current, total, message.to_string()));
        }));
        
        assert!(importer.progress_callback.is_some());
    }

    #[tokio::test]
    async fn test_import_from_nonexistent_directory() {
        let database = create_test_database().await;
        let importer = MaildirImporter::new(database);
        
        let result = importer
            .import_from_directory("/nonexistent/path", "test_account")
            .await;
        
        assert!(result.is_err());
        if let Err(MaildirImportError::InvalidStructure(msg)) = result {
            assert!(msg.contains("does not exist"));
        } else {
            panic!("Expected InvalidStructure error");
        }
    }

    #[tokio::test]
    async fn test_import_from_file_not_directory() {
        let temp_dir = TempDir::new().unwrap();
        let database = create_test_database().await;
        let importer = MaildirImporter::new(database);
        
        // Create a file instead of directory
        let file_path = temp_dir.path().join("not_a_directory");
        fs::write(&file_path, "content").await.unwrap();
        
        let result = importer
            .import_from_directory(&file_path, "test_account")
            .await;
        
        assert!(result.is_err());
        if let Err(MaildirImportError::InvalidStructure(msg)) = result {
            assert!(msg.contains("not a directory"));
        } else {
            panic!("Expected InvalidStructure error");
        }
    }
}