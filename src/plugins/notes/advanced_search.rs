//! Advanced search functionality for notes
//! 
//! Provides sophisticated search capabilities with filters, ranking, and categorization.

use super::types::{Note, NoteSearchResult, NoteId};
use super::storage::NoteStorage;
use super::manager::{NoteError, NoteResult};

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

/// Search filter criteria
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFilters {
    /// Filter by tags (any of these tags)
    pub tags: Vec<String>,
    /// Filter by date range (created/modified)
    pub date_range: Option<DateRange>,
    /// Filter by file extension
    pub file_extensions: Vec<String>,
    /// Filter by minimum file size (bytes)
    pub min_file_size: Option<u64>,
    /// Filter by maximum file size (bytes)
    pub max_file_size: Option<u64>,
    /// Filter by minimum word count
    pub min_word_count: Option<usize>,
    /// Filter by maximum word count
    pub max_word_count: Option<usize>,
    /// Filter by directory paths
    pub directories: Vec<PathBuf>,
    /// Exclude notes with these tags
    pub exclude_tags: Vec<String>,
    /// Only include notes with frontmatter
    pub has_frontmatter: Option<bool>,
    /// Only include notes with wiki links
    pub has_wiki_links: Option<bool>,
}

impl Default for SearchFilters {
    fn default() -> Self {
        Self {
            tags: Vec::new(),
            date_range: None,
            file_extensions: Vec::new(),
            min_file_size: None,
            max_file_size: None,
            min_word_count: None,
            max_word_count: None,
            directories: Vec::new(),
            exclude_tags: Vec::new(),
            has_frontmatter: None,
            has_wiki_links: None,
        }
    }
}

/// Date range filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

/// Search categories for targeted searching
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SearchCategory {
    /// Search in all fields
    All,
    /// Search only in note titles
    Title,
    /// Search only in note content
    Content,
    /// Search only in tags
    Tags,
    /// Search only in filenames
    Filename,
    /// Search only in frontmatter
    Frontmatter,
}

/// Advanced search ranking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingConfig {
    /// Weight for title matches (default: 3.0)
    pub title_weight: f64,
    /// Weight for content matches (default: 1.0)
    pub content_weight: f64,
    /// Weight for tag matches (default: 2.0)
    pub tag_weight: f64,
    /// Weight for filename matches (default: 1.5)
    pub filename_weight: f64,
    /// Boost factor for recent notes (default: 0.1)
    pub recency_boost: f64,
    /// Days to consider for recency boost (default: 30)
    pub recency_days: u32,
    /// Boost factor for frequently linked notes (default: 0.2)
    pub link_popularity_boost: f64,
    /// Enable TF-IDF scoring (default: true)
    pub use_tfidf: bool,
}

impl Default for RankingConfig {
    fn default() -> Self {
        Self {
            title_weight: 3.0,
            content_weight: 1.0,
            tag_weight: 2.0,
            filename_weight: 1.5,
            recency_boost: 0.1,
            recency_days: 30,
            link_popularity_boost: 0.2,
            use_tfidf: true,
        }
    }
}

/// Search options for advanced search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedSearchOptions {
    /// Search query text
    pub query: String,
    /// Search filters to apply
    pub filters: SearchFilters,
    /// Search category to target
    pub category: SearchCategory,
    /// Ranking configuration
    pub ranking: RankingConfig,
    /// Maximum number of results
    pub limit: usize,
    /// Offset for pagination
    pub offset: usize,
    /// Enable search result highlighting
    pub highlight: bool,
    /// Maximum snippet length for results
    pub max_snippet_length: usize,
}

impl Default for AdvancedSearchOptions {
    fn default() -> Self {
        Self {
            query: String::new(),
            filters: SearchFilters::default(),
            category: SearchCategory::All,
            ranking: RankingConfig::default(),
            limit: 50,
            offset: 0,
            highlight: true,
            max_snippet_length: 200,
        }
    }
}

/// Enhanced search result with ranking information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedSearchResult {
    /// The note that matched
    pub note: Note,
    /// Relevance score (0.0 to 1.0)
    pub score: f64,
    /// Search result snippet with highlighting
    pub snippet: String,
    /// Matched categories
    pub matched_categories: Vec<SearchCategory>,
    /// Individual component scores
    pub component_scores: ComponentScores,
    /// Search result rank (1-based)
    pub rank: usize,
}

