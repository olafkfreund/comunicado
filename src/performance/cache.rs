//! Intelligent caching system for messages and folders
//!
//! This module provides high-performance caching for email messages, folders,
//! and metadata to reduce IMAP requests and improve UI responsiveness.

use crate::email::database::{StoredMessage, FolderSyncState};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Maximum number of cached messages per folder
const MAX_MESSAGES_PER_FOLDER: usize = 1000;
/// Cache expiry time for messages
const MESSAGE_CACHE_TTL: Duration = Duration::from_secs(300); // 5 minutes
/// Cache expiry time for folder metadata
const FOLDER_CACHE_TTL: Duration = Duration::from_secs(60); // 1 minute

/// Message cache for storing frequently accessed messages
pub struct MessageCache {
    /// Cached messages organized by account and folder
    cache: Arc<RwLock<HashMap<String, FolderMessageCache>>>,
    /// Cache statistics
    stats: Arc<Mutex<CacheStats>>,
}

/// Cache for messages within a specific folder
struct FolderMessageCache {
    messages: HashMap<u32, CachedMessage>,
    last_updated: Instant,
}

/// Cached message with metadata
#[derive(Clone)]
struct CachedMessage {
    message: StoredMessage,
    cached_at: Instant,
    access_count: u32,
}

/// Folder metadata cache
pub struct FolderCache {
    /// Cached folder states
    cache: Arc<RwLock<HashMap<String, CachedFolderState>>>,
    /// Cache statistics
    stats: Arc<Mutex<CacheStats>>,
}

/// Cached folder state with metadata
#[derive(Clone)]
struct CachedFolderState {
    state: FolderSyncState,
    cached_at: Instant,
    access_count: u32,
}

/// Cache statistics for monitoring performance
#[derive(Debug, Default, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub total_requests: u64,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.hits as f64 / self.total_requests as f64
        }
    }
}

/// Comprehensive cache manager
pub struct CacheManager {
    message_cache: MessageCache,
    folder_cache: FolderCache,
    /// Global cache settings
    settings: CacheSettings,
}

/// Cache configuration settings
#[derive(Debug, Clone)]
pub struct CacheSettings {
    pub max_memory_mb: usize,
    pub message_ttl: Duration,
    pub folder_ttl: Duration,
    pub enable_preloading: bool,
    pub cleanup_interval: Duration,
}

impl Default for CacheSettings {
    fn default() -> Self {
        Self {
            max_memory_mb: 100, // 100MB default cache size
            message_ttl: MESSAGE_CACHE_TTL,
            folder_ttl: FOLDER_CACHE_TTL,
            enable_preloading: true,
            cleanup_interval: Duration::from_secs(60),
        }
    }
}

