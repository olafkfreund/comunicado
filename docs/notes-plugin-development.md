# Notes Plugin Development Guide

This document provides detailed information for developers working on or extending the Comunicado Notes Plugin.

## Architecture Overview

The notes plugin follows a modular architecture with clear separation of concerns:

```
src/plugins/notes/
├── mod.rs                    # Module exports and public API
├── types.rs                  # Core data structures and types
├── plugin.rs                 # Main plugin implementation
├── storage.rs               # Note storage abstraction
├── database.rs              # SQLite database layer
├── manager.rs               # High-level note management
├── parser.rs                # Markdown parsing and processing
├── indexer.rs               # Search indexing
├── linker.rs                # Wiki link resolution
├── watcher.rs               # File system monitoring
├── scanner.rs               # Directory scanning
├── integration.rs           # Integration coordination
├── advanced_search.rs       # Advanced search engine
├── email_integration.rs     # Email-to-notes conversion
├── mobile_integration.rs    # Mobile device integration
├── calendar_integration.rs  # Calendar event integration
├── tui.rs                   # Terminal user interface
└── tui_render.rs           # TUI rendering implementation
```

## Core Components

### 1. Storage Layer (`storage.rs`)

The storage layer provides a unified interface for note persistence:

```rust
pub struct NoteStorage {
    database: Arc<NotesDatabase>,
    base_directory: PathBuf,
    watched_directories: Arc<RwLock<Vec<WatchedDirectory>>>,
}

impl NoteStorage {
    pub async fn new(base_dir: &str) -> NoteResult<Self>
    pub async fn create_note(&self, note: Note) -> NoteResult<NoteId>
    pub async fn get_note(&self, id: &NoteId) -> NoteResult<Option<Note>>
    pub async fn update_note(&self, note: Note) -> NoteResult<()>
    pub async fn delete_note(&self, id: &NoteId) -> NoteResult<()>
    pub async fn list_notes(&self) -> NoteResult<Vec<Note>>
    pub async fn search_notes(&self, query: &str) -> NoteResult<Vec<NoteSearchResult>>
}
```

**Key Features:**
- Asynchronous operations for non-blocking I/O
- SQLite database with FTS5 full-text search
- File system integration with directory watching
- Transaction support for data consistency

### 2. Note Manager (`manager.rs`)

High-level orchestration of note operations:

```rust
pub struct NoteManager {
    storage: Arc<NoteStorage>,
    indexer: Arc<NoteIndexer>,
    watcher: Arc<FileWatcher>,
    config: NotesConfig,
}

impl NoteManager {
    pub async fn new(
        storage: NoteStorage,
        indexer: NoteIndexer,
        watcher: FileWatcher,
        config: NotesConfig,
    ) -> NoteResult<Self>
    
    pub async fn create_note(&self, title: String, content: String) -> NoteResult<Note>
    pub async fn search_notes(&self, query: &str) -> NoteResult<Vec<NoteSearchResult>>
    pub async fn get_note(&self, note_id: &str) -> NoteResult<Option<Note>>
    pub async fn update_note(&self, note: Note) -> NoteResult<()>
}
```

### 3. Markdown Parser (`parser.rs`)

CommonMark-compliant parsing with extensions:

```rust
pub struct MarkdownParser {
    options: ParserOptions,
    link_resolver: Arc<LinkResolver>,
}

impl MarkdownParser {
    pub fn new(options: ParserOptions, link_resolver: Arc<LinkResolver>) -> Self
    pub fn parse(&self, content: &str) -> ParseResult<ParsedNote>
    pub fn parse_frontmatter(&self, content: &str) -> Option<NoteFrontmatter>
    pub fn extract_wiki_links(&self, content: &str) -> Vec<WikiLink>
}
```

**Supported Features:**
- YAML frontmatter parsing
- Wiki-style `[[link]]` extraction
- Table support
- Task list parsing
- Code block highlighting
- Math expressions (KaTeX)

### 4. Search Engine (`advanced_search.rs`)

Sophisticated search with ranking and filtering:

