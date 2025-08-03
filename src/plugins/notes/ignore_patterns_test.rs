//! Comprehensive tests for ignore patterns and file filtering
//! 
//! Tests ignore pattern functionality across FileWatcher, DirectoryScanner, and integration.

#[cfg(test)]
mod tests {
    use crate::plugins::notes::watcher::{FileWatcher, WatcherConfig};
    use crate::plugins::notes::scanner::{DirectoryScanner, ScanConfig};
    use crate::plugins::notes::integration::FileSystemMonitor;
    use crate::plugins::notes::types::WatchedDirectory;
    
    use std::path::{Path, PathBuf};
    use std::time::Duration;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;
    
    fn create_comprehensive_ignore_config() -> WatcherConfig {
        WatcherConfig {
            debounce_duration: Duration::from_millis(100),
            max_batch_size: 50,
            watched_extensions: vec!["md".to_string(), "txt".to_string(), "markdown".to_string()],
            ignore_patterns: vec![
                // File extensions
                "*.tmp".to_string(),
                "*.swp".to_string(), 
                "*.bak".to_string(),
                "*.log".to_string(),
                "*~".to_string(),
                
                // Directories
                ".git/".to_string(),
                ".obsidian/".to_string(),
                ".vscode/".to_string(),
                "node_modules/".to_string(),
                "target/".to_string(),
                
                // Specific files
                ".DS_Store".to_string(),
                "Thumbs.db".to_string(),
                "desktop.ini".to_string(),
                
                // Prefix patterns
                "temp_*".to_string(),
                "tmp_*".to_string(),
                ".#*".to_string(),
                
                // Complex patterns
                "*.lock".to_string(),
                "cache/".to_string(),
                "build/".to_string(),
            ],
            watch_hidden: false,
        }
    }
    
    fn create_test_files_and_dirs(temp_dir: &Path) -> Vec<PathBuf> {
        let mut created_paths = Vec::new();
        
        // Create various files that should be included
        let included_files = vec![
            "note1.md",
            "note2.txt", 
            "document.markdown",
            "readme.md",
            "todo.txt",
            "chapter1.md",
        ];
        
        for file in included_files {
            let path = temp_dir.join(file);
            let mut f = File::create(&path).unwrap();
            f.write_all(format!("# {}\n\nContent here.", file).as_bytes()).unwrap();
            created_paths.push(path);
        }
        
        // Create files that should be ignored
        let ignored_files = vec![
            "backup.bak",
            "tempfile.tmp", 
            "editor.swp",
            "app.log",
            "config~",
            ".DS_Store",
            "Thumbs.db",
            "desktop.ini",
            "temp_data.txt",
            "tmp_notes.md",
            ".#lockfile",
            "package.lock",
        ];
        
        for file in ignored_files {
            let path = temp_dir.join(file);
            let mut f = File::create(&path).unwrap();
            f.write_all(b"ignored content").unwrap();
            created_paths.push(path);
        }
        
        // Create directories that should be ignored
        let ignored_dirs = vec![
            ".git",
            ".obsidian", 
            ".vscode",
            "node_modules",
            "target",
            "cache",
            "build",
        ];
        
        for dir in ignored_dirs {
            let dir_path = temp_dir.join(dir);
            fs::create_dir_all(&dir_path).unwrap();
            
            // Add a file inside each ignored directory
            let file_path = dir_path.join("test.md");
            let mut f = File::create(&file_path).unwrap();
            f.write_all(b"# Test\n\nThis should be ignored.").unwrap();
            created_paths.push(file_path);
        }
        
        // Create nested structure with valid files
        let nested_dir = temp_dir.join("docs").join("chapters");
        fs::create_dir_all(&nested_dir).unwrap();
        
        let nested_file = nested_dir.join("chapter2.md");
        let mut f = File::create(&nested_file).unwrap();
        f.write_all(b"# Chapter 2\n\nNested content.").unwrap();
        created_paths.push(nested_file);
        
        // Create hidden files (should be ignored by default)
        let hidden_files = vec![
            ".hidden_note.md",
            ".secret.txt",
            ".config.md",
        ];
        
        for file in hidden_files {
            let path = temp_dir.join(file);
            let mut f = File::create(&path).unwrap();
            f.write_all(b"hidden content").unwrap();
            created_paths.push(path);
        }
        
        created_paths
    }
    
