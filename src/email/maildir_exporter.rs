use crate::email::{
    EmailDatabase, FolderHierarchyMapper, MaildirErrorHandler, MaildirMapper, 
    MaildirOperationContext, MaildirOperationError, StoredMessage, TimestampUtils,
};
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;
use tokio::fs;

/// Errors that can occur during Maildir export operations
#[derive(Error, Debug)]
pub enum MaildirExportError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("Path error: {0}")]
    Path(String),
    
    #[error("Export cancelled by user")]
    Cancelled,
    
    #[error("Permission denied: {0}")]
    Permission(String),
    
    #[error("Disk space insufficient")]
    DiskSpace,
}

pub type MaildirExportResult<T> = Result<T, MaildirExportError>;

/// Statistics for export operations
#[derive(Debug, Clone, Default)]
pub struct ExportStats {
    /// Total folders exported
    pub folders_exported: usize,
    /// Total messages found in database
    pub messages_found: usize,
    /// Successfully exported messages
    pub messages_exported: usize,
    /// Failed message exports
    pub messages_failed: usize,
    /// Total bytes written
    pub bytes_written: u64,
    /// Export errors encountered
    pub errors: Vec<String>,
}

impl ExportStats {
    /// Calculate success rate as percentage
    pub fn success_rate(&self) -> f64 {
        if self.messages_found == 0 {
            0.0
        } else {
            (self.messages_exported as f64 / self.messages_found as f64) * 100.0
        }
    }
    
    /// Check if export was successful overall
    pub fn is_successful(&self) -> bool {
        self.messages_failed == 0 && self.errors.is_empty()
    }
    
    /// Get human-readable size of bytes written
    pub fn bytes_written_human(&self) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = self.bytes_written as f64;
        let mut unit_index = 0;
        
        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }
        
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

/// Configuration for export operations
#[derive(Debug, Clone)]
pub struct ExportConfig {
    /// Include draft messages in export
    pub include_drafts: bool,
    /// Include deleted messages in export
    pub include_deleted: bool,
    /// Preserve original message timestamps
    pub preserve_timestamps: bool,
    /// Compress output (future feature)
    pub compress: bool,
    /// Show progress during export
    pub show_progress: bool,
    /// Maximum number of messages per folder (None = unlimited)
    pub max_messages_per_folder: Option<usize>,
    /// Overwrite existing files
    pub overwrite_existing: bool,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            include_drafts: true,
            include_deleted: false,
            preserve_timestamps: true,
            compress: false,
            show_progress: true,
            max_messages_per_folder: None,
            overwrite_existing: false,
        }
    }
}

/// Export progress callback function type
pub type ExportProgressCallback = Box<dyn Fn(usize, usize, &str) + Send + Sync>;

/// Async Maildir exporter with progress tracking and error handling
pub struct MaildirExporter {
    /// Database connection
    database: Arc<EmailDatabase>,
    /// Maildir metadata mapper
    mapper: MaildirMapper,
    /// Folder hierarchy mapper
    folder_mapper: FolderHierarchyMapper,
    /// Export configuration
    config: ExportConfig,
    /// Progress callback
    progress_callback: Option<ExportProgressCallback>,
    /// Cancellation flag
    cancelled: Arc<std::sync::atomic::AtomicBool>,
    /// Error handler for robust error handling
    error_handler: MaildirErrorHandler,
    /// Operation context for detailed error reporting
    operation_context: MaildirOperationContext,
}

impl MaildirExporter {
    /// Create a new Maildir exporter
    pub fn new(database: Arc<EmailDatabase>) -> Self {
        Self {
            database,
            mapper: MaildirMapper::new(),
            folder_mapper: FolderHierarchyMapper::new(),
            config: ExportConfig::default(),
            progress_callback: None,
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            error_handler: MaildirErrorHandler::default(),
            operation_context: MaildirOperationContext::new("Maildir Export".to_string()),
        }
    }

    /// Create exporter with custom configuration
    pub fn with_config(database: Arc<EmailDatabase>, config: ExportConfig) -> Self {
        Self {
            database,
            mapper: MaildirMapper::new(),
            folder_mapper: FolderHierarchyMapper::new(),
            config,
            progress_callback: None,
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            error_handler: MaildirErrorHandler::default(),
            operation_context: MaildirOperationContext::new("Maildir Export".to_string()),
        }
    }

