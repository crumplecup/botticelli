//! Retry behavior trait for error types.

/// Trait for errors that support retry logic.
///
/// Errors implementing this trait can indicate whether they represent
/// transient failures that should be retried, and provide retry strategy
/// parameters.
///
/// # Examples
///
/// ```
/// use botticelli_interface::RetryableError;
///
/// struct MyError {
///     retryable: bool,
/// }
///
/// impl RetryableError for MyError {
///     fn is_retryable(&self) -> bool {
///         self.retryable
///     }
/// }
/// ```
pub trait RetryableError {
    /// Returns true if this error should trigger a retry.
    ///
    /// Transient errors like 503 (service unavailable), 429 (rate limit),
    /// or network timeouts should return true. Permanent errors like 401
    /// (unauthorized) or 400 (bad request) should return false.
    fn is_retryable(&self) -> bool;

    /// Get retry strategy parameters for this error.
    ///
    /// Returns `(initial_backoff_ms, max_retries, max_delay_secs)`.
    /// Default implementation returns standard parameters.
    ///
    /// Override this to provide error-specific retry strategies:
    /// - Rate limit errors (429): Longer delays, fewer retries
    /// - Server overload (503): Standard delays, more patient
    /// - Server errors (500): Quick retries, fail fast
    fn retry_strategy_params(&self) -> (u64, usize, u64) {
        (2000, 5, 60) // Default: 2s initial, 5 retries, 60s cap
    }
}
