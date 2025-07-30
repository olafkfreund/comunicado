# API Specification

This is the API specification for the spec detailed in @.agent-os/specs/2025-07-29-maildir-support/spec.md

> Created: 2025-07-29
> Version: 1.0.0

## Internal API Endpoints

Since Comunicado is a TUI application, these are internal Rust module APIs rather than HTTP endpoints.

### MaildirImporter API

#### `async fn import_maildir()`

**Purpose:** Import emails from a Maildir directory into Comunicado
**Parameters:**
- `source_path: PathBuf` - Path to the Maildir directory
- `account_id: i64` - Target account ID for import
- `folder_selection: Vec<String>` - List of folder names to import (empty = all)
- `options: ImportOptions` - Import configuration options
- `progress_callback: Box<dyn Fn(ImportProgress)>` - Progress update callback

**Response:** `Result<ImportSummary, MaildirError>`
```rust
pub struct ImportSummary {
    pub total_messages: usize,
    pub successful_imports: usize,
    pub failed_imports: usize,
    pub folder_count: usize,
    pub errors: Vec<ImportError>,
    pub duration: Duration,
}
```

**Errors:**
- `MaildirError::InvalidDirectory` - Source path is not a valid Maildir
- `MaildirError::PermissionDenied` - Insufficient permissions to read source
- `MaildirError::DatabaseError` - Database operation failed
- `MaildirError::DuplicateAccount` - Account already has imported data

#### `async fn validate_maildir_directory()`

**Purpose:** Validate that a directory is a proper Maildir format
**Parameters:**
- `path: PathBuf` - Directory path to validate

**Response:** `Result<MaildirInfo, MaildirError>`
```rust
pub struct MaildirInfo {
    pub is_valid: bool,
    pub folder_count: usize,
    pub estimated_message_count: usize,
    pub folders: Vec<FolderInfo>,
    pub warnings: Vec<String>,
}
```

#### `async fn scan_maildir_folders()`

**Purpose:** Scan Maildir directory and return folder structure
**Parameters:**
- `path: PathBuf` - Maildir root directory
- `recursive: bool` - Whether to scan nested folders

**Response:** `Result<Vec<MaildirFolder>, MaildirError>`
```rust
pub struct MaildirFolder {
    pub name: String,
    pub path: PathBuf,
    pub message_count: usize,
    pub size_bytes: u64,
    pub last_modified: SystemTime,
}
```

### MaildirExporter API

#### `async fn export_to_maildir()`

**Purpose:** Export Comunicado emails to Maildir format
**Parameters:**
- `destination_path: PathBuf` - Target directory for Maildir export
- `account_id: i64` - Source account ID for export
- `folder_selection: Vec<i64>` - List of folder IDs to export (empty = all)
- `options: ExportOptions` - Export configuration options
- `progress_callback: Box<dyn Fn(ExportProgress)>` - Progress update callback

**Response:** `Result<ExportSummary, MaildirError>`
```rust
pub struct ExportSummary {
    pub total_messages: usize,
    pub successful_exports: usize,
    pub failed_exports: usize,
    pub folder_count: usize,
    pub output_path: PathBuf,
    pub errors: Vec<ExportError>,
    pub duration: Duration,
}
```

**Errors:**
- `MaildirError::DestinationExists` - Target directory already exists and not empty
- `MaildirError::InsufficientSpace` - Not enough disk space for export
- `MaildirError::CreateDirectoryFailed` - Cannot create Maildir structure
- `MaildirError::WritePermissionDenied` - Cannot write to destination

#### `async fn estimate_export_size()`

**Purpose:** Calculate estimated disk space needed for export
**Parameters:**
- `account_id: i64` - Account to analyze
- `folder_selection: Vec<i64>` - Folders to include in estimate

**Response:** `Result<ExportEstimate, MaildirError>`
```rust
pub struct ExportEstimate {
    pub total_messages: usize,
    pub estimated_size_bytes: u64,
    pub folder_breakdown: Vec<FolderSizeInfo>,
}
```

### MaildirManager API

#### `async fn get_import_history()`

**Purpose:** Retrieve import history for an account
**Parameters:**
- `account_id: i64` - Account ID to query

**Response:** `Result<Vec<ImportHistoryRecord>, DatabaseError>`

#### `async fn get_export_history()`

**Purpose:** Retrieve export history for an account  
**Parameters:**
- `account_id: i64` - Account ID to query

**Response:** `Result<Vec<ExportHistoryRecord>, DatabaseError>`

#### `async fn cleanup_failed_operation()`

**Purpose:** Clean up partial data from failed import/export
**Parameters:**
- `operation_id: i64` - ID of failed operation
- `operation_type: OperationType` - Import or Export

**Response:** `Result<(), DatabaseError>`

## Data Structures

### Configuration Types

```rust
pub struct ImportOptions {
    pub skip_duplicates: bool,
    pub preserve_flags: bool,
    pub create_missing_folders: bool,
    pub batch_size: usize,
    pub validate_messages: bool,
}

pub struct ExportOptions {
    pub include_attachments: bool,
    pub preserve_folder_structure: bool,
    pub compress_output: bool,
    pub overwrite_existing: bool,
    pub maildir_format_version: MaildirVersion,
}
```

### Progress Tracking Types

```rust
pub struct ImportProgress {
    pub current_folder: String,
    pub processed_messages: usize,
    pub total_messages: usize,
    pub current_message_subject: String,
    pub elapsed_time: Duration,
    pub estimated_remaining: Option<Duration>,
}

pub struct ExportProgress {
    pub current_folder: String,
    pub exported_messages: usize,
    pub total_messages: usize,
    pub current_message_subject: String,
    pub bytes_written: u64,
    pub elapsed_time: Duration,
}
```

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum MaildirError {
    #[error("Invalid Maildir directory: {0}")]
    InvalidDirectory(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Database operation failed: {0}")]
    DatabaseError(#[from] sqlx::Error),
    
    #[error("I/O operation failed: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Maildir format error: {0}")]
    FormatError(String),
    
    #[error("Operation cancelled by user")]
    Cancelled,
}
```

## Integration Points

### TUI Interface Integration

**Import Wizard Controller**
- `handle_directory_selection()` - File browser for Maildir selection
- `handle_folder_preview()` - Display folder structure before import
- `handle_import_progress()` - Progress dialog with cancel option
- `handle_import_completion()` - Summary dialog with error details

**Export Interface Controller**
- `handle_folder_selection()` - Multi-select from existing folders
- `handle_destination_selection()` - Directory picker for export target
- `handle_export_options()` - Configuration dialog for export settings
- `handle_export_progress()` - Progress monitoring with ETA

### Database Integration

**Transaction Management**
- Batch imports in chunks of 1000 messages for optimal performance
- Rollback capability for failed imports to maintain data integrity
- Progress checkpointing for resumable operations

**Metadata Synchronization**
- Bidirectional mapping between Comunicado flags and Maildir flags
- Folder hierarchy preservation with path translation
- Timestamp accuracy preservation across import/export cycles

### Error Handling Integration

**User-Friendly Error Messages**
- Contextual error information with suggested solutions
- Partial success reporting (completed items vs. failed items)
- Recovery options for common error scenarios
- Detailed logging for troubleshooting purposes