    /// Set progress callback for tracking export progress
    pub fn set_progress_callback(&mut self, callback: ExportProgressCallback) {
        self.progress_callback = Some(callback);
    }

    /// Cancel the current export operation
    pub fn cancel(&self) {
        self.cancelled.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    /// Check if export has been cancelled
    fn is_cancelled(&self) -> bool {
        self.cancelled.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Export all messages from an account to Maildir directory structure
    pub async fn export_account<P: AsRef<Path>>(
        &self,
        account_id: &str,
        output_path: P,
    ) -> MaildirExportResult<ExportStats> {
        let output = output_path.as_ref();
        
        // Create base directory
        fs::create_dir_all(output).await?;
        
        let mut stats = ExportStats::default();
        
        // Get all folders for the account
        let folders = self.database
            .get_folders(account_id)
            .await
            .map_err(|e| MaildirExportError::Database(e.to_string()))?;

        // Count total messages for progress tracking
        for folder in &folders {
            let messages = self.database
                .get_messages(account_id, &folder.name, None, None)
                .await
                .map_err(|e| MaildirExportError::Database(e.to_string()))?;
            stats.messages_found += messages.len();
        }

        let progress_bar = if self.config.show_progress {
            Some(self.create_progress_bar(stats.messages_found))
        } else {
            None
        };

        // Export each folder
        for folder in folders {
            if self.is_cancelled() {
                return Err(MaildirExportError::Cancelled);
            }

            self.export_folder(
                account_id,
                &folder.name,
                output,
                &mut stats,
                &progress_bar,
            ).await?;
        }

        if let Some(pb) = progress_bar {
            pb.finish_with_message("Export completed");
        }

        Ok(stats)
    }

    /// Export a single folder to Maildir format
    pub async fn export_folder<P: AsRef<Path>>(
        &self,
        account_id: &str,
        folder_name: &str,
        output_path: P,
        stats: &mut ExportStats,
        progress_bar: &Option<ProgressBar>,
    ) -> MaildirExportResult<()> {
        let output = output_path.as_ref();
        
        // Create folder-specific Maildir structure
        let folder_path = self.folder_mapper
            .create_maildir_path(output, account_id, folder_name)
            .map_err(|e| MaildirExportError::Path(e.to_string()))?;

        self.ensure_maildir_structure(&folder_path).await?;

        // Get messages from database
        let messages = self.database
            .get_messages(account_id, folder_name, None, None)
            .await
            .map_err(|e| MaildirExportError::Database(e.to_string()))?;

        // Filter messages based on configuration
        let filtered_messages = self.filter_messages(messages);
        
        // Limit messages if configured
        let limited_messages = if let Some(max) = self.config.max_messages_per_folder {
            filtered_messages.into_iter().take(max).collect()
        } else {
            filtered_messages
        };

        // Export each message
        for message in limited_messages {
            if self.is_cancelled() {
                return Err(MaildirExportError::Cancelled);
            }

            match self.export_message(&folder_path, &message).await {
                Ok(bytes_written) => {
                    stats.messages_exported += 1;
                    stats.bytes_written += bytes_written;
                }
                Err(e) => {
                    stats.messages_failed += 1;
                    stats.errors.push(format!("Message {}: {}", message.id, e));
                }
            }

            if let Some(ref pb) = progress_bar {
                pb.inc(1);
            }

            // Call progress callback if set
            if let Some(ref callback) = self.progress_callback {
                callback(
                    stats.messages_exported + stats.messages_failed,
                    stats.messages_found,
                    &format!("Exporting {}", folder_name),
                );
            }
        }

        stats.folders_exported += 1;
        Ok(())
    }

    /// Export a single message to Maildir format
    async fn export_message<P: AsRef<Path>>(
        &self,
        folder_path: P,
        message: &StoredMessage,
    ) -> MaildirExportResult<u64> {
        let folder = folder_path.as_ref();
        
        // Determine if message should go in new/ or cur/ based on flags
        let is_seen = message.flags.iter().any(|f| f == "\\Seen");
        let target_dir = if is_seen {
            folder.join("cur")
        } else {
            folder.join("new")
        };

        // Generate Maildir-compliant filename
        let filename = self.mapper
            .generate_maildir_filename(message, is_seen)
            .map_err(|e| MaildirExportError::Serialization(e.to_string()))?;

        let file_path = target_dir.join(filename);

        // Check if file already exists and handle accordingly
        if file_path.exists() && !self.config.overwrite_existing {
            return Err(MaildirExportError::Io(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("File already exists: {:?}", file_path),
            )));
        }

        // Serialize message to RFC822 format
        let email_content = self.serialize_message_to_rfc822(message)?;
        
        // Write to file
        fs::write(&file_path, &email_content).await?;
        
        // Preserve original timestamp if configured
        if self.config.preserve_timestamps {
            if let Err(e) = TimestampUtils::set_file_modification_time(&file_path, &message.date) {
                // Log warning but don't fail the export
                tracing::warn!("Failed to preserve timestamp for {:?}: {}", file_path, e);
            }
        }

        Ok(email_content.len() as u64)
    }

    /// Serialize a StoredMessage to RFC822 email format
    fn serialize_message_to_rfc822(&self, message: &StoredMessage) -> MaildirExportResult<String> {
        let mut email = String::new();

        // Required headers
        if let Some(ref message_id) = message.message_id {
            email.push_str(&format!("Message-ID: {}\r\n", message_id));
        } else {
            // Generate a message ID if missing
            email.push_str(&format!("Message-ID: <{}.{}@comunicado>\r\n", 
                message.date.timestamp(), message.id));
        }

        email.push_str(&format!(
            "Date: {}\r\n",
            TimestampUtils::format_rfc2822(&message.date)
        ));

        email.push_str(&format!("From: {}\r\n", message.from_addr));
        
        if let Some(ref from_name) = message.from_name {
            email = email.replace(&format!("From: {}", message.from_addr), 
                &format!("From: {} <{}>\r\n", from_name, message.from_addr));
        }

        email.push_str(&format!("Subject: {}\r\n", message.subject));

        // Recipient headers
        if !message.to_addrs.is_empty() {
            email.push_str(&format!("To: {}\r\n", message.to_addrs.join(", ")));
        }

        if !message.cc_addrs.is_empty() {
            email.push_str(&format!("Cc: {}\r\n", message.cc_addrs.join(", ")));
        }

        // Optional headers
        if let Some(ref reply_to) = message.reply_to {
            email.push_str(&format!("Reply-To: {}\r\n", reply_to));
        }

        if let Some(ref in_reply_to) = message.in_reply_to {
            email.push_str(&format!("In-Reply-To: {}\r\n", in_reply_to));
        }

        if !message.references.is_empty() {
            email.push_str(&format!("References: {}\r\n", message.references.join(" ")));
        }

        // Priority header
        if let Some(ref priority) = message.priority {
            email.push_str(&format!("X-Priority: {}\r\n", priority));
        }

        // Content headers based on message content
        if message.body_html.is_some() && message.body_text.is_some() {
            // Multipart message
            let boundary = format!("boundary_{}", message.id.to_string().replace('-', ""));
            email.push_str(&format!("Content-Type: multipart/alternative; boundary=\"{}\"\r\n", boundary));
            email.push_str("MIME-Version: 1.0\r\n");
            email.push_str("\r\n");

            // Text part
            email.push_str(&format!("--{}\r\n", boundary));
            email.push_str("Content-Type: text/plain; charset=UTF-8\r\n");
            email.push_str("Content-Transfer-Encoding: 8bit\r\n");
            email.push_str("\r\n");
            if let Some(ref body_text) = message.body_text {
                email.push_str(body_text);
            }
            email.push_str("\r\n");

            // HTML part
            email.push_str(&format!("--{}\r\n", boundary));
            email.push_str("Content-Type: text/html; charset=UTF-8\r\n");
            email.push_str("Content-Transfer-Encoding: 8bit\r\n");
            email.push_str("\r\n");
            if let Some(ref body_html) = message.body_html {
                email.push_str(body_html);
            }
            email.push_str("\r\n");

            email.push_str(&format!("--{}--\r\n", boundary));
        } else if let Some(ref body_html) = message.body_html {
            // HTML only
            email.push_str("Content-Type: text/html; charset=UTF-8\r\n");
            email.push_str("Content-Transfer-Encoding: 8bit\r\n");
            email.push_str("MIME-Version: 1.0\r\n");
            email.push_str("\r\n");
            email.push_str(body_html);
        } else {
            // Plain text or no body
            email.push_str("Content-Type: text/plain; charset=UTF-8\r\n");
            email.push_str("Content-Transfer-Encoding: 8bit\r\n");
            email.push_str("MIME-Version: 1.0\r\n");
            email.push_str("\r\n");
            if let Some(ref body_text) = message.body_text {
                email.push_str(body_text);
            }
        }

        Ok(email)
    }

    /// Filter messages based on export configuration
    fn filter_messages(&self, messages: Vec<StoredMessage>) -> Vec<StoredMessage> {
        messages.into_iter()
            .filter(|msg| {
                // Filter drafts if not included
                if msg.is_draft && !self.config.include_drafts {
                    return false;
                }
                
                // Filter deleted messages if not included
                if msg.is_deleted && !self.config.include_deleted {
                    return false;
                }
                
                true
            })
            .collect()
    }

    /// Ensure Maildir directory structure exists
    async fn ensure_maildir_structure<P: AsRef<Path>>(&self, path: P) -> MaildirExportResult<()> {
        let path = path.as_ref();
        fs::create_dir_all(path.join("new")).await?;
        fs::create_dir_all(path.join("cur")).await?;
        fs::create_dir_all(path.join("tmp")).await?;
        Ok(())
    }

    /// Create progress bar for export operations
    fn create_progress_bar(&self, total: usize) -> ProgressBar {
        let pb = ProgressBar::new(total as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg} ({eta})")
                .unwrap()
                .progress_chars("#>-"),
        );
        pb.set_message("Exporting messages...");
        pb
    }

