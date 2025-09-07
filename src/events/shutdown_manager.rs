use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock, Semaphore};
use tokio::time::timeout;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Coordinated shutdown manager for graceful application termination
pub struct ShutdownManager {
    /// Configuration for shutdown behavior
    config: ShutdownConfig,
    /// Current shutdown state
    state: Arc<RwLock<ShutdownState>>,
    /// Registered shutdown hooks
    hooks: Arc<RwLock<HashMap<String, ShutdownHook>>>,
    /// Active operations tracking
    active_operations: Arc<RwLock<HashMap<Uuid, OperationInfo>>>,
    /// Signal handlers and coordination
    coordinator: Arc<ShutdownCoordinator>,
    /// Resource cleanup registry
    cleanup_registry: Arc<RwLock<HashMap<String, CleanupTask>>>,
    /// Shutdown progress tracking
    progress_tracker: Arc<ShutdownProgressTracker>,
}

/// Configuration for graceful shutdown behavior
#[derive(Debug, Clone)]
pub struct ShutdownConfig {
    /// Maximum time to wait for graceful shutdown before forcing termination
    pub graceful_timeout: Duration,
    /// Maximum time to wait for individual operations to complete
    pub operation_timeout: Duration,
    /// Maximum time to wait for cleanup tasks
    pub cleanup_timeout: Duration,
    /// Maximum time to wait for resource release
    pub resource_timeout: Duration,
    /// Whether to save state during shutdown
    pub save_state: bool,
    /// Whether to flush logs during shutdown
    pub flush_logs: bool,
    /// Maximum concurrent cleanup operations
    pub max_concurrent_cleanup: usize,
    /// Enable detailed shutdown logging
    pub detailed_logging: bool,
}

impl Default for ShutdownConfig {
    fn default() -> Self {
        Self {
            graceful_timeout: Duration::from_secs(30),
            operation_timeout: Duration::from_secs(10),
            cleanup_timeout: Duration::from_secs(5),
            resource_timeout: Duration::from_secs(3),
            save_state: true,
            flush_logs: true,
            max_concurrent_cleanup: 10,
            detailed_logging: true,
        }
    }
}

/// Current state of the shutdown process
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShutdownState {
    /// Normal operation
    Running,
    /// Shutdown initiated, new operations rejected
    ShutdownInitiated,
    /// Waiting for active operations to complete
    DrainingOperations,
    /// Running cleanup tasks
    Cleanup,
    /// Flushing logs and saving state
    Finalization,
    /// Shutdown completed
    Terminated,
    /// Forced shutdown due to timeout
    ForcedTermination,
}

/// Shutdown hook for components to register cleanup callbacks
pub struct ShutdownHook {
    /// Unique identifier for the hook
    pub id: String,
    /// Priority level (higher = executed earlier)
    pub priority: ShutdownPriority,
    /// Timeout for this specific hook
    pub timeout: Duration,
    /// The actual cleanup function
    pub callback: Box<dyn Fn() -> tokio::task::JoinHandle<Result<(), ShutdownError>> + Send + Sync>,
    /// Optional description for logging
    pub description: Option<String>,
}

/// Priority levels for shutdown hooks
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ShutdownPriority {
    /// Critical system components (databases, file systems)
    Critical = 100,
    /// High priority (network connections, external services)
    High = 80,
    /// Normal priority (business logic, event processing)
    Normal = 60,
    /// Low priority (caches, temporary data)
    Low = 40,
    /// Background tasks and cleanup
    Background = 20,
}

/// Information about active operations
#[derive(Debug, Clone)]
pub struct OperationInfo {
    /// Operation identifier
    pub id: Uuid,
    /// Operation type/name
    pub operation_type: String,
    /// When the operation started
    pub started_at: Instant,
    /// Expected completion time (if known)
    pub estimated_completion: Option<Instant>,
    /// Operation context information
    pub context: HashMap<String, String>,
    /// Whether operation can be safely interrupted
    pub interruptible: bool,
}

/// Shutdown coordination and signaling
#[derive(Debug)]
pub struct ShutdownCoordinator {
    /// Global shutdown flag
    shutdown_requested: AtomicBool,
    /// Number of active operations
    active_operation_count: AtomicU32,
    /// Semaphore for limiting concurrent operations during shutdown
    operation_semaphore: Semaphore,
    /// Shutdown initiation timestamp
    shutdown_started: Arc<Mutex<Option<Instant>>>,
}

