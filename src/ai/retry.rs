//! AI retry logic and error recovery mechanisms

use crate::ai::{AIError, AIResult};
use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};

/// Configuration for retry behavior
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: usize,
    /// Base delay for exponential backoff
    pub base_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Multiplier for exponential backoff
    pub backoff_multiplier: f64,
    /// Whether to add jitter to retry delays
    pub jitter_enabled: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(1000),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            jitter_enabled: true,
        }
    }
}

impl RetryConfig {
    /// Create a new retry config with custom parameters
    pub fn new(
        max_attempts: usize,
        base_delay: Duration,
        max_delay: Duration,
        backoff_multiplier: f64,
    ) -> Self {
        Self {
            max_attempts,
            base_delay,
            max_delay,
            backoff_multiplier,
            jitter_enabled: true,
        }
    }

    /// Create a retry config optimized for network operations
    pub fn for_network() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 2.0,
            jitter_enabled: true,
        }
    }

    /// Create a retry config optimized for rate limiting
    pub fn for_rate_limits() -> Self {
        Self {
            max_attempts: 5,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            backoff_multiplier: 2.5,
            jitter_enabled: true,
        }
    }

    /// Create a retry config for provider unavailability
    pub fn for_provider_unavailable() -> Self {
        Self {
            max_attempts: 2,
            base_delay: Duration::from_secs(5),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            jitter_enabled: false,
        }
    }
}

/// Retry mechanism for AI operations
#[derive(Clone)]
pub struct RetryManager {
    config: RetryConfig,
}

impl RetryManager {
    /// Create a new retry manager
    pub fn new(config: RetryConfig) -> Self {
        Self { config }
    }

    /// Create a retry manager with default configuration
    pub fn default() -> Self {
        Self {
            config: RetryConfig::default(),
        }
    }

    /// Execute an operation with retry logic
    pub async fn execute_with_retry<F, Fut, T>(&self, operation: F) -> AIResult<T>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = AIResult<T>>,
    {
        let mut attempt = 0;
        let mut last_error = None;

        while attempt < self.config.max_attempts {
            attempt += 1;

            debug!("Attempting AI operation (attempt {}/{})", attempt, self.config.max_attempts);

            match operation().await {
                Ok(result) => {
                    if attempt > 1 {
                        debug!("AI operation succeeded on attempt {}", attempt);
                    }
                    return Ok(result);
                }
                Err(error) => {
                    last_error = Some(error.clone());

                    // Check if the error is retryable
                    if !error.is_retryable() {
                        warn!("AI operation failed with non-retryable error: {}", error);
                        return Err(error);
                    }

                    // Check if we've exhausted all attempts
                    if attempt >= self.config.max_attempts {
                        warn!(
                            "AI operation failed after {} attempts, last error: {}",
                            self.config.max_attempts, error
                        );
                        break;
                    }

                    // Calculate delay for next attempt
                    let delay = self.calculate_delay(attempt, &error);
                    
                    warn!(
                        "AI operation failed (attempt {}), retrying in {:?}: {}",
                        attempt, delay, error
                    );

                    // Wait before next attempt
                    sleep(delay).await;
                }
            }
        }

        // Return the last error if all attempts failed
        Err(last_error.unwrap_or_else(|| {
            AIError::internal_error("Retry loop failed without capturing error")
        }))
    }

    /// Execute an operation with a custom retry configuration
    pub async fn execute_with_custom_config<F, Fut, T>(
        &self,
        operation: F,
        custom_config: RetryConfig,
    ) -> AIResult<T>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = AIResult<T>>,
    {
        let temp_manager = RetryManager::new(custom_config);
        temp_manager.execute_with_retry(operation).await
    }

    /// Calculate delay for next retry attempt
    fn calculate_delay(&self, attempt: usize, error: &AIError) -> Duration {
        // Check if the error specifies a retry delay
        if let Some(error_delay) = error.retry_delay() {
            return std::cmp::min(error_delay, self.config.max_delay);
        }

        // Calculate exponential backoff delay
        let delay_ms = (self.config.base_delay.as_millis() as f64
            * self.config.backoff_multiplier.powi((attempt - 1) as i32)) as u64;

        let mut delay = Duration::from_millis(delay_ms);

        // Apply maximum delay limit
        delay = std::cmp::min(delay, self.config.max_delay);

        // Add jitter if enabled
        if self.config.jitter_enabled {
            delay = self.add_jitter(delay);
        }

        delay
    }

    /// Add random jitter to delay to prevent thundering herd
    fn add_jitter(&self, delay: Duration) -> Duration {
        use rand::Rng;
        
        let jitter_range = delay.as_millis() as f64 * 0.1; // 10% jitter
        let mut rng = rand::thread_rng();
        let jitter = rng.gen_range(-jitter_range..=jitter_range);
        
        let adjusted_ms = (delay.as_millis() as f64 + jitter).max(0.0) as u64;
        Duration::from_millis(adjusted_ms)
    }

