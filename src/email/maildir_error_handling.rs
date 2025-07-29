/// Comprehensive error handling for Maildir operations
/// 
/// This module provides robust error handling, recovery mechanisms, and user-friendly
/// error reporting for Maildir import/export operations.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::fs;

/// Enhanced error types with detailed context and recovery suggestions
#[derive(Error, Debug)]
pub enum MaildirOperationError {
    #[error("Permission denied: {message}")]
    PermissionDenied {
        message: String,
        path: PathBuf,
        suggestion: String,
    },
    
    #[error("Insufficient disk space: {message}")]
    InsufficientSpace {
        message: String,
        required_bytes: u64,
        available_bytes: u64,
        suggestion: String,
    },
    
    #[error("File system full: {message}")]
    FileSystemFull {
        message: String,
        path: PathBuf,
        suggestion: String,
    },
    
    #[error("Network drive unavailable: {message}")]
    NetworkDriveUnavailable {
        message: String,
        path: PathBuf,
        suggestion: String,
    },
    
    #[error("File or directory not found: {message}")]
    NotFound {
        message: String,
        path: PathBuf,
        suggestion: String,
    },
    
    #[error("File already exists: {message}")]
    AlreadyExists {
        message: String,
        path: PathBuf,
        suggestion: String,
    },
    
    #[error("Invalid file name: {message}")]
    InvalidFileName {
        message: String,
        filename: String,
        suggestion: String,
    },
    
    #[error("Path too long: {message}")]
    PathTooLong {
        message: String,
        path: PathBuf,
        max_length: usize,
        suggestion: String,
    },
    
    #[error("Read-only file system: {message}")]
    ReadOnlyFileSystem {
        message: String,
        path: PathBuf,
        suggestion: String,
    },
    
    #[error("File is locked or in use: {message}")]
    FileLocked {
        message: String,
        path: PathBuf,
        suggestion: String,
    },
    
    #[error("Corrupted file system: {message}")]
    CorruptedFileSystem {
        message: String,
        path: PathBuf,
        suggestion: String,
    },
    
    #[error("Interrupted operation: {message}")]
    Interrupted {
        message: String,
        operation: String,
        progress: f64,
        suggestion: String,
    },
    
    #[error("Unknown I/O error: {message}")]
    Unknown {
        message: String,
        source: Option<io::Error>,
        suggestion: String,
    },
}

impl MaildirOperationError {
    /// Get the user-friendly suggestion for resolving this error
    pub fn suggestion(&self) -> &str {
        match self {
            Self::PermissionDenied { suggestion, .. } => suggestion,
            Self::InsufficientSpace { suggestion, .. } => suggestion,
            Self::FileSystemFull { suggestion, .. } => suggestion,
            Self::NetworkDriveUnavailable { suggestion, .. } => suggestion,
            Self::NotFound { suggestion, .. } => suggestion,
            Self::AlreadyExists { suggestion, .. } => suggestion,
            Self::InvalidFileName { suggestion, .. } => suggestion,
            Self::PathTooLong { suggestion, .. } => suggestion,
            Self::ReadOnlyFileSystem { suggestion, .. } => suggestion,
            Self::FileLocked { suggestion, .. } => suggestion,
            Self::CorruptedFileSystem { suggestion, .. } => suggestion,
            Self::Interrupted { suggestion, .. } => suggestion,
            Self::Unknown { suggestion, .. } => suggestion,
        }
    }
    
    /// Get the affected path (if any)
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::PermissionDenied { path, .. } => Some(path),
            Self::FileSystemFull { path, .. } => Some(path),
            Self::NetworkDriveUnavailable { path, .. } => Some(path),
            Self::NotFound { path, .. } => Some(path),
            Self::AlreadyExists { path, .. } => Some(path),
            Self::PathTooLong { path, .. } => Some(path),
            Self::ReadOnlyFileSystem { path, .. } => Some(path),
            Self::FileLocked { path, .. } => Some(path),
            Self::CorruptedFileSystem { path, .. } => Some(path),
            _ => None,
        }
    }
    
    /// Check if this error is recoverable (e.g., retry might work)
    pub fn is_recoverable(&self) -> bool {
        matches!(self, 
            Self::FileLocked { .. } |
            Self::NetworkDriveUnavailable { .. } |
            Self::Interrupted { .. }
        )
    }
    
    /// Check if this error requires user intervention
    pub fn requires_user_intervention(&self) -> bool {
        matches!(self,
            Self::PermissionDenied { .. } |
            Self::InsufficientSpace { .. } |
            Self::FileSystemFull { .. } |
            Self::ReadOnlyFileSystem { .. } |
            Self::PathTooLong { .. } |
            Self::CorruptedFileSystem { .. }
        )
    }
}

