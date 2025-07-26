use std::path::PathBuf;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::fs;

/// Spell check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpellCheckConfig {
    /// Default language for spell checking
    pub default_language: String,
    
    /// Additional languages to load
    pub additional_languages: Vec<String>,
    
    /// Enable automatic language detection
    pub auto_detect_language: bool,
    
    /// Check spelling as user types
    pub check_as_you_type: bool,
    
    /// Underline style for misspelled words
    pub underline_style: UnderlineStyle,
    
    /// Maximum number of suggestions to show
    pub max_suggestions: usize,
    
    /// Ignore words in ALL CAPS
    pub ignore_all_caps: bool,
    
    /// Ignore words with numbers
    pub ignore_words_with_numbers: bool,
    
    /// Minimum word length to check
    pub min_word_length: usize,
    
    /// Custom dictionary words
    pub custom_words: Vec<String>,
    
    /// Technical terms dictionary
    pub technical_terms: Vec<String>,
}

/// Style for underlining misspelled words
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnderlineStyle {
    /// Red wavy underline
    RedWavy,
    /// Red solid underline
    RedSolid,
    /// Red dashed underline
    RedDashed,
    /// Custom color and style
    Custom { color: String, style: String },
}

impl Default for SpellCheckConfig {
    fn default() -> Self {
        Self {
            default_language: "en_US".to_string(),
            additional_languages: vec!["en_GB".to_string()],
            auto_detect_language: true,
            check_as_you_type: true,
            underline_style: UnderlineStyle::RedWavy,
            max_suggestions: 5,
            ignore_all_caps: true,
            ignore_words_with_numbers: true,
            min_word_length: 2,
            custom_words: vec![
                // Common technical terms
                "API".to_string(),
                "JSON".to_string(),
                "XML".to_string(),
                "HTTP".to_string(),
                "HTTPS".to_string(),
                "URL".to_string(),
                "CSS".to_string(),
                "HTML".to_string(),
                "JavaScript".to_string(),
                "TypeScript".to_string(),
                "Rust".to_string(),
                "Python".to_string(),
                "GitHub".to_string(),
                "OAuth".to_string(),
                "JWT".to_string(),
                "SQL".to_string(),
                "NoSQL".to_string(),
                "MongoDB".to_string(),
                "PostgreSQL".to_string(),
                "MySQL".to_string(),
                "Redis".to_string(),
                "Docker".to_string(),
                "Kubernetes".to_string(),
                "AWS".to_string(),
                "Azure".to_string(),
                "GCP".to_string(),
            ],
            technical_terms: vec![
                // Programming and tech terms
                "async".to_string(),
                "await".to_string(),
                "const".to_string(),
                "enum".to_string(),
                "impl".to_string(),
                "struct".to_string(),
                "trait".to_string(),
                "fn".to_string(),
                "pub".to_string(),
                "mut".to_string(),
                "ref".to_string(),
                "self".to_string(),
                "super".to_string(),
                "use".to_string(),
                "mod".to_string(),
                "crate".to_string(),
                "std".to_string(),
                "vec".to_string(),
                "str".to_string(),
                "bool".to_string(),
                "i32".to_string(),
                "i64".to_string(),
                "u32".to_string(),
                "u64".to_string(),
                "f32".to_string(),
                "f64".to_string(),
                "usize".to_string(),
                "isize".to_string(),
                "Option".to_string(),
                "Result".to_string(),
                "Some".to_string(),
                "None".to_string(),
                "Ok".to_string(),
                "Err".to_string(),
                "match".to_string(),
                "if".to_string(),
                "else".to_string(),
                "while".to_string(),
                "for".to_string(),
                "loop".to_string(),
                "break".to_string(),
                "continue".to_string(),
                "return".to_string(),
                "let".to_string(),
                "mut".to_string(),
                "static".to_string(),
                "extern".to_string(),
                "unsafe".to_string(),
                "where".to_string(),
                "impl".to_string(),
                "dyn".to_string(),
                "move".to_string(),
                "box".to_string(),
                "ref".to_string(),
                "macro".to_string(),
                "cfg".to_string(),
                "derive".to_string(),
                "debug".to_string(),
                "clone".to_string(),
                "copy".to_string(),
                "send".to_string(),
                "sync".to_string(),
                "unpin".to_string(),
                "sized".to_string(),
            ],
        }
    }
}

impl SpellCheckConfig {
    /// Load configuration from file
    pub async fn load() -> Result<Self> {
        let config_path = Self::config_file_path()?;
        
        if config_path.exists() {
            let content = fs::read_to_string(&config_path).await?;
            let config: SpellCheckConfig = toml::from_str(&content)?;
            Ok(config)
        } else {
            // Create default config and save it
            let config = Self::default();
            config.save().await?;
            Ok(config)
        }
    }

    /// Save configuration to file
    pub async fn save(&self) -> Result<()> {
        let config_path = Self::config_file_path()?;
        
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        
        let content = toml::to_string_pretty(self)?;
        fs::write(&config_path, content).await?;
        
        Ok(())
    }


    /// Get configuration file path
    fn config_file_path() -> Result<PathBuf> {
        if let Some(config_dir) = dirs::config_dir() {
            Ok(config_dir.join("comunicado").join("spell_check.toml"))
        } else {
            Ok(PathBuf::from(".").join("spell_check.toml"))
        }
    }

    /// Add custom word
    pub fn add_custom_word(&mut self, word: String) {
        if !self.custom_words.contains(&word) {
            self.custom_words.push(word);
        }
    }

    /// Remove custom word
    pub fn remove_custom_word(&mut self, word: &str) {
        self.custom_words.retain(|w| w != word);
    }

    /// Add technical term
    pub fn add_technical_term(&mut self, term: String) {
        if !self.technical_terms.contains(&term) {
            self.technical_terms.push(term);
        }
    }

    /// Remove technical term
    pub fn remove_technical_term(&mut self, term: &str) {
        self.technical_terms.retain(|t| t != term);
    }

    /// Get all custom words (including technical terms)
    pub fn all_custom_words(&self) -> Vec<String> {
        let mut words = self.custom_words.clone();
        words.extend(self.technical_terms.clone());
        words
    }

    /// Check if word should be ignored based on config rules
    pub fn should_ignore_word(&self, word: &str) -> bool {
        // Check minimum length
        if word.len() < self.min_word_length {
            return true;
        }

        // Check if all caps and should ignore
        if self.ignore_all_caps && word.chars().all(|c| c.is_uppercase() || !c.is_alphabetic()) {
            return true;
        }

        // Check if contains numbers and should ignore
        if self.ignore_words_with_numbers && word.chars().any(|c| c.is_numeric()) {
            return true;
        }

        false
    }

    /// Set language list
    pub fn set_languages(&mut self, languages: Vec<String>) {
        if let Some(first) = languages.first() {
            self.default_language = first.clone();
            self.additional_languages = languages[1..].to_vec();
        }
    }

    /// Get all configured languages
    pub fn all_languages(&self) -> Vec<String> {
        let mut languages = vec![self.default_language.clone()];
        languages.extend(self.additional_languages.clone());
        languages
    }
}