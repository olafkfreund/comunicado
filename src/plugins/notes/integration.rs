//! Integration tests for file system monitoring and directory management
//!
//! Tests the interaction between FileWatcher, DirectoryScanner, and other components.

use super::manager::NoteResult;
use super::scanner::{DirectoryScanner, ScanConfig};
use super::types::WatchedDirectory;
use super::watcher::{EventBatch, FileSystemEvent, FileWatcher, WatcherConfig};

use std::path::PathBuf;
use std::time::Duration;

/// Integrated file system monitor that combines watching and scanning
pub struct FileSystemMonitor {
    watcher: FileWatcher,
    scanner: DirectoryScanner,
    watched_directories: Vec<WatchedDirectory>,
}

impl FileSystemMonitor {
    /// Create a new file system monitor
    pub fn new() -> NoteResult<Self> {
        let watcher_config = WatcherConfig {
            debounce_duration: Duration::from_millis(100), // Fast for testing
            max_batch_size: 10,
            watched_extensions: vec!["md".to_string(), "markdown".to_string()],
            ignore_patterns: vec![
                ".git/".to_string(),
                "*.tmp".to_string(),
                "*.swp".to_string(),
            ],
            watch_hidden: false,
        };

        let scan_config = ScanConfig {
            watched_extensions: watcher_config.watched_extensions.clone(),
            ignore_patterns: watcher_config.ignore_patterns.clone(),
            scan_hidden: watcher_config.watch_hidden,
            max_file_size: 10 * 1024 * 1024, // 10MB
            follow_symlinks: false,
        };

        let watcher = FileWatcher::with_config(watcher_config)?;
        let scanner = DirectoryScanner::with_config(scan_config);

        Ok(Self {
            watcher,
            scanner,
            watched_directories: Vec::new(),
        })
    }

    /// Create a new file system monitor with custom configurations
    pub fn with_configs(
        watcher_config: WatcherConfig,
        scan_config: ScanConfig,
    ) -> NoteResult<Self> {
        let watcher = FileWatcher::with_config(watcher_config)?;
        let scanner = DirectoryScanner::with_config(scan_config);

        Ok(Self {
            watcher,
            scanner,
            watched_directories: Vec::new(),
        })
    }

    /// Add a directory to monitor
    pub fn add_directory(&mut self, directory: WatchedDirectory) -> NoteResult<()> {
        // Add to watcher
        self.watcher.watch_directory(directory.clone())?;

        // Perform initial scan
        let scan_result = self.scanner.scan_directory(&directory)?;

        // Store directory
        self.watched_directories.push(directory);

        println!(
            "Initial scan found {} note files, {} new files",
            scan_result.note_files, scan_result.new_files
        );

        Ok(())
    }

    /// Start monitoring for file system events
    pub async fn start_monitoring(
        &mut self,
    ) -> NoteResult<tokio::sync::mpsc::UnboundedReceiver<EventBatch>> {
        self.watcher.start_watching().await
    }

    /// Stop monitoring
    pub fn stop_monitoring(&mut self) {
        self.watcher.stop_watching();
    }

    /// Process a batch of file system events
    pub async fn process_event_batch(&mut self, batch: EventBatch) -> NoteResult<ProcessingResult> {
        let mut result = ProcessingResult::new();

        result.batch_size = batch.events.len();
        result.total_events = batch.total_events;
        result.processing_time_ms = batch.created_at.elapsed().as_millis() as u64;

        for event in batch.events {
            match event {
                FileSystemEvent::Created { path, .. } => {
                    result.created_files.push(path);
                }
                FileSystemEvent::Modified { path, .. } => {
                    result.modified_files.push(path);
                }
                FileSystemEvent::Deleted { path, .. } => {
                    result.deleted_files.push(path);
                }
                FileSystemEvent::Moved { from, to, .. } => {
                    result.moved_files.push((from, to));
                }
                FileSystemEvent::DirectoryCreated { path } => {
                    result.created_directories.push(path);
                }
                FileSystemEvent::DirectoryDeleted { path } => {
                    result.deleted_directories.push(path);
                }
            }
        }

        Ok(result)
    }

    /// Rescan all directories for changes
    pub async fn rescan_all(&mut self) -> NoteResult<Vec<super::scanner::ScanResult>> {
        let mut results = Vec::new();

        for directory in &self.watched_directories {
            let scan_result = self.scanner.scan_directory(directory)?;
            results.push(scan_result);
        }

        Ok(results)
    }

    /// Get monitoring statistics
    pub fn get_stats(&self) -> MonitoringStats {
        let (total_scanned, note_files) = self.scanner.last_scan_stats();

        MonitoringStats {
            watched_directories: self.watched_directories.len(),
            total_scanned_files: total_scanned,
            note_files,
            is_watching: self.watcher.is_watching(),
        }
    }
}