    /// Get retry statistics for an operation
    pub async fn execute_with_stats<F, Fut, T>(&self, operation: F) -> (AIResult<T>, RetryStats)
    where
        F: Fn() -> Fut,
        Fut: Future<Output = AIResult<T>>,
    {
        let start_time = std::time::Instant::now();
        let mut attempts = 0;
        let mut total_delay = Duration::ZERO;
        let mut errors = Vec::new();

        let mut attempt = 0;
        let mut last_error = None;

        while attempt < self.config.max_attempts {
            attempt += 1;
            attempts += 1;

            let _attempt_start = std::time::Instant::now();

            match operation().await {
                Ok(result) => {
                    let stats = RetryStats {
                        total_attempts: attempts,
                        total_duration: start_time.elapsed(),
                        total_delay,
                        errors,
                        success: true,
                    };
                    return (Ok(result), stats);
                }
                Err(error) => {
                    errors.push(error.clone());
                    last_error = Some(error.clone());

                    if !error.is_retryable() || attempt >= self.config.max_attempts {
                        break;
                    }

                    let delay = self.calculate_delay(attempt, &error);
                    total_delay += delay;
                    sleep(delay).await;
                }
            }
        }

        let stats = RetryStats {
            total_attempts: attempts,
            total_duration: start_time.elapsed(),
            total_delay,
            errors,
            success: false,
        };

        let final_error = last_error.unwrap_or_else(|| {
            AIError::internal_error("Retry loop failed without capturing error")
        });

        (Err(final_error), stats)
    }
}

/// Statistics about retry attempts
#[derive(Debug, Clone)]
pub struct RetryStats {
    /// Total number of attempts made
    pub total_attempts: usize,
    /// Total duration including delays
    pub total_duration: Duration,
    /// Total time spent waiting between retries
    pub total_delay: Duration,
    /// All errors encountered during retry attempts
    pub errors: Vec<AIError>,
    /// Whether the operation ultimately succeeded
    pub success: bool,
}

impl RetryStats {
    /// Get the average delay between attempts
    pub fn average_delay(&self) -> Duration {
        if self.total_attempts <= 1 {
            Duration::ZERO
        } else {
            self.total_delay / (self.total_attempts - 1) as u32
        }
    }

    /// Get the success rate (1.0 if succeeded, 0.0 if failed)
    pub fn success_rate(&self) -> f64 {
        if self.success { 1.0 } else { 0.0 }
    }

    /// Get a summary of error types encountered
    pub fn error_summary(&self) -> std::collections::HashMap<String, usize> {
        let mut summary = std::collections::HashMap::new();
        
        for error in &self.errors {
            let error_type = match error {
                AIError::NetworkError { .. } => "NetworkError",
                AIError::Timeout { .. } => "Timeout",
                AIError::RateLimitExceeded { .. } => "RateLimitExceeded",
                AIError::ProviderUnavailable { .. } => "ProviderUnavailable",
                AIError::AuthenticationFailure { .. } => "AuthenticationFailure",
                AIError::InsufficientQuota { .. } => "InsufficientQuota",
                AIError::InternalError { .. } => "InternalError",
                _ => "Other",
            };
            
            *summary.entry(error_type.to_string()).or_insert(0) += 1;
        }
        
        summary
    }
}

/// Helper function to create appropriate retry configs based on error type
pub fn retry_config_for_error(error: &AIError) -> RetryConfig {
    match error {
        AIError::NetworkError { .. } => RetryConfig::for_network(),
        AIError::RateLimitExceeded { .. } => RetryConfig::for_rate_limits(),
        AIError::ProviderUnavailable { .. } => RetryConfig::for_provider_unavailable(),
        AIError::Timeout { .. } => RetryConfig::for_network(),
        AIError::InternalError { .. } => RetryConfig::default(),
        _ => RetryConfig::new(1, Duration::from_millis(100), Duration::from_secs(1), 1.0), // No retry for non-retryable errors
    }
}

