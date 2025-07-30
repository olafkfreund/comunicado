use crate::spell::MisspelledWord;
use std::collections::HashMap;

/// Spell check suggestion with context
#[derive(Debug, Clone)]
pub struct SpellSuggestion {
    pub original_word: String,
    pub suggestions: Vec<SuggestionOption>,
    pub position: usize,
    pub length: usize,
    pub context: String,
}

/// A suggestion option for a misspelled word
#[derive(Debug, Clone)]
pub struct SuggestionOption {
    pub word: String,
    pub confidence: f32,
    pub reason: SuggestionReason,
}

/// Reason why a suggestion was generated
#[derive(Debug, Clone)]
pub enum SuggestionReason {
    /// Dictionary suggestion
    Dictionary,
    /// Common typo pattern
    TypoPattern,
    /// Phonetic similarity
    Phonetic,
    /// Custom word suggestion
    Custom,
    /// Technical term
    Technical,
}

/// Manages spell check suggestions and improvements
pub struct SuggestionManager {
    common_typos: HashMap<String, String>,
    phonetic_replacements: HashMap<String, Vec<String>>,
    context_suggestions: HashMap<String, Vec<String>>,
}

impl SuggestionManager {
    /// Create new suggestion manager
    pub fn new() -> Self {
        let mut manager = Self {
            common_typos: HashMap::new(),
            phonetic_replacements: HashMap::new(),
            context_suggestions: HashMap::new(),
        };

        manager.load_common_typos();
        manager.load_phonetic_patterns();
        manager.load_context_suggestions();

        manager
    }

    /// Load common typo corrections
    fn load_common_typos(&mut self) {
        let typos = vec![
            // Common English typos
            ("recieve", "receive"),
            ("seperate", "separate"),
            ("definately", "definitely"),
            ("occured", "occurred"),
            ("occurence", "occurrence"),
            ("begining", "beginning"),
            ("independant", "independent"),
            ("accomodate", "accommodate"),
            ("embarass", "embarrass"),
            ("existance", "existence"),
            ("maintainance", "maintenance"),
            ("neccessary", "necessary"),
            ("noticable", "noticeable"),
            ("priviledge", "privilege"),
            ("rythm", "rhythm"),
            ("tommorrow", "tomorrow"),
            ("untill", "until"),
            ("wich", "which"),
            ("wierd", "weird"),
            ("youre", "you're"),
            ("its", "it's"),
            ("loose", "lose"),
            ("affect", "effect"),
            ("alot", "a lot"),
            ("arguement", "argument"),
            ("calender", "calendar"),
            ("cemetary", "cemetery"),
            ("changable", "changeable"),
            ("collegue", "colleague"),
            ("comittee", "committee"),
            ("concious", "conscious"),
            ("dilemna", "dilemma"),
            ("enviroment", "environment"),
            ("excercise", "exercise"),
            ("goverment", "government"),
            ("harrass", "harass"),
            ("lenght", "length"),
            ("libary", "library"),
            ("liesure", "leisure"),
            ("mispell", "misspell"),
            ("occassion", "occasion"),
            ("persue", "pursue"),
            ("playwrite", "playwright"),
            ("reccomend", "recommend"),
            ("refered", "referred"),
            ("relevent", "relevant"),
            ("resistence", "resistance"),
            ("shedule", "schedule"),
            ("succesful", "successful"),
            ("supercede", "supersede"),
            ("thier", "their"),
            ("truely", "truly"),
            ("writting", "writing"),
            // Technical typos
            ("javasript", "javascript"),
            ("pytohn", "python"),
            ("databse", "database"),
            ("functino", "function"),
            ("varialbe", "variable"),
            ("methdo", "method"),
            ("clsas", "class"),
            ("strign", "string"),
            ("lenght", "length"),
            ("heigth", "height"),
            ("widht", "width"),
            ("retrun", "return"),
            ("whiel", "while"),
            ("fro", "for"),
            ("fi", "if"),
            ("esle", "else"),
            ("swithc", "switch"),
            ("braek", "break"),
            ("contiue", "continue"),
            ("treu", "true"),
            ("flase", "false"),
            ("nul", "null"),
            ("udnefined", "undefined"),
            ("aync", "async"),
            ("awiat", "await"),
            ("promies", "promise"),
            ("obejct", "object"),
            ("aray", "array"),
            ("elemnt", "element"),
            ("atribute", "attribute"),
            ("propertie", "property"),
            ("inherits", "inherit"),
            ("extedns", "extends"),
            ("implemnts", "implements"),
            ("interfac", "interface"),
            ("abstrat", "abstract"),
            ("publci", "public"),
            ("privat", "private"),
            ("protectd", "protected"),
            ("statci", "static"),
            ("fianl", "final"),
            ("consructor", "constructor"),
            ("destruct", "destructor"),
            ("inhereted", "inherited"),
            ("polymorph", "polymorphism"),
            ("encapsula", "encapsulation"),
            ("abstractio", "abstraction"),
        ];

        for (typo, correction) in typos {
            self.common_typos
                .insert(typo.to_string(), correction.to_string());
        }
    }

