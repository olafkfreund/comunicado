# Spec Tasks

These are the tasks to be completed for the spec detailed in @.agent-os/specs/2025-07-29-maildir-support/spec.md

> Created: 2025-07-29
> Status: Ready for Implementation

## Tasks

- [x] 1. Implement Core Maildir Support Infrastructure
  - [x] 1.1 Write tests for Maildir format parsing and validation
  - [x] 1.2 Add maildir, walkdir, and indicatif crates to Cargo.toml dependencies
  - [x] 1.3 Create MaildirMapper module for metadata conversion between Comunicado and Maildir formats
  - [x] 1.4 Implement folder hierarchy mapping between IMAP-style paths and filesystem structure
  - [x] 1.5 Add timestamp preservation and timezone handling utilities
  - [x] 1.6 Verify all tests pass for core infrastructure

- [x] 2. Build Maildir Import Functionality
  - [x] 2.1 Write tests for MaildirImporter module with mock Maildir directories
  - [x] 2.2 Implement async directory traversal and Maildir format validation
  - [x] 2.3 Create email parsing and metadata extraction from Maildir files
  - [x] 2.4 Integrate with existing SQLite database schema for imported emails
  - [x] 2.5 Add progress tracking and error handling for import operations
  - [x] 2.6 Verify all import tests pass including error scenarios

- [x] 3. Create Maildir Export Functionality
  - [x] 3.1 Write tests for MaildirExporter module with temporary directories
  - [x] 3.2 Implement email serialization from SQLite to Maildir format
  - [x] 3.3 Create proper Maildir directory structure (cur/, new/, tmp/) generation
  - [x] 3.4 Add filename generation with Maildir flag encoding
  - [x] 3.5 Implement batch processing with progress feedback
  - [x] 3.6 Verify all export tests pass with format compliance validation

- [x] 4. Build User Interface Components
  - [x] 4.1 Write tests for Import Wizard TUI components
  - [x] 4.2 Create directory browser with Maildir directory detection
  - [x] 4.3 Implement folder selection interface with message count previews
  - [x] 4.4 Add progress dialog with cancellation support for import operations
  - [x] 4.5 Create export interface with folder selection and destination browsing
  - [x] 4.6 Verify all UI components work correctly with keyboard navigation

- [x] 5. Integration and Error Handling
  - [x] 5.1 Write comprehensive integration tests for full import/export workflows
  - [x] 5.2 Implement robust error handling for file system operations
  - [x] 5.3 Add graceful handling of permission issues and disk space errors
  - [x] 5.4 Create user-friendly error messages with actionable suggestions
  - [x] 5.5 Add resume capability for interrupted operations
  - [x] 5.6 Verify all error scenarios are handled gracefully with proper user feedback