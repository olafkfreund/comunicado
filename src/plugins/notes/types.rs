//! Core data types for the notes plugin
//! 
//! This module defines the fundamental data structures used throughout the notes system,
//! including Note, NoteFrontmatter, WikiLink, and related types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

/// Unique identifier for notes
pub type NoteId = String;

/// A complete note with metadata, content, and links
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Note {
    /// Unique identifier for the note
    pub id: NoteId,
    /// Note title (extracted from frontmatter or filename)
    pub title: String,
    /// Full note content including frontmatter
    pub content: String,
    /// Absolute path to the note file
    pub path: PathBuf,
    /// Parsed frontmatter metadata
    pub frontmatter: Option<NoteFrontmatter>,
    /// When the note was created
    pub created_at: DateTime<Utc>,
    /// When the note was last modified
    pub modified_at: DateTime<Utc>,
    /// Word count of the note content
    pub word_count: usize,
    /// Tags associated with the note
    pub tags: Vec<String>,
    /// Wiki links found in the note
    pub links: Vec<WikiLink>,
    /// File size in bytes
    pub file_size: u64,
    /// SHA-256 hash of content for change detection
    pub content_hash: String,
    /// Whether the note is marked as deleted
    pub is_deleted: bool,
}

impl Note {
    /// Create a new note with minimal required fields
    pub fn new(id: NoteId, title: String, content: String, path: PathBuf) -> Self {
        let now = Utc::now();
        Self {
            id,
            title,
            content,
            path,
            frontmatter: None,
            created_at: now,
            modified_at: now,
            word_count: 0,
            tags: Vec::new(),
            links: Vec::new(),
            file_size: 0,
            content_hash: String::new(),
            is_deleted: false,
        }
    }

    /// Update the note's content and related metadata
    pub fn update_content(&mut self, new_content: String) {
        self.content = new_content;
        self.modified_at = Utc::now();
        // Note: word_count, content_hash, and links should be updated by the parser
    }

    /// Mark the note as deleted
    pub fn mark_deleted(&mut self) {
        self.is_deleted = true;
        self.modified_at = Utc::now();
    }

    /// Get the note's display name (title or filename)
    pub fn display_name(&self) -> &str {
        if self.title.is_empty() {
            self.path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Untitled")
        } else {
            &self.title
        }
    }

    /// Check if the note has a specific tag
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t.eq_ignore_ascii_case(tag))
    }

    /// Get all outgoing wiki links from this note
    pub fn outgoing_links(&self) -> Vec<&WikiLink> {
        self.links.iter().filter(|link| link.source_note_id == self.id).collect()
    }
}

/// YAML frontmatter metadata for notes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoteFrontmatter {
    /// Note title override
    #[serde(default)]
    pub title: Option<String>,
    /// List of tags
    #[serde(default)]
    pub tags: Vec<String>,
    /// Creation date override
    #[serde(default)]
    pub date: Option<DateTime<Utc>>,
    /// Author information
    #[serde(default)]
    pub author: Option<String>,
    /// Custom metadata fields
    #[serde(default)]
    pub metadata: HashMap<String, serde_yaml::Value>,
    /// Template to use for this note
    #[serde(default)]
    pub template: Option<String>,
    /// Whether this note is a draft
    #[serde(default)]
    pub draft: Option<bool>,
    /// Categories for organization
    #[serde(default)]
    pub categories: Vec<String>,
    /// Aliases for the note
    #[serde(default)]
    pub aliases: Vec<String>,
}

impl NoteFrontmatter {
    /// Create a new empty frontmatter
    pub fn new() -> Self {
        Self {
            title: None,
            tags: Vec::new(),
            date: None,
            author: None,
            metadata: HashMap::new(),
            template: None,
            draft: None,
            categories: Vec::new(),
            aliases: Vec::new(),
        }
    }

    /// Create frontmatter with just a title
    pub fn with_title(title: String) -> Self {
        Self {
            title: Some(title),
            ..Self::new()
        }
    }