```rust
pub struct AdvancedSearchEngine {
    storage: Arc<NoteStorage>,
    cache: Arc<tokio::sync::RwLock<HashMap<String, SearchCacheEntry>>>,
    link_popularity: Arc<tokio::sync::RwLock<HashMap<NoteId, usize>>>,
    default_options: AdvancedSearchOptions,
    cache_ttl: Duration,
}

impl AdvancedSearchEngine {
    pub fn new(storage: Arc<NoteStorage>) -> Self
    pub async fn search(&self, options: &AdvancedSearchOptions) -> NoteResult<SearchResultSummary>
    pub async fn suggest(&self, partial_query: &str) -> NoteResult<Vec<String>>
    pub async fn update_link_popularity(&self) -> NoteResult<()>
}
```

**Search Features:**
- TF-IDF relevance scoring
- Configurable ranking weights
- Result caching with TTL
- Query suggestions
- Category-specific search
- Date range filtering
- Tag-based filtering

## Plugin System Integration

### Plugin Trait Implementation

The notes plugin implements the core `Plugin` trait:

```rust
impl Plugin for NotesPlugin {
    fn info(&self) -> PluginInfo
    fn initialize(&mut self, config: &PluginConfig) -> PluginResult<()>
    fn start(&mut self) -> PluginResult<()>
    fn stop(&mut self) -> PluginResult<()>
    fn pause(&mut self) -> PluginResult<()>
    fn resume(&mut self) -> PluginResult<()>
    fn config_schema(&self) -> Option<serde_json::Value>
    fn validate_config(&self, config: &serde_json::Value) -> PluginResult<()>
    fn update_config(&mut self, config: &PluginConfig) -> PluginResult<()>
    fn health_check(&self) -> PluginResult<PluginHealthStatus>
    fn as_any(&self) -> &dyn Any
    fn as_any_mut(&mut self) -> &mut dyn Any
}
```

### Configuration Schema

The plugin provides a comprehensive JSON schema for validation:

```json
{
  "type": "object",
  "properties": {
    "default_directory": {
      "type": "string",
      "description": "Default directory for notes"
    },
    "max_search_results": {
      "type": "integer",
      "minimum": 1,
      "maximum": 1000
    },
    "auto_index": {
      "type": "boolean",
      "description": "Whether to automatically index notes"
    },
    "vim_mode": {
      "type": "boolean",
      "description": "Enable vim-style keybindings in TUI"
    },
    "enable_tui": {
      "type": "boolean",
      "description": "Enable TUI interface"
    },
    "enable_email_integration": {
      "type": "boolean",
      "description": "Enable email to notes integration"
    }
  },
  "required": ["default_directory"]
}
```

## Integration Components

### Email Integration (`email_integration.rs`)

Converts emails to structured notes:

```rust
pub struct EmailIntegrationService {
    note_storage: Arc<NoteStorage>,
    contact_db: Arc<RwLock<HashMap<String, EmailContact>>>,
    thread_db: Arc<RwLock<HashMap<String, EmailThread>>>,
    config: EmailIntegrationConfig,
}

impl EmailIntegrationService {
    pub fn new(note_storage: Arc<NoteStorage>) -> Self
    pub async fn create_note_from_email(&self, email: &EmailMessage) -> NoteResult<Note>
    pub async fn link_email_to_note(&self, email_id: &str, note_id: &NoteId) -> NoteResult<()>
    pub async fn get_email_threads(&self) -> NoteResult<Vec<EmailThread>>
}
```

**Email Note Generation:**
- Automatic subject extraction
- Content cleaning and formatting
- Attachment handling
- Thread grouping
- Contact management

### Mobile Integration (`mobile_integration.rs`)

KDE Connect integration for mobile devices:

```rust
pub struct MobileNotesIntegration {
    note_storage: Arc<NoteStorage>,
    mobile_client: Arc<RwLock<KdeConnectClient>>,
    message_store: Arc<MessageStore>,
    config: MobileNotesConfig,
}

impl MobileNotesIntegration {
    pub async fn new(
        note_storage: Arc<NoteStorage>,
        mobile_client: Arc<RwLock<KdeConnectClient>>,
        message_store: Arc<MessageStore>,
        config: MobileNotesConfig,
    ) -> NoteResult<Self>
    
    pub async fn process_sms_message(&self, message: &SmsMessage) -> NoteResult<Option<Note>>
    pub async fn send_note_to_mobile(&self, note: &Note, device_id: &str) -> NoteResult<()>
}
```

### Calendar Integration (`calendar_integration.rs`)

Automatic meeting note generation:

```rust
pub struct CalendarNotesIntegration {
    note_storage: Arc<NoteStorage>,
    calendar_manager: Arc<CalendarManager>,
    event_notes: Arc<RwLock<HashMap<String, NoteId>>>,
    config: CalendarNotesConfig,
}

impl CalendarNotesIntegration {
    pub async fn process_calendar_event(&self, event: &Event) -> NoteResult<Option<Note>>
    pub async fn create_meeting_note(&self, event: &Event) -> NoteResult<Note>
    pub async fn link_event_to_note(&self, event_id: &str, note_id: &NoteId) -> NoteResult<()>
}
```

## Terminal User Interface (TUI)

### TUI Architecture (`tui.rs`, `tui_render.rs`)

The TUI is built using the `ratatui` framework:

```rust
pub struct NoteTUI {
    storage: Arc<NoteStorage>,
    search_engine: Arc<AdvancedSearchEngine>,
    mode: TUIMode,
    notes: Vec<Note>,
    list_state: ListState,
    current_note: Option<Note>,
    editor: TextArea<'static>,
    search_input: TextArea<'static>,
    config: TUIConfig,
    theme: TUITheme,
}

impl NoteTUI {
    pub async fn new(
        storage: Arc<NoteStorage>,
        search_engine: Arc<AdvancedSearchEngine>,
    ) -> NoteResult<Self>
    
    pub async fn run(&mut self) -> NoteResult<()>
    pub async fn handle_key(&mut self, key: KeyEvent) -> NoteResult<bool>
    pub fn render(&mut self, f: &mut Frame)
}
```

### TUI Modes

The interface supports multiple interaction modes:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TUIMode {
    Browse,        // Navigate note list
    View,          // Read-only note viewing
    Edit,          // Note editing with syntax highlighting
    Search,        // Interactive search
    Create,        // New note creation
    Settings,      // Configuration
    Help,          // Keyboard shortcuts and help
}
```

### Key Binding System

Configurable keyboard shortcuts with vim-style navigation:

```rust
pub struct KeyBindings {
    pub browse_mode: BrowseModeKeys,
    pub edit_mode: EditModeKeys,
    pub search_mode: SearchModeKeys,
    pub global: GlobalKeys,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            browse_mode: BrowseModeKeys {
                up: vec![KeyCode::Char('k'), KeyCode::Up],
                down: vec![KeyCode::Char('j'), KeyCode::Down],
                select: vec![KeyCode::Enter],
                new_note: vec![KeyCode::Char('n')],
                edit_note: vec![KeyCode::Char('e')],
                delete_note: vec![KeyCode::Char('d')],
                search: vec![KeyCode::Char('/')],
                quit: vec![KeyCode::Char('q'), KeyCode::Esc],
            },
            // ... other modes
        }
    }
}
```

## Testing Infrastructure

### Test Organization

Tests are organized by component with comprehensive coverage:

```
tests/
├── basic_ai_integration_test.rs
├── storage_tests.rs
├── parser_tests.rs
├── search_tests.rs
├── integration_tests.rs
├── email_integration_tests.rs
├── mobile_integration_tests.rs
├── calendar_integration_tests.rs
└── tui_tests.rs
```

### Test Utilities

Common test utilities for consistent testing:

```rust
// Test storage with in-memory database
pub fn create_test_storage() -> Arc<NoteStorage> {
    Arc::new(NoteStorage::new_in_memory().await.unwrap())
}

// Test note generation
pub fn create_test_note(title: &str, content: &str) -> Note {
    Note {
        id: NoteId::new(),
        title: title.to_string(),
        content: content.to_string(),
        created: Utc::now(),
        modified: Utc::now(),
        file_path: None,
        frontmatter: None,
        word_count: content.split_whitespace().count(),
        character_count: content.len(),
        tags: Vec::new(),
        links: Vec::new(),
        backlinks: Vec::new(),
    }
}