    /// Get export configuration
    pub fn config(&self) -> &ExportConfig {
        &self.config
    }

    /// Update export configuration
    pub fn set_config(&mut self, config: ExportConfig) {
        self.config = config;
    }

    /// Validate that the export directory has sufficient space
    pub async fn check_disk_space<P: AsRef<Path>>(
        output_path: P,
        estimated_size: u64,
    ) -> MaildirExportResult<bool> {
        // This is a simplified check - in production, you'd use system calls
        // to check actual disk space
        let path = output_path.as_ref();
        
        if !path.exists() {
            return Err(MaildirExportError::Path(format!(
                "Output path does not exist: {:?}",
                path
            )));
        }

        // For now, just return true - in production, implement actual disk space check
        Ok(true)
    }

    /// Get export statistics for an account without actually exporting
    pub async fn get_export_preview(
        &self,
        account_id: &str,
    ) -> MaildirExportResult<ExportPreview> {
        let folders = self.database
            .get_folders(account_id)
            .await
            .map_err(|e| MaildirExportError::Database(e.to_string()))?;

        let mut preview = ExportPreview {
            total_folders: folders.len(),
            total_messages: 0,
            estimated_size: 0,
            folders: Vec::new(),
        };

        for folder in folders {
            let messages = self.database
                .get_messages(account_id, &folder.name, None, None)
                .await
                .map_err(|e| MaildirExportError::Database(e.to_string()))?;

            let filtered_messages = self.filter_messages(messages);
            let message_count = filtered_messages.len();
            let estimated_folder_size: u64 = filtered_messages
                .iter()
                .map(|msg| msg.size.unwrap_or(1024) as u64)
                .sum();

            preview.total_messages += message_count;
            preview.estimated_size += estimated_folder_size;
            preview.folders.push(ExportFolderPreview {
                name: folder.name,
                message_count,
                estimated_size: estimated_folder_size,
            });
        }

        Ok(preview)
    }
}

