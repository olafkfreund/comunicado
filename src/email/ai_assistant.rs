//! AI-powered email assistant for intelligent email management

use crate::ai::{AIFactory, AIService, AIConfig, EmailCategory};
use crate::email::EmailMessage;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Email composition assistance data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailCompositionAssistance {
    /// Suggested subject lines
    pub subject_suggestions: Vec<String>,
    /// Suggested email body content
    pub body_suggestions: Vec<String>,
    /// Tone recommendations
    pub tone_suggestions: Vec<String>,
    /// Key points to include
    pub key_points: Vec<String>,
    /// Suggested next actions
    pub next_actions: Vec<String>,
}

/// Email reply assistance data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailReplyAssistance {
    /// Generated reply suggestions
    pub reply_suggestions: Vec<String>,
    /// Detected tone of original email
    pub original_tone: String,
    /// Suggested reply tone
    pub suggested_tone: String,
    /// Context summary
    pub context_summary: String,
}

/// Email summary data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailSummary {
    /// Concise summary of email content
    pub summary: String,
    /// Key information points
    pub key_points: Vec<String>,
    /// Detected email category
    pub category: EmailCategory,
    /// Action items extracted from email
    pub action_items: Vec<String>,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
}

/// Bulk email analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkEmailAnalysis {
    /// Email summaries by message ID
    pub summaries: HashMap<String, EmailSummary>,
    /// Category distribution
    pub category_distribution: HashMap<EmailCategory, usize>,
    /// Overall insights
    pub insights: Vec<String>,
    /// Processing statistics
    pub stats: BulkAnalysisStats,
}

/// Statistics for bulk email analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkAnalysisStats {
    /// Total emails processed
    pub total_processed: usize,
    /// Successfully analyzed emails
    pub successful: usize,
    /// Failed analyses
    pub failed: usize,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
}

/// AI-powered email assistant
pub struct AIEmailAssistant {
    ai_service: Arc<AIService>,
    #[allow(dead_code)]
    config: Arc<RwLock<AIConfig>>,
}

impl AIEmailAssistant {
    /// Create a new AI email assistant
    pub fn new(ai_service: Arc<AIService>, config: Arc<RwLock<AIConfig>>) -> Self {
        Self { ai_service, config }
    }

    /// Create AI email assistant from configuration
    pub async fn from_config(config: AIConfig) -> Result<Self> {
        let ai_service = Arc::new(AIFactory::create_ai_service(config.clone()).await?);
        let config = Arc::new(RwLock::new(config));
        
        Ok(Self::new(ai_service, config))
    }

    /// Check if AI assistance is available
    pub async fn is_available(&self) -> bool {
        self.ai_service.is_enabled().await
    }

    /// Generate email composition assistance
    pub async fn get_composition_assistance(
        &self,
        prompt: &str,
        context: Option<&str>,
    ) -> Result<EmailCompositionAssistance> {
        if !self.is_available().await {
            return Err(anyhow::anyhow!("AI assistance is not available"));
        }

        // Get comprehensive email assistance from AI service
        let assistance = self.ai_service.get_email_assistance(prompt, context).await
            .map_err(|e| anyhow::anyhow!("AI composition assistance failed: {}", e))?;

        Ok(EmailCompositionAssistance {
            subject_suggestions: assistance.subject_suggestions,
            body_suggestions: assistance.body_suggestions,
            tone_suggestions: assistance.tone_suggestions,
            key_points: assistance.key_points,
            next_actions: assistance.next_actions,
        })
    }

    /// Generate intelligent email replies
    pub async fn get_reply_assistance(
        &self,
        original_email: &EmailMessage,
        context: Option<&str>,
    ) -> Result<EmailReplyAssistance> {
        if !self.is_available().await {
            return Err(anyhow::anyhow!("AI assistance is not available"));
        }

        // Extract email content for analysis
        let email_content = format!(
            "Subject: {}\nFrom: {}\nContent: {}",
            original_email.subject(),
            original_email.sender(),
            original_email.content()
        );

        let user_context = context.unwrap_or("Professional email reply");

        // Get reply suggestions from AI service
        let reply_suggestions = self.ai_service
            .suggest_email_reply(&email_content, user_context)
            .await
            .map_err(|e| anyhow::anyhow!("AI reply assistance failed: {}", e))?;

        // Analyze original email tone
        let tone_analysis_prompt = format!(
            "Analyze the tone of this email and suggest an appropriate reply tone: {}",
            email_content
        );
        
        let tone_analysis = self.ai_service
            .complete_text(&tone_analysis_prompt, None)
            .await
            .unwrap_or_else(|_| "Professional".to_string());

        // Generate context summary
        let summary = self.ai_service
            .summarize_email(original_email.content(), Some(100))
            .await
            .unwrap_or_else(|_| "Unable to generate summary".to_string());

        Ok(EmailReplyAssistance {
            reply_suggestions,
            original_tone: self.extract_tone(&tone_analysis, "original"),
            suggested_tone: self.extract_tone(&tone_analysis, "suggested"),
            context_summary: summary,
        })
    }

