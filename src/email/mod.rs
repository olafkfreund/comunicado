pub mod advanced_filters;
pub mod advanced_filters_ui;
pub mod attachment_viewer;
pub mod attachments;
pub mod database;
pub mod database_optimizations;
pub mod desktop_notifications;
pub mod filters;
pub mod folder_hierarchy;
pub mod maildir;
pub mod maildir_exporter;
pub mod maildir_export_wizard;
pub mod maildir_importer;
pub mod maildir_import_wizard;
#[cfg(test)]
pub mod maildir_integration_tests;
pub mod maildir_error_handling;
pub mod maildir_mapper;
pub mod maildir_ui;
pub mod message;
pub mod notifications;
pub mod sorting;
pub mod sync_engine;
pub mod imap_service;
pub mod thread;
pub mod threading_engine;
pub mod timestamp_utils;

pub use advanced_filters::{
    AdvancedEmailFilter, AdvancedFilterEngine, AdvancedFilterResult, AdvancedCondition,
    ConditionGroup, BooleanLogic, AdvancedFilterField, AdvancedFilterOperator, FilterValue,
    ActionRule, AdvancedFilterAction, FilterTemplateLibrary, FilterStatistics, TimePeriod,
    NotificationPriority, MessagePriority,
};
pub use advanced_filters_ui::{AdvancedFiltersUI, FilterUIAction, FilterTab};
pub use attachment_viewer::{AttachmentViewer, ViewResult, ViewerMode};
pub use attachments::{AttachmentInfo, AttachmentManager, AttachmentType};
pub use database::{
    BackupResult, CleanupResult, DatabaseError, DatabaseResult, DatabaseStats, EmailDatabase,
    FolderSyncState, RestoreResult, StoredAttachment, StoredMessage, SyncStatus,
};
pub use database_optimizations::{
    OptimizedDatabase, DatabaseOptimizationConfig, PaginationConfig, SearchFilters,
    SortDirection, QueryStats, BatchOperationResult, FolderMessageCount,
};
pub use desktop_notifications::DesktopNotificationService;
pub use filters::{
    EmailFilter, FilterAction, FilterCondition, FilterEngine, FilterField, FilterOperator,
    FilterResult, FilterTemplates,
};
pub use folder_hierarchy::{
    FolderHierarchy, FolderHierarchyError, FolderHierarchyMapper, FolderHierarchyResult,
};
pub use maildir::{MaildirError, MaildirFolderStats, MaildirHandler, MaildirResult, MaildirStats};
pub use maildir_exporter::{
    ExportConfig, ExportStats, MaildirExportError, MaildirExporter, MaildirExportResult,
    ExportProgressCallback,
};
pub use maildir_export_wizard::{
    ExportProgress, ExportWizard, ExportWizardError, ExportWizardResult, ExportWizardState,
    ExportWizardStep,
};
pub use maildir_importer::{
    ImportConfig, ImportStats, MaildirImportError, MaildirImporter, MaildirImportResult,
    ProgressCallback,
};
pub use maildir_import_wizard::{
    DirectoryEntry, ImportProgress, ImportWizard, ImportWizardError, ImportWizardResult,
    ImportWizardState, MaildirFolderEntry, WizardStep,
};
#[cfg(test)]
pub use maildir_integration_tests::MaildirTestEnvironment;
pub use maildir_error_handling::{
    MaildirErrorHandler, MaildirOperationContext, MaildirOperationError,
};
pub use maildir_mapper::{
    FlagMapping, MaildirFilenameInfo, MaildirMapper, MaildirMapperError, MaildirMapperResult,
    MaildirMessageMetadata,
};
pub use maildir_ui::{MaildirExportPreview, MaildirImportFolder, MaildirImportPreview, MaildirUI};
pub use message::{EmailMessage, MessageId};
pub use notifications::{
    EmailNotification, EmailNotificationHandler, EmailNotificationManager, UIEmailUpdater,
};
pub use sorting::{MultiCriteriaSorter, SortCriteria, SortOrder};
pub use sync_engine::{
    ConflictResolution, SyncEngine, SyncError, SyncPhase, SyncProgress, SyncResult, SyncStrategy,
};
pub use imap_service::{ImapService, IdleUpdate};
pub use thread::{EmailThread, ThreadStatistics};
pub use threading_engine::{ThreadingAlgorithm, ThreadingEngine};
pub use timestamp_utils::{TimestampError, TimestampPreserver, TimestampResult, TimestampUtils};
