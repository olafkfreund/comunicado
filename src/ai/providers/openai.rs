//! OpenAI provider implementation

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

/// OpenAI API request structure
#[derive(Debug, Clone, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    presence_penalty: Option<f32>,
}

/// OpenAI message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
struct OpenAIMessage {
    role: String,
    content: String,
}

/// OpenAI API response structure
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct OpenAIResponse {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<OpenAIChoice>,
    usage: OpenAIUsage,
}

/// OpenAI choice structure
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct OpenAIChoice {
    index: u32,
    message: OpenAIMessage,
    finish_reason: Option<String>,
}

/// OpenAI usage statistics
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

/// OpenAI error response
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct OpenAIErrorResponse {
    error: OpenAIErrorDetail,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct OpenAIErrorDetail {
    message: String,
    #[serde(rename = "type")]
    error_type: String,
    param: Option<String>,
    code: Option<String>,
}

/// OpenAI provider implementation
pub struct OpenAIProvider {
    client: Client,
    api_key: String,
    model: String,
    capabilities: ProviderCapabilities,
    request_timeout: Duration,
}

impl OpenAIProvider {
    /// Create a new OpenAI provider
    pub fn new(api_key: String, model: String, request_timeout: Duration) -> Self {
        let client = Client::builder()
            .timeout(request_timeout)
            .build()
            .unwrap_or_default();

        let capabilities = ProviderCapabilities {
            name: "OpenAI".to_string(),
            text_completion: true,
            summarization: true,
            email_replies: true,
            scheduling: true,
            categorization: true,
            max_context_length: match model.as_str() {
                "gpt-4" | "gpt-4-0613" => 8192,
                "gpt-4-32k" | "gpt-4-32k-0613" => 32768,
                "gpt-4-turbo" | "gpt-4-turbo-preview" => 128000,
                "gpt-3.5-turbo" | "gpt-3.5-turbo-0613" => 4096,
                "gpt-3.5-turbo-16k" => 16384,
                _ => 4096,
            },
            streaming: true,
            local_processing: false,
            available_models: vec![
                "gpt-4".to_string(),
                "gpt-4-turbo".to_string(),
                "gpt-3.5-turbo".to_string(),
                "gpt-3.5-turbo-16k".to_string(),
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

    /// Create OpenAI provider from config
    pub fn from_config(config: &AIConfig) -> AIResult<Self> {
        let api_key = config.get_api_key("openai")
            .ok_or_else(|| AIError::config_error("OpenAI API key not configured"))?
            .clone();

        let model = config.local_model
            .as_ref()
            .unwrap_or(&"gpt-3.5-turbo".to_string())
            .clone();

        Ok(Self::new(api_key, model, config.request_timeout))
    }

    /// Make a request to OpenAI API
    async fn make_request(&self, messages: Vec<OpenAIMessage>, temperature: Option<f32>) -> AIResult<String> {
        let request = OpenAIRequest {
            model: self.model.clone(),
            messages,
            temperature,
            max_tokens: Some(1000),
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
        };

        let response = timeout(self.request_timeout,
            self.client
                .post("https://api.openai.com/v1/chat/completions")
                .header("Authorization", format!("Bearer {}", self.api_key))
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
                401 => Err(AIError::auth_failure("OpenAI")),
                429 => Err(AIError::rate_limit("OpenAI", Some(Duration::from_secs(60)))),
                400 => {
                    if let Ok(error_response) = serde_json::from_str::<OpenAIErrorResponse>(&error_text) {
                        if error_response.error.code.as_deref() == Some("content_policy_violation") {
                            Err(AIError::content_filtered(error_response.error.message))
                        } else {
                            Err(AIError::invalid_response(error_response.error.message))
                        }
                    } else {
                        Err(AIError::invalid_response(error_text))
                    }
                },
                413 => Err(AIError::request_too_large(0)), // Size unknown from status
                500..=599 => Err(AIError::provider_unavailable("OpenAI server error")),
                _ => Err(AIError::provider_unavailable(format!("OpenAI API error: {}", status))),
            };
        }

        let openai_response: OpenAIResponse = response
            .json()
            .await
            .map_err(|e| AIError::invalid_response(format!("Failed to parse OpenAI response: {}", e)))?;

        if openai_response.choices.is_empty() {
            return Err(AIError::invalid_response("No choices in OpenAI response"));
        }

        Ok(openai_response.choices[0].message.content.clone())
    }

    /// Create system and user messages for OpenAI
    fn create_messages(&self, system_prompt: &str, user_prompt: &str) -> Vec<OpenAIMessage> {
        vec![
            OpenAIMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            OpenAIMessage {
                role: "user".to_string(),
                content: user_prompt.to_string(),
            },
        ]
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

    /// Parse scheduling intent from OpenAI response
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

        // Extract title from AI response (look for structured output)
        let title = ai_response
            .lines()
            .find(|line| line.to_lowercase().contains("title") || line.to_lowercase().contains("subject"))
            .and_then(|line| line.split(':').nth(1))
            .map(|s| s.trim().to_string());

        // Calculate confidence based on response quality and input complexity
        let confidence = if ai_response.len() > 100 && title.is_some() {
            0.9
        } else if ai_response.len() > 50 {
            0.7
        } else {
            0.5
        };

        SchedulingIntent {
            intent_type: intent_type.to_string(),
            title,
            datetime: None, // Would need more sophisticated parsing
            duration: None,
            participants: vec![],
            location: None,
            description: Some(ai_response.to_string()),
            confidence,
        }
    }
}

#[async_trait]
impl AIProvider for OpenAIProvider {
    fn name(&self) -> &str {
        "OpenAI"
    }

    fn capabilities(&self) -> &ProviderCapabilities {
        &self.capabilities
    }

    async fn health_check(&self) -> AIResult<bool> {
        // Simple API availability check
        let messages = vec![
            OpenAIMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }
        ];

        match timeout(Duration::from_secs(10), 
            self.client
                .post("https://api.openai.com/v1/chat/completions")
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&OpenAIRequest {
                    model: self.model.clone(),
                    messages,
                    temperature: Some(0.1),
                    max_tokens: Some(5),
                    top_p: None,
                    frequency_penalty: None,
                    presence_penalty: None,
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

        let system_prompt = "You are a helpful AI assistant for email and calendar management. Provide clear, concise, and professional responses.";
        
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

        let messages = self.create_messages(system_prompt, &enhanced_prompt);
        self.make_request(messages, Some(temperature)).await
    }

    async fn summarize_content(&self, content: &str, max_length: Option<usize>) -> AIResult<String> {
        let max_len = max_length.unwrap_or(200);
        let system_prompt = "You are an expert at creating concise, informative summaries. Focus on key points and main messages.";
        let user_prompt = format!(
            "Please summarize the following content in approximately {} characters or less. Focus on the key points and main message:\n\n{}",
            max_len, content
        );

        let messages = self.create_messages(system_prompt, &user_prompt);
        self.make_request(messages, Some(0.3)).await
    }

    async fn suggest_reply(&self, email_content: &str, context: &str) -> AIResult<Vec<String>> {
        let system_prompt = "You are an expert email assistant. Generate appropriate, professional email reply suggestions.";
        let user_prompt = format!(
            "Based on this email content: \"{}\"\n\nAnd this context: \"{}\"\n\nPlease suggest 3 appropriate email replies. Format your response as:\n1. [First reply]\n2. [Second reply]\n3. [Third reply]",
            email_content, context
        );

        let messages = self.create_messages(system_prompt, &user_prompt);
        let response = self.make_request(messages, Some(0.6)).await?;
        
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
        let system_prompt = "You are a scheduling assistant. Parse scheduling requests and extract structured information.";
        let user_prompt = format!(
            "Parse this scheduling request and extract key information: \"{}\"\n\nProvide a structured response including:\n- Event type (meeting/appointment/reminder)\n- Suggested title\n- Date/time if mentioned\n- Duration if specified\n- Participants if mentioned\n- Location if specified",
            text
        );

        let messages = self.create_messages(system_prompt, &user_prompt);
        let response = self.make_request(messages, Some(0.4)).await?;
        Ok(self.parse_schedule_from_response(text, &response))
    }

    async fn categorize_email(&self, content: &str) -> AIResult<EmailCategory> {
        let system_prompt = "You are an email categorization expert. Classify emails into appropriate categories.";
        let user_prompt = format!(
            "Categorize this email content into one of these categories: Work, Personal, Promotional, Social, Financial, Travel, Shopping, Newsletter, System, Spam, or Uncategorized.\n\nEmail content: \"{}\"\n\nRespond with the category name and a brief explanation.",
            content
        );

        let messages = self.create_messages(system_prompt, &user_prompt);
        let response = self.make_request(messages, Some(0.2)).await?;
        Ok(self.parse_category_from_response(&response))
    }

    async fn compose_email(&self, prompt: &str, context: Option<&str>) -> AIResult<String> {
        let system_prompt = "You are a professional email writing assistant. Compose clear, well-structured emails.";
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

        let messages = self.create_messages(system_prompt, &user_prompt);
        self.make_request(messages, Some(0.6)).await
    }

    async fn extract_key_info(&self, content: &str) -> AIResult<Vec<String>> {
        let system_prompt = "You are an information extraction expert. Identify and list key information points clearly.";
        let user_prompt = format!(
            "Extract the key information points from this text. List each important point on a separate line:\n\n{}",
            content
        );

        let messages = self.create_messages(system_prompt, &user_prompt);
        let response = self.make_request(messages, Some(0.3)).await?;
        
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

    fn create_test_provider() -> OpenAIProvider {
        OpenAIProvider::new(
            "test-api-key".to_string(),
            "gpt-3.5-turbo".to_string(),
            Duration::from_secs(30),
        )
    }

    #[test]
    fn test_provider_creation() {
        let provider = create_test_provider();
        assert_eq!(provider.name(), "OpenAI");
        assert!(!provider.capabilities().local_processing);
        assert!(provider.capabilities().text_completion);
        assert_eq!(provider.capabilities().max_context_length, 4096);
    }

    #[test]
    fn test_message_creation() {
        let provider = create_test_provider();
        let messages = provider.create_messages("System prompt", "User prompt");
        
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[0].content, "System prompt");
        assert_eq!(messages[1].role, "user");
        assert_eq!(messages[1].content, "User prompt");
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
        let gpt4_provider = OpenAIProvider::new(
            "test".to_string(),
            "gpt-4".to_string(),
            Duration::from_secs(30),
        );
        assert_eq!(gpt4_provider.capabilities().max_context_length, 8192);

        let gpt4_turbo_provider = OpenAIProvider::new(
            "test".to_string(),
            "gpt-4-turbo".to_string(),
            Duration::from_secs(30),
        );
        assert_eq!(gpt4_turbo_provider.capabilities().max_context_length, 128000);
    }

    #[tokio::test]
    async fn test_config_creation() {
        let mut config = AIConfig::default();
        config.set_api_key("openai".to_string(), "test-key".to_string());
        config.local_model = Some("gpt-4".to_string());
        
        let provider = OpenAIProvider::from_config(&config);
        assert!(provider.is_ok());
        
        let provider = provider.unwrap();
        assert_eq!(provider.model, "gpt-4");
        assert_eq!(provider.api_key, "test-key");
    }
}