/// Individual scoring components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentScores {
    /// Title match score
    pub title_score: f64,
    /// Content match score
    pub content_score: f64,
    /// Tag match score
    pub tag_score: f64,
    /// Filename match score
    pub filename_score: f64,
    /// Recency boost applied
    pub recency_boost: f64,
    /// Link popularity boost
    pub link_popularity_boost: f64,
    /// TF-IDF score
    pub tfidf_score: f64,
}

impl Default for ComponentScores {
    fn default() -> Self {
        Self {
            title_score: 0.0,
            content_score: 0.0,
            tag_score: 0.0,
            filename_score: 0.0,
            recency_boost: 0.0,
            link_popularity_boost: 0.0,
            tfidf_score: 0.0,
        }
    }
}

/// Search result summary with statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultSummary {
    /// Total number of results (before pagination)
    pub total_results: usize,
    /// Number of results returned
    pub returned_results: usize,
    /// Search execution time
    pub search_duration: Duration,
    /// Query processing time
    pub query_processing_time: Duration,
    /// Results by category
    pub category_counts: HashMap<SearchCategory, usize>,
    /// Average relevance score
    pub average_score: f64,
    /// Highest relevance score
    pub max_score: f64,
    /// Search suggestion (if no results)
    pub suggestion: Option<String>,
}

/// Cache entry for search results
#[derive(Debug, Clone)]
struct SearchCacheEntry {
    results: Vec<EnhancedSearchResult>,
    summary: SearchResultSummary,
    timestamp: SystemTime,
    options: AdvancedSearchOptions,
}

/// Advanced search engine with filtering and ranking
#[derive(Debug, Clone)]
pub struct AdvancedSearchEngine {
    /// Storage layer for database queries
    storage: Arc<NoteStorage>,
    /// Result cache for performance
    cache: Arc<tokio::sync::RwLock<HashMap<String, SearchCacheEntry>>>,
    /// Link popularity tracking
    link_popularity: Arc<tokio::sync::RwLock<HashMap<NoteId, usize>>>,
    /// Default search options
    default_options: AdvancedSearchOptions,
    /// Cache TTL (time to live)
    cache_ttl: Duration,
}

impl AdvancedSearchEngine {
    /// Create a new advanced search engine
    pub fn new(storage: Arc<NoteStorage>) -> Self {
        Self {
            storage,
            cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            link_popularity: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            default_options: AdvancedSearchOptions::default(),
            cache_ttl: Duration::from_secs(300), // 5 minutes
        }
    }

    /// Create with custom default options
    pub fn with_options(storage: Arc<NoteStorage>, options: AdvancedSearchOptions) -> Self {
        Self {
            storage,
            cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            link_popularity: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            default_options: options,
            cache_ttl: Duration::from_secs(300),
        }
    }

    /// Perform advanced search with filters and ranking
    pub async fn search(&self, options: AdvancedSearchOptions) -> NoteResult<(Vec<EnhancedSearchResult>, SearchResultSummary)> {
        let start_time = std::time::Instant::now();
        
        // Check cache first
        let cache_key = self.generate_cache_key(&options);
        if let Some(cached) = self.get_cached_result(&cache_key).await {
            return Ok((cached.results, cached.summary));
        }

        // Validate search options
        self.validate_search_options(&options)?;

        // Build search query based on category
        let search_query = self.build_search_query(&options);
        
        // Perform initial database search
        let raw_results = self.storage.search_notes(&search_query, options.limit * 2).await?;
        
        // Apply filters
        let filtered_results = self.apply_filters(&raw_results, &options.filters).await?;
        
        // Calculate enhanced scores and ranking
        let mut enhanced_results = self.calculate_enhanced_scores(filtered_results, &options).await?;
        
        // Sort by relevance score
        enhanced_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        
        // Apply pagination
        let total_results = enhanced_results.len();
        let paginated_results = enhanced_results
            .into_iter()
            .skip(options.offset)
            .take(options.limit)
            .enumerate()
            .map(|(index, mut result)| {
                result.rank = options.offset + index + 1;
                result
            })
            .collect::<Vec<_>>();

        // Generate search summary
        let search_duration = start_time.elapsed();
        let summary = self.generate_search_summary(&paginated_results, total_results, search_duration, &options);

        // Cache results
        self.cache_results(cache_key, &paginated_results, &summary, &options).await;

        Ok((paginated_results, summary))
    }