impl MessageCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(Mutex::new(CacheStats::default())),
        }
    }

    /// Get a message from cache
    pub async fn get_message(
        &self,
        account_id: &str,
        folder_name: &str,
        uid: u32,
    ) -> Option<StoredMessage> {
        let cache_key = format!("{}:{}", account_id, folder_name);
        
        {
            let mut cache = self.cache.write().unwrap();
            if let Some(folder_cache) = cache.get_mut(&cache_key) {
                if let Some(cached_msg) = folder_cache.messages.get_mut(&uid) {
                    // Check if message is still valid
                    if cached_msg.cached_at.elapsed() < MESSAGE_CACHE_TTL {
                        cached_msg.access_count += 1;
                        
                        // Update statistics
                        tokio::spawn({
                            let stats = self.stats.clone();
                            async move {
                                let mut stats = stats.lock().await;
                                stats.hits += 1;
                                stats.total_requests += 1;
                            }
                        });
                        
                        return Some(cached_msg.message.clone());
                    } else {
                        // Remove expired message
                        folder_cache.messages.remove(&uid);
                    }
                }
            }
        }

        // Update miss statistics
        tokio::spawn({
            let stats = self.stats.clone();
            async move {
                let mut stats = stats.lock().await;
                stats.misses += 1;
                stats.total_requests += 1;
            }
        });

        None
    }

    /// Cache a message
    pub async fn cache_message(
        &self,
        account_id: &str,
        folder_name: &str,
        message: StoredMessage,
    ) {
        let cache_key = format!("{}:{}", account_id, folder_name);
        let uid = message.id.as_u128() as u32;
        
        let cached_message = CachedMessage {
            message,
            cached_at: Instant::now(),
            access_count: 0,
        };

        {
            let mut cache = self.cache.write().unwrap();
            let folder_cache = cache.entry(cache_key).or_insert_with(|| {
                FolderMessageCache {
                    messages: HashMap::new(),
                    last_updated: Instant::now(),
                }
            });

            // Limit cache size per folder
            if folder_cache.messages.len() >= MAX_MESSAGES_PER_FOLDER {
                // Remove oldest entries (simplified LRU)
                let oldest_uid = folder_cache.messages.iter()
                    .min_by_key(|(_, cached)| cached.cached_at)
                    .map(|(uid, _)| *uid);
                
                if let Some(uid) = oldest_uid {
                    folder_cache.messages.remove(&uid);
                }
            }

            folder_cache.messages.insert(uid, cached_message);
            folder_cache.last_updated = Instant::now();
        }
    }

    /// Get multiple messages from cache
    pub async fn get_messages_batch(
        &self,
        account_id: &str,
        folder_name: &str,
        uids: &[u32],
    ) -> (Vec<StoredMessage>, Vec<u32>) {
        let mut cached_messages = Vec::new();
        let mut missing_uids = Vec::new();

        for &uid in uids {
            if let Some(message) = self.get_message(account_id, folder_name, uid).await {
                cached_messages.push(message);
            } else {
                missing_uids.push(uid);
            }
        }

        (cached_messages, missing_uids)
    }

    /// Cache multiple messages
    pub async fn cache_messages_batch(
        &self,
        account_id: &str,
        folder_name: &str,
        messages: Vec<StoredMessage>,
    ) {
        for message in messages {
            self.cache_message(account_id, folder_name, message).await;
        }
    }

    /// Clear cache for a specific folder
    pub async fn clear_folder_cache(&self, account_id: &str, folder_name: &str) {
        let cache_key = format!("{}:{}", account_id, folder_name);
        
        {
            let mut cache = self.cache.write().unwrap();
            cache.remove(&cache_key);
        }
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> CacheStats {
        let stats = self.stats.lock().await;
        stats.clone()
    }

    /// Cleanup expired entries
    pub async fn cleanup_expired(&self) {
        let mut cache = self.cache.write().unwrap();
        
        cache.retain(|_, folder_cache| {
            folder_cache.messages.retain(|_, cached_msg| {
                let expired = cached_msg.cached_at.elapsed() > MESSAGE_CACHE_TTL;
                if expired {
                    tokio::spawn({
                        let stats = self.stats.clone();
                        async move {
                            let mut stats = stats.lock().await;
                            stats.evictions += 1;
                        }
                    });
                }
                !expired
            });
            
            // Keep folder cache if it has messages or was recently updated
            !folder_cache.messages.is_empty() || folder_cache.last_updated.elapsed() < Duration::from_secs(300)
        });
    }
}

