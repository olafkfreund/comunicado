//! Core backup engine for creating and managing backups

use crate::backup::{DataCategory, BackupTarget};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

/// Backup engine errors
#[derive(Error, Debug)]
pub enum BackupError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Compression error: {0}")]
    Compression(String),
    
    #[error("Encryption error: {0}")]
    Encryption(String),
    
    #[error("Invalid backup target: {0}")]
    InvalidTarget(String),
    
    #[error("Backup not found: {0}")]
    BackupNotFound(Uuid),
    
    #[error("Insufficient space: need {needed} bytes, have {available}")]
    InsufficientSpace { needed: u64, available: u64 },
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Authentication failed: {0}")]
    Authentication(String),
}

pub type BackupResult<T> = Result<T, BackupError>;

/// Types of backups
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BackupType {
    /// Complete backup of all selected data
    Full,
    /// Only changes since last backup
    Incremental,
    /// Changes since last full backup
    Differential,
    /// Snapshot at a specific point in time
    Snapshot,
}

/// Backup status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BackupStatus {
    /// Backup is being prepared
    Preparing,
    /// Backup is in progress
    Running { progress: f32, current_item: String },
    /// Backup completed successfully
    Completed { duration_ms: u64, bytes_backed_up: u64 },
    /// Backup failed with error
    Failed { error: String },
    /// Backup was cancelled by user
    Cancelled,
}

/// Backup configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BackupConfig {
    /// Unique identifier for this backup configuration
    pub id: Uuid,
    /// Human-readable name for the backup
    pub name: String,
    /// Description of what this backup contains
    pub description: String,
    /// Type of backup to perform
    pub backup_type: BackupType,
    /// Target location for the backup
    pub target: BackupTarget,
    /// Categories of data to include
    pub included_categories: HashSet<DataCategory>,
    /// Enable compression
    pub compression_enabled: bool,
    /// Compression level (0-9)
    pub compression_level: u8,
    /// Enable encryption
    pub encryption_enabled: bool,
    /// Encryption password (hashed)
    pub encryption_key_hash: Option<String>,
    /// Maximum number of versions to keep
    pub max_versions: u32,
    /// Exclude patterns (glob patterns)
    pub exclude_patterns: Vec<String>,
    /// Include patterns (glob patterns) 
    pub include_patterns: Vec<String>,
    /// Follow symbolic links
    pub follow_symlinks: bool,
    /// Verify backup integrity after creation
    pub verify_integrity: bool,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Last modified timestamp
    pub updated_at: DateTime<Utc>,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Default Backup".to_string(),
            description: "Full backup of all user data".to_string(),
            backup_type: BackupType::Full,
            target: BackupTarget::Local(PathBuf::from("~/comunicado-backups")),
            included_categories: DataCategory::all_categories().into_iter().collect(),
            compression_enabled: true,
            compression_level: 6,
            encryption_enabled: false,
            encryption_key_hash: None,
            max_versions: 10,
            exclude_patterns: vec![
                "*.tmp".to_string(),
                "*.log".to_string(),
                "*/cache/*".to_string(),
                "*/temp/*".to_string(),
            ],
            include_patterns: vec!["*".to_string()],
            follow_symlinks: false,
            verify_integrity: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

/// Backup metadata and statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    pub id: Uuid,
    pub config_id: Uuid,
    pub backup_type: BackupType,
    pub status: BackupStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub total_files: u64,
    pub total_bytes: u64,
    pub compressed_bytes: Option<u64>,
    pub files_processed: u64,
    pub bytes_processed: u64,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub checksum: Option<String>,
    pub version: u32,
    pub parent_backup: Option<Uuid>,
}

/// Core backup engine
pub struct BackupEngine {
    /// Base directory for storing backups
    base_dir: PathBuf,
    /// Application data directory
    data_dir: PathBuf,
    /// Current backup operations
    active_backups: HashMap<Uuid, BackupMetadata>,
}

