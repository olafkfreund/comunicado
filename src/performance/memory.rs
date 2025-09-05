//! Memory management and optimization system

use crate::performance::{PerformanceResult, PerformanceError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

/// Memory management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryManagerConfig {
    /// Enable automatic garbage collection optimization
    pub auto_gc_optimization: bool,
    /// Memory pressure threshold (0.0-1.0)
    pub pressure_threshold: f64,
    /// Enable memory leak detection
    pub leak_detection_enabled: bool,
    /// Allocation tracking threshold
    pub large_allocation_threshold: usize,
    /// Enable memory profiling
    pub profiling_enabled: bool,
    /// Memory monitoring interval
    pub monitoring_interval: Duration,
    /// Enable heap analysis
    pub heap_analysis_enabled: bool,
    /// Memory warning threshold (bytes)
    pub warning_threshold_bytes: usize,
    /// Critical memory threshold (bytes)
    pub critical_threshold_bytes: usize,
}

/// Memory allocation tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationInfo {
    /// Allocation ID
    pub id: u64,
    /// Size in bytes
    pub size: usize,
    /// Allocation timestamp
    pub allocated_at: Instant,
    /// Allocation location (file:line)
    pub location: String,
    /// Allocation type/category
    pub category: String,
    /// Whether allocation is still active
    pub is_active: bool,
    /// Deallocation timestamp
    pub deallocated_at: Option<Instant>,
}

/// Memory statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    /// Total allocated bytes
    pub total_allocated: usize,
    /// Total deallocated bytes
    pub total_deallocated: usize,
    /// Currently allocated bytes
    pub current_allocated: usize,
    /// Peak memory usage
    pub peak_memory: usize,
    /// Number of allocations
    pub allocation_count: u64,
    /// Number of deallocations
    pub deallocation_count: u64,
    /// Average allocation size
    pub avg_allocation_size: usize,
    /// Memory fragmentation ratio
    pub fragmentation_ratio: f64,
    /// Garbage collection count
    pub gc_count: u64,
    /// Total GC time
    pub total_gc_time: Duration,
    /// Last GC timestamp
    pub last_gc: Option<Instant>,
}

/// Heap analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeapAnalysis {
    /// Total heap size
    pub total_heap_size: usize,
    /// Used heap size
    pub used_heap_size: usize,
    /// Free heap size
    pub free_heap_size: usize,
    /// Heap utilization ratio
    pub utilization_ratio: f64,
    /// Number of memory blocks
    pub block_count: usize,
    /// Average block size
    pub avg_block_size: usize,
    /// Largest free block
    pub largest_free_block: usize,
    /// Fragmentation analysis
    pub fragmentation_info: FragmentationInfo,
}

/// Memory fragmentation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FragmentationInfo {
    /// External fragmentation ratio
    pub external_fragmentation: f64,
    /// Internal fragmentation ratio
    pub internal_fragmentation: f64,
    /// Number of free fragments
    pub free_fragment_count: usize,
    /// Average fragment size
    pub avg_fragment_size: usize,
    /// Fragmentation severity (Low, Medium, High)
    pub severity: FragmentationSeverity,
}

/// Fragmentation severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FragmentationSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Memory leak detection result
#[derive(Debug, Clone)]
pub struct MemoryLeak {
    /// Leak ID
    pub id: u64,
    /// Estimated leaked bytes
    pub leaked_bytes: usize,
    /// Leak detection timestamp
    pub detected_at: Instant,
    /// Allocation location
    pub location: String,
    /// Leak category/type
    pub category: String,
    /// Confidence score (0.0-1.0)
    pub confidence: f64,
    /// Stack trace if available
    pub stack_trace: Option<Vec<String>>,
}

