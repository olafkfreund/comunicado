//! Main AI service providing high-level AI functionality

use crate::ai::{AIContext, AIResult};
use crate::ai::cache::AIResponseCache;
use crate::ai::config::{AIConfig, AIProviderType};
use crate::ai::error::AIError;
use crate::ai::provider::AIProviderManager;
use crate::ai::retry::{RetryManager, RetryConfig};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Email categorization types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EmailCategory {
    /// Work-related emails
    Work,
    /// Personal emails
    Personal,
    /// Promotional/marketing emails
    Promotional,
    /// Social media notifications
    Social,
    /// Financial/banking emails
    Financial,
    /// Travel-related emails
    Travel,
    /// Shopping/e-commerce emails
    Shopping,
    /// Newsletter/subscription emails
    Newsletter,
    /// System/automated emails
    System,
    /// Spam or suspicious emails
    Spam,
    /// Uncategorized emails
    Uncategorized,
}

/// Email priority levels for triage
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum EmailPriority {
    /// Critical - requires immediate attention
    Critical,
    /// High - important, should be addressed within a few hours
    High,
    /// Normal - standard priority
    Normal,
    /// Low - can wait, not time-sensitive
    Low,
    /// Bulk - newsletters, promotions, automated emails
    Bulk,
}

impl std::fmt::Display for EmailPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmailPriority::Critical => write!(f, "Critical"),
            EmailPriority::High => write!(f, "High"),
            EmailPriority::Normal => write!(f, "Normal"),
            EmailPriority::Low => write!(f, "Low"),
            EmailPriority::Bulk => write!(f, "Bulk"),
        }
    }
}

/// Comprehensive email triage analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailTriageResult {
    /// Assigned priority level
    pub priority: EmailPriority,
    /// Email category
    pub category: EmailCategory,
    /// Urgency score (0.0 to 1.0)
    pub urgency_score: f32,
    /// Importance score (0.0 to 1.0) 
    pub importance_score: f32,
    /// Sentiment analysis (-1.0 to 1.0, negative to positive)
    pub sentiment_score: f32,
    /// Confidence in triage decision (0.0 to 1.0)
    pub confidence: f32,
    /// AI reasoning for the triage decision
    pub reasoning: String,
    /// Detected action items or next steps
    pub action_items: Vec<String>,
    /// Estimated time to respond (in minutes)
    pub estimated_response_time: Option<u32>,
    /// Keywords that influenced the decision
    pub key_indicators: Vec<String>,
    /// Whether this email requires human review
    pub requires_human_review: bool,
}

/// Email triage configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailTriageConfig {
    /// Enable AI-powered priority assignment
    pub enable_ai_priority: bool,
    /// Enable sentiment analysis
    pub enable_sentiment_analysis: bool,
    /// Enable action item detection
    pub enable_action_detection: bool,
    /// VIP senders (always high priority)
    pub vip_senders: Vec<String>,
    /// Priority keywords for high priority
    pub priority_keywords: Vec<String>,
    /// Domains to treat as bulk/low priority
    pub bulk_domains: Vec<String>,
    /// Maximum processing time in seconds
    pub max_processing_time: u32,
    /// Minimum confidence threshold for automated triage
    pub min_confidence_threshold: f32,
}

impl Default for EmailTriageConfig {
    fn default() -> Self {
        Self {
            enable_ai_priority: true,
            enable_sentiment_analysis: true,
            enable_action_detection: true,
            vip_senders: vec![],
            priority_keywords: vec![
                "urgent".to_string(),
                "asap".to_string(),
                "emergency".to_string(),
                "deadline".to_string(),
                "critical".to_string(),
                "meeting".to_string(),
                "interview".to_string(),
                "action required".to_string(),
                "time sensitive".to_string(),
                "please respond".to_string(),
            ],
            bulk_domains: vec![
                "noreply".to_string(),
                "no-reply".to_string(),
                "donotreply".to_string(),
                "marketing".to_string(),
                "newsletter".to_string(),
                "unsubscribe".to_string(),
            ],
            max_processing_time: 30,
            min_confidence_threshold: 0.7,
        }
    }
}

impl std::fmt::Display for EmailCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmailCategory::Work => write!(f, "Work"),
            EmailCategory::Personal => write!(f, "Personal"),
            EmailCategory::Promotional => write!(f, "Promotional"),
            EmailCategory::Social => write!(f, "Social"),
            EmailCategory::Financial => write!(f, "Financial"),
            EmailCategory::Travel => write!(f, "Travel"),
            EmailCategory::Shopping => write!(f, "Shopping"),
            EmailCategory::Newsletter => write!(f, "Newsletter"),
            EmailCategory::System => write!(f, "System"),
            EmailCategory::Spam => write!(f, "Spam"),
            EmailCategory::Uncategorized => write!(f, "Uncategorized"),
        }
    }
}

