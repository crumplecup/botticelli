//! Rate limiting for Gemini Live API WebSocket connections.
//!
//! Unlike REST APIs where each request is independent, WebSocket connections are persistent.
//! This module tracks messages sent over WebSocket connections to enforce rate limits.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, trace, warn};

/// Rate limiter for WebSocket messages.
///
/// Tracks messages sent over persistent WebSocket connections and enforces
/// rate limits by pausing before sending when limits are approached.
///
/// # Design
///
/// - **Message-based**: Tracks individual messages, not connections
/// - **Rolling window**: Uses 60-second windows for RPM limits
/// - **Proactive**: Sleeps before sending if limit would be exceeded
/// - **Thread-safe**: Can be shared across multiple sessions
///
/// # Example
///
/// ```no_run
/// use botticelli::LiveRateLimiter;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let limiter = LiveRateLimiter::new(10); // 10 messages per minute
///
/// // Before sending each message
/// limiter.acquire().await;
/// // ... send message ...
/// limiter.record();
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct LiveRateLimiter {
    /// Messages sent in current window
    messages_sent: AtomicU32,
    /// Start of current time window
    window_start: Arc<Mutex<Instant>>,
    /// Maximum messages per minute (RPM limit)
    max_messages_per_minute: u32,
}

impl LiveRateLimiter {
    /// Create a new rate limiter.
    ///
    /// # Arguments
    ///
    /// * `max_messages_per_minute` - Maximum messages allowed per 60-second window
    ///
    /// # Example
    ///
    /// ```
    /// use botticelli::LiveRateLimiter;
    ///
    /// let limiter = LiveRateLimiter::new(10); // 10 messages per minute
    /// ```
    pub fn new(max_messages_per_minute: u32) -> Self {
        debug!(
            "Creating LiveRateLimiter with {} messages/minute",
            max_messages_per_minute
        );

        Self {
            messages_sent: AtomicU32::new(0),
            window_start: Arc::new(Mutex::new(Instant::now())),
            max_messages_per_minute,
        }
    }

    /// Acquire permission to send a message.
    ///
    /// Blocks if sending would exceed the rate limit, waiting until the next
    /// time window begins.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use botticelli::LiveRateLimiter;
    /// # #[tokio::main]
    /// # async fn main() {
    /// # let limiter = LiveRateLimiter::new(10);
    /// // Wait for permission to send
    /// limiter.acquire().await;
    /// // ... send message ...
    /// limiter.record();
    /// # }
    /// ```
    pub async fn acquire(&self) {
        let current_count = self.messages_sent.load(Ordering::SeqCst);

        // Check if we're at the limit
        if current_count >= self.max_messages_per_minute {
            let window_start = self.window_start.lock().await;
            let elapsed = window_start.elapsed();

            // If we're still in the current window, wait until it expires
            if elapsed < Duration::from_secs(60) {
                let wait_time = Duration::from_secs(60) - elapsed;
                warn!(
                    "Rate limit reached ({}/{}), waiting {:?}",
                    current_count, self.max_messages_per_minute, wait_time
                );
                drop(window_start); // Release lock before sleeping
                tokio::time::sleep(wait_time).await;

                // Reset for new window
                self.reset_window().await;
            } else {
                // Window already expired, reset immediately
                drop(window_start);
                self.reset_window().await;
            }
        }

        trace!(
            "Rate limit permission granted ({}/{})",
            current_count + 1,
            self.max_messages_per_minute
        );
    }

    /// Record that a message was sent.
    ///
    /// Should be called immediately after successfully sending a message.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use botticelli::LiveRateLimiter;
    /// # #[tokio::main]
    /// # async fn main() {
    /// # let limiter = LiveRateLimiter::new(10);
    /// limiter.acquire().await;
    /// // ... send message ...
    /// limiter.record();
    /// # }
    /// ```
    pub fn record(&self) {
        let new_count = self.messages_sent.fetch_add(1, Ordering::SeqCst) + 1;
        trace!(
            "Message recorded ({}/{})",
            new_count, self.max_messages_per_minute
        );
    }

    /// Reset the rate limit window.
    ///
    /// Starts a new 60-second window with zero messages sent.
    async fn reset_window(&self) {
        let mut window_start = self.window_start.lock().await;
        *window_start = Instant::now();
        self.messages_sent.store(0, Ordering::SeqCst);
        debug!("Rate limit window reset");
    }

    /// Get the current message count.
    ///
    /// Useful for monitoring and debugging.
    pub fn current_count(&self) -> u32 {
        self.messages_sent.load(Ordering::SeqCst)
    }

    /// Get the maximum messages per minute.
    pub fn max_per_minute(&self) -> u32 {
        self.max_messages_per_minute
    }
}

impl Clone for LiveRateLimiter {
    fn clone(&self) -> Self {
        Self {
            messages_sent: AtomicU32::new(self.messages_sent.load(Ordering::SeqCst)),
            window_start: Arc::clone(&self.window_start),
            max_messages_per_minute: self.max_messages_per_minute,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_allows_under_limit() {
        let limiter = LiveRateLimiter::new(5);

        // Should allow 5 messages without blocking
        for i in 0..5 {
            limiter.acquire().await;
            limiter.record();
            assert_eq!(limiter.current_count(), i + 1);
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_blocks_at_limit() {
        let limiter = LiveRateLimiter::new(2);

        // Send 2 messages
        limiter.acquire().await;
        limiter.record();
        limiter.acquire().await;
        limiter.record();

        // Third message should block (but we won't wait the full minute in test)
        let start = Instant::now();
        tokio::select! {
            _ = limiter.acquire() => {
                // Should not complete immediately
                let elapsed = start.elapsed();
                assert!(elapsed >= Duration::from_millis(100), "Should have blocked");
            }
            _ = tokio::time::sleep(Duration::from_millis(200)) => {
                // Test times out after 200ms, which is expected
            }
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_resets_window() {
        let limiter = LiveRateLimiter::new(3);

        // Send 3 messages
        for _ in 0..3 {
            limiter.acquire().await;
            limiter.record();
        }

        assert_eq!(limiter.current_count(), 3);

        // Reset window
        limiter.reset_window().await;

        assert_eq!(limiter.current_count(), 0);

        // Should allow 3 more messages
        for i in 0..3 {
            limiter.acquire().await;
            limiter.record();
            assert_eq!(limiter.current_count(), i + 1);
        }
    }
}