// Mock email for testing
pub fn create_test_email() -> EmailMessage {
    // Implementation
}
```

### Integration Testing

Comprehensive integration tests verify component interactions:

```rust
#[tokio::test]
async fn test_email_to_note_workflow() {
    let storage = create_test_storage();
    let email_service = EmailIntegrationService::new(storage.clone());
    
    let email = create_test_email();
    let note = email_service.create_note_from_email(&email).await.unwrap();
    
    assert_eq!(note.title, format!("Email: {}", email.subject));
    assert!(note.content.contains(&email.body));
    
    // Verify note is searchable
    let results = storage.search_notes(&email.subject).await.unwrap();
    assert!(!results.is_empty());
}
```

## Performance Considerations

### Database Optimization

- **Indexing Strategy**: Comprehensive indexes on search columns
- **Connection Pooling**: SQLite connection pool for concurrent access
- **Batch Operations**: Bulk insert/update for large datasets
- **FTS5 Configuration**: Optimized full-text search configuration

### Memory Management

- **Arc/RwLock**: Shared ownership with minimal locking contention
- **Streaming**: Large file processing with streaming
- **Caching**: Intelligent caching with TTL and memory limits
- **Lazy Loading**: On-demand loading of note content

### File System Monitoring

- **Efficient Watching**: Optimized notify configuration
- **Debouncing**: Event debouncing to reduce noise
- **Batch Processing**: Batched file system updates
- **Error Handling**: Robust error recovery

## Error Handling

### Error Types

Comprehensive error types for different failure modes:

```rust
#[derive(Debug, Error)]
pub enum NoteError {
    #[error("Note not found: {0}")]
    NotFound(String),
    
    #[error("Invalid note format: {0}")]
    InvalidFormat(String),
    
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Parsing error: {0}")]
    Parsing(String),
    
    #[error("Search error: {0}")]
    Search(String),
    
    #[error("Integration error: {0}")]
    Integration(String),
}
```

### Error Recovery

- **Graceful Degradation**: Continue operating with reduced functionality
- **Automatic Retry**: Retry transient failures with exponential backoff
- **User Notification**: Clear error messages with suggested actions
- **Logging**: Comprehensive error logging for debugging

## Extending the Plugin

### Adding New Integrations

To add a new integration (e.g., Slack, Discord):

1. **Create Integration Module**:
   ```rust
   // src/plugins/notes/slack_integration.rs
   pub struct SlackNotesIntegration {
       note_storage: Arc<NoteStorage>,
       slack_client: Arc<SlackClient>,
       config: SlackIntegrationConfig,
   }
   ```

2. **Implement Integration Trait**:
   ```rust
   impl Integration for SlackNotesIntegration {
       async fn process_message(&self, message: &SlackMessage) -> NoteResult<Option<Note>>;
       async fn send_note(&self, note: &Note, channel: &str) -> NoteResult<()>;
   }
   ```

3. **Register with Plugin**:
   ```rust
   // In plugin.rs
   pub async fn register_slack_integration(&mut self, config: SlackIntegrationConfig) -> PluginResult<()>
   ```

### Custom Search Providers

Add custom search functionality:

```rust
pub trait SearchProvider: Send + Sync {
    async fn search(&self, query: &SearchQuery) -> SearchResult<Vec<SearchResultItem>>;
    fn get_capabilities(&self) -> Vec<SearchCapability>;
}

// Register with search engine
search_engine.register_provider(Box::new(CustomSearchProvider::new()));
```

### TUI Extensions

Add custom TUI modes and panels:

```rust
pub trait TUIExtension: Send + Sync {
    fn get_mode_name(&self) -> &str;
    fn handle_key(&mut self, key: KeyEvent, context: &TUIContext) -> TUIResult<bool>;
    fn render(&self, frame: &mut Frame, area: Rect, context: &TUIContext);
}
```

## Debugging

### Debug Configuration

Enable detailed logging for troubleshooting:

```toml
[plugins.notes.debug]
log_level = "trace"
log_database_queries = true
log_file_operations = true
log_search_operations = true
log_integration_events = true
enable_performance_metrics = true
```

### Debug Tools

Built-in debugging utilities:

```rust
// Database inspection
pub async fn debug_database_state(&self) -> DebugInfo;

// Search index analysis  
pub async fn debug_search_index(&self) -> IndexDebugInfo;

// File system monitoring status
pub async fn debug_file_watcher(&self) -> WatcherDebugInfo;
```

### Performance Monitoring

Track performance metrics:

```rust
pub struct PerformanceMetrics {
    pub search_latency: Histogram,
    pub database_operations: Counter,
    pub file_operations: Counter,
    pub memory_usage: Gauge,
    pub cache_hit_rate: Gauge,
}
```

This development guide provides the foundation for understanding, maintaining, and extending the Comunicado Notes Plugin. For specific implementation details, refer to the inline documentation and test files.