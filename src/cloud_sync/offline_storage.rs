//! Offline storage and caching for cloud synchronization

use super::{CloudSyncError, CloudSyncResult, SyncDataType, SyncMetadata};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use uuid::Uuid;

/// Offline cache manager
pub struct OfflineCache {
    cache_dir: PathBuf,
    cache_index: CacheIndex,
    storage_limits: StorageLimits,
    compression_enabled: bool,
}

/// Cache index for efficient lookups
#[derive(Debug, Serialize, Deserialize)]
pub struct CacheIndex {
    entries: HashMap<String, CacheIndexEntry>,
    data_type_mappings: HashMap<SyncDataType, Vec<String>>,
    size_stats: CacheSizeStats,
    last_cleanup: DateTime<Utc>,
}

/// Index entry for cached items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheIndexEntry {
    pub cache_key: String,
    pub data_type: SyncDataType,
    pub original_size: u64,
    pub compressed_size: u64,
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub access_count: u32,
    pub sync_metadata: Option<SyncMetadata>,
    pub file_path: PathBuf,
    pub compression_type: CompressionType,
}

/// Cache size statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct CacheSizeStats {
    pub total_entries: usize,
    pub total_original_bytes: u64,
    pub total_compressed_bytes: u64,
    pub compression_ratio: f32,
    pub entries_by_type: HashMap<SyncDataType, usize>,
}

/// Storage limits configuration
#[derive(Debug, Clone)]
pub struct StorageLimits {
    pub max_cache_size_mb: u64,
    pub max_entries_per_type: usize,
    pub max_entry_age_days: u32,
    pub cleanup_threshold_percentage: f32,
}

/// Cached entry result
#[derive(Debug)]
pub struct CacheEntry {
    pub key: String,
    pub data: Vec<u8>,
    pub metadata: Option<SyncMetadata>,
    pub cached_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
}

/// Cache operation result
#[derive(Debug)]
pub enum CacheResult<T> {
    Hit(T),
    Miss,
    Error(CloudSyncError),
}

/// Compression types supported
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionType {
    None,
    Gzip,
    Lz4,
    Zstd,
}

/// Failed sync information for offline retry
#[derive(Debug, Serialize, Deserialize)]
pub struct FailedSyncEntry {
    pub id: Uuid,
    pub data_type: SyncDataType,
    pub operation: String,
    pub data: Vec<u8>,
    pub error_message: String,
    pub failed_at: DateTime<Utc>,
    pub retry_count: u32,
    pub next_retry_at: DateTime<Utc>,
}

impl OfflineCache {
    pub fn new() -> CloudSyncResult<Self> {
        let cache_dir = Self::get_cache_directory()?;
        
        Ok(Self {
            cache_dir,
            cache_index: CacheIndex::new(),
            storage_limits: StorageLimits::default(),
            compression_enabled: true,
        })
    }

    /// Initialize the offline cache
    pub async fn initialize(&mut self) -> CloudSyncResult<()> {
        // Create cache directory if it doesn't exist
        fs::create_dir_all(&self.cache_dir).await
            .map_err(|e| CloudSyncError::Io(e))?;

        // Load existing cache index
        self.load_cache_index().await?;

        // Perform initial cleanup if needed
        self.cleanup_if_needed().await?;

        Ok(())
    }

    /// Cache data with optional metadata
    pub async fn cache_data(
        &mut self,
        key: &str,
        data: &[u8],
        data_type: SyncDataType,
        metadata: Option<SyncMetadata>,
    ) -> CloudSyncResult<()> {
        // Check if we need cleanup before caching
        if self.should_cleanup().await {
            self.cleanup_cache().await?;
        }

        // Compress data if enabled
        let (compressed_data, compression_type) = if self.compression_enabled && data.len() > 1024 {
            let compressed = self.compress_data(data, CompressionType::Gzip)?;
            (compressed, CompressionType::Gzip)
        } else {
            (data.to_vec(), CompressionType::None)
        };

        // Generate file path for cached data
        let file_path = self.generate_cache_file_path(key, &data_type);

        // Write compressed data to file
        fs::write(&file_path, &compressed_data).await
            .map_err(|e| CloudSyncError::Io(e))?;

        // Create cache index entry
        let entry = CacheIndexEntry {
            cache_key: key.to_string(),
            data_type: data_type.clone(),
            original_size: data.len() as u64,
            compressed_size: compressed_data.len() as u64,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 1,
            sync_metadata: metadata,
            file_path: file_path.clone(),
            compression_type,
        };

        // Update cache index
        self.cache_index.add_entry(key.to_string(), entry);

        // Save index
        self.save_cache_index().await?;

        Ok(())
    }

