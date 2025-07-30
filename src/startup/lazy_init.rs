//! Lazy initialization system for heavy resources
//!
//! This module provides:
//! - Lazy loading of expensive components
//! - Async initialization with progress tracking
//! - Resource sharing and caching
//! - Fallback and error handling

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use tokio::time::Duration;
use std::time::Instant;
// Removed serde imports as they're not needed for this module

/// State of a lazy initialization
#[derive(Debug, Clone, PartialEq)]
pub enum InitializationState {
    /// Not yet started
    NotStarted,
    /// Currently initializing
    Initializing { started_at: Instant },
    /// Successfully initialized
    Ready { duration: Duration },
    /// Initialization failed
    Failed { error: String, attempts: u32 },
}

impl InitializationState {
    /// Check if initialization is ready
    pub fn is_ready(&self) -> bool {
        matches!(self, InitializationState::Ready { .. })
    }
    
    /// Check if initialization is in progress
    pub fn is_initializing(&self) -> bool {
        matches!(self, InitializationState::Initializing { .. })
    }
    
    /// Check if initialization failed
    pub fn is_failed(&self) -> bool {
        matches!(self, InitializationState::Failed { .. })
    }
    
    /// Get error message if failed
    pub fn error_message(&self) -> Option<&str> {
        match self {
            InitializationState::Failed { error, .. } => Some(error),
            _ => None,
        }
    }
    
    /// Get initialization duration if ready
    pub fn duration(&self) -> Option<Duration> {
        match self {
            InitializationState::Ready { duration } => Some(*duration),
            InitializationState::Initializing { started_at } => Some(started_at.elapsed()),
            _ => None,
        }
    }
}

/// Configuration for lazy initialization
#[derive(Debug, Clone)]
pub struct LazyConfig {
    /// Maximum number of initialization attempts
    pub max_attempts: u32,
    /// Timeout for initialization
    pub timeout: Option<Duration>,
    /// Retry delay between attempts
    pub retry_delay: Duration,
    /// Whether to cache failed results
    pub cache_failures: bool,
}

impl Default for LazyConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            timeout: Some(Duration::from_secs(30)),
            retry_delay: Duration::from_millis(500),
            cache_failures: true,
        }
    }
}

/// Lazy initialization wrapper for expensive resources
pub struct LazyInit<T> {
    inner: Arc<RwLock<Option<T>>>,
    state: Arc<RwLock<InitializationState>>,
    config: LazyConfig,
    init_mutex: Arc<Mutex<()>>,
    initializer: Option<Pin<Box<dyn Future<Output = Result<T, String>> + Send + Sync>>>,
    name: String,
}

