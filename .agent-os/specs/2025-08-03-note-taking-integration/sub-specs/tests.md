# Tests Specification

This is the tests coverage details for the spec detailed in @.agent-os/specs/2025-08-03-note-taking-integration/spec.md

> Created: 2025-08-03
> Version: 1.0.0

## Test Coverage Strategy

The notes plugin requires comprehensive testing across multiple layers: unit tests for core functionality, integration tests for file system and database operations, and feature tests for end-to-end workflows.

## Unit Tests

### Note Management (`src/plugins/notes/manager.rs`)

**NoteManager Core Operations**
- `test_create_note_with_frontmatter` - Create note with YAML frontmatter, verify file creation and metadata extraction
- `test_create_note_without_frontmatter` - Create note with title-only, verify auto-generated frontmatter
- `test_get_note_by_id` - Retrieve existing note, verify all fields populated correctly
- `test_get_note_nonexistent` - Attempt to get missing note, verify proper error handling
- `test_update_note_content` - Update note content, verify file write and index update
- `test_update_note_readonly` - Attempt to update read-only note, verify error handling
- `test_delete_note_with_links` - Delete note that has incoming links, verify link cleanup
- `test_delete_note_nonexistent` - Attempt to delete missing note, verify error handling

**Directory Management**
- `test_add_directory_recursive` - Add directory with recursive scanning, verify all notes found
- `test_add_directory_nonrecursive` - Add directory without recursion, verify only root notes found
- `test_add_directory_duplicate` - Add same directory twice, verify error handling
- `test_remove_directory_with_notes` - Remove directory, verify notes marked as deleted
- `test_refresh_directory_with_changes` - Manual refresh after external changes, verify updates

### Markdown Parser (`src/plugins/notes/parser.rs`)

**Frontmatter Parsing**
- `test_parse_yaml_frontmatter` - Parse valid YAML frontmatter with various data types
- `test_parse_empty_frontmatter` - Handle notes without frontmatter
- `test_parse_invalid_yaml` - Handle malformed YAML gracefully
- `test_parse_custom_fields` - Parse custom metadata fields in frontmatter

**Wiki Link Parsing**
- `test_parse_simple_wiki_links` - Parse `[[Note Name]]` style links
- `test_parse_wiki_links_with_display` - Parse `[[Note Name|Display Text]]` style links
- `test_parse_nested_brackets` - Handle nested brackets in link text
- `test_parse_invalid_links` - Handle malformed link syntax gracefully
- `test_extract_all_links_from_content` - Find all links in markdown content

**Content Processing**
- `test_extract_title_from_h1` - Extract title from first H1 heading
- `test_extract_title_from_filename` - Use filename as title when no H1 present
- `test_extract_tags_from_frontmatter` - Parse tags from YAML frontmatter
- `test_extract_tags_from_content` - Find hashtags in content
- `test_calculate_word_count` - Count words excluding frontmatter and code blocks

### Search Indexer (`src/plugins/notes/indexer.rs`)

**FTS Indexing**
- `test_index_new_note` - Index new note content and metadata
- `test_reindex_updated_note` - Update index when note content changes
- `test_remove_deleted_note_from_index` - Clean up index when note deleted
- `test_index_large_note` - Handle notes exceeding size limits

**Search Operations**
- `test_search_by_title` - Search by note title with ranking
- `test_search_by_content` - Full-text content search with snippets
- `test_search_by_tags` - Filter search results by tags
- `test_search_with_filters` - Combined search with multiple filters
- `test_search_ranking` - Verify search result ranking algorithm
- `test_search_empty_query` - Handle empty search gracefully

### Link Resolver (`src/plugins/notes/linker.rs`)

**Link Resolution**
- `test_resolve_valid_wiki_link` - Resolve link to existing note
- `test_resolve_invalid_wiki_link` - Handle links to nonexistent notes
- `test_resolve_ambiguous_link` - Handle multiple notes with same title
- `test_update_links_on_note_rename` - Update all links when target note renamed

**Backlink Discovery**
- `test_find_all_backlinks` - Find all notes linking to target note
- `test_update_backlinks_on_content_change` - Update backlinks when note content changes
- `test_broken_link_detection` - Identify and track broken links
- `test_bidirectional_link_graph` - Generate complete link graph for navigation