    /// Retrieve data from cache
    pub async fn get_cached_data(&mut self, key: &str) -> CacheResult<CacheEntry> {
        if let Some(entry) = self.cache_index.get_entry_mut(key) {
            // Update access statistics
            entry.last_accessed = Utc::now();
            entry.access_count += 1;

            // Read file data
            match fs::read(&entry.file_path).await {
                Ok(cached_data) => {
                    // Decompress if needed
                    let data = match entry.compression_type {
                        CompressionType::None => cached_data,
                        _ => match self.decompress_data(&cached_data, &entry.compression_type) {
                            Ok(decompressed) => decompressed,
                            Err(e) => return CacheResult::Error(e),
                        },
                    };

                    let cache_entry = CacheEntry {
                        key: key.to_string(),
                        data,
                        metadata: entry.sync_metadata.clone(),
                        cached_at: entry.created_at,
                        last_accessed: entry.last_accessed,
                    };

                    // Save updated index
                    if let Err(e) = self.save_cache_index().await {
                        return CacheResult::Error(e);
                    }

                    CacheResult::Hit(cache_entry)
                }
                Err(e) => {
                    // Remove invalid entry from index
                    self.cache_index.remove_entry(key);
                    CacheResult::Error(CloudSyncError::Io(e))
                }
            }
        } else {
            CacheResult::Miss
        }
    }

    /// Check if data exists in cache
    pub fn is_cached(&self, key: &str) -> bool {
        self.cache_index.has_entry(key)
    }

