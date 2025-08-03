# Technical Specification

This is the technical specification for the spec detailed in @.agent-os/specs/2025-08-03-note-taking-integration/spec.md

> Created: 2025-08-03
> Version: 1.0.0

## Technical Requirements

### Core Functionality Requirements

- **Markdown File Processing**: Support for CommonMark with YAML frontmatter parsing, handling files up to 10MB, and preserving original formatting
- **Real-time File System Monitoring**: Watch configured directories for changes using `notify` crate with debounced events (500ms delay)
- **Full-Text Search**: SQLite FTS5 integration with indexing of title, content, tags, and metadata with sub-second search response times
- **Wiki-Link Resolution**: Parse `[[Note Name]]` and `[[Note Name|Display Text]]` syntax with automatic bidirectional link discovery
- **Cross-Application Integration**: API for linking notes to emails (message-id) and calendar events (event-id) with persistent reference storage

### Performance Requirements

- **Startup Time**: Initial directory scan and indexing must complete within 5 seconds for 1000 notes
- **Search Performance**: Full-text search across 10,000 notes must return results within 200ms
- **Memory Usage**: Plugin should use no more than 50MB RAM for typical usage (1000 notes)
- **File Watching**: Changes should be reflected in the interface within 1 second of external modification

### UI/UX Requirements

- **Keyboard Navigation**: Full vim-style navigation with `j/k` for movement, `/` for search, `Enter` to open notes
- **Multi-pane Interface**: Resizable panes for note list, note content, and metadata/links panel
- **Syntax Highlighting**: Markdown syntax highlighting using existing terminal capabilities
- **Preview Mode**: Read-only note preview with rendered markdown (links, formatting, etc.)

### Integration Requirements

- **Plugin System**: Implement as standard Comunicado plugin using existing `PluginManager` and service registration
- **Email Integration**: Hooks into email view and composition to add note references and create notes from email content
- **Calendar Integration**: Integration with calendar events for meeting notes and event documentation
- **Configuration**: User-configurable note directories, file patterns, and behavior settings

## Approach Options

**Option A: File System Only (Selected)**
- Pros: Simple implementation, works with any markdown tool, no lock-in, respects existing workflows
- Cons: Limited metadata handling, relies on file system performance, no advanced features

**Option B: Database-Centric with File Sync**
- Pros: Better performance, advanced features, rich metadata
- Cons: Complex sync logic, potential data conflicts, tool compatibility issues

**Option C: Hybrid Approach**
- Pros: Best of both worlds, gradual migration path
- Cons: Implementation complexity, maintenance overhead

**Rationale**: Option A provides the best user experience for terminal users who already have established markdown workflows. It maintains compatibility with existing tools while providing the integration benefits within Comunicado.

## Architecture Design

### Plugin Structure
```
src/plugins/notes/
├── mod.rs              # Plugin entry point and registration
├── manager.rs          # Note management service
├── watcher.rs          # File system monitoring
├── parser.rs           # Markdown and frontmatter parsing
├── indexer.rs          # Search indexing and FTS
├── linker.rs           # Wiki-link resolution and backlinks
├── ui/
│   ├── mod.rs          # UI module exports
│   ├── note_list.rs    # Note browser interface
│   ├── note_view.rs    # Note display and editing
│   ├── search.rs       # Search interface
│   └── integrations.rs # Email/calendar integration UI
├── storage.rs          # SQLite schema and operations
└── config.rs           # Configuration management
```

### Service Integration
```rust
pub struct NotesPlugin {
    manager: Arc<NoteManager>,
    ui_service: Arc<NotesUiService>,
    config: NotesConfig,
}

impl Plugin for NotesPlugin {
    fn name(&self) -> &str { "notes" }
    fn register_services(&self, registry: &mut ServiceRegistry) {
        registry.register("notes_manager", self.manager.clone());
        registry.register("notes_ui", self.ui_service.clone());
    }
    fn register_shortcuts(&self) -> Vec<KeyBinding> {
        vec![KeyBinding::new("ctrl-n", "open_notes")]
    }
}
```

## External Dependencies

### Core Dependencies
- **notify v6.1**: File system watching with cross-platform support
- **pulldown-cmark v0.9**: CommonMark parser with GitHub Flavored Markdown extensions
- **serde_yaml v0.9**: YAML frontmatter parsing and serialization
- **sqlx v0.7**: Database operations with SQLite FTS5 support
- **regex v1.10**: Wiki-link pattern matching and text processing

**Justification**: These are lightweight, well-maintained crates that align with Comunicado's existing dependencies and provide the necessary functionality without adding significant complexity.

### Optional Dependencies
- **git2 v0.18**: Git integration for versioning and conflict resolution (feature-gated)
- **syntect v5.1**: Enhanced syntax highlighting for code blocks in notes (feature-gated)

## Data Flow Architecture

### Note Discovery and Indexing
```
Directory Scan → File Parsing → Metadata Extraction → Link Analysis → Database Storage
     ↓              ↓              ↓                 ↓              ↓
File Watcher → Change Detection → Incremental Update → Search Index → UI Refresh
```

### Search and Retrieval
```
User Query → Query Parser → FTS Search → Result Ranking → Link Resolution → UI Display
```

### Cross-Application Linking
```
Email/Calendar Event → Link Creation → Reference Storage → Bidirectional Index → Navigation UI
```

## Error Handling Strategy

### File System Errors
- **Permission Issues**: Graceful degradation with user notification
- **Missing Directories**: Automatic creation with user confirmation
- **Corrupted Files**: Skip with logging, continue processing other files
- **Watch Failures**: Fallback to polling mode with user notification

### Database Errors
- **Schema Migration**: Automatic migration with backup creation
- **Corruption**: Rebuild index from file system with progress indication
- **Disk Space**: Clear warnings with cleanup suggestions

### Integration Errors
- **Plugin Conflicts**: Isolated error handling to prevent system crashes
- **Service Unavailable**: Graceful fallback with reduced functionality