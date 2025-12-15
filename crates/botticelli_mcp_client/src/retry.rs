//! Retry and circuit breaker logic for tool execution.

use crate::McpClientResult;
use std::time::Duration;
use tokio::time::sleep;

/// Retry configuration for tool execution.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts.
    pub max_attempts: usize,
    /// Initial backoff duration.
    pub initial_backoff: Duration,
    /// Maximum backoff duration.
    pub max_backoff: Duration,
    /// Backoff multiplier.
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(30),
            backoff_multiplier: 2.0,
        }
    }
}

/// Circuit breaker states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CircuitState {
    /// Circuit is closed, requests flow normally.
    Closed,
    /// Circuit is open, requests are rejected.
    Open,
    /// Circuit is half-open, testing if service recovered.
    HalfOpen,
}

/// Circuit breaker for preventing cascading failures.
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    state: CircuitState,
    failure_threshold: usize,
    failure_count: usize,
    success_threshold: usize,
    success_count: usize,
}

impl CircuitBreaker {
    /// Creates a new circuit breaker.
    pub fn new(failure_threshold: usize, success_threshold: usize) -> Self {
        Self {
            state: CircuitState::Closed,
            failure_threshold,
            failure_count: 0,
            success_threshold,
            success_count: 0,
        }
    }

    /// Records a successful execution.
    #[tracing::instrument(skip(self))]
    pub fn record_success(&mut self) {
        match self.state {
            CircuitState::Closed => {
                self.failure_count = 0;
            }
            CircuitState::HalfOpen => {
                self.success_count += 1;
                if self.success_count >= self.success_threshold {
                    tracing::debug!("Circuit breaker closing after successful recovery");
                    self.state = CircuitState::Closed;
                    self.failure_count = 0;
                    self.success_count = 0;
                }
            }
            CircuitState::Open => {
                // Shouldn't happen, but reset if it does
                self.state = CircuitState::Closed;
                self.failure_count = 0;
                self.success_count = 0;
            }
        }
    }

    /// Records a failed execution.
    #[tracing::instrument(skip(self))]
    pub fn record_failure(&mut self) {
        match self.state {
            CircuitState::Closed => {
                self.failure_count += 1;
                if self.failure_count >= self.failure_threshold {
                    tracing::warn!(
                        "Circuit breaker opening after {} failures",
                        self.failure_count
                    );
                    self.state = CircuitState::Open;
                }
            }
            CircuitState::HalfOpen => {
                tracing::warn!("Circuit breaker reopening after failure in half-open state");
                self.state = CircuitState::Open;
                self.failure_count = self.failure_threshold;
                self.success_count = 0;
            }
            CircuitState::Open => {
                // Already open, nothing to do
            }
        }
    }

    /// Attempts to transition from open to half-open.
    #[tracing::instrument(skip(self))]
    pub fn try_reset(&mut self) {
        if self.state == CircuitState::Open {
            tracing::debug!("Circuit breaker entering half-open state");
            self.state = CircuitState::HalfOpen;
            self.success_count = 0;
        }
    }

    /// Returns true if requests should be allowed.
    pub fn is_closed(&self) -> bool {
        matches!(self.state, CircuitState::Closed | CircuitState::HalfOpen)
    }

    /// Returns current state.
    pub fn state(&self) -> &CircuitState {
        &self.state
    }
}

/// Helper for managing retry state and backoff calculation.
pub struct RetryState {
    config: RetryConfig,
    attempt: usize,
    backoff: Duration,
}

impl RetryState {
    /// Creates a new retry state from configuration.
    pub fn new(config: RetryConfig) -> Self {
        let backoff = config.initial_backoff;
        Self {
            config,
            attempt: 0,
            backoff,
        }
    }

    /// Records a retry attempt and sleeps for backoff duration.
    /// Returns None if max attempts reached, Some(attempt_number) otherwise.
    pub async fn retry(&mut self) -> Option<usize> {
        self.attempt += 1;

        if self.attempt > self.config.max_attempts {
            return None;
        }

        if self.attempt > 1 {
            tracing::debug!(
                backoff_ms = self.backoff.as_millis(),
                "Retrying after failure"
            );
            sleep(self.backoff).await;

            // Exponential backoff with cap
            self.backoff = std::cmp::min(
                Duration::from_secs_f64(
                    self.backoff.as_secs_f64() * self.config.backoff_multiplier,
                ),
                self.config.max_backoff,
            );
        }

        Some(self.attempt)
    }
}

/// Retries an operation with exponential backoff.
///
/// This is a convenience function for simple retry scenarios where the operation
/// can be wrapped in a closure. For more control (e.g., when working with &mut self),
/// use `RetryState` directly.
///
/// # Example
///
/// ```no_run
/// use botticelli_mcp_client::retry::{retry_with_backoff, RetryConfig};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = RetryConfig::default();
/// let result = retry_with_backoff(&config, || async {
///     // Your operation here
///     Ok(42)
/// }).await?;
/// # Ok(())
/// # }
/// ```
#[tracing::instrument(skip(operation))]
pub async fn retry_with_backoff<F, Fut, T>(
    config: &RetryConfig,
    mut operation: F,
) -> McpClientResult<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = McpClientResult<T>>,
{
    let mut retry_state = RetryState::new(config.clone());

    loop {
        let attempt = retry_state.retry().await.ok_or_else(|| {
            crate::McpClientError::new(crate::McpClientErrorKind::ToolExecutionFailed(
                "Max retry attempts exhausted".to_string(),
            ))
        })?;

        tracing::debug!(attempt, "Executing operation");

        match operation().await {
            Ok(result) => {
                if attempt > 1 {
                    tracing::debug!(attempt, "Operation succeeded after retry");
                }
                return Ok(result);
            }
            Err(err) => {
                if !err.kind.is_retryable() {
                    tracing::warn!("Error is not retryable, failing immediately");
                    return Err(err);
                }

                if err.kind.should_backoff() {
                    tracing::debug!(
                        backoff_ms = retry_state.backoff.as_millis(),
                        "Backing off due to rate limit"
                    );
                }

                // Continue to next retry attempt
                if attempt >= retry_state.config.max_attempts {
                    tracing::warn!(attempt, "All retry attempts exhausted");
                    return Err(err);
                }
            }
        }
    }
}