    #[test]
    fn test_file_watcher_ignore_patterns() {
        let config = create_comprehensive_ignore_config();
        let watcher = FileWatcher::with_config(config).unwrap();
        
        // Test file extension patterns
        assert!(watcher.should_ignore(Path::new("temp.tmp")));
        assert!(watcher.should_ignore(Path::new("editor.swp")));
        assert!(watcher.should_ignore(Path::new("backup.bak")));
        assert!(watcher.should_ignore(Path::new("app.log")));
        assert!(watcher.should_ignore(Path::new("file~")));
        
        // Test directory patterns
        assert!(watcher.should_ignore(Path::new("/repo/.git/config")));
        assert!(watcher.should_ignore(Path::new("project/.obsidian/workspace")));
        assert!(watcher.should_ignore(Path::new("/path/node_modules/package.json")));
        assert!(watcher.should_ignore(Path::new("rust/target/debug/app")));
        
        // Test specific files
        assert!(watcher.should_ignore(Path::new(".DS_Store")));
        assert!(watcher.should_ignore(Path::new("Thumbs.db")));
        assert!(watcher.should_ignore(Path::new("desktop.ini")));
        
        // Test prefix patterns
        assert!(watcher.should_ignore(Path::new("temp_data.txt")));
        assert!(watcher.should_ignore(Path::new("tmp_notes.md")));
        assert!(watcher.should_ignore(Path::new(".#lockfile")));
        
        // Test hidden files (should be ignored when watch_hidden = false)
        assert!(watcher.should_ignore(Path::new(".hidden_file")));
        assert!(watcher.should_ignore(Path::new("/path/.secret.txt")));
        
        // Test files that should NOT be ignored
        assert!(!watcher.should_ignore(Path::new("normal.md")));
        assert!(!watcher.should_ignore(Path::new("document.txt")));
        assert!(!watcher.should_ignore(Path::new("readme.markdown")));
        assert!(!watcher.should_ignore(Path::new("/docs/chapter1.md")));
    }
    
    #[test]
    fn test_file_watcher_extension_filtering() {
        let config = create_comprehensive_ignore_config();
        let watcher = FileWatcher::with_config(config).unwrap();
        
        // Test watched extensions
        assert!(watcher.has_watched_extension(Path::new("note.md")));
        assert!(watcher.has_watched_extension(Path::new("document.txt")));
        assert!(watcher.has_watched_extension(Path::new("readme.markdown")));
        
        // Test non-watched extensions
        assert!(!watcher.has_watched_extension(Path::new("image.png")));
        assert!(!watcher.has_watched_extension(Path::new("video.mp4")));
        assert!(!watcher.has_watched_extension(Path::new("config.json")));
        assert!(!watcher.has_watched_extension(Path::new("script.js")));
        
        // Test files without extensions
        assert!(!watcher.has_watched_extension(Path::new("README")));
        assert!(!watcher.has_watched_extension(Path::new("Makefile")));
    }
    
