//! Budget tracking for carousel operations.

use botticelli_error::{RateLimitError, RateLimitErrorKind};
use botticelli_interface::Tier;
use derive_getters::Getters;
use rmcp::tool;
use std::time::{Duration, Instant};

/// Budget tracker for carousel operations.
///
/// Tracks token and request consumption across rate limit windows
/// to ensure carousel operations stay within configured limits.
#[derive(Debug, Clone)]
pub struct Budget<T: Tier + std::fmt::Debug> {
    /// Rate limit configuration
    config: T,

    /// Tokens consumed in current minute
    tokens_per_minute: u64,

    /// Tokens consumed in current day
    tokens_per_day: u64,

    /// Requests consumed in current minute
    requests_per_minute: u64,

    /// Requests consumed in current day
    requests_per_day: u64,

    /// Start of current minute window
    minute_window_start: Instant,

    /// Start of current day window
    day_window_start: Instant,
}

impl<T: Tier + std::fmt::Debug> Budget<T> {
    /// Creates a new budget tracker with the given rate limits.
    #[tool]
    #[tracing::instrument]
    pub fn new(config: T) -> Self {
        let now = Instant::now();
        Self {
            config,
            tokens_per_minute: 0,
            tokens_per_day: 0,
            requests_per_minute: 0,
            requests_per_day: 0,
            minute_window_start: now,
            day_window_start: now,
        }
    }

    /// Gets the rate limit configuration.
    #[tool]
    #[tracing::instrument(skip(self))]
    pub fn config(&self) -> &T {
        &self.config
    }

    /// Resets window counters if windows have expired.
    #[tool]
    #[tracing::instrument(skip(self))]
    pub fn reset_windows(&mut self) {
        let now = Instant::now();

        // Reset minute window if it has expired
        if now.duration_since(self.minute_window_start) >= Duration::from_secs(60) {
            self.tokens_per_minute = 0;
            self.requests_per_minute = 0;
            self.minute_window_start = now;
        }

        // Reset day window if it has expired (86400 seconds = 24 hours)
        if now.duration_since(self.day_window_start) >= Duration::from_secs(86400) {
            self.tokens_per_day = 0;
            self.requests_per_day = 0;
            self.day_window_start = now;
        }
    }

    /// Checks if the budget can accommodate a request with the given token count.
    ///
    /// Returns true if the request fits within all rate limit windows.
    #[tool]
    #[tracing::instrument(skip(self))]
    pub fn can_afford(&mut self, tokens: u64) -> bool {
        self.reset_windows();

        // Check minute limits (None = unlimited)
        let tokens_ok_minute = self
            .config
            .tpm()
            .is_none_or(|limit| self.tokens_per_minute + tokens <= limit);
        let requests_ok_minute = self
            .config
            .rpm()
            .is_none_or(|limit| self.requests_per_minute < limit as u64);

        // Check day limits (None = unlimited)
        let tokens_ok_day = self
            .config
            .tpd()
            .is_none_or(|limit| self.tokens_per_day + tokens <= limit);
        let requests_ok_day = self
            .config
            .rpd()
            .is_none_or(|limit| self.requests_per_day < limit as u64);

        tokens_ok_minute && requests_ok_minute && tokens_ok_day && requests_ok_day
    }

    /// Records consumption of tokens and a request.
    ///
    /// # Errors
    ///
    /// Returns an error if the consumption would exceed rate limits.
    #[tool]
    #[tracing::instrument(skip(self))]
    pub fn consume(&mut self, tokens: u64) -> Result<(), RateLimitError> {
        if !self.can_afford(tokens) {
            return Err(RateLimitError::new(RateLimitErrorKind::BudgetExceeded {
                requested_tokens: tokens,
                available_tokens_minute: self.config.tpm().map_or(u64::MAX, |limit| {
                    limit.saturating_sub(self.tokens_per_minute)
                }),
                available_tokens_day: self
                    .config
                    .tpd()
                    .map_or(u64::MAX, |limit| limit.saturating_sub(self.tokens_per_day)),
                available_requests_minute: self.config.rpm().map_or(u64::MAX, |limit| {
                    (limit as u64).saturating_sub(self.requests_per_minute)
                }),
                available_requests_day: self.config.rpd().map_or(u64::MAX, |limit| {
                    (limit as u64).saturating_sub(self.requests_per_day)
                }),
            }));
        }

        self.tokens_per_minute += tokens;
        self.tokens_per_day += tokens;
        self.requests_per_minute += 1;
        self.requests_per_day += 1;

        tracing::debug!(
            tokens_per_minute = self.tokens_per_minute,
            tokens_per_day = self.tokens_per_day,
            requests_per_minute = self.requests_per_minute,
            requests_per_day = self.requests_per_day,
            "Consumed budget"
        );

        Ok(())
    }

    /// Returns the remaining budget in the current windows.
    #[tool]
    #[tracing::instrument(skip(self))]
    pub fn remaining(&mut self) -> BudgetRemaining {
        self.reset_windows();

        BudgetRemaining {
            tokens_per_minute: self.config.tpm().map_or(u64::MAX, |limit| {
                limit.saturating_sub(self.tokens_per_minute)
            }),
            tokens_per_day: self
                .config
                .tpd()
                .map_or(u64::MAX, |limit| limit.saturating_sub(self.tokens_per_day)),
            requests_per_minute: self.config.rpm().map_or(u64::MAX, |limit| {
                (limit as u64).saturating_sub(self.requests_per_minute)
            }),
            requests_per_day: self.config.rpd().map_or(u64::MAX, |limit| {
                (limit as u64).saturating_sub(self.requests_per_day)
            }),
        }
    }
}

/// Remaining budget across rate limit windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema, Getters, elicitation::Elicit)]
pub struct BudgetRemaining {
    /// Remaining tokens in current minute
    tokens_per_minute: u64,

    /// Remaining tokens in current day
    tokens_per_day: u64,

    /// Remaining requests in current minute
    requests_per_minute: u64,

    /// Remaining requests in current day
    requests_per_day: u64,
}