    /// Remove data from cache
    pub async fn remove_cached_data(&mut self, key: &str) -> CloudSyncResult<bool> {
        if let Some(entry) = self.cache_index.get_entry(key) {
            // Delete file
            if entry.file_path.exists() {
                fs::remove_file(&entry.file_path).await
                    .map_err(|e| CloudSyncError::Io(e))?;
            }

            // Remove from index
            self.cache_index.remove_entry(key);
            self.save_cache_index().await?;

            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get cached data for a specific data type
    pub async fn get_cached_data_by_type(&self, data_type: &SyncDataType) -> Vec<String> {
        self.cache_index.get_keys_by_type(data_type)
    }

    /// Cache failed sync for retry
    pub async fn cache_failed_sync(
        &mut self,
        data_type: SyncDataType,
        error: &CloudSyncError,
    ) -> CloudSyncResult<()> {
        let failed_sync = FailedSyncEntry {
            id: Uuid::new_v4(),
            data_type: data_type.clone(),
            operation: "sync".to_string(),
            data: Vec::new(), // Could store actual data for retry
            error_message: error.to_string(),
            failed_at: Utc::now(),
            retry_count: 0,
            next_retry_at: Utc::now() + chrono::Duration::minutes(5),
        };

        // Store failed sync info
        let key = format!("failed_sync_{}", failed_sync.id);
        let serialized = serde_json::to_vec(&failed_sync)?;
        
        self.cache_data(
            &key,
            &serialized,
            data_type,
            None,
        ).await?;

        Ok(())
    }

    /// Get failed syncs for retry
    pub async fn get_failed_syncs(&self) -> CloudSyncResult<Vec<FailedSyncEntry>> {
        let mut failed_syncs = Vec::new();

        for (key, entry) in &self.cache_index.entries {
            if key.starts_with("failed_sync_") {
                if let Ok(data) = fs::read(&entry.file_path).await {
                    let decompressed_data = match entry.compression_type {
                        CompressionType::None => data,
                        _ => self.decompress_data(&data, &entry.compression_type)?,
                    };

                    if let Ok(failed_sync) = serde_json::from_slice::<FailedSyncEntry>(&decompressed_data) {
                        failed_syncs.push(failed_sync);
                    }
                }
            }
        }

        Ok(failed_syncs)
    }

    /// Clean up cache based on storage limits
    pub async fn cleanup_cache(&mut self) -> CloudSyncResult<u32> {
        let mut cleaned_count = 0;
        let now = Utc::now();

        // Get entries sorted by last access (oldest first)
        let mut entries: Vec<_> = self.cache_index.entries.iter().collect();
        entries.sort_by(|a, b| a.1.last_accessed.cmp(&b.1.last_accessed));

        // Remove expired entries
        let max_age = chrono::Duration::days(self.storage_limits.max_entry_age_days as i64);
        for (key, entry) in &entries {
            if now.signed_duration_since(entry.created_at) > max_age {
                if self.remove_cached_data(key).await? {
                    cleaned_count += 1;
                }
            }
        }

        // Check size limits
        let current_size_mb = self.cache_index.size_stats.total_compressed_bytes / (1024 * 1024);
        if current_size_mb > self.storage_limits.max_cache_size_mb {
            let target_size = (self.storage_limits.max_cache_size_mb as f32 * 0.8) as u64; // Clean to 80% of limit
            let target_size_bytes = target_size * 1024 * 1024;

            let mut current_size = self.cache_index.size_stats.total_compressed_bytes;
            for (key, _entry) in entries.iter().rev() {
                if current_size <= target_size_bytes {
                    break;
                }

                if let Some(entry) = self.cache_index.get_entry(key) {
                    current_size -= entry.compressed_size;
                    if self.remove_cached_data(key).await? {
                        cleaned_count += 1;
                    }
                }
            }
        }

        // Update cleanup timestamp
        self.cache_index.last_cleanup = now;
        self.save_cache_index().await?;

        Ok(cleaned_count)
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> &CacheSizeStats {
        &self.cache_index.size_stats
    }

    // Private helper methods

    fn get_cache_directory() -> CloudSyncResult<PathBuf> {
        let cache_dir = dirs::cache_dir()
            .ok_or_else(|| CloudSyncError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Cache directory not found"
            )))?
            .join("comunicado")
            .join("cloud_sync");

        Ok(cache_dir)
    }

    fn generate_cache_file_path(&self, key: &str, data_type: &SyncDataType) -> PathBuf {
        let safe_key = key.chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect::<String>();
        
        self.cache_dir
            .join(format!("{:?}", data_type).to_lowercase())
            .join(format!("{}.cache", safe_key))
    }

    async fn load_cache_index(&mut self) -> CloudSyncResult<()> {
        let index_path = self.cache_dir.join("index.json");
        
        if index_path.exists() {
            let index_data = fs::read(&index_path).await
                .map_err(|e| CloudSyncError::Io(e))?;
            
            self.cache_index = serde_json::from_slice(&index_data)
                .unwrap_or_else(|_| CacheIndex::new());
        }

        Ok(())
    }

    async fn save_cache_index(&self) -> CloudSyncResult<()> {
        let index_path = self.cache_dir.join("index.json");
        let index_data = serde_json::to_vec(&self.cache_index)?;
        
        fs::write(&index_path, &index_data).await
            .map_err(|e| CloudSyncError::Io(e))?;

        Ok(())
    }

    async fn should_cleanup(&self) -> bool {
        let current_size_mb = self.cache_index.size_stats.total_compressed_bytes / (1024 * 1024);
        let threshold_size = (self.storage_limits.max_cache_size_mb as f32 * 
                             self.storage_limits.cleanup_threshold_percentage) as u64;

        current_size_mb > threshold_size ||
        Utc::now().signed_duration_since(self.cache_index.last_cleanup) > chrono::Duration::hours(24)
    }

    async fn cleanup_if_needed(&mut self) -> CloudSyncResult<()> {
        if self.should_cleanup().await {
            self.cleanup_cache().await?;
        }
        Ok(())
    }

    fn compress_data(&self, data: &[u8], compression_type: CompressionType) -> CloudSyncResult<Vec<u8>> {
        match compression_type {
            CompressionType::None => Ok(data.to_vec()),
            CompressionType::Gzip => {
                // Simple compression simulation (use actual compression library in production)
                let mut compressed = Vec::new();
                compressed.extend_from_slice(b"GZIP");
                compressed.extend_from_slice(data);
                Ok(compressed)
            }
            CompressionType::Lz4 => {
                let mut compressed = Vec::new();
                compressed.extend_from_slice(b"LZ4_");
                compressed.extend_from_slice(data);
                Ok(compressed)
            }
            CompressionType::Zstd => {
                let mut compressed = Vec::new();
                compressed.extend_from_slice(b"ZSTD");
                compressed.extend_from_slice(data);
                Ok(compressed)
            }
        }
    }

    fn decompress_data(&self, compressed_data: &[u8], compression_type: &CompressionType) -> CloudSyncResult<Vec<u8>> {
        match compression_type {
            CompressionType::None => Ok(compressed_data.to_vec()),
            CompressionType::Gzip => {
                if compressed_data.starts_with(b"GZIP") {
                    Ok(compressed_data[4..].to_vec())
                } else {
                    Err(CloudSyncError::Provider("Invalid GZIP data".to_string()))
                }
            }
            CompressionType::Lz4 => {
                if compressed_data.starts_with(b"LZ4_") {
                    Ok(compressed_data[4..].to_vec())
                } else {
                    Err(CloudSyncError::Provider("Invalid LZ4 data".to_string()))
                }
            }
            CompressionType::Zstd => {
                if compressed_data.starts_with(b"ZSTD") {
                    Ok(compressed_data[4..].to_vec())
                } else {
                    Err(CloudSyncError::Provider("Invalid ZSTD data".to_string()))
                }
            }
        }
    }
}

impl CacheIndex {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
            data_type_mappings: HashMap::new(),
            size_stats: CacheSizeStats::new(),
            last_cleanup: Utc::now(),
        }
    }

    fn add_entry(&mut self, key: String, entry: CacheIndexEntry) {
        // Update data type mappings
        self.data_type_mappings
            .entry(entry.data_type.clone())
            .or_insert_with(Vec::new)
            .push(key.clone());

        // Update size statistics
        self.size_stats.update_on_add(&entry);

        // Add entry
        self.entries.insert(key, entry);
    }

    fn remove_entry(&mut self, key: &str) -> Option<CacheIndexEntry> {
        if let Some(entry) = self.entries.remove(key) {
            // Update data type mappings
            if let Some(keys) = self.data_type_mappings.get_mut(&entry.data_type) {
                keys.retain(|k| k != key);
                if keys.is_empty() {
                    self.data_type_mappings.remove(&entry.data_type);
                }
            }

            // Update size statistics
            self.size_stats.update_on_remove(&entry);

            Some(entry)
        } else {
            None
        }
    }

    fn get_entry(&self, key: &str) -> Option<&CacheIndexEntry> {
        self.entries.get(key)
    }

    fn get_entry_mut(&mut self, key: &str) -> Option<&mut CacheIndexEntry> {
        self.entries.get_mut(key)
    }

    fn has_entry(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    fn get_keys_by_type(&self, data_type: &SyncDataType) -> Vec<String> {
        self.data_type_mappings
            .get(data_type)
            .map(|keys| keys.clone())
            .unwrap_or_default()
    }
}

