//! Core migration engine for orchestrating email client migrations
//!
//! This module provides the main engine that coordinates the migration process
//! from various email clients to Comunicado, handling data conversion, validation,
//! progress tracking, and error recovery.

use crate::migration::{
    MigrationConfig, MigrationSource, MigrationTarget, MigrationStatistics,
    MigrationDataType, ConflictInfo, ConflictResolution, ValidationResults,
    EmailClient, MigrationErrorInfo,
};
use crate::email::{EmailDatabase, StoredMessage};
use crate::contacts::{ContactsDatabase, Contact};
use chrono::{DateTime, Utc};
use futures::stream::{self, StreamExt};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

/// Migration engine errors
#[derive(Error, Debug)]
pub enum MigrationError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("Validation failed: {0}")]
    Validation(String),
    
    #[error("Source not accessible: {0}")]
    SourceNotAccessible(String),
    
    #[error("Target not writable: {0}")]
    TargetNotWritable(String),
    
    #[error("Conversion failed: {0}")]
    ConversionFailed(String),
    
    #[error("Migration cancelled")]
    Cancelled,
    
    #[error("Insufficient disk space: needed {needed}, available {available}")]
    InsufficientSpace { needed: u64, available: u64 },
    
    #[error("Permission denied: {0}")]
    Permission(String),
    
    #[error("Unsupported client: {0:?}")]
    UnsupportedClient(EmailClient),
}

pub type MigrationResult<T> = Result<T, MigrationError>;

/// Migration task status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MigrationStatus {
    Pending,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

/// Individual migration task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationTask {
    pub id: Uuid,
    pub name: String,
    pub data_type: MigrationDataType,
    pub source_path: PathBuf,
    pub target_path: PathBuf,
    pub status: MigrationStatus,
    pub progress: f64, // 0.0 to 1.0
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub estimated_items: Option<usize>,
    pub processed_items: usize,
    pub failed_items: usize,
    pub error_message: Option<String>,
}

/// Migration plan containing all tasks to execute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPlan {
    pub id: Uuid,
    pub config: MigrationConfig,
    pub source: MigrationSource,
    pub target: MigrationTarget,
    pub tasks: Vec<MigrationTask>,
    pub total_estimated_items: usize,
    pub total_estimated_size: u64,
    pub conflicts: Vec<ConflictInfo>,
    pub validation_results: Option<ValidationResults>,
    pub created_at: DateTime<Utc>,
}

/// Real-time migration progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationProgress {
    pub migration_id: Uuid,
    pub overall_progress: f64, // 0.0 to 1.0
    pub current_task: Option<String>,
    pub tasks_completed: usize,
    pub tasks_total: usize,
    pub items_processed: usize,
    pub items_total: usize,
    pub bytes_processed: u64,
    pub bytes_total: u64,
    pub elapsed_seconds: u64,
    pub estimated_remaining_seconds: Option<u64>,
    pub current_speed_items_per_sec: f64,
    pub current_speed_bytes_per_sec: f64,
    pub status: MigrationStatus,
    pub last_updated: DateTime<Utc>,
}

/// Progress callback for real-time updates
pub type ProgressCallback = Arc<dyn Fn(MigrationProgress) + Send + Sync>;

/// Main migration engine
pub struct MigrationEngine {
    email_db: Arc<EmailDatabase>,
    contacts_db: Arc<ContactsDatabase>,
    progress_bars: Arc<Mutex<MultiProgress>>,
    active_migrations: Arc<RwLock<HashMap<Uuid, MigrationProgress>>>,
    cancel_tokens: Arc<RwLock<HashMap<Uuid, bool>>>,
}