    /// Load phonetic replacement patterns
    fn load_phonetic_patterns(&mut self) {
        let patterns = vec![
            ("ph", vec!["f"]),
            ("f", vec!["ph"]),
            ("c", vec!["k", "s"]),
            ("k", vec!["c"]),
            ("s", vec!["c", "z"]),
            ("z", vec!["s"]),
            ("ei", vec!["ie"]),
            ("ie", vec!["ei"]),
            ("ou", vec!["ow"]),
            ("ow", vec!["ou"]),
            ("tion", vec!["sion"]),
            ("sion", vec!["tion"]),
        ];

        for (pattern, replacements) in patterns {
            self.phonetic_replacements.insert(
                pattern.to_string(),
                replacements.into_iter().map(|s| s.to_string()).collect(),
            );
        }
    }

    /// Load context-based suggestions
    fn load_context_suggestions(&mut self) {
        let contexts = vec![
            ("email", vec!["e-mail", "mail", "message", "correspondence"]),
            ("website", vec!["web site", "site", "web page", "homepage"]),
            (
                "database",
                vec!["data base", "DB", "datastore", "repository"],
            ),
            ("username", vec!["user name", "login", "account", "user ID"]),
            (
                "password",
                vec!["passcode", "PIN", "credentials", "authentication"],
            ),
            ("internet", vec!["web", "online", "net", "worldwide web"]),
            ("computer", vec!["PC", "machine", "system", "device"]),
            ("software", vec!["program", "application", "app", "tool"]),
            (
                "hardware",
                vec!["equipment", "device", "component", "system"],
            ),
            (
                "network",
                vec!["connection", "link", "communication", "internet"],
            ),
        ];

        for (word, suggestions) in contexts {
            self.context_suggestions.insert(
                word.to_string(),
                suggestions.into_iter().map(|s| s.to_string()).collect(),
            );
        }
    }

    /// Generate enhanced suggestions for a misspelled word
    pub fn generate_suggestions(
        &self,
        misspelled: &MisspelledWord,
        context: &str,
    ) -> SpellSuggestion {
        let mut suggestions = Vec::new();

        // Add dictionary suggestions with high confidence
        for suggestion in &misspelled.suggestions {
            suggestions.push(SuggestionOption {
                word: suggestion.clone(),
                confidence: 0.9,
                reason: SuggestionReason::Dictionary,
            });
        }

        // Check for common typos
        if let Some(correction) = self.common_typos.get(&misspelled.word.to_lowercase()) {
            suggestions.insert(
                0,
                SuggestionOption {
                    word: correction.clone(),
                    confidence: 0.95,
                    reason: SuggestionReason::TypoPattern,
                },
            );
        }

        // Generate phonetic suggestions
        self.add_phonetic_suggestions(&misspelled.word, &mut suggestions);

        // Add context-based suggestions
        self.add_context_suggestions(&misspelled.word, context, &mut suggestions);

        // Sort by confidence and remove duplicates
        suggestions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        suggestions.dedup_by(|a, b| a.word == b.word);

        // Limit to reasonable number
        suggestions.truncate(8);

        SpellSuggestion {
            original_word: misspelled.word.clone(),
            suggestions,
            position: misspelled.position,
            length: misspelled.length,
            context: self.extract_context(context, misspelled.position, 20),
        }
    }

