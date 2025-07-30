pub mod manager;
pub mod progress;
pub mod screen;
pub mod background_tasks;
pub mod lazy_init;
pub mod optimization;

pub use manager::StartupProgressManager;
pub use progress::{PhaseStatus, StartupPhase};
pub use screen::StartupProgressScreen;
pub use background_tasks::{BackgroundTaskManager, TaskContext, TaskPriority, TaskHandle};
pub use lazy_init::{LazyInit, LazyInitManager, InitializationState};
pub use optimization::{StartupOptimizer, StartupCache, StartupConfig};