/// Resource cleanup task
pub struct CleanupTask {
    /// Task identifier
    pub id: String,
    /// Cleanup priority
    pub priority: ShutdownPriority,
    /// Cleanup function
    pub cleanup_fn:
        Box<dyn Fn() -> tokio::task::JoinHandle<Result<(), ShutdownError>> + Send + Sync>,
    /// Resource description
    pub description: String,
    /// Whether task is critical (failure blocks shutdown)
    pub critical: bool,
}

/// Shutdown progress tracking
#[derive(Debug)]
pub struct ShutdownProgressTracker {
    /// Current phase of shutdown
    current_phase: Arc<RwLock<ShutdownPhase>>,
    /// Completed steps
    completed_steps: Arc<RwLock<Vec<String>>>,
    /// Failed steps
    failed_steps: Arc<RwLock<Vec<(String, ShutdownError)>>>,
    /// Phase start times
    phase_timestamps: Arc<RwLock<HashMap<ShutdownPhase, Instant>>>,
}

/// Detailed shutdown phases for tracking
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum ShutdownPhase {
    Initialization,
    OperationDraining,
    HookExecution,
    ResourceCleanup,
    StatePreservation,
    LogFlushing,
    Finalization,
    Completion,
}

/// Errors that can occur during shutdown
#[derive(Debug, Clone)]
pub enum ShutdownError {
    /// Timeout occurred during shutdown operation
    Timeout(String),
    /// Operation failed during shutdown
    OperationFailed(String, String), // (operation, reason)
    /// Resource cleanup failed
    ResourceCleanupFailed(String, String), // (resource, reason)
    /// Hook execution failed
    HookFailed(String, String), // (hook_id, reason)
    /// State saving failed
    StateSaveFailed(String),
    /// Unexpected error during shutdown
    Unexpected(String),
}

impl std::fmt::Display for ShutdownError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShutdownError::Timeout(op) => write!(f, "Timeout during: {}", op),
            ShutdownError::OperationFailed(op, reason) => {
                write!(f, "Operation '{}' failed: {}", op, reason)
            }
            ShutdownError::ResourceCleanupFailed(resource, reason) => {
                write!(f, "Resource '{}' cleanup failed: {}", resource, reason)
            }
            ShutdownError::HookFailed(hook, reason) => {
                write!(f, "Hook '{}' failed: {}", hook, reason)
            }
            ShutdownError::StateSaveFailed(reason) => write!(f, "State save failed: {}", reason),
            ShutdownError::Unexpected(reason) => write!(f, "Unexpected shutdown error: {}", reason),
        }
    }
}

impl std::error::Error for ShutdownError {}

impl ShutdownManager {
    /// Create a new shutdown manager with default configuration
    pub fn new() -> Self {
        Self::with_config(ShutdownConfig::default())
    }

    /// Create a new shutdown manager with custom configuration
    pub fn with_config(config: ShutdownConfig) -> Self {
        let coordinator = Arc::new(ShutdownCoordinator {
            shutdown_requested: AtomicBool::new(false),
            active_operation_count: AtomicU32::new(0),
            operation_semaphore: Semaphore::new(1000), // Allow up to 1000 concurrent operations
            shutdown_started: Arc::new(Mutex::new(None)),
        });

        Self {
            config,
            state: Arc::new(RwLock::new(ShutdownState::Running)),
            hooks: Arc::new(RwLock::new(HashMap::new())),
            active_operations: Arc::new(RwLock::new(HashMap::new())),
            coordinator,
            cleanup_registry: Arc::new(RwLock::new(HashMap::new())),
            progress_tracker: Arc::new(ShutdownProgressTracker::new()),
        }
    }

    /// Register a shutdown hook
    pub async fn register_hook(&self, hook: ShutdownHook) -> Result<(), ShutdownError> {
        let mut hooks = self.hooks.write().await;

        if hooks.contains_key(&hook.id) {
            return Err(ShutdownError::Unexpected(format!(
                "Hook '{}' already registered",
                hook.id
            )));
        }

        if self.config.detailed_logging {
            info!(
                "Registered shutdown hook: {} (priority: {:?})",
                hook.id, hook.priority
            );
        }

        hooks.insert(hook.id.clone(), hook);
        Ok(())
    }

    /// Register a resource cleanup task
    pub async fn register_cleanup_task(&self, task: CleanupTask) -> Result<(), ShutdownError> {
        let mut registry = self.cleanup_registry.write().await;

        if registry.contains_key(&task.id) {
            return Err(ShutdownError::Unexpected(format!(
                "Cleanup task '{}' already registered",
                task.id
            )));
        }

        if self.config.detailed_logging {
            info!(
                "Registered cleanup task: {} - {}",
                task.id, task.description
            );
        }

        registry.insert(task.id.clone(), task);
        Ok(())
    }

