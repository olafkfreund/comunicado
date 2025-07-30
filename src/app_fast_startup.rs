//! Fast startup implementation for App - EXPERIMENTAL
//! 
//! This module implements the performance-optimized startup system that gets
//! the UI responsive in under 3 seconds while moving heavy operations to background.
//! 
//! NOTE: This is experimental code that needs refactoring to use public App APIs

/*
// TEMPORARILY DISABLED - This experimental code accesses private App fields
// and needs to be refactored to use public App APIs

use anyhow::Result;
use std::sync::Arc;
use tokio::time::{Duration, Instant};

use crate::app::App;
use crate::performance::{
    PerformanceSystem, BackgroundTask, 
    BackgroundTaskType, TaskPriority
};
use crate::email::EmailDatabase;
use crate::oauth2::TokenManager;
use crate::imap::ImapAccountManager;
use crate::contacts::ContactsManager;

impl App {
    /// Fast startup that gets UI responsive in ~3 seconds
    /// Heavy operations are moved to background tasks
    pub async fn fast_startup(&mut self) -> Result<()> {
        let startup_start = Instant::now();
        
        // Initialize performance system
        let perf_system = PerformanceSystem::new();
        perf_system.initialize().await?;
        
        println!("🚀 Starting Comunicado (Fast Mode)...");
        
        // Phase 1: Core Systems (200ms) - Essential for UI
        self.startup_phase_1_core().await?;
        println!("✅ Core systems ready ({:?})", startup_start.elapsed());
        
        // Phase 2: Database Setup (300ms) - Required for basic functionality  
        self.startup_phase_2_database().await?;
        println!("✅ Database ready ({:?})", startup_start.elapsed());
        
        // Phase 3: UI Initialization (400ms) - Show UI to user
        self.startup_phase_3_ui().await?;
        println!("✅ UI ready ({:?})", startup_start.elapsed());
        
        // Phase 4: Background Services (100ms) - Start background processors
        self.startup_phase_4_background_services(&perf_system).await?;
        println!("✅ Background services started ({:?})", startup_start.elapsed());
        
        // Phase 5: Queue Heavy Operations (100ms) - Queue but don't wait
        self.startup_phase_5_queue_heavy_operations(&perf_system).await?;
        println!("✅ Heavy operations queued ({:?})", startup_start.elapsed());
        
        let total_startup = startup_start.elapsed();
        println!("🎉 Comunicado ready! Total startup time: {:?}", total_startup);
        println!("📋 Background operations continue loading...");
        
        // Mark initialization as complete (UI is responsive)
        self.initialization_complete = true;
        
        Ok(())
    }
    
    /// Phase 1: Initialize only the most essential systems
    async fn startup_phase_1_core(&mut self) -> Result<()> {
        // Only initialize what's absolutely required for UI
        self.storage = crate::oauth2::SecureStorage::new("comunicado".to_string())
            .map_err(|e| anyhow::anyhow!("Failed to initialize secure storage: {}", e))?;
        
        // Set startup progress
        self.startup_progress_manager.start_phase("Core")
            .map_err(|e| anyhow::anyhow!("Progress manager error: {}", e))?;
        
        // Minimal delay for essential systems
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        self.startup_progress_manager.complete_phase("Core")
            .map_err(|e| anyhow::anyhow!("Progress manager error: {}", e))?;
        
        Ok(())
    }
    
    /// Phase 2: Setup database connection (required for basic functionality)
    async fn startup_phase_2_database(&mut self) -> Result<()> {
        self.startup_progress_manager.start_phase("Database")
            .map_err(|e| anyhow::anyhow!("Progress manager error: {}", e))?;
        
        // Create database path
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("comunicado");
        
        // Create directory if it doesn't exist
        std::fs::create_dir_all(&data_dir)?;
        
        let db_path = data_dir.join("messages.db");
        let db_path_str = db_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid database path"))?;
        
        // Create database connection (this is the heavy part)
        let database = EmailDatabase::new(db_path_str)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to initialize database: {}", e))?;
        
        let database_arc = Arc::new(database);
        self.database = Some(database_arc.clone());
        
        // Set database in UI
        self.ui.set_database(database_arc);
        
        self.startup_progress_manager.complete_phase("Database")
            .map_err(|e| anyhow::anyhow!("Progress manager error: {}", e))?;
        
        Ok(())
    }
    
    /// Phase 3: Initialize UI components (show UI to user)
    async fn startup_phase_3_ui(&mut self) -> Result<()> {
        self.startup_progress_manager.start_phase("UI")
            .map_err(|e| anyhow::anyhow!("Progress manager error: {}", e))?;
        
        // UI is already initialized in new(), just mark as ready
        // Any UI-specific startup work would go here
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        self.startup_progress_manager.complete_phase("UI")
            .map_err(|e| anyhow::anyhow!("Progress manager error: {}", e))?;
        
        Ok(())
    }
    
    /// Phase 4: Start background processing systems
    async fn startup_phase_4_background_services(&mut self, perf_system: &PerformanceSystem) -> Result<()> {
        self.startup_progress_manager.start_phase("Background Services")
            .map_err(|e| anyhow::anyhow!("Progress manager error: {}", e))?;
        
        // Background processors are already started in perf_system.initialize()
        // Just mark as complete
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        self.startup_progress_manager.complete_phase("Background Services")
            .map_err(|e| anyhow::anyhow!("Progress manager error: {}", e))?;
        
        Ok(())
    }
    
    /// Phase 5: Queue heavy operations to run in background
    async fn startup_phase_5_queue_heavy_operations(&mut self, perf_system: &PerformanceSystem) -> Result<()> {
        self.startup_progress_manager.start_phase("Queue Operations")
            .map_err(|e| anyhow::anyhow!("Progress manager error: {}", e))?;
        
        // Queue account loading (this was taking 1+ seconds)
        let account_loading_task = BackgroundTask {
            id: uuid::Uuid::new_v4(),
            name: "Load Email Accounts".to_string(),
            priority: TaskPriority::High,
            account_id: "system".to_string(),
            folder_name: None,
            task_type: BackgroundTaskType::AccountSync {
                strategy: crate::email::sync_engine::SyncStrategy::HeadersOnly,
            },
            created_at: Instant::now(),
            estimated_duration: Some(Duration::from_secs(3)),
        };
        
        perf_system.background_processor.queue_task(account_loading_task).await
            .map_err(|e| anyhow::anyhow!("Failed to queue account loading: {}", e))?;
        
        // Queue notification system setup
        let notification_task = BackgroundTask {
            id: uuid::Uuid::new_v4(),
            name: "Setup Notifications".to_string(),
            priority: TaskPriority::Normal,
            account_id: "system".to_string(),
            folder_name: None,
            task_type: BackgroundTaskType::CachePreload {
                folder_name: "notifications".to_string(),
                message_count: 10,
            },
            created_at: Instant::now(),
            estimated_duration: Some(Duration::from_millis(500)),
        };
        
        perf_system.background_processor.queue_task(notification_task).await
            .map_err(|e| anyhow::anyhow!("Failed to queue notification setup: {}", e))?;
        
        // Queue contacts loading
        let contacts_task = BackgroundTask {
            id: uuid::Uuid::new_v4(),
            name: "Load Contacts".to_string(),
            priority: TaskPriority::Normal,
            account_id: "system".to_string(),
            folder_name: None,
            task_type: BackgroundTaskType::Indexing {
                folder_name: "contacts".to_string(),
            },
            created_at: Instant::now(),
            estimated_duration: Some(Duration::from_secs(1)),
        };
        
        perf_system.background_processor.queue_task(contacts_task).await
            .map_err(|e| anyhow::anyhow!("Failed to queue contacts loading: {}", e))?;
        
        // Queue plugin system loading (if enabled)
        let plugin_task = BackgroundTask {
            id: uuid::Uuid::new_v4(),
            name: "Load Plugins".to_string(),
            priority: TaskPriority::Low,
            account_id: "system".to_string(),
            folder_name: None,
            task_type: BackgroundTaskType::CachePreload {
                folder_name: "plugins".to_string(),
                message_count: 5,
            },
            created_at: Instant::now(),
            estimated_duration: Some(Duration::from_secs(2)),
        };
        
        perf_system.background_processor.queue_task(plugin_task).await
            .map_err(|e| anyhow::anyhow!("Failed to queue plugin loading: {}", e))?;
        
        self.startup_progress_manager.complete_phase("Queue Operations")
            .map_err(|e| anyhow::anyhow!("Progress manager error: {}", e))?;
        
        println!("📋 Queued 4 background tasks for heavy operations");
        Ok(())
    }
    
    /// Initialize components as they complete in background
    pub async fn handle_background_completion(&mut self, task_name: &str) -> Result<()> {
        match task_name {
            "Load Email Accounts" => {
                self.initialize_accounts_from_background().await?;
                println!("✅ Email accounts loaded in background");
            }
            "Setup Notifications" => {
                self.initialize_notifications_from_background().await?;
                println!("✅ Notifications ready");
            }
            "Load Contacts" => {
                self.initialize_contacts_from_background().await?;
                println!("✅ Contacts loaded");
            }
            "Load Plugins" => {
                println!("✅ Plugins loaded");
            }
            _ => {}
        }
        Ok(())
    }
    
    /// Initialize accounts when background task completes
    async fn initialize_accounts_from_background(&mut self) -> Result<()> {
        // This would be the heavy account initialization that was blocking startup
        if let Some(database) = &self.database {
            // Create token manager
            let token_manager = TokenManager::new();
            
            // Create IMAP manager  
            let imap_manager = Arc::new(ImapAccountManager::new(database.clone()));
            
            // Set in app
            self.token_manager = Some(token_manager);
            self.imap_manager = Some(imap_manager.clone());
            
            // Update UI
            self.ui.set_imap_manager(imap_manager);
        }
        Ok(())
    }
    
    /// Initialize notifications when background task completes
    async fn initialize_notifications_from_background(&mut self) -> Result<()> {
        if let Some(database) = &self.database {
            // Create notification manager
            let notification_manager = Arc::new(crate::email::EmailNotificationManager::new(database.clone()));
            
            // Start the notification processing
            notification_manager.start().await;
            
            // Create unified notification manager
            let notification_config = crate::notifications::NotificationConfig::default();
            let unified_notification_manager = Arc::new(
                crate::notifications::UnifiedNotificationManager::new()
                    .with_desktop_notifications(notification_config)
            );
            
            // Connect email notifications
            let email_receiver = notification_manager.subscribe();
            unified_notification_manager.connect_email_notifications(email_receiver);
            
            // Set in app
            self.notification_manager = Some(notification_manager.clone());
            self.unified_notification_manager = Some(unified_notification_manager);
            
            // Update UI
            self.ui.set_notification_manager(notification_manager);
        }
        Ok(())
    }
    
    /// Initialize contacts when background task completes
    async fn initialize_contacts_from_background(&mut self) -> Result<()> {
        if let Some(database) = &self.database {
            let contacts_manager = Arc::new(ContactsManager::new(database.clone()));
            self.contacts_manager = Some(contacts_manager.clone());
            self.ui.set_contacts_manager(contacts_manager);
        }
        Ok(())
    }
    
    /// Check if a component is ready (for UI status display)
    pub fn is_component_ready(&self, component: &str) -> bool {
        match component {
            "database" => self.database.is_some(),
            "accounts" => self.imap_manager.is_some(),
            "notifications" => self.notification_manager.is_some(),
            "contacts" => self.contacts_manager.is_some(),
            _ => false,
        }
    }
    
    /// Get startup progress for UI display
    pub fn get_startup_progress(&self) -> (f32, String) {
        // Calculate progress based on completed components
        let mut completed = 0;
        let mut total = 4; // database, accounts, notifications, contacts
        
        if self.database.is_some() { completed += 1; }
        if self.imap_manager.is_some() { completed += 1; }
        if self.notification_manager.is_some() { completed += 1; }
        if self.contacts_manager.is_some() { completed += 1; }
        
        let progress = (completed as f32 / total as f32) * 100.0;
        let status = if completed == total {
            "All systems ready".to_string()
        } else {
            format!("Loading background services... ({}/{})", completed, total)
        };
        
        (progress, status)
    }
}
*/