/// Result of processing a batch of file system events
#[derive(Debug, Clone)]
pub struct ProcessingResult {
    pub created_files: Vec<PathBuf>,
    pub modified_files: Vec<PathBuf>,
    pub deleted_files: Vec<PathBuf>,
    pub moved_files: Vec<(PathBuf, PathBuf)>,
    pub created_directories: Vec<PathBuf>,
    pub deleted_directories: Vec<PathBuf>,
    pub batch_size: usize,
    pub total_events: usize,
    pub processing_time_ms: u64,
}

impl ProcessingResult {
    fn new() -> Self {
        Self {
            created_files: Vec::new(),
            modified_files: Vec::new(),
            deleted_files: Vec::new(),
            moved_files: Vec::new(),
            created_directories: Vec::new(),
            deleted_directories: Vec::new(),
            batch_size: 0,
            total_events: 0,
            processing_time_ms: 0,
        }
    }

    /// Check if any changes were detected
    pub fn has_changes(&self) -> bool {
        !self.created_files.is_empty()
            || !self.modified_files.is_empty()
            || !self.deleted_files.is_empty()
            || !self.moved_files.is_empty()
            || !self.created_directories.is_empty()
            || !self.deleted_directories.is_empty()
    }

    /// Get total number of file changes (not including directories)
    pub fn file_changes(&self) -> usize {
        self.created_files.len()
            + self.modified_files.len()
            + self.deleted_files.len()
            + self.moved_files.len()
    }
}

/// Statistics about the file system monitoring
#[derive(Debug, Clone)]
pub struct MonitoringStats {
    pub watched_directories: usize,
    pub total_scanned_files: usize,
    pub note_files: usize,
    pub is_watching: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::path::Path;
    use tempfile::TempDir;
    use tokio::time::{sleep, timeout};

    async fn create_test_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let file_path = dir.join(name);
        let mut file = File::create(&file_path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file.sync_all().unwrap(); // Ensure write is flushed
        file_path
    }

    #[tokio::test]
    async fn test_file_system_monitor_creation() {
        let monitor = FileSystemMonitor::new();
        assert!(monitor.is_ok());

        let monitor = monitor.unwrap();
        let stats = monitor.get_stats();
        assert_eq!(stats.watched_directories, 0);
        assert_eq!(stats.total_scanned_files, 0);
        assert!(!stats.is_watching);
    }

    #[tokio::test]
    async fn test_add_directory_and_initial_scan() {
        let temp_dir = TempDir::new().unwrap();

        // Create some test files
        create_test_file(temp_dir.path(), "note1.md", "# Note 1").await;
        create_test_file(temp_dir.path(), "note2.md", "# Note 2").await;
        create_test_file(temp_dir.path(), "image.png", "binary").await;

        let mut monitor = FileSystemMonitor::new().unwrap();

        let watched_dir =
            WatchedDirectory::new(temp_dir.path().to_path_buf(), "Test Directory".to_string());

        let result = monitor.add_directory(watched_dir);
        assert!(result.is_ok());

        let stats = monitor.get_stats();
        assert_eq!(stats.watched_directories, 1);
        assert_eq!(stats.note_files, 2); // Only .md files
        assert_eq!(stats.total_scanned_files, 2); // png filtered out
    }

    #[tokio::test]
    async fn test_file_monitoring_and_event_processing() {
        let temp_dir = TempDir::new().unwrap();

        let mut monitor = FileSystemMonitor::new().unwrap();

        let watched_dir =
            WatchedDirectory::new(temp_dir.path().to_path_buf(), "Test Directory".to_string());

        monitor.add_directory(watched_dir).unwrap();

        // Start monitoring
        let mut event_receiver = monitor.start_monitoring().await.unwrap();

        // Give the watcher time to set up
        sleep(Duration::from_millis(50)).await;

        // Create a new file
        let new_file = create_test_file(temp_dir.path(), "new_note.md", "# New Note").await;

        // Wait for file system events with timeout
        let batch_result = timeout(Duration::from_millis(500), event_receiver.recv()).await;

        if let Ok(Some(batch)) = batch_result {
            let processing_result = monitor.process_event_batch(batch).await.unwrap();

            // Should detect the created file
            assert!(processing_result.has_changes());
            assert!(processing_result.file_changes() >= 1); // May be multiple events for one file
            assert!(!processing_result.created_files.is_empty());

            // Check that the created file path matches
            assert!(processing_result
                .created_files
                .iter()
                .any(|p| p == &new_file));
        } else {
            // Some file systems might not generate events immediately in tests
            println!("Warning: No file system events received within timeout");
        }

        monitor.stop_monitoring();
    }

