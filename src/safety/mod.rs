//! Safety and stability module for preventing system crashes

use std::time::{Duration, Instant};
use tracing::{warn, error};

/// Resource monitor to prevent system overload
pub struct ResourceMonitor {
    last_memory_check: Instant,
    memory_check_interval: Duration,
    memory_limit_mb: usize,
    high_memory_warnings: usize,
}

impl ResourceMonitor {
    pub fn new() -> Self {
        Self {
            last_memory_check: Instant::now(),
            memory_check_interval: Duration::from_secs(10),
            memory_limit_mb: 500, // 500MB limit
            high_memory_warnings: 0,
        }
    }

    /// Check if we should proceed with operations based on resource usage
    pub fn should_proceed(&mut self) -> bool {
        if self.last_memory_check.elapsed() >= self.memory_check_interval {
            self.check_memory_usage();
            self.last_memory_check = Instant::now();
        }
        
        // If we've had too many memory warnings, be more conservative
        self.high_memory_warnings < 5
    }

    fn check_memory_usage(&mut self) {
        // Simple memory check using /proc/self/status on Linux
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            if let Some(line) = status.lines().find(|line| line.starts_with("VmRSS:")) {
                if let Some(kb_str) = line.split_whitespace().nth(1) {
                    if let Ok(kb) = kb_str.parse::<usize>() {
                        let mb = kb / 1024;
                        if mb > self.memory_limit_mb {
                            warn!("High memory usage detected: {} MB", mb);
                            self.high_memory_warnings += 1;
                            
                            if mb > self.memory_limit_mb * 2 {
                                error!("Excessive memory usage: {} MB - system may be unstable", mb);
                            }
                        } else if self.high_memory_warnings > 0 {
                            // Memory usage is back to normal
                            self.high_memory_warnings = self.high_memory_warnings.saturating_sub(1);
                        }
                    }
                }
            }
        }
    }

    /// Force garbage collection if available (Rust doesn't have GC but we can drop large objects)
    pub fn optimize_memory(&self) {
        // In Rust, we can't force GC, but we can suggest the allocator to release memory
        // This is a placeholder for memory optimization strategies
        std::hint::spin_loop(); // Small delay to let system breathe
    }
}

/// Safe event loop wrapper that prevents infinite loops
pub struct SafeEventLoop {
    max_iterations: usize,
    current_iterations: usize,
    start_time: Instant,
    max_runtime: Duration,
}

impl SafeEventLoop {
    pub fn new() -> Self {
        Self {
            max_iterations: 10000, // Maximum loop iterations before forced break
            current_iterations: 0,
            start_time: Instant::now(),
            max_runtime: Duration::from_secs(300), // 5 minutes max runtime
        }
    }

    /// Check if it's safe to continue the loop
    pub fn should_continue(&mut self) -> bool {
        self.current_iterations += 1;

        // Check iteration limit
        if self.current_iterations >= self.max_iterations {
            error!("Event loop iteration limit reached: {}", self.max_iterations);
            return false;
        }

        // Check runtime limit
        if self.start_time.elapsed() >= self.max_runtime {
            error!("Event loop runtime limit reached: {:?}", self.max_runtime);
            return false;
        }

        // Reset iteration count periodically
        if self.current_iterations % 1000 == 0 {
            warn!("Event loop high iteration count: {}", self.current_iterations);
        }

        true
    }

    /// Reset the safety counters (call this when the loop successfully yields control)
    pub fn reset_if_idle(&mut self) {
        if self.current_iterations > 100 && self.start_time.elapsed() > Duration::from_millis(100) {
            // Reset counters if we've been running for a while and processing events normally
            self.current_iterations = 0;
            self.start_time = Instant::now();
        }
    }
}

impl Default for ResourceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for SafeEventLoop {
    fn default() -> Self {
        Self::new()
    }
}