//! AI response caching system for performance optimization

use crate::ai::error::{AIError, AIResult};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use sha2::{Digest, Sha256};
use tracing::{debug, info, warn};

/// Cache invalidation strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InvalidationStrategy {
    /// Time-based expiration (TTL)
    TimeBasedTTL,
    /// Least Recently Used (LRU) eviction
    LeastRecentlyUsed,
    /// Least Frequently Used (LFU) eviction
    LeastFrequentlyUsed,
    /// Content-based invalidation (provider changes, etc.)
    ContentBased,
    /// Manual invalidation only
    Manual,
}

/// Cache entry priority level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CachePriority {
    /// Low priority - evicted first
    Low,
    /// Normal priority - default
    Normal, 
    /// High priority - kept longer
    High,
    /// Critical priority - never evicted automatically
    Critical,
}

/// Cached response with enhanced metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResponse {
    /// The AI response content
    pub content: String,
    /// Timestamp when cached
    pub cached_at: u64,
    /// Time-to-live in seconds
    pub ttl: u64,
    /// Number of times this response has been accessed
    pub access_count: u64,
    /// Last access timestamp
    pub last_accessed: u64,
    /// Hash of the original prompt for validation
    pub prompt_hash: String,
    /// AI provider that generated this response
    pub provider: String,
    /// Response metadata
    pub metadata: HashMap<String, String>,
    /// Cache priority level
    pub priority: CachePriority,
    /// Tags for content-based invalidation
    pub tags: HashSet<String>,
    /// Size estimate in bytes
    pub size_bytes: usize,
    /// Invalidation strategy for this entry
    pub invalidation_strategy: InvalidationStrategy,
}

impl CachedResponse {
    /// Check if this cached response is still valid
    pub fn is_valid(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        now < self.cached_at + self.ttl
    }

    /// Check if this response is expired
    pub fn is_expired(&self) -> bool {
        !self.is_valid()
    }

    /// Get age of this cached response in seconds
    pub fn age_seconds(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        now.saturating_sub(self.cached_at)
    }

    /// Increment access count and update last accessed time
    pub fn increment_access(&mut self) {
        self.access_count += 1;
        self.last_accessed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// Get time since last access in seconds
    pub fn seconds_since_last_access(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now.saturating_sub(self.last_accessed)
    }

    /// Check if entry should be evicted based on strategy and priority
    pub fn should_evict(&self, strategy: &InvalidationStrategy) -> bool {
        match self.priority {
            CachePriority::Critical => false, // Never evict critical entries
            _ => match strategy {
                InvalidationStrategy::TimeBasedTTL => self.is_expired(),
                InvalidationStrategy::LeastRecentlyUsed => self.seconds_since_last_access() > 3600, // 1 hour
                InvalidationStrategy::LeastFrequentlyUsed => self.access_count < 5,
                InvalidationStrategy::ContentBased => false, // Handled separately
                InvalidationStrategy::Manual => false, // Only manual invalidation
            }
        }
    }

    /// Add a tag for content-based invalidation
    pub fn add_tag(&mut self, tag: String) {
        self.tags.insert(tag);
    }

    /// Check if entry has a specific tag
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.contains(tag)
    }

    /// Create a new cached response with default values
    pub fn new(
        content: String,
        prompt_hash: String,
        provider: String,
        ttl_seconds: u64,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let size_bytes = content.len() + prompt_hash.len() + provider.len();

        Self {
            content,
            cached_at: now,
            ttl: ttl_seconds,
            access_count: 0,
            last_accessed: now,
            prompt_hash,
            provider,
            metadata: HashMap::new(),
            priority: CachePriority::Normal,
            tags: HashSet::new(),
            size_bytes,
            invalidation_strategy: InvalidationStrategy::TimeBasedTTL,
        }
    }
}

