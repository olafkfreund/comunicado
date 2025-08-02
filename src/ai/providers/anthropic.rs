//! Anthropic Claude provider implementation

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

/// Anthropic API request structure
#[derive(Debug, Clone, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<u32>,
}

/// Anthropic message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
struct AnthropicMessage {
    role: String,
    content: String,
}

/// Anthropic API response structure
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct AnthropicResponse {
    id: String,
    #[serde(rename = "type")]
    response_type: String,
    role: String,
    content: Vec<AnthropicContent>,
    model: String,
    stop_reason: Option<String>,
    stop_sequence: Option<String>,
    usage: AnthropicUsage,
}

/// Anthropic content structure
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct AnthropicContent {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

/// Anthropic usage statistics
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

/// Anthropic error response
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct AnthropicErrorResponse {
    #[serde(rename = "type")]
    error_type: String,
    error: AnthropicErrorDetail,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct AnthropicErrorDetail {
    #[serde(rename = "type")]
    detail_type: String,
    message: String,
}

/// Anthropic provider implementation
pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    model: String,
    capabilities: ProviderCapabilities,
    request_timeout: Duration,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider
    pub fn new(api_key: String, model: String, request_timeout: Duration) -> Self {
        let client = Client::builder()
            .timeout(request_timeout)
            .build()
            .unwrap_or_default();

        let capabilities = ProviderCapabilities {
            name: "Anthropic".to_string(),
            text_completion: true,
            summarization: true,
            email_replies: true,
            scheduling: true,
            categorization: true,
            max_context_length: match model.as_str() {
                "claude-3-opus-20240229" => 200000,
                "claude-3-sonnet-20240229" => 200000,
                "claude-3-haiku-20240307" => 200000,
                "claude-2.1" => 200000,
                "claude-2.0" => 100000,
                "claude-instant-1.2" => 100000,
                _ => 100000,
            },
            streaming: true,
            local_processing: false,
            available_models: vec![
                "claude-3-opus-20240229".to_string(),
                "claude-3-sonnet-20240229".to_string(),
                "claude-3-haiku-20240307".to_string(),
                "claude-2.1".to_string(),
                "claude-instant-1.2".to_string(),
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

    /// Create Anthropic provider from config
    pub fn from_config(config: &AIConfig) -> AIResult<Self> {
        let api_key = config.get_api_key("anthropic")
            .ok_or_else(|| AIError::config_error("Anthropic API key not configured"))?
            .clone();

        let model = config.local_model
            .as_ref()
            .unwrap_or(&"claude-3-haiku-20240307".to_string())
            .clone();

        Ok(Self::new(api_key, model, config.request_timeout))
    }

    /// Make a request to Anthropic API
    async fn make_request(
        &self,
        messages: Vec<AnthropicMessage>,
        system_prompt: Option<String>,
        temperature: Option<f32>,
    ) -> AIResult<String> {
        let request = AnthropicRequest {
            model: self.model.clone(),
            max_tokens: 1000,
            messages,
            system: system_prompt,
            temperature,
            top_p: None,
            top_k: None,
        };

        let response = timeout(self.request_timeout,
            self.client
                .post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", "2023-06-01")
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
                401 => Err(AIError::auth_failure("Anthropic")),
                429 => Err(AIError::rate_limit("Anthropic", Some(Duration::from_secs(60)))),
                400 => {
                    if let Ok(error_response) = serde_json::from_str::<AnthropicErrorResponse>(&error_text) {
                        if error_response.error.detail_type == "invalid_request_error" {
                            Err(AIError::invalid_response(error_response.error.message))
                        } else {
                            Err(AIError::provider_unavailable(error_response.error.message))
                        }
                    } else {
                        Err(AIError::invalid_response(error_text))
                    }
                },
                413 => Err(AIError::request_too_large(0)),
                500..=599 => Err(AIError::provider_unavailable("Anthropic server error")),
                _ => Err(AIError::provider_unavailable(format!("Anthropic API error: {}", status))),
            };
        }

        let anthropic_response: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| AIError::invalid_response(format!("Failed to parse Anthropic response: {}", e)))?;

        if anthropic_response.content.is_empty() {
            return Err(AIError::invalid_response("No content in Anthropic response"));
        }

        Ok(anthropic_response.content[0].text.clone())
    }

    /// Create user message for Anthropic
    fn create_user_message(&self, content: &str) -> Vec<AnthropicMessage> {
        vec![AnthropicMessage {
            role: "user".to_string(),
            content: content.to_string(),
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

    /// Parse scheduling intent from Anthropic response
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

        // Claude typically provides high-quality structured responses
        let confidence = if ai_response.len() > 100 && title.is_some() {
            0.95
        } else if ai_response.len() > 50 {
            0.8
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
impl AIProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "Anthropic"
    }

    fn capabilities(&self) -> &ProviderCapabilities {
        &self.capabilities
    }

    async fn health_check(&self) -> AIResult<bool> {
        // Simple API availability check
        let messages = vec![AnthropicMessage {
            role: "user".to_string(),
            content: "Hello".to_string(),
        }];

        match timeout(Duration::from_secs(10),
            self.client
                .post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", "2023-06-01")
                .header("Content-Type", "application/json")
                .json(&AnthropicRequest {
                    model: self.model.clone(),
                    max_tokens: 5,
                    messages,
                    system: None,
                    temperature: Some(0.1),
                    top_p: None,
                    top_k: None,
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

        let system_prompt = Some("You are Claude, a helpful AI assistant for email and calendar management. Provide clear, concise, and professional responses.".to_string());
        
        let enhanced_prompt = if let Some(ctx) = context {
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
            
            full_prompt
        } else {
            prompt.to_string()
        };

        let messages = self.create_user_message(&enhanced_prompt);
        self.make_request(messages, system_prompt, Some(temperature)).await
    }

    async fn summarize_content(&self, content: &str, max_length: Option<usize>) -> AIResult<String> {
        let max_len = max_length.unwrap_or(200);
        let system_prompt = Some("You are an expert at creating concise, informative summaries. Focus on key points and main messages.".to_string());
        let user_prompt = format!(
            "Please summarize the following content in approximately {} characters or less. Focus on the key points and main message:\n\n{}",
            max_len, content
        );

        let messages = self.create_user_message(&user_prompt);
        self.make_request(messages, system_prompt, Some(0.3)).await
    }

    async fn suggest_reply(&self, email_content: &str, context: &str) -> AIResult<Vec<String>> {
        let system_prompt = Some("You are an expert email assistant. Generate appropriate, professional email reply suggestions.".to_string());
        let user_prompt = format!(
            "Based on this email content: \"{}\"\n\nAnd this context: \"{}\"\n\nPlease suggest 3 appropriate email replies. Format your response as:\n1. [First reply]\n2. [Second reply]\n3. [Third reply]",
            email_content, context
        );

        let messages = self.create_user_message(&user_prompt);
        let response = self.make_request(messages, system_prompt, Some(0.6)).await?;
        
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
        let system_prompt = Some("You are a scheduling assistant. Parse scheduling requests and extract structured information with high accuracy.".to_string());
        let user_prompt = format!(
            "Parse this scheduling request and extract key information: \"{}\"\n\nProvide a structured response including:\n- Event type (meeting/appointment/reminder)\n- Suggested title\n- Date/time if mentioned\n- Duration if specified\n- Participants if mentioned\n- Location if specified",
            text
        );

        let messages = self.create_user_message(&user_prompt);
        let response = self.make_request(messages, system_prompt, Some(0.4)).await?;
        Ok(self.parse_schedule_from_response(text, &response))
    }

    async fn categorize_email(&self, content: &str) -> AIResult<EmailCategory> {
        let system_prompt = Some("You are an email categorization expert. Classify emails into appropriate categories with high accuracy.".to_string());
        let user_prompt = format!(
            "Categorize this email content into one of these categories: Work, Personal, Promotional, Social, Financial, Travel, Shopping, Newsletter, System, Spam, or Uncategorized.\n\nEmail content: \"{}\"\n\nRespond with the category name and a brief explanation.",
            content
        );

        let messages = self.create_user_message(&user_prompt);
        let response = self.make_request(messages, system_prompt, Some(0.2)).await?;
        Ok(self.parse_category_from_response(&response))
    }

    async fn compose_email(&self, prompt: &str, context: Option<&str>) -> AIResult<String> {
        let system_prompt = Some("You are a professional email writing assistant. Compose clear, well-structured, and thoughtful emails.".to_string());
        let user_prompt = if let Some(ctx) = context {
            format!(
                "Compose a professional email based on this request: \"{}\"\n\nAdditional context: \"{}\"\n\nPlease provide a complete email with subject and body.",
                prompt, ctx
            )
        } else {
            format!(
                "Compose a professional email based on this request: \"{}\"\n\nPlease provide a complete email with subject and body.",
                prompt
            )
        };

        let messages = self.create_user_message(&user_prompt);
        self.make_request(messages, system_prompt, Some(0.6)).await
    }

    async fn extract_key_info(&self, content: &str) -> AIResult<Vec<String>> {
        let system_prompt = Some("You are an information extraction expert. Identify and list key information points clearly and accurately.".to_string());
        let user_prompt = format!(
            "Extract the key information points from this text. List each important point on a separate line:\n\n{}",
            content
        );

        let messages = self.create_user_message(&user_prompt);
        let response = self.make_request(messages, system_prompt, Some(0.3)).await?;
        
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

    fn create_test_provider() -> AnthropicProvider {
        AnthropicProvider::new(
            "test-api-key".to_string(),
            "claude-3-haiku-20240307".to_string(),
            Duration::from_secs(30),
        )
    }

    #[test]
    fn test_provider_creation() {
        let provider = create_test_provider();
        assert_eq!(provider.name(), "Anthropic");
        assert!(!provider.capabilities().local_processing);
        assert!(provider.capabilities().text_completion);
        assert_eq!(provider.capabilities().max_context_length, 200000);
    }

    #[test]
    fn test_message_creation() {
        let provider = create_test_provider();
        let messages = provider.create_user_message("Test prompt");
        
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[0].content, "Test prompt");
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
        let claude3_provider = AnthropicProvider::new(
            "test".to_string(),
            "claude-3-opus-20240229".to_string(),
            Duration::from_secs(30),
        );
        assert_eq!(claude3_provider.capabilities().max_context_length, 200000);

        let claude2_provider = AnthropicProvider::new(
            "test".to_string(),
            "claude-2.0".to_string(),
            Duration::from_secs(30),
        );
        assert_eq!(claude2_provider.capabilities().max_context_length, 100000);
    }

    #[test]
    fn test_schedule_parsing_confidence() {
        let provider = create_test_provider();
        let intent = provider.parse_schedule_from_response(
            "Schedule a meeting for tomorrow at 3 PM",
            "This is a detailed meeting request with structured information about the event scheduled for tomorrow afternoon."
        );
        
        assert_eq!(intent.intent_type, "meeting");
        assert!(intent.confidence > 0.8); // Claude typically provides high confidence
    }

    #[tokio::test]
    async fn test_config_creation() {
        let mut config = AIConfig::default();
        config.set_api_key("anthropic".to_string(), "test-key".to_string());
        config.local_model = Some("claude-3-sonnet-20240229".to_string());
        
        let provider = AnthropicProvider::from_config(&config);
        assert!(provider.is_ok());
        
        let provider = provider.unwrap();
        assert_eq!(provider.model, "claude-3-sonnet-20240229");
        assert_eq!(provider.api_key, "test-key");
    }
}