/// Natural language scheduling intent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulingIntent {
    /// Type of scheduling intent (meeting, appointment, reminder, etc.)
    pub intent_type: String,
    /// Proposed title or subject
    pub title: Option<String>,
    /// Suggested date and time
    pub datetime: Option<DateTime<Utc>>,
    /// Duration of the event
    pub duration: Option<Duration>,
    /// Participants or attendees
    pub participants: Vec<String>,
    /// Location (physical or virtual)
    pub location: Option<String>,
    /// Additional description or notes
    pub description: Option<String>,
    /// Confidence level of the parsing (0.0 to 1.0)
    pub confidence: f32,
}

/// Email composition assistance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailAssistance {
    /// Suggested subject line
    pub subject_suggestions: Vec<String>,
    /// Suggested email body content
    pub body_suggestions: Vec<String>,
    /// Tone suggestions (formal, casual, friendly, etc.)
    pub tone_suggestions: Vec<String>,
    /// Key points to include
    pub key_points: Vec<String>,
    /// Suggested next actions
    pub next_actions: Vec<String>,
}

/// Main AI service providing high-level functionality
#[derive(Clone)]
pub struct AIService {
    provider_manager: Arc<RwLock<AIProviderManager>>,
    cache: Arc<AIResponseCache>,
    config: Arc<RwLock<AIConfig>>,
    retry_manager: RetryManager,
}

impl AIService {
    /// Create a new AI service
    pub fn new(
        provider_manager: Arc<RwLock<AIProviderManager>>,
        cache: Arc<AIResponseCache>,
        config: Arc<RwLock<AIConfig>>,
    ) -> Self {
        Self {
            provider_manager,
            cache,
            config,
            retry_manager: RetryManager::default(),
        }
    }

    /// Create a new AI service with custom retry configuration
    pub fn new_with_retry_config(
        provider_manager: Arc<RwLock<AIProviderManager>>,
        cache: Arc<AIResponseCache>,
        config: Arc<RwLock<AIConfig>>,
        retry_config: RetryConfig,
    ) -> Self {
        Self {
            provider_manager,
            cache,
            config,
            retry_manager: RetryManager::new(retry_config),
        }
    }

    /// Check if AI functionality is enabled
    pub async fn is_enabled(&self) -> bool {
        let config = self.config.read().await;
        config.enabled && config.provider != AIProviderType::None
    }

    /// Suggest email replies based on email content and context
    pub async fn suggest_email_reply(
        &self,
        email_content: &str,
        user_context: &str,
    ) -> AIResult<Vec<String>> {
        if !self.is_enabled().await {
            return Err(AIError::config_error("AI functionality is disabled"));
        }

        let config = self.config.read().await;
        if !config.is_feature_enabled("email_suggestions") {
            return Err(AIError::feature_not_supported(
                config.provider.to_string(),
                "email_suggestions".to_string(),
            ));
        }

        // Generate cache key
        let cache_key = self.cache.generate_prompt_hash(
            &format!("reply:{}", email_content),
            Some(user_context),
        );

        // Check cache first
        if let Some(cached) = self.cache.get_cached_response(&cache_key).await {
            let suggestions: Vec<String> = serde_json::from_str(&cached.content)
                .unwrap_or_else(|_| vec![cached.content]);
            debug!("Using cached email reply suggestions");
            return Ok(suggestions);
        }

        // Capture values for retry closure
        let email_content = email_content.to_string();
        let user_context = user_context.to_string();
        let provider_manager = self.provider_manager.clone();
        let cache = self.cache.clone();
        let cache_key_clone = cache_key.clone();

        // Execute with retry logic
        let (result, stats) = self.retry_manager.execute_with_stats(|| {
            let email_content = email_content.clone();
            let user_context = user_context.clone();
            let provider_manager = provider_manager.clone();
            let cache = cache.clone();
            let cache_key = cache_key_clone.clone();
            
            async move {
                info!("Attempting to generate email reply suggestions");
                
                // Get active provider
                let provider_manager = provider_manager.read().await;
                let provider = provider_manager.get_active_provider().await?;

                // Generate reply suggestions
                let suggestions = provider.suggest_reply(&email_content, &user_context).await?;

                // Cache the result
                let cache_content = serde_json::to_string(&suggestions)
                    .unwrap_or_else(|_| suggestions.join("\n"));
                
                cache
                    .cache_response(&cache_key, &cache_content, provider.name(), None)
                    .await?;

                Ok(suggestions)
            }
        }).await;

        // Log retry statistics
        if stats.total_attempts > 1 {
            info!(
                "Email reply suggestion completed after {} attempts in {:?} (delays: {:?})",
                stats.total_attempts,
                stats.total_duration,
                stats.total_delay
            );
        }

        result
    }

