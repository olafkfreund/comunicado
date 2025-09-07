//! Performance-optimized caching system

use crate::performance::PerformanceResult; // PerformanceError
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
// use uuid::Uuid;

/// Cache eviction strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheEvictionStrategy {
    /// Least Recently Used - evict oldest accessed items
    LRU,
    /// Least Frequently Used - evict least accessed items
    LFU,
    /// Time-based expiration
    TTL,
    /// First In First Out - evict oldest items
    FIFO,
    /// Random eviction
    Random,
    /// Adaptive Replacement Cache - combines LRU and LFU
    ARC,
}

/// Cache priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CachePriority {
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

/// Cache level hierarchy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheLevel {
    /// In-memory L1 cache (fastest access)
    L1Memory,
    /// SSD-based L2 cache (fast access)
    L2Disk,
    /// Network-based L3 cache (slower access)
    L3Network,
}

/// Cache policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachePolicy {
    /// Maximum number of items in cache
    pub max_items: usize,
    /// Maximum memory usage in bytes
    pub max_memory_bytes: usize,
    /// Default TTL for cached items
    pub default_ttl: Duration,
    /// Eviction strategy to use
    pub eviction_strategy: CacheEvictionStrategy,
    /// Enable cache compression
    pub enable_compression: bool,
    /// Cache hit ratio threshold for optimization
    pub hit_ratio_threshold: f64,
    /// Background cleanup interval
    pub cleanup_interval: Duration,
}

/// Cache entry metadata
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct CacheEntry<T> {
    /// The cached value
    value: T,
    /// When the entry was created
    created_at: Instant,
    /// When the entry was last accessed
    last_accessed: Instant,
    /// Number of times accessed
    access_count: u64,
    /// Time-to-live for this entry
    ttl: Duration,
    /// Priority level
    priority: CachePriority,
    /// Size in bytes (approximate)
    size_bytes: usize,
}

/// Cache statistics and metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStatistics {
    /// Total cache hits
    pub total_hits: u64,
    /// Total cache misses
    pub total_misses: u64,
    /// Current hit ratio
    pub hit_ratio: f64,
    /// Total cache entries
    pub total_entries: usize,
    /// Current memory usage in bytes
    pub memory_usage_bytes: usize,
    /// Average access time in microseconds
    pub avg_access_time_us: f64,
    /// Cache efficiency score (0.0-1.0)
    pub efficiency_score: f64,
    /// Eviction count
    pub eviction_count: u64,
    /// Last cleanup time (not serialized)
    #[serde(skip)]
    pub last_cleanup: Option<Instant>,
}

/// High-performance cache implementation
pub struct PerformanceCache<K, V> {
    /// Internal cache storage
    cache: Arc<RwLock<HashMap<K, CacheEntry<V>>>>,
    /// Cache configuration policy
    policy: CachePolicy,
    /// Cache statistics
    stats: Arc<RwLock<CacheStatistics>>,
    /// LRU ordering for eviction
    lru_order: Arc<RwLock<Vec<K>>>,
    /// Cache level
    level: CacheLevel,
}