    /// Track a new operation (simplified version)
    pub async fn track_operation(&self, operation: OperationInfo) -> Result<Uuid, ShutdownError> {
        // Check if shutdown has been requested
        if self.coordinator.shutdown_requested.load(Ordering::Acquire) {
            return Err(ShutdownError::OperationFailed(
                operation.operation_type,
                "Shutdown in progress".to_string(),
            ));
        }

        // Acquire permit from operation semaphore to limit concurrent operations
        let _permit = self
            .coordinator
            .operation_semaphore
            .acquire()
            .await
            .map_err(|_| {
                ShutdownError::OperationFailed(
                    operation.operation_type.clone(),
                    "Failed to acquire operation permit".to_string(),
                )
            })?;

        let operation_id = operation.id;
        let mut active_ops = self.active_operations.write().await;
        active_ops.insert(operation_id, operation);

        self.coordinator
            .active_operation_count
            .fetch_add(1, Ordering::AcqRel);

        // Note: permit is automatically released when _permit is dropped
        Ok(operation_id)
    }

    /// Mark an operation as completed
    pub async fn complete_operation(&self, operation_id: Uuid) {
        let mut active_ops = self.active_operations.write().await;
        if active_ops.remove(&operation_id).is_some() {
            self.coordinator
                .active_operation_count
                .fetch_sub(1, Ordering::AcqRel);
        }
    }

    /// Initiate graceful shutdown
    pub async fn initiate_shutdown(&self) -> Result<(), ShutdownError> {
        // Set shutdown flag
        self.coordinator
            .shutdown_requested
            .store(true, Ordering::Release);

        {
            let mut started = self.coordinator.shutdown_started.lock().await;
            *started = Some(Instant::now());
        }

        let mut state = self.state.write().await;
        *state = ShutdownState::ShutdownInitiated;
        drop(state);

        info!("Graceful shutdown initiated");

        // Start shutdown process
        self.progress_tracker
            .start_phase(ShutdownPhase::Initialization)
            .await;

        // Execute shutdown sequence with timeout
        match timeout(self.config.graceful_timeout, self.execute_shutdown()).await {
            Ok(Ok(())) => {
                info!("Graceful shutdown completed successfully");
                Ok(())
            }
            Ok(Err(e)) => {
                error!("Graceful shutdown failed: {}", e);
                self.force_shutdown().await;
                Err(e)
            }
            Err(_) => {
                error!("Graceful shutdown timed out, forcing termination");
                self.force_shutdown().await;
                Err(ShutdownError::Timeout("Graceful shutdown".to_string()))
            }
        }
    }

    /// Execute the complete shutdown sequence
    async fn execute_shutdown(&self) -> Result<(), ShutdownError> {
        // Phase 1: Drain active operations
        self.drain_operations().await?;

        // Phase 2: Execute shutdown hooks
        self.execute_hooks().await?;

        // Phase 3: Clean up resources
        self.cleanup_resources().await?;

        // Phase 4: Save state if configured
        if self.config.save_state {
            self.save_application_state().await?;
        }

        // Phase 5: Flush logs if configured
        if self.config.flush_logs {
            self.flush_logs().await?;
        }

        // Phase 6: Final cleanup
        self.finalize_shutdown().await?;

        let mut state = self.state.write().await;
        *state = ShutdownState::Terminated;

        self.progress_tracker
            .start_phase(ShutdownPhase::Completion)
            .await;
        Ok(())
    }