/// Error handler for Maildir operations with advanced diagnostics
pub struct MaildirErrorHandler {
    /// Enable detailed error diagnostics
    pub detailed_diagnostics: bool,
    /// Maximum retry attempts for recoverable errors
    pub max_retry_attempts: usize,
    /// Retry delay in milliseconds
    pub retry_delay_ms: u64,
}

impl Default for MaildirErrorHandler {
    fn default() -> Self {
        Self {
            detailed_diagnostics: true,
            max_retry_attempts: 3,
            retry_delay_ms: 1000,
        }
    }
}

impl MaildirErrorHandler {
    /// Create a new error handler with custom settings
    pub fn new(detailed_diagnostics: bool, max_retry_attempts: usize, retry_delay_ms: u64) -> Self {
        Self {
            detailed_diagnostics,
            max_retry_attempts,
            retry_delay_ms,
        }
    }
    
    /// Convert a standard IO error to a detailed Maildir operation error
    pub async fn classify_error<P: AsRef<Path>>(
        &self,
        error: io::Error,
        path: P,
        operation: &str,
    ) -> MaildirOperationError {
        let path = path.as_ref().to_path_buf();
        
        match error.kind() {
            io::ErrorKind::PermissionDenied => {
                let suggestion = self.suggest_permission_fix(&path).await;
                MaildirOperationError::PermissionDenied {
                    message: format!("Permission denied while {}: {}", operation, error),
                    path,
                    suggestion,
                }
            },
            
            io::ErrorKind::NotFound => {
                let suggestion = self.suggest_not_found_fix(&path, operation).await;
                MaildirOperationError::NotFound {
                    message: format!("File or directory not found during {}: {}", operation, error),
                    path,
                    suggestion,
                }
            },
            
            io::ErrorKind::AlreadyExists => {
                let suggestion = self.suggest_already_exists_fix(&path, operation).await;
                MaildirOperationError::AlreadyExists {
                    message: format!("File already exists during {}: {}", operation, error),
                    path,
                    suggestion,
                }
            },
            
            io::ErrorKind::InvalidInput => {
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    MaildirOperationError::InvalidFileName {
                        message: format!("Invalid file name during {}: {}", operation, error),
                        filename: filename.to_string(),
                        suggestion: "Use only valid characters in filenames. Avoid special characters like <>:\"|?*".to_string(),
                    }
                } else {
                    self.classify_generic_error(error, path, operation).await
                }
            },
            
            io::ErrorKind::StorageFull => {
                let (available, required) = self.check_disk_space(&path).await;
                if available == 0 {
                    MaildirOperationError::FileSystemFull {
                        message: format!("File system full during {}: {}", operation, error),
                        path,
                        suggestion: "Free up disk space by deleting unnecessary files or moving the operation to a different drive.".to_string(),
                    }
                } else {
                    MaildirOperationError::InsufficientSpace {
                        message: format!("Insufficient disk space during {}: {}", operation, error),
                        required_bytes: required,
                        available_bytes: available,
                        suggestion: format!("Need {} MB more space. Free up disk space or choose a different location.", 
                                          (required - available) / 1024 / 1024),
                    }
                }
            },
            
            io::ErrorKind::Interrupted => {
                MaildirOperationError::Interrupted {
                    message: format!("Operation interrupted during {}: {}", operation, error),
                    operation: operation.to_string(),
                    progress: 0.0, // Would be tracked externally
                    suggestion: "The operation was interrupted. You can retry or resume from where it left off.".to_string(),
                }
            },
            
            _ => self.classify_generic_error(error, path, operation).await,
        }
    }
    
    /// Classify generic or platform-specific errors
    async fn classify_generic_error(
        &self,
        error: io::Error,
        path: PathBuf,
        operation: &str,
    ) -> MaildirOperationError {
        // Check for platform-specific error codes
        #[cfg(windows)]
        {
            if let Some(raw_error) = error.raw_os_error() {
                match raw_error {
                    5 => return MaildirOperationError::PermissionDenied {
                        message: format!("Access denied during {}: {}", operation, error),
                        path,
                        suggestion: "Run as administrator or check file permissions.".to_string(),
                    },
                    32 => return MaildirOperationError::FileLocked {
                        message: format!("File is locked or in use during {}: {}", operation, error),
                        path,
                        suggestion: "Close any applications using this file and try again.".to_string(),
                    },
                    206 => return MaildirOperationError::PathTooLong {
                        message: format!("Path too long during {}: {}", operation, error),
                        path: path.clone(),
                        max_length: 260, // Windows MAX_PATH
                        suggestion: "Use shorter file names or move to a directory with a shorter path.".to_string(),
                    },
                    _ => {}
                }
            }
        }
        
        #[cfg(unix)]
        {
            if let Some(raw_error) = error.raw_os_error() {
                match raw_error {
                    1 => return MaildirOperationError::PermissionDenied {
                        message: format!("Operation not permitted during {}: {}", operation, error),
                        path,
                        suggestion: "Check file permissions or run with appropriate privileges.".to_string(),
                    },
                    13 => return MaildirOperationError::PermissionDenied {
                        message: format!("Permission denied during {}: {}", operation, error),
                        path,
                        suggestion: "Check file permissions. You may need to use 'chmod' or 'sudo'.".to_string(),
                    },
                    26 => return MaildirOperationError::FileLocked {
                        message: format!("Text file busy during {}: {}", operation, error),
                        path,
                        suggestion: "File is being used by another process. Wait and try again.".to_string(),
                    },
                    28 => return MaildirOperationError::InsufficientSpace {
                        message: format!("No space left on device during {}: {}", operation, error),
                        required_bytes: 0,
                        available_bytes: 0,
                        suggestion: "Free up disk space by deleting unnecessary files.".to_string(),
                    },
                    30 => return MaildirOperationError::ReadOnlyFileSystem {
                        message: format!("Read-only file system during {}: {}", operation, error),
                        path,
                        suggestion: "The file system is mounted read-only. Remount with write permissions.".to_string(),
                    },
                    36 => return MaildirOperationError::PathTooLong {
                        message: format!("File name too long during {}: {}", operation, error),
                        path: path.clone(),
                        max_length: 255, // Most Unix filesystems
                        suggestion: "Use shorter file names.".to_string(),
                    },
                    _ => {}
                }
            }
        }
        
        // Check if it might be a network drive issue
        if self.is_network_path(&path).await {
            return MaildirOperationError::NetworkDriveUnavailable {
                message: format!("Network drive unavailable during {}: {}", operation, error),
                path,
                suggestion: "Check network connection and ensure the remote drive is accessible.".to_string(),
            };
        }
        
        // Fallback to unknown error
        MaildirOperationError::Unknown {
            message: format!("Unknown error during {}: {}", operation, error),
            source: Some(error),
            suggestion: "This is an unexpected error. Please check the system logs and try again.".to_string(),
        }
    }
    
    /// Generate suggestion for permission errors
    async fn suggest_permission_fix(&self, path: &Path) -> String {
        if !self.detailed_diagnostics {
            return "Check file permissions and try again.".to_string();
        }
        
        // Check if we can determine more specific permission issues
        if path.is_dir() {
            "This directory requires write permissions. Check ownership and permissions, or run with elevated privileges.".to_string()
        } else if path.exists() {
            "This file is write-protected. Check file permissions or ownership.".to_string()
        } else if let Some(parent) = path.parent() {
            if parent.exists() {
                "Cannot create file in this directory. Check directory write permissions.".to_string()
            } else {
                "Parent directory does not exist or is not accessible. Create the directory structure first.".to_string()
            }
        } else {
            "Path is not accessible. Check permissions and path validity.".to_string()
        }
    }
    
    /// Generate suggestion for not found errors
    async fn suggest_not_found_fix(&self, path: &Path, operation: &str) -> String {
        if !self.detailed_diagnostics {
            return "File or directory not found.".to_string();
        }
        
        if operation.contains("import") {
            "The source Maildir directory was not found. Verify the path and ensure the directory exists.".to_string()
        } else if operation.contains("export") {
            "Cannot create export directory. Check the parent directory exists and is writable.".to_string()
        } else {
            format!("The path '{}' was not found. Verify the path is correct and accessible.", path.display())
        }
    }
    
    /// Generate suggestion for already exists errors
    async fn suggest_already_exists_fix(&self, path: &Path, operation: &str) -> String {
        if operation.contains("export") {
            "Export destination already exists. Choose a different location or enable overwrite mode.".to_string()
        } else {
            format!("'{}' already exists. Use a different name or remove the existing item.", path.display())
        }
    }
    
    /// Check available disk space
    async fn check_disk_space(&self, path: &Path) -> (u64, u64) {
        // This would use platform-specific APIs to check disk space
        // For now, return placeholder values
        (0, 1024 * 1024 * 100) // 0 available, 100MB required
    }
    
    /// Check if path is on a network drive
    async fn is_network_path(&self, path: &Path) -> bool {
        // Simple heuristic - in reality would check if path is a network mount
        path.to_string_lossy().starts_with("//") || 
        path.to_string_lossy().contains(":\\\\")
    }
    
    /// Retry a fallible operation with exponential backoff
    pub async fn retry_operation<F, T, E>(&self, mut operation: F) -> Result<T, E>
    where
        F: FnMut() -> Result<T, E>,
        E: fmt::Debug,
    {
        let mut last_error = None;
        
        for attempt in 0..=self.max_retry_attempts {
            match operation() {
                Ok(result) => return Ok(result),
                Err(error) => {
                    last_error = Some(error);
                    
                    if attempt < self.max_retry_attempts {
                        let delay = self.retry_delay_ms * (2_u64.pow(attempt as u32));
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                    }
                }
            }
        }
        
        Err(last_error.unwrap())
    }
    
    /// Create a user-friendly error report
    pub fn create_error_report(&self, error: &MaildirOperationError) -> String {
        let mut report = String::new();
        
        report.push_str("❌ **Maildir Operation Failed**\n\n");
        report.push_str(&format!("**Error:** {}\n\n", error));
        
        if let Some(path) = error.path() {
            report.push_str(&format!("**Affected Path:** `{}`\n\n", path.display()));
        }
        
        report.push_str("**What you can do:**\n");
        report.push_str(&format!("• {}\n", error.suggestion()));
        
        if error.is_recoverable() {
            report.push_str("• This error is temporary - try again in a few moments\n");
        }
        
        if error.requires_user_intervention() {
            report.push_str("• This error requires your attention before proceeding\n");
        }
        
        match error {
            MaildirOperationError::InsufficientSpace { required_bytes, available_bytes, .. } => {
                report.push_str(&format!("• Space needed: {:.2} MB\n", *required_bytes as f64 / 1024.0 / 1024.0));
                report.push_str(&format!("• Space available: {:.2} MB\n", *available_bytes as f64 / 1024.0 / 1024.0));
            },
            MaildirOperationError::PathTooLong { max_length, .. } => {
                report.push_str(&format!("• Maximum path length: {} characters\n", max_length));
            },
            MaildirOperationError::Interrupted { progress, .. } => {
                report.push_str(&format!("• Progress before interruption: {:.1}%\n", progress * 100.0));
            },
            _ => {}
        }
        
        report.push_str("\n**Need help?** Check the documentation or contact support if this problem persists.");
        
        report
    }
}

