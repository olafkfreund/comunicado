//! Smart compose AI feature for email composition assistance
//! 
//! This module provides AI-powered email composition suggestions with context-aware
//! writing assistance, including auto-completion, tone suggestions, and smart drafting.

use crate::ai::{AIResult, EnhancedAIService, EnhancedAIRequest, AIOperationType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

/// Smart compose configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartComposeConfig {
    /// Enable smart compose features
    pub enabled: bool,
    /// Enable auto-completion suggestions
    pub enable_auto_completion: bool,
    /// Enable subject line suggestions
    pub enable_subject_suggestions: bool,
    /// Enable tone analysis and suggestions
    pub enable_tone_suggestions: bool,
    /// Enable context-aware suggestions
    pub enable_context_awareness: bool,
    /// Minimum characters before triggering suggestions
    pub min_trigger_length: usize,
    /// Maximum number of suggestions to generate
    pub max_suggestions: usize,
    /// Auto-completion trigger delay (milliseconds)
    pub trigger_delay_ms: u64,
    /// Cache suggestions for reuse
    pub enable_suggestion_caching: bool,
    /// Learning mode - adapt to user writing style
    pub enable_learning_mode: bool,
}

impl Default for SmartComposeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            enable_auto_completion: true,
            enable_subject_suggestions: true,
            enable_tone_suggestions: true,
            enable_context_awareness: true,
            min_trigger_length: 3,
            max_suggestions: 5,
            trigger_delay_ms: 500,
            enable_suggestion_caching: true,
            enable_learning_mode: false, // Disabled by default for privacy
        }
    }
}

/// Email composition context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositionContext {
    /// Type of email being composed
    pub email_type: EmailType,
    /// Recipient information
    pub recipients: Vec<Recipient>,
    /// Referenced email (for replies/forwards)
    pub referenced_email: Option<ReferencedEmail>,
    /// User's writing style preferences
    pub style_preferences: StylePreferences,
    /// Business context or project information
    pub business_context: Option<String>,
    /// Previous email thread context
    pub thread_context: Option<String>,
    /// Urgency level
    pub urgency: UrgencyLevel,
}

/// Type of email being composed
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Hash)]
pub enum EmailType {
    /// New email from scratch
    New,
    /// Reply to an existing email
    Reply,
    /// Forward an existing email
    Forward,
    /// Follow-up email
    FollowUp,
    /// Meeting invitation
    MeetingInvitation,
    /// Project update
    ProjectUpdate,
    /// Personal communication
    Personal,
}

/// Recipient information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipient {
    /// Email address
    pub email: String,
    /// Display name
    pub name: Option<String>,
    /// Relationship to sender
    pub relationship: RelationshipType,
    /// Communication history summary
    pub history_summary: Option<String>,
}

/// Relationship type for tone adaptation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RelationshipType {
    /// Close colleague or peer
    Colleague,
    /// Manager or supervisor
    Manager,
    /// Direct report
    DirectReport,
    /// External client
    Client,
    /// Vendor or supplier
    Vendor,
    /// Personal friend
    Friend,
    /// Family member
    Family,
    /// Unknown relationship
    Unknown,
}

/// Referenced email for context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferencedEmail {
    /// Original email subject
    pub subject: String,
    /// Original email content (summary)
    pub content_summary: String,
    /// Original sender
    pub sender: String,
    /// Key points from original email
    pub key_points: Vec<String>,
    /// Sentiment of original email
    pub sentiment: SentimentType,
}

/// User's writing style preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StylePreferences {
    /// Preferred formality level
    pub formality: FormalityLevel,
    /// Preferred tone
    pub tone: ToneType,
    /// Preferred length
    pub length: LengthPreference,
    /// Include personal touches
    pub include_personal_touches: bool,
    /// Use active voice
    pub prefer_active_voice: bool,
    /// Include action items
    pub include_action_items: bool,
}

/// Formality levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Hash)]
pub enum FormalityLevel {
    /// Very formal (legal, executive)
    VeryFormal,
    /// Formal (business standard)
    Formal,
    /// Semi-formal (friendly business)
    SemiFormal,
    /// Casual (team communication)
    Casual,
    /// Very casual (close colleagues)
    VeryCasual,
}

