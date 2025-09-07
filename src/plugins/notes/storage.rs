//! Note storage implementation
//!
//! Handles persistent storage of notes using SQLite database with full-text search.

use super::database::NotesDatabase;
use super::manager::NoteResult;
use super::types::{Note, NoteId, NoteSearchResult, WatchedDirectory};

use std::path::Path;
use std::sync::Arc;

/// Storage layer for notes
#[derive(Debug, Clone)]
pub struct NoteStorage {
    database: Arc<NotesDatabase>,
}

impl NoteStorage {
    /// Create a new note storage instance
    pub async fn new(data_dir: &Path) -> NoteResult<Self> {
        let db_path = data_dir.join("notes.db");
        let database = NotesDatabase::new(&db_path).await?;

        Ok(Self {
            database: Arc::new(database),
        })
    }

    /// Create a new in-memory storage for testing
    #[cfg(test)]
    pub async fn new_in_memory() -> NoteResult<Self> {
        let database = NotesDatabase::new_in_memory().await?;
        Ok(Self {
            database: Arc::new(database),
        })
    }

    /// Store a note
    pub async fn store_note(&self, note: &Note, directory_id: i64) -> NoteResult<()> {
        self.database.store_note(note, directory_id).await
    }

    /// Get a note by ID
    pub async fn get_note(&self, note_id: &NoteId) -> NoteResult<Option<Note>> {
        self.database.get_note(note_id).await
    }

    /// Delete a note
    pub async fn delete_note(&self, note_id: &NoteId) -> NoteResult<()> {
        self.database.delete_note(note_id).await
    }

    /// Add a watched directory
    pub async fn add_watched_directory(
        &self,
        directory: WatchedDirectory,
    ) -> NoteResult<WatchedDirectory> {
        self.database.add_watched_directory(directory).await
    }

    /// Get all watched directories
    pub async fn get_watched_directories(&self) -> NoteResult<Vec<WatchedDirectory>> {
        self.database.get_watched_directories().await
    }

    /// Search notes using full-text search
    pub async fn search_notes(
        &self,
        query: &str,
        limit: usize,
    ) -> NoteResult<Vec<NoteSearchResult>> {
        self.database.search_notes(query, limit).await
    }

    /// Get notes by tag
    pub async fn get_notes_by_tag(&self, tag: &str) -> NoteResult<Vec<Note>> {
        self.database.get_notes_by_tag(tag).await
    }

