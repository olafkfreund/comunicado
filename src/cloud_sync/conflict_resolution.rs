//! Conflict resolution for cloud synchronization

use super::{CloudSyncError, CloudSyncResult, SyncDataType};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Conflict resolution strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictStrategy {
    /// Use the most recently modified version
    LastModified,
    /// Use the local version
    LocalWins,
    /// Use the remote version
    RemoteWins,
    /// Attempt to merge changes automatically
    Merge,
    /// Ask the user to resolve manually
    Manual,
    /// Use different strategies per data type
    PerType(HashMap<SyncDataType, ConflictStrategy>),
}

/// Conflict resolution result
#[derive(Debug, Serialize, Deserialize)]
pub enum MergeResult {
    /// Conflict resolved successfully
    Resolved(Vec<u8>),
    /// Conflict requires manual resolution
    RequiresManual {
        local_data: Vec<u8>,
        remote_data: Vec<u8>,
        conflict_info: ConflictInfo,
    },
    /// No conflict detected
    NoConflict,
}

/// Information about a detected conflict
#[derive(Debug, Serialize, Deserialize)]
pub struct ConflictInfo {
    pub data_type: SyncDataType,
    pub local_modified: DateTime<Utc>,
    pub remote_modified: DateTime<Utc>,
    pub conflict_type: ConflictType,
    pub affected_fields: Vec<String>,
}

/// Types of conflicts that can occur
#[derive(Debug, Serialize, Deserialize)]
pub enum ConflictType {
    /// Both versions modified simultaneously
    ConcurrentModification,
    /// Local version deleted, remote version modified
    LocalDeletedRemoteModified,
    /// Local version modified, remote version deleted
    LocalModifiedRemoteDeleted,
    /// Schema or format conflicts
    FormatMismatch,
    /// Data type conflicts
    TypeMismatch,
}

/// Conflict resolver implementation
pub struct ConflictResolver {
    strategy: ConflictStrategy,
    merger: DataMerger,
    conflict_history: ConflictHistory,
}

/// Data merger for automatic conflict resolution
pub struct DataMerger {
    merge_strategies: HashMap<SyncDataType, Box<dyn MergeStrategy>>,
}

/// Conflict history tracker
pub struct ConflictHistory {
    resolved_conflicts: Vec<ResolvedConflict>,
    manual_resolutions: Vec<ManualResolution>,
}

/// Information about a resolved conflict
#[derive(Debug, Serialize, Deserialize)]
pub struct ResolvedConflict {
    pub id: uuid::Uuid,
    pub data_type: SyncDataType,
    pub resolved_at: DateTime<Utc>,
    pub strategy_used: ConflictStrategy,
    pub resolution_time_ms: u64,
}

/// Manual conflict resolution record
#[derive(Debug, Serialize, Deserialize)]
pub struct ManualResolution {
    pub id: uuid::Uuid,
    pub data_type: SyncDataType,
    pub presented_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub user_choice: Option<UserResolutionChoice>,
}

/// User's choice for manual conflict resolution
#[derive(Debug, Serialize, Deserialize)]
pub enum UserResolutionChoice {
    UseLocal,
    UseRemote,
    UseMerged(Vec<u8>),
    SkipSync,
}

/// Trait for data type specific merge strategies
pub trait MergeStrategy: Send + Sync {
    fn can_merge(&self, local: &[u8], remote: &[u8]) -> bool;
    fn merge(&self, local: &[u8], _remote: &[u8]) -> CloudSyncResult<Vec<u8>>;
    fn detect_conflicts(&self, local: &[u8], remote: &[u8]) -> Vec<String>;
}

impl ConflictResolver {
    pub fn new(strategy: ConflictStrategy) -> CloudSyncResult<Self> {
        Ok(Self {
            strategy,
            merger: DataMerger::new()?,
            conflict_history: ConflictHistory::new(),
        })
    }