impl<K, V> PerformanceCache<K, V>
where
    K: Clone + Eq + std::hash::Hash + Send + Sync,
    V: Clone + Send + Sync,
{
    /// Create a new performance cache with policy
    pub fn new(policy: CachePolicy, level: CacheLevel) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            policy,
            stats: Arc::new(RwLock::new(CacheStatistics {
                total_hits: 0,
                total_misses: 0,
                hit_ratio: 0.0,
                total_entries: 0,
                memory_usage_bytes: 0,
                avg_access_time_us: 0.0,
                efficiency_score: 0.0,
                eviction_count: 0,
                last_cleanup: None,
            })),
            lru_order: Arc::new(RwLock::new(Vec::new())),
            level,
        }
    }

    /// Get cache level
    pub fn level(&self) -> CacheLevel {
        self.level.clone()
    }

    /// Get value from cache
    pub async fn get(&self, key: &K) -> Option<V> {
        let start_time = Instant::now();

        let mut cache = self.cache.write().await;
        let mut stats = self.stats.write().await;

        if let Some(entry) = cache.get_mut(key) {
            // Check if entry is expired
            if entry.created_at.elapsed() > entry.ttl {
                cache.remove(key);
                stats.total_misses += 1;
                None
            } else {
                // Update access metadata
                entry.last_accessed = Instant::now();
                entry.access_count += 1;

                // Update LRU order
                self.update_lru_order(key).await;

                stats.total_hits += 1;
                let value = entry.value.clone();

                // Update statistics
                let access_time = start_time.elapsed().as_micros() as f64;
                stats.avg_access_time_us = (stats.avg_access_time_us + access_time) / 2.0;

                Some(value)
            }
        } else {
            stats.total_misses += 1;
            None
        }
    }

    /// Put value in cache
    pub async fn put(&self, key: K, value: V, priority: CachePriority) -> PerformanceResult<()> {
        self.put_with_ttl(key, value, priority, self.policy.default_ttl)
            .await
    }

    /// Put value in cache with custom TTL
    pub async fn put_with_ttl(
        &self,
        key: K,
        value: V,
        priority: CachePriority,
        ttl: Duration,
    ) -> PerformanceResult<()> {
        let mut cache = self.cache.write().await;
        let mut stats = self.stats.write().await;

        // Estimate size (rough approximation)
        let size_bytes = std::mem::size_of::<V>() + std::mem::size_of::<K>();

        // Check if we need to evict entries
        if cache.len() >= self.policy.max_items
            || stats.memory_usage_bytes + size_bytes > self.policy.max_memory_bytes
        {
            self.evict_entries(&mut cache, &mut stats).await?;
        }

        let entry = CacheEntry {
            value,
            created_at: Instant::now(),
            last_accessed: Instant::now(),
            access_count: 1,
            ttl,
            priority,
            size_bytes,
        };

        cache.insert(key.clone(), entry);
        stats.total_entries = cache.len();
        stats.memory_usage_bytes += size_bytes;

        // Update LRU order
        self.update_lru_order(&key).await;

        Ok(())
    }

    /// Remove entry from cache
    pub async fn remove(&self, key: &K) -> Option<V> {
        let mut cache = self.cache.write().await;
        let mut stats = self.stats.write().await;

        if let Some(entry) = cache.remove(key) {
            stats.total_entries = cache.len();
            stats.memory_usage_bytes = stats.memory_usage_bytes.saturating_sub(entry.size_bytes);

            // Remove from LRU order
            let mut lru_order = self.lru_order.write().await;
            lru_order.retain(|k| k != key);

            Some(entry.value)
        } else {
            None
        }
    }

    /// Clear all cache entries
    pub async fn clear(&self) -> PerformanceResult<()> {
        let mut cache = self.cache.write().await;
        let mut stats = self.stats.write().await;
        let mut lru_order = self.lru_order.write().await;

        cache.clear();
        lru_order.clear();

        stats.total_entries = 0;
        stats.memory_usage_bytes = 0;

        Ok(())
    }

    /// Get cache statistics
    pub async fn statistics(&self) -> CacheStatistics {
        let mut stats = self.stats.write().await;

        // Update hit ratio
        let total_requests = stats.total_hits + stats.total_misses;
        if total_requests > 0 {
            stats.hit_ratio = stats.total_hits as f64 / total_requests as f64;
        }

        // Calculate efficiency score based on hit ratio and access time
        stats.efficiency_score =
            stats.hit_ratio * (1.0 / (1.0 + stats.avg_access_time_us / 1000.0));

        stats.clone()
    }

    /// Check if key exists in cache (without updating access metadata)
    pub async fn contains_key(&self, key: &K) -> bool {
        let cache = self.cache.read().await;
        cache.contains_key(key)
    }

    /// Get cache size (number of entries)
    pub async fn len(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }

    /// Check if cache is empty
    pub async fn is_empty(&self) -> bool {
        let cache = self.cache.read().await;
        cache.is_empty()
    }

    /// Run background cleanup to remove expired entries
    pub async fn cleanup_expired(&self) -> PerformanceResult<usize> {
        let mut cache = self.cache.write().await;
        let mut stats = self.stats.write().await;
        let mut lru_order = self.lru_order.write().await;

        let now = Instant::now();
        let mut removed_count = 0;
        let mut removed_size = 0;

        // Collect expired keys
        let expired_keys: Vec<K> = cache
            .iter()
            .filter(|(_, entry)| now.duration_since(entry.created_at) > entry.ttl)
            .map(|(key, _)| key.clone())
            .collect();

        // Remove expired entries
        for key in expired_keys {
            if let Some(entry) = cache.remove(&key) {
                removed_count += 1;
                removed_size += entry.size_bytes;

                // Remove from LRU order
                lru_order.retain(|k| k != &key);
            }
        }

        // Update statistics
        stats.total_entries = cache.len();
        stats.memory_usage_bytes = stats.memory_usage_bytes.saturating_sub(removed_size);
        stats.last_cleanup = Some(now);

        Ok(removed_count)
    }

    /// Update LRU ordering for a key
    async fn update_lru_order(&self, key: &K) {
        let mut lru_order = self.lru_order.write().await;

        // Remove key if it exists
        lru_order.retain(|k| k != key);

        // Add to front (most recently used)
        lru_order.insert(0, key.clone());
    }

    /// Evict entries based on policy
    async fn evict_entries(
        &self,
        cache: &mut HashMap<K, CacheEntry<V>>,
        stats: &mut CacheStatistics,
    ) -> PerformanceResult<()> {
        match self.policy.eviction_strategy {
            CacheEvictionStrategy::LRU => {
                self.evict_lru(cache, stats).await?;
            }
            CacheEvictionStrategy::LFU => {
                self.evict_lfu(cache, stats).await?;
            }
            CacheEvictionStrategy::TTL => {
                self.evict_expired(cache, stats).await?;
            }
            CacheEvictionStrategy::FIFO => {
                self.evict_fifo(cache, stats).await?;
            }
            CacheEvictionStrategy::Random => {
                self.evict_random(cache, stats).await?;
            }
            CacheEvictionStrategy::ARC => {
                self.evict_arc(cache, stats).await?;
            }
        }

        Ok(())
    }

    /// Evict least recently used entries
    async fn evict_lru(
        &self,
        cache: &mut HashMap<K, CacheEntry<V>>,
        stats: &mut CacheStatistics,
    ) -> PerformanceResult<()> {
        let mut lru_order = self.lru_order.write().await;

        // Evict from the end (least recently used)
        while cache.len() > self.policy.max_items / 2 && !lru_order.is_empty() {
            if let Some(key) = lru_order.pop() {
                if let Some(entry) = cache.remove(&key) {
                    stats.memory_usage_bytes =
                        stats.memory_usage_bytes.saturating_sub(entry.size_bytes);
                    stats.eviction_count += 1;
                }
            }
        }

        Ok(())
    }

    /// Evict least frequently used entries
    async fn evict_lfu(
        &self,
        cache: &mut HashMap<K, CacheEntry<V>>,
        stats: &mut CacheStatistics,
    ) -> PerformanceResult<()> {
        // Sort by access count and remove least frequently used
        let mut entries: Vec<(K, u64)> = cache
            .iter()
            .map(|(k, v)| (k.clone(), v.access_count))
            .collect();

        entries.sort_by_key(|(_, count)| *count);

        let evict_count = cache.len().saturating_sub(self.policy.max_items / 2);
        for (key, _) in entries.into_iter().take(evict_count) {
            if let Some(entry) = cache.remove(&key) {
                stats.memory_usage_bytes =
                    stats.memory_usage_bytes.saturating_sub(entry.size_bytes);
                stats.eviction_count += 1;
            }
        }

        Ok(())
    }

    /// Evict expired entries
    async fn evict_expired(
        &self,
        cache: &mut HashMap<K, CacheEntry<V>>,
        stats: &mut CacheStatistics,
    ) -> PerformanceResult<()> {
        let now = Instant::now();
        let expired_keys: Vec<K> = cache
            .iter()
            .filter(|(_, entry)| now.duration_since(entry.created_at) > entry.ttl)
            .map(|(key, _)| key.clone())
            .collect();

        for key in expired_keys {
            if let Some(entry) = cache.remove(&key) {
                stats.memory_usage_bytes =
                    stats.memory_usage_bytes.saturating_sub(entry.size_bytes);
                stats.eviction_count += 1;
            }
        }

        Ok(())
    }

    /// Evict entries in FIFO order
    async fn evict_fifo(
        &self,
        cache: &mut HashMap<K, CacheEntry<V>>,
        stats: &mut CacheStatistics,
    ) -> PerformanceResult<()> {
        // Sort by creation time and remove oldest
        let mut entries: Vec<(K, Instant)> = cache
            .iter()
            .map(|(k, v)| (k.clone(), v.created_at))
            .collect();

        entries.sort_by_key(|(_, created)| *created);

        let evict_count = cache.len().saturating_sub(self.policy.max_items / 2);
        for (key, _) in entries.into_iter().take(evict_count) {
            if let Some(entry) = cache.remove(&key) {
                stats.memory_usage_bytes =
                    stats.memory_usage_bytes.saturating_sub(entry.size_bytes);
                stats.eviction_count += 1;
            }
        }

        Ok(())
    }

    /// Evict random entries
    async fn evict_random(
        &self,
        cache: &mut HashMap<K, CacheEntry<V>>,
        stats: &mut CacheStatistics,
    ) -> PerformanceResult<()> {
        use rand::seq::SliceRandom;
        use rand::thread_rng;

        let keys: Vec<K> = cache.keys().cloned().collect();
        let mut rng = thread_rng();
        let mut shuffled_keys = keys;
        shuffled_keys.shuffle(&mut rng);

        let evict_count = cache.len().saturating_sub(self.policy.max_items / 2);
        for key in shuffled_keys.into_iter().take(evict_count) {
            if let Some(entry) = cache.remove(&key) {
                stats.memory_usage_bytes =
                    stats.memory_usage_bytes.saturating_sub(entry.size_bytes);
                stats.eviction_count += 1;
            }
        }

        Ok(())
    }

    /// Evict using Adaptive Replacement Cache algorithm
    async fn evict_arc(
        &self,
        cache: &mut HashMap<K, CacheEntry<V>>,
        stats: &mut CacheStatistics,
    ) -> PerformanceResult<()> {
        // Simplified ARC implementation - combines LRU and LFU
        // In practice, this would maintain separate T1, T2, B1, B2 lists

        // Calculate score based on both recency and frequency
        let mut entries: Vec<(K, f64)> = cache
            .iter()
            .map(|(k, v)| {
                let recency_score = 1.0 / (1.0 + v.last_accessed.elapsed().as_secs_f64());
                let frequency_score = v.access_count as f64;
                let combined_score = recency_score * frequency_score;
                (k.clone(), combined_score)
            })
            .collect();

        entries.sort_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let evict_count = cache.len().saturating_sub(self.policy.max_items / 2);
        for (key, _) in entries.into_iter().take(evict_count) {
            if let Some(entry) = cache.remove(&key) {
                stats.memory_usage_bytes =
                    stats.memory_usage_bytes.saturating_sub(entry.size_bytes);
                stats.eviction_count += 1;
            }
        }

        Ok(())
    }
}