/// Specialized error handler for specific Maildir operations
pub struct MaildirOperationContext {
    pub operation_type: String,
    pub source_path: Option<PathBuf>,
    pub destination_path: Option<PathBuf>,
    pub current_file: Option<String>,
    pub progress: f64,
    pub total_items: usize,
    pub processed_items: usize,
}

impl MaildirOperationContext {
    pub fn new(operation_type: String) -> Self {
        Self {
            operation_type,
            source_path: None,
            destination_path: None,
            current_file: None,
            progress: 0.0,
            total_items: 0,
            processed_items: 0,
        }
    }
    
    pub fn with_paths(mut self, source: Option<PathBuf>, destination: Option<PathBuf>) -> Self {
        self.source_path = source;
        self.destination_path = destination;
        self
    }
    
    pub fn update_progress(&mut self, processed: usize, total: usize, current_file: Option<String>) {
        self.processed_items = processed;
        self.total_items = total;
        self.current_file = current_file;
        self.progress = if total > 0 {
            processed as f64 / total as f64
        } else {
            0.0
        };
    }
    
    pub fn create_detailed_error(&self, base_error: MaildirOperationError) -> String {
        let mut report = String::new();
        
        report.push_str(&format!("**Operation:** {}\n", self.operation_type));
        
        if let Some(ref source) = self.source_path {
            report.push_str(&format!("**Source:** `{}`\n", source.display()));
        }
        
        if let Some(ref destination) = self.destination_path {
            report.push_str(&format!("**Destination:** `{}`\n", destination.display()));
        }
        
        if let Some(ref current) = self.current_file {
            report.push_str(&format!("**Current File:** `{}`\n", current));
        }
        
        report.push_str(&format!("**Progress:** {}/{} items ({:.1}%)\n\n", 
                                self.processed_items, self.total_items, self.progress * 100.0));
        
        report.push_str(&format!("{}\n", base_error));
        report.push_str(&format!("**Suggestion:** {}\n", base_error.suggestion()));
        
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Error, ErrorKind};
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_error_classification() {
        let handler = MaildirErrorHandler::default();
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("test.txt");
        
        // Test permission denied error
        let perm_error = Error::new(ErrorKind::PermissionDenied, "Permission denied");
        let classified = handler.classify_error(perm_error, &test_path, "writing file").await;
        
        match classified {
            MaildirOperationError::PermissionDenied { path, suggestion, .. } => {
                assert_eq!(path, test_path);
                assert!(!suggestion.is_empty());
            },
            _ => panic!("Expected PermissionDenied error"),
        }
    }
    