impl BackupEngine {
    /// Create a new backup engine
    pub fn new(base_dir: PathBuf, data_dir: PathBuf) -> BackupResult<Self> {
        // Ensure base directory exists
        if !base_dir.exists() {
            fs::create_dir_all(&base_dir)?;
        }

        Ok(Self {
            base_dir,
            data_dir,
            active_backups: HashMap::new(),
        })
    }

    /// Start a new backup operation
    pub async fn start_backup(&mut self, config: &BackupConfig) -> BackupResult<Uuid> {
        let backup_id = Uuid::new_v4();
        
        // Create backup metadata
        let metadata = BackupMetadata {
            id: backup_id,
            config_id: config.id,
            backup_type: config.backup_type.clone(),
            status: BackupStatus::Preparing,
            started_at: Utc::now(),
            completed_at: None,
            total_files: 0,
            total_bytes: 0,
            compressed_bytes: None,
            files_processed: 0,
            bytes_processed: 0,
            errors: Vec::new(),
            warnings: Vec::new(),
            checksum: None,
            version: self.get_next_version(&config.id)?,
            parent_backup: self.get_last_backup(&config.id)?,
        };

        self.active_backups.insert(backup_id, metadata);

        // Start backup in background
        let backup_engine = self.clone();
        let config = config.clone();
        tokio::spawn(async move {
            if let Err(e) = backup_engine.perform_backup(backup_id, &config).await {
                eprintln!("Backup failed: {}", e);
            }
        });

        Ok(backup_id)
    }

    /// Perform the actual backup operation
    async fn perform_backup(&self, backup_id: Uuid, config: &BackupConfig) -> BackupResult<()> {
        // Update status to running
        self.update_backup_status(backup_id, BackupStatus::Running {
            progress: 0.0,
            current_item: "Analyzing data...".to_string(),
        })?;

        // Analyze what needs to be backed up
        let backup_plan = self.create_backup_plan(config)?;
        
        // Calculate total size
        let total_size = self.calculate_backup_size(&backup_plan)?;
        
        // Check available space
        self.check_available_space(&config.target, total_size)?;

        // Create backup directory
        let backup_dir = self.create_backup_directory(backup_id, config)?;

        let mut files_processed = 0;
        let mut bytes_processed = 0;
        let total_files = backup_plan.len() as u64;

        // Process each file in the backup plan
        for (i, file_path) in backup_plan.iter().enumerate() {
            let progress = (i as f32 / total_files as f32) * 100.0;
            let current_item = file_path.display().to_string();

            self.update_backup_status(backup_id, BackupStatus::Running {
                progress,
                current_item,
            })?;

            // Copy file to backup
            match self.backup_file(file_path, &backup_dir, config).await {
                Ok(size) => {
                    files_processed += 1;
                    bytes_processed += size;
                }
                Err(e) => {
                    self.add_backup_error(backup_id, format!("Failed to backup {}: {}", 
                        file_path.display(), e))?;
                }
            }
        }

        // Apply compression if enabled
        if config.compression_enabled {
            self.update_backup_status(backup_id, BackupStatus::Running {
                progress: 90.0,
                current_item: "Compressing backup...".to_string(),
            })?;

            self.compress_backup(&backup_dir, config.compression_level)?;
        }

        // Apply encryption if enabled
        if config.encryption_enabled {
            self.update_backup_status(backup_id, BackupStatus::Running {
                progress: 95.0,
                current_item: "Encrypting backup...".to_string(),
            })?;

            self.encrypt_backup(&backup_dir, config)?;
        }

        // Verify integrity if enabled
        if config.verify_integrity {
            self.update_backup_status(backup_id, BackupStatus::Running {
                progress: 98.0,
                current_item: "Verifying backup integrity...".to_string(),
            })?;

            self.verify_backup_integrity(&backup_dir)?;
        }

        // Update final status
        let duration_ms = self.get_backup_duration(backup_id)?;
        self.update_backup_status(backup_id, BackupStatus::Completed {
            duration_ms,
            bytes_backed_up: bytes_processed,
        })?;

        Ok(())
    }

