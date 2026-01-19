//! Rate limit detection for model selection.
//!
//! This module provides integration between the rate limit detection system
//! and the model selection fallback strategy.

use crate::ModelId;
use derive_getters::Getters;
use derive_new::new;
use rmcp::tool;
use std::time::{Duration, Instant};
use tracing::{debug, instrument, warn};

/// Tracks rate limit status for a specific model.
#[derive(Debug, Clone, Getters, new)]
pub struct RateLimitStatus {
    /// The model that hit rate limit
    model_id: ModelId,
    /// When the rate limit was detected
    detected_at: Instant,
    /// Estimated time when limit will reset (if known)
    reset_at: Option<Instant>,
}

impl RateLimitStatus {
    /// Check if this rate limit is likely expired based on elapsed time.
    ///
    /// Uses conservative estimates:
    /// - If reset_at is known, check if current time is past it
    /// - Otherwise, assume 60 second cooldown period
    #[tool]
    #[instrument(skip(self))]
    pub fn is_likely_expired(&self) -> bool {
        let now = Instant::now();

        if let Some(reset_at) = self.reset_at {
            let expired = now >= reset_at;
            debug!(
                model_id = ?self.model_id,
                expired,
                elapsed_secs = now.duration_since(self.detected_at).as_secs(),
                "Checking known reset time"
            );
            expired
        } else {
            // Conservative 60 second cooldown if no reset time known
            let elapsed = now.duration_since(self.detected_at);
            let expired = elapsed >= Duration::from_secs(60);
            debug!(
                model_id = ?self.model_id,
                expired,
                elapsed_secs = elapsed.as_secs(),
                "Checking default cooldown period"
            );
            expired
        }
    }
}

/// Detects rate limit errors from provider-specific error conditions.
#[derive(Debug, Clone, Default)]
pub struct RateLimitDetector {
    tracked: std::collections::HashMap<crate::ModelFamily, RateLimitStatus>,
}

impl RateLimitDetector {
    /// Create a new detector with no tracked rate limits.
    #[tool]
    #[instrument]
    pub fn new() -> Self {
        Self::default()
    }
    /// Check if an error message indicates a rate limit.
    #[tool]
    #[instrument(skip(self))]
    pub fn is_rate_limit_message(&self, error: &str) -> bool {
        let msg_lower = error.to_lowercase();
        msg_lower.contains("rate limit")
            || msg_lower.contains("quota exceeded")
            || msg_lower.contains("resource exhausted")
            || msg_lower.contains("too many requests")
    }

    /// Detect rate limit status from error message.
    #[tool]
    #[instrument(skip(self))]
    pub fn detect_from_message(&self, error: &str) -> Option<Duration> {
        if !self.is_rate_limit_message(error) {
            return None;
        }
        Self::parse_reset_from_message(error)
    }

    /// Record a rate limit for a family.
    #[tool]
    #[instrument(skip(self))]
    pub fn record(&mut self, family: crate::ModelFamily, model_id: ModelId) {
        let status = RateLimitStatus::new(model_id, Instant::now(), None);
        self.tracked.insert(family, status);
    }

    /// Check if a family is currently rate limited.
    #[tool]
    #[instrument(skip(self))]
    pub fn is_rate_limited(&self, family: crate::ModelFamily) -> bool {
        self.tracked
            .get(&family)
            .map(|status| !status.is_likely_expired())
            .unwrap_or(false)
    }

    /// Get status for a family.
    #[tool]
    #[instrument(skip(self))]
    pub fn get_status(&self, family: crate::ModelFamily) -> Option<&RateLimitStatus> {
        self.tracked.get(&family)
    }

    /// Parse reset duration from error message.
    ///
    /// Looks for patterns like "retry after 30s" or "reset in 1m".
    fn parse_reset_from_message(message: &str) -> Option<Duration> {
        let msg_lower = message.to_lowercase();

        // Try to find "retry after Ns" or "retry after Nm"
        if let Some(pos) = msg_lower.find("retry after") {
            let remainder = &msg_lower[pos + 11..];

            // Try to parse number + unit
            if let Some(seconds) = remainder.trim().strip_suffix('s')
                && let Ok(secs) = seconds.trim().parse::<u64>()
            {
                debug!(seconds = secs, "Parsed retry duration (seconds)");
                return Some(Duration::from_secs(secs));
            }

            if let Some(minutes) = remainder.trim().strip_suffix('m')
                && let Ok(mins) = minutes.trim().parse::<u64>()
            {
                debug!(minutes = mins, "Parsed retry duration (minutes)");
                return Some(Duration::from_secs(mins * 60));
            }
        }

        warn!(
            message,
            "Could not parse reset duration from rate limit message"
        );
        None
    }
}
