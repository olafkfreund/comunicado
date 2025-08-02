//! Google Gemini provider implementation

use crate::ai::{AIContext, AIResult};
use crate::ai::config::AIConfig;
use crate::ai::error::AIError;
use crate::ai::provider::{AIProvider, ProviderCapabilities};
use crate::ai::service::{EmailCategory, SchedulingIntent};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::timeout;

/// Google Gemini API request structure
#[derive(Debug, Clone, Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GeminiGenerationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    safety_settings: Option<Vec<GeminiSafetySetting>>,
}

/// Gemini content structure
#[derive(Debug, Clone, Serialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
}

/// Gemini part structure
#[derive(Debug, Clone, Serialize)]
struct GeminiPart {
    text: String,
}

/// Gemini generation configuration
#[derive(Debug, Clone, Serialize)]
struct GeminiGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    candidate_count: Option<u32>,
}

/// Gemini safety setting
#[derive(Debug, Clone, Serialize)]
struct GeminiSafetySetting {
    category: String,
    threshold: String,
}

/// Gemini API response structure
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
    #[serde(default)]
    prompt_feedback: Option<GeminiPromptFeedback>,
    #[serde(default)]
    usage_metadata: Option<GeminiUsageMetadata>,
}

/// Gemini candidate structure
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct GeminiCandidate {
    content: GeminiResponseContent,
    finish_reason: Option<String>,
    index: u32,
    #[serde(default)]
    safety_ratings: Vec<GeminiSafetyRating>,
}

/// Gemini response content structure
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct GeminiResponseContent {
    parts: Vec<GeminiResponsePart>,
    role: String,
}

/// Gemini response part structure
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct GeminiResponsePart {
    text: String,
}

/// Gemini prompt feedback
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct GeminiPromptFeedback {
    #[serde(default)]
    safety_ratings: Vec<GeminiSafetyRating>,
}

/// Gemini safety rating
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct GeminiSafetyRating {
    category: String,
    probability: String,
}

/// Gemini usage metadata
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct GeminiUsageMetadata {
    prompt_token_count: u32,
    candidates_token_count: u32,
    total_token_count: u32,
}

