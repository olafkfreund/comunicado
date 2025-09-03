//! Comprehensive backup and synchronization system
//!
//! This module provides enterprise-grade backup and sync capabilities including:
//! - Full and incremental backups
//! - Version control and restore points
//! - Remote synchronization
//! - Automated scheduling
//! - Compression and encryption
//! - Multi-device sync

pub mod backup_engine;
pub mod backup_manager;
pub mod backup_ui;
pub mod sync_engine;
pub mod scheduler;
pub mod compression;
pub mod encryption;
pub mod versioning;
pub mod remote_sync;

pub use backup_engine::{
    BackupEngine, BackupConfig, BackupResult, BackupError, BackupType, BackupStatus, BackupMetadata,
};
pub use backup_manager::{
    BackupManager, BackupPlan, BackupRestorePoint,
};
pub use backup_ui::{BackupUI, BackupAction, BackupTab};
pub use sync_engine::{
    SyncEngine, SyncConfig, SyncResult, SyncError, SyncDirection, ConflictResolution,
};
pub use scheduler::{
    BackupScheduler, ScheduleConfig, ScheduleFrequency, ScheduledTask,
};
pub use compression::{CompressionEngine, CompressionType, CompressionLevel};
pub use encryption::{EncryptionEngine, EncryptionType, KeyDerivation};
pub use versioning::{VersionManager, Version, VersionHistory, VersionDiff};
pub use remote_sync::{RemoteSyncProvider, RemoteConfig, RemoteCredentials};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

/// Backup target types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BackupTarget {
    /// Local filesystem backup
    Local(PathBuf),
    /// Remote cloud storage
    Remote(RemoteConfig),
    /// Network attached storage
    Network(NetworkConfig),
    /// Multiple targets for redundancy
    Redundant(Vec<BackupTarget>),
}

/// Network storage configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NetworkConfig {
    pub protocol: NetworkProtocol,
    pub host: String,
    pub port: Option<u16>,
    pub path: PathBuf,
    pub credentials: Option<NetworkCredentials>,
}

/// Network protocols for backup
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NetworkProtocol {
    Ssh,
    Ftp,
    Sftp,
    Nfs,
    Smb,
}

/// Network authentication credentials
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NetworkCredentials {
    pub username: String,
    pub password: Option<String>,
    pub key_file: Option<PathBuf>,
}

/// Data categories for selective backup
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DataCategory {
    /// Email messages and metadata
    Emails,
    /// Calendar events and schedules
    Calendar,
    /// Contact information
    Contacts,
    /// Application configuration
    Configuration,
    /// User preferences and customizations
    Settings,
    /// Email filters and rules
    Filters,
    /// Account credentials and authentication
    Accounts,
    /// Plugin data and settings
    Plugins,
    /// Themes and customizations
    Themes,
    /// Search indexes and caches
    Indexes,
    /// All data categories
    All,
}

impl DataCategory {
    pub fn all_categories() -> Vec<DataCategory> {
        vec![
            DataCategory::Emails,
            DataCategory::Calendar,
            DataCategory::Contacts,
            DataCategory::Configuration,
            DataCategory::Settings,
            DataCategory::Filters,
            DataCategory::Accounts,
            DataCategory::Plugins,
            DataCategory::Themes,
            DataCategory::Indexes,
        ]
    }

    pub fn description(&self) -> &'static str {
        match self {
            DataCategory::Emails => "Email messages and attachments",
            DataCategory::Calendar => "Calendar events and appointments",
            DataCategory::Contacts => "Contact information and address books",
            DataCategory::Configuration => "Application configuration files",
            DataCategory::Settings => "User preferences and customizations",
            DataCategory::Filters => "Email filters and rules",
            DataCategory::Accounts => "Account settings and credentials",
            DataCategory::Plugins => "Plugin data and configurations",
            DataCategory::Themes => "Themes and visual customizations",
            DataCategory::Indexes => "Search indexes and performance caches",
            DataCategory::All => "All user data and settings",
        }
    }
}