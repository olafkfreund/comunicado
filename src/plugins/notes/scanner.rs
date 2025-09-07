//! Directory scanning implementation
//!
//! Provides recursive directory scanning for note files with change detection.

use super::manager::{NoteError, NoteResult};
use super::types::{NoteId, WatchedDirectory};

use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs::{File, Metadata};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::{DirEntry, WalkDir};

/// Information about a scanned file
#[derive(Debug, Clone, PartialEq)]
pub struct ScannedFile {
    /// Absolute path to the file
    pub path: PathBuf,
    /// File size in bytes
    pub size: u64,
    /// Last modified timestamp
    pub modified: SystemTime,
    /// SHA-256 hash of content
    pub content_hash: String,
    /// Whether this is a new file (not previously scanned)
    pub is_new: bool,
    /// Whether this file has been modified since last scan
    pub is_modified: bool,
    /// Note ID if this file was previously indexed
    pub note_id: Option<NoteId>,
}

impl ScannedFile {
    /// Create a new scanned file entry
    pub fn new(path: PathBuf, metadata: &Metadata) -> NoteResult<Self> {
        let modified = metadata.modified().map_err(|e| {
            NoteError::FileSystem(format!(
                "Failed to get modified time for {}: {}",
                path.display(),
                e
            ))
        })?;

        let size = metadata.len();

        // Calculate content hash
        let content_hash = Self::calculate_content_hash(&path)?;

        Ok(Self {
            path,
            size,
            modified,
            content_hash,
            is_new: true,       // Will be updated by scanner
            is_modified: false, // Will be updated by scanner
            note_id: None,      // Will be updated by scanner
        })
    }

    /// Calculate SHA-256 hash of file content
    fn calculate_content_hash(path: &Path) -> NoteResult<String> {
        let mut file = File::open(path).map_err(|e| {
            NoteError::FileSystem(format!("Failed to open file {}: {}", path.display(), e))
        })?;

        let mut hasher = Sha256::new();
        let mut buffer = [0; 8192];

        loop {
            let bytes_read = file.read(&mut buffer).map_err(|e| {
                NoteError::FileSystem(format!("Failed to read file {}: {}", path.display(), e))
            })?;

            if bytes_read == 0 {
                break;
            }

            hasher.update(&buffer[..bytes_read]);
        }

        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Check if this file should be processed based on extension
    pub fn has_note_extension(&self) -> bool {
        if let Some(ext) = self.path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            matches!(ext_str.as_str(), "md" | "markdown" | "txt")
        } else {
            false
        }
    }
}

/// Result of a directory scan operation
#[derive(Debug, Clone)]
pub struct ScanResult {
    /// Files found during the scan
    pub files: Vec<ScannedFile>,
    /// Total number of files scanned
    pub total_files: usize,
    /// Number of new files discovered
    pub new_files: usize,
    /// Number of modified files
    pub modified_files: usize,
    /// Number of files with note extensions
    pub note_files: usize,
    /// Paths that were deleted since last scan
    pub deleted_paths: Vec<PathBuf>,
    /// Scan duration in milliseconds
    pub scan_duration_ms: u64,
    /// Any errors encountered during scanning
    pub errors: Vec<String>,
}

impl ScanResult {
    /// Create a new scan result
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            total_files: 0,
            new_files: 0,
            modified_files: 0,
            note_files: 0,
            deleted_paths: Vec::new(),
            scan_duration_ms: 0,
            errors: Vec::new(),
        }
    }

    /// Add a file to the scan result
    pub fn add_file(&mut self, file: ScannedFile) {
        if file.is_new {
            self.new_files += 1;
        }
        if file.is_modified {
            self.modified_files += 1;
        }
        if file.has_note_extension() {
            self.note_files += 1;
        }

        self.files.push(file);
        self.total_files += 1;
    }

    /// Add an error to the scan result
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    /// Check if scan found any changes
    pub fn has_changes(&self) -> bool {
        self.new_files > 0 || self.modified_files > 0 || !self.deleted_paths.is_empty()
    }
}

/// Configuration for directory scanning
#[derive(Debug, Clone)]
pub struct ScanConfig {
    /// File extensions to scan
    pub watched_extensions: Vec<String>,
    /// Patterns to ignore (glob-style)
    pub ignore_patterns: Vec<String>,
    /// Whether to scan hidden files
    pub scan_hidden: bool,
    /// Maximum file size to scan (in bytes)
    pub max_file_size: u64,
    /// Whether to follow symbolic links
    pub follow_symlinks: bool,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            watched_extensions: vec!["md".to_string(), "markdown".to_string(), "txt".to_string()],
            ignore_patterns: vec![
                ".git/".to_string(),
                ".obsidian/".to_string(),
                "*.tmp".to_string(),
                "*.swp".to_string(),
                "*~".to_string(),
                ".DS_Store".to_string(),
            ],
            scan_hidden: false,
            max_file_size: 10 * 1024 * 1024, // 10MB
            follow_symlinks: false,
        }
    }
}