    #[tokio::test]
    async fn test_error_recoverability() {
        let file_locked = MaildirOperationError::FileLocked {
            message: "File is locked".to_string(),
            path: PathBuf::from("/test"),
            suggestion: "Try again".to_string(),
        };
        
        assert!(file_locked.is_recoverable());
        assert!(!file_locked.requires_user_intervention());
        
        let permission_denied = MaildirOperationError::PermissionDenied {
            message: "Permission denied".to_string(),
            path: PathBuf::from("/test"),
            suggestion: "Check permissions".to_string(),
        };
        
        assert!(!permission_denied.is_recoverable());
        assert!(permission_denied.requires_user_intervention());
    }
    
    #[tokio::test]
    async fn test_error_report_generation() {
        let handler = MaildirErrorHandler::default();
        let error = MaildirOperationError::InsufficientSpace {
            message: "Not enough space".to_string(),
            required_bytes: 1024 * 1024 * 100, // 100MB
            available_bytes: 1024 * 1024 * 50,  // 50MB
            suggestion: "Free up space".to_string(),
        };
        
        let report = handler.create_error_report(&error);
        
        assert!(report.contains("Maildir Operation Failed"));
        assert!(report.contains("Not enough space"));
        assert!(report.contains("Free up space"));
        assert!(report.contains("Space needed"));
        assert!(report.contains("Space available"));
    }
    
