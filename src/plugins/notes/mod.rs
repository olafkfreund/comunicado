//! Notes plugin for Comunicado
//! 
//! Provides comprehensive note-taking functionality with markdown support,
//! wiki-style linking, full-text search, and integration with email and calendar systems.

pub mod types;
pub mod manager;
pub mod parser;
pub mod storage;
pub mod database;
pub mod indexer;
pub mod linker;
pub mod watcher;
pub mod scanner;
pub mod integration;
pub mod plugin;
pub mod advanced_search;
pub mod email_integration;

#[cfg(test)]
mod ignore_patterns_test;

#[cfg(test)]
mod search_tests;

pub use types::*;
pub use plugin::NotesPlugin;

// Plugin core types will be used when integration is complete

/// Re-export commonly used types
pub use types::{Note, NoteFrontmatter, WikiLink, NoteId, LinkType};
pub use manager::NoteManager;
pub use parser::MarkdownParser;
pub use storage::NoteStorage;
pub use database::NotesDatabase;
pub use indexer::{NoteIndexer, IndexerConfig, IndexingStats};
pub use linker::LinkResolver;
pub use watcher::FileWatcher;
pub use scanner::{DirectoryScanner, ScannedFile, ScanResult, ScanConfig};
pub use integration::{FileSystemMonitor, ProcessingResult, MonitoringStats};
pub use advanced_search::{AdvancedSearchEngine, AdvancedSearchOptions, SearchFilters, SearchCategory, RankingConfig, EnhancedSearchResult, SearchResultSummary};
pub use email_integration::{EmailIntegrationService, EmailNote, EmailLinkType, EmailContact, EmailThread, EmailNotesStats};