/// Directory scanner for discovering and tracking note files
#[derive(Debug)]
pub struct DirectoryScanner {
    /// Scan configuration
    config: ScanConfig,
    /// Previously scanned file information
    previous_scan: HashMap<PathBuf, ScannedFile>,
}

impl DirectoryScanner {
    /// Create a new directory scanner
    pub fn new() -> Self {
        Self::with_config(ScanConfig::default())
    }

    /// Create a directory scanner with custom configuration
    pub fn with_config(config: ScanConfig) -> Self {
        Self {
            config,
            previous_scan: HashMap::new(),
        }
    }

    /// Scan a directory recursively for note files
    pub fn scan_directory(&mut self, directory: &WatchedDirectory) -> NoteResult<ScanResult> {
        let start_time = std::time::Instant::now();
        let mut result = ScanResult::new();

        if !directory.path.exists() {
            return Err(NoteError::FileSystem(format!(
                "Directory does not exist: {}",
                directory.path.display()
            )));
        }

        if !directory.path.is_dir() {
            return Err(NoteError::FileSystem(format!(
                "Path is not a directory: {}",
                directory.path.display()
            )));
        }

        // Build a set of currently scanned paths for deletion detection
        let mut current_paths = HashSet::new();

        // Configure walkdir
        let walker = WalkDir::new(&directory.path)
            .follow_links(self.config.follow_symlinks)
            .min_depth(0);

        let walker = if directory.recursive {
            walker.max_depth(usize::MAX)
        } else {
            walker.max_depth(1)
        };

        // Scan directory entries
        for entry in walker {
            match entry {
                Ok(entry) => {
                    if let Err(e) = self.process_entry(&entry, &mut result, &mut current_paths) {
                        result.add_error(format!(
                            "Error processing {}: {}",
                            entry.path().display(),
                            e
                        ));
                    }
                }
                Err(e) => {
                    result.add_error(format!("Walk error: {}", e));
                }
            }
        }

        // Detect deleted files
        self.detect_deleted_files(&current_paths, &mut result);

        // Update scan duration
        result.scan_duration_ms = start_time.elapsed().as_millis() as u64;

        // Update previous scan with current results
        self.update_previous_scan(&result);

        Ok(result)
    }

    /// Process a single directory entry
    fn process_entry(
        &self,
        entry: &DirEntry,
        result: &mut ScanResult,
        current_paths: &mut HashSet<PathBuf>,
    ) -> NoteResult<()> {
        let path = entry.path().to_path_buf();
        current_paths.insert(path.clone());

        // Skip directories
        if entry.file_type().is_dir() {
            return Ok(());
        }

        // Check if file should be ignored
        if self.should_ignore_file(&path) {
            return Ok(());
        }

        // Get file metadata
        let metadata = entry.metadata().map_err(|e| {
            NoteError::FileSystem(format!(
                "Failed to get metadata for {}: {}",
                path.display(),
                e
            ))
        })?;

        // Check file size limit
        if metadata.len() > self.config.max_file_size {
            return Ok(());
        }

        // Check if file has a watched extension
        if !self.has_watched_extension(&path) {
            return Ok(());
        }

        // Create scanned file entry
        let mut scanned_file = ScannedFile::new(path.clone(), &metadata)?;

        // Check if this is a new or modified file
        if let Some(previous_file) = self.previous_scan.get(&path) {
            scanned_file.note_id = previous_file.note_id.clone();
            scanned_file.is_new = false;

            // Check if modified
            if scanned_file.content_hash != previous_file.content_hash {
                scanned_file.is_modified = true;
            }
        }

        result.add_file(scanned_file);
        Ok(())
    }

    /// Check if a file should be ignored based on patterns
    fn should_ignore_file(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        let filename = path
            .file_name()
            .map(|name| name.to_string_lossy())
            .unwrap_or_default();

        // Check hidden files
        if !self.config.scan_hidden {
            if filename.starts_with('.') {
                return true;
            }
        }

        // Check ignore patterns
        for pattern in &self.config.ignore_patterns {
            // Check pattern against full path (for directory patterns)
            if self.matches_pattern(pattern, &path_str) {
                return true;
            }
            // Also check pattern against just the filename (for file patterns)
            if self.matches_pattern(pattern, &filename) {
                return true;
            }
        }

        false
    }