    /// Add a tag to the frontmatter
    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }

    /// Add multiple tags to the frontmatter
    pub fn add_tags(&mut self, tags: Vec<String>) {
        for tag in tags {
            self.add_tag(tag);
        }
    }

    /// Get a custom metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&serde_yaml::Value> {
        self.metadata.get(key)
    }

    /// Set a custom metadata value
    pub fn set_metadata(&mut self, key: String, value: serde_yaml::Value) {
        self.metadata.insert(key, value);
    }

    /// Check if this note is marked as a draft
    pub fn is_draft(&self) -> bool {
        self.draft.unwrap_or(false)
    }
}

impl Default for NoteFrontmatter {
    fn default() -> Self {
        Self::new()
    }
}

/// A wiki-style link between notes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WikiLink {
    /// Unique identifier for the link
    pub id: Uuid,
    /// Source note containing the link
    pub source_note_id: NoteId,
    /// Target note (None if not found)
    pub target_note_id: Option<NoteId>,
    /// Original link text as written [[Target]]
    pub link_text: String,
    /// Custom display text [[Target|Display]]
    pub display_text: Option<String>,
    /// Line number where the link appears
    pub line_number: usize,
    /// Type of link
    pub link_type: LinkType,
    /// Whether the target note exists
    pub is_valid: bool,
    /// When the link was created/discovered
    pub created_at: DateTime<Utc>,
    /// Last time the link was validated
    pub updated_at: DateTime<Utc>,
}

impl WikiLink {
    /// Create a new wiki link
    pub fn new(
        source_note_id: NoteId,
        link_text: String,
        line_number: usize,
        link_type: LinkType,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            source_note_id,
            target_note_id: None,
            link_text,
            display_text: None,
            line_number,
            link_type,
            is_valid: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a wiki link with display text
    pub fn with_display_text(
        source_note_id: NoteId,
        link_text: String,
        display_text: String,
        line_number: usize,
        link_type: LinkType,
    ) -> Self {
        let mut link = Self::new(source_note_id, link_text, line_number, link_type);
        link.display_text = Some(display_text);
        link
    }

    /// Get the text to display for this link
    pub fn display(&self) -> &str {
        self.display_text.as_ref().unwrap_or(&self.link_text)
    }

    /// Update the link's target
    pub fn set_target(&mut self, target_id: Option<NoteId>) {
        self.is_valid = target_id.is_some();
        self.target_note_id = target_id;
        self.updated_at = Utc::now();
    }

    /// Check if this is a broken link
    pub fn is_broken(&self) -> bool {
        !self.is_valid || self.target_note_id.is_none()
    }
}

/// Types of wiki links
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkType {
    /// Standard wiki link [[Target]]
    Wiki,
    /// Embed link ![[Target]]
    Embed,
    /// Tag reference #tag
    Tag,
    /// Block reference [[Target#^block]]
    Block,
}

impl LinkType {
    /// Get a string representation of the link type
    pub fn as_str(&self) -> &'static str {
        match self {
            LinkType::Wiki => "wiki",
            LinkType::Embed => "embed",
            LinkType::Tag => "tag",
            LinkType::Block => "block",
        }
    }
}

/// Search result for notes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoteSearchResult {
    /// The matching note
    pub note: Note,
    /// Search relevance score (0.0 to 1.0)
    pub score: f64,
    /// Text snippets showing matches
    pub snippets: Vec<String>,
    /// Which fields matched the search
    pub matched_fields: Vec<String>,
}

impl NoteSearchResult {
    /// Create a new search result
    pub fn new(note: Note, score: f64) -> Self {
        Self {
            note,
            score,
            snippets: Vec::new(),
            matched_fields: Vec::new(),
        }
    }

    /// Add a snippet showing a match
    pub fn add_snippet(&mut self, snippet: String) {
        self.snippets.push(snippet);
    }

    /// Add a matched field
    pub fn add_matched_field(&mut self, field: String) {
        if !self.matched_fields.contains(&field) {
            self.matched_fields.push(field);
        }
    }
}

/// Configuration for the notes plugin
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NotesConfig {
    /// Default directory for notes
    pub default_directory: PathBuf,
    /// List of watched directories
    pub watched_directories: Vec<WatchedDirectory>,
    /// Maximum search results to return
    pub max_search_results: usize,
    /// Whether to enable automatic indexing
    pub auto_index: bool,
    /// File patterns to ignore
    pub ignore_patterns: Vec<String>,
    /// Default template for new notes
    pub default_template: String,
    /// Whether to enable vim-style keybindings
    pub vim_mode: bool,
}