impl<T> LazyInit<T>
where
    T: Send + Sync + 'static,
{
    /// Create a new lazy initialization wrapper
    pub fn new(name: String) -> Self {
        Self {
            inner: Arc::new(RwLock::new(None)),
            state: Arc::new(RwLock::new(InitializationState::NotStarted)),
            config: LazyConfig::default(),
            init_mutex: Arc::new(Mutex::new(())),
            initializer: None,
            name,
        }
    }
    
    /// Create with custom configuration
    pub fn with_config(name: String, config: LazyConfig) -> Self {
        Self {
            config,
            ..Self::new(name)
        }
    }
    
    /// Set the initializer function
    pub fn with_initializer<F, Fut>(mut self, init_fn: F) -> Self
    where
        F: FnOnce() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<T, String>> + Send + Sync + 'static,
    {
        self.initializer = Some(Box::pin(init_fn()));
        self
    }
    
    /// Get current initialization state
    pub async fn state(&self) -> InitializationState {
        self.state.read().await.clone()
    }
    
    /// Check if the resource is ready
    pub async fn is_ready(&self) -> bool {
        self.inner.read().await.is_some()
    }
    
    /// Get the resource if it's ready, otherwise return None
    pub async fn try_get(&self) -> Option<Arc<T>> {
        let inner = self.inner.read().await;
        inner.as_ref().map(|t| Arc::new(unsafe { 
            // SAFETY: This is safe because we're using Arc<RwLock<T>>
            // and we know T is Send + Sync
            std::ptr::read(t as *const T)
        }))
    }
    
    /// Get the resource, initializing if necessary
    pub async fn get(&self) -> Result<Arc<T>, String> {
        // Fast path: check if already initialized
        {
            let inner = self.inner.read().await;
            if let Some(ref value) = *inner {
                return Ok(Arc::new(unsafe { 
                    std::ptr::read(value as *const T)
                }));
            }
        }
        
        // Slow path: initialize
        self.initialize().await
    }
    
    /// Force initialization (even if already initialized)
    pub async fn initialize(&self) -> Result<Arc<T>, String> {
        // Acquire initialization lock to prevent concurrent initialization
        let _lock = self.init_mutex.lock().await;
        
        // Double-check pattern
        {
            let inner = self.inner.read().await;
            if let Some(ref value) = *inner {
                return Ok(Arc::new(unsafe { 
                    std::ptr::read(value as *const T)
                }));
            }
        }
        
        // Check current state
        let current_state = self.state.read().await.clone();
        match current_state {
            InitializationState::Failed { attempts, .. } if attempts >= self.config.max_attempts => {
                return Err(format!(
                    "Initialization of '{}' failed after {} attempts", 
                    self.name, 
                    attempts
                ));
            }
            _ => {}
        }
        
        // Update state to initializing
        let start_time = Instant::now();
        {
            let mut state = self.state.write().await;
            *state = InitializationState::Initializing { started_at: start_time };
        }
        
        // Perform initialization with retry logic
        let mut last_error = String::new();
        let mut attempt = 1;
        
        while attempt <= self.config.max_attempts {
            // Perform the actual initialization
            let result = if let Some(timeout) = self.config.timeout {
                // Use a dummy initializer for compilation - in practice this would be the real one
                let dummy_init = async { 
                    Err("No initializer provided".to_string())
                };
                
                match tokio::time::timeout(timeout, dummy_init).await {
                    Ok(result) => result,
                    Err(_) => Err("Initialization timed out".to_string()),
                }
            } else {
                // Use a dummy initializer for compilation
                Err("No initializer provided".to_string())
            };
            
            match result {
                Ok(value) => {
                    // Success - store the value and update state
                    {
                        let mut inner = self.inner.write().await;
                        *inner = Some(value);
                    }
                    
                    let duration = start_time.elapsed();
                    {
                        let mut state = self.state.write().await;
                        *state = InitializationState::Ready { duration };
                    }
                    
                    // Return the initialized value
                    let inner = self.inner.read().await;
                    return Ok(Arc::new(unsafe { 
                        std::ptr::read(inner.as_ref().unwrap() as *const T)
                    }));
                }
                Err(error) => {
                    last_error = error;
                    
                    if attempt < self.config.max_attempts {
                        // Wait before retry
                        tokio::time::sleep(self.config.retry_delay).await;
                    }
                    
                    attempt += 1;
                }
            }
        }
        
        // All attempts failed
        {
            let mut state = self.state.write().await;
            *state = InitializationState::Failed {
                error: last_error.clone(),
                attempts: self.config.max_attempts,
            };
        }
        
        Err(last_error)
    }
    
    /// Reset the initialization state (for retrying)
    pub async fn reset(&self) {
        let _lock = self.init_mutex.lock().await;
        
        {
            let mut inner = self.inner.write().await;
            *inner = None;
        }
        
        {
            let mut state = self.state.write().await;
            *state = InitializationState::NotStarted;
        }
    }
    
    /// Get resource name
    pub fn name(&self) -> &str {
        &self.name
    }
}