/// Gemini error response
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct GeminiErrorResponse {
    error: GeminiError,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct GeminiError {
    code: u32,
    message: String,
    status: String,
}

/// Google Gemini provider implementation
pub struct GoogleProvider {
    client: Client,
    api_key: String,
    model: String,
    capabilities: ProviderCapabilities,
    request_timeout: Duration,
}

impl GoogleProvider {
    /// Create a new Google Gemini provider
    pub fn new(api_key: String, model: String, request_timeout: Duration) -> Self {
        let client = Client::builder()
            .timeout(request_timeout)
            .build()
            .unwrap_or_default();

        let capabilities = ProviderCapabilities {
            name: "Google".to_string(),
            text_completion: true,
            summarization: true,
            email_replies: true,
            scheduling: true,
            categorization: true,
            max_context_length: match model.as_str() {
                "gemini-1.5-pro" => 2000000,      // 2M tokens
                "gemini-1.5-flash" => 1000000,    // 1M tokens
                "gemini-1.0-pro" => 32768,        // 32K tokens
                "gemini-1.0-pro-vision" => 16384, // 16K tokens
                _ => 32768,
            },
            streaming: true,
            local_processing: false,
            available_models: vec![
                "gemini-1.5-pro".to_string(),
                "gemini-1.5-flash".to_string(),
                "gemini-1.0-pro".to_string(),
                "gemini-1.0-pro-vision".to_string(),
            ],
        };

        Self {
            client,
            api_key,
            model,
            capabilities,
            request_timeout,
        }
    }

    /// Create Google provider from config
    pub fn from_config(config: &AIConfig) -> AIResult<Self> {
        let api_key = config.get_api_key("google")
            .ok_or_else(|| AIError::config_error("Google API key not configured"))?
            .clone();

        let model = config.local_model
            .as_ref()
            .unwrap_or(&"gemini-1.5-flash".to_string())
            .clone();

        Ok(Self::new(api_key, model, config.request_timeout))
    }

    /// Make a request to Google Gemini API
    async fn make_request(&self, contents: Vec<GeminiContent>, temperature: Option<f32>) -> AIResult<String> {
        let generation_config = Some(GeminiGenerationConfig {
            temperature,
            top_p: None,
            top_k: None,
            max_output_tokens: Some(1000),
            candidate_count: Some(1),
        });

        let safety_settings = Some(vec![
            GeminiSafetySetting {
                category: "HARM_CATEGORY_HATE_SPEECH".to_string(),
                threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
            },
            GeminiSafetySetting {
                category: "HARM_CATEGORY_DANGEROUS_CONTENT".to_string(),
                threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
            },
            GeminiSafetySetting {
                category: "HARM_CATEGORY_SEXUALLY_EXPLICIT".to_string(),
                threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
            },
            GeminiSafetySetting {
                category: "HARM_CATEGORY_HARASSMENT".to_string(),
                threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
            },
        ]);

        let request = GeminiRequest {
            contents,
            generation_config,
            safety_settings,
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let response = timeout(self.request_timeout,
            self.client
                .post(&url)
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
        )
        .await
        .map_err(|_| AIError::timeout(self.request_timeout))?
        .map_err(AIError::from)?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            
            return match status.as_u16() {
                401 => Err(AIError::auth_failure("Google")),
                429 => Err(AIError::rate_limit("Google", Some(Duration::from_secs(60)))),
                400 => {
                    if let Ok(error_response) = serde_json::from_str::<GeminiErrorResponse>(&error_text) {
                        if error_response.error.status == "INVALID_ARGUMENT" {
                            Err(AIError::invalid_response(error_response.error.message))
                        } else {
                            Err(AIError::provider_unavailable(error_response.error.message))
                        }
                    } else {
                        Err(AIError::invalid_response(error_text))
                    }
                },
                413 => Err(AIError::request_too_large(0)),
                500..=599 => Err(AIError::provider_unavailable("Google server error")),
                _ => Err(AIError::provider_unavailable(format!("Google API error: {}", status))),
            };
        }

        let gemini_response: GeminiResponse = response
            .json()
            .await
            .map_err(|e| AIError::invalid_response(format!("Failed to parse Gemini response: {}", e)))?;

        if gemini_response.candidates.is_empty() {
            return Err(AIError::invalid_response("No candidates in Gemini response"));
        }

        let candidate = &gemini_response.candidates[0];
        if candidate.content.parts.is_empty() {
            return Err(AIError::invalid_response("No parts in Gemini candidate"));
        }

        // Check for safety concerns
        if let Some(finish_reason) = &candidate.finish_reason {
            if finish_reason == "SAFETY" {
                return Err(AIError::content_filtered("Content blocked by safety filters"));
            }
        }

        Ok(candidate.content.parts[0].text.clone())
    }

    /// Create content for Gemini
    fn create_content(&self, text: &str, role: Option<String>) -> Vec<GeminiContent> {
        vec![GeminiContent {
            parts: vec![GeminiPart { text: text.to_string() }],
            role,
        }]
    }

    /// Parse email category from AI response
    fn parse_category_from_response(&self, response: &str) -> EmailCategory {
        let response_lower = response.to_lowercase();
        
        if response_lower.contains("work") || response_lower.contains("business") || response_lower.contains("professional") {
            EmailCategory::Work
        } else if response_lower.contains("personal") || response_lower.contains("family") || response_lower.contains("friend") {
            EmailCategory::Personal
        } else if response_lower.contains("promotional") || response_lower.contains("marketing") || response_lower.contains("advertisement") {
            EmailCategory::Promotional
        } else if response_lower.contains("social") || response_lower.contains("facebook") || response_lower.contains("twitter") {
            EmailCategory::Social
        } else if response_lower.contains("financial") || response_lower.contains("bank") || response_lower.contains("payment") {
            EmailCategory::Financial
        } else if response_lower.contains("travel") || response_lower.contains("flight") || response_lower.contains("hotel") {
            EmailCategory::Travel
        } else if response_lower.contains("shopping") || response_lower.contains("order") || response_lower.contains("purchase") {
            EmailCategory::Shopping
        } else if response_lower.contains("newsletter") || response_lower.contains("subscription") || response_lower.contains("digest") {
            EmailCategory::Newsletter
        } else if response_lower.contains("system") || response_lower.contains("automated") || response_lower.contains("notification") {
            EmailCategory::System
        } else if response_lower.contains("spam") || response_lower.contains("suspicious") || response_lower.contains("phishing") {
            EmailCategory::Spam
        } else {
            EmailCategory::Uncategorized
        }
    }

    /// Parse scheduling intent from Gemini response
    fn parse_schedule_from_response(&self, text: &str, ai_response: &str) -> SchedulingIntent {
        let intent_type = if text.to_lowercase().contains("meeting") {
            "meeting"
        } else if text.to_lowercase().contains("appointment") {
            "appointment"
        } else if text.to_lowercase().contains("reminder") {
            "reminder"
        } else {
            "event"
        };

        // Extract title from AI response
        let title = ai_response
            .lines()
            .find(|line| line.to_lowercase().contains("title") || line.to_lowercase().contains("subject"))
            .and_then(|line| line.split(':').nth(1))
            .map(|s| s.trim().to_string());

        // Gemini provides good structured responses
        let confidence = if ai_response.len() > 100 && title.is_some() {
            0.9
        } else if ai_response.len() > 50 {
            0.75
        } else {
            0.6
        };

        SchedulingIntent {
            intent_type: intent_type.to_string(),
            title,
            datetime: None,
            duration: None,
            participants: vec![],
            location: None,
            description: Some(ai_response.to_string()),
            confidence,
        }
    }
}

#[async_trait]
impl AIProvider for GoogleProvider {
    fn name(&self) -> &str {
        "Google"
    }

    fn capabilities(&self) -> &ProviderCapabilities {
        &self.capabilities
    }

    async fn health_check(&self) -> AIResult<bool> {
        // Simple API availability check
        let contents = self.create_content("Hello", Some("user".to_string()));
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        match timeout(Duration::from_secs(10),
            self.client
                .post(&url)
                .header("Content-Type", "application/json")
                .json(&GeminiRequest {
                    contents,
                    generation_config: Some(GeminiGenerationConfig {
                        temperature: Some(0.1),
                        top_p: None,
                        top_k: None,
                        max_output_tokens: Some(5),
                        candidate_count: Some(1),
                    }),
                    safety_settings: None,
                })
                .send()
        ).await {
            Ok(Ok(response)) => Ok(response.status().is_success()),
            _ => Ok(false),
        }
    }

    async fn complete_text(&self, prompt: &str, context: Option<&AIContext>) -> AIResult<String> {
        let temperature = context
            .and_then(|c| c.creativity)
            .unwrap_or(0.7);

        let enhanced_prompt = if let Some(ctx) = context {
            let system_instruction = "You are a helpful AI assistant for email and calendar management. Provide clear, concise, and professional responses.".to_string();
            let mut full_prompt = prompt.to_string();
            
            if let Some(ref email_thread) = ctx.email_thread {
                full_prompt = format!("Email context: {}\n\nRequest: {}", email_thread, prompt);
            }
            
            if let Some(ref calendar_context) = ctx.calendar_context {
                full_prompt = format!("{}\n\nCalendar context: {}", full_prompt, calendar_context);
            }
            
            if let Some(max_length) = ctx.max_length {
                full_prompt = format!("{}\n\nPlease keep the response under {} characters.", full_prompt, max_length);
            }
            
            format!("{}\n\n{}", system_instruction, full_prompt)
        } else {
            format!("You are a helpful AI assistant for email and calendar management. Provide clear, concise, and professional responses.\n\n{}", prompt)
        };

        let contents = self.create_content(&enhanced_prompt, Some("user".to_string()));
        self.make_request(contents, Some(temperature)).await
    }

    async fn summarize_content(&self, content: &str, max_length: Option<usize>) -> AIResult<String> {
        let max_len = max_length.unwrap_or(200);
        let prompt = format!(
            "You are an expert at creating concise, informative summaries. Please summarize the following content in approximately {} characters or less. Focus on the key points and main message:\n\n{}",
            max_len, content
        );

        let contents = self.create_content(&prompt, Some("user".to_string()));
        self.make_request(contents, Some(0.3)).await
    }

    async fn suggest_reply(&self, email_content: &str, context: &str) -> AIResult<Vec<String>> {
        let prompt = format!(
            "You are an expert email assistant. Based on this email content: \"{}\"\n\nAnd this context: \"{}\"\n\nPlease suggest 3 appropriate email replies. Format your response as:\n1. [First reply]\n2. [Second reply]\n3. [Third reply]",
            email_content, context
        );

        let contents = self.create_content(&prompt, Some("user".to_string()));
        let response = self.make_request(contents, Some(0.6)).await?;
        
        // Parse the numbered responses
        let suggestions: Vec<String> = response
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if let Some(content) = trimmed.strip_prefix("1. ")
                    .or_else(|| trimmed.strip_prefix("2. "))
                    .or_else(|| trimmed.strip_prefix("3. ")) {
                    Some(content.to_string())
                } else {
                    None
                }
            })
            .filter(|s| !s.is_empty())
            .collect();

        if suggestions.is_empty() {
            Ok(vec![response]) // Fallback if parsing fails
        } else {
            Ok(suggestions)
        }
    }

    async fn parse_schedule_request(&self, text: &str) -> AIResult<SchedulingIntent> {
        let prompt = format!(
            "You are a scheduling assistant. Parse this scheduling request and extract key information: \"{}\"\n\nProvide a structured response including:\n- Event type (meeting/appointment/reminder)\n- Suggested title\n- Date/time if mentioned\n- Duration if specified\n- Participants if mentioned\n- Location if specified",
            text
        );

        let contents = self.create_content(&prompt, Some("user".to_string()));
        let response = self.make_request(contents, Some(0.4)).await?;
        Ok(self.parse_schedule_from_response(text, &response))
    }

    async fn categorize_email(&self, content: &str) -> AIResult<EmailCategory> {
        let prompt = format!(
            "You are an email categorization expert. Categorize this email content into one of these categories: Work, Personal, Promotional, Social, Financial, Travel, Shopping, Newsletter, System, Spam, or Uncategorized.\n\nEmail content: \"{}\"\n\nRespond with the category name and a brief explanation.",
            content
        );

        let contents = self.create_content(&prompt, Some("user".to_string()));
        let response = self.make_request(contents, Some(0.2)).await?;
        Ok(self.parse_category_from_response(&response))
    }

    async fn compose_email(&self, prompt: &str, context: Option<&str>) -> AIResult<String> {
        let full_prompt = if let Some(ctx) = context {
            format!(
                "You are a professional email writing assistant. Compose a professional email based on this request: \"{}\"\n\nAdditional context: \"{}\"\n\nPlease provide a complete email with subject and body.",
                prompt, ctx
            )
        } else {
            format!(
                "You are a professional email writing assistant. Compose a professional email based on this request: \"{}\"\n\nPlease provide a complete email with subject and body.",
                prompt
            )
        };

        let contents = self.create_content(&full_prompt, Some("user".to_string()));
        self.make_request(contents, Some(0.6)).await
    }

    async fn extract_key_info(&self, content: &str) -> AIResult<Vec<String>> {
        let prompt = format!(
            "You are an information extraction expert. Extract the key information points from this text. List each important point on a separate line:\n\n{}",
            content
        );

        let contents = self.create_content(&prompt, Some("user".to_string()));
        let response = self.make_request(contents, Some(0.3)).await?;
        
        let key_points: Vec<String> = response
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty() && !line.starts_with('-') && !line.starts_with('•'))
            .collect();

        Ok(key_points)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn create_test_provider() -> GoogleProvider {
        GoogleProvider::new(
            "test-api-key".to_string(),
            "gemini-1.5-flash".to_string(),
            Duration::from_secs(30),
        )
    }

    #[test]
    fn test_provider_creation() {
        let provider = create_test_provider();
        assert_eq!(provider.name(), "Google");
        assert!(!provider.capabilities().local_processing);
        assert!(provider.capabilities().text_completion);
        assert_eq!(provider.capabilities().max_context_length, 1000000);
    }

    #[test]
    fn test_content_creation() {
        let provider = create_test_provider();
        let contents = provider.create_content("Test prompt", Some("user".to_string()));
        
        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0].role, Some("user".to_string()));
        assert_eq!(contents[0].parts.len(), 1);
        assert_eq!(contents[0].parts[0].text, "Test prompt");
    }

    #[test]
    fn test_category_parsing() {
        let provider = create_test_provider();
        
        assert_eq!(
            provider.parse_category_from_response("This appears to be a work-related email"),
            EmailCategory::Work
        );
        
        assert_eq!(
            provider.parse_category_from_response("This looks like a promotional marketing email"),
            EmailCategory::Promotional
        );
    }

    #[test]
    fn test_model_context_lengths() {
        let gemini_pro_provider = GoogleProvider::new(
            "test".to_string(),
            "gemini-1.5-pro".to_string(),
            Duration::from_secs(30),
        );
        assert_eq!(gemini_pro_provider.capabilities().max_context_length, 2000000);

        let gemini_10_provider = GoogleProvider::new(
            "test".to_string(),
            "gemini-1.0-pro".to_string(),
            Duration::from_secs(30),
        );
        assert_eq!(gemini_10_provider.capabilities().max_context_length, 32768);
    }

    #[test]
    fn test_schedule_parsing_confidence() {
        let provider = create_test_provider();
        let intent = provider.parse_schedule_from_response(
            "Schedule a meeting for tomorrow at 3 PM",
            "This is a detailed meeting request with structured information about the event scheduled for tomorrow afternoon. Title: Team standup meeting"
        );
        
        assert_eq!(intent.intent_type, "meeting");
        assert!(intent.confidence > 0.8);
        assert!(intent.title.is_some());
    }

    #[tokio::test]
    async fn test_config_creation() {
        let mut config = AIConfig::default();
        config.set_api_key("google".to_string(), "test-key".to_string());
        config.local_model = Some("gemini-1.5-pro".to_string());
        
        let provider = GoogleProvider::from_config(&config);
        assert!(provider.is_ok());
        
        let provider = provider.unwrap();
        assert_eq!(provider.model, "gemini-1.5-pro");
        assert_eq!(provider.api_key, "test-key");
    }
}