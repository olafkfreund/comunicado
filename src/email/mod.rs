pub mod message;
pub mod thread;
pub mod threading_engine;
pub mod sorting;
pub mod database;
pub mod sync_engine;

pub use message::{EmailMessage, MessageId};
pub use thread::{EmailThread, ThreadStatistics};
pub use threading_engine::{ThreadingEngine, ThreadingAlgorithm};
pub use sorting::{SortCriteria, SortOrder, MultiCriteriaSorter};
pub use database::{EmailDatabase, StoredMessage, StoredAttachment, FolderSyncState, SyncStatus, DatabaseStats, DatabaseError, DatabaseResult};
pub use sync_engine::{SyncEngine, SyncStrategy, SyncProgress, SyncPhase, ConflictResolution, SyncError, SyncResult};