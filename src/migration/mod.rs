//! Email client migration system
//! 
//! This module provides comprehensive migration capabilities for importing/exporting
//! emails, contacts, and settings from various email clients including:
//! - Thunderbird
//! - Outlook
//! - Apple Mail
//! - Gmail (via takeout)
//! - Mutt/Neomutt
//! - Evolution
//! - KMail
//! - And more...

pub mod client_detection;
pub mod thunderbird;
pub mod outlook;
pub mod apple_mail;
pub mod gmail_takeout;
pub mod mutt;
pub mod evolution;
pub mod kmail;
pub mod migration_engine;
pub mod migration_ui;
pub mod migration_wizard;
pub mod format_converters;
pub mod profile_detector;

pub use client_detection::{ClientDetector, DetectedClient, ClientInfo};
pub use thunderbird::{ThunderbirdMigrator, ThunderbirdProfile};
pub use outlook::{OutlookMigrator, OutlookProfile, PstReader};
pub use apple_mail::{AppleMailMigrator, AppleMailProfile};
pub use gmail_takeout::{GmailTakeoutMigrator, TakeoutArchive};
pub use mutt::{MuttMigrator, MuttConfig};
pub use evolution::{EvolutionMigrator, EvolutionProfile};
pub use kmail::{KMailMigrator, KMailProfile};
pub use migration_engine::{
    MigrationEngine, MigrationPlan, MigrationProgress, MigrationResult,
    MigrationError, MigrationTask, MigrationStatus,
};
pub use migration_ui::{MigrationUI, MigrationAction};
pub use migration_wizard::{MigrationWizard, WizardStep, WizardState};
pub use format_converters::{
    MboxConverter, EmlConverter, PstConverter, VcfConverter, IcsConverter,
};
pub use profile_detector::{ProfileDetector, EmailClientProfile};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

/// Supported email clients for migration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EmailClient {
    Thunderbird,
    Outlook,
    AppleMail,
    Gmail,
    Mutt,
    Neomutt,
    Evolution,
    KMail,
    Claws,
    Sylpheed,
    Alpine,
    Pine,
    Webmail(String), // Provider name
    Other(String),   // Custom client name
}

/// Types of data that can be migrated
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MigrationDataType {
    Emails,
    Contacts,
    Calendar,
    Filters,
    Settings,
    Accounts,
    Signatures,
    Templates,
    AddressBooks,
    RSS,
}

/// Migration strategy options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MigrationStrategy {
    /// Full migration - everything from source
    Complete,
    /// Selective migration - user chooses what to migrate
    Selective(Vec<MigrationDataType>),
    /// Incremental - only new data since last migration
    Incremental { since: DateTime<Utc> },
    /// Merge - combine with existing data
    Merge,
    /// Replace - overwrite existing data
    Replace,
}

/// Migration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationConfig {
    pub id: Uuid,
    pub name: String,
    pub source_client: EmailClient,
    pub source_path: PathBuf,
    pub target_path: PathBuf,
    pub strategy: MigrationStrategy,
    pub data_types: Vec<MigrationDataType>,
    pub preserve_folder_structure: bool,
    pub convert_formats: bool,
    pub validate_data: bool,
    pub create_backup: bool,
    pub overwrite_existing: bool,
    pub batch_size: usize,
    pub max_concurrent_tasks: usize,
    pub timeout_seconds: u64,
    pub created_at: DateTime<Utc>,
}

impl Default for MigrationConfig {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "New Migration".to_string(),
            source_client: EmailClient::Other("Unknown".to_string()),
            source_path: PathBuf::new(),
            target_path: PathBuf::new(),
            strategy: MigrationStrategy::Selective(vec![
                MigrationDataType::Emails,
                MigrationDataType::Contacts,
            ]),
            data_types: vec![MigrationDataType::Emails],
            preserve_folder_structure: true,
            convert_formats: true,
            validate_data: true,
            create_backup: true,
            overwrite_existing: false,
            batch_size: 100,
            max_concurrent_tasks: 4,
            timeout_seconds: 3600,
            created_at: Utc::now(),
        }
    }
}