impl FolderCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(Mutex::new(CacheStats::default())),
        }
    }

    /// Get folder state from cache
    pub async fn get_folder_state(
        &self,
        account_id: &str,
        folder_name: &str,
    ) -> Option<FolderSyncState> {
        let cache_key = format!("{}:{}", account_id, folder_name);
        
        {
            let mut cache = self.cache.write().unwrap();
            if let Some(cached_state) = cache.get_mut(&cache_key) {
                if cached_state.cached_at.elapsed() < FOLDER_CACHE_TTL {
                    cached_state.access_count += 1;
                    
                    // Update statistics
                    tokio::spawn({
                        let stats = self.stats.clone();
                        async move {
                            let mut stats = stats.lock().await;
                            stats.hits += 1;
                            stats.total_requests += 1;
                        }
                    });
                    
                    return Some(cached_state.state.clone());
                } else {
                    // Remove expired state
                    cache.remove(&cache_key);
                }
            }
        }

        // Update miss statistics
        tokio::spawn({
            let stats = self.stats.clone();
            async move {
                let mut stats = stats.lock().await;
                stats.misses += 1;
                stats.total_requests += 1;
            }
        });

        None
    }

    /// Cache folder state
    pub async fn cache_folder_state(
        &self,
        account_id: &str,
        folder_name: &str,
        state: FolderSyncState,
    ) {
        let cache_key = format!("{}:{}", account_id, folder_name);
        
        let cached_state = CachedFolderState {
            state,
            cached_at: Instant::now(),
            access_count: 0,
        };

        {
            let mut cache = self.cache.write().unwrap();
            cache.insert(cache_key, cached_state);
        }
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> CacheStats {
        let stats = self.stats.lock().await;
        stats.clone()
    }

    /// Cleanup expired entries
    pub async fn cleanup_expired(&self) {
        let mut cache = self.cache.write().unwrap();
        
        cache.retain(|_, cached_state| {
            let expired = cached_state.cached_at.elapsed() > FOLDER_CACHE_TTL;
            if expired {
                tokio::spawn({
                    let stats = self.stats.clone();
                    async move {
                        let mut stats = stats.lock().await;
                        stats.evictions += 1;
                    }
                });
            }
            !expired
        });
    }
}

impl CacheManager {
    pub fn new() -> Self {
        Self {
            message_cache: MessageCache::new(),
            folder_cache: FolderCache::new(),
            settings: CacheSettings::default(),
        }
    }

    pub fn with_settings(settings: CacheSettings) -> Self {
        Self {
            message_cache: MessageCache::new(),
            folder_cache: FolderCache::new(),
            settings,
        }
    }

    /// Get message cache
    pub fn message_cache(&self) -> &MessageCache {
        &self.message_cache
    }

    /// Get folder cache
    pub fn folder_cache(&self) -> &FolderCache {
        &self.folder_cache
    }

    /// Start background cleanup task
    pub async fn start_cleanup_task(&self) {
        let message_cache = self.message_cache.cache.clone();
        let folder_cache = self.folder_cache.cache.clone();
        let interval = self.settings.cleanup_interval;
        
        tokio::spawn(async move {
            let mut cleanup_interval = tokio::time::interval(interval);
            
            loop {
                cleanup_interval.tick().await;
                
                // Cleanup message cache
                {
                    let mut cache = message_cache.write().unwrap();
                    
                    cache.retain(|_, folder_cache| {
                        folder_cache.messages.retain(|_, cached_msg| {
                            cached_msg.cached_at.elapsed() <= MESSAGE_CACHE_TTL
                        });
                        
                        !folder_cache.messages.is_empty() || 
                        folder_cache.last_updated.elapsed() <= Duration::from_secs(300)
                    });
                }
                
                // Cleanup folder cache
                {
                    let mut cache = folder_cache.write().unwrap();
                    
                    cache.retain(|_, cached_state| {
                        cached_state.cached_at.elapsed() <= FOLDER_CACHE_TTL
                    });
                }
            }
        });
    }

    /// Get comprehensive cache statistics
    pub async fn get_comprehensive_stats(&self) -> (CacheStats, CacheStats) {
        let message_stats = self.message_cache.get_stats().await;
        let folder_stats = self.folder_cache.get_stats().await;
        (message_stats, folder_stats)
    }

    /// Clear all caches
    pub async fn clear_all_caches(&self) {
        {
            let mut cache = self.message_cache.cache.write().unwrap();
            cache.clear();
        }
        
        {
            let mut cache = self.folder_cache.cache.write().unwrap();
            cache.clear();
        }
    }
}