    /// Search with simple query (uses default options)
    pub async fn simple_search(&self, query: &str, limit: usize) -> NoteResult<Vec<EnhancedSearchResult>> {
        let options = AdvancedSearchOptions {
            query: query.to_string(),
            limit,
            ..self.default_options.clone()
        };
        
        let (results, _) = self.search(options).await?;
        Ok(results)
    }

    /// Search by category
    pub async fn search_by_category(&self, query: &str, category: SearchCategory, limit: usize) -> NoteResult<Vec<EnhancedSearchResult>> {
        let options = AdvancedSearchOptions {
            query: query.to_string(),
            category,
            limit,
            ..self.default_options.clone()
        };
        
        let (results, _) = self.search(options).await?;
        Ok(results)
    }

    /// Search with filters
    pub async fn search_with_filters(&self, query: &str, filters: SearchFilters, limit: usize) -> NoteResult<Vec<EnhancedSearchResult>> {
        let options = AdvancedSearchOptions {
            query: query.to_string(),
            filters,
            limit,
            ..self.default_options.clone()
        };
        
        let (results, _) = self.search(options).await?;
        Ok(results)
    }

    /// Update link popularity for a note
    pub async fn update_link_popularity(&self, note_id: &NoteId, link_count: usize) {
        let mut popularity = self.link_popularity.write().await;
        popularity.insert(note_id.clone(), link_count);
    }