/// Cache configuration for advanced settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Maximum number of entries
    pub max_entries: usize,
    /// Default TTL for new entries
    pub default_ttl: Duration,
    /// Maximum memory usage in bytes (0 = unlimited)
    pub max_memory_bytes: usize,
    /// Cleanup interval
    pub cleanup_interval: Duration,
    /// Default invalidation strategy
    pub default_strategy: InvalidationStrategy,
    /// Enable cache warming
    pub enable_warming: bool,
    /// Warming batch size
    pub warming_batch_size: usize,
    /// Preload common prompts on startup
    pub preload_common_prompts: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            default_ttl: Duration::from_secs(3600), // 1 hour
            max_memory_bytes: 50 * 1024 * 1024, // 50MB
            cleanup_interval: Duration::from_secs(300), // 5 minutes
            default_strategy: InvalidationStrategy::TimeBasedTTL,
            enable_warming: true,
            warming_batch_size: 10,
            preload_common_prompts: true,
        }
    }
}

/// Enhanced statistics about cache usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStatistics {
    /// Total number of cache entries
    pub total_entries: usize,
    /// Number of valid (non-expired) entries
    pub valid_entries: usize,
    /// Number of expired entries
    pub expired_entries: usize,
    /// Cache hit rate (0.0 to 1.0)
    pub hit_rate: f64,
    /// Total cache hits
    pub total_hits: u64,
    /// Total cache misses
    pub total_misses: u64,
    /// Total memory usage estimate in bytes
    pub memory_usage_bytes: usize,
    /// Maximum memory limit in bytes
    pub max_memory_bytes: usize,
    /// Memory usage percentage (0.0 to 1.0)
    pub memory_usage_percent: f64,
    /// Average response size in bytes
    pub avg_response_size: usize,
    /// Most frequently accessed entries
    pub top_entries: Vec<(String, u64)>,
    /// Cache efficiency metrics
    pub efficiency_score: f64,
    /// Number of evictions performed
    pub total_evictions: u64,
    /// Number of cache cleanups performed
    pub total_cleanups: u64,
    /// Number of cache warms performed
    pub total_warms: u64,
    /// Entries by priority level
    pub entries_by_priority: HashMap<String, usize>,
    /// Entries by invalidation strategy
    pub entries_by_strategy: HashMap<String, usize>,
    /// Average age of cached entries in seconds
    pub avg_entry_age_seconds: f64,
    /// Cache warming status
    pub warming_active: bool,
}

/// AI response cache for performance optimization
pub struct AIResponseCache {
    /// Cache storage
    cache: Arc<RwLock<HashMap<String, CachedResponse>>>,
    /// Cache statistics
    stats: Arc<RwLock<CacheStats>>,
    /// Cache configuration
    config: Arc<RwLock<CacheConfig>>,
    /// Last cleanup timestamp
    last_cleanup: Arc<RwLock<Instant>>,
    /// Cache warming in progress
    warming_active: Arc<RwLock<bool>>,
    /// Common prompts for warming
    common_prompts: Arc<RwLock<Vec<String>>>,
}

/// Internal cache statistics
#[derive(Debug, Default)]
struct CacheStats {
    hits: u64,
    misses: u64,
    evictions: u64,
    cleanups: u64,
    warms: u64,
}

impl AIResponseCache {
    /// Create a new AI response cache with default configuration
    pub fn new(max_entries: usize, default_ttl: Duration) -> Self {
        let mut config = CacheConfig::default();
        config.max_entries = max_entries;
        config.default_ttl = default_ttl;
        
        Self::with_config(config)
    }

