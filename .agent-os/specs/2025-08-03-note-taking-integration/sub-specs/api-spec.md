# API Specification

This is the API specification for the spec detailed in @.agent-os/specs/2025-08-03-note-taking-integration/spec.md

> Created: 2025-08-03
> Version: 1.0.0

## Core APIs

### NoteManager Service

The central service for note management operations.

#### Note CRUD Operations

**`create_note(title: &str, content: &str, directory: Option<&Path>) -> Result<NoteId>`**
- **Purpose**: Create a new markdown note with frontmatter
- **Parameters**: 
  - `title`: Note title (becomes filename)
  - `content`: Markdown content
  - `directory`: Optional specific directory (uses default if None)
- **Response**: Unique note identifier
- **Errors**: `IOError`, `InvalidTitle`, `DirectoryNotFound`

**`get_note(id: &NoteId) -> Result<Note>`**
- **Purpose**: Retrieve a note by its identifier
- **Parameters**: Note ID (path-based or UUID)
- **Response**: Complete note with metadata and content
- **Errors**: `NoteNotFound`, `PermissionDenied`

**`update_note(id: &NoteId, content: &str) -> Result<()>`**
- **Purpose**: Update note content and refresh metadata
- **Parameters**: Note ID and new content
- **Response**: Success confirmation
- **Errors**: `NoteNotFound`, `IOError`, `ReadOnlyNote`

**`delete_note(id: &NoteId) -> Result<()>`**
- **Purpose**: Delete note file and remove from index
- **Parameters**: Note ID
- **Response**: Success confirmation
- **Errors**: `NoteNotFound`, `PermissionDenied`

#### Search and Discovery

**`search_notes(query: &SearchQuery) -> Result<Vec<NoteSearchResult>>`**
- **Purpose**: Full-text search across notes with ranking
- **Parameters**: Structured search query with filters
- **Response**: Ranked search results with snippets
- **Errors**: `SearchError`, `IndexCorrupted`

**`list_notes(filter: &NoteFilter) -> Result<Vec<NoteSummary>>`**
- **Purpose**: List notes with optional filtering and sorting
- **Parameters**: Filter criteria (tags, dates, directories)
- **Response**: Note summaries with metadata
- **Errors**: `DatabaseError`

**`get_note_links(id: &NoteId) -> Result<NoteLinkGraph>`**
- **Purpose**: Get all links (incoming and outgoing) for a note
- **Parameters**: Note ID
- **Response**: Graph of bidirectional links
- **Errors**: `NoteNotFound`

### Directory and File Management

**`add_directory(path: &Path, recursive: bool) -> Result<DirectoryId>`**
- **Purpose**: Add directory to watch list and scan for notes
- **Parameters**: Directory path and recursive flag
- **Response**: Directory identifier
- **Errors**: `DirectoryNotFound`, `PermissionDenied`, `AlreadyWatched`

**`remove_directory(id: &DirectoryId) -> Result<()>`**
- **Purpose**: Remove directory from watch list and index
- **Parameters**: Directory ID
- **Response**: Success confirmation
- **Errors**: `DirectoryNotFound`

**`refresh_directory(id: &DirectoryId) -> Result<RefreshStats>`**
- **Purpose**: Force rescan of directory for changes
- **Parameters**: Directory ID
- **Response**: Statistics of changes found
- **Errors**: `DirectoryNotFound`, `ScanError`

### Integration APIs

#### Email Integration

**`create_note_from_email(email_id: &EmailId, template: Option<&str>) -> Result<NoteId>`**
- **Purpose**: Create a note from email content with optional template
- **Parameters**: Email identifier and optional template
- **Response**: Created note ID
- **Errors**: `EmailNotFound`, `TemplateError`

**`link_note_to_email(note_id: &NoteId, email_id: &EmailId) -> Result<()>`**
- **Purpose**: Create bidirectional link between note and email
- **Parameters**: Note and email identifiers
- **Response**: Success confirmation
- **Errors**: `NoteNotFound`, `EmailNotFound`

**`get_email_notes(email_id: &EmailId) -> Result<Vec<NoteSummary>>`**
- **Purpose**: Get all notes linked to an email
- **Parameters**: Email identifier
- **Response**: List of linked notes
- **Errors**: `EmailNotFound`

#### Calendar Integration

**`create_meeting_note(event_id: &EventId, template: Option<&str>) -> Result<NoteId>`**
- **Purpose**: Create meeting notes from calendar event
- **Parameters**: Event ID and optional template
- **Response**: Created note ID
- **Errors**: `EventNotFound`, `TemplateError`

**`link_note_to_event(note_id: &NoteId, event_id: &EventId) -> Result<()>`**
- **Purpose**: Link note to calendar event
- **Parameters**: Note and event identifiers
- **Response**: Success confirmation
- **Errors**: `NoteNotFound`, `EventNotFound`

