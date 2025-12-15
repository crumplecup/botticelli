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

/// Retries an operation with exponential backoff.
#[tracing::instrument(skip(operation))]
pub async fn retry_with_backoff<F, Fut, T>(
    config: &RetryConfig,
    mut operation: F,
) -> McpClientResult<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = McpClientResult<T>>,
{
    let mut attempt = 0;
    let mut backoff = config.initial_backoff;

    loop {
        attempt += 1;
        tracing::debug!(attempt, "Executing operation");

        match operation().await {
            Ok(result) => {
                if attempt > 1 {
                    tracing::debug!(attempt, "Operation succeeded after retry");
                }
                return Ok(result);
            }
            Err(err) => {
                if attempt >= config.max_attempts {
                    tracing::warn!(attempt, "All retry attempts exhausted");
                    return Err(err);
                }

                if !err.kind.is_retryable() {
                    tracing::warn!("Error is not retryable, failing immediately");
                    return Err(err);
                }

                if err.kind.should_backoff() {
                    tracing::debug!(
                        backoff_ms = backoff.as_millis(),
                        "Backing off due to rate limit"
                    );
                } else {
                    tracing::debug!(backoff_ms = backoff.as_millis(), "Retrying after failure");
                }

                sleep(backoff).await;

                // Exponential backoff with cap
                backoff = std::cmp::min(
                    Duration::from_secs_f64(backoff.as_secs_f64() * config.backoff_multiplier),
                    config.max_backoff,
                );
            }
        }
    }
}
