pub mod attachment_viewer;
pub mod attachments;
pub mod database;
pub mod desktop_notifications;
pub mod filters;
pub mod maildir;
pub mod maildir_ui;
pub mod message;
pub mod notifications;
pub mod sorting;
pub mod sync_engine;
pub mod thread;
pub mod threading_engine;

pub use attachment_viewer::{AttachmentViewer, ViewResult, ViewerMode};
pub use attachments::{AttachmentInfo, AttachmentManager, AttachmentType};
pub use database::{
    BackupResult, CleanupResult, DatabaseError, DatabaseResult, DatabaseStats, EmailDatabase,
    FolderSyncState, RestoreResult, StoredAttachment, StoredMessage, SyncStatus,
};
pub use desktop_notifications::DesktopNotificationService;
pub use filters::{
    EmailFilter, FilterAction, FilterCondition, FilterEngine, FilterField, FilterOperator,
    FilterResult, FilterTemplates,
};
pub use maildir::{MaildirError, MaildirFolderStats, MaildirHandler, MaildirResult, MaildirStats};
pub use maildir_ui::{MaildirExportPreview, MaildirImportFolder, MaildirImportPreview, MaildirUI};
pub use message::{EmailMessage, MessageId};
pub use notifications::{
    EmailNotification, EmailNotificationHandler, EmailNotificationManager, UIEmailUpdater,
};
pub use sorting::{MultiCriteriaSorter, SortCriteria, SortOrder};
pub use sync_engine::{
    ConflictResolution, SyncEngine, SyncError, SyncPhase, SyncProgress, SyncResult, SyncStrategy,
};
pub use thread::{EmailThread, ThreadStatistics};
pub use threading_engine::{ThreadingAlgorithm, ThreadingEngine};