/// Memory profiler for tracking allocations
pub struct MemoryProfiler {
    /// Configuration
    config: MemoryManagerConfig,
    /// Allocation tracking
    allocations: Arc<RwLock<HashMap<u64, AllocationInfo>>>,
    /// Memory statistics
    stats: Arc<RwLock<MemoryStats>>,
    /// Allocation counter
    allocation_counter: AtomicU64,
    /// Current memory usage
    current_memory: AtomicUsize,
    /// Peak memory usage
    peak_memory: AtomicUsize,
    /// Potential memory leaks
    detected_leaks: Arc<RwLock<Vec<MemoryLeak>>>,
}

impl MemoryProfiler {
    /// Create a new memory profiler
    pub fn new(config: MemoryManagerConfig) -> Self {
        Self {
            config,
            allocations: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(MemoryStats {
                total_allocated: 0,
                total_deallocated: 0,
                current_allocated: 0,
                peak_memory: 0,
                allocation_count: 0,
                deallocation_count: 0,
                avg_allocation_size: 0,
                fragmentation_ratio: 0.0,
                gc_count: 0,
                total_gc_time: Duration::from_secs(0),
                last_gc: None,
            })),
            allocation_counter: AtomicU64::new(1),
            current_memory: AtomicUsize::new(0),
            peak_memory: AtomicUsize::new(0),
            detected_leaks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Track a memory allocation
    pub async fn track_allocation(
        &self,
        size: usize,
        location: String,
        category: String,
    ) -> PerformanceResult<u64> {
        let allocation_id = self.allocation_counter.fetch_add(1, Ordering::SeqCst);
        
        let allocation = AllocationInfo {
            id: allocation_id,
            size,
            allocated_at: Instant::now(),
            location,
            category,
            is_active: true,
            deallocated_at: None,
        };

        // Store location before moving allocation
        let allocation_location = allocation.location.clone();
        
        // Update tracking
        {
            let mut allocations = self.allocations.write().await;
            allocations.insert(allocation_id, allocation);
        }

        // Update statistics
        let current = self.current_memory.fetch_add(size, Ordering::SeqCst) + size;
        let mut peak = self.peak_memory.load(Ordering::SeqCst);
        while current > peak {
            match self.peak_memory.compare_exchange_weak(
                peak, current, Ordering::SeqCst, Ordering::SeqCst
            ) {
                Ok(_) => break,
                Err(actual) => peak = actual,
            }
        }

        {
            let mut stats = self.stats.write().await;
            stats.total_allocated += size;
            stats.current_allocated = current;
            stats.peak_memory = self.peak_memory.load(Ordering::SeqCst);
            stats.allocation_count += 1;
            stats.avg_allocation_size = stats.total_allocated / stats.allocation_count as usize;
        }

        // Check for large allocations
        if self.config.large_allocation_threshold > 0 && size >= self.config.large_allocation_threshold {
            tracing::warn!(
                "Large allocation detected: {} bytes at {}",
                size,
                allocation_location
            );
        }

        Ok(allocation_id)
    }

    /// Track a memory deallocation
    pub async fn track_deallocation(&self, allocation_id: u64) -> PerformanceResult<()> {
        let mut allocations = self.allocations.write().await;
        
        if let Some(allocation) = allocations.get_mut(&allocation_id) {
            if allocation.is_active {
                allocation.is_active = false;
                allocation.deallocated_at = Some(Instant::now());
                
                // Update current memory usage
                self.current_memory.fetch_sub(allocation.size, Ordering::SeqCst);
                
                // Update statistics
                let mut stats = self.stats.write().await;
                stats.total_deallocated += allocation.size;
                stats.current_allocated = self.current_memory.load(Ordering::SeqCst);
                stats.deallocation_count += 1;
            }
        }

        Ok(())
    }

    /// Get current memory statistics
    pub async fn get_memory_stats(&self) -> MemoryStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Detect potential memory leaks
    pub async fn detect_memory_leaks(&self) -> PerformanceResult<Vec<MemoryLeak>> {
        if !self.config.leak_detection_enabled {
            return Ok(Vec::new());
        }

        let allocations = self.allocations.read().await;
        let mut detected_leaks = self.detected_leaks.write().await;
        let current_time = Instant::now();
        
        // Look for long-lived allocations
        let leak_threshold = Duration::from_secs(3600); // 1 hour
        let mut new_leaks = Vec::new();
        
        for (id, allocation) in allocations.iter() {
            if allocation.is_active && 
               current_time.duration_since(allocation.allocated_at) > leak_threshold {
                
                // Calculate confidence based on allocation age and size
                let age_hours = current_time.duration_since(allocation.allocated_at).as_secs() as f64 / 3600.0;
                let size_factor = (allocation.size as f64 / 1024.0).ln().max(1.0) / 10.0;
                let confidence = (age_hours / 24.0 * size_factor).min(1.0);
                
                let leak = MemoryLeak {
                    id: *id,
                    leaked_bytes: allocation.size,
                    detected_at: current_time,
                    location: allocation.location.clone(),
                    category: allocation.category.clone(),
                    confidence,
                    stack_trace: None, // Would be populated with actual stack trace
                };
                
                new_leaks.push(leak);
            }
        }
        
        // Add new leaks to detection history
        for leak in &new_leaks {
            detected_leaks.push(leak.clone());
        }
        
        // Limit leak history size
        if detected_leaks.len() > 1000 {
            detected_leaks.truncate(800);
        }

        Ok(new_leaks)
    }

    /// Get all detected memory leaks
    pub async fn get_detected_leaks(&self) -> Vec<MemoryLeak> {
        let detected_leaks = self.detected_leaks.read().await;
        detected_leaks.clone()
    }

    /// Clean up old allocation records to prevent unbounded growth
    pub async fn cleanup_old_allocations(&self, retention_hours: u64) -> PerformanceResult<usize> {
        let cutoff = Instant::now() - Duration::from_secs(retention_hours * 3600);
        let mut allocations = self.allocations.write().await;
        let mut removed_count = 0;
        
        // Remove old, inactive allocations
        let keys_to_remove: Vec<u64> = allocations
            .iter()
            .filter(|(_, allocation)| {
                !allocation.is_active &&
                allocation.deallocated_at
                    .map(|dealloc_time| dealloc_time < cutoff)
                    .unwrap_or(false)
            })
            .map(|(id, _)| *id)
            .collect();
        
        for key in keys_to_remove {
            allocations.remove(&key);
            removed_count += 1;
        }

        Ok(removed_count)
    }
}

/// Allocation tracker for specific memory categories
pub struct AllocationTracker {
    /// Category name
    category: String,
    /// Profiler reference
    profiler: Arc<MemoryProfiler>,
    /// Category-specific stats
    category_stats: Arc<RwLock<CategoryStats>>,
}

/// Statistics for a specific allocation category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryStats {
    pub category: String,
    pub total_allocated: usize,
    pub current_allocated: usize,
    pub peak_allocated: usize,
    pub allocation_count: u64,
    pub avg_allocation_size: usize,
    pub last_allocation: Option<Instant>,
}