    #[test]
    fn test_directory_scanner_ignore_patterns() {
        let temp_dir = TempDir::new().unwrap();
        create_test_files_and_dirs(temp_dir.path());
        
        let scan_config = ScanConfig {
            watched_extensions: vec!["md".to_string(), "txt".to_string(), "markdown".to_string()],
            ignore_patterns: create_comprehensive_ignore_config().ignore_patterns,
            scan_hidden: false,
            max_file_size: 10 * 1024 * 1024,
            follow_symlinks: false,
        };
        
        let mut scanner = DirectoryScanner::with_config(scan_config);
        
        let watched_dir = WatchedDirectory::new(
            temp_dir.path().to_path_buf(),
            "Ignore Test".to_string(),
        );
        
        let result = scanner.scan_directory(&watched_dir).unwrap();
        
        // Should only find the legitimate note files, not ignored ones
        // Expected: note1.md, note2.txt, document.markdown, readme.md, todo.txt, chapter1.md, chapter2.md
        // That's 7 files that should be included
        assert_eq!(result.note_files, 7);
        assert_eq!(result.total_files, 7); // Only note files are processed
        
        // Verify specific files were found
        let found_paths: Vec<String> = result.files.iter()
            .map(|f| f.path.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        
        let expected_files = vec![
            "note1.md", "note2.txt", "document.markdown", 
            "readme.md", "todo.txt", "chapter1.md", "chapter2.md"
        ];
        
        for expected in expected_files {
            assert!(found_paths.contains(&expected.to_string()), 
                   "Expected file {} not found. Found: {:?}", expected, found_paths);
        }
        
        // Verify ignored files were NOT found
        let ignored_files = vec![
            "backup.bak", "tempfile.tmp", "editor.swp", "app.log", 
            ".DS_Store", "temp_data.txt", ".hidden_note.md"
        ];
        
        for ignored in ignored_files {
            assert!(!found_paths.contains(&ignored.to_string()), 
                   "Ignored file {} was found when it shouldn't be", ignored);
        }
    }
    
    #[test]
    fn test_watched_directory_ignore_patterns() {
        let mut watched_dir = WatchedDirectory::new(
            PathBuf::from("/test/path"),
            "Custom Ignore Test".to_string(),
        );
        
        // Add custom ignore patterns
        watched_dir.ignore_patterns = vec![
            "*.draft".to_string(),
            "private_*".to_string(),
            "archive/".to_string(),
        ];
        
        // Test custom patterns
        assert!(watched_dir.should_ignore("document.draft"));
        assert!(watched_dir.should_ignore("private_notes.md"));
        assert!(watched_dir.should_ignore("archive/old.md")); // Remove leading slash
        
        // Test files that should not be ignored
        assert!(!watched_dir.should_ignore("public.md"));
        assert!(!watched_dir.should_ignore("notes.txt"));
        assert!(!watched_dir.should_ignore("/docs/chapter.md"));
    }
    
    #[test]
    fn test_scan_config_with_hidden_files() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create hidden and normal files
        let normal_file = temp_dir.path().join("normal.md");
        let mut f = File::create(&normal_file).unwrap();
        f.write_all(b"# Normal\n\nContent.").unwrap();
        
        let hidden_file = temp_dir.path().join(".hidden.md");
        let mut f = File::create(&hidden_file).unwrap();
        f.write_all(b"# Hidden\n\nContent.").unwrap();
        
        // Test with scan_hidden = false (default)
        let config_no_hidden = ScanConfig {
            watched_extensions: vec!["md".to_string()],
            ignore_patterns: vec![],
            scan_hidden: false,
            max_file_size: 10 * 1024 * 1024,
            follow_symlinks: false,
        };
        
        let mut scanner = DirectoryScanner::with_config(config_no_hidden.clone());
        let watched_dir = WatchedDirectory::new(
            temp_dir.path().to_path_buf(),
            "Hidden Test".to_string(),
        );
        
        let result = scanner.scan_directory(&watched_dir).unwrap();
        assert_eq!(result.note_files, 1); // Only normal file
        
        // Test with scan_hidden = true
        let config_with_hidden = ScanConfig {
            scan_hidden: true,
            ..config_no_hidden
        };
        
        scanner.update_config(config_with_hidden);
        scanner.clear_cache(); // Clear previous scan
        
        let result = scanner.scan_directory(&watched_dir).unwrap();
        assert_eq!(result.note_files, 2); // Both normal and hidden files
    }
    
    #[test]
    fn test_file_size_filtering() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create small file
        let small_file = temp_dir.path().join("small.md");
        let mut f = File::create(&small_file).unwrap();
        f.write_all(b"Small content.").unwrap();
        
        // Create large file
        let large_file = temp_dir.path().join("large.md");
        let mut f = File::create(&large_file).unwrap();
        let large_content = "Large content.\n".repeat(1000); // ~14KB
        f.write_all(large_content.as_bytes()).unwrap();
        
        // Test with small size limit
        let config_small_limit = ScanConfig {
            watched_extensions: vec!["md".to_string()],
            ignore_patterns: vec![],
            scan_hidden: false,
            max_file_size: 100, // 100 bytes limit
            follow_symlinks: false,
        };
        
        let mut scanner = DirectoryScanner::with_config(config_small_limit.clone());
        let watched_dir = WatchedDirectory::new(
            temp_dir.path().to_path_buf(),
            "Size Test".to_string(),
        );
        
        let result = scanner.scan_directory(&watched_dir).unwrap();
        assert_eq!(result.note_files, 1); // Only small file
        