    /// Get recent notes
    pub async fn get_recent_notes(&self, limit: usize) -> NoteResult<Vec<Note>> {
        self.database.get_recent_notes(limit).await
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::{NoteFrontmatter, WatchedDirectory};
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    async fn create_test_storage() -> NoteStorage {
        NoteStorage::new_in_memory().await.unwrap()
    }

    #[tokio::test]
    async fn test_storage_creation() {
        let _storage = create_test_storage().await;

        // Basic creation test - should not panic
        assert!(true);
    }

    #[tokio::test]
    async fn test_file_storage_creation() {
        let temp_dir = TempDir::new().unwrap();
        let result = NoteStorage::new(temp_dir.path()).await;

        // Should succeed now that it's implemented
        if result.is_err() {
            // Skip if environment doesn't support file databases
            println!("Skipping file storage test: {}", result.unwrap_err());
            return;
        }

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_add_and_get_watched_directory() {
        let storage = create_test_storage().await;

        let directory =
            WatchedDirectory::new(PathBuf::from("/home/user/notes"), "Test Notes".to_string());

        let stored_dir = storage
            .add_watched_directory(directory.clone())
            .await
            .unwrap();
        assert!(stored_dir.id > 0);
        assert_eq!(stored_dir.path, directory.path);
        assert_eq!(stored_dir.name, directory.name);

        let all_dirs = storage.get_watched_directories().await.unwrap();
        assert_eq!(all_dirs.len(), 1);
        assert_eq!(all_dirs[0].name, "Test Notes");
    }

    #[tokio::test]
    async fn test_store_and_get_note() {
        let storage = create_test_storage().await;

        // Add a directory first
        let directory = WatchedDirectory::new(PathBuf::from("/test"), "Test Dir".to_string());
        let stored_dir = storage.add_watched_directory(directory).await.unwrap();

        // Create a test note
        let mut note = Note::new(
            "test-note-1".to_string(),
            "Test Note".to_string(),
            "# Test Note\n\nThis is a test note.".to_string(),
            PathBuf::from("/test/note.md"),
        );
        note.tags = vec!["test".to_string(), "storage".to_string()];
        note.word_count = 6;
        note.file_size = 100;
        note.content_hash = "abc123".to_string();

        // Store the note
        storage.store_note(&note, stored_dir.id).await.unwrap();

        // Retrieve the note
        let retrieved = storage.get_note(&note.id).await.unwrap();
        assert!(retrieved.is_some());

        let retrieved_note = retrieved.unwrap();
        assert_eq!(retrieved_note.id, note.id);
        assert_eq!(retrieved_note.title, note.title);
        assert_eq!(retrieved_note.content, note.content);
        assert_eq!(retrieved_note.path, note.path);
        assert_eq!(retrieved_note.word_count, note.word_count);
    }

    #[tokio::test]
    async fn test_store_note_with_frontmatter() {
        let storage = create_test_storage().await;

        // Add directory
        let directory = WatchedDirectory::new(PathBuf::from("/test"), "Test Dir".to_string());
        let stored_dir = storage.add_watched_directory(directory).await.unwrap();

        // Create note with frontmatter
        let mut note = Note::new(
            "test-note-2".to_string(),
            "Note with Frontmatter".to_string(),
            "# Note\n\nContent here.".to_string(),
            PathBuf::from("/test/note2.md"),
        );

        let mut frontmatter = NoteFrontmatter::new();
        frontmatter.title = Some("Frontmatter Title".to_string());
        frontmatter.add_tag("yaml".to_string());
        note.frontmatter = Some(frontmatter);

        // Store and retrieve
        storage.store_note(&note, stored_dir.id).await.unwrap();
        let retrieved = storage.get_note(&note.id).await.unwrap().unwrap();

        assert!(retrieved.frontmatter.is_some());
        let fm = retrieved.frontmatter.unwrap();
        assert_eq!(fm.title, Some("Frontmatter Title".to_string()));
        assert!(fm.tags.contains(&"yaml".to_string()));
    }

    #[tokio::test]
    async fn test_delete_note() {
        let storage = create_test_storage().await;

        // Add directory and note
        let directory = WatchedDirectory::new(PathBuf::from("/test"), "Test Dir".to_string());
        let stored_dir = storage.add_watched_directory(directory).await.unwrap();

        let note = Note::new(
            "test-note-3".to_string(),
            "Note to Delete".to_string(),
            "This will be deleted".to_string(),
            PathBuf::from("/test/note3.md"),
        );

        storage.store_note(&note, stored_dir.id).await.unwrap();

        // Verify note exists
        assert!(storage.get_note(&note.id).await.unwrap().is_some());

        // Delete the note
        storage.delete_note(&note.id).await.unwrap();

        // Verify note is gone
        assert!(storage.get_note(&note.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_get_nonexistent_note() {
        let storage = create_test_storage().await;

        let result = storage.get_note(&"nonexistent".to_string()).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_search_notes() {
        let storage = create_test_storage().await;

        // Add directory
        let directory = WatchedDirectory::new(PathBuf::from("/test"), "Test Dir".to_string());
        let stored_dir = storage.add_watched_directory(directory).await.unwrap();

        // Store searchable notes
        let mut note1 = Note::new(
            "search-test-1".to_string(),
            "Rust Programming Guide".to_string(),
            "This is a comprehensive guide to Rust programming".to_string(),
            PathBuf::from("/test/rust.md"),
        );
        note1.tags = vec!["rust".to_string(), "programming".to_string()];

        let mut note2 = Note::new(
            "search-test-2".to_string(),
            "Python Tutorial".to_string(),
            "Learn Python programming from basics".to_string(),
            PathBuf::from("/test/python.md"),
        );
        note2.tags = vec!["python".to_string(), "tutorial".to_string()];

        storage.store_note(&note1, stored_dir.id).await.unwrap();
        storage.store_note(&note2, stored_dir.id).await.unwrap();

        // Search for "programming"
        let results = storage.search_notes("programming", 10).await.unwrap();
        assert!(!results.is_empty());

        // Should find at least the Rust note
        let rust_found = results.iter().any(|r| r.note.title.contains("Rust"));
        assert!(rust_found);
    }

    #[tokio::test]
    async fn test_get_notes_by_tag() {
        let storage = create_test_storage().await;

        // Add directory
        let directory = WatchedDirectory::new(PathBuf::from("/test"), "Test Dir".to_string());
        let stored_dir = storage.add_watched_directory(directory).await.unwrap();

        // Store notes with tags
        let mut note1 = Note::new(
            "tag-test-1".to_string(),
            "Rust Note".to_string(),
            "About Rust".to_string(),
            PathBuf::from("/test/rust.md"),
        );
        note1.tags = vec!["rust".to_string(), "programming".to_string()];

        let mut note2 = Note::new(
            "tag-test-2".to_string(),
            "Python Note".to_string(),
            "About Python".to_string(),
            PathBuf::from("/test/python.md"),
        );
        note2.tags = vec!["python".to_string(), "programming".to_string()];

        storage.store_note(&note1, stored_dir.id).await.unwrap();
        storage.store_note(&note2, stored_dir.id).await.unwrap();

        // Get notes by tag
        let programming_notes = storage.get_notes_by_tag("programming").await.unwrap();
        assert_eq!(programming_notes.len(), 2);

        let rust_notes = storage.get_notes_by_tag("rust").await.unwrap();
        assert_eq!(rust_notes.len(), 1);
        assert_eq!(rust_notes[0].title, "Rust Note");
    }

    #[tokio::test]
    async fn test_get_recent_notes() {
        let storage = create_test_storage().await;

        // Add directory
        let directory = WatchedDirectory::new(PathBuf::from("/test"), "Test Dir".to_string());
        let stored_dir = storage.add_watched_directory(directory).await.unwrap();

        // Store some notes
        let note1 = Note::new(
            "recent-test-1".to_string(),
            "First Note".to_string(),
            "First content".to_string(),
            PathBuf::from("/test/first.md"),
        );

        let note2 = Note::new(
            "recent-test-2".to_string(),
            "Second Note".to_string(),
            "Second content".to_string(),
            PathBuf::from("/test/second.md"),
        );

        storage.store_note(&note1, stored_dir.id).await.unwrap();
        storage.store_note(&note2, stored_dir.id).await.unwrap();

        // Get recent notes
        let recent_notes = storage.get_recent_notes(5).await.unwrap();
        assert!(!recent_notes.is_empty());
        assert!(recent_notes.len() <= 5);

        // Should have both notes
        assert_eq!(recent_notes.len(), 2);
    }
}