impl AllocationTracker {
    /// Create a new allocation tracker for a category
    pub fn new(category: String, profiler: Arc<MemoryProfiler>) -> Self {
        Self {
            category: category.clone(),
            profiler,
            category_stats: Arc::new(RwLock::new(CategoryStats {
                category,
                total_allocated: 0,
                current_allocated: 0,
                peak_allocated: 0,
                allocation_count: 0,
                avg_allocation_size: 0,
                last_allocation: None,
            })),
        }
    }

    /// Track allocation for this category
    pub async fn allocate(&self, size: usize, location: String) -> PerformanceResult<u64> {
        let allocation_id = self.profiler.track_allocation(
            size,
            location,
            self.category.clone(),
        ).await?;

        // Update category stats
        {
            let mut stats = self.category_stats.write().await;
            stats.total_allocated += size;
            stats.current_allocated += size;
            stats.peak_allocated = stats.peak_allocated.max(stats.current_allocated);
            stats.allocation_count += 1;
            stats.avg_allocation_size = stats.total_allocated / stats.allocation_count as usize;
            stats.last_allocation = Some(Instant::now());
        }

        Ok(allocation_id)
    }

    /// Track deallocation for this category
    pub async fn deallocate(&self, allocation_id: u64, size: usize) -> PerformanceResult<()> {
        self.profiler.track_deallocation(allocation_id).await?;

        // Update category stats
        {
            let mut stats = self.category_stats.write().await;
            stats.current_allocated = stats.current_allocated.saturating_sub(size);
        }

        Ok(())
    }

