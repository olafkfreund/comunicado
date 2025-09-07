//! Notes plugin for Comunicado
//!
//! Provides comprehensive note-taking functionality with markdown support,
//! wiki-style linking, full-text search, and integration with email and calendar systems.

pub mod advanced_search;
pub mod calendar_integration;
pub mod conversions;
pub mod database;
pub mod email_integration;
pub mod indexer;
pub mod integration;
pub mod linker;
pub mod manager;
pub mod mobile_integration;
pub mod parser;
pub mod plugin;
pub mod scanner;
pub mod storage;
pub mod tui;
pub mod tui_render;
pub mod types;
pub mod watcher;

#[cfg(test)]
mod ignore_patterns_test;

#[cfg(test)]
mod search_tests;

pub use plugin::NotesPlugin;
pub use types::*;

// Plugin core types will be used when integration is complete

pub use advanced_search::{
    AdvancedSearchEngine, AdvancedSearchOptions, EnhancedSearchResult, RankingConfig,
    SearchCategory, SearchFilters, SearchResultSummary,
};
pub use calendar_integration::{
    ActionItem, CalendarNoteEvent, CalendarNotesConfig, CalendarNotesIntegration,
    CalendarNotesStats, EventLinkType, MeetingAttendee, MeetingNote,
};
pub use conversions::NoteConversionService;
pub use database::NotesDatabase;
pub use email_integration::{
    EmailContact, EmailIntegrationService, EmailLinkType, EmailNote, EmailNotesStats, EmailThread,
};
pub use indexer::{IndexerConfig, IndexingStats, NoteIndexer};
pub use integration::{FileSystemMonitor, MonitoringStats, ProcessingResult};
pub use linker::LinkResolver;
pub use manager::NoteManager;
pub use mobile_integration::{
    MobileNoteEvent, MobileNotesConfig, MobileNotesIntegration, MobileNotesStats,
    SmsConversionCandidate,
};
pub use parser::MarkdownParser;
pub use scanner::{DirectoryScanner, ScanConfig, ScanResult, ScannedFile};
pub use storage::NoteStorage;
pub use tui::{NoteTUI, PopupState, TUIConfig, TUIMode, TUIStats, TUITheme};
/// Re-export commonly used types
pub use types::{LinkType, Note, NoteFrontmatter, NoteId, WikiLink};
pub use watcher::FileWatcher;