/// Preview information for export operations
#[derive(Debug, Clone)]
pub struct ExportPreview {
    /// Total number of folders to export
    pub total_folders: usize,
    /// Total number of messages to export
    pub total_messages: usize,
    /// Estimated size in bytes
    pub estimated_size: u64,
    /// Per-folder preview information
    pub folders: Vec<ExportFolderPreview>,
}

impl ExportPreview {
    /// Get human-readable size estimate
    pub fn estimated_size_human(&self) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = self.estimated_size as f64;
        let mut unit_index = 0;
        
        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }
        
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

/// Preview information for a single folder
#[derive(Debug, Clone)]
pub struct ExportFolderPreview {
    /// Folder name
    pub name: String,
    /// Number of messages in folder
    pub message_count: usize,
    /// Estimated size in bytes
    pub estimated_size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::email::EmailDatabase;
    use chrono::{TimeZone, Utc};
    use tempfile::TempDir;
    use uuid::Uuid;

    /// Create a test EmailDatabase
    async fn create_test_database() -> Arc<EmailDatabase> {
        Arc::new(EmailDatabase::new_in_memory().await.unwrap())
    }

    /// Create a test StoredMessage
    fn create_test_message(id: &str, folder: &str, subject: &str) -> StoredMessage {
        StoredMessage {
            id: Uuid::new_v4(),
            account_id: "test_account".to_string(),
            folder_name: folder.to_string(),
            imap_uid: 123,
            message_id: Some(format!("<{}@example.com>", id)),
            thread_id: None,
            in_reply_to: None,
            references: Vec::new(),
            subject: subject.to_string(),
            from_addr: "sender@example.com".to_string(),
            from_name: Some("Test Sender".to_string()),
            to_addrs: vec!["recipient@example.com".to_string()],
            cc_addrs: Vec::new(),
            bcc_addrs: Vec::new(),
            reply_to: None,
            date: Utc.with_ymd_and_hms(2021, 1, 1, 12, 0, 0).unwrap(),
            body_text: Some("This is the body text.".to_string()),
            body_html: Some("<p>This is the body text.</p>".to_string()),
            attachments: Vec::new(),
            flags: vec!["\\Seen".to_string()],
            labels: Vec::new(),
            size: Some(1024),
            priority: None,
            created_at: TimestampUtils::now_utc(),
            updated_at: TimestampUtils::now_utc(),
            last_synced: TimestampUtils::now_utc(),
            sync_version: 1,
            is_draft: false,
            is_deleted: false,
        }
    }