    /// Resolve conflicts between local and remote data
    pub async fn resolve(
        &mut self,
        local_data: &[u8],
        remote_data: &[u8],
        data_type: SyncDataType,
    ) -> CloudSyncResult<Vec<u8>> {
        let start_time = std::time::Instant::now();

        // Check if there's actually a conflict
        if local_data == remote_data {
            return Ok(local_data.to_vec());
        }

        // Detect conflict type and information
        let conflict_info = self.analyze_conflict(local_data, remote_data, data_type.clone())?;

        // Apply resolution strategy
        let result = match &self.strategy {
            ConflictStrategy::LastModified => {
                self.resolve_by_timestamp(local_data, remote_data, &conflict_info)
            }
            ConflictStrategy::LocalWins => Ok(local_data.to_vec()),
            ConflictStrategy::RemoteWins => Ok(remote_data.to_vec()),
            ConflictStrategy::Merge => {
                self.resolve_by_merge(local_data, remote_data, data_type.clone())
                    .await
            }
            ConflictStrategy::Manual => {
                // Return special result indicating manual resolution needed
                return Err(CloudSyncError::ConflictResolution(
                    "Manual conflict resolution required".to_string(),
                ));
            }
            ConflictStrategy::PerType(strategies) => {
                let strategy = strategies
                    .get(&data_type)
                    .unwrap_or(&ConflictStrategy::LastModified);
                match strategy {
                    ConflictStrategy::LastModified => {
                        self.resolve_by_timestamp(local_data, remote_data, &conflict_info)
                    }
                    ConflictStrategy::LocalWins => Ok(local_data.to_vec()),
                    ConflictStrategy::RemoteWins => Ok(remote_data.to_vec()),
                    ConflictStrategy::Merge => {
                        self.resolve_by_merge(local_data, remote_data, data_type.clone())
                            .await
                    }
                    _ => Ok(local_data.to_vec()), // Fallback to local
                }
            }
        };

        // Record resolution
        match &result {
            Ok(_) => {
                let resolution_time = start_time.elapsed().as_millis() as u64;
                self.conflict_history.record_resolution(ResolvedConflict {
                    id: uuid::Uuid::new_v4(),
                    data_type,
                    resolved_at: Utc::now(),
                    strategy_used: self.strategy.clone(),
                    resolution_time_ms: resolution_time,
                });
            }
            Err(_) => {
                // Record failed resolution
            }
        }

        result
    }

    /// Get conflict resolution statistics
    pub fn get_statistics(&self) -> ConflictResolutionStats {
        self.conflict_history.get_statistics()
    }

    /// Set resolution strategy
    pub fn set_strategy(&mut self, strategy: ConflictStrategy) {
        self.strategy = strategy;
    }

    fn analyze_conflict(
        &self,
        local_data: &[u8],
        remote_data: &[u8],
        data_type: SyncDataType,
    ) -> CloudSyncResult<ConflictInfo> {
        // Parse timestamps from data if available
        let local_modified = self.extract_timestamp(local_data).unwrap_or_else(Utc::now);
        let remote_modified = self.extract_timestamp(remote_data).unwrap_or_else(Utc::now);

        // Determine conflict type
        let conflict_type = if local_data.is_empty() && !remote_data.is_empty() {
            ConflictType::LocalDeletedRemoteModified
        } else if !local_data.is_empty() && remote_data.is_empty() {
            ConflictType::LocalModifiedRemoteDeleted
        } else {
            ConflictType::ConcurrentModification
        };

        // Detect affected fields
        let affected_fields = self.detect_field_differences(local_data, remote_data, &data_type)?;

        Ok(ConflictInfo {
            data_type,
            local_modified,
            remote_modified,
            conflict_type,
            affected_fields,
        })
    }

    fn resolve_by_timestamp(
        &self,
        local_data: &[u8],
        remote_data: &[u8],
        conflict_info: &ConflictInfo,
    ) -> CloudSyncResult<Vec<u8>> {
        if conflict_info.remote_modified > conflict_info.local_modified {
            Ok(remote_data.to_vec())
        } else {
            Ok(local_data.to_vec())
        }
    }