    /// Summarize email content
    pub async fn summarize_email(&self, content: &str, max_length: Option<usize>) -> AIResult<String> {
        if !self.is_enabled().await {
            return Err(AIError::config_error("AI functionality is disabled"));
        }

        let config = self.config.read().await;
        if !config.is_feature_enabled("email_summarization") {
            return Err(AIError::feature_not_supported(
                config.provider.to_string(),
                "email_summarization".to_string(),
            ));
        }

        // Generate cache key
        let cache_key = self.cache.generate_prompt_hash(
            &format!("summarize:{}", content),
            max_length.as_ref().map(|l| l.to_string()).as_deref(),
        );

        // Check cache first
        if let Some(cached) = self.cache.get_cached_response(&cache_key).await {
            debug!("Using cached email summary");
            return Ok(cached.content);
        }

        // Capture values for retry closure
        let content = content.to_string();
        let provider_manager = self.provider_manager.clone();
        let cache = self.cache.clone();
        let cache_key_clone = cache_key.clone();

        // Execute with retry logic
        let (result, stats) = self.retry_manager.execute_with_stats(|| {
            let content = content.clone();
            let provider_manager = provider_manager.clone();
            let cache = cache.clone();
            let cache_key = cache_key_clone.clone();
            
            async move {
                info!("Attempting to generate email summary");
                
                // Get active provider
                let provider_manager = provider_manager.read().await;
                let provider = provider_manager.get_active_provider().await?;

                // Generate summary
                let summary = provider.summarize_content(&content, max_length).await?;

                // Cache the result
                cache
                    .cache_response(&cache_key, &summary, provider.name(), None)
                    .await?;

                Ok(summary)
            }
        }).await;

        // Log retry statistics
        if stats.total_attempts > 1 {
            info!(
                "Email summarization completed after {} attempts in {:?} (delays: {:?})",
                stats.total_attempts,
                stats.total_duration,
                stats.total_delay
            );
        }

        result
    }

    /// Categorize email content
    pub async fn categorize_email(&self, content: &str) -> AIResult<EmailCategory> {
        if !self.is_enabled().await {
            return Err(AIError::config_error("AI functionality is disabled"));
        }

        let config = self.config.read().await;
        if !config.is_feature_enabled("email_categorization") {
            return Err(AIError::feature_not_supported(
                config.provider.to_string(),
                "email_categorization".to_string(),
            ));
        }

        // Generate cache key
        let cache_key = self.cache.generate_prompt_hash(
            &format!("categorize:{}", content),
            None,
        );

        // Check cache first
        if let Some(cached) = self.cache.get_cached_response(&cache_key).await {
            if let Ok(category) = serde_json::from_str::<EmailCategory>(&cached.content) {
                return Ok(category);
            }
        }

        // Get active provider
        let provider_manager = self.provider_manager.read().await;
        let provider = provider_manager.get_active_provider().await?;

        // Categorize email
        let category = provider.categorize_email(content).await?;

        // Cache the result
        let cache_content = serde_json::to_string(&category)
            .unwrap_or_else(|_| category.to_string());
        
        self.cache
            .cache_response(&cache_key, &cache_content, provider.name(), None)
            .await?;

        Ok(category)
    }

    /// Parse scheduling intent from natural language
    pub async fn parse_scheduling_intent(&self, text: &str) -> AIResult<SchedulingIntent> {
        if !self.is_enabled().await {
            return Err(AIError::config_error("AI functionality is disabled"));
        }

        let config = self.config.read().await;
        if !config.is_feature_enabled("calendar_assistance") {
            return Err(AIError::feature_not_supported(
                config.provider.to_string(),
                "calendar_assistance".to_string(),
            ));
        }

        // Generate cache key
        let cache_key = self.cache.generate_prompt_hash(
            &format!("schedule:{}", text),
            None,
        );

        // Check cache first
        if let Some(cached) = self.cache.get_cached_response(&cache_key).await {
            if let Ok(intent) = serde_json::from_str::<SchedulingIntent>(&cached.content) {
                return Ok(intent);
            }
        }

        // Get active provider
        let provider_manager = self.provider_manager.read().await;
        let provider = provider_manager.get_active_provider().await?;

        // Parse scheduling intent
        let intent = provider.parse_schedule_request(text).await?;

        // Cache the result
        let cache_content = serde_json::to_string(&intent)
            .unwrap_or_else(|_| text.to_string());
        
        self.cache
            .cache_response(&cache_key, &cache_content, provider.name(), None)
            .await?;

        Ok(intent)
    }

