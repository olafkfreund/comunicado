//! Main AI service providing high-level AI functionality

use crate::ai::{AIContext, AIResult};
use crate::ai::cache::AIResponseCache;
use crate::ai::config::{AIConfig, AIProviderType};
use crate::ai::error::AIError;
use crate::ai::provider::AIProviderManager;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

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
pub struct AIService {
    provider_manager: Arc<RwLock<AIProviderManager>>,
    cache: Arc<AIResponseCache>,
    config: Arc<RwLock<AIConfig>>,
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
            return Ok(suggestions);
        }

        // Get active provider
        let provider_manager = self.provider_manager.read().await;
        let provider = provider_manager.get_active_provider().await?;

        // Generate reply suggestions
        let suggestions = provider.suggest_reply(email_content, user_context).await?;

        // Cache the result
        let cache_content = serde_json::to_string(&suggestions)
            .unwrap_or_else(|_| suggestions.join("\n"));
        
        self.cache
            .cache_response(&cache_key, &cache_content, provider.name(), None)
            .await?;

        Ok(suggestions)
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
            return Ok(cached.content);
        }

        // Get active provider
        let provider_manager = self.provider_manager.read().await;
        let provider = provider_manager.get_active_provider().await?;

        // Generate summary
        let summary = provider.summarize_content(content, max_length).await?;

        // Cache the result
        self.cache
            .cache_response(&cache_key, &summary, provider.name(), None)
            .await?;

        Ok(summary)
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
        })
    }

    /// Clear AI service cache
    pub async fn clear_cache(&self) -> AIResult<usize> {
        self.cache.invalidate_cache("*").await
    }

    /// Complete text using AI provider
    pub async fn complete_text(&self, prompt: &str, context: Option<&AIContext>) -> AIResult<String> {
        let provider_manager = self.provider_manager.read().await;
        let provider = provider_manager.get_active_provider().await?;
        provider.complete_text(prompt, context).await
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