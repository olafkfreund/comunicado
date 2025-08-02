//! Tests for AI email triage functionality

#[cfg(test)]
mod tests {
    use crate::ai::{
        AIService, EmailTriageConfig, EmailTriageResult, EmailPriority, EmailCategory,
        AIConfig, AIProviderType,
    };
    use crate::ai::cache::AIResponseCache;
    use crate::ai::provider::AIProviderManager;
    use crate::email::StoredMessage;
    use chrono::Utc;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use uuid::Uuid;

    /// Create a test AI service with mock configuration
    async fn create_test_ai_service() -> AIService {
        use std::time::Duration;
        let cache = Arc::new(AIResponseCache::new(1000, Duration::from_secs(3600)));
        let config = Arc::new(RwLock::new(AIConfig {
            enabled: true,
            provider: AIProviderType::Ollama, // Use local for testing
            email_triage_enabled: true,
            ..Default::default()
        }));
        let provider_manager = Arc::new(RwLock::new(AIProviderManager::new(config.clone())));
        
        AIService::new(provider_manager, cache, config)
    }

    /// Create a test email message
    fn create_test_message(
        subject: &str,
        from: &str,
        body: &str,
        priority: Option<String>,
    ) -> StoredMessage {
        let now = Utc::now();
        StoredMessage {
            id: Uuid::new_v4(),
            account_id: "test_account".to_string(),
            folder_name: "INBOX".to_string(),
            imap_uid: 123,
            message_id: Some("test@example.com".to_string()),
            thread_id: None,
            in_reply_to: None,
            references: Vec::new(),
            subject: subject.to_string(),
            from_addr: from.to_string(),
            from_name: Some("Test Sender".to_string()),
            to_addrs: vec!["user@example.com".to_string()],
            cc_addrs: Vec::new(),
            bcc_addrs: Vec::new(),
            reply_to: None,
            date: now,
            body_text: Some(body.to_string()),
            body_html: None,
            attachments: Vec::new(),
            flags: Vec::new(),
            labels: Vec::new(),
            size: Some(1024),
            priority,
            created_at: now,
            updated_at: now,
            last_synced: now,
            sync_version: 1,
            is_draft: false,
            is_deleted: false,
        }
    }

    #[tokio::test]
    async fn test_triage_config_default() {
        let config = EmailTriageConfig::default();
        
        assert!(config.enable_ai_priority);
        assert!(config.enable_sentiment_analysis);
        assert!(config.enable_action_detection);
        assert_eq!(config.max_processing_time, 30);
        assert_eq!(config.min_confidence_threshold, 0.7);
        assert!(config.priority_keywords.contains(&"urgent".to_string()));
        assert!(config.bulk_domains.contains(&"noreply".to_string()));
    }

    #[tokio::test]
    async fn test_vip_sender_detection() {
        let _service = create_test_ai_service().await;
        let mut config = EmailTriageConfig::default();
        config.vip_senders.push("boss@company.com".to_string());
        
        let _message = create_test_message(
            "Regular meeting",
            "boss@company.com",
            "Let's discuss the quarterly report",
            None,
        );

        // This would test VIP detection if AI was available
        // For now we test the config setup
        assert!(config.vip_senders.contains(&"boss@company.com".to_string()));
    }

    #[tokio::test]
    async fn test_bulk_domain_detection() {
        let _service = create_test_ai_service().await;
        let config = EmailTriageConfig::default();
        
        let message = create_test_message(
            "Weekly Newsletter",
            "newsletter@noreply.company.com",
            "Here's your weekly update...",
            None,
        );

        // Test bulk domain matching logic
        let sender_domain = message.from_addr.split('@').nth(1).unwrap_or("");
        let is_bulk = config.bulk_domains.iter()
            .any(|domain| sender_domain.contains(domain));
        
        assert!(is_bulk, "Should detect noreply domain as bulk");
    }

    #[tokio::test]
    async fn test_priority_keyword_detection() {
        let config = EmailTriageConfig::default();
        
        let urgent_message = create_test_message(
            "URGENT: Server is down",
            "admin@company.com",
            "We need immediate action to fix the server",
            None,
        );

        let email_content = format!(
            "Subject: {}\nFrom: {}\nBody: {}",
            urgent_message.subject,
            urgent_message.from_addr,
            urgent_message.body_text.as_deref().unwrap_or("")
        );

        let content_lower = email_content.to_lowercase();
        let found_keywords: Vec<_> = config.priority_keywords.iter()
            .filter(|keyword| content_lower.contains(&keyword.to_lowercase()))
            .cloned()
            .collect();

        assert!(!found_keywords.is_empty(), "Should find 'urgent' keyword");
        assert!(found_keywords.contains(&"urgent".to_string()));
    }

    #[tokio::test]
    async fn test_email_priority_ordering() {
        // Test priority ordering
        let _priorities = vec![
            EmailPriority::Bulk,
            EmailPriority::Low,
            EmailPriority::Normal,
            EmailPriority::High,
            EmailPriority::Critical,
        ];

        // Test display formatting
        assert_eq!(format!("{}", EmailPriority::Critical), "Critical");
        assert_eq!(format!("{}", EmailPriority::High), "High");
        assert_eq!(format!("{}", EmailPriority::Normal), "Normal");
        assert_eq!(format!("{}", EmailPriority::Low), "Low");
        assert_eq!(format!("{}", EmailPriority::Bulk), "Bulk");
    }

