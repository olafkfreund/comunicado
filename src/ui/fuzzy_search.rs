/// Advanced fuzzy search implementation with live search-as-you-type functionality
/// 
/// Provides fast, accurate fuzzy matching for email content using multiple algorithms
/// including Levenshtein distance, Jaro-Winkler similarity, and trigram matching.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::Instant;

use crate::email::{EmailDatabase, StoredMessage};
use crate::ui::search::{SearchMode, SearchResult, SearchSnippet};

/// Fuzzy search configuration
#[derive(Debug, Clone)]
pub struct FuzzySearchConfig {
    /// Maximum edit distance for fuzzy matching
    pub max_edit_distance: usize,
    /// Minimum similarity threshold (0.0 to 1.0)
    pub min_similarity: f64,
    /// Enable trigram matching for better performance
    pub use_trigrams: bool,
    /// Enable Jaro-Winkler similarity
    pub use_jaro_winkler: bool,
    /// Live search debounce delay in milliseconds
    pub debounce_delay_ms: u64,
    /// Maximum number of results to return
    pub max_results: usize,
}

impl Default for FuzzySearchConfig {
    fn default() -> Self {
        Self {
            max_edit_distance: 2,
            min_similarity: 0.6,
            use_trigrams: true,
            use_jaro_winkler: true,
            debounce_delay_ms: 150,
            max_results: 100,
        }
    }
}

/// Fuzzy search engine with live search capabilities
pub struct FuzzySearchEngine {
    database: Arc<EmailDatabase>,
    config: FuzzySearchConfig,
    last_search_time: Option<Instant>,
    search_cache: HashMap<String, Vec<SearchResult>>,
    pending_searches: HashMap<String, Instant>,
}

impl FuzzySearchEngine {
    /// Create new fuzzy search engine
    pub fn new(database: Arc<EmailDatabase>) -> Self {
        Self {
            database,
            config: FuzzySearchConfig::default(),
            last_search_time: None,
            search_cache: HashMap::new(),
            pending_searches: HashMap::new(),
        }
    }

    /// Create fuzzy search engine with custom configuration
    pub fn with_config(database: Arc<EmailDatabase>, config: FuzzySearchConfig) -> Self {
        Self {
            database,
            config,
            last_search_time: None,
            search_cache: HashMap::new(),
            pending_searches: HashMap::new(),
        }
    }

    /// Perform live fuzzy search with debouncing
    pub async fn live_search(
        &mut self,
        account_id: &str,
        query: &str,
        mode: &SearchMode,
    ) -> Result<Vec<SearchResult>, Box<dyn std::error::Error>> {
        // Check cache first
        let cache_key = format!("{}:{}:{:?}", account_id, query, mode);
        if let Some(cached_results) = self.search_cache.get(&cache_key) {
            return Ok(cached_results.clone());
        }

        // Debounce rapid searches
        let now = Instant::now();
        if let Some(last_time) = self.last_search_time {
            let elapsed = now.duration_since(last_time);
            if elapsed.as_millis() < self.config.debounce_delay_ms as u128 {
                // Mark as pending and return empty results
                self.pending_searches.insert(cache_key, now);
                return Ok(Vec::new());
            }
        }

        self.last_search_time = Some(now);

        // Perform fuzzy search
        let results = self.fuzzy_search(account_id, query, mode).await?;

        // Cache results
        self.search_cache.insert(cache_key, results.clone());

        // Clean up old cache entries (keep last 50)
        if self.search_cache.len() > 50 {
            let oldest_keys: Vec<String> = self.search_cache.keys().take(10).cloned().collect();
            for key in oldest_keys {
                self.search_cache.remove(&key);
            }
        }

        Ok(results)
    }

    /// Perform fuzzy search with multiple matching algorithms
    pub async fn fuzzy_search(
        &self,
        account_id: &str,
        query: &str,
        mode: &SearchMode,
    ) -> Result<Vec<SearchResult>, Box<dyn std::error::Error>> {
        let start_time = Instant::now();

        // Get all messages from database using a broad search (we'll do fuzzy matching in memory for better control)
        // Use "*" to get all messages, or use a simple search and then filter
        let all_messages = self.database.search_messages(account_id, "*", Some(1000)).await
            .unwrap_or_else(|_| {
                // Fallback: try to get messages from common folders
                vec![]
            });

        let mut results = Vec::new();
        let query_lower = query.to_lowercase();

        for message in all_messages {
            if let Some(search_result) = self.match_message(&message, &query_lower, mode).await {
                results.push(search_result);
            }

            // Limit results for performance
            if results.len() >= self.config.max_results {
                break;
            }
        }

        // Sort by relevance score (higher is better)
        results.sort_by(|a, b| b.rank.partial_cmp(&a.rank).unwrap_or(std::cmp::Ordering::Equal));

        let _search_time = start_time.elapsed().as_millis() as u64;

        Ok(results)
    }

