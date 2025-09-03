//! Version management for incremental backups

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Version {
    pub id: Uuid,
    pub number: u32,
    pub backup_id: Uuid,
    pub parent_version: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub files_changed: u64,
    pub bytes_changed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionHistory {
    pub backup_config_id: Uuid,
    pub versions: Vec<Version>,
    pub current_version: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDiff {
    pub from_version: Uuid,
    pub to_version: Uuid,
    pub added_files: Vec<String>,
    pub modified_files: Vec<String>,
    pub deleted_files: Vec<String>,
}

pub struct VersionManager {
    histories: HashMap<Uuid, VersionHistory>,
}

impl VersionManager {
    pub fn new() -> Self {
        Self {
            histories: HashMap::new(),
        }
    }

    pub fn create_version(&mut self, backup_config_id: Uuid, backup_id: Uuid) -> Version {
        let history = self.histories
            .entry(backup_config_id)
            .or_insert_with(|| VersionHistory {
                backup_config_id,
                versions: Vec::new(),
                current_version: None,
            });

        let version = Version {
            id: Uuid::new_v4(),
            number: history.versions.len() as u32 + 1,
            backup_id,
            parent_version: history.current_version,
            created_at: Utc::now(),
            files_changed: 0,
            bytes_changed: 0,
        };

        history.versions.push(version.clone());
        history.current_version = Some(version.id);

        version
    }

    pub fn get_version_diff(&self, _from: Uuid, _to: Uuid) -> Option<VersionDiff> {
        None // Placeholder
    }
}

impl Default for VersionManager {
    fn default() -> Self {
        Self::new()
    }
}