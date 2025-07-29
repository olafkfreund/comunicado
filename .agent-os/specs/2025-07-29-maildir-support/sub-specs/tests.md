# Tests Specification

This is the tests coverage details for the spec detailed in @.agent-os/specs/2025-07-29-maildir-support/spec.md

> Created: 2025-07-29
> Version: 1.0.0

## Test Coverage

### Unit Tests

**MaildirImporter Module**
- `test_validate_maildir_directory_valid()` - Validates proper Maildir structure recognition
- `test_validate_maildir_directory_invalid()` - Rejects non-Maildir directories
- `test_scan_maildir_folders_nested()` - Handles nested folder structures correctly
- `test_parse_maildir_filename()` - Extracts metadata from Maildir filenames
- `test_convert_maildir_flags_to_comunicado()` - Maps flags bidirectionally
- `test_handle_duplicate_message_ids()` - Prevents duplicate imports
- `test_import_with_corrupted_messages()` - Handles malformed email files gracefully
- `test_folder_hierarchy_mapping()` - Preserves IMAP-style folder structure

**MaildirExporter Module**
- `test_create_maildir_structure()` - Creates proper cur/new/tmp directories
- `test_generate_maildir_filename()` - Produces spec-compliant filenames
- `test_convert_comunicado_flags_to_maildir()` - Maps internal flags to Maildir format
- `test_export_with_folder_hierarchy()` - Maintains nested folder structure
- `test_handle_filesystem_errors()` - Graceful handling of I/O failures
- `test_export_large_message_batches()` - Performance with high message counts
- `test_preserve_message_timestamps()` - Maintains original email timestamps
- `test_handle_special_characters_in_filenames()` - Filesystem-safe filename generation

**MaildirMapper Module**
- `test_bidirectional_flag_conversion()` - Round-trip flag preservation
- `test_folder_path_conversion()` - IMAP path to Maildir path mapping
- `test_handle_unicode_folder_names()` - UTF-8 folder name support
- `test_preserve_folder_hierarchy()` - Nested folder structure integrity
- `test_handle_invalid_characters()` - Filesystem character sanitization
- `test_timestamp_timezone_handling()` - Proper timezone preservation

**Database Schema Module**
- `test_maildir_import_history_insertion()` - Import history record creation
- `test_maildir_export_history_insertion()` - Export history record creation
- `test_maildir_filename_uniqueness()` - Prevents duplicate filename conflicts
- `test_foreign_key_constraints()` - Proper cascade deletion behavior
- `test_index_performance()` - Query optimization validation
- `test_migration_script_execution()` - Database schema upgrade testing

### Integration Tests

**End-to-End Import Workflow**
- `test_complete_maildir_import()` - Full import process from directory selection to completion
- `test_import_with_folder_selection()` - Selective folder import functionality
- `test_import_progress_tracking()` - Progress callback integration and accuracy
- `test_import_error_recovery()` - Handling of partial failures and recovery
- `test_import_cancellation()` - User-initiated import cancellation
- `test_large_maildir_import()` - Performance testing with 10,000+ messages

**End-to-End Export Workflow**
- `test_complete_maildir_export()` - Full export process from folder selection to completion
- `test_export_with_folder_filtering()` - Selective folder export functionality
- `test_export_progress_tracking()` - Progress monitoring and ETA calculation
- `test_export_disk_space_validation()` - Insufficient space handling
- `test_export_cancellation()` - User-initiated export cancellation
- `test_large_maildir_export()` - Performance testing with 10,000+ messages

**Cross-Client Compatibility**
- `test_mutt_import_compatibility()` - Import Maildir created by mutt
- `test_dovecot_import_compatibility()` - Import Maildir from dovecot server
- `test_thunderbird_import_compatibility()` - Import Maildir from Thunderbird export
- `test_export_mutt_compatibility()` - Verify exported Maildir works with mutt
- `test_export_dovecot_compatibility()` - Verify exported Maildir works with dovecot
- `test_round_trip_fidelity()` - Import → Export → Import maintains integrity

**TUI Interface Integration**
- `test_import_wizard_navigation()` - Directory browser and folder selection UI
- `test_export_interface_navigation()` - Folder selection and destination picker UI
- `test_progress_dialog_updates()` - Real-time progress display accuracy
- `test_error_dialog_handling()` - Error message display and user interaction
- `test_completion_summary_display()` - Success/failure summary presentation