    /// Get category statistics
    pub async fn get_stats(&self) -> CategoryStats {
        let stats = self.category_stats.read().await;
        stats.clone()
    }
}

/// Garbage collector optimizer
pub struct GarbageCollector {
    config: MemoryManagerConfig,
    gc_stats: Arc<RwLock<GCStats>>,
}

/// Garbage collection statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GCStats {
    pub total_collections: u64,
    pub total_gc_time: Duration,
    pub avg_gc_time: Duration,
    pub last_gc_time: Option<Instant>,
    pub bytes_collected: usize,
    pub objects_collected: u64,
    pub gc_efficiency: f64,
}

impl GarbageCollector {
    /// Create a new garbage collector
    pub fn new(config: MemoryManagerConfig) -> Self {
        Self {
            config,
            gc_stats: Arc::new(RwLock::new(GCStats {
                total_collections: 0,
                total_gc_time: Duration::from_secs(0),
                avg_gc_time: Duration::from_secs(0),
                last_gc_time: None,
                bytes_collected: 0,
                objects_collected: 0,
                gc_efficiency: 0.0,
            })),
        }
    }

    /// Trigger garbage collection optimization
    pub async fn optimize_gc(&self) -> PerformanceResult<GCResult> {
        if !self.config.auto_gc_optimization {
            return Ok(GCResult {
                bytes_freed: 0,
                objects_freed: 0,
                gc_duration: Duration::from_secs(0),
                efficiency: 0.0,
            });
        }

        let start_time = Instant::now();
        
        // This would trigger actual garbage collection
        // For now, we simulate the process
        let bytes_freed = simulate_gc_collection().await;
        let objects_freed = bytes_freed / 64; // Assume average object size
        let gc_duration = start_time.elapsed();
        
        // Update statistics
        {
            let mut stats = self.gc_stats.write().await;
            stats.total_collections += 1;
            stats.total_gc_time += gc_duration;
            stats.avg_gc_time = stats.total_gc_time / stats.total_collections as u32;
            stats.last_gc_time = Some(start_time);
            stats.bytes_collected += bytes_freed;
            stats.objects_collected += objects_freed;
            
            // Calculate efficiency (bytes collected per millisecond)
            let gc_ms = gc_duration.as_millis().max(1) as f64;
            stats.gc_efficiency = bytes_freed as f64 / gc_ms;
        }

        let efficiency = if gc_duration.as_millis() > 0 {
            bytes_freed as f64 / gc_duration.as_millis() as f64
        } else {
            0.0
        };

        Ok(GCResult {
            bytes_freed,
            objects_freed,
            gc_duration,
            efficiency,
        })
    }

    /// Get garbage collection statistics
    pub async fn get_gc_stats(&self) -> GCStats {
        let stats = self.gc_stats.read().await;
        stats.clone()
    }

    /// Check if garbage collection should be triggered
    pub async fn should_trigger_gc(&self, memory_pressure: f64) -> bool {
        memory_pressure > self.config.pressure_threshold
    }
}

/// Garbage collection result
#[derive(Debug, Clone)]
pub struct GCResult {
    pub bytes_freed: usize,
    pub objects_freed: u64,
    pub gc_duration: Duration,
    pub efficiency: f64,
}