    async fn resolve_by_merge(
        &mut self,
        local_data: &[u8],
        remote_data: &[u8],
        data_type: SyncDataType,
    ) -> CloudSyncResult<Vec<u8>> {
        self.merger.merge(local_data, remote_data, data_type).await
    }

    fn extract_timestamp(&self, data: &[u8]) -> Option<DateTime<Utc>> {
        // Try to parse JSON and extract timestamp
        if let Ok(json_value) = serde_json::from_slice::<serde_json::Value>(data) {
            if let Some(timestamp_str) = json_value.get("updated_at") {
                if let Some(timestamp_str) = timestamp_str.as_str() {
                    return DateTime::parse_from_rfc3339(timestamp_str)
                        .ok()
                        .map(|dt| dt.with_timezone(&Utc));
                }
            }
        }
        None
    }

    fn detect_field_differences(
        &self,
        local_data: &[u8],
        remote_data: &[u8],
        _data_type: &SyncDataType,
    ) -> CloudSyncResult<Vec<String>> {
        let mut affected_fields = Vec::new();

        // Try to parse as JSON and compare fields
        if let (Ok(local_json), Ok(remote_json)) = (
            serde_json::from_slice::<serde_json::Value>(local_data),
            serde_json::from_slice::<serde_json::Value>(remote_data),
        ) {
            self.compare_json_objects(&local_json, &remote_json, "", &mut affected_fields);
        } else {
            // Fallback to binary comparison
            if local_data != remote_data {
                affected_fields.push("content".to_string());
            }
        }

        Ok(affected_fields)
    }

    fn compare_json_objects(
        &self,
        local: &serde_json::Value,
        remote: &serde_json::Value,
        path: &str,
        affected_fields: &mut Vec<String>,
    ) {
        match (local, remote) {
            (serde_json::Value::Object(local_obj), serde_json::Value::Object(remote_obj)) => {
                // Compare all keys from both objects
                let mut all_keys: std::collections::HashSet<&String> =
                    std::collections::HashSet::new();
                all_keys.extend(local_obj.keys());
                all_keys.extend(remote_obj.keys());

                for key in all_keys {
                    let new_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", path, key)
                    };

                    match (local_obj.get(key), remote_obj.get(key)) {
                        (Some(local_val), Some(remote_val)) => {
                            if local_val != remote_val {
                                self.compare_json_objects(
                                    local_val,
                                    remote_val,
                                    &new_path,
                                    affected_fields,
                                );
                            }
                        }
                        (Some(_), None) | (None, Some(_)) => {
                            affected_fields.push(new_path);
                        }
                        (None, None) => {} // This shouldn't happen due to the key iteration
                    }
                }
            }
            _ => {
                if local != remote {
                    affected_fields.push(path.to_string());
                }
            }
        }
    }
}

impl DataMerger {
    fn new() -> CloudSyncResult<Self> {
        let mut merge_strategies: HashMap<SyncDataType, Box<dyn MergeStrategy>> = HashMap::new();

        // Register merge strategies for each data type
        merge_strategies.insert(SyncDataType::Settings, Box::new(SettingsMergeStrategy));
        merge_strategies.insert(
            SyncDataType::EmailAccounts,
            Box::new(EmailAccountsMergeStrategy),
        );
        merge_strategies.insert(
            SyncDataType::KeyboardShortcuts,
            Box::new(KeyboardShortcutsMergeStrategy),
        );
        // Add more strategies as needed

        Ok(Self { merge_strategies })
    }

    async fn merge(
        &mut self,
        local_data: &[u8],
        remote_data: &[u8],
        data_type: SyncDataType,
    ) -> CloudSyncResult<Vec<u8>> {
        if let Some(strategy) = self.merge_strategies.get(&data_type) {
            if strategy.can_merge(local_data, remote_data) {
                strategy.merge(local_data, remote_data)
            } else {
                // Fallback to timestamp-based resolution
                Err(CloudSyncError::ConflictResolution(
                    "Automatic merge not possible".to_string(),
                ))
            }
        } else {
            // No specific strategy, use generic JSON merge
            self.generic_json_merge(local_data, remote_data)
        }
    }