        // Test with large size limit
        let config_large_limit = ScanConfig {
            max_file_size: 50 * 1024, // 50KB limit
            ..config_small_limit
        };
        
        scanner.update_config(config_large_limit);
        scanner.clear_cache();
        
        let result = scanner.scan_directory(&watched_dir).unwrap();
        assert_eq!(result.note_files, 2); // Both files
    }
    
    #[tokio::test]
    async fn test_integration_ignore_patterns() {
        let temp_dir = TempDir::new().unwrap();
        create_test_files_and_dirs(temp_dir.path());
        
        // Create a monitor with comprehensive ignore patterns
        let comprehensive_config = create_comprehensive_ignore_config();
        let watcher_config = WatcherConfig {
            debounce_duration: Duration::from_millis(100),
            max_batch_size: 10,
            watched_extensions: vec!["md".to_string(), "txt".to_string(), "markdown".to_string()],
            ignore_patterns: comprehensive_config.ignore_patterns.clone(),
            watch_hidden: false,
        };
        
        let scan_config = ScanConfig {
            watched_extensions: watcher_config.watched_extensions.clone(),
            ignore_patterns: watcher_config.ignore_patterns.clone(),
            scan_hidden: watcher_config.watch_hidden,
            max_file_size: 10 * 1024 * 1024,
            follow_symlinks: false,
        };
        
        let mut monitor = FileSystemMonitor::with_configs(watcher_config, scan_config).unwrap();
        
        let watched_dir = WatchedDirectory::new(
            temp_dir.path().to_path_buf(),
            "Integration Ignore Test".to_string(),
        );
        
        monitor.add_directory(watched_dir).unwrap();
        
        let stats = monitor.get_stats();
        
        // Should only count legitimate note files, ignoring all the ignored files
        assert_eq!(stats.note_files, 7);
        assert_eq!(stats.total_scanned_files, 7);
        assert_eq!(stats.watched_directories, 1);
    }
    
    #[test]
    fn test_complex_ignore_patterns() {
        let config = create_comprehensive_ignore_config();
        let watcher = FileWatcher::with_config(config).unwrap();
        
        // Test complex real-world scenarios
        let test_cases = vec![
            // Should be ignored
            ("/project/.git/config", true),
            ("/notes/.obsidian/workspace.json", true),
            ("/app/node_modules/package/index.js", true),
            ("/rust/target/debug/app", true),
            ("/docs/cache/generated.html", true),
            ("/src/build/output.js", true),
            ("temp_backup_2024.md", true),
            ("tmp_notes_draft.txt", true),
            (".#unsaved_changes", true),
            ("package.lock", true),
            ("backup_v1.bak", true),
            ("error.log", true),
            ("file~", true),
            
            // Should NOT be ignored
            ("/docs/notes.md", false),
            ("/chapters/chapter1.txt", false),
            ("/readme.markdown", false),
            ("/documentation/api.md", false),
            ("important.txt", false),
            ("/nested/deep/note.md", false),
            ("config.md", false), // Not .lock or in ignored dir
            ("git_notes.md", false), // Not in .git/
            ("build_process.md", false), // Not in build/
        ];
        
        for (path, should_ignore) in test_cases {
            let result = watcher.should_ignore(Path::new(path));
            assert_eq!(result, should_ignore, 
                      "Path '{}' should_ignore={}, but got {}", path, should_ignore, result);
        }
    }
    
    #[test]
    fn test_performance_with_many_patterns() {
        let config = create_comprehensive_ignore_config();
        let watcher = FileWatcher::with_config(config).unwrap();
        
        let start_time = std::time::Instant::now();
        
        // Test performance with many path checks
        let test_paths = (0..1000).map(|i| {
            format!("/path/to/file{}.md", i)
        }).collect::<Vec<_>>();
        
        let mut ignored_count = 0;
        for path in &test_paths {
            if watcher.should_ignore(Path::new(path)) {
                ignored_count += 1;
            }
        }
        
        let elapsed = start_time.elapsed();
        
        println!("Processed {} paths in {:?}, {} ignored", 
                test_paths.len(), elapsed, ignored_count);
        
        // Should be very fast
        assert!(elapsed < std::time::Duration::from_millis(100));
        
        // None of these should be ignored (all are .md files in valid paths)
        assert_eq!(ignored_count, 0);
    }
}