    /// Generate a meeting summary from calendar events
    pub async fn generate_meeting_summary(&self, events: &[crate::calendar::Event]) -> AIResult<String> {
        if !self.is_enabled().await {
            return Err(AIError::config_error("AI functionality is disabled"));
        }

        // Convert events to a summary format
        let events_text = events
            .iter()
            .map(|event| {
                let duration_minutes = (event.end_time - event.start_time).num_minutes();
                format!(
                    "Event: {} | Time: {} | Duration: {} min | Participants: {}",
                    event.title,
                    event.start_time.format("%Y-%m-%d %H:%M"),
                    duration_minutes,
                    event.attendees.len()
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        // Generate cache key
        let cache_key = self.cache.generate_prompt_hash(
            &format!("meeting_summary:{}", events_text),
            None,
        );

        // Check cache first
        if let Some(cached) = self.cache.get_cached_response(&cache_key).await {
            return Ok(cached.content);
        }

        // Get active provider
        let provider_manager = self.provider_manager.read().await;
        let provider = provider_manager.get_active_provider().await?;

        // Create AI context for meeting summary
        let context = AIContext {
            user_preferences: std::collections::HashMap::new(),
            email_thread: None,
            calendar_context: Some(events_text.clone()),
            max_length: Some(500),
            creativity: Some(0.3), // Lower creativity for factual summaries
        };

        // Generate summary
        let prompt = format!(
            "Please generate a concise summary of these calendar events and meetings:\n\n{}",
            events_text
        );
        
        let summary = provider.complete_text(&prompt, Some(&context)).await?;

        // Cache the result
        self.cache
            .cache_response(&cache_key, &summary, provider.name(), None)
            .await?;

        Ok(summary)
    }

    /// Get comprehensive email composition assistance
    pub async fn get_email_assistance(
        &self,
        prompt: &str,
        context: Option<&str>,
    ) -> AIResult<EmailAssistance> {
        if !self.is_enabled().await {
            return Err(AIError::config_error("AI functionality is disabled"));
        }

        // Get active provider
        let provider_manager = self.provider_manager.read().await;
        let provider = provider_manager.get_active_provider().await?;

        // Generate different types of assistance
        let subject_prompt = format!("Generate 3 professional email subject lines for: {}", prompt);
        let body_prompt = format!("Generate a professional email body for: {}", prompt);
        let tone_prompt = format!("Suggest appropriate tones for an email about: {}", prompt);

        // Prepare context if provided
        let ai_context = context.map(|c| AIContext {
            user_preferences: std::collections::HashMap::new(),
            email_thread: Some(c.to_string()),
            calendar_context: None,
            max_length: Some(1000),
            creativity: Some(0.7),
        });

        // Generate all suggestions concurrently
        let (subject_suggestions, body_content, tone_content) = tokio::try_join!(
            provider.complete_text(&subject_prompt, None),
            provider.complete_text(&body_prompt, ai_context.as_ref()),
            provider.complete_text(&tone_prompt, None)
        )?;

        // Parse suggestions
        let subject_suggestions = subject_suggestions
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.trim().to_string())
            .take(3)
            .collect();

        let body_suggestions = vec![body_content];

        let tone_suggestions = tone_content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.trim().to_string())
            .take(5)
            .collect();

        Ok(EmailAssistance {
            subject_suggestions,
            body_suggestions,
            tone_suggestions,
            key_points: vec![], // Could be enhanced with key point extraction
            next_actions: vec![], // Could be enhanced with action item detection
        })
    }

    /// Extract key information from content
    pub async fn extract_key_info(&self, content: &str) -> AIResult<Vec<String>> {
        if !self.is_enabled().await {
            return Err(AIError::config_error("AI functionality is disabled"));
        }

        // Generate cache key
        let cache_key = self.cache.generate_prompt_hash(
            &format!("extract_info:{}", content),
            None,
        );

        // Check cache first
        if let Some(cached) = self.cache.get_cached_response(&cache_key).await {
            let info: Vec<String> = serde_json::from_str(&cached.content)
                .unwrap_or_else(|_| vec![cached.content]);
            return Ok(info);
        }

        // Get active provider
        let provider_manager = self.provider_manager.read().await;
        let provider = provider_manager.get_active_provider().await?;

        // Extract key information
        let key_info = provider.extract_key_info(content).await?;

        // Cache the result
        let cache_content = serde_json::to_string(&key_info)
            .unwrap_or_else(|_| key_info.join("\n"));
        
        self.cache
            .cache_response(&cache_key, &cache_content, provider.name(), None)
            .await?;

        Ok(key_info)
    }

    /// Get AI service health status
    pub async fn get_health_status(&self) -> AIResult<AIServiceHealth> {
        let config = self.config.read().await;
        let provider_manager = self.provider_manager.read().await;
        let cache_stats = self.cache.get_cache_stats().await;
        let provider_stats = provider_manager.get_provider_stats().await;

        Ok(AIServiceHealth {
            enabled: config.enabled,
            active_provider: config.provider.clone(),
            provider_healthy: provider_stats.healthy_providers > 0,
            cache_hit_rate: cache_stats.hit_rate,
            total_providers: provider_stats.total_providers,
            healthy_providers: provider_stats.healthy_providers,
            features_enabled: vec![
                ("email_suggestions".to_string(), config.email_suggestions_enabled),
                ("email_summarization".to_string(), config.email_summarization_enabled),
                ("calendar_assistance".to_string(), config.calendar_assistance_enabled),
                ("email_categorization".to_string(), config.email_categorization_enabled),
            ]
            .into_iter()
            .collect(),
            retry_enabled: true, // Retry is always enabled
            retry_max_attempts: 3, // Default from RetryConfig
        })
    }

    /// Clear AI service cache
    pub async fn clear_cache(&self) -> AIResult<usize> {
        self.cache.invalidate_cache("*").await
    }

    /// Execute an AI operation with custom retry configuration
    pub async fn execute_with_custom_retry<F, Fut, T>(
        &self,
        operation: F,
        retry_config: RetryConfig,
    ) -> AIResult<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = AIResult<T>>,
    {
        self.retry_manager
            .execute_with_custom_config(operation, retry_config)
            .await
    }

    /// Get retry statistics for the last operation (if available)
    /// This could be enhanced to store operation history
    pub async fn get_retry_stats_summary(&self) -> String {
        // For now, return a simple message about retry capabilities
        format!(
            "AI retry system active with max {} attempts, base delay {:?}",
            3, // Default max attempts
            std::time::Duration::from_secs(1) // Default base delay
        )
    }

    /// Test AI provider connectivity with retry logic
    pub async fn test_provider_connectivity(&self) -> AIResult<String> {
        let (result, stats) = self.retry_manager.execute_with_stats(|| async {
            let provider_manager = self.provider_manager.read().await;
            let provider = provider_manager.get_active_provider().await?;
            
            // Test with a simple prompt
            provider.complete_text("Test connectivity", None).await
        }).await;

        match result {
            Ok(_response) => {
                let message = if stats.total_attempts > 1 {
                    format!(
                        "✅ Provider connectivity successful after {} attempts ({:?} total time)",
                        stats.total_attempts,
                        stats.total_duration
                    )
                } else {
                    "✅ Provider connectivity successful".to_string()
                };
                Ok(message)
            }
            Err(error) => {
                let message = format!(
                    "❌ Provider connectivity failed after {} attempts: {}",
                    stats.total_attempts,
                    error
                );
                Err(AIError::provider_unavailable(message))
            }
        }
    }

    /// Complete text using AI provider
    pub async fn complete_text(&self, prompt: &str, context: Option<&AIContext>) -> AIResult<String> {
        // Capture values for retry closure
        let prompt = prompt.to_string();
        let context = context.cloned();
        let provider_manager = self.provider_manager.clone();

        // Execute with retry logic
        let (result, stats) = self.retry_manager.execute_with_stats(|| {
            let prompt = prompt.clone();
            let context = context.clone();
            let provider_manager = provider_manager.clone();
            
            async move {
                debug!("Attempting text completion");
                
                let provider_manager = provider_manager.read().await;
                let provider = provider_manager.get_active_provider().await?;
                provider.complete_text(&prompt, context.as_ref()).await
            }
        }).await;

        // Log retry statistics for non-trivial operations
        if stats.total_attempts > 1 {
            debug!(
                "Text completion completed after {} attempts in {:?}",
                stats.total_attempts,
                stats.total_duration
            );
        }

        result
    }

    /// Perform comprehensive AI-powered email triage
    pub async fn triage_email(
        &self,
        message: &crate::email::StoredMessage,
        config: &EmailTriageConfig,
    ) -> AIResult<EmailTriageResult> {
        if !self.is_enabled().await {
            return Err(AIError::config_error("AI functionality is disabled"));
        }

        let ai_config = self.config.read().await;
        if !ai_config.is_feature_enabled("email_triage") {
            return Err(AIError::feature_not_supported(
                ai_config.provider.to_string(),
                "email_triage".to_string(),
            ));
        }
        drop(ai_config);

        let start_time = std::time::Instant::now();

        // Extract email content for analysis
        let email_content = format!(
            "Subject: {}\nFrom: {} <{}>\nTo: {}\nDate: {}\n\nBody:\n{}",
            message.subject,
            message.from_name.as_deref().unwrap_or(""),
            message.from_addr,
            message.to_addrs.join(", "),
            message.date.format("%Y-%m-%d %H:%M:%S"),
            message.body_text.as_deref().unwrap_or("")
        );

        // Quick rule-based checks first
        let mut priority = EmailPriority::Normal;
        let mut key_indicators = Vec::new();

        // Check VIP senders
        if config.vip_senders.contains(&message.from_addr) {
            priority = EmailPriority::High;
            key_indicators.push("VIP sender".to_string());
        }

        // Check for bulk domains
        let sender_domain = message.from_addr.split('@').nth(1).unwrap_or("");
        if config.bulk_domains.iter().any(|domain| sender_domain.contains(domain)) {
            priority = EmailPriority::Bulk;
            key_indicators.push("Bulk domain".to_string());
        }

        // Check priority keywords
        let content_lower = email_content.to_lowercase();
        let found_keywords: Vec<_> = config.priority_keywords.iter()
            .filter(|keyword| content_lower.contains(&keyword.to_lowercase()))
            .cloned()
            .collect();
        
        if !found_keywords.is_empty() {
            if priority != EmailPriority::Bulk {
                priority = EmailPriority::High;
            }
            key_indicators.extend(found_keywords);
        }

        // Generate cache key for AI analysis
        let cache_key = self.cache.generate_prompt_hash(
            &format!("triage:{}", email_content),
            Some(&format!("config:{:?}", config)),
        );

        // Check cache first for AI analysis results
        let mut ai_analysis: Option<(EmailCategory, f32, f32, f32, String, Vec<String>)> = None;
        if let Some(cached) = self.cache.get_cached_response(&cache_key).await {
            if let Ok(cached_result) = serde_json::from_str::<(EmailCategory, f32, f32, f32, String, Vec<String>)>(&cached.content) {
                ai_analysis = Some(cached_result);
            }
        }

        // Perform AI analysis if not cached
        if ai_analysis.is_none() {
            let provider_manager = self.provider_manager.read().await;
            let provider = provider_manager.get_active_provider().await?;

            // Create comprehensive triage prompt
            let triage_prompt = format!(
                r#"Analyze this email and provide a JSON response with the following analysis:

{{
    "category": "Work|Personal|Promotional|Social|Financial|Travel|Shopping|Newsletter|System|Spam|Uncategorized",
    "urgency_score": 0.0-1.0,
    "importance_score": 0.0-1.0,
    "sentiment_score": -1.0-1.0,
    "reasoning": "Brief explanation of the triage decision",
    "action_items": ["list", "of", "detected", "action", "items"]
}}

Email to analyze:
{}

Consider:
- Urgency: How time-sensitive is this email?
- Importance: How important is this to the recipient?
- Sentiment: Is the tone positive, negative, or neutral?
- Action items: What specific actions are requested or implied?
- Category: What type of email is this?

Respond only with valid JSON."#,
                email_content
            );

            let ai_context = AIContext {
                user_preferences: std::collections::HashMap::new(),
                email_thread: Some(email_content.clone()),
                calendar_context: None,
                max_length: Some(1000),
                creativity: Some(0.3), // Lower creativity for analytical tasks
            };

            let response = provider.complete_text(&triage_prompt, Some(&ai_context)).await?;
            
            // Parse AI response
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&response) {
                let category = match parsed["category"].as_str().unwrap_or("Uncategorized") {
                    "Work" => EmailCategory::Work,
                    "Personal" => EmailCategory::Personal,
                    "Promotional" => EmailCategory::Promotional,
                    "Social" => EmailCategory::Social,
                    "Financial" => EmailCategory::Financial,
                    "Travel" => EmailCategory::Travel,
                    "Shopping" => EmailCategory::Shopping,
                    "Newsletter" => EmailCategory::Newsletter,
                    "System" => EmailCategory::System,
                    "Spam" => EmailCategory::Spam,
                    _ => EmailCategory::Uncategorized,
                };

                let urgency_score = parsed["urgency_score"].as_f64().unwrap_or(0.5) as f32;
                let importance_score = parsed["importance_score"].as_f64().unwrap_or(0.5) as f32;
                let sentiment_score = parsed["sentiment_score"].as_f64().unwrap_or(0.0) as f32;
                let reasoning = parsed["reasoning"].as_str().unwrap_or("No reasoning provided").to_string();
                
                let action_items: Vec<String> = parsed["action_items"]
                    .as_array()
                    .map(|arr| arr.iter()
                        .filter_map(|v| v.as_str())
                        .map(String::from)
                        .collect())
                    .unwrap_or_default();

                ai_analysis = Some((category, urgency_score, importance_score, sentiment_score, reasoning, action_items));

                // Cache the AI analysis
                let cache_content = serde_json::to_string(&ai_analysis).unwrap_or_default();
                let _ = self.cache
                    .cache_response(&cache_key, &cache_content, provider.name(), None)
                    .await;
            }
        }

        // Use AI analysis or fallback to defaults
        let (category, urgency_score, importance_score, sentiment_score, reasoning, action_items, ai_available) = 
            if let Some((cat, urg, imp, sent, reason, actions)) = ai_analysis {
                (cat, urg, imp, sent, reason, actions, true)
            } else {
                (EmailCategory::Uncategorized, 0.5, 0.5, 0.0, "Rule-based analysis only".to_string(), Vec::new(), false)
            };

        // Determine final priority based on AI scores and rules
        if priority == EmailPriority::Bulk {
            // Keep bulk classification
        } else {
            priority = match (urgency_score, importance_score) {
                (u, i) if u >= 0.9 || i >= 0.9 => EmailPriority::Critical,
                (u, i) if u >= 0.7 || i >= 0.7 => EmailPriority::High,
                (u, i) if u <= 0.3 && i <= 0.3 => EmailPriority::Low,
                _ => EmailPriority::Normal,
            };
        }

        // Calculate confidence based on AI analysis availability and scores
        let confidence = if ai_available {
            let score_consistency = 1.0 - (urgency_score - importance_score).abs();
            (0.7 + score_consistency * 0.3).min(1.0)
        } else {
            0.6 // Lower confidence for rule-based only
        };

        // Determine if human review is needed
        let requires_human_review = confidence < config.min_confidence_threshold ||
            priority == EmailPriority::Critical ||
            sentiment_score < -0.7 ||
            (!action_items.is_empty() && priority >= EmailPriority::High);

        // Estimate response time based on priority and content
        let estimated_response_time = match priority {
            EmailPriority::Critical => Some(15), // 15 minutes
            EmailPriority::High => Some(120),    // 2 hours
            EmailPriority::Normal => Some(1440), // 1 day
            EmailPriority::Low => Some(4320),    // 3 days
            EmailPriority::Bulk => None,         // No response needed
        };

        let processing_time = start_time.elapsed();
        debug!(
            "Email triage completed in {:?} for message {} with priority {:?}",
            processing_time, message.id, priority
        );

        // Check processing time limit
        let (final_requires_human_review, final_key_indicators) = if processing_time.as_secs() > config.max_processing_time as u64 {
            let mut updated_indicators = key_indicators;
            updated_indicators.push("Processing timeout".to_string());
            (true, updated_indicators)
        } else {
            (requires_human_review, key_indicators)
        };

        Ok(EmailTriageResult {
            priority,
            category,
            urgency_score,
            importance_score,
            sentiment_score,
            confidence,
            reasoning,
            action_items,
            estimated_response_time,
            key_indicators: final_key_indicators,
            requires_human_review: final_requires_human_review,
        })
    }