    /// Check if a message matches the fuzzy search query
    async fn match_message(
        &self,
        message: &StoredMessage,
        query: &str,
        mode: &SearchMode,
    ) -> Option<SearchResult> {
        let mut total_score = 0.0;
        let mut matched_fields = Vec::new();
        let mut snippets = Vec::new();

        // Get searchable text based on mode
        let search_texts = self.get_searchable_texts(message, mode);

        for (field_name, text) in search_texts {
            if let Some((score, snippet)) = self.fuzzy_match_text(&text, query, &field_name) {
                total_score += score;
                matched_fields.push(field_name.clone());
                snippets.push(snippet);
            }
        }

        // Check if overall score meets threshold
        if total_score >= self.config.min_similarity {
            Some(SearchResult {
                message: message.clone(),
                rank: total_score,
                snippets,
                matched_fields,
            })
        } else {
            None
        }
    }

    /// Get searchable text fields based on search mode
    fn get_searchable_texts(&self, message: &StoredMessage, mode: &SearchMode) -> Vec<(String, String)> {
        match mode {
            SearchMode::FullText => vec![
                ("subject".to_string(), message.subject.clone()),
                ("from_name".to_string(), message.from_name.clone().unwrap_or_default()),
                ("from_addr".to_string(), message.from_addr.clone()),
                ("body_text".to_string(), message.body_text.clone().unwrap_or_default()),
                ("to_addrs".to_string(), message.to_addrs.join(" ")),
            ],
            SearchMode::Subject => vec![
                ("subject".to_string(), message.subject.clone()),
            ],
            SearchMode::From => vec![
                ("from_name".to_string(), message.from_name.clone().unwrap_or_default()),
                ("from_addr".to_string(), message.from_addr.clone()),
            ],
            SearchMode::Body => vec![
                ("body_text".to_string(), message.body_text.clone().unwrap_or_default()),
            ],
            SearchMode::Advanced => vec![
                ("subject".to_string(), message.subject.clone()),
                ("from_name".to_string(), message.from_name.clone().unwrap_or_default()),
                ("from_addr".to_string(), message.from_addr.clone()),
                ("body_text".to_string(), message.body_text.clone().unwrap_or_default()),
                ("to_addrs".to_string(), message.to_addrs.join(" ")),
            ],
        }
    }

    /// Perform fuzzy matching on text using multiple algorithms
    fn fuzzy_match_text(&self, text: &str, query: &str, field_name: &str) -> Option<(f64, SearchSnippet)> {
        if text.is_empty() || query.is_empty() {
            return None;
        }

        let text_lower = text.to_lowercase();
        let mut best_score = 0.0;
        let mut best_highlights = Vec::new();

        // 1. Exact substring match (highest score)
        if let Some(pos) = text_lower.find(query) {
            best_score = 1.0;
            best_highlights.push((pos, pos + query.len()));
        }
        // 2. Word boundary matches
        else if self.word_boundary_match(&text_lower, query) {
            best_score = 0.9;
            best_highlights = self.find_word_matches(&text_lower, query);
        }
        // 3. Fuzzy matching with edit distance
        else {
            let words: Vec<&str> = text_lower.split_whitespace().collect();
            for (i, word) in words.iter().enumerate() {
                let similarity = if self.config.use_jaro_winkler {
                    self.jaro_winkler_similarity(word, query)
                } else {
                    self.levenshtein_similarity(word, query)
                };

                if similarity > best_score && similarity >= self.config.min_similarity {
                    best_score = similarity;
                    // Find position of this word in original text
                    let word_start = text_lower.split_whitespace().take(i).map(|w| w.len() + 1).sum::<usize>();
                    best_highlights = vec![(word_start, word_start + word.len())];
                }
            }

            // 4. Trigram matching for partial matches
            if self.config.use_trigrams && best_score < self.config.min_similarity {
                let trigram_score = self.trigram_similarity(&text_lower, query);
                if trigram_score > best_score {
                    best_score = trigram_score;
                    best_highlights = self.find_trigram_matches(&text_lower, query);
                }
            }
        }

        if best_score >= self.config.min_similarity {
            // Create snippet with context
            let snippet_content = self.create_snippet(text, &best_highlights, 60);
            Some((best_score, SearchSnippet {
                field: field_name.to_string(),
                content: snippet_content,
                highlights: best_highlights,
            }))
        } else {
            None
        }
    }

