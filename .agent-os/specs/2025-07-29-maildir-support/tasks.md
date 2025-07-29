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

- [ ] 2. Build Maildir Import Functionality
  - [ ] 2.1 Write tests for MaildirImporter module with mock Maildir directories
  - [ ] 2.2 Implement async directory traversal and Maildir format validation
  - [ ] 2.3 Create email parsing and metadata extraction from Maildir files
  - [ ] 2.4 Integrate with existing SQLite database schema for imported emails
  - [ ] 2.5 Add progress tracking and error handling for import operations
  - [ ] 2.6 Verify all import tests pass including error scenarios

- [ ] 3. Create Maildir Export Functionality
  - [ ] 3.1 Write tests for MaildirExporter module with temporary directories
  - [ ] 3.2 Implement email serialization from SQLite to Maildir format
  - [ ] 3.3 Create proper Maildir directory structure (cur/, new/, tmp/) generation
  - [ ] 3.4 Add filename generation with Maildir flag encoding
  - [ ] 3.5 Implement batch processing with progress feedback
  - [ ] 3.6 Verify all export tests pass with format compliance validation

- [ ] 4. Build User Interface Components
  - [ ] 4.1 Write tests for Import Wizard TUI components
  - [ ] 4.2 Create directory browser with Maildir directory detection
  - [ ] 4.3 Implement folder selection interface with message count previews
  - [ ] 4.4 Add progress dialog with cancellation support for import operations
  - [ ] 4.5 Create export interface with folder selection and destination browsing
  - [ ] 4.6 Verify all UI components work correctly with keyboard navigation

- [ ] 5. Integration and Error Handling
  - [ ] 5.1 Write comprehensive integration tests for full import/export workflows
  - [ ] 5.2 Implement robust error handling for file system operations
  - [ ] 5.3 Add graceful handling of permission issues and disk space errors
  - [ ] 5.4 Create user-friendly error messages with actionable suggestions
  - [ ] 5.5 Add resume capability for interrupted operations
  - [ ] 5.6 Verify all error scenarios are handled gracefully with proper user feedback