    /// Check if a file has a watched extension
    fn has_watched_extension(&self, path: &Path) -> bool {
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

    /// Detect files that were deleted since the last scan
    fn detect_deleted_files(&self, current_paths: &HashSet<PathBuf>, result: &mut ScanResult) {
        for previous_path in self.previous_scan.keys() {
            if !current_paths.contains(previous_path) {
                result.deleted_paths.push(previous_path.clone());
            }
        }
    }

    /// Update the previous scan cache with current results
    fn update_previous_scan(&mut self, result: &ScanResult) {
        // Clear previous scan
        self.previous_scan.clear();

        // Add current files
        for file in &result.files {
            self.previous_scan.insert(file.path.clone(), file.clone());
        }
    }

    /// Get statistics from the last scan
    pub fn last_scan_stats(&self) -> (usize, usize) {
        let total_files = self.previous_scan.len();
        let note_files = self
            .previous_scan
            .values()
            .filter(|f| f.has_note_extension())
            .count();

        (total_files, note_files)
    }

    /// Clear the scan cache
    pub fn clear_cache(&mut self) {
        self.previous_scan.clear();
    }

    /// Update file note ID after indexing
    pub fn update_file_note_id(&mut self, path: &Path, note_id: NoteId) {
        if let Some(file) = self.previous_scan.get_mut(path) {
            file.note_id = Some(note_id);
        }
    }

    /// Get configuration
    pub fn config(&self) -> &ScanConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: ScanConfig) {
        self.config = config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_config() -> ScanConfig {
        ScanConfig {
            watched_extensions: vec!["md".to_string(), "txt".to_string()],
            ignore_patterns: vec!["*.tmp".to_string(), ".git/".to_string()],
            scan_hidden: false,
            max_file_size: 1024 * 1024, // 1MB
            follow_symlinks: false,
        }
    }

    fn create_test_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let file_path = dir.join(name);
        let mut file = File::create(&file_path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file_path
    }

    #[test]
    fn test_scanned_file_creation() {
        let temp_dir = TempDir::new().unwrap();
        let file_path =
            create_test_file(temp_dir.path(), "test.md", "# Test Note\n\nContent here.");

        let metadata = fs::metadata(&file_path).unwrap();
        let scanned_file = ScannedFile::new(file_path.clone(), &metadata).unwrap();

        assert_eq!(scanned_file.path, file_path);
        assert!(scanned_file.size > 0);
        assert!(!scanned_file.content_hash.is_empty());
        assert!(scanned_file.is_new);
        assert!(!scanned_file.is_modified);
        assert!(scanned_file.note_id.is_none());
    }

    #[test]
    fn test_scanned_file_note_extension() {
        let temp_dir = TempDir::new().unwrap();

        let md_file = create_test_file(temp_dir.path(), "note.md", "content");
        let md_metadata = fs::metadata(&md_file).unwrap();
        let md_scanned = ScannedFile::new(md_file, &md_metadata).unwrap();
        assert!(md_scanned.has_note_extension());

        let txt_file = create_test_file(temp_dir.path(), "note.txt", "content");
        let txt_metadata = fs::metadata(&txt_file).unwrap();
        let txt_scanned = ScannedFile::new(txt_file, &txt_metadata).unwrap();
        assert!(txt_scanned.has_note_extension());

        let png_file = create_test_file(temp_dir.path(), "image.png", "binary");
        let png_metadata = fs::metadata(&png_file).unwrap();
        let png_scanned = ScannedFile::new(png_file, &png_metadata).unwrap();
        assert!(!png_scanned.has_note_extension());
    }

    #[test]
    fn test_scan_result_operations() {
        let mut result = ScanResult::new();

        // Create test files
        let temp_dir = TempDir::new().unwrap();
        let file1_path = create_test_file(temp_dir.path(), "file1.md", "content1");
        let file2_path = create_test_file(temp_dir.path(), "file2.txt", "content2");

        let metadata1 = fs::metadata(&file1_path).unwrap();
        let metadata2 = fs::metadata(&file2_path).unwrap();

        let mut file1 = ScannedFile::new(file1_path, &metadata1).unwrap();
        file1.is_new = true;

        let mut file2 = ScannedFile::new(file2_path, &metadata2).unwrap();
        file2.is_new = false; // Not a new file
        file2.is_modified = true;

        result.add_file(file1);
        result.add_file(file2);

        assert_eq!(result.total_files, 2);
        assert_eq!(result.new_files, 1);
        assert_eq!(result.modified_files, 1);
        assert_eq!(result.note_files, 2);
        assert!(result.has_changes());
    }

    #[test]
    fn test_directory_scanner_creation() {
        let scanner = DirectoryScanner::new();
        assert!(!scanner.config.scan_hidden);
        assert_eq!(scanner.config.watched_extensions.len(), 3);
        assert!(scanner.previous_scan.is_empty());

        let custom_config = create_test_config();
        let custom_scanner = DirectoryScanner::with_config(custom_config.clone());
        assert_eq!(
            custom_scanner.config.max_file_size,
            custom_config.max_file_size
        );
    }

    #[test]
    fn test_should_ignore_file() {
        let config = create_test_config();
        let scanner = DirectoryScanner::with_config(config);

        // Test hidden files
        assert!(scanner.should_ignore_file(Path::new(".hidden")));
        assert!(scanner.should_ignore_file(Path::new("/path/.hidden")));
        assert!(!scanner.should_ignore_file(Path::new("visible.md")));

        // Test ignore patterns
        assert!(scanner.should_ignore_file(Path::new("temp.tmp")));
        assert!(scanner.should_ignore_file(Path::new("/repo/.git/config")));
        assert!(!scanner.should_ignore_file(Path::new("normal.md")));
    }

    #[test]
    fn test_pattern_matching() {
        let scanner = DirectoryScanner::new();

        // Test prefix patterns
        assert!(scanner.matches_pattern("temp*", "temp.txt"));
        assert!(scanner.matches_pattern("temp*", "tempfile"));
        assert!(!scanner.matches_pattern("temp*", "mytemp"));

        // Test suffix patterns
        assert!(scanner.matches_pattern("*.tmp", "file.tmp"));
        assert!(scanner.matches_pattern("*.tmp", "backup.tmp"));
        assert!(!scanner.matches_pattern("*.tmp", "file.txt"));

        // Test directory patterns
        assert!(scanner.matches_pattern(".git/", "/repo/.git/config"));
        assert!(scanner.matches_pattern(".git/", "project/.git/HEAD"));
        assert!(!scanner.matches_pattern(".git/", "gitignore"));

        // Test exact patterns
        assert!(scanner.matches_pattern("exact", "exact"));
        assert!(!scanner.matches_pattern("exact", "inexact"));
    }

    #[test]
    fn test_scan_simple_directory() {
        let temp_dir = TempDir::new().unwrap();

        // Create test files
        create_test_file(temp_dir.path(), "note1.md", "# Note 1\n\nContent here.");
        create_test_file(temp_dir.path(), "note2.txt", "Text note content");
        create_test_file(temp_dir.path(), "image.png", "binary content");
        create_test_file(temp_dir.path(), "temp.tmp", "temporary file");

        let watched_dir =
            WatchedDirectory::new(temp_dir.path().to_path_buf(), "Test Directory".to_string());

        let mut scanner = DirectoryScanner::with_config(create_test_config());
        let result = scanner.scan_directory(&watched_dir).unwrap();

        // Should find 2 note files (md and txt), ignore png and tmp
        assert_eq!(result.note_files, 2);
        assert_eq!(result.new_files, 2); // All files are new on first scan
        assert_eq!(result.modified_files, 0);
        assert!(result.errors.is_empty());
        assert!(result.has_changes());
    }

    #[test]
    fn test_scan_recursive_directory() {
        let temp_dir = TempDir::new().unwrap();

        // Create nested directory structure
        let subdir1 = temp_dir.path().join("subdir1");
        let subdir2 = temp_dir.path().join("subdir1/subdir2");
        fs::create_dir_all(&subdir2).unwrap();

        // Create files at different levels
        create_test_file(temp_dir.path(), "root.md", "Root note");
        create_test_file(&subdir1, "sub1.md", "Subdir 1 note");
        create_test_file(&subdir2, "sub2.md", "Subdir 2 note");

        // Create watched directory with recursion enabled
        let watched_dir =
            WatchedDirectory::new(temp_dir.path().to_path_buf(), "Recursive Test".to_string());

        let mut scanner = DirectoryScanner::with_config(create_test_config());
        let result = scanner.scan_directory(&watched_dir).unwrap();

        // Should find all 3 markdown files recursively
        assert_eq!(result.note_files, 3);
        assert_eq!(result.new_files, 3);

        // Test non-recursive scan
        let mut non_recursive_dir = watched_dir.clone();
        non_recursive_dir.recursive = false;

        scanner.clear_cache(); // Clear previous scan
        let result = scanner.scan_directory(&non_recursive_dir).unwrap();

        // Should only find root file
        assert_eq!(result.note_files, 1);
    }

    #[test]
    fn test_change_detection() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = create_test_file(temp_dir.path(), "note.md", "# Original Content");

        let watched_dir =
            WatchedDirectory::new(temp_dir.path().to_path_buf(), "Change Test".to_string());

        let mut scanner = DirectoryScanner::with_config(create_test_config());

        // First scan
        let result1 = scanner.scan_directory(&watched_dir).unwrap();
        assert_eq!(result1.new_files, 1);
        assert_eq!(result1.modified_files, 0);

        // Modify file content
        std::thread::sleep(std::time::Duration::from_millis(10)); // Ensure timestamp difference
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"# Modified Content\n\nNew content here.")
            .unwrap();
        drop(file);