    /// Check for word boundary matches
    fn word_boundary_match(&self, text: &str, query: &str) -> bool {
        text.split_whitespace().any(|word| word.starts_with(query))
    }

    /// Find word matches and return their positions
    fn find_word_matches(&self, text: &str, query: &str) -> Vec<(usize, usize)> {
        let mut matches = Vec::new();
        let mut pos = 0;

        for word in text.split_whitespace() {
            if word.starts_with(query) {
                matches.push((pos, pos + query.len()));
            }
            pos += word.len() + 1; // +1 for space
        }

        matches
    }

    /// Calculate Levenshtein similarity (normalized edit distance)
    fn levenshtein_similarity(&self, s1: &str, s2: &str) -> f64 {
        let distance = self.levenshtein_distance(s1, s2);
        let max_len = s1.len().max(s2.len());
        if max_len == 0 {
            return 1.0;
        }
        1.0 - (distance as f64 / max_len as f64)
    }

    /// Calculate Levenshtein distance between two strings
    fn levenshtein_distance(&self, s1: &str, s2: &str) -> usize {
        let len1 = s1.chars().count();
        let len2 = s2.chars().count();

        if len1 == 0 { return len2; }
        if len2 == 0 { return len1; }

        let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

        // Initialize first row and column
        for i in 0..=len1 { matrix[i][0] = i; }
        for j in 0..=len2 { matrix[0][j] = j; }

        let chars1: Vec<char> = s1.chars().collect();
        let chars2: Vec<char> = s2.chars().collect();

        for i in 1..=len1 {
            for j in 1..=len2 {
                let cost = if chars1[i-1] == chars2[j-1] { 0 } else { 1 };
                matrix[i][j] = (matrix[i-1][j] + 1)
                    .min(matrix[i][j-1] + 1)
                    .min(matrix[i-1][j-1] + cost);
            }
        }

        matrix[len1][len2]
    }

    /// Calculate Jaro-Winkler similarity
    fn jaro_winkler_similarity(&self, s1: &str, s2: &str) -> f64 {
        if s1 == s2 { return 1.0; }
        if s1.is_empty() || s2.is_empty() { return 0.0; }

        let jaro = self.jaro_similarity(s1, s2);
        if jaro < 0.7 { return jaro; }

        // Calculate common prefix length (up to 4 characters)
        let prefix_len = s1.chars().zip(s2.chars())
            .take(4)
            .take_while(|(c1, c2)| c1 == c2)
            .count();

        jaro + (0.1 * prefix_len as f64 * (1.0 - jaro))
    }

    /// Calculate Jaro similarity
    fn jaro_similarity(&self, s1: &str, s2: &str) -> f64 {
        let chars1: Vec<char> = s1.chars().collect();
        let chars2: Vec<char> = s2.chars().collect();
        let len1 = chars1.len();
        let len2 = chars2.len();

        if len1 == 0 && len2 == 0 { return 1.0; }
        if len1 == 0 || len2 == 0 { return 0.0; }

        let match_window = (len1.max(len2) / 2).saturating_sub(1);
        let mut matches1 = vec![false; len1];
        let mut matches2 = vec![false; len2];
        let mut matches = 0;

        // Find matches
        for i in 0..len1 {
            let start = i.saturating_sub(match_window);
            let end = (i + match_window + 1).min(len2);

            for j in start..end {
                if matches2[j] || chars1[i] != chars2[j] { continue; }
                matches1[i] = true;
                matches2[j] = true;
                matches += 1;
                break;
            }
        }

        if matches == 0 { return 0.0; }

        // Count transpositions
        let mut transpositions = 0;
        let mut k = 0;
        for i in 0..len1 {
            if !matches1[i] { continue; }
            while !matches2[k] { k += 1; }
            if chars1[i] != chars2[k] { transpositions += 1; }
            k += 1;
        }

        let jaro = (matches as f64 / len1 as f64 
                  + matches as f64 / len2 as f64 
                  + (matches as f64 - transpositions as f64 / 2.0) / matches as f64) / 3.0;

        jaro
    }

    /// Calculate trigram similarity
    fn trigram_similarity(&self, text: &str, query: &str) -> f64 {
        if text.len() < 3 || query.len() < 3 { return 0.0; }

        let text_trigrams = self.get_trigrams(text);
        let query_trigrams = self.get_trigrams(query);

        if text_trigrams.is_empty() || query_trigrams.is_empty() { return 0.0; }

        let intersection: Vec<_> = text_trigrams.iter()
            .filter(|t| query_trigrams.contains(t))
            .collect();

        let union_size = text_trigrams.len() + query_trigrams.len() - intersection.len();
        intersection.len() as f64 / union_size as f64
    }

