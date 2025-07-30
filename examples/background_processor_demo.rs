//! Background Processor Demo
//! 
//! This example demonstrates how to use the background processor
//! to handle long-running operations without blocking the UI.

use comunicado::performance::background_processor::{
    BackgroundProcessor, BackgroundTask, BackgroundTaskType, ProcessorSettings, TaskPriority,
};
use comunicado::email::sync_engine::{SyncProgress, SyncStrategy};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::init();

    println!("🚀 Background Processor Demo");
    println!("=============================");

    // Create channels for progress updates and task completion
    let (progress_tx, mut progress_rx) = mpsc::unbounded_channel::<SyncProgress>();
    let (completion_tx, mut completion_rx) = mpsc::unbounded_channel();

    // Create background processor with demo settings
    let settings = ProcessorSettings {
        max_concurrent_tasks: 3,
        task_timeout: Duration::from_secs(60),
        max_queue_size: 20,
        result_cache_size: 10,
        processing_interval: Duration::from_millis(100),
    };

    let processor = Arc::new(BackgroundProcessor::with_settings(
        progress_tx,
        completion_tx,
        settings,
    ));

    // Start the background processor
    processor.start().await?;
    println!("✅ Background processor started");

    // Demo 1: Folder Refresh Tasks
    println!("\n📂 Demo 1: Folder Refresh Tasks");
    let folders = ["INBOX", "Sent", "Drafts"];
    
    for folder in &folders {
        let task = BackgroundTask {
            id: Uuid::new_v4(),
            name: format!("Refresh {}", folder),
            priority: TaskPriority::Normal,
            account_id: "demo@example.com".to_string(),
            folder_name: Some(folder.to_string()),
            task_type: BackgroundTaskType::FolderRefresh {
                folder_name: folder.to_string()
            },
            created_at: std::time::Instant::now(),
            estimated_duration: Some(Duration::from_secs(2)),
        };

        let task_id = processor.queue_task(task).await?;
        println!("   📥 Queued refresh task for {} (ID: {})", folder, task_id);
        
        // Small delay between tasks
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Demo 2: High Priority Email Sync  
    println!("\n📧 Demo 2: High Priority Email Sync");
    let sync_task = BackgroundTask {
        id: Uuid::new_v4(),
        name: "Full Account Sync".to_string(),
        priority: TaskPriority::High,
        account_id: "demo@example.com".to_string(),
        folder_name: None,
        task_type: BackgroundTaskType::AccountSync {
            strategy: SyncStrategy::Full
        },
        created_at: std::time::Instant::now(),
        estimated_duration: Some(Duration::from_secs(10)),
    };

    let sync_task_id = processor.queue_task(sync_task).await?;
    println!("   🔄 Queued high-priority account sync (ID: {})", sync_task_id);

    // Demo 3: Search Operation
    println!("\n🔍 Demo 3: Search Operation");
    let search_task = BackgroundTask {
        id: Uuid::new_v4(),
        name: "Search 'important'".to_string(),
        priority: TaskPriority::High,
        account_id: "demo@example.com".to_string(),
        folder_name: None,
        task_type: BackgroundTaskType::Search {
            query: "important".to_string(),
            folders: vec!["INBOX".to_string(), "Sent".to_string()]
        },
        created_at: std::time::Instant::now(),
        estimated_duration: Some(Duration::from_secs(3)),
    };

    let search_task_id = processor.queue_task(search_task).await?;
    println!("   🔍 Queued search task (ID: {})", search_task_id);

    // Monitor progress and completion
    println!("\n📊 Monitoring Task Progress");
    println!("===========================");

    let mut completed_tasks = 0;
    let total_tasks = 5; // 3 folders + 1 sync + 1 search

    // Process updates for 30 seconds or until all tasks complete
    let timeout = Duration::from_secs(30);
    let start = std::time::Instant::now();

    while start.elapsed() < timeout && completed_tasks < total_tasks {
        tokio::select! {
            // Handle progress updates
            Some(progress) = progress_rx.recv() => {
                println!("📈 Progress: {} - {} ({}/{})", 
                    progress.folder_name,
                    format!("{:?}", progress.phase),
                    progress.messages_processed,
                    progress.total_messages
                );
            }
            
            // Handle task completion
            Some(result) = completion_rx.recv() => {
                println!("✅ Task completed: {:?} - Status: {:?}", 
                    result.task_id, 
                    result.status
                );
                completed_tasks += 1;
                
                if let Some(duration) = result.completed_at.map(|c| c.duration_since(result.started_at)) {
                    println!("   ⏱️  Duration: {:.2}s", duration.as_secs_f64());
                }
            }
            
            // Periodic status check
            _ = tokio::time::sleep(Duration::from_secs(2)) => {
                let queued = processor.get_queued_tasks().await;
                let running = processor.get_running_tasks().await;
                
                if !queued.is_empty() || !running.is_empty() {
                    println!("📋 Status: {} queued, {} running", queued.len(), running.len());
                }
            }
        }
    }

    // Final status
    println!("\n📊 Final Status");
    println!("===============");
    let queued = processor.get_queued_tasks().await;
    let running = processor.get_running_tasks().await;
    
    println!("Tasks completed: {}/{}", completed_tasks, total_tasks);
    println!("Tasks still queued: {}", queued.len());
    println!("Tasks still running: {}", running.len());

    // Shutdown
    processor.stop().await?;
    println!("\n✅ Background processor demo completed");

    Ok(())
}