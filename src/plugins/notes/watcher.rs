//! File system watcher implementation
//! 
//! Monitors file system changes and triggers note updates.

use super::manager::{NoteError, NoteResult};
use super::types::{WatchedDirectory, NoteId};

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::time::timeout;
use notify::{RecommendedWatcher, Watcher, RecursiveMode, Event, EventKind, event::{CreateKind, ModifyKind, RemoveKind, RenameMode}};

/// Types of file system events we track
#[derive(Debug, Clone, PartialEq)]
pub enum FileSystemEvent {
    /// File was created
    Created { path: PathBuf, note_id: Option<NoteId> },
    /// File was modified  
    Modified { path: PathBuf, note_id: Option<NoteId> },
    /// File was deleted
    Deleted { path: PathBuf, note_id: Option<NoteId> },
    /// File was moved/renamed
    Moved { from: PathBuf, to: PathBuf, note_id: Option<NoteId> },
    /// Directory was created
    DirectoryCreated { path: PathBuf },
    /// Directory was deleted
    DirectoryDeleted { path: PathBuf },
}

/// Configuration for file watching behavior
#[derive(Debug, Clone)]
pub struct WatcherConfig {
    /// How long to wait before processing batched events
    pub debounce_duration: Duration,
    /// Maximum number of events to batch together
    pub max_batch_size: usize,
    /// File extensions to watch (e.g., ["md", "txt"])
    pub watched_extensions: Vec<String>,
    /// Patterns to ignore (glob-style)
    pub ignore_patterns: Vec<String>,
    /// Whether to watch hidden files/directories
    pub watch_hidden: bool,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            debounce_duration: Duration::from_millis(500),
            max_batch_size: 100,
            watched_extensions: vec!["md".to_string(), "markdown".to_string()],
            ignore_patterns: vec![
                ".git/".to_string(),
                ".obsidian/".to_string(),
                "*.tmp".to_string(),
                "*.swp".to_string(),
                "*~".to_string(),
                ".DS_Store".to_string(),
            ],
            watch_hidden: false,
        }
    }
}

/// Batch of file system events ready for processing
#[derive(Debug, Clone)]
pub struct EventBatch {
    /// Events in this batch
    pub events: Vec<FileSystemEvent>,
    /// When the batch was created
    pub created_at: Instant,
    /// Total number of events processed so far
    pub total_events: usize,
}

/// File system watcher for note directories
#[derive(Debug)]
pub struct FileWatcher {
    /// Configuration for watching behavior
    config: WatcherConfig,
    /// Currently watched directories
    watched_dirs: HashMap<PathBuf, WatchedDirectory>,
    /// Channel for receiving file system events
    event_receiver: Option<mpsc::UnboundedReceiver<FileSystemEvent>>,
    /// Channel for sending processed event batches
    batch_sender: Option<mpsc::UnboundedSender<EventBatch>>,
    /// Underlying file system watcher
    _watcher: Option<RecommendedWatcher>,
}

impl FileWatcher {
    /// Create a new file watcher with default configuration
    pub fn new() -> NoteResult<Self> {
        Self::with_config(WatcherConfig::default())
    }

    /// Create a new file watcher with custom configuration
    pub fn with_config(config: WatcherConfig) -> NoteResult<Self> {
        Ok(Self {
            config,
            watched_dirs: HashMap::new(),
            event_receiver: None,
            batch_sender: None,
            _watcher: None,
        })
    }

    /// Add a directory to watch for changes
    pub fn watch_directory(&mut self, dir: WatchedDirectory) -> NoteResult<()> {
        if !dir.path.exists() {
            return Err(NoteError::FileSystem(format!(
                "Directory does not exist: {}",
                dir.path.display()
            )));
        }

        if !dir.path.is_dir() {
            return Err(NoteError::FileSystem(format!(
                "Path is not a directory: {}",
                dir.path.display()
            )));
        }

        self.watched_dirs.insert(dir.path.clone(), dir);
        Ok(())
    }