    /// Create a backup plan (list of files to backup)
    fn create_backup_plan(&self, config: &BackupConfig) -> BackupResult<Vec<PathBuf>> {
        let mut files = Vec::new();
        
        for category in &config.included_categories {
            match category {
                DataCategory::Emails => {
                    files.extend(self.get_email_files()?);
                }
                DataCategory::Calendar => {
                    files.extend(self.get_calendar_files()?);
                }
                DataCategory::Contacts => {
                    files.extend(self.get_contact_files()?);
                }
                DataCategory::Configuration => {
                    files.extend(self.get_config_files()?);
                }
                DataCategory::Settings => {
                    files.extend(self.get_settings_files()?);
                }
                DataCategory::Filters => {
                    files.extend(self.get_filter_files()?);
                }
                DataCategory::Accounts => {
                    files.extend(self.get_account_files()?);
                }
                DataCategory::Plugins => {
                    files.extend(self.get_plugin_files()?);
                }
                DataCategory::Themes => {
                    files.extend(self.get_theme_files()?);
                }
                DataCategory::Indexes => {
                    files.extend(self.get_index_files()?);
                }
                DataCategory::All => {
                    // Add all files from data directory
                    files.extend(self.get_all_data_files()?);
                }
            }
        }

        // Apply include/exclude patterns
        files = self.apply_patterns(files, &config.include_patterns, &config.exclude_patterns)?;

        Ok(files)
    }

    /// Get files for each data category
    fn get_email_files(&self) -> BackupResult<Vec<PathBuf>> {
        let mut files = Vec::new();
        let email_dir = self.data_dir.join("emails");
        if email_dir.exists() {
            files.extend(self.scan_directory(&email_dir)?);
        }
        Ok(files)
    }

    fn get_calendar_files(&self) -> BackupResult<Vec<PathBuf>> {
        let mut files = Vec::new();
        let calendar_dir = self.data_dir.join("calendar");
        if calendar_dir.exists() {
            files.extend(self.scan_directory(&calendar_dir)?);
        }
        Ok(files)
    }

    fn get_contact_files(&self) -> BackupResult<Vec<PathBuf>> {
        let mut files = Vec::new();
        let contacts_dir = self.data_dir.join("contacts");
        if contacts_dir.exists() {
            files.extend(self.scan_directory(&contacts_dir)?);
        }
        Ok(files)
    }

    fn get_config_files(&self) -> BackupResult<Vec<PathBuf>> {
        let mut files = Vec::new();
        if let Some(config_dir) = dirs::config_dir() {
            let app_config = config_dir.join("comunicado");
            if app_config.exists() {
                files.extend(self.scan_directory(&app_config)?);
            }
        }
        Ok(files)
    }

    fn get_settings_files(&self) -> BackupResult<Vec<PathBuf>> {
        self.get_config_files() // Settings are part of config
    }

    fn get_filter_files(&self) -> BackupResult<Vec<PathBuf>> {
        let mut files = Vec::new();
        let filters_file = self.data_dir.join("filters.json");
        if filters_file.exists() {
            files.push(filters_file);
        }
        Ok(files)
    }

    fn get_account_files(&self) -> BackupResult<Vec<PathBuf>> {
        let mut files = Vec::new();
        let accounts_dir = self.data_dir.join("accounts");
        if accounts_dir.exists() {
            files.extend(self.scan_directory(&accounts_dir)?);
        }
        Ok(files)
    }

    fn get_plugin_files(&self) -> BackupResult<Vec<PathBuf>> {
        let mut files = Vec::new();
        let plugins_dir = self.data_dir.join("plugins");
        if plugins_dir.exists() {
            files.extend(self.scan_directory(&plugins_dir)?);
        }
        Ok(files)
    }

    fn get_theme_files(&self) -> BackupResult<Vec<PathBuf>> {
        let mut files = Vec::new();
        let themes_dir = self.data_dir.join("themes");
        if themes_dir.exists() {
            files.extend(self.scan_directory(&themes_dir)?);
        }
        Ok(files)
    }

    fn get_index_files(&self) -> BackupResult<Vec<PathBuf>> {
        let mut files = Vec::new();
        let index_dir = self.data_dir.join("indexes");
        if index_dir.exists() {
            files.extend(self.scan_directory(&index_dir)?);
        }
        Ok(files)
    }