    #[tokio::test]
    async fn test_exporter_creation() {
        let database = create_test_database().await;
        let exporter = MaildirExporter::new(database);
        assert!(!exporter.is_cancelled());
        assert!(exporter.progress_callback.is_none());
    }

    #[tokio::test]
    async fn test_exporter_with_config() {
        let database = create_test_database().await;
        let config = ExportConfig {
            include_drafts: false,
            include_deleted: true,
            preserve_timestamps: false,
            ..Default::default()
        };
        let exporter = MaildirExporter::with_config(database, config.clone());
        assert!(!exporter.config.include_drafts);
        assert!(exporter.config.include_deleted);
        assert!(!exporter.config.preserve_timestamps);
    }

    #[tokio::test]
    async fn test_cancellation() {
        let database = create_test_database().await;
        let exporter = MaildirExporter::new(database);
        
        assert!(!exporter.is_cancelled());
        exporter.cancel();
        assert!(exporter.is_cancelled());
    }

    #[tokio::test]
    async fn test_ensure_maildir_structure() {
        let temp_dir = TempDir::new().unwrap();
        let database = create_test_database().await;
        let exporter = MaildirExporter::new(database);
        
        let test_path = temp_dir.path().join("test_folder");
        exporter.ensure_maildir_structure(&test_path).await.unwrap();
        
        assert!(test_path.join("new").exists());
        assert!(test_path.join("cur").exists());
        assert!(test_path.join("tmp").exists());
    }

    #[tokio::test]
    async fn test_serialize_message_to_rfc822() {
        let database = create_test_database().await;
        let exporter = MaildirExporter::new(database);
        
        let message = create_test_message("test1", "INBOX", "Test Subject");
        let serialized = exporter.serialize_message_to_rfc822(&message).unwrap();
        
        assert!(serialized.contains("Message-ID: <test1@example.com>"));
        assert!(serialized.contains("Subject: Test Subject"));
        assert!(serialized.contains("From: Test Sender <sender@example.com>"));
        assert!(serialized.contains("To: recipient@example.com"));
        assert!(serialized.contains("multipart/alternative")); // Has both HTML and text
        assert!(serialized.contains("This is the body text."));
        assert!(serialized.contains("<p>This is the body text.</p>"));
    }

    #[tokio::test]
    async fn test_serialize_plain_text_only() {
        let database = create_test_database().await;
        let exporter = MaildirExporter::new(database);
        
        let mut message = create_test_message("test2", "INBOX", "Plain Text");
        message.body_html = None; // Remove HTML body
        
        let serialized = exporter.serialize_message_to_rfc822(&message).unwrap();
        
        assert!(serialized.contains("Content-Type: text/plain"));
        assert!(!serialized.contains("multipart"));
        assert!(serialized.contains("This is the body text."));
    }