### Feature Tests

**Import Edge Cases**
- `test_import_empty_maildir()` - Handles Maildir with no messages
- `test_import_maildir_with_symlinks()` - Follows symbolic links appropriately
- `test_import_maildir_with_permissions_issues()` - Handles read permission errors
- `test_import_maildir_with_special_folders()` - Processes IMAP special folders correctly
- `test_import_maildir_with_custom_flags()` - Preserves non-standard Maildir flags
- `test_import_interrupted_maildir()` - Handles partially delivered messages

**Export Edge Cases**
- `test_export_to_existing_directory()` - Handles destination conflicts appropriately
- `test_export_with_insufficient_permissions()` - Handles write permission errors
- `test_export_with_long_folder_names()` - Handles filesystem path length limits
- `test_export_with_special_characters()` - Sanitizes problematic characters
- `test_export_with_large_attachments()` - Handles messages with large attachments
- `test_export_empty_folders()` - Creates proper structure for empty folders

**Data Integrity Tests**
- `test_message_content_preservation()` - Email body and headers unchanged
- `test_attachment_integrity()` - Binary attachments preserved correctly
- `test_timestamp_accuracy()` - Message timestamps maintained precisely
- `test_folder_hierarchy_fidelity()` - Nested folder structure preserved
- `test_metadata_round_trip()` - All metadata survives import/export cycle
- `test_unicode_content_handling()` - Proper UTF-8 content preservation

## Mocking Requirements

### File System Operations
- **Mock Maildir Directory Structure:** Create temporary test Maildirs with known content
- **Mock File System Errors:** Simulate permission denied, disk full, and I/O errors
- **Mock Large Directory Traversal:** Simulate performance characteristics of large Maildirs

### Database Operations
- **Mock SQLite Transactions:** Test transaction rollback scenarios
- **Mock Database Errors:** Simulate constraint violations and connection failures
- **Mock Migration Failures:** Test database schema upgrade error handling

### Progress Tracking
- **Mock Progress Callbacks:** Verify progress reporting accuracy and frequency
- **Mock Time-based Operations:** Control timing for ETA calculation testing
- **Mock Cancellation Signals:** Test user-initiated operation cancellation

### External Dependencies
- **Mock Maildir Crate Operations:** Test error conditions from maildir crate
- **Mock File System Traversal:** Control walkdir behavior for edge case testing
- **Mock Progress Bar Updates:** Test indicatif integration without visual output

## Performance Testing

### Import Performance Benchmarks
- **Small Maildir (100 messages):** Target < 5 seconds import time
- **Medium Maildir (1,000 messages):** Target < 30 seconds import time  
- **Large Maildir (10,000 messages):** Target < 5 minutes import time
- **Memory Usage:** Target < 100MB peak memory during import
- **Database Performance:** Target < 100ms per message insertion

### Export Performance Benchmarks
- **Small Export (100 messages):** Target < 3 seconds export time
- **Medium Export (1,000 messages):** Target < 20 seconds export time
- **Large Export (10,000 messages):** Target < 3 minutes export time
- **Disk I/O Efficiency:** Minimize temporary file creation and unnecessary reads
- **Progress Accuracy:** Progress updates within 5% of actual completion

### Memory Efficiency Tests
- **Stream Processing:** Verify constant memory usage regardless of Maildir size
- **Batch Processing:** Optimal batch sizes for memory vs. performance trade-offs
- **Garbage Collection:** No memory leaks during long-running operations
- **Resource Cleanup:** Proper file handle and database connection cleanup

## Test Data Requirements

### Sample Maildir Structures
- **Basic Maildir:** Simple cur/new/tmp structure with varied message types
- **Nested Maildir:** Complex folder hierarchy with 3+ levels of nesting
- **Mixed Flag Maildir:** Messages with all standard and some custom Maildir flags
- **Unicode Maildir:** Folder names and message content with international characters
- **Corrupted Maildir:** Some malformed messages and missing directories

### Message Content Varieties
- **Plain Text Messages:** Simple text-only emails
- **HTML Messages:** Rich HTML emails with embedded images
- **Multi-part Messages:** Messages with both text and HTML parts
- **Messages with Attachments:** Various attachment types and sizes
- **Messages with Custom Headers:** Non-standard email headers preserved