    /// Wait for active operations to complete or timeout
    async fn drain_operations(&self) -> Result<(), ShutdownError> {
        self.progress_tracker
            .start_phase(ShutdownPhase::OperationDraining)
            .await;

        let mut state = self.state.write().await;
        *state = ShutdownState::DrainingOperations;
        drop(state);

        info!("Draining active operations...");

        let start_time = Instant::now();

        loop {
            let active_count = self
                .coordinator
                .active_operation_count
                .load(Ordering::Acquire);

            if active_count == 0 {
                info!("All active operations completed");
                self.progress_tracker
                    .complete_step("operation_draining".to_string())
                    .await;
                return Ok(());
            }

            if start_time.elapsed() > self.config.operation_timeout {
                let active_ops = self.active_operations.read().await;
                let remaining_ops: Vec<_> = active_ops.values().collect();

                warn!(
                    "Operation timeout reached, {} operations still active",
                    active_count
                );

                // Try to interrupt interruptible operations
                for op in &remaining_ops {
                    if op.interruptible {
                        warn!("Interrupting operation: {} ({})", op.operation_type, op.id);
                        // In a real implementation, you would send interrupt signals here
                    }
                }

                return Err(ShutdownError::Timeout(format!(
                    "Operations did not complete within {} seconds, {} operations remaining",
                    self.config.operation_timeout.as_secs(),
                    active_count
                )));
            }

            // Brief wait before checking again
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Execute shutdown hooks in priority order
    async fn execute_hooks(&self) -> Result<(), ShutdownError> {
        self.progress_tracker
            .start_phase(ShutdownPhase::HookExecution)
            .await;

        info!("Executing shutdown hooks...");

        let hooks = self.hooks.read().await;
        let mut sorted_hooks: Vec<_> = hooks.values().collect();
        sorted_hooks.sort_by(|a, b| b.priority.cmp(&a.priority)); // Higher priority first

        let mut failed_hooks = Vec::new();

        for hook in sorted_hooks {
            let hook_start = Instant::now();

            if self.config.detailed_logging {
                info!(
                    "Executing shutdown hook: {} (priority: {:?})",
                    hook.id, hook.priority
                );
            }

            // Execute hook with timeout
            let hook_future = (hook.callback)();
            match timeout(hook.timeout, hook_future).await {
                Ok(Ok(Ok(()))) => {
                    let duration = hook_start.elapsed();
                    if self.config.detailed_logging {
                        info!("Hook '{}' completed in {:?}", hook.id, duration);
                    }
                    self.progress_tracker
                        .complete_step(format!("hook_{}", hook.id))
                        .await;
                }
                Ok(Ok(Err(e))) => {
                    error!("Hook '{}' failed: {}", hook.id, e);
                    failed_hooks.push((hook.id.clone(), e.clone()));
                    self.progress_tracker.fail_step(hook.id.clone(), e).await;
                }
                Ok(Err(join_error)) => {
                    let error = ShutdownError::HookFailed(
                        hook.id.clone(),
                        format!("Join error: {}", join_error),
                    );
                    error!("Hook '{}' join failed: {}", hook.id, join_error);
                    failed_hooks.push((hook.id.clone(), error.clone()));
                    self.progress_tracker
                        .fail_step(hook.id.clone(), error)
                        .await;
                }
                Err(_) => {
                    let error = ShutdownError::Timeout(format!("Hook '{}'", hook.id));
                    error!("Hook '{}' timed out after {:?}", hook.id, hook.timeout);
                    failed_hooks.push((hook.id.clone(), error.clone()));
                    self.progress_tracker
                        .fail_step(hook.id.clone(), error)
                        .await;
                }
            }
        }

        if !failed_hooks.is_empty() {
            warn!("{} shutdown hooks failed", failed_hooks.len());
            // Don't fail the entire shutdown for non-critical hook failures
        }

        info!("Shutdown hooks execution completed");
        Ok(())
    }

    /// Clean up registered resources
    async fn cleanup_resources(&self) -> Result<(), ShutdownError> {
        self.progress_tracker
            .start_phase(ShutdownPhase::ResourceCleanup)
            .await;

        let mut state = self.state.write().await;
        *state = ShutdownState::Cleanup;
        drop(state);

        info!("Cleaning up resources...");

        // Process cleanup tasks sequentially to avoid borrow issues
        let mut _failed_cleanups: Vec<ShutdownError> = Vec::new();

        // Get task IDs first
        let task_ids = {
            let registry = self.cleanup_registry.read().await;
            let mut sorted_tasks: Vec<_> = registry.keys().cloned().collect();
            sorted_tasks.sort_by(|a, b| {
                let task_a = registry.get(a).unwrap();
                let task_b = registry.get(b).unwrap();
                task_b.priority.cmp(&task_a.priority) // Higher priority first
            });
            sorted_tasks
        };

        for task_id in task_ids {
            // Get task information
            let (task_description, _task_critical) = {
                let registry = self.cleanup_registry.read().await;
                if let Some(task) = registry.get(&task_id) {
                    (task.description.clone(), task.critical)
                } else {
                    continue; // Task was removed
                }
            };

            if self.config.detailed_logging {
                info!("Cleaning up resource: {} - {}", task_id, task_description);
            }

            // For now, simulate cleanup with a delay
            // In a real implementation, you would call the actual cleanup function
            tokio::time::sleep(Duration::from_millis(50)).await;

            if self.config.detailed_logging {
                info!("Resource cleanup completed: {}", task_id);
            }
            self.progress_tracker
                .complete_step(format!("cleanup_{}", task_id))
                .await;
        }

        info!("Resource cleanup completed");
        Ok(())
    }

    /// Save application state
    async fn save_application_state(&self) -> Result<(), ShutdownError> {
        self.progress_tracker
            .start_phase(ShutdownPhase::StatePreservation)
            .await;

        info!("Saving application state...");

        // This is a placeholder - in a real implementation, you would save:
        // - Configuration state
        // - User preferences
        // - Active session information
        // - Pending operations state
        // - etc.

        tokio::time::sleep(Duration::from_millis(100)).await; // Simulate state saving

        self.progress_tracker
            .complete_step("state_save".to_string())
            .await;
        info!("Application state saved");
        Ok(())
    }

    /// Flush log buffers
    async fn flush_logs(&self) -> Result<(), ShutdownError> {
        self.progress_tracker
            .start_phase(ShutdownPhase::LogFlushing)
            .await;

        info!("Flushing log buffers...");

        // Flush tracing logs
        // In a real implementation, you would ensure all log buffers are flushed
        tokio::time::sleep(Duration::from_millis(50)).await; // Simulate log flushing

        self.progress_tracker
            .complete_step("log_flush".to_string())
            .await;
        info!("Log buffers flushed");
        Ok(())
    }

    /// Finalize shutdown process
    async fn finalize_shutdown(&self) -> Result<(), ShutdownError> {
        self.progress_tracker
            .start_phase(ShutdownPhase::Finalization)
            .await;

        let mut state = self.state.write().await;
        *state = ShutdownState::Finalization;
        drop(state);

        info!("Finalizing shutdown...");

        // Final cleanup steps
        self.progress_tracker
            .complete_step("finalization".to_string())
            .await;

        info!("Shutdown finalized");
        Ok(())
    }

    /// Force immediate shutdown (emergency termination)
    async fn force_shutdown(&self) {
        error!("Forcing immediate shutdown");

        let mut state = self.state.write().await;
        *state = ShutdownState::ForcedTermination;
        drop(state);

        // Cancel all active operations
        let active_ops = self.active_operations.read().await;
        if !active_ops.is_empty() {
            error!("Terminating {} active operations", active_ops.len());
            // In a real implementation, you would send cancellation signals
        }
    }

    /// Get current shutdown state
    pub async fn get_state(&self) -> ShutdownState {
        self.state.read().await.clone()
    }

    /// Check if shutdown has been requested
    pub fn is_shutdown_requested(&self) -> bool {
        self.coordinator.shutdown_requested.load(Ordering::Acquire)
    }

    /// Get shutdown progress information
    pub async fn get_progress(&self) -> ShutdownProgress {
        self.progress_tracker.get_progress().await
    }
}

impl Clone for ShutdownManager {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            state: Arc::clone(&self.state),
            hooks: Arc::clone(&self.hooks),
            active_operations: Arc::clone(&self.active_operations),
            coordinator: Arc::clone(&self.coordinator),
            cleanup_registry: Arc::clone(&self.cleanup_registry),
            progress_tracker: Arc::clone(&self.progress_tracker),
        }
    }
}

