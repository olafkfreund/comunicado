//! Performance optimization module
//!
//! This module provides comprehensive performance optimizations including:
//! - Async background processing for non-blocking operations
//! - Intelligent caching systems for messages and folders
//! - Real-time progress indicators and status updates
//! - Optimized startup process with deferred loading
//! - UI responsiveness improvements

pub mod cache;
pub mod background_processor;
pub mod progress_tracker;
pub mod startup_optimizer;

// Re-export main types for easy access
pub use cache::{MessageCache, FolderCache, CacheManager, CacheSettings, CacheStats};
pub use background_processor::{
    BackgroundProcessor, BackgroundTask, BackgroundTaskType, TaskPriority, TaskStatus, 
    TaskResult, ProcessorSettings
};
pub use progress_tracker::{
    ProgressTracker, ProgressUpdate, ProgressStatus, ProgressBuilder
};
pub use startup_optimizer::{
    StartupOptimizer, StartupPhase, StartupProgress, StartupSettings, 
    DeferredTask, DeferredTaskPriority
};

use std::sync::Arc;
use tokio::sync::mpsc;

/// Comprehensive performance optimization system
pub struct PerformanceSystem {
    /// Cache manager for messages and folders
    pub cache_manager: Arc<CacheManager>,
    /// Background task processor
    pub background_processor: Arc<BackgroundProcessor>,
    /// Progress tracker for UI updates
    pub progress_tracker: Arc<ProgressTracker>,
    /// Startup optimizer
    pub startup_optimizer: Arc<StartupOptimizer>,
}

impl PerformanceSystem {
    /// Create a new performance system with default settings
    pub fn new() -> Self {
        let (progress_tx, _) = mpsc::unbounded_channel();
        let (completion_tx, _) = mpsc::unbounded_channel();
        
        Self {
            cache_manager: Arc::new(CacheManager::new()),
            background_processor: Arc::new(BackgroundProcessor::new_standalone(progress_tx, completion_tx)),
            progress_tracker: Arc::new(ProgressTracker::new()),
            startup_optimizer: Arc::new(StartupOptimizer::new()),
        }
    }

    /// Initialize all performance systems
    pub async fn initialize(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Start background processor
        self.background_processor.start().await?;
        
        // Start cache cleanup task
        self.cache_manager.start_cleanup_task().await;
        
        // Run optimized startup
        self.startup_optimizer.start_optimized_startup().await
            .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as Box<dyn std::error::Error + Send + Sync>)?;

        Ok(())
    }

    /// Shutdown performance systems
    pub async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.background_processor.stop().await?;
        Ok(())
    }

    /// Get system-wide performance statistics
    pub async fn get_performance_stats(&self) -> PerformanceStats {
        let (message_cache_stats, folder_cache_stats) = self.cache_manager.get_comprehensive_stats().await;
        let active_operations = self.progress_tracker.active_operation_count().await;
        let startup_progress = self.startup_optimizer.get_progress().await;
        
        PerformanceStats {
            message_cache_hit_rate: message_cache_stats.hit_rate(),
            folder_cache_hit_rate: folder_cache_stats.hit_rate(),
            active_background_operations: active_operations,
            startup_complete: startup_progress.current_phase == StartupPhase::Complete,
            total_startup_time: self.startup_optimizer.total_startup_time(),
        }
    }
}

impl Default for PerformanceSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// System-wide performance statistics
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub message_cache_hit_rate: f64,
    pub folder_cache_hit_rate: f64,
    pub active_background_operations: usize,
    pub startup_complete: bool,
    pub total_startup_time: std::time::Duration,
}