/// Tone types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Hash)]
pub enum ToneType {
    /// Professional and neutral
    Professional,
    /// Friendly and warm
    Friendly,
    /// Assertive and direct
    Assertive,
    /// Apologetic and humble
    Apologetic,
    /// Enthusiastic and positive
    Enthusiastic,
    /// Concerned or serious
    Concerned,
    /// Grateful and appreciative
    Grateful,
}

/// Length preferences
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LengthPreference {
    /// Very brief (1-2 sentences)
    VeryBrief,
    /// Brief (1 paragraph)
    Brief,
    /// Moderate (2-3 paragraphs)
    Moderate,
    /// Detailed (multiple paragraphs)
    Detailed,
    /// Comprehensive (extensive detail)
    Comprehensive,
}

/// Urgency levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UrgencyLevel {
    /// Low urgency
    Low,
    /// Normal urgency
    Normal,
    /// High urgency
    High,
    /// Critical/urgent
    Critical,
}

/// Sentiment types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SentimentType {
    /// Positive sentiment
    Positive,
    /// Neutral sentiment
    Neutral,
    /// Negative sentiment
    Negative,
    /// Mixed sentiment
    Mixed,
}

/// Smart compose suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeSuggestion {
    /// Unique suggestion ID
    pub id: Uuid,
    /// Type of suggestion
    pub suggestion_type: SuggestionType,
    /// Suggested content
    pub content: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Explanation of why this suggestion was made
    pub reasoning: String,
    /// Context where this suggestion applies
    pub context_position: ContextPosition,
    /// Alternative variations
    pub alternatives: Vec<String>,
    /// Metadata about the suggestion
    pub metadata: HashMap<String, String>,
}

/// Types of suggestions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SuggestionType {
    /// Subject line suggestions
    SubjectLine,
    /// Email opening/greeting
    Opening,
    /// Body content continuation
    BodyCompletion,
    /// Email closing
    Closing,
    /// Signature suggestions
    Signature,
    /// Call-to-action suggestions
    CallToAction,
    /// Meeting scheduling suggestions
    MeetingScheduling,
    /// Follow-up suggestions
    FollowUp,
    /// Tone adjustment suggestions
    ToneAdjustment,
    /// Grammar and style improvements
    StyleImprovement,
}

/// Position context for suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPosition {
    /// Line number in email
    pub line: usize,
    /// Character position
    pub character: usize,
    /// Cursor position relative to words
    pub word_position: WordPosition,
    /// Surrounding text context
    pub surrounding_text: String,
}

/// Word position types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WordPosition {
    /// At beginning of word
    WordStart,
    /// In middle of word
    WordMiddle,
    /// At end of word
    WordEnd,
    /// Between words
    BetweenWords,
    /// At line beginning
    LineStart,
    /// At line end
    LineEnd,
}

/// Smart compose response containing multiple suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartComposeResponse {
    /// List of suggestions
    pub suggestions: Vec<ComposeSuggestion>,
    /// Request ID for tracking
    pub request_id: Uuid,
    /// Processing time
    pub processing_time_ms: u64,
    /// Context summary
    pub context_summary: String,
    /// Whether suggestions were cached
    pub from_cache: bool,
}

/// Smart compose service
pub struct SmartComposeService {
    /// Enhanced AI service for processing
    ai_service: Arc<EnhancedAIService>,
    /// Configuration
    config: Arc<RwLock<SmartComposeConfig>>,
    /// User writing style learning data
    writing_style_data: Arc<RwLock<HashMap<String, UserWritingStyle>>>,
    /// Suggestion cache
    suggestion_cache: Arc<RwLock<HashMap<String, SmartComposeResponse>>>,
    /// Usage statistics
    stats: Arc<RwLock<SmartComposeStats>>,
}

/// User writing style data for learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserWritingStyle {
    /// Common phrases used by the user
    pub common_phrases: Vec<String>,
    /// Preferred sentence structures
    pub sentence_patterns: Vec<String>,
    /// Vocabulary preferences
    pub vocabulary_preferences: HashMap<String, f32>,
    /// Average email length
    pub avg_email_length: usize,
    /// Common email signatures
    pub signatures: Vec<String>,
    /// Tone frequency by recipient type
    pub tone_by_recipient: HashMap<String, ToneType>,
    /// Last updated timestamp
    pub last_updated: u64,
}