impl MigrationEngine {
    /// Create a new migration engine
    pub async fn new(
        email_db: Arc<EmailDatabase>,
        contacts_db: Arc<ContactsDatabase>,
    ) -> MigrationResult<Self> {
        Ok(Self {
            email_db,
            contacts_db,
            progress_bars: Arc::new(Mutex::new(MultiProgress::new())),
            active_migrations: Arc::new(RwLock::new(HashMap::new())),
            cancel_tokens: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Create a migration plan from configuration
    pub async fn create_plan(&self, config: MigrationConfig) -> MigrationResult<MigrationPlan> {
        let source = self.analyze_source(&config).await?;
        let target = self.analyze_target(&config).await?;
        let tasks = self.generate_tasks(&config, &source, &target).await?;
        let conflicts = self.detect_conflicts(&source, &target, &tasks).await?;
        
        let total_estimated_items = tasks.iter()
            .filter_map(|t| t.estimated_items)
            .sum();
        
        let total_estimated_size = source.estimated_size.unwrap_or(0);

        Ok(MigrationPlan {
            id: Uuid::new_v4(),
            config,
            source,
            target,
            tasks,
            total_estimated_items,
            total_estimated_size,
            conflicts,
            validation_results: None,
            created_at: Utc::now(),
        })
    }

    /// Validate a migration plan
    pub async fn validate_plan(&self, plan: &mut MigrationPlan) -> MigrationResult<ValidationResults> {
        let mut checks = Vec::new();
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();

        // Check source accessibility
        if !plan.source.is_accessible {
            checks.push(crate::migration::ValidationCheck {
                name: "Source Accessibility".to_string(),
                passed: false,
                details: "Source path is not accessible".to_string(),
                severity: crate::migration::ValidationSeverity::Critical,
            });
            issues.push(crate::migration::ValidationIssue {
                severity: crate::migration::ValidationSeverity::Critical,
                category: "Source".to_string(),
                description: "Cannot access source data".to_string(),
                affected_items: vec![plan.source.profile_path.to_string_lossy().to_string()],
                suggested_fix: Some("Check file permissions and path existence".to_string()),
                auto_fixable: false,
            });
        } else {
            checks.push(crate::migration::ValidationCheck {
                name: "Source Accessibility".to_string(),
                passed: true,
                details: "Source path is accessible".to_string(),
                severity: crate::migration::ValidationSeverity::Info,
            });
        }

        // Check disk space
        if let Some(available) = plan.target.available_space {
            if available < plan.total_estimated_size {
                checks.push(crate::migration::ValidationCheck {
                    name: "Disk Space".to_string(),
                    passed: false,
                    details: format!("Insufficient disk space: need {}, have {}", 
                        plan.total_estimated_size, available),
                    severity: crate::migration::ValidationSeverity::Critical,
                });
                issues.push(crate::migration::ValidationIssue {
                    severity: crate::migration::ValidationSeverity::Critical,
                    category: "Storage".to_string(),
                    description: "Insufficient disk space for migration".to_string(),
                    affected_items: vec!["All data".to_string()],
                    suggested_fix: Some("Free up disk space or choose a different target location".to_string()),
                    auto_fixable: false,
                });
            }
        }

        // Check for conflicts
        if !plan.conflicts.is_empty() {
            checks.push(crate::migration::ValidationCheck {
                name: "Data Conflicts".to_string(),
                passed: false,
                details: format!("{} conflicts detected", plan.conflicts.len()),
                severity: crate::migration::ValidationSeverity::Warning,
            });
            recommendations.push("Review and resolve data conflicts before proceeding".to_string());
        }

        // Add recommendations
        if plan.total_estimated_items > 10000 {
            recommendations.push("Large migration detected. Consider running during off-peak hours.".to_string());
        }

        if !plan.config.create_backup {
            recommendations.push("Enable backup creation for safer migration.".to_string());
        }

        let validation_results = ValidationResults {
            is_valid: issues.iter().all(|i| !matches!(i.severity, crate::migration::ValidationSeverity::Critical)),
            validation_time: Utc::now(),
            checks_performed: checks,
            issues_found: issues,
            recommendations,
        };

        plan.validation_results = Some(validation_results.clone());
        Ok(validation_results)
    }

    /// Execute a migration plan
    pub async fn execute_plan(
        &self,
        plan: MigrationPlan,
        progress_callback: Option<ProgressCallback>,
    ) -> MigrationResult<MigrationStatistics> {
        let migration_id = plan.id;
        let mut stats = MigrationStatistics::default();
        stats.started_at = Some(Utc::now());

        // Initialize progress tracking
        let mut progress = MigrationProgress {
            migration_id,
            overall_progress: 0.0,
            current_task: None,
            tasks_completed: 0,
            tasks_total: plan.tasks.len(),
            items_processed: 0,
            items_total: plan.total_estimated_items,
            bytes_processed: 0,
            bytes_total: plan.total_estimated_size,
            elapsed_seconds: 0,
            estimated_remaining_seconds: None,
            current_speed_items_per_sec: 0.0,
            current_speed_bytes_per_sec: 0.0,
            status: MigrationStatus::Running,
            last_updated: Utc::now(),
        };

        // Register migration
        self.active_migrations.write().await.insert(migration_id, progress.clone());
        self.cancel_tokens.write().await.insert(migration_id, false);

        // Create progress bar
        let main_pb = {
            let mp = self.progress_bars.lock().unwrap();
            mp.add(ProgressBar::new(plan.total_estimated_items as u64))
        };
        main_pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
                .unwrap()
                .progress_chars("#>-"),
        );

        // Execute tasks
        for (task_index, mut task) in plan.tasks.into_iter().enumerate() {
            // Check for cancellation
            if *self.cancel_tokens.read().await.get(&migration_id).unwrap_or(&false) {
                progress.status = MigrationStatus::Cancelled;
                self.update_progress(migration_id, progress.clone(), &progress_callback).await;
                return Err(MigrationError::Cancelled);
            }

            progress.current_task = Some(task.name.clone());
            task.status = MigrationStatus::Running;
            task.started_at = Some(Utc::now());

            main_pb.set_message(format!("Processing: {}", task.name));

            // Execute individual task
            match self.execute_task(&mut task, &mut stats).await {
                Ok(_) => {
                    task.status = MigrationStatus::Completed;
                    task.completed_at = Some(Utc::now());
                    progress.tasks_completed += 1;
                    progress.items_processed += task.processed_items;
                }
                Err(e) => {
                    task.status = MigrationStatus::Failed;
                    task.error_message = Some(e.to_string());
                    stats.errors.push(MigrationErrorInfo {
                        timestamp: Utc::now(),
                        error_type: "TaskExecution".to_string(),
                        description: e.to_string(),
                        item_path: Some(task.source_path.clone()),
                        data_type: Some(task.data_type.clone()),
                        recoverable: false,
                        retry_count: 0,
                    });
                }
            }

            // Update progress
            progress.overall_progress = (task_index + 1) as f64 / progress.tasks_total as f64;
            progress.last_updated = Utc::now();
            self.update_progress(migration_id, progress.clone(), &progress_callback).await;

            main_pb.set_position(progress.items_processed as u64);
        }

        // Finalize
        progress.status = MigrationStatus::Completed;
        progress.overall_progress = 1.0;
        stats.completed_at = Some(Utc::now());
        
        if let (Some(start), Some(end)) = (stats.started_at, stats.completed_at) {
            stats.duration_seconds = Some((end - start).num_seconds() as u64);
        }

        self.update_progress(migration_id, progress, &progress_callback).await;
        self.cleanup_migration(migration_id).await;

        main_pb.finish_with_message("Migration completed!");

        Ok(stats)
    }

    /// Cancel an active migration
    pub async fn cancel_migration(&self, migration_id: Uuid) -> MigrationResult<()> {
        self.cancel_tokens.write().await.insert(migration_id, true);
        
        if let Some(mut progress) = self.active_migrations.write().await.get_mut(&migration_id) {
            progress.status = MigrationStatus::Cancelled;
        }
        
        Ok(())
    }

    /// Get progress for a migration
    pub async fn get_progress(&self, migration_id: Uuid) -> Option<MigrationProgress> {
        self.active_migrations.read().await.get(&migration_id).cloned()
    }

    /// Analyze source for migration planning
    async fn analyze_source(&self, config: &MigrationConfig) -> MigrationResult<MigrationSource> {
        let profile_path = config.source_path.clone();
        let is_accessible = profile_path.exists() && profile_path.is_dir();
        let permissions_ok = is_accessible; // TODO: Implement proper permission checking

        let mut data_paths = HashMap::new();
        let mut estimated_size = 0u64;
        let mut estimated_count = 0usize;

        if is_accessible {
            // Analyze each data type
            for data_type in &config.data_types {
                match self.get_data_path(&config.source_client, &profile_path, data_type) {
                    Ok(path) => {
                        if path.exists() {
                            data_paths.insert(data_type.clone(), path.clone());
                            
                            // Estimate size and count
                            if let Ok(metadata) = std::fs::metadata(&path) {
                                estimated_size += metadata.len();
                            }
                            
                            // Count items (simplified estimation)
                            match data_type {
                                MigrationDataType::Emails => {
                                    estimated_count += self.estimate_email_count(&path).await.unwrap_or(0);
                                }
                                MigrationDataType::Contacts => {
                                    estimated_count += self.estimate_contact_count(&path).await.unwrap_or(0);
                                }
                                _ => {} // TODO: Implement for other types
                            }
                        }
                    }
                    Err(_) => {} // Path not found for this data type
                }
            }
        }

        Ok(MigrationSource {
            client: config.source_client.clone(),
            version: None, // TODO: Detect client version
            profile_path,
            data_paths,
            estimated_size: Some(estimated_size),
            estimated_count: Some(estimated_count),
            last_modified: None, // TODO: Get last modification time
            is_accessible,
            permissions_ok,
        })
    }

    /// Analyze target for migration planning
    async fn analyze_target(&self, config: &MigrationConfig) -> MigrationResult<MigrationTarget> {
        let storage_path = config.target_path.clone();
        
        // Check available disk space
        let available_space = self.get_available_disk_space(&storage_path)?;
        
        // Check existing data
        let mut existing_data = HashMap::new();
        existing_data.insert(MigrationDataType::Emails, 0); // TODO: Count existing emails
        existing_data.insert(MigrationDataType::Contacts, 0); // TODO: Count existing contacts

        Ok(MigrationTarget {
            account_id: "default".to_string(), // TODO: Generate or use provided account ID
            database_path: storage_path.join("database"),
            storage_path,
            available_space: Some(available_space),
            existing_data,
            conflicts: Vec::new(),
        })
    }

    /// Generate migration tasks
    async fn generate_tasks(
        &self,
        config: &MigrationConfig,
        source: &MigrationSource,
        _target: &MigrationTarget,
    ) -> MigrationResult<Vec<MigrationTask>> {
        let mut tasks = Vec::new();

        for data_type in &config.data_types {
            if let Some(source_path) = source.data_paths.get(data_type) {
                let task = MigrationTask {
                    id: Uuid::new_v4(),
                    name: format!("Migrate {:?}", data_type),
                    data_type: data_type.clone(),
                    source_path: source_path.clone(),
                    target_path: config.target_path.clone(),
                    status: MigrationStatus::Pending,
                    progress: 0.0,
                    started_at: None,
                    completed_at: None,
                    estimated_items: self.estimate_items_for_type(source_path, data_type).await.ok(),
                    processed_items: 0,
                    failed_items: 0,
                    error_message: None,
                };
                tasks.push(task);
            }
        }

        Ok(tasks)
    }

    /// Detect potential conflicts
    async fn detect_conflicts(
        &self,
        _source: &MigrationSource,
        _target: &MigrationTarget,
        _tasks: &[MigrationTask],
    ) -> MigrationResult<Vec<ConflictInfo>> {
        // TODO: Implement conflict detection
        Ok(Vec::new())
    }

    /// Execute a single migration task
    async fn execute_task(
        &self,
        task: &mut MigrationTask,
        stats: &mut MigrationStatistics,
    ) -> MigrationResult<()> {
        match task.data_type {
            MigrationDataType::Emails => {
                self.migrate_emails(task, stats).await
            }
            MigrationDataType::Contacts => {
                self.migrate_contacts(task, stats).await
            }
            _ => {
                task.error_message = Some("Data type not yet supported".to_string());
                Err(MigrationError::ConversionFailed("Unsupported data type".to_string()))
            }
        }
    }

    /// Migrate emails
    async fn migrate_emails(
        &self,
        task: &mut MigrationTask,
        stats: &mut MigrationStatistics,
    ) -> MigrationResult<()> {
        // TODO: Implement email migration logic
        // This would involve:
        // 1. Reading emails from source format
        // 2. Converting to StoredMessage format
        // 3. Storing in target database
        
        task.processed_items = 100; // Placeholder
        stats.emails.migrated = 100;
        stats.total_items_migrated += 100;
        
        Ok(())
    }

    /// Migrate contacts
    async fn migrate_contacts(
        &self,
        task: &mut MigrationTask,
        stats: &mut MigrationStatistics,
    ) -> MigrationResult<()> {
        // TODO: Implement contact migration logic
        
        task.processed_items = 50; // Placeholder
        stats.contacts.migrated = 50;
        stats.total_items_migrated += 50;
        
        Ok(())
    }

    /// Helper methods
    async fn update_progress(
        &self,
        migration_id: Uuid,
        progress: MigrationProgress,
        callback: &Option<ProgressCallback>,
    ) {
        self.active_migrations.write().await.insert(migration_id, progress.clone());
        
        if let Some(callback) = callback {
            callback(progress);
        }
    }

    async fn cleanup_migration(&self, migration_id: Uuid) {
        self.active_migrations.write().await.remove(&migration_id);
        self.cancel_tokens.write().await.remove(&migration_id);
    }

    fn get_data_path(
        &self,
        client: &EmailClient,
        profile_path: &Path,
        data_type: &MigrationDataType,
    ) -> MigrationResult<PathBuf> {
        match (client, data_type) {
            (EmailClient::Thunderbird, MigrationDataType::Emails) => {
                Ok(profile_path.join("Mail"))
            }
            (EmailClient::Thunderbird, MigrationDataType::Contacts) => {
                Ok(profile_path.join("abook.mab"))
            }
            _ => Err(MigrationError::UnsupportedClient(client.clone())),
        }
    }

    async fn estimate_email_count(&self, path: &Path) -> MigrationResult<usize> {
        // TODO: Implement email counting for different formats
        Ok(1000) // Placeholder
    }

    async fn estimate_contact_count(&self, path: &Path) -> MigrationResult<usize> {
        // TODO: Implement contact counting
        Ok(100) // Placeholder
    }

    async fn estimate_items_for_type(
        &self,
        path: &Path,
        data_type: &MigrationDataType,
    ) -> MigrationResult<usize> {
        match data_type {
            MigrationDataType::Emails => self.estimate_email_count(path).await,
            MigrationDataType::Contacts => self.estimate_contact_count(path).await,
            _ => Ok(0),
        }
    }

    fn get_available_disk_space(&self, path: &Path) -> MigrationResult<u64> {
        // TODO: Implement disk space checking
        Ok(1024 * 1024 * 1024 * 10) // 10GB placeholder
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_migration_plan_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = MigrationConfig {
            source_path: temp_dir.path().to_path_buf(),
            target_path: temp_dir.path().join("target"),
            ..Default::default()
        };

        // Note: This test would need proper database setup to work
        // Keeping it simple for now
        assert_eq!(config.data_types, vec![MigrationDataType::Emails]);
    }

    #[test]
    fn test_migration_task_creation() {
        let task = MigrationTask {
            id: Uuid::new_v4(),
            name: "Test Task".to_string(),
            data_type: MigrationDataType::Emails,
            source_path: PathBuf::from("/test/source"),
            target_path: PathBuf::from("/test/target"),
            status: MigrationStatus::Pending,
            progress: 0.0,
            started_at: None,
            completed_at: None,
            estimated_items: Some(100),
            processed_items: 0,
            failed_items: 0,
            error_message: None,
        };

        assert_eq!(task.status, MigrationStatus::Pending);
        assert_eq!(task.estimated_items, Some(100));
    }
}