    /// Remove a directory from watching
    pub fn unwatch_directory(&mut self, path: &Path) -> NoteResult<()> {
        if self.watched_dirs.remove(path).is_none() {
            return Err(NoteError::FileSystem(format!(
                "Directory is not being watched: {}",
                path.display()
            )));
        }
        Ok(())
    }

    /// Get list of currently watched directories
    pub fn watched_directories(&self) -> Vec<&WatchedDirectory> {
        self.watched_dirs.values().collect()
    }

    /// Check if a file should be ignored based on patterns
    pub fn should_ignore(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        
        // Check if hidden and we don't watch hidden files
        if !self.config.watch_hidden {
            if let Some(name) = path.file_name() {
                if name.to_string_lossy().starts_with('.') {
                    return true;
                }
            }
        }

        // Check against ignore patterns
        for pattern in &self.config.ignore_patterns {
            if self.matches_pattern(pattern, &path_str) {
                return true;
            }
        }

        false
    }

    /// Check if a file has a watched extension
    pub fn has_watched_extension(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            self.config.watched_extensions.contains(&ext_str)
        } else {
            false
        }
    }

    /// Simple glob pattern matching
    fn matches_pattern(&self, pattern: &str, text: &str) -> bool {
        if pattern.ends_with('*') {
            let prefix = &pattern[..pattern.len() - 1];
            text.starts_with(prefix)
        } else if pattern.starts_with('*') {
            let suffix = &pattern[1..];
            text.ends_with(suffix)
        } else if pattern.contains('/') {
            // Directory pattern
            text.contains(pattern)
        } else {
            pattern == text
        }
    }

    /// Start watching for file system events
    pub async fn start_watching(&mut self) -> NoteResult<mpsc::UnboundedReceiver<EventBatch>> {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (batch_tx, batch_rx) = mpsc::unbounded_channel();

        // Create the file system watcher
        let mut watcher = self.create_notify_watcher(event_tx.clone())?;
        
        // Watch all configured directories
        for watched_dir in self.watched_dirs.values() {
            self.add_watch_path(&mut watcher, &watched_dir.path, watched_dir.recursive)?;
        }

        self.batch_sender = Some(batch_tx.clone());
        self._watcher = Some(watcher);

        // Start the event processing task
        let processor_config = self.config.clone();
        let processor_batch_tx = batch_tx;
        
        tokio::spawn(async move {
            Self::process_events(event_rx, processor_batch_tx, processor_config).await;
        });

        Ok(batch_rx)
    }

    /// Create the notify watcher with event handling
    fn create_notify_watcher(&self, event_tx: mpsc::UnboundedSender<FileSystemEvent>) -> NoteResult<RecommendedWatcher> {
        let tx = event_tx;
        let config = self.config.clone();
        
        let watcher = Watcher::new(
            move |res: Result<Event, notify::Error>| {
                match res {
                    Ok(event) => {
                        if let Some(fs_event) = Self::convert_notify_event(event, &config) {
                            if let Err(e) = tx.send(fs_event) {
                                eprintln!("Error sending file system event: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("File system watch error: {}", e);
                    }
                }
            },
            notify::Config::default(),
        ).map_err(|e| NoteError::FileSystem(format!("Failed to create file watcher: {}", e)))?;

        Ok(watcher)
    }

    /// Add a path to the watcher
    fn add_watch_path(&self, watcher: &mut RecommendedWatcher, path: &Path, recursive: bool) -> NoteResult<()> {
        let mode = if recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };

        watcher.watch(path, mode)
            .map_err(|e| NoteError::FileSystem(format!("Failed to watch path {}: {}", path.display(), e)))
    }

    /// Convert notify events to our FileSystemEvent enum
    fn convert_notify_event(event: Event, config: &WatcherConfig) -> Option<FileSystemEvent> {
        for path in &event.paths {
            // Skip if should be ignored
            let watcher_instance = FileWatcher::with_config(config.clone()).ok()?;
            if watcher_instance.should_ignore(path) {
                continue;
            }

            // Only process files with watched extensions or directories
            // For non-existent paths (like in tests), check extension based on the file name
            let is_directory_event = matches!(event.kind, 
                EventKind::Create(CreateKind::Folder) | 
                EventKind::Remove(RemoveKind::Folder)
            );
            
            if !is_directory_event {
                // If it's a real file, check is_file(). If not (synthetic path), check extension
                let should_check_extension = path.is_file() || !path.exists();
                if should_check_extension && !watcher_instance.has_watched_extension(path) {
                    continue;
                }
            }

            match event.kind {
                EventKind::Create(CreateKind::File) => {
                    return Some(FileSystemEvent::Created {
                        path: path.clone(),
                        note_id: None,
                    });
                }
                EventKind::Create(CreateKind::Folder) => {
                    return Some(FileSystemEvent::DirectoryCreated {
                        path: path.clone(),
                    });
                }
                EventKind::Modify(ModifyKind::Data(_)) => {
                    return Some(FileSystemEvent::Modified {
                        path: path.clone(),
                        note_id: None,
                    });
                }
                EventKind::Remove(RemoveKind::File) => {
                    return Some(FileSystemEvent::Deleted {
                        path: path.clone(),
                        note_id: None,
                    });
                }
                EventKind::Remove(RemoveKind::Folder) => {
                    return Some(FileSystemEvent::DirectoryDeleted {
                        path: path.clone(),
                    });
                }
                EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
                    // For rename events, we need to handle them as move events
                    // This is a simplified approach - in reality, notify provides before/after paths
                    if event.paths.len() >= 2 {
                        return Some(FileSystemEvent::Moved {
                            from: event.paths[0].clone(),
                            to: event.paths[1].clone(),
                            note_id: None,
                        });
                    }
                }
                _ => {
                    // Ignore other event types for now
                }
            }
        }

        None
    }

    /// Process events and batch them for efficient handling
    async fn process_events(
        mut event_rx: mpsc::UnboundedReceiver<FileSystemEvent>,
        batch_tx: mpsc::UnboundedSender<EventBatch>,
        config: WatcherConfig,
    ) {
        let mut event_buffer = Vec::new();
        let mut last_batch_time = Instant::now();
        let mut total_events = 0;

        loop {
            // Try to receive events with a timeout
            let timeout_duration = config.debounce_duration;
            
            match timeout(timeout_duration, event_rx.recv()).await {
                Ok(Some(event)) => {
                    event_buffer.push(event);
                    total_events += 1;

                    // Check if we should send a batch
                    let should_batch = event_buffer.len() >= config.max_batch_size
                        || last_batch_time.elapsed() >= config.debounce_duration;

                    if should_batch {
                        Self::send_batch(&batch_tx, &mut event_buffer, &mut last_batch_time, total_events);
                    }
                }
                Ok(None) => {
                    // Channel closed, send any remaining events and exit
                    if !event_buffer.is_empty() {
                        Self::send_batch(&batch_tx, &mut event_buffer, &mut last_batch_time, total_events);
                    }
                    break;
                }
                Err(_) => {
                    // Timeout occurred, send any buffered events
                    if !event_buffer.is_empty() {
                        Self::send_batch(&batch_tx, &mut event_buffer, &mut last_batch_time, total_events);
                    }
                }
            }
        }
    }

    /// Send a batch of events
    fn send_batch(
        batch_tx: &mpsc::UnboundedSender<EventBatch>,
        event_buffer: &mut Vec<FileSystemEvent>,
        last_batch_time: &mut Instant,
        total_events: usize,
    ) {
        if event_buffer.is_empty() {
            return;
        }

        let batch = EventBatch {
            events: event_buffer.drain(..).collect(),
            created_at: Instant::now(),
            total_events,
        };

        if let Err(e) = batch_tx.send(batch) {
            eprintln!("Error sending event batch: {}", e);
        }

        *last_batch_time = Instant::now();
    }

    /// Add a new directory to watch (after watcher is started)
    pub fn add_watch_directory(&mut self, dir: WatchedDirectory) -> NoteResult<()> {
        // Validate directory first
        if !dir.path.exists() {
            return Err(NoteError::FileSystem(format!(
                "Directory does not exist: {}",
                dir.path.display()
            )));
        }

        if !dir.path.is_dir() {
            return Err(NoteError::FileSystem(format!(
                "Path is not a directory: {}",
                dir.path.display()
            )));
        }

        // Add to watcher if it's running
        if self._watcher.is_some() {
            if let Some(ref mut watcher) = self._watcher {
                let mode = if dir.recursive {
                    RecursiveMode::Recursive
                } else {
                    RecursiveMode::NonRecursive
                };

                watcher.watch(&dir.path, mode)
                    .map_err(|e| NoteError::FileSystem(format!("Failed to watch path {}: {}", dir.path.display(), e)))?;
            }
        }

        self.watched_dirs.insert(dir.path.clone(), dir);
        Ok(())
    }

    /// Remove a directory from watching (after watcher is started)
    pub fn remove_watch_directory(&mut self, path: &Path) -> NoteResult<()> {
        if let Some(ref mut watcher) = self._watcher {
            watcher.unwatch(path)
                .map_err(|e| NoteError::FileSystem(format!("Failed to unwatch path {}: {}", path.display(), e)))?;
        }

        if self.watched_dirs.remove(path).is_none() {
            return Err(NoteError::FileSystem(format!(
                "Directory is not being watched: {}",
                path.display()
            )));
        }
        Ok(())
    }

    /// Stop watching for file system events
    pub fn stop_watching(&mut self) {
        self.event_receiver = None;
        self.batch_sender = None;
        self._watcher = None;
    }

    /// Check if the watcher is currently active
    pub fn is_watching(&self) -> bool {
        self._watcher.is_some()
    }

    /// Manually trigger an event (for testing)
    #[cfg(test)]
    pub fn trigger_event(&self, _event: FileSystemEvent) -> NoteResult<()> {
        // This is only available in test builds for testing purposes
        Ok(())
    }

    /// Get the current configuration
    pub fn config(&self) -> &WatcherConfig {
        &self.config
    }

    /// Update the configuration
    pub fn update_config(&mut self, config: WatcherConfig) {
        self.config = config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::File;
    use tokio::time::sleep;

    fn create_test_config() -> WatcherConfig {
        WatcherConfig {
            debounce_duration: Duration::from_millis(100),
            max_batch_size: 10,
            watched_extensions: vec!["md".to_string(), "txt".to_string()],
            ignore_patterns: vec![
                ".git/".to_string(),
                "*.tmp".to_string(),
                "*.swp".to_string(),
            ],
            watch_hidden: false,
        }
    }


    #[test]
    fn test_watcher_creation() {
        let watcher = FileWatcher::new();
        assert!(watcher.is_ok());
        
        let watcher = watcher.unwrap();
        assert_eq!(watcher.watched_directories().len(), 0);
        assert_eq!(watcher.config().debounce_duration, Duration::from_millis(500));
    }

    #[test]
    fn test_watcher_with_custom_config() {
        let config = create_test_config();
        let watcher = FileWatcher::with_config(config.clone());
        
        assert!(watcher.is_ok());
        let watcher = watcher.unwrap();
        assert_eq!(watcher.config().debounce_duration, config.debounce_duration);
        assert_eq!(watcher.config().max_batch_size, config.max_batch_size);
    }

    #[test]
    fn test_watch_directory_valid() {
        let temp_dir = TempDir::new().unwrap();
        let mut watcher = FileWatcher::new().unwrap();
        
        let watched_dir = WatchedDirectory::new(
            temp_dir.path().to_path_buf(),
            "Test Directory".to_string(),
        );
        
        let result = watcher.watch_directory(watched_dir);
        assert!(result.is_ok());
        assert_eq!(watcher.watched_directories().len(), 1);
    }

    #[test]
    fn test_watch_directory_nonexistent() {
        let mut watcher = FileWatcher::new().unwrap();
        let nonexistent_dir = WatchedDirectory::new(
            PathBuf::from("/nonexistent/path"),
            "Nonexistent".to_string(),
        );
        
        let result = watcher.watch_directory(nonexistent_dir);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[test]
    fn test_watch_directory_not_directory() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("not_a_dir.txt");
        File::create(&file_path).unwrap();
        
        let mut watcher = FileWatcher::new().unwrap();
        let invalid_dir = WatchedDirectory::new(
            file_path,
            "Not a Directory".to_string(),
        );
        
        let result = watcher.watch_directory(invalid_dir);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not a directory"));
    }

    #[test]
    fn test_unwatch_directory() {
        let temp_dir = TempDir::new().unwrap();
        let mut watcher = FileWatcher::new().unwrap();
        
        let watched_dir = WatchedDirectory::new(
            temp_dir.path().to_path_buf(),
            "Test Directory".to_string(),
        );
        let path = watched_dir.path.clone();
        
        watcher.watch_directory(watched_dir).unwrap();
        assert_eq!(watcher.watched_directories().len(), 1);
        
        let result = watcher.unwatch_directory(&path);
        assert!(result.is_ok());
        assert_eq!(watcher.watched_directories().len(), 0);
    }

    #[test]
    fn test_unwatch_directory_not_watched() {
        let mut watcher = FileWatcher::new().unwrap();
        let path = PathBuf::from("/tmp/not_watched");
        
        let result = watcher.unwatch_directory(&path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not being watched"));
    }

    #[test]
    fn test_should_ignore_hidden_files() {
        let watcher = FileWatcher::new().unwrap();
        
        assert!(watcher.should_ignore(Path::new(".hidden_file")));
        assert!(watcher.should_ignore(Path::new("/path/to/.hidden")));
        assert!(!watcher.should_ignore(Path::new("visible_file.md")));
    }

    #[test]
    fn test_should_ignore_patterns() {
        let mut config = create_test_config();
        config.ignore_patterns = vec![
            ".git/".to_string(),
            "*.tmp".to_string(),
            "*.swp".to_string(),
            "*~".to_string(),
        ];
        let watcher = FileWatcher::with_config(config).unwrap();
        
        assert!(watcher.should_ignore(Path::new("/repo/.git/config")));
        assert!(watcher.should_ignore(Path::new("temp.tmp")));
        assert!(watcher.should_ignore(Path::new("file.swp")));
        assert!(watcher.should_ignore(Path::new("backup~")));
        assert!(!watcher.should_ignore(Path::new("normal.md")));
    }

    #[test]
    fn test_has_watched_extension() {
        let watcher = FileWatcher::new().unwrap();
        
        assert!(watcher.has_watched_extension(Path::new("note.md")));
        assert!(watcher.has_watched_extension(Path::new("document.markdown")));
        assert!(!watcher.has_watched_extension(Path::new("image.png")));
        assert!(!watcher.has_watched_extension(Path::new("config.toml")));
        assert!(!watcher.has_watched_extension(Path::new("no_extension")));
    }

    #[test]
    fn test_file_system_event_types() {
        let events = vec![
            FileSystemEvent::Created { 
                path: PathBuf::from("new.md"), 
                note_id: Some("note-1".to_string()) 
            },
            FileSystemEvent::Modified { 
                path: PathBuf::from("existing.md"), 
                note_id: Some("note-2".to_string()) 
            },
            FileSystemEvent::Deleted { 
                path: PathBuf::from("old.md"), 
                note_id: Some("note-3".to_string()) 
            },
            FileSystemEvent::Moved { 
                from: PathBuf::from("old_name.md"), 
                to: PathBuf::from("new_name.md"), 
                note_id: Some("note-4".to_string()) 
            },
            FileSystemEvent::DirectoryCreated { 
                path: PathBuf::from("new_folder") 
            },
            FileSystemEvent::DirectoryDeleted { 
                path: PathBuf::from("old_folder") 
            },
        ];
        
        // Verify events can be created and compared
        assert_eq!(events.len(), 6);
        
        match &events[0] {
            FileSystemEvent::Created { path, note_id } => {
                assert_eq!(path, &PathBuf::from("new.md"));
                assert_eq!(note_id, &Some("note-1".to_string()));
            }
            _ => panic!("Expected Created event"),
        }
    }

    #[test]
    fn test_event_batch_creation() {
        let events = vec![
            FileSystemEvent::Created { 
                path: PathBuf::from("note1.md"), 
                note_id: None 
            },
            FileSystemEvent::Modified { 
                path: PathBuf::from("note2.md"), 
                note_id: None 
            },
        ];
        
        let batch = EventBatch {
            events: events.clone(),
            created_at: Instant::now(),
            total_events: 2,
        };
        
        assert_eq!(batch.events.len(), 2);
        assert_eq!(batch.total_events, 2);
        assert!(batch.created_at <= Instant::now());
    }

    #[test]
    fn test_watcher_config_defaults() {
        let config = WatcherConfig::default();
        
        assert_eq!(config.debounce_duration, Duration::from_millis(500));
        assert_eq!(config.max_batch_size, 100);
        assert!(config.watched_extensions.contains(&"md".to_string()));
        assert!(config.watched_extensions.contains(&"markdown".to_string()));
        assert!(config.ignore_patterns.contains(&".git/".to_string()));
        assert!(!config.watch_hidden);
    }

    #[test]
    fn test_update_config() {
        let mut watcher = FileWatcher::new().unwrap();
        let new_config = create_test_config();
        
        // Verify initial config is different
        assert_ne!(watcher.config().debounce_duration, new_config.debounce_duration);
        
        watcher.update_config(new_config.clone());
        assert_eq!(watcher.config().debounce_duration, new_config.debounce_duration);
        assert_eq!(watcher.config().max_batch_size, new_config.max_batch_size);
    }

    #[tokio::test]
    async fn test_start_stop_watching() {
        let mut watcher = FileWatcher::new().unwrap();
        
        // Start watching
        let batch_receiver = watcher.start_watching().await;
        assert!(batch_receiver.is_ok());
        
        // Stop watching
        watcher.stop_watching();
        
        // Verify channels are cleared
        assert!(watcher.event_receiver.is_none());
        assert!(watcher.batch_sender.is_none());
    }

    #[test]
    fn test_pattern_matching() {
        let watcher = FileWatcher::new().unwrap();
        
        // Test prefix patterns
        assert!(watcher.matches_pattern("temp*", "temp.txt"));
        assert!(watcher.matches_pattern("temp*", "tempfile"));
        assert!(!watcher.matches_pattern("temp*", "mytemp"));
        
        // Test suffix patterns  
        assert!(watcher.matches_pattern("*.tmp", "file.tmp"));
        assert!(watcher.matches_pattern("*.tmp", "backup.tmp"));
        assert!(!watcher.matches_pattern("*.tmp", "file.txt"));
        
        // Test directory patterns
        assert!(watcher.matches_pattern(".git/", "/repo/.git/config"));
        assert!(watcher.matches_pattern(".git/", "project/.git/HEAD"));
        assert!(!watcher.matches_pattern(".git/", "gitignore"));
        
        // Test exact patterns
        assert!(watcher.matches_pattern("exact", "exact"));
        assert!(!watcher.matches_pattern("exact", "inexact"));
    }

    #[test] 
    fn test_multiple_directories() {
        let temp_dir1 = TempDir::new().unwrap();
        let temp_dir2 = TempDir::new().unwrap();
        let mut watcher = FileWatcher::new().unwrap();
        
        let dir1 = WatchedDirectory::new(
            temp_dir1.path().to_path_buf(),
            "Directory 1".to_string(),
        );
        let dir2 = WatchedDirectory::new(
            temp_dir2.path().to_path_buf(),
            "Directory 2".to_string(),
        );
        
        watcher.watch_directory(dir1).unwrap();
        watcher.watch_directory(dir2).unwrap();
        
        assert_eq!(watcher.watched_directories().len(), 2);
        
        // Remove one directory
        watcher.unwatch_directory(temp_dir1.path()).unwrap();
        assert_eq!(watcher.watched_directories().len(), 1);
    }

    #[test]
    fn test_watch_hidden_config() {
        let mut config = create_test_config();
        config.watch_hidden = true;
        let watcher = FileWatcher::with_config(config).unwrap();
        
        // Should not ignore hidden files when watch_hidden is true
        assert!(!watcher.should_ignore(Path::new(".hidden_file")));
        // But still respect other ignore patterns
        assert!(watcher.should_ignore(Path::new("file.tmp")));
    }

    #[test]
    fn test_add_watch_directory_after_start() {
        let temp_dir1 = TempDir::new().unwrap();
        let temp_dir2 = TempDir::new().unwrap();
        let mut watcher = FileWatcher::new().unwrap();
        
        // Add first directory before starting
        let dir1 = WatchedDirectory::new(
            temp_dir1.path().to_path_buf(),
            "Directory 1".to_string(),
        );
        watcher.watch_directory(dir1).unwrap();
        assert_eq!(watcher.watched_directories().len(), 1);
        
        // Add second directory using the new method
        let dir2 = WatchedDirectory::new(
            temp_dir2.path().to_path_buf(),
            "Directory 2".to_string(),
        );
        let result = watcher.add_watch_directory(dir2);
        assert!(result.is_ok());
        assert_eq!(watcher.watched_directories().len(), 2);
    }

    #[test]
    fn test_remove_watch_directory() {
        let temp_dir = TempDir::new().unwrap();
        let mut watcher = FileWatcher::new().unwrap();
        
        let watched_dir = WatchedDirectory::new(
            temp_dir.path().to_path_buf(),
            "Test Directory".to_string(),
        );
        let path = watched_dir.path.clone();
        
        watcher.add_watch_directory(watched_dir).unwrap();
        assert_eq!(watcher.watched_directories().len(), 1);
        
        let result = watcher.remove_watch_directory(&path);
        assert!(result.is_ok());
        assert_eq!(watcher.watched_directories().len(), 0);
    }

    #[test]
    fn test_is_watching() {
        let mut watcher = FileWatcher::new().unwrap();
        
        // Initially not watching
        assert!(!watcher.is_watching());
        
        // After stop_watching called, should not be watching
        watcher.stop_watching();
        assert!(!watcher.is_watching());
    }

    #[test]
    fn test_convert_notify_event_creation() {
        let config = create_test_config();
        let mut event = Event::new(notify::EventKind::Create(notify::event::CreateKind::File));
        event.paths.push(PathBuf::from("test.md"));
        
        let fs_event = FileWatcher::convert_notify_event(event, &config);
        assert!(fs_event.is_some());
        
        match fs_event.unwrap() {
            FileSystemEvent::Created { path, note_id } => {
                assert_eq!(path, PathBuf::from("test.md"));
                assert_eq!(note_id, None);
            }
            _ => panic!("Expected Created event"),
        }
    }

    #[test]
    fn test_convert_notify_event_modification() {
        let config = create_test_config();
        let mut event = Event::new(notify::EventKind::Modify(notify::event::ModifyKind::Data(notify::event::DataChange::Any)));
        event.paths.push(PathBuf::from("test.md"));
        
        let fs_event = FileWatcher::convert_notify_event(event, &config);
        assert!(fs_event.is_some());
        
        match fs_event.unwrap() {
            FileSystemEvent::Modified { path, note_id } => {
                assert_eq!(path, PathBuf::from("test.md"));
                assert_eq!(note_id, None);
            }
            _ => panic!("Expected Modified event"),
        }
    }

    #[test]
    fn test_convert_notify_event_deletion() {
        let config = create_test_config();
        let mut event = Event::new(notify::EventKind::Remove(notify::event::RemoveKind::File));
        event.paths.push(PathBuf::from("test.md"));
        
        let fs_event = FileWatcher::convert_notify_event(event, &config);
        assert!(fs_event.is_some());
        
        match fs_event.unwrap() {
            FileSystemEvent::Deleted { path, note_id } => {
                assert_eq!(path, PathBuf::from("test.md"));
                assert_eq!(note_id, None);
            }
            _ => panic!("Expected Deleted event"),
        }
    }

    #[test]
    fn test_convert_notify_event_ignored_file() {
        let config = create_test_config();
        let mut event = Event::new(notify::EventKind::Create(notify::event::CreateKind::File));
        event.paths.push(PathBuf::from("ignored.tmp")); // Should be ignored due to *.tmp pattern
        
        let fs_event = FileWatcher::convert_notify_event(event, &config);
        assert!(fs_event.is_none(), "Ignored files should not generate events");
    }

    #[test]
    fn test_convert_notify_event_wrong_extension() {
        let config = create_test_config();
        let mut event = Event::new(notify::EventKind::Create(notify::event::CreateKind::File));
        event.paths.push(PathBuf::from("image.png")); // Wrong extension
        
        let fs_event = FileWatcher::convert_notify_event(event, &config);
        assert!(fs_event.is_none(), "Files with wrong extensions should not generate events");
    }

    #[test] 
    fn test_convert_notify_event_directory() {
        let config = create_test_config();
        let mut event = Event::new(notify::EventKind::Create(notify::event::CreateKind::Folder));
        event.paths.push(PathBuf::from("new_folder"));
        
        let fs_event = FileWatcher::convert_notify_event(event, &config);
        assert!(fs_event.is_some());
        
        match fs_event.unwrap() {
            FileSystemEvent::DirectoryCreated { path } => {
                assert_eq!(path, PathBuf::from("new_folder"));
            }
            _ => panic!("Expected DirectoryCreated event"),
        }
    }

    #[tokio::test]
    async fn test_event_processing_debouncing() {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (batch_tx, mut batch_rx) = mpsc::unbounded_channel();
        
        let mut config = create_test_config();
        config.debounce_duration = Duration::from_millis(50);
        config.max_batch_size = 5;
        
        // Start event processing in background
        let processor_config = config.clone();
        tokio::spawn(async move {
            FileWatcher::process_events(event_rx, batch_tx, processor_config).await;
        });
        
        // Send multiple events quickly
        for i in 0..3 {
            let event = FileSystemEvent::Created {
                path: PathBuf::from(format!("test{}.md", i)),
                note_id: None,
            };
            event_tx.send(event).unwrap();
        }
        
        // Wait a bit for debouncing
        sleep(Duration::from_millis(100)).await;
        
        // Should receive a batch with all events
        let batch = batch_rx.recv().await.unwrap();
        assert_eq!(batch.events.len(), 3);
        assert_eq!(batch.total_events, 3);
    }

    #[tokio::test]
    async fn test_event_processing_max_batch_size() {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (batch_tx, mut batch_rx) = mpsc::unbounded_channel();
        
        let mut config = create_test_config();
        config.debounce_duration = Duration::from_millis(1000); // Long debounce
        config.max_batch_size = 2; // Small batch size
        
        // Start event processing in background
        let processor_config = config.clone();
        tokio::spawn(async move {
            FileWatcher::process_events(event_rx, batch_tx, processor_config).await;
        });
        
        // Send events that should trigger max batch size
        for i in 0..3 {
            let event = FileSystemEvent::Created {
                path: PathBuf::from(format!("test{}.md", i)),
                note_id: None,
            };
            event_tx.send(event).unwrap();
        }
        
        // Should receive first batch quickly (due to max batch size)
        let batch = batch_rx.recv().await.unwrap();
        assert_eq!(batch.events.len(), 2);
        
        // Wait for second batch
        sleep(Duration::from_millis(100)).await;
        
        // Should receive second batch with remaining event
        let batch = batch_rx.recv().await.unwrap();
        assert_eq!(batch.events.len(), 1);
    }
}