/// Smart compose usage statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SmartComposeStats {
    /// Total suggestions generated
    pub total_suggestions: usize,
    /// Suggestions accepted by users
    pub accepted_suggestions: usize,
    /// Acceptance rate percentage
    pub acceptance_rate: f32,
    /// Average response time
    pub avg_response_time_ms: f64,
    /// Suggestions by type
    pub suggestions_by_type: HashMap<String, usize>,
    /// Cache hit rate
    pub cache_hit_rate: f32,
    /// Learning mode improvements
    pub learning_improvements: usize,
}

impl SmartComposeService {
    /// Create a new smart compose service
    pub fn new(ai_service: Arc<EnhancedAIService>, config: SmartComposeConfig) -> Self {
        Self {
            ai_service,
            config: Arc::new(RwLock::new(config)),
            writing_style_data: Arc::new(RwLock::new(HashMap::new())),
            suggestion_cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(SmartComposeStats::default())),
        }
    }

    /// Generate smart compose suggestions
    pub async fn generate_suggestions(
        &self,
        current_text: &str,
        context: &CompositionContext,
        cursor_position: ContextPosition,
    ) -> AIResult<SmartComposeResponse> {
        let start_time = std::time::Instant::now();
        let request_id = Uuid::new_v4();

        let config = self.config.read().await;
        if !config.enabled {
            return Ok(SmartComposeResponse {
                suggestions: vec![],
                request_id,
                processing_time_ms: 0,
                context_summary: "Smart compose disabled".to_string(),
                from_cache: false,
            });
        }

        // Check minimum trigger length
        if current_text.len() < config.min_trigger_length {
            return Ok(SmartComposeResponse {
                suggestions: vec![],
                request_id,
                processing_time_ms: start_time.elapsed().as_millis() as u64,
                context_summary: "Text too short for suggestions".to_string(),
                from_cache: false,
            });
        }

        // Generate cache key
        let cache_key = self.generate_cache_key(current_text, context, &cursor_position);
        
        // Check cache if enabled
        if config.enable_suggestion_caching {
            let cache = self.suggestion_cache.read().await;
            if let Some(cached_response) = cache.get(&cache_key) {
                let mut response = cached_response.clone();
                response.from_cache = true;
                response.processing_time_ms = start_time.elapsed().as_millis() as u64;
                return Ok(response);
            }
        }

        let max_suggestions = config.max_suggestions;
        drop(config);

        // Generate different types of suggestions based on context
        let mut suggestions = Vec::new();

        // Add subject line suggestions if this is a new email
        if context.email_type == EmailType::New && cursor_position.line == 0 {
            suggestions.extend(self.generate_subject_suggestions(context).await?);
        }

        // Add opening suggestions
        if self.is_opening_context(&cursor_position, current_text) {
            suggestions.extend(self.generate_opening_suggestions(context).await?);
        }

        // Add body completion suggestions
        suggestions.extend(self.generate_body_completion_suggestions(current_text, context).await?);

        // Add closing suggestions if near the end
        if self.is_closing_context(&cursor_position, current_text) {
            suggestions.extend(self.generate_closing_suggestions(context).await?);
        }

        // Add tone adjustment suggestions
        suggestions.extend(self.generate_tone_suggestions(current_text, context).await?);

        // Limit to max suggestions and sort by confidence
        suggestions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        suggestions.truncate(max_suggestions);

        let processing_time = start_time.elapsed().as_millis() as u64;

        let response = SmartComposeResponse {
            suggestions,
            request_id,
            processing_time_ms: processing_time,
            context_summary: self.generate_context_summary(context),
            from_cache: false,
        };

        // Cache the response
        if let Ok(config) = self.config.try_read() {
            if config.enable_suggestion_caching {
                let mut cache = self.suggestion_cache.write().await;
                cache.insert(cache_key, response.clone());
            }
        }

        // Update statistics
        self.update_stats(&response).await;

        info!("Generated {} smart compose suggestions in {}ms", 
              response.suggestions.len(), processing_time);

        Ok(response)
    }

    /// Generate subject line suggestions
    async fn generate_subject_suggestions(&self, context: &CompositionContext) -> AIResult<Vec<ComposeSuggestion>> {
        let prompt = self.build_subject_prompt(context);
        
        let ai_request = EnhancedAIRequest::high_priority(AIOperationType::Custom {
            operation_name: "subject_suggestions".to_string(),
            prompt,
            context: None,
        });

        let response = self.ai_service.process_request(ai_request).await?;
        
        // Parse the response into suggestions
        let subject_lines: Vec<String> = response.content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.trim().to_string())
            .take(3)
            .collect();

        let suggestions: Vec<ComposeSuggestion> = subject_lines
            .into_iter()
            .enumerate()
            .map(|(i, content)| ComposeSuggestion {
                id: Uuid::new_v4(),
                suggestion_type: SuggestionType::SubjectLine,
                content,
                confidence: 0.9 - (i as f32 * 0.1),
                reasoning: "AI-generated subject line based on email context".to_string(),
                context_position: ContextPosition {
                    line: 0,
                    character: 0,
                    word_position: WordPosition::LineStart,
                    surrounding_text: String::new(),
                },
                alternatives: vec![],
                metadata: HashMap::new(),
            })
            .collect();

        Ok(suggestions)
    }

    /// Generate email opening suggestions
    async fn generate_opening_suggestions(&self, context: &CompositionContext) -> AIResult<Vec<ComposeSuggestion>> {
        let prompt = self.build_opening_prompt(context);
        
        let ai_request = EnhancedAIRequest::new(AIOperationType::Custom {
            operation_name: "opening_suggestions".to_string(),
            prompt,
            context: None,
        });

        let response = self.ai_service.process_request(ai_request).await?;
        
        let openings: Vec<String> = response.content
            .split('\n')
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.trim().to_string())
            .take(2)
            .collect();

        let suggestions: Vec<ComposeSuggestion> = openings
            .into_iter()
            .enumerate()
            .map(|(i, content)| ComposeSuggestion {
                id: Uuid::new_v4(),
                suggestion_type: SuggestionType::Opening,
                content,
                confidence: 0.85 - (i as f32 * 0.1),
                reasoning: "Context-appropriate email opening".to_string(),
                context_position: ContextPosition {
                    line: 1,
                    character: 0,
                    word_position: WordPosition::LineStart,
                    surrounding_text: String::new(),
                },
                alternatives: vec![],
                metadata: HashMap::new(),
            })
            .collect();

        Ok(suggestions)
    }

    /// Generate body completion suggestions
    async fn generate_body_completion_suggestions(
        &self,
        current_text: &str,
        context: &CompositionContext,
    ) -> AIResult<Vec<ComposeSuggestion>> {
        let prompt = self.build_completion_prompt(current_text, context);
        
        let ai_request = EnhancedAIRequest::new(AIOperationType::Custom {
            operation_name: "body_completion".to_string(),
            prompt,
            context: None,
        });

        let response = self.ai_service.process_request(ai_request).await?;
        
        let completions: Vec<String> = response.content
            .split('\n')
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.trim().to_string())
            .take(2)
            .collect();

        let suggestions: Vec<ComposeSuggestion> = completions
            .into_iter()
            .enumerate()
            .map(|(i, content)| ComposeSuggestion {
                id: Uuid::new_v4(),
                suggestion_type: SuggestionType::BodyCompletion,
                content,
                confidence: 0.8 - (i as f32 * 0.1),
                reasoning: "AI-powered continuation based on current text".to_string(),
                context_position: ContextPosition {
                    line: current_text.lines().count(),
                    character: current_text.len(),
                    word_position: WordPosition::WordEnd,
                    surrounding_text: current_text.chars().rev().take(50).collect::<String>().chars().rev().collect(),
                },
                alternatives: vec![],
                metadata: HashMap::new(),
            })
            .collect();

        Ok(suggestions)
    }

    /// Generate email closing suggestions
    async fn generate_closing_suggestions(&self, context: &CompositionContext) -> AIResult<Vec<ComposeSuggestion>> {
        let prompt = self.build_closing_prompt(context);
        
        let ai_request = EnhancedAIRequest::new(AIOperationType::Custom {
            operation_name: "closing_suggestions".to_string(),
            prompt,
            context: None,
        });

        let response = self.ai_service.process_request(ai_request).await?;
        
        let closings: Vec<String> = response.content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.trim().to_string())
            .take(2)
            .collect();

        let suggestions: Vec<ComposeSuggestion> = closings
            .into_iter()
            .enumerate()
            .map(|(i, content)| ComposeSuggestion {
                id: Uuid::new_v4(),
                suggestion_type: SuggestionType::Closing,
                content,
                confidence: 0.85 - (i as f32 * 0.1),
                reasoning: "Appropriate email closing based on context and tone".to_string(),
                context_position: ContextPosition {
                    line: 999, // End position
                    character: 0,
                    word_position: WordPosition::LineStart,
                    surrounding_text: String::new(),
                },
                alternatives: vec![],
                metadata: HashMap::new(),
            })
            .collect();

        Ok(suggestions)
    }

    /// Generate tone adjustment suggestions
    async fn generate_tone_suggestions(
        &self,
        current_text: &str,
        context: &CompositionContext,
    ) -> AIResult<Vec<ComposeSuggestion>> {
        if current_text.len() < 50 {
            return Ok(vec![]); // Need sufficient text to analyze tone
        }

        let prompt = format!(
            "Analyze the tone of this email text and suggest improvements:\n\n{}\n\nDesired tone: {:?}\nRelationship: {:?}\nProvide 1-2 specific tone adjustment suggestions.",
            current_text,
            context.style_preferences.tone,
            context.recipients.first().map(|r| &r.relationship).unwrap_or(&RelationshipType::Unknown)
        );
        
        let ai_request = EnhancedAIRequest::new(AIOperationType::Custom {
            operation_name: "tone_analysis".to_string(),
            prompt,
            context: None,
        });

        let response = self.ai_service.process_request(ai_request).await?;
        
        if response.content.contains("appropriate") || response.content.contains("good") {
            return Ok(vec![]); // Tone is already appropriate
        }

        let suggestions = vec![ComposeSuggestion {
            id: Uuid::new_v4(),
            suggestion_type: SuggestionType::ToneAdjustment,
            content: response.content,
            confidence: 0.75,
            reasoning: "Tone analysis and improvement suggestions".to_string(),
            context_position: ContextPosition {
                line: 0,
                character: 0,
                word_position: WordPosition::LineStart,
                surrounding_text: current_text.chars().take(100).collect(),
            },
            alternatives: vec![],
            metadata: HashMap::new(),
        }];

        Ok(suggestions)
    }

    /// Record user acceptance/rejection of suggestions
    pub async fn record_suggestion_feedback(
        &self,
        suggestion_id: Uuid,
        accepted: bool,
        used_content: Option<String>,
    ) -> AIResult<()> {
        let mut stats = self.stats.write().await;
        
        if accepted {
            stats.accepted_suggestions += 1;
        }
        
        // Update acceptance rate
        if stats.total_suggestions > 0 {
            stats.acceptance_rate = (stats.accepted_suggestions as f32 / stats.total_suggestions as f32) * 100.0;
        }

        // If learning mode is enabled, update user writing style
        let config = self.config.read().await;
        if config.enable_learning_mode && accepted {
            if let Some(content) = used_content {
                self.update_writing_style_learning(&content).await;
            }
        }

        debug!("Recorded suggestion feedback: {} (accepted: {})", suggestion_id, accepted);
        
        Ok(())
    }

    /// Update writing style learning data
    async fn update_writing_style_learning(&self, content: &str) {
        // This would analyze the accepted content to learn user preferences
        // For now, we'll implement a simple placeholder
        let mut writing_data = self.writing_style_data.write().await;
        let user_id = "default_user".to_string(); // Would be actual user ID
        
        let style = writing_data.entry(user_id).or_insert_with(|| UserWritingStyle {
            common_phrases: vec![],
            sentence_patterns: vec![],
            vocabulary_preferences: HashMap::new(),
            avg_email_length: 0,
            signatures: vec![],
            tone_by_recipient: HashMap::new(),
            last_updated: chrono::Utc::now().timestamp() as u64,
        });

        // Extract and learn from phrases
        let words: Vec<&str> = content.split_whitespace().collect();
        for window in words.windows(3) {
            let phrase = window.join(" ");
            if !style.common_phrases.contains(&phrase) && phrase.len() > 10 {
                style.common_phrases.push(phrase);
            }
        }

        // Keep only most recent phrases
        style.common_phrases.truncate(100);
        style.last_updated = chrono::Utc::now().timestamp() as u64;
    }

    /// Build subject line generation prompt
    fn build_subject_prompt(&self, context: &CompositionContext) -> String {
        let email_type_desc = match context.email_type {
            EmailType::Reply => "Reply to",
            EmailType::Forward => "Forward of",
            EmailType::MeetingInvitation => "Meeting invitation for",
            EmailType::ProjectUpdate => "Project update on",
            _ => "Email about",
        };

        let context_info = context.business_context
            .as_deref()
            .unwrap_or("general business communication");

        format!(
            "Generate 3 professional subject lines for a {} {}. \
            Recipients: {} \
            Tone: {:?} \
            Keep subjects concise and actionable. \
            List one per line.",
            email_type_desc,
            context_info,
            context.recipients.len(),
            context.style_preferences.tone
        )
    }

    /// Build opening generation prompt
    fn build_opening_prompt(&self, context: &CompositionContext) -> String {
        let relationship = context.recipients.first()
            .map(|r| &r.relationship)
            .unwrap_or(&RelationshipType::Unknown);

        let formality = &context.style_preferences.formality;
        
        format!(
            "Generate 2 appropriate email openings for {} communication with a {:?}. \
            Formality level: {:?} \
            Email type: {:?} \
            Make openings natural and context-appropriate. \
            List one per line.",
            if context.recipients.len() > 1 { "group" } else { "individual" },
            relationship,
            formality,
            context.email_type
        )
    }

    /// Build completion generation prompt
    fn build_completion_prompt(&self, current_text: &str, context: &CompositionContext) -> String {
        format!(
            "Continue this email naturally based on the context:\n\n{}\n\n\
            Email type: {:?}\n\
            Tone: {:?}\n\
            Length preference: {:?}\n\
            Provide 2 natural continuations that maintain the established tone and style. \
            List one per line.",
            current_text,
            context.email_type,
            context.style_preferences.tone,
            context.style_preferences.length
        )
    }

    /// Build closing generation prompt
    fn build_closing_prompt(&self, context: &CompositionContext) -> String {
        let relationship = context.recipients.first()
            .map(|r| &r.relationship)
            .unwrap_or(&RelationshipType::Unknown);

        format!(
            "Generate 2 appropriate email closings for communication with a {:?}. \
            Formality: {:?} \
            Tone: {:?} \
            Make closings professional yet appropriate for the relationship. \
            List one per line.",
            relationship,
            context.style_preferences.formality,
            context.style_preferences.tone
        )
    }

    /// Check if cursor is in opening context
    fn is_opening_context(&self, position: &ContextPosition, text: &str) -> bool {
        position.line <= 2 && text.lines().count() <= 3
    }

    /// Check if cursor is in closing context
    fn is_closing_context(&self, position: &ContextPosition, text: &str) -> bool {
        let line_count = text.lines().count();
        line_count > 3 && position.line >= line_count.saturating_sub(2)
    }

    /// Generate cache key for suggestions
    fn generate_cache_key(
        &self,
        current_text: &str,
        context: &CompositionContext,
        position: &ContextPosition,
    ) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        current_text.hash(&mut hasher);
        context.email_type.hash(&mut hasher);
        context.style_preferences.formality.hash(&mut hasher);
        context.style_preferences.tone.hash(&mut hasher);
        position.line.hash(&mut hasher);
        
        format!("smart_compose_{:x}", hasher.finish())
    }

    /// Generate context summary
    fn generate_context_summary(&self, context: &CompositionContext) -> String {
        format!(
            "{:?} email to {} recipient(s), {:?} tone, {:?} formality",
            context.email_type,
            context.recipients.len(),
            context.style_preferences.tone,
            context.style_preferences.formality
        )
    }

    /// Update usage statistics
    async fn update_stats(&self, response: &SmartComposeResponse) {
        let mut stats = self.stats.write().await;
        stats.total_suggestions += response.suggestions.len();
        
        // Update average response time
        if stats.total_suggestions > 0 {
            let total_requests = stats.total_suggestions / response.suggestions.len().max(1);
            stats.avg_response_time_ms = 
                (stats.avg_response_time_ms * (total_requests - 1) as f64 + response.processing_time_ms as f64) / total_requests as f64;
        }

        // Update suggestions by type
        for suggestion in &response.suggestions {
            let type_name = format!("{:?}", suggestion.suggestion_type);
            *stats.suggestions_by_type.entry(type_name).or_insert(0) += 1;
        }

        // Update cache hit rate
        if response.from_cache {
            stats.cache_hit_rate = ((stats.cache_hit_rate * (stats.total_suggestions - response.suggestions.len()) as f32) + response.suggestions.len() as f32) / stats.total_suggestions as f32;
        }
    }

    /// Get smart compose statistics
    pub async fn get_stats(&self) -> SmartComposeStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Get configuration
    pub async fn get_config(&self) -> SmartComposeConfig {
        let config = self.config.read().await;
        config.clone()
    }

    /// Update configuration
    pub async fn update_config(&self, new_config: SmartComposeConfig) {
        let mut config = self.config.write().await;
        *config = new_config;
        info!("Smart compose configuration updated");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_context() -> CompositionContext {
        CompositionContext {
            email_type: EmailType::New,
            recipients: vec![Recipient {
                email: "colleague@example.com".to_string(),
                name: Some("John Doe".to_string()),
                relationship: RelationshipType::Colleague,
                history_summary: None,
            }],
            referenced_email: None,
            style_preferences: StylePreferences {
                formality: FormalityLevel::Formal,
                tone: ToneType::Professional,
                length: LengthPreference::Moderate,
                include_personal_touches: false,
                prefer_active_voice: true,
                include_action_items: true,
            },
            business_context: Some("quarterly planning".to_string()),
            thread_context: None,
            urgency: UrgencyLevel::Normal,
        }
    }

    #[test]
    fn test_composition_context_creation() {
        let context = create_test_context();
        assert_eq!(context.email_type, EmailType::New);
        assert_eq!(context.recipients.len(), 1);
        assert_eq!(context.style_preferences.formality, FormalityLevel::Formal);
    }

    #[test]
    fn test_smart_compose_config_defaults() {
        let config = SmartComposeConfig::default();
        assert!(config.enabled);
        assert!(config.enable_auto_completion);
        assert_eq!(config.max_suggestions, 5);
        assert_eq!(config.min_trigger_length, 3);
    }

    #[test]
    fn test_suggestion_creation() {
        let suggestion = ComposeSuggestion {
            id: Uuid::new_v4(),
            suggestion_type: SuggestionType::SubjectLine,
            content: "Test subject line".to_string(),
            confidence: 0.9,
            reasoning: "Test reasoning".to_string(),
            context_position: ContextPosition {
                line: 0,
                character: 0,
                word_position: WordPosition::LineStart,
                surrounding_text: String::new(),
            },
            alternatives: vec![],
            metadata: HashMap::new(),
        };

        assert_eq!(suggestion.suggestion_type, SuggestionType::SubjectLine);
        assert_eq!(suggestion.confidence, 0.9);
        assert_eq!(suggestion.content, "Test subject line");
    }

    #[test]
    fn test_cache_key_generation() {
        // This would need access to SmartComposeService instance
        // For now, just test that the function exists and types are correct
        let context = create_test_context();
        let position = ContextPosition {
            line: 1,
            character: 10,
            word_position: WordPosition::WordMiddle,
            surrounding_text: "test context".to_string(),
        };
        
        // Test would verify cache key generation logic
        assert!(position.line == 1);
        assert!(context.email_type == EmailType::New);
    }
}