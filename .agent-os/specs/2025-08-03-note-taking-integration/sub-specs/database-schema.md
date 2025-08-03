# Database Schema

This is the database schema implementation for the spec detailed in @.agent-os/specs/2025-08-03-note-taking-integration/spec.md

> Created: 2025-08-03
> Version: 1.0.0

## Database Requirements

The notes plugin requires SQLite database tables for indexing, search, and cross-reference management. The schema supports full-text search, bidirectional linking, and integration with email and calendar systems.

## Core Schema

### Notes Table

Stores note metadata and enables fast lookup and filtering.

```sql
CREATE TABLE notes (
    id TEXT PRIMARY KEY,                    -- UUID or path-based identifier
    title TEXT NOT NULL,                    -- Note title (from frontmatter or filename)
    file_path TEXT NOT NULL UNIQUE,        -- Absolute file path
    directory_id INTEGER NOT NULL,         -- Reference to watched directories
    content_hash TEXT NOT NULL,            -- SHA-256 of content for change detection
    word_count INTEGER NOT NULL DEFAULT 0, -- Word count for statistics
    created_at INTEGER NOT NULL,           -- Unix timestamp (milliseconds)
    modified_at INTEGER NOT NULL,          -- File modification time
    indexed_at INTEGER NOT NULL,           -- Last indexing time
    file_size INTEGER NOT NULL DEFAULT 0,  -- File size in bytes
    is_deleted BOOLEAN NOT NULL DEFAULT FALSE, -- Soft delete flag
    metadata TEXT,                          -- JSON metadata from frontmatter
    FOREIGN KEY (directory_id) REFERENCES watched_directories(id) ON DELETE CASCADE
);

-- Indexes for performance
CREATE INDEX idx_notes_title ON notes(title);
CREATE INDEX idx_notes_modified_at ON notes(modified_at DESC);
CREATE INDEX idx_notes_directory_id ON notes(directory_id);
CREATE INDEX idx_notes_created_at ON notes(created_at DESC);
CREATE INDEX idx_notes_is_deleted ON notes(is_deleted) WHERE is_deleted = FALSE;
```

### Full-Text Search Table

SQLite FTS5 virtual table for content search.

```sql
CREATE VIRTUAL TABLE notes_fts USING fts5(
    note_id UNINDEXED,                     -- Reference to notes.id
    title,                                 -- Searchable title
    content,                               -- Full note content
    tags,                                  -- Space-separated tags
    content='',                            -- External content storage
    content_rowid='note_id'                -- Link to external content
);

-- Triggers to maintain FTS index
CREATE TRIGGER notes_fts_insert AFTER INSERT ON notes BEGIN
    INSERT INTO notes_fts(note_id, title, content, tags) 
    VALUES (new.id, new.title, 
            (SELECT content FROM note_content WHERE note_id = new.id),
            (SELECT GROUP_CONCAT(tag, ' ') FROM note_tags WHERE note_id = new.id));
END;

CREATE TRIGGER notes_fts_update AFTER UPDATE ON notes BEGIN
    UPDATE notes_fts SET 
        title = new.title,
        content = (SELECT content FROM note_content WHERE note_id = new.id),
        tags = (SELECT GROUP_CONCAT(tag, ' ') FROM note_tags WHERE note_id = new.id)
    WHERE note_id = new.id;
END;

CREATE TRIGGER notes_fts_delete AFTER DELETE ON notes BEGIN
    DELETE FROM notes_fts WHERE note_id = old.id;
END;
```

### Note Content Table

Stores actual note content separately for performance.

```sql
CREATE TABLE note_content (
    note_id TEXT PRIMARY KEY,              -- Reference to notes.id
    content TEXT NOT NULL,                 -- Full markdown content
    frontmatter TEXT,                      -- YAML frontmatter as JSON
    parsed_links TEXT,                     -- JSON array of parsed wiki links
    updated_at INTEGER NOT NULL,           -- Content update timestamp
    FOREIGN KEY (note_id) REFERENCES notes(id) ON DELETE CASCADE
);

CREATE INDEX idx_note_content_updated_at ON note_content(updated_at DESC);
```

### Tags Table

Tag management with frequency tracking.