/// Macro for creating retry operations with less boilerplate
#[macro_export]
macro_rules! retry_ai_operation {
    ($retry_manager:expr, $operation:expr) => {
        $retry_manager.execute_with_retry(|| async { $operation }).await
    };
    
    ($retry_manager:expr, $config:expr, $operation:expr) => {
        $retry_manager.execute_with_custom_config(|| async { $operation }, $config).await
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_successful_operation_no_retry() {
        let retry_manager = RetryManager::default();
        let result = retry_manager
            .execute_with_retry(|| async { Ok::<i32, AIError>(42) })
            .await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_retryable_error_eventually_succeeds() {
        let retry_manager = RetryManager::new(RetryConfig {
            max_attempts: 3,
            base_delay: Duration::from_millis(10),
            max_delay: Duration::from_millis(100),
            backoff_multiplier: 2.0,
            jitter_enabled: false,
        });

        let attempt_count = Arc::new(AtomicUsize::new(0));
        let attempt_count_clone = attempt_count.clone();

        let result = retry_manager
            .execute_with_retry(|| {
                let count = attempt_count_clone.clone();
                async move {
                    let current = count.fetch_add(1, Ordering::SeqCst);
                    if current < 2 {
                        Err(AIError::network_error("Temporary failure"))
                    } else {
                        Ok(42)
                    }
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempt_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_non_retryable_error_fails_immediately() {
        let retry_manager = RetryManager::default();
        let attempt_count = Arc::new(AtomicUsize::new(0));
        let attempt_count_clone = attempt_count.clone();

        let result = retry_manager
            .execute_with_retry(|| {
                let count = attempt_count_clone.clone();
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    Err::<i32, AIError>(AIError::auth_failure("Invalid API key"))
                }
            })
            .await;

        assert!(result.is_err());
        assert_eq!(attempt_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_max_attempts_exhausted() {
        let retry_manager = RetryManager::new(RetryConfig {
            max_attempts: 2,
            base_delay: Duration::from_millis(10),
            max_delay: Duration::from_millis(100),
            backoff_multiplier: 2.0,
            jitter_enabled: false,
        });

        let attempt_count = Arc::new(AtomicUsize::new(0));
        let attempt_count_clone = attempt_count.clone();

        let result = retry_manager
            .execute_with_retry(|| {
                let count = attempt_count_clone.clone();
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    Err::<i32, AIError>(AIError::network_error("Always fails"))
                }
            })
            .await;

        assert!(result.is_err());
        assert_eq!(attempt_count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_rate_limit_respects_retry_after() {
        let retry_manager = RetryManager::new(RetryConfig {
            max_attempts: 2,
            base_delay: Duration::from_millis(10),
            max_delay: Duration::from_secs(1),
            backoff_multiplier: 2.0,
            jitter_enabled: false,
        });

        let attempt_count = Arc::new(AtomicUsize::new(0));
        let attempt_count_clone = attempt_count.clone();
        let start_time = std::time::Instant::now();

        let result = retry_manager
            .execute_with_retry(|| {
                let count = attempt_count_clone.clone();
                async move {
                    let current = count.fetch_add(1, Ordering::SeqCst);
                    if current == 0 {
                        Err(AIError::rate_limit(
                            "provider",
                            Some(Duration::from_millis(50)),
                        ))
                    } else {
                        Ok(42)
                    }
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert!(start_time.elapsed() >= Duration::from_millis(50));
    }

    #[tokio::test]
    async fn test_retry_stats() {
        let retry_manager = RetryManager::new(RetryConfig {
            max_attempts: 3,
            base_delay: Duration::from_millis(10),
            max_delay: Duration::from_millis(100),
            backoff_multiplier: 2.0,
            jitter_enabled: false,
        });

        let attempt_count = Arc::new(AtomicUsize::new(0));
        let attempt_count_clone = attempt_count.clone();

        let (result, stats) = retry_manager
            .execute_with_stats(|| {
                let count = attempt_count_clone.clone();
                async move {
                    let current = count.fetch_add(1, Ordering::SeqCst);
                    if current < 2 {
                        Err(AIError::network_error("Temporary failure"))
                    } else {
                        Ok(42)
                    }
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(stats.total_attempts, 3);
        assert!(stats.success);
        assert_eq!(stats.errors.len(), 2);
        assert!(stats.total_delay > Duration::ZERO);
    }

    #[test]
    fn test_retry_config_creation() {
        let config = RetryConfig::for_network();
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.base_delay, Duration::from_millis(500));

        let config = RetryConfig::for_rate_limits();
        assert_eq!(config.max_attempts, 5);
        assert_eq!(config.base_delay, Duration::from_secs(1));

        let config = RetryConfig::for_provider_unavailable();
        assert_eq!(config.max_attempts, 2);
        assert_eq!(config.base_delay, Duration::from_secs(5));
    }

    #[test]
    fn test_error_based_config_selection() {
        let network_error = AIError::network_error("Connection failed");
        let config = retry_config_for_error(&network_error);
        assert_eq!(config.max_attempts, 3);

        let rate_limit_error = AIError::rate_limit("provider", None);
        let config = retry_config_for_error(&rate_limit_error);
        assert_eq!(config.max_attempts, 5);

        let auth_error = AIError::auth_failure("provider");
        let config = retry_config_for_error(&auth_error);
        assert_eq!(config.max_attempts, 1);
    }
}