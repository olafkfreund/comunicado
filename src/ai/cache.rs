//! AI response caching system for performance optimization

use crate::ai::error::{AIError, AIResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use sha2::{Digest, Sha256};

/// Cached response with metadata
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
    /// Hash of the original prompt for validation
    pub prompt_hash: String,
    /// AI provider that generated this response
    pub provider: String,
    /// Response metadata
    pub metadata: HashMap<String, String>,
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

    /// Increment access count
    pub fn increment_access(&mut self) {
        self.access_count += 1;
    }
}

/// Statistics about cache usage
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
    /// Average response size in bytes
    pub avg_response_size: usize,
    /// Most frequently accessed entries
    pub top_entries: Vec<(String, u64)>,
}

/// AI response cache for performance optimization
pub struct AIResponseCache {
    /// Cache storage
    cache: Arc<RwLock<HashMap<String, CachedResponse>>>,
    /// Cache statistics
    stats: Arc<RwLock<CacheStats>>,
    /// Maximum number of entries to store
    max_entries: usize,
    /// Default TTL for cached responses
    default_ttl: Duration,
    /// Cleanup interval
    cleanup_interval: Duration,
    /// Last cleanup timestamp
    last_cleanup: Arc<RwLock<Instant>>,
}

/// Internal cache statistics
#[derive(Debug, Default)]
struct CacheStats {
    hits: u64,
    misses: u64,
    evictions: u64,
    cleanups: u64,
}

impl AIResponseCache {
    /// Create a new AI response cache
    pub fn new(max_entries: usize, default_ttl: Duration) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(CacheStats::default())),
            max_entries,
            default_ttl,
            cleanup_interval: Duration::from_secs(300), // 5 minutes
            last_cleanup: Arc::new(RwLock::new(Instant::now())),
        }
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

    /// Cache a response with specified TTL
    pub async fn cache_response(
        &self,
        prompt_hash: &str,
        response: &str,
        provider: &str,
        ttl: Option<Duration>,
    ) -> AIResult<()> {
        let ttl = ttl.unwrap_or(self.default_ttl);
        let cached_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| AIError::cache_error(format!("System time error: {}", e)))?
            .as_secs();

        let cached_response = CachedResponse {
            content: response.to_string(),
            cached_at,
            ttl: ttl.as_secs(),
            access_count: 0,
            prompt_hash: prompt_hash.to_string(),
            provider: provider.to_string(),
            metadata: HashMap::new(),
        };

        let mut cache = self.cache.write().await;
        
        // Check if we need to evict entries
        if cache.len() >= self.max_entries {
            self.evict_oldest_entries(&mut cache).await;
        }

        cache.insert(prompt_hash.to_string(), cached_response);
        
        tracing::debug!(
            "Cached response for prompt hash: {} (TTL: {:?})",
            prompt_hash,
            ttl
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
        
        let mut valid_entries = 0;
        let mut expired_entries = 0;
        let mut memory_usage = 0;
        let mut top_entries = Vec::new();
        
        for (key, response) in cache.iter() {
            if response.is_expired() {
                expired_entries += 1;
            } else {
                valid_entries += 1;
            }
            
            // Estimate memory usage
            memory_usage += key.len() + response.content.len() + response.provider.len();
            
            // Collect access counts for top entries
            top_entries.push((key.clone(), response.access_count));
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
        
        CacheStatistics {
            total_entries: cache.len(),
            valid_entries,
            expired_entries,
            hit_rate,
            total_hits: stats.hits,
            total_misses: stats.misses,
            memory_usage_bytes: memory_usage,
            avg_response_size,
            top_entries,
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
        
        if last_cleanup.elapsed() >= self.cleanup_interval {
            self.cleanup_expired().await;
        }
    }

    /// Evict oldest entries to make room for new ones
    async fn evict_oldest_entries(&self, cache: &mut HashMap<String, CachedResponse>) {
        let evict_count = cache.len() / 4; // Evict 25% of entries
        
        // Collect entries with their ages and access counts
        let mut entries: Vec<(String, u64, u64)> = cache
            .iter()
            .map(|(key, response)| {
                (key.clone(), response.age_seconds(), response.access_count)
            })
            .collect();
        
        // Sort by age (oldest first) and access count (least accessed first)
        entries.sort_by(|a, b| {
            b.1.cmp(&a.1) // Age (oldest first)
                .then(a.2.cmp(&b.2)) // Then by access count (least first)
        });
        
        // Remove oldest entries
        for (key, _, _) in entries.iter().take(evict_count) {
            cache.remove(key);
        }
        
        tracing::debug!("Evicted {} cache entries to make room", evict_count);
        
        // Update eviction statistics
        if let Ok(mut stats) = self.stats.try_write() {
            stats.evictions += evict_count as u64;
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
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let cache = AIResponseCache::new(100, Duration::from_millis(100));
        let prompt_hash = cache.generate_prompt_hash("test prompt", None);
        
        // Cache response with short TTL
        cache
            .cache_response(
                &prompt_hash,
                "test response",
                "test_provider",
                Some(Duration::from_millis(50)),
            )
            .await
            .unwrap();
        
        // Should be available immediately
        assert!(cache.get_cached_response(&prompt_hash).await.is_some());
        
        // Wait for expiration
        sleep(Duration::from_millis(100)).await;
        
        // Should be expired and removed
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