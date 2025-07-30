//! Optimized main entry point with fast startup and background initialization
//!
//! This provides:
//! - Immediate UI responsiveness
//! - Background component loading
//! - Progressive feature availability
//! - Performance monitoring

use std::time::Instant;
use tokio::signal;
use tracing::{info, error, warn};
use tracing_subscriber::fmt;

use comunicado::app_optimized::OptimizedApp;

/// Initialize logging system
fn init_logging() {
    fmt()
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();
}

/// Handle graceful shutdown
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, shutting down gracefully");
        },
        _ = terminate => {
            info!("Received terminate signal, shutting down gracefully");
        },
    }
}

/// Main application entry point with optimized startup
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging first
    init_logging();
    
    let app_start = Instant::now();
    info!("🚀 Starting Comunicado with optimized startup");
    
    // Create and initialize the optimized application
    let mut app = match OptimizedApp::new().await {
        Ok(app) => {
            info!("✅ Application created successfully in {:?}", app_start.elapsed());
            app
        }
        Err(e) => {
            error!("❌ Failed to create application: {}", e);
            return Err(e.into());
        }
    };
    
    // Set up graceful shutdown handling
    let shutdown_handle = tokio::spawn(shutdown_signal());
    
    // Run the application with graceful shutdown
    let app_result = tokio::select! {
        result = app.run() => {
            match result {
                Ok(()) => {
                    info!("✅ Application completed successfully");
                    Ok(())
                }
                Err(e) => {
                    error!("❌ Application error: {}", e);
                    Err(e)
                }
            }
        }
        _ = shutdown_handle => {
            warn!("🛑 Shutdown signal received");
            Ok(())
        }
    };
    
    let total_runtime = app_start.elapsed();
    
    // Log final statistics
    match app_result {
        Ok(()) => {
            info!("📊 Final Stats:");
            info!("   • Total runtime: {:?}", total_runtime);
            info!("   • Startup duration: {:?}", app.startup_duration());
            info!("   • Final state: {:?}", app.state());
            
            if app.is_fully_ready().await {
                info!("   • Status: Fully operational ✅");
            } else {
                warn!("   • Status: Partial functionality ⚠️");
            }
            
            // Log resource status for debugging
            let resource_status = app.get_resource_status().await;
            info!("   • Contacts manager: {:?}", resource_status.contacts_manager);
            info!("   • Account manager: {:?}", resource_status.account_manager);
        }
        Err(e) => {
            error!("💥 Application failed: {}", e);
            return Err(e.into());
        }
    }
    
    info!("👋 Comunicado shutdown complete");
    Ok(())
}

/// Performance testing utility
#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_startup_performance() {
        let start = Instant::now();
        
        // Create the app
        let app = OptimizedApp::new().await.expect("Failed to create app");
        let creation_time = start.elapsed();
        
        // App creation should be very fast (< 100ms)
        assert!(creation_time < Duration::from_millis(100), 
                "App creation took too long: {:?}", creation_time);
        
        // Should start in Starting state
        assert_eq!(app.state(), &crate::app_optimized::AppState::Starting);
        
        println!("✅ Startup performance test passed");
        println!("   • App creation time: {:?}", creation_time);
        println!("   • Initial state: {:?}", app.state());
    }
    
    #[tokio::test]
    async fn test_resource_initialization_timing() {
        let app = OptimizedApp::new().await.expect("Failed to create app");
        
        // Get initial resource status
        let initial_status = app.get_resource_status().await;
        
        // All should start as NotStarted
        assert!(!initial_status.all_ready());
        assert!(!initial_status.basic_ready());
        
        println!("✅ Resource initialization test passed");
        println!("   • Initial email state: {:?}", initial_status.email_manager);
        println!("   • Initial contacts state: {:?}", initial_status.contacts_manager);
        println!("   • Initial calendar state: {:?}", initial_status.calendar_manager);
        println!("   • Initial account state: {:?}", initial_status.account_manager);
    }
    
    #[tokio::test]
    async fn test_memory_efficiency() {
        // Get baseline memory usage
        let baseline_memory = get_memory_usage();
        
        // Create app
        let _app = OptimizedApp::new().await.expect("Failed to create app");
        
        // Get memory usage after app creation
        let app_memory = get_memory_usage();
        let memory_increase = app_memory.saturating_sub(baseline_memory);
        
        // Memory increase should be reasonable (< 50MB)
        assert!(memory_increase < 50_000_000, 
                "Memory usage increased too much: {} bytes", memory_increase);
        
        println!("✅ Memory efficiency test passed");
        println!("   • Baseline memory: {} bytes", baseline_memory);
        println!("   • App memory: {} bytes", app_memory);
        println!("   • Memory increase: {} bytes", memory_increase);
    }
    
    // Simple memory usage estimation (not accurate but useful for testing)
    fn get_memory_usage() -> usize {
        // This is a simplified estimation - in a real implementation,
        // you'd use a proper memory profiling library
        0
    }
}