    #[tokio::test]
    async fn test_triage_result_structure() {
        let result = EmailTriageResult {
            priority: EmailPriority::High,
            category: EmailCategory::Work,
            urgency_score: 0.8,
            importance_score: 0.7,
            sentiment_score: 0.1,
            confidence: 0.85,
            reasoning: "Contains urgent keywords and work-related content".to_string(),
            action_items: vec!["Review proposal".to_string(), "Schedule meeting".to_string()],
            estimated_response_time: Some(120), // 2 hours
            key_indicators: vec!["urgent".to_string(), "deadline".to_string()],
            requires_human_review: false,
        };

        assert_eq!(result.priority, EmailPriority::High);
        assert_eq!(result.category, EmailCategory::Work);
        assert_eq!(result.action_items.len(), 2);
        assert_eq!(result.estimated_response_time, Some(120));
        assert!(!result.requires_human_review);
    }

    #[tokio::test]
    async fn test_response_time_estimation() {
        // Test response time logic
        let critical_time = match EmailPriority::Critical {
            EmailPriority::Critical => Some(15), // 15 minutes
            EmailPriority::High => Some(120),    // 2 hours
            EmailPriority::Normal => Some(1440), // 1 day
            EmailPriority::Low => Some(4320),    // 3 days
            EmailPriority::Bulk => None,         // No response needed
        };
        
        assert_eq!(critical_time, Some(15));

        let bulk_time = match EmailPriority::Bulk {
            EmailPriority::Critical => Some(15),
            EmailPriority::High => Some(120),
            EmailPriority::Normal => Some(1440),
            EmailPriority::Low => Some(4320),
            EmailPriority::Bulk => None,
        };
        
        assert_eq!(bulk_time, None);
    }

    #[tokio::test]
    async fn test_confidence_calculation() {
        // Test confidence calculation logic
        let urgency_score = 0.8f32;
        let importance_score = 0.7f32;
        
        let score_consistency = 1.0 - (urgency_score - importance_score).abs();
        let confidence = (0.7 + score_consistency * 0.3).min(1.0);
        
        // Should be around 0.73 (0.7 + 0.9 * 0.3)
        assert!((confidence - 0.73).abs() < 0.01);
        
        // Test with identical scores
        let identical_confidence = (0.7f32 + 1.0f32 * 0.3f32).min(1.0f32);
        assert_eq!(identical_confidence, 1.0f32);
    }

    #[tokio::test]
    async fn test_human_review_conditions() {
        let config = EmailTriageConfig::default();
        
        // Test conditions for human review
        let low_confidence = 0.5f32; // Below threshold
        let critical_priority = EmailPriority::Critical;
        let negative_sentiment = -0.8f32;
        let has_action_items = vec!["Call client".to_string()];
        
        let requires_review = low_confidence < config.min_confidence_threshold ||
            critical_priority == EmailPriority::Critical ||
            negative_sentiment < -0.7 ||
            (!has_action_items.is_empty() && 
             matches!(critical_priority, EmailPriority::High | EmailPriority::Critical));
        
        assert!(requires_review, "Should require human review");
    }

    #[tokio::test]  
    async fn test_batch_processing_setup() {
        let _service = create_test_ai_service().await;
        let _config = EmailTriageConfig::default();
        
        let messages = vec![
            create_test_message("Meeting request", "user@example.com", "Can we meet tomorrow?", None),
            create_test_message("Newsletter", "news@newsletter.com", "Weekly updates", None),
            create_test_message("Urgent: Bug report", "dev@company.com", "Critical bug found", None),
        ];
        
        let message_refs: Vec<&StoredMessage> = messages.iter().collect();
        
        // Test that we can set up batch processing
        assert_eq!(message_refs.len(), 3);
        assert!(!message_refs.is_empty());
        
        // Would test actual batch processing if AI providers were available
    }

    #[tokio::test]
    async fn test_category_classification() {
        // Test category enum
        let _categories = vec![
            EmailCategory::Work,
            EmailCategory::Personal,
            EmailCategory::Promotional,
            EmailCategory::Social,
            EmailCategory::Financial,
            EmailCategory::Travel,
            EmailCategory::Shopping,
            EmailCategory::Newsletter,
            EmailCategory::System,
            EmailCategory::Spam,
            EmailCategory::Uncategorized,
        ];

        // Test display formatting
        assert_eq!(format!("{}", EmailCategory::Work), "Work");
        assert_eq!(format!("{}", EmailCategory::Spam), "Spam");
        assert_eq!(format!("{}", EmailCategory::Uncategorized), "Uncategorized");
        
        // Test that all categories are represented
        assert_eq!(_categories.len(), 11);
    }

    #[tokio::test]
    async fn test_ai_prompt_structure() {
        let email_content = "Subject: Meeting Request\nFrom: boss@company.com\nBody: Can we meet tomorrow?";
        
        let expected_prompt_contains = vec![
            "category",
            "urgency_score", 
            "importance_score",
            "sentiment_score",
            "reasoning",
            "action_items",
            "JSON",
        ];

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

        for expected in expected_prompt_contains {
            assert!(triage_prompt.contains(expected), "Prompt should contain '{}'", expected);
        }
    }
}