    /// Create a new AI response cache with custom configuration
    pub fn with_config(config: CacheConfig) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(CacheStats::default())),
            config: Arc::new(RwLock::new(config)),
            last_cleanup: Arc::new(RwLock::new(Instant::now())),
            warming_active: Arc::new(RwLock::new(false)),
            common_prompts: Arc::new(RwLock::new(Self::default_common_prompts())),
        }
    }

    /// Get default common prompts for warming
    fn default_common_prompts() -> Vec<String> {
        vec![
            "Summarize this email".to_string(),
            "Generate a professional reply".to_string(), 
            "What are the key points?".to_string(),
            "Schedule a meeting".to_string(),
            "Categorize this email".to_string(),
            "Draft a response".to_string(),
            "Extract action items".to_string(),
            "Translate to English".to_string(),
            "Make this more concise".to_string(),
            "Add meeting to calendar".to_string(),
        ]
    }

    /// Generate a hash for a prompt to use as cache key
    pub fn generate_prompt_hash(&self, prompt: &str, context: Option<&str>) -> String {
        let mut hasher = Sha256::new();
        hasher.update(prompt);
        if let Some(ctx) = context {
            hasher.update(ctx);
        }
        format!("{:x}", hasher.finalize())
    }

    /// Get a cached response by prompt hash
    pub async fn get_cached_response(&self, prompt_hash: &str) -> Option<CachedResponse> {
        // Periodic cleanup
        self.maybe_cleanup().await;

        let mut cache = self.cache.write().await;
        
        if let Some(mut response) = cache.get(prompt_hash).cloned() {
            if response.is_valid() {
                response.increment_access();
                cache.insert(prompt_hash.to_string(), response.clone());
                
                // Update hit statistics
                let mut stats = self.stats.write().await;
                stats.hits += 1;
                
                tracing::debug!("Cache hit for prompt hash: {}", prompt_hash);
                Some(response)
            } else {
                // Remove expired entry
                cache.remove(prompt_hash);
                tracing::debug!("Removed expired cache entry: {}", prompt_hash);
                
                // Update miss statistics
                let mut stats = self.stats.write().await;
                stats.misses += 1;
                
                None
            }
        } else {
            // Update miss statistics
            let mut stats = self.stats.write().await;
            stats.misses += 1;
            
            tracing::debug!("Cache miss for prompt hash: {}", prompt_hash);
            None
        }
    }

    /// Cache a response with specified TTL and optional configuration
    pub async fn cache_response(
        &self,
        prompt_hash: &str,
        response: &str,
        provider: &str,
        ttl: Option<Duration>,
    ) -> AIResult<()> {
        self.cache_response_with_priority(
            prompt_hash,
            response,
            provider,
            ttl,
            CachePriority::Normal,
            vec![],
        ).await
    }

    /// Cache a response with priority and tags
    pub async fn cache_response_with_priority(
        &self,
        prompt_hash: &str,
        response: &str,
        provider: &str,
        ttl: Option<Duration>,
        priority: CachePriority,
        tags: Vec<String>,
    ) -> AIResult<()> {
        let config = self.config.read().await;
        let ttl = ttl.unwrap_or(config.default_ttl);
        
        let mut cached_response = CachedResponse::new(
            response.to_string(),
            prompt_hash.to_string(),
            provider.to_string(),
            ttl.as_secs(),
        );
        
        cached_response.priority = priority.clone();
        cached_response.invalidation_strategy = config.default_strategy.clone();
        
        for tag in tags {
            cached_response.add_tag(tag);
        }

        let mut cache = self.cache.write().await;
        
        // Check memory limits and entry limits
        let current_memory = self.calculate_memory_usage(&cache).await;
        let entry_memory = cached_response.size_bytes;
        
        if current_memory + entry_memory > config.max_memory_bytes && config.max_memory_bytes > 0 {
            self.evict_by_memory(&mut cache, entry_memory).await;
        } else if cache.len() >= config.max_entries {
            self.evict_by_strategy(&mut cache, &config.default_strategy).await;
        }

        cache.insert(prompt_hash.to_string(), cached_response);
        
        debug!(
            "Cached response for prompt hash: {} (TTL: {:?}, Priority: {:?})",
            prompt_hash,
            ttl,
            priority
        );
        
        Ok(())
    }

    /// Invalidate cache entries matching a pattern
    pub async fn invalidate_cache(&self, pattern: &str) -> AIResult<usize> {
        let mut cache = self.cache.write().await;
        let initial_size = cache.len();
        
        if pattern == "*" {
            // Clear all cache
            cache.clear();
            tracing::info!("Cleared entire AI response cache");
        } else {
            // Remove entries matching pattern (simple substring match)
            cache.retain(|key, _| !key.contains(pattern));
            tracing::info!("Invalidated cache entries matching pattern: {}", pattern);
        }
        
        let removed_count = initial_size - cache.len();
        Ok(removed_count)
    }

    /// Get comprehensive cache statistics
    pub async fn get_cache_stats(&self) -> CacheStatistics {
        let cache = self.cache.read().await;
        let stats = self.stats.read().await;
        let config = self.config.read().await;
        let warming_active = *self.warming_active.read().await;
        
        let mut valid_entries = 0;
        let mut expired_entries = 0;
        let mut memory_usage = 0;
        let mut top_entries = Vec::new();
        let mut entries_by_priority = HashMap::new();
        let mut entries_by_strategy = HashMap::new();
        let mut total_age_seconds = 0u64;
        
        for (key, response) in cache.iter() {
            if response.is_expired() {
                expired_entries += 1;
            } else {
                valid_entries += 1;
            }
            
            // Accurate memory usage from cached response
            memory_usage += response.size_bytes;
            
            // Collect access counts for top entries
            top_entries.push((key.clone(), response.access_count));
            
            // Count by priority
            let priority_str = format!("{:?}", response.priority);
            *entries_by_priority.entry(priority_str).or_insert(0) += 1;
            
            // Count by strategy
            let strategy_str = format!("{:?}", response.invalidation_strategy);
            *entries_by_strategy.entry(strategy_str).or_insert(0) += 1;
            
            // Accumulate age for average calculation
            total_age_seconds += response.age_seconds();
        }
        
        // Sort by access count and take top 10
        top_entries.sort_by(|a, b| b.1.cmp(&a.1));
        top_entries.truncate(10);
        
        let total_requests = stats.hits + stats.misses;
        let hit_rate = if total_requests > 0 {
            stats.hits as f64 / total_requests as f64
        } else {
            0.0
        };
        
        let avg_response_size = if !cache.is_empty() {
            memory_usage / cache.len()
        } else {
            0
        };

        let memory_usage_percent = if config.max_memory_bytes > 0 {
            memory_usage as f64 / config.max_memory_bytes as f64
        } else {
            0.0
        };

        // Calculate efficiency score based on hit rate, memory usage, and entry utilization
        let entry_utilization = cache.len() as f64 / config.max_entries as f64;
        let efficiency_score = (hit_rate * 0.5) + 
                              ((1.0 - memory_usage_percent.min(1.0)) * 0.3) + 
                              (entry_utilization.min(1.0) * 0.2);

        let avg_entry_age_seconds = if !cache.is_empty() {
            total_age_seconds as f64 / cache.len() as f64
        } else {
            0.0
        };
        
        CacheStatistics {
            total_entries: cache.len(),
            valid_entries,
            expired_entries,
            hit_rate,
            total_hits: stats.hits,
            total_misses: stats.misses,
            memory_usage_bytes: memory_usage,
            max_memory_bytes: config.max_memory_bytes,
            memory_usage_percent,
            avg_response_size,
            top_entries,
            efficiency_score,
            total_evictions: stats.evictions,
            total_cleanups: stats.cleanups,
            total_warms: stats.warms,
            entries_by_priority,
            entries_by_strategy,
            avg_entry_age_seconds,
            warming_active,
        }
    }

    /// Cleanup expired entries
    pub async fn cleanup_expired(&self) -> usize {
        let mut cache = self.cache.write().await;
        let initial_size = cache.len();
        
        cache.retain(|_, response| response.is_valid());
        
        let removed_count = initial_size - cache.len();
        
        if removed_count > 0 {
            tracing::info!("Cleaned up {} expired cache entries", removed_count);
            
            let mut stats = self.stats.write().await;
            stats.cleanups += 1;
        }
        
        let mut last_cleanup = self.last_cleanup.write().await;
        *last_cleanup = Instant::now();
        
        removed_count
    }

    /// Maybe perform cleanup if enough time has passed
    async fn maybe_cleanup(&self) {
        let last_cleanup = *self.last_cleanup.read().await;
        let config = self.config.read().await;
        
        if last_cleanup.elapsed() >= config.cleanup_interval {
            drop(config); // Release config lock before cleanup
            self.cleanup_expired().await;
        }
    }

    /// Calculate total memory usage of cache
    async fn calculate_memory_usage(&self, cache: &HashMap<String, CachedResponse>) -> usize {
        cache.values().map(|entry| entry.size_bytes).sum()
    }

    /// Evict entries by memory usage
    async fn evict_by_memory(&self, cache: &mut HashMap<String, CachedResponse>, needed_bytes: usize) {
        let mut total_freed = 0;
        let target_freed = needed_bytes + (needed_bytes / 4); // Free 25% extra
        
        // Collect entries sorted by priority and age
        let mut entries: Vec<(String, CachePriority, u64, usize)> = cache
            .iter()
            .map(|(key, response)| {
                (key.clone(), response.priority.clone(), response.age_seconds(), response.size_bytes)
            })
            .collect();

        // Sort by priority (low first), then by age (oldest first), then by size (largest first)
        entries.sort_by(|a, b| {
            let priority_order = |p: &CachePriority| match p {
                CachePriority::Low => 0,
                CachePriority::Normal => 1,
                CachePriority::High => 2,
                CachePriority::Critical => 3,
            };
            
            priority_order(&a.1).cmp(&priority_order(&b.1))
                .then(b.2.cmp(&a.2)) // Age (oldest first)
                .then(b.3.cmp(&a.3)) // Size (largest first)
        });

        let mut evicted_count = 0;
        for (key, priority, _, size) in entries {
            if priority == CachePriority::Critical {
                continue; // Never evict critical entries
            }
            
            if total_freed >= target_freed {
                break;
            }
            
            cache.remove(&key);
            total_freed += size;
            evicted_count += 1;
        }

        if evicted_count > 0 {
            info!("Evicted {} entries to free {} bytes of memory", evicted_count, total_freed);
            
            if let Ok(mut stats) = self.stats.try_write() {
                stats.evictions += evicted_count;
            }
        }
    }

    /// Evict entries based on invalidation strategy
    async fn evict_by_strategy(&self, cache: &mut HashMap<String, CachedResponse>, strategy: &InvalidationStrategy) {
        let evict_count = cache.len() / 4; // Evict 25% of entries
        
        // Collect entries based on strategy
        let mut entries: Vec<(String, u64)> = match strategy {
            InvalidationStrategy::LeastRecentlyUsed => {
                cache.iter()
                    .filter(|(_, entry)| entry.priority != CachePriority::Critical)
                    .map(|(key, entry)| (key.clone(), entry.seconds_since_last_access()))
                    .collect()
            },
            InvalidationStrategy::LeastFrequentlyUsed => {
                cache.iter()
                    .filter(|(_, entry)| entry.priority != CachePriority::Critical)
                    .map(|(key, entry)| (key.clone(), u64::MAX - entry.access_count)) // Invert for sorting
                    .collect()
            },
            _ => {
                // Default to age-based eviction (oldest first)
                cache.iter()
                    .filter(|(_, entry)| entry.priority != CachePriority::Critical)
                    .map(|(key, entry)| (key.clone(), entry.age_seconds()))
                    .collect()
            }
        };

        // Sort by the strategy metric (highest values first for eviction)
        entries.sort_by(|a, b| b.1.cmp(&a.1));

        // Remove entries
        let mut evicted_count = 0;
        for (key, _) in entries.iter().take(evict_count) {
            cache.remove(key);
            evicted_count += 1;
        }

        if evicted_count > 0 {
            debug!("Evicted {} cache entries using {:?} strategy", evicted_count, strategy);
            
            if let Ok(mut stats) = self.stats.try_write() {
                stats.evictions += evicted_count as u64;
            }
        }
    }


    /// Get cache entry by exact key
    pub async fn get_entry(&self, key: &str) -> Option<CachedResponse> {
        let cache = self.cache.read().await;
        cache.get(key).cloned()
    }

    /// Remove specific cache entry
    pub async fn remove_entry(&self, key: &str) -> bool {
        let mut cache = self.cache.write().await;
        cache.remove(key).is_some()
    }

    /// Get all cache keys
    pub async fn get_all_keys(&self) -> Vec<String> {
        let cache = self.cache.read().await;
        cache.keys().cloned().collect()
    }

    /// Set cache metadata for an entry
    pub async fn set_entry_metadata(
        &self,
        key: &str,
        metadata: HashMap<String, String>,
    ) -> AIResult<()> {
        let mut cache = self.cache.write().await;
        
        if let Some(entry) = cache.get_mut(key) {
            entry.metadata = metadata;
            Ok(())
        } else {
            Err(AIError::cache_error(format!("Cache entry not found: {}", key)))
        }
    }

    /// Invalidate cache entries by tags
    pub async fn invalidate_by_tags(&self, tags: &[String]) -> AIResult<usize> {
        let mut cache = self.cache.write().await;
        let initial_size = cache.len();
        
        cache.retain(|_, entry| {
            !tags.iter().any(|tag| entry.has_tag(tag))
        });
        
        let removed_count = initial_size - cache.len();
        
        if removed_count > 0 {
            info!("Invalidated {} cache entries by tags: {:?}", removed_count, tags);
        }
        
        Ok(removed_count)
    }

    /// Invalidate cache entries by provider
    pub async fn invalidate_by_provider(&self, provider: &str) -> AIResult<usize> {
        let mut cache = self.cache.write().await;
        let initial_size = cache.len();
        
        cache.retain(|_, entry| entry.provider != provider);
        
        let removed_count = initial_size - cache.len();
        
        if removed_count > 0 {
            info!("Invalidated {} cache entries for provider: {}", removed_count, provider);
        }
        
        Ok(removed_count)
    }

    /// Warm cache with common prompts
    pub async fn warm_cache<F, Fut>(&self, ai_provider: F) -> AIResult<usize>
    where
        F: Fn(String) -> Fut + Send + Sync + Clone,
        Fut: std::future::Future<Output = AIResult<String>> + Send,
    {
        let config = self.config.read().await;
        if !config.enable_warming {
            return Ok(0);
        }

        let mut warming_active = self.warming_active.write().await;
        if *warming_active {
            return Err(AIError::cache_error("Cache warming already in progress".to_string()));
        }
        *warming_active = true;
        drop(warming_active);

        let common_prompts = self.common_prompts.read().await.clone();
        let batch_size = config.warming_batch_size;
        drop(config);

        let mut warmed_count = 0;
        
        for chunk in common_prompts.chunks(batch_size) {
            for prompt in chunk {
                let prompt_hash = self.generate_prompt_hash(prompt, None);
                
                // Skip if already cached
                if self.get_cached_response(&prompt_hash).await.is_some() {
                    continue;
                }
                
                // Execute warming request
                let ai_provider_clone = ai_provider.clone();
                let prompt_clone = prompt.clone();
                
                match ai_provider_clone(prompt_clone).await {
                    Ok(response) => {
                        if let Err(e) = self.cache_response_with_priority(
                            &prompt_hash,
                            &response,
                            "warming",
                            None,
                            CachePriority::Low,
                            vec!["warming".to_string()],
                        ).await {
                            warn!("Failed to cache warming response: {}", e);
                        } else {
                            warmed_count += 1;
                        }
                    }
                    Err(e) => {
                        debug!("Failed to warm cache for prompt: {}", e);
                    }
                }
            }
        }

        // Update statistics
        if let Ok(mut stats) = self.stats.try_write() {
            stats.warms += 1;
        }

        let mut warming_active = self.warming_active.write().await;
        *warming_active = false;

        info!("Cache warming completed: {} entries warmed", warmed_count);
        Ok(warmed_count)
    }

    /// Add custom prompts to warming list
    pub async fn add_warming_prompts(&self, prompts: Vec<String>) {
        let mut common_prompts = self.common_prompts.write().await;
        common_prompts.extend(prompts);
    }

    /// Get cache configuration
    pub async fn get_config(&self) -> CacheConfig {
        self.config.read().await.clone()
    }

    /// Update cache configuration
    pub async fn update_config(&self, config: CacheConfig) {
        let mut current_config = self.config.write().await;
        *current_config = config;
    }

    /// Check if cache warming is active
    pub async fn is_warming_active(&self) -> bool {
        *self.warming_active.read().await
    }
}