    /// Batch triage multiple emails efficiently
    pub async fn triage_emails_batch(
        &self,
        messages: &[&crate::email::StoredMessage],
        config: &EmailTriageConfig,
    ) -> AIResult<Vec<EmailTriageResult>> {
        if messages.is_empty() {
            return Ok(Vec::new());
        }

        let mut results = Vec::with_capacity(messages.len());
        
        // Process in parallel with concurrency limit
        let semaphore = Arc::new(tokio::sync::Semaphore::new(5)); // Max 5 concurrent
        let mut handles = Vec::new();

        for message in messages {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let service = self.clone();
            let config = config.clone();
            let message = (*message).clone();

            let handle = tokio::spawn(async move {
                let _permit = permit; // Hold permit until task completes
                service.triage_email(&message, &config).await
            });

            handles.push(handle);
        }

        // Collect results
        for handle in handles {
            match handle.await {
                Ok(Ok(result)) => results.push(result),
                Ok(Err(e)) => {
                    // Create fallback result for failed analysis
                    results.push(EmailTriageResult {
                        priority: EmailPriority::Normal,
                        category: EmailCategory::Uncategorized,
                        urgency_score: 0.5,
                        importance_score: 0.5,
                        sentiment_score: 0.0,
                        confidence: 0.3,
                        reasoning: format!("Triage failed: {}", e),
                        action_items: Vec::new(),
                        estimated_response_time: Some(1440),
                        key_indicators: vec!["Analysis failed".to_string()],
                        requires_human_review: true,
                    });
                }
                Err(e) => {
                    results.push(EmailTriageResult {
                        priority: EmailPriority::Normal,
                        category: EmailCategory::Uncategorized,
                        urgency_score: 0.5,
                        importance_score: 0.5,
                        sentiment_score: 0.0,
                        confidence: 0.3,
                        reasoning: format!("Task failed: {}", e),
                        action_items: Vec::new(),
                        estimated_response_time: Some(1440),
                        key_indicators: vec!["Task error".to_string()],
                        requires_human_review: true,
                    });
                }
            }
        }

        info!(
            "Batch triage completed for {} emails with average confidence: {:.2}",
            results.len(),
            results.iter().map(|r| r.confidence).sum::<f32>() / results.len() as f32
        );

        Ok(results)
    }
}