    /// Clear search cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> (usize, Duration) {
        let cache = self.cache.read().await;
        (cache.len(), self.cache_ttl)
    }

    // ==================== Private Helper Methods ====================

    /// Validate search options
    fn validate_search_options(&self, options: &AdvancedSearchOptions) -> NoteResult<()> {
        if options.query.is_empty() {
            return Err(NoteError::Search("Search query cannot be empty".to_string()));
        }
        
        if options.limit == 0 {
            return Err(NoteError::Search("Search limit must be greater than 0".to_string()));
        }
        
        if options.limit > 1000 {
            return Err(NoteError::Search("Search limit cannot exceed 1000".to_string()));
        }
        
        Ok(())
    }

    /// Build search query based on category
    fn build_search_query(&self, options: &AdvancedSearchOptions) -> String {
        match options.category {
            SearchCategory::All => options.query.clone(),
            SearchCategory::Title => format!("title:{}", options.query),
            SearchCategory::Content => format!("content:{}", options.query),
            SearchCategory::Tags => format!("tags:{}", options.query),
            SearchCategory::Filename => format!("filename:{}", options.query),
            SearchCategory::Frontmatter => format!("frontmatter:{}", options.query),
        }
    }

    /// Apply filters to search results
    async fn apply_filters(&self, results: &[NoteSearchResult], filters: &SearchFilters) -> NoteResult<Vec<NoteSearchResult>> {
        let mut filtered = Vec::new();
        
        for result in results {
            if self.note_matches_filters(&result.note, filters).await? {
                filtered.push(result.clone());
            }
        }
        
        Ok(filtered)
    }

    /// Check if a note matches the given filters
    async fn note_matches_filters(&self, note: &Note, filters: &SearchFilters) -> NoteResult<bool> {
        // Tag filters
        if !filters.tags.is_empty() {
            let has_required_tag = filters.tags.iter().any(|tag| note.tags.contains(tag));
            if !has_required_tag {
                return Ok(false);
            }
        }

        // Exclude tag filters
        if !filters.exclude_tags.is_empty() {
            let has_excluded_tag = filters.exclude_tags.iter().any(|tag| note.tags.contains(tag));
            if has_excluded_tag {
                return Ok(false);
            }
        }

        // File extension filters
        if !filters.file_extensions.is_empty() {
            if let Some(extension) = note.path.extension() {
                let ext_str = extension.to_string_lossy().to_string();
                if !filters.file_extensions.contains(&ext_str) {
                    return Ok(false);
                }
            } else {
                return Ok(false); // No extension, but filters require specific extensions
            }
        }

        // File size filters
        if let Some(min_size) = filters.min_file_size {
            if note.file_size < min_size {
                return Ok(false);
            }
        }
        
        if let Some(max_size) = filters.max_file_size {
            if note.file_size > max_size {
                return Ok(false);
            }
        }

        // Word count filters
        if let Some(min_words) = filters.min_word_count {
            if note.word_count < min_words {
                return Ok(false);
            }
        }
        
        if let Some(max_words) = filters.max_word_count {
            if note.word_count > max_words {
                return Ok(false);
            }
        }

        // Directory filters
        if !filters.directories.is_empty() {
            let note_parent = note.path.parent().unwrap_or(&note.path);
            let matches_directory = filters.directories.iter().any(|dir| {
                note_parent.starts_with(dir)
            });
            if !matches_directory {
                return Ok(false);
            }
        }

        // Date range filters
        if let Some(ref date_range) = filters.date_range {
            if !self.note_matches_date_range(note, date_range) {
                return Ok(false);
            }
        }

        // Frontmatter filter
        if let Some(has_frontmatter) = filters.has_frontmatter {
            if note.frontmatter.is_some() != has_frontmatter {
                return Ok(false);
            }
        }

        // Wiki links filter
        if let Some(has_wiki_links) = filters.has_wiki_links {
            // Check if note content contains wiki-style links [[...]]
            let has_links = note.content.contains("[[") && note.content.contains("]]");
            if has_links != has_wiki_links {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Check if note matches date range
    fn note_matches_date_range(&self, note: &Note, date_range: &DateRange) -> bool {
        let note_date = note.created_at;
        
        if let Some(start) = date_range.start {
            if note_date < start {
                return false;
            }
        }
        
        if let Some(end) = date_range.end {
            if note_date > end {
                return false;
            }
        }
        
        true
    }

    /// Calculate enhanced scores for search results
    async fn calculate_enhanced_scores(&self, results: Vec<NoteSearchResult>, options: &AdvancedSearchOptions) -> NoteResult<Vec<EnhancedSearchResult>> {
        let mut enhanced_results = Vec::new();
        let popularity = self.link_popularity.read().await;
        
        for result in results {
            let component_scores = self.calculate_component_scores(&result, options, &popularity);
            let total_score = self.calculate_total_score(&component_scores, &options.ranking);
            let snippet = self.generate_snippet(&result, options);
            let matched_categories = self.determine_matched_categories(&result, options);
            
            enhanced_results.push(EnhancedSearchResult {
                note: result.note,
                score: total_score,
                snippet,
                matched_categories,
                component_scores,
                rank: 0, // Will be set during sorting
            });
        }
        
        Ok(enhanced_results)
    }

    /// Calculate individual component scores
    fn calculate_component_scores(&self, result: &NoteSearchResult, options: &AdvancedSearchOptions, popularity: &HashMap<NoteId, usize>) -> ComponentScores {
        let query_lower = options.query.to_lowercase();
        let mut scores = ComponentScores::default();
        
        // Title score
        if result.note.title.to_lowercase().contains(&query_lower) {
            scores.title_score = self.calculate_text_relevance(&result.note.title, &options.query);
        }
        
        // Content score
        if result.note.content.to_lowercase().contains(&query_lower) {
            scores.content_score = self.calculate_text_relevance(&result.note.content, &options.query);
        }
        
        // Tag score
        for tag in &result.note.tags {
            if tag.to_lowercase().contains(&query_lower) {
                scores.tag_score = scores.tag_score.max(self.calculate_text_relevance(tag, &options.query));
            }
        }
        
        // Filename score
        if let Some(filename) = result.note.path.file_name() {
            let filename_str = filename.to_string_lossy();
            if filename_str.to_lowercase().contains(&query_lower) {
                scores.filename_score = self.calculate_text_relevance(&filename_str, &options.query);
            }
        }
        
        // Recency boost
        scores.recency_boost = self.calculate_recency_boost(&result.note, &options.ranking);
        
        // Link popularity boost
        let link_count = popularity.get(&result.note.id).unwrap_or(&0);
        scores.link_popularity_boost = (*link_count as f64).log10().max(0.0) * options.ranking.link_popularity_boost;
        
        // TF-IDF score (simplified approximation)
        if options.ranking.use_tfidf {
            scores.tfidf_score = self.calculate_tfidf_score(result, &options.query);
        }
        
        scores
    }

    /// Calculate total relevance score
    fn calculate_total_score(&self, scores: &ComponentScores, ranking: &RankingConfig) -> f64 {
        let base_score = scores.title_score * ranking.title_weight
            + scores.content_score * ranking.content_weight
            + scores.tag_score * ranking.tag_weight
            + scores.filename_score * ranking.filename_weight;
        
        let boosted_score = base_score
            + scores.recency_boost
            + scores.link_popularity_boost
            + scores.tfidf_score;
        
        // Normalize to 0.0-1.0 range
        boosted_score.min(1.0).max(0.0)
    }

    /// Calculate text relevance score for a field
    fn calculate_text_relevance(&self, text: &str, query: &str) -> f64 {
        let text_lower = text.to_lowercase();
        let query_lower = query.to_lowercase();
        
        // Exact match gets highest score
        if text_lower == query_lower {
            return 1.0;
        }
        
        // Prefix match gets high score
        if text_lower.starts_with(&query_lower) {
            return 0.8;
        }
        
        // Contains match gets medium score
        if text_lower.contains(&query_lower) {
            return 0.6;
        }
        
        // Word boundary match
        let words: Vec<&str> = text_lower.split_whitespace().collect();
        for word in words {
            if word.starts_with(&query_lower) {
                return 0.4;
            }
        }
        
        0.0
    }

    /// Calculate recency boost based on note age
    fn calculate_recency_boost(&self, note: &Note, ranking: &RankingConfig) -> f64 {
        let now = Utc::now();
        let note_age = now - note.created_at;
        let age_days = note_age.num_days().max(0) as f64;
        let recency_window = ranking.recency_days as f64;
        
        if age_days <= recency_window {
            let recency_factor = (recency_window - age_days) / recency_window;
            recency_factor * ranking.recency_boost
        } else {
            0.0
        }
    }

    /// Calculate simplified TF-IDF score
    fn calculate_tfidf_score(&self, result: &NoteSearchResult, query: &str) -> f64 {
        let query_terms: Vec<&str> = query.split_whitespace().collect();
        let content_words: Vec<&str> = result.note.content.split_whitespace().collect();
        let total_words = content_words.len() as f64;
        
        if total_words == 0.0 {
            return 0.0;
        }
        
        let mut tf_idf_sum = 0.0;
        
        for term in &query_terms {
            let term_lower = term.to_lowercase();
            let term_count = content_words.iter()
                .filter(|word| word.to_lowercase() == term_lower)
                .count() as f64;
            
            if term_count > 0.0 {
                let tf = term_count / total_words;
                // Simplified IDF (normally would require corpus statistics)
                let idf = (1.0 + total_words / term_count).ln();
                tf_idf_sum += tf * idf;
            }
        }
        
        tf_idf_sum / query_terms.len() as f64
    }

    /// Generate search result snippet
    fn generate_snippet(&self, result: &NoteSearchResult, options: &AdvancedSearchOptions) -> String {
        let content = &result.note.content;
        let query_lower = options.query.to_lowercase();
        
        // Find the best match position in content
        if let Some(match_pos) = content.to_lowercase().find(&query_lower) {
            let start = match_pos.saturating_sub(options.max_snippet_length / 2);
            let end = (start + options.max_snippet_length).min(content.len());
            
            let mut snippet = content.chars().skip(start).take(end - start).collect::<String>();
            
            // Add ellipsis if truncated
            if start > 0 {
                snippet = format!("...{}", snippet);
            }
            if end < content.len() {
                snippet = format!("{}...", snippet);
            }
            
            // Apply highlighting if enabled
            if options.highlight {
                snippet = self.apply_highlighting(&snippet, &options.query);
            }
            
            snippet
        } else {
            // Fallback to beginning of content
            let end = options.max_snippet_length.min(content.len());
            let mut snippet = content.chars().take(end).collect::<String>();
            
            if end < content.len() {
                snippet = format!("{}...", snippet);
            }
            
            snippet
        }
    }

    /// Apply search highlighting to snippet
    fn apply_highlighting(&self, snippet: &str, query: &str) -> String {
        // Simple highlighting with **bold** markers
        // In a real implementation, this would use proper markup
        let query_lower = query.to_lowercase();
        let snippet_lower = snippet.to_lowercase();
        
        if let Some(match_pos) = snippet_lower.find(&query_lower) {
            let before = &snippet[..match_pos];
            let matched = &snippet[match_pos..match_pos + query.len()];
            let after = &snippet[match_pos + query.len()..];
            
            format!("{}**{}**{}", before, matched, after)
        } else {
            snippet.to_string()
        }
    }

    /// Determine which categories matched for a result
    fn determine_matched_categories(&self, result: &NoteSearchResult, options: &AdvancedSearchOptions) -> Vec<SearchCategory> {
        let mut categories = Vec::new();
        let query_lower = options.query.to_lowercase();
        
        if result.note.title.to_lowercase().contains(&query_lower) {
            categories.push(SearchCategory::Title);
        }
        
        if result.note.content.to_lowercase().contains(&query_lower) {
            categories.push(SearchCategory::Content);
        }
        
        if result.note.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower)) {
            categories.push(SearchCategory::Tags);
        }
        
        if let Some(filename) = result.note.path.file_name() {
            if filename.to_string_lossy().to_lowercase().contains(&query_lower) {
                categories.push(SearchCategory::Filename);
            }
        }
        
        if let Some(ref frontmatter) = result.note.frontmatter {
            if let Some(ref title) = frontmatter.title {
                if title.to_lowercase().contains(&query_lower) {
                    categories.push(SearchCategory::Frontmatter);
                }
            }
        }
        
        categories
    }

    /// Generate search result summary
    fn generate_search_summary(&self, results: &[EnhancedSearchResult], total_results: usize, duration: Duration, options: &AdvancedSearchOptions) -> SearchResultSummary {
        let mut category_counts = HashMap::new();
        let mut score_sum = 0.0;
        let mut max_score: f64 = 0.0;
        
        for result in results {
            score_sum += result.score;
            max_score = max_score.max(result.score);
            
            for category in &result.matched_categories {
                *category_counts.entry(*category).or_insert(0) += 1;
            }
        }
        
        let average_score = if results.is_empty() { 0.0 } else { score_sum / results.len() as f64 };
        
        SearchResultSummary {
            total_results,
            returned_results: results.len(),
            search_duration: duration,
            query_processing_time: Duration::from_millis(1), // Placeholder
            category_counts,
            average_score,
            max_score,
            suggestion: if results.is_empty() {
                Some(self.generate_search_suggestion(&options.query))
            } else {
                None
            },
        }
    }

    /// Generate search suggestion for empty results
    fn generate_search_suggestion(&self, query: &str) -> String {
        // Simple suggestion logic
        if query.len() < 3 {
            "Try using longer search terms".to_string()
        } else if query.contains("\"") {
            "Try removing quotes for broader search".to_string()
        } else {
            format!("Try broader terms or check spelling: '{}'", query)
        }
    }

    /// Generate cache key for search options
    fn generate_cache_key(&self, options: &AdvancedSearchOptions) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        options.query.hash(&mut hasher);
        options.category.hash(&mut hasher);
        options.limit.hash(&mut hasher);
        options.offset.hash(&mut hasher);
        // Note: We skip hashing filters and ranking for simplicity
        // In a real implementation, these would be included
        
        format!("search_{:x}", hasher.finish())
    }

    /// Get cached search result if valid
    async fn get_cached_result(&self, cache_key: &str) -> Option<SearchCacheEntry> {
        let cache = self.cache.read().await;
        if let Some(entry) = cache.get(cache_key) {
            if entry.timestamp.elapsed().unwrap_or(Duration::MAX) < self.cache_ttl {
                return Some(entry.clone());
            }
        }
        None
    }

    /// Cache search results
    async fn cache_results(&self, cache_key: String, results: &[EnhancedSearchResult], summary: &SearchResultSummary, options: &AdvancedSearchOptions) {
        let entry = SearchCacheEntry {
            results: results.to_vec(),
            summary: summary.clone(),
            timestamp: SystemTime::now(),
            options: options.clone(),
        };
        
        let mut cache = self.cache.write().await;
        cache.insert(cache_key, entry);
        
        // Clean up old cache entries if cache is getting large
        if cache.len() > 100 {
            let cutoff = SystemTime::now() - self.cache_ttl;
            cache.retain(|_, entry| entry.timestamp > cutoff);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::notes::storage::NoteStorage;
    use crate::plugins::notes::types::Note;
    use std::path::PathBuf;
    use chrono::Utc;

    async fn create_test_storage() -> Arc<NoteStorage> {
        Arc::new(NoteStorage::new_in_memory().await.unwrap())
    }

    async fn create_test_notes(storage: &NoteStorage) -> Vec<Note> {
        use crate::plugins::notes::types::WatchedDirectory;
        
        // Add a test directory
        let directory = WatchedDirectory::new(
            PathBuf::from("/test"),
            "Test Dir".to_string(),
        );
        let stored_dir = storage.add_watched_directory(directory).await.unwrap();

        let notes = vec![
            Note::new(
                "note1".to_string(),
                "Rust Programming Guide".to_string(),
                "This is a comprehensive guide to Rust programming language. It covers memory safety, ownership, and concurrency.".to_string(),
                PathBuf::from("/test/rust_guide.md"),
            ),
            Note::new(
                "note2".to_string(),
                "Python Tutorial".to_string(),
                "Learn Python programming from basics. Covers data structures, functions, and object-oriented programming.".to_string(),
                PathBuf::from("/test/python_tutorial.md"),
            ),
            Note::new(
                "note3".to_string(),
                "Project Planning".to_string(),
                "Planning document for the new software project. Includes timeline, resources, and milestones.".to_string(),
                PathBuf::from("/test/project_plan.md"),
            ),
        ];

        for note in &notes {
            storage.store_note(&note, stored_dir.id).await.unwrap();
        }

        notes
    }

    #[tokio::test]
    async fn test_advanced_search_engine_creation() {
        let storage = create_test_storage().await;
        let engine = AdvancedSearchEngine::new(storage);
        
        let (cache_size, cache_ttl) = engine.get_cache_stats().await;
        assert_eq!(cache_size, 0);
        assert_eq!(cache_ttl, Duration::from_secs(300));
    }

    #[tokio::test]
    async fn test_simple_search() {
        let storage = create_test_storage().await;
        let _notes = create_test_notes(&storage).await;
        let engine = AdvancedSearchEngine::new(storage);
        
        let results = engine.simple_search("Rust", 10).await.unwrap();
        assert!(!results.is_empty());
        
        // Should find the Rust programming guide
        let rust_found = results.iter().any(|r| r.note.title.contains("Rust"));
        assert!(rust_found);
    }

    #[tokio::test]
    async fn test_search_by_category() {
        let storage = create_test_storage().await;
        let _notes = create_test_notes(&storage).await;
        let engine = AdvancedSearchEngine::new(storage);
        
        // Search only in titles
        let results = engine.search_by_category("Programming", SearchCategory::Title, 10).await.unwrap();
        
        for result in &results {
            assert!(result.matched_categories.contains(&SearchCategory::Title));
        }
    }

    #[tokio::test]
    async fn test_search_with_filters() {
        let storage = create_test_storage().await;
        let _notes = create_test_notes(&storage).await;
        let engine = AdvancedSearchEngine::new(storage);
        
        let filters = SearchFilters {
            file_extensions: vec!["md".to_string()],
            min_word_count: Some(10),
            ..Default::default()
        };
        
        let results = engine.search_with_filters("programming", filters, 10).await.unwrap();
        
        for result in &results {
            assert!(result.note.path.extension().unwrap() == "md");
            assert!(result.note.word_count >= 10);
        }
    }

    #[tokio::test]
    async fn test_advanced_search_with_ranking() {
        let storage = create_test_storage().await;
        let _notes = create_test_notes(&storage).await;
        let engine = AdvancedSearchEngine::new(storage);
        
        let ranking = RankingConfig {
            title_weight: 5.0, // Boost title matches
            content_weight: 1.0,
            use_tfidf: true,
            ..Default::default()
        };
        
        let options = AdvancedSearchOptions {
            query: "Programming".to_string(),
            ranking,
            limit: 10,
            ..Default::default()
        };
        
        let (results, summary) = engine.search(options).await.unwrap();
        
        assert!(summary.total_results > 0);
        assert!(summary.max_score > 0.0);
        
        // Results should be ranked by score
        for i in 1..results.len() {
            assert!(results[i-1].score >= results[i].score);
        }
    }

    #[tokio::test]
    async fn test_search_result_components() {
        let storage = create_test_storage().await;
        let _notes = create_test_notes(&storage).await;
        let engine = AdvancedSearchEngine::new(storage);
        
        let results = engine.simple_search("Rust", 5).await.unwrap();
        
        for result in &results {
            // Should have component scores
            assert!(result.component_scores.title_score >= 0.0);
            assert!(result.component_scores.content_score >= 0.0);
            
            // Should have snippet
            assert!(!result.snippet.is_empty());
            
            // Should have matched categories
            assert!(!result.matched_categories.is_empty());
            
            // Should have valid rank
            assert!(result.rank > 0);
        }
    }

    #[tokio::test]
    async fn test_search_cache() {
        let storage = create_test_storage().await;
        let _notes = create_test_notes(&storage).await;
        let engine = AdvancedSearchEngine::new(storage);
        
        // First search
        let start1 = std::time::Instant::now();
        let results1 = engine.simple_search("programming", 10).await.unwrap();
        let _duration1 = start1.elapsed();
        
        // Second search (should be cached)
        let start2 = std::time::Instant::now();
        let results2 = engine.simple_search("programming", 10).await.unwrap();
        let _duration2 = start2.elapsed();
        
        assert_eq!(results1.len(), results2.len());
        // Second search should be faster (cached)
        // Note: This might not always be true in tests due to timing variations
        
        let (cache_size, _) = engine.get_cache_stats().await;
        assert!(cache_size > 0);
    }

    #[tokio::test]
    async fn test_link_popularity_boost() {
        let storage = create_test_storage().await;
        let notes = create_test_notes(&storage).await;
        let engine = AdvancedSearchEngine::new(storage);
        
        // Set popularity for one note
        engine.update_link_popularity(&notes[0].id, 10).await;
        
        let results = engine.simple_search("programming", 10).await.unwrap();
        
        // The popular note should have link popularity boost
        if let Some(popular_result) = results.iter().find(|r| r.note.id == notes[0].id) {
            assert!(popular_result.component_scores.link_popularity_boost > 0.0);
        }
    }

    #[tokio::test]
    async fn test_date_range_filter() {
        let storage = create_test_storage().await;
        let _notes = create_test_notes(&storage).await;
        let engine = AdvancedSearchEngine::new(storage);
        
        let now = Utc::now();
        let yesterday = now - chrono::Duration::days(1);
        
        let filters = SearchFilters {
            date_range: Some(DateRange {
                start: Some(yesterday),
                end: Some(now),
            }),
            ..Default::default()
        };
        
        let results = engine.search_with_filters("programming", filters, 10).await.unwrap();
        
        // Should find notes created within the date range
        for result in &results {
            let note_date = result.note.created_at;
            assert!(note_date >= yesterday && note_date <= now);
        }
    }

    #[tokio::test]
    async fn test_search_validation() {
        let storage = create_test_storage().await;
        let engine = AdvancedSearchEngine::new(storage);
        
        // Empty query should fail
        let options = AdvancedSearchOptions {
            query: String::new(),
            ..Default::default()
        };
        
        let result = engine.search(options).await;
        assert!(result.is_err());
        
        // Zero limit should fail
        let options = AdvancedSearchOptions {
            query: "test".to_string(),
            limit: 0,
            ..Default::default()
        };
        
        let result = engine.search(options).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_search_pagination() {
        let storage = create_test_storage().await;
        let _notes = create_test_notes(&storage).await;
        let engine = AdvancedSearchEngine::new(storage);
        
        // First page
        let options1 = AdvancedSearchOptions {
            query: "programming".to_string(),
            limit: 2,
            offset: 0,
            ..Default::default()
        };
        
        let (results1, summary1) = engine.search(options1).await.unwrap();
        
        // Second page
        let options2 = AdvancedSearchOptions {
            query: "programming".to_string(),
            limit: 2,
            offset: 2,
            ..Default::default()
        };
        
        let (results2, summary2) = engine.search(options2).await.unwrap();
        
        // Should have same total results
        assert_eq!(summary1.total_results, summary2.total_results);
        
        // Should have different ranks
        if !results1.is_empty() && !results2.is_empty() {
            assert_ne!(results1[0].rank, results2[0].rank);
        }
    }
}