impl NotesConfig {
    /// Create a default configuration
    pub fn default() -> Self {
        Self {
            default_directory: PathBuf::from("~/notes"),
            watched_directories: Vec::new(),
            max_search_results: 100,
            auto_index: true,
            ignore_patterns: vec![
                "*.tmp".to_string(),
                "*.swp".to_string(),
                ".git/".to_string(),
                ".obsidian/".to_string(),
            ],
            default_template: "# {{title}}\n\nCreated: {{date}}\nTags: \n\n".to_string(),
            vim_mode: true,
        }
    }

    /// Create a test configuration
    #[cfg(test)]
    pub fn test_default() -> Self {
        Self {
            default_directory: PathBuf::from("/tmp/test_notes"),
            watched_directories: Vec::new(),
            max_search_results: 50,
            auto_index: true,
            ignore_patterns: vec!["*.tmp".to_string()],
            default_template: "# {{title}}\n\n".to_string(),
            vim_mode: false,
        }
    }
}

impl Default for NotesConfig {
    fn default() -> Self {
        NotesConfig::default()
    }
}

/// A directory being watched for notes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchedDirectory {
    /// Unique identifier
    pub id: i64,
    /// Absolute path to the directory
    pub path: PathBuf,
    /// User-friendly name
    pub name: String,
    /// Whether to scan subdirectories
    pub recursive: bool,
    /// Whether monitoring is enabled
    pub enabled: bool,
    /// When the directory was last scanned
    pub last_scan: Option<DateTime<Utc>>,
    /// Number of notes in this directory
    pub note_count: usize,
    /// Patterns to ignore in this directory
    pub ignore_patterns: Vec<String>,
    /// When the directory was added
    pub created_at: DateTime<Utc>,
    /// Last configuration update
    pub updated_at: DateTime<Utc>,
}