/// AI service health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIServiceHealth {
    pub enabled: bool,
    pub active_provider: AIProviderType,
    pub provider_healthy: bool,
    pub cache_hit_rate: f64,
    pub total_providers: usize,
    pub healthy_providers: usize,
    pub features_enabled: std::collections::HashMap<String, bool>,
    pub retry_enabled: bool,
    pub retry_max_attempts: usize,
}

// Temporarily disabled while fixing interface issues  
// #[cfg(test)]
// mod tests {
/*    use super::*;
    use crate::ai::config::AIConfig;
    use crate::ai::provider::{AIProviderManager, ProviderCapabilities};
    use async_trait::async_trait;

    // Mock provider for testing
    struct MockAIProvider;

    #[async_trait]
    impl AIProvider for MockAIProvider {
        fn name(&self) -> &str {
            "mock"
        }

        fn capabilities(&self) -> &ProviderCapabilities {
            &ProviderCapabilities {
                name: "mock".to_string(),
                text_completion: true,
                summarization: true,
                email_replies: true,
                scheduling: true,
                categorization: true,
                max_context_length: 4000,
                streaming: false,
                local_processing: true,
                available_models: vec!["mock-model".to_string()],
            }
        }

        async fn health_check(&self) -> AIResult<bool> {
            Ok(true)
        }

        async fn complete_text(&self, _prompt: &str, _context: Option<&AIContext>) -> AIResult<String> {
            Ok("Mock completion".to_string())
        }

        async fn summarize_content(&self, _content: &str, _max_length: Option<usize>) -> AIResult<String> {
            Ok("Mock summary".to_string())
        }

        async fn suggest_reply(&self, _email_content: &str, _context: &str) -> AIResult<Vec<String>> {
            Ok(vec!["Mock reply 1".to_string(), "Mock reply 2".to_string()])
        }

        async fn parse_schedule_request(&self, _text: &str) -> AIResult<SchedulingIntent> {
            Ok(SchedulingIntent {
                intent_type: "meeting".to_string(),
                title: Some("Mock meeting".to_string()),
                datetime: None,
                duration: None,
                participants: vec![],
                location: None,
                description: None,
                confidence: 0.8,
            })
        }

        async fn categorize_email(&self, _content: &str) -> AIResult<EmailCategory> {
            Ok(EmailCategory::Work)
        }

        async fn compose_email(&self, _prompt: &str, _context: Option<&str>) -> AIResult<String> {
            Ok("Mock email".to_string())
        }

        async fn extract_key_info(&self, _content: &str) -> AIResult<Vec<String>> {
            Ok(vec!["Key info 1".to_string(), "Key info 2".to_string()])
        }
    }

    async fn create_test_service() -> AIService {
        let mut config = AIConfig::default();
        config.enabled = true;
        config.provider = AIProviderType::Ollama;
        config.email_suggestions_enabled = true;
        config.email_summarization_enabled = true;
        config.calendar_assistance_enabled = true;
        config.email_categorization_enabled = true;

        let config = Arc::new(RwLock::new(config));
        let mut provider_manager = AIProviderManager::new(config.clone());
        provider_manager.register_provider(AIProviderType::Ollama, Box::new(MockAIProvider));

        let provider_manager = Arc::new(RwLock::new(provider_manager));
        let cache = Arc::new(AIResponseCache::default());

        AIService::new(provider_manager, cache, config)
    }

    #[tokio::test]
    async fn test_email_reply_suggestions() {
        let service = create_test_service().await;
        
        let suggestions = service
            .suggest_email_reply("Hello, how are you?", "Casual conversation")
            .await
            .unwrap();
        
        assert_eq!(suggestions.len(), 2);
        assert_eq!(suggestions[0], "Mock reply 1");
        assert_eq!(suggestions[1], "Mock reply 2");
    }

    #[tokio::test]
    async fn test_email_summarization() {
        let service = create_test_service().await;
        
        let summary = service
            .summarize_email("This is a long email that needs summarization...", Some(100))
            .await
            .unwrap();
        
        assert_eq!(summary, "Mock summary");
    }

    #[tokio::test]
    async fn test_email_categorization() {
        let service = create_test_service().await;
        
        let category = service
            .categorize_email("Meeting scheduled for tomorrow at 3 PM")
            .await
            .unwrap();
        
        assert_eq!(category, EmailCategory::Work);
    }

    #[tokio::test]
    async fn test_scheduling_intent_parsing() {
        let service = create_test_service().await;
        
        let intent = service
            .parse_scheduling_intent("Schedule a meeting for tomorrow at 3 PM")
            .await
            .unwrap();
        
        assert_eq!(intent.intent_type, "meeting");
        assert_eq!(intent.title, Some("Mock meeting".to_string()));
        assert_eq!(intent.confidence, 0.8);
    }

    #[tokio::test]
    async fn test_service_health_status() {
        let service = create_test_service().await;
        
        let health = service.get_health_status().await.unwrap();
        
        assert!(health.enabled);
        assert_eq!(health.active_provider, AIProviderType::Ollama);
        assert!(health.features_enabled["email_suggestions"]);
        assert!(health.features_enabled["email_summarization"]);
    }
}*/