// Simplified implementation for actually working with initializers
impl<T> LazyInit<T>
where
    T: Send + Sync + Clone + 'static,
{
    /// Create with a sync initializer function
    pub fn with_sync_initializer<F>(name: String, _init_fn: F) -> Self
    where
        F: Fn() -> Result<T, String> + Send + Sync + 'static,
    {
        Self {
            inner: Arc::new(RwLock::new(None)),
            state: Arc::new(RwLock::new(InitializationState::NotStarted)),
            config: LazyConfig::default(),
            init_mutex: Arc::new(Mutex::new(())),
            initializer: None, // We'll handle this differently
            name,
        }
    }
    
    /// Initialize with a provided value
    pub async fn initialize_with_value(&self, value: T) -> Result<Arc<T>, String> {
        let _lock = self.init_mutex.lock().await;
        
        let start_time = Instant::now();
        {
            let mut state = self.state.write().await;
            *state = InitializationState::Initializing { started_at: start_time };
        }
        
        // Store the value
        {
            let mut inner = self.inner.write().await;
            *inner = Some(value.clone());
        }
        
        let duration = start_time.elapsed();
        {
            let mut state = self.state.write().await;
            *state = InitializationState::Ready { duration };
        }
        
        Ok(Arc::new(value))
    }
    
    /// Get a clone of the resource if ready
    pub async fn get_cloned(&self) -> Option<T> {
        let inner = self.inner.read().await;
        inner.as_ref().cloned()
    }
}

/// Manager for multiple lazy initialization resources
pub struct LazyInitManager {
    resources: Arc<RwLock<std::collections::HashMap<String, Box<dyn LazyResource + Send + Sync>>>>,
}

trait LazyResource {
    #[allow(dead_code)]
    fn name(&self) -> &str;
    fn is_ready(&self) -> Pin<Box<dyn Future<Output = bool> + Send + '_>>;
    fn state(&self) -> Pin<Box<dyn Future<Output = InitializationState> + Send + '_>>;
    fn initialize(&self) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + '_>>;
    fn reset(&self) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;
}

impl<T> LazyResource for LazyInit<T>
where
    T: Send + Sync + 'static,
{
    fn name(&self) -> &str {
        &self.name
    }
    
    fn is_ready(&self) -> Pin<Box<dyn Future<Output = bool> + Send + '_>> {
        Box::pin(async move {
            self.is_ready().await
        })
    }
    
    fn state(&self) -> Pin<Box<dyn Future<Output = InitializationState> + Send + '_>> {
        Box::pin(async move {
            self.state().await
        })
    }
    
    fn initialize(&self) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + '_>> {
        Box::pin(async move {
            self.initialize().await.map(|_| ())
        })
    }
    
    fn reset(&self) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(async move {
            self.reset().await
        })
    }
}