/// Migration source information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationSource {
    pub client: EmailClient,
    pub version: Option<String>,
    pub profile_path: PathBuf,
    pub data_paths: HashMap<MigrationDataType, PathBuf>,
    pub estimated_size: Option<u64>,
    pub estimated_count: Option<usize>,
    pub last_modified: Option<DateTime<Utc>>,
    pub is_accessible: bool,
    pub permissions_ok: bool,
}

/// Migration target information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationTarget {
    pub account_id: String,
    pub database_path: PathBuf,
    pub storage_path: PathBuf,
    pub available_space: Option<u64>,
    pub existing_data: HashMap<MigrationDataType, usize>,
    pub conflicts: Vec<ConflictInfo>,
}

/// Information about potential conflicts during migration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictInfo {
    pub data_type: MigrationDataType,
    pub conflict_type: ConflictType,
    pub description: String,
    pub suggested_resolution: ConflictResolution,
}

/// Types of conflicts that can occur
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictType {
    DuplicateData,
    FormatIncompatibility,
    PathConflict,
    PermissionError,
    DiskSpaceInsufficient,
    VersionMismatch,
    CorruptedData,
}

/// Suggested resolutions for conflicts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictResolution {
    Skip,
    Overwrite,
    Merge,
    Rename,
    Convert,
    ManualReview,
}

/// Comprehensive migration statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MigrationStatistics {
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_seconds: Option<u64>,
    
    // Data type specific counts
    pub emails: DataTypeStats,
    pub contacts: DataTypeStats,
    pub calendar: DataTypeStats,
    pub filters: DataTypeStats,
    pub settings: DataTypeStats,
    pub accounts: DataTypeStats,
    
    // Overall statistics
    pub total_items_found: usize,
    pub total_items_migrated: usize,
    pub total_items_skipped: usize,
    pub total_items_failed: usize,
    pub total_size_bytes: u64,
    pub total_size_migrated: u64,
    
    // Performance metrics
    pub items_per_second: f64,
    pub bytes_per_second: f64,
    pub peak_memory_usage: Option<u64>,
    pub disk_io_operations: u64,
    
    // Error information
    pub errors: Vec<MigrationErrorInfo>,
    pub warnings: Vec<String>,
}

/// Statistics for each data type
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DataTypeStats {
    pub found: usize,
    pub migrated: usize,
    pub skipped: usize,
    pub failed: usize,
    pub size_bytes: u64,
    pub conversion_needed: usize,
    pub conflicts_resolved: usize,
}

/// Detailed error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationErrorInfo {
    pub timestamp: DateTime<Utc>,
    pub error_type: String,
    pub description: String,
    pub item_path: Option<PathBuf>,
    pub data_type: Option<MigrationDataType>,
    pub recoverable: bool,
    pub retry_count: usize,
}

/// Migration validation results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResults {
    pub is_valid: bool,
    pub validation_time: DateTime<Utc>,
    pub checks_performed: Vec<ValidationCheck>,
    pub issues_found: Vec<ValidationIssue>,
    pub recommendations: Vec<String>,
}

/// Individual validation check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationCheck {
    pub name: String,
    pub passed: bool,
    pub details: String,
    pub severity: ValidationSeverity,
}

/// Validation issue severity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Validation issue details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub severity: ValidationSeverity,
    pub category: String,
    pub description: String,
    pub affected_items: Vec<String>,
    pub suggested_fix: Option<String>,
    pub auto_fixable: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_config_creation() {
        let config = MigrationConfig::default();
        assert!(!config.id.is_nil());
        assert_eq!(config.name, "New Migration");
        assert!(config.preserve_folder_structure);
        assert!(config.validate_data);
    }

    #[test]
    fn test_email_client_serialization() {
        let client = EmailClient::Thunderbird;
        let serialized = serde_json::to_string(&client).unwrap();
        let deserialized: EmailClient = serde_json::from_str(&serialized).unwrap();
        assert_eq!(client, deserialized);
    }

    #[test]
    fn test_migration_statistics() {
        let mut stats = MigrationStatistics::default();
        stats.total_items_found = 1000;
        stats.total_items_migrated = 950;
        stats.total_items_failed = 50;
        
        assert_eq!(stats.total_items_found, 1000);
        assert_eq!(stats.total_items_migrated, 950);
    }
}