    fn generic_json_merge(
        &self,
        local_data: &[u8],
        remote_data: &[u8],
    ) -> CloudSyncResult<Vec<u8>> {
        // Parse both as JSON
        let local_json: serde_json::Value = serde_json::from_slice(local_data).map_err(|e| {
            CloudSyncError::ConflictResolution(format!("Failed to parse local JSON: {}", e))
        })?;
        let remote_json: serde_json::Value = serde_json::from_slice(remote_data).map_err(|e| {
            CloudSyncError::ConflictResolution(format!("Failed to parse remote JSON: {}", e))
        })?;

        // Perform deep merge
        let merged = self.merge_json_values(&local_json, &remote_json);

        serde_json::to_vec(&merged).map_err(|e| {
            CloudSyncError::ConflictResolution(format!("Failed to serialize merged JSON: {}", e))
        })
    }

    fn merge_json_values(
        &self,
        local: &serde_json::Value,
        remote: &serde_json::Value,
    ) -> serde_json::Value {
        match (local, remote) {
            (serde_json::Value::Object(local_obj), serde_json::Value::Object(remote_obj)) => {
                let mut merged = serde_json::Map::new();

                // Add all local keys
                for (key, value) in local_obj {
                    merged.insert(key.clone(), value.clone());
                }

                // Merge remote keys
                for (key, remote_value) in remote_obj {
                    if let Some(local_value) = local_obj.get(key) {
                        // Merge if both exist
                        merged.insert(
                            key.clone(),
                            self.merge_json_values(local_value, remote_value),
                        );
                    } else {
                        // Use remote value if not in local
                        merged.insert(key.clone(), remote_value.clone());
                    }
                }

                serde_json::Value::Object(merged)
            }
            (serde_json::Value::Array(local_arr), serde_json::Value::Array(remote_arr)) => {
                // Merge arrays by combining unique elements
                let mut merged = local_arr.clone();
                for remote_item in remote_arr {
                    if !merged.contains(remote_item) {
                        merged.push(remote_item.clone());
                    }
                }
                serde_json::Value::Array(merged)
            }
            // For non-object/array values, prefer remote (more recent)
            _ => remote.clone(),
        }
    }
}

impl ConflictHistory {
    fn new() -> Self {
        Self {
            resolved_conflicts: Vec::new(),
            manual_resolutions: Vec::new(),
        }
    }

    fn record_resolution(&mut self, conflict: ResolvedConflict) {
        self.resolved_conflicts.push(conflict);

        // Keep only recent conflicts (last 1000)
        if self.resolved_conflicts.len() > 1000 {
            self.resolved_conflicts.drain(0..100);
        }
    }

    fn get_statistics(&self) -> ConflictResolutionStats {
        let total_conflicts = self.resolved_conflicts.len();
        let avg_resolution_time = if total_conflicts > 0 {
            self.resolved_conflicts
                .iter()
                .map(|c| c.resolution_time_ms)
                .sum::<u64>()
                / total_conflicts as u64
        } else {
            0
        };

        let mut by_strategy = HashMap::new();
        for conflict in &self.resolved_conflicts {
            *by_strategy
                .entry(format!("{:?}", conflict.strategy_used))
                .or_insert(0) += 1;
        }

        ConflictResolutionStats {
            total_conflicts,
            avg_resolution_time_ms: avg_resolution_time,
            conflicts_by_strategy: by_strategy,
            manual_resolutions_pending: self
                .manual_resolutions
                .iter()
                .filter(|r| r.resolved_at.is_none())
                .count(),
        }
    }
}