/// Heap analyzer for memory layout analysis
pub struct HeapAnalyzer {
    config: MemoryManagerConfig,
}

impl HeapAnalyzer {
    /// Create a new heap analyzer
    pub fn new(config: MemoryManagerConfig) -> Self {
        Self { config }
    }

    /// Analyze current heap state
    pub async fn analyze_heap(&self) -> PerformanceResult<HeapAnalysis> {
        if !self.config.heap_analysis_enabled {
            return Err(PerformanceError::ConfigurationError(
                "Heap analysis is disabled".to_string()
            ));
        }

        // This would integrate with actual heap introspection
        // For now, we simulate the analysis
        let heap_info = simulate_heap_analysis().await;
        
        Ok(heap_info)
    }

    /// Detect heap fragmentation
    pub async fn analyze_fragmentation(&self) -> PerformanceResult<FragmentationInfo> {
        let heap_analysis = self.analyze_heap().await?;
        Ok(heap_analysis.fragmentation_info)
    }

    /// Recommend heap optimization actions
    pub async fn recommend_optimizations(&self) -> PerformanceResult<Vec<HeapOptimization>> {
        let analysis = self.analyze_heap().await?;
        let mut recommendations = Vec::new();

        // Check fragmentation
        match analysis.fragmentation_info.severity {
            FragmentationSeverity::High | FragmentationSeverity::Critical => {
                recommendations.push(HeapOptimization {
                    optimization_type: "Defragmentation".to_string(),
                    description: "High heap fragmentation detected - consider triggering compaction".to_string(),
                    priority: OptimizationPriority::High,
                    estimated_benefit: 0.4,
                });
            }
            FragmentationSeverity::Medium => {
                recommendations.push(HeapOptimization {
                    optimization_type: "Monitoring".to_string(),
                    description: "Moderate fragmentation - monitor and consider future optimization".to_string(),
                    priority: OptimizationPriority::Medium,
                    estimated_benefit: 0.2,
                });
            }
            FragmentationSeverity::Low => {}
        }

        // Check utilization
        if analysis.utilization_ratio > 0.9 {
            recommendations.push(HeapOptimization {
                optimization_type: "Heap Expansion".to_string(),
                description: "High heap utilization - consider increasing heap size".to_string(),
                priority: OptimizationPriority::High,
                estimated_benefit: 0.3,
            });
        }

        // Check for large free blocks
        if analysis.largest_free_block < analysis.avg_block_size * 2 {
            recommendations.push(HeapOptimization {
                optimization_type: "Memory Compaction".to_string(),
                description: "Limited large free blocks available - compaction may help".to_string(),
                priority: OptimizationPriority::Medium,
                estimated_benefit: 0.25,
            });
        }

        Ok(recommendations)
    }
}

/// Heap optimization recommendation
#[derive(Debug, Clone)]
pub struct HeapOptimization {
    pub optimization_type: String,
    pub description: String,
    pub priority: OptimizationPriority,
    pub estimated_benefit: f64,
}

/// Optimization priority levels
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptimizationPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// Memory manager that coordinates all memory optimization components
pub struct MemoryManager {
    config: MemoryManagerConfig,
    profiler: Arc<MemoryProfiler>,
    gc: Arc<GarbageCollector>,
    heap_analyzer: Arc<HeapAnalyzer>,
    category_trackers: Arc<RwLock<HashMap<String, Arc<AllocationTracker>>>>,
}