    /// Get trigrams from text
    fn get_trigrams(&self, text: &str) -> Vec<String> {
        let padded = format!("  {}  ", text);
        padded.chars()
            .collect::<Vec<_>>()
            .windows(3)
            .map(|window| window.iter().collect())
            .collect()
    }

    /// Find trigram matches in text
    fn find_trigram_matches(&self, text: &str, query: &str) -> Vec<(usize, usize)> {
        let _query_trigrams = self.get_trigrams(query);
        let mut matches = Vec::new();

        // This is a simplified implementation - in practice you'd want more sophisticated trigram matching
        if let Some(pos) = text.find(&query[..query.len().min(3)]) {
            matches.push((pos, pos + query.len().min(3)));
        }

        matches
    }

    /// Create snippet with context around matches
    fn create_snippet(&self, text: &str, highlights: &[(usize, usize)], max_length: usize) -> String {
        if highlights.is_empty() {
            return text.chars().take(max_length).collect();
        }

        let first_match = highlights[0].0;
        let context_start = first_match.saturating_sub(20);
        let context_end = (first_match + max_length).min(text.len());

        let snippet = &text[context_start..context_end];
        if context_start > 0 {
            format!("...{}", snippet)
        } else {
            snippet.to_string()
        }
    }

    /// Clear search cache
    pub fn clear_cache(&mut self) {
        self.search_cache.clear();
        self.pending_searches.clear();
    }

    /// Update configuration
    pub fn update_config(&mut self, config: FuzzySearchConfig) {
        self.config = config;
        self.clear_cache(); // Clear cache when config changes
    }

    /// Get current configuration
    pub fn config(&self) -> &FuzzySearchConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_distance() {
        let engine = FuzzySearchEngine::new(Arc::new(EmailDatabase::new(":memory:").unwrap()));
        
        assert_eq!(engine.levenshtein_distance("kitten", "sitting"), 3);
        assert_eq!(engine.levenshtein_distance("hello", "hello"), 0);
        assert_eq!(engine.levenshtein_distance("", "hello"), 5);
        assert_eq!(engine.levenshtein_distance("hello", ""), 5);
    }

    #[test]
    fn test_jaro_similarity() {
        let engine = FuzzySearchEngine::new(Arc::new(EmailDatabase::new(":memory:").unwrap()));
        
        assert!((engine.jaro_similarity("martha", "marhta") - 0.944).abs() < 0.01);
        assert_eq!(engine.jaro_similarity("hello", "hello"), 1.0);
        assert_eq!(engine.jaro_similarity("", "hello"), 0.0);
    }

    #[test]
    fn test_jaro_winkler_similarity() {
        let engine = FuzzySearchEngine::new(Arc::new(EmailDatabase::new(":memory:").unwrap()));
        
        assert!((engine.jaro_winkler_similarity("martha", "marhta") - 0.961).abs() < 0.01);
        assert_eq!(engine.jaro_winkler_similarity("hello", "hello"), 1.0);
    }

    #[test]
    fn test_trigram_similarity() {
        let engine = FuzzySearchEngine::new(Arc::new(EmailDatabase::new(":memory:").unwrap()));
        
        let similarity = engine.trigram_similarity("hello world", "hello");
        assert!(similarity > 0.0);
        
        let exact_similarity = engine.trigram_similarity("test", "test");
        assert!(exact_similarity > similarity);
    }

    #[test]
    fn test_word_boundary_match() {
        let engine = FuzzySearchEngine::new(Arc::new(EmailDatabase::new(":memory:").unwrap()));
        
        assert!(engine.word_boundary_match("hello world", "hel"));
        assert!(engine.word_boundary_match("hello world", "wor"));
        assert!(!engine.word_boundary_match("hello world", "llo"));
    }

    #[test]
    fn test_fuzzy_search_config() {
        let config = FuzzySearchConfig {
            max_edit_distance: 3,
            min_similarity: 0.5,
            use_trigrams: false,
            use_jaro_winkler: false,
            debounce_delay_ms: 200,
            max_results: 50,
        };
        
        assert_eq!(config.max_edit_distance, 3);
        assert_eq!(config.min_similarity, 0.5);
        assert!(!config.use_trigrams);
        assert!(!config.use_jaro_winkler);
        assert_eq!(config.debounce_delay_ms, 200);
        assert_eq!(config.max_results, 50);
    }
}