    /// Add phonetic-based suggestions
    fn add_phonetic_suggestions(&self, word: &str, suggestions: &mut Vec<SuggestionOption>) {
        let word_lower = word.to_lowercase();

        for (pattern, replacements) in &self.phonetic_replacements {
            if word_lower.contains(pattern) {
                for replacement in replacements {
                    let suggested_word = word_lower.replace(pattern, replacement);
                    if suggested_word != word_lower {
                        suggestions.push(SuggestionOption {
                            word: self.preserve_case(&suggested_word, word),
                            confidence: 0.6,
                            reason: SuggestionReason::Phonetic,
                        });
                    }
                }
            }
        }
    }

    /// Add context-based suggestions
    fn add_context_suggestions(
        &self,
        word: &str,
        context: &str,
        suggestions: &mut Vec<SuggestionOption>,
    ) {
        let word_lower = word.to_lowercase();
        let context_lower = context.to_lowercase();

        // Look for similar words in context suggestions
        for (context_word, context_suggestions) in &self.context_suggestions {
            if self.is_similar(&word_lower, context_word) || context_lower.contains(context_word) {
                for suggestion in context_suggestions {
                    if self.is_similar(&word_lower, suggestion) {
                        suggestions.push(SuggestionOption {
                            word: suggestion.clone(),
                            confidence: 0.7,
                            reason: SuggestionReason::Custom,
                        });
                    }
                }
            }
        }
    }

    /// Check if two words are similar (simple Levenshtein distance)
    fn is_similar(&self, word1: &str, word2: &str) -> bool {
        let distance = self.levenshtein_distance(word1, word2);
        let max_len = word1.len().max(word2.len());

        if max_len == 0 {
            return true;
        }

        (distance as f32 / max_len as f32) < 0.4
    }

    /// Calculate Levenshtein distance between two strings
    fn levenshtein_distance(&self, word1: &str, word2: &str) -> usize {
        let len1 = word1.chars().count();
        let len2 = word2.chars().count();

        if len1 == 0 {
            return len2;
        }
        if len2 == 0 {
            return len1;
        }

        let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

        for i in 0..=len1 {
            matrix[i][0] = i;
        }
        for j in 0..=len2 {
            matrix[0][j] = j;
        }

        let chars1: Vec<char> = word1.chars().collect();
        let chars2: Vec<char> = word2.chars().collect();

        for i in 1..=len1 {
            for j in 1..=len2 {
                let cost = if chars1[i - 1] == chars2[j - 1] { 0 } else { 1 };
                matrix[i][j] = [
                    matrix[i - 1][j] + 1,        // deletion
                    matrix[i][j - 1] + 1,        // insertion
                    matrix[i - 1][j - 1] + cost, // substitution
                ]
                .iter()
                .min()
                .unwrap()
                .clone();
            }
        }

        matrix[len1][len2]
    }

    /// Preserve the original case pattern in a suggestion
    fn preserve_case(&self, suggestion: &str, original: &str) -> String {
        let original_chars: Vec<char> = original.chars().collect();
        let suggestion_chars: Vec<char> = suggestion.chars().collect();

        let mut result = String::new();

        for (i, &ch) in suggestion_chars.iter().enumerate() {
            if i < original_chars.len() {
                if original_chars[i].is_uppercase() {
                    result.push(ch.to_uppercase().next().unwrap_or(ch));
                } else {
                    result.push(ch.to_lowercase().next().unwrap_or(ch));
                }
            } else {
                result.push(ch);
            }
        }

        result
    }

    /// Extract context around a word position
    fn extract_context(&self, text: &str, position: usize, radius: usize) -> String {
        let start = position.saturating_sub(radius);
        let end = (position + radius).min(text.len());

        text.chars().skip(start).take(end - start).collect()
    }
}

impl Default for SuggestionManager {
    fn default() -> Self {
        Self::new()
    }
}