    /// Summarize a single email
    pub async fn summarize_email(&self, email: &EmailMessage) -> Result<EmailSummary> {
        if !self.is_available().await {
            return Err(anyhow::anyhow!("AI assistance is not available"));
        }

        let email_content = format!(
            "Subject: {}\nFrom: {}\nContent: {}",
            email.subject(),
            email.sender(),
            email.content()
        );

        // Generate summary
        let summary = self.ai_service
            .summarize_email(&email_content, Some(200))
            .await
            .map_err(|e| anyhow::anyhow!("Email summarization failed: {}", e))?;

        // Extract key information
        let key_points = self.ai_service
            .extract_key_info(&email_content)
            .await
            .unwrap_or_else(|_| vec!["Unable to extract key points".to_string()]);

        // Categorize email
        let category = self.ai_service
            .categorize_email(&email_content)
            .await
            .unwrap_or(EmailCategory::Uncategorized);

        // Extract action items
        let action_items = self.extract_action_items(&email_content).await;

        Ok(EmailSummary {
            summary,
            key_points,
            category,
            action_items,
            confidence: 0.8, // Default confidence, could be improved with provider feedback
        })
    }

    /// Perform bulk analysis on multiple emails
    pub async fn analyze_emails_bulk(
        &self,
        emails: Vec<&EmailMessage>,
        max_concurrent: usize,
    ) -> Result<BulkEmailAnalysis> {
        if !self.is_available().await {
            return Err(anyhow::anyhow!("AI assistance is not available"));
        }

        let start_time = std::time::Instant::now();
        let mut summaries = HashMap::new();
        let mut category_distribution = HashMap::new();
        let mut successful = 0;
        let mut failed = 0;

        // Process emails in batches to avoid overwhelming the AI service
        let chunks: Vec<_> = emails.chunks(max_concurrent).collect();
        
        for chunk in chunks {
            let tasks: Vec<_> = chunk
                .iter()
                .map(|email| async move {
                    let message_id = email.message_id().to_string();
                    match self.summarize_email(email).await {
                        Ok(summary) => Some((message_id, summary)),
                        Err(e) => {
                            tracing::warn!("Failed to analyze email {}: {}", message_id, e);
                            None
                        }
                    }
                })
                .collect();

            let results = futures::future::join_all(tasks).await;
            
            for result in results {
                if let Some((message_id, summary)) = result {
                    // Update category distribution
                    *category_distribution.entry(summary.category.clone()).or_insert(0) += 1;
                    summaries.insert(message_id, summary);
                    successful += 1;
                } else {
                    failed += 1;
                }
            }
        }

        let processing_time_ms = start_time.elapsed().as_millis() as u64;

        // Generate insights
        let insights = self.generate_insights(&category_distribution, successful).await;

        Ok(BulkEmailAnalysis {
            summaries,
            category_distribution,
            insights,
            stats: BulkAnalysisStats {
                total_processed: emails.len(),
                successful,
                failed,
                processing_time_ms,
            },
        })
    }

    /// Smart email categorization with confidence scoring
    pub async fn categorize_email_smart(&self, email: &EmailMessage) -> Result<(EmailCategory, f32)> {
        if !self.is_available().await {
            return Ok((EmailCategory::Uncategorized, 0.0));
        }

        let email_content = format!(
            "Subject: {}\nFrom: {}\nContent: {}",
            email.subject(),
            email.sender(),
            email.content()
        );

        let category = self.ai_service
            .categorize_email(&email_content)
            .await
            .map_err(|e| anyhow::anyhow!("Email categorization failed: {}", e))?;

        // Calculate confidence based on content characteristics
        let confidence = self.calculate_categorization_confidence(email, &category);

        Ok((category, confidence))
    }

    /// Extract action items from email content
    async fn extract_action_items(&self, content: &str) -> Vec<String> {
        let action_prompt = format!(
            "Extract action items and tasks from this email. List each action item on a separate line:\n\n{}",
            content
        );

        self.ai_service
            .extract_key_info(&action_prompt)
            .await
            .unwrap_or_else(|_| vec![])
            .into_iter()
            .filter(|item| {
                let item_lower = item.to_lowercase();
                item_lower.contains("todo") || 
                item_lower.contains("action") || 
                item_lower.contains("task") ||
                item_lower.contains("follow up") ||
                item_lower.contains("deadline") ||
                item_lower.contains("schedule") ||
                item_lower.contains("meeting")
            })
            .collect()
    }

    /// Extract tone information from analysis text
    fn extract_tone(&self, analysis: &str, _tone_type: &str) -> String {
        let analysis_lower = analysis.to_lowercase();
        
        let tones = ["professional", "friendly", "formal", "casual", "urgent", "polite", "direct"];
        
        for tone in &tones {
            if analysis_lower.contains(&format!("{} tone", tone)) || 
               analysis_lower.contains(&format!("{} style", tone)) {
                return tone.to_string();
            }
        }
        
        "Professional".to_string() // Default fallback
    }