    #[tokio::test]
    async fn test_serialize_html_only() {
        let database = create_test_database().await;
        let exporter = MaildirExporter::new(database);
        
        let mut message = create_test_message("test3", "INBOX", "HTML Only");
        message.body_text = None; // Remove text body
        
        let serialized = exporter.serialize_message_to_rfc822(&message).unwrap();
        
        assert!(serialized.contains("Content-Type: text/html"));
        assert!(!serialized.contains("multipart"));
        assert!(serialized.contains("<p>This is the body text.</p>"));
    }

    #[tokio::test]
    async fn test_filter_messages_include_all() {
        let database = create_test_database().await;
        let config = ExportConfig {
            include_drafts: true,
            include_deleted: true,
            ..Default::default()
        };
        let exporter = MaildirExporter::with_config(database, config);
        
        let mut messages = vec![
            create_test_message("msg1", "INBOX", "Normal"),
            create_test_message("msg2", "INBOX", "Draft"),
            create_test_message("msg3", "INBOX", "Deleted"),
        ];
        
        messages[1].is_draft = true;
        messages[2].is_deleted = true;
        
        let filtered = exporter.filter_messages(messages);
        assert_eq!(filtered.len(), 3); // All messages included with this config
    }

    #[tokio::test]
    async fn test_filter_messages_exclude_drafts() {
        let database = create_test_database().await;
        let mut exporter = MaildirExporter::new(database);
        exporter.config.include_drafts = false;
        
        let mut messages = vec![
            create_test_message("msg1", "INBOX", "Normal"),
            create_test_message("msg2", "INBOX", "Draft"),
        ];
        
        messages[1].is_draft = true;
        
        let filtered = exporter.filter_messages(messages);
        assert_eq!(filtered.len(), 1); // Draft excluded
        assert_eq!(filtered[0].subject, "Normal");
    }

    #[tokio::test]
    async fn test_filter_messages_exclude_deleted() {
        let database = create_test_database().await;
        let mut exporter = MaildirExporter::new(database);
        exporter.config.include_deleted = false; // Default behavior
        
        let mut messages = vec![
            create_test_message("msg1", "INBOX", "Normal"),
            create_test_message("msg2", "INBOX", "Deleted"),
        ];
        
        messages[1].is_deleted = true;
        
        let filtered = exporter.filter_messages(messages);
        assert_eq!(filtered.len(), 1); // Deleted excluded
        assert_eq!(filtered[0].subject, "Normal");
    }

