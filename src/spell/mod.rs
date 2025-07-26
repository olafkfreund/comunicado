use std::collections::{HashMap, HashSet};
use anyhow::Result;

pub mod dictionary;
pub mod config;
pub mod suggestion;

use dictionary::DictionaryManager;
use config::SpellCheckConfig;

/// Spell checking service for multi-language text validation
pub struct SpellChecker {
    dictionaries: HashMap<String, HashSet<String>>,
    dictionary_manager: DictionaryManager,
    config: SpellCheckConfig,
    current_language: String,
    custom_words: Vec<String>,
}

/// A word that failed spell checking
#[derive(Debug, Clone)]
pub struct MisspelledWord {
    pub word: String,
    pub position: usize,
    pub length: usize,
    pub suggestions: Vec<String>,
}

/// Spell check result for a text
#[derive(Debug, Clone)]
pub struct SpellCheckResult {
    pub misspelled_words: Vec<MisspelledWord>,
    pub word_count: usize,
    pub error_count: usize,
}

impl SpellChecker {
    /// Create a new spell checker with default configuration
    pub fn new() -> Result<Self> {
        let config = SpellCheckConfig::default();
        let dictionary_manager = DictionaryManager::new()?;
        
        Ok(Self {
            dictionaries: HashMap::new(),
            dictionary_manager,
            config,
            current_language: "en_US".to_string(),
            custom_words: Vec::new(),
        })
    }

    /// Create spell checker with custom configuration
    pub fn with_config(config: SpellCheckConfig) -> Result<Self> {
        let dictionary_manager = DictionaryManager::new()?;
        let current_language = config.default_language.clone();
        
        Ok(Self {
            dictionaries: HashMap::new(),
            dictionary_manager,
            config,
            current_language,
            custom_words: Vec::new(),
        })
    }

    /// Load a dictionary for the specified language
    pub async fn load_dictionary(&mut self, language: &str) -> Result<()> {
        if self.dictionaries.contains_key(language) {
            return Ok(()); // Already loaded
        }

        let dictionary_path = self.dictionary_manager.get_dictionary_path(language).await?;
        
        // Load dictionary from .dic file
        let dictionary_content = tokio::fs::read_to_string(&dictionary_path.dic).await?;
        let mut word_set = HashSet::new();
        
        // Parse dictionary file (skip first line which contains word count)
        for (i, line) in dictionary_content.lines().enumerate() {
            if i == 0 {
                continue; // Skip word count line
            }
            
            let word = line.split('/').next().unwrap_or(line).trim().to_lowercase();
            if !word.is_empty() {
                word_set.insert(word);
            }
        }
        
        self.dictionaries.insert(language.to_string(), word_set);
        tracing::info!("Loaded spell check dictionary for language: {} ({} words)", language, self.dictionaries[language].len());
        
        Ok(())
    }

    /// Set the current active language
    pub async fn set_language(&mut self, language: &str) -> Result<()> {
        if !self.dictionaries.contains_key(language) {
            self.load_dictionary(language).await?;
        }
        
        self.current_language = language.to_string();
        Ok(())
    }

    /// Get the current active language
    pub fn current_language(&self) -> &str {
        &self.current_language
    }

    /// Get list of available languages
    pub fn available_languages(&self) -> Vec<String> {
        self.dictionary_manager.available_languages()
    }

    /// Add custom words to the spell checker
    pub fn add_custom_words(&mut self, words: Vec<String>) {
        self.custom_words.extend(words);
    }

    /// Add a single custom word
    pub fn add_custom_word(&mut self, word: String) {
        if !self.custom_words.contains(&word) {
            self.custom_words.push(word);
        }
    }

    /// Check if a word is spelled correctly
    pub fn check_word(&self, word: &str) -> bool {
        // Check custom words first
        if self.custom_words.iter().any(|w| w.eq_ignore_ascii_case(word)) {
            return true;
        }

        // Check against current dictionary
        if let Some(dictionary) = self.dictionaries.get(&self.current_language) {
            dictionary.contains(&word.to_lowercase())
        } else {
            true // If no dictionary loaded, assume correct
        }
    }

