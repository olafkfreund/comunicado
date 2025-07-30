# Database Schema

This is the database schema implementation for the spec detailed in @.agent-os/specs/2025-07-29-maildir-support/spec.md

> Created: 2025-07-29
> Version: 1.0.0

## Schema Changes

### New Tables

**maildir_import_history**
```sql
CREATE TABLE maildir_import_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id INTEGER NOT NULL,
    source_path TEXT NOT NULL,
    import_timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    total_messages INTEGER NOT NULL,
    successful_imports INTEGER NOT NULL,
    failed_imports INTEGER NOT NULL,
    folder_count INTEGER NOT NULL,
    import_options TEXT, -- JSON blob for import configuration
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE
);
```

**maildir_export_history**
```sql
CREATE TABLE maildir_export_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id INTEGER NOT NULL,
    destination_path TEXT NOT NULL,
    export_timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    total_messages INTEGER NOT NULL,
    successful_exports INTEGER NOT NULL,
    failed_exports INTEGER NOT NULL,
    folder_count INTEGER NOT NULL,
    export_options TEXT, -- JSON blob for export configuration
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE
);
```

### New Columns

**emails table modifications**
```sql
-- Add column to track original Maildir filename for round-trip fidelity
ALTER TABLE emails ADD COLUMN original_maildir_filename TEXT;

-- Add column to track Maildir-specific flags
ALTER TABLE emails ADD COLUMN maildir_flags TEXT;

-- Add index for efficient Maildir operations
CREATE INDEX idx_emails_maildir_filename ON emails(original_maildir_filename);
CREATE INDEX idx_emails_maildir_flags ON emails(maildir_flags);
```

**folders table modifications**
```sql
-- Add column to track original Maildir folder path
ALTER TABLE folders ADD COLUMN maildir_path TEXT;

-- Add index for Maildir path lookups
CREATE INDEX idx_folders_maildir_path ON folders(maildir_path);
```

## Migration Scripts

### Migration 001: Add Maildir Support Tables
```sql
BEGIN TRANSACTION;

-- Create import history table
CREATE TABLE maildir_import_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id INTEGER NOT NULL,
    source_path TEXT NOT NULL,
    import_timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    total_messages INTEGER NOT NULL,
    successful_imports INTEGER NOT NULL,
    failed_imports INTEGER NOT NULL,
    folder_count INTEGER NOT NULL,
    import_options TEXT,
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE
);

-- Create export history table  
CREATE TABLE maildir_export_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id INTEGER NOT NULL,
    destination_path TEXT NOT NULL,
    export_timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    total_messages INTEGER NOT NULL,
    successful_exports INTEGER NOT NULL,
    failed_exports INTEGER NOT NULL,
    folder_count INTEGER NOT NULL,
    export_options TEXT,
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE
);

-- Add Maildir-specific columns to existing tables
ALTER TABLE emails ADD COLUMN original_maildir_filename TEXT;
ALTER TABLE emails ADD COLUMN maildir_flags TEXT;
ALTER TABLE folders ADD COLUMN maildir_path TEXT;

-- Create indexes for performance
CREATE INDEX idx_emails_maildir_filename ON emails(original_maildir_filename);
CREATE INDEX idx_emails_maildir_flags ON emails(maildir_flags);
CREATE INDEX idx_folders_maildir_path ON folders(maildir_path);

-- Update schema version
INSERT OR REPLACE INTO schema_version (version, description) 
VALUES (12, 'Add Maildir import/export support');

COMMIT;
```

## Data Mapping Specifications

### Maildir Flag Mapping

**Standard Maildir Flags to Comunicado**
- `P` (Passed) → `forwarded` flag in Comunicado
- `R` (Replied) → `answered` flag in Comunicado  
- `S` (Seen) → `read` flag in Comunicado
- `T` (Trashed) → `deleted` flag in Comunicado
- `D` (Draft) → `draft` flag in Comunicado
- `F` (Flagged) → `flagged` flag in Comunicado

**Extended Flags Handling**
- Custom flags stored in `maildir_flags` column as comma-separated string
- Bidirectional conversion ensures round-trip fidelity
- Unknown flags preserved during import for export compatibility

### Folder Path Mapping

**IMAP to Maildir Conversion**
- IMAP hierarchy separator (`.` or `/`) → Maildir subdirectory structure
- Special IMAP folders (INBOX, Sent, Drafts) → Standard Maildir names
- Unicode folder names → filesystem-safe encoding with UTF-8 support

**Example Mappings:**
```
IMAP: "INBOX.Work.Projects"     → Maildir: ".Work.Projects/"
IMAP: "Sent Messages"           → Maildir: ".Sent/"
IMAP: "Drafts"                  → Maildir: ".Drafts/"
IMAP: "Archive/2024"            → Maildir: ".Archive.2024/"
```

## Query Performance Considerations

### Indexes for Import Operations
- `idx_emails_message_id` for duplicate detection during import
- `idx_folders_account_path` for efficient folder lookups
- `idx_maildir_import_history_account` for history queries

### Indexes for Export Operations  
- `idx_emails_folder_id_date` for chronological export ordering
- `idx_emails_maildir_flags` for flag-based filtering
- `idx_folders_maildir_path` for reverse path mapping

### Bulk Operation Optimization
- Use prepared statements for batch email insertions
- Transaction batching (1000 emails per transaction) for optimal performance
- Progress tracking through periodic count queries with LIMIT/OFFSET

## Data Integrity Rules

### Constraints
- `original_maildir_filename` must be unique within folder for imported emails
- `maildir_path` must be unique within account for folders
- Import/export history records are immutable (no updates, only inserts)

### Validation Rules
- Maildir filenames must follow `timestamp.pid.hostname:2,flags` format
- Folder paths must not contain invalid filesystem characters
- Flag strings must contain only valid Maildir flag characters

### Cleanup Procedures
- Orphaned import/export history cleanup after account deletion
- Temporary data cleanup after failed import/export operations
- Index maintenance for optimal query performance