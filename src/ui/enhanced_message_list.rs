//! Enhanced message list with precaching and background sync
//!
//! This replaces the old message list with intelligent preloading and real-time updates

use crate::email::{EmailDatabase, precache_system::EmailPrecacheSystem};
use crate::performance::ProgressTracker;
use crate::ui::message_list::{MessageItem, MessageList};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Enhanced message list with precaching capabilities
pub struct EnhancedMessageList {
    /// Base message list
    base: MessageList,
    /// Precache system for background loading
    precache_system: Option<Arc<EmailPrecacheSystem>>,
    /// Progress tracker for loading status
    progress_tracker: Option<Arc<ProgressTracker>>,
    /// Last refresh time per folder
    folder_refresh_times: Arc<RwLock<std::collections::HashMap<String, Instant>>>,
    /// Cache of message counts for folders
    folder_message_counts: Arc<RwLock<std::collections::HashMap<String, usize>>>,
    /// Loading state
    is_loading: bool,
    /// Auto-refresh settings
    auto_refresh_enabled: bool,
    auto_refresh_interval: Duration,
}

impl EnhancedMessageList {
    /// Create new enhanced message list
    pub fn new() -> Self {
        Self {
            base: MessageList::new(),
            precache_system: None,
            progress_tracker: None,
            folder_refresh_times: Arc::new(RwLock::new(std::collections::HashMap::new())),
            folder_message_counts: Arc::new(RwLock::new(std::collections::HashMap::new())),
            is_loading: false,
            auto_refresh_enabled: true,
            auto_refresh_interval: Duration::from_secs(60),
        }
    }

    /// Set precache system
    pub fn set_precache_system(&mut self, precache_system: Arc<EmailPrecacheSystem>) {
        self.precache_system = Some(precache_system);
    }

    /// Set progress tracker
    pub fn set_progress_tracker(&mut self, progress_tracker: Arc<ProgressTracker>) {
        self.progress_tracker = Some(progress_tracker);
    }

    /// Set database (delegates to base)
    pub fn set_database(&mut self, database: Arc<EmailDatabase>) {
        self.base.set_database(database);
    }

    /// Load messages with intelligent precaching
    pub async fn load_messages_smart(
        &mut self,
        account_id: String,
        folder_name: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let folder_key = format!("{}:{}", account_id, folder_name);
        
        println!("📧 Smart loading messages for {}", folder_key);
        
        // Start progress tracking
        if let Some(progress_tracker) = &self.progress_tracker {
            let progress = progress_tracker
                .start_folder_sync(&account_id, &folder_name)
                .await;
            
            let mut progress_update = progress;
            progress_update.update_step("Checking cached messages...".to_string());
            progress_tracker.update_progress(progress_update).await;
        }

        self.is_loading = true;

        // Step 1: Load cached messages immediately for instant UI response
        println!("📦 Loading cached messages...");
        let cached_result = self.base.load_messages(account_id.clone(), folder_name.clone()).await;
        
        match cached_result {
            Ok(()) => {
                let message_count = self.base.messages().len();
                println!("✅ Loaded {} cached messages", message_count);
                
                // Update cache count
                {
                    let mut counts = self.folder_message_counts.write().await;
                    counts.insert(folder_key.clone(), message_count);
                }
                
                // If we have cached messages, show them immediately
                if message_count > 0 {
                    if let Some(progress_tracker) = &self.progress_tracker {
                        let mut progress = progress_tracker
                            .get_operation(uuid::Uuid::new_v4()).await // This would be stored
                            .unwrap_or_else(|| {
                                // Create a new progress if not found
                                use crate::performance::ProgressUpdate;
                                ProgressUpdate::new(uuid::Uuid::new_v4(), "Loading messages".to_string())
                            });
                        
                        progress.update_step("Cached messages loaded".to_string());
                        progress.update_progress(50, Some(100));
                        progress_tracker.update_progress(progress).await;
                    }
                }
            }
            Err(e) => {
                println!("⚠️  Failed to load cached messages: {}", e);
            }
        }

        // Step 2: Check if we need to fetch new messages
        let needs_refresh = self.should_refresh_folder(&folder_key).await;
        
        if needs_refresh {
            println!("🔄 Folder needs refresh, triggering background sync...");
            
            // Update progress
            if let Some(progress_tracker) = &self.progress_tracker {
                let mut progress = progress_tracker
                    .get_operation(uuid::Uuid::new_v4()).await
                    .unwrap_or_else(|| {
                        use crate::performance::ProgressUpdate;
                        ProgressUpdate::new(uuid::Uuid::new_v4(), "Loading messages".to_string())
                    });
                
                progress.update_step("Checking for new messages...".to_string());
                progress.update_progress(75, Some(100));
                progress_tracker.update_progress(progress).await;
            }

            // Step 3: Trigger background sync
            if let Some(precache_system) = &self.precache_system {
                match precache_system.preload_folder_on_demand(&account_id, &folder_name).await {
                    Ok(()) => {
                        println!("✅ Background sync triggered");
                        
                        // Wait a short time for quick syncs to complete
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        
                        // Reload messages to get any newly synced emails
                        if let Ok(()) = self.base.load_messages(account_id.clone(), folder_name.clone()).await {
                            let new_count = self.base.messages().len();
                            println!("📧 Reloaded {} messages after sync", new_count);
                            
                            // Update cache count
                            {
                                let mut counts = self.folder_message_counts.write().await;
                                counts.insert(folder_key.clone(), new_count);
                            }
                        }
                    }
                    Err(e) => {
                        println!("❌ Background sync failed: {}", e);
                    }
                }
            }
        }

        // Update refresh time
        {
            let mut refresh_times = self.folder_refresh_times.write().await;
            refresh_times.insert(folder_key, Instant::now());
        }

        // Complete progress tracking
        if let Some(progress_tracker) = &self.progress_tracker {
            // In a real implementation, you'd get the actual operation ID
            progress_tracker.complete_operation(uuid::Uuid::new_v4()).await;
        }

        self.is_loading = false;
        println!("✅ Smart message loading complete");
        
        Ok(())
    }