**`get_event_notes(event_id: &EventId) -> Result<Vec<NoteSummary>>`**
- **Purpose**: Get notes linked to calendar event
- **Parameters**: Event identifier
- **Response**: List of linked notes
- **Errors**: `EventNotFound`

## Data Structures

### Core Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: NoteId,
    pub title: String,
    pub content: String,
    pub path: PathBuf,
    pub frontmatter: Option<NoteFrontmatter>,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub word_count: usize,
    pub tags: Vec<String>,
    pub links: Vec<WikiLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteFrontmatter {
    pub title: Option<String>,
    pub tags: Vec<String>,
    pub created: Option<DateTime<Utc>>,
    pub modified: Option<DateTime<Utc>>,
    pub template: Option<String>,
    pub metadata: HashMap<String, serde_yaml::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiLink {
    pub target: String,
    pub display_text: Option<String>,
    pub line_number: usize,
    pub is_valid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub text: String,
    pub tags: Vec<String>,
    pub directories: Vec<DirectoryId>,
    pub date_range: Option<DateRange>,
    pub limit: Option<usize>,
    pub sort_by: SortCriteria,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteSearchResult {
    pub note: NoteSummary,
    pub score: f64,
    pub snippets: Vec<SearchSnippet>,
    pub matched_tags: Vec<String>,
}
```

### Integration Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailNoteLink {
    pub note_id: NoteId,
    pub email_id: EmailId,
    pub link_type: LinkType,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventNoteLink {
    pub note_id: NoteId,
    pub event_id: EventId,
    pub link_type: LinkType,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LinkType {
    Reference,    // Note references email/event
    CreatedFrom,  // Note created from email/event
    Attachment,   // Note attached to email/event
}
```

## Plugin Interface

### Service Registration

```rust
impl Plugin for NotesPlugin {
    fn name(&self) -> &str { "notes" }
    
    fn register_services(&self, registry: &mut ServiceRegistry) -> Result<()> {
        registry.register("notes_manager", Box::new(self.manager.clone()))?;
        registry.register("notes_ui", Box::new(self.ui_service.clone()))?;
        Ok(())
    }
    
    fn register_commands(&self) -> Vec<Command> {
        vec![
            Command::new("notes.open", "Open notes interface"),
            Command::new("notes.create", "Create new note"),
            Command::new("notes.search", "Search notes"),
            Command::new("notes.link_email", "Link current email to note"),
        ]
    }
    
    fn register_shortcuts(&self) -> Vec<KeyBinding> {
        vec![
            KeyBinding::new("ctrl-n", "notes.open"),
            KeyBinding::new("ctrl-shift-n", "notes.create"),
            KeyBinding::new("ctrl-shift-l", "notes.link_email"),
        ]
    }
}
```

### Event Handling

```rust
pub enum NotesEvent {
    NoteCreated(NoteId),
    NoteUpdated(NoteId),
    NoteDeleted(NoteId),
    DirectoryAdded(DirectoryId),
    LinkCreated(NoteId, LinkTarget),
    SearchPerformed(SearchQuery, usize), // query and result count
}

impl EventHandler<NotesEvent> for NotesPlugin {
    fn handle_event(&self, event: &NotesEvent) -> Result<()> {
        match event {
            NotesEvent::NoteCreated(id) => {
                self.ui_service.refresh_note_list()?;
                self.indexer.index_note(id)?;
            }
            NotesEvent::NoteUpdated(id) => {
                self.ui_service.refresh_note_view(id)?;
                self.indexer.reindex_note(id)?;
            }
            // ... other events
        }
        Ok(())
    }
}
```

## Configuration API

### Settings Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotesConfig {
    pub directories: Vec<WatchedDirectory>,
    pub default_directory: PathBuf,
    pub file_extensions: Vec<String>,
    pub auto_index: bool,
    pub index_interval_seconds: u64,
    pub max_file_size_mb: u64,
    pub templates: HashMap<String, String>,
    pub ui_settings: NotesUiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchedDirectory {
    pub path: PathBuf,
    pub recursive: bool,
    pub enabled: bool,
    pub ignore_patterns: Vec<String>,
}
```

### Configuration Methods

**`get_config() -> NotesConfig`**
**`update_config(config: &NotesConfig) -> Result<()>`**
**`reset_config() -> Result<()>`**

## Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum NotesError {
    #[error("Note not found: {0}")]
    NoteNotFound(NoteId),
    
    #[error("Directory not found: {0}")]
    DirectoryNotFound(PathBuf),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Invalid note title: {0}")]
    InvalidTitle(String),
    
    #[error("Search error: {0}")]
    SearchError(String),
    
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    
    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),
    
    #[error("Parse error: {0}")]
    ParseError(String),
}
```