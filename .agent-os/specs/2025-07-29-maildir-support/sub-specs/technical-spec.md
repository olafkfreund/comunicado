# Technical Specification

This is the technical specification for the spec detailed in @.agent-os/specs/2025-07-29-maildir-support/spec.md

> Created: 2025-07-29
> Version: 1.0.0

## Technical Requirements

- **Maildir Format Compliance:** Full compliance with the Maildir specification including cur/, new/, and tmp/ subdirectories
- **File System Operations:** Robust file handling with proper error recovery for large directory operations
- **Metadata Mapping:** Bidirectional mapping between Comunicado's SQLite metadata and Maildir filename flags
- **Progress Tracking:** Asynchronous processing with real-time progress updates for import/export operations
- **Memory Efficiency:** Stream-based processing to handle large Maildir directories without excessive memory usage
- **Folder Hierarchy Preservation:** Support for nested folder structures and proper IMAP folder name mapping

## Approach Options

**Option A: Direct File System Operations**
- Pros: Simple implementation, direct control over file operations, no external dependencies
- Cons: Manual handling of Maildir format complexities, potential for format compliance issues

**Option B: Use Existing Maildir Crate** (Selected)
- Pros: Proven Maildir format handling, community-maintained compliance, reduced development time
- Cons: Additional dependency, potential feature limitations

**Option C: Hybrid Approach with Custom Extensions**
- Pros: Best of both worlds, extensible for Comunicado-specific needs
- Cons: Increased complexity, potential maintenance overhead

**Rationale:** Option B provides the most reliable Maildir format compliance while allowing us to focus on integration with Comunicado's existing architecture. The maildir crate in the Rust ecosystem provides robust, tested Maildir handling that reduces the risk of format compliance issues.

## Implementation Architecture

### Core Components

**MaildirImporter Module**
- Async directory traversal and email parsing
- Metadata extraction and mapping to Comunicado's data model
- Progress tracking and error handling
- Integration with existing account management system

**MaildirExporter Module**  
- Email serialization to Maildir format
- Folder structure creation and management
- Filename generation with proper flag encoding
- Batch processing with progress feedback

**MaildirMapper Module**
- Bidirectional conversion between Comunicado metadata and Maildir flags
- Folder hierarchy mapping between IMAP-style paths and filesystem structure
- Timestamp preservation and timezone handling

### User Interface Integration

**Import Wizard Component**
- Directory browser with Maildir detection
- Folder selection with preview capabilities
- Progress dialog with cancellation support
- Error reporting and recovery options

**Export Interface Component**
- Folder selection from existing accounts
- Destination directory selection
- Export options (include attachments, preserve structure, etc.)
- Progress monitoring with ETA estimates

## External Dependencies

- **maildir crate** - Rust crate for Maildir format handling and manipulation
  - **Justification:** Provides tested, compliant Maildir operations and reduces implementation complexity
  - **Version:** Latest stable (likely 0.6+)

- **walkdir crate** - Recursive directory traversal for efficient folder scanning
  - **Justification:** Efficient directory walking with proper error handling for large directory structures
  - **Version:** Latest stable (likely 2.3+)

- **indicatif crate** - Progress bar implementation for TUI progress displays
  - **Justification:** Provides rich progress indicators that integrate well with ratatui-based interface
  - **Version:** Latest compatible with ratatui integration

## Data Flow Architecture

### Import Process Flow
1. User selects Maildir directory through file browser
2. System scans directory structure and validates Maildir format
3. User selects folders to import with preview of message counts
4. Async import process begins with progress tracking
5. Each email is parsed, metadata extracted, and inserted into SQLite database
6. Folder hierarchy is mapped to Comunicado's internal folder structure
7. Completion summary shows imported message counts and any errors

### Export Process Flow
1. User selects folders/accounts to export from main interface
2. User chooses destination directory and export options
3. System creates Maildir directory structure at destination
4. Emails are retrieved from SQLite, serialized to Maildir format
5. Files are written to appropriate cur/, new/, tmp/ directories
6. Metadata is encoded in filenames according to Maildir specification
7. Completion summary shows exported message counts and directory location

## Error Handling Strategy

- **File System Errors:** Graceful handling of permission issues, disk space, and I/O errors
- **Format Validation:** Strict validation of Maildir structure with user-friendly error messages
- **Partial Failures:** Ability to continue processing after individual message failures
- **Recovery Options:** Resume capability for interrupted import/export operations
- **User Feedback:** Clear error reporting with actionable suggestions for resolution