    #[tokio::test]
    async fn test_operation_context() {
        let mut context = MaildirOperationContext::new("Import".to_string())
            .with_paths(
                Some(PathBuf::from("/source")),
                Some(PathBuf::from("/destination"))
            );
        
        context.update_progress(50, 100, Some("test.msg".to_string()));
        
        let base_error = MaildirOperationError::FileLocked {
            message: "File locked".to_string(),
            path: PathBuf::from("/test"),
            suggestion: "Try again".to_string(),
        };
        
        let detailed_error = context.create_detailed_error(base_error);
        
        assert!(detailed_error.contains("Operation: Import"));
        assert!(detailed_error.contains("Source: `/source`"));
        assert!(detailed_error.contains("Destination: `/destination`"));
        assert!(detailed_error.contains("Current File: `test.msg`"));
        assert!(detailed_error.contains("Progress: 50/100"));
        assert!(detailed_error.contains("50.0%"));
    }
    
    #[tokio::test]
    async fn test_retry_operation() {
        let handler = MaildirErrorHandler::new(true, 2, 10); // 2 retries, 10ms delay
        let mut attempt_count = 0;
        
        let result = handler.retry_operation(|| {
            attempt_count += 1;
            if attempt_count < 3 {
                Err("Temporary failure")
            } else {
                Ok("Success")
            }
        }).await;
        
        assert_eq!(result, Ok("Success"));
        assert_eq!(attempt_count, 3); // Initial attempt + 2 retries
    }
    
    #[tokio::test]
    async fn test_retry_operation_exhausted() {
        let handler = MaildirErrorHandler::new(true, 1, 10); // 1 retry, 10ms delay
        let mut attempt_count = 0;
        
        let result: Result<&str, &str> = handler.retry_operation(|| {
            attempt_count += 1;
            Err("Persistent failure")
        }).await;
        
        assert_eq!(result, Err("Persistent failure"));
        assert_eq!(attempt_count, 2); // Initial attempt + 1 retry
    }
}