## Integration Tests

### File System Watcher (`src/plugins/notes/watcher.rs`)

**File Change Detection**
- `test_detect_new_file_creation` - Detect when new markdown file created
- `test_detect_file_modification` - Detect when existing file modified
- `test_detect_file_deletion` - Detect when file deleted or moved
- `test_detect_directory_changes` - Detect when directories added/removed
- `test_ignore_non_markdown_files` - Verify only markdown files trigger events
- `test_debounce_rapid_changes` - Handle rapid file changes with debouncing

**Cross-Platform Compatibility**
- `test_watcher_on_linux` - Verify inotify-based watching works
- `test_watcher_on_macos` - Verify FSEvents-based watching works  
- `test_watcher_on_windows` - Verify ReadDirectoryChanges watching works
- `test_symlink_handling` - Handle symbolic links appropriately
- `test_permission_changes` - Handle permission changes gracefully

### Database Operations (`src/plugins/notes/storage.rs`)

**CRUD Operations**
- `test_store_and_retrieve_note` - Full roundtrip note storage
- `test_update_note_metadata` - Update note without content changes
- `test_concurrent_note_access` - Handle concurrent database access
- `test_transaction_rollback` - Verify database consistency on errors

**Search Integration**
- `test_fts_index_synchronization` - Verify FTS index stays synchronized
- `test_search_after_bulk_changes` - Search performance after bulk operations
- `test_search_result_ranking` - Verify search ranking algorithm
- `test_search_with_special_characters` - Handle special characters in search

**Link Management**
- `test_store_and_retrieve_links` - Store wiki links with metadata
- `test_update_link_validity` - Update link status when targets change
- `test_cascade_delete_links` - Verify links deleted when notes deleted
- `test_link_statistics` - Generate accurate link statistics

### Plugin Integration (`src/plugins/notes/mod.rs`)

**Plugin Lifecycle**
- `test_plugin_registration` - Verify plugin registers correctly with system
- `test_service_availability` - Confirm services available after registration
- `test_plugin_shutdown` - Clean shutdown without resource leaks
- `test_plugin_configuration` - Load and validate plugin configuration

**Event System Integration**
- `test_note_events_published` - Verify events published on note operations
- `test_event_handler_registration` - Register and receive plugin events
- `test_cross_plugin_communication` - Communicate with other plugins
- `test_error_isolation` - Verify errors don't crash main application

## Feature Tests

### End-to-End Note Management

**Complete Note Workflow**
- `test_full_note_lifecycle` - Create, read, update, delete note with UI
- `test_note_creation_from_email` - Create note from email content
- `test_note_creation_from_calendar` - Create meeting notes from calendar event
- `test_note_search_and_navigation` - Search and navigate between notes
- `test_cross_reference_workflow` - Link notes to emails and calendar events

**Multi-Directory Support**
- `test_multiple_vault_management` - Manage notes across multiple directories
- `test_vault_switching` - Switch between different note vaults
- `test_directory_sync_conflicts` - Handle conflicts when directories change
- `test_nested_directory_support` - Support deeply nested directory structures

### UI Integration Tests

**Terminal Interface**
- `test_notes_ui_keyboard_navigation` - Navigate using vim-style keys
- `test_search_interface_interaction` - Interactive search with real-time results
- `test_note_editing_interface` - Create and edit notes through TUI
- `test_link_navigation_ui` - Follow wiki links through interface
- `test_multi_pane_layout` - Verify multi-pane interface layout and resizing

**Email Integration UI**
- `test_email_to_note_workflow` - Create note from email through UI
- `test_note_references_in_email` - Display note references in email view
- `test_email_note_linking_ui` - Link existing notes to emails
- `test_email_composition_with_notes` - Reference notes during email composition

**Calendar Integration UI**
- `test_calendar_meeting_notes` - Create and link meeting notes
- `test_event_note_references` - Display linked notes in calendar events
- `test_agenda_note_integration` - Generate agenda from linked notes
- `test_recurring_meeting_notes` - Handle notes for recurring calendar events

## Performance Tests

### Scalability Testing

