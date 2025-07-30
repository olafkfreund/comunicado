# API Specification

This is the API specification for the spec detailed in @.agent-os/specs/2025-07-30-ai-integration/spec.md

> Created: 2025-07-30
> Version: 1.0.0

## Internal Service APIs

### AIService

**Purpose:** Main service interface for AI functionality integration
**Methods:** 
- `suggest_email_reply(email_content: &str, user_context: &str) -> Result<Vec<String>, AIError>`
- `summarize_email(content: &str) -> Result<String, AIError>`
- `categorize_email(content: &str) -> Result<EmailCategory, AIError>`
- `parse_scheduling_intent(text: &str) -> Result<SchedulingIntent, AIError>`
- `generate_meeting_summary(events: &[Event]) -> Result<String, AIError>`

### AIProviderManager

**Purpose:** Manages multiple AI provider implementations and routing
**Methods:**
- `get_active_provider() -> Result<Box<dyn AIProvider>, AIError>`
- `switch_provider(provider: AIProviderType) -> Result<(), AIError>`
- `test_provider_connectivity(provider: AIProviderType) -> Result<bool, AIError>`
- `get_provider_capabilities(provider: AIProviderType) -> ProviderCapabilities`

### AIResponseCache

**Purpose:** Caches AI responses to reduce API calls and improve performance
**Methods:**
- `get_cached_response(prompt_hash: &str) -> Option<CachedResponse>`
- `cache_response(prompt_hash: &str, response: &str, ttl: Duration) -> Result<(), CacheError>`
- `invalidate_cache(pattern: &str) -> Result<(), CacheError>`
- `get_cache_stats() -> CacheStatistics`

## External AI Provider Integration

### Ollama Local API

**Endpoint:** `POST http://localhost:11434/api/generate`
**Purpose:** Local AI model inference for privacy-first processing
**Parameters:** 
- `model`: String - Model name (e.g., "llama2", "codellama")
- `prompt`: String - Input prompt with context
- `stream`: Boolean - Whether to stream response
**Response:** JSON with generated text and metadata
**Errors:** Connection refused, model not found, generation timeout

### OpenAI API Integration

**Endpoint:** `POST https://api.openai.com/v1/chat/completions`
**Purpose:** Cloud-based AI processing with advanced capabilities
**Parameters:**
- `model`: String - GPT model version
- `messages`: Array - Conversation context
- `max_tokens`: Integer - Response length limit
- `temperature`: Float - Response creativity level
**Response:** JSON with completion choices and usage statistics
**Errors:** Rate limiting, authentication failure, content policy violation

### Anthropic Claude API Integration

**Endpoint:** `POST https://api.anthropic.com/v1/messages`
**Purpose:** Alternative cloud AI provider for diversity and fallback
**Parameters:**
- `model`: String - Claude model version
- `max_tokens`: Integer - Response length limit
- `messages`: Array - Message history with roles
**Response:** JSON with message content and metadata
**Errors:** API quota exceeded, invalid request format, service unavailable

## Error Handling Patterns

### AIError Types
- `ProviderUnavailable`: AI service is down or unreachable
- `AuthenticationFailure`: Invalid API keys or authentication
- `RateLimitExceeded`: Too many requests to AI provider
- `ContentFiltered`: AI provider blocked content due to policies
- `InvalidResponse`: Malformed or unexpected AI response
- `ConfigurationError`: Invalid AI configuration or missing setup

### Retry Logic
- Exponential backoff for transient failures (network, rate limits)
- Provider fallback for service unavailability
- Local caching to reduce dependency on external services
- Graceful degradation when AI features are unavailable