impl WatchedDirectory {
    /// Create a new watched directory
    pub fn new(path: PathBuf, name: String) -> Self {
        let now = Utc::now();
        Self {
            id: 0, // Will be set by database
            path,
            name,
            recursive: true,
            enabled: true,
            last_scan: None,
            note_count: 0,
            ignore_patterns: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Update the scan timestamp
    pub fn mark_scanned(&mut self, note_count: usize) {
        self.last_scan = Some(Utc::now());
        self.note_count = note_count;
        self.updated_at = Utc::now();
    }

    /// Check if a file should be ignored
    pub fn should_ignore(&self, file_path: &str) -> bool {
        for pattern in &self.ignore_patterns {
            if glob_match(pattern, file_path) {
                return true;
            }
        }
        false
    }
}

/// Simple glob pattern matching
fn glob_match(pattern: &str, text: &str) -> bool {
    // Simple implementation - would use a proper glob library in production
    if pattern.ends_with('*') {
        let prefix = &pattern[..pattern.len() - 1];
        text.starts_with(prefix)
    } else if pattern.starts_with('*') {
        let suffix = &pattern[1..];
        text.ends_with(suffix)
    } else if pattern.contains('/') {
        // Directory pattern - check if text contains the pattern
        text.contains(pattern)
    } else {
        pattern == text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_note_creation() {
        let note = Note::new(
            "test-note-1".to_string(),
            "Test Note".to_string(),
            "# Test Note\n\nThis is a test.".to_string(),
            PathBuf::from("/tmp/test.md"),
        );

        assert_eq!(note.id, "test-note-1");
        assert_eq!(note.title, "Test Note");
        assert_eq!(note.content, "# Test Note\n\nThis is a test.");
        assert_eq!(note.path, Path::new("/tmp/test.md"));
        assert!(!note.is_deleted);
        assert_eq!(note.word_count, 0); // Not calculated yet
        assert!(note.tags.is_empty());
        assert!(note.links.is_empty());
    }

    #[test]
    fn test_note_update_content() {
        let mut note = Note::new(
            "test-note-1".to_string(),
            "Test Note".to_string(),
            "Original content".to_string(),
            PathBuf::from("/tmp/test.md"),
        );

        let original_modified = note.modified_at;
        
        // Small delay to ensure timestamp changes
        std::thread::sleep(std::time::Duration::from_millis(1));
        
        note.update_content("Updated content".to_string());

        assert_eq!(note.content, "Updated content");
        assert!(note.modified_at > original_modified);
    }

    #[test]
    fn test_note_mark_deleted() {
        let mut note = Note::new(
            "test-note-1".to_string(),
            "Test Note".to_string(),
            "Content".to_string(),
            PathBuf::from("/tmp/test.md"),
        );

        assert!(!note.is_deleted);
        
        note.mark_deleted();
        
        assert!(note.is_deleted);
    }

    #[test]
    fn test_note_display_name() {
        // Note with title
        let note_with_title = Note::new(
            "test-1".to_string(),
            "My Great Note".to_string(),
            "Content".to_string(),
            PathBuf::from("/tmp/test.md"),
        );
        assert_eq!(note_with_title.display_name(), "My Great Note");

        // Note without title
        let note_without_title = Note::new(
            "test-2".to_string(),
            "".to_string(),
            "Content".to_string(),
            PathBuf::from("/tmp/my-file.md"),
        );
        assert_eq!(note_without_title.display_name(), "my-file");
    }

    #[test]
    fn test_note_has_tag() {
        let mut note = Note::new(
            "test-note-1".to_string(),
            "Test Note".to_string(),
            "Content".to_string(),
            PathBuf::from("/tmp/test.md"),
        );

        note.tags = vec!["rust".to_string(), "programming".to_string()];

        assert!(note.has_tag("rust"));
        assert!(note.has_tag("RUST")); // Case insensitive
        assert!(note.has_tag("programming"));
        assert!(!note.has_tag("javascript"));
    }

    #[test]
    fn test_frontmatter_creation() {
        let frontmatter = NoteFrontmatter::new();
        
        assert!(frontmatter.title.is_none());
        assert!(frontmatter.tags.is_empty());
        assert!(frontmatter.date.is_none());
        assert!(frontmatter.author.is_none());
        assert!(frontmatter.metadata.is_empty());
        assert!(!frontmatter.is_draft());
    }

    #[test]
    fn test_frontmatter_with_title() {
        let frontmatter = NoteFrontmatter::with_title("My Note".to_string());
        
        assert_eq!(frontmatter.title, Some("My Note".to_string()));
        assert!(frontmatter.tags.is_empty());
    }

    #[test]
    fn test_frontmatter_add_tags() {
        let mut frontmatter = NoteFrontmatter::new();
        
        frontmatter.add_tag("rust".to_string());
        frontmatter.add_tag("programming".to_string());
        frontmatter.add_tag("rust".to_string()); // Duplicate should be ignored
        
        assert_eq!(frontmatter.tags.len(), 2);
        assert!(frontmatter.tags.contains(&"rust".to_string()));
        assert!(frontmatter.tags.contains(&"programming".to_string()));
    }

    #[test]
    fn test_frontmatter_metadata() {
        let mut frontmatter = NoteFrontmatter::new();
        
        frontmatter.set_metadata(
            "custom_field".to_string(),
            serde_yaml::Value::String("custom_value".to_string()),
        );
        
        let value = frontmatter.get_metadata("custom_field");
        assert!(value.is_some());
        
        if let Some(serde_yaml::Value::String(s)) = value {
            assert_eq!(s, "custom_value");
        } else {
            panic!("Expected string value");
        }
    }

    #[test]
    fn test_wiki_link_creation() {
        let link = WikiLink::new(
            "source-note".to_string(),
            "Target Note".to_string(),
            5,
            LinkType::Wiki,
        );

        assert_eq!(link.source_note_id, "source-note");
        assert_eq!(link.link_text, "Target Note");
        assert_eq!(link.line_number, 5);
        assert_eq!(link.link_type, LinkType::Wiki);
        assert!(!link.is_valid);
        assert!(link.target_note_id.is_none());
        assert_eq!(link.display(), "Target Note");
    }

    #[test]
    fn test_wiki_link_with_display_text() {
        let link = WikiLink::with_display_text(
            "source-note".to_string(),
            "target-note-id".to_string(),
            "Custom Display".to_string(),
            3,
            LinkType::Wiki,
        );

        assert_eq!(link.link_text, "target-note-id");
        assert_eq!(link.display_text, Some("Custom Display".to_string()));
        assert_eq!(link.display(), "Custom Display");
    }

    #[test]
    fn test_wiki_link_set_target() {
        let mut link = WikiLink::new(
            "source-note".to_string(),
            "Target Note".to_string(),
            1,
            LinkType::Wiki,
        );

        assert!(!link.is_valid);
        assert!(link.is_broken());

        link.set_target(Some("target-note-id".to_string()));

        assert!(link.is_valid);
        assert!(!link.is_broken());
        assert_eq!(link.target_note_id, Some("target-note-id".to_string()));
    }

    #[test]
    fn test_link_type_as_str() {
        assert_eq!(LinkType::Wiki.as_str(), "wiki");
        assert_eq!(LinkType::Embed.as_str(), "embed");
        assert_eq!(LinkType::Tag.as_str(), "tag");
        assert_eq!(LinkType::Block.as_str(), "block");
    }

    #[test]
    fn test_search_result_creation() {
        let note = Note::new(
            "test-note".to_string(),
            "Test Note".to_string(),
            "Content".to_string(),
            PathBuf::from("/tmp/test.md"),
        );

        let mut result = NoteSearchResult::new(note, 0.85);
        result.add_snippet("This is a matching snippet".to_string());
        result.add_matched_field("title".to_string());
        result.add_matched_field("content".to_string());
        result.add_matched_field("title".to_string()); // Duplicate

        assert_eq!(result.score, 0.85);
        assert_eq!(result.snippets.len(), 1);
        assert_eq!(result.matched_fields.len(), 2); // No duplicates
        assert!(result.matched_fields.contains(&"title".to_string()));
        assert!(result.matched_fields.contains(&"content".to_string()));
    }

    #[test]
    fn test_notes_config_default() {
        let config = NotesConfig::default();
        
        assert_eq!(config.max_search_results, 100);
        assert!(config.auto_index);
        assert!(config.vim_mode);
        assert!(!config.ignore_patterns.is_empty());
        assert!(config.ignore_patterns.contains(&"*.tmp".to_string()));
    }

    #[test]
    fn test_watched_directory_creation() {
        let dir = WatchedDirectory::new(
            PathBuf::from("/home/user/notes"),
            "My Notes".to_string(),
        );

        assert_eq!(dir.path, Path::new("/home/user/notes"));
        assert_eq!(dir.name, "My Notes");
        assert!(dir.recursive);
        assert!(dir.enabled);
        assert!(dir.last_scan.is_none());
        assert_eq!(dir.note_count, 0);
    }

    #[test]
    fn test_watched_directory_mark_scanned() {
        let mut dir = WatchedDirectory::new(
            PathBuf::from("/home/user/notes"),
            "My Notes".to_string(),
        );

        assert!(dir.last_scan.is_none());
        assert_eq!(dir.note_count, 0);

        dir.mark_scanned(42);

        assert!(dir.last_scan.is_some());
        assert_eq!(dir.note_count, 42);
    }

    #[test]
    fn test_glob_match() {
        assert!(glob_match("*.md", "test.md"));
        assert!(glob_match("*.md", "document.md"));
        assert!(!glob_match("*.md", "test.txt"));
        
        assert!(glob_match("test*", "test.md"));
        assert!(glob_match("test*", "testing"));
        assert!(!glob_match("test*", "example.md"));
        
        assert!(glob_match("exact", "exact"));
        assert!(!glob_match("exact", "inexact"));
    }

    #[test]
    fn test_watched_directory_should_ignore() {
        let mut dir = WatchedDirectory::new(
            PathBuf::from("/test"),
            "Test".to_string(),
        );
        
        dir.ignore_patterns = vec!["*.tmp".to_string(), ".git/*".to_string()];

        assert!(dir.should_ignore("temp.tmp"));
        assert!(dir.should_ignore(".git/config"));
        assert!(!dir.should_ignore("note.md"));
    }
}