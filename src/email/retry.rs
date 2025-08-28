//! Retry mechanisms for email operations
//!
//! This module provides configurable retry logic with exponential backoff for email operations
//! that may fail due to network issues, server errors, or temporary failures.

use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn, error};

/// Configuration for retry behavior
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: usize,
    /// Initial delay before first retry
    pub initial_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Backoff multiplier (e.g., 2.0 for exponential backoff)
    pub backoff_multiplier: f64,
    /// Whether to add jitter to prevent thundering herd
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

/// Configuration for different operation types
#[derive(Debug, Clone)]
pub struct RetryPolicies {
    pub email_operations: RetryConfig,
    pub calendar_operations: RetryConfig,
    pub search_operations: RetryConfig,
    pub sync_operations: RetryConfig,
}

impl Default for RetryPolicies {
    fn default() -> Self {
        Self {
            // Email operations: Medium retry with reasonable delays
            email_operations: RetryConfig {
                max_attempts: 3,
                initial_delay: Duration::from_millis(200),
                max_delay: Duration::from_secs(10),
                backoff_multiplier: 2.0,
                jitter: true,
            },
            
            // Calendar operations: More aggressive retry for CalDAV
            calendar_operations: RetryConfig {
                max_attempts: 5,
                initial_delay: Duration::from_millis(500),
                max_delay: Duration::from_secs(30),
                backoff_multiplier: 1.5,
                jitter: true,
            },
            
            // Search operations: Quick retry for user-facing operations
            search_operations: RetryConfig {
                max_attempts: 2,
                initial_delay: Duration::from_millis(50),
                max_delay: Duration::from_secs(2),
                backoff_multiplier: 2.0,
                jitter: false,
            },
            
            // Sync operations: Patient retry for background operations
            sync_operations: RetryConfig {
                max_attempts: 5,
                initial_delay: Duration::from_secs(1),
                max_delay: Duration::from_secs(60),
                backoff_multiplier: 2.0,
                jitter: true,
            },
        }
    }
}

/// Determines if an error is retryable
pub trait RetryableError {
    fn is_retryable(&self) -> bool;
    fn is_permanent(&self) -> bool {
        !self.is_retryable()
    }
}

/// Error categories for retry decisions
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorCategory {
    /// Network connectivity issues (retryable)
    Network,
    /// Authentication failures (usually not retryable)
    Authentication,
    /// Server errors (5xx, retryable)
    ServerError,
    /// Client errors (4xx, usually not retryable)
    ClientError,
    /// Resource limits (retryable with backoff)
    ResourceLimit,
    /// Protocol errors (usually not retryable)
    Protocol,
    /// Unknown errors (cautiously retryable)
    Unknown,
}

impl ErrorCategory {
    pub fn is_retryable(&self) -> bool {
        match self {
            ErrorCategory::Network => true,
            ErrorCategory::Authentication => false,
            ErrorCategory::ServerError => true,
            ErrorCategory::ClientError => false,
            ErrorCategory::ResourceLimit => true,
            ErrorCategory::Protocol => false,
            ErrorCategory::Unknown => true, // Conservative approach
        }
    }
}

/// Result of a retry attempt
#[derive(Debug)]
pub struct RetryResult<T, E> {
    pub result: Result<T, E>,
    pub attempt: usize,
    pub total_duration: Duration,
    pub final_attempt: bool,
}