impl Default for AIResponseCache {
    fn default() -> Self {
        Self::new(1000, Duration::from_secs(3600)) // 1000 entries, 1 hour TTL
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_cache_basic_operations() {
        let cache = AIResponseCache::new(100, Duration::from_secs(60));
        let prompt_hash = cache.generate_prompt_hash("test prompt", None);
        
        // Cache miss initially
        assert!(cache.get_cached_response(&prompt_hash).await.is_none());
        
        // Cache response
        cache
            .cache_response(&prompt_hash, "test response", "test_provider", None)
            .await
            .unwrap();
        
        // Cache hit
        let cached = cache.get_cached_response(&prompt_hash).await.unwrap();
        assert_eq!(cached.content, "test response");
        assert_eq!(cached.provider, "test_provider");
        assert_eq!(cached.priority, CachePriority::Normal);
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let cache = AIResponseCache::new(100, Duration::from_secs(60));
        let prompt_hash = cache.generate_prompt_hash("test prompt", None);
        
        // Cache response with very short TTL (1 second)
        cache
            .cache_response(
                &prompt_hash,
                "test response",
                "test_provider",
                Some(Duration::from_secs(1)),
            )
            .await
            .unwrap();
        
        // Should be available immediately
        assert!(cache.get_cached_response(&prompt_hash).await.is_some());
        
        // Wait for expiration
        sleep(Duration::from_millis(1100)).await;
        
        // Manually trigger cleanup to verify expiration logic
        let removed_count = cache.cleanup_expired().await;
        assert!(removed_count > 0);
        
        // Should be expired and removed now
        assert!(cache.get_cached_response(&prompt_hash).await.is_none());
    }

    #[tokio::test]
    async fn test_cache_statistics() {
        let cache = AIResponseCache::new(100, Duration::from_secs(60));
        let prompt_hash = cache.generate_prompt_hash("test prompt", None);
        
        // Generate some cache activity
        cache.get_cached_response(&prompt_hash).await; // Miss
        cache
            .cache_response(&prompt_hash, "test response", "test_provider", None)
            .await
            .unwrap();
        cache.get_cached_response(&prompt_hash).await; // Hit
        cache.get_cached_response(&prompt_hash).await; // Hit
        
        let stats = cache.get_cache_stats().await;
        assert_eq!(stats.total_hits, 2);
        assert_eq!(stats.total_misses, 1);
        assert_eq!(stats.total_entries, 1);
        assert_eq!(stats.valid_entries, 1);
        assert_eq!(stats.expired_entries, 0);
    }

    #[tokio::test]
    async fn test_cache_invalidation() {
        let cache = AIResponseCache::new(100, Duration::from_secs(60));
        
        // Cache multiple responses
        for i in 0..5 {
            let prompt_hash = cache.generate_prompt_hash(&format!("test prompt {}", i), None);
            cache
                .cache_response(&prompt_hash, &format!("response {}", i), "test_provider", None)
                .await
                .unwrap();
        }
        
        assert_eq!(cache.get_cache_stats().await.total_entries, 5);
        
        // Invalidate all
        let removed = cache.invalidate_cache("*").await.unwrap();
        assert_eq!(removed, 5);
        assert_eq!(cache.get_cache_stats().await.total_entries, 0);
    }

    #[test]
    fn test_prompt_hash_generation() {
        let cache = AIResponseCache::new(100, Duration::from_secs(60));
        
        let hash1 = cache.generate_prompt_hash("test prompt", None);
        let hash2 = cache.generate_prompt_hash("test prompt", None);
        let hash3 = cache.generate_prompt_hash("different prompt", None);
        let hash4 = cache.generate_prompt_hash("test prompt", Some("context"));
        
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_ne!(hash1, hash4);
    }
}