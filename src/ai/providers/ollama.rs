//! Ollama local AI provider implementation

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

/// Ollama API request structure
#[derive(Debug, Clone, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    context: Option<Vec<i32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
}

/// Ollama API response structure
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct OllamaResponse {
    model: String,
    response: String,
    done: bool,
    #[serde(default)]
    context: Vec<i32>,
    #[serde(default)]
    total_duration: u64,
    #[serde(default)]
    load_duration: u64,
    #[serde(default)]
    prompt_eval_count: u32,
    #[serde(default)]
    prompt_eval_duration: u64,
    #[serde(default)]
    eval_count: u32,
    #[serde(default)]
    eval_duration: u64,
}

/// Ollama models list response
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct OllamaModelsResponse {
    models: Vec<OllamaModel>,
}

/// Ollama model information
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct OllamaModel {
    name: String,
    size: u64,
    digest: String,
    #[serde(default)]
    details: OllamaModelDetails,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[allow(dead_code)]
struct OllamaModelDetails {
    #[serde(default)]
    format: String,
    #[serde(default)]
    family: String,
    #[serde(default)]
    families: Vec<String>,
    #[serde(default)]
    parameter_size: String,
    #[serde(default)]
    quantization_level: String,
}

/// Ollama health check response
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct OllamaHealthResponse {
    status: String,
}

/// Ollama local AI provider
pub struct OllamaProvider {
    client: Client,
    endpoint: String,
    model: String,
    capabilities: ProviderCapabilities,
    request_timeout: Duration,
}

impl OllamaProvider {
    /// Create a new Ollama provider
    pub fn new(endpoint: String, model: String, request_timeout: Duration) -> Self {
        let client = Client::builder()
            .timeout(request_timeout)
            .build()
            .unwrap_or_default();

        let capabilities = ProviderCapabilities {
            name: "Ollama".to_string(),
            text_completion: true,
            summarization: true,
            email_replies: true,
            scheduling: true,
            categorization: true,
            max_context_length: 4096, // Default, can be model-specific
            streaming: true,
            local_processing: true,
            available_models: vec![model.clone()],
        };

        Self {
            client,
            endpoint,
            model,
            capabilities,
            request_timeout,
        }
    }

    /// Create Ollama provider from config
    pub fn from_config(config: &AIConfig) -> AIResult<Self> {
        let model = config.local_model
            .as_ref()
            .ok_or_else(|| AIError::config_error("No local model specified for Ollama"))?
            .clone();

        Ok(Self::new(
            config.ollama_endpoint.clone(),
            model,
            config.request_timeout,
        ))
    }

    /// Make a request to Ollama API
    async fn make_request(&self, prompt: &str, temperature: Option<f32>) -> AIResult<String> {
        let request = OllamaRequest {
            model: self.model.clone(),
            prompt: prompt.to_string(),
            stream: false,
            context: None,
            temperature,
            max_tokens: Some(1000), // Reasonable default
        };

        let url = format!("{}/api/generate", self.endpoint);
        
        let response = timeout(self.request_timeout, 
            self.client
                .post(&url)
                .json(&request)
                .send()
        )
        .await
        .map_err(|_| AIError::timeout(self.request_timeout))?
        .map_err(AIError::from)?;

        if !response.status().is_success() {
            return Err(AIError::provider_unavailable(
                format!("Ollama API returned status: {}", response.status())
            ));
        }

        let ollama_response: OllamaResponse = response
            .json()
            .await
            .map_err(|e| AIError::invalid_response(format!("Failed to parse Ollama response: {}", e)))?;

        if !ollama_response.done {
            return Err(AIError::invalid_response("Incomplete response from Ollama"));
        }

        Ok(ollama_response.response)
    }

    /// Get available models from Ollama
    #[allow(dead_code)]
    async fn get_available_models(&self) -> AIResult<Vec<String>> {
        let url = format!("{}/api/tags", self.endpoint);
        
        let response = timeout(self.request_timeout,
            self.client.get(&url).send()
        )
        .await
        .map_err(|_| AIError::timeout(self.request_timeout))?
        .map_err(AIError::from)?;

        if !response.status().is_success() {
            return Err(AIError::provider_unavailable(
                format!("Failed to get Ollama models: {}", response.status())
            ));
        }

        let models_response: OllamaModelsResponse = response
            .json()
            .await
            .map_err(|e| AIError::invalid_response(format!("Failed to parse models response: {}", e)))?;

        Ok(models_response.models.into_iter().map(|m| m.name).collect())
    }

    /// Parse scheduling intent from text
    fn parse_schedule_from_text(&self, text: &str, ai_response: &str) -> SchedulingIntent {
        // Simple parsing logic - in a real implementation, this would be more sophisticated
        let intent_type = if text.to_lowercase().contains("meeting") {
            "meeting"
        } else if text.to_lowercase().contains("appointment") {
            "appointment"
        } else if text.to_lowercase().contains("reminder") {
            "reminder"
        } else {
            "event"
        };

        // Extract potential title from AI response
        let title = ai_response
            .lines()
            .find(|line| line.to_lowercase().contains("title") || line.to_lowercase().contains("subject"))
            .map(|line| line.trim().to_string());

        // Simple confidence calculation based on keywords
        let confidence = if text.len() > 20 && (
            text.contains("tomorrow") || 
            text.contains("next week") || 
            text.contains("at ") ||
            text.contains("pm") ||
            text.contains("am")
        ) {
            0.8
        } else if text.len() > 10 {
            0.6
        } else {
            0.4
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
}

#[async_trait]
impl AIProvider for OllamaProvider {
    fn name(&self) -> &str {
        "Ollama"
    }

    fn capabilities(&self) -> &ProviderCapabilities {
        &self.capabilities
    }

    async fn health_check(&self) -> AIResult<bool> {
        let url = format!("{}/api/version", self.endpoint);
        
        match timeout(Duration::from_secs(5), self.client.get(&url).send()).await {
            Ok(Ok(response)) => Ok(response.status().is_success()),
            Ok(Err(_)) | Err(_) => Ok(false),
        }
    }

    async fn complete_text(&self, prompt: &str, context: Option<&AIContext>) -> AIResult<String> {
        let temperature = context
            .and_then(|c| c.creativity)
            .unwrap_or(0.7);

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

        self.make_request(&enhanced_prompt, Some(temperature)).await
    }

    async fn summarize_content(&self, content: &str, max_length: Option<usize>) -> AIResult<String> {
        let max_len = max_length.unwrap_or(200);
        let prompt = format!(
            "Please summarize the following content in approximately {} characters or less. Focus on the key points and main message:\n\n{}",
            max_len, content
        );

        self.make_request(&prompt, Some(0.3)).await
    }

    async fn suggest_reply(&self, email_content: &str, context: &str) -> AIResult<Vec<String>> {
        let prompt = format!(
            "Based on this email content: \"{}\"\n\nAnd this context: \"{}\"\n\nPlease suggest 3 appropriate email replies. Each reply should be on a separate line, starting with a number (1., 2., 3.):",
            email_content, context
        );

        let response = self.make_request(&prompt, Some(0.6)).await?;
        
        // Parse the numbered responses
        let suggestions: Vec<String> = response
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if trimmed.starts_with("1.") || trimmed.starts_with("2.") || trimmed.starts_with("3.") {
                    Some(trimmed.split_once('.').map(|(_, reply)| reply.trim().to_string()).unwrap_or_default())
                } else {
                    None
                }
            })
            .filter(|s| !s.is_empty())
            .collect();

        if suggestions.is_empty() {
            // Fallback if parsing fails
            Ok(vec![response])
        } else {
            Ok(suggestions)
        }
    }

    async fn parse_schedule_request(&self, text: &str) -> AIResult<SchedulingIntent> {
        let prompt = format!(
            "Parse this scheduling request and extract key information: \"{}\"\n\nProvide a structured response including:\n- Event type (meeting/appointment/reminder)\n- Suggested title\n- Date/time if mentioned\n- Duration if specified\n- Participants if mentioned\n- Location if specified",
            text
        );

        let response = self.make_request(&prompt, Some(0.4)).await?;
        Ok(self.parse_schedule_from_text(text, &response))
    }

    async fn categorize_email(&self, content: &str) -> AIResult<EmailCategory> {
        let prompt = format!(
            "Categorize this email content into one of these categories: Work, Personal, Promotional, Social, Financial, Travel, Shopping, Newsletter, System, Spam, or Uncategorized.\n\nEmail content: \"{}\"\n\nRespond with just the category name and a brief explanation.",
            content
        );

        let response = self.make_request(&prompt, Some(0.2)).await?;
        Ok(self.parse_category_from_response(&response))
    }

    async fn compose_email(&self, prompt: &str, context: Option<&str>) -> AIResult<String> {
        let enhanced_prompt = if let Some(ctx) = context {
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

        self.make_request(&enhanced_prompt, Some(0.6)).await
    }

    async fn extract_key_info(&self, content: &str) -> AIResult<Vec<String>> {
        let prompt = format!(
            "Extract the key information points from this text. List each important point on a separate line:\n\n{}",
            content
        );

        let response = self.make_request(&prompt, Some(0.3)).await?;
        
        let key_points: Vec<String> = response
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect();

        Ok(key_points)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::Duration;

    fn create_test_provider() -> OllamaProvider {
        OllamaProvider::new(
            "http://localhost:11434".to_string(),
            "llama2".to_string(),
            Duration::from_secs(30),
        )
    }

    #[test]
    fn test_provider_creation() {
        let provider = create_test_provider();
        assert_eq!(provider.name(), "Ollama");
        assert!(provider.capabilities().local_processing);
        assert!(provider.capabilities().text_completion);
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
        
        assert_eq!(
            provider.parse_category_from_response("This seems to be a personal message from family"),
            EmailCategory::Personal
        );
    }

    #[test]
    fn test_schedule_parsing() {
        let provider = create_test_provider();
        let intent = provider.parse_schedule_from_text(
            "Schedule a meeting for tomorrow at 3 PM",
            "This is a meeting request for tomorrow afternoon"
        );
        
        assert_eq!(intent.intent_type, "meeting");
        assert!(intent.confidence > 0.7);
    }

    #[tokio::test]
    async fn test_config_creation() {
        let mut config = AIConfig::default();
        config.local_model = Some("llama2".to_string());
        config.ollama_endpoint = "http://localhost:11434".to_string();
        
        let provider = OllamaProvider::from_config(&config);
        assert!(provider.is_ok());
        
        let provider = provider.unwrap();
        assert_eq!(provider.model, "llama2");
        assert_eq!(provider.endpoint, "http://localhost:11434");
    }
}