    /// Get spelling suggestions for a word
    pub fn suggest_word(&self, word: &str) -> Vec<String> {
        if let Some(dictionary) = self.dictionaries.get(&self.current_language) {
            // Simple suggestion algorithm based on edit distance
            let word_lower = word.to_lowercase();
            let mut suggestions = Vec::new();
            
            // Find words with similar length and characters
            for dict_word in dictionary.iter() {
                if self.is_similar_word(&word_lower, dict_word) {
                    suggestions.push(dict_word.clone());
                    if suggestions.len() >= 5 {
                        break;
                    }
                }
            }
            
            suggestions
        } else {
            Vec::new()
        }
    }

    /// Check spelling of entire text
    pub fn check_text(&self, text: &str) -> SpellCheckResult {
        let mut misspelled_words = Vec::new();
        let mut word_count = 0;
        // Split text into words while preserving positions
        for word_match in regex::Regex::new(r"\b\w+\b").unwrap().find_iter(text) {
            let word = word_match.as_str();
            word_count += 1;
            let current_pos = word_match.start();

            if !self.check_word(word) {
                let suggestions = self.suggest_word(word);
                misspelled_words.push(MisspelledWord {
                    word: word.to_string(),
                    position: current_pos,
                    length: word.len(),
                    suggestions,
                });
            }
        }

        SpellCheckResult {
            error_count: misspelled_words.len(),
            misspelled_words,
            word_count,
        }
    }

    /// Check spelling with language detection
    pub async fn check_text_with_detection(&mut self, text: &str) -> Result<SpellCheckResult> {
        // Try to detect language from text
        let detected_language = self.detect_language(text).await?;
        
        if detected_language != self.current_language {
            self.set_language(&detected_language).await?;
        }

        Ok(self.check_text(text))
    }

    /// Simple language detection based on common words
    async fn detect_language(&mut self, text: &str) -> Result<String> {
        let words: Vec<&str> = text.split_whitespace().take(50).collect();
        let mut language_scores = HashMap::new();

        // Test against available languages
        for language in self.available_languages() {
            if !self.dictionaries.contains_key(&language) {
                if let Err(_) = self.load_dictionary(&language).await {
                    continue; // Skip if can't load dictionary
                }
            }

            let mut correct_words = 0;
            for word in &words {
                if let Some(dictionary) = self.dictionaries.get(&language) {
                    if dictionary.contains(&word.to_lowercase()) {
                        correct_words += 1;
                    }
                }
            }

            let score = if words.is_empty() { 0.0 } else { correct_words as f32 / words.len() as f32 };
            language_scores.insert(language, score);
        }

        // Return language with highest score, or default if none score well
        let best_language = language_scores
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(lang, score)| (lang.clone(), *score))
            .unwrap_or((self.config.default_language.clone(), 0.0));

        if best_language.1 > 0.3 { // At least 30% of words recognized
            Ok(best_language.0)
        } else {
            Ok(self.config.default_language.clone())
        }
    }

    /// Save current configuration
    pub async fn save_config(&self) -> Result<()> {
        self.config.save().await
    }

    /// Update configuration
    pub fn update_config(&mut self, config: SpellCheckConfig) {
        self.config = config;
    }
    
    /// Helper method to check word similarity for suggestions
    fn is_similar_word(&self, word1: &str, word2: &str) -> bool {
        let len_diff = (word1.len() as i32 - word2.len() as i32).abs();
        if len_diff > 2 {
            return false;
        }
        
        // Simple character overlap check
        let chars1: std::collections::HashSet<char> = word1.chars().collect();
        let chars2: std::collections::HashSet<char> = word2.chars().collect();
        let intersection = chars1.intersection(&chars2).count();
        let union = chars1.union(&chars2).count();
        
        if union == 0 {
            return false;
        }
        
        let similarity = intersection as f32 / union as f32;
        similarity > 0.6
    }
}

impl Default for SpellChecker {
    fn default() -> Self {
        Self::new().expect("Failed to create default SpellChecker")
    }
}