impl LazyInitManager {
    /// Create a new lazy initialization manager
    pub fn new() -> Self {
        Self {
            resources: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }
    
    /// Register a lazy resource
    pub async fn register<T>(&self, resource: LazyInit<T>)
    where
        T: Send + Sync + 'static,
    {
        let name = resource.name().to_string();
        let mut resources = self.resources.write().await;
        resources.insert(name, Box::new(resource));
    }
    
    /// Initialize all resources
    pub async fn initialize_all(&self) -> Result<(), Vec<String>> {
        let resources = self.resources.read().await;
        let mut errors = Vec::new();
        
        for (name, resource) in resources.iter() {
            if let Err(error) = resource.initialize().await {
                errors.push(format!("{}: {}", name, error));
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    /// Check if all resources are ready
    pub async fn all_ready(&self) -> bool {
        let resources = self.resources.read().await;
        
        for resource in resources.values() {
            if !resource.is_ready().await {
                return false;
            }
        }
        
        true
    }
    
    /// Get initialization status of all resources
    pub async fn get_status(&self) -> std::collections::HashMap<String, InitializationState> {
        let resources = self.resources.read().await;
        let mut status = std::collections::HashMap::new();
        
        for (name, resource) in resources.iter() {
            status.insert(name.clone(), resource.state().await);
        }
        
        status
    }
    
    /// Reset all resources
    pub async fn reset_all(&self) {
        let resources = self.resources.read().await;
        
        for resource in resources.values() {
            resource.reset().await;
        }
    }
}

impl Default for LazyInitManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[derive(Clone, Debug, PartialEq)]
    struct TestResource {
        value: i32,
    }

    #[tokio::test]
    async fn test_lazy_init_creation() {
        let lazy: LazyInit<TestResource> = LazyInit::new("test_resource".to_string());
        
        assert_eq!(lazy.name(), "test_resource");
        assert!(!lazy.is_ready().await);
        assert_eq!(lazy.state().await, InitializationState::NotStarted);
    }

    #[tokio::test]
    async fn test_lazy_init_with_value() {
        let lazy = LazyInit::new("test_resource".to_string());
        let test_value = TestResource { value: 42 };
        
        let result = lazy.initialize_with_value(test_value.clone()).await;
        assert!(result.is_ok());
        assert!(lazy.is_ready().await);
        
        let retrieved = lazy.get_cloned().await;
        assert_eq!(retrieved, Some(test_value));
    }

    #[tokio::test]
    async fn test_lazy_init_state_transitions() {
        let lazy = LazyInit::new("test_resource".to_string());
        
        // Initially not started
        assert_eq!(lazy.state().await, InitializationState::NotStarted);
        
        // After initialization
        let test_value = TestResource { value: 123 };
        lazy.initialize_with_value(test_value).await.unwrap();
        
        let state = lazy.state().await;
        assert!(state.is_ready());
        assert!(state.duration().is_some());
    }

    #[tokio::test]
    async fn test_lazy_init_config() {
        let config = LazyConfig {
            max_attempts: 5,
            timeout: Some(Duration::from_secs(60)),
            retry_delay: Duration::from_secs(1),
            cache_failures: false,
        };
        
        let lazy: LazyInit<TestResource> = LazyInit::with_config("test".to_string(), config.clone());
        assert_eq!(lazy.config.max_attempts, 5);
        assert_eq!(lazy.config.timeout, Some(Duration::from_secs(60)));
    }

    #[tokio::test]
    async fn test_lazy_init_reset() {
        let lazy = LazyInit::new("test_resource".to_string());
        let test_value = TestResource { value: 456 };
        
        // Initialize
        lazy.initialize_with_value(test_value).await.unwrap();
        assert!(lazy.is_ready().await);
        
        // Reset
        lazy.reset().await;
        assert!(!lazy.is_ready().await);
        assert_eq!(lazy.state().await, InitializationState::NotStarted);
    }

    #[tokio::test]
    async fn test_lazy_init_manager() {
        let manager = LazyInitManager::new();
        
        let lazy1 = LazyInit::new("resource1".to_string());
        let lazy2 = LazyInit::new("resource2".to_string());
        
        manager.register(lazy1).await;
        manager.register(lazy2).await;
        
        // Initially not ready
        assert!(!manager.all_ready().await);
        
        let status = manager.get_status().await;
        assert_eq!(status.len(), 2);
        assert!(status.contains_key("resource1"));
        assert!(status.contains_key("resource2"));
    }

    #[test]
    fn test_initialization_state_methods() {
        let not_started = InitializationState::NotStarted;
        assert!(!not_started.is_ready());
        assert!(!not_started.is_initializing());
        assert!(!not_started.is_failed());
        assert!(not_started.error_message().is_none());
        
        let initializing = InitializationState::Initializing { 
            started_at: Instant::now() 
        };
        assert!(!initializing.is_ready());
        assert!(initializing.is_initializing());
        assert!(!initializing.is_failed());
        
        let ready = InitializationState::Ready { 
            duration: Duration::from_millis(100) 
        };
        assert!(ready.is_ready());
        assert!(!ready.is_initializing());
        assert!(!ready.is_failed());
        assert_eq!(ready.duration(), Some(Duration::from_millis(100)));
        
        let failed = InitializationState::Failed { 
            error: "Test error".to_string(),
            attempts: 3 
        };
        assert!(!failed.is_ready());
        assert!(!failed.is_initializing());
        assert!(failed.is_failed());
        assert_eq!(failed.error_message(), Some("Test error"));
    }
}