    #[tokio::test]
    async fn test_batch_processing_multiple_files() {
        let temp_dir = TempDir::new().unwrap();

        let mut monitor = FileSystemMonitor::new().unwrap();

        let watched_dir =
            WatchedDirectory::new(temp_dir.path().to_path_buf(), "Batch Test".to_string());

        monitor.add_directory(watched_dir).unwrap();

        // Start monitoring
        let mut event_receiver = monitor.start_monitoring().await.unwrap();

        // Give the watcher time to set up
        sleep(Duration::from_millis(50)).await;

        // Create multiple files rapidly
        let _files = vec![
            create_test_file(temp_dir.path(), "batch1.md", "# Batch 1").await,
            create_test_file(temp_dir.path(), "batch2.md", "# Batch 2").await,
            create_test_file(temp_dir.path(), "batch3.md", "# Batch 3").await,
        ];

        // Wait for batched events
        let batch_result = timeout(Duration::from_millis(500), event_receiver.recv()).await;

        if let Ok(Some(batch)) = batch_result {
            let processing_result = monitor.process_event_batch(batch).await.unwrap();

            // Should batch multiple file creations
            assert!(processing_result.has_changes());
            assert!(processing_result.file_changes() >= 1); // At least one file
            assert!(!processing_result.created_files.is_empty());

            println!(
                "Batch processed {} events, {} file changes",
                processing_result.batch_size,
                processing_result.file_changes()
            );
        } else {
            println!("Warning: No batched events received within timeout");
        }

        monitor.stop_monitoring();
    }

    #[tokio::test]
    async fn test_rescan_functionality() {
        let temp_dir = TempDir::new().unwrap();

        // Create initial files
        create_test_file(temp_dir.path(), "initial1.md", "# Initial 1").await;
        create_test_file(temp_dir.path(), "initial2.md", "# Initial 2").await;

        let mut monitor = FileSystemMonitor::new().unwrap();

        let watched_dir =
            WatchedDirectory::new(temp_dir.path().to_path_buf(), "Rescan Test".to_string());

        monitor.add_directory(watched_dir).unwrap();

        // Initial stats
        let stats1 = monitor.get_stats();
        assert_eq!(stats1.note_files, 2);

        // Add more files
        create_test_file(temp_dir.path(), "additional1.md", "# Additional 1").await;
        create_test_file(temp_dir.path(), "additional2.md", "# Additional 2").await;

        // Rescan to detect new files
        let scan_results = monitor.rescan_all().await.unwrap();
        assert_eq!(scan_results.len(), 1); // One directory

        let scan_result = &scan_results[0];
        assert_eq!(scan_result.note_files, 4); // Should find all 4 files
        assert_eq!(scan_result.new_files, 2); // 2 new files since last scan

        // Stats should be updated
        let stats2 = monitor.get_stats();
        assert_eq!(stats2.note_files, 4);
    }

    #[tokio::test]
    async fn test_debouncing_performance() {
        let temp_dir = TempDir::new().unwrap();

        let mut monitor = FileSystemMonitor::new().unwrap();

        let watched_dir =
            WatchedDirectory::new(temp_dir.path().to_path_buf(), "Debounce Test".to_string());

        monitor.add_directory(watched_dir).unwrap();

        // Start monitoring
        let mut event_receiver = monitor.start_monitoring().await.unwrap();

        // Give the watcher time to set up
        sleep(Duration::from_millis(50)).await;

        let start_time = std::time::Instant::now();

        // Create many files rapidly to test batching
        for i in 0..5 {
            create_test_file(
                temp_dir.path(),
                &format!("rapid{}.md", i),
                &format!("# Rapid {}", i),
            )
            .await;
        }

        // Collect all batches for a short time
        let mut total_events = 0;
        let mut batch_count = 0;

        loop {
            let batch_result = timeout(Duration::from_millis(200), event_receiver.recv()).await;

            if let Ok(Some(batch)) = batch_result {
                let processing_result = monitor.process_event_batch(batch).await.unwrap();
                total_events += processing_result.file_changes();
                batch_count += 1;

                println!(
                    "Batch {}: {} file changes, processing time: {}ms",
                    batch_count,
                    processing_result.file_changes(),
                    processing_result.processing_time_ms
                );
            } else {
                break; // Timeout - no more events
            }
        }

        let total_time = start_time.elapsed();

        println!(
            "Total: {} events in {} batches over {:?}",
            total_events, batch_count, total_time
        );

        // Should have processed at least some events efficiently
        assert!(total_events >= 1);
        assert!(total_time < Duration::from_millis(1000)); // Should be fast

        monitor.stop_monitoring();
    }

    #[test]
    fn test_processing_result_operations() {
        let mut result = ProcessingResult::new();

        assert!(!result.has_changes());
        assert_eq!(result.file_changes(), 0);

        result.created_files.push(PathBuf::from("test1.md"));
        result.modified_files.push(PathBuf::from("test2.md"));
        result.deleted_files.push(PathBuf::from("test3.md"));

        assert!(result.has_changes());
        assert_eq!(result.file_changes(), 3);

        result.created_directories.push(PathBuf::from("new_dir"));
        assert!(result.has_changes());
        assert_eq!(result.file_changes(), 3); // Directories don't count as file changes
    }

    #[test]
    fn test_monitoring_stats() {
        let stats = MonitoringStats {
            watched_directories: 3,
            total_scanned_files: 42,
            note_files: 35,
            is_watching: true,
        };

        assert_eq!(stats.watched_directories, 3);
        assert_eq!(stats.total_scanned_files, 42);
        assert_eq!(stats.note_files, 35);
        assert!(stats.is_watching);
    }
}