/// Multi-level cache manager
pub struct CacheManager {
    /// L1 in-memory cache (fastest)
    l1_cache: HashMap<String, Box<dyn std::any::Any + Send + Sync>>,
    /// L2 disk cache (fast)
    l2_cache: HashMap<String, Box<dyn std::any::Any + Send + Sync>>,
    /// L3 network cache (slower)
    l3_cache: HashMap<String, Box<dyn std::any::Any + Send + Sync>>,
    /// Cache statistics
    stats: Arc<RwLock<CacheStatistics>>,
}

impl CacheManager {
    /// Create a new cache manager
    pub fn new() -> Self {
        Self {
            l1_cache: HashMap::new(),
            l2_cache: HashMap::new(),
            l3_cache: HashMap::new(),
            stats: Arc::new(RwLock::new(CacheStatistics {
                total_hits: 0,
                total_misses: 0,
                hit_ratio: 0.0,
                total_entries: 0,
                memory_usage_bytes: 0,
                avg_access_time_us: 0.0,
                efficiency_score: 0.0,
                eviction_count: 0,
                last_cleanup: None,
            })),
        }
    }

    /// Get comprehensive cache statistics
    pub async fn get_statistics(&self) -> CacheStatistics {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Optimize cache configuration based on usage patterns
    pub async fn optimize_configuration(&self) -> PerformanceResult<CachePolicy> {
        let stats = self.get_statistics().await;

        let optimized_policy = CachePolicy {
            max_items: if stats.hit_ratio > 0.8 {
                // High hit ratio - can increase capacity
                (stats.total_entries * 2).max(1000)
            } else {
                // Low hit ratio - might need better eviction
                stats.total_entries.max(500)
            },
            max_memory_bytes: stats.memory_usage_bytes * 2,
            default_ttl: if stats.avg_access_time_us < 100.0 {
                Duration::from_secs(300) // 5 minutes for fast access
            } else {
                Duration::from_secs(60) // 1 minute for slow access
            },
            eviction_strategy: if stats.hit_ratio > 0.7 {
                CacheEvictionStrategy::LRU
            } else {
                CacheEvictionStrategy::ARC
            },
            enable_compression: stats.memory_usage_bytes > 1024 * 1024 * 100, // 100MB
            hit_ratio_threshold: 0.7,
            cleanup_interval: Duration::from_secs(300),
        };

        Ok(optimized_policy)
    }

    /// Run maintenance tasks on all cache levels
    pub async fn run_maintenance(&self) -> PerformanceResult<()> {
        tracing::info!("Running cache maintenance tasks");

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.last_cleanup = Some(Instant::now());
        stats.total_entries = self.l1_cache.len() + self.l2_cache.len() + self.l3_cache.len();

        Ok(())
    }
}

impl Default for CachePolicy {
    fn default() -> Self {
        Self {
            max_items: 10000,
            max_memory_bytes: 256 * 1024 * 1024,    // 256MB
            default_ttl: Duration::from_secs(3600), // 1 hour
            eviction_strategy: CacheEvictionStrategy::LRU,
            enable_compression: false,
            hit_ratio_threshold: 0.7,
            cleanup_interval: Duration::from_secs(300), // 5 minutes
        }
    }
}

impl Default for CacheStatistics {
    fn default() -> Self {
        Self {
            total_hits: 0,
            total_misses: 0,
            hit_ratio: 0.0,
            total_entries: 0,
            memory_usage_bytes: 0,
            avg_access_time_us: 0.0,
            efficiency_score: 0.0,
            eviction_count: 0,
            last_cleanup: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_cache_basic_operations() {
        let policy = CachePolicy::default();
        let cache = PerformanceCache::new(policy, CacheLevel::L1Memory);

        // Test put and get
        cache
            .put(
                "key1".to_string(),
                "value1".to_string(),
                CachePriority::Normal,
            )
            .await
            .unwrap();
        let result = cache.get(&"key1".to_string()).await;
        assert_eq!(result, Some("value1".to_string()));

        // Test miss
        let result = cache.get(&"nonexistent".to_string()).await;
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let policy = CachePolicy::default();
        let cache = PerformanceCache::new(policy, CacheLevel::L1Memory);

        // Put with short TTL
        cache
            .put_with_ttl(
                "key1".to_string(),
                "value1".to_string(),
                CachePriority::Normal,
                Duration::from_millis(100),
            )
            .await
            .unwrap();

        // Should be available immediately
        let result = cache.get(&"key1".to_string()).await;
        assert_eq!(result, Some("value1".to_string()));

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Should be expired
        let result = cache.get(&"key1".to_string()).await;
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_cache_statistics() {
        let policy = CachePolicy::default();
        let cache = PerformanceCache::new(policy, CacheLevel::L1Memory);

        // Generate some cache activity
        cache
            .put(
                "key1".to_string(),
                "value1".to_string(),
                CachePriority::Normal,
            )
            .await
            .unwrap();
        cache.get(&"key1".to_string()).await; // Hit
        cache.get(&"nonexistent".to_string()).await; // Miss

        let stats = cache.statistics().await;
        assert_eq!(stats.total_hits, 1);
        assert_eq!(stats.total_misses, 1);
        assert_eq!(stats.hit_ratio, 0.5);
    }
}