/// Execute an operation with retry logic
pub async fn retry_async<F, Fut, T, E>(
    operation_name: &str,
    mut operation: F,
    config: &RetryConfig,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: RetryableError + std::fmt::Debug,
{
    let start_time = std::time::Instant::now();
    let mut current_delay = config.initial_delay;
    
    for attempt in 1..=config.max_attempts {
        let attempt_start = std::time::Instant::now();
        
        debug!("Attempting {} (attempt {}/{})", operation_name, attempt, config.max_attempts);
        
        match operation().await {
            Ok(result) => {
                if attempt > 1 {
                    debug!(
                        "{} succeeded on attempt {} after {:?}",
                        operation_name,
                        attempt,
                        start_time.elapsed()
                    );
                }
                return Ok(result);
            }
            Err(error) => {
                let attempt_duration = attempt_start.elapsed();
                let total_duration = start_time.elapsed();
                
                if attempt == config.max_attempts {
                    error!(
                        "{} failed permanently after {} attempts ({:?} total): {:?}",
                        operation_name, attempt, total_duration, error
                    );
                    return Err(error);
                }
                
                if error.is_permanent() {
                    error!(
                        "{} failed with permanent error on attempt {}: {:?}",
                        operation_name, attempt, error
                    );
                    return Err(error);
                }
                
                warn!(
                    "{} failed on attempt {} (took {:?}), retrying in {:?}: {:?}",
                    operation_name, attempt, attempt_duration, current_delay, error
                );
                
                // Wait before next attempt
                if config.jitter {
                    let jitter_factor = 1.0 + (rand::random::<f64>() - 0.5) * 0.2; // ±10% jitter
                    let jittered_delay = Duration::from_secs_f64(
                        current_delay.as_secs_f64() * jitter_factor
                    );
                    sleep(jittered_delay).await;
                } else {
                    sleep(current_delay).await;
                }
                
                // Calculate next delay with exponential backoff
                current_delay = std::cmp::min(
                    Duration::from_secs_f64(current_delay.as_secs_f64() * config.backoff_multiplier),
                    config.max_delay,
                );
            }
        }
    }
    
    unreachable!("Loop should have returned or reached max attempts")
}

/// Circuit breaker state
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,      // Normal operation
    Open,        // Failing fast
    HalfOpen,    // Testing if service recovered
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures to open circuit
    pub failure_threshold: usize,
    /// Time to wait before trying half-open state
    pub recovery_timeout: Duration,
    /// Number of successful requests to close circuit from half-open
    pub success_threshold: usize,
    /// Rolling window size for failure tracking
    pub rolling_window: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(60),
            success_threshold: 3,
            rolling_window: Duration::from_secs(300), // 5 minutes
        }
    }
}

/// Circuit breaker for protecting against cascading failures
#[derive(Debug)]
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: tokio::sync::RwLock<CircuitState>,
    failure_count: tokio::sync::RwLock<usize>,
    success_count: tokio::sync::RwLock<usize>,
    last_failure_time: tokio::sync::RwLock<Option<std::time::Instant>>,
    last_request_time: tokio::sync::RwLock<Option<std::time::Instant>>,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: tokio::sync::RwLock::new(CircuitState::Closed),
            failure_count: tokio::sync::RwLock::new(0),
            success_count: tokio::sync::RwLock::new(0),
            last_failure_time: tokio::sync::RwLock::new(None),
            last_request_time: tokio::sync::RwLock::new(None),
        }
    }

    /// Execute operation with circuit breaker protection
    pub async fn call<F, Fut, T, E>(&self, operation: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: std::fmt::Debug,
    {
        // Update last request time
        {
            let mut last_request = self.last_request_time.write().await;
            *last_request = Some(std::time::Instant::now());
        }

        // Check if circuit should be closed after recovery timeout
        self.check_recovery().await;

        // Check current state
        let state = {
            let state_guard = self.state.read().await;
            state_guard.clone()
        };

        match state {
            CircuitState::Open => {
                debug!("Circuit breaker is OPEN, failing fast");
                return Err(CircuitBreakerError::Open);
            }
            CircuitState::Closed | CircuitState::HalfOpen => {
                match operation().await {
                    Ok(result) => {
                        self.on_success().await;
                        Ok(result)
                    }
                    Err(error) => {
                        self.on_failure().await;
                        Err(CircuitBreakerError::Operation(error))
                    }
                }
            }
        }
    }

    async fn on_success(&self) {
        let mut success_count = self.success_count.write().await;
        let mut failure_count = self.failure_count.write().await;
        let mut state = self.state.write().await;

        *success_count += 1;

        match *state {
            CircuitState::HalfOpen => {
                if *success_count >= self.config.success_threshold {
                    debug!("Circuit breaker transitioning to CLOSED after {} successes", *success_count);
                    *state = CircuitState::Closed;
                    *failure_count = 0;
                    *success_count = 0;
                }
            }
            CircuitState::Closed => {
                // Reset failure count on success in closed state
                *failure_count = 0;
            }
            CircuitState::Open => {
                // Should not happen, but handle gracefully
                warn!("Received success while circuit was open - transitioning to half-open");
                *state = CircuitState::HalfOpen;
                *success_count = 1;
            }
        }
    }

    async fn on_failure(&self) {
        let mut failure_count = self.failure_count.write().await;
        let mut success_count = self.success_count.write().await;
        let mut state = self.state.write().await;
        let mut last_failure_time = self.last_failure_time.write().await;

        *failure_count += 1;
        *success_count = 0;
        *last_failure_time = Some(std::time::Instant::now());

        match *state {
            CircuitState::Closed => {
                if *failure_count >= self.config.failure_threshold {
                    debug!("Circuit breaker transitioning to OPEN after {} failures", *failure_count);
                    *state = CircuitState::Open;
                }
            }
            CircuitState::HalfOpen => {
                debug!("Circuit breaker transitioning back to OPEN after failure in half-open state");
                *state = CircuitState::Open;
            }
            CircuitState::Open => {
                // Already open, just update failure time
            }
        }
    }

    async fn check_recovery(&self) {
        let last_failure_time = {
            let guard = self.last_failure_time.read().await;
            *guard
        };

        if let Some(last_failure) = last_failure_time {
            if last_failure.elapsed() >= self.config.recovery_timeout {
                let mut state = self.state.write().await;
                if *state == CircuitState::Open {
                    debug!("Circuit breaker transitioning to HALF_OPEN for recovery test");
                    *state = CircuitState::HalfOpen;
                    
                    // Reset counters
                    let mut success_count = self.success_count.write().await;
                    *success_count = 0;
                }
            }
        }
    }

    pub async fn get_state(&self) -> CircuitState {
        let guard = self.state.read().await;
        guard.clone()
    }

    pub async fn get_failure_count(&self) -> usize {
        let guard = self.failure_count.read().await;
        *guard
    }
}