    /// Check if folder should be refreshed
    async fn should_refresh_folder(&self, folder_key: &str) -> bool {
        let refresh_times = self.folder_refresh_times.read().await;
        
        match refresh_times.get(folder_key) {
            Some(last_refresh) => {
                // Refresh if more than 2 minutes since last refresh
                last_refresh.elapsed() > Duration::from_secs(120)
            }
            None => {
                // Never refreshed, definitely refresh
                true
            }
        }
    }

    /// Force refresh of current folder
    pub async fn force_refresh(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let (Some(account), Some(folder)) = self.get_current_context() {
            let account = account.clone();
            let folder = folder.clone();
            
            println!("🔄 Force refreshing {}:{}", account, folder);
            
            // Clear refresh time to force sync
            {
                let mut refresh_times = self.folder_refresh_times.write().await;
                refresh_times.remove(&format!("{}:{}", account, folder));
            }
            
            // Reload with force refresh
            self.load_messages_smart(account, folder).await?;
        }
        
        Ok(())
    }

    /// Get loading status
    pub fn is_loading(&self) -> bool {
        self.is_loading
    }

    /// Get cached message count for a folder
    pub async fn get_cached_count(&self, account_id: &str, folder_name: &str) -> usize {
        let folder_key = format!("{}:{}", account_id, folder_name);
        let counts = self.folder_message_counts.read().await;
        counts.get(&folder_key).copied().unwrap_or(0)
    }

    /// Enable/disable auto-refresh
    pub fn set_auto_refresh(&mut self, enabled: bool, interval: Option<Duration>) {
        self.auto_refresh_enabled = enabled;
        if let Some(interval) = interval {
            self.auto_refresh_interval = interval;
        }
    }

    /// Start auto-refresh background task
    pub async fn start_auto_refresh(&self) {
        if !self.auto_refresh_enabled {
            return;
        }

        let refresh_times = self.folder_refresh_times.clone();
        let precache_system = self.precache_system.clone();
        let interval = self.auto_refresh_interval;

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            
            loop {
                ticker.tick().await;
                
                // Check all folders that might need refresh
                let folders_to_refresh = {
                    let times = refresh_times.read().await;
                    times.iter()
                        .filter(|(_, &last_refresh)| last_refresh.elapsed() > interval)
                        .map(|(folder_key, _)| folder_key.clone())
                        .collect::<Vec<_>>()
                };

                // Trigger background refresh for stale folders
                if let Some(precache_system) = &precache_system {
                    for folder_key in folders_to_refresh {
                        if let Some((account_id, folder_name)) = folder_key.split_once(':') {
                            if let Err(e) = precache_system
                                .preload_folder_on_demand(account_id, folder_name)
                                .await 
                            {
                                eprintln!("Auto-refresh failed for {}: {}", folder_key, e);
                            }
                        }
                    }
                }
            }
        });

        println!("⏰ Auto-refresh started (interval: {:?})", interval);
    }

    // Delegate all other methods to base message list
    pub fn render(&mut self, f: &mut ratatui::Frame<'_>, area: ratatui::layout::Rect, theme: &crate::theme::Theme) {
        use ratatui::widgets::{Block, Borders};
        let block = Block::default().borders(Borders::ALL);
        self.base.render(f, area, block, false, theme)
    }

    pub fn handle_key(&mut self, key: &crossterm::event::KeyEvent) -> bool {
        // MessageList doesn't have a generic handle_key method
        // Instead it has handle_up, handle_down, handle_enter methods
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Up => { self.base.handle_up(); true },
            KeyCode::Down => { self.base.handle_down(); true },
            KeyCode::Enter => { self.base.handle_enter(); true },
            _ => false,
        }
    }

    pub fn select_next(&mut self) {
        self.base.handle_down()
    }

    pub fn select_previous(&mut self) {
        self.base.handle_up()
    }

    pub fn get_selected_message(&self) -> Option<&MessageItem> {
        self.base.get_selected_stored_message()
    }

    pub fn messages(&self) -> &Vec<MessageItem> {
        self.base.messages()
    }

    pub fn get_current_context(&self) -> (Option<&String>, Option<&String>) {
        self.base.get_current_context()
    }

    pub fn get_selected_message_for_preview(&self) -> Option<MessageItem> {
        self.base.get_selected_message_for_preview()
    }

    pub fn has_database(&self) -> bool {
        self.base.has_database()
    }
}

impl Default for EnhancedMessageList {
    fn default() -> Self {
        Self::new()
    }
}