    fn get_all_data_files(&self) -> BackupResult<Vec<PathBuf>> {
        self.scan_directory(&self.data_dir)
    }

    /// Recursively scan a directory for files
    fn scan_directory(&self, dir: &Path) -> BackupResult<Vec<PathBuf>> {
        let mut files = Vec::new();
        
        if !dir.exists() {
            return Ok(files);
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                files.push(path);
            } else if path.is_dir() {
                files.extend(self.scan_directory(&path)?);
            }
        }

        Ok(files)
    }

    /// Apply include/exclude patterns to file list
    fn apply_patterns(
        &self,
        files: Vec<PathBuf>,
        include_patterns: &[String],
        exclude_patterns: &[String],
    ) -> BackupResult<Vec<PathBuf>> {
        // For now, implement simple pattern matching
        // In a full implementation, you'd use glob patterns
        let mut filtered_files = Vec::new();
        
        for file in files {
            let file_str = file.to_string_lossy();
            
            // Check exclude patterns first
            let mut excluded = false;
            for pattern in exclude_patterns {
                if file_str.contains(pattern.trim_start_matches('*').trim_end_matches('*')) {
                    excluded = true;
                    break;
                }
            }
            
            if !excluded {
                // Check include patterns
                let mut included = include_patterns.is_empty() || include_patterns.contains(&"*".to_string());
                for pattern in include_patterns {
                    if pattern == "*" || file_str.contains(pattern.trim_start_matches('*').trim_end_matches('*')) {
                        included = true;
                        break;
                    }
                }
                
                if included {
                    filtered_files.push(file);
                }
            }
        }

        Ok(filtered_files)
    }

    // Placeholder implementations for remaining methods
    fn calculate_backup_size(&self, _plan: &[PathBuf]) -> BackupResult<u64> {
        Ok(1024 * 1024 * 100) // 100MB placeholder
    }

    fn check_available_space(&self, _target: &BackupTarget, _needed: u64) -> BackupResult<()> {
        Ok(()) // Placeholder
    }

    fn create_backup_directory(&self, backup_id: Uuid, _config: &BackupConfig) -> BackupResult<PathBuf> {
        let dir = self.base_dir.join(format!("backup-{}", backup_id));
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    async fn backup_file(&self, source: &Path, backup_dir: &Path, _config: &BackupConfig) -> BackupResult<u64> {
        let relative_path = source.strip_prefix(&self.data_dir).unwrap_or(source);
        let dest_path = backup_dir.join(relative_path);
        
        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        fs::copy(source, &dest_path)?;
        Ok(fs::metadata(source)?.len())
    }

    fn compress_backup(&self, _backup_dir: &Path, _level: u8) -> BackupResult<()> {
        Ok(()) // Placeholder
    }

    fn encrypt_backup(&self, _backup_dir: &Path, _config: &BackupConfig) -> BackupResult<()> {
        Ok(()) // Placeholder
    }

    fn verify_backup_integrity(&self, _backup_dir: &Path) -> BackupResult<()> {
        Ok(()) // Placeholder
    }

    fn update_backup_status(&self, _backup_id: Uuid, _status: BackupStatus) -> BackupResult<()> {
        Ok(()) // Placeholder
    }

    fn add_backup_error(&self, _backup_id: Uuid, _error: String) -> BackupResult<()> {
        Ok(()) // Placeholder
    }

    fn get_backup_duration(&self, _backup_id: Uuid) -> BackupResult<u64> {
        Ok(1000) // Placeholder
    }

    fn get_next_version(&self, _config_id: &Uuid) -> BackupResult<u32> {
        Ok(1) // Placeholder
    }

    fn get_last_backup(&self, _config_id: &Uuid) -> BackupResult<Option<Uuid>> {
        Ok(None) // Placeholder
    }
}

impl Clone for BackupEngine {
    fn clone(&self) -> Self {
        Self {
            base_dir: self.base_dir.clone(),
            data_dir: self.data_dir.clone(),
            active_backups: HashMap::new(), // Don't clone active backups
        }
    }
}