        // Second scan should detect modification
        let result2 = scanner.scan_directory(&watched_dir).unwrap();
        assert_eq!(result2.new_files, 0);
        assert_eq!(result2.modified_files, 1);
        assert!(result2.has_changes());
    }

    #[test]
    fn test_deletion_detection() {
        let temp_dir = TempDir::new().unwrap();
        let file1_path = create_test_file(temp_dir.path(), "note1.md", "Content 1");
        let _file2_path = create_test_file(temp_dir.path(), "note2.md", "Content 2");

        let watched_dir =
            WatchedDirectory::new(temp_dir.path().to_path_buf(), "Deletion Test".to_string());

        let mut scanner = DirectoryScanner::with_config(create_test_config());

        // First scan
        let result1 = scanner.scan_directory(&watched_dir).unwrap();
        assert_eq!(result1.note_files, 2);
        assert!(result1.deleted_paths.is_empty());

        // Delete one file
        fs::remove_file(&file1_path).unwrap();

        // Second scan should detect deletion
        let result2 = scanner.scan_directory(&watched_dir).unwrap();
        assert_eq!(result2.note_files, 1);
        assert_eq!(result2.deleted_paths.len(), 1);
        assert_eq!(result2.deleted_paths[0], file1_path);
        assert!(result2.has_changes());
    }

    #[test]
    fn test_scan_nonexistent_directory() {
        let watched_dir = WatchedDirectory::new(
            PathBuf::from("/nonexistent/path"),
            "Nonexistent".to_string(),
        );

        let mut scanner = DirectoryScanner::new();
        let result = scanner.scan_directory(&watched_dir);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[test]
    fn test_scanner_stats_and_cache() {
        let temp_dir = TempDir::new().unwrap();
        create_test_file(temp_dir.path(), "note1.md", "Content 1");
        create_test_file(temp_dir.path(), "note2.txt", "Content 2");
        create_test_file(temp_dir.path(), "image.png", "binary");

        let watched_dir =
            WatchedDirectory::new(temp_dir.path().to_path_buf(), "Stats Test".to_string());

        let mut scanner = DirectoryScanner::with_config(create_test_config());

        // Before scan
        let (total, notes) = scanner.last_scan_stats();
        assert_eq!(total, 0);
        assert_eq!(notes, 0);

        // After scan
        scanner.scan_directory(&watched_dir).unwrap();
        let (total, notes) = scanner.last_scan_stats();
        assert_eq!(total, 2); // Only note files (png filtered out)
        assert_eq!(notes, 2); // Same as total since all are note files

        // Test cache clear
        scanner.clear_cache();
        let (total, notes) = scanner.last_scan_stats();
        assert_eq!(total, 0);
        assert_eq!(notes, 0);
    }

    #[test]
    fn test_update_file_note_id() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = create_test_file(temp_dir.path(), "note.md", "Content");

        let watched_dir =
            WatchedDirectory::new(temp_dir.path().to_path_buf(), "ID Test".to_string());

        let mut scanner = DirectoryScanner::new();

        // Scan first
        scanner.scan_directory(&watched_dir).unwrap();

        // Update note ID
        scanner.update_file_note_id(&file_path, "note-123".to_string());

        // Verify update
        if let Some(file) = scanner.previous_scan.get(&file_path) {
            assert_eq!(file.note_id, Some("note-123".to_string()));
        } else {
            panic!("File not found in scan cache");
        }
    }

    #[test]
    fn test_config_update() {
        let mut scanner = DirectoryScanner::new();

        let new_config = ScanConfig {
            watched_extensions: vec!["md".to_string()],
            ignore_patterns: vec!["*.backup".to_string()],
            scan_hidden: true,
            max_file_size: 5000,
            follow_symlinks: true,
        };

        scanner.update_config(new_config.clone());

        assert_eq!(scanner.config.max_file_size, 5000);
        assert!(scanner.config.scan_hidden);
        assert!(scanner.config.follow_symlinks);
        assert_eq!(scanner.config.watched_extensions.len(), 1);
    }
}