    /// Calculate confidence score for email categorization
    fn calculate_categorization_confidence(&self, email: &EmailMessage, category: &EmailCategory) -> f32 {
        let mut confidence: f32 = 0.5; // Base confidence
        
        let subject = email.subject().to_lowercase();
        let sender = email.sender().to_lowercase();
        let content = email.content().to_lowercase();
        
        // Increase confidence based on strong indicators
        match category {
            EmailCategory::Work => {
                if subject.contains("meeting") || subject.contains("project") || 
                   sender.contains("@company") || content.contains("deadline") {
                    confidence += 0.3;
                }
            },
            EmailCategory::Promotional => {
                if subject.contains("sale") || subject.contains("offer") || 
                   content.contains("unsubscribe") || content.contains("discount") {
                    confidence += 0.4;
                }
            },
            EmailCategory::Personal => {
                if !sender.contains("noreply") && !content.contains("unsubscribe") {
                    confidence += 0.2;
                }
            },
            _ => {}
        }
        
        // Cap confidence at 0.95
        confidence.min(0.95)
    }

    /// Generate insights from bulk email analysis
    async fn generate_insights(
        &self,
        category_distribution: &HashMap<EmailCategory, usize>,
        total_successful: usize,
    ) -> Vec<String> {
        let mut insights = Vec::new();
        
        if total_successful == 0 {
            insights.push("No emails were successfully analyzed".to_string());
            return insights;
        }

        // Find most common category
        if let Some((most_common_category, count)) = category_distribution
            .iter()
            .max_by_key(|(_, &count)| count) {
            let percentage = (*count as f32 / total_successful as f32) * 100.0;
            insights.push(format!(
                "Most common email type: {} ({:.1}% of emails)",
                most_common_category, percentage
            ));
        }

        // Check for promotional email overload
        if let Some(&promo_count) = category_distribution.get(&EmailCategory::Promotional) {
            let promo_percentage = (promo_count as f32 / total_successful as f32) * 100.0;
            if promo_percentage > 30.0 {
                insights.push(format!(
                    "High promotional email volume: {:.1}% - consider unsubscribing from unnecessary lists",
                    promo_percentage
                ));
            }
        }

        // Check work-life balance
        let work_count = category_distribution.get(&EmailCategory::Work).copied().unwrap_or(0);
        let personal_count = category_distribution.get(&EmailCategory::Personal).copied().unwrap_or(0);
        
        if work_count > 0 && personal_count > 0 {
            let work_ratio = work_count as f32 / (work_count + personal_count) as f32;
            if work_ratio > 0.8 {
                insights.push("Work emails dominate your inbox - consider setting boundaries".to_string());
            } else if work_ratio < 0.2 {
                insights.push("Mostly personal emails - good work-life separation".to_string());
            }
        }

        insights
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::config::AIConfig;
    use crate::email::MessageId;
    use chrono::Utc;

    fn create_test_email() -> EmailMessage {
        EmailMessage::new(
            MessageId::new("test@example.com".to_string()),
            "Test Subject".to_string(),
            "sender@example.com".to_string(),
            vec!["recipient@example.com".to_string()],
            "This is a test email content with some important information.".to_string(),
            Utc::now(),
        )
    }

    #[tokio::test]
    async fn test_categorization_confidence() {
        let config = AIConfig::default();
        let ai_service = Arc::new(AIFactory::create_ai_service(config.clone()).await.unwrap());
        
        // Test is focused on the confidence calculation logic which doesn't require AI
        let assistant = AIEmailAssistant::new(ai_service, Arc::new(RwLock::new(config)));
        let email = create_test_email();
        
        let confidence = assistant.calculate_categorization_confidence(&email, &EmailCategory::Personal);
        assert!(confidence >= 0.5 && confidence <= 0.95);
    }

    #[tokio::test]
    async fn test_tone_extraction() {
        let config = AIConfig::default();
        let ai_service = Arc::new(AIFactory::create_ai_service(config.clone()).await.unwrap());
        let assistant = AIEmailAssistant::new(ai_service, Arc::new(RwLock::new(config)));
        
        let analysis = "The original email has a professional tone, I suggest a friendly tone for the reply";
        assert_eq!(assistant.extract_tone(analysis, "original"), "professional");
        
        let analysis2 = "This message uses a casual style of communication";
        assert_eq!(assistant.extract_tone(analysis2, "casual"), "casual");
    }

    #[tokio::test]
    async fn test_insights_generation() {
        let config = AIConfig::default();
        let ai_service = Arc::new(AIFactory::create_ai_service(config.clone()).await.unwrap());
        let assistant = AIEmailAssistant::new(ai_service, Arc::new(RwLock::new(config)));
        
        let mut category_distribution = HashMap::new();
        category_distribution.insert(EmailCategory::Work, 8);
        category_distribution.insert(EmailCategory::Personal, 2);
        
        let insights = assistant.generate_insights(&category_distribution, 10).await;
        assert!(!insights.is_empty());
        assert!(insights[0].contains("Most common email type: Work"));
    }
}