#[derive(Debug)]
pub struct ConflictResolutionStats {
    pub total_conflicts: usize,
    pub avg_resolution_time_ms: u64,
    pub conflicts_by_strategy: HashMap<String, usize>,
    pub manual_resolutions_pending: usize,
}

// Concrete merge strategies
struct SettingsMergeStrategy;
struct EmailAccountsMergeStrategy;
struct KeyboardShortcutsMergeStrategy;

impl MergeStrategy for SettingsMergeStrategy {
    fn can_merge(&self, local: &[u8], remote: &[u8]) -> bool {
        // Settings can usually be merged
        serde_json::from_slice::<serde_json::Value>(local).is_ok()
            && serde_json::from_slice::<serde_json::Value>(remote).is_ok()
    }

    fn merge(&self, local: &[u8], remote: &[u8]) -> CloudSyncResult<Vec<u8>> {
        // Parse as settings objects and merge
        let local_settings: serde_json::Value = serde_json::from_slice(local)?;
        let remote_settings: serde_json::Value = serde_json::from_slice(remote)?;

        // For settings, prefer remote values but keep local-only settings
        let merged = match (local_settings, remote_settings) {
            (serde_json::Value::Object(mut local_obj), serde_json::Value::Object(remote_obj)) => {
                for (key, value) in remote_obj {
                    local_obj.insert(key, value);
                }
                serde_json::Value::Object(local_obj)
            }
            (_, remote) => remote, // Fallback to remote
        };

        Ok(serde_json::to_vec(&merged)?)
    }

    fn detect_conflicts(&self, _local: &[u8], _remote: &[u8]) -> Vec<String> {
        Vec::new() // Settings rarely have true conflicts
    }
}

impl MergeStrategy for EmailAccountsMergeStrategy {
    fn can_merge(&self, local: &[u8], remote: &[u8]) -> bool {
        // Email accounts should be merged by account ID
        serde_json::from_slice::<serde_json::Value>(local).is_ok()
            && serde_json::from_slice::<serde_json::Value>(remote).is_ok()
    }

    fn merge(&self, local: &[u8], remote: &[u8]) -> CloudSyncResult<Vec<u8>> {
        // Implement account-specific merge logic
        // For now, use simple merge
        let local_accounts: serde_json::Value = serde_json::from_slice(local)?;
        let remote_accounts: serde_json::Value = serde_json::from_slice(remote)?;

        // Merge account arrays by ID
        match (local_accounts, remote_accounts) {
            (serde_json::Value::Array(local_arr), serde_json::Value::Array(remote_arr)) => {
                let mut merged = local_arr.clone();

                // Add or update remote accounts
                for remote_account in remote_arr {
                    if let Some(remote_id) = remote_account.get("id") {
                        // Find and replace or add
                        if let Some(pos) = merged
                            .iter()
                            .position(|local_account| local_account.get("id") == Some(remote_id))
                        {
                            merged[pos] = remote_account;
                        } else {
                            merged.push(remote_account);
                        }
                    }
                }

                Ok(serde_json::to_vec(&serde_json::Value::Array(merged))?)
            }
            (_, remote) => Ok(serde_json::to_vec(&remote)?), // Fallback
        }
    }

    fn detect_conflicts(&self, _local: &[u8], _remote: &[u8]) -> Vec<String> {
        Vec::new()
    }
}

impl MergeStrategy for KeyboardShortcutsMergeStrategy {
    fn can_merge(&self, local: &[u8], remote: &[u8]) -> bool {
        serde_json::from_slice::<serde_json::Value>(local).is_ok()
            && serde_json::from_slice::<serde_json::Value>(remote).is_ok()
    }

    fn merge(&self, local: &[u8], _remote: &[u8]) -> CloudSyncResult<Vec<u8>> {
        // For keyboard shortcuts, prefer local customizations
        Ok(local.to_vec())
    }

    fn detect_conflicts(&self, local: &[u8], remote: &[u8]) -> Vec<String> {
        if local != remote {
            vec!["shortcuts".to_string()]
        } else {
            Vec::new()
        }
    }
}