    #[tokio::test]
    async fn test_export_message() {
        let temp_dir = TempDir::new().unwrap();
        let database = create_test_database().await;
        let exporter = MaildirExporter::new(database);
        
        let folder_path = temp_dir.path().join("test_folder");
        exporter.ensure_maildir_structure(&folder_path).await.unwrap();
        
        let message = create_test_message("test_export", "INBOX", "Export Test");
        let bytes_written = exporter.export_message(&folder_path, &message).await.unwrap();
        
        assert!(bytes_written > 0);
        
        // Verify file was created in cur/ (message has \\Seen flag)
        let cur_dir = folder_path.join("cur");
        let entries: Vec<_> = std::fs::read_dir(cur_dir).unwrap().collect();
        assert_eq!(entries.len(), 1);
        
        // Verify file content
        let file_path = entries[0].as_ref().unwrap().path();
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("Subject: Export Test"));
        assert!(content.contains("From: Test Sender"));
    }

    #[tokio::test]
    async fn test_export_message_new_folder() {
        let temp_dir = TempDir::new().unwrap();
        let database = create_test_database().await;
        let exporter = MaildirExporter::new(database);
        
        let folder_path = temp_dir.path().join("test_folder");
        exporter.ensure_maildir_structure(&folder_path).await.unwrap();
        
        let mut message = create_test_message("test_new", "INBOX", "New Message");
        message.flags.clear(); // Remove \\Seen flag
        
        exporter.export_message(&folder_path, &message).await.unwrap();
        
        // Verify file was created in new/ (no \\Seen flag)
        let new_dir = folder_path.join("new");
        let entries: Vec<_> = std::fs::read_dir(new_dir).unwrap().collect();
        assert_eq!(entries.len(), 1);
    }

    #[tokio::test]
    async fn test_export_stats_calculations() {
        let mut stats = ExportStats::default();
        stats.messages_found = 100;
        stats.messages_exported = 85;
        stats.messages_failed = 15;
        stats.bytes_written = 1024 * 1024; // 1MB
        
        assert_eq!(stats.success_rate(), 85.0);
        assert!(!stats.is_successful()); // Has failures
        assert_eq!(stats.bytes_written_human(), "1.0 MB");
        
        // Test with no failures
        stats.messages_failed = 0;
        stats.errors.clear();
        assert!(stats.is_successful());
    }

    #[tokio::test]
    async fn test_export_config_defaults() {
        let config = ExportConfig::default();
        assert!(config.include_drafts);
        assert!(!config.include_deleted);
        assert!(config.preserve_timestamps);
        assert!(!config.compress);
        assert!(config.show_progress);
        assert_eq!(config.max_messages_per_folder, None);
        assert!(!config.overwrite_existing);
    }

    #[tokio::test]
    async fn test_progress_callback() {
        let database = create_test_database().await;
        let mut exporter = MaildirExporter::new(database);
        
        let progress_data = Arc::new(std::sync::Mutex::new(Vec::new()));
        let progress_data_clone = progress_data.clone();
        
        exporter.set_progress_callback(Box::new(move |current, total, message| {
            progress_data_clone.lock().unwrap().push((current, total, message.to_string()));
        }));
        
        assert!(exporter.progress_callback.is_some());
    }

    #[tokio::test]
    async fn test_check_disk_space() {
        let temp_dir = TempDir::new().unwrap();
        
        // Existing path should return true
        let result = MaildirExporter::check_disk_space(temp_dir.path(), 1024).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
        
        // Non-existent path should return error
        let result = MaildirExporter::check_disk_space("/nonexistent/path", 1024).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_export_preview_empty() {
        let database = create_test_database().await;
        let exporter = MaildirExporter::new(database);
        
        // Since we're using in-memory database with no data, this should return empty preview
        let preview = exporter.get_export_preview("nonexistent_account").await.unwrap();
        assert_eq!(preview.total_folders, 0);
        assert_eq!(preview.total_messages, 0);
        assert_eq!(preview.estimated_size, 0);
        assert!(preview.folders.is_empty());
    }

    #[tokio::test]
    async fn test_export_preview_size_formatting() {
        let preview = ExportPreview {
            total_folders: 1,
            total_messages: 100,
            estimated_size: 1536, // 1.5 KB
            folders: Vec::new(),
        };
        
        assert_eq!(preview.estimated_size_human(), "1.5 KB");
        
        let large_preview = ExportPreview {
            total_folders: 1,
            total_messages: 1000,
            estimated_size: 2 * 1024 * 1024 * 1024, // 2 GB
            folders: Vec::new(),
        };
        
        assert_eq!(large_preview.estimated_size_human(), "2.0 GB");
    }

    #[tokio::test]
    async fn test_overwrite_existing_disabled() {
        let temp_dir = TempDir::new().unwrap();
        let database = create_test_database().await;
        let mut exporter = MaildirExporter::new(database);
        exporter.config.overwrite_existing = false;
        
        let folder_path = temp_dir.path().join("test_folder");
        exporter.ensure_maildir_structure(&folder_path).await.unwrap();
        
        let message = create_test_message("test_overwrite", "INBOX", "Overwrite Test");
        
        // First export should succeed
        exporter.export_message(&folder_path, &message).await.unwrap();
        
        // Second export should fail due to existing file
        let result = exporter.export_message(&folder_path, &message).await;
        assert!(result.is_err());
        if let Err(MaildirExportError::Io(io_err)) = result {
            assert_eq!(io_err.kind(), std::io::ErrorKind::AlreadyExists);
        } else {
            panic!("Expected IO error with AlreadyExists kind");
        }
    }

    #[tokio::test]
    async fn test_overwrite_existing_enabled() {
        let temp_dir = TempDir::new().unwrap();
        let database = create_test_database().await;
        let mut exporter = MaildirExporter::new(database);
        exporter.config.overwrite_existing = true;
        
        let folder_path = temp_dir.path().join("test_folder");
        exporter.ensure_maildir_structure(&folder_path).await.unwrap();
        
        let message = create_test_message("test_overwrite", "INBOX", "Overwrite Test");
        
        // First export
        exporter.export_message(&folder_path, &message).await.unwrap();
        
        // Second export should succeed with overwrite enabled
        let result = exporter.export_message(&folder_path, &message).await;
        assert!(result.is_ok());
    }
}