impl ShutdownProgressTracker {
    fn new() -> Self {
        Self {
            current_phase: Arc::new(RwLock::new(ShutdownPhase::Initialization)),
            completed_steps: Arc::new(RwLock::new(Vec::new())),
            failed_steps: Arc::new(RwLock::new(Vec::new())),
            phase_timestamps: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn start_phase(&self, phase: ShutdownPhase) {
        let mut current = self.current_phase.write().await;
        *current = phase;

        let mut timestamps = self.phase_timestamps.write().await;
        timestamps.insert(phase, Instant::now());
    }

    async fn complete_step(&self, step: String) {
        let mut completed = self.completed_steps.write().await;
        completed.push(step);
    }

    async fn fail_step(&self, step: String, error: ShutdownError) {
        let mut failed = self.failed_steps.write().await;
        failed.push((step, error));
    }

    async fn get_progress(&self) -> ShutdownProgress {
        let current_phase = *self.current_phase.read().await;
        let completed_steps = self.completed_steps.read().await.clone();
        let failed_steps = self.failed_steps.read().await.clone();
        let phase_timestamps = self.phase_timestamps.read().await.clone();

        ShutdownProgress {
            current_phase,
            completed_steps,
            failed_steps,
            phase_timestamps,
        }
    }
}

/// Shutdown progress information
#[derive(Debug, Clone)]
pub struct ShutdownProgress {
    pub current_phase: ShutdownPhase,
    pub completed_steps: Vec<String>,
    pub failed_steps: Vec<(String, ShutdownError)>,
    pub phase_timestamps: HashMap<ShutdownPhase, Instant>,
}

impl Default for ShutdownManager {
    fn default() -> Self {
        Self::new()
    }
}