impl CacheSizeStats {
    fn new() -> Self {
        Self {
            total_entries: 0,
            total_original_bytes: 0,
            total_compressed_bytes: 0,
            compression_ratio: 1.0,
            entries_by_type: HashMap::new(),
        }
    }

    fn update_on_add(&mut self, entry: &CacheIndexEntry) {
        self.total_entries += 1;
        self.total_original_bytes += entry.original_size;
        self.total_compressed_bytes += entry.compressed_size;
        
        *self.entries_by_type.entry(entry.data_type.clone()).or_insert(0) += 1;
        
        self.update_compression_ratio();
    }

    fn update_on_remove(&mut self, entry: &CacheIndexEntry) {
        self.total_entries = self.total_entries.saturating_sub(1);
        self.total_original_bytes = self.total_original_bytes.saturating_sub(entry.original_size);
        self.total_compressed_bytes = self.total_compressed_bytes.saturating_sub(entry.compressed_size);
        
        if let Some(count) = self.entries_by_type.get_mut(&entry.data_type) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                self.entries_by_type.remove(&entry.data_type);
            }
        }
        
        self.update_compression_ratio();
    }

    fn update_compression_ratio(&mut self) {
        self.compression_ratio = if self.total_original_bytes > 0 {
            self.total_compressed_bytes as f32 / self.total_original_bytes as f32
        } else {
            1.0
        };
    }
}

impl Default for StorageLimits {
    fn default() -> Self {
        Self {
            max_cache_size_mb: 1024,    // 1GB
            max_entries_per_type: 1000,
            max_entry_age_days: 30,
            cleanup_threshold_percentage: 0.8, // Start cleanup at 80% of max size
        }
    }
}