/// Circuit breaker specific errors
#[derive(Debug, thiserror::Error)]
pub enum CircuitBreakerError<E> {
    #[error("Circuit breaker is open, failing fast")]
    Open,
    #[error("Operation failed: {0}")]
    Operation(E),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[derive(Debug, Clone)]
    struct TestError {
        retryable: bool,
    }

    impl RetryableError for TestError {
        fn is_retryable(&self) -> bool {
            self.retryable
        }
    }

    #[tokio::test]
    async fn test_retry_success_on_first_attempt() {
        let config = RetryConfig {
            max_attempts: 3,
            initial_delay: Duration::from_millis(10),
            ..Default::default()
        };

        let result = retry_async(
            "test_operation",
            || async { Ok::<i32, TestError>(42) },
            &config,
        ).await;

        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_retry_success_after_failures() {
        let attempt_count = Arc::new(AtomicUsize::new(0));
        let config = RetryConfig {
            max_attempts: 3,
            initial_delay: Duration::from_millis(10),
            jitter: false,
            ..Default::default()
        };

        let result = retry_async(
            "test_operation",
            || {
                let count = attempt_count.clone();
                async move {
                    let current_attempt = count.fetch_add(1, Ordering::SeqCst) + 1;
                    if current_attempt < 3 {
                        Err(TestError { retryable: true })
                    } else {
                        Ok(42)
                    }
                }
            },
            &config,
        ).await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempt_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_permanent_failure() {
        let config = RetryConfig {
            max_attempts: 3,
            initial_delay: Duration::from_millis(10),
            ..Default::default()
        };

        let result = retry_async(
            "test_operation",
            || async { Err::<i32, TestError>(TestError { retryable: false }) },
            &config,
        ).await;

        assert!(result.is_err());
        assert!(!result.unwrap_err().is_retryable());
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens_after_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            recovery_timeout: Duration::from_millis(100),
            success_threshold: 2,
            rolling_window: Duration::from_secs(60),
        };
        
        let circuit_breaker = CircuitBreaker::new(config);
        
        // Cause failures to open circuit
        for _ in 0..3 {
            let result = circuit_breaker.call(|| async { Err::<(), &str>("test error") }).await;
            assert!(matches!(result, Err(CircuitBreakerError::Operation(_))));
        }
        
        // Circuit should be open now
        assert_eq!(circuit_breaker.get_state().await, CircuitState::Open);
        
        // Next call should fail fast
        let result = circuit_breaker.call(|| async { Ok::<(), &str>(()) }).await;
        assert!(matches!(result, Err(CircuitBreakerError::Open)));
    }

    #[tokio::test]
    async fn test_circuit_breaker_recovery() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            recovery_timeout: Duration::from_millis(50),
            success_threshold: 1,
            rolling_window: Duration::from_secs(60),
        };
        
        let circuit_breaker = CircuitBreaker::new(config);
        
        // Cause failures to open circuit
        for _ in 0..2 {
            let _ = circuit_breaker.call(|| async { Err::<(), &str>("test error") }).await;
        }
        
        assert_eq!(circuit_breaker.get_state().await, CircuitState::Open);
        
        // Wait for recovery timeout
        sleep(Duration::from_millis(60)).await;
        
        // Circuit should transition to half-open and allow success
        let result = circuit_breaker.call(|| async { Ok::<(), &str>(()) }).await;
        assert!(result.is_ok());
        
        // Circuit should be closed now
        assert_eq!(circuit_breaker.get_state().await, CircuitState::Closed);
    }
}