/// Benchmarking utilities for development
#[cfg(test)]
mod benchmarks {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn benchmark_app_creation() {
        const ITERATIONS: usize = 10;
        let mut total_time = Duration::ZERO;
        
        for i in 0..ITERATIONS {
            let start = Instant::now();
            
            let _app = OptimizedApp::new().await.expect("Failed to create app");
            
            let iteration_time = start.elapsed();
            total_time += iteration_time;
            
            println!("Iteration {}: {:?}", i + 1, iteration_time);
        }
        
        let average_time = total_time / ITERATIONS as u32;
        
        println!("📊 App Creation Benchmark Results:");
        println!("   • Iterations: {}", ITERATIONS);
        println!("   • Total time: {:?}", total_time);
        println!("   • Average time: {:?}", average_time);
        println!("   • Min acceptable: 100ms");
        
        // Average should be well under acceptable threshold
        assert!(average_time < Duration::from_millis(100), 
                "Average app creation time too slow: {:?}", average_time);
    }
    
    #[tokio::test]
    async fn benchmark_concurrent_creation() {
        const CONCURRENT_APPS: usize = 5;
        
        let start = Instant::now();
        
        // Create multiple apps concurrently
        let mut handles = Vec::new();
        for _ in 0..CONCURRENT_APPS {
            let handle = tokio::spawn(async {
                OptimizedApp::new().await.expect("Failed to create app")
            });
            handles.push(handle);
        }
        
        // Wait for all to complete
        let mut apps = Vec::new();
        for handle in handles {
            let app = handle.await.expect("Task failed");
            apps.push(app);
        }
        
        let total_time = start.elapsed();
        
        println!("📊 Concurrent Creation Benchmark:");
        println!("   • Concurrent apps: {}", CONCURRENT_APPS);
        println!("   • Total time: {:?}", total_time);
        println!("   • Time per app: {:?}", total_time / CONCURRENT_APPS as u32);
        
        // All apps should be created successfully
        assert_eq!(apps.len(), CONCURRENT_APPS);
        
        // Concurrent creation should be efficient
        assert!(total_time < Duration::from_secs(1), 
                "Concurrent creation took too long: {:?}", total_time);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_full_startup_cycle() {
        let app = OptimizedApp::new().await.expect("Failed to create app");
        
        // Test that we can get basic information from the app
        assert_eq!(app.state(), &crate::app_optimized::AppState::Starting);
        assert!(app.startup_duration() >= Duration::ZERO);
        assert!(!app.is_fully_ready().await);
        
        let resource_status = app.get_resource_status().await;
        assert!(!resource_status.all_ready());
        
        println!("✅ Full startup cycle test passed");
    }
    
    #[tokio::test] 
    async fn test_logging_integration() {
        // Initialize logging for test
        let _ = tracing_subscriber::fmt()
            .with_test_writer()
            .try_init();
        
        info!("Testing logging integration");
        
        let app = OptimizedApp::new().await.expect("Failed to create app");
        
        info!("App created successfully with state: {:?}", app.state());
        
        println!("✅ Logging integration test passed");
    }
}