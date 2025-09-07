pub mod advanced_filters;
pub mod advanced_filters_ui;
pub mod ai_assistant;
pub mod async_sync_service;
pub mod attachment_viewer;
pub mod attachments;
pub mod auto_sync_scheduler;
pub mod database;
pub mod database_optimizations;
pub mod desktop_notifications;
pub mod filters;
pub mod folder_hierarchy;
pub mod imap_service;
pub mod maildir;
pub mod maildir_error_handling;
pub mod maildir_export_wizard;
pub mod maildir_exporter;
pub mod maildir_import_wizard;
pub mod maildir_importer;
#[cfg(test)]
pub mod maildir_integration_tests;
pub mod maildir_mapper;
pub mod maildir_ui;
pub mod message;
pub mod notifications;
pub mod operations_service;
pub mod retry;
pub mod service_health;
pub mod sorting;
pub mod sync_config;
pub mod sync_engine;
pub mod thread;
pub mod threading_engine;
pub mod timestamp_utils;

pub use advanced_filters::{
    ActionRule, AdvancedCondition, AdvancedEmailFilter, AdvancedFilterAction, AdvancedFilterEngine,
    AdvancedFilterField, AdvancedFilterOperator, AdvancedFilterResult, BooleanLogic,
    ConditionGroup, FilterStatistics, FilterTemplateLibrary, FilterValue, MessagePriority,
    NotificationPriority, TimePeriod,
};
pub use advanced_filters_ui::{AdvancedFiltersUI, FilterTab, FilterUIAction};
pub use ai_assistant::{
    AIEmailAssistant, BulkAnalysisStats, BulkEmailAnalysis, EmailCompositionAssistance,
    EmailReplyAssistance, EmailSummary,
};
pub use async_sync_service::AsyncSyncService;
pub use attachment_viewer::{AttachmentViewer, ViewResult, ViewerMode};
pub use attachments::{AttachmentInfo, AttachmentManager, AttachmentType};
pub use auto_sync_scheduler::{AutoSyncConfig, AutoSyncScheduler, AutoSyncStats};
pub use database::{
    BackupResult, CleanupResult, DatabaseError, DatabaseResult, DatabaseStats, EmailAccount,
    EmailDatabase, FolderSyncState, RestoreResult, StoredAttachment, StoredMessage, SyncStatus,
};
pub use database_optimizations::{
    BatchOperationResult, DatabaseOptimizationConfig, FolderMessageCount, OptimizedDatabase,
    PaginationConfig, QueryStats, SearchFilters, SortDirection,
};
pub use desktop_notifications::DesktopNotificationService;
pub use filters::{
    EmailFilter, FilterAction, FilterCondition, FilterEngine, FilterField, FilterOperator,
    FilterResult, FilterTemplates,
};
pub use folder_hierarchy::{
    FolderHierarchy, FolderHierarchyError, FolderHierarchyMapper, FolderHierarchyResult,
};
pub use imap_service::{IdleUpdate, ImapService};
pub use maildir::{MaildirError, MaildirFolderStats, MaildirHandler, MaildirResult, MaildirStats};
pub use maildir_error_handling::{
    MaildirErrorHandler, MaildirOperationContext, MaildirOperationError,
};
pub use maildir_export_wizard::{
    ExportProgress, ExportWizard, ExportWizardError, ExportWizardResult, ExportWizardState,
    ExportWizardStep,
};
pub use maildir_exporter::{
    ExportConfig, ExportProgressCallback, ExportStats, MaildirExportError, MaildirExportResult,
    MaildirExporter,
};
pub use maildir_import_wizard::{
    DirectoryEntry, ImportProgress, ImportWizard, ImportWizardError, ImportWizardResult,
    ImportWizardState, MaildirFolderEntry, WizardStep,
};
pub use maildir_importer::{
    ImportConfig, ImportStats, MaildirImportError, MaildirImportResult, MaildirImporter,
    ProgressCallback,
};
#[cfg(test)]
pub use maildir_integration_tests::MaildirTestEnvironment;
pub use maildir_mapper::{
    FlagMapping, MaildirFilenameInfo, MaildirMapper, MaildirMapperError, MaildirMapperResult,
    MaildirMessageMetadata,
};
pub use maildir_ui::{MaildirExportPreview, MaildirImportFolder, MaildirImportPreview, MaildirUI};
pub use message::{EmailMessage, MessageId};
pub use notifications::{
    EmailNotification, EmailNotificationHandler, EmailNotificationManager, UIEmailUpdater,
};
pub use operations_service::{EmailOperationError, EmailOperationResult, EmailOperationsService};
pub use sorting::{MultiCriteriaSorter, SortCriteria, SortOrder};
pub use sync_config::{AccountSyncSettings, ConfigStats, SyncConfigFile, SyncConfigManager};
pub use sync_engine::{
    ConflictResolution, SyncEngine, SyncError, SyncPhase, SyncProgress, SyncResult, SyncStrategy,
};
pub use thread::{EmailThread, ThreadStatistics};
pub use threading_engine::{ThreadingAlgorithm, ThreadingEngine};
pub use timestamp_utils::{TimestampError, TimestampPreserver, TimestampResult, TimestampUtils};