```sql
CREATE TABLE note_tags (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    note_id TEXT NOT NULL,                 -- Reference to notes.id
    tag TEXT NOT NULL,                     -- Tag name (lowercase, normalized)
    created_at INTEGER NOT NULL,           -- When tag was added
    FOREIGN KEY (note_id) REFERENCES notes(id) ON DELETE CASCADE,
    UNIQUE(note_id, tag)
);

CREATE INDEX idx_note_tags_note_id ON note_tags(note_id);
CREATE INDEX idx_note_tags_tag ON note_tags(tag);

-- Tag frequency view for popular tags
CREATE VIEW tag_frequency AS
SELECT tag, COUNT(*) as note_count, MAX(created_at) as last_used
FROM note_tags 
GROUP BY tag 
ORDER BY note_count DESC;
```

## Linking Schema

### Wiki Links Table

Stores wiki-style links between notes.

```sql
CREATE TABLE wiki_links (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_note_id TEXT NOT NULL,          -- Note containing the link
    target_note_id TEXT,                   -- Target note (NULL if not found)
    link_text TEXT NOT NULL,               -- Original link text [[Target]]
    display_text TEXT,                     -- Custom display text [[Target|Display]]
    line_number INTEGER NOT NULL,          -- Line number in source note
    is_valid BOOLEAN NOT NULL DEFAULT FALSE, -- Whether target exists
    created_at INTEGER NOT NULL,           -- When link was discovered
    updated_at INTEGER NOT NULL,           -- Last validation check
    FOREIGN KEY (source_note_id) REFERENCES notes(id) ON DELETE CASCADE,
    FOREIGN KEY (target_note_id) REFERENCES notes(id) ON DELETE SET NULL
);

CREATE INDEX idx_wiki_links_source ON wiki_links(source_note_id);
CREATE INDEX idx_wiki_links_target ON wiki_links(target_note_id);
CREATE INDEX idx_wiki_links_text ON wiki_links(link_text);
CREATE INDEX idx_wiki_links_valid ON wiki_links(is_valid) WHERE is_valid = TRUE;

-- Backlinks view for efficient bidirectional navigation
CREATE VIEW note_backlinks AS
SELECT 
    target_note_id as note_id,
    source_note_id as backlink_note_id,
    link_text,
    display_text,
    line_number
FROM wiki_links 
WHERE is_valid = TRUE AND target_note_id IS NOT NULL;
```

## Integration Schema

### Email-Note Links

Links between notes and email messages.

```sql
CREATE TABLE email_note_links (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    note_id TEXT NOT NULL,                 -- Reference to notes.id
    email_id TEXT NOT NULL,                -- Email message ID
    link_type TEXT NOT NULL,               -- 'reference', 'created_from', 'attachment'
    context TEXT,                          -- Additional context or description
    created_at INTEGER NOT NULL,           -- When link was created
    created_by TEXT,                       -- User or system that created link
    FOREIGN KEY (note_id) REFERENCES notes(id) ON DELETE CASCADE,
    UNIQUE(note_id, email_id, link_type)
);

CREATE INDEX idx_email_note_links_note_id ON email_note_links(note_id);
CREATE INDEX idx_email_note_links_email_id ON email_note_links(email_id);
CREATE INDEX idx_email_note_links_type ON email_note_links(link_type);
```

### Calendar-Note Links

Links between notes and calendar events.

```sql
CREATE TABLE calendar_note_links (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    note_id TEXT NOT NULL,                 -- Reference to notes.id
    event_id TEXT NOT NULL,                -- Calendar event ID
    link_type TEXT NOT NULL,               -- 'meeting_notes', 'reference', 'agenda'
    context TEXT,                          -- Meeting type, agenda item, etc.
    created_at INTEGER NOT NULL,           -- When link was created
    created_by TEXT,                       -- User or system that created link
    FOREIGN KEY (note_id) REFERENCES notes(id) ON DELETE CASCADE,
    UNIQUE(note_id, event_id, link_type)
);

CREATE INDEX idx_calendar_note_links_note_id ON calendar_note_links(note_id);
CREATE INDEX idx_calendar_note_links_event_id ON calendar_note_links(event_id);
CREATE INDEX idx_calendar_note_links_type ON calendar_note_links(link_type);
```

## Directory Management Schema

### Watched Directories

Tracks directories being monitored for notes.

```sql
CREATE TABLE watched_directories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL UNIQUE,             -- Absolute directory path
    name TEXT NOT NULL,                    -- User-friendly name
    recursive BOOLEAN NOT NULL DEFAULT TRUE, -- Scan subdirectories
    enabled BOOLEAN NOT NULL DEFAULT TRUE,  -- Active monitoring
    last_scan INTEGER,                      -- Last successful scan timestamp
    note_count INTEGER NOT NULL DEFAULT 0, -- Cached note count
    ignore_patterns TEXT,                  -- JSON array of glob patterns to ignore
    created_at INTEGER NOT NULL,           -- When directory was added
    updated_at INTEGER NOT NULL,           -- Last configuration change
    CHECK(path != '')
);

CREATE INDEX idx_watched_directories_enabled ON watched_directories(enabled) WHERE enabled = TRUE;
CREATE INDEX idx_watched_directories_path ON watched_directories(path);
```