**Large Dataset Performance**
- `test_performance_1000_notes` - Performance with 1,000 notes
- `test_performance_10000_notes` - Performance with 10,000 notes
- `test_search_performance_large_dataset` - Search speed with large datasets
- `test_indexing_performance` - Time to index large number of notes
- `test_memory_usage_large_dataset` - Memory consumption with large datasets

**Concurrent Operations**
- `test_concurrent_note_creation` - Multiple notes created simultaneously
- `test_concurrent_search_operations` - Multiple search operations in parallel
- `test_file_watcher_under_load` - File watching performance under high load
- `test_database_concurrency` - Database performance with concurrent access

### Resource Usage Tests

**Memory Management**
- `test_memory_leak_detection` - Verify no memory leaks during operations
- `test_cache_memory_limits` - Respect configured memory limits
- `test_large_file_handling` - Handle large markdown files efficiently
- `test_resource_cleanup` - Proper cleanup of resources on shutdown

## Mocking Requirements

### External Service Mocks

**File System Operations**
- **MockFileWatcher**: Simulate file system events without real files
- **MockFileSystem**: Control file operations and permissions for testing
- **TemporaryDirectory**: Create isolated test environments with cleanup

**Database Operations**
- **InMemorySQLite**: Use in-memory database for fast test execution
- **MockDatabase**: Simulate database errors and edge conditions
- **TransactionMock**: Test transaction rollback scenarios

**Plugin System Mocks**
- **MockPluginRegistry**: Test plugin registration without full system
- **MockServiceRegistry**: Verify service registration and lookup
- **MockEventBus**: Test event publishing and handling

### Integration Mocks

**Email System Integration**
- **MockEmailService**: Simulate email operations for integration tests
- **FakeEmailMessage**: Generate test email data for note creation
- **MockEmailStorage**: Test email-note linking without real emails

**Calendar System Integration**
- **MockCalendarService**: Simulate calendar operations for testing
- **FakeCalendarEvent**: Generate test calendar data for note linking
- **MockEventStorage**: Test calendar-note integration without real events

### UI Testing Mocks

**Terminal Interface Mocking**
- **MockTerminal**: Simulate terminal input/output for UI tests
- **FakeKeyboardInput**: Generate keyboard events for interaction testing
- **MockRatatui**: Test UI components without real terminal rendering

## Test Data Management

### Fixture Data

**Sample Notes**
- **basic_note.md**: Simple note with title and content
- **frontmatter_note.md**: Note with comprehensive YAML frontmatter
- **linked_note.md**: Note containing wiki links to other notes
- **large_note.md**: Large note for performance testing
- **malformed_note.md**: Note with parsing challenges

**Sample Directories**
- **vault_basic/**: Simple directory with a few notes
- **vault_complex/**: Complex nested directory structure
- **vault_mixed/**: Directory with various file types
- **vault_obsidian/**: Real Obsidian vault structure for compatibility testing

### Test Environment Setup

**Database Setup**
```rust
async fn setup_test_database() -> TestDatabase {
    let db = TestDatabase::in_memory().await;
    db.run_migrations().await;
    db.seed_test_data().await;
    db
}
```

**File System Setup**
```rust
fn setup_test_filesystem() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    create_sample_notes(&temp_dir);
    temp_dir
}
```

**Plugin Setup**
```rust
async fn setup_test_plugin() -> NotesPlugin {
    let config = NotesConfig::test_default();
    let plugin = NotesPlugin::new(config).await;
    plugin.initialize_test_data().await;
    plugin
}
```

## Continuous Integration

### Automated Test Execution

**Test Categories**
- **Unit Tests**: Run on every commit, fast execution
- **Integration Tests**: Run on pull requests, moderate execution time
- **Feature Tests**: Run nightly, comprehensive but slow
- **Performance Tests**: Run weekly, benchmark against baselines

**Platform Testing**
- **Linux**: Primary development platform, full test suite
- **macOS**: Secondary platform, core functionality tests
- **Windows**: Limited support, basic functionality tests

**Test Reporting**
- **Coverage Reports**: Minimum 90% line coverage required
- **Performance Benchmarks**: Track performance regression
- **Integration Status**: Monitor external service dependencies
- **Documentation Tests**: Verify all examples work correctly