impl MemoryManager {
    /// Create a new memory manager
    pub fn new(config: MemoryManagerConfig) -> Self {
        let profiler = Arc::new(MemoryProfiler::new(config.clone()));
        let gc = Arc::new(GarbageCollector::new(config.clone()));
        let heap_analyzer = Arc::new(HeapAnalyzer::new(config.clone()));

        Self {
            config,
            profiler: profiler.clone(),
            gc,
            heap_analyzer,
            category_trackers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get allocation tracker for a category
    pub async fn get_tracker(&self, category: &str) -> Arc<AllocationTracker> {
        let mut trackers = self.category_trackers.write().await;
        
        trackers.entry(category.to_string())
            .or_insert_with(|| {
                Arc::new(AllocationTracker::new(
                    category.to_string(),
                    self.profiler.clone(),
                ))
            })
            .clone()
    }

    /// Get comprehensive memory statistics
    pub async fn get_memory_statistics(&self) -> PerformanceResult<MemoryStatistics> {
        let memory_stats = self.profiler.get_memory_stats().await;
        let gc_stats = self.gc.get_gc_stats().await;
        let heap_analysis = self.heap_analyzer.analyze_heap().await?;
        let detected_leaks = self.profiler.get_detected_leaks().await;
        
        // Get category statistics
        let trackers = self.category_trackers.read().await;
        let mut category_stats = HashMap::new();
        for (category, tracker) in trackers.iter() {
            category_stats.insert(category.clone(), tracker.get_stats().await);
        }

        Ok(MemoryStatistics {
            overall_stats: memory_stats,
            gc_stats,
            heap_analysis,
            category_stats,
            detected_leaks: detected_leaks.len(),
            last_updated: Some(Instant::now()),
        })
    }

    /// Run comprehensive memory optimization
    pub async fn optimize_memory(&self) -> PerformanceResult<MemoryOptimizationResult> {
        let mut optimizations = Vec::new();
        let mut total_bytes_freed = 0;

        // Run garbage collection
        let gc_result = self.gc.optimize_gc().await?;
        total_bytes_freed += gc_result.bytes_freed;
        optimizations.push(format!(
            "GC freed {} bytes in {:?}",
            gc_result.bytes_freed,
            gc_result.gc_duration
        ));

        // Detect memory leaks
        let leaks = self.profiler.detect_memory_leaks().await?;
        if !leaks.is_empty() {
            optimizations.push(format!("Detected {} potential memory leaks", leaks.len()));
        }

        // Clean up old allocation records
        let cleaned_allocations = self.profiler.cleanup_old_allocations(24).await?;
        optimizations.push(format!("Cleaned up {} old allocation records", cleaned_allocations));

        // Get heap recommendations
        let heap_recommendations = self.heap_analyzer.recommend_optimizations().await?;
        for rec in heap_recommendations {
            optimizations.push(format!("{}: {}", rec.optimization_type, rec.description));
        }

        Ok(MemoryOptimizationResult {
            total_bytes_freed,
            optimizations_applied: optimizations,
            memory_leaks_detected: leaks.len(),
            cleanup_records_removed: cleaned_allocations,
            gc_result,
        })
    }
}

/// Comprehensive memory statistics
#[derive(Debug, Clone)]
pub struct MemoryStatistics {
    pub overall_stats: MemoryStats,
    pub gc_stats: GCStats,
    pub heap_analysis: HeapAnalysis,
    pub category_stats: HashMap<String, CategoryStats>,
    pub detected_leaks: usize,
    pub last_updated: Option<Instant>,
}

/// Memory optimization result
#[derive(Debug, Clone)]
pub struct MemoryOptimizationResult {
    pub total_bytes_freed: usize,
    pub optimizations_applied: Vec<String>,
    pub memory_leaks_detected: usize,
    pub cleanup_records_removed: usize,
    pub gc_result: GCResult,
}

// Helper simulation functions
async fn simulate_gc_collection() -> usize {
    // Simulate GC collection - in practice this would trigger actual GC
    tokio::time::sleep(Duration::from_millis(10)).await;
    1024 * 1024 // Assume 1MB freed
}

async fn simulate_heap_analysis() -> HeapAnalysis {
    // Simulate heap analysis - in practice this would inspect actual heap
    tokio::time::sleep(Duration::from_millis(5)).await;
    
    HeapAnalysis {
        total_heap_size: 64 * 1024 * 1024, // 64MB
        used_heap_size: 48 * 1024 * 1024,  // 48MB
        free_heap_size: 16 * 1024 * 1024,  // 16MB
        utilization_ratio: 0.75,
        block_count: 1000,
        avg_block_size: 64 * 1024, // 64KB
        largest_free_block: 2 * 1024 * 1024, // 2MB
        fragmentation_info: FragmentationInfo {
            external_fragmentation: 0.15,
            internal_fragmentation: 0.08,
            free_fragment_count: 50,
            avg_fragment_size: 32 * 1024, // 32KB
            severity: FragmentationSeverity::Low,
        },
    }
}

impl Default for MemoryManagerConfig {
    fn default() -> Self {
        Self {
            auto_gc_optimization: true,
            pressure_threshold: 0.8,
            leak_detection_enabled: true,
            large_allocation_threshold: 1024 * 1024, // 1MB
            profiling_enabled: true,
            monitoring_interval: Duration::from_secs(60),
            heap_analysis_enabled: true,
            warning_threshold_bytes: 512 * 1024 * 1024, // 512MB
            critical_threshold_bytes: 1024 * 1024 * 1024, // 1GB
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_profiler() {
        let config = MemoryManagerConfig::default();
        let profiler = MemoryProfiler::new(config);

        // Track an allocation
        let allocation_id = profiler.track_allocation(
            1024,
            "test.rs:100".to_string(),
            "test".to_string(),
        ).await.unwrap();

        // Check statistics
        let stats = profiler.get_memory_stats().await;
        assert_eq!(stats.allocation_count, 1);
        assert_eq!(stats.current_allocated, 1024);

        // Track deallocation
        profiler.track_deallocation(allocation_id).await.unwrap();

        let stats = profiler.get_memory_stats().await;
        assert_eq!(stats.deallocation_count, 1);
        assert_eq!(stats.current_allocated, 0);
    }

    #[tokio::test]
    async fn test_allocation_tracker() {
        let config = MemoryManagerConfig::default();
        let profiler = Arc::new(MemoryProfiler::new(config));
        let tracker = AllocationTracker::new("test_category".to_string(), profiler);

        // Track allocation
        let allocation_id = tracker.allocate(2048, "test.rs:200".to_string()).await.unwrap();

        // Check category stats
        let stats = tracker.get_stats().await;
        assert_eq!(stats.category, "test_category");
        assert_eq!(stats.total_allocated, 2048);
        assert_eq!(stats.current_allocated, 2048);

        // Track deallocation
        tracker.deallocate(allocation_id, 2048).await.unwrap();

        let stats = tracker.get_stats().await;
        assert_eq!(stats.current_allocated, 0);
    }

    #[tokio::test]
    async fn test_garbage_collector() {
        let config = MemoryManagerConfig::default();
        let gc = GarbageCollector::new(config);

        // Test GC optimization
        let result = gc.optimize_gc().await.unwrap();
        assert!(result.bytes_freed > 0);

        // Check if GC should trigger
        let should_trigger = gc.should_trigger_gc(0.9).await;
        assert!(should_trigger);
    }

    #[tokio::test]
    async fn test_memory_manager() {
        let config = MemoryManagerConfig::default();
        let manager = MemoryManager::new(config);

        // Get tracker and use it
        let tracker = manager.get_tracker("test").await;
        let _allocation_id = tracker.allocate(1024, "test.rs:300".to_string()).await.unwrap();

        // Get comprehensive statistics
        let stats = manager.get_memory_statistics().await.unwrap();
        assert!(stats.overall_stats.allocation_count > 0);
        assert!(stats.category_stats.contains_key("test"));

        // Run optimization
        let optimization_result = manager.optimize_memory().await.unwrap();
        assert!(!optimization_result.optimizations_applied.is_empty());
    }
}