### File Change Tracking

Tracks file system changes for incremental updates.

```sql
CREATE TABLE file_changes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    directory_id INTEGER NOT NULL,         -- Reference to watched_directories
    file_path TEXT NOT NULL,               -- Absolute file path
    change_type TEXT NOT NULL,             -- 'created', 'modified', 'deleted', 'renamed'
    old_path TEXT,                         -- For rename operations
    detected_at INTEGER NOT NULL,          -- When change was detected
    processed_at INTEGER,                  -- When change was processed (NULL = pending)
    error_message TEXT,                    -- Error during processing
    FOREIGN KEY (directory_id) REFERENCES watched_directories(id) ON DELETE CASCADE
);

CREATE INDEX idx_file_changes_processed ON file_changes(processed_at) WHERE processed_at IS NULL;
CREATE INDEX idx_file_changes_directory ON file_changes(directory_id);
CREATE INDEX idx_file_changes_detected_at ON file_changes(detected_at DESC);
```

## Configuration Schema

### Plugin Settings

Stores plugin configuration and user preferences.

```sql
CREATE TABLE note_settings (
    key TEXT PRIMARY KEY,                  -- Setting key (hierarchical with dots)
    value TEXT NOT NULL,                   -- JSON-encoded value
    value_type TEXT NOT NULL,              -- 'string', 'number', 'boolean', 'array', 'object'
    description TEXT,                      -- Human-readable description
    updated_at INTEGER NOT NULL,           -- Last update time
    CHECK(key != '')
);

-- Default settings
INSERT INTO note_settings (key, value, value_type, description, updated_at) VALUES
('directories.default_path', '"/home/user/notes"', 'string', 'Default directory for new notes', strftime('%s', 'now') * 1000),
('ui.editor.vim_mode', 'true', 'boolean', 'Enable vim-style keybindings', strftime('%s', 'now') * 1000),
('search.max_results', '100', 'number', 'Maximum search results to display', strftime('%s', 'now') * 1000),
('indexing.auto_refresh', 'true', 'boolean', 'Automatically refresh index on file changes', strftime('%s', 'now') * 1000),
('templates.default_note', '"# {{title}}\n\nCreated: {{date}}\nTags: \n\n"', 'string', 'Default template for new notes', strftime('%s', 'now') * 1000);
```

## Migration Scripts

### Initial Schema Creation

```sql
-- Migration 001: Initial schema
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;

-- Create all tables in dependency order
-- (Tables created in order shown above)

-- Create version tracking
CREATE TABLE schema_version (
    version INTEGER PRIMARY KEY,
    applied_at INTEGER NOT NULL,
    description TEXT
);

INSERT INTO schema_version (version, applied_at, description) 
VALUES (1, strftime('%s', 'now') * 1000, 'Initial notes plugin schema');
```

### Performance Optimization

```sql
-- Additional indexes for common queries
CREATE INDEX idx_notes_composite_active ON notes(is_deleted, modified_at) WHERE is_deleted = FALSE;
CREATE INDEX idx_wiki_links_composite ON wiki_links(source_note_id, is_valid) WHERE is_valid = TRUE;

-- Statistics for query optimization
ANALYZE;
```

## Data Integrity Constraints

### Referential Integrity
- All foreign key constraints are enforced with appropriate CASCADE/SET NULL actions
- Unique constraints prevent duplicate links and ensure data consistency
- Check constraints validate data format and ranges

### Data Validation
- File paths must be absolute and non-empty
- Timestamps are stored as Unix milliseconds for consistency
- Link types are restricted to predefined values
- JSON fields are validated during application-level operations

### Cleanup Procedures

```sql
-- Cleanup orphaned records
DELETE FROM wiki_links WHERE source_note_id NOT IN (SELECT id FROM notes);
DELETE FROM note_tags WHERE note_id NOT IN (SELECT id FROM notes);
DELETE FROM note_content WHERE note_id NOT IN (SELECT id FROM notes);

-- Update statistics
UPDATE watched_directories SET 
    note_count = (SELECT COUNT(*) FROM notes WHERE directory_id = watched_directories.id AND is_deleted = FALSE);